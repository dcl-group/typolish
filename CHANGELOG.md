# Changelog

All notable changes to this crate are recorded in this file. The format follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and the crate follows
[Semantic Versioning](https://semver.org/spec/v2.0.0.html) as Cargo applies it.

## Unreleased

Nothing released yet. Version 0.1.0 is in development.

- `Typographer::report` and `Typographer::report_template` return the result together with the
  protected spans (`ProtectedSpan`, `ProtectedKind`, `TechKind`) and the diagnostics (`Diagnostic`,
  `DiagnosticKind`, `Severity`) of protection and the fragment classifier: backtick spans, fenced
  blocks and own delimiter pairs, paragraph windows, and technical fragments such as flags, paths,
  addresses, versions and identifiers. `Report` has no `Display` yet, and no typographic rule
  changes text yet, so `changes()` is always empty.

- Known limitation of the development state: `Output::Html` and `Output::Escaped` are accepted but
  written as Unicode, without encoding, until their encoders are added; do not use them for
  untrusted text.
