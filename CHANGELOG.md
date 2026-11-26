# Changelog

All notable changes to this crate are recorded in this file. The format follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and the crate follows
[Semantic Versioning](https://semver.org/spec/v2.0.0.html) as Cargo applies it.

## 0.1.3 - unreleased

In development: changes since 0.1.2, not tagged and not published. Minimum supported Rust version:
1.100.

### Added

- A stable string id for every diagnostic kind (design section 12.4), for callers that keep lists of
  allowed diagnostics in configuration: `DiagnosticKind::id` (`UnmatchedQuote` → `unmatched-quote`,
  `TypographicChar { .. }` → `typographic-char`), `DiagnosticKind::IDS` with the id of every kind,
  `DiagnosticKind::canonical_id`, which reads an id back (`Some("mixed-script")` for
  `"mixed-script"`, `None` for `"Mixed-Script"`), and a `Display` of `DiagnosticKind` that writes
  the id. The ids are `unmatched-quote`, `crossing-quotes`, `ambiguous-quote`, `unclosed-delimiter`,
  `ambiguous-hyphen`, `mixed-script`, `slot-glue`, `nowrap-fallback` and `typographic-char`. An id
  names the kind, not its severity, rule or data, and never changes meaning, as a rule id does not.
  The output and the `Display` of `Diagnostic` do not change.

### Documentation

- The journal of edits as the supported way to distribute a template result over adjacent text
  pieces (design section 2.9): the rustdoc of `Typographer::report_template`, `Report::changes`,
  `Piece::Text` and `Template::texts` and the README say that adjacent pieces merge in `texts()`,
  that `Change::piece` and `Change::range` locate each edit in the input, that the first record of a
  split edit holds the replacement, and show a compiled example that splits a result back over three
  pieces. The replacements are Unicode, so the records rebuild the result of `Output::Unicode`; for
  HTML or an escaped output each rebuilt part is encoded on its own (a compiled example encodes the
  parts as one-piece templates with `RuleSet::EMPTY`), and nowrap groups that span pieces are not
  rebuilt that way. No output changes.
- Which piece owns an edit in a template report (design sections 2.6, 2.9 and 3.11): a replaced or
  removed character belongs to the `Text` piece that holds it; an inserted no-break space belongs to
  the `Text` piece of the token immediately before the insertion point when that token is text, and
  to the `Text` piece of the token immediately after it otherwise, and where markup stands between
  the two characters the point comes after the markup when text follows (`№<b>5` → `№<b>⍽5`, owned
  by the piece on the right) and before it when a slot or a protected piece follows (`№<b>{n}` →
  `№⍽<b>{n}`, owned by the piece on the left); a slot, a protected piece or markup never owns an
  edit, and no rule inserts between a protected piece and a slot. Stated in the rustdoc of
  `Change::piece` and `Typographer::report_template` and pinned by tests. No output changes.

- The opt-in `"interface,+range.digits"` (`RuleSet::INTERFACE.with(Rule::RangeDigits)`) for UI
  strings that hold literal ranges, documented in the rustdoc of `RuleSet::INTERFACE` and the README
  with its known false positives (`123-4567`, `11-26`, `ver 1-2`, `2-3`, `8000-8100` get an en
  dash); `range.digits` stays out of `INTERFACE` (design sections 9.2, 10.8 and 15.3, C68). No
  output changes.

### Changed

- In a language whose dash style glues the space before the dash (`DashStyle::SpacedGlued`: ru-RU
  and declared languages with that style), a dash never starts a line after a slot, whatever its
  kind (design sections 2.6, 8.7 and 15.5, item 18, decided by the maintainer). `nbsp.dash` now
  glues the space before the dash after a slot of kind `Other` and after a fragment that holds one,
  as it already did after a `Number`, `Word` or `Date` slot and after a word; it was the one place
  where a dash after a slot kept a breakable space. Russian, both sets (`⍽` stands for U+00A0):
  `{o} - слово` gave `{o} — слово` and now gives `{o}⍽— слово`; `"{o}" - слово` gave
  `«{o}» — слово`, now `«{o}»⍽— слово`; `{o} - {p}` gave `{o} — {p}`, now `{o}⍽— {p}`. Unchanged:
  the space after the dash (`слово - {o}`), the dialogue dash before an `Other` slot (`- {o} пришёл`
  → `— {o} пришёл`), the ellipsis, the Unicode and plain HTML output of English, whose closed dash
  glues nothing before a dash and keeps the author's space next to a slot of every kind
  (`{x} - word` → `{x} — word`, `{x} — word` stays), and that of `Language::COMMON`, which has no
  dash rules and keeps the space too.
- With a nowrap style (`HtmlStyle::nowrap_class`, `HtmlStyle::nowrap_style`) a slot of kind `Other`
  never belongs to a nowrap group, in every language, `Language::COMMON` included (design section
  11.5): its value stays breakable, the glue next to it is written as an entity, and a group that
  touches the slot ends before it or starts after it. The texts of the result with
  `nowrap_class("nw")`, where `<nw>` and `</nw>` stand for `<span class="nw">` and `</span>` and `⍽`
  for a U+00A0 in the input:
  - ru, glue of `nbsp.dash`: `{o} - слово` gave `["", " &mdash; слово"]` and now gives
    `["", "&nbsp;&mdash; слово"]`; `"{o}" - слово` gave `["&laquo;", "&raquo; &mdash; слово"]`, now
    `["&laquo;", "<nw>&raquo; &mdash;</nw> слово"]`;
  - every language, glue the caller typed next to the slot: `a⍽{o}⍽b` gave `["<nw>a ", " b</nw>"]`,
    now `["a&nbsp;", "&nbsp;b"]`; `{o}⍽{o}` gave `["<nw>", " ", "</nw>"]`, now `["", "&nbsp;", ""]`;
    ru and en `{o}⍽- слово` gave `["<nw>", " &mdash;</nw> слово"]`, now
    `["", "&nbsp;&mdash; слово"]`, and COMMON, which keeps the `-`, `["<nw>", " -</nw> слово"]`, now
    `["", "&nbsp;- слово"]`;
  - every language, a group that ends at the slot with no glue next to it: `a⍽b{o} c⍽d` gave
    `["<nw>a b", "</nw> <nw>c d</nw>"]`, now `["<nw>a b</nw>", " <nw>c d</nw>"]`.

  The output without a nowrap style, and the groups next to a slot of the other kinds, do not
  change.

## 0.1.2 - 2026-11-26

A patch release, tagged on 2026-11-26 and not published yet; publication follows the tag. It holds
everything since 0.1.1. Each change of the output gives an input, the output of 0.1.1 and the output
now (`⍽` stands for U+00A0). Minimum supported Rust version: 1.100.

### Added

- `Piece::BlockMarkup`, markup at a block boundary such as `</p><p>` (design sections 2.6 and 3.3):
  written like `Piece::Markup`, as it is in Unicode and HTML output and escaped in escaped output,
  and it also ends the paragraph, so no rule reads across it and the text after it starts a line. A
  splitter no longer needs one call per block or a `\n\n` before the tag. Russian, Prose, HTML:
  `Текст.`, `BlockMarkup("</p><p>")`, `- Да, - сказал он.` gives
  `Текст.</p><p>&mdash;&nbsp;Да,&nbsp;&mdash; сказал он.`, and `Он сказал "да`,
  `BlockMarkup("</p><p>")`, `Нет" - ответила она.` keeps both quotes straight, as two paragraphs do.
  The output for the existing pieces does not change.

### Changed

- The `Display` of errors, of a `Language` and of the report table escapes every code point with the
  Unicode property `Default_Ignorable_Code_Point` of Unicode 15.1, not only those of general
  category `Cf` and the reserved ones (design sections 2.11 and 12.5): U+034F, the Hangul fillers,
  the Khmer inherent vowels and the variation selectors no longer stay raw.
  `LanguageBuilder::new("a\u{3164}b").build()` reports ``invalid language data for `a\u{3164}b` ``
  where it printed the filler itself. The typeset output does not change.
- `==` of `Language` returns at once for two handles of one table, a clone of a built-in language or
  of one built with `LanguageBuilder` (design section 2.2); other pairs compare the tag and the
  table as before, and the result does not change. Measured in a release build: `a == a.clone()` 1.7
  µs → 4 ns.
- The output stability policy names an exception (design sections 5.6 and 15.1): a later 1.x of
  `unicode-segmentation` with newer Unicode tables can change cluster boundaries of characters
  assigned long before, and with them the output of the built-in sets. Measured between 1.12.0 and
  1.13.3, the version this release requires: U+11A3A ZANABAZAR SQUARE CLUSTER-INITIAL LETTER RA no
  longer joins the character after it, so `"𑨺"` gives `«𑨺»` (Russian, both sets) where 1.12.0 left
  it as written; `𑨺...` gives `𑨺…`. `Extended_Pictographic` no longer holds for 660 old symbols such
  as `★`, so a letter that is `Extended_Pictographic` (`ℹ`, `Ⓜ`, `🅰`, `🅱`, `🅾`, `🅿`), a ZWJ and such
  a symbol are no longer one word: in English, both sets, `ℹ\u{200D}★ - y` gives `ℹ\u{200D}★ — y`
  where 1.12.0 gave `ℹ\u{200D}★—y`, `x - Ⓜ\u{200D}★` gives `x — Ⓜ\u{200D}★` where it gave
  `x—Ⓜ\u{200D}★`, and in both languages `🅰\u{200D}★'s` stays as written where 1.12.0 gave
  `🅰\u{200D}★’s`. Callers who need byte-stable output keep the version of the dependency in their
  lock file. Nothing changes for 0.1.1, which already requires 1.13.3.
- `report`, `report_template`, `lint` and `lint_template` no longer collect the pieces of a record
  into a temporary vector before recording it; the result does not change. Measured in a release
  build, Russian Prose, `Он - "врач", см. host:8080 и т. д. ` repeated 20 times (140 changes, 20
  protected spans): 314 → 154 allocations per `report` call.
- The reason of `Error::InvalidSetting` for `group_separator` no longer reads as if a digit were not
  a digit: `group_separator('5')` gave
  `'5' is not U+00A0, U+202F or a visible character of its own (a digit, a letter, whitespace or a dash)`,
  now
  `'5' is not U+00A0, U+202F or a visible character that is not a digit, a letter, whitespace or a dash`.
  The checked set does not change.

### Fixed

- `nbsp.abbreviations` glues an entry of a declared language in full or not at all (design section
  8.7): a match holds at most six edits, and an entry that needed more had its first six spaces
  glued and the rest left for a second pass. Such an entry is now matched and left as it is, so the
  output is a fixed point; a space the author glued needs no edit, so the rest may still fit. The
  built-in languages have no such entry, and `LanguageBuilder::build` and `Language::from_data`
  accept the same data as before. A declared language, Prose (and the rule alone): `Next` entry
  `a b c d e f word`, `a b c d e f word x`: `a⍽b⍽c⍽d⍽e⍽f⍽word x` (then `a⍽b⍽c⍽d⍽e⍽f⍽word⍽x`) →
  `a b c d e f word x`; `Inner` entry `a. b. c. d. e. f. g. h.`, `a. b. c. d. e. f. g. h.`:
  `a.⍽b.⍽c.⍽d.⍽e.⍽f.⍽g. h.` → `a. b. c. d. e. f. g. h.`, and `a.b.c.d.e.f.g.h.`:
  `a.⍽b.⍽c.⍽d.⍽e.⍽f.⍽g.h.` → `a.b.c.d.e.f.g.h.`; `NextCapitalized` entry of seven parts `a. … g.`,
  `a. b. c. d. e. f. g. Word`: `a.⍽b.⍽c.⍽d.⍽e.⍽f.⍽g. Word` → `a. b. c. d. e. f. g. Word`. Found by
  review.
- A space inserted before the last of 256 tokens that one match takes is written once (design
  section 3.8): the engine wrote it a second time as the insertion after the match. Russian,
  `nbsp.signs` alone and Prose, `№`, 254 × U+200B, `5`: `№…⍽⍽5` (258 characters) → `№…⍽5` (257). The
  same path served a compact abbreviation and a `Markup` piece of a template. Found by review.
- `range.context` reads a `Joined` core before the range, and `nbsp.dates` one before a day, as its
  last word, the fragment it was before the closed style merged it (design sections 4.3, 8.5 and
  8.7), as the rules that read the fragment after them already did. So the typed form gives what the
  plain form gives. Russian, Prose and `ALL`: `поле—с. 1941-45`: `поле—с. 1941-45` →
  `поле—с. 1941–45`; `книга—стр. 1941-45`: `книга—стр.⍽1941-45` → `книга—стр.⍽1941–45` (as
  `книга - стр. 1941-45` gives with `DashStyle::Closed`). English, Prose and `ALL`:
  `field—p. 1941-45`: `field—p.⍽1941-45` → `field—p.⍽1941–45`; Prose, Interface and `ALL`:
  `spring—May 5`: `spring—May 5` → `spring—May⍽5`. Interface has no `range.context`, and its output
  of the range rows does not change. Found by review.
- `nbsp.initials` reads a compound surname with words of two scripts, one of them the language's, as
  a surname, as design section 8.2 reads such a word as a word of each script (design section 8.7,
  C62). Russian, Prose, Interface and `ALL`: `А. Пушкин-Smith`: `А. Пушкин-Smith` →
  `А.⍽Пушкин-Smith`; `Пушкин-Smith А. С.`: `Пушкин-Smith А. С.` → `Пушкин-Smith⍽А.⍽С.`;
  `А. Smith-Пушкин`: `А. Smith-Пушкин` → `А.⍽Smith-Пушкин`. English, the same sets:
  `J. Smith-Петров`: `J. Smith-Петров` → `J.⍽Smith-Петров`; `Петров-Smith J.`: `Петров-Smith J.` →
  `Петров-Smith⍽J.`. Where initials stand between such a surname and a surname before them, the
  forward form takes them, as for any two surnames (design section 15.2, item 22):
  `Иванов И. И. Пушкин-Smith`: `Иванов⍽И.⍽И. Пушкин-Smith` → `Иванов И.⍽И.⍽Пушкин-Smith`. A compound
  word of two scripts is a surname only where every word of it, in either script, is capitalized and
  not in capitals and one is of the language's script alone (C65), so `IT-отдел`, `SMS-рассылка`,
  `XIX-XXм` and `Smith-jones-Петров` take no initials and keep their 0.1.1 output in every language
  (`Иванов⍽И.⍽И. IT-отдел`, `дом XIX-XXм А. С. дом`), and so do acronyms and single letters in
  Russian (`Иванов И. И. МВД-FBI`, `Иванов И. И. СМИ-Smith`, `Иванов И. И. IT-Отдел`,
  `Иванов И. И. Пушкин-J`: `Иванов⍽И.⍽И. …`; `А. ПУШКИН-Smith`, `А. IT-Отдел`, `А. Пушкин-SMITH`
  unglued) and in English (`J. XIX-XXм`, `J. МВД-Office` unglued, `Smith J. CNN-Россия` →
  `Smith⍽J. CNN-Россия`, `Smith J. R. IT-Отдел` → `Smith⍽J.⍽R. IT-Отдел`, `Smith J. R. МВД-Office` →
  `Smith⍽J.⍽R. МВД-Office`, `J. SMITH-Петров`, `J. Smith-ПЕТРОВ` unglued); a surname with no word of
  the language's script stays unglued (`А. Smith-Jones`). Over every compound of up to three words
  from a set of names, acronyms, single letters, lowercase words and numerals of both scripts, in
  fourteen positions around initials, the only rows that differ from 0.1.1 are such double surnames.
  A word of one script is read as in 0.1.1 (`Иванов И. И. СМИ-отдел` → `Иванов И.⍽И.⍽СМИ-отдел`,
  unchanged). Found by review.
- `report_template` gives a `Protected` piece longer than `u32::MAX` bytes its whole length in its
  protected span; the range was cut to `u32::MAX` (design section 3.11). Such a piece is no part of
  the call's text, so the limit of design section 3.4 does not apply to it. Found by review.
- English (en-US) glues the area and volume units of the metre that Russian glues (design section
  7.3): a design choice of the lead (C63, design section 15.3) that extends the families in the
  spirit of the maintainer's extension of the unit lists (design section 15.2, item 35), which does
  not list these units, for the maintainer to overrule: `km²`, `cm²`, `m³`, `cm³` (CLDR 49) and
  `mm²`, `m2`, `m3`, `km2`, `cm2`, `cm3`, `mm2`. English, Prose and Interface:
  `5 km² 5 cm² 5 mm² 5 m³ 5 cm³`: unchanged → `5⍽km² 5⍽cm² 5⍽mm² 5⍽m³ 5⍽cm³`;
  `5 m2 5 m3 5 km2 5 cm2 5 cm3 5 mm2`: unchanged → `5⍽m2 5⍽m3 5⍽km2 5⍽cm2 5⍽cm3 5⍽mm2`; `5-10 m³`:
  `5–10 m³` → `5–10⍽m³` (Prose), `5-10 m³` → `5-10⍽m³` (Interface). A unit is context for
  `range.context` too, Prose: `1.5-2.5 m³`: `1.5-2.5 m³` → `1.5–2.5⍽m³`. `5 km³`, `5 mm³` and
  `5 ft²` stay. Found by review.
- The `Display` of a report writes the trailing spaces of a protected text; it trimmed every
  trailing U+0020 of the line, the text's own included (design section 12.5). `report_template` of
  `Text("a ")`, `Protected("foo  ")`, `Text("b")` printed `protected  ProtectedPiece  1:0..5  foo`
  and prints `… foo  ` now. The typeset output does not change. Found by review.
- The dash rules read a range with its percent sign attached as a percentage range, as they read the
  range with the sign after spaces, whatever its sides and its joint (design sections 3.6 and 8.2):
  a side may be a Roman numeral (`XIX-XX %` and `XIX–10 %` are ranges by check 17) or a decimal
  number, and with `–` the core may be one that no check reads as a range (`1,5–2,5%` is a `List`
  core). Such a core reads as one range, never as its first side, also after a dialogue dash.
  Before, the percent setting that attaches the sign made a core that the dash rules read otherwise,
  and a second pass set the dash after it: Russian, Prose, `PercentSpacing::Attached`,
  `XIX-XX % - 27` gave `XIX-XX% - 27`, then `XIX-XX%⍽— 27`; `стр. 1,5-2,5 % - и` gave
  `стр.⍽1,5–2,5% - и`, then `стр.⍽1,5–2,5%⍽— и`; English, `PercentSpacing::Language`:
  `p. 1.5-2.5 % - x` gave `p.⍽1.5–2.5% - x`, then `p.⍽1.5–2.5% — x`. The output of the first pass on
  such input is unchanged; on input with the sign attached it is now the same as with the sign after
  spaces. Russian, Prose (and Interface where the rule is in it): `XIX-XX% - 27`: `XIX-XX%⍽— 27` →
  `XIX-XX% - 27`; `5 - XIX-XX%`: `5⍽— XIX-XX%` → `5 - XIX-XX%`; `XIX–XX% - 27`: `XIX–XX%⍽— 27` →
  `XIX–XX% - 27`; `1,5–2,5% - и`: `1,5–2,5%⍽— и` → `1,5–2,5% - и`; `5 - 1,5–2,5%`: `5⍽— 1,5–2,5%` →
  `5 - 1,5–2,5%`; `10–1,5% - 27`: `10–1,5%⍽— 27` → `10–1,5% - 27`; `word - I–10%`: `word - I–10%` →
  `word⍽— I–10%`, as `word - I–10 %` gives `word⍽— I–10⍽%`. English, Prose and Interface:
  `I-5% - x`: `I-5% — x` → `I-5% - x`; `x - V-X%`: `x — V-X%` → `x - V-X%`; `I–V% - x`: `I–V% — x` →
  `I–V% - x`; `1.5–2.5% - x`: `1.5–2.5% — x` → `1.5–2.5% - x`; `XIX–XX% -- x` (Prose): `XIX–XX% — x`
  → `XIX–XX% -- x`. A setting that spaces the sign spaces it in such a core with a Roman side too:
  Russian, Prose, `PercentSpacing::Spaced`, `5 - I–10%`: `5 - I–10%` → `5 - I–10⍽%`. The dash before
  such a core follows the same rule for any Roman first side as for a decimal one. Russian, Prose
  and Interface: `слово - XIX–10%`: `слово - XIX–10%` → `слово⍽— XIX–10%`; `word - V–10%`:
  `word - V–10%` → `word⍽— V–10%`; English, the same sets: `word - XIX–10%`: `word - XIX–10%` →
  `word — XIX–10%`. `x - XIX–10%` and `- XIX–10%` stay as written, as in 0.1.1, because a single
  letter on the left and the start of the line get no dash. Next to a word outside the language's
  script or a technical fragment such a core is neither, as the `Range` core of the spaced form is
  not (design section 8.4), so it gets the dash that the spaced form gets, also with a hyphen joint
  (older than 0.1.2). Russian, Prose and Interface: `XIX-XX% - word`: `XIX-XX% - word` →
  `XIX-XX%⍽— word`, as `XIX-XX % - word` gives `XIX-XX⍽%⍽— word`; `XIX–XX% - word`: `XIX–XX% - word`
  → `XIX–XX%⍽— word`; `word - 1,5–2,5%`: `word - 1,5–2,5%` → `word⍽— 1,5–2,5%`; `word - V–X%`:
  `word - V–X%` → `word⍽— V–X%`; `word - 5-10%`: `word - 5-10%` → `word⍽— 5-10%`, as `word - 5-10 %`
  gives `word⍽— 5–10⍽%` (Prose). English, Prose and Interface: `/usr/bin - 5-10%`:
  `/usr/bin - 5-10%` → `/usr/bin — 5-10%`; `host:8080 - 1.5–2.5%`: `host:8080 - 1.5–2.5%` →
  `host:8080 — 1.5–2.5%`. Found by the grammar generator of design section 13.5 and by review.
- A trailing possessive inside a pair that `’` closes stays straight where a second pass or
  `to_plain` would read it as the closer of the pair and change what stands between them (design
  section 6.6): a straight `'` opener that stayed unpaired stands after it, and a straight `'` or a
  prime that could close that opener follows; or the closer follows `s` or `S` and such an opener or
  a quote of a pair stands between them. So the first pass is a fixed point and P2 holds on these
  inputs. English, Prose, Interface and `ALL`, `NestedOnly` and `Everywhere` (with
  `SingleQuotes::Never` these rows keep the output of 0.1.1): `""s' 'x."'"`: `“‘s’ 'x.’'”` (then
  `“‘s’ ‘x.’’”` on a second pass) → `“‘s' 'x.’'”`; `“"s' '."'”`: `“‘s’ '.’'”` → `“‘s' '.’'”`;
  `"a "dogs' 's" b"`: `“a⍽‘dogs’ 's’ b”` → `“a⍽‘dogs' 's’ b”`; `"a "DOGS' 'S" b"`:
  `“a⍽‘DOGS’ 'S’ b”` → `“a⍽‘DOGS' 'S’ b”`; `"a "dogs' 'x' bowls" b"`: `“a⍽‘dogs’ “x” bowls’ b”` →
  `“a⍽‘dogs' “x” bowls’ b”`. A declared language whose primary pair is `‘’` (en-GB), Prose,
  Interface and `ALL`, `SingleQuotes::NestedOnly` and `Everywhere` (with `SingleQuotes::Never` these
  rows keep the output of 0.1.1): `"S' 's"`: `‘S’ 's’` → `‘S' 's’`; `"the dogs' 'x' bowls"`:
  `‘the⍽dogs’ “x” bowls’` → `‘the⍽dogs' “x” bowls’`; `"the dogs' 'bowls" today`:
  `‘the⍽dogs’ 'bowls’ today` → `‘the⍽dogs' 'bowls’ today`; `"S'. a(\n'3,".'`: `‘S’. a(\n'3,’.'` →
  `‘S'. a(\n'3,’.'`; `it) - 'cause' - 'cause -- "'cause) DOGS' - 'house' - it, - oz.) '90s" a`:
  `… ‘’cause) DOGS’ — “house” — …` → `… ‘’cause) DOGS' — “house” — …`. Typed text, English and
  en-GB, the same sets, `NestedOnly` and `Everywhere`: `‘bowls' 'x S’ a`: `‘bowls’ 'x S’ a` →
  `‘bowls' 'x S’ a`; `to_plain` reads such a `’` back as itself, and `lint` no longer reports it
  (en-GB, `NestedOnly` and `Everywhere`: `“S’ 'dogs" -the` gave `"S' 'dogs" -the` and a
  `TypographicChar` for each of `“` and `’`, now `"S’ 'dogs" -the` and one for the `“` only;
  `"S’ 'dogs" -the` gave `TypographicChar` and now has no report). Where nothing could pair with the
  opener, the possessive is placed as before (`"a "dogs' 'x" b"` gives `“a⍽‘dogs’ 'x’ b”`, en-GB
  `"s' 'x"` gives `‘s’ 'x’`), and with only text between the possessive and the closer nothing
  changes (`“He said ‘the⍽dogs’ bowls’”`). In en-GB, where `’` closes the primary pair, a typed `’`
  inside a straight pair keeps the possessive before the pair straight too, for `to_plain`; Prose,
  Interface and `ALL`: `‘S' "x’" s’`: `‘S’ “x’” s’` → `‘S' “x’” s’` (every `SingleQuotes` mode);
  `‘"s'" s' 's’`: `‘“s’” s’ 's’` → `‘“s’” s' 's’` (`NestedOnly` and `Everywhere`). Where a
  possessive placed inside the same pair, or a typed `’` that a straight pair covers on the first
  pass, would close the pair early on a second pass and move its closer, the quote phase settles the
  window (below) and gives the output of 0.1.1, which is a fixed point: English `‘S' ‘s'’ s’` gives
  `‘S’ ‘s’’ s’`, `‘S' "x’" s’` gives `‘S’ ‘x’’ s’`, and en-GB `"a’ s' 's"` gives `‘a’ s’ 's’`, as in
  0.1.1.
- A typed `’` closes the `‘` of its pair when only signs, or nothing, stand between them and the
  neighbors allow the sign-only pair (design section 6.3), as the plain `""` that a first pass
  typesets as `‘’` does; other rules read the same closing role. In a declared language whose
  primary pair is `‘’` (en-GB), Prose: `'a ""-3'` gives `'a⍽‘’-3'` as before, and the second pass no
  longer gives `'a⍽‘’-3′`. Typed `'a ‘’-3'`, and likewise `'a ‘’ 3'`: in en-GB, Prose and `ALL`,
  `NestedOnly`, `'a⍽‘’-3′` → `'a⍽‘’-3'`; in English with `SingleQuotes::Everywhere` the outer pair
  now pairs: `'a⍽‘’-3′` (Prose, `ALL`) and `'a⍽‘’-3'` (Interface) → `“a⍽‘’-3”`. English, both sets:
  `"a ‘’ b"`: `"a⍽‘’ b"` → `“a⍽‘’ b”`; `"a ‘-’ b"`: `"a⍽‘-’ b"` → `“a⍽‘-’ b”`; `"a ‘’, b"`:
  `"a⍽‘’, b"` → `“a⍽‘’, b”`. The closed `‘` no longer counts as a neutral opener, so a trailing
  possessive after it is placed (en-GB, Prose, Interface and `ALL`: `‘’\n”bowls'`: `‘’\n”bowls'` →
  `‘’\n”bowls’`; English gives `‘’\n”bowls'` as in 0.1.1: a typed `‘’` is no sign-only pair of the
  first level there, see below), and the rules that read the role of a quote read it too (English
  and en-GB, the same sets: `)(\n ‘’ - ')“` gives `‘’ —` where it gave `‘’ -`). Wherever the dash
  style glues the space before a dash (`DashStyle::SpacedGlued`, the style of Russian data), the
  dash after a sign-only pair that the first pass writes as `‘’` is glued, as after `«»` or `“”`. In
  a declared language with Russian data whose primary pair is `‘’`: Prose, Interface and `ALL`,
  every `SingleQuotes` mode, `Он сказал "" - и ушёл`: `Он⍽сказал ‘’ — и⍽ушёл` →
  `Он⍽сказал ‘’⍽— и⍽ушёл` (Interface: `Он сказал ‘’⍽— и⍽ушёл`); `"" - ""`: `‘’ — ‘’` → `‘’⍽— ‘’`;
  `Он сказал "-" - и ушёл`: `‘-’ —` → `‘-’⍽—`. en-GB with `DashStyle::SpacedGlued`, the same sets
  and modes: `"" - x`: `‘’ — x` → `‘’⍽— x`; `а "" - б`: `а ‘’ — б` → `а ‘’⍽— б`; `"" - ""`:
  `‘’ — ‘’` → `‘’⍽— ‘’`. English with `DashStyle::SpacedGlued`, Prose and `ALL`, `NestedOnly` and
  `Everywhere`: `"5 '' -- 100"`: `“5 ‘’ — 100”` → `“5 ‘’⍽— 100”`. Such a pair is no longer reported:
  en-GB `x ‘’ y` gave `UnmatchedQuote` (info), English `"a ‘’ b"` gave `CrossingQuotes` (warning).
  The quote phase reads such a pair only where it stands at a level whose opening glyph is its `‘`,
  the level at which a first pass writes a plain sign-only pair as `‘’` (design section 6.3);
  elsewhere the window is decided as in 0.1.1, since a typed `‘’` at the level of `“”` is no output
  of plain text and its plain form would not typeset back. So the typed `'a ‘’-3'` and `'a ‘’ 3'`
  keep the output of 0.1.1 in English with `NestedOnly` (`'a⍽‘’-3′`) and in en-GB with `Everywhere`
  (`'a⍽‘’-3′` in Prose and `ALL`, `'a⍽‘’-3'` in Interface).
- The quote phase settles its result (design section 6.6): it runs again over the paragraph with its
  decisions applied, as a second call would read it, and takes what that run adds, at most three
  runs; a paragraph that the third run still changes, or where a quote that keeps its text would get
  another role in a place that another rule reads (after a space, before a space and a dash, after
  an ellipsis), keeps its straight quotes with `CrossingQuotes` (warning). So `typeset` is
  idempotent on the windows where its first pass was not: sign-only pairs that share a quote, a
  typed `‘` that the first pass left neutral and a second pass paired, a possessive or a covered `’`
  that closes a pair early on a second pass, and a typed `’` after a straight quote and signs that a
  second pass reads as the closer of a sign-only pair. P2 holds again on the en-GB inputs where
  0.1.1 held it and the first pass of 0.1.2 broke it (`"a’ s' 's"`, `‘S' ‘s'’ s’`,
  `She said "the boys’ dogs' 'bowls" were empty.`, with `NestedOnly`), and on the sign-only chains
  in English (`"x ''-'' y"`; `""-''` with `SingleQuotes::Everywhere`). Each row gives the output of
  0.1.1, which a second pass changed, and the output now. English, Prose, Interface and `ALL`,
  `NestedOnly` and `Everywhere`: `"x ''-'' y"`: `“x ''-‘’ y”` → `“x ‘’-‘’ y”`; `"""x\n''-''\ne"""`:
  `“‘“x\n''-‘’\ne”’”` → `“‘“x\n‘’-‘’\ne”’”`; Russian, the same: `"x ''-'' y"`: `«x ''-„“ y»` →
  `«x „“-„“ y»`. With `SingleQuotes::Everywhere`: English `''-''`: `''-“”` → `“”-“”`, `'('')'`:
  `'('“)”` → `“(”“)”`; Russian `''—''`: `''—«»` → `«»—«»`. English, every `SingleQuotes` mode:
  `He said ‘the dogs' "5"’`: `He said ‘the⍽dogs' “5”’` → `He said ‘the⍽dogs’ “5”’`; `‘s' "0"’`:
  `‘s' “0”’` → `‘s’ “0”’`; `‘"0" S'’`: `‘“0” S'’` → `‘“0” S’’`; `‘"8"’ s'`: `‘“8”’ s'` → `‘“8”’ s’`.
  English, `NestedOnly`: `x ‘a 'b' "2"’ y`: `x ‘a⍽'b' “2”’ y` → `x ‘a⍽‘b’ “2”’ y`. en-GB (primary
  pair `‘’`), every `SingleQuotes` mode: `‘ "dogs'’" x`: `‘ ‘dogs'’’ x` → `‘ ‘dogs’’’ x`;
  `‘"s'»’".`: `‘‘s'»’’.` → `‘‘s’»’’.`; `NestedOnly`: `‘ '' "2"’`: `‘ '' ‘2’’` → `‘ “” ‘2’’`. Where
  the result would give a quote that the dash rule reads another role than the input gave it, the
  quotes stay straight. 0.1.1 gave a fixed point on the following inputs, since it read the `’`
  after `‘-` as no closer; since the typed sign-only pair above a second pass reads it as the closer
  of `‘-’` and sets the dash after it, so the paragraph keeps its straight quotes. English, Prose
  and Interface, `NestedOnly` and `Everywhere`: `"a '-’ - x' c"`: `“a⍽‘-’ - x’ c”` →
  `"a⍽'-’ - x' c"`; en-GB, every set and `SingleQuotes` mode: `"-’ - x"`: `‘-’ - x’` → `"-’ - x"`,
  `s Paul "(’ -- St.)"`: `s Paul ‘(’ -- St.)’` → `s Paul "(’ -- St.)"` (design section 10.8). An
  opening role that the result drops after a space that follows anything but a dash does not count:
  a glue that the first pass set stays, so these windows keep the placed quotes of 0.1.1, which are
  fixed points. Russian, English and en-GB, Prose and Interface, `Everywhere`: `a ''.-'` gives
  `a '«.-»`, `a '“.-”` and `a '‘.-’`, `a '.-''` gives `a '.-«»`, `a '.-“”` and `a '.-‘’`, and
  `. ''.-'` gives `. '«.-»`, `. '“.-”` and `. '‘.-’`, unchanged from 0.1.1. After a dash, which the
  dash rule matched by that role and would not match again on a second pass (design section 4.3,
  I4), the window keeps its straight quotes (design section 10.8). The same sets: `- ''.-'`:
  `—⍽'«.-»` → `—⍽''.-'` (Russian Prose), `- '«.-»` → `- ''.-'` (Russian Interface), `- '“.-”` →
  `- ''.-'` (English), `- '‘.-’` → `- ''.-'` (en-GB); `- '.-''`: `—⍽'.-«»` → `—⍽'.-''` (Russian
  Prose), `- '.-“”` → `- '.-''` (English); `word - ''.-'`: `word — '“.-”` → `word — ''.-'`
  (English), `word — '«.-»` → `word⍽— ''.-'` (Russian). The settling runs the phase again on a
  paragraph whose quotes it changes; measured in a release build, English,
  `SingleQuotes::Everywhere`, one paragraph of a unit repeated to 1 MiB, median of three runs,
  nanoseconds per byte in 0.1.1 → now: `"a" `, Interface 213 → 271 and Prose 224 → 278; the
  quote-dense worst case, a chain of sign-only pairs that the phase runs again over, `''-'' `,
  Interface 239 → 413 and Prose 251 → 425, and `"x ''-'' y" `, Prose 239 → 392. The cost stays
  linear in the length of the paragraph.
- `to_plain`, `to_plain_template`, `lint` and `lint_template` are linear in the length of the text
  on long runs of quote characters in one paragraph as well (design section 12.1): the neighbors of
  a series of quotes, the open pairs and the later closer of a `’` after `s` were searched again for
  every quote. Measured in a release build, one paragraph of 96 KB: `’` repeated (Russian Prose) 5.7
  s → 17 ms; `«„` repeated, `x`, `“»` repeated (Russian Prose) 4.4 s → 43 ms; `“‘a dogs’ ` repeated,
  `b`, `’”` repeated (English Prose) 0.6 s → 31 ms. The results are the same.
- `range.context` reads a compact abbreviation after a range as its first part, the fragment that
  `nbsp.abbreviations` splits off (design sections 4.3 and 8.5), so the first pass is a fixed point
  and P2 holds: the whole `т.е.` was no unit, the split `т.` of the output is the unit `т`, and a
  second pass placed the range dash. Russian, Prose and `ALL`: `1,5-2,5 т.е.`: `1,5-2,5⍽т.⍽е.` (then
  `1,5–2,5⍽т.⍽е.` on a second pass) → `1,5–2,5⍽т.⍽е.`, as `1,5-2,5 т. е.` gives;
  `Ждать 1,5-2,5 т.е. и т.д.`: `Ждать 1,5-2,5⍽т.⍽е. и⍽т.⍽д.` → `Ждать 1,5–2,5⍽т.⍽е. и⍽т.⍽д.`.
  Interface has no `range.context` and does not change. Found by the grammar generator of design
  section 13.5.
- `dash.dialogue` and `range.context` read a `Joined` core after the dash or the range, the form
  `word—word` that the closed style writes, as its first word, as the other rules that read a
  neighbor do (design sections 4.3, 8.4 and 8.5). So a second pass matches what the first pass
  matched, and the typed form gives what the plain form gives. Russian, Prose: `- река—ООО`:
  `- река—ООО` → `—⍽река—ООО`; `— река—ООО`: `— река—ООО` → `—⍽река—ООО`; `1,5-2,5 ккал—бы`:
  `1,5-2,5⍽ккал—бы` → `1,5–2,5⍽ккал—бы`. English, Prose: `1.5-2.5 mW—rock`: `1.5-2.5⍽mW—rock` →
  `1.5–2.5⍽mW—rock`. Only a `Joined` core reads so: a label after the dash or the range reads as
  itself, a word after a dialogue dash, not as its sign (`- №5 дом`, `- §5 текст` and `- ¶IV дом`
  give `—⍽№⍽5 дом`, `—⍽§⍽5 текст` and `—⍽¶⍽IV дом`, as in 0.1.1), and plain input with spaces around
  the dash does not change. Found by the grammar generator of design section 13.5.

## 0.1.1 - unreleased

The first version, prepared and not published; the date is set when the version is tagged. Version
0.1.0 was prepared but never published. Minimum supported Rust version: 1.100.

### Added

- `Typographer`, built once with a `Language` and a `RuleSet` (`Typographer::new`, a `const fn`
  usable in a `static`, or `Typographer::builder` with settings) and then only applied. It is
  immutable and `Send + Sync`; applying it never fails and never panics, and an unchanged string is
  returned borrowed, without allocation after the first call on a thread for an input up to about 50
  KiB. `Scratch` is reusable working memory for callers that manage it themselves (`typeset_with`,
  `typeset_template_with`); `typeset_into` writes into a buffer.
- Three inputs: a whole string (`typeset`), a message template of `Piece`s (`Text`, `Slot` with a
  `SlotKind` of number, word, date or other, `Protected`, `Markup`, which is inline markup and
  transparent for rules) whose slots keep their place in the resulting `Template`
  (`typeset_template`), and a `Number` (`format_number`, `format_range`).
- 27 rules with permanent ids, applied in a fixed order of phases whatever the order of enabling:
  - quotes by nesting level, apostrophes and primes (`quotes.apostrophe`, `quotes.pairs`,
    `quotes.primes`); an unpaired or crossing quote is left as it is and reported;
  - dashes (`dash.dialogue`, `dash.double-hyphen`, `dash.spaced-hyphen`), with `AmbiguousHyphen`
    where a hyphen is not clearly a dash;
  - ranges (`range.placeholders`, `range.digits`, `range.context`, `range.time`);
  - ellipses (`ellipsis.dots`, `ellipsis.inner`);
  - no-break spaces (`nbsp.abbreviations`, `nbsp.initials`, `nbsp.dates`, `nbsp.units`,
    `nbsp.signs`, `nbsp.phone`, `nbsp.short-words`, `nbsp.short-length`, `nbsp.particles`,
    `nbsp.dash`);
  - numbers in text (`number.minus`, `number.times`);
  - symbols (`symbols.copyright`, `symbols.arrows`, `symbols.math`).
- Rule sets `RuleSet::PROSE` (all 27 rules) and `RuleSet::INTERFACE` (13 rules that cannot alter a
  technical token), `RuleSet::ALL` (every rule of the running version, so its output, unlike that of
  the two sets, may change with `cargo update`), `RuleSet::EMPTY`, sets of one's own, and parsing of
  rule sets and rule ids from strings (`"interface,-nbsp.short-words,+symbols.copyright"`).
- Settings of the builder: quote pairs, single quotes (`SingleQuotes`), dash style, percent spacing
  (`PercentSpacing`), units, short words and particles added or removed, short word length, short
  words of any script, narrow glue (`NarrowGlue`), minimum grouping digits, group separator, minus
  sign, backtick protection and protection delimiters of one's own. `build` validates every setting.
- Languages as data: `Language::RU_RU` and `Language::EN_US` (checked against CLDR 49),
  `Language::COMMON` for the rules that need no language data, `Language::from_tag` with an explicit
  error for a language without data, `LanguageData` with `const fn` setters for a language declared
  as a `const`, accepted by `Language::from_data` (validated) and `Language::from_static` (for a
  `static` typographer), and `LanguageBuilder` for a language built at run time; validation also
  requires the range dash to be `–` or `—`, the decimal separator to be a visible character of its
  own that is not a letter, a digit, whitespace, a quote character or a character with the Unicode
  `Dash` property, the group separator to be U+00A0, U+202F or a visible character of its own that
  is not a letter, a digit, whitespace or a dash, the not-a-number and infinity symbols to be
  visible text without digits, invisible characters or a mark at an end that joins the text next to
  them, signs to be punctuation characters other than `%` and `‰` and different from every unit, and
  each ellipsis form to be its own signs followed by full stops and `…`. The `Display` of a
  `Language` escapes invisible characters of a tag that `from_static` did not validate.
- Protection: backtick spans and fenced blocks, delimiters of one's own, `Piece::Protected`, and the
  technical fragments that the classifier recognizes by their shape (URLs, e-mail addresses and
  mentions, flags, paths, `host:port`, operators, UUIDs, hex numbers, ISO dates, versions, dotted
  names, lists, identifiers); slots and protected text are never edited.
- Number formatting by the language: decimal separator, grouping from `min_grouping_digits`, group
  separator, minus sign U+2212, range pattern, and the symbols of not-a-number and infinity, for
  every integer type, `f32`, `f64`, `Number::decimal` (digits as written) and `Number::fixed`;
  integers compare by value whatever their type. The lazy displays `NumberDisplay` and
  `RangeDisplay` allocate nothing.
- Output kinds, given with every call: `Output::Unicode`; `Output::Html` with named, decimal or
  hexadecimal references or characters (`HtmlStyle`) and no-break groups as entities or as nowrap
  elements (`nowrap_class`, `nowrap_style`); `Output::Escaped` with the body of a string literal for
  JavaScript (`JS`, `JS_HEX`, `JS_CODE_POINT`), JSON, Rust and CSS (`EscapeStyle`), the typographic
  set or with `non_ascii()` every non-ASCII character escaped; `$` before `{` and at the end of a
  text is escaped in the JavaScript styles, and the bidi controls in every escaped style.
- Explanation: `report` and `report_template` return the result with every edit (`Change`), the
  protected spans (`ProtectedSpan`, `ProtectedKind`, `TechKind`) and the diagnostics (`Diagnostic`,
  `DiagnosticKind`, `Severity`); the `Display` of `Report` prints a table of the edits and the
  protected spans.
- Source check: `lint` and `lint_template` return the diagnostics of a source that must stay plain,
  with a `TypographicChar` for every character that `to_plain` would change.
- Reverse transformation: `to_plain` and `to_plain_template` map typeset Unicode text back to the
  canonical plain form, undoing exactly the enabled rules; the result is stable under a second call,
  and typesetting it gives the typeset text back except for the losses listed in the repository's
  `roundtrip-known.tsv`.
- Examples: `numbers` (number formatting), `per_request` (typesetting at run time) and
  `fluent_generator` (a build-time generator over messages mapped from Fluent syntax to pieces, with
  `report_template` and `Output::Escaped(EscapeStyle::RUST)`).

### Changed before the first publication

The output of the built-in sets changed after the text above was prepared and before the first
publication; each change below gives an input, the output before and the output after (`⍽` stands
for U+00A0).

- A rule that reads a fragment next to it reads a `Joined` core of the closed style as the word at
  its end and a label (`№5`) as its numeral or its sign (design section 4.3, I4), so the first pass
  is a fixed point and `to_plain` gives the input back:
  - en `Moscow - a, - b`: `Moscow—a, - b`, then `Moscow—a, — b` on a second pass → `Moscow—a, - b`;
    en `A - (b - cc)`: `A - (b—cc)`, then `A — (b—cc)` → `A - (b—cc)`;
  - en `5 in - yes`: `5 in—yes`, then `5⍽in—yes` → `5 in—yes`; typed `5 in—yes`: `5⍽in—yes` →
    `5 in—yes`; ru `12 с—у`: `12⍽с—у` → `12 с—у`;
  - en `Hello—a, - b`: `Hello—a, — b` → `Hello—a, - b`;
  - ru `№5 ли ты`: `№⍽5⍽ли ты` (Prose; `№⍽5⍽ли⍽ты` on a second pass) → `№⍽5 ли⍽ты`, Interface
    `№⍽5⍽ли ты` → `№⍽5 ли ты`; ru `§5 же ты`: `§⍽5⍽же ты` → `§⍽5 же⍽ты`;
  - ru `№5 - x`: `№⍽5⍽— x` → `№⍽5 - x`; ru `X - №V`: `X - №⍽V` → `X⍽— №⍽V`;
  - ru `т. н.э.`: `т. н.⍽э.` (`т.⍽н.⍽э.` on a second pass) → `т.⍽н.э.`;
  - ru `а—г. Пушкин`: `а—г. Пушкин` → `а—г.⍽Пушкин`; ru `дом—и т. д. и`: `дом—и т.⍽д. и` →
    `дом—и⍽т.⍽д. и`; en `x—St. "a"`: `x—St. “a”` → `x—St.⍽“a”` (an abbreviation may start at a part
    of a `Joined` core).
- `ellipsis.dots` no longer reads three full stops right after `/` or `\` as an ellipsis (design
  section 8.6), in every language and both sets: `go test ./...`: `go test ./…` → `go test ./...`;
  `src/...`: `src/…` → `src/...`; `C:\dir\... now`: `C:\dir\… now` → `C:\dir\... now`;
  `https://a.b/c...` still gives `https://a.b/c…`.
- A run of spaces cut by a `Markup` piece or by format characters is no longer read as a single
  pattern space by `number.minus`, `symbols.arrows` and `symbols.math` (design section 8.2), in
  Prose: the template `слово `, `<b>`, ` -15`: `слово <b> −15` → `слово <b> -15`; `a `, `<b>`,
  ` -> b`: `a <b> → b` → `a <b> -> b`; `x `, `<b>`, ` +- y`: `x <b> ± y` → `x <b> +- y`; a plain
  string `слово \u{200b} -15`: `слово \u{200b} −15` → `слово \u{200b} -15` (the same with
  `\u{2060}`); `a \u{2066} -> b`: `a \u{2066} → b` → `a \u{2066} -> b`; `x \u{200b} +- y`:
  `x \u{200b} ± y` → `x \u{200b} +- y`.
- U+061C, U+180E, U+206A to U+206F and U+FFF9 to U+FFFB are format characters of the lexer (design
  section 5.4), as U+200E is, instead of freezing their word as `Technical(Unclassified)`: ru Prose
  `в\u{61c} лесу`: unchanged → `в\u{61c}⍽лесу`.
- The first part of a Russian abbreviation of several parts is relaxed even when it is one letter
  with a full stop (design section 7.1), so a capitalized `Т. е.` at a sentence start is the
  abbreviation: `Т. е. так`: unchanged → `Т.⍽е. так`; `Т.е. так`: unchanged → `Т.⍽е. так`;
  `Т. д. так`: `Т. д.⍽так` → `Т.⍽д. так`; `Буква Т. е. не`: `Буква⍽Т. е. не` → `Буква⍽Т.⍽е. не`.
  `nbsp.initials` reads a compact abbreviation after a surname as its first part, the fragment
  `nbsp.abbreviations` splits off (design section 8.7), so the first pass is a fixed point:
  `Москва Т.е.`: unchanged → `Москва⍽Т.⍽е.` (with `nbsp.initials` alone `Москва⍽Т.е.`);
  `Пушкин А. Т.е.`: `Пушкин⍽А. Т.е.` → `Пушкин⍽А.⍽Т.⍽е.`.
- An English elision of the list that a straight `'` closes at once waits for pairing (design
  section 6.2), as `'n'` does, in both sets: `Choose the 'round' button`:
  `Choose the ’round' button` → `Choose the⍽'round' button`; `"Say 'twas' or 'tis'"`:
  `“Say ’twas’ or ’tis’”` → `“Say ‘twas’ or⍽‘tis’”`; `'tis'`: `’tis’` → `'tis'`.
- The English trailing possessive goes after a capital `S` too (design section 6.6), in both sets:
  `DOGS' and JONES' BOOK`: `DOGS' and⍽JONES' BOOK` → `DOGS’ and⍽JONES’ BOOK`; `to_plain` reads it
  back.
- An apostrophe between two letters inside a frozen core no longer keeps the quotes around the
  fragment straight (design section 6.1), in both sets: en `Say "it's:fine" now`: unchanged →
  `Say “it's:fine” now`; ru `Файл "it's:fine" тут`: unchanged → `Файл «it's:fine» тут`.
- `to_plain` reads the primary quote pair as `'` where double quotes would not pair again (design
  section 12.1), so typesetting its result gives the text back: ru `«„«6»“»`: `"""6"""` → `""'6'""`;
  ru `«a „«9» е“»`: `"a ""9" е""` → `"a "'9' е""`; with `SingleQuotes::Everywhere`, in both sets, en
  `“3" 0”`: `"3" 0"` → `'3" 0'`, ru `«5" 5»`: `"5" 5"` → `'5" 5'`.
- `to_plain` keeps a `’` read as an apostrophe where its `'` would pair as a quote (design section
  12.1), so typesetting its result gives the text back, in both sets: en `“a⍽‘dogs’ bowl ’twas’ b”`:
  `"a "dogs" bowl 'twas' b"` → `"a "dogs" bowl ’twas' b"`; en `“a⍽‘s’ ’twas’ b”`: `"a "s" 'twas' b"`
  → `"a "s" ’twas' b"`. `lint` no longer reports such a `’`.
- `Output::Escaped` escapes the bidi controls U+061C, U+200E, U+200F, U+202A to U+202E and U+2066 to
  U+2069 in every style (design section 11.6), so a `RUST` body compiles: `a\u{202E}b` in `RUST`:
  the raw control → `a\u{202e}b`; in `JS`: the raw control → `a\u202Eb`.
- The built-in unit lists are extended (design sections 7.2 and 7.3): ru `ГВт`, `Дж`, `Ом`, `Па`,
  `Тбит`, `кДж`, `кОм`, `кбит`, `кбит/с`, `ккал`, `км²`, `м³`, `мВт`, `мм²`, `см²`, `см³`; en `GW`,
  `kJ`, `kPa`, `mW`, `a.m.`, `p.m.`. `nbsp.units` glues them in both sets: ru `5 ГВт`: unchanged →
  `5⍽ГВт`; ru `10 м³`: unchanged → `10⍽м³`; ru `5-10 м³`: `5–10 м³` → `5–10⍽м³` (Prose), unchanged →
  `5-10⍽м³` (Interface); ru `5 м³ 5 см³ 5 км² 5 см² 5 мм²`: `5 м³⍽5 см³⍽5 км²⍽5 см²⍽5 мм²` →
  `5⍽м³⍽5⍽см³⍽5⍽км²⍽5⍽см²⍽5⍽мм²` (Prose), unchanged → `5⍽м³ 5⍽см³ 5⍽км² 5⍽см² 5⍽мм²` (Interface); ru
  `5 кОм и 10 кОм`: `5 кОм и⍽10 кОм` → `5⍽кОм и⍽10⍽кОм`; en `9 a.m.`: unchanged → `9⍽a.m.`; en
  `5 p.m. and 5 a.m.`: `5 p.m. and⍽5 a.m.` → `5⍽p.m. and⍽5⍽a.m.`; en `5 kPa`: unchanged → `5⍽kPa`;
  ru `Скорость 5 кбит/с - это мало`: `Скорость 5 кбит/с⍽— это мало` → `Скорость 5⍽кбит/с⍽— это мало`
  (the dash is kept, by the change below). `at 5:30 p.m. today` and `5 P.M.` are not changed by
  this.
- A `Dictionary` core that is not a word and has no full stop inside (`км/ч`, `кВт⋅ч`, `°C`, `₽`,
  `г-жа`) is an operand of the dash rules and `nbsp.dash`, read as the word, compound word or
  technical fragment it would be without the list (design section 8.2), so an entry of the units or
  the abbreviations changes no dash. In both sets: ru `Скорость 60 км/ч - это много`:
  `Скорость 60⍽км/ч - это много` → `Скорость 60⍽км/ч⍽— это много`; ru `Было 5 °C - это мало`:
  `Было 5⍽°C - это мало` → `Было 5⍽°C⍽— это мало`; en `Speed 60 km/h - fast`: `Speed 60⍽km/h - fast`
  → `Speed 60⍽km/h — fast`. `км/ч - м/с` is not changed and now reports `AmbiguousHyphen`, as two
  technical fragments do; `See - e.g. this` is not changed. An entry of letters or digits joined by
  single hyphens reads as a compound word, by the script of its letters, in both sets: ru
  `г-жа - г-н`: unchanged → `г-жа⍽— г-н`, as `кто-то - что-то` gives `кто-то⍽— что-то`; ru
  `Пришла г-жа - и ушла`: `Пришла г-жа - и⍽ушла` → `Пришла г-жа⍽— и⍽ушла`; ru `г-н — это`: unchanged
  → `г-н⍽— это`; ru `Это - г-н Иванов`: `Это - г-н⍽Иванов` → `Это⍽— г-н⍽Иванов`; ru `г-н - cat`:
  unchanged → `г-н⍽— cat`. `dash.dialogue` reads such an entry, and one of letters and digits, as a
  word, in Prose: ru `- г-н Иванов`: `- г-н⍽Иванов` → `—⍽г-н⍽Иванов`; ru `- м2 это`: unchanged →
  `—⍽м2 это`.
