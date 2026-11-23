# typolish design: Rule order, idempotence and determinism

Part of the [typolish design](./README.md). Section numbers are those of the whole design.

## 4. Rule order, idempotence and determinism

### 4.1 Order table

The execution order is the constant table `ORDER` in `src/rule.rs`; `Rule::ALL` lists the rules in
this order. `RuleSet` is a bit set and the engine walks `ORDER`, so the order in which rules were
enabled has no effect. Within a phase the position in the table is also the priority when two rules
match at the same token (section 3.8).

| Step | Phase    | Rule                 | Prose | Interface | Languages (by data)        |
| ---: | -------- | -------------------- | :---: | :-------: | -------------------------- |
|    1 | symbols  | `symbols.copyright`  |   +   |     -     | all                        |
|    2 | symbols  | `symbols.arrows`     |   +   |     -     | all                        |
|    3 | symbols  | `symbols.math`       |   +   |     -     | all                        |
|    4 | ellipsis | `ellipsis.dots`      |   +   |     +     | all (forms from data)      |
|    5 | ellipsis | `ellipsis.inner`     |   +   |     -     | all                        |
|    6 | dash     | `dash.dialogue`      |   +   |     -     | ru                         |
|    7 | dash     | `dash.double-hyphen` |   +   |     -     | ru, en                     |
|    8 | dash     | `dash.spaced-hyphen` |   +   |     +     | ru, en                     |
|    9 | range    | `range.placeholders` |   +   |     +     | all                        |
|   10 | range    | `range.digits`       |   +   |     -     | all                        |
|   11 | range    | `range.context`      |   +   |     -     | ru, en                     |
|   12 | range    | `range.time`         |   +   |     -     | all                        |
|   13 | number   | `number.minus`       |   +   |     -     | all                        |
|   14 | number   | `number.times`       |   +   |     -     | all                        |
|   15 | quotes   | `quotes.apostrophe`  |   +   |     +     | all                        |
|   16 | quotes   | `quotes.pairs`       |   +   |     +     | ru, en (needs quote pairs) |
|   17 | quotes   | `quotes.primes`      |   +   |     -     | all                        |
|   18 | nbsp     | `nbsp.abbreviations` |   +   |     +     | ru, en                     |
|   19 | nbsp     | `nbsp.initials`      |   +   |     +     | ru, en                     |
|   20 | nbsp     | `nbsp.dates`         |   +   |     +     | ru, en                     |
|   21 | nbsp     | `nbsp.units`         |   +   |     +     | all                        |
|   22 | nbsp     | `nbsp.signs`         |   +   |     +     | all                        |
|   23 | nbsp     | `nbsp.phone`         |   +   |     -     | all                        |
|   24 | nbsp     | `nbsp.short-words`   |   +   |     +     | ru, en                     |
|   25 | nbsp     | `nbsp.short-length`  |   +   |     -     | ru                         |
|   26 | nbsp     | `nbsp.particles`     |   +   |     +     | ru                         |
|   27 | nbsp     | `nbsp.dash`          |   +   |     +     | ru                         |

"Languages" shows where the built-in data makes a rule active; an enabled rule without data for the
language is inert (not an error). The quote phase runs its three rules as fixed sub-steps over the
whole window (section 6): certain apostrophes and primes, roles and pairing, quotes after numbers,
the all-or-nothing decision and levels, trailing possessive apostrophes, primes. Each edit is
attributed to the rule whose behavior it is. A test checks that `ORDER`, `Rule::ALL` and this table
agree and that every rule appears once.

### 4.2 Why this order

1. **Symbols first.** `(c)` is one token and is decided before brackets and quotes are looked at;
   `->` and `<->` are decided as whole operator fragments before any dash rule looks at a `-`.
2. **Ellipsis before quotes.** `?...` becomes `?..` or `?…` first; the quote phase then sees `Dots`
   as closing punctuation in either form.
3. **Line-start dashes before dashes between words**, and both before ranges: `dash.dialogue` takes
   `- ` at the start of a line, `dash.spaced-hyphen` takes a lone `-` with spaces on both sides,
   range rules take a `-` with numbers glued on both sides. The sets do not overlap: `dash.dialogue`
   needs a line start and no operand on the left, the other two need an operand on each side; the
   order fixes which rule a report names.
4. **Ranges before the minus.** `5-10` (a digit on the left) belongs to range rules, `-5` after a
   space, a line start or `(` to `number.minus`. These sets do not overlap either.
5. **Quotes after dashes.** Quote roles use dashes as context in either form (`—"Да"`, `"no"—and`),
   so the order does not change roles; it only fixes the report.
6. **No-break spaces last.** They bind final tokens: dashes, quotes, abbreviations, units.
   `nbsp.dash` depends on the dashes produced in the dash phase, which comes earlier. Within the
   phase, rules that want the same space want the same result (a glue mark); the order only decides
   which rule a report names.

### 4.3 Invariants

- **I1. Class invariance.** An edit changes the text or the glue mark of a token, never its kind.
  Insertion and removal apply only to `Space` tokens (section 3.5).
- **I2. Stable classification.** On the output of a pass, the classifier computes the kinds and
  flags (`FROZEN`, `LABEL`) that the rules of that pass relied on, except inside a fragment that a
  removed space merged (third item) or that an inserted space split (second item). A space is
  inserted or removed only in the cases listed here, and none of them changes what a rule reads,
  except that the parts of a merged `Joined` fragment stop being whole cores (third item):
  - checks accept plain and typographic forms alike: a range with `-` or `–`, dimensions with `x`,
    `х` or `×`, a signed number with `-` or `−`, `+-` or `±`, an abbreviation as `т.д.`, `т. д.` or
    `т.⍽д.`, its last full stop also as the first of `...` or as `…`, a joined form with `...` or
    `…`, a dash fragment in any form, a year and month with `-` or `–`, a punctuation-only fragment
    with `...` or `…`, a quote at a word start as `'` or `’`;
  - an inserted space splits only a non-technical fragment, into parts that the rules read as
    before: `т.е.` → `т.` and `е.` (abbreviation parts, read as entries only by
    `nbsp.abbreviations`, which matches both forms, and as words by the other rules in both forms:
    the compact form is a compact abbreviation, a word by section 8.2); `т.д…` → `т.` and `д…` (both
    abbreviation parts, the second through its stop); `№5` → `№` and `5` (a label number in both
    forms, section 3.6, so no rule that reads a quantity or a year starts to match; the sign alone
    is a `Sign` fragment); `¶IV` → `¶` and `IV` (a Roman numeral after a sign is a label in both
    forms); `100%` → `100` and `%`, `100‰` → `100` and `‰`, `5–10%` → `5–10` and `%` with
    `PercentSpacing::Language` (ru) or `Spaced` (a percentage or a percentage range in both forms,
    section 8.2, the dash rules reading the sign after spaces with its number or range; a lone `%`
    is an `OperatorRun` by check 7, a lone `‰` falls through to check 24 and is frozen, but no rule
    edits the sign, and the space before it lies outside its fragment, section 3.7);
  - a removed space merges fragments only into a non-technical `Joined` fragment (the closed-style
    dash) or a percentage into a percentage, a range and its sign into a percentage range (the
    percent setting). The closed-style dash removes its two spaces, each a single U+0020, only
    between two plain words (section 8.4), so `word`, `—`, `word` become one `Joined` fragment
    (check 22), never a technical one; a compound word is not a plain word, because its hyphens
    would make the merged core fail check 22. On the next pass no rule matches across the removed
    spaces: `dash.spaced-hyphen` and `dash.double-hyphen` need a dash fragment with a space on each
    side (section 8.4), and a trigger word of a no-break rule must be a whole core (section 8.2), so
    `the` in `Moscow—the` triggers nothing. The edits of the first pass stay, because no rule
    changes a typographic form or a glued space (I3): the dash stays `—`, and so does the glue that
    `nbsp.short-words` set after `the` in the same pass from the classification of its input
    (section 3.6; `Moscow - the capital` → `Moscow—the⍽capital`). No rule that did not match before
    matches now: the only rule that looks inside a `Joined` fragment is `ellipsis.inner`, which
    reads only the two words around its `...` or `…`, and the merge does not change them (and
    `nbsp.units` at the sign of a percentage range, which inserts only with the settings that never
    remove that space). `100 %` → `100%` with `PercentSpacing::Attached` or `Language` (en) merges a
    percentage into a percentage.

  Each case has a fixed seed in the idempotence property (section 13.5): `Школа №5 г. Москвы`,
  `§5 ч. 2`, `¶IV`, `т.е.`, `и т.д...`, `Слово - т.е.`, `Москва – т.е. столица`, `Слово -- т.д.`,
  `5 - т.е.`, `т.е. - 5`, `Moscow - the capital`, `See https://example.com/docs - the guide`,
  `He said - "a cat"`, `100%`, `100‰`, `100 %`, `5–10%`, `5–10 %`, `100 % - норма`, `5–10% - норма`
  and `Рост - 5–10%` under every `PercentSpacing`, `100⍽%` under `Attached` and `Language` (en), en
  Closed `word⍽- word`, `Sign in - it's free`.

- **I3. Products are not triggers.** A rule changes only plain forms: straight `"` and `'`, `-`,
  `--`, `---`, a run of exactly three full stops, `(c)` `(C)` `(с)` `(С)` `(r)` `(R)` `(tm)` `(TM)`,
  `->` `<-` `<->` `>=` `<=` `!=` `+-`, `x` `X` `х` `Х` between numbers, and a single U+0020. Its
  result is a typographic form that no rule changes. The ru forms `?..`, `!..`, `?!..` are plain
  characters, but no rule changes them (section 8.6). A no-break rule changes only a `Space` of one
  U+0020 and treats any glued space (U+00A0, U+202F) as already done.
- **I4. Matching by class.** Whether a rule matches, and its `consume`, depend on token kinds, the
  texts of words and numbers, fragment kinds and flags, and the length of plain sign runs (`..`
  against `...`). Every typographic form counts as the plain form it stands for: `…` as `...`, ru
  `?..` and en `?…` as `?...`, a lone `—` or `–` as a lone `-`, `→` as `->`. `nbsp.dash` alone reads
  a typed `—` or `–` differently: with spaces it is a dash between any operands, numbers included,
  while a plain `-` counts only where `dash.spaced-hyphen` accepts it (section 8.7). No rule turns a
  refused `-` into a spaced dash, so the output of the first pass, f(x) of section 4.4, has the same
  matches. A rule gives an empty edit with the same `consume` when the pattern is already in target
  form, and also when it carries a typed typographic sign that the rule must not change (section
  8.4: typed dashes and the spaces around them; section 8.6: a typed `…`). The only exceptions are
  the structural changes of I2: where a removed space merged fragments, a match whose tokens lie in
  the merged fragment may not recur: the dash match that covered a removed space, and a no-break
  match triggered by a word that is now inside the `Joined` core (`nbsp.short-words` on `the` in
  `Moscow—the⍽capital`); their edits stay (I2, I3); where an inserted space split a fragment, the
  rules that read the parts match them with empty edits. So on f(x) no rule matches with a non-empty
  edit.
- **I5. Quotes are all or nothing per window** (section 6.6).

### 4.4 Proof sketch

Let f = P7 ∘ … ∘ P1, where Pk is phase k. The claim is f(f(x)) = f(x) for Unicode output.

1. Each phase Pk on an input y is determined by its set of matches: anchor, rule, `consume`. By I4
   the set depends only on token kinds, fragment kinds and flags, the texts of words and numbers and
   the length of plain sign runs, with every typographic form read as the plain form it stands for.
   By I1 and I2 these are the same in the input of Pk on the first pass and in f(x) on the second,
   except inside fragments split or merged by I2 (step 2): everything that later phases change is
   the form of signs and the glue of spaces. Spaces inserted by phase k are seen on the first pass
   by phases k+1 to 7, as on the second pass by all phases. Phases 1 to k−1, which first see those
   spaces on the second pass, do not change them by I3: the space is there, the pattern "space
   missing" does not match, only the pattern "already done" matches, and it gives an empty edit.
   Spaces inserted by `nbsp.*` rules and removed by the closed-style dash or the percent setting are
   covered by the second and third items of I2.
2. Hence on the second pass every match of the first pass recurs with the same `consume` and the
   same rule, except that a spaced dash produced by `dash.double-hyphen` is matched by
   `dash.spaced-hyphen` (section 8.4), and except on fragments split or merged by I2: the parts of a
   split fragment are matched in their "already done" form, and a match in a merged fragment recurs
   only if it neither covered a removed space nor was triggered by a part of the fragment as a whole
   core (I2, I4); no match has a non-empty edit, and the edits of each match lead signs to the
   target form that f(x) already has: the same phase put it there on the first pass, and later
   phases did not change it (I3). Every edit is empty.
3. Matches that the engine refused on the first pass because they touched a frozen token are refused
   again: freezing comes from the classifier, which by I2 yields the same kinds.
4. The quote phase is proved separately (section 6.11).

By induction over phases, every phase is the identity on f(x), so f(f(x)) = f(x). The sketch rests
on I1 to I5; each invariant has its own property test, and the final equality is tested on generated
strings and on the corpus, with a fixed-point oracle (section 13.5).

### 4.5 Already typeset input

Typographic characters (`«»„“”‘’`, `—`, `–`, `…`, U+00A0, `−`, `×`, `©`, `→`) are never rewritten
(I3). They take part in context: typed quotes count in nesting levels (section 6.7), and a typed `—`
or `–` with spaces between operands in Russian, numbers included, gets glue before it from
`nbsp.dash` (`Москва — столица` → `Москва⍽— столица`, `Москва – столица` → `Москва⍽– столица`,
`10 – 20` → `10⍽– 20`, section 8.7). That is a normalization of the space, not damage, and it is
idempotent. A "wrong" typographic character is not corrected: `10—20` keeps its em dash, `Что?…` in
Russian keeps its ellipsis character. The rule tables of section 13.2 have a required "typeset
input" row for every rule; those rows check that typeset input comes back as `Cow::Borrowed`
wherever no normalization of spaces applies.

### 4.6 Output kinds and idempotence

Idempotence is a property of the Unicode result:
`typeset(typeset(x, Unicode), Unicode) == typeset(x, Unicode)`. For the other outputs the guarantees
are:

- `decode(typeset(x, out)) == typeset(x, Unicode)`, where `decode` is the inverse of the encoding
  (HTML entities and nowrap spans, or string-body escapes);
- `typeset(decode(typeset(x, out)), out) == typeset(x, out)`.

Decoding is exact for every output kind, nowrap styles included (U+0000 in `CSS` is the one
exception, section 11.8): inside a nowrap element an ordinary space can only be U+00A0 glue, because
U+202F and U+2011 are written as references there and protected spans never belong to a group
(section 11.5).

Feeding encoded output back as input is not supported and not idempotent: input is plain text, so
`&nbsp;` in the input is a literal ampersand and becomes `&amp;nbsp;` in HTML output, exactly as any
other text containing `&` would. The rustdoc of `Output` states this.

### 4.7 Determinism

| Source of nondeterminism                  | Measure                                                                                                                                                                                                                                                                                                                                                                                                                               |
| ----------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Order in which rules were enabled         | `RuleSet` is a bit set; execution walks the static `ORDER`.                                                                                                                                                                                                                                                                                                                                                                           |
| Hash-map iteration                        | No hash maps in the engine; lists are sorted slices with binary search.                                                                                                                                                                                                                                                                                                                                                               |
| Floating point                            | None in rule decisions; `f64` formatting uses core, which is deterministic on every platform.                                                                                                                                                                                                                                                                                                                                         |
| Environment, OS locale, clock, randomness | Not read. A conformance test searches `src/` for `std::env`, `std::time`, `std::fs`, `std::net`.                                                                                                                                                                                                                                                                                                                                      |
| Word size, byte order                     | Positions are `usize` and carry no meaning; bit operations work on `u128`.                                                                                                                                                                                                                                                                                                                                                            |
| Unicode tables                            | Scripts: fixed ranges in code (section 5.2). Grapheme clusters: the tables of `unicode-segmentation` (Unicode 17.0.0 in 1.13.3). `char::is_alphabetic` and `is_whitespace`: std (Unicode 18.0.0 in Rust 1.100). The result is the same on every platform for the same versions of the crate, its dependencies and Rust; only characters assigned in newer Unicode versions can classify differently. The crate documentation says so. |
| Cargo features                            | No feature changes the result; the grapheme dependency is therefore mandatory, not a feature.                                                                                                                                                                                                                                                                                                                                         |
