# `Display` of errors and of the report leaves default-ignorable characters raw

## Problem

The `Display` of errors and of the report table escapes every character of general category `Cf` and
the reserved default-ignorable code points (design sections 2.11 and 7.6, choice C41 of section
15.3; `report::invisible`). Other characters of the Unicode property `Default_Ignorable_Code_Point`
are printed raw, so a log line can hide them: U+3164 HANGUL FILLER, the variation selectors
(U+FE0F), U+034F COMBINING GRAPHEME JOINER, U+115F and U+1160, U+FFA0. Example:
`LanguageBuilder::new("a\u{3164}b").build()` reports ``invalid language data for `aㅤb`: tag: …``,
with the filler raw. The code agrees with the design; only the readability of the error suffers.

## Goal

Every `Default_Ignorable_Code_Point` is escaped in the `Display` of errors and of the report table,
by a `const fn` next to `is_reserved_default_ignorable`, with the ranges written out in code (no
dependency), so the result does not depend on the Rust version.

## Acceptance

- A test in `tests/negative_report.rs`: the error of `LanguageBuilder::new("a\u{3164}b")` and a
  report row with U+FE0F and U+034F show `\u{3164}`, `\u{fe0f}`, `\u{34f}` and none of the raw
  characters.
- Design sections 2.11 and 12.5 and choice C41 name the wider set.
