# typolish design: Testing

Part of the [typolish design](./README.md). Section numbers are those of the whole design.

## 13. Testing

### 13.1 Principles

- Tests live in `dev/typolish/tests/` and run from `dev/` with no network; helpers are in
  `tests/common/mod.rs` and the property generators in `tests/common/strategies.rs` (lint allows:
  section 14.5). Slow tests are `#[ignore]` and run by hand in stage 5 and before releases.
- Fixtures are synthetic. No text from internal networks or third-party works enters the repository,
  except two extracts of standards kept with the source and license of each in a header and excluded
  from the published package: `tests/fixtures/unicode/` (section 13.6) and
  `tests/fixtures/whatwg-entities.txt` (section 13.2). Host names are neutral (`example.com`,
  `db.example.internal`).
- Expectations are written with visible markers: the helper `vis("5⍽МБ")` replaces `⍽` with U+00A0
  and `⎵` with U+202F (inputs in the tables are written with markers and go through `vis` too), and
  another helper asserts that a string passed to the typographer, after `vis`, contains no marker
  characters. On failure the helper prints invisible characters as markers.
- The reference implementation is not run by the tests. Its recorded pairs are plain tables; its
  name appears nowhere in the repository (a script outside the repository checks `git diff --cached`
  and the commit message before every commit, since a test in the repository would have to contain
  the name).

### 13.2 Rule tables

`tests/rules_<group>.rs` (`quotes`, `dash`, `range`, `ellipsis`, `nbsp`, `number`, `symbols`): for
every rule and every language, an array of `(&str, &str)` pairs. Each rule is tested alone
(`RuleSet::EMPTY.with(rule)`) and inside both built-in sets. Required rows per rule: firing; not
firing at each boundary of its pattern ("does not touch" items of section 8); typeset input (no
change, `Cow::Borrowed`); a slot next to it; the rule disabled (no change); and the `Change` record
with the right rule in `report`. Specific rows and further tables follow. Each specific row holds in
the configuration it names (a set, a rule "alone", or the rules it runs with). A row that names none
runs, in `rules_quotes.rs`, with the three quote rules and, in the other `rules_*.rs` files, with
the one rule of its file that the row exercises, alone (`RuleSet::EMPTY.with(rule)`). As in section
6.12, the outputs show only the edits of the rules that run; a row that names Prose or Interface
gives the output of the whole set.

- classifier tables (unit tests of `engine/classify.rs`, stage 2): `Ph.D.` and `А.С.` are
  `Abbreviation`, `report().protected()` is empty; `№`, `§§` and `¶` alone are `Sign` fragments, not
  frozen, and `Документ № 5` has an empty `report().protected()`; `¶IV` and `№5` are `Word`
  (check 23) with the `LABEL` flag, and so is `IV` in `¶ IV`; `(c)`, `©` and `(c)2026` are `Word`
  cores, not frozen, and `(c) 2026` has an empty `report().protected()`, while `2026(c)` falls
  through to check 24; `XIX`, `IV` and `MCMXCIV` are Roman numerals (`XIX-XX` is a `Range`, `IV` in
  `¶ IV` has `LABEL`), while `IIII`, `IC`, `VV`, `xix`, `Mix` and `ХIХ` (Cyrillic `Х`) are not
  (`IIII-V` and `xix-xx` are `Compound`, `VV` in `¶ VV` has no `LABEL`).
- `tests/rules_quotes.rs`: ru `"?"` → `«?»`, `«»` unchanged, `report().protected()` empty; en
  `He is 5'10" tall.` (Prose) → `He is 5′10″ tall.`; en `the 90's` → `the 90’s`; en `1'000'000` →
  unchanged; en `He is 5’10 tall.` → unchanged; ru `Он сказал "" и ушёл.` → `Он сказал «» и ушёл.`;
  ru `Пустая строка "".` → `Пустая строка «».`; ru `Имя: "\u{2068}\u{2069}"` →
  `Имя: «\u{2068}\u{2069}»`, `report().protected()` empty; ru `Он сказал """ и ушёл.` → unchanged,
  `AmbiguousQuote` ×3; ru `Задайте key="" и нажмите "OK".` → `Задайте key="" и нажмите «OK».`; ru
  `Токен "SGVsbG8gd29ybGQ=" и "имя".` → `Токен «SGVsbG8gd29ybGQ=» и «имя».`; en
  `‘Note: 'beta' builds of the developers' tools` → unchanged, `UnmatchedQuote` (info), no
  possessive `’` after a neutral `‘` (section 6.6); ru `a""b` → unchanged, no diagnostic; ru
  `Поле Значение:"" и "да".` → unchanged, `UnmatchedQuote` (warning); ru `Путь /etc/"" и "да".` →
  unchanged, `UnmatchedQuote` (warning); en `"She said "no.""` → `“She said ‘no.’”`; en
  `"a 'b c" 5'` → `“a 'b c” 5′` (Prose) and `“a 'b c” 5'` (Interface); en `“Say "hi" at 5”` →
  `“Say ‘hi’ at 5”`; ru `Цена 27»` → unchanged, `UnmatchedQuote` (info); ru `Папка "/etc/" пуста` →
  `Папка «/etc/» пуста`; en `Run "x->" now` → `Run “x->” now`; ru `Адрес "::1"` → `Адрес «::1»`; en
  `She loves 'the dogs'` → unchanged, and with `Never` → `She loves 'the dogs’`; en `'42'` with
  `Never` → `’42′` (Prose) and `’42'` (Interface); en `He said ‘x’(see note)` → unchanged,
  `UnmatchedQuote` (info); ru `Он сказал:"привет" и ушёл.` → unchanged, no diagnostic (frozen
  fragment, section 10.8).
- `tests/rules_dash.rs`: ru `Ждите - ...` with `dash.spaced-hyphen` alone → unchanged, no
  `AmbiguousHyphen`, `report().protected()` empty; en Closed `Moscow — the capital` with
  `dash.spaced-hyphen` alone → unchanged, `Cow::Borrowed` (the sets give `Moscow — the⍽capital`); en
  Closed `Moscow - the capital` in Prose and Interface → `Moscow—the⍽capital` (stage 2, without the
  `nbsp.*` rules: `Moscow—the capital`), and `Moscow—the capital` in both sets → unchanged,
  `Cow::Borrowed`; en Closed `5 - x - Moscow` with `dash.spaced-hyphen` alone → `5 - x—Moscow`, one
  `AmbiguousHyphen`, and `5 - x—Moscow` → unchanged, no `AmbiguousHyphen` (a `Joined` core is not an
  operand, section 8.4); ru `1920x1080 — монитор` with `nbsp.dash` alone → unchanged,
  `Cow::Borrowed` (a `Dimensions` core is not an operand); ru `Москва – столица` with `nbsp.dash`
  alone → `Москва⍽– столица` (en dash kept, glue from `nbsp.dash`); ru `10 – 20` with `nbsp.dash`
  alone and in both sets → `10⍽– 20` (a typed dash between numbers gets the glue too, section 8.7);
  en and ru, Interface: `git checkout -- file` and `Запустите -- сейчас.` → unchanged,
  `Cow::Borrowed` (`dash.spaced-hyphen` does not match `--`, section 8.4);
  `RuleSet::PROSE.without(Rule::DashDoubleHyphen)` with `Запустите -- сейчас.` → unchanged;
  `-- file` at a line start: ru (Prose) → `—⍽file` (`dash.dialogue`), en → unchanged; en Closed
  `word  -  word` and `word\t-\tword` → unchanged, no diagnostic; en Closed `word⍽- word` →
  `word⍽— word`; `5 - x`, `x - 5`, `{a} - x` → unchanged, `AmbiguousHyphen`; ru `5–10 - 20` →
  unchanged, `AmbiguousHyphen`; ru `1990–2000 - годы` with `dash.spaced-hyphen` and `nbsp.dash` →
  `1990–2000⍽— годы`; ru `Глава 5 - введение` with `dash.spaced-hyphen` and `nbsp.dash` →
  `Глава 5⍽— введение` (`dash.spaced-hyphen` alone gives `Глава 5 — введение`); en
  `He said - " a cat"` in Prose and Interface → `He said - " a⍽cat"` (no dash: a quote without a
  role is not an opening quote, section 6.3; `AmbiguousQuote` and `UnmatchedQuote`, no
  `AmbiguousHyphen`); en Closed, `dash.spaced-hyphen` alone: `Sign in - it's free` →
  `Sign in—it's free` and `Sign up - it's free` → `Sign up—it's free` (the sets also give `it’s`);
  ru `кто-то - что-то` with `dash.spaced-hyphen` and `nbsp.dash` and in both sets →
  `кто-то⍽— что-то` (a compound word is an operand of its script, section 8.2); ru
  `tls-server - agent-ca` in both sets → unchanged, `AmbiguousHyphen` (Latin compound words are
  foreign words); en `tls-server - agent-ca` → `tls-server — agent-ca`; en Closed
  `mother-in-law - father-in-law` in Prose and Interface → `mother-in-law — father-in-law` (no merge
  next to a compound word, section 8.4), and `mother-in-law — father-in-law` → unchanged,
  `Cow::Borrowed`; ru and en `123-45-67 - 123-45-68` in both sets → unchanged, `AmbiguousHyphen` (a
  compound word without letters counts as a foreign word, section 8.2); ru `cat - 123-45-67` →
  unchanged, `AmbiguousHyphen`; ru `123-45-67 - звоните` → `123-45-67⍽— звоните`; ru
  `IT-отдел - что-то` in both sets → `IT-отдел⍽— что-то` (a compound word of two scripts is a word
  of each, section 8.2).
- `tests/rules_range.rs`: `1941-1945 гг.` → the `Change` record of the dash names `range.digits`;
  `10-10` → unchanged, no diagnostic; `XIX-XX вв.` → `XIX–XX вв.`; `XX-XIX вв.`, `xix-xx вв.` and
  `ХIХ-ХХ вв.` (Cyrillic `Х`) → unchanged, the last with `MixedScript`.
- `tests/rules_ellipsis.rs`: `Ждите - ...` with `ellipsis.dots` alone → `Ждите - …`; `...` alone →
  `…`; in both `report().protected()` is empty. `и т.д...` → `и т.д…`, `Осталось 5...` →
  `Осталось 5…`, `10%...` → `10%…` (the dots attach to a number or a sign, section 8.6). Boundaries:
  `Подождите...\nДальше` → `Подождите…\nДальше`, `... слово` → `… слово`, `Слово ...` → `Слово …`,
  `Подождите...\tДальше` → `Подождите…\tДальше`, `Подождите...  Дальше` → `Подождите…  Дальше`,
  `Слово\t...и тогда` (Prose) → `Слово\t…и тогда`, ru `Что?...\nДа` → `Что?..\nДа`; ru
  `Подождите..." дальше` in Prose and Interface → `Подождите…" дальше` (the quote is closing by its
  role, section 6.3, so `ellipsis.dots` fires; `UnmatchedQuote`).
- `tests/rules_number.rs` (`number.times`): `10 x 5` with `add_units(["x"])` → `10 × 5`; `a × b` →
  unchanged, `report` lists `Technical(Unclassified)` for `×`; `10 X 5` → `10 × 5`; `10 Х 5` →
  `10 × 5`; `010 x 5` → unchanged.
- `tests/rules_symbols.rs` (`symbols.copyright`): `(c) 2026` → `© 2026`, `(C) 2026` and `(с) 2026`
  (Cyrillic `с`) → `© 2026`, `(c)2026` → `©2026`, `2026 (c)` → `2026 ©`, `2020-2026 (c)` →
  `2020-2026 ©`, `(c) 2020-2026` → `© 2020-2026`, `Brand (r)` → `Brand ®`, `Brand(tm)` → `Brand™`;
  `вариант (c)`, `f(c)`, `f(c) 2026`, `(R) знак`, `S(r) = πr²` → unchanged; `2020-2026(c)`,
  `(c)2020-2026`, `2026(c)` and en `Section 1001(c) applies.` → unchanged, and
  `report().protected()` lists `Technical(Unclassified)` for `2020-2026(c)`, `(c)2020-2026`,
  `2026(c)` and `1001(c)`; `Brand(Tm)` → unchanged; `McDonald’s(tm)` → `McDonald’s™`,
  `McDonald’s (r)` → `McDonald’s ®`, `Rock’n’Roll (tm)` → `Rock’n’Roll ™`, `IT-Systems (tm)` →
  `IT-Systems ™`, while `don’t(tm)`, `90’s (tm)` and `IT-Systems(tm)` → unchanged; in Prose
  `McDonald's(tm)` → `McDonald’s™`; `symbols.arrows` and `symbols.math`: `x  ->  y`, `x\t>=\t5` →
  unchanged, no diagnostic.
- `tests/rules_nbsp.rs` (stage 3): ru `А.С. Пушкин` → `А.С.⍽Пушкин` (`nbsp.initials`); `и т.д...`
  with `ellipsis.dots` and `nbsp.abbreviations` → `и⍽т.⍽д…`; `и т. д…` → `и⍽т.⍽д…`; en `etc...` with
  `ellipsis.dots` and `nbsp.abbreviations` → `etc…` (`etc` with its stop is `Dictionary`);
  `nbsp.signs`: `¶IV` → `¶⍽IV`, not borrowed; `¶XIX` → `¶⍽XIX`; `¶VV` → unchanged; ru `и т. д.` with
  `narrow_glue(NarrowGlue::ABBREVIATIONS)` → `и\u{A0}т.\u{202F}д.`; ru `5 м2` → `5⍽м2`, `5–10 м2` →
  `5–10⍽м2` (Prose), `5 м2x` → unchanged; en `I said no. Then I left.` → unchanged; en `See No. 5` →
  `See No.⍽5`; ru `См. рис. 5` → `См.⍽рис.⍽5`; ru Prose `стр. 9-12` → `стр.⍽9–12` and
  `в 2010-11 гг.` → `в⍽2010–11⍽гг.` (a range is an operand, section 8.2); ru `1990–2000 — годы` →
  `1990–2000⍽— годы` in both sets; en `the 90's` and `of don't` with `nbsp.short-words` alone →
  `the⍽90's` and `of⍽don't`, ru `и don't` → `и⍽don't` (a core of kind `Word` is a word whatever its
  number of tokens, section 8.2); ru `кто-то же` with `nbsp.particles` alone → `кто-то⍽же` (a
  compound word is a word, section 8.2); the particles of section 15.2, items 18 and 19, in both
  sets: `Знаешь ли ты` → `Знаешь⍽ли ты`, `Что ж,` → `Что⍽ж,`, `что же бы ты` → `что⍽же⍽бы ты`
  (section 15.2, item 24), `Что ж - ладно` → `Что⍽ж⍽— ладно`, an initial whose letter is a particle
  `Иван Б. Петров` → `Иван Б.⍽Петров`, typed `Знаешь ли⍽ты` → `Знаешь⍽ли⍽ты`, a particle after a
  `Word` slot (`{x}⍽же ты`) and after an `Other` slot (Prose `{x} же⍽ты`, Interface unchanged), and
  with `nbsp.short-length` alone `Знаешь ли ты` → unchanged (section 15.2, item 25); ru `21 Ноября`
  → `21⍽Ноября`; en `chapter 5 may be` → unchanged; en `on 5 May` → `on 5⍽May` (`on⍽5⍽May` in both
  sets); ru `Привезли 5 т груза` → `Привезли 5⍽т груза`; en with `PercentSpacing::Attached`: `100⍽%`
  → unchanged (only a single U+0020 is removed, section 8.7); a label number with a percent sign
  after spaces (section 8.7): ru `№ 5 %` and `№5 %` → `№⍽5⍽%` (`AsWritten`), `№⍽5%` (`Attached`),
  `№ 5%` → `№⍽5⍽%` (`Spaced`, ru `Language`), en `№ 5 %` → `№⍽5%` (`Language`), `№5%` and `§5%`
  unchanged in every setting, `№5 % - норма` → `№⍽5⍽%⍽— норма`, and `№5 %` → `№5⍽%` without
  `nbsp.signs`; ru `1990–2000 г. Затем`, `2010-11 г. Затем`, `5–10 ч. Потом` → no glue after `г.` or
  `ч.`, while `§§ 5-7 г. Москвы` glues `г.⍽Москвы` (a label range; with `nbsp.abbreviations` alone,
  and in Prose in full `§§⍽5–7 г.⍽Москвы`, pinned in `ranges_in_the_full_sets` of
  `tests/rules_range.rs` with `№ 5-7` → `№⍽5–7` and en `§§ 5-7` → `§§⍽5–7`); ru `21.11.2026 г.` →
  `21.11.2026⍽г.`; ru `Договор от 21.11.2026 г. Затем` → no glue after `г.`, and
  `report().protected()` lists `Technical(Version)` for the date; ru `21.11.26 г.` → `21.11.26⍽г.`;
  ru `Договор от 21.11.26 г. Затем` → no glue after `г.`; ru `0.2.27 г.` → unchanged (not a date
  core); ru, Prose and Interface: `в §5` → `в⍽§⍽5`, `в ¶5` → `в⍽¶⍽5`, `и ¶ 5` → `и⍽¶⍽5`; en
  `J. R. R. R. Tolkien` → unchanged (Interface); `nbsp.initials`: ru `Об этом сказал А. Петров.` →
  `Об этом сказал А.⍽Петров.`, ru `Выберите вариант А. После этого` →
  `Выберите вариант А.⍽После этого`, en `I chose plan B. Then I left.` →
  `I chose plan B.⍽Then I left.`, en `Neither did I. Then we left.` → `Neither did I.⍽Then we left.`
  (a single initial is glued after any word, section 10.8); ru `Выберите вариант Д. После этого` and
  en `I chose plan P. Then I left.` with `nbsp.abbreviations` alone → unchanged, and ru
  `П. 5 устава` with `nbsp.abbreviations` alone → unchanged while `п. 5 устава` → `п.⍽5 устава` (a
  single letter with a full stop matches an abbreviation only as written, section 7.1), and with
  `nbsp.initials` → `Выберите вариант Д.⍽После этого`, `I chose plan P.⍽Then I left.`.
- `tests/numbers.rs`: `format_number` and `format_range` for ru and en on integers, negatives,
  ranges with negative ends (`−5–−1`) and equal ends (ru `8⍽000–8⍽000`, en `8,000–8,000`), decimals
  (`Number::decimal` keeps precision), `Number::decimal` signs, leading zeros and zero (`+5` → `5`,
  `007` → `007`, ru `0012345` → `0⍽012⍽345`, ru `-1.50` → `−1,50`, ru `-0.00` → `0,00`) and its
  rejects (every case of section 2.11 → `InvalidNumber`), `Number::fixed(-0.001, 1)` → ru `0,0`,
  `f32` values (ru `0.1f32` → `0,1`, ru `1.1f32` → `1,1`, en `1.1f32` → `1.1`, `f32::MAX`), negative
  infinity (ru `−∞`), `i128::MIN`, `u128::MAX`, `f64::NAN`, infinities, `-0.0`, the grouping
  threshold (3 and 4 digits: ru `999`, `1⍽000`, en `999`, `1,000`), `min_grouping_digits(0)` (no
  grouping for `1234567890`), `min_grouping_digits(4)` (`123456` ungrouped, `1234567` grouped),
  `COMMON` (`1234567` ungrouped), `Number::fixed` ties, every output kind, and nowrap with and
  without glue (`123` in ru has no element); the builder rejects `group_separator(' ')`,
  `group_separator('\u{2009}')`, `group_separator('-')`, `minus_sign(' ')` and `minus_sign('"')`
  with `InvalidSetting`; `group_separator('\u{202F}')` and `group_separator('’')` build, and nowrap
  output wraps `12⎵345` in one element.
- `tests/output.rs`: every output kind on a fixed set of inputs, including unpaired quotes,
  backslashes in protected spans, line breaks, U+2028, U+2029 and blank lines between paragraphs
  with quotes on both sides, in every output kind, `<script>`, `${`, characters above the BMP;
  inputs that no rule touches but the style must encode (`a</script>b` in `JS` with Interface,
  `AT&T` in `JSON`, `Да` with `non_ascii()`), which must never come back borrowed; nowrap groups
  with U+202F, U+2011 and a protected span next to glue; for `CSS`, the inputs `a\u{0}b`, `a\u{c}b`,
  `a\r\nb`, `a\u{85}b` and `a\u{2028}b` give `a\fffd b`, `a\c b`, `a\d \a b`, `a\u{85}b` (a raw
  U+0085) and `a\u{2028}b` (a raw U+2028); the fixed set stays as a regression table next to the
  safe-alphabet property; the typographic-set table of section 11.2 is compared with
  `tests/fixtures/whatwg-entities.txt`, an extract of `entities.json` of the HTML Living Standard
  taken on 2026-11-21 (section 14.8), whose header names the source, the date and the license (CC BY
  4.0, the license of WHATWG Living Standards, with its attribution line; the file is an unmodified
  selection of entries), excluded from the published package like the Unicode fixture (section
  13.6), with this comparison skipped when the file is absent: every name the table writes exists
  there with that code point, U+202F and U+2011 have no name, and the test fails when
  `HtmlStyle::NAMED` writes a numeric reference for a character that has a name there; en with
  `quote_pairs` set to primary `« »` and nested `‹ ›` turns `"a "b" c"` into
  `&laquo;a &lsaquo;b&rsaquo; c&raquo;` with `HtmlStyle::NAMED` and into
  `\u00ABa \u2039b\u203A c\u00BB` with `EscapeStyle::JS`.
- `tests/parts.rs`: templates, including the Fluent mapping table of section 2.14 (each row as a
  piece list), select variants, `Other` slots, nowrap groups starting and ending at slots, the slot
  participation table of section 2.6 row by row (with `{Date}-{Date}` → `{Date}–{Date}` and
  `{Number}-{Date}` unchanged), labels (`№{n} г. Москвы`), and backtick protection across pieces
  (``Run `git checkout -- { $b }` ``); tokens across pieces: `[Text("Ждём."), Text("..")]` → `Ждём…`
  with two `Change` records, and `[Text("Ждём."), Markup("<b>"), Text("..")]` → unchanged; percent
  and signs next to slots: `{n}%` in ru with `PercentSpacing::Spaced` and with `Language` → `{n}⍽%`;
  `{n}‰` with `RuleSet::EMPTY.with(Rule::NbspUnits)` and `Spaced` → `{n}⍽‰`; `¶{n}` → `¶⍽{n}`; none
  of them borrowed; `{a}-5 кг` → `{a}-5⍽кг`; `5-{d} г.` → unchanged; ru `{d} г. Затем` (a `Date`
  slot) → no glue after `г.`; ru `Text("Скажи \"раз\n")`, `Slot(Other)`, `Text("\nдва\"")` → quotes
  paired `«раз … два»`, one window; an empty list of pieces → `texts() == [""]` (borrowed), no
  slots, `changed() == false`; `fill` inserts values unchanged (a value `<b>` stays `<b>` in
  `Output::Html`), as section 11.1 states; the flow of section 2.14 for `EscapeStyle::RUST` and
  `EscapeStyle::JSON`: ru `{n} руб.` with a `Number` slot, each text decoded by the test decoder of
  section 13.5 and joined with `format_number(1000, Output::Unicode)` gives `1⍽000⍽руб.` with a real
  U+00A0, equal to `fill` of the `Output::Unicode` template, and contains no `\`; the same flow for
  `Output::Html(HtmlStyle::NAMED.nowrap_class("nowrap"))` with ``Цена - 5 руб. в `C:\temp` `` writes
  each text with `{t:?}` (section 2.14), and the test decoder of `EscapeStyle::RUST` reads the body
  of every such literal back to the text unchanged.
- `tests/api.rs`: the API contracts of section 2 that are not output: builder validation and error
  choice, `FromStr` of `Rule`, `RuleSet` and `Language`, tag round trips, accessors of `Report`,
  `Change`, `ProtectedSpan` and `Diagnostic`. Rows: a builder with two violations (`RuleConflict`
  and `InvalidSetting`) gives equal `Error`s for two orders of its calls; the accessors of
  `ProtectedSpan` on the example of section 12.5 (`kind() == Technical(Colon)`, `piece() == 0`,
  `range() == 25..34`); a delimited span over ``Text("Run `ls")``, ``Text(" -la`")`` → two
  `Delimited` records, `0:4..7` and `1:0..5`; `MixedScript` on `Text("Мо")`, `Text("sква")` → one
  diagnostic, `piece() == 0`, `range() == 0..4`, equal in `report_template` and `lint_template`;
  `RuleSet::from_str`, one row per case of section 2.3: an unknown id → `UnknownRule`, an id both
  added and removed, a base name after the first item (with and without a sign), an empty item and
  an empty string → `InvalidRuleSet` with the reason of section 2.3; `" prose , +number.times "` →
  Ok, `"+nbsp.dash,+nbsp.dash"` → Ok, `"   "` → `InvalidRuleSet`, `"+ nbsp.dash"` → `UnknownRule`;
  `"-nbsp.dash"` → Ok (`EMPTY`), `"+"` and `"prose,-"` → `InvalidRuleSet` ("empty item"), `"+prose"`
  → `InvalidRuleSet` ("sign before a base set name"); `protect_between`: `("|", "|")` → Ok,
  `("[", "[[")` → `InvalidSetting`, `("{{", "}}")` together with `("{{", "]]")` → `InvalidSetting`,
  `("a1", "|")` → `InvalidSetting`; a language built by
  `LanguageBuilder::from_language(&Language::RU_RU, "ru-KZ")` prints `ru-KZ`, and
  `"ru-KZ".parse::<Language>()` → `UnknownLanguage`; a language built by
  `LanguageBuilder::from_language(&Language::RU_RU, "ru").short_words(["в"])` prints `ru`, and
  `"ru".parse::<Language>()` → `Ok(Language::RU_RU)`, which is not equal to it; `Language::RU_RU`
  equals a copy built by `LanguageBuilder::from_language(&Language::RU_RU, "ru-RU")` with no
  changes, and their hashes are equal; a copy with a different list of short words is not equal.
  Rows of `tests/api.rs` and the related checks are split into further files (section 14.6): builder
  validation and error choice in `tests/builder.rs`; `Language`, tag round trips and the activity of
  rules per language in `tests/language.rs`; the compile-time and auto-trait commitments of sections
  2.4 and 2.13 in `tests/traits.rs`; the application methods, `Cow` and the scratch (sections 2.5,
  2.6, 2.12) in `tests/typeset_api.rs`; `FromStr` and the string round trips of `Rule` and `RuleSet`
  against the tables of sections 2.3, 4.1 and 9.2 in `tests/rules.rs`; the self-tests of
  `tests/common/` in `tests/helpers.rs`. `tests/api.rs` keeps the rows above that none of these
  files holds.
- `tests/lint.rs`: every diagnostic kind of section 12.4 with a firing and a non-firing row;
  `TypographicChar` is absent inside `https://example.com/a–b`, inside backticks and in `Markup`,
  absent for `©` with `symbols.copyright` off, present for `©` in `© 2026` with `symbols.copyright`
  on (a lone legal mark is not frozen, section 3.6), absent for `©` in `© Example Corp` with
  `symbols.copyright` on (`to_plain` returns the input), absent for a typed `–` with spaces (ru
  Prose: `Москва – столица`, `10 – 20`, where `to_plain` returns the input), exactly one entry, on
  the U+00A0, for ru Prose `Москва⍽– столица` and `10⍽– 20`, present for `—` between words with the
  dash rules on; inputs without a trigger byte keep their diagnostics and protected spans through
  `report` and `lint` (no prefilter, section 3.2): ru Interface `Мoсква` (Latin `o`) →
  `MixedScript`; `report_template([Slot(n, NUMBER), Text("МБ")])` → `SlotGlue` (warning);
  `[Slot(a), Slot(b)]` → `SlotGlue` (info); ``don`t`` → `UnclosedDelimiter`; `` `code` `` →
  `report().protected()` holds one `Delimited` record.
- `tests/fluent_isolation.rs`: run-time strings with U+2068 and U+2069 around values give the same
  typesetting as without them, with the isolates unchanged, an empty value included
  (`"\u{2068}\u{2069}"` → `«\u{2068}\u{2069}»`, as `""` → `«»`), and an inch mark after an isolated
  number (`Экран \u{2068}27\u{2069}" и "ручка"` → `Экран \u{2068}27\u{2069}″ и «ручка»`, Prose)
  (section 5.4).

### 13.3 Negative cases

`tests/negative.rs`:

- a table of the rows of section 10.5, each with its number, a name (`flag_force`, `ls_la`,
  `lone_double_hyphen`, …, `underscore_identifier`), its mechanisms and the `TechKind` of its frozen
  fragments, run for ru and en, alone, in the two sentence frames of each language, as `Piece::Text`
  between two slots with spaces, and as `Piece::Text` glued to a `Number` slot (the output equals
  the input where the row stays unchanged; row 10 glued to a slot keeps its hyphen in both sets,
  section 10.6). Through `report`, the tokens of the case stay byte for byte with no edit but glue
  outside them, and the frozen fragments have the expected `TechKind`. In Interface the output also
  equals the input and is `Cow::Borrowed`, except for the two cases that section 10.5 names (row
  24's `if a<b and c>d` in English, with its expected glue; row 25, checked in English with short
  Cyrillic words); in Prose the expectation is the last column of section 10.5. A failure prints the
  row's number, name and mechanisms;
- the stress strings of section 10.7; the interaction strings of section 10.7 are compared by exact
  output in both sets, written by hand where the form is not in the scouting corpus; `@user` and
  `@here` come out unchanged in Interface, and `report` lists `Technical(Email)` for them;
- fenced blocks: a fenced block indented by three spaces with an info string is protected whole; a
  longer closing run does not close;
- Swiss grouping `1'000'000` (frozen, section 10.8): unchanged in both sets, for ru and en;
- all 405 negative cases of the scouting corpus (groups `neg.flags`, `neg.net`, `neg.ver`,
  `neg.ids`, `neg.ident`, `neg.list`, `neg.html`, `neg.mixed`, `neg.shortlat`, `neg.lookalike`; ru
  207, en 198, frames included), which are the rows of the `neg.*` groups in
  `tests/reference/{ru,en_us}.rs` (section 13.4), the same tables that section 13.4 reads;
  `tests/negative.rs` includes them with `#[path]` (section 14.6) and selects these rows by case id;
  the script outside the repository that wrote the tables replaced internal host names by neutral
  ones. Some of these cases sit in sentences with listed short words (`Это a и b в тексте`), so the
  check is through `report`: the case's token stays byte for byte, and the only other edits allowed
  are glue marks set by `nbsp.*` rules outside the token. A case of several fragments with a listed
  short word between them gets that glue between its fragments, as row 24 of section 10.5 does
  (`vec<int> and⍽map<k,v>` in English, case `neg.html.ctx.en-US.759`); its fragments stay byte for
  byte. In Interface that holds for every case. In Prose the token may also change in the groups
  that section 10.5 allows (the lone `--` between words, literal ranges, arrows), and only as
  recorded.

### 13.4 Reference pairs

The 820 cases of the scouting corpus, with the output of the reference implementation (default
settings, recorded outside the repository), become `tests/reference/{ru,en_us}.rs` through a script
outside the repository, data modules included by the test target `tests/reference.rs` (section
14.6). Every row holds a case id, the input and the recorded output. A case id starts with the
corpus group of the case and ends with its language and number (`neg.list.ru.518`,
`neg.list.ctx.ru.519`, `quote.apostrophe.en-US.023`); the `neg.*` groups of section 13.3 are found
by this prefix. From stage 5 every case is also in one of two groups, each a plain table row:

- `AGREE`: our output equals the recorded one;
- `DIVERGE`: our output is written out together with one or more reason codes from section 8.12
  (`NoGuess`, `LookAlike`, `NoSpaceEdit`, `NoDateRewrite`, `AmbiguousHyphen`, `PercentAsWritten`,
  `Technical`, `AuthorPunctuation`, `DashStyleEn`, `RangeLiteral`, `NoLastShortWord`, `Slots`,
  `NoSpelling`, `Apostrophe`, `NestedSingle`, `EnNbsp`, `RuNbsp`, `LegalMarkContext`, `Primes`,
  `WideGlue`, `NoDigitChange`, `MinusSign`, `AlreadyTypeset`, `NoTrim`, `KeepSpaces`,
  `EllipsisDigits`). A row with several causes names each of them.

The test compares our output with the expectation in both groups, using Prose (closest to the
reference defaults). A row whose input holds `{name}` slots (the `quote.subst`, `quote.already` and
`mix` cases) runs as a template: the slot kind comes from the name (`n`, `a`, `b`, `pct`, `price`
are numbers, `from` and `to` dates, the rest words), and the result is compared as the template
written out with its `{name}` slots. A `DIVERGE` row must differ from the recorded output, must name
a reason, and where section 8.12 names a diagnostic for a reason of the row (`NoGuess` with
`UnmatchedQuote`, `LookAlike` with `MixedScript`, `AmbiguousHyphen`) the report (`report_template`
for a template row) holds that diagnostic. Every reason has at least one row. The cause of a row is
decided by the script that wrote the tables and checked by hand: a row that differs in more than
glue gets the reasons of its content change, and a glue mark that differs outside a changed fragment
gets the reason that explains it, if section 8.12 has one: `DashStyleEn` for the space before a
spaced dash in en-US (`nbsp.dash` needs the style `SpacedGlued`, so `Closed` never glues it,
`Moscow⍽— the` against `Moscow — the`), `Slots` for glue next to a slot that the reference
implementation reads as text (`{n}⍽МБ` against `{n} МБ`); otherwise `NoLastShortWord` (the reference
implementation glues, we do not), `EnNbsp` or `RuNbsp` (we glue, it does not) or `WideGlue` (the
character differs). The 405 cases of the `neg.*` groups (section 13.3) are also run with Interface
and checked as section 13.3 says: the case's token byte for byte through `report`, glue of `nbsp.*`
rules outside it allowed; whole-string equality with the input is not required, because Interface
keeps `nbsp.short-words` (`Он вызвал ls на⍽сервере.`). Both sections read the same tables,
`tests/reference/{ru,en_us}.rs`. Stage 2 generates them with every row (case id, input and recorded
output), which `tests/negative.rs` reads from then on; stage 5 adds to each row its group and reason
code, and adds the target `tests/reference.rs` (section 15.4). A failure prints the input, our
output, the recorded output and the `report` table (section 12.5). The stage 5 report prints a
summary per reason code. The phrase used in tables and comments is "reference implementation".

**Provenance and drift.** Each generated file (`tests/reference/ru.rs` and
`tests/reference/en_us.rs`, which also hold the negative cases of section 13.3) starts with a
comment: reference implementation version 7.9.1, recorded 2026-11-21, default settings, SHA-256 of
the input corpus, SHA-256 of the recorded outputs. The tables are the record of that version; the
conversion scripts stay outside the repository (section 15.2, decision 3). A newer version of the
reference implementation changes nothing by itself: a re-recording is a deliberate commit that
updates the header, and every row whose recorded output changed is reviewed by hand and either kept
in `AGREE` or moved to `DIVERGE` with a reason code. The stage 5 comparison (section 12.5) uses the
version named in the header.

### 13.5 Properties

Test files: properties 1 to 4, 6, 8 to 12 and 14 to 17 live in `tests/properties.rs` (the prefilter
part of property 8 is a unit test, below); properties 5 and 7, and the parts of properties 4, 9 and
14 that `report` shows on strings, live in `tests/engine_properties.rs`; property 13 lives in
`tests/roundtrip.rs` together with the table test of `roundtrip-known.tsv`; `tests/corpus.rs`
compares the snapshots of section 13.7 (`expected/`) and runs idempotence on the corpus; the
`#[ignore]` test `load_is_linear` of section 13.8 lives in `tests/load.rs`.

`proptest` generators: a grammar of risky pieces (straight and typed quotes, `-`, `--`, `—`, `–`,
`.`, `...`, `…`, numbers, units, short words of both languages, technical fragments from section
10.5, slots of every kind for templates, also next to signs (`{n}%`, `№{n}`, `{a}-{b}`), protection
delimiters, spaces of every kind and line breaks, emoji with ZWJ, combining marks, format
characters, and the characters that output kinds encode: `& < > " ' \` and backtick, `$`, `{`, `${`,
`</script>`, `</style>`, `-->`, `&nbsp;` and U+00A0 typed as text, C0 controls (U+0000, U+000C
included), U+007F, U+0085, U+2028, U+2029, characters above the BMP, and a typographic character
followed by a hex digit); strings from the corpus with insertions; and arbitrary Unicode strings for
the no-panic test.

A structured grammar (`grammar_sentence` in `tests/common/strategies.rs`, run by `run_grammar` and
`run_grammar_long`) builds sentences instead of joining fragments: words of the language (compound
words and words with an apostrophe among them), short words and particles, abbreviations compact and
with spaces (`т. е.`, `и т.д.`, `St.`, `p.m.`), numbers (ranges with a Roman or a decimal side among
them, which a `%` after them makes a percentage range, section 8.2) with units after them, with a
space or attached, labels (`№5`, `§V`), percentage ranges and percentages (a Roman or a decimal
side, a typed `–` or a hyphen, the sign attached or after a space: `XIX–10%`, `1,5–2,5%`, `I-5%`,
`XIX–10 %`), alone, after a spaced dash (a dialogue dash at a line start) or before one, technical
fragments, dashes in every form (a spaced `-`, a spaced `--`, an attached `-`, a spaced `–`, an
attached `—`, a spaced `—`, and a dialogue dash at a line start), parentheses, ellipses, sentence
ends, line breaks, stray quotes, and quotes of every glyph, straight, typed and mixed, nested with
the parentheses up to a depth of four. Its configuration (`grammar_config`) varies the rule set
(Prose, Interface, `ALL`), the dash style (the language's own, `Spaced`, `SpacedGlued`, `Closed`),
the `SingleQuotes` mode and the percent setting, in five languages (`grammar_language`): `ru-RU`,
`en-US`, `COMMON` with the words of both, and two languages declared with `LanguageBuilder`, `en-GB`
(primary pair `‘’`, nested `“”`) and `xx` (Russian data, `«»` with `‹›` nested, the closed style,
the ellipsis forms `?…`, `!…`, `?!…`). Each sentence is written twice from the same steps
(`Sentence`): with its typed glyphs, and plain (`write_plain_grammar`), where every step picks only
from the entries without a dash, an ellipsis or a typed quote. The runs are deterministic: fixed
seeds, no clock and no file of failures (`run_grammar`). Properties 1 to 3
(`grammar_properties_hold` in `tests/properties.rs`: idempotence, the second pass without a change,
and I1 to I5 as the public API shows them, on the sentence with its typed glyphs) and P2
(`p2_over_the_grammar` in `tests/roundtrip.rs`, on the plain sentence and on `to_plain` of the typed
one where that holds no typographic character, with P1 on the plain form of the output) run 512
sentences per language in a normal run. The `#[ignore]` tests `grammar_properties_hold_long` and
`p2_over_the_grammar_long` run 100 000 sentences for each configuration of
`grammar_long_configurations`: each of the five languages with each dash style and each
`SingleQuotes` mode, 60 configurations and 6 000 000 sentences, the rule set and the percent setting
drawn per sentence, on all available threads (`run_grammar_long`); every failing configuration is
reported. No failure of idempotence is skipped: since the quote phase settles its result (section
6.6, choice C59) the known limits of section 10.8 hold no window that is no fixed point. Only a
sentence that fails P2 is measured against the known limit of `to_plain` in section 10.8, so every
other sentence is checked in full: a failure of P2 is skipped when `grammar_known_p2_failure` names
the limit, where the output is a fixed point, P2 fails on it, `to_plain` read a `"` of the input as
`'` or the other way round (`p2_misread`), and that reading alone explains the failure: the plain
form with the quotes of the input in their place (`with_quotes_of`) typesets to the output, so
`to_plain` lost nothing but the quotes. `the_p2_filters_name_their_limits` checks that the filter
names the inputs of the limit and that those inputs still fail, and the filter goes with its limit;
`the_p2_filters_name_nothing_else` holds the rows the filter must not name (no misreading, quotes of
another number, an output that is no fixed point, an input where P2 holds, a failure that the
reading of the quotes does not explain), so a filter that names more than its limit fails;
`the_grammar_rows_of_the_settled_windows_are_fixed_points` keeps the sentences that failed
idempotence before the settling as fixed points.

1. **Idempotence**: `f(f(x)) == f(x)` for {ru, en, COMMON} × {Prose, Interface, all, random subsets}
   × the three `SingleQuotes` modes, and over the structured grammar for its five languages, two of
   them declared, × {Prose, Interface, all} × every dash style, with fixed seeds for every split and
   merge of invariant I2 (every case that section 4.3 lists under I2, among them
   `details - a - т.е.sk` and `5 - x - Moscow`; `5'10"`, `90's`, `'90s`, `'tis`, `""`, `"""`,
   `key=""`, `"He said 'hi'"`, `"a 'b c" 5'`, `"/etc/"`, `Токен "SGVsbG8gd29ybGQ=" и "имя".`,
   `‘Note: 'beta' builds of the developers' tools`, `‚Wort‘` with a `LanguageBuilder` language whose
   pairs are `„“` and `‚‘`; the sign-only pair next to a dash, `Слово - "" да`, `"" - 5`, `- "" да`;
   the rule of three, `"--"""` and `''''- a' b'`; and a neutral typed closer inside a new pair of
   its family, `x '"»b 5' c"`, `'"»90'"`, en `’'"”»90'„"`; and neighbors of a merged or a split
   fragment read in their plain form, section 4.3, I4: `Moscow - a, - b`, `№5 ли ты`, `т. н.э.`,
   which the table test `merged_and_split_fragments_read_as_their_plain_form` also runs with
   `DashStyle::Closed` and checks for P2).
2. **Fixed-point oracle**: `f` repeated until nothing changes (at most three times) equals `f`; a
   difference prints the reports of both passes.
3. **Invariants**, one property each: I1 (edits never change a token kind; insertions and removals
   only of spaces), I2 (fragment kinds of `f(x)` match those of `x`, modulo the splits and merges of
   section 4.3), I3 (no output character of a rule triggers a rule), I4 (every match computed on
   `f(x)` has an empty edit, and every match on `x` outside the fragments merged or split by I2
   recurs on `f(x)` with the same rule and `consume`, except that a spaced dash produced by
   `dash.double-hyphen` is matched by `dash.spaced-hyphen`, and by no rule when `dash.spaced-hyphen`
   is off, section 8.4, and that a dash match whose operand a removed space merged into a `Joined`
   fragment does not recur, section 4.3), I5 (in a non-converging window no straight quote changed).
   The full I2 and I4, every fragment with its kind, flags and extent and every match with its rule
   and `consume`, is the unit test `i2_and_i4` in `src/engine/mod.rs`: over `any_text` and over the
   structured grammar in its five languages (both forms of each sentence; an `#[ignore]` long run of
   20 000 sentences per configuration) without the `nbsp.*` rules, and with every rule on the splits
   and merges of I2. It leaves out what section 4.3 names and nothing else
   (`the_exemptions_of_i2_and_i4_name_only_their_cases`): a fragment next to a split or a merge,
   also one that a replacement makes by dropping a space (the percent setting); a match inside such
   a fragment or a short word or a particle whose trigger word is one; a fragment that starts with a
   straight `'` that became an opener; and a dash before a straight `'` that became `’`. Over the
   grammar, `grammar_properties` compares the protected spans of both passes outside the fragments
   next to a split or a merge (`local_kinds`, with the rows of
   `i2_over_the_grammar_leaves_out_only_the_fragments_next_to_a_split`).
4. **Slots and protected spans**: number, order, kinds and payloads of slots are kept;
   `texts().len() == slots().len() + 1`; protected spans and markup are byte for byte in place.
5. **Technical fragments**: fragments classified as technical in the input, and compound words
   (section 8.2), appear in the output byte for byte.
6. **Letters and digits**: the sequence of letters and digits of the output equals that of the input
   (with `symbols.*` and `number.times` off).
7. **Graphemes**: every edit of `report` starts and ends at a cluster boundary of the input, and
   every cluster of the input that no replacement or removal covers appears byte for byte in the
   output. A cluster is an extended grapheme cluster of UAX #29 (a combining sequence, a ZWJ
   sequence, a keycap, a regional-indicator pair and `\r\n` are one cluster each), not a token:
   `(c)` → `©` replaces three clusters.
8. **Cow**: the output is `Borrowed` exactly when it equals the input byte for byte, for every
   output kind (`Html` and `Escaped` included), and a test-only entry point `typeset_unfiltered` (a
   `#[cfg(test)]` function in `src/engine/mod.rs`; this part of the property runs as a unit test
   there, with the generators of this section, not in `tests/properties.rs`; the generators live in
   `tests/common/strategies.rs`, which `tests/common/mod.rs` declares as `pub mod strategies;` and
   `src/engine/mod.rs` includes as
   `#[cfg(test)] #[path = "../../tests/common/strategies.rs"] mod strategies;` (its unit test uses
   only some generators; the file's own `#![allow(dead_code, ...)]` covers that, and a second
   `#[allow(dead_code)]` on the module would be a duplicated attribute for Clippy), and `lib.rs` has
   `#[cfg(test)] extern crate self as typolish;` so that the `typolish::` paths of that file resolve
   in both crates; so it adds nothing to the public API or to the features), which runs the whole
   pipeline without the prefilter, gives the same result as `typeset` and `typeset_template` on
   every generated input and template, for every rule set and every output kind; so a missing
   trigger byte or encoded byte shows up as a difference.
9. **Recorder independence**: `typeset`, `typeset_with` and `report` give the same result; `lint(x)`
   without its `TypographicChar` entries equals `report(x, Output::Unicode).diagnostics()`; applying
   `report(x).changes()` to `x` yields the result.
10. **Pieces and strings**: a template of `Text` pieces only gives the same as `typeset` of their
    concatenation, protection delimiters included (they are matched across pieces, section 3.4),
    with the text cut at every byte boundary of a character, including inside runs of dots, dashes
    and spaces; applying `report_template(...).changes()` piece by piece yields the result. `Text`
    pieces with `BlockMarkup` pieces between them give the results of separate `typeset` and
    `to_plain` calls joined by the markup (texts without a run of three backticks, since a fenced
    block may run across the piece), and seeds of such templates are idempotent.
11. **Determinism of order**: a set built in different orders of `with`/`without` gives byte-equal
    output.
12. **Output kinds**: `decode(typeset(x, out)) == typeset(x, Unicode)` and
    `typeset(decode(typeset(x, out)), out) == typeset(x, out)` for every output kind, nowrap styles
    included (an ordinary space inside an element decodes to U+00A0, section 11.5); decoders are
    test helpers (for `CSS`, every U+0000 of the generated `x` is replaced by U+FFFD before both
    equations, section 11.8).
13. **Round trip**: P1 on generated canonical strings (the canonical form of section 12.1 for the
    settings under test, percentages included, without the `NestedPossessive` pattern) and on the
    corpus as section 12.2 says; P2 on generated plain strings and on the corpus; the table test of
    `roundtrip-known.tsv` (section 12.2). Generated strings run for {ru, en} × {Prose, Interface,
    each rule alone, random subsets}; the corpus runs with Prose and Interface only; P2 also runs on
    the plain forms of the structured grammar in its five languages with Prose, Interface and `ALL`.
14. **No panic** on arbitrary strings and arbitrary pieces, including lone combining marks, control
    characters and noncharacters: 10 000 cases in a normal run, 1 000 000 in an `#[ignore]` test.
15. **lint matches to_plain**: `lint(x)` contains a `TypographicChar` exactly when
    `to_plain(x) != x`, and the entries stand on exactly the characters that `to_plain` changes:
    `to_plain(x)` is `x` with each covered character replaced by a text that is not empty and not
    that character, and every other character kept in place (`entries_explain`, whose rows refuse an
    entry on another copy of the same character, an extra entry and a missing one). The same holds
    per text of `lint_template` and `to_plain_template`, and over the structured grammar
    (`lint_matches_to_plain_over_the_grammar`, with an `#[ignore]` long run).
16. **to_plain is stable**: `to_plain(to_plain(x)) == to_plain(x)` for {ru, en} × {Prose, Interface}
    on generated strings and the corpus, and `to_plain(x)` is `Borrowed` exactly when it equals `x`.
17. **Safe alphabet**: for every output kind, the output of `typeset` and of `typeset_template`
    (each text) contains none of these raw characters: `Html` — `<`, `>`, `"`, `'`, and `&` not
    starting a reference (`Markup` pieces excepted); `JS`, `JS_HEX`, `JS_CODE_POINT` — `"`, `'`,
    backtick, `<`, `>`, `&`, `$` followed by `{` or at the end, U+2028, U+2029, controls; `JSON` —
    `"`, `<`, `>`, `&`, U+2028, U+2029, controls; `RUST` — `"`, controls; `CSS` — `"`, `'`, `<` and
    the controls of section 11.6; in every escaped kind each `\` starts an escape that a strict
    parser of that syntax accepts (a CSS hex escape always ends with its space); with `non_ascii()`
    no byte from 0x80; except with `HtmlStyle::CHARACTERS`, no raw character of the typographic set.
    The strict parsers are test helpers next to the decoders. "Controls" here are those of the table
    of section 11.6, U+0000 to U+001F and U+007F; U+0085 and the other C1 characters are not among
    them and are written raw by every escaped kind unless `non_ascii()` is set.

### 13.6 Unicode conformance

- `tests/graphemes.rs` runs every line of `GraphemeBreakTest.txt` of Unicode 17.0.0 (the version of
  `unicode-segmentation` 1.13.3) through both rule sets, alone and inside sentence frames, and
  checks through `report` that every edit starts and ends at a cluster boundary of the input and
  that every cluster of the input that no replacement or removal covers appears byte for byte in the
  output. The file is kept verbatim with its Unicode license header in `tests/fixtures/unicode/`,
  excluded from the published package (`exclude` in `Cargo.toml`); the test is skipped when the file
  is absent. A conformance test pins `unicode_segmentation::UNICODE_VERSION` to the version of the
  fixture, so a dependency update forces a fixture update. The file is fetched from unicode.org in
  stage 2. Two tables of the same file pin what differs between versions of the dependency (section
  5.6): the clusters of sequences of old characters whose boundaries changed between 1.10.1, 1.12.0
  and 1.13.3 (GB9c, `Extended_Pictographic`, U+11A3A), with the measured clusters of the older
  versions, and the typeset output of the inputs where the change reaches the output, with the
  output measured under 1.12.0.
- Tables for spaces with combining marks (section 5.3), NFD words (section 5.5), mixed scripts
  (section 5.7, with the units `μs` and `µs` raising no `MixedScript`) and format characters
  (section 5.4, with U+00AD inside words: `21 ноя\u{AD}бря` gets its glue, `в Мо\u{AD}скве` is a
  `Word`; `сло\u{200D}во` is a `Word`; `21 ноя\u{200C}бря` gets its glue (`21⍽ноя\u{200C}бря`); ZWNJ
  at the start of a text is a `Format` token; a ZWNJ or ZWJ after a tab, U+0000, U+007F, U+0085 or
  U+00AD (`a\t\u{200D}b`, `a\u{0}\u{200C}b`, `a\u{7F}\u{200D}b`, `a\u{85}\u{200D}b`,
  `a\u{AD}\u{200D}b`) is a `Format` token of its own, and `\n\u{200C}\u{200D}a` gives one `Format`
  token for the two characters; all come out unchanged; `"\u{200C}a` and `1\u{200D}2` are a `Symbol`
  followed by a letter or digit and come out unchanged; U+0020 + U+200D is a frozen `Space`; through
  `report`, no edit falls between a ZWNJ or ZWJ and the character before it).
- Tables for clusters led by a non-letter: keycaps `1️⃣` and `#️⃣`, `"` + U+0301, each alone and right
  after ASCII text (`Шаги 1️⃣-3️⃣`, `a"\u{301}b`), so that the fast path of section 3.5 hands them to
  the segmenter; they come out unchanged and are not numbers, signs or quotes. Clusters led by a
  `Prepend` character before ASCII: `\u{600}123`, `\u{6DD}` + space at the end of the text,
  `a\u{600}b`. They come out unchanged; `\u{600}1` is a `Symbol`, and no edit falls between U+0600
  and the ASCII character after it. A lexer unit test compares the cluster boundaries of the fast
  path with `grapheme_indices(true)` on generated strings that mix ASCII with a character of every
  `Grapheme_Cluster_Break` class and Indic conjunct class that the fixture uses (`Prepend`,
  `Extend`, `ZWJ`, `SpacingMark`, the Hangul classes, regional indicators, pictographs and the
  rest), and another checks that every simple character of section 3.5 breaks from `a`, the Hangul
  jamo, a regional indicator and a line feed on either side. The fixture itself is read only by
  `tests/graphemes.rs`, which runs every line of it: a unit test in `src/` reads no file (section
  13.9 checks that `src/` contains no `std::fs`).

### 13.7 Own corpus

There is no external corpus (decided by the maintainer on 2026-11-21); the corpus is written for the
project, with no third-party texts; `prose.txt` and `ui.tmpl` are in the canonical plain form for
both sets, `tech.txt` for Interface (section 12.2):

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
failing test prints a diff, and files are updated deliberately by hand (no `insta`): the `#[ignore]`
test `update_snapshots` writes them only with the variable `TYPOLISH_UPDATE_SNAPSHOTS=1` (the test
reads it, the crate reads no environment), so `cargo test -- --ignored`, which runs the long tests,
leaves them alone; the diff of `expected/` is read before it is committed. `.gitattributes` marks
`tests/corpus/**` and `tests/fixtures/**` as `-text`, since they contain U+00A0 and must not be
normalized.

### 13.8 Load and allocation

- `benches/load.rs` (`harness = false`, std only: `Instant`, `black_box`, the median of N runs), run
  as `cargo bench --bench load` in release (the file carries the allow list of section 14.5). It
  prints: the whole corpus per set and output kind (MB/s); interface strings of 42 to 83 bytes
  (about 60 on average) and one-word strings without trigger bytes (prefilter), median and p99 over
  100 000 runs, with and without `typeset_with`; time per byte for 1 KiB, 64 KiB, 1 MiB and 16 MiB
  of joined corpus and for one 1 MiB paragraph; and the pathological inputs of section 3.12, with
  the peak memory for one paragraph of 10^5 ZWJ emoji clusters (section 3.12) and whether its result
  is borrowed.
- `#[ignore]` test `load_is_linear`: the median time per byte of five runs for 16 MiB is at most 2.5
  times that for 1 MiB, separately for each pathological input; both sizes are past the caches, so
  the ratio holds no step out of them, and a quadratic input gives 16. A guard first stops an input
  whose time per byte for 1 MiB is more than 8 times that for 64 KiB (median of 15 runs), before the
  large input is built. No absolute thresholds: the machine is shared, and the medians keep the test
  stable under load. Indicative targets, measured in stage 5 and not promised: at least 20 MB/s on
  the corpus in Unicode output, a median below 10 µs for one interface string. Measured on
  2026-11-23 (release, one shared machine, Rust 1.100), first at the end of stage 5: the corpus ran
  at 1.8 to 3.6 MB/s in every output kind (about 280 to 550 ns per byte), the interface strings of
  the bench took 13 to 15 µs at the median for 42 to 44 bytes of en-US and 31 to 33 µs for 71 to 83
  bytes of ru-RU, and one paragraph of 10^5 ZWJ emoji clusters raised the peak memory by about 13
  MB. The time went to linear scans of word lists for every word (check 0 and `nbsp.abbreviations`),
  to Unicode tables for Cyrillic text (the segmenter, `char::is_alphabetic`, `char::to_lowercase`),
  to every rule being called at every token and every phase copying every token, and to the per-call
  setup; sections 3.5, 3.6, 3.8 and 3.12 describe what replaced them, without a change of any
  output. Measured again after that work: the corpus runs at 6.8 to 14.6 MB/s in Unicode output
  (about 70 to 150 ns per byte; ru-RU 10.6 to 14.6, en-US 6.8 to 11.0, the templates of `ui.tmpl`
  the slowest) and at 4.5 to 11.3 MB/s in the other output kinds; time per byte of joined corpus is
  65 to 100 ns from 1 KiB to 16 MiB; the interface strings take 3.2 to 5.6 µs at the median and 6.4
  to 12.4 µs at p99 for en-US, 3.1 to 6.3 µs at the median and 5.9 to 14.3 µs at p99 for ru-RU; a
  one-word string that the prefilter rejects takes 28 to 47 ns (each call is timed on its own, so
  the figure includes two clock reads, about 17 ns on that machine; timed in batches the call takes
  about 12 to 31 ns). One paragraph of 10^5 ZWJ emoji clusters raises the peak memory by about 7 MB
  (about 70 bytes per cluster, 68 to 113 bytes from 10^4 to 10^6 clusters, so linear); the result is
  borrowed, so this is engine memory only (section 3.12). The latency target is met; the throughput
  target is missed by a factor of 1.4 to 3, which the maintainer accepted for version 0.1 (section
  15.2, item 27). What remains is spread over the passes the design prescribes rather than
  concentrated in one place: per fragment the classifier reads the wrapper, the units, the signature
  of check 0, the table and the labels (about 150 to 250 ns per fragment); per space every active
  no-break rule re-reads its context and its lists (the no-break phase is about 35 to 40 ns per
  byte); and the lexer classes every character. The cost is linear: the step between 64 KiB and 1
  MiB, where the token buffers of one window leave the caches, is most of the growth of the time per
  byte, and its size depends on the machine and its load (the faster the passes, the larger that
  memory-bound share). So `load_is_linear` compares 16 MiB with 1 MiB, past that step; on
  2026-11-25, at a load average of about 10, that ratio was at most 1.33 over every pathological
  input and the 1 MiB to 64 KiB ratio at most 1.6. The earlier form, a single run of each size
  compared with 64 KiB against a bound of 2, failed two runs of three in a review at a load average
  of 6 to 8.
- `#[ignore]` test `to_plain_is_linear_on_prose`: time per byte of `to_plain` for 512 KiB of
  ordinary en-US prose with spaced dashes, one line and in paragraphs, is at most twice that for 64
  KiB (section 12.1; `lint` runs the same check).
  `to_plain_is_linear_on_one_long_paragraph_of_quotes` does the same for long runs of quote
  characters in one paragraph: `’` repeated, `«„` repeated around `x` with `“»` repeated, `«`
  repeated around `x` with `»` repeated (Russian Prose), and nested English pairs whose `’` after
  `s` wait for a later closer.
- `tests/alloc.rs`: a separate test binary with a counting `#[global_allocator]` (the only file with
  `#![allow(unsafe_code)]`, section 14.5; the allocator counts and delegates to
  `std::alloc::System`) checks the table of section 2.12: zero allocations for an unchanged string
  after warm-up (both through the prefilter and through the full pipeline), one for a changed
  string, zero for `format_number` written with `write!` into a `String` with room (the `Display`
  path of section 2.12) and for `typeset_into` into a buffer with room, zero in `Typographer::new`;
  ru `Сохранить` in Interface → `Cow::Borrowed`, no lexing; an iterator of template pieces that
  calls `typeset` adds no allocations beyond the table of section 2.12; a paragraph of 16 KiB
  unchanged and of 48 KiB changed costs zero and one allocation after warm-up, and none with
  `typeset_into`; after an input that grew the scratch past 4 MiB the first short call allocates and
  the second does not. The file exists from stage 2 with the rows that hold before the rules:
  `Typographer::new`, an unchanged string after warm-up, templates of 1 to 100 slots (three vectors,
  no reallocation), a text merged from several pieces (one string) and the iterator that calls
  `typeset`; the other rows join in stage 3 (section 15.4).

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
`empty`), `ALL`, `PROSE`, `INTERFACE` and generated sets (deprecated rules included while their
variant exists); the composition of Prose and Interface, and the rule lists in their rustdoc, match
section 9.2 and the Prose and Interface columns of sections 4.1 and 8.1; the rustdoc of every
builder setting names exactly the rules that list it in the settings column of section 8.1; `src/`
contains no `std::env`, `std::fs`, `std::net`, `std::time` or `unsafe`; the `#![allow(...)]`
attributes of every `tests/*.rs` list exactly the lints of section 14.5 (`tests/alloc.rs` also
`unsafe_code`, first), and that of `tests/common/mod.rs` only `dead_code`; `src/lang/ru_ru.rs`,
`src/lang/en_us.rs` and `src/lang/common.rs` contain no `fn`, `impl`, `match` or `if` (section 7.1);
the replacements that rules write in every built-in language and `COMMON` with `RuleSet::ALL` on a
fixed set of texts, with one template for `range.placeholders`, contain no letter and no digit; the
test checks that the set reaches every active rule of the language, and that the writes include the
legal marks, arrows, math signs, times sign, primes and apostrophe of section 11.2 and, in a
language with dash data, the dash (the fields of language data themselves are checked by
`tests/data.rs`); every character the lexer reads as `Quote` (section 3.5) other than `"` and `'` is
in the table of section 11.2, `′` and `″` in the primes row and the others in the quotes row;
built-in languages × built-in sets build without error. A unit test next to the prefilter checks,
for every built-in set and language, that the trigger set equals the union of the trigger constants
of the active rules. Checks that read `docs/design/` skip when the directory is absent (the
published package has no design); a missing file or heading inside the repository fails.

`tests/package.rs` checks the package (stage 5, section 15.4): the manifests and `dev/deny.toml`
against section 14.5, the tree of `Cargo.lock` against section 14.1, the layout of section 14.6, the
license and ignore files, the allow lists of `benches/load.rs` and `examples/*.rs` (section 14.5;
those of `tests/*.rs` are checked by `tests/conformance.rs`), the record of the name check in
`history.md`, the list and size of `cargo package --list`, and the build from its own tarball; it
runs the three examples. The checks of the repository skip outside it: the crate must be the
repository's `dev/typolish/` and `docs/design/README.md` must exist. The tarball check then runs
`cargo test` for `tests/documentation.rs` and `tests/package.rs` in the package that `cargo package`
unpacked and verified (under `<target-dir>/tmp/package-check/target/package/`, where Cargo does not
look for the workspace `dev/`, so the check passes with any target directory), so the published
tests are known to pass without the repository. The two targets stay in the package: excluding an
auto-discovered target makes `cargo package` warn, and the tarball check rejects any warning.

`tests/data.rs` runs the validation of section 7.6 on every built-in table (so the order and
duplicates of `months` are checked too), checks that the validation rejects a `minus`, `group` or
`decimal` that is a letter or a digit, and compares the CLDR values (quote pairs, decimal, group,
minimum grouping digits, percent pattern, range pattern, `nan`, `infinity`, month names (the union
of format wide and abbreviated forms, without duplicates), units marked "CLDR 49") with an extract
of cldr-json 49.0.0 kept in the test (for en-US the extract holds the values of `en`, section 7.3),
so a CLDR update is a visible change. It compares `LanguageData::COMMON` with the table of section
7.4 and checks `LanguageData::new("xx")` field by field. It also checks that a table whose
`QuotePairs` holds `「」` is rejected with `Error::InvalidLanguageData`; that `LanguageBuilder` with
the pairs `(«, »)` and `(«, „)` gives `Error::InvalidLanguageData` and the typographer builder with
the same pairs `Error::InvalidSetting`; that `Language::from_data` with the tag `EN-gb` gives
`Error::InvalidLanguageData`; and that `LanguageBuilder::new("EN-gb").build()` gives a language with
`tag() == "en-GB"`.

### 13.10 Documentation

Every public item has rustdoc with an example (`missing_docs = "deny"`); examples are doctests and
run with `cargo test`. "Public item" here is every public type, function, method and constant,
associated constants included, and every variant of a public enum: the example of the type does not
stand for its variants (decided by the maintainer, section 15.2, item 29). Struct fields have
rustdoc (`missing_docs` checks it) and are shown by the example of their type. The rustdoc lint
`missing_doc_code_examples` is unstable and skips items re-exported from private modules, which is
every item of this crate. Therefore `tests/documentation.rs` checks inside the gates, from the
source text, that every `pub` item of `src/`, every variant of a public enum included, has an
example of its own (section 15.2, item 29) and that every example of `src/` is run (no `ignore`,
`no_run` or other language); the stage 5 report cross-checks the count with the rustdoc JSON of a
nightly toolchain, outside the gates. README and CHANGELOG are in English, and every text of the
repository (README, CHANGELOG, the design and every Rust file of the crate) is in American English.
The README states the known limits of section 10.8 and how to protect commands; its code blocks run
as doctests through a `#[cfg(doctest)]` item of `lib.rs` that includes it. `tests/documentation.rs`
also checks that every README block is a `rust` block, that the numbers the README states (rules,
version, MSRV) are those of the code, that its links point at files of the repository, that its
known limits are those of section 10.8 and are what the crate does (`known_limits_hold`: each row
with its exact output and a second pass, in Prose and, where the row says so, Interface, in its
`SingleQuotes` modes, also in en-GB, the declared language of section 13.5 whose primary pair is
`‘’`, for the windows of sections 6.6 and 10.8 that exist only there), that the CHANGELOG has the
undated entry of the version of `Cargo.toml` in development first, the dated entry of the tagged
`0.1.2` next and the `0.1.1` entry naming every rule, and that every comment of the crate's Rust
files that refers to this design says "design section N.M", never a bare "section N.M" (section
15.2, item 33). It reads the README through the crate's `README.md`, which the package holds; the
checks that need the CHANGELOG, the design or the targets of links skip outside the repository, as
in `tests/package.rs` (section 13.9).
