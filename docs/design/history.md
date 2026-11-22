# typolish design: History

Part of the [typolish design](./README.md). Section numbers are those of the whole design. This file
holds one-time material that the other sections rely on.

### 14.8 Checks behind this design

All checks were run on 2026-11-21 on the development machine, outside the repository, unless a
passage gives another date.

**Dependency versions** (crates.io sparse index and crates.io API):

| Crate                  | Latest non-yanked | `rust-version` | License           | Published  |
| ---------------------- | ----------------- | -------------- | ----------------- | ---------- |
| `thiserror`            | 2.0.21            | 1.77           | MIT OR Apache-2.0 | 2026-09-23 |
| `thiserror-impl`       | 2.0.21            | 1.77           | MIT OR Apache-2.0 | 2026-09-23 |
| `unicode-segmentation` | 1.13.3            | 1.85.0         | MIT OR Apache-2.0 | 2026-06-01 |
| `proptest`             | 1.12.0            | 1.88           | MIT OR Apache-2.0 | 2026-10-10 |
| `insta` (rejected)     | 1.49.0            |                | Apache-2.0        |            |
| `divan` (rejected)     | 0.1.21            |                |                   | 2025-04-10 |

`proptest` 1.12.0 features read from the index:
`std = ["rand/std", "rand/sys_rng", "regex-syntax", "num-traits/std"]`;
`default = ["std", "fork", "timeout", "bit-set"]`; `fork` pulls `rusty-fork` and `tempfile`.

**`unicode-segmentation` 1.13.3 source** (the published `.crate`): `UNICODE_VERSION = (17, 0, 0)`;
no normal dependencies; `#![deny(missing_docs, unsafe_code)]`; public `Graphemes`,
`GraphemeIndices`, `GraphemeCursor`; one feature, `no_std`.

**Toolchain**: `rustc 1.100.0 (0ef1bc386 2026-11-09)`; `char::UNICODE_VERSION == (18, 0, 0)`;
`cargo-deny 0.20.2`, `cargo-hack` and clippy installed; the rustup toolchain `1.100.0` is installed
besides `stable` (`cargo +1.100.0 -V` prints `cargo 1.100.0 (3acc79c0c 2026-11-05)`; checked on
2026-11-22).

**Dependency tree and cargo-deny**: a throwaway workspace with the manifests and `deny.toml` of
section 14.5 resolved 23 packages besides `typolish` (all listed in section 14.1, including the
build dependency `autocfg` and the UEFI-only `r-efi`); `cargo deny check` printed
`advisories ok, bans ok, licenses ok, sources ok`; `cargo deny list` showed only MIT, Apache-2.0
and, for `unicode-ident`, Unicode-3.0 in the normal tree.

**CLDR 49** (cldr-json 49.0.0, published 2026-10-22): from `cldr-numbers-full` for `ru`, `en` and
`en-GB` (the values of `en-GB` equal those of `en`): decimal, group (U+00A0 for ru), minusSign
(U+002D for both), percentFormat (`#,##0 %` with U+00A0 for ru, `#,##0%` for en),
minimumGroupingDigits (1 for both), miscPatterns range (`{0}–{1}`), nan (`не число` with U+00A0,
`NaN`), infinity (`∞`). From `cldr-misc-full`: delimiters for `ru`, `en`, `en-GB`; ellipsis U+2026.
From `cldr-dates-full` `ca-gregorian` for `ru` and `en`: format and stand-alone month names, wide
and abbreviated (section 7.2, 7.3). From `cldr-units-full`: short and narrow unit patterns for `ru`
(all with U+0020 between number and unit) and `en` (`{0}°C`, `{0}%` without a space; `μs`, `μm` with
U+03BC). `en-US` has no file of its own; `availableLocales` lists `en`, `en-GB`, `ru` among these
locales, and `supplemental/parentLocales.json` of cldr-core 49.0.0 (198 entries) has no entry for
`en-US`, so its parent is `en` by truncation of the tag and `en-US` uses the values of `en`
(checked). From `cldr-core` `supplemental/likelySubtags.json` (cldr-json 49.0.0): `ru` →
`ru-Cyrl-RU`, `en` → `en-Latn-US`, the likely-subtags rule behind section 2.2.

**HTML entities**: every name in section 11.2 is in the WHATWG `entities.json`; U+202F and U+2011
have no named reference. The license of the extract, CC BY 4.0, was read from §7.1.1 of the WHATWG
IPR policy on 2026-11-22.

**crates.io categories**: `text-processing` and `internationalization` exist.

**API sketch**: compiled with `rustc 1.100.0 --edition 2024 -D warnings`:
`const fn Typographer::new` with `Vec::new()` used in a `static`; `LanguageData` with `const fn`
setters and derived `PartialEq` and `Hash` over `&'static` slices; a table built from another
`const` table (as a downstream crate would do) and placed in a `static`; `Language` equality over
the data; a `thread_local!` `RefCell` with `const` initialization and a `try_borrow_mut` fallback; a
generic `Piece<'t, S>`; a `const` assertion of `Send + Sync`. The thread-local held no token type
with a lifetime.

**`const fn` in Rust 1.100** (a probe compiled during the review of the design):
`char::is_lowercase` is a `const fn`; `char::is_alphabetic` and `char::to_lowercase` are not (error
E0015), so the validation of language data cannot run in a `const fn` (section 2.2).

**zonewright** (the maintainer's crate, read only): `docs/design/` is in English, one document in
numbered files with a `README.md` index and `NN-decisions.md` and `NN-open-points.md`; neither
`dev/Cargo.toml` nor the crate manifests have an `authors` field; `LICENSE-MIT` begins `MIT License`
and `Copyright (c) 2026 Dmitry Bikman`; the root and crate copies of both license files are
byte-identical.

**Reference implementation**: version 7.9.1, installed outside the repository, run on the scouting
corpus of 820 cases (432 ru, 388 en-US; 405 negative cases: ru 207, en-US 198) with default
settings. Section 8.12 summarizes its recorded behavior. Nine of the 820 outputs were not idempotent
(lists `a,b,c` and look-alike replacement), which motivated sections 3.13 and 4.

**Atom and token representation** (stage 2, 2026-11-22): compiled with `rustc 1.100.0` as the first
code of the crate (`src/engine/ingest.rs`, `src/engine/lex.rs`, `src/engine/scratch.rs`), with the
lints of section 14.5 and `-D warnings`. `Atom` and `Tok` hold piece numbers and `u32` byte offsets,
no references; `Scratch` holds vectors of them and lives in a `thread_local!` `RefCell` with `const`
initialization; it is filled from inputs of any lifetime, and `Typographer` and `Scratch` are
asserted `Send + Sync + 'static` in a `const` item. `Tok` is at most 40 bytes (a unit test).

**Not checked:**

- Whether the name `typolish` is free on crates.io: the maintainer allowed one query to the
  crates.io API in stage 5 (section 15.5, item 3), and none was made before.
- Mil'chin's handbook, GOST standards and the Chicago Manual of Style were not read; the Russian and
  English norms marked "secondary" rest on Wikipedia articles (section 15.5, item 16).
- The design was not compiled as a whole; the allocation and linearity claims are verified by tests
  in stages 3 and 5 (section 15.4).
- `GraphemeBreakTest.txt` for Unicode 17.0.0 is fetched in stage 2.
