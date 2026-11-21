# typolish design: Processing model

Part of the [typolish design](./README.md). Section numbers are those of the whole design.

## 3. Processing model

The core is a pass over characters with context; it uses no regular expressions. Source files named
here are planned under `dev/typolish/src/` (section 14.6).

### 3.1 Pipeline

```text
input (&str | pieces)
  │  0. prefilter: no trigger byte for any active rule → Cow::Borrowed     (3.2)
  │  1. ingest: pieces → stream of atoms (Text, Slot, Protected, Markup);
  │     protection delimiters cut Protected atoms out of Text             (3.4)
  │  2. windows: split by paragraph                                        (3.3)  ┐
  │  3. lexer: atoms → class-invariant tokens over grapheme clusters      (3.5)  │
  │  4. classifier: fragments → kind, FROZEN flag                          (3.6)  │ per window
  │  5. phases 1..7: one pass each over a snapshot, rules in table order   (3.8)  │
  │  6. render: tokens → output in the requested kind                      (3.10) ┘
  ▼
Cow<str> | Template (+ diagnostics, + journal of edits and protected spans when recording)
```

Files: `engine/ingest.rs`, `engine/window.rs`, `engine/lex.rs`, `engine/classify.rs`,
`engine/phase.rs`, `engine/render.rs`, `engine/record.rs`, `engine/scratch.rs`; rules in
`rules/{symbols,ellipsis,dash,range,number,quotes,nbsp}.rs`.

### 3.2 Prefilter

When a typographer is built, every active rule contributes the bytes that can trigger it to a
256-bit set: `"` `'` `-` `.` `(` `<` `>` `!` `+` `x` `X` the digits, `№` and `§` lead bytes, the
lead bytes of `х` `Х` and of typographic characters that rules use as context, and the space when a
no-break rule is active. A call first scans the input bytes against the set and against the bytes
the output kind must encode (`& < > " '` for HTML, `\ " '` and controls for escaped output, and the
typographic set). If no byte matches, it returns `Cow::Borrowed` without lexing. For short interface
strings without signs this skips the whole pipeline; the scan is linear and allocates nothing.

### 3.3 Windows

A window is a paragraph: its boundary is two or more line breaks with only horizontal whitespace
between them, or U+2029 PARAGRAPH SEPARATOR. No rule looks across a window boundary, and the scope
of quote pairing is the window (section 6.10). For pieces, boundaries are found the same way in the
text of the pieces; a slot is never a boundary, so a message without blank lines is one window.

Why: a long text does not hold all its tokens in memory at once; an error in one paragraph (an
unpaired quote) does not affect its neighbours; and the window is the natural unit for "all or
nothing" in quote pairing.

### 3.4 Ingest

Input becomes a stream of atoms:

```rust
enum Atom<'s> {
    Text { piece: u32, text: &'s str, base: u32 },     // base: byte offset in the input piece
    Slot { piece: u32, slot: Slot },
    Protected { piece: u32, text: &'s str, base: u32, delimited: bool },
    Markup { piece: u32, text: &'s str, base: u32 },
}
```

String input is one `Text` piece with index 0. Protection delimiters (section 10.1) cut `Text` atoms
into `Text` and `Protected { delimited: true }`; the delimiters belong to the protected atom.
Delimiters are searched within one text piece only.

A slot or a protected atom becomes one opaque token: it is always `FROZEN`, takes part in rule
context through its class (section 3.5), and is never inside the range of an edit. A markup atom
becomes a zero-width token that is transparent for context: rules look through it for neighbours,
and edits never touch it.

### 3.5 Lexer: class-invariant tokens

Text is cut into tokens along extended grapheme clusters (`unicode-segmentation`, section 5.1). For
ASCII runs a cluster is one byte except `\r\n`, so the fast path skips the tables. A token's class
comes from the first character of its cluster.

```rust
pub(crate) struct Tok<'s> {
    kind: Kind,
    text: &'s str,  // current text: a slice of the input, a 'static literal, or language data
    src: SrcSpan,   // piece and byte range in the input piece (positions for reports)
    glue: Glue,     // for Space tokens: None | NoBreak | Narrow (section 3.9)
    flags: Flags,   // FROZEN, CHANGED, INSERTED
}

pub(crate) enum Kind {
    Word(Script),     // letters with their combining marks; Script: Cyrillic | Latin | Greek | Other | Mixed
    Number,           // a run of ASCII digits
    Space,            // a run of horizontal whitespace: U+0020, U+00A0, U+202F, U+2009, tab, ...
    Newline,          // \n, \r\n, \r, U+0085, U+2028
    Format,           // U+200B..U+200F, U+202A..U+202E, U+2060..U+2064, U+2066..U+2069, U+FEFF
    Quote,            // " ' « » „ “ ” ‘ ’ ‚ ′ ″
    Dash,             // a run of '-' (length 1..), U+2010 ‐, U+2011 ‑, U+2012 ‒, – — U+2015 ―, −
    Dots,             // a run of '.' (length 1..), …
    Operator(Op),     // -> <- <-> >= <= != +- => >> << == and → ← ↔ ≥ ≤ ≠ ±
    LegalMark(Mark),  // (c) (с) (r) (tm) in any case, and © ® ™
    Times,            // x X х Х between numbers (assigned by the classifier), and ×
    Open(char),       // ( [ {
    Close(char),      // ) ] }
    Punct(char),      // , ; : ! ? № § ¶ % ‰ ° / \ @ # & * + = < > | ~ ^ $ _ `
    Symbol,           // other clusters: emoji, CJK, a punctuation mark with a combining mark
    Slot(SlotKind),   // opaque placeholder
    Protected,        // protected text
    Markup,           // zero-width, transparent
}
```

**Class invariance.** A plain form and all its typographic forms belong to one kind: `"` and `«` are
`Quote`; `-`, `--` and `—` are `Dash`; `...` and `…` are `Dots`; `->` and `→` are `Operator`; `(c)`
and `©` are `LegalMark`; ` ` and `⍽` are `Space`. Kinds carry no field describing the form; the form
is read from `text`. `Op` and `Mark` are semantic ids (arrow right, copyright), the same for both
forms. Multi-character plain forms are lexed greedily from a fixed table into one token, so
replacing `(c)` with `©` changes the text of one token rather than turning three tokens into one.
Operators are lexed only whole from the table: `-->` is not in it, so the lexer sees `Dash` of
length 2 and `Punct('>')`, and no dash rule fires (there is no space to the right of `--`).

Every rule edit changes only the `text` or the `glue` of a token, never its kind. The only
structural edits are inserting a `Space` token and removing a `Space` token. Section 4 builds the
idempotence argument on this.

A cluster of several characters that starts with a punctuation mark (`"` + U+0301) is a `Symbol` and
no rule changes it. A cluster that starts with a whitespace character and carries a combining mark
is a `Space` token flagged `FROZEN`: it separates fragments but is never glued or removed, and the
mark stays where it is (section 5.3). Emoji with ZWJ, modifiers, flags and keycaps (`1️⃣`) are single
`Symbol` tokens and are never cut or replaced.

### 3.6 Fragments and the classifier

A **fragment** is a maximal run of tokens of one window without `Space` and `Newline` tokens
(no-break spaces are `Space` too, since `char::is_whitespace` holds for U+00A0, U+202F and U+2009).
Opaque tokens belong to fragments like text: `{host}:{port}` is one fragment.

Before classification the **wrapper** is cut off by tokens: at the start `( « „ “ ‘ " '` and runs of
dots; at the end `) » “ ” ’ " ' . , ; : ! ? …`, runs of dots and `″ ′`. `[ ]`, `{ }` and `< >` are
not wrapper: they belong to technical notation (`[::1]:8080`, `{x}`, `<T>`). The wrapper stays
editable (it is sentence punctuation). The rest, the **core**, gets one kind. A `LegalMark` token is
one token, so `(c)` keeps its parentheses.

The classifier scans the core left to right at most three times (constant work per character) and
applies the checks strictly in this order; the first check that matches decides:

| No. | Check on the core                                                                                                                                                                                                                                                     | Kind                                        | Frozen                                  |
| --- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------- | --------------------------------------- |
| 0   | equals an entry of the language's dictionaries: units (also user-added), abbreviation parts, elisions, month forms, atom exceptions (`e.g.`, `i.e.`, `etc.`, `vs.`, `cf.`, `a.m.`, `p.m.`); an entry ending in `.` matches the core plus the first stripped full stop | `Dictionary`                                | no                                      |
| 1   | contains `://`, or starts with `www.`                                                                                                                                                                                                                                 | `Url`                                       | whole core                              |
| 2   | `@` not first and not last; or `@` followed by a letter or digit at the start                                                                                                                                                                                         | `Email`                                     | whole core                              |
| 3   | starts with `-` and an ASCII letter, or with `--` and an ASCII letter or digit                                                                                                                                                                                        | `Flag`                                      | whole core                              |
| 4   | contains `/` or `\`                                                                                                                                                                                                                                                   | `Path`                                      | whole core                              |
| 5   | `HH:MM[:SS]` `-` or `–` `HH:MM[:SS]` (hours 0 to 23, minutes 00 to 59)                                                                                                                                                                                                | `TimeRange`                                 | the times; the separator stays editable |
| 6   | `:` not first and not last, or the core starts with `::`                                                                                                                                                                                                              | `Colon` (host:port, IPv6, MAC, time)        | whole core                              |
| 7   | only operator characters `- < > = ! + ~ \| & * ^ %` (`->`, `>=`, `<->`, `=>`, `>>`, `--`)                                                                                                                                                                             | `OperatorRun`                               | no (symbol and dash rules decide)       |
| 8   | any of `= < > \| & ^ ~ * # $ % + _` or a backtick not first and not last; or the core starts with `<` or ends with `>`                                                                                                                                                | `Operator`, `Identifier` (for `_`), `Angle` | whole core                              |
| 9   | UUID 8-4-4-4-12 of hex digits                                                                                                                                                                                                                                         | `Uuid`                                      | whole core                              |
| 10  | `0x` or `0X` and hex digits                                                                                                                                                                                                                                           | `Hex`                                       | whole core                              |
| 11  | at least 8 characters, all hex, with a digit and a letter a to f                                                                                                                                                                                                      | `Hex`                                       | whole core                              |
| 12  | `YYYY-MM-DD` or `YYYY-MM` with month 01 to 12                                                                                                                                                                                                                         | `IsoDate`                                   | whole core                              |
| 13  | `[vV]?` number (`.` number){2,} with an optional `-` or `+` tail; four numbers 0 to 255 joined by `.`; `D.M.YYYY`                                                                                                                                                     | `Version`                                   | whole core                              |
| 14  | two or more groups of one or two letters joined by `.`, ending in `.` (`т.е.`, `А.С.`, `Ph.D.`)                                                                                                                                                                       | `Abbreviation`                              | no                                      |
| 15  | a number: optional sign (`-`, `+`, `+-`, `−`, `±`), digits, optional decimal part with `.` or `,`, optional `%` or `‰`                                                                                                                                                | `Number`                                    | no                                      |
| 16  | `D+` then `x`, `X`, `х`, `Х` or `×` then `D+`, the first factor without a leading zero                                                                                                                                                                                | `Dimensions` (the letter becomes `Times`)   | no                                      |
| 17  | `N-N` or `N–N`, N an unsigned integer, a decimal with the language's separator, or a Roman numeral                                                                                                                                                                    | `Range`                                     | no (only range rules edit its dash)     |
| 18  | a single `.` between letters or digits (runs of two or more dots do not count)                                                                                                                                                                                        | `Dotted` (hosts, files)                     | whole core                              |
| 19  | `,` or `;` not first and not last                                                                                                                                                                                                                                     | `List`                                      | whole core                              |
| 20  | both ASCII letters and digits (`x86`, `h264`, `2FA`, `5MB`, `abc123`)                                                                                                                                                                                                 | `Identifier`                                | whole core                              |
| 21  | letters or digits joined by single hyphens (`tls-server`, `кто-то`, `db-01`, `25-й`, `123-45-67`)                                                                                                                                                                     | `Compound`                                  | the hyphens                             |
| 22  | words or numbers (signed numbers too) joined by `—`, `–` or `…` without spaces (`word—word`, `да…нет`), or by `...` (`да...нет`, `-5...5`)                                                                                                                            | `Joined`                                    | no                                      |
| 23  | letters with their marks and digits only, with apostrophes between letters (`don't`), optionally a sign with a number (`№5`), or a word with a legal mark (`Brand(tm)`)                                                                                               | `Word`                                      | no                                      |
| 24  | anything else                                                                                                                                                                                                                                                         | `Unclassified`                              | whole core                              |

The patterns in the table describe the checks; in code every check is a function over the tokens of
the core, without regular expressions. "Not first and not last" is what "inside" means everywhere in
the table: `50%` passes check 8 (the `%` is last) and is a number by check 15; `+-5` passes (the `+`
is first) and is a signed number.

Order matters in these places:

- 0 before everything: units and abbreviations with `/` or `.` (`км/ч`, `МБ/с`, `req/s`, `т.е.`,
  `e.g.`) would otherwise be paths or dotted names, and no rule could put a no-break space before
  them.
- 7 before 8: `->` and `>=` end with `>` and would otherwise be frozen as `Angle`, and symbol rules
  could never fire.
- 10 before 16: `0x10` is hex, not dimensions.
- 12 before 17: `2026-11` is a year and month, not a range; `1941-45` (45 is not a month) is a
  range; `2010-11` is an ISO year and month and stays a date.
- 13 before 18: `0.2.27` and `10.0.0.1` are versions and addresses, not host names.
- 15 to 17 before 18: `1.5` is a number and `1.5-2.5` (en) a range, not dotted names.
- 24 last: anything the grammar does not recognize as a word or a number is frozen. The classifier
  is a white list; when it is wrong, it is wrong on the safe side.

Special cases:

- A lone fragment `x`, `х` or `×` between two number fragments (`10 x 5`) has kind `Times`, in both
  forms, so the kind survives the replacement.
- A fragment consisting only of `-`, `--` or `---` is an `OperatorRun` ("dash fragment"); dash rules
  decide its fate by context.
- A number slot inside a core counts as a number (so `{a}-{b}` reaches check 17 as a range of
  slots); other slots count as letters.
- The wrapper `"` of `27"` is cut off: the core `27` is a number, and the quote goes to the quote
  algorithm (section 6.5).

Why per fragment, not per string: a technical fragment, once found, is frozen whole, and no rule,
including a future one, can damage it. The reference implementation works with regular expressions
over the whole string; its damage to `host:8080`, `::1`, `a,b,c`, `0x1F`, `1/2/2026` happens because
a rule such as "space after a colon" does not know that the colon belongs to an address. Why not
"leave a string alone if it looks technical": technical fragments stand inside ordinary sentences
("Сервис слушает host:8080 - проверьте"), and the quotes, dashes and spaces around them are wanted.

The classifier runs once per window, before the phases. Edits inside a pass never trigger
reclassification; section 4.3 (invariant I2) states why the kinds on the output are the ones the
next pass computes.

### 3.7 Freezing and edit refusal

`FROZEN` is set by ingest (slots, protected atoms) and by the classifier (table 3.6). A rule gets no
mutable access to tokens. It returns a proposal:

```rust
pub(crate) struct Match<'s> {
    consume: u8,      // tokens from the anchor that are closed to other rules of the phase
    edits: Edits<'s>, // at most 4 edits inside [anchor, anchor + consume]
}

pub(crate) enum Edit<'s> {
    Replace { at: u8, text: &'s str },          // new text of a token; the kind stays
    Glue { at: u8, glue: Glue },                // set the glue mark of a Space token
    Insert { before: u8, glue: Glue },          // insert a Space token carrying glue
    Remove { at: u8 },                          // remove a Space token
}
```

The engine applies a proposal only if no affected token is `FROZEN` and no insertion falls inside a
frozen fragment; otherwise the whole proposal is dropped. The engine does not call rules at all when
the anchor is frozen. This check sits in one place of the engine, so protection does not depend on
the care of each rule. Edits next to a frozen fragment are allowed: the space between `в` and
`10.0.0.0/8` is not part of the fragment, so `в⍽10.0.0.0/8` is fine, while the fragment itself is
byte for byte intact.

### 3.8 Phases

```rust
type RuleFn = for<'s> fn(&Cx<'_, 's>, usize) -> Option<Match<'s>>;

fn run_phase<'s, R: Recorder>(cx: &Cx<'_, 's>, input: &[Tok<'s>], out: &mut Vec<Tok<'s>>, rec: &mut R) {
    out.clear();
    let mut i = 0;
    while i < input.len() {
        let m = if input[i].is_frozen() { None } else {
            cx.active_in_phase().iter().find_map(|&r| RULES[r as usize](cx, i).map(|m| (r, m)))
        };
        match m {
            Some((rule, m)) if cx.edits_allowed(i, &m) => { emit(out, input, i, rule, &m, rec); i += m.consume.max(1) as usize; }
            _ => { out.push(input[i]); i += 1; }
        }
    }
}
```

This is a sketch: real code has no panicking indexing. Properties of a phase:

1. **Snapshot.** All rules of a phase read the input of the phase and do not see each other's edits.
   A phase is a pure function of a token sequence.
2. **First match.** At each position the rules of the phase are tried in the fixed order of section
   4.1. The first rule that matches takes the anchor and `consume` tokens; positions inside the
   taken range are not considered again in this phase.
3. **Match by class, not by product.** A rule matches a pattern whether or not it is already in its
   target form. If it is, the rule returns a `Match` with no edits and the same `consume`. For
   example `т.⍽д.` matches `nbsp.abbreviations` like `т.д.` and `т. д.` do. So the partition into
   taken ranges is the same on a second pass (section 4.4).
4. **Bounded context.** A rule looks no further than a fixed number of tokens around its anchor (the
   longest are abbreviations and units, eight tokens). The exception is the quote phase: it has a
   linear preliminary pass over the window (section 6), after which its rules only read decisions.

`Cx` gives rules read-only access: neighbours with `Format` and `Markup` tokens skipped, the
fragment and its kind, language data, settings, line and window starts, piece numbers. Two token
vectors per window swap between phases.

### 3.9 Glue marks

No-break rules do not write U+00A0 into the text. They set the glue mark of a `Space` token
(`Glue::NoBreak` or `Glue::Narrow`) or insert a `Space` token that carries one. A U+00A0 already in
the input is lexed as a `Space` with `Glue::NoBreak`, U+202F as `Glue::Narrow`. Rendering turns the
mark into U+00A0 or U+202F, `&nbsp;` or `&#8239;`, `\u00A0` or `\u{a0}`, or into a nowrap group
(section 11.5). A U+2011 NON-BREAKING HYPHEN between two fragments binds them into the same nowrap
group as a glue mark does.

Why: nowrap output needs the groups of glued tokens, which is natural when glue is a relation
between tokens; the report shows which rule glued; and when two rules want the same space, the
result does not depend on their order (`NoBreak` wins over `Narrow`). A space is glued only if it is
a single U+0020 (one `Space` token of one character); tabs, runs of spaces and line breaks are never
glued.

### 3.10 Rendering and Cow

Rendering walks the tokens and writes each one's text (or glue) through the encoder of the output
kind (section 11). The output string is created lazily: while no window has changed and encoding
changes nothing, nothing is built. At the first change the output gets
`String::with_capacity(len + len / 8)`, the unchanged prefix of the input is copied in, and windows
are written from there. If nothing changed to the end, the result is `Cow::Borrowed(input)`. For
templates the same holds per text: a text that maps to one unchanged input piece borrows it.

### 3.11 Recording

```rust
pub(crate) trait Recorder {
    fn change(&mut self, rule: Rule, at: SrcSpan, replacement: &str);
    fn protected(&mut self, at: SrcSpan, kind: ProtectedKind);
    fn diagnostic(&mut self, kind: DiagnosticKind, rule: Option<Rule>, at: SrcSpan);
}
pub(crate) struct NoRecord;     // every method empty and #[inline(always)]
pub(crate) struct Diagnostics;  // diagnostics only (lint)
pub(crate) struct Journal;      // everything (report)
```

The engine is generic over `R: Recorder` and monomorphized: `typeset` uses `NoRecord`, so the plain
path contains no recording code; `lint` uses `Diagnostics`; `report` uses `Journal`. Rules do not
depend on the recorder; a test compares the three modes on the corpus.

### 3.12 Complexity and pathological input

- Time: lexer O(n); classifier O(n); seven phases, each O(n) with bounded context; quote matching
  with a stack, one push and one pop per quote; lookups in sorted static lists by binary search,
  O(log k) per fragment with k the longest list (hundreds); rendering O(n). Total O(n log k). There
  is no recursion and no backtracking.
- Protection delimiters: if the closing delimiter of a pair is not found from position p, it is not
  found further on either, so the search for that pair stops. A hundred thousand unclosed backticks
  are processed in linear time.
- Memory: the token buffers of the longest window (about 40 bytes per token), the output string, and
  the journal only when recording. For text with paragraphs, memory does not grow with the input. A
  10 MB paragraph without line breaks is the pathological case: memory is linear in its length.
- Inputs exercised by the load tests (section 13.8): long runs of `"`, `'`, backticks, `-`, `.`,
  `(`; alternating Cyrillic and Latin letters; a string of ZWJ emoji; one 1 MB line without spaces
  (one fragment: the classifier is linear, freezing is one pass); quote nesting 10 000 deep.

### 3.13 Rejected engine designs

| Design                                         | Why rejected                                                                                                                                                                 |
| ---------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Regular expressions over the whole string      | No tokens, so no notion of a technical fragment; overlapping matches break idempotence (`a,b,c` → `a, b,c` → `a, b, c` in the reference implementation); a heavy dependency. |
| Editing one `String` in place, rule after rule | Shifting offsets, O(n) per insertion, edits cannot be related to the input for reports, protection would rest on every rule.                                                 |
| One fused automaton for all rules              | Hidden interactions; rules could not be switched one by one; order invisible; edits not attributable to rules.                                                               |
| A rope or a piece table                        | Two token buffers per window suffice; insertions are few and are only spaces.                                                                                                |
| Iterating to a fixed point to get idempotence  | Masks rule bugs, costs two to three times more, convergence is not guaranteed. Used only as a test oracle (section 13.5).                                                    |
| Rules as `dyn Trait` registered from outside   | Order with foreign rules cannot be fixed, and the rule API is too young to stabilize.                                                                                        |
| Sentence or syntax parsing                     | Out of scope; local contexts suffice for every rule of version 1.                                                                                                            |
