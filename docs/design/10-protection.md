# typolish design: Protection and the mandatory negative cases

Part of the [typolish design](./README.md). Section numbers are those of the whole design.

## 10. Protection and the mandatory negative cases

### 10.1 Explicit protection

- **String input, backticks** (on by default; `protect_backticks(false)` turns them off). A run of N
  backticks opens a protected span, and the next run of exactly N backticks closes it, as code spans
  in CommonMark: single, double (which may contain a single backtick) and longer runs. A run of
  three or more backticks at the start of a line, after optional indentation of spaces and tabs,
  opens a **fenced block**, closed by a run of exactly the same length at the start of a later line,
  also after optional indentation (CommonMark also closes with a longer run and limits indentation
  to three spaces; typolish does neither). The whole text from the opening run to the closing run,
  the info string (` ```rust `) included, is protected; a fenced block may contain blank lines. A
  run at a line start with no closing run later in the input opens no fenced block: like any other
  run it is matched as an inline run within its window (section 3.4), and if that finds no closing
  run either, it is an unclosed delimiter (below).
- **Own pairs**: `protect_between("{{", "}}")`, `protect_between("[[", "]]")`. Opening and closing
  delimiters may be equal.
- **Scope.** Every span other than a fenced block, own pairs included, opens and closes within one
  window (a paragraph, section 3.3), as a CommonMark code span never crosses a blank line; it may
  cross single line breaks.
- **Matching** is one pass for fenced blocks over the whole input, then one pass per window for the
  other delimiters: positions are collected first, pairs are formed left to right, and spans do not
  nest (inside an open span only its closing delimiter is looked for). Time is O(n log n) at worst
  (one sort of the backtick runs), with no quadratic behavior, even for 10⁵ unclosed backticks
  (section 3.12).
- **An unclosed delimiter** is an ordinary character, not protection to the end of the text: a stray
  backtick in prose (``don`t``) does not switch off processing of the rest, and two stray backticks
  in different paragraphs do not pair (``Don`t.\n\nОн сказал "да".\n\nIt`s ok.`` gets `«да»` and two
  diagnostics). Diagnostic `UnclosedDelimiter`.
- **Delimiters stay in the output.** There is no mode that strips them: after stripping, a second
  run would typeset the content, and idempotence would break. A caller who does not want the
  delimiters parses the text itself and passes `Piece::Protected`.
- **Pieces**: `Piece::Protected` protects without delimiters. Delimiters inside `Piece::Text` work
  as in a string and are matched across the pieces of a window (section 3.4): in
  ``[Piece::Text("Run `git checkout -- "), Piece::Slot("b", Slot::WORD), Piece::Text("` to reset.")]``
  the command is protected with the slot inside, and the slot stays a slot. A template of `Text`
  pieces gives the same result as their concatenation.
- **Validation** in `build()`: a delimiter is non-empty and contains no whitespace and no letter or
  digit; it is neither equal to nor a prefix of a delimiter of another pair; within one pair the two
  delimiters may be equal, but neither may be a proper prefix of the other
  (`Error::InvalidSetting`).
- **For its neighbors a protected span is one word**: a preposition before a span in backticks gets
  glue (`` в⍽`--config` ``), and quotes around it follow the language (``"`--force`"`` →
  ``«`--force`»``). Its content is written as is and encoded for the output kind (`<` in code
  becomes `&lt;` in HTML): protection means "do not typeset", not "do not encode".
- No form of protection needs HTML.

### 10.2 Implicit protection

The classifier (section 3.6) freezes technical fragments in both rule sets. Protection does not
depend on the set: a technical fragment is intact in Prose too. The sets differ only in which rules
look at the context around it. Explicit protection is still needed for commands that contain spaces
(`` `rm -rf /tmp/x` ``), for code, and for anything that must stay literal in Prose.

### 10.3 Guarantee

1. Slots, protected spans and frozen fragments are never inside an applied edit: the engine checks
   it in one place (section 3.7).
2. A property test: for every input and every rule set, every protected span and every slot of the
   input appears in the output byte for byte and in place; every fragment classified as technical in
   the input, and every compound word, appears in the output byte for byte (section 13.5).
3. `Report::protected()` shows which spans were recognized and why (`ProtectedKind`, `TechKind`),
   which answers "why did no rule fire here" (section 12.5).

### 10.4 Mechanisms

- **M1 Frozen fragment.** The classifier freezes the core; the engine refuses every edit that
  touches it (sections 3.6, 3.7).
- **M2 Lone token.** Dash rules fire only on a fragment that is nothing but `-`, `--` or `---` (a
  typed `–` or `—` is matched with an empty edit) with one pattern space (section 8.2) and an
  operand on each side (closing punctuation on the left or an opening quote or bracket on the right
  counts too; `dash.dialogue` needs only a line start on the left), never next to a flag or between
  two technical fragments or foreign words (a compound word counts by the script of its letters,
  section 8.2), and the closed style never joins a technical fragment or a compound word to a word
  (section 8.4).
- **M3 Whole operator.** A run of operator characters is one token and changes only if the whole run
  equals a table entry; `=>`, `>>`, `-->` are in no table.
- **M4 Exact shape.** A shape rule needs the whole core to match: `D+-D+` with exactly two numbers,
  `D+xD+` without a leading zero.
- **M5 Lists and script.** No-break rules work only from the language's lists, compare only words of
  the language's script (unless the caller asks otherwise), and never glue before a fragment that
  starts with `-`.
- **M6 No such rule.** Version 1 has no rule that inserts spaces after `, : ; ! ?`, rewrites dates,
  replaces letters or reads HTML (section 1.5).
- **M7 Letters and digits.** No rule changes a letter or a digit inside a `Word` or `Number` token;
  the only letters replaced are the `LegalMark` and `Times` tokens of `symbols.copyright` and
  `number.times`, which the property test turns off (section 13.5).
- **M8 Not in the set.** The rule exists but is not in Interface (section 9.3).
- **M9 Opaque slots and protected spans** (sections 3.4, 6.9).
- **M10 No HTML in the core.** `<` and `>` are ordinary characters: the classifier reads them as
  operator characters (checks 7 and 8 of section 3.6), and only `symbols.arrows` and `symbols.math`
  edit them, and only at whole operator fragments.

### 10.5 Mandatory negative cases

Every case of the task's list, with the mechanisms that keep it unchanged in Interface, for both
languages. Every row is a test (section 13.3): alone, inside the sentence frames
`Запустите X сейчас.` and `Значение: X, затем далее.` (ru) and `Run X now.` and
`Value: X, then more.` (en), and as a `Piece::Text` between two slots, separated from them by spaces
(`{a} X {b}`); glued to a slot, a case is a different fragment (section 3.6) and is checked against
the row's expected output, the exceptions below included (Interface, and Prose outside rows 3 and
10). The frames contain no listed short word. In Interface every row is checked through `report`:
the tokens of the case stay byte for byte, and no edit other than glue outside them occurs; the
expected output also equals the input byte for byte and is `Cow::Borrowed`, with two exceptions that
come from the case itself, not from the frame (below). In Prose the same check through `report`
applies, except where the last column says that Prose changes the tokens of the case (rows 3 and
10): there the expected output is the one that column gives, and `report` names the rules it names.

The two exceptions in Interface:

- row 24, variant `if a<b and c>d`, in English: `if` and `and` are listed short words, so the
  expected output is `if⍽a<b and⍽c>d` (glue outside the frozen fragments);
- row 25 is the task's case for Russian text; in English its mirror is checked instead, short
  Cyrillic words (`и`, `в`, `к`) that get no glue, because the Latin `a` and `to` are English short
  words and are glued there by design.

The last column gives the behavior in Prose. Host names in tests are neutral (`example.com`,
`db.example.internal`), never names from internal networks.

| No. | Case from the task                                                                             | Classification                                                   | Mechanisms in Interface                                        | Prose                                                                                                                                                     |
| --: | ---------------------------------------------------------------------------------------------- | ---------------------------------------------------------------- | -------------------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------- |
|   1 | flag `--force` (also `--long-opt=value`)                                                       | `Flag` (check 3)                                                 | M1, M2, M8                                                     | unchanged                                                                                                                                                 |
|   2 | command `ls -la`                                                                               | word `ls`, `Flag` `-la`                                          | M1, M2, M5                                                     | unchanged                                                                                                                                                 |
|   3 | lone `--` (`--`, `git checkout -- file`)                                                       | dash fragment (special case, section 3.6)                        | M8 (`dash.double-hyphen`), M2                                  | `--` alone unchanged; `Запустите -- сейчас.` → `Запустите⍽— сейчас.`; `git checkout -- file` unchanged in ru (foreign words), a dash in en (section 10.8) |
|   4 | `a>>b`                                                                                         | `Operator` (check 8)                                             | M1, M3, M8                                                     | unchanged                                                                                                                                                 |
|   5 | `=>`                                                                                           | `OperatorRun` (check 7)                                          | M3, M8                                                         | unchanged                                                                                                                                                 |
|   6 | `host:8080`                                                                                    | `Colon` (check 6)                                                | M1, M6                                                         | unchanged                                                                                                                                                 |
|   7 | `::1` (also `fe80::1%eth0`, `[::1]:8080`)                                                      | `Colon` (check 6)                                                | M1, M6                                                         | unchanged                                                                                                                                                 |
|   8 | `10.0.0.0/8`                                                                                   | `Path` (check 4)                                                 | M1                                                             | unchanged                                                                                                                                                 |
|   9 | `a:b`                                                                                          | `Colon` (check 6)                                                | M1, M6                                                         | unchanged                                                                                                                                                 |
|  10 | port range `8000-8100`                                                                         | `Range` (check 17), not frozen                                   | M8 (`range.digits`, `range.context`), M4                       | `8000–8100` (section 10.6)                                                                                                                                |
|  11 | date `2026-11-21` (also `2026-11-21T10:20:30Z`)                                                | `IsoDate` (check 12); with time `Colon`                          | M1, M4, M6                                                     | unchanged                                                                                                                                                 |
|  12 | version `0.2.27`                                                                               | `Version` (check 13)                                             | M1                                                             | unchanged                                                                                                                                                 |
|  13 | version `v1.2.3-rc.1` (also `1.2.3-beta.2+build.5`)                                            | `Version` (check 13); `Operator` (check 8) for the form with `+` | M1                                                             | unchanged                                                                                                                                                 |
|  14 | hexadecimal strings (`0x1F`, `deadbeef0123`, `d41d8cd98f00b204e9800998ecf8427e`)               | `Hex` (checks 10, 11); `deadbeef` is a word                      | M1, M4, M7                                                     | unchanged                                                                                                                                                 |
|  15 | UUID (`550e8400-e29b-41d4-a716-446655440000`)                                                  | `Uuid` (check 9)                                                 | M1                                                             | unchanged                                                                                                                                                 |
|  16 | fingerprints (`SHA256:47DEQpj8HBSa+/TImW+5JCeuQeRkm5NMpJWZG3hSuFU`, `43:51:43:a1:b5:fc:8b:b7`) | `Path` or `Colon` (checks 4, 6)                                  | M1                                                             | unchanged                                                                                                                                                 |
|  17 | base64 (`SGVsbG8gd29ybGQ=`, `a+b/c==`)                                                         | `Unclassified` (check 24), `Path`                                | M1                                                             | unchanged                                                                                                                                                 |
|  18 | paths (`/etc/hosts`, `C:\Users\x`, `~/.config/app`, `../x`)                                    | `Path` (check 4)                                                 | M1                                                             | unchanged                                                                                                                                                 |
|  19 | URLs (`https://example.com/a?b=c`, `www.example.com`)                                          | `Url` (check 1)                                                  | M1                                                             | unchanged                                                                                                                                                 |
|  20 | e-mail addresses (`user@example.com`, `git@example.com:org/repo.git`)                          | `Email` (check 2)                                                | M1                                                             | unchanged                                                                                                                                                 |
|  21 | host names (`db.example.internal`, `db-01.example.com`, `localhost`)                           | `Dotted` (check 18); `localhost` a word                          | M1, M7                                                         | unchanged                                                                                                                                                 |
|  22 | lists without spaces (`a,b,c`, `x,y;z`, `1,2,3`, `red,green,blue`)                             | `List` (check 19)                                                | M1, M6                                                         | unchanged                                                                                                                                                 |
|  23 | Latin letters next to Cyrillic (`Мoсква`, `Cервер`, `xлеб`)                                    | word, script `Mixed`                                             | M6, M7; `MixedScript` warning                                  | unchanged                                                                                                                                                 |
|  24 | text like `<word>` (also `vec<int>`, `a <b> c`, `if a<b and c>d`)                              | `Angle`, `Operator` (check 8)                                    | M1, M10                                                        | unchanged                                                                                                                                                 |
|  25 | short Latin words in Russian text (`ls`, `a`, `go`, `db`, `to`); en: Cyrillic `и`, `в`, `к`    | words, script Latin (Cyrillic in en)                             | M5 (`short_words_any_script(false)`), M8 (`nbsp.short-length`) | unchanged (M5)                                                                                                                                            |
|  26 | identifiers with a hyphen: `tls-server`, `agent-ca`                                            | `Compound` (check 21)                                            | M1 (hyphens frozen), M2                                        | unchanged                                                                                                                                                 |
|  27 | identifier with an underscore: `kv_lang`                                                       | `Identifier` (check 8)                                           | M1                                                             | unchanged                                                                                                                                                 |

Every row also runs with the 405 negative cases of the scouting corpus (both languages, alone and in
frames), with internal host names replaced by neutral ones (section 13.3).

### 10.6 The rule for port ranges

An en dash between two numbers is written in exactly these cases:

1. The caller passed two values explicitly: `format_range(a, b, output)`.
2. In pieces, `Slot(Number)`, a `-` and `Slot(Number)` (or two `Date` slots) stand without spaces:
   the template author put a hyphen between two declared values, which is a range of values
   (`range.placeholders`, in both sets).
3. In Prose only, a literal `D+-D+` with exactly two numbers, no leading zeros, ascending
   (`range.digits`), or a literal range with a context word (`range.context`), or a time range
   (`range.time`).

Interface has neither 3 nor any other literal path, so the range in `8000-8100`, `ports 8000-8100`
and `Порты 8000-8100 с балансировкой` never changes there (the last gets only the glue of
`nbsp.short-words`: `с⍽балансировкой`). In Prose `8000-8100` becomes `8000–8100` deliberately: the
task asks for en dashes in numeric ranges, and technical text belongs in Interface or in backticks
(section 15.5, item 7).

A literal range glued to a slot is not a literal range: in `{n}8000-8100` or `8000-8100{n}` the
slot's value joins the number, so the core is not `N-N` (check 17) but a `Compound` (check 21), and
its hyphen stays in both sets. The range the value would give is unknown (`{n} = 5` gives
`58000-8100`), the same reason a `Number` slot next to a literal number blocks `range.digits`
(section 3.6).

### 10.7 Additional stress strings

Beyond the task's list, the negative tests include, in Interface unchanged and in Prose with the
expectation given: `rm -rf --no-preserve-root /`, `git commit -m msg --amend`,
`cargo build --release`, `a=>b`, `x -> y; z => w` (Prose: `x → y; z => w`), `192.168.1.1:443`,
`12:30:45`, `21.11.2026`, `nats-server-2.10`, `my_var-name`, `CamelCase-Mixed_id`, `pre-commit`,
`map<k,v>`, `C#`, `AT&T`, `50%-ный`, `key="value"`, `f(c)`, `f(r)`, `S(r) = πr²`, `x86`, `2x`,
`0x10`, `1/2/2026`, `a <-> b` (Prose: `a ↔ b`), `-->`, `<-->`, `>>=`, `x == y`,
`diff -u - file.txt`, `tar -cf - -C dir`, `npm test -- --watch`, `Use - --force`, `@user`, `@here`.

Interaction strings. These forms come from the patterns of section 8 and are checked by exact output
in both sets, ru and en unless a language is given (glue from other rules is part of the
expectation):

| Input                                 | Interface                    | Prose                        |
| ------------------------------------- | ---------------------------- | ---------------------------- |
| `+7 495 123-4567`                     | unchanged                    | `+7⍽495⍽123–4567`            |
| `123-4567`                            | unchanged                    | `123–4567`                   |
| `+1 300 200 100 50`                   | unchanged                    | `+1⍽300⍽200⍽100⍽50`          |
| `2010-11 гг.` (ru)                    | `2010-11⍽гг.`                | `2010–11⍽гг.`                |
| `2026-11`                             | unchanged                    | unchanged                    |
| `2026-11 гг.` (ru)                    | `2026-11⍽гг.`                | `2026-11⍽гг.`                |
| `21.11.2026 г. Затем` (ru)            | `21.11.2026⍽г. Затем`        | `21.11.2026⍽г. Затем`        |
| `21.11.26 г. Затем` (ru)              | `21.11.26⍽г. Затем`          | `21.11.26⍽г. Затем`          |
| `5 - x`, `x - 5`                      | unchanged, `AmbiguousHyphen` | unchanged, `AmbiguousHyphen` |
| `-- file` at a line start             | unchanged                    | ru `—⍽file`; en unchanged    |
| `Подождите...\nДальше` (ru)           | `Подождите…\nДальше`         | `Подождите…\nДальше`         |
| `Подождите...\tДальше` (ru)           | `Подождите…\tДальше`         | `Подождите…\tДальше`         |
| `... слово` (ru)                      | `… слово`                    | `… слово`                    |
| `word  -  word`, `word\t-\tword` (en) | unchanged                    | unchanged                    |
| `word⍽- word` (en)                    | `word⍽— word`                | `word⍽— word`                |
| `т.  е.`, `№\t5` (ru)                 | unchanged                    | unchanged                    |

### 10.8 Known limits

- A lone `-` between two words of the language is a dash in both sets. In English text `cat - file`
  becomes `cat—file`, and `tls-server - agent-ca` becomes `tls-server — agent-ca` (compound words
  are words of their script, section 8.2); in Russian text both stay (foreign words on both sides).
  Next to a flag a `-` is never a dash, even where one was meant (`флаг -v - он включает вывод`).
  Commands with spaces must be protected with backticks or passed as `Piece::Protected`; the crate
  README says so (section 13.10).
- With the closed style (en) the dash keeps the author's spaces next to anything but a plain word
  (`Hello, — he said`, `Chapter — 5`, `See https://example.com — the guide`,
  `mother-in-law — father-in-law`); only two plain words are joined (`word—word`).
- A capital letter with a full stop at a sentence end before a capitalized word gets a no-break
  space: a one-letter label (`вариант А.⍽После`, `plan B.⍽Then`), the English pronoun
  (`Neither did I.⍽Then`), a one-letter unit after a number (`220⍽В.⍽Затем`). The pattern cannot
  tell them from `сказал А. Петров`, and the glue changes no visible character; it only forbids a
  line break there.
- `rock 'n' roll` with spaces is a top-level single pair and stays straight under `NestedOnly`;
  `rock'n'roll` gets apostrophes.
- In Prose, `dash.double-hyphen` turns `git checkout -- file` into a dash in English text, and
  `number.minus` turns `tail -5` into `tail −5`; `range.digits` turns a telephone fragment
  `123-4567` into a range. Prose assumes commands are protected; Interface has none of these rules.
- `ellipsis.inner` turns `...args` into `…args` in Prose.
- A mention such as `@user` is frozen whole as an `Email` (check 2): rules never edit inside it,
  while spaces and dashes next to it are typeset as next to any technical fragment.
- Swiss grouping `1'000'000` and digit separators `0b1'0000` are frozen (section 3.6) and keep `'`.
- A number grouped with commas is read as a number only with one group and no decimal part
  (`1,234 kg` → `1,234⍽kg`, check 15 reads the group as a decimal part): `1,234,567 kg` is a `List`
  (check 19) and `1,000.5 kg` a `Dotted` core (check 18), both frozen, so `number.minus` and
  `nbsp.units` do not apply to them, also where `format_number` wrote them. A value is passed as a
  `Slot::NUMBER`. Recognizing grouped numbers is a change of check 15 (section 15.3).
- Frozen fragments are left alone even where a rule would be welcome: `50%-ный`, `C#`, `AT&T`
  (sections 3.6, 4.3). This errs on the safe side by design.
- A quote glued to a punctuation-joined word (`сказал:"привет"`, `сказал,"привет"`) belongs to a
  frozen fragment (`Colon`, `List`) and stays straight, without a diagnostic. With the space
  (`сказал: "привет"`) the quotes are placed. An empty value glued the same way (`сказал:""`,
  `key:""`) is not frozen, because the core is the word alone: its two quotes are closing quotes
  with no opener, so the paragraph keeps its straight quotes with `UnmatchedQuote` (warning).
  `key: ""` gives `key: «»`. A frozen core that does not end in `=` keeps its wrapper quotes as
  candidates, so `Путь /etc/"" и "да".` fails the paragraph the same way, with `UnmatchedQuote`
  (warning, section 6.1), while `key=""` stays as written.
- Straight quotes nested out of order around a trailing possessive are not idempotent in English:
  `""s' 'x."'"` gives `“‘s’ 'x.’'”`, and a second pass pairs the remaining `'` (`“‘s’ ‘x.’’”`). The
  same class breaks P2 (section 12.2) in the built-in English sets when an unpaired `'s` or `'S`
  follows a nested trailing possessive: `"a "dogs' 's" b"` gives `“a⍽‘dogs’ 's’ b”`, whose
  `to_plain` `"a "dogs" 's' b"` is typeset as `“a⍽‘dogs’ ‘s’ b”` (likewise `"a "DOGS' 'S" b"`). This
  is a defect of the quote phase (section 6.11), not a choice.
- In a declared language whose primary pair is `‘’` (en-GB), Prose, a sign-only pair inside a
  top-level straight single pair before a number is not idempotent: `'a ""-3'` gives `'a⍽‘’-3'`, and
  a second pass reads the last `'` as a prime (`'a⍽‘’-3′`). This is a defect of the quote phase
  (sections 6.5 and 6.11), not a choice.
- A quoted value with a space glued to `=` is cut by the space: `title="My` is frozen (`Operator`),
  the quote after `App` closes nothing, and `Set title="My App" for the "main" window.` keeps every
  quote of the paragraph straight with `UnmatchedQuote` (warning, sections 6.1 and 6.6). Protect
  such a notation with backticks.
- `Piece::Markup` is inline markup, transparent for rules (section 3.4): a block tag passed as
  `Markup` ends no paragraph, so `"да`, `</p><p>`, `Нет"` pair across the tag (section 6.10), and a
  dialogue dash right after it is not placed. A splitter passes each block in a call of its own, or
  ends the `Text` piece before a block tag with `\n\n` (section 2.6).
- A quote glued to a dash (`он —"Да"`, `"no"—and`) belongs to a frozen `Unclassified` core and stays
  straight, without a diagnostic; with a space (`он — "Да"`) the quotes are placed.
- A number with a unit attached without a space (`-15°C`) is not a number core: it is frozen whole
  and keeps its hyphen, while `-15 °C` gets the minus and the glue in `PROSE` (`−15⍽°C`).
- A time range with spaces (`10:00 - 12:00`) is not a dash: both sides are technical fragments; the
  hyphen stays and `AmbiguousHyphen` is reported. `10:00-12:00` gets the en dash in `PROSE`.
- A chain of short words is glued whole, with no limit on its length (`Он⍽и⍽я⍽и⍽ты⍽и⍽мы`, Russian
  `PROSE`): a text made of words of one or two letters has no place to break a line.
- After a number a unit symbol that is also a word, a name or an initial is read as a unit and
  glued: a capital `В` or `А` (volt, ampere), and the word after it in turn (`Часть 3⍽В⍽лесу`); `Дж`
  (joule) before the full stop of an initial (`В⍽2001⍽Дж. Буш`); `Ом` and `Па` (ohm, pascal) as a
  name or a word (`В⍽1827⍽Ом⍽открыл` in `PROSE`, `Глава 5⍽Па-де-де`); English `GW` (gigawatt) as
  initials (`In⍽2001⍽GW Bush`). The glue changes no visible character, and `remove_units` takes a
  unit out of the list (`remove_units(["Дж"])` gives `В⍽2001 Дж. Буш`).
- A percentage range with a hyphen and the sign attached (`10-20%`) is a frozen fragment and keeps
  its hyphen; with a space (`10-20 %`) it gets the en dash in `PROSE` (`10–20⍽%`).
- A range of two quantities with units and no spaces around the dash (`5 kg–10 kg`, `9 a.m.–5 p.m.`)
  is one fragment from the first unit to the second number, so only the first unit is glued:
  `5⍽kg–10 kg`, `9⍽a.m.–5 p.m.`. With spaces around the dash both are glued (`9⍽a.m. – 5⍽p.m.`).
