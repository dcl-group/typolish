# typolish design: Versioning, decisions, stage plan

Part of the [typolish design](./README.md). Section numbers are those of the whole design.

## 15. Versioning, decisions, stage plan

### 15.1 Versioning and output stability

- The first release is `0.1.0`. SemVer as Cargo applies it: before 1.0 the second number is the
  breaking one.
- API changes follow SemVer; the commitments of section 2.13 keep extensions additive.
- **Output changes.** The output of `PROSE` and `INTERFACE` for built-in languages changes only in a
  release that raises the breaking number: new rules in a set, changed lists, changed language data.
  A patch release may only fix output that contradicts this design or the rustdoc, and every such
  fix is recorded in the CHANGELOG with a "before → after" example. A new rule may arrive in a minor
  or patch release, but outside both sets. This keeps the snapshots of build-time generators and the
  tests of callers stable across `cargo update` (open point 13).
- Adding a language does not change output for existing tags. A tag that was unknown starts to
  resolve; the CHANGELOG says so.
- Rule ids are permanent (section 2.3).

### 15.2 Settled by the maintainer

Decided by the maintainer on 2026-11-21; not reopened here:

1. The crate is `typolish`, public on crates.io, independent of other projects; nothing is pushed or
   published without the maintainer's word.
2. There is no external corpus; the round trip is tested on the project's own corpus, and stage
   reports say so (sections 12.2, 13.7).
3. The reference implementation is installed only outside the repository. Its name appears nowhere
   in the repository (code, documentation including this design, commits, tests); the text says
   "reference implementation". Its pairs enter the tests as plain tables (section 13.4).
4. Git: the author is configured locally and is not changed; commits end with
   `Co-Authored-By: Claude <noreply@anthropic.com>`.
5. Dependency versions are the latest published, checked against crates.io, not memory (section
   14.1).
6. Rust 1.100 is both the MSRV and the installed toolchain; `rust-version = "1.100"` (section 14.4).
7. License, authors and copyright exactly as in zonewright: `license = "MIT OR Apache-2.0"`, no
   `authors` field (zonewright has none), the license texts copied verbatim with the copyright line
   `Copyright (c) 2026 Dmitry Bikman`; only the year changes when needed (section 14.5).
8. Stages 2 to 5 run without stops after this design is approved; stage 6 (HTML) is not done.
9. Working notes and Cargo's target directory live outside the repository.

The task itself fixes further points used throughout: the minus sign U+2212 for numbers, the
mandatory negative cases, HTML outside the core, `thiserror` for errors, `cargo deny check` clean,
clippy without warnings, rustdoc with an example on every public item, README and CHANGELOG in
English.

### 15.3 Design choices

Each is the lead's choice, recorded with its reason so that the maintainer can overrule it. Items
with an open point are marked.

| No. | Choice                                                                                                                                                   | Rejected                                                             | Section   |
| --- | -------------------------------------------------------------------------------------------------------------------------------------------------------- | -------------------------------------------------------------------- | --------- |
| C1  | A pass over grapheme tokens with a fragment classifier and phases; no regular expressions                                                                | regular expressions over the string, in-place editing, one automaton | 3         |
| C2  | Technical fragments frozen by a white-list classifier, dictionary first; the engine refuses edits to frozen tokens in one place                          | black lists of patterns; protection inside each rule                 | 3.6, 3.7  |
| C3  | Class-invariant tokens; idempotence argued from invariants I1 to I5 and tested with a fixed-point oracle                                                 | iterating rules to a fixed point at run time                         | 4         |
| C4  | Quotes: roles, families and a stack; quotes after numbers decided one by one; all or nothing per paragraph (open point 8)                                | partial placement; global attempts; scope of a string or a sentence  | 6         |
| C5  | Glue as a mark on space tokens; nowrap groups as an output kind                                                                                          | writing U+00A0 into the text                                         | 3.9, 11.5 |
| C6  | `const fn Typographer::new`; the rule set is mandatory; builder order-independent with a conflict error                                                  | `Default`; "last call wins"                                          | 2.4       |
| C7  | Unknown language: `Result` plus `Language::COMMON`; no region substitution                                                                               | a policy enum; `en-GB` → `en-US`                                     | 2.2       |
| C8  | `LanguageData` public, private fields, `const fn` setters, not `Copy`; `LanguageBuilder` for run-time languages (open point 14)                          | internal only; public fields; TOML or JSON                           | 7.1       |
| C9  | Pieces with a caller payload; output in the n + 1 form; `Protected` and `Markup` from the start                                                          | index-based output parts; strings with markers                       | 2.6       |
| C10 | Numbers as lazy `Display`, CLDR symbols for NaN and infinity                                                                                             | `String` results; errors on NaN                                      | 2.7       |
| C11 | Output kind per call; HTML escapes `& < > " '`; escaped bodies with complete container escaping (`JS`, `JS_HEX`, `JS_CODE_POINT`, `JSON`, `RUST`, `CSS`) | entities without markup escaping; escapes that leave `\` and quotes  | 11        |
| C12 | A thread-local working buffer plus an explicit `Scratch` for callers who manage memory                                                                   | a new vector per call                                                | 2.12      |
| C13 | `report` with edits, protected spans and diagnostics through a generic recorder                                                                          | callbacks; a result struct on every call                             | 2.9, 3.11 |
| C14 | `to_plain` undoes exactly the enabled rules; P1 and P2                                                                                                   | normalizing every typographic character                              | 12.1      |
| C15 | Diagnostics are not rules; `lint` adds the typographic-character check                                                                                   | checks as rules inside rule sets                                     | 12.3      |
| C16 | `unicode-segmentation` mandatory; GraphemeBreakTest as a test fixture                                                                                    | a hand-made table; a feature                                         | 5.1       |
| C17 | Regular-expression engines banned in `deny.toml`                                                                                                         | a rule in AGENTS.md only                                             | 14.5      |
| C18 | No whitespace normalization and no spaces after punctuation                                                                                              | space rules in Prose                                                 | 1.5       |
| C19 | Interface composition of section 9.2                                                                                                                     | a stricter Interface without no-break rules                          | 9         |
| C20 | En dash between numbers only from declared values in Interface; literal shapes and contexts in Prose (open point 7)                                      | context words only; never in text                                    | 10.6      |
| C21 | One glue character, U+00A0, by default; U+202F by setting (open point 10)                                                                                | U+202F after `№` and `§` by default                                  | 2.4       |
| C22 | `dash.spaced-hyphen` refuses two numbers, two single letters and two foreign words                                                                       | converting every lone hyphen                                         | 8.4       |
| C23 | Snapshots as plain files; load measured with std only                                                                                                    | `insta`, `criterion`, `divan`                                        | 13        |
| C24 | The corpus is written for the project; no third-party texts                                                                                              | public-domain prose                                                  | 13.7      |
| C25 | One error enum; application never fails and never panics                                                                                                 | `Result` from `typeset`                                              | 2.11      |

### 15.4 Stage plan

Every stage ends with a report: what was done, the list of commits, test results with output of
failures and of the comparison with the reference tables, what was not done or not verified, and
questions for the maintainer. In every stage the gates of section 14.7 pass before each commit, and
any deviation from this design is written into the design in the same change.

**Stage 2: core.** Repository skeleton after zonewright (`AGENTS.md`, `CLAUDE.md`, README and
CHANGELOG with an `Unreleased` section, license files, `dev/` workspace, `deny.toml`, `clippy.toml`,
`.gitignore`, `.gitattributes`, Prettier files). `Rule` with all 28 ids and `ORDER`, `RuleSet` with
`FromStr`, `Language` and the built-in tables as far as these rules need them, `Typographer::new`
and the builder, `Piece` and `Template`, ingest and protection, windows, lexer, classifier,
freezing, the phase engine, the recorder, rendering for `Output::Unicode`, the thread-local and
explicit scratch. Rules: `quotes.*`, `dash.*`, `range.*`, `ellipsis.*` for ru and en. Ready when:

- `typeset` and `typeset_template` work for ru, en and COMMON with these rules;
- the rule tables of these rules pass, alone and in both sets;
- every row of section 10.5 comes back unchanged and borrowed in Interface, and all 405 negative
  corpus cases keep their token byte for byte (section 13.3);
- the properties idempotence, fixed-point oracle, I1, I2, I4, I5, slots and protected spans,
  technical fragments, letters and digits, graphemes, Cow and no panic pass;
- the GraphemeBreakTest test and the conformance test pass; `cargo deny check` is clean.

**Stage 3: no-break spaces, numbers, language data, rule sets, output.** `nbsp.*`, `number.*`,
`symbols.*`; `LanguageData` complete with the ru-RU, en-US and COMMON tables of section 7,
`Language::from_data` with validation; every builder setting; `PROSE` and `INTERFACE` complete;
`format_number` and `format_range`; `Output::Html` (all styles, nowrap) and `Output::Escaped` (all
syntaxes). Ready when:

- all rule tables pass, and the slot participation table of section 2.6 passes row by row;
- the data test matches the CLDR 49 extract, and the built-in tables pass validation;
- the output tables and the decoder properties pass for every output kind;
- the Fluent isolation tests pass;
- the allocation test confirms section 2.12 (zero allocations for an unchanged string after warm-up,
  one for a changed string, zero for numbers written into a buffer with room);
- invariant I3 and the determinism-of-order property pass with all 28 rules.

**Stage 4: reverse transformation, source check, explain mode.** `to_plain`, `to_plain_template`,
`lint`, `lint_template`, every diagnostic of section 12.4, `report` and `report_template` with
protected spans and `Display`; the own corpus (section 13.7) with snapshots and
`roundtrip-known.tsv`. Ready when:

- P1 and P2 pass on the corpus with only the listed mismatches, and on generated strings;
- recorder independence passes, and applying `changes()` reproduces the result;
- every diagnostic has a table test, and `report` names the expected `TechKind` for every frozen row
  of section 10.5.

**Stage 5: comparison, load, documentation, release preparation (no release).** The reference tables
for all 820 cases; `benches/load.rs` and the linearity tests; rustdoc with an example on every
public item; README (with the known limits of section 10.8) and CHANGELOG (a prepared `0.1.0`
entry); `cargo package --list` reviewed. Ready when:

- every reference case is `AGREE` or `DIVERGE` with a reason code, and the report prints the summary
  per code;
- `load_is_linear` passes for every pathological input, and the measured numbers are in the report;
- `missing_docs` passes and every doctest passes;
- the package builds from its own tarball (`cargo package`), and nothing is published;
- the name check is done as the maintainer decides in open point 3.
