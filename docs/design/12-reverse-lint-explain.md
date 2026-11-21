# typolish design: Reverse transformation, source check, explain mode

Part of the [typolish design](./README.md). Section numbers are those of the whole design.

## 12. Reverse transformation, source check, explain mode

All three run on the engine of section 3: `to_plain` is a separate pass over the same tokens with
inverse tables, `lint` is the engine with the `Diagnostics` recorder, and `report` is the engine
with the `Journal` recorder (section 3.11).

### 12.1 Reverse transformation

`to_plain` (section 2.10) maps typeset Unicode text to the **canonical plain form** of the
typographer: it undoes exactly the rules that are enabled, so the round trip holds for every rule
set on its own. Characters that no enabled rule produces are left alone (a `©` stays when
`symbols.copyright` is off). Slots, protected spans (delimiters work as in `typeset`), markup and
frozen fragments are never touched.

| Rule                                       | Typeset                                                                   | Plain                                                       |
| ------------------------------------------ | ------------------------------------------------------------------------- | ----------------------------------------------------------- |
| `quotes.pairs`                             | glyphs of both quote pairs of the language, at every level                | `"`                                                         |
| `quotes.apostrophe`                        | `’` that is an apostrophe by the rule below the table                     | `'`                                                         |
| `quotes.primes`                            | `″`, `′`                                                                  | `"`, `'`                                                    |
| `dash.spaced-hyphen`, `dash.double-hyphen` | `—` with spaces (glued or not) between operands; `—` closed between words | `-` with the same spaces; spaced `-` for the closed form    |
| `dash.dialogue`                            | `—⍽` at a line start                                                      | `- `                                                        |
| `range.*`                                  | `–` between numbers, slots, times, Roman numerals                         | `-`                                                         |
| `ellipsis.*`                               | `…`                                                                       | `...`; `?..` and `!..` stay (they are plain)                |
| `nbsp.*`                                   | glue (U+00A0, U+202F) set by a rule's pattern                             | a space; inserted glue (`т.⍽е.`, `№⍽5`) becomes a space too |
| `number.minus`                             | `−` before a number                                                       | `-`                                                         |
| `number.times`                             | `×` between numbers                                                       | `x`                                                         |
| `symbols.*`                                | `©` `®` `™` `→` `←` `↔` `≥` `≤` `≠` `±`                                   | `(c)` `(r)` `(tm)` `->` `<-` `<->` `>=` `<=` `!=` `+-`      |

**Which `’` is an apostrophe** (English, where `’` is both the apostrophe and the nested closing
quote). A `’` in a fixed apostrophe context of section 6.2 (between letters, after a digit before a
letter, an elision, at a word start before digits) is an apostrophe. Any other `’` with a closing
role closes the innermost open `‘` of its depth, mirroring the forward algorithm, with one
exception: a `’` after `s` and before a non-letter, where the forward algorithm puts a trailing
possessive, leaves the `‘` open if a later `’` of the same depth, before the next `‘` of that depth
and before the closer of the enclosing pair, has a closing role and does not follow `s`; that later
`’` closes it. A `’` that closes nothing is an apostrophe. So `“He said ‘the dogs’ bowl is empty’”`
→ `"He said "the dogs' bowl is empty""`, and `“‘Hi,’ said the dogs’ owner”` →
`""Hi," said the dogs' owner"`. When every candidate follows `s` (`‘the dogs’ bowls’`), the first
one closes, and the round trip lists the case as a known loss (`NestedPossessive`, section 12.2).

The canonical plain form, for the enabled rules: no typographic character that they produce; a dash
written `-` (not `--`) with spaces on both sides; abbreviations with a space (`т. е.`); `№ 5` with a
space; double quotes at every level; `'` only for apostrophes and primes; `...` for an ellipsis;
`?..` after a question in Russian.

It is a method of the typographer because it needs the enabled rules, the quote pairs and dash style
of the language, and the protection settings. Input is Unicode; decoding HTML or escapes is the
caller's (tests have decoders, section 13.5).

### 12.2 Round trip

Two properties, tested for both languages and both rule sets:

- **P1**: `to_plain(typeset(x)) == x` for `x` in the canonical plain form;
- **P2**: `typeset(to_plain(y)) == y` for `y = typeset(x)` and any plain `x`.

There is no external corpus (decided by the maintainer on 2026-11-21). P1 and P2 run on the
project's own corpus (section 13.7) and on a generator of canonical strings; stage reports say that
the corpus is the project's own. Known losses are listed in `tests/corpus/roundtrip-known.tsv` with
a reason code; a new mismatch or a vanished one fails the test and prints the list. All but one fall
outside the canonical form; `NestedPossessive` is an ambiguity inside it, and the generator of
canonical strings for P1 avoids its pattern (two `s'` at the same depth of one nested pair).

| Input (non-canonical unless noted)          | `typeset`                 | `to_plain`                | Code               |
| ------------------------------------------- | ------------------------- | ------------------------- | ------------------ |
| `--` between words                          | `—`                       | `-`                       | `DoubleHyphen`     |
| `т.д.`                                      | `т.⍽д.`                   | `т. д.`                   | `CompactAbbr`      |
| `№5`                                        | `№⍽5`                     | `№ 5`                     | `AttachedSign`     |
| `?...` (ru)                                 | `?..`                     | `?..`                     | `TerminalEllipsis` |
| `'x'` with `SingleQuotes::Everywhere`       | `“x”`                     | `"x"`                     | `SingleQuote`      |
| `"a 'b' c"` (en)                            | `“a ‘b’ c”`               | `"a "b" c"`               | `NestedSingle`     |
| `1920х1080` (Cyrillic `х`)                  | `1920×1080`               | `1920x1080`               | `CyrillicTimes`    |
| `100%` with `PercentSpacing::Language` (ru) | `100⍽%`                   | `100 %`                   | `PercentInserted`  |
| U+00A0 typed by the author                  | U+00A0                    | a space                   | `AuthorGlue`       |
| `"a "the dogs' bowls" b"` (en; canonical)   | `“a ‘the dogs’ bowls’ b”` | `"a "the dogs" bowls' b"` | `NestedPossessive` |

### 12.3 Source check

`lint` checks a source that a caller's project requires to stay plain, without building output. It
returns the diagnostics that typesetting would produce (quotes, delimiters, hyphens, mixed scripts,
slots) plus `TypographicChar` for every character of the typographic set (section 11.2) outside
protected spans and slots. Results are ordered by position. `to_plain` is the matching fix: a
repository with a "plain sources only" policy runs `lint` in its checks and `to_plain` to repair.

A generator calls `report_template` instead, which gives the same diagnostics together with the
result, and decides by `Severity` whether to warn or to fail the build. The crate reports levels; it
never decides for the caller.

### 12.4 Diagnostics

| Kind                | Severity | Raised by                                  | When                                                                                 |
| ------------------- | -------- | ------------------------------------------ | ------------------------------------------------------------------------------------ |
| `UnmatchedQuote`    | warning  | `quotes.pairs`                             | a straight `"` without a pair; its window keeps straight quotes (section 6.6)        |
| `UnmatchedQuote`    | info     | `quotes.pairs`                             | a typed quote glyph of the language without a partner; it stays neutral              |
| `CrossingQuotes`    | warning  | `quotes.pairs`                             | pairs would cross each other or a typed pair                                         |
| `AmbiguousQuote`    | info     | `quotes.pairs`                             | a straight quote with spaces or letters on both sides: no role                       |
| `UnclosedDelimiter` | warning  | protection (no rule)                       | an opening delimiter without its closing one; it was treated as text                 |
| `AmbiguousHyphen`   | info     | `dash.spaced-hyphen`, `dash.double-hyphen` | `-` that section 8.4 refuses (numbers, single letters, foreign words, a flag)        |
| `MixedScript`       | warning  | classifier (no rule)                       | a word with letters of two scripts (`Мoсква`), unless a dictionary entry (`μs`)      |
| `SlotGlue`          | warning  | classifier (no rule)                       | a `Number` slot glued to letters (`{n}МБ`)                                           |
| `SlotGlue`          | info     | classifier (no rule)                       | two slots glued together (`{a}{b}`)                                                  |
| `NowrapFallback`    | info     | renderer (no rule)                         | a nowrap group split at a `Markup` piece; the glue at the split written as an entity |
| `TypographicChar`   | info     | `lint` only                                | a character of the typographic set in a plain source                                 |

`Diagnostic` carries its kind, severity, the rule that raised it (`Option<Rule>`), the piece and the
byte range. `Severity` is ordered (`Info < Warning`). `Diagnostic` implements `Display`:
`warning quotes.pairs 0:10..11: straight quote has no pair; left as is`.

### 12.5 Explain mode

`report` and `report_template` (section 2.9) return the result of `typeset` together with:

- `changes()`: one record per applied edit: rule, piece, byte range in the input, replacement in
  Unicode. Records are ordered by position, then by rule order; if two rules touched one position (a
  replacement and a glue next to it), there are two records. Empty edits ("already done") are not
  recorded.
- `protected()`: the spans no rule may touch, with their kind: `Delimited` (backticks or own pairs),
  `ProtectedPiece`, `Slot`, or `Technical(TechKind)` for a frozen fragment (`Url`, `Email`, `Flag`,
  `Path`, `TimeRange` (the times only), `Colon`, `Operator`, `Identifier`, `Angle`, `Uuid`, `Hex`,
  `IsoDate`, `Version`, `Dotted`, `List`, `CompoundHyphens`, `Unclassified`). This is the main tool
  for "why did no rule fire here".
- `diagnostics()`: as in section 12.4.

`Display` prints a table for reports of differences. For Russian Prose, input
`Он - "врач", см. host:8080`, result `Он⍽— «врач», см.⍽host:8080`:

```text
nbsp.dash           0:4..5    " " -> U+00A0
dash.spaced-hyphen  0:5..6    "-" -> "—"
quotes.pairs        0:7..8    "\"" -> "«"
quotes.pairs        0:16..17  "\"" -> "»"
nbsp.abbreviations  0:24..25  " " -> U+00A0
protected  Technical(Colon)  0:25..34  host:8080
```

`report` runs the same engine as `typeset`, so its result equals `typeset` byte for byte (a property
test), and applying `changes()` to the input reproduces the result. In stage 5 the comparison with
the reference implementation prints, for every case that differs and is not in the table of expected
differences, the input, our result, the reference result and this table.
