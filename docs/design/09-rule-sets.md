# typolish design: Rule sets Prose and Interface

Part of the [typolish design](./README.md). Section numbers are those of the whole design.

## 9. Rule sets Prose and Interface

### 9.1 Purpose

- **Prose** (`RuleSet::PROSE`): running text written for reading: articles, letters, reports, books.
  Every rule of version 1. Technical fragments are protected in Prose too (classifier, section 3.6);
  what Prose adds over Interface are rules whose patterns also occur in commands and code, accepted
  because prose rarely contains unprotected commands.
- **Interface** (`RuleSet::INTERFACE`): UI strings, error messages and technical text where
  commands, paths, addresses and versions stand next to words. Only rules that carry meaning and
  cannot alter a technical token standing as its own fragment: quotes, apostrophes, the spaced dash
  between words, the ellipsis attached to what precedes it, never after `/` or `\`, ranges between
  declared values, and no-break spaces that only glue an existing space or insert one inside a known
  abbreviation or after `№`, `§`, `¶`; with a non-default `PercentSpacing`, `nbsp.units` also
  inserts or removes the space before `%` and `‰` (section 8.7).

The two sets differ only in their rule lists; settings and language data are the same for both
(section 2.4).

### 9.2 Composition

`PROSE` holds all 27 rules of version 1. It equals `ALL` in 0.1.1 and 0.1.2 only: a rule added in a
later release joins `ALL` but not the built-in sets; the sets change only in a release that raises
the breaking number (section 15.1).

`INTERFACE = PROSE − {symbols.copyright, symbols.arrows, symbols.math, ellipsis.inner, dash.dialogue, dash.double-hyphen, range.digits, range.context, range.time, number.minus, number.times, quotes.primes, nbsp.phone, nbsp.short-length}`
(13 rules): `ellipsis.dots`, `dash.spaced-hyphen`, `range.placeholders`, `quotes.apostrophe`,
`quotes.pairs`, `nbsp.abbreviations`, `nbsp.initials`, `nbsp.dates`, `nbsp.units`, `nbsp.signs`,
`nbsp.short-words`, `nbsp.particles`, `nbsp.dash`.

The sets are `const RuleSet` values in `src/rule.rs`. The rustdoc of each lists its rules, and
`tests/conformance.rs` (section 13.9) compares the constants and the rule lists of their rustdoc
with this section and with the Prose and Interface columns of the tables of sections 4.1 and 8.1.

### 9.3 What Interface leaves out and why

| Rule                 | What it would damage in technical text                                                                                                                                       |
| -------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `symbols.copyright`  | `(c)` in code and list items; the task turns symbols off in Interface                                                                                                        |
| `symbols.arrows`     | `->` in code and commands                                                                                                                                                    |
| `symbols.math`       | `>=`, `!=`, `+-` in code                                                                                                                                                     |
| `ellipsis.inner`     | `...args`, `a...b` in code                                                                                                                                                   |
| `dash.dialogue`      | `- item` list items at a line start                                                                                                                                          |
| `dash.double-hyphen` | the lone `--` of commands (`git checkout -- file`), a mandatory negative case                                                                                                |
| `range.digits`       | port ranges `8000-8100` (mandatory negative case), numbers like `12-15`                                                                                                      |
| `range.context`      | literal pairs of numbers before a unit (`ports 8000-8100 MB`); in Interface ranges come only from declared values (section 10.6)                                             |
| `range.time`         | time ranges in logs and schedules written by programs                                                                                                                        |
| `number.minus`       | `head -1`, `ls -1`                                                                                                                                                           |
| `number.times`       | `2x`, dimensions inside identifiers; the task makes the sign optional: it is a switch of the rule set (`RuleSet::INTERFACE.with(Rule::NumberTimes)`), not a separate setting |
| `quotes.primes`      | `'1'`, `"2"` as arguments; an inch mark adds nothing to a UI string                                                                                                          |
| `nbsp.phone`         | sequences of numbers in technical text (`+1 300 200 100 50`); a heuristic                                                                                                    |
| `nbsp.short-length`  | two-letter commands and names                                                                                                                                                |

Kept in Interface although one could argue: `nbsp.short-words`, `nbsp.particles`, `nbsp.initials`,
`nbsp.dates` only glue spaces and never change a token, and for Russian UI text they visibly improve
line breaking (`в⍽Москве`); `nbsp.abbreviations` and `nbsp.signs` insert a glue only inside a listed
abbreviation and after `№`, `§`, `¶`; `quotes.pairs` is required by the task's own example
(`"{name}"`); `ellipsis.dots` turns `Загрузка...` into `Загрузка…`; `dash.spaced-hyphen` needs a
lone `-` with spaces and operands on both sides, refuses a number, a range or a single letter on
each side in any mix, foreign words and technical fragments on both sides and flags on either side,
and in the closed style removes spaces only between two plain words, so it never joins a technical
token to a word.

### 9.4 Custom sets

A set is built in code (`RuleSet::INTERFACE.with(Rule::NumberTimes)`, usable in `const`), from a
string (`"interface,+number.times,-nbsp.initials".parse()`), or through `TypographerBuilder::enable`
and `disable`. The execution order never changes (section 4.1). Rule ids are permanent; changes to
the composition of Prose and Interface follow the output stability policy of section 15.1.
