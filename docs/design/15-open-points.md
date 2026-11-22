# typolish design: Points put to the maintainer

Part of the [typolish design](./README.md). Section numbers are those of the whole design.

### 15.5 Points put to the maintainer, all settled

No point is open. The maintainer settled all seventeen points on 2026-11-22; each records the option
chosen, and the rest of the design follows it. The other decisions of the maintainer, those of
2026-11-21 and of 2026-11-22, are listed in section 15.2. Numbers never change; settled points keep
their number and record the date of the decision.

| Item | Subject                                     | Status             | Decision                                                            |
| ---- | ------------------------------------------- | ------------------ | ------------------------------------------------------------------- |
| 1    | Language of `docs/design/`                  | Settled 2026-11-22 | American English, as in zonewright                                  |
| 2    | Repository URL                              | Settled 2026-11-22 | `https://github.com/dcl-group/typolish`                             |
| 3    | Checking the name on crates.io              | Settled 2026-11-22 | one query to the crates.io API in stage 5, none before              |
| 4    | Grouping threshold for ru                   | Settled 2026-11-22 | 1, as in CLDR 49: from 4 digits                                     |
| 5    | Group separator for ru                      | Settled 2026-11-22 | U+00A0                                                              |
| 6    | Em dash style for en-US                     | Settled 2026-11-22 | closed, `word—word`                                                 |
| 7    | Literal numeric ranges in Prose             | Settled 2026-11-22 | shape and context rules in Prose, none in Interface                 |
| 8    | Scope of an unpaired quote                  | Settled 2026-11-22 | all or nothing per paragraph                                        |
| 9    | Single quotes at the top level              | Settled 2026-11-22 | `NestedOnly`                                                        |
| 10   | Narrow no-break space                       | Settled 2026-11-22 | U+00A0 everywhere, U+202F by setting                                |
| 11   | Default spacing before `%`                  | Settled 2026-11-22 | `AsWritten`                                                         |
| 12   | Dialogue dash in Prose                      | Settled 2026-11-22 | in Prose for Russian                                                |
| 13   | Output stability policy                     | Settled 2026-11-22 | output of the sets changes only in breaking releases; `ALL` outside |
| 14   | Public language data in version 1           | Settled 2026-11-22 | public `LanguageData` and `LanguageBuilder`                         |
| 15   | `nbsp.phone` in Interface                   | Settled 2026-11-22 | Prose only                                                          |
| 16   | Word lists and norms from secondary sources | Settled 2026-11-22 | lists as in section 7                                               |
| 17   | Digit grouping in text                      | Settled 2026-11-22 | no text rule; `format_number` groups values                         |

#### Settled on 2026-11-22

**1.** **Language of `docs/design/`.** Settled 2026-11-22: (a) American English, as in zonewright,
where everything in the repository, the design included, is in American English; code refers to it
as "design section N". Rejected: (b) Russian; (c) English with a Russian summary in each stage
report.

**2.** **Repository URL.** Settled 2026-11-22: (a) the maintainer named the URL,
`https://github.com/dcl-group/typolish`; `dev/Cargo.toml` states it as `repository` (section 14.5).
Rejected: (b) leaving the field out of `Cargo.toml` until stage 5.

**3.** **Checking the name on crates.io.** Settled 2026-11-22: (b) the lead makes one query to the
crates.io API in stage 5. This decision is the maintainer's permission for that one query, which
discloses the project name to an outside service; no query is made before stage 5 (section 15.4). If
the name is taken, the maintainer chooses another. Rejected: (a) the maintainer checks; (c) the
query is made now.

**4.** **Grouping threshold for ru.** Settled 2026-11-22: (b) `min_grouping_digits = 1`, as in CLDR
49: Russian numbers are grouped from four digits (`1⍽000`, `12⍽345`), as English ones are (sections
2.7, 7.2). Rejected: (a) 2, grouping from five digits (`1000`, `10⍽000`), a Russian norm confirmed
only by a secondary source. `min_grouping_digits` changes the threshold for a typographer.

**5.** **Group separator for ru.** Settled 2026-11-22: (a) U+00A0, as in CLDR 49. Rejected: (b)
U+202F, often preferred in typesetting but with weaker font support and no named HTML entity;
`group_separator` gives it.

**6.** **Em dash style for en-US.** Settled 2026-11-22: (a) closed `word—word` (Chicago Manual of
Style, through secondary sources), joining only two plain words: next to a number, a slot, a
protected span, a technical fragment, a compound word (section 15.2, item 16) or punctuation the
author's spaces stay, so a URL never swallows the next word (section 8.4). With the default
`PercentSpacing`, (a) is the only rule of Interface that removes spaces (a non-default
`PercentSpacing` also removes the space before `%` and `‰`, section 8.2). Rejected: (b)
`word⍽— word` with glue before the dash, as in Russian (the reference implementation does this); (c)
spaced `word — word` without glue (AP style); `dash_style` gives both.

**7.** **Literal numeric ranges in Prose** (`1941-1945`, `10-20`, `12-15`). Settled 2026-11-22: (a)
in Prose `range.digits` (two ascending numbers without leading zeros) and `range.context` (a unit,
year, century or page word next to it), in Interface neither, because the task asks for en dashes in
numeric ranges and ranges of years, and technical text is protected by Interface and by backticks.
Rejected: (b) only the context rule, as the reference implementation does for Russian; (c) never in
text, only `range.placeholders` and `format_range`. Known cost in Prose: a port range `8000-8100`
written as plain text and a telephone fragment `123-4567` get an en dash (section 10.5, row 10, for
the port range; sections 10.7 and 10.8 for the telephone fragment); Interface and backticks keep
them (sections 9.2, 10.6).

**8.** **Scope of an unpaired quote.** Settled 2026-11-22: (a) all or nothing per paragraph: an
unpaired or crossing straight quote leaves every straight quote of its paragraph (of its template,
for pieces) unchanged, with a diagnostic; it never guesses which quotes form pairs and is provably
idempotent (section 6.11). Rejected: (b) partial placement, which guesses and is not idempotent; (c)
all or nothing per sentence, whose boundaries are unreliable. Cost: one typo leaves a whole
paragraph with straight quotes, reported exactly.

**9.** **Single quotes at the top level** (`She said 'hi'`). Settled 2026-11-22: (a) `NestedOnly`:
converted only inside double quotes, because a straight `'` in plain text is more often an
apostrophe or code than a quote. Rejected as the default: (b) `Everywhere`; (c) `Never`;
`single_quotes` gives both.

**10.** **Narrow no-break space.** Settled 2026-11-22: (a) U+00A0 everywhere, U+202F by the
`narrow_glue` setting: one character in the whole output, better font support, a named entity, a
simpler reverse transformation. Rejected: (b) U+202F after `№` and `§` in Russian (as the reference
implementation does); (c) U+202F also in abbreviations and between initials (as the Russian
Wikipedia suggests); the norms behind (b) and (c) were not checked against a primary source.

**11.** **Default spacing before `%`.** Settled 2026-11-22: (a) `AsWritten`: an existing space is
glued, an attached sign stays, so by default the crate neither inserts nor removes characters around
`%`. Rejected as the default: (b) `Language`, the CLDR 49 pattern (`5⍽%` in Russian, `5%` in
English); (c) `Attached`; `percent` gives them.

**12.** **Dialogue dash in Prose.** Settled 2026-11-22: (a) `dash.dialogue` is in Prose for Russian;
callers whose text has Markdown lists (`- item`) disable it. Rejected: (b) in neither set, enabled
explicitly. The task asks for the dash at the start of dialogue, and the core works on plain text.

**13.** **Output stability policy** (section 15.1). Settled 2026-11-22: (a) the output of Prose and
Interface changes only in releases that raise the breaking number, except that patch releases fix
output that contradicts the documentation (the design, the README or the rustdoc), with a "before →
after" entry in the CHANGELOG; a wrong design is corrected in a breaking release, so build-time
generators and snapshot tests of callers do not break on `cargo update`. For `RuleSet::ALL` and the
base `all`: (i) they mean every rule of the running version and stay outside the policy, with
rustdoc saying so, because a frozen `ALL` that lacks rules would mislead, and callers who want
stable output already have the two sets and explicit lists. New built-in languages and raises of the
MSRV also wait for a release that raises the breaking number (section 15.1). Rejected: (b) output
improvements in any release; (ii) `ALL` frozen at the rules of 0.1.

**14.** **Public language data in version 1.** Settled 2026-11-22: (a) `LanguageData` is public with
private fields and `const fn` setters, `Language::from_data` (validated) and `Language::from_static`
(for a `static` typographer) accept compile-time tables, and `LanguageBuilder` builds languages at
run time from owned lists (sections 2.2, 7.1). It meets "languages are data" for users, not only for
maintainers (a downstream crate declares `en-GB` as a `const`, section 7.7); private fields and the
absence of `Copy` let the format grow without a breaking release. Rejected: (b) only the
compile-time `LanguageData`; (c) language data internal in version 1. Cost: the setters and builder
methods are API, and renaming one is a breaking change.

**15.** **`nbsp.phone` in Interface.** Settled 2026-11-22: (a) Prose only: the pattern is a
heuristic, and Interface does not guess; Interface callers can enable the rule, and numbers typed
with U+00A0 or U+2011 form nowrap groups in every set. Rejected: (b) both sets; (c) neither, enabled
explicitly.

**16.** **Word lists and norms from secondary sources.** Settled 2026-11-22: (a) for English, the 23
short words of section 7.3; (a) for Russian, the lists of section 7.2. Rejected: for English, (b)
`a`, `an`, `the` only and (c) none; for Russian, (b) edits by the maintainer before the first
release. The Russian lists and the norms "glue before a dash", "glue after a dialogue dash" and
`?..` rest on Wikipedia articles (Mil'chin's handbook and GOST were not read); the English lists,
elisions, abbreviations and the closed dash rest on project choices and secondary sources (the
Chicago Manual of Style was not read). After 0.1.0 list changes follow the output stability policy
(section 15.1).

**17.** **Digit grouping in text** (`1000000 рублей` → `1⍽000⍽000 рублей`). Settled 2026-11-22: (a)
no grouping rule in version 1; values are grouped by `format_number` (sections 2.7, 8.8). A run of
digits is one token, rules insert spaces only between tokens, English groups with a comma, and
numbers in text are often ports, years and codes (`port 65535`). No release has had a rule
`number.grouping`, so that id is not reserved. Rejected: (b) a rule in no set that only glues a
number the author already grouped by single spaces in threes (`1 000 000` → `1⍽000⍽000`), inert
where the language's group separator is not a space; (c) full grouping in text, which needs digit
runs lexed in groups and commas inserted for English. (b) can still be added later as a new rule
outside both sets (section 15.1).

#### Earlier decisions

The other decisions of 2026-11-21 and 2026-11-22 are in section 15.2.
