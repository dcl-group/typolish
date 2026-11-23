# Changelog

All notable changes to this crate are recorded in this file. The format follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and the crate follows
[Semantic Versioning](https://semver.org/spec/v2.0.0.html) as Cargo applies it.

## Unreleased

Nothing released yet. Version 0.1.0 is in development.

- `Typographer::to_plain` and `Typographer::to_plain_template` map typeset Unicode text back to the
  canonical plain form, undoing exactly the enabled rules: a typographic character goes back only
  where an enabled rule produces it (glue that no enabled rule sets stays, a typed `–` with spaces
  stays), quote glyphs at every level become `"`, protected spans and markup are left alone, slots
  keep their place, and the result is stable under a second call. Typesetting the plain form gives
  the typeset text back; the known losses are listed in `tests/corpus/roundtrip-known.tsv`.
- `Typographer::report` and `Typographer::report_template` return the result together with every
  edit (`Change`), the protected spans (`ProtectedSpan`, `ProtectedKind`, `TechKind`) and the
  diagnostics (`Diagnostic`, `DiagnosticKind`, `Severity`): those of protection and the fragment
  classifier (backtick spans, fenced blocks and own delimiter pairs, paragraph windows, and
  technical fragments such as flags, paths, addresses, versions and identifiers) and those of the
  rules. `Report` has no `Display` yet.
- The phase engine applies the rules to strings and templates for `ru-RU`, `en-US` and
  `Language::COMMON`: quotes by nesting level, apostrophes and primes (`quotes.apostrophe`,
  `quotes.pairs`, `quotes.primes`, with `UnmatchedQuote`, `CrossingQuotes` and `AmbiguousQuote`),
  dashes (`dash.dialogue`, `dash.double-hyphen`, `dash.spaced-hyphen`, with `AmbiguousHyphen`),
  ranges (`range.placeholders`, `range.digits`, `range.context`, `range.time`), ellipses
  (`ellipsis.dots`, `ellipsis.inner`), no-break spaces as glue marks (`nbsp.abbreviations`,
  `nbsp.initials`, `nbsp.dates`, `nbsp.units` with `PercentSpacing`, `nbsp.signs`, `nbsp.phone`,
  `nbsp.short-words`, `nbsp.short-length`, `nbsp.particles`, `nbsp.dash`, with the `narrow_glue`,
  `add_units`, `add_short_words`, `add_particles`, `short_word_length` and `short_words_any_script`
  settings; no glue sticks to a fragment that holds an `Other` slot, quoted or bracketed alike),
  numbers in text (`number.minus` with the `minus_sign` setting, which accepts `-` and the dashes
  U+2010 to U+2015 and U+2212, the signs the second pass reads as the sign of a number;
  `number.times`, which reads a number with its percent sign after spaces as a left factor), and
  symbols (`symbols.copyright`, `symbols.arrows`, `symbols.math`).
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
