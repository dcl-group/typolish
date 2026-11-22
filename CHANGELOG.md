# Changelog

All notable changes to this crate are recorded in this file. The format follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and the crate follows
[Semantic Versioning](https://semver.org/spec/v2.0.0.html) as Cargo applies it.

## Unreleased

Nothing released yet. Version 0.1.0 is in development.

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
  ranges (`range.placeholders`, `range.digits`, `range.context`, `range.time`) and ellipses
  (`ellipsis.dots`, `ellipsis.inner`). The `symbols.*`, `number.*` and `nbsp.*` rules are accepted
  in rule sets but change nothing yet.
- `LanguageBuilder` builds a language at run time from an existing language or from
  `LanguageData::new` under a new tag, with one method per field of `LanguageData` (`abbreviations`
  takes the parts and the binding of each entry); `build` lowercases the word lists, sorts and
  deduplicates every list, and validates the table as `Language::from_data` does.

- Known limitation of the development state: `Output::Html` and `Output::Escaped` are accepted but
  written as Unicode, without encoding, until their encoders are added; do not use them for
  untrusted text.
