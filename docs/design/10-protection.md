# typolish design: Protection and the mandatory negative cases

Part of the [typolish design](./README.md). Section numbers are those of the whole design.

## 10. Protection and the mandatory negative cases

### 10.1 Explicit protection

- **String input, backticks** (on by default; `protect_backticks(false)` turns them off). A run of N
  backticks opens a protected span, and the next run of exactly N backticks closes it, as code spans
  in CommonMark: single, double (which may contain a single backtick) and triple runs, including
  blocks over several lines.
- **Own pairs**: `protect_between("{{", "}}")`, `protect_between("[[", "]]")`. Opening and closing
  delimiters may be equal.
- **Matching** is one pass: positions of all delimiters are collected first, pairs are formed left
  to right, and spans do not nest (inside an open span only its closing delimiter is looked for).
  Time is O(n) even for 10⁵ unclosed backticks (section 3.12).
- **An unclosed delimiter** is an ordinary character, not protection to the end of the text: a stray
  backtick in prose
  (`don`t`) does not switch off processing of the rest. Diagnostic `UnclosedDelimiter`.
- **Delimiters stay in the output.** There is no mode that strips them: after stripping, a second
  run would typeset the content, and idempotence would break. A caller who does not want the
  delimiters parses the text itself and passes `Piece::Protected`.
- **Pieces**: `Piece::Protected` protects without delimiters. Delimiters inside `Piece::Text` work
  as in a string.
- **Validation** in `build()`: a delimiter is non-empty, is not whitespace, contains no letter or
  digit, is not equal to another delimiter and is not a prefix of another one
  (`Error::InvalidSetting`).
- **For its neighbours a protected span is one word**: a preposition before a span in backticks gets
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
   the input appears in the output byte for byte (section 13.5).
3. `Report::protected()` shows which spans were recognized and why (`ProtectedKind`, `TechKind`),
   which answers "why did no rule fire here" (section 12.5).

### 10.4 Mechanisms

- **M1 Frozen fragment.** The classifier freezes the core; the engine refuses every edit that
  touches it (sections 3.6, 3.7).
- **M2 Lone token.** Dash rules fire only on a fragment that is nothing but `-` or `--` with one
  space and an operand on each side.
- **M3 Whole operator.** A run of operator characters is one token and changes only if the whole run
  equals a table entry; `=>`, `>>`, `-->` are in no table.
- **M4 Exact shape.** A shape rule needs the whole core to match: `D+-D+` with exactly two numbers,
  `D+xD+` without a leading zero.
- **M5 Lists and script.** No-break rules work only from the language's lists, compare only words of
  the language's script (unless the caller asks otherwise), and never glue before a fragment that
  starts with `-`.
- **M6 No such rule.** Version 1 has no rule that inserts spaces after punctuation, rewrites dates,
  replaces letters or reads HTML (section 1.5).
- **M7 Letters and digits.** No rule changes a letter or a digit (property test, section 13.5).
- **M8 Not in the set.** The rule exists but is not in Interface (section 9.3).
- **M9 Opaque slots and protected spans** (sections 3.4, 6.9).
- **M10 No HTML in the core.** `<` and `>` are ordinary characters; only `symbols.math` looks at
  them, and only at whole operator fragments.

### 10.5 Mandatory negative cases

Every case of the task's list, with the mechanisms that keep it unchanged in Interface, for both
languages. Every row is a test (section 13.3): alone, inside the sentence frames
`Запустите X сейчас.` and `Значение: X, затем далее.` (ru) and `Run X now.` and
`Value: X, then more.` (en), and as a `Piece::Text` next to slots. In Interface the expected output
equals the input byte for byte and is `Cow::Borrowed` (the frames contain no listed short word). The
last column gives the behavior in Prose. Host names in tests are neutral (`example.com`,
`db.example.internal`), never names from internal networks.

| No. | Case from the task                                                                             | Classification                              | Mechanisms in Interface                                        | Prose                                                                                                                                                     |
| --: | ---------------------------------------------------------------------------------------------- | ------------------------------------------- | -------------------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------- |
|   1 | flag `--force` (also `--long-opt=value`)                                                       | `Flag` (check 3)                            | M1, M2, M8                                                     | unchanged                                                                                                                                                 |
|   2 | command `ls -la`                                                                               | word `ls`, `Flag` `-la`                     | M1, M2, M5                                                     | unchanged                                                                                                                                                 |
|   3 | lone `--` (`--`, `git checkout -- file`)                                                       | dash fragment (check 7)                     | M8 (`dash.double-hyphen`), M2                                  | `--` alone unchanged; `Запустите -- сейчас.` → `Запустите⍽— сейчас.`; `git checkout -- file` unchanged in ru (foreign words), a dash in en (section 10.8) |
|   4 | `a>>b`                                                                                         | `Operator` (check 8)                        | M1, M3, M8                                                     | unchanged                                                                                                                                                 |
|   5 | `=>`                                                                                           | `OperatorRun` (check 7)                     | M3, M8                                                         | unchanged                                                                                                                                                 |
|   6 | `host:8080`                                                                                    | `Colon` (check 6)                           | M1, M6                                                         | unchanged                                                                                                                                                 |
|   7 | `::1` (also `fe80::1%eth0`, `[::1]:8080`)                                                      | `Colon` (check 6)                           | M1, M6                                                         | unchanged                                                                                                                                                 |
|   8 | `10.0.0.0/8`                                                                                   | `Path` (check 4)                            | M1                                                             | unchanged                                                                                                                                                 |
|   9 | `a:b`                                                                                          | `Colon` (check 6)                           | M1, M6                                                         | unchanged                                                                                                                                                 |
|  10 | port range `8000-8100`                                                                         | `Range` (check 17), not frozen              | M8 (`range.digits`, `range.context`), M4                       | `8000–8100` (section 10.6)                                                                                                                                |
|  11 | date `2026-11-21` (also `2026-11-21T10:20:30Z`)                                                | `IsoDate` (check 12); with time `Colon`     | M1, M4, M6                                                     | unchanged                                                                                                                                                 |
|  12 | version `0.2.27`                                                                               | `Version` (check 13)                        | M1                                                             | unchanged                                                                                                                                                 |
|  13 | version `v1.2.3-rc.1` (also `1.2.3-beta.2+build.5`)                                            | `Version` (check 13), `Operator`            | M1                                                             | unchanged                                                                                                                                                 |
|  14 | hexadecimal strings (`0x1F`, `deadbeef0123`, `d41d8cd98f00b204e9800998ecf8427e`)               | `Hex` (checks 10, 11); `deadbeef` is a word | M1, M4, M7                                                     | unchanged                                                                                                                                                 |
|  15 | UUID (`550e8400-e29b-41d4-a716-446655440000`)                                                  | `Uuid` (check 9)                            | M1                                                             | unchanged                                                                                                                                                 |
|  16 | fingerprints (`SHA256:47DEQpj8HBSa+/TImW+5JCeuQeRkm5NMpJWZG3hSuFU`, `43:51:43:a1:b5:fc:8b:b7`) | `Path` or `Colon` (checks 4, 6)             | M1                                                             | unchanged                                                                                                                                                 |
|  17 | base64 (`SGVsbG8gd29ybGQ=`, `a+b/c==`)                                                         | `Unclassified` (check 24), `Path`           | M1                                                             | unchanged                                                                                                                                                 |
|  18 | paths (`/etc/hosts`, `C:\Users\x`, `~/.config/app`, `../x`)                                    | `Path` (check 4)                            | M1                                                             | unchanged                                                                                                                                                 |
|  19 | URLs (`https://example.com/a?b=c`, `www.example.com`)                                          | `Url` (check 1)                             | M1                                                             | unchanged                                                                                                                                                 |
|  20 | e-mail addresses (`user@example.com`, `git@example.com:org/repo.git`)                          | `Email` (check 2)                           | M1                                                             | unchanged                                                                                                                                                 |
|  21 | host names (`db.example.internal`, `db-01.example.com`, `localhost`)                           | `Dotted` (check 18); `localhost` a word     | M1, M7                                                         | unchanged                                                                                                                                                 |
|  22 | lists without spaces (`a,b,c`, `x,y;z`, `1,2,3`, `red,green,blue`)                             | `List` (check 19)                           | M1, M6                                                         | unchanged                                                                                                                                                 |
|  23 | Latin letters next to Cyrillic (`Мoсква`, `Cервер`, `xлеб`)                                    | word, script `Mixed`                        | M6, M7; `MixedScript` warning                                  | unchanged                                                                                                                                                 |
|  24 | text like `<word>` (also `vec<int>`, `a <b> c`, `if a<b and c>d`)                              | `Angle`, `Operator` (check 8)               | M1, M10                                                        | unchanged                                                                                                                                                 |
|  25 | short Latin words in Russian text (`ls`, `a`, `go`, `db`, `to`)                                | words, script Latin                         | M5 (`short_words_any_script(false)`), M8 (`nbsp.short-length`) | unchanged (M5)                                                                                                                                            |
|  26 | identifiers with a hyphen: `tls-server`, `agent-ca`                                            | `Compound` (check 21)                       | M1 (hyphens frozen), M2                                        | unchanged                                                                                                                                                 |
|  27 | identifier with an underscore: `kv_lang`                                                       | `Identifier` (check 8)                      | M1                                                             | unchanged                                                                                                                                                 |

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

Interface has neither 3 nor any other literal path, so `8000-8100`, `ports 8000-8100` and
`Порты 8000-8100 с балансировкой` never change there. In Prose `8000-8100` becomes `8000–8100`
deliberately: the task asks for en dashes in numeric ranges, and technical text belongs in Interface
or in backticks (open point 7).

### 10.7 Additional stress strings

Beyond the task's list, the negative tests include, in Interface unchanged and in Prose with the
expectation given: `rm -rf --no-preserve-root /`, `git commit -m msg --amend`,
`cargo build --release`, `a=>b`, `x -> y; z => w` (Prose: `x → y; z => w`), `192.168.1.1:443`,
`12:30:45`, `21.11.2026`, `nats-server-2.10`, `my_var-name`, `CamelCase-Mixed_id`, `pre-commit`,
`map<k,v>`, `C#`, `AT&T`, `50%-ный`, `key="value"`, `f(c)`, `x86`, `2x`, `0x10`, `1/2/2026`,
`a <-> b` (Prose: `a ↔ b`), `-->`, `<-->`, `>>=`, `x == y`.

### 10.8 Known limits

- A lone `-` between two plain words of the language is a dash in both sets. In English text
  `cat - file` becomes `cat—file`; in Russian text it stays (foreign words on both sides). Commands
  with spaces must be protected with backticks or passed as `Piece::Protected`; the README says so.
- In Prose, `dash.double-hyphen` turns `git checkout -- file` into a dash in English text, and
  `number.minus` turns `tail -5` into `tail −5`; `range.digits` turns a telephone fragment
  `123-4567` into a range. Prose assumes commands are protected; Interface has none of these rules.
- `ellipsis.inner` turns `...args` into `…args` in Prose.
- Frozen fragments are left alone even where a rule would be welcome: `50%-ный`, `C#`, `AT&T`
  (sections 3.6, 4.3). This errs on the safe side by design.
