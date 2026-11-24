# Changelog

All notable changes to this crate are recorded in this file. The format follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and the crate follows
[Semantic Versioning](https://semver.org/spec/v2.0.0.html) as Cargo applies it.

## Unreleased

Nothing yet.

## 0.1.0 - unreleased

The first version, prepared and not published; the date is set when the version is tagged. Minimum
supported Rust version: 1.100.

### Added

- `Typographer`, built once with a `Language` and a `RuleSet` (`Typographer::new`, a `const fn`
  usable in a `static`, or `Typographer::builder` with settings) and then only applied. It is
  immutable and `Send + Sync`; applying it never fails and never panics, and an unchanged string is
  returned borrowed, without allocation after the first call on a thread. `Scratch` is reusable
  working memory for callers that manage it themselves (`typeset_with`, `typeset_template_with`);
  `typeset_into` writes into a buffer.
- Three inputs: a whole string (`typeset`), a message template of `Piece`s (`Text`, `Slot` with a
  `SlotKind` of number, word, date or other, `Protected`, `Markup`) whose slots keep their place in
  the resulting `Template` (`typeset_template`), and a `Number` (`format_number`, `format_range`).
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
  is not a letter, a digit, whitespace or a dash, and signs to be punctuation characters. The
  `Display` of a `Language` escapes invisible characters of a tag that `from_static` did not
  validate.
- Protection: backtick spans and fenced blocks, delimiters of one's own, `Piece::Protected`, and the
  technical fragments that the classifier recognizes by their shape (URLs, e-mail addresses and
  mentions, flags, paths, `host:port`, operators, UUIDs, hex numbers, ISO dates, versions, dotted
  names, lists, identifiers); slots and protected text are never edited.
- Number formatting by the language: decimal separator, grouping from `min_grouping_digits`, group
  separator, minus sign U+2212, range pattern, and the symbols of not-a-number and infinity, for
  every integer type, `f32`, `f64`, `Number::decimal` (digits as written) and `Number::fixed`. The
  lazy displays `NumberDisplay` and `RangeDisplay` allocate nothing.
- Output kinds, given with every call: `Output::Unicode`; `Output::Html` with named, decimal or
  hexadecimal references or characters (`HtmlStyle`) and no-break groups as entities or as nowrap
  elements (`nowrap_class`, `nowrap_style`); `Output::Escaped` with the body of a string literal for
  JavaScript (`JS`, `JS_HEX`, `JS_CODE_POINT`), JSON, Rust and CSS (`EscapeStyle`), the typographic
  set or with `non_ascii()` every non-ASCII character escaped; `$` before `{` and at the end of a
  text is escaped in the JavaScript styles.
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
  `fluent_generator` (a build-time generator over Fluent messages with `report_template` and
  `Output::Escaped(EscapeStyle::RUST)`).

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
- A run of spaces cut by a `Markup` piece is no longer read as a single pattern space by
  `number.minus`, `symbols.arrows` and `symbols.math` (design section 8.2), in Prose: the template
  `слово `, `<b>`, ` -15`: `слово <b> −15` → `слово <b> -15`; `a `, `<b>`, ` -> b`: `a <b> → b` →
  `a <b> -> b`; `x `, `<b>`, ` +- y`: `x <b> ± y` → `x <b> +- y`.
- U+061C, U+180E, U+206A to U+206F and U+FFF9 to U+FFFB are format characters of the lexer (design
  section 5.4), as U+200E is, instead of freezing their word as `Technical(Unclassified)`: ru Prose
  `в\u{61c} лесу`: unchanged → `в\u{61c}⍽лесу`.
