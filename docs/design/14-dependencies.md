# typolish design: Dependencies, MSRV, layout, gates

Part of the [typolish design](./README.md). Section numbers are those of the whole design.

## 14. Dependencies, MSRV, layout, gates

### 14.1 Dependencies

Versions are the latest published releases, checked against crates.io on 2026-11-21 (section 14.8),
as the maintainer decided. `Cargo.toml` states them as caret requirements `"x.y.z"` with these
versions as the minimum.

| Crate                  | Version | Where | Purpose                                                                                                   | License           | `rust-version` | Published  |
| ---------------------- | ------- | ----- | --------------------------------------------------------------------------------------------------------- | ----------------- | -------------- | ---------- |
| `thiserror`            | 2.0.21  | core  | `Error` (required by the task)                                                                            | MIT OR Apache-2.0 | 1.77           | 2026-09-23 |
| `unicode-segmentation` | 1.13.3  | core  | extended grapheme clusters (section 5.1); Unicode 17.0.0 tables, no dependencies, `#![deny(unsafe_code)]` | MIT OR Apache-2.0 | 1.85.0         | 2026-06-01 |
| `proptest`             | 1.12.0  | dev   | property tests with shrinking (section 13.5); `default-features = false, features = ["std"]`              | MIT OR Apache-2.0 | 1.88           | 2026-10-10 |

The resolved tree was checked with a throwaway workspace outside the repository (section 14.8):

- normal: `thiserror` 2.0.21, `thiserror-impl` 2.0.21, `proc-macro2` 1.0.107, `quote` 1.0.47, `syn`
  3.0.10, `unicode-ident` 1.0.27 (`(MIT OR Apache-2.0) AND Unicode-3.0`), `unicode-segmentation`
  1.13.3;
- dev only: `proptest` 1.12.0 with `bitflags` 2.13.2, `cfg-if` 1.0.5, `chacha20` 0.10.2,
  `core_detect` 1.0.0, `cpufeatures` 0.3.1, `getrandom` 0.4.3, `libc` 0.2.191, `num-traits` 0.2.19,
  `rand` 0.10.4, `rand_core` 0.10.1, `rand_xorshift` 0.5.0, `regex-syntax` 0.8.12 (a parser, not a
  matching engine), `unarray` 0.1.4, `autocfg` 1.5.1 (a build dependency of `num-traits`) and
  `r-efi` 6.0.0 (a dependency of `getrandom` for UEFI targets only;
  `MIT OR Apache-2.0 OR LGPL-2.1-or-later`, allowed through `OR`). The `fork` and `timeout` features
  (with `rusty-fork` and `tempfile`) and `bit-set` are off.

With the 7 normal packages that makes 23 packages in `Cargo.lock` besides `typolish`.

`cargo deny check` on that tree, with the `deny.toml` of section 14.5 (licenses MIT, Apache-2.0,
Unicode-3.0; regex engines banned; all features), printed
`advisories ok, bans ok, licenses ok, sources ok`. No C code is compiled; `libc` and `getrandom` are
Rust bindings in the dev tree only.

### 14.2 Rejected dependencies

- `regex`, `regex-lite` and other engines: rules are functions over tokens; the task prefers no
  engine in the core, and `deny.toml` bans them (section 14.5).
- A hand-made grapheme table instead of `unicode-segmentation`: incomplete (`SpacingMark`,
  `Prepend`, regional indicators, Hangul) and a maintenance burden with every Unicode version
  (section 5.1). `unicode-segmentation` behind a feature: features unify across the graph and would
  change output.
- `memchr`: the prefilter is a 256-bit set over bytes.
- `criterion`, `divan` (last release 0.1.21 on 2025-04-10): section 13.8. `insta`: snapshots are
  plain files compared byte for byte and updated by hand (section 13.7).
- `serde`: language data is Rust (section 7.1); callers parse their own settings and use `FromStr`
  on `Rule` and `RuleSet`.
- `fluent-*`: the crate knows nothing of Fluent; the generator maps it (section 2.14).

### 14.3 Features

`default = []` and no features in version 1, so no feature can change the result. The name `html` is
reserved for the stage 6 module (a splitter that produces `Piece::Markup` and `Piece::Protected`)
and is not declared now. `cargo hack --each-feature` is trivial until a feature exists; it stays in
the gates.

### 14.4 MSRV

MSRV 1.100, decided by the maintainer: `rust-version = "1.100"`. It is also the installed toolchain
(`rustc 1.100.0`, checked). Every dependency needs less (at most 1.88 for `proptest`, dev only).
Language features relied on: edition 2024, `const fn` with `Vec::new`, `thread_local!` with `const`
initialization, `LazyLock` in examples. The MSRV is checked by a gate with the pinned toolchain
(section 14.7).

### 14.5 Cargo, lints, cargo-deny

`dev/Cargo.toml` (virtual workspace, as in the maintainer's crate zonewright):

```toml
[workspace]
members = ["typolish"]
resolver = "3"

[workspace.package]
edition = "2024"
rust-version = "1.100"
license = "MIT OR Apache-2.0"
# repository: open point 2; absent until the maintainer names the URL

[workspace.lints.rust]
unsafe_code = "deny"
missing_docs = "deny"

[workspace.lints.clippy]
# Library code does not panic; tests may (see clippy.toml).
unwrap_used = "deny"
expect_used = "deny"
panic = "deny"
unreachable = "deny"
todo = "deny"
unimplemented = "deny"
indexing_slicing = "deny"
dbg_macro = "deny"
print_stdout = "deny"
print_stderr = "deny"
```

`dev/typolish/Cargo.toml`: `name = "typolish"`, `version = "0.1.0"`, a one-line description,
`readme = "README.md"`, `keywords = ["typography", "typesetting", "quotes", "nbsp", "i18n"]`,
`categories = ["text-processing", "internationalization"]` (both slugs exist on crates.io, checked
on 2026-11-21), workspace inheritance for edition, rust-version, license and repository,
`exclude = ["tests/fixtures/unicode/"]`, `[lints] workspace = true`, and the dependencies of section
14.1. There is no `authors` field: zonewright has none, and the maintainer decided that license,
authors and copyright are copied from it exactly. `lib.rs` also carries `#![forbid(unsafe_code)]`.
The one exception is `tests/alloc.rs`: its counting allocator needs `unsafe impl GlobalAlloc`, so
the file starts with `#![allow(unsafe_code)]` and a comment giving this reason. It is the only
`unsafe` code in the package; `unsafe_code` stays `deny` for the workspace and `forbid` in `lib.rs`.
`dev/clippy.toml` allows unwrap, expect, panic, indexing, dbg and print in tests, as in zonewright;
these settings cover only `#[test]` functions and `#[cfg(test)]` modules, not test helpers, benches
or examples. Therefore every root file of a non-library target starts with a `//!` line
(`missing_docs` applies to each crate root) and an allow list: every `tests/*.rs` has
`#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::indexing_slicing, clippy::print_stdout, clippy::print_stderr, clippy::dbg_macro)]`,
which also covers `tests/common/mod.rs` through `mod common;`; `benches/load.rs` and every
`examples/*.rs` have
`#![allow(clippy::print_stdout, clippy::print_stderr, clippy::unwrap_used, clippy::expect_used, clippy::indexing_slicing)]`.
`src/` keeps the full deny set and has no `allow` of these lints.

License files: `LICENSE-MIT` and `LICENSE-APACHE` at the root and byte-identical copies in
`dev/typolish/`, copied verbatim from zonewright; the MIT file keeps the line
`Copyright (c) 2026 Dmitry Bikman`, and only the year changes when a later year requires it. The
crate name appears in neither text.

`dev/deny.toml`:

```toml
# cargo-deny configuration. Run from this directory: `cargo deny check`.

[graph]
all-features = true

[advisories]
version = 2

[licenses]
allow = ["MIT", "Apache-2.0", "Unicode-3.0"]
confidence-threshold = 0.8

[bans]
multiple-versions = "warn"
wildcards = "deny"
# No TLS or crypto stacks, no regular-expression engines (the core is a pass over characters).
deny = [
    { crate = "ring" },
    { crate = "openssl" },
    { crate = "openssl-sys" },
    { crate = "openssl-src" },
    { crate = "native-tls" },
    { crate = "regex" },
    { crate = "regex-lite" },
    { crate = "regex-automata" },
    { crate = "fancy-regex" },
    { crate = "onig" },
    { crate = "pcre2" },
]

[sources]
unknown-registry = "deny"
unknown-git = "deny"
allow-registry = ["https://github.com/rust-lang/crates.io-index"]
```

`regex-syntax` is not banned: it is a parser without a matching engine, used by `proptest` in the
dev tree. A license is added to `allow` only when the actual tree needs it, with the reason in a
comment.

### 14.6 Repository layout

Following zonewright; stage 2 creates the skeleton (everything except `docs/design/`, which this
commit adds, and the modules, tests and examples that later stages of section 15.4 deliver).

```text
AGENTS.md  CLAUDE.md  README.md  CHANGELOG.md  LICENSE-MIT  LICENSE-APACHE
.gitignore  .gitattributes  .prettierrc.json  .prettierignore
docs/design/                 this design
dev/
  Cargo.toml  Cargo.lock  deny.toml  clippy.toml
  typolish/
    Cargo.toml  LICENSE-MIT  LICENSE-APACHE  README.md -> ../../README.md
    src/
      lib.rs  error.rs  typographer.rs  language.rs  rule.rs  piece.rs  number.rs  output.rs
      report.rs  plain.rs  lint.rs
      lang/    mod.rs  data.rs  ru_ru.rs  en_us.rs  common.rs
      engine/  mod.rs  ingest.rs  window.rs  lex.rs  classify.rs  phase.rs  render.rs  record.rs  scratch.rs
      rules/   mod.rs  symbols.rs  ellipsis.rs  dash.rs  range.rs  number.rs  quotes.rs  nbsp.rs
    tests/
      common/mod.rs  rules_*.rs  negative.rs  parts.rs  numbers.rs  output.rs  fluent_isolation.rs
      reference/{ru,en_us}.rs  properties.rs  graphemes.rs  roundtrip.rs  corpus.rs  alloc.rs
      load.rs  conformance.rs  data.rs  api.rs  lint.rs
      corpus/  fixtures/unicode/
    benches/load.rs
    examples/fluent_generator.rs  examples/per_request.rs  examples/numbers.rs
```

`AGENTS.md` follows the skeleton of zonewright and adds two hard rules: the name of the reference
implementation never appears in the repository, and commits end with
`Co-Authored-By: Claude <noreply@anthropic.com>`. `CLAUDE.md` is a pointer to `AGENTS.md`. Cargo's
target directory lives outside the repository.

### 14.7 Gates

Before every commit, from `dev/`, for the `typolish` package only: `cargo fmt --check`,
`cargo clippy --all-targets -- -D warnings`, `cargo deny check`, `cargo test` (doctests included),
`cargo hack --each-feature test`, `cargo +1.100.0 check --all-targets` (the MSRV toolchain;
identical to the other gates while the installed stable is 1.100, it keeps the MSRV checked when
stable moves on). Formatting of Markdown and JSON is a manual step before the commit, not a gate, as
in zonewright: `npx --yes prettier@3 --write --ignore-path .prettierignore -- <paths>`
(`proseWrap: always`, width 100); TOML is tidied by hand. Plus the script outside the repository
that rejects the reference implementation's name in `git diff --cached` (contents and paths) and, as
a `commit-msg` hook, in the commit message. Commits are small, in Conventional Commits style with
English subjects, staged by explicit path; nothing is pushed or published without the maintainer's
word.
