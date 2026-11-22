# Changelog

All notable changes to this crate are recorded in this file. The format follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and the crate follows
[Semantic Versioning](https://semver.org/spec/v2.0.0.html) as Cargo applies it.

## Unreleased

Nothing released yet. Version 0.1.0 is in development.

- Known limitation of the development state: `Output::Html` and `Output::Escaped` are accepted but
  written as Unicode, without encoding, until their encoders are added; do not use them for
  untrusted text.
