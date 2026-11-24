# typolish design

Status: the design of version 0.1, approved by the maintainer on 2026-11-22; stages 2 to 5 are
implemented in `dev/typolish`, and version 0.1.0 is prepared and not published (section 15.4). Where
this document and the code disagree, one of them is fixed in the same change.

Conventions used below:

- "Plain text" is text typed the easy way: straight quotes `"` and `'`, hyphen-minus `-`, three full
  stops, ordinary spaces. "Typeset text" is what the crate makes of it for a language.
- In examples `⍽` stands for U+00A0 NO-BREAK SPACE and `⎵` for U+202F NARROW NO-BREAK SPACE. Rust
  code in examples writes them as `\u{a0}` and `\u{202f}`. Examples are in the language they show
  (Russian or English); the rest of the design is in English.
- "Prose" and "Interface" are the two built-in rule sets (section 9). "Caller" is the code that uses
  the crate; "generator" is a caller that turns Fluent messages into code at build time.
- "Fragment" is a run of tokens between spaces; its "core" is what remains after the leading and
  trailing punctuation ("wrapper") is cut off (section 3.6). "Window" is a paragraph (section 3.3).
- "The task" is the assignment the maintainer gave for this crate; it is not in the repository, and
  the design restates every requirement of it that it relies on. "The lead" is the lead programmer
  of the project, who wrote this design and carries out stages 2 to 5 (section 15.4).
- "The reference implementation" is the JavaScript library the task names as the model and the
  benchmark. Its name does not appear anywhere in this repository; its behavior was recorded outside
  the repository and enters the tests only as plain tables of input and output (section 13.4).
- The "scouting corpus" is the set of 820 plain-text cases (432 Russian, 388 English; 405 of them
  negative) that was run through the reference implementation before this design was written. The
  cases and the recorded outputs stay outside the repository and enter it as the generated tables
  `tests/reference/{ru,en_us}.rs` (section 13.4).
- Section numbers run through all files and do not change when a file is renamed. Code refers to a
  place as "design section 6.5".
- Facts about dependencies, CLDR and HTML entities were checked on 2026-11-21, unless section 14.8
  gives a check another date; the checks and their results are listed in section 14.8. Statements
  that rest on secondary sources say so where they are made.

## Where each section is

| Section                                                | File                                                         |
| ------------------------------------------------------ | ------------------------------------------------------------ |
| 1. Scope and non-goals                                 | this file                                                    |
| 2. Public API                                          | [`02-public-api.md`](./02-public-api.md)                     |
| 3. Processing model                                    | [`03-processing.md`](./03-processing.md)                     |
| 4. Rule order, idempotence and determinism             | [`04-order-idempotence.md`](./04-order-idempotence.md)       |
| 5. Unicode and mixed languages                         | [`05-unicode.md`](./05-unicode.md)                           |
| 6. Quotes                                              | [`06-quotes.md`](./06-quotes.md)                             |
| 7. Language data                                       | [`07-language-data.md`](./07-language-data.md)               |
| 8. Rules of version 1                                  | [`08-rules.md`](./08-rules.md)                               |
| 9. Rule sets Prose and Interface                       | [`09-rule-sets.md`](./09-rule-sets.md)                       |
| 10. Protection and the mandatory negative cases        | [`10-protection.md`](./10-protection.md)                     |
| 11. Output kinds                                       | [`11-output.md`](./11-output.md)                             |
| 12. Reverse transformation, source check, explain mode | [`12-reverse-lint-explain.md`](./12-reverse-lint-explain.md) |
| 13. Testing                                            | [`13-testing.md`](./13-testing.md)                           |
| 14. Dependencies, MSRV, layout, gates (14.1 to 14.7)   | [`14-dependencies.md`](./14-dependencies.md)                 |
| 14.8 Checks behind this design                         | [`history.md`](./history.md)                                 |
| 15. Versioning, decisions, stage plan (15.1 to 15.4)   | [`15-decisions.md`](./15-decisions.md)                       |
| 15.5 Points put to the maintainer, all settled         | [`15-open-points.md`](./15-open-points.md)                   |

## 1. Scope and non-goals

### 1.1 Name

The crate is `typolish` (typography + polish), the name the maintainer gave. It is one library crate
with no binary. Its modules are private and every public type is re-exported from the crate root
(`typolish::Typographer`): the surface is small, and a flat namespace is easier to use than nested
modules. Whether the name is free on crates.io is checked in stage 5 with one query to the crates.io
API, which the maintainer allowed (section 15.5, item 3); no query is made before then, because it
discloses the project name to an outside service. The query was made on 2026-11-23: the name was
free (section 14.8).

### 1.2 What the crate does

It takes plain text and returns typeset text for a language: quotes by nesting level, apostrophes,
dashes, ellipses, no-break spaces, and rules for words, signs and numbers. It works on plain text
only; HTML parsing is not in the core (section 1.5). It also explains which rule changed what,
checks a source for problems, and maps typeset text back to plain text.

### 1.3 How it is used

The task names four uses. Each shapes part of the API:

1. **Build time, message templates.** A generator parses Fluent files and passes each message as a
   sequence of pieces: text and opaque placeholders ("slots") whose kind is known (number, word,
   date, other) and whose value is not. Rules treat a slot as an opaque element of its kind:
   `{n} дн.` gets a no-break space after the number, `{a}-{b}` between two numbers gets an en dash,
   quotes around `"{name}"` follow the language. Slots are returned untouched and in place (section
   2.6, build-time use in 2.14).
2. **Run time, a finished string** (a site, a report, a letter): a string and a language in, a
   typeset string out, on every request, without allocation when nothing changes for an input up to
   about 50 KiB (sections 2.5, 2.12, 2.15).
3. **Run time, numbers**: group separator, decimal separator and minus sign by the language, in a
   separate function, so that a template and the value put into it are typeset consistently (section
   2.7).
4. **Different outputs**: Unicode characters, HTML entities (`&nbsp;`, `&mdash;`), escaped string
   bodies for JavaScript, JSON, Rust and CSS (`\x00`, `\u00A0`, `\u{a0}`), and no-break groups
   rendered as `white-space: nowrap` where characters cannot do the job, for example in telephone
   numbers. The output kind is an argument of every call (section 11).

### 1.4 Principles

These invariants carry the guarantees of later sections:

1. **Rules consume plain forms and produce typographic forms.** A rule replaces straight quotes,
   hyphens, runs of full stops, single ordinary spaces and a few short sequences (`(c)`, `->`, `x`
   between numbers). Characters that are already typographic (`«`, `—`, `…`, U+00A0) are never
   rewritten; they only serve as context (section 4.5).
2. **Letters and digits never change.** No rule adds, removes or replaces a letter or a digit
   (section 10.4, mechanism M7). The only replacements of letters are `(c)`, `(r)`, `(tm)` (with the
   Cyrillic `(с)`) and `x` or `х` between numbers, each a separate token kind (section 3.5).
3. **Ordinary spaces are kept.** Rules turn a single U+0020 into a no-break space or leave it. They
   insert a space only where a rule says so explicitly (inside `т.е.`, after `№`, and by the percent
   setting), and remove spaces only around a closed-style dash between two plain words and by the
   percent setting (section 8.2). No insertion or removal changes what the rules read (section 4.3,
   invariant I2).
4. **Technical text is safe by construction.** A fragment that looks technical (a flag, a path, an
   address, a version, an identifier, anything the classifier does not recognize as a word or a
   number) is frozen, and the engine refuses every edit that touches it (sections 3.6, 3.7).
5. **Report, do not guess.** Where a rule cannot decide (an unpaired quote, `10 - 20`), the text
   stays as it is and the caller gets a diagnostic (section 12.4).
6. **Explicit configuration.** The language, the rules and their settings are fixed when the
   typographer is built; the output kind is given with every call. Nothing is read from the
   environment, the locale, the clock or the network.
7. **Languages are data.** Language differences live in a data table; rules never branch on a
   language tag (section 7).

### 1.5 Out of scope in version 1

| Not in version 1                                                                               | Reason                                                                                                                          |
| ---------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------- |
| HTML parsing (text between tags, skipping `code`, `pre`, `kbd`, attributes)                    | The task keeps it out of the core; stage 6 is not done. The core is ready for it: `Piece::Markup` and `Piece::Protected` (2.6). |
| Markdown                                                                                       | A caller's format. Callers pass text nodes as text and code as protected pieces; `dash.dialogue` can be disabled for lists.     |
| Normalizing whitespace (repeated spaces, tabs, trimming, line ends)                            | Breaks code and alignment; a caller's decision before typesetting. `lint` reports nothing about it either.                      |
| Spaces after `, : ; ! ?` and removal of spaces before them                                     | Breaks addresses, lists and IPv6 (section 10.5). This is source editing, not typesetting.                                       |
| Spelling and grammar: dictionary hyphens (`кто то`), `25-ый`, commas, `ё`, accents             | Not typography; changes letters and meaning.                                                                                    |
| Replacing look-alike Latin letters in Cyrillic words, and any other letter change              | Forbidden by the task (negative case "Latin letters next to Cyrillic").                                                         |
| Author's punctuation: `!!`, `??`, `,,`, `..`, `!?` → `?!`                                      | Editorial change of intonation and meaning.                                                                                     |
| Rewriting dates (ISO to local form), weekday and month case                                    | Breaks technical text (negative case `2026-11-21`).                                                                             |
| Fractions `1/2` → `½`, `20C` → `20 °C`, `м2` → `м²`                                            | Change digits or letters; break dates (`1/2/2026`), identifiers (`Код C`) and model names (`m2`).                               |
| Digit grouping of numbers in text (`1000000` → `1⍽000⍽000`)                                    | A digit run is one token and English groups with a comma (section 8.8); `format_number` groups values; section 15.5, item 17.   |
| Currency placement (`$100` → `100 $`), `руб.` → `₽`, percent and currency formatting of values | House style and i18n formatting, not typesetting.                                                                               |
| Reformatting telephone numbers                                                                 | National formats. Only no-break grouping (`nbsp.phone`, section 8.7) and nowrap output (11.5).                                  |
| A no-break space before the last short word of a sentence                                      | Noisy in English, useless in interface strings.                                                                                 |
| A leading ellipsis glued to a word in Interface (`...args`)                                    | Indistinguishable from a spread operator; `ellipsis.inner` exists in Prose only.                                                |
| Hanging punctuation, optical alignment, inserting soft hyphens                                 | Layout work that needs markup. A U+00AD already in the input is transparent (section 5.4).                                      |
| Language detection, switching language inside one call                                         | Nondeterministic at boundaries. One call is one language; mixed text is defined in section 5.7.                                 |
| Unicode normalization (NFC)                                                                    | Needs large tables; an NFD word simply does not match a list (section 5.5).                                                     |
| Quotes spanning paragraphs (an opening quote at the start of every paragraph)                  | The scope of quote pairing is a paragraph; such quotes stay straight with a diagnostic (section 6.10).                          |
| Ellipsis for ranges of negative numbers, other range conventions                               | No verified norm; `format_range` uses the CLDR pattern for every value (section 2.7).                                           |
| `en-GB` and other languages                                                                    | Added later as data (section 7.7). British punctuation placement with quotes is not verified against a primary source.          |
| User-defined rules as code (a `Rule` trait)                                                    | Would freeze the internal token model. Extension in version 1 is through data and settings.                                     |
| `no_std`                                                                                       | The run-time path uses a thread-local buffer (2.12). Nothing in the API prevents `no_std + alloc` later.                        |
| Streaming input                                                                                | Quote pairing needs a whole paragraph.                                                                                          |
| `serde`, a command-line tool                                                                   | Callers map their own settings files through `FromStr` on `Rule` and `RuleSet`.                                                 |
