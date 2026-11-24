# Changelog

All notable changes to this crate are recorded in this file. The format follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and the crate follows
[Semantic Versioning](https://semver.org/spec/v2.0.0.html) as Cargo applies it.

## Unreleased

These entries are for version 0.1.2; the heading takes the version when the release sets it in
`Cargo.toml`. Each change of the output gives an input, the output of 0.1.1 and the output now (`⍽`
stands for U+00A0).

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

### Fixed

- A trailing possessive inside a pair that `’` closes stays straight where a second pass or
  `to_plain` would read it as the closer of the pair and change what stands between them (design
  section 6.6): a straight `'` opener that stayed unpaired stands after it, and a straight `'` or a
  prime that could close that opener follows; or the closer follows `s` or `S` and such an opener or
  a quote of a pair stands between them. So the first pass is a fixed point and P2 holds. English,
  Prose, Interface and `ALL`: `""s' 'x."'"`: `“‘s’ 'x.’'”` (then `“‘s’ ‘x.’’”` on a second pass) →
  `“‘s' 'x.’'”`; `“"s' '."'”`: `“‘s’ '.’'”` → `“‘s' '.’'”`; `"a "dogs' 's" b"`: `“a⍽‘dogs’ 's’ b”` →
  `“a⍽‘dogs' 's’ b”`; `"a "DOGS' 'S" b"`: `“a⍽‘DOGS’ 'S’ b”` → `“a⍽‘DOGS' 'S’ b”`;
  `"a "dogs' 'x' bowls" b"`: `“a⍽‘dogs’ “x” bowls’ b”` → `“a⍽‘dogs' “x” bowls’ b”`. A declared
  language whose primary pair is `‘’` (en-GB), Prose, Interface and `ALL`, every `SingleQuotes`
  mode: `"S' 's"`: `‘S’ 's’` → `‘S' 's’`; `"the dogs' 'x' bowls"`: `‘the⍽dogs’ “x” bowls’` →
  `‘the⍽dogs' “x” bowls’`; `"the dogs' 'bowls" today`: `‘the⍽dogs’ 'bowls’ today` →
  `‘the⍽dogs' 'bowls’ today`; `"S'. a(\n'3,".'`: `‘S’. a(\n'3,’.'` → `‘S'. a(\n'3,’.'`;
  `it) - 'cause' - 'cause -- "'cause) DOGS' - 'house' - it, - oz.) '90s" a`:
  `… ‘’cause) DOGS’ — “house” — …` → `… ‘’cause) DOGS' — “house” — …`. Typed text, English and
  en-GB, the same sets: `‘bowls' 'x S’ a`: `‘bowls’ 'x S’ a` → `‘bowls' 'x S’ a`; `to_plain` reads
  such a `’` back as itself, and `lint` no longer reports it (en-GB: `“S’ 'dogs" -the` gave
  `"S' 'dogs" -the` and `TypographicChar`, now `"S’ 'dogs" -the` and no report). Where nothing could
  pair with the opener, the possessive is placed as before (`"a "dogs' 'x" b"` gives
  `“a⍽‘dogs’ 'x’ b”`, en-GB `"s' 'x"` gives `‘s’ 'x’`), and with only text between the possessive
  and the closer nothing changes (`“He said ‘the⍽dogs’ bowls’”`).

- A typed `’` closes the `‘` of its pair when only signs, or nothing, stand between them and the
  neighbors allow the sign-only pair (design section 6.3), as the plain `""` that a first pass
  typesets as `‘’` does; other rules read the same closing role. In a declared language whose
  primary pair is `‘’` (en-GB), Prose: `'a ""-3'` gives `'a⍽‘’-3'` as before, and the second pass no
  longer gives `'a⍽‘’-3′`. Typed `'a ‘’-3'`, and likewise `'a ‘’ 3'`: in English and en-GB, Prose
  and `ALL`, `'a⍽‘’-3′` → `'a⍽‘’-3'`; with `SingleQuotes::Everywhere` the outer pair now pairs:
  English `'a⍽‘’-3′` (Prose, `ALL`) and `'a⍽‘’-3'` (Interface) → `“a⍽‘’-3”`, en-GB `'a⍽‘’-3′`
  (Prose, `ALL`) and `'a⍽‘’-3'` (Interface) → `‘a⍽‘’-3’`. English, both sets: `"a ‘’ b"`: `"a⍽‘’ b"`
  → `“a⍽‘’ b”`; `"a ‘-’ b"`: `"a⍽‘-’ b"` → `“a⍽‘-’ b”`; `"a ‘’, b"`: `"a⍽‘’, b"` → `“a⍽‘’, b”`. The
  closed `‘` no longer counts as a neutral opener, so a trailing possessive after it is placed
  (English and en-GB, Prose, Interface and `ALL`: `‘’\n”bowls'`: `‘’\n”bowls'` → `‘’\n”bowls’`), and
  the rules that read the role of a quote read it too (English and en-GB, the same sets:
  `)(\n ‘’ - ')“` gives `‘’ —` where it gave `‘’ -`). Such a pair is no longer reported: `x ‘’ y`
  gave `UnmatchedQuote` (info), `"a ‘’ b"` `CrossingQuotes` (warning).
- `to_plain`, `to_plain_template`, `lint` and `lint_template` are linear in the length of the text
  on long runs of quote characters in one paragraph as well (design section 12.1): the neighbors of
  a series of quotes, the open pairs and the later closer of a `’` after `s` were searched again for
  every quote. Measured in a release build, one paragraph of 96 KB: `’` repeated (Russian Prose) 5.7
  s → 17 ms; `«„` repeated, `x`, `“»` repeated (Russian Prose) 4.4 s → 43 ms; `“‘a dogs’ ` repeated,
  `b`, `’”` repeated (English Prose) 0.6 s → 31 ms. The results are the same.

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
