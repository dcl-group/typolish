# typolish design: Reverse transformation, source check, explain mode

Part of the [typolish design](./README.md). Section numbers are those of the whole design.

## 12. Reverse transformation, source check, explain mode

All three run on the engine of section 3: `to_plain` is a separate pass over the same tokens with
inverse tables, `lint` is the engine with the `Diagnostics` recorder, plus the `to_plain` pass that
raises `TypographicChar` (section 12.3), and `report` is the engine with the `Journal` recorder
(section 3.11).

### 12.1 Reverse transformation

`to_plain` (section 2.10) maps typeset Unicode text to the **canonical plain form** of the
typographer: it undoes exactly the rules that are enabled, so the round trip holds for every rule
set on its own. Characters that no enabled rule produces are left alone (a `©` stays when
`symbols.copyright` is off). The spans of `protected()` (section 12.5: delimited spans, with
delimiters working as in `typeset`, protected pieces, slots and frozen fragments) and markup are
never touched.

| Rule                                       | Typeset                                                                   | Plain                                                       |
| ------------------------------------------ | ------------------------------------------------------------------------- | ----------------------------------------------------------- |
| `quotes.pairs`                             | glyphs of both quote pairs of the language, at every level                | `"`                                                         |
| `quotes.apostrophe`                        | `’` that is an apostrophe by the rule below the table                     | `'`                                                         |
| `quotes.primes`                            | `″`, `′`                                                                  | `"`, `'`                                                    |
| `dash.spaced-hyphen`, `dash.double-hyphen` | `—` with spaces (glued or not) between operands; `—` closed between words | `-` with the same spaces; spaced `-` for the closed form    |
| `dash.dialogue`                            | `—⍽` at a line start                                                      | `- `                                                        |
| `range.*`                                  | `–` without spaces between numbers, slots, times, Roman numerals          | `-`                                                         |
| `ellipsis.*`                               | `…`                                                                       | `...`; `?..` and `!..` stay (they are plain)                |
| `nbsp.*`                                   | glue (U+00A0, U+202F) set by a rule's pattern                             | a space; inserted glue (`т.⍽е.`, `№⍽5`) becomes a space too |
| `number.minus`                             | `−` before a number                                                       | `-`                                                         |
| `number.times`                             | `×` between numbers                                                       | `x`                                                         |
| `symbols.*`                                | `©` `®` `™` `→` `←` `↔` `≥` `≤` `≠` `±`                                   | `(c)` `(r)` `(tm)` `->` `<-` `<->` `>=` `<=` `!=` `+-`      |

A typed `–` with spaces (`Москва⍽– столица`, `10⍽– 20`) is not a product of the range rules and
stays; only the glue that `nbsp.dash` set before it becomes a space (section 8.7).

`to_plain` changes only the typographic characters of the table above. Plain forms (`--`, `т.д.`,
`?...`, `(C)`) are not normalized: `to_plain("a -- b")` returns the input. Its output contains no
character it would change, so `to_plain(to_plain(x)) == to_plain(x)`, and text without such
characters comes back as `Cow::Borrowed`.

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

The canonical plain form, for the enabled rules, is the Plain column of the table above, with these
choices: no typographic character that the rules produce; a dash between words written `-` (not
`--`) with spaces on both sides; a range written `a-b` with no spaces; `x`, `(c)`, `(r)`, `(tm)` in
lowercase Latin letters; abbreviations with a space (`т. е.`); `№ 5` with a space; double quotes at
every level; `'` only for apostrophes and primes; `...` for an ellipsis; `?..` after a question in
Russian; a percentage in the form that `PercentSpacing` keeps: either form with `AsWritten`, `100%`
with `Attached` and with `Language` for en, `100 %` with `Spaced` and with `Language` for ru.

It is a method of the typographer because it needs the enabled rules, the quote pairs, single-quote
mode and dash style of the typographer (the language data with the builder's overrides), and the
protection settings. Input is Unicode; decoding HTML or escapes is the caller's (tests have
decoders, section 13.5).

### 12.2 Round trip

Two properties and one table, for both languages and the rule sets of section 13.5 (property "Round
trip"):

- **P1**: `to_plain(typeset(x)) == x` for every `x` in the canonical plain form (section 12.1) for
  the settings under test, except the pattern of `NestedPossessive` (two `s'` at the same depth of
  one nested pair). Every mismatch fails: inside the canonical form there is no other loss.
- **P2**: `typeset(to_plain(y)) == y` for `y = typeset(x)` and any plain `x`, the rows of the table
  below included.

There is no external corpus (decided by the maintainer on 2026-11-21); stage reports say that the
corpus is the project's own. P1 runs on a generator of canonical strings and on the corpus files of
section 13.7: `prose.txt` and `ui.tmpl` with both sets, `tech.txt` with Interface only (Prose turns
a lone `--` between words into `—`, section 10.5). P2 runs on a generator of any plain strings and
on every corpus file with both sets.

Known losses are rows of `tests/corpus/roundtrip-known.tsv` with the columns language, settings,
input, `typeset`, `to_plain` and code. The test checks each row: `typeset(input)` and `to_plain` of
it equal the columns, and `to_plain(typeset(input)) != input`; a row whose loss vanished fails, and
the failure prints the list. A new loss inside the canonical form is a P1 failure. Every row but
`NestedPossessive` is outside the canonical form; that one is the only ambiguity inside it, the
generator for P1 avoids its pattern, and its row also passes P2.

| Input (non-canonical unless noted)                 | Settings                   | `typeset`                 | `to_plain`                | Code                |
| -------------------------------------------------- | -------------------------- | ------------------------- | ------------------------- | ------------------- |
| `--` between words                                 | `default`                  | `—`                       | `-`                       | `DoubleHyphen`      |
| `т.д.`                                             | `default`                  | `т.⍽д.`                   | `т. д.`                   | `CompactAbbr`       |
| `№5`                                               | `default`                  | `№⍽5`                     | `№ 5`                     | `AttachedSign`      |
| `?...` (ru)                                        | `default`                  | `?..`                     | `?..`                     | `TerminalEllipsis`  |
| `'x'`                                              | `SingleQuotes::Everywhere` | `“x”`                     | `"x"`                     | `SingleQuote`       |
| `"a 'b' c"` (en)                                   | `default`                  | `“a ‘b’ c”`               | `"a "b" c"`               | `NestedSingle`      |
| `1920х1080` (Cyrillic `х`)                         | `default`                  | `1920×1080`               | `1920x1080`               | `CyrillicTimes`     |
| `100%` (ru)                                        | `PercentSpacing::Language` | `100⍽%`                   | `100 %`                   | `PercentInserted`   |
| `100 %` (en)                                       | `PercentSpacing::Attached` | `100%`                    | `100%`                    | `PercentRemoved`    |
| `(C) 2026`, `(с) 2026` (Cyrillic `с`), `Brand(TM)` | `default`                  | `© 2026`, `Brand™`        | `(c) 2026`, `Brand(tm)`   | `LegalMarkSpelling` |
| `1920X1080`                                        | `default`                  | `1920×1080`               | `1920x1080`               | `TimesSpelling`     |
| U+00A0 typed by the author                         | `default`                  | U+00A0                    | a space                   | `AuthorGlue`        |
| `"a "the dogs' bowls" b"` (en; canonical)          | `default`                  | `“a ‘the dogs’ bowls’ b”` | `"a "the dogs" bowls' b"` | `NestedPossessive`  |

### 12.3 Source check

`lint` checks a source that a caller's project requires to stay plain, without building output. It
returns the diagnostics that typesetting would produce (quotes, delimiters, hyphens, mixed scripts,
slots) plus `TypographicChar` for every character that `to_plain` would change (section 12.1): a
character of the typographic set that an enabled rule produces, outside the spans of `protected()`
(section 12.5: delimited spans, protected pieces, slots and frozen fragments) and outside `Markup`
pieces. `lint` renders nothing, so it never gives `NowrapFallback`; its other diagnostics are those
of `report(x, Output::Unicode)`. Characters of the set that `to_plain` leaves (no enabled rule
produces them) are not reported, because typesetting would not bring them back. So `lint(x)` has a
`TypographicChar` exactly when `to_plain(x) != x`, and `to_plain` is the matching fix: a repository
with a "plain sources only" policy runs `lint` in its checks and `to_plain` to repair.

A generator calls `report_template` instead, which gives the same diagnostics together with the
result, and decides by `Severity` whether to warn or to fail the build. The crate reports levels; it
never decides for the caller.

### 12.4 Diagnostics

| Kind                | Severity | Raised by                                  | When                                                                                                       |
| ------------------- | -------- | ------------------------------------------ | ---------------------------------------------------------------------------------------------------------- |
| `UnmatchedQuote`    | warning  | `quotes.pairs`                             | a straight `"` without a pair; its window keeps straight quotes (section 6.6)                              |
| `UnmatchedQuote`    | info     | `quotes.pairs`                             | a typed quote glyph of the language without a partner; it stays neutral                                    |
| `CrossingQuotes`    | warning  | `quotes.pairs`                             | pairs would cross each other or a typed pair                                                               |
| `AmbiguousQuote`    | info     | `quotes.pairs`                             | a straight quote with spaces or letters on both sides, or one of three or more quotes in a series: no role |
| `UnclosedDelimiter` | warning  | protection (no rule)                       | an opening delimiter without its closing one; it was treated as text                                       |
| `AmbiguousHyphen`   | info     | `dash.spaced-hyphen`, `dash.double-hyphen` | `-` that section 8.4 refuses (numbers, single letters, foreign words, a flag)                              |
| `MixedScript`       | warning  | classifier (no rule)                       | a word with letters of two scripts (`Мoсква`), unless a dictionary entry (`μs`)                            |
| `SlotGlue`          | warning  | classifier (no rule)                       | a `Number` slot glued to letters (`{n}МБ`)                                                                 |
| `SlotGlue`          | info     | classifier (no rule)                       | two slots glued together (`{a}{b}`)                                                                        |
| `NowrapFallback`    | info     | renderer (no rule)                         | a nowrap group split at a `Markup` piece; the glue at the split written as an entity                       |
| `TypographicChar`   | info     | `lint` only                                | a character that `to_plain` would change (section 12.3)                                                    |

`Diagnostic` carries its kind, severity, the rule that raised it (`Option<Rule>`), the piece and the
byte range. A diagnostic over several pieces is reported at its first piece, with the range clipped
to that piece (section 3.11), in `lint` as in `report`. `Severity` is ordered (`Info < Warning`).
Diagnostics, in `report`, `report_template`, `lint` and `lint_template`, are ordered by piece, then
byte start, then byte end, then kind in the order of the `DiagnosticKind` declaration
(`TypographicChar` last). `Diagnostic` implements `Display`:
`warning quotes.pairs 0:10..11: straight quote has no pair; left as is`.

### 12.5 Explain mode

`report` and `report_template` (section 2.9) return the result of `typeset` together with:

- `changes()`: one record per applied edit and piece (an edit across a boundary of `Text` pieces
  gives one record per piece, section 2.9): rule, piece, byte range in the input, replacement in
  Unicode. Records are ordered by position, then by rule order; if two rules touched one position (a
  replacement and a glue next to it), there are two records. Empty edits ("already done") are not
  recorded.
- `protected()`: the spans no rule may touch, one record per piece a span covers (section 3.11),
  with their kind: `Delimited` (backticks or own pairs), `ProtectedPiece`, `Slot`, or
  `Technical(TechKind)` for a frozen fragment (`Url`, `Email`, `Flag`, `Path`, `TimeRange` (the
  times only), `Colon`, `Operator`, `Identifier`, `Angle`, `Uuid`, `Hex`, `IsoDate`, `Version`,
  `Dotted`, `List`, `CompoundHyphens`, `Unclassified`). This is the main tool for "why did no rule
  fire here". A `Punctuation` fragment is not a protected span.
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
