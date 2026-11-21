# typolish design: Testing

Part of the [typolish design](./README.md). Section numbers are those of the whole design.

## 13. Testing

### 13.1 Principles

- Tests live in `dev/typolish/tests/` and run from `dev/` with no network; helpers are in
  `tests/common/mod.rs`. Slow tests are `#[ignore]` and run by hand in stage 5 and before releases.
- Fixtures are synthetic. No text from internal networks or third-party works enters the repository;
  host names are neutral (`example.com`, `db.example.internal`).
- Expectations are written with visible markers: the helper `vis("5⍽МБ")` replaces `⍽` with U+00A0
  and `⎵` with U+202F, and another helper checks that inputs contain no markers. On failure the
  helper prints invisible characters as markers.
- The reference implementation is not run by the tests. Its recorded pairs are plain tables; its
  name appears nowhere in the repository (a script outside the repository checks `git diff --cached`
  before every commit, since a test in the repository would have to contain the name).

### 13.2 Rule tables

`tests/rules_<group>.rs` (`quotes`, `dash`, `range`, `ellipsis`, `nbsp`, `number`, `symbols`): for
every rule and every language, an array of `(&str, &str)` pairs. Each rule is tested alone
(`RuleSet::EMPTY.with(rule)`) and inside both built-in sets. Required rows per rule: firing; not
firing at each boundary of its pattern ("does not touch" items of section 8); typeset input (no
change, `Cow::Borrowed`); a slot next to it; the rule disabled (no change); and the `Change` record
with the right rule in `report`. Specific rows and further tables:

- classifier tables (unit tests of `engine/classify.rs`, stage 2): `Ph.D.` and `А.С.` are
  `Abbreviation`, `report().protected()` is empty.
- `tests/rules_quotes.rs`: ru `"?"` → `«?»`, `«»` unchanged, `report().protected()` empty; en
  `He is 5'10" tall.` (Prose) → `He is 5′10″ tall.`; en `the 90's` → `the 90’s`; en `1'000'000` →
  unchanged; ru `Он сказал "" и ушёл.` → `Он сказал «» и ушёл.`; ru `Пустая строка "".` →
  `Пустая строка «».`; ru `Он сказал """ и ушёл.` → unchanged, `AmbiguousQuote` ×3; ru
  `Задайте key="" и нажмите "OK".` → `Задайте key="" и нажмите «OK».`; ru `a""b` → unchanged, no
  diagnostic; en `"a 'b c" 5'` → `“a 'b c” 5′` (Prose) and `“a 'b c” 5'` (Interface); en
  `“Say "hi" at 5”` → `“Say ‘hi’ at 5”`; ru `Цена 27»` → unchanged, `UnmatchedQuote` (info); ru
  `Папка "/etc/" пуста` → `Папка «/etc/» пуста`; en `Run "x->" now` → `Run “x->” now`; ru
  `Адрес "::1"` → `Адрес «::1»`; en `She loves 'the dogs'` → unchanged, and with `Never` →
  `She loves 'the dogs’`; en `He said ‘x’(see note)` → unchanged, `UnmatchedQuote` (info); ru
  `Он сказал:"привет" и ушёл.` → unchanged, no diagnostic (frozen fragment, section 10.8).
- `tests/rules_dash.rs`: ru `Ждите - ...` with `dash.spaced-hyphen` alone → unchanged, no
  `AmbiguousHyphen`, `report().protected()` empty; en Closed `Moscow — the capital` → unchanged,
  `Cow::Borrowed`; ru `Москва – столица` → `Москва⍽– столица` (en dash kept, glue from `nbsp.dash`);
  `-- file` at a line start: ru (Prose) → `—⍽file` (`dash.dialogue`), en → unchanged; en Closed
  `word  -  word` → unchanged, no diagnostic; `5 - x`, `x - 5`, `{a} - x` → unchanged,
  `AmbiguousHyphen`; ru `Глава 5 - введение` → `Глава 5⍽— введение`; en `He said - " a cat"` in
  Prose and Interface (the quote counts as an opening quote only by its role in section 6.3); en
  Closed `Sign in - it's free` → `Sign in—it's free` and `Sign up - it's free` →
  `Sign up—it's free`.
- `tests/rules_range.rs`: `1941-1945 гг.` → the `Change` record of the dash names `range.digits`;
  `10-10` → unchanged, no diagnostic.
- `tests/rules_ellipsis.rs`: `Ждите - ...` with `ellipsis.dots` alone → `Ждите - …`; `...` alone →
  `…`; in both `report().protected()` is empty. `и т.д...` → `и т.д…`. Boundaries:
  `Подождите...\nДальше` → `Подождите…\nДальше`, `... слово` → `… слово`, `Слово ...` → `Слово …`,
  ru `Что?...\nДа` → `Что?..\nДа`; `Подождите..." дальше` in Prose and Interface (the quote counts
  as a closing quote only by its role in section 6.3).
- `tests/rules_number.rs` (`number.times`): `10 x 5` with `add_units(["x"])` → `10 × 5`; `a × b` →
  unchanged, `report` lists `Technical(Unclassified)` for `×`; `10 X 5` → `10 × 5`; `10 Х 5` →
  `10 × 5`; `010 x 5` → unchanged.
- `tests/rules_symbols.rs` (`symbols.copyright`): `Brand(Tm)` → unchanged; `symbols.arrows` and
  `symbols.math`: `x  ->  y`, `x\t>=\t5` → unchanged, no diagnostic.
- `tests/rules_nbsp.rs` (stage 3): ru `А.С. Пушкин` → `А.С.⍽Пушкин` (`nbsp.initials`); `и т.д...` →
  `и⍽т.⍽д…`; `и т. д…` → `и⍽т.⍽д…`; en `etc...` → `etc…` (`etc` with its stop is `Dictionary`);
  `nbsp.signs`: `¶IV` → `¶⍽IV`, not borrowed; ru `и т. д.` with `narrow_glue` for abbreviations →
  `и\u{A0}т.\u{202F}д.`; ru `5 м2` → `5⍽м2`, `5–10 м2` → `5–10⍽м2` (Prose), `5 м2x` → unchanged; en
  `I said no. Then I left.` → unchanged; en `See No. 5` → `See No.⍽5`; ru `См. рис. 5` →
  `См.⍽рис.⍽5`; ru `21 Ноября` → `21⍽Ноября`; en `chapter 5 may be` → unchanged; en `on 5 May` →
  `on 5⍽May`; ru `Привезли 5 т груза` → `Привезли 5⍽т груза`; en `Attached` `100⍽%` → unchanged; ru
  `1990–2000 г. Затем`, `2010-11 г. Затем`, `5–10 ч. Потом` → no glue after `г.` or `ч.`, while
  `§§ 5-7 г. Москвы` → `г.⍽Москвы` (a label range); ru `21.11.2026 г.` → `21.11.2026⍽г.`; ru
  `Договор от 21.11.2026 г. Затем` → no glue after `г.`, and `report().protected()` lists
  `Technical(Version)` for the date; ru, Prose and Interface: `в §5` → `в⍽§⍽5`, `в ¶5` → `в⍽¶⍽5`,
  `и ¶ 5` → `и⍽¶⍽5`; en `J. R. R. R. Tolkien` → unchanged (Interface).
- `tests/numbers.rs`: `format_number` and `format_range` for ru and en on integers, negatives,
  ranges with negative ends (`−5–−1`) and equal ends (`8000–8000`), decimals (`Number::decimal`
  keeps precision), `f32` values (`0.1f32` → `0,1`, `1.1f32` → `1.1`, `f32::MAX`), `i128::MIN`,
  `u128::MAX`, `f64::NAN`, infinities, `-0.0`, the grouping thresholds (4 and 5 digits),
  `min_grouping_digits(0)` (no grouping for `1234567890`), `min_grouping_digits(4)` (`123456`
  ungrouped, `1234567` grouped), `COMMON` (`1234567` ungrouped), `Number::fixed` ties, every output
  kind, and nowrap with and without glue (`1234` in ru has no element); the builder rejects
  `group_separator(' ')`, `group_separator('\u{2009}')`, `minus_sign(' ')` and `minus_sign('"')`
  with `InvalidSetting`; `group_separator('\u{202F}')` builds, and nowrap output wraps `12⎵345` in
  one element.
- `tests/output.rs`: every output kind on a fixed set of inputs, including unpaired quotes,
  backslashes in protected spans, line breaks, U+2028, U+2029 and blank lines between paragraphs
  with quotes on both sides, in every output kind, `<script>`, `${`, characters above the BMP;
  inputs that no rule touches but the style must encode (`a</script>b` in `JS` with Interface,
  `AT&T` in `JSON`, `Да` with `non_ascii()`), which must never come back borrowed; nowrap groups
  with U+202F, U+2011 and a protected span next to glue.
- `tests/parts.rs`: templates, including the walk-through of section 2.14, select variants, `Other`
  slots, nowrap groups starting and ending at slots, the slot participation table of section 2.6 row
  by row (with `{Date}-{Date}` → `{Date}–{Date}` and `{Number}-{Date}` unchanged), labels
  (`№{n} г. Москвы`), and backtick protection across pieces (``Run `git checkout -- { $b }` ``);
  tokens across pieces: `[Text("Ждём."), Text("..")]` → `Ждём…` with two `Change` records, and
  `[Text("Ждём."), Markup("<b>"), Text("..")]` → unchanged; percent and signs next to slots: `{n}%`
  in ru with `PercentSpacing::Spaced` and with `Language` → `{n}⍽%`; `{n}‰` with
  `RuleSet::EMPTY.with(Rule::NbspUnits)` and `Spaced` → `{n}⍽‰`; `¶{n}` → `¶⍽{n}`; none of them
  borrowed; `{a}-5 кг` → `{a}-5⍽кг`; `5-{d} г.` → unchanged; ru `{d} г. Затем` (a `Date` slot) → no
  glue after `г.`; ru `Text("Скажи \"раз\n")`, `Slot(Other)`, `Text("\nдва\"")` → quotes paired
  `«раз … два»`, one window; an empty list of pieces → `texts() == [""]` (borrowed), no slots,
  `changed() == false`.
- `tests/api.rs`: the API contracts of section 2 that are not output: builder validation and error
  choice, `FromStr` of `Rule`, `RuleSet` and `Language`, tag round trips, accessors of `Report`,
  `Change`, `ProtectedSpan` and `Diagnostic`. Rows: a builder with two violations (`RuleConflict`
  and `InvalidSetting`) gives equal `Error`s for two orders of its calls; the accessors of
  `ProtectedSpan` on the example of section 12.5 (`kind() == Technical(Colon)`, `piece() == 0`,
  `range() == 25..34`); `RuleSet::from_str`, one row per case of section 2.3: an unknown id →
  `UnknownRule`, an id both added and removed, a base name after the first item (with and without a
  sign), an empty item and an empty string → `InvalidRuleSet` with the reason of section 2.3;
  `" prose , +number.times "` → Ok, `"+nbsp.dash,+nbsp.dash"` → Ok, `"   "` → `InvalidRuleSet`,
  `"+ nbsp.dash"` → `UnknownRule`; `protect_between`: `("|", "|")` → Ok, `("[", "[[")` →
  `InvalidSetting`, `("{{", "}}")` together with `("{{", "]]")` → `InvalidSetting`, `("a1", "|")` →
  `InvalidSetting`; `Language::RU_RU` equals a copy built by
  `LanguageBuilder::from_language(&Language::RU_RU, "ru-RU")` with no changes, and their hashes are
  equal; a copy with a different list of short words is not equal.
- `tests/fluent_isolation.rs`: run-time strings with U+2068 and U+2069 around values give the same
  typesetting as without them, with the isolates unchanged (section 5.4).

### 13.3 Negative cases

`tests/negative.rs`:

- one function per row of section 10.5 (`neg_flag_force`, `neg_ls_la`, `neg_lone_double_hyphen`,
  ..., `neg_identifier_underscore`), for ru and en, alone, in the two sentence frames of each
  language, and as `Piece::Text` next to slots. Through `report`, the tokens of the case stay byte
  for byte with no edit but glue outside them. In Interface the output also equals the input and is
  `Cow::Borrowed`, except for the two cases that section 10.5 names (row 24's `if a<b and c>d` in
  English, with its expected glue; row 25, checked in English with short Cyrillic words); in Prose
  the expectation is the last column of section 10.5. The comment of each test names its mechanisms,
  and the test also checks that `report` lists the expected `TechKind` for frozen fragments;
- the stress strings of section 10.7; the interaction strings of section 10.7 are compared by exact
  output in both sets, written by hand where the form is not in the scouting corpus; `@user` and
  `@here` come out unchanged in Interface, and `report` lists `Technical(Email)` for them;
- fenced blocks: a fenced block indented by three spaces with an info string is protected whole; a
  longer closing run does not close;
- Swiss grouping `1'000'000` (frozen, section 10.8): unchanged in both sets, for ru and en;
- all 405 negative cases of the scouting corpus (groups `neg.flags`, `neg.net`, `neg.ver`,
  `neg.ids`, `neg.ident`, `neg.list`, `neg.html`, `neg.mixed`, `neg.shortlat`, `neg.lookalike`; ru
  207, en 198, frames included), converted once by a script outside the repository into a plain
  table, with internal host names replaced by neutral ones. Some of these cases sit in sentences
  with listed short words (`Это a и b в тексте`), so the check is through `report`: the case's token
  stays byte for byte, and the only other edits allowed are glue marks set by `nbsp.*` rules outside
  the token. In Interface that holds for every case. In Prose the token may also change in the
  groups that section 10.5 allows (the lone `--` between words, literal ranges, arrows), and only as
  recorded.

### 13.4 Reference pairs

The 820 cases of the scouting corpus, with the output of the reference implementation (default
settings, recorded outside the repository), become `tests/reference/{ru,en_us}.rs` through a script
outside the repository. Every case is in one of two groups, each a plain table row:

- `AGREE`: our output equals the recorded one;
- `DIVERGE`: our output is written out together with a reason code from section 8.12 (`NoGuess`,
  `LookAlike`, `NoSpaceEdit`, `NoDateRewrite`, `AmbiguousHyphen`, `PercentAsWritten`, `Technical`,
  `AuthorPunctuation`, `DashStyleEn`, `RangeLiteral`, `NoLastShortWord`, `Slots`, `NoSpelling`,
  `Apostrophe`, `EnNbsp`, `LegalMarkContext`, `Primes`, `WideGlue`, `NoDigitChange`,
  `AlreadyTypeset`, `NoTrim`, `KeepSpaces`, `EllipsisDigits`).

The test compares our output with the expectation in both groups, using Prose (closest to the
reference defaults); for the negative groups also Interface (where the expectation is the input). A
failure prints the input, our output, the recorded output and the `report` table (section 12.5). The
stage 5 report prints a summary per reason code. The phrase used in tables and comments is
"reference implementation".

### 13.5 Properties

`proptest` generators: a grammar of risky pieces (straight and typed quotes, `-`, `--`, `—`, `–`,
`.`, `...`, `…`, numbers, units, short words of both languages, technical fragments from section
10.5, slots of every kind for templates, protection delimiters, spaces of every kind and line
breaks, emoji with ZWJ, combining marks, format characters); strings from the corpus with
insertions; and arbitrary Unicode strings for the no-panic test.

1. **Idempotence**: `f(f(x)) == f(x)` for {ru, en, COMMON} × {Prose, Interface, all, random subsets}
   × the three `SingleQuotes` modes, with fixed seeds for every split and merge of invariant I2
   (`Школа №5 г. Москвы`, `§5 ч. 2`, `т.е.`, `и т.д...`, `Moscow - the capital`,
   `See https://example.com/docs - the guide`, `He said - "a cat"`, `5'10"`, `90's`, `'90s`, `'tis`,
   `""`, `"""`, `key=""`, `"He said 'hi'"`, `"a 'b c" 5'`, `"/etc/"`, `‚Wort‘` with a
   `LanguageBuilder` language whose pairs are `„“` and `‚‘`, `100%` and `100 %` under every
   `PercentSpacing`, `100⍽%` under `Attached` and `Language` (en), en Closed `word⍽- word`,
   `Sign in - it's free`).
2. **Fixed-point oracle**: `f` repeated until nothing changes (at most three times) equals `f`; a
   difference prints the reports of both passes.
3. **Invariants**, one property each: I1 (edits never change a token kind; insertions and removals
   only of spaces), I2 (fragment kinds of `f(x)` match those of `x`, modulo the splits and merges of
   section 4.3), I3 (no output character of a rule triggers a rule), I4 (every match computed on
   `f(x)` has an empty edit, and every match on `x` outside the fragments merged or split by I2
   recurs on `f(x)` with the same rule and `consume`, except that a dash produced by
   `dash.double-hyphen` is matched by `dash.spaced-hyphen`, section 8.4), I5 (in a non-converging
   window no straight quote changed).
4. **Slots and protected spans**: number, order, kinds and payloads of slots are kept;
   `texts().len() == slots().len() + 1`; protected spans and markup are byte for byte in place.
5. **Technical fragments**: fragments classified as technical in the input appear in the output byte
   for byte.
6. **Letters and digits**: the sequence of letters and digits of the output equals that of the input
   (with `symbols.*` and `number.times` off).
7. **Graphemes**: every edit of `report` starts and ends at a cluster boundary of the input, and
   every cluster of the input that no replacement or removal covers appears byte for byte in the
   output. A cluster is an extended grapheme cluster of UAX #29 (a combining sequence, a ZWJ
   sequence, a keycap, a regional-indicator pair and `\r\n` are one cluster each), not a token:
   `(c)` → `©` replaces three clusters.
8. **Cow**: the output is `Borrowed` exactly when it equals the input byte for byte, for every
   output kind (`Html` and `Escaped` included), so the prefilter can never skip encoding.
9. **Recorder independence**: `typeset`, `typeset_with`, `report` and `lint` agree on the result and
   on the diagnostics; applying `report(x).changes()` to `x` yields the result.
10. **Pieces and strings**: a template of `Text` pieces only gives the same as `typeset` of their
    concatenation, protection delimiters included (they are matched across pieces, section 3.4),
    with the text cut at every byte boundary of a character, including inside runs of dots, dashes
    and spaces; applying `report_template(...).changes()` piece by piece yields the result.
11. **Determinism of order**: a set built in different orders of `with`/`without` gives byte-equal
    output.
12. **Output kinds**: `decode(typeset(x, out)) == typeset(x, Unicode)` and
    `typeset(decode(typeset(x, out)), out) == typeset(x, out)` for every output kind, nowrap styles
    included (an ordinary space inside an element decodes to U+00A0, section 11.5); decoders are
    test helpers.
13. **Round trip**: P1 on generated canonical strings (without the `NestedPossessive` pattern), P2
    on generated strings (section 12.2).
14. **No panic** on arbitrary strings and arbitrary pieces, including lone combining marks, control
    characters and noncharacters: 10 000 cases in a normal run, 1 000 000 in an `#[ignore]` test.

### 13.6 Unicode conformance

- `tests/graphemes.rs` runs every line of `GraphemeBreakTest.txt` of Unicode 17.0.0 (the version of
  `unicode-segmentation` 1.13.3) through both rule sets, alone and inside sentence frames, and
  checks through `report` that every edit starts and ends at a cluster boundary of the input and
  that every cluster of the input that no replacement or removal covers appears byte for byte in the
  output. The file is kept verbatim with its Unicode license header in `tests/fixtures/unicode/`,
  excluded from the published package (`exclude` in `Cargo.toml`); the test is skipped when the file
  is absent. A conformance test pins `unicode_segmentation::UNICODE_VERSION` to the version of the
  fixture, so a dependency update forces a fixture update. The file is fetched from unicode.org in
  stage 2.
- Tables for spaces with combining marks (section 5.3), NFD words (section 5.5), mixed scripts
  (section 5.7, with the units `μs` and `µs` raising no `MixedScript`) and format characters
  (section 5.4, with U+00AD inside words: `21 ноя\u{AD}бря` gets its glue, `в Мо\u{AD}скве` is a
  `Word`; `сло\u{200D}во` is a `Word`; `21 ноя\u{200C}бря` gets its glue (`21⍽ноя\u{200C}бря`); ZWNJ
  at the start of a text is a `Format` token; `"\u{200C}a` and `1\u{200D}2` are a `Symbol` followed
  by a letter or digit and come out unchanged; U+0020 + U+200D is a frozen `Space`; through
  `report`, no edit falls between a ZWNJ or ZWJ and the character before it).
- Tables for clusters led by a non-letter: keycaps `1️⃣` and `#️⃣`, `"` + U+0301, each alone and right
  after ASCII text (`Шаги 1️⃣-3️⃣`, `a"\u{301}b`), so that the ASCII fast path of section 3.5 hands
  them to the segmenter; they come out unchanged and are not numbers, signs or quotes. Clusters led
  by a `Prepend` character before ASCII: `\u{600}123`, `\u{6DD}` + space at the end of the text,
  `a\u{600}b`. They come out unchanged; `\u{600}1` is a `Symbol`, and no edit falls between U+0600
  and the ASCII character after it. A lexer unit test compares the cluster boundaries of the fast
  path with `grapheme_indices(true)` on every line of the fixture and on generated strings that mix
  ASCII with `Prepend`, `Extend`, `ZWJ` and `SpacingMark` characters.

### 13.7 Own corpus

There is no external corpus (decided by the maintainer on 2026-11-21); the corpus is written for the
project, in the canonical plain form, with no third-party texts:

```text
tests/corpus/
  ru-RU/prose.txt   ~150 lines: narration, dialogue, dates, numbers, units, abbreviations
  ru-RU/ui.tmpl     ~150 message templates with slots
  ru-RU/tech.txt    ~100 lines: commands, logs, addresses, versions mixed with prose
  en-US/prose.txt, en-US/ui.tmpl, en-US/tech.txt
  expected/         typeset output of every file in each set and output kind
  roundtrip-known.tsv
```

`.tmpl` is a test-only mini syntax: `id = text {n:number} {name:word} {d:date} {x}`, parsed by a
helper of about 50 lines; no Fluent dependency. The corpus serves idempotence, P1 and P2, load
measurements and snapshots. Snapshots in `expected/` are plain files compared byte for byte; a
failing test prints a diff, and files are updated deliberately by hand (no `insta`).
`.gitattributes` marks `tests/corpus/**` and `tests/fixtures/**` as `-text`, since they contain
U+00A0 and must not be normalized.

### 13.8 Load and allocation

- `benches/load.rs` (`harness = false`, std only: `Instant`, `black_box`, the median of N runs), run
  as `cargo bench --bench load` in release. It prints: the whole corpus per set and output kind
  (MB/s); one interface string of about 60 bytes and one without trigger bytes (prefilter), median
  and p99 over 100 000 runs, with and without `typeset_with`; time per byte for 1 KiB, 64 KiB, 1 MiB
  and 16 MiB of joined corpus and for one 1 MiB paragraph; and the pathological inputs of section
  3.12, with the peak memory for one paragraph of 10^5 ZWJ emoji clusters (section 3.12 estimates
  about 4 MB of tokens in each of the two buffers).
- `#[ignore]` test `load_is_linear`: time per byte for 16 MiB is at most twice that for 64 KiB,
  separately for each pathological input. No absolute thresholds: the machine is shared. Indicative
  targets, measured in stage 5 and not promised: at least 20 MB/s on the corpus in Unicode output, a
  median below 10 µs for one interface string.
- `tests/alloc.rs`: a separate test binary with a counting `#[global_allocator]` checks the table of
  section 2.12: zero allocations for an unchanged string after warm-up (both through the prefilter
  and through the full pipeline), one for a changed string, zero for `format_number` written into a
  buffer with room, zero in `Typographer::new`; ru `Сохранить` in Interface → `Cow::Borrowed`, no
  lexing; an iterator of template pieces that calls `typeset` adds no allocations beyond the table
  of section 2.12; after an input larger than 64 KiB the first short call allocates and the second
  does not.

Rejected: `criterion` (a heavy tree: `clap`, `serde`, `regex`, `rayon`) and `divan` (last release in
2025-04); linear behavior needs no statistics package.

### 13.9 Conformance and data

`tests/conformance.rs` checks repository facts: `LICENSE-*` at the root and in `dev/typolish/` are
byte-identical; `dev/typolish/README.md` is a symlink to the root README;
`Typographer: Send + Sync + 'static` and `Clone`; `Scratch: Send + Sync + Default`; `ORDER`,
`Rule::ALL` and the table of section 4.1 contain every rule that is not deprecated once, ids are
unique and of the documented form, `Rule::from_str(r.id()) == Ok(r)`;
`Language::from_tag(l.tag()) == Ok(l)` for `Language::builtin()` and `COMMON`, and
`from_tag("UND") == Ok(COMMON)`; `s.to_string().parse() == Ok(s)` for `RuleSet::EMPTY` (printed
`empty`), `ALL`, `PROSE`, `INTERFACE` and generated sets; the composition of Prose and Interface
matches section 9.2; the rustdoc of every builder setting names exactly the rules that list it in
the settings column of section 8.1; `src/` contains no `std::env`, `std::fs`, `std::net`,
`std::time` or `unsafe`; `src/lang/*.rs` contain no `fn`, `impl`, `match` or `if`; replacement
tables produce no letter and no digit; built-in languages × built-in sets build without error. A
unit test next to the prefilter checks, for every built-in set and language, that the trigger set
equals the union of the trigger constants of the active rules. Checks that read `docs/design/` skip
when the file is absent (the published package has no design).

`tests/data.rs` runs the validation of section 7.6 on every built-in table (so the order and
duplicates of `months` are checked too) and compares the CLDR values (quote pairs, decimal, group,
minimum grouping digits where not overridden, percent pattern, range pattern, `nan`, `infinity`,
month names (the union of format wide and abbreviated forms, without duplicates), units marked "CLDR
49") with an extract of cldr-json 49.0.0 kept in the test, so a CLDR update is a visible change. It
compares `LanguageData::COMMON` with the table of section 7.4 and checks `LanguageData::new("xx")`
field by field. It also checks that a table whose `QuotePairs` holds `「」` is rejected with
`Error::InvalidLanguageData`; that `LanguageBuilder` with the pairs `(«, »)` and `(«, „)` gives
`Error::InvalidLanguageData` and the typographer builder with the same pairs
`Error::InvalidSetting`; that `LanguageBuilder` with `apostrophe('ʼ')` gives
`Error::InvalidLanguageData`; that `Language::from_data` with the tag `EN-gb` gives
`Error::InvalidLanguageData`; and that `LanguageBuilder::new("EN-gb").build()` gives a language with
`tag() == "en-GB"`.

### 13.10 Documentation

Every public item has rustdoc with an example (`missing_docs = "deny"`); examples are doctests and
run with `cargo test`. README and CHANGELOG are in English. The README states the known limits of
section 10.8 and how to protect commands.
