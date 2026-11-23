# typolish design: Processing model

Part of the [typolish design](./README.md). Section numbers are those of the whole design.

## 3. Processing model

The core is a pass over characters with context; it uses no regular expressions. Source files named
here are planned under `dev/typolish/src/` (section 14.6).

### 3.1 Pipeline

```text
input (&str | pieces)
  │  0. prefilter (typeset* only): no byte that an active rule or the
  │     output kind reacts to → Cow::Borrowed                              (3.2)
  │  1. ingest: pieces → stream of atoms (Text, Slot, Protected, Markup);
  │     fenced backtick blocks cut out as Delimited atoms                  (3.4)
  │  2. windows: split by paragraph; a fenced block is never split         (3.3)  ┐
  │  3. delimiters: other protection delimiters cut Delimited atoms out    (3.4)  │
  │     of Text, matched across pieces within the window                          │
  │  4. lexer: atoms → class-invariant tokens over grapheme clusters      (3.5)  │ per window
  │  5. classifier: fragments → kind, FROZEN and LABEL flags               (3.6)  │
  │  6. phases 1..7: one pass each over a snapshot, rules in table order   (3.8)  │
  │  7. render: tokens → output in the requested kind                      (3.10) ┘
  ▼
Cow<str> | Template (+ diagnostics, + journal of edits and protected spans when recording)
```

Files: `engine/mod.rs` (the pipeline entry points, section 13.5, property 8), `engine/ingest.rs`,
`engine/window.rs`, `engine/lex.rs`, `engine/classify.rs`, `engine/phase.rs`, `engine/render.rs`,
`engine/record.rs`, `engine/scratch.rs`; rules in `rules/mod.rs` (the table of rule functions) and
`rules/{symbols,ellipsis,dash,range,number,quotes,nbsp}.rs`.

### 3.2 Prefilter

When a typographer is built, the 256-bit trigger set is the union of the trigger bytes of its active
rules; each rule declares its trigger bytes as a constant next to its code (`rules/*.rs`).
`TypographerBuilder::build` computes the set and the active rules once; `Typographer::new` is a
`const fn` and cannot, so a typographer it made computes them on its first call and keeps them (a
`OnceLock` in the typographer, which allocates nothing). In version 1 the set is: `"` `'` `-` `.`
`(` `<` `>` `!` `+` `x` `X`, the lead bytes of the Cyrillic `х` and `Х` when `number.times` is
active (`{w} х {h}` between two `Number` slots has no other byte), the digits, the lead bytes of
every sign of the language's `signs` (`№`, `§`, `¶` in the built-in tables), the lead bytes of
typographic characters that rules use as context, `%` and the lead byte of `‰` when `nbsp.units` is
active and `PercentSpacing` is not `AsWritten`, and the space when a no-break rule is active. A call
first scans the input bytes against that set and against the set of bytes that the output kind
encodes. The second set is derived, once per output style, from the same constant table the encoder
uses (section 11), so the two cannot drift apart:

- `Unicode`: empty;
- `Html`: `& < > " '`, and the lead bytes of the typographic set except with
  `HtmlStyle::CHARACTERS`; with `CHARACTERS` and a nowrap style, the lead bytes of U+00A0, U+202F
  and U+2011 (0xC2, 0xE2), which a nowrap element changes or makes joints;
- `Escaped`: `\`, `"`, controls (0x00 to 0x1F, 0x7F), the lead bytes of U+2028, U+2029 and of the
  typographic set; in addition `'`, the backtick, `< > &` and `$` for `JS`, `JS_HEX` and
  `JS_CODE_POINT`, `< > &` for `JSON`, and `'` and `<` for `CSS`; with `non_ascii()`, every byte
  from 0x80. C1 controls such as U+0085 never skip the scan: their lead byte 0xC2 is already in the
  set through U+00A0.

If no byte matches, the call returns `Cow::Borrowed` without lexing. Lead bytes over-approximate (a
Cyrillic letter matches only while `number.times` is active, while every character from U+2000 to
U+2FFF does): a false match costs a full run, never a wrong result (property 8 of section 13.5
compares the prefilter with a run without it). So
`typeset("a</script>b", Output::Escaped(EscapeStyle::JS))` is never borrowed, even in Interface,
where no rule reacts to `<`. For short interface strings without signs this skips the whole
pipeline; the scan is linear and allocates nothing.

For `typeset_template` the trigger set of the rules is matched against the bytes of all `Text`
pieces together; a slot contributes no byte, so every rule that can edit next to a slot needs its
trigger byte in the text beside the slot (`{n}%` is caught by `%`, `№{n}` by `№`, `{n} МБ` by the
space). The set of bytes that the output kind encodes is matched against the `Text` and `Protected`
pieces and, for `Output::Escaped` only, the `Markup` pieces too, because those are the pieces the
encoder writes (section 11). If no byte matches either set, the call returns every text borrowed
without lexing.

The prefilter belongs to `typeset`, `typeset_with`, `typeset_into`, `typeset_template` and
`typeset_template_with` only. `report`, `report_template`, `lint` and `lint_template` always run the
whole pipeline: the diagnostics of the classifier and of protection (`MixedScript`, `SlotGlue`,
`UnclosedDelimiter`) and the records of `protected()` do not come from a rule and need no trigger
byte (`Мoсква`, `{a}{b}`, `{n}МБ`, ``don`t`` and `` `code` `` contain none). The output of `report`
and `report_template` still equals that of `typeset` and `typeset_template`, and the diagnostics of
`lint` and `lint_template` (without `TypographicChar`) equal those of `report` and `report_template`
(property 9 of section 13.5).

### 3.3 Windows

A window is a paragraph: its boundary is two or more line breaks with only horizontal whitespace
(section 3.5) between them, or U+2029 PARAGRAPH SEPARATOR. No rule looks across a window boundary,
and the scope of quote pairing is the window (section 6.10). For pieces, boundaries are found the
same way in the text of the pieces; a slot is never a boundary: for the condition "only horizontal
whitespace between them", every atom other than `Text` (a slot, a protected span or piece, a markup
piece) is content, so `Text("a\n")`, `Slot`, `Text("\nb")` is one window, and a message without
blank lines is one window. A fenced backtick block (section 10.1), which ingest cuts out before
windows are found, lies inside one window even when it contains blank lines.

The characters of a boundary (the line breaks, a U+2029 and the horizontal whitespace between them)
belong to no window: they are not tokenized, no rule reads or changes them, and the renderer writes
them between the windows through the encoder of the output kind. Horizontal whitespace before the
first or after the last line break of a boundary belongs to the window on that side. Windows are
found before the inline protection delimiters (section 3.4), so a U+2029 can be inside an atom only
in a fenced block or in a protected or markup piece; there it is part of the atom's text.

Why: a long text does not hold all its tokens in memory at once; an error in one paragraph (an
unpaired quote) does not affect its neighbours; and the window is the natural unit for "all or
nothing" in quote pairing.

### 3.4 Ingest

The text of all `Text` pieces of a call is read as one string, the **call's text**: for string input
it is the input itself (one `Text` piece with index 0); for pieces it is the concatenation of their
`Text` pieces, copied into a buffer of the scratch (section 2.12) unless at most one of them is
non-empty. Input becomes a stream of atoms over it:

```rust
enum Atom {
    Text { start: u32, end: u32 },                 // a range of the call's text: adjacent Text pieces
    Delimited { start: u32, end: u32 },            // cut out by delimiters, the delimiters included
    Slot { piece: u32, at: u32, kind: SlotKind },  // at: its position in the call's text
    Protected { piece: u32, at: u32 },             // a Piece::Protected
    Markup { piece: u32, at: u32 },
    Boundary { start: u32, end: u32 },             // the characters of a window boundary (3.3)
}
```

An atom that is not text has no extent in the call's text: it stands at a position between two of
its bytes and keeps its piece. An empty `Protected` or `Markup` piece is an atom like a non-empty
one (it ends a token and is content for windows); only an empty `Text` piece adds nothing. A table
of the `Text` pieces with their ranges in the call's text resolves a position to a piece when a
record is made (section 3.11) and when output is written. Atoms, like tokens, hold positions and no
references, so the buffers that keep them can be reused across calls (section 2.12). Positions are
`u32`: an input with more than `u32::MAX` pieces or a call's text longer than `u32::MAX` bytes is
returned unchanged, the safe outcome of section 2.11 for input the engine cannot represent. For
templates, the pieces are first collected into the call-local vector (section 2.12), and only then
is the scratch borrowed.

Protection delimiters (section 10.1) cut `Text` atoms into `Text` and `Delimited` in two steps; the
delimiters belong to the `Delimited` atoms:

1. Over the whole input, before windows are found, fenced backtick blocks (as section 10.1 defines
   them; 10.1 is normative) are cut out. They may contain blank lines.
2. Within each window, the other delimiters (single and double backtick runs, longer runs that do
   not open a fenced block, own pairs) are matched over the text of the window across pieces. At a
   position, an own delimiter is tried before a backtick run (the validation of section 10.1 lets at
   most one own delimiter match there). Adjacent `Text` pieces are read as one text, so a delimiter
   may even be split between them; slots, protected pieces and markup between an opening and a
   closing delimiter end up inside the span. Inside it, `Text` atoms become `Delimited`, while a
   slot stays a slot and markup stays markup; so does a protected piece. A fenced block is cut the
   same way.

So the Fluent message ``Run `git checkout -- { $branch }` to reset.`` protects the whole command
with the slot inside, and a template of `Text` pieces behaves exactly as their concatenation
(property 10 of section 13.5).

A slot, a protected piece or a `Delimited` atom becomes one opaque token (`Slot` or `Protected`): it
is always `FROZEN`, takes part in rule context through its class (section 3.5), and is never inside
the range of an edit. A markup atom becomes a zero-width token that is transparent for context:
rules look through it for neighbours, and edits never touch it.

Lexing reads a run of adjacent `Text` atoms with no other atom between them as one text, so a token
(a run of dots, dashes or spaces, a word, a number) may span a piece boundary; property 10 of
section 13.5 depends on it. A slot, a protected atom or a markup atom ends a token: the pieces
`Text("Ждём.")`, `Markup("<b>")`, `Text("..")` give two `Dots` tokens (`.` and `..`) and no
ellipsis, because an edit may not include markup.

### 3.5 Lexer: class-invariant tokens

Text is cut into tokens along extended grapheme clusters (`unicode-segmentation`, section 5.1). A
fast path skips the tables for ASCII. No ASCII character is `Prepend`, `Extend`, `ZWJ` or
`SpacingMark`, so a position between two ASCII bytes is a cluster boundary (except inside `\r\n`),
and so are the start and the end of the text. The lexer cuts the text at these positions: a piece of
one ASCII byte is a cluster of its own (`\r\n` is one cluster), and every other piece, which holds
non-ASCII characters together with the ASCII bytes directly before and after them, goes to the
segmenter whole. So the ASCII byte before a non-ASCII character is segmented with it, because a
combining mark, ZWJ or variation selector may extend it (`"` + U+0301, `1` + U+FE0F + U+20E3), and
the ASCII byte after it is segmented with it too, because a `Prepend` character glues the next
character to itself (U+0600 + `1`, rule GB9b of UAX #29). The gain is for Latin text; in Cyrillic
text most ASCII bytes stand next to non-ASCII characters and go to the segmenter. A token's class
comes from the first character of its first cluster, except that a cluster of several characters
whose first character is not a letter (a digit of another system counts as a letter, section 5.2) is
a `Symbol` (keycaps `1️⃣` and `#️⃣`, `"` + U+0301, `"` + U+200C, `1` + U+200D), with three exceptions:
a cluster of ZWNJ and ZWJ only is a `Format` token (section 5.4); `\r\n` is a `Newline`; and a
cluster that starts with a whitespace character and carries a combining mark, ZWNJ or ZWJ is a
frozen `Space` (below).

"Horizontal whitespace" is every character with `char::is_whitespace` that is not a line break
(`\n`, `\r`, U+0085, U+2028) and not U+2029: U+0020, U+00A0, U+202F, tab, U+000B, U+000C, U+2000 to
U+200A, U+205F, U+3000 and the rest (section 5.3).

Clusters join into tokens as follows: clusters led by a letter join into one `Word`, ASCII digits
into one `Number`, `-` into one `Dash`, `.` into one `Dots`, horizontal whitespace into one `Space`
(its `glue` is `NoBreak` or `Narrow` only when the run is exactly one U+00A0 or one U+202F; every
other run has `Glue::None` and is written as its source text, section 3.9). The multi-character
plain forms of the fixed table (operators, legal marks, section "Class invariance") are one token
each. Every other cluster, each line break included (`\r\n` is one), is a token of its own.

```rust
pub(crate) struct Tok {
    kind: Kind,
    src: SrcSpan,   // a byte range of the call's text (3.4): a token may span adjacent Text pieces
    piece: u32,     // the piece of a token made from a slot, a protected piece or markup; none for text
    repl: Repl,     // Source (the text is the slice at src) | Char(char) | Ellipsis(EllipsisForm)
    glue: Glue,     // for Space tokens: None | NoBreak | Narrow (section 3.9)
    flags: Flags,   // FROZEN, LABEL, CHANGED, INSERTED, CORE (a token of the core of its fragment)
    frag: u32,      // its fragment in the classification of the pass (3.6); none for Space and Newline
}

pub(crate) enum Kind {
    Word(Script),     // letters with their combining marks; Script: Cyrillic | Latin | Greek | Other | Mixed
    Number,           // a run of ASCII digits
    Space,            // a run of horizontal whitespace: U+0020, U+00A0, U+202F, U+2009, tab, ...
    Newline,          // \n, \r\n, \r, U+0085, U+2028; U+2029 always ends a window and is never a token
    Format,           // U+00AD, U+200B, U+200E, U+200F, U+202A..U+202E, U+2060..U+2064, U+2066..U+2069, U+FEFF; U+200C and U+200D only in a cluster of ZWNJ and ZWJ only (section 5.4)
    Quote,            // " ' « » „ “ ” ‘ ’ ‚ ‹ › ′ ″
    Dash,             // a run of '-' (length 1..), U+2010 ‐, U+2011 ‑, U+2012 ‒, – — U+2015 ―, −
    Dots,             // a run of '.' (length 1..), …
    Operator(Op),     // -> <- <-> >= <= != +- => >> << == and → ← ↔ ≥ ≤ ≠ ±
    LegalMark(Mark),  // exactly (c) (C) (с) (С) (r) (R) (tm) (TM), and © ® ™; a mixed case such as (Tm) is not a LegalMark
    Times,            // x X х Х between numbers (assigned by the classifier: special case and check 16 of section 3.6), and × everywhere
    Open(char),       // ( [ {
    Close(char),      // ) ] }
    Punct(char),      // , ; : ! ? № § ¶ % ‰ ° / \ @ # & * + = < > | ~ ^ $ _ `
    Symbol,           // other clusters: emoji, CJK punctuation, any multi-character cluster led by a non-letter
    Slot(SlotKind),   // opaque placeholder
    Protected,        // protected text
    Markup,           // zero-width, transparent
}
```

`SrcSpan` is `{ start: u32, end: u32 }`, a range of the call's text; the table of `Text` pieces
(section 3.4) gives the pieces it covers. A token made from a slot, a protected piece or markup has
an empty range at its position and names its piece in `piece`. `Protected` covers both a `Delimited`
atom (a range, no piece) and a protected piece (a piece, an empty range). The quote glyphs a
language may use (section 7.6) are exactly the `Quote` characters other than `"`, `'`, `′` and `″`.

A token holds no reference. Its current text is the source slice at `src`, unless a rule replaced
it; `repl` then names the replacement. Every replacement of version 1 is one character (a quote
glyph, a dash, `…`, `−`, `×`, `′`, `″`, `©`, `®`, `™`, an arrow, `≥`, `≤`, `≠`, `±`) or a form of
the language's ellipsis data (the `..` or `…` after `?` and `!`), which the renderer reads from the
typographer's language. An inserted `Space` token has an empty `src` at the insertion point. So
`Tok` is `'static`: the vectors that hold tokens live in `Scratch` and in a thread-local without
`unsafe`, and are reused for inputs of any lifetime (section 2.12). `Cx` resolves the text of a
token for rules from the call's inputs and the language.

**Class invariance.** A plain form and all its typographic forms belong to one kind: `"` and `«` are
`Quote`; `-`, `--` and `—` are `Dash`; `...` and `…` are `Dots`; `->` and `→` are `Operator`; `(c)`
and `©` are `LegalMark`; ` ` and `⍽` are `Space`. Kinds carry no field describing the form; the form
is read from the token's text. `Op` and `Mark` are semantic ids (arrow right, copyright), the same
for both forms. Multi-character plain forms are lexed greedily from a fixed table into one token, so
replacing `(c)` with `©` changes the text of one token rather than turning three tokens into one.
Operators are lexed only whole from the table: the lexer takes the maximal run of the characters `-`
`<` `>` `=` `!` `+` (each a cluster of its own) and makes it one `Operator` token only if the whole
run is an entry; otherwise it lexes the run character by character. `-->` is not in the table, so
the lexer sees `Dash` of length 2 and `Punct('>')`, and no dash rule fires (there is no space to the
right of `--`); `<-->` is `Punct('<')`, `Dash` of length 2 and `Punct('>')`.

Every rule edit changes only the replacement or the `glue` of a token, never its kind. The only
structural edits are inserting a `Space` token and removing a `Space` token. Section 4 builds the
idempotence argument on this.

A cluster of several characters that starts with a non-letter (`"` + U+0301, `1️⃣`, `#️⃣`) is a
`Symbol` (except a cluster of ZWNJ and ZWJ only, section 5.4) and no rule changes it. A cluster that
starts with a whitespace character and carries a combining mark, ZWNJ or ZWJ is a `Space` token
flagged `FROZEN`: it separates fragments but is never glued or removed, and the mark stays where it
is (section 5.3). Emoji with ZWJ, modifiers, flags and keycaps are single `Symbol` tokens and are
never cut or replaced.

### 3.6 Fragments and the classifier

A **fragment** is a maximal run of tokens of one window without `Space` and `Newline` tokens
(no-break spaces are `Space` too, since `char::is_whitespace` holds for U+00A0, U+202F and U+2009).
Opaque tokens belong to fragments like text: `{host}:{port}` is one fragment.

Before classification the **wrapper** is cut off by tokens: at the start `(`, every `Quote` token
except `′` and `″`, and runs of dots; at the end `)`, every `Quote` token, `. , ; : ! ? …` and runs
of dots. `[ ]`, `{ }` and `< >` are not wrapper: they belong to technical notation (`[::1]:8080`,
`{x}`, `<T>`). The wrapper stays editable (it is sentence punctuation). The rest, the **core**, gets
one kind. The **stop** of a core is the token right after it, when that token is a `Dots` token in
any form (`.`, `..`, `...`, `…`). Checks 0 and 14 read a core that has a stop as if a single full
stop followed it. A `LegalMark` token is one token, so `(c)` keeps its parentheses, and its core is
not frozen (special cases below).

If the start wrapper and the end wrapper together take every token of a fragment other than `Format`
and `Markup` tokens (`...`, `…`, `"`, `«»`, `()`, `?!`, `,`, `"?"`, `"\u{2068}\u{2069}"`), the
fragment has no core and its kind is `Punctuation`. No check of the table runs on it, nothing in it
is frozen, it is neither technical nor an operand (section 8.2), and its tokens stay editable like
any wrapper. A fragment with any other token keeps that token in its core: `"-"` has the core `-` (a
dash fragment, special cases below), and `«{x}»` has the core `{x}`.

The classifier scans the core left to right at most three times (constant work per character) and
applies the checks strictly in this order; the first check that matches decides. The Kind column
lists the values of the fragment kind, an engine-private enum `FragKind` with one variant per name
in that column, plus `Times`, `Sign` and `Punctuation` (special cases below). It is a different type
from the token `Kind` of section 3.5, although some names coincide (`Number`, `Word`, `Operator`,
`Times`); "kind" of a fragment or a core below always means `FragKind`:

| No. | Check on the core                                                                                                                                                                                                                                                                                                                                                             | Kind                                        | Frozen                                  |
| --- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------- | --------------------------------------- |
| 0   | equals an entry of the language's dictionaries: units (also user-added), abbreviation parts, elisions, month forms, atom exceptions (`e.g.`, `i.e.`, `etc.`, `vs.`, `cf.`, `a.m.`, `p.m.`), each compared as section 7.1 says; an entry ending in `.` matches a core that has a stop (`т.е.`, `т.е...`, `т.е…`)                                                               | `Dictionary`                                | no                                      |
| 1   | contains `://`, or starts with `www.`                                                                                                                                                                                                                                                                                                                                         | `Url`                                       | whole core                              |
| 2   | `@` not first and not last; or `@` followed by a letter or digit at the start                                                                                                                                                                                                                                                                                                 | `Email`                                     | whole core                              |
| 3   | starts with `-` and an ASCII letter, or with `--` and an ASCII letter or digit                                                                                                                                                                                                                                                                                                | `Flag`                                      | whole core                              |
| 4   | contains `/` or `\`                                                                                                                                                                                                                                                                                                                                                           | `Path`                                      | whole core                              |
| 5   | `HH:MM[:SS]` `-` or `–` `HH:MM[:SS]` (hours 0 to 23, minutes 00 to 59)                                                                                                                                                                                                                                                                                                        | `TimeRange`                                 | the times; the separator stays editable |
| 6   | `:` not first and not last, or the core starts with `::`                                                                                                                                                                                                                                                                                                                      | `Colon` (host:port, IPv6, MAC, time)        | whole core                              |
| 7   | only operator characters `- < > = ! + ~ \| & * ^ %` (`->`, `>=`, `<->`, `=>`, `>>`, `--`)                                                                                                                                                                                                                                                                                     | `OperatorRun`                               | no (symbol and dash rules decide)       |
| 8   | any of `= < > \| & ^ ~ * # $ % + _` or a backtick not first and not last; or the core starts with `<` or ends with `>`                                                                                                                                                                                                                                                        | `Operator`, `Identifier` (for `_`), `Angle` | whole core                              |
| 9   | UUID 8-4-4-4-12 of hex digits                                                                                                                                                                                                                                                                                                                                                 | `Uuid`                                      | whole core                              |
| 10  | `0x` or `0X` and hex digits                                                                                                                                                                                                                                                                                                                                                   | `Hex`                                       | whole core                              |
| 11  | at least 8 characters, all hex, with a digit and a letter a to f                                                                                                                                                                                                                                                                                                              | `Hex`                                       | whole core                              |
| 12  | `YYYY-MM-DD`; or `YYYY-MM` or `YYYY–MM` with month 01 to 12                                                                                                                                                                                                                                                                                                                   | `IsoDate`; `YearMonth` for the short form   | whole core; no for `YearMonth`          |
| 13  | `[vV]?` number (`.` number){2,} with an optional `-` or `+` tail; four numbers 0 to 255 joined by `.`; `D.M.YYYY`                                                                                                                                                                                                                                                             | `Version`                                   | whole core                              |
| 14  | two or more groups of one or two letters joined by `.`, the core having a stop (`т.е.`, `А.С.`, `Ph.D.`)                                                                                                                                                                                                                                                                      | `Abbreviation`                              | no                                      |
| 15  | a number: optional sign (`-`, `+`, `+-`, `−`, `±`, or the minus sign of the settings, section 2.11), digits, optional decimal part with `.` or `,`, optional `%` or `‰`                                                                                                                                                                                                       | `Number`                                    | no                                      |
| 16  | `D+` then `x`, `X`, `х`, `Х` or `×` then `D+`, the first factor without a leading zero                                                                                                                                                                                                                                                                                        | `Dimensions` (the letter becomes `Times`)   | no                                      |
| 17  | `N-N` or `N–N`, each N an unsigned integer, a decimal with the language's separator, a Roman numeral or a `Number` slot, in any mix (`5-10`, `{a}-5`, `{a}-{b}`); or two `Date` slots joined by `-` or `–`                                                                                                                                                                    | `Range`                                     | no (only range rules edit its dash)     |
| 18  | a single `.` between letters or digits (runs of two or more dots do not count)                                                                                                                                                                                                                                                                                                | `Dotted` (hosts, files)                     | whole core                              |
| 19  | `,` or `;` not first and not last                                                                                                                                                                                                                                                                                                                                             | `List`                                      | whole core                              |
| 20  | only ASCII letters and digits, both present (`x86`, `h264`, `2FA`, `5MB`, `abc123`)                                                                                                                                                                                                                                                                                           | `Identifier`                                | whole core                              |
| 21  | letters or digits joined by single hyphens (`tls-server`, `кто-то`, `db-01`, `25-й`, `123-45-67`)                                                                                                                                                                                                                                                                             | `Compound`                                  | the hyphens                             |
| 22  | two or more parts joined by single `—`, `–` or `…` without spaces, or by `...` (`word—word`, `да…нет`, `да...нет`, `-5...5`), each part a word by check 23 or a number by check 15 (signed numbers too)                                                                                                                                                                       | `Joined`                                    | no                                      |
| 23  | letters with their marks and digits only, with a quote `'` or `’` between two letters (`don't`) or between a digit and a letter (`90's`), or a quote `'` or `′` between a number of one to three digits and a final number of one or two digits (`5'10`, `5′10`), optionally a sign with a number or a Roman numeral (`№5`, `¶IV`), or a word with a legal mark (`Brand(tm)`) | `Word`                                      | no                                      |
| 24  | anything else                                                                                                                                                                                                                                                                                                                                                                 | `Unclassified`                              | whole core                              |

A **technical fragment** is one whose core the table freezes, whole or in part (the Frozen column is
not "no"), other than a `Compound` core. A `Compound` core is not technical: only its hyphens are
frozen, and for the rules it is a compound word, a word whose script is that of its letters (section
8.2), so `кто-то`, `tls-server` and `mother-in-law` are operands like any word.
`Report::protected()` still lists its hyphens, as `CompoundHyphens` (section 12.5).

The patterns in the table describe the checks; in code every check is a function over the tokens of
the core, without regular expressions. "Not first and not last" is what "inside" means everywhere in
the table: `50%` does not match check 8 (the `%` is last) and is a number by check 15; `+-5` does
not match it either (the `+` is first) and is a signed number.

Order matters in these places:

- 0 before everything: units and abbreviations with `/` or `.` (`км/ч`, `МБ/с`, `req/s`, `т.е.`,
  `e.g.`) would otherwise be paths or dotted names, and no rule could put a no-break space before
  them.
- 7 before 8: `->` and `>=` end with `>` and would otherwise be frozen as `Angle`, and symbol rules
  could never fire.
- 10 before 16: `0x10` is hex, not dimensions.
- 12 before 17: `2026-11` is a year and month (`YearMonth`), not a `Range`, so `range.digits` never
  takes it, and only `range.context`, with a year or century word next to it and a tail greater than
  the last two digits of the year, turns its hyphen into a range dash (`2010-11 гг.` →
  `2010–11⍽гг.`; `2026-11`, `в 2010-11` and `2026-11 гг.` keep the hyphen). `1941-45` (45 is not a
  month) is a range. A full `YYYY-MM-DD` is frozen.
- 13 before 18: `0.2.27` and `10.0.0.1` are versions and addresses, not host names.
- 15 to 17 before 18: `1.5` is a number and `1.5-2.5` (en) a range, not dotted names.
- 20 before 23: a quote makes check 20 fail, so `90's` and `5'10` are words by check 23 and step 1
  of section 6.2 can edit them, while `1'000'000` stays frozen.
- 24 last: anything the grammar does not recognize as a word or a number is frozen. The classifier
  is a white list; when it is wrong, it is wrong on the safe side.

How some checks read a core, where the table is short:

- Check 0: a multi-part abbreviation is matched written together, as its consecutive parts that all
  end in a full stop (`т.е.` for `т.` `е.`, `т.ч.` for `в` `т.` `ч.`); an elision, which starts with
  an apostrophe that the start wrapper cuts off, is matched together with that apostrophe.
- Check 5: hours have one or two digits, minutes and seconds two.
- Check 7: operator tokens count in both forms (`→` as `->`), so a replacement keeps the kind.
- Check 8: the kind is `Operator` when an operator character other than `_` stands inside,
  `Identifier` when only `_` does, and `Angle` otherwise (the core starts with `<` or ends with
  `>`); `x->` is `Angle`, `vec<int>` `Operator`.
- Check 11: the letters `a` to `f` count in either case.

Special cases. The first four are decided before the table and win over every check, check 0
included (the first on the whole fragment, the other three on the core); the others change how the
checks read a core:

- A lone fragment `x`, `X`, `х`, `Х` or `×` between two number fragments (`10 x 5`: the fragments
  directly before and after it in the window have kind `Number`) has kind `Times`, in both forms, so
  the kind survives the replacement. On the left a sign of the language with a number attached
  (`№10 x 5`) counts as a number fragment too: it is the label number that `nbsp.signs` splits off
  (`№⍽10 x 5`), and a label counts as a number for `number.times` (section 8.2), so the split
  changes nothing for the rule (invariant I2). A lone `%` or `‰` after spaces on the left is read
  with the number before the spaces (`10 % x 5` and `10⍽% x 5` read as `10% x 5`, section 8.2), so
  the space that the percent setting inserts or removes changes nothing either. The lexer gives `×`
  the token kind `Times` in every position; a lone `×` fragment that does not stand between two
  number fragments falls through the table to check 24 (`Unclassified`, frozen).
- A core that is a single `Dash` token in any form (`-`, `--`, `---`, `–`, `—`) is an `OperatorRun`
  ("dash fragment"; its wrapper stays editable, so `"-"` is one too), so a lone `-` and the `—` that
  replaces it have one kind; dash rules decide its fate by context. A typed `—` or `–` is matched by
  `dash.spaced-hyphen` with an empty edit, and a run `--` or `---` only by `dash.double-hyphen`
  (section 8.4).
- A core that is only a sign of the language's `signs`, single or doubled (`№`, `§§`, `¶`), has kind
  `Sign`: it is not frozen and not technical, and no check of the table runs on it; `nbsp.signs`,
  `nbsp.short-words` and the `LABEL` rule below read it.
- A core that is a single `LegalMark` token in either form, alone or with one `Number` token
  attached after it (`(c)`, `©`, `(r)`, `(tm)`, `(c)2026`), has kind `Word`: it is not frozen and
  not technical, and no check of the table runs on it. Whether `(c)`, `(r)` or `(tm)` changes is
  decided by `symbols.copyright` from the context (section 8.9: `вариант (c)` stays). A legal mark
  attached to a word is a `Word` by check 23 (`Brand(tm)`, `f(c)`); one attached after a number or
  anything else (`2026(c)`, `1001(c)`, `2020-2026(c)`) falls through to check 24.
- A `Number` slot inside a core counts as a number. A `Date` slot counts as a number only in check
  17 and only when both sides are `Date` slots (`{d1}-{d2}` is a `Range`); in every other position
  it counts as letters, so `{n}-{d}` and `5-{d}` are `Compound`. A `Number` slot and a literal
  number (`{a}-5`, `5-{b}`) form a `Range` whose dash no rule edits: `range.placeholders` needs two
  slots, `range.digits` and `range.context` need two literals; the fragment still counts as a range
  for `nbsp.units` and `nbsp.dates`.
- The wrapper `"` of `27"` is cut off: the core `27` is a number, and the quote goes to the quote
  algorithm (section 6.5).
- `Format` tokens (section 5.4) inside a fragment, and ZWNJ and ZWJ inside the text of a token, are
  skipped by every check and by list lookups: `ноя\u{AD}бря` is the word `ноября` for check 23 and
  for the month list, and `сло\u{200D}во` is the word `слово`. They stay in the text.

**Roman numerals.** A Roman numeral is a `Word` token of Latin capital letters in the standard form
`M{0,3}(CM|CD|D?C{0,3})(XC|XL|L?X{0,3})(IX|IV|V?I{0,3})`, not empty; its value is 1 to 3999
(`Format` tokens and ZWNJ/ZWJ are skipped as everywhere in this section). `XIX`, `IV`, `MCMXCIV` and
a single `I`, `V` or `X` are Roman numerals; `IIII`, `IC`, `VV`, `XM`, lowercase `xix`, mixed-case
`Mix` and words with Cyrillic look-alike letters are not (`ХIХ` with Cyrillic `Х` is a `Mixed` word
and raises `MixedScript`, section 5.7). A Roman numeral counts only where a check or a rule names it
(checks 17 and 23, the `LABEL` flag below, `range.context`, `nbsp.dates`, `nbsp.signs`), so `I` or
`C` elsewhere is an ordinary word. `range.context` compares two Roman numerals by value (`XIX-XX`:
19 < 20); a Roman numeral and an Arabic number are not compared.

**Label numbers.** A sign of the language's `signs` (`№`, `§`, `¶`, also doubled) followed by a
number names something; it counts nothing. After the checks, the classifier sets the `LABEL` flag
on:

- a `Number` or `Range` core, a core that is a Roman numeral, or a `Number` slot, whose fragment
  follows a `Sign` fragment (a sign or a doubled sign, special cases above), with nothing but spaces
  of any kind between them on the same line (`№ 5`, `№⍽5`, `§§ 5–7`, `№ {n}`, `¶ IV`); `Format` and
  `Markup` tokens are transparent here as everywhere (section 5.4), so a fragment made of them only,
  such as an empty Fluent value between the sign and the number, does not separate them;
- the core of a sign attached to a number or a Roman numeral (`№5`, `§12`, `¶IV`, `№{n}`; kind
  `Word` by check 23).

Both forms are labels, so the space that `nbsp.signs` inserts into `№5` or `¶IV` leaves the number a
label (invariant I2 of section 4.3). The rules that read a quantity or a year do not take a label
for a number (section 8.2): in `Школа №5 г. Москвы`, `г.` is a city, not a year. Marking labels
looks at one neighbouring fragment (past fragments of transparent tokens), so it stays linear.

Why per fragment, not per string: a technical fragment, once found, is frozen whole, and no rule,
including a future one, can damage it. The reference implementation works with regular expressions
over the whole string; its damage to `host:8080`, `::1`, `a,b,c`, `0x1F`, `1/2/2026` happens because
a rule such as "space after a colon" does not know that the colon belongs to an address. Why not
"leave a string alone if it looks technical": technical fragments stand inside ordinary sentences
("Сервис слушает host:8080 - проверьте"), and the quotes, dashes and spaces around them are wanted.

The classifier runs once per window, before the phases. Edits inside a pass never trigger
reclassification: the fragments, kinds and flags it computed hold for every phase of the pass, also
where an earlier phase removed the space between two fragments (in `Moscow - the capital` the
no-break phase still reads `the` as a whole core after the dash phase made `Moscow—the`). Section
4.3 (invariant I2) states why the kinds and flags on the output are the ones the next pass computes.

### 3.7 Freezing and edit refusal

`FROZEN` is set by ingest (slots, protected atoms) and by the classifier (table 3.6). A rule gets no
mutable access to tokens. It returns a proposal:

```rust
pub(crate) struct Match {
    consume: u8,      // the tokens [anchor, anchor + consume] are closed to other rules of the phase
    edits: Edits,     // at most 6 edits inside [anchor, anchor + consume]; nbsp.phone glues up to five spaces
    note: Option<u8>, // a refusal the rule reports (AmbiguousHyphen) at this offset; then no edits
}

pub(crate) enum Edit {
    Replace { at: u8, repl: Repl },             // new text of a token; the kind stays
    Glue { at: u8, glue: Glue },                // set the glue mark of a Space token
    Insert { before: u8, glue: Glue },          // insert a Space token carrying glue
    Remove { at: u8 },                          // remove a Space token
}
```

A rule that refuses a pattern and reports it (`AmbiguousHyphen`, section 8.4) returns a match
without edits whose `note` names the token of the diagnostic; the engine records the diagnostic when
it applies the match. The engine applies a proposal only if no affected token is `FROZEN` and no
insertion falls inside a frozen fragment; otherwise the whole proposal is dropped. An insertion
falls inside a frozen fragment when a token next to it is a frozen token of the call's text (a
frozen core, the hyphen of a compound word, a frozen space); next to a slot or protected text, which
are frozen but opaque, it falls outside them (`№{n}` → `№⍽{n}`). Every edit also lies within the
tokens the proposal takes. The engine does not call rules at all when the anchor is frozen. This
check sits in one place of the engine, so protection does not depend on the care of each rule. Edits
next to a frozen fragment are allowed: the space between `в` and `10.0.0.0/8` is not part of the
fragment, so `в⍽10.0.0.0/8` is fine, while the fragment itself is byte for byte intact.

### 3.8 Phases

```rust
type RuleFn = fn(&Cx<'_>, usize) -> Option<Match>;
// RULES: [RuleFn; 27] in `rules/mod.rs`, in the order of ORDER (section 4.1);
// cx.active_in_phase() yields the indices into RULES of the active rules of the phase, in that order.

fn run_phase<R: Recorder>(cx: &Cx<'_>, input: &[Tok], out: &mut Vec<Tok>, rec: &mut R) {
    out.clear();
    let mut i = 0;
    while i < input.len() {
        let m = if input[i].is_frozen() { None } else {
            cx.active_in_phase().iter().find_map(|&r| RULES[r as usize](cx, i).map(|m| (r, m)))
        };
        match m {
            Some((rule, m)) if cx.edits_allowed(i, &m) => { emit(out, input, i, rule, &m, rec); i += m.consume as usize + 1; }
            _ => { out.push(input[i]); i += 1; }
        }
    }
}
```

This is a sketch: real code has no panicking indexing. Properties of a phase:

1. **Snapshot.** All rules of a phase read the input of the phase and do not see each other's edits.
   A phase is a pure function of its input tokens and of the classification of the pass (the
   fragments with their kinds and flags that `Cx` gives, section 3.6), which is computed once per
   window before the first phase and is not recomputed after an edit.
2. **First match.** At each position the rules of the phase are tried in the fixed order of section
   4.1. The first rule that matches takes the tokens from the anchor to anchor + `consume`;
   positions inside the taken range are not considered again in this phase. An edit can only reach
   tokens from the anchor on, so `dash.double-hyphen` and `dash.spaced-hyphen`, whose closed style
   removes the space before the dash, anchor at that space and take the space, the dash and the
   space after it; `dash.dialogue` and the range, ellipsis and quote rules anchor at the token they
   replace. The no-break rules anchor at the first space they glue, so that the order table decides
   between two of them that want the same space; the rules that may insert a space anchor before it:
   `nbsp.abbreviations` at the first token of its entry (`и` of `и т.д.`), `nbsp.signs` at the sign,
   and `nbsp.units` at an attached `%` or `‰`.
3. **Match by class, not by product.** A rule matches a pattern whether or not it is already in its
   target form. If it is, the rule returns a `Match` with no edits and the same `consume`, and so it
   does on a typed typographic sign that it must not change (a typed dash, a typed `…`). For example
   `т.⍽д.` matches `nbsp.abbreviations` like `т.д.` and `т. д.` do. So the partition into taken
   ranges is the same on a second pass, except on fragments that a removed space merged or an
   inserted space split (section 4.3, I2; section 4.4).
4. **Bounded context.** A rule looks no further than a fixed number of tokens around its anchor (the
   longest is `nbsp.phone`, at most 24 tokens for its 10 to 15 digits in at most six fragments;
   abbreviations and units take up to eight). The exception is the quote phase: it has a linear
   preliminary pass over the window (section 6), after which its rules only read decisions.

`Cx` gives rules read-only access: the text of a token (resolved from the call's inputs and its
replacement), neighbours with `Format` and `Markup` tokens skipped, the fragment with its kind and
flags, language data, settings, line and window starts, piece numbers. Two token vectors per window
swap between phases. Every token names its fragment (`frag`, and `CORE` for a token of the core);
after a phase that inserted or removed a `Space` token, the token ranges of the fragments are moved
to their tokens, while the fragments, their kinds and flags stay those of the pass.

### 3.9 Glue marks

No-break rules do not write U+00A0 into the text. They set the glue mark of a `Space` token
(`Glue::NoBreak` or `Glue::Narrow`) or insert a `Space` token that carries one. A U+00A0 already in
the input that forms a `Space` token on its own is lexed with `Glue::NoBreak`, a lone U+202F with
`Glue::Narrow`; a longer run of horizontal whitespace has `Glue::None` even when it contains them
(section 3.5). Rendering turns the mark into U+00A0 or U+202F, `&nbsp;` or `&#x202F;`
(`HtmlStyle::NAMED`), `\u00A0` or `\u202F` (`EscapeStyle::JS`), `\u{a0}` or `\u{202f}`
(`EscapeStyle::RUST`), or into a nowrap group (section 11.5). A U+2011 NON-BREAKING HYPHEN inside a
fragment is a joint of its nowrap group, as a glue mark is between fragments.

Why: nowrap output needs the groups of glued tokens, which is natural when glue is a relation
between tokens; the report shows which rule glued; and when two rules could glue the same space, the
first of them in the order table takes the pattern (section 3.8) and its glue stands.

A space is glued only if it is a single U+0020 (one `Space` token of one character); tabs, runs of
spaces and line breaks are never glued.

### 3.10 Rendering and Cow

Rendering walks the tokens and writes each one's text (or glue) through the encoder of the output
kind (section 11). The tokens of a changed window give one edit of the call's text per replaced
token, glued space and inserted space, and the phases list the spaces they removed; the output is
the input with these edits applied. The output string is created lazily: while no window has changed
and encoding changes nothing, nothing is built. At the first change the output gets
`String::with_capacity(len + len / 8)`, the unchanged prefix of the input is copied in, and windows
are written from there, with the boundary characters between them (section 3.3). If nothing changed
to the end, the result is `Cow::Borrowed(input)`. For templates the same holds per text: a text that
maps to one unchanged input piece borrows it, and a changed text gets one string sized from its
pieces plus the new text of its edits, a bound on its length, so it is never grown (section 2.12: a
glued space takes two bytes for one, which `len + len / 8` would not cover in a short text such as
`{n} дн.`).

### 3.11 Recording

```rust
pub(crate) trait Recorder {
    const ACTIVE: bool;                          // false for NoRecord: the engine builds no record
    fn change(&mut self, change: Change);        // one record of one piece (stage 2, with the phases)
    fn protected(&mut self, span: ProtectedSpan);
    fn diagnostic(&mut self, diagnostic: Diagnostic);
}
pub(crate) struct NoRecord;     // every method empty and #[inline(always)]
pub(crate) struct Diagnostics;  // diagnostics only (lint)
pub(crate) struct Journal;      // everything (report)
```

The engine is generic over `R: Recorder` and monomorphized: `typeset` uses `NoRecord`, so the plain
path contains no recording code; `lint` uses `Diagnostics`; `report` uses `Journal`. Rules do not
depend on the recorder; a test compares the three modes on the corpus. The engine resolves the
positions of the call's text to pieces before it calls the recorder, and splits a span that crosses
piece boundaries: a change becomes one record per piece in input order, the first holding the whole
replacement and the others an empty replacement (a deletion); a protected span becomes one record
per piece it covers; a diagnostic is reported at its first piece, with the range clipped to that
piece. A span or a diagnostic that starts at a slot or a protected piece is reported at that piece.
`Diagnostics` gets diagnostics clipped the same way, so `lint` and `report` give equal diagnostics
(section 12.3).

### 3.12 Complexity and pathological input

- Time: lexer O(n); classifier O(n); seven phases, each O(n) with bounded context; quote matching
  with a stack, one push and one pop per quote; lookups in sorted static lists by binary search,
  O(log k) per fragment with k the longest list (hundreds); rendering O(n). Total O(n log k). There
  is no recursion and no backtracking.
- Protection delimiters: fenced blocks are found in one pass over the input, other delimiters in one
  pass per window. If the closing delimiter of a pair is not found from position p, it is not found
  further on either, so the search for that pair stops. A hundred thousand unclosed backticks are
  processed in linear time.
- Memory: the token buffers of the longest window (about 40 bytes per token), the output string, and
  the journal only when recording. For text with paragraphs, memory does not grow with the input. A
  10 MB paragraph without line breaks is the pathological case: memory is linear in its length. Cost
  is per token, not per byte: a paragraph of 10^5 emoji clusters holds about 4 MB of tokens in each
  of the two buffers.
- Inputs exercised by the load tests (section 13.8): long runs of `"`, `'`, backticks, `-`, `.`,
  `(`; alternating Cyrillic and Latin letters; a string of ZWJ emoji; one 1 MiB line without spaces
  (one fragment: the classifier is linear, freezing is one pass); quote nesting 10 000 deep; a long
  run of `Format` tokens (U+200B, U+2060 U+200C) before many dashes, each a fragment of its own (a
  rule reads the nearest non-transparent neighbors of a token, never the whole run again); many
  straight quotes in the wrapper of one technical fragment (what the quote phase reads of a fragment
  is computed once per fragment).

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
