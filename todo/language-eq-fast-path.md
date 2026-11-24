# `==` of `Language` always compares the whole table

## Problem

`PartialEq` of `Language` compares the tag and then every field and list of the data table
(`Repr::eq` in `src/language.rs`), even when both sides hold the same table: a clone of a language
built at run time shares its `Arc`, and two copies of a `Language::RU_RU` value may point at the
same `LanguageData`. Measured in a release build, `Language::RU_RU == Language::from_tag("ru")?` and
`a == a` both take about 1.5 µs.

`==` is not on the hot path of `typeset`, and the addresses of promoted `const` tables are not
guaranteed to be equal (`Language::RU_RU` and `from_tag("ru")` may point at different copies), so
the gain is only for clones of one value.

## Goal

`Repr::eq` returns `true` at once when both sides point at the same table, before the full
comparison: `core::ptr::eq` for two `Repr::Static`, `Arc::ptr_eq` for two `Repr::Owned`. The result
of `==` does not change.

## Acceptance

A test in `tests/language.rs` checks `assert_eq!(Language::RU_RU, Language::RU_RU.clone())` and the
same for a language built with `LanguageBuilder` and its clone, and the existing tests of equality
across the two representations still pass.
