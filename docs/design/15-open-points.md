# typolish design: Open points

Part of the [typolish design](./README.md). Section numbers are those of the whole design.

### 15.5 Open points for the maintainer

None of these points blocks stage 2. Each names the default that this design applies until the
maintainer decides; the default is the recommendation. The decisions already made by the maintainer
are listed in section 15.2. Numbers never change; settled points will keep their number and record
the date of the decision.

| Item | Subject                                     | Status         | Default applied                                      |
| ---- | ------------------------------------------- | -------------- | ---------------------------------------------------- |
| 1    | Language of `docs/design/`                  | Open: confirm  | English, as in zonewright                            |
| 2    | Repository URL                              | Open: data     | no `repository` field                                |
| 3    | Checking the name on crates.io              | Open: decision | one query in stage 5                                 |
| 4    | Grouping threshold for ru                   | Open: decision | from 5 digits                                        |
| 5    | Group separator for ru                      | Open: decision | U+00A0                                               |
| 6    | Em dash style for en-US                     | Open: decision | closed, `word—word`                                  |
| 7    | Literal numeric ranges in Prose             | Open: decision | shape and context rules in Prose, none in Interface  |
| 8    | Scope of an unpaired quote                  | Open: decision | all or nothing per paragraph                         |
| 9    | Single quotes at the top level              | Open: decision | `NestedOnly`                                         |
| 10   | Narrow no-break space                       | Open: decision | U+00A0 everywhere                                    |
| 11   | Default spacing before `%`                  | Open: decision | as written                                           |
| 12   | Dialogue dash in Prose                      | Open: decision | in Prose                                             |
| 13   | Output stability policy                     | Open: decision | output of the sets changes only in breaking releases |
| 14   | Public language data in version 1           | Open: decision | public `LanguageData` and `LanguageBuilder`          |
| 15   | `nbsp.phone` in Interface                   | Open: decision | Prose only                                           |
| 16   | Word lists and norms from secondary sources | Open: decision | lists as in section 7                                |
| 17   | Digit grouping in text                      | Open: decision | no text rule; `format_number` groups values          |

#### Open: needs the maintainer's decision or data

**1.** **Language of `docs/design/`.** The task requires English only for README and CHANGELOG. In
zonewright everything in the repository, the design included, is in American English, and this
design follows it; code refers to it as "design section N". Options: (a) English, as in zonewright;
(b) Russian; (c) English, with a Russian summary in each stage report. **Recommendation: (a).**

**2.** **Repository URL.** `Cargo.toml` of zonewright has
`repository = "https://github.com/dcl-group/zonewright"`; no URL is given for typolish, and the
decision about license, authors and copyright does not cover it. Options: (a) the maintainer names
the URL now (by analogy it would be `https://github.com/dcl-group/typolish`, which is a guess); (b)
the field stays out of `Cargo.toml` until the maintainer names it, at the latest in stage 5.
**Recommendation: (b)** until the URL is confirmed; the field is never invented.

**3.** **Checking the name on crates.io.** Whether `typolish` is free was not checked: the query
would disclose the project name to an outside service without permission. Options: (a) the
maintainer checks; (b) the lead makes one query to the crates.io API in stage 5; (c) the query is
made now. **Recommendation: (b).** If the name is taken, the maintainer chooses another.

**4.** **Grouping threshold for ru.** CLDR 49 gives `minimumGroupingDigits = 1` (`1 000`). The
Russian norm, confirmed only by a secondary source, groups from five digits (`1000`, `10 000`).
Options: (a) 2, from five digits; (b) 1, as CLDR. **Recommendation: (a)**; en stays at 1 (from four
digits); `min_grouping_digits` changes either.

**5.** **Group separator for ru.** CLDR 49 uses U+00A0; U+202F is often preferred in typesetting but
has weaker font support and no named HTML entity. Options: (a) U+00A0; (b) U+202F. **Recommendation:
(a)**; `group_separator` gives (b).

**6.** **Em dash style for en-US.** Options: (a) closed `word—word` (Chicago Manual of Style,
through secondary sources); (b) `word⍽— word` with glue before the dash, as in Russian (the
reference implementation does this); (c) spaced `word — word` without glue (AP style).
**Recommendation: (a)**; `dash_style` gives (b) and (c). With the default `PercentSpacing`, (a) is
the only rule of Interface that removes spaces (a non-default `PercentSpacing` also removes the
space before `%` and `‰`, section 8.2), and it does so only between two plain words: next to a
number, a slot, a protected span, a technical fragment or punctuation it keeps the author's spaces
(`Hello, — he said`, `See https://example.com — the guide`), so a URL never swallows the next word
(section 8.4).

**7.** **Literal numeric ranges in Prose** (`1941-1945`, `10-20`, `12-15`). Options: (a) in Prose
`range.digits` (two ascending numbers without leading zeros) and `range.context` (a unit, year,
century or page word next to it), in Interface neither; (b) only the context rule, as the reference
implementation does for Russian; (c) never in text, only `range.placeholders` and `format_range`.
**Recommendation: (a)**: the task asks for en dashes in numeric ranges and ranges of years, and
technical text is protected by Interface and by backticks. Known cost in Prose: a port range
`8000-8100` written as plain text and a telephone fragment `123-4567` get an en dash (section 10.5,
row 10); Interface and backticks keep them (sections 9.2, 10.6).

**8.** **Scope of an unpaired quote.** Options: (a) all or nothing per paragraph: an unpaired or
crossing straight quote leaves every straight quote of its paragraph (of its template, for pieces)
unchanged, with a diagnostic; (b) partial placement: pair what pairs, leave the unpaired quote; (c)
all or nothing per sentence. **Recommendation: (a)**: it is the only option that never guesses which
quotes form pairs and is provably idempotent (section 6.11); (b) guesses and is not idempotent;
sentence boundaries for (c) are unreliable. Cost of (a): one typo leaves a whole paragraph with
straight quotes, reported exactly.

**9.** **Single quotes at the top level** (`She said 'hi'`). Options: (a) `NestedOnly`: converted
only inside double quotes; (b) `Everywhere`: converted by level like double quotes; (c) `Never`.
**Recommendation: (a)**: a straight `'` in plain text is more often an apostrophe or code than a
quote; `single_quotes` gives (b) and (c).

**10.** **Narrow no-break space.** Options: (a) U+00A0 everywhere, U+202F by the `narrow_glue`
setting; (b) U+202F after `№` and `§` in Russian (as the reference implementation does); (c) U+202F
also in abbreviations and between initials (as the Russian Wikipedia suggests). **Recommendation:
(a)**: one character in the whole output, better font support, a named entity, a simpler reverse
transformation; the norms behind (b) and (c) were not checked against a primary source.

**11.** **Default spacing before `%`.** The task says "by setting"; CLDR 49 writes `5⍽%` in Russian
and `5%` in English. Options: (a) `AsWritten`: glue an existing space, leave an attached sign; (b)
`Language`: the CLDR pattern (insert in ru, remove in en); (c) `Attached`. **Recommendation: (a)**:
by default the crate neither inserts nor removes characters around `%`; `percent` gives the others.

**12.** **Dialogue dash in Prose.** `dash.dialogue` turns `- item` at a line start into `—⍽item`,
which breaks Markdown lists. Options: (a) in Prose for Russian; callers with Markdown disable it;
(b) in neither set, enabled explicitly. **Recommendation: (a)**: the task asks for the dash at the
start of dialogue, and the core works on plain text, not Markdown.

**13.** **Output stability policy** (section 15.1). Options: (a) the output of Prose and Interface
changes only in releases that raise the breaking number, except that patch releases fix output that
contradicts the documentation (the design, the README or the rustdoc), with a "before → after" entry
in the CHANGELOG, and a wrong design is corrected in a breaking release; (b) output improvements in
any release. **Recommendation: (a)**: build-time generators and snapshot tests of callers must not
break on `cargo update`. A related choice: `RuleSet::ALL` and the base `all` either (i) mean every
rule of the running version and stay outside the policy, with rustdoc saying so, or (ii) are frozen
at the rules of 0.1, so that a new rule is only ever enabled by name. **Recommendation: (i)**: a
frozen `ALL` that lacks rules would mislead, and callers who want stable output already have the two
sets and explicit lists.

**14.** **Public language data in version 1.** Options: (a) `LanguageData` is public with private
fields and `const fn` setters, `Language::from_data` (validated) and `Language::from_static` (for a
`static` typographer) accept compile-time tables (a downstream crate declares `en-GB` as a `const`,
section 7.7), and `LanguageBuilder` builds languages at run time from owned lists; (b) only the
compile-time `LanguageData` is public, without a run-time builder; (c) language data stays internal
in version 1 and a language is added by changing the crate. **Recommendation: (a)**: it meets
"languages are data" for users, not only for maintainers; private fields and the absence of `Copy`
let the format grow without a breaking release. Cost: the setters and builder methods are API, and
renaming one is a breaking change.

**15.** **`nbsp.phone` in Interface.** The rule glues telephone numbers typed with ordinary spaces
by a pattern (a country code or a bracketed area code, groups of digits, 10 to 15 digits). Options:
(a) Prose only; (b) both sets; (c) neither, enabled explicitly. **Recommendation: (a)**: it is a
heuristic, and Interface does not guess; callers who type numbers with U+00A0 or U+2011 get nowrap
groups in every set, and Interface callers can enable the rule.

**16.** **Word lists and norms from secondary sources.** The Russian short words (37), particles,
abbreviations and the norms "glue before a dash", "glue after a dialogue dash", "group from five
digits" and `?..` rest on Wikipedia articles; Mil'chin's handbook and GOST were not read. The
English short words (23), elisions, abbreviations and the closed dash rest on project choices and
secondary sources; the Chicago Manual of Style was not read. Options for the English short words:
(a) the 23-word list of section 7.3; (b) `a`, `an`, `the` only; (c) none. Options for the Russian
lists: (a) as in section 7.2; (b) the maintainer edits them before the first release.
**Recommendation:** English (a), Russian (a); the maintainer's edits are welcome at any time before
the first release; until 0.1.0 they are ordinary changes of the design, and after it list changes
follow the output stability policy (section 15.1).

**17.** **Digit grouping in text** (`1000000 рублей` → `1⍽000⍽000 рублей`). The task asks for
numbers formatted by the language; `format_number` does that for values (section 2.7). A text rule
does not fit the engine: a run of digits is one token, rules insert spaces only between tokens, and
English groups with a comma, which is not a space; numbers in text are also often ports, years and
codes (`port 65535`). The first draft had such a rule (`number.grouping`, in no set); it was dropped
before any release, so its id was never published. Options: (a) no grouping rule in version 1;
values are grouped by `format_number`; (b) a rule in no set that only glues a number the author
already grouped by single spaces in threes (`1 000 000` → `1⍽000⍽000`), inert where the language's
group separator is not a space; (c) full grouping in text, which needs digit runs lexed in groups
and an insertion of commas for English. **Recommendation: (a)**; (b) can be added later as a new
rule outside both sets (section 15.1).

#### Settled

The decisions of 2026-11-21 are in section 15.2.
