# Changelog

All notable changes to this crate are recorded in this file. The format follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and the crate follows
[Semantic Versioning](https://semver.org/spec/v2.0.0.html) as Cargo applies it.

## Unreleased

Nothing released yet. Version 0.1.0 is in development.

- `Typographer::to_plain` and `Typographer::to_plain_template` map typeset Unicode text back to the
  canonical plain form, undoing exactly the enabled rules: a typographic character goes back only
  where an enabled rule produces it (glue that no enabled rule sets stays, a typed `–` with spaces
  stays), quote glyphs at every level become `"` (the nested pair becomes `'` only where `"` would
  not pair again, as in `«x „“.»`), protected spans and markup are left alone, slots keep their
  place, and the result is stable under a second call. Typesetting the plain form gives the typeset
  text back; the known losses are listed in `tests/corpus/roundtrip-known.tsv`.
- `Typographer::report` and `Typographer::report_template` return the result together with every
  edit (`Change`), the protected spans (`ProtectedSpan`, `ProtectedKind`, `TechKind`) and the
  diagnostics (`Diagnostic`, `DiagnosticKind`, `Severity`): those of protection and the fragment
  classifier (backtick spans, fenced blocks and own delimiter pairs, paragraph windows, and
  technical fragments such as flags, paths, addresses, versions and identifiers) and those of the
  rules. The `Display` of `Report` prints a table of the edits and the protected spans, one line per
  record, and the `Display` of `Diagnostic` prints its severity, rule, piece, range and a message.
- `Typographer::lint` and `Typographer::lint_template` check a source that must stay plain without
  building output: they return the diagnostics of `report` with `Output::Unicode` (never
  `NowrapFallback`) and a `TypographicChar` for every character that `to_plain` would change, so a
  source has such an entry exactly when `to_plain` changes it.
- `examples/fluent_generator.rs` shows the build-time flow with Fluent: messages as pieces,
  `report_template` with `Output::Escaped(EscapeStyle::RUST)`, warnings as `cargo::warning` lines.
- The phase engine applies the rules to strings and templates for `ru-RU`, `en-US` and
  `Language::COMMON`: quotes by nesting level, apostrophes and primes (`quotes.apostrophe`,
  `quotes.pairs`, `quotes.primes`, with `UnmatchedQuote`, `CrossingQuotes` and `AmbiguousQuote`),
  dashes (`dash.dialogue`, `dash.double-hyphen`, `dash.spaced-hyphen`, with `AmbiguousHyphen`),
  ranges (`range.placeholders`, `range.digits`, `range.context`, `range.time`), ellipses
  (`ellipsis.dots`, `ellipsis.inner`), no-break spaces as glue marks (`nbsp.abbreviations`,
  `nbsp.initials`, `nbsp.dates`, `nbsp.units` with `PercentSpacing`, `nbsp.signs`, `nbsp.phone`,
  `nbsp.short-words`, `nbsp.short-length`, `nbsp.particles`, `nbsp.dash`, with the `narrow_glue`,
  `add_units`, `add_short_words`, `add_particles`, `short_word_length` and `short_words_any_script`
  settings; no glue sticks to a fragment that holds an `Other` slot, quoted or bracketed alike; a
  percentage is no digit group of `nbsp.phone` in either of its forms, so the percent setting
  changes nothing for that rule; a hyphen range with its percent sign attached, `10-20%`, is a
  percentage range for the dash rules as `10-20 %` is), numbers in text (`number.minus` with the
  `minus_sign` setting, which accepts `-` and the dashes U+2010 to U+2015 and U+2212, the signs the
  second pass reads as the sign of a number; `number.times`, which reads a number with its percent
  sign after spaces as a left factor), and symbols (`symbols.copyright`, `symbols.arrows`,
  `symbols.math`).
- A Russian particle (`б бы ж же ли ль`, or the caller's list) is glued to the word or slot before
  it also when closing punctuation follows it (`Что ж,` gets a no-break space after `Что`; before,
  it stayed as typed), and a particle so placed glues nothing after itself: `nbsp.short-words` and
  `nbsp.short-length` no longer glue the next word to it, so Prose gives `Знаешь ли ты` a no-break
  space after `Знаешь` only (before, after `ли` too). Each particle of a run is glued to the word
  before it (`что же бы ты`: after `что` and `же`).
- `Typographer::format_number` and `Typographer::format_range` format a `Number` (every integer
  type, `f32`, `f64`, `Number::decimal` for digits as written, `Number::fixed` for a float rounded
  to a number of fraction digits) by the language: decimal separator, grouping from
  `min_grouping_digits`, group separator and minus sign of the settings, the range pattern of the
  language, and the symbols of `NaN` and infinity. They return the lazy displays `NumberDisplay` and
  `RangeDisplay`, which allocate nothing.
- `Output::Html` escapes `& < > " '` and writes the typographic set as named, decimal or hexadecimal
  references or as characters (`HtmlStyle`), with no-break groups as entities or as nowrap elements
  (`nowrap_class`, `nowrap_style`); a group split at a `Markup` piece gives `NowrapFallback`.
  `Output::Escaped` writes the body of a string literal for JavaScript (`JS`, `JS_HEX`,
  `JS_CODE_POINT`), JSON, Rust and CSS, with the container escaped completely and the typographic
  set, or with `non_ascii()` every non-ASCII character, as escapes. A result is borrowed exactly
  when its encoded form equals the input.
- `LanguageBuilder` builds a language at run time from an existing language or from
  `LanguageData::new` under a new tag, with one method per field of `LanguageData` (`abbreviations`
  takes the parts and the binding of each entry); `build` lowercases the word lists, sorts and
  deduplicates every list, and validates the table as `Language::from_data` does.
- The forward pass (`typeset*`, `report*`) is about three (en-US) to six (ru-RU) times faster on the
  corpus, with unchanged results: about 7 to 15 MB/s in Unicode output and 3 to 6 µs for an
  interface string of 40 to 80 bytes on the development machine (design section 13.8). A typographer
  of `Typographer::new` still computes its tables on its first call, without allocation.
