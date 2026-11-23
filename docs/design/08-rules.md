# typolish design: Rules of version 1

Part of the [typolish design](./README.md). Section numbers are those of the whole design.

## 8. Rules of version 1

### 8.1 Rule table

Version 1 has 27 rules. "Languages" says where the built-in data makes a rule active (section 7);
"all" includes `Language::COMMON`. Prose and Interface are the built-in sets (section 9). The order
of the table is the execution order (section 4.1).

| Id                   | What it does                                                                                                                                                                                        | Languages                              | Settings                                                          | Prose | Interface |
| -------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | -------------------------------------- | ----------------------------------------------------------------- | :---: | :-------: |
| `symbols.copyright`  | `(c)` next to a year → `©`; `(r)`, `(tm)` after a capitalized word of two or more letters → `®`, `™`                                                                                                | all                                    |                                                                   |   +   |     -     |
| `symbols.arrows`     | a lone `->`, `<-`, `<->` between spaces → `→`, `←`, `↔`                                                                                                                                             | all                                    |                                                                   |   +   |     -     |
| `symbols.math`       | a lone `>=`, `<=`, `!=` → `≥`, `≤`, `≠`; `+-` alone or before a number → `±`                                                                                                                        | all                                    |                                                                   |   +   |     -     |
| `ellipsis.dots`      | exactly `...` at a word end or alone (spaces, line breaks and window edges are boundaries) → `…`; `?...`, `!...`, `?!...` → the language's forms                                                    | all                                    |                                                                   |   +   |     +     |
| `ellipsis.inner`     | `...` at a word start or between words (`...и тогда`, `да...нет`) → `…`                                                                                                                             | all                                    |                                                                   |   +   |     -     |
| `dash.dialogue`      | `- ` or `-- ` at a line start before a word or an opening quote → `—` and glue after it                                                                                                             | ru                                     |                                                                   |   +   |     -     |
| `dash.double-hyphen` | a lone `--` or `---` between operands → a dash in the language's style                                                                                                                              | ru, en                                 | `dash_style`                                                      |   +   |     -     |
| `dash.spaced-hyphen` | a lone `-` between operands → a dash in the language's style; not when each side is a number, a range or a single-letter word, nor between foreign words or technical fragments, nor next to a flag | ru, en                                 | `dash_style`                                                      |   +   |     +     |
| `range.placeholders` | `{Number}-{Number}`, `{Date}-{Date}` → range dash                                                                                                                                                   | all                                    |                                                                   |   +   |     +     |
| `range.digits`       | a core `D+-D+`: exactly two numbers, no leading zeros, left smaller than right → range dash                                                                                                         | all                                    |                                                                   |   +   |     -     |
| `range.context`      | `N-N` (integers, decimals, Roman numerals, `YYYY-YY`) with a unit, year or century word after it, or a page word before it → range dash                                                             | ru, en                                 | `add_units`, `remove_units`                                       |   +   |     -     |
| `range.time`         | `HH:MM-HH:MM` → `HH:MM–HH:MM`                                                                                                                                                                       | all                                    |                                                                   |   +   |     -     |
| `number.minus`       | a core `-D+` (with an optional decimal part and percent sign) after a space, a line start or `(` → minus sign                                                                                       | all                                    | `minus_sign`                                                      |   +   |     -     |
| `number.times`       | `1920x1080` → `1920×1080`; `10 x 5` → `10 × 5` (`x`, `X`, `х`, `Х`; first factor without a leading zero)                                                                                            | all                                    |                                                                   |   +   |     -     |
| `quotes.apostrophe`  | `'` between letters, after a digit or a slot before a letter, elisions, `'90s`, trailing possessive (`dogs'`) → `’`                                                                                 | all (elisions, decade, possessive: en) |                                                                   |   +   |     +     |
| `quotes.pairs`       | straight quotes → pairs by level, quotes around slots; an unpaired quote stays and is reported                                                                                                      | ru, en                                 | `quote_pairs`, `single_quotes`                                    |   +   |     +     |
| `quotes.primes`      | a quote after a number that is not a closing quote → `″`, `′`; `5'10` → `5′10`                                                                                                                      | all                                    |                                                                   |   +   |     -     |
| `nbsp.abbreviations` | glue inside listed abbreviations (inserting a missing space: `т.е.` → `т.⍽е.`) and to the next word where the entry says                                                                            | ru, en                                 | `narrow_glue`                                                     |   +   |     +     |
| `nbsp.initials`      | `А. С. Пушкин`, `Пушкин А. С.`, `J. R. R. Tolkien` → glue                                                                                                                                           | ru, en                                 | `narrow_glue`                                                     |   +   |     +     |
| `nbsp.dates`         | day and month, month and day; a year and `г.`/`гг.`; a century and `в.`/`вв.` → glue                                                                                                                | ru, en                                 |                                                                   |   +   |     +     |
| `nbsp.units`         | a number (literal, slot or range) and a unit → glue; percent by setting                                                                                                                             | all                                    | `add_units`, `remove_units`, `percent`                            |   +   |     +     |
| `nbsp.signs`         | `№ 5` and `№5` → `№⍽5`; the same for `§` and `¶`                                                                                                                                                    | all                                    | `narrow_glue`                                                     |   +   |     +     |
| `nbsp.phone`         | the spaces inside a telephone number → glue                                                                                                                                                         | all                                    |                                                                   |   +   |     -     |
| `nbsp.short-words`   | glue after a word of the language's list                                                                                                                                                            | ru, en                                 | `add_short_words`, `remove_short_words`, `short_words_any_script` |   +   |     +     |
| `nbsp.short-length`  | glue after any word of the language's script of at most N letters                                                                                                                                   | ru                                     | `short_word_length`, `short_words_any_script`                     |   +   |     -     |
| `nbsp.particles`     | glue before a particle of the language's list                                                                                                                                                       | ru                                     | `add_particles`, `remove_particles`                               |   +   |     +     |
| `nbsp.dash`          | glue before a spaced dash between operands: a produced or typed `—`, or a typed `–`                                                                                                                 | ru (style `SpacedGlued`)               | `dash_style`                                                      |   +   |     +     |

Prose: 27 rules (every rule of version 1). Interface: 13 rules.

### 8.2 Common conditions

Terms: a **word** is a fragment whose core has kind `Word`, whatever its number of tokens (`word`,
`it's`, `don't`, `90's`, `5'10`, `Brand(tm)`; check 23 and the special cases of section 3.6), and a
one-word `Dictionary` core, a compact abbreviation and a compound word count as one (below); a
**number** is a `Number` fragment or a `Number` slot; an **operand** is a word, a number, a range (a
`Range` or `YearMonth` core), a slot, a protected span or a technical fragment (section 3.6). A
**Roman numeral** is defined in section 3.6 (Latin capitals in the standard form, value 1 to 3999).
A `Punctuation` fragment (wrapper tokens only, such as a lone `...`, section 3.6) is not an operand.
A **date core** is a `Version` core (check 13 of section 3.6) of three numbers in the shape
`D.M.YYYY` or `D.M.YY`, with the day 1 to 31 and the month 1 to 12, leading zeros allowed
(`21.11.2026`, `21.11.26`, `01.02.26`); the shape decides, so `1.2.27` is one and `0.2.27` is not.
Two kinds of number name or measure something other than a quantity:

- A **label number** follows a sign (`№5`, `№ 5`, `§⍽12`, `№ {n}`, and a Roman numeral such as `¶IV`
  or `¶ IV`; flag `LABEL`, section 3.6).
- A **percentage** is a number with `%` or `‰`, attached (`5%`) or after spaces (`5 %`, `5⍽%`). A
  range with the sign attached to its last side (`5–10%`, `{a}–{b}%`: the two sides of check 17
  joined by `–`, a `Joined` core by check 22 of section 3.6) is a **percentage range**: it counts as
  a range, and as a percentage, in the same way; after spaces (`5–10 %`) the range is a `Range` core
  and the sign its percent sign.

Both count as numbers everywhere (operands, the "numbers on both sides" refusal of the dash rules; a
percent sign after spaces is read there with its number, label number or range, so `100 % - норма`
reads as `100% - норма`; and the left factor of a lone `x` of `number.times`, so `10 % x 5` reads as
`10% x 5`), except in the rules that read a quantity or a year: `nbsp.units` (other than its percent
handling), `nbsp.dates`, the year of `symbols.copyright`, the context of `range.context`, the
"directly after a number" exception of `nbsp.abbreviations` and the digit groups of `nbsp.phone`.
There a label or a percentage is not a number, in both of its forms, so the space that `nbsp.signs`
or the percent setting inserts or removes changes nothing for them (invariant I2). `nbsp.signs`
itself glues a label to its sign. So `Школа №5 г. Москвы` reads `г.` as a city, not a year:
`Школа №⍽5 г.⍽Москвы`.

A **compound word** is a `Compound` core (check 21 of section 3.6: letters or digits joined by
single hyphens, `кто-то`, `что-то`, `tls-server`, `mother-in-law`, `25-й`). It is not a technical
fragment: only its hyphens are frozen, and no rule changes them (section 8.4). Wherever a rule reads
a word it reads a compound word as one: an operand of the dash rules (`кто-то - что-то` →
`кто-то⍽— что-то`, section 8.4), the operand after a short word (`в⍽tls-server`), the word before a
particle (`кто-то⍽же`), a word inside quotes (`«кто-то»`); a list compares it as a whole core, as
any trigger word. Its script is the script of its `Word` tokens (section 5.2): `кто-то` and `25-й`
are Cyrillic words, `tls-server`, `agent-ca`, `db-01` and `mother-in-law` Latin words; a compound
word with `Word` tokens of two scripts (`IT-отдел`) is a word of each of them, so it is not a word
outside the language's script in Russian or in English text; one without a `Word` token
(`123-45-67`, `{n}-{d}`) has no script and counts as a word outside the language's script in every
language, so the refusal of section 8.4 keeps `123-45-67 - 123-45-68` and ru `cat - 123-45-67`
unchanged (`AmbiguousHyphen`), while ru `123-45-67 - звоните` gets the dash (`123-45-67⍽— звоните`).
A compound word is never a single-letter word, and it is not a plain word of the closed dash style
(section 8.4).

Conditions that hold for every rule:

- A fragment of kind `Dictionary` (check 0 of section 3.6) whose core is one `Word` token (`in`,
  `min`, `А`, `с`, `May`) counts as a fragment of kind `Word` wherever a rule reads a fragment kind
  (`Closed`, section 8.4); only the rules that look its core up in a list (`nbsp.units`,
  `nbsp.abbreviations`, `nbsp.dates`, `range.context`) read it as a dictionary entry.
- A **compact abbreviation** is a `Dictionary` core of two or more consecutive parts of an
  abbreviation of the language that all end in a full stop, written together (`т.е.`, `т.д.`, `Т.е.`
  by the relaxation of section 7.1). It counts as a word, as the one-word parts that
  `nbsp.abbreviations` splits it into do (`т.е.` → `т.⍽е.`), so the dash rules read it alike in both
  forms (`Слово - т.е.` → `Слово⍽— т.⍽е.` in one pass, invariant I2 of section 4.3). Where a rule
  asks whether a word has one letter it reads the part next to the other side: the last part on the
  left of a dash, the first part on the right (`5 - т.е.` is refused as `5 - т. е.` is).
- No rule changes letters or digits inside `Word` and `Number` tokens. The letter-like replacements
  are `(c)`, `(r)`, `(tm)` in either case and the Cyrillic `(с)`, `(С)` (kind `LegalMark`) and `x`,
  `X`, `х`, `Х` between numbers (kind `Times`).
- A no-break rule glues only a single U+0020 between two fragments on one line. Tabs, runs of
  spaces, line breaks and spaces at a window boundary are never glued.
- A **pattern space** is a `Space` token of exactly one character: U+0020, or a glued U+00A0 or
  U+202F (read as already done, invariants I3 and I4). A tab, a run of spaces, U+2009 and a frozen
  `Space` are not pattern spaces. Wherever a pattern says "a space", "one space", "single spaces" or
  "one space away" (`dash.*`, `symbols.*`, `range.context`, `number.minus`, `nbsp.*`), it needs a
  pattern space; at any other `Space` token the pattern does not match, so the rule does not fire
  and reports no diagnostic. The boundaries of `ellipsis.*` (section 8.6) are not patterns of this
  kind: there a space is any `Space` token, a tab or a run of spaces included, since those rules
  change no space. The indentation before `dash.dialogue` may be any horizontal whitespace; between
  the dash and the word stands one pattern space.
- A space is glued only if the next fragment starts with a letter, a digit, a slot (not `Other`), a
  protected span, an opening quote or the opening bracket `(` (`[` and `{` start technical notation,
  section 3.6, so `в [х]` stays), except where a rule names the next token (`nbsp.dash` before a
  dash, `nbsp.units` before a unit, `nbsp.short-words` before a sign of `signs`). Never before a
  fragment that starts with `-` (`ls -la` stays intact whatever the lists say), and never next to a
  fragment that holds an `Other` slot, on either side, whatever wraps the slot (`and "{o}"` stays;
  section 2.6).
- A trigger word is a whole core, compared as section 7.1 (Lookups) says for its list. The wrapper
  does not hide it: `(в 2026)` → `(в⍽2026)`. A word followed by closing punctuation (`и,`) triggers
  nothing.
- An 'opening quote' or a 'closing quote' in a pattern of a rule is a quote with that role by the
  local rule of section 6.3, the sign-only pair included, computed from its neighbours when the rule
  runs; a typed glyph has its own role, a straight quote that 6.3 gives neither role is neither. The
  sign-only pair belongs here because the quote phase places it: in `Слово - "" да` the `""` is an
  opening quote on the first pass as `«»` is on the second (section 4.3, I4). Where one quote ends a
  sign-only pair and starts another, the later pair wins, as in the quote phase.
- Rules that insert a space: `nbsp.abbreviations` (`т.е.`), `nbsp.signs` (`№5`), `nbsp.units` with
  `PercentSpacing::Language` (ru) or `Spaced` (`100%` → `100⍽%`). Rules that remove spaces (each
  removes only a single U+0020): `dash.spaced-hyphen` and `dash.double-hyphen` with the `Closed`
  style, only between two plain words (section 8.4), and `nbsp.units` with
  `PercentSpacing::Language` (en) or `Attached`. No other rule adds or removes a character, and
  these cases change what the rules read only as invariant I2 of section 4.3 lists.

### 8.3 Quotes

`quotes.apostrophe`, `quotes.pairs` and `quotes.primes` are described in section 6: candidates, step
1, roles, families, quotes after numbers, all or nothing per window, levels, single quotes, slots.
Diagnostics: `UnmatchedQuote`, `CrossingQuotes`, `AmbiguousQuote`.

### 8.4 Dashes

**`dash.spaced-hyphen`.** A dash fragment whose token is a single `-`, `–` or `—` (section 3.6; a
run `--` or `---` belongs to `dash.double-hyphen`, and this rule does not match it, not even with an
empty edit), with one space on each side (not a line break), an operand or closing punctuation
(`, . ! ? … ) »`, a closing quote, or a prime `′` `″`, which `quotes.primes` writes after a number
in place of a straight quote that has the closing role there, so `5'10" - x` and `5′10″ - x` read
alike, I4 of section 4.3) on the left, and an operand, an opening quote or an opening bracket on the
right → em dash. It does not fire, and reports `AmbiguousHyphen`, when:

- each side is a number, a range or a single-letter word, in any mix (`10 - 20`, `5 - 3 = 2`,
  `{a} - {b}`, `a - b`, `x - y`, `5 - x`, `x - 5`, `{a} - x`, `5–10 - 20`): a range, a minus, a
  formula or a list item, which the rule cannot tell apart; a `Number` slot counts as a number, a
  `Word` slot is not a single-letter word;
- a flag or another fragment that starts with `-` stands on either side (`diff -u - file.txt`,
  `tar -cf - -C dir`, `Use - --force`): the `-` is most likely an argument (standard input, the end
  of options); a signed number counts with `-`, with `−` and with the minus sign of the settings
  (`-5`, `−5`; the form that `number.minus` writes), so the minus of the first pass changes nothing
  for the second (I4 of section 4.3);
- words outside the language's script or technical fragments stand on both sides, in any mix
  (`cat - file`, `cat - /etc/hosts` and `tls-server - agent-ca` in Russian text,
  `host:8080 - 10.0.0.1`): most often a command line. A technical fragment counts as a foreign word
  here, in every language; a technical fragment on one side and a word of the language on the other
  (`Сервис слушает host:8080 - проверьте`) gets the dash. A compound word counts by the script of
  its letters, as any word (section 8.2): in Russian text `кто-то - что-то` gets the dash, while
  `tls-server - agent-ca` (Latin compound words) does not, as `cat - file` does not; a compound word
  without letters (`123-45-67`) counts as a foreign word in every language.

Spacing by `DashStyle`: `SpacedGlued` (ru) and `Spaced` keep the author's spaces (`nbsp.dash` glues
the one before the dash under `SpacedGlued`). `Closed` (en) removes both spaces (`word—word`) only
when each of them is a single U+0020 and only between two plain words; with a glued space on either
side the rule fires and `Closed` keeps the author's spaces, as `Spaced` does (`word⍽- word` →
`word⍽— word`); next to a tab or a run of spaces the rule does not fire (a pattern space, section
8.2), and the hyphen stays. Two words are plain when the cores on both sides are words of letters
(kind `Word`, or a one-word `Dictionary` core, section 8.2; letters with their marks and inner
apostrophes only, no digit, sign or legal mark), and neither has a wrapper on the side facing the
dash. A compound word is not a plain word: merged with the dash, its hyphens would give a core that
check 22 does not accept (`mother-in-law—father-in-law` is not a `Joined` core and would be frozen
by check 24), against invariant I2. Next to a number, a slot, a protected span, a technical
fragment, a compound word, or a wrapper on the inner side (a closing quote or bracket, `, . ! ? …`
on the left, an opening quote or bracket on the right), `Closed` keeps the author's spaces, as
`Spaced` does: `See https://example.com/docs - the guide` →
`See https://example.com/docs — the guide`, `He said - "a cat"` → `He said — “a cat”`,
`Hello, - he said` → `Hello, — he said`, `mother-in-law - father-in-law` →
`mother-in-law — father-in-law`. So a merge always gives a `Joined` fragment (invariant I2), a URL
or a flag never swallows the next word (autolinkers that cut a link at the next space would take
`—the` into it), and `to_plain` can restore the spaces. Typed `—` and `–` are never changed, and no
space around them is removed. On such a typed dash the rule matches with an empty edit and the same
`consume` (the tokens a match closes to the other rules of its phase, section 3.7) when its context
holds; it raises `AmbiguousHyphen` only on a plain `-`. A `Joined` or a `Dimensions` core is not an
operand (section 8.2): a dash whose neighbour on one side is such a core, with no wrapper between
them, is matched by no dash rule and gets no glue from `nbsp.dash`, and no diagnostic is raised
(`Moscow—the - capital`, `5 - x—Moscow` and ru `1920x1080 — монитор` stay). Were a `Joined` core an
operand, a `-` refused next to a single letter would fire on the next pass once a merge had made
that letter part of a `Joined` core (`5 - x - Moscow` → `5 - x—Moscow` → `5 — x—Moscow`), against
invariant I2.

**`dash.double-hyphen`** (Prose). The same for a lone `--` or `---`, with the same refusals:
`npm test -- --watch` stays. A typed `—` or `–` is matched by `dash.spaced-hyphen` (with an empty
edit), never by this rule, so on the output of this rule a dash that keeps its spaces (`Spaced`,
`SpacedGlued`, and `Closed` where it keeps the author's spaces) is matched by `dash.spaced-hyphen`
with the same `consume`, while a dash that `Closed` joined to its words (`word—word`) stands inside
a `Joined` fragment and is matched by no dash rule (invariant I2). A lone `--` or `---` without an
operand on each side (a line that is just `--`) is not changed by this rule. At the start of a line,
`-- word` is taken by `dash.dialogue` where the language has it (ru, Prose): `-- file` → `—⍽file`;
in English, and in Interface, it stays.

**`dash.dialogue`** (Prose, languages with `dialogue`). At the start of the text or of a line, after
optional indentation: `-` or `--`, one space, then a word or an opening quote → `—`, and the space
after it is glued. A typed `—` or `–` there counts as a `-` (section 4.3, I4): the dash stays and
only a single U+0020 after it is glued (`— Привет` → `—⍽Привет`), the normalization of a space that
`nbsp.dash` also makes (section 4.5). A `Word` or `Other` slot counts as the word (section 8.10) and
so does a protected span (section 2.6); a `Number` or `Date` slot does not. Before an `Other` slot
the dash is set and the space stays, since no glue sticks to an `Other` slot (section 8.2):
`- {o} пришёл` → `— {o} пришёл`, `- {w} пришёл` → `—⍽{w} пришёл`. Not in Interface: a line starting
with `- ` in UI text is a list item. Settled in section 15.5, item 12.

Outputs show the rules named in the last column (with `quotes.pairs` where quotes appear); in the
full sets other no-break rules add glue too (`Moscow—the⍽capital`, `“a⍽cat”`). The glue after `the`
comes from the classification of the input of the pass, in which `the` is still a whole core
(section 3.6); an already closed `Moscow—the capital` gets none, because there `the` is inside a
`Joined` fragment and triggers nothing (section 4.3, I2).

| Language | Input                           | Output                          | Mechanism                                                         |
| -------- | ------------------------------- | ------------------------------- | ----------------------------------------------------------------- |
| ru       | `Москва - столица`              | `Москва⍽— столица`              | `dash.spaced-hyphen`, `nbsp.dash`                                 |
| en       | `Moscow - the capital`          | `Moscow—the capital`            | style `Closed`                                                    |
| en       | `Open host:8080 - now`          | `Open host:8080 — now`          | spaces kept by technical text                                     |
| en       | `He said - "a cat"`             | `He said — “a cat”`             | spaces kept by a quote                                            |
| ru       | `Привет, - сказал он.`          | `Привет,⍽— сказал он.`          | closing punctuation allowed on the left; `nbsp.dash`              |
| ru       | `- Привет!\n- Пока.` (Prose)    | `—⍽Привет!\n—⍽Пока.`            | `dash.dialogue`                                                   |
| ru       | `-- file` (Prose)               | `—⍽file`                        | `dash.dialogue`                                                   |
| ru       | `Запустите -- сейчас.` (Prose)  | `Запустите⍽— сейчас.`           | `dash.double-hyphen`, `nbsp.dash`                                 |
| ru       | `кто-то - что-то`               | `кто-то⍽— что-то`               | `dash.spaced-hyphen`, `nbsp.dash`; compound words of the language |
| en       | `mother-in-law - father-in-law` | `mother-in-law — father-in-law` | style `Closed` keeps the spaces next to a compound word           |
| ru       | `кто-то, tls-server`            | unchanged                       | no rule touches an inner hyphen                                   |
| ru       | `10 - 20 штук`, `5 - 3 = 2`     | unchanged, `AmbiguousHyphen`    | numbers on both sides                                             |
| en       | `x - 5`, `5 - x`                | unchanged, `AmbiguousHyphen`    | a number and a single letter                                      |
| ru       | `Выполните cat - file`          | unchanged, `AmbiguousHyphen`    | foreign words on both sides                                       |
| ru       | `tls-server - agent-ca`         | unchanged, `AmbiguousHyphen`    | Latin compound words: foreign words on both sides                 |
| ru       | `Выполните diff -u - file.txt`  | unchanged, `AmbiguousHyphen`    | a flag next to the `-`                                            |
| en       | `tar -cf - -C dir`              | unchanged, `AmbiguousHyphen`    | flags                                                             |
| ru       | `npm test -- --watch` (Prose)   | unchanged, `AmbiguousHyphen`    | a flag next to the `--`                                           |

The inner hyphen of a compound word or identifier (`tls-server`, `кто-то`, `mother-in-law`, `db-01`)
is changed by no rule: dash rules need spaces on both sides, range rules need numbers on both sides,
and the classifier freezes the hyphens of compounds (check 21 of section 3.6). The compound word
itself is an ordinary word for the rules, with the script of its letters (section 8.2).

### 8.5 Ranges

- **`range.placeholders`** (both sets): a `-` without spaces between two `Number` slots or two
  `Date` slots → the range dash of the language. The caller declared both sides as values, so this
  is a range of values. With spaces (`{a} - {b}`) nothing changes: it may be a subtraction. A slot
  next to a literal (`{a}-5`) is not edited by any range rule.
- **`range.digits`** (Prose): a core `D+-D+` (check 17) with exactly two numbers, both without
  leading zeros, the left smaller than the right → `D–D`: `1941-1945`, `10-20`, `5-10`. Not
  `2026-11` (a year and month, check 12), `1-2-3` (three numbers), `01-12` (leading zero), `20-10`
  (descending), `10-10` (equal); none of these reports a diagnostic.
- **`range.context`** (Prose): `N-N` where N is an unsigned integer, a decimal with the language's
  separator or a Roman numeral, left smaller than right where both sides are numbers or both are
  Roman numerals (by value, section 3.6), or `YYYY-YY` with the tail greater than the last two
  digits of the year, with a context: a unit, a year word or a century word as the next fragment, or
  a range lead word as the previous one, one pattern space between them (section 8.2). `1941-45 гг.`
  → `1941–45⍽гг.`, `XIX-XX вв.` → `XIX–XX⍽вв.`, `1,5-2,5 кг` → `1,5–2,5⍽кг`, `стр. 9-12` →
  `стр.⍽9–12`. A `YearMonth` core (check 12) is read as `YYYY-YY` here: `в 2010-11 гг.` →
  `в⍽2010–11⍽гг.`, `сезон 2011-12 годов` → `сезон 2011–12 годов`, while `2026-11 гг.` (tail not
  greater) and `2010-11` without a year word stay. Leading zeros still refuse (`стр. 010-020`
  stays). This path also catches the shapes that `range.digits` refuses on purpose (Roman numerals,
  decimals, two-digit year tails); where both rules match (`1941-1945 гг.`) the edits are equal and
  the report names `range.digits`, the earlier rule. An ambiguous unit counts as context only where
  `nbsp.units` would glue it (before punctuation, a line break or the end), so
  `Порты 8000-8100 с балансировкой` has no context for this rule (`range.digits` still gives
  `8000–8100` in Prose, section 10.6). A label range takes no context (`§§ 5-7 г.`, section 8.2).
- **`range.time`** (Prose): `HH:MM[:SS]-HH:MM[:SS]` (check 5) → the same two times with `–` in place
  of the hyphen (`10:00-12:30` → `10:00–12:30`, `10:00:15-12:30:45` → `10:00:15–12:30:45`).

The dash comes from `NumberData::range_dash` of the language, the same field `format_range` uses.
The rule for port ranges such as `8000-8100` is section 10.6.

### 8.6 Ellipsis

- **`ellipsis.dots`** (both sets): exactly three full stops at the end of a word, followed by a
  space, a line break, the end of the window, closing punctuation or a closing quote, or forming a
  fragment of their own whose neighbour on each side is a space, a line break or a window boundary
  (`... и тогда`, `Слово ...`, `Подождите...\nДальше`) → `…`. After `?`, `!` or `?!` → the forms of
  `LanguageData::ellipsis` (ru `?..`, `!..`, `?!..`; en `?…`, `!…`, `?!…`). Two full stops
  (`А если..`, `../x`, `0..5`) and four or more (`....`, leaders in tables of contents) never
  change. A typed `…` or `?…` never changes. On its products and on typed forms (`…`, ru `?..`,
  `!..`, `?!..`, en `?…`) the rule matches with an empty edit. Digits on both sides (`-5...5`) never
  change.
- **`ellipsis.inner`** (Prose): `...` at the start of a word after a space or a line start
  (`...и тогда`) and between two words without spaces (`да...нет`) → `…`. A `Word` or `Other` slot
  counts as a word (section 8.10) and so does a protected span (section 2.6): `{a}...{b}` →
  `{a}…{b}`. Not in Interface: `...args` and `a...b` occur in code.

Examples: `Подождите...` → `Подождите…`; `Подождите...\tДальше` → `Подождите…\tДальше` and
`Подождите...  Дальше` → `Подождите…  Дальше` (any `Space` token is a boundary, section 8.2);
`... и тогда` → `… и тогда`; ru `Что?...` → `Что?..`; en `What?...` → `What?…`; `Что?…` unchanged.
`и т.д...` → `и т.д…`: the run is replaced as a whole, its first full stop included, and
`nbsp.abbreviations` reads the `…` as the full stop of the abbreviation (section 8.7).

### 8.7 No-break spaces

- **`nbsp.abbreviations`.** Multi-part abbreviations of the list (`Inner`): the spaces inside are
  glued, a missing space after a full stop is inserted, and a leading word that belongs to the entry
  (`и`, `в`, `до`) is part of the pattern: `т.е.` → `т.⍽е.`, `и т. д.` → `и⍽т.⍽д.`, `и т.п.` →
  `и⍽т.⍽п.`. The full stop that ends an entry may be the first full stop of a `Dots` token in any
  form: `и т.д...` → `и⍽т.⍽д…` (the ellipsis phase has already replaced the run), `и т. д…` →
  `и⍽т.⍽д…`. Entries with `Next` glue the space to the following operand (`см.⍽рис.⍽5`,
  `ул.⍽Ленина`, `Mr.⍽Smith`); `NextCapitalized` only before a capitalized word (`г.⍽Москва`). A
  `Next` or `NextCapitalized` entry directly after a number, a range, a `YearMonth` core, a date
  core (section 8.2) or a `Number` or `Date` slot does not bind forward: it is a unit or a year with
  a sentence full stop (`5 ч. Потом`, `5–10 ч. Потом`, `2026 г. Затем`, `1990–2000 г. Затем`,
  `21.11.2026 г. Затем`, `21.11.26 г. Затем`, `{d} г. Затем`). A label number or label range and a
  percentage do not count here (section 8.2). A label number is not a number here:
  `Школа №5 г. Москвы` → `Школа №⍽5 г.⍽Москвы`, `§5 ч. 2` → `§⍽5 ч.⍽2`. With `narrow_glue` for
  abbreviations, `Narrow` goes only to the spaces between the parts of an entry (`т.⎵д.`); the space
  after a leading word (`и⍽т.`) and the forward glue of `Next` entries are always `NoBreak`.
- **`nbsp.initials`.** `И. О. Фамилия`, `И. Фамилия`, `Фамилия И. О.`, `Фамилия И.`: an initial is
  one capital letter of the language's script with a full stop (up to three); a run of four or more
  initials is not a match anywhere in the run, and the rule glues none of its spaces (no partial
  match, no single-initial match inside it); a surname is a capitalized word of the language's
  script of at least two letters. The spaces inside the pattern are glued: `А. С. Пушкин` →
  `А.⍽С.⍽Пушкин`, `J. R. R. Tolkien` → `J.⍽R.⍽R.⍽Tolkien`. A compact `А.С.` gets no inserted space;
  only the space before the surname is glued. Two or more initials, the reversed form and a single
  initial before a surname are glued wherever they stand (`от⍽И.⍽Петрова`, `Автор: А.⍽Пушкин`,
  `сказал А.⍽Петров`). A capital letter with a full stop that ends a sentence (a one-letter label,
  en `I`) before a capitalized word gets the same glue (`вариант А.⍽После`): the rule does not look
  for sentence ends (section 15.2, item 11), and the glue only forbids a line break there (section
  10.8). Where initials stand between two capitalized words, the forward form takes them and the
  word before stays apart (`Иван А.⍽Петров`, `Пушкин А.⍽С.⍽Иванов`).
- **`nbsp.dates`.** By `month_position`: `AfterDay` (ru) glues a day number 1 to 31 (or a `Number`
  slot) before a month form (`21⍽ноября`); `BeforeDay` glues a month form before a day number;
  `Both` (en) glues both orders (`November⍽21`, `21⍽November`). With year and century words (only in
  languages that have them): a number, a range, a `YearMonth` core, a date core (section 8.2; the
  core stays frozen, only the space after it is glued) or a `Number`/`Date` slot before a year word
  ending in `.` (`2026⍽г.`, `1990–2000⍽гг.`, `2010-11⍽гг.`, `21.11.2026⍽г.`, `21.11.26⍽г.`); a Roman
  numeral, a number or a range of them before a century word ending in `.` (`XIX⍽в.`, `XIX–XX⍽вв.`).
  Labels and percentages are not numbers here (section 8.2).
- **`nbsp.units`.** A number (not a label or a percentage), a `Number` slot or a range, one space,
  then a unit of the list (longest match, case-sensitive, followed by a non-letter and non-digit) →
  glue: `5⍽МБ`, `30⍽с.`, `12⍽дн.`, `5–10⍽кг`, `{n}⍽дн.`, `5⍽ГБ/с`, `60⍽км/ч`. An ambiguous unit
  (`с`, `in`) is glued only before punctuation, a line break or the end: `Ждать 30⍽с.`, but
  `30 с друзьями` stays. Only existing spaces: `5MB` is an identifier and stays. `%` and `‰` follow
  `PercentSpacing`: `AsWritten` (default) glues an existing space and leaves an attached sign;
  `Language` applies the CLDR pattern (`NumberData::percent_spaced`; ru `100%` → `100⍽%`, en `100 %`
  → `100%` (only a single U+0020 is removed)); `Attached` removes a single U+0020 between the number
  and the sign; a U+00A0 or U+202F typed by the author, a tab or a run of spaces stays (section
  5.3); `Spaced` glues or inserts. A range takes the setting as a number does (`5–10%` → `5–10⍽%`
  with ru `Language` or `Spaced`, `5–10 %` → `5–10%` with `Attached`). Settled in section 15.5,
  item 11.
- **`nbsp.signs`.** `№`, `§`, `¶` before a number (its label number, section 8.2), a Roman numeral
  or a slot (not `Other`): a space is glued, a missing space is inserted (`№5` → `№⍽5`, `¶IV` →
  `¶⍽IV`; the attached forms are words by check 23 of section 3.6, and the sign alone is a `Sign`
  fragment). Doubled signs (`№№ 5`, `§§ 3`) count as one.
- **`nbsp.phone`** (Prose). A run of fragments separated by single spaces that starts with a country
  code `+D` (one to three digits) or with an area code in brackets `(DDD)` (optionally after a
  one-digit trunk prefix: `8 (800)`), followed by one to four further fragments, each a group of two
  to four digits, a bracketed code `(DDD)` or a hyphenated group (`123-45-67`, `123-4567`; a
  `Compound` or a `Range` fragment, counted as one; a percentage is not a group, in both of its
  forms, section 8.2: `+7 495 123-45-67 100%` and `… 100 %` glue the telephone number only), with 10
  to 15 digits in total. Every space inside is glued: `+7 495 123-45-67` → `+7⍽495⍽123-45-67`,
  rendered in nowrap output as `<span class="nowrap">+7 495 123-45-67</span>`. This rule changes no
  hyphen; in Prose `range.digits` has already turned the hyphen of a two-group fragment into an en
  dash (`+7 495 123-4567` → `+7⍽495⍽123–4567`, section 10.8), and the pattern reads both forms
  (invariant I4). Not in Interface: the pattern is a heuristic (section 15.5, item 15). A number
  typed with U+00A0 or U+2011 by the caller forms a nowrap group in every set.
- **`nbsp.short-words`.** A word of the list (no case), one space, then an operand, an opening quote
  or bracket, or a sign of the language's `signs` (`№`, `§`, `¶`) → glue. Chains glue whole:
  `и в Москве` → `и⍽в⍽Москве`. With `short_words_any_script(false)` (default) the word must be in
  the language's script.
- **`nbsp.short-length`** (Prose, ru). The same for any word of the language's script of at most
  `short_word_length` letters (`Он⍽пошёл`, `Мы⍽и⍽они`). Latin `ls`, `go`, `db` in Russian text do
  not count. Only letters are counted, and a compound word is a word (section 8.2): `25-й⍽этаж`.
- **`nbsp.particles`.** A word or a slot (not `Other`), one space, then a particle of the list (a
  whole core, section 8.2) → glue: `сказал⍽бы`, `Знаешь⍽ли`, `{name}⍽же`, `кто-то⍽же` (a compound
  word is a word, section 8.2).
- **`nbsp.dash`.** The space before a dash fragment that stands for a spaced dash (a produced or
  typed `—`, or a typed `–`, section 8.4) in the context of `dash.spaced-hyphen` (an operand or
  closing punctuation on the left, an operand, an opening quote or an opening bracket on the right,
  a pattern space on each side, section 8.4) is glued when the style is `SpacedGlued`:
  `Москва — столица` → `Москва⍽— столица`, `Москва – столица` → `Москва⍽– столица`. The refusals of
  `dash.spaced-hyphen` do not apply here: a typed dash gets the glue between any operands, numbers
  included (`10 – 20` → `10⍽– 20`), while a plain `-` that `dash.spaced-hyphen` refused is not a
  dash and gets none.

In the tables and examples of sections 8.5 to 8.9 an output shows the edits of the rules named with
it, as in section 8.4; a row marked Prose or Interface shows the output of the whole set. In the
full sets other rules add their glue and ranges too (Prose gives `Об⍽этом сказал А.⍽Петров.` for the
row on initials after any word).

| Language | Input                                 | Output                            | Rule                                                |
| -------- | ------------------------------------- | --------------------------------- | --------------------------------------------------- |
| ru       | `в 2026 году`                         | `в⍽2026 году`                     | short-words                                         |
| ru       | `Как бы то ни было`                   | `Как⍽бы то⍽ни⍽было`               | particles, short-words                              |
| ru       | `и т. д.`, `и т.д.`                   | `и⍽т.⍽д.`                         | abbreviations                                       |
| ru       | `Документ № 5`, `№5`                  | `Документ №⍽5`, `№⍽5`             | signs                                               |
| ru       | `Школа №5 г. Москвы`                  | `Школа №⍽5 г.⍽Москвы`             | signs, abbreviations (label)                        |
| ru       | `§5 ч. 2`                             | `§⍽5 ч.⍽2`                        | signs, abbreviations (label)                        |
| ru       | `5 МБ`, `30 с.`, `12 дн.`             | `5⍽МБ`, `30⍽с.`, `12⍽дн.`         | units                                               |
| ru       | `100 %` / `100%`                      | `100⍽%` / `100%`                  | units, `AsWritten`                                  |
| ru       | `60 км/ч`                             | `60⍽км/ч`                         | units (dictionary before path)                      |
| ru       | `А. С. Пушкин`                        | `А.⍽С.⍽Пушкин`                    | initials                                            |
| ru       | `Об этом сказал А. Петров.`           | `Об этом сказал А.⍽Петров.`       | initials after any word                             |
| ru       | `Выберите вариант А. После этого`     | `Выберите вариант А.⍽После этого` | initials: label at a sentence end (section 10.8)    |
| en       | `I chose plan B. Then I left.`        | `I chose plan B.⍽Then I left.`    | the same                                            |
| ru       | `Выберите вариант Д. После этого`     | `Выберите вариант Д.⍽После этого` | the same; not abbreviations: `д.` matches only `д.` |
| en       | `I chose plan P. Then I left.`        | `I chose plan P.⍽Then I left.`    | the same; not abbreviations: `p.` matches only `p.` |
| ru       | `21 ноября 2026 г.`                   | `21⍽ноября 2026⍽г.`               | dates                                               |
| ru       | `Москва – столица`, `10 – 20`         | `Москва⍽– столица`, `10⍽– 20`     | dash (typed `–`, numbers too)                       |
| ru       | `+7 495 123-45-67` (Prose)            | `+7⍽495⍽123-45-67`                | phone                                               |
| ru       | `+7 495 123-4567` (Prose)             | `+7⍽495⍽123–4567`                 | phone, range.digits                                 |
| en       | `Mr. Smith`, `J. R. R. Tolkien`       | `Mr.⍽Smith`, `J.⍽R.⍽R.⍽Tolkien`   | abbreviations, initials                             |
| en       | `On November 21, in 5 MB`             | `On⍽November⍽21, in⍽5⍽MB`         | short-words, dates, units                           |
| ru       | `Он вызвал ls на сервере` (Interface) | `Он вызвал ls на⍽сервере`         | `ls` is in no list                                  |

### 8.8 Numbers in text

Text rules do not rewrite numbers: separators, decimal marks and groups in text belong to the author
(they are often codes, years, ports, versions). `format_number` formats values (section 2.7).

- **`number.minus`** (Prose): a core `-D+` or `-D+[.,]D+`, with an optional `%` or `‰` attached,
  whose fragment starts after a space, a line start or `(` → minus sign: `Минус -5 градусов` →
  `Минус −5 градусов`, `-12 °C` → `−12⍽°C`, `-5%` → `−5%`. A percentage reads alike in both of its
  forms (section 8.2), so the space that the percent setting inserts (`-5%` → `-5⍽%`) changes
  nothing for the rule (invariant I2). Not in Interface (`head -1`, `tail -5`); in Prose such
  commands are protected with backticks.
- **`number.times`** (Prose): a `Dimensions` core (check 16) → `×` in place of the letter
  (`1920x1080` → `1920×1080`, Cyrillic `х` too); a lone `x`, `X`, `х` or `Х` fragment between two
  number fragments (on the left also a label number attached to its sign, `№10`, and a number with
  its percent sign after spaces, `10 % x 5`, section 3.6), the left one without a leading zero → `×`
  with the author's spaces kept (`10 x 5` → `10 × 5`). `0x1F` (hex, check 10) and `x86` (identifier)
  never change.
- Digit grouping in text (`1000000` → `1⍽000⍽000`) is not a rule of version 1. A run of digits is
  one `Number` token, rules insert spaces only between tokens, and English groups with a comma,
  which is not a space at all; numbers in text are also often ports, years and codes (`port 65535`).
  Values are grouped by `format_number` with the same settings, so a template and its value agree
  (section 2.7; settled in section 15.5, item 17).

### 8.9 Symbols

- **`symbols.copyright`**: `(c)`, `(C)` and Cyrillic `(с)`, `(С)` → `©` only with a year next to the
  mark: a four-digit year after the mark, attached or one space away (`(c)2026`, `(c) 2026`), or one
  space before the mark (`2026 (c)`); a range of years only one space away, on either side
  (`(c) 2020-2026`, `2020-2026 (c)`). So `(c) 2026` → `© 2026`, `(c)2026` → `©2026`, `2026 (c)` →
  `2026 ©`, `2020-2026 (c)` → `2020-2026 ©`, `(c) 2020-2026` → `© 2020-2026`. A mark attached after
  a number or a range (`Section 1001(c)`, `2026(c)`, `2020-2026(c)`), or with a range attached after
  it (`(c)2020-2026`), is not a word (section 3.6) and stays: after a number it is usually a
  subsection label. A mark attached to a word (`f(c)`, a `Word` by check 23 of section 3.6) is not a
  copyright sign even when a year follows it: `f(c) 2026` stays. A label number or a percentage is
  not a year in either of its forms (section 8.2): `(c) 2026%`, `(c) 2026 %`, `(c) 2020-2026 %` and
  `№2026 (c)` stay. Otherwise `(c)` is a list item and stays (`вариант (c)`). `(tm)`, `(TM)`, `(r)`,
  `(R)` → `™`, `®` after a brand name: a capitalized word of at least two letters, attached
  (`Brand(tm)` → `Brand™`) or one space before (`Brand (r)` → `Brand ®`), and not followed by an
  operator fragment (`=`, `+`, `<`, `>=`). Otherwise it is a function argument and stays:
  `S(r) = πr²`, `f(r)`, `V(r)`, `area(r)`.
- **`symbols.arrows`**: an operator fragment that is exactly `->`, `<-` or `<->`, with a space on
  each side → `→`, `←`, `↔`. `=>`, `-->`, `<-->`, `->>` are not in the table; `a->b` is frozen.
- **`symbols.math`**: an operator fragment that is exactly `>=`, `<=` or `!=`, with a space on each
  side → `≥`, `≤`, `≠`; `+-` as such an operator fragment, with a space on each side, or as the sign
  of a number (a `Number` core with the sign `+-`, check 15) → `±` (`a +- b` → `a ± b`, `+-5` →
  `±5`). `>>`, `<<`, `==`, `>>=` are not in the table; `a>>b` is frozen.

Examples: `x -> y` → `x → y`; `a <-> b` → `a ↔ b`; `x >= 5` → `x ≥ 5`; `a->b`, `-->`, `=>`, `a>>b`,
`x == y` unchanged.

### 8.10 Slots in rules

How each kind of slot takes part in each rule is the table of section 2.6. In short: a slot is one
frozen token; `Number` slots count as numbers for units, ranges, dates, dimensions and primes,
except after a sign, where they are labels; `Date` slots count as years for year words and pair with
each other in ranges (check 17); `Word` and `Other` slots count as words; no glue sticks to an
`Other` slot.

### 8.11 Coverage of the task's rule list

| Requirement of the task                                                                    | Rules                                                                               |
| ------------------------------------------------------------------------------------------ | ----------------------------------------------------------------------------------- |
| quotes by language, nested (ru «…„…“…», en “…‘…’…”), next to slots                         | `quotes.pairs` (section 6)                                                          |
| apostrophe (`don't`, `'90s`)                                                               | `quotes.apostrophe`                                                                 |
| unpaired quote: leave as is and report, do not guess                                       | `quotes.pairs`, `UnmatchedQuote` (section 6.6)                                      |
| inch and second marks after a number                                                       | `quotes.primes` (section 6.5)                                                       |
| em dash between words (`-` and `--`), no-break space before the dash in Russian            | `dash.spaced-hyphen`, `dash.double-hyphen`, `nbsp.dash`                             |
| en dash in numeric ranges and ranges of years                                              | `range.placeholders`, `range.digits`, `range.context`, `range.time`, `format_range` |
| hyphen in compound words and identifiers untouched                                         | rule conditions and classifier check 21 (section 8.4)                               |
| dash at the start of a line of dialogue                                                    | `dash.dialogue`                                                                     |
| ellipsis and combinations (`?..`, `!..`)                                                   | `ellipsis.dots`, `ellipsis.inner`                                                   |
| no-break space after short prepositions, conjunctions, particles by the language's list    | `nbsp.short-words`, `nbsp.short-length`                                             |
| before particles (`бы`, `же`, `ли`)                                                        | `nbsp.particles`                                                                    |
| in abbreviations (`т. е.`, `и т. д.`, `и т. п.`)                                           | `nbsp.abbreviations`                                                                |
| after `№` and `§`                                                                          | `nbsp.signs`                                                                        |
| between a number and a unit or abbreviation (`5 МБ`, `30 с`, `12 дн.`, `100 %` by setting) | `nbsp.units` with `PercentSpacing`                                                  |
| between initials and surname                                                               | `nbsp.initials`                                                                     |
| in dates (`21 ноября`, `2026 г.`)                                                          | `nbsp.dates`                                                                        |
| numbers by language (group separator, threshold, decimal separator, minus U+2212)          | `format_number`, `number.minus`                                                     |
| ranges of numbers                                                                          | `format_range`, range rules                                                         |
| multiplication sign between numbers (`1920x1080`) by setting                               | `number.times` (a rule switch; off in Interface)                                    |
| `(c)`, `(r)`, `(tm)`, `->`, `>=`, `!=` as rules, off in Interface                          | `symbols.copyright`, `symbols.arrows`, `symbols.math`                               |
| nowrap for telephone numbers                                                               | `nbsp.phone` and nowrap output (section 11.5)                                       |

### 8.12 Deliberate differences from the reference implementation

Recorded from its behavior on the scouting corpus of 820 cases with default settings. In stage 5
they become the table of expected differences, each with a reason code (section 13.4).

| Code                | Input                                             | Reference implementation           | typolish (Prose)                        | Reason                                                                   |
| ------------------- | ------------------------------------------------- | ---------------------------------- | --------------------------------------- | ------------------------------------------------------------------------ |
| `NoGuess`           | `Он сказал "привет.`, `"Один "два три"`           | `«привет.`, `«Один „два три“`      | unchanged, `UnmatchedQuote`             | the task: do not guess                                                   |
| `LookAlike`         | `Мoсква`, `Cервер`, `xлеб`                        | letters replaced                   | unchanged, `MixedScript`                | the task: never replace look-alike letters                               |
| `NoSpaceEdit`       | `host:8080`, `::1`, `a,b,c`, `x,y;z`              | `host: 8080`, `: 1`, `a, b,c`      | unchanged                               | technical fragments; no space rules                                      |
| `NoDateRewrite`     | `2026-11-21` (ru)                                 | `21.11.2026`                       | unchanged                               | negative case                                                            |
| `AmbiguousHyphen`   | `10 - 20 штук`, `5 - 3 = 2`                       | `10⍽— 20`                          | unchanged, `AmbiguousHyphen`            | range or minus is unknown                                                |
| `PercentAsWritten`  | `Скидка 100 %.`                                   | `100%`                             | `100⍽%`                                 | the task: by setting; never removes a space by default                   |
| `Technical`         | `a>>b`, `0x1F`, `1/2/2026`, `a <-> b`             | `a≫b`, `0×1F`, `½/2026`, `a <→ b`  | unchanged; `a ↔ b`                      | frozen fragments, whole operators, no fractions                          |
| `AuthorPunctuation` | `А если..`, `Конец....`, `Да!!`, `Да!?`           | `А⍽если.`, `Конец…`, `Да!`, `Да?!` | unchanged (glue aside)                  | the author's punctuation                                                 |
| `DashStyleEn`       | `Moscow - the capital` (en)                       | `Moscow⍽— the`                     | `Moscow—the`                            | CMOS closed dash; settled, 15.5 item 6                                   |
| `RangeLiteral`      | `1941-1945` (en), `Страницы 10-20.`               | unchanged                          | `1941–1945`, `10–20`                    | the task: en dash in numeric ranges                                      |
| `NoLastShortWord`   | `Run X now.`, `He went to the shop`               | `X⍽now.`, `He⍽went to⍽the⍽shop`    | `Run X now.`, `He went to⍽the⍽shop`     | no glue before a short last word (section 1.5); no length rule in en     |
| `Slots`             | `{n} дн.`, `{a}-{b}`, `"{name}"`                  | unchanged                          | `{n}⍽дн.`, `{a}–{b}`, `«{name}»`        | slots are typed elements (pieces input)                                  |
| `NoSpelling`        | `кто то`, `из за`, `25-ый этаж`                   | `кто-то`, `из-за`, `25-й`          | unchanged (glue aside: `из⍽за`)         | spelling is not typography                                               |
| `Apostrophe`        | `'90s`, `dogs'` (en), `rock'n'roll`, `don't` (ru) | unchanged                          | `’90s`, `dogs’`, `rock’n’roll`, `don’t` | Unicode apostrophe in every script                                       |
| `EnNbsp`            | `Mr. Smith`, `J. R. R. Tolkien`, `5 MB`           | unchanged                          | `Mr.⍽Smith`, `J.⍽R.⍽R.⍽Tolkien`, `5⍽MB` | the task's no-break rules apply to en too                                |
| `LegalMarkContext`  | `Вариант (c)`, `(R) знак`                         | `Вариант ©`, `® знак`              | unchanged                               | `(c)` without a year is a list item; `(r)` only after a brand name       |
| `Primes`            | `5'10"`, `Угол 30° 15' 20"`                       | `5'10″`, `15' 20″`                 | `5′10″`, `15′ 20″`                      | primes for both quote characters                                         |
| `WideGlue`          | `Документ № 5`                                    | `№⎵5` (U+202F)                     | `№⍽5` (U+00A0)                          | one glue character by default; settled, 15.5 item 10                     |
| `NoDigitChange`     | `5 м2`, `20C`, `$100`                             | `5⍽м²`, `20` U+2009 `°C`, (off)    | `5⍽м2`, `20C`, `$100`                   | letters and digits never change; `м2` is a listed unit, so it is glued   |
| `AlreadyTypeset`    | `Что?…` (ru)                                      | `Что?..`                           | unchanged                               | typographic characters are never rewritten                               |
| `NoTrim`            | trailing spaces, tabs, repeated blank lines       | removed or replaced                | unchanged                               | whitespace is the caller's                                               |
| `KeepSpaces`        | `Размер 10 x 5.`                                  | `10×5`                             | `10 × 5`                                | the author's spaces are kept                                             |
| `EllipsisDigits`    | `-5...5`                                          | `-5…5`                             | unchanged                               | dots between digits are not an ellipsis; the core is not a signed number |
