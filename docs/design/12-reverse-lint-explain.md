# typolish design: Reverse transformation, source check, explain mode

Part of the [typolish design](./README.md). Section numbers are those of the whole design.

## 12. Reverse transformation, source check, explain mode

All three run on the engine of section 3: `to_plain` reads the text with inverse tables and checks
its readings with forward runs of the engine (section 12.1), `lint` is the engine with the
`Diagnostics` recorder, plus the `to_plain` pass that raises `TypographicChar` (section 12.3), and
`report` is the engine with the `Journal` recorder (section 3.11).

### 12.1 Reverse transformation

`to_plain` (section 2.10) maps typeset Unicode text to the **canonical plain form** of the
typographer: it undoes exactly the rules that are enabled, so the round trip of section 12.2 (P1 and
P2) holds for every rule set on its own. Characters that no enabled rule produces are left alone (a
`©` stays when `symbols.copyright` is off). The spans of `protected()` (section 12.5: delimited
spans, with delimiters working as in `typeset`, protected pieces, slots and frozen fragments) and
markup are never touched. In a template a slot or a protected piece next to a quote reads as a word,
and a `BlockMarkup` piece as the edge of the text, since the paragraph ends there (choice C9a).
Where such pieces stand together, the one next to the character decides: in `‘x’`, a slot,
`</p><p>`, the `’` is read before a word, as in a separate call that ends with the slot, and in
`</p><p>`, a slot, `’x`, the `’` is read after a word.

| Rule                                       | Typeset                                                                                                                                                                                                                                                                                                                                                 | Plain                                                                                                          |
| ------------------------------------------ | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------- |
| `quotes.pairs`                             | glyphs of both quote pairs of the language, at every level                                                                                                                                                                                                                                                                                              | `"`                                                                                                            |
| `quotes.apostrophe`                        | `’` that is an apostrophe by the rule below the table                                                                                                                                                                                                                                                                                                   | `'`                                                                                                            |
| `quotes.primes`                            | `″`, `′`                                                                                                                                                                                                                                                                                                                                                | `"`, `'`                                                                                                       |
| `dash.spaced-hyphen`, `dash.double-hyphen` | `—` with spaces (glued or not) between operands; `—` closed between two letters of words (whatever the dash style)                                                                                                                                                                                                                                      | `-` with the same spaces; spaced `-` for the closed form; `--` instead of `-` when `dash.spaced-hyphen` is off |
| `dash.dialogue`                            | `—⍽` at a line start                                                                                                                                                                                                                                                                                                                                    | `- `                                                                                                           |
| `range.*`                                  | the range dash of the language (`–`, or `—` for a declared language, sections 7.6 and 8.5) without spaces between numbers, slots, times, Roman numerals                                                                                                                                                                                                 | `-`                                                                                                            |
| `ellipsis.*`                               | `…`                                                                                                                                                                                                                                                                                                                                                     | `...`; `?..` and `!..` stay (they are plain)                                                                   |
| `nbsp.*`                                   | glue (U+00A0, U+202F) set by a rule's pattern                                                                                                                                                                                                                                                                                                           | a space; inserted glue (`т.⍽е.`, `№⍽5`) becomes a space too                                                    |
| `number.minus`                             | `−` before a number                                                                                                                                                                                                                                                                                                                                     | `-`                                                                                                            |
| `number.times`                             | `×` between numbers                                                                                                                                                                                                                                                                                                                                     | `x`                                                                                                            |
| `symbols.*`                                | `©` `®` `™` where `symbols.copyright` would produce them (section 8.9: `©` before a four-digit year, attached or one space away, `©` one space after a four-digit year, or `©` one space from a range of years; `®`, `™` attached to or one space after a brand name of section 8.9, not followed by an operator fragment), `→` `←` `↔` `≥` `≤` `≠` `±` | `(c)` `(r)` `(tm)` `->` `<-` `<->` `>=` `<=` `!=` `+-`                                                         |

Glue that no enabled rule sets at its place stays: with `nbsp.units` alone, `5⍽МБ в⍽доме` →
`5 МБ в⍽доме`; in Prose `Знаешь⍽ли⍽ты` → `Знаешь ли⍽ты`, since no rule glues after a particle bound
to the word before it (section 8.7). A typed `–` with spaces (`Москва⍽– столица`, `10⍽– 20`) is not
a product of the range rules and stays; only the glue that `nbsp.dash` set before it becomes a space
(section 8.7). A closed `—` next to a number, a slot, a sign or a protected span is no product of
the dash rules and stays (`word—5`, `in 2020—the year`), unless it is the range dash of the language
(section 8.5). Between two letters the closed `—` reads with spaces first and `-` last, so the range
dash `—` of a declared language between Roman numerals (`XIX—XX века`) comes back as `-`. A lone
`©`, `®` or `™` outside those contexts (`© Example Corp`, `® знак`) is not a product of the rule and
stays.

`to_plain` changes only the typographic characters of the table above. Plain forms (`--`, `т.д.`,
`?...`, `(C)`) are not normalized: `to_plain("a -- b")` returns the input. Its output contains no
character it would change, so `to_plain(to_plain(x)) == to_plain(x)`, and text without such
characters comes back as `Cow::Borrowed`.

**How a reading is checked.** The table gives the reading of every typographic character outside the
spans of `protected()`: the plain text it came from (`a—b` reads as `a - b`, or `a -- b` when only
`dash.double-hyphen` is on; `a — b` as `a - b` or `a -- b`). Quote glyphs and apostrophes are undone
by the table and the rule below, since their plain form does not depend on the level the forward
rule would choose (`„x“` and `‘x’` at the top level give `"x"` too). Their readings are checked
first, because they are context for the rules that check the others: the engine runs forward with
the quote readings in place, and a quote reading comes back when it gives a glyph of the pairs at
any level (or `’`). In a window where one does not come back, the glyphs of the nested pair read as
`'` instead, if that brings every quote of the window back: the double quotes of `«x „“.»`, read as
`"x "".`, do not pair again (the closer of the nested pair and the last quote form the sign-only
pair `"."`, section 6.3), while `"x ''."` gives `«x „“.»`. In a window that this does not bring
back, the glyphs of the nested pair and, with `SingleQuotes::Everywhere`, those of the primary pair
read as `'` (`“3" 0”` as `'3" 0'`, since `"3" 0"` gives `“3” 0"` in Interface and `“3” 0″` in
Prose); in a window that neither brings back, the glyphs of the primary pair inside a nested pair
(the third level of ru) read as `'` alone (`«„«6»“»` as `""'6'""`, since `"""6"""` is a series that
does not pair). Each try is kept only where it brings every quote of the window back. Otherwise the
double quotes stay. Every other character is undone only where an enabled rule produces it, and the
rules decide that themselves: the engine runs forward on the text with the readings in place, and a
reading is dropped (the character stays) when it does not come back as its character, or when it
makes a rule edit the text next to it that the text as it was does not get, so that the round trip
would break there. Glue of either width counts as glue, and the spaces that the dash style sets
around a dash do not count; the restored spaces of a closed dash are exempt from the second test,
because the closed form joins two words into one fragment, which by design changes the rules around
it (section 8.4). A reading that failed only because a neighboring reading was wrong is tried again
alone in its neighborhood: readings farther apart than any rule looks (256 bytes; the longest
pattern, `nbsp.phone`, spans at most 24 tokens) are tried in one forward run, and a problem of the
run is laid at the nearest of them, so the number of runs does not grow with the length of the text,
and the bookkeeping of the candidates is linear (a reading is compared with its neighbors only). The
whole pass repeats on its own output until nothing changes, so `to_plain` is stable by construction.
There is no second copy of the rule patterns to drift from them. The price is up to about (readings
within 256 bytes) × (readings of a character) forward runs per pass, each over the whole text: on
dense typographic text that is hundreds of runs. On ordinary text the time is linear in its length
and about 100 to 200 times that of `typeset`, which the tooling path allows (section 2.12) and the
maintainer accepted (section 15.2, item 28). A long run of quote characters in one paragraph is
linear too. The readings of the quotes are found in one pass over them: the neighbors of a series of
quotes (section 6.3) are found once for the series, not once for each of its quotes; the open pairs
are kept with the places of the entries of each kind, so the innermost pair of a kind and whether a
nested pair is open below it are found in constant time; and the later `’` that the rule for `’`
below looks for is found for every `’` after `s` or `S` in one pass, the `’` that wait at the same
depth grouped together. Before version 0.1.2 each of these was searched again for every quote, and
96 KB of `’` in one paragraph took about 5 s; it now takes about 20 ms, and `«„` repeated, `x`, `“»`
repeated (96 KB, Russian Prose) about 45 ms instead of 4.4 s. Every pass that changes something
turns at least one non-ASCII character into ASCII text, so the passes end; on the inputs measured,
ordinary prose and long runs of quotes in one paragraph alike, there are one or two.

**Which `’` is an apostrophe** (English, where `’` is both the apostrophe and the nested closing
quote). A `’` in a fixed apostrophe context of section 6.2 (between letters, after a digit before a
letter, an elision, at a word start before digits) is an apostrophe. A `’` that has nothing but
signs (or nothing) between it and the `‘` on top of the open pairs, the `‘` of the pair whose closer
is `’` (primary or nested), where that `‘` has L of class `Space`, `OpenP` or `DashC` and the `’`
has R of class `Space`, `CloseP` or `DashC`, closes it: the mirror of the sign-only pair of section
6.3, by which plain `""`, `"-"` or `"?"` inside a pair is typeset as `‘’`, `‘-’`, `‘?’` whatever the
roles of its series (`“a ‘’ b”` → `"a "" b"`); two adjacent quotes in a series of three or more do
not count. Any other `’` with a closing role closes the innermost open `‘` of its depth, mirroring
the forward algorithm, with one exception: a `’` after `s` or `S` and before a non-letter, where the
forward algorithm puts a trailing possessive, leaves the `‘` open if a later `’` of the same depth,
before the next `‘` of that depth and before the closer of the enclosing pair, has a closing role
and does not follow `s` or `S`; that later `’` closes it. The pairs opened after the first `’` are
followed by their kind: a `’` inside a deeper `“…”` closes nothing there (it is an apostrophe), so
it neither ends that pair nor the search (`“‘a dogs’ b “c dogs’ d” e’ f”` →
`""a dogs' b "c dogs' d" e" f"`). A `’` that closes nothing is an apostrophe. So
`“He said ‘the dogs’ bowl is empty’”` → `"He said "the dogs' bowl is empty""`, and
`“‘Hi,’ said the dogs’ owner”` → `""Hi," said the dogs' owner"`. When every candidate follows `s` or
`S` (`‘the dogs’ bowls’`, `‘THE DOGS’ BOWLS’`), the first one closes, and the round trip lists the
case as a known loss (`NestedPossessive`, section 12.2). That reading gives the same glyphs as long
as only text stands between the two; where a quote of a pair or an unpaired straight `'` opener
stands between them, the forward rule leaves the possessive straight (section 6.6:
`“a ‘dogs' 's’ b”`), so the pair has one candidate. A `’` read as an apostrophe must come back as
`’`; where its `'` would pair as a quote instead, the `’` stays: in `“a ‘dogs’ bowl ’twas’ b”` the
possessive closes the nested pair, and `'twas'` would be a pair (section 6.2), so it reads as
`"a "dogs" bowl ’twas' b"`. The classes of the neighbors are those of section 6.3: a character
inside a protected span (a frozen core, a delimited span) is of class `Word`, so the `’` of
`“x ‘y 2026{’”` closes.

The canonical plain form, for the enabled rules, is the Plain column of the table above, with these
choices: no typographic character that the rules produce; a dash between words written `-` (not
`--`, unless `dash.double-hyphen` is on and `dash.spaced-hyphen` off) with spaces on both sides; a
range written `a-b` with no spaces; `x`, `(c)`, `(r)`, `(tm)` in lowercase Latin letters;
abbreviations with a space (`т. е.`); `№ 5` with a space; double quotes at every level (a nested
pair, or a primary pair in the cases above, reads as `'` only where the double quotes do not pair
again, see above, and such text is not canonical); `'` only for apostrophes and primes; `...` for an
ellipsis; `?..` after a question in Russian; a percentage in the form that `PercentSpacing` keeps:
either form with `AsWritten`, `100%` with `Attached` and with `Language` for en, `100 %` with
`Spaced` and with `Language` for ru.

It is a method of the typographer because it needs the enabled rules, the quote pairs, single-quote
mode and dash style of the typographer (the language data with the builder's overrides), and the
protection settings. Input is Unicode; decoding HTML or escapes is the caller's (tests have
decoders, section 13.5).

### 12.2 Round trip

Two properties and one table, for both languages and the declared languages of the test setup and
the rule sets of section 13.5 (property "Round trip"):

- **P1**: `to_plain(typeset(x)) == x` for every `x` in the canonical plain form (section 12.1) for
  the settings under test, except the pattern of `NestedPossessive` (two `s'` or `S'` at the same
  depth of one nested pair). Every mismatch fails: inside the canonical form there is no other loss.
- **P2**: `typeset(to_plain(y)) == y` for `y = typeset(x)` and any plain `x`, the rows of the table
  below included, except the defects listed in section 10.8: sign-only pairs that share a quote, a
  neutral `‘` before a straight pair in a language whose primary pair is `‘’`, and windows where no
  reading of the quotes that `to_plain` tries brings them back (a straight `"` without a role inside
  a pair, a weak quote after a digit that closes a pair, a reading that brings the quotes back but
  not a dash that reads them).

There is no external corpus (decided by the maintainer on 2026-11-21); stage reports say that the
corpus is the project's own. P1 runs on a generator of canonical strings and on the corpus files of
section 13.7: `prose.txt` and `ui.tmpl` with both sets, `tech.txt` with Interface only (Prose turns
a lone `--` between words into `—`, section 10.5). P2 runs on a generator of any plain strings, on
the plain forms of the sentences of the structured grammar of section 13.5 in `ru-RU`, `en-US`,
`COMMON` and two declared languages, and on every corpus file with both sets.

Known losses are rows of `tests/corpus/roundtrip-known.tsv` with the columns language, settings,
input, `typeset`, `to_plain` and code. The test checks each row: `typeset(input)` and `to_plain` of
it equal the columns, and `to_plain(typeset(input)) != input`; a row whose loss vanished fails, and
the failure prints the list. A new loss inside the canonical form is a P1 failure. Every row but
`NestedPossessive` is outside the canonical form; that one is the only ambiguity inside it, the
generator for P1 avoids its pattern, and its row also passes P2.

In the table, `default` means `RuleSet::PROSE` with the language's own settings; a named setting
changes only that one setting. The language is ru unless a row says `en`. The `typeset` column is
the exact output, glue included (`roundtrip-known.tsv` holds the same rows with whole inputs).

| Input (non-canonical unless noted)                 | Settings                   | `typeset`                 | `to_plain`                | Code                |
| -------------------------------------------------- | -------------------------- | ------------------------- | ------------------------- | ------------------- |
| `--` between words                                 | `default`                  | `—`                       | `-`                       | `DoubleHyphen`      |
| `т.д.`                                             | `default`                  | `т.⍽д.`                   | `т. д.`                   | `CompactAbbr`       |
| `№5`                                               | `default`                  | `№⍽5`                     | `№ 5`                     | `AttachedSign`      |
| `?...` (ru; also `!...`, `?!...`, section 8.6)     | `default`                  | `?..`                     | `?..`                     | `TerminalEllipsis`  |
| `'x'` (en)                                         | `SingleQuotes::Everywhere` | `“x”`                     | `"x"`                     | `SingleQuote`       |
| `"a 'b' c"` (en)                                   | `default`                  | `“a⍽‘b’ c”`               | `"a "b" c"`               | `NestedSingle`      |
| `1920х1080` (Cyrillic `х`)                         | `default`                  | `1920×1080`               | `1920x1080`               | `CyrillicTimes`     |
| `100%` (ru)                                        | `PercentSpacing::Language` | `100⍽%`                   | `100 %`                   | `PercentInserted`   |
| `100 %` (en)                                       | `PercentSpacing::Attached` | `100%`                    | `100%`                    | `PercentRemoved`    |
| `(C) 2026`, `(с) 2026` (Cyrillic `с`), `Brand(TM)` | `default`                  | `© 2026`, `Brand™`        | `(c) 2026`, `Brand(tm)`   | `LegalMarkSpelling` |
| `1920X1080`                                        | `default`                  | `1920×1080`               | `1920x1080`               | `TimesSpelling`     |
| U+00A0 typed by the author where a rule sets glue  | `default`                  | U+00A0                    | a space                   | `AuthorGlue`        |
| `"a "the dogs' bowls" b"` (en; canonical)          | `default`                  | `“a⍽‘the⍽dogs’ bowls’ b”` | `"a "the dogs" bowls' b"` | `NestedPossessive`  |

### 12.3 Source check

`lint` checks a source that a caller's project requires to stay plain, without building output. It
returns the diagnostics that typesetting would produce (quotes, delimiters, hyphens, mixed scripts,
slots) plus `TypographicChar` for every occurrence of a character that `to_plain` would change
(section 12.1), outside the spans of `protected()` (section 12.5: delimited spans, protected pieces,
slots and frozen fragments) and outside `Markup` and `BlockMarkup` pieces. `lint` renders nothing,
so it never gives `NowrapFallback`; its other diagnostics are those of `report(x, Output::Unicode)`.
Characters of the set that `to_plain` leaves (no enabled rule produces them there: a `©` with
`symbols.copyright` off or outside its contexts, a typed `–` with spaces) are not reported, because
typesetting would not bring them back. So `lint(x)` has a `TypographicChar` exactly when
`to_plain(x) != x`, and `to_plain` is the matching fix: a repository with a "plain sources only"
policy runs `lint` in its checks and `to_plain` to repair.

A generator calls `report_template` instead, which gives the same diagnostics together with the
result, and decides by `Severity` whether to warn or to fail the build. The crate reports levels; it
never decides for the caller.

### 12.4 Diagnostics

| Kind                | Severity | Raised by                                  | When                                                                                                          |
| ------------------- | -------- | ------------------------------------------ | ------------------------------------------------------------------------------------------------------------- |
| `UnmatchedQuote`    | warning  | `quotes.pairs`                             | a straight `"` without a pair; its window keeps straight quotes (section 6.6)                                 |
| `UnmatchedQuote`    | info     | `quotes.pairs`                             | a typed quote glyph of the language without a partner; it stays neutral                                       |
| `CrossingQuotes`    | warning  | `quotes.pairs`                             | pairs would cross each other or a typed pair                                                                  |
| `AmbiguousQuote`    | info     | `quotes.pairs`                             | a straight quote with spaces or letters on both sides, or one of three or more quotes in a series: no role    |
| `UnclosedDelimiter` | warning  | protection (no rule)                       | an opening delimiter without its closing one; it was treated as text                                          |
| `AmbiguousHyphen`   | info     | `dash.spaced-hyphen`, `dash.double-hyphen` | `-` that section 8.4 refuses (numbers, ranges, single letters, foreign words, a flag)                         |
| `MixedScript`       | warning  | classifier (no rule)                       | a word with letters of two of Cyrillic, Latin and Greek (`Мoсква`), unless a dictionary entry (`μs`)          |
| `SlotGlue`          | warning  | classifier (no rule)                       | a `Number` slot glued to letters (`{n}МБ`)                                                                    |
| `SlotGlue`          | info     | classifier (no rule)                       | two slots glued together (`{a}{b}`)                                                                           |
| `NowrapFallback`    | info     | renderer (no rule)                         | a nowrap group split at a `Markup` piece (reported at that piece); the glue at the split written as an entity |
| `TypographicChar`   | info     | `lint` only                                | a character that `to_plain` would change (section 12.3)                                                       |

`Diagnostic` carries its kind, severity, the rule that raised it (`Option<Rule>`), the piece and the
byte range. A diagnostic over several pieces is reported at its first piece, with the range clipped
to that piece (section 3.11), in `lint` as in `report`. `Severity` is ordered (`Info < Warning`). A
diagnostic that starts at a slot is reported at the slot's piece with the empty range `0..0`:
`SlotGlue` for `{n}МБ` and `{a}{b}` at the first slot, for `МБ{n}` at the letters. Diagnostics, in
`report`, `report_template`, `lint` and `lint_template`, are ordered by piece, then byte start, then
byte end, then kind in the order of the `DiagnosticKind` declaration (`TypographicChar` last).
`Diagnostic` implements `Display`:
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
  `Dotted`, `List`, `CompoundHyphens` (the hyphens of a compound word only; the word itself is not
  technical and is an operand like any word, section 8.2), `Unclassified`). This is the main tool
  for "why did no rule fire here". A `Punctuation` fragment is not a protected span. A slot has no
  text, so its `Slot` record holds its piece and the empty range `0..0`; a `ProtectedPiece` record
  covers its whole piece; a frozen core with a slot or protected text inside gives records for its
  text only, around the records of the slot or the protected text (`{host}:{port}` gives `Slot`,
  `Technical(Colon)` for `:`, `Slot`).
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

The table has one line per record of `changes()` (rule id, `piece:range`, the input text, `->`, the
replacement), then one line per record of `protected()` (the word `protected`, the kind,
`piece:range`, the protected text). Each column is as wide as its longest entry plus two spaces,
computed separately for the two parts, which gives the widths of both examples (section 2.9 and
above). A text that is a single invisible character (a space other than U+0020, a control, a
character of general category `Cf` or a code point with the Unicode property
`Default_Ignorable_Code_Point` of Unicode 15.1, assigned (U+034F, the Hangul fillers, the variation
selectors) or reserved (the code points of section 7.6), written out in the code so that the set
does not depend on the Rust version, wider than the format characters of the lexer in section 5.4)
is written as `U+XXXX`; any other text of a change is written in double quotes, with `"`, `\` and
invisible characters escaped (`\"`, `\\`, `\u{a0}`). A protected text is written as it is, with
invisible characters escaped the same way, so that every record stays on one line, and trailing
spaces are not written (a `Slot` record ends after its range). A report without records prints
nothing. Like the `Display` of `Diagnostic`, the text is not part of the contract (section 2.13).

`report` runs the same engine as `typeset`, so its result equals `typeset` byte for byte (a property
test), and applying `changes()` to the input reproduces the result. In stage 5 the comparison with
the reference implementation prints, for every case that differs and is not in the table of expected
differences, the input, our result, the reference result and the `Display` table of `report` (the
example above).
