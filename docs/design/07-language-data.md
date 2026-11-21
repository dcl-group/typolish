# typolish design: Language data

Part of the [typolish design](./README.md). Section numbers are those of the whole design.

## 7. Language data

### 7.1 Format

A language is one value of `LanguageData`: a table of fixed structure with no logic. Built-in tables
live in `src/lang/ru_ru.rs`, `src/lang/en_us.rs` and `src/lang/common.rs` as `const` struct
literals, and are public as `LanguageData::RU_RU`, `LanguageData::EN_US` and `LanguageData::COMMON`.
A conformance test checks that these files contain no `fn`, `impl`, `match` or `if`: rules cannot
leak into data.

```rust
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct LanguageData {                       // fields private; const fn setters below; no drop glue
    tag: &'static str,                          // "ru-RU"; "und" for COMMON
    cldr_version: &'static str,                 // "49": where quotes, numbers, months were checked
    script: Script,                             // Cyrillic | Latin | Greek | Other: filter for word lists
    quotes: Option<QuotePairs>,                 // None: quotes.pairs inert
    apostrophe: char,                           // U+2019
    elisions: &'static [&'static str],          // sorted, lowercase: "'cause", "'em", "'tis"
    decade_apostrophe: bool,                    // '90s; '26 only when unpaired (section 6.6)
    trailing_possessive: bool,                  // dogs'
    dash: Option<DashData>,                     // None: dash.* and nbsp.dash inert
    ellipsis: EllipsisForms,                    // forms after ? and !
    short_words: &'static [&'static str],       // sorted, lowercase
    short_word_length: u8,                      // 0: nbsp.short-length inert
    particles: &'static [&'static str],         // sorted, lowercase
    abbreviations: &'static [Abbreviation],     // sorted by parts, as printed
    atom_exceptions: &'static [&'static str],   // dotted words that are not technical: "e.g."
    units: &'static [&'static str],             // sorted by code point, case-sensitive
    ambiguous_units: &'static [&'static str],   // units that are also words: glued only before punctuation or the end
    months: &'static [&'static str],            // sorted by code point, as printed: wide and abbreviated forms used next to a day number, one list (lookups: section 7.1)
    month_position: MonthPosition,              // AfterDay | BeforeDay | Both
    year_words: &'static [&'static str],        // after a year; entries ending in '.' also serve nbsp.dates
    century_words: &'static [&'static str],     // after a century
    range_lead_words: &'static [&'static str],  // before a range: "стр.", "pp."
    signs: &'static [char],                     // '№', '§', '¶'
    numbers: NumberData,
}

// The parts below: #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)], private fields (section 2.13).
pub struct QuotePairs { primary: (char, char), nested: (char, char) }   // QuotePairs::new is const
pub struct DashData { style: DashStyle, dialogue: bool }               // DashStyle: Closed | Spaced | SpacedGlued
pub struct EllipsisForms { question: &'static str, exclamation: &'static str, both: &'static str }
pub struct Abbreviation { parts: &'static [&'static str], bind: Bind } // Bind: Inner | Next | NextCapitalized
pub struct NumberData {
    decimal: char,
    group: char,
    min_grouping_digits: u8,   // 1..=4: CLDR meaning (group from 3 + n digits); 0 = never group (not a CLDR value)
    minus: char,
    percent_spaced: bool,      // CLDR percentFormat: "#,##0 %" or "#,##0%"
    range_dash: char,          // CLDR miscPatterns range "{0}–{1}"
    range_spaced: bool,
    nan: &'static str,
    infinity: &'static str,
}

impl LanguageData {
    pub const RU_RU: LanguageData;
    pub const EN_US: LanguageData;
    pub const COMMON: LanguageData;
    /// An empty table with the given tag: the values of COMMON (section 7.4) with every list
    /// empty, `units` and `signs` included, and `cldr_version` "".
    pub const fn new(tag: &'static str) -> LanguageData;
    pub const fn tag(&self) -> &'static str;
    /// The tag must be canonical (section 7.6).
    pub const fn with_tag(self, tag: &'static str) -> LanguageData;
    pub const fn with_quotes(self, quotes: Option<QuotePairs>) -> LanguageData;
    pub const fn with_dash_style(self, style: DashStyle) -> LanguageData;
    pub const fn with_short_words(self, words: &'static [&'static str]) -> LanguageData;
    pub const fn with_units(self, units: &'static [&'static str]) -> LanguageData;
    // ... one with_* per field
}
```

Why `const` tables in Rust and not TOML or JSON:

- The compiler checks types; there is no parsing at run time and no parse error. Tables sit in
  statics, which makes `const fn Typographer::new` possible (section 2.4).
- TOML or JSON compiled in `build.rs` would need build dependencies (`serde`, a format crate) and a
  build script; reading such files at run time means allocation and error paths on the hot path.
- "Data, not code" is ensured by the structure, not the file format: a language file is one struct
  literal, and a downstream crate declares a language as a `const` built with `const fn` setters
  (section 2.2). Fields stay private so that the format can grow without a breaking release: a new
  field gets a setter, and existing tables keep the value of the table they started from.

`LanguageData` holds only `&'static` references and plain values, so it has no drop glue and its
setters can be `const fn`; it is `Clone` but deliberately not `Copy`, which keeps its representation
free to change.

Languages built at run time (for example from a caller's configuration file) use `LanguageBuilder`,
which owns its lists:

```rust
#[must_use]
pub struct LanguageBuilder { /* owned copies of every field */ }
impl LanguageBuilder {
    /// Starts from an existing language's data under a new tag.
    pub fn from_language(base: &Language, tag: &str) -> LanguageBuilder;
    /// Starts from the values of `LanguageData::new` under a tag.
    pub fn new(tag: &str) -> LanguageBuilder;
    pub fn quotes(self, quotes: Option<QuotePairs>) -> Self;
    pub fn dash_style(self, style: DashStyle) -> Self;
    pub fn short_words<I>(self, words: I) -> Self where I: IntoIterator, I::Item: AsRef<str>;
    pub fn units<I>(self, units: I) -> Self where I: IntoIterator, I::Item: AsRef<str>;
    // ... one method per field
    /// Normalizes (lowercases the word lists of section 7.6, not `months`, `abbreviations` or
    /// units; sorts; removes duplicates), validates (section 7.6) and returns a `Language` that
    /// holds the data in an `Arc`.
    pub fn build(self) -> Result<Language, Error>;
}
```

Inside the engine both kinds of language are read through one private view, so rules do not know
where the data came from.

Lookups: lists are sorted (by the author for `const` tables, validated; by `LanguageBuilder` for
run-time languages) and searched by binary search without allocation. Word lists (section 7.6) are
compared without case (per-character simple lowercase mapping of std). `units` and `ambiguous_units`
are compared with case (`В` is a volt, `в.` a century; `MB` is not `mb`). `abbreviations` (each
part) and `months` are compared with case, except that an entry that starts with a lowercase letter
also matches the same text with that letter in uppercase, unless the entry is a single letter with a
full stop (`д.`, `п.`, `ч.`, `г.`, en `p.`), which matches only as written: `см.` matches `См.`,
`ноября` matches `Ноября`, while `д.` does not match `Д.` (a one-letter label at a sentence end,
section 8.7), `No.` does not match `no.` and `May` does not match `may`. The lookup tries the text
as written and then, if it starts with an uppercase letter and is not a single letter with a full
stop, once more with that letter lowercased. A part of `abbreviations` is found by scanning the
list, which is short. The typographer builder keeps caller additions and removals in small sorted
vectors next to the language's lists.

### 7.2 ru-RU

"CLDR 49" values were read from the cldr-json 49.0.0 files on 2026-11-21 (section 14.8). "Project"
values were compiled for this crate; "secondary" marks values taken from secondary sources (the
Russian Wikipedia articles on the dash and on the no-break space); Art. Lebedev Studio's Kovodstvo
§62 and §97 were read directly. Mil'chin's handbook and the GOST standards were not read (section
15.5, item 16).

The lists in the tables of sections 7.2 to 7.4 are written for reading (units split by source,
months in calendar order). In `src/lang/*.rs` each list is stored merged from all rows of its field
and in the order section 7.6 requires; the data test checks the stored order.

| Field                                      | Value                                                                                                                                                                                     | Source                                                                                                                        |
| ------------------------------------------ | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------- |
| `tag`, `script`                            | `ru-RU`, Cyrillic; bare `ru` resolves here                                                                                                                                                |                                                                                                                               |
| `quotes`                                   | primary `«` U+00AB `»` U+00BB; nested `„` U+201E `“` U+201C                                                                                                                               | CLDR 49 `delimiters` (agrees with the task)                                                                                   |
| `apostrophe`                               | `’` U+2019                                                                                                                                                                                | Unicode Standard ch. 6                                                                                                        |
| `elisions`                                 | none                                                                                                                                                                                      |                                                                                                                               |
| `decade_apostrophe`, `trailing_possessive` | false, false                                                                                                                                                                              |                                                                                                                               |
| `dash`                                     | `SpacedGlued` (`слово⍽— слово`), `dialogue: true` (`—⍽Реплика`)                                                                                                                           | secondary; Kovodstvo §97 for the dash itself                                                                                  |
| `ellipsis`                                 | `?..`, `!..`, `?!..`                                                                                                                                                                      | secondary                                                                                                                     |
| `short_words`                              | а без близ в вне во да для до если за и из изо или к как ко на над не ни но о об обо от ото по под подо при про с со то у (37)                                                            | secondary, project selection (settled, 15.5 item 16)                                                                          |
| `short_word_length`                        | 2 (Cyrillic words of one or two letters)                                                                                                                                                  | project                                                                                                                       |
| `particles`                                | б бы ж же ли ль                                                                                                                                                                           | the task, secondary                                                                                                           |
| `abbreviations` Inner                      | в т. ч.; до н. э.; и др.; и пр.; и т. д.; и т. п.; н. э.; т. д.; т. е.; т. к.; т. н.; т. п.; т. ч.                                                                                        | the task, secondary                                                                                                           |
| `abbreviations` Next                       | гл. г-жа г-н д. им. илл. кв. корп. обл. п. пер. пос. пп. просп. разд. рис. см. ср. ст. стр. табл. тов. ул. ч.                                                                             | project                                                                                                                       |
| `abbreviations` NextCapitalized            | г. (a city: `г.⍽Москва`; never after a number, a range, a year and month, a `D.M.YYYY` date or a `Number`/`Date` slot, where it is a year)                                                | project                                                                                                                       |
| `atom_exceptions`                          | none (dotted Russian forms are abbreviations)                                                                                                                                             |                                                                                                                               |
| `units` (CLDR 49 short)                    | Б кБ МБ ГБ ТБ Мбит нс мкс мс с мин ч дн. нед. мес. мг г кг т нм мкм мм см м км л мл га м² Вт кВт МВт кВт⋅ч В А мА Гц кГц МГц ГГц гПа °C °F K км/ч м/с дюйм. фт ярд. фнт унц. пкс тчк/дюйм | CLDR 49 units, short (checked; CLDR separates with U+0020)                                                                    |
| `units` (project)                          | КБ КиБ МиБ ГиБ ТиБ бит Кбит Гбит Кбит/с Мбит/с Гбит/с КБ/с МБ/с ГБ/с сек. мин. сут. дм ц ° °С ₽ руб. коп. $ € тыс. млн млрд трлн шт. экз. чел. об/мин px dpi м2 м3 км2 см2 см3 мм2        | project                                                                                                                       |
| `ambiguous_units`                          | с (`Ждать 30⍽с.`, but `30 с друзьями` stays)                                                                                                                                              | project                                                                                                                       |
| `months`                                   | января февраля марта апреля мая июня июля августа сентября октября ноября декабря; янв. февр. мар. апр. июн. июл. авг. сент. окт. нояб. дек.                                              | CLDR 49 gregorian, format wide and abbreviated (checked) (`мая` is both the wide and the abbreviated form and is listed once) |
| `month_position`                           | AfterDay (`21⍽ноября`)                                                                                                                                                                    |                                                                                                                               |
| `year_words`                               | г. гг. год года году годах годов годы лет                                                                                                                                                 | the task (`2026 г.`), project                                                                                                 |
| `century_words`                            | в. вв. век века веке веков веках                                                                                                                                                          | project                                                                                                                       |
| `range_lead_words`                         | стр. с.                                                                                                                                                                                   | project                                                                                                                       |
| `signs`                                    | № § ¶                                                                                                                                                                                     | the task, Kovodstvo §62                                                                                                       |
| `numbers.decimal`                          | `,` U+002C                                                                                                                                                                                | CLDR 49                                                                                                                       |
| `numbers.group`                            | U+00A0                                                                                                                                                                                    | CLDR 49 (U+202F is a setting; settled, 15.5 item 5)                                                                           |
| `numbers.min_grouping_digits`              | 1 (group from 4 digits: `999`, `1⍽000`)                                                                                                                                                   | CLDR 49 (settled, 15.5 item 4)                                                                                                |
| `numbers.minus`                            | `−` U+2212                                                                                                                                                                                | the task; CLDR 49 gives U+002D                                                                                                |
| `numbers.percent_spaced`                   | true (`5⍽%`)                                                                                                                                                                              | CLDR 49 percentFormat `#,##0 %` with U+00A0                                                                                   |
| `numbers.range_dash`, `range_spaced`       | `–` U+2013, no spaces                                                                                                                                                                     | CLDR 49 miscPatterns `{0}–{1}`; Kovodstvo §97                                                                                 |
| `numbers.nan`, `infinity`                  | `не⍽число` (with U+00A0), `∞`                                                                                                                                                             | CLDR 49                                                                                                                       |

`°С` in the project units has a Cyrillic `С` (U+0421), as typed on a Russian keyboard; the CLDR unit
`°C` has a Latin `C`. Both are glued. The project units `м2`, `м3`, `км2`, `см2`, `см3`, `мм2` are
the forms typed without a superscript; they are glued like `м²` and their digit is never changed
(section 8.12, `NoDigitChange`).

`г.` and `в.` are duration units in CLDR; here they are year and century words, so `nbsp.dates`
handles them (section 8.7) and they are not in `units`.

### 7.3 en-US

CLDR 49 has no separate `en-US` file; `en-US` inherits `en`, and the values below are those of `en`
(checked: no file, `availableLocales` lists only `en`, `en-GB`, `ru` among these, and
`parentLocales` has no entry for `en-US`; section 14.8).

| Field                                      | Value                                                                                                                                                                                  | Source                                                                 |
| ------------------------------------------ | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ---------------------------------------------------------------------- |
| `tag`, `script`                            | `en-US`, Latin; bare `en` resolves here                                                                                                                                                |                                                                        |
| `quotes`                                   | primary `“` U+201C `”` U+201D; nested `‘` U+2018 `’` U+2019                                                                                                                            | CLDR 49 `en` `delimiters`                                              |
| `apostrophe`                               | `’` U+2019                                                                                                                                                                             | Unicode Standard ch. 6                                                 |
| `elisions`                                 | 'bout 'cause 'em 'nother 'round 'til 'tis 'twas (not `'n'`, which also quotes a letter: section 6.2)                                                                                   | project                                                                |
| `decade_apostrophe`, `trailing_possessive` | true, true                                                                                                                                                                             | project                                                                |
| `dash`                                     | `Closed` (`word—word`), `dialogue: false`                                                                                                                                              | secondary (CMOS through Wikipedia); settled, 15.5 item 6               |
| `ellipsis`                                 | `?…`, `!…`, `?!…`                                                                                                                                                                      | CLDR 49 ellipsis character U+2026                                      |
| `short_words`                              | a an and as at but by for if in nor not of off on or per so the to up via yet (23)                                                                                                     | project (settled, 15.5 item 16)                                        |
| `short_word_length`                        | 0 (`nbsp.short-length` inert)                                                                                                                                                          | project                                                                |
| `particles`                                | none                                                                                                                                                                                   |                                                                        |
| `abbreviations` Next                       | Dr. Fig. Figs. Mr. Mrs. Ms. Mt. No. Nos. Prof. St. ch. p. pp. sec. vol.                                                                                                                | project                                                                |
| `atom_exceptions`                          | a.m. cf. e.g. etc. i.e. p.m. vs.                                                                                                                                                       | project                                                                |
| `units` (CLDR 49)                          | short: kB MB GB TB ns μs ms sec min hr mg g kg t nm μm mm cm m km in ft yd mi lb oz mL L ha m² W kW MW kWh V A mA Hz kHz MHz GHz hPa °C °F K km/h m/s px dpi; narrow: B s h            | CLDR 49 units, short and narrow (checked)                              |
| `units` (project)                          | KB KiB MiB GiB TiB bit kbit Mbit Gbit kbps Mbps Gbps MB/s GB/s µs µm (U+00B5 forms) mph rpm ppi pt °                                                                                   | project                                                                |
| `ambiguous_units`                          | in (`27⍽in.`, but `5 in the box` stays)                                                                                                                                                | project                                                                |
| `months`                                   | January February March April May June July August September October November December; Jan Feb Mar Apr Jun Jul Aug Sep Oct Nov Dec; Jan. Feb. Mar. Apr. Aug. Sep. Sept. Oct. Nov. Dec. | CLDR 49 gregorian wide and abbreviated (checked); dotted forms project |
| `month_position`                           | Both (`November⍽21`, `21⍽November`)                                                                                                                                                    |                                                                        |
| `year_words`, `century_words`              | none                                                                                                                                                                                   |                                                                        |
| `range_lead_words`                         | p. pp.                                                                                                                                                                                 | project                                                                |
| `signs`                                    | § ¶ №                                                                                                                                                                                  | project                                                                |
| `numbers`                                  | decimal `.`, group `,`, min_grouping_digits 1 (from 4 digits), minus `−` U+2212, percent_spaced false (`5%`), range `–` U+2013 without spaces, nan `NaN`, infinity `∞`                 | CLDR 49; minus by the task (CLDR 49 gives U+002D)                      |

CLDR 49 writes `{0}°C` and `{0}%` without a space in English. With the default
`PercentSpacing::AsWritten` the crate neither inserts nor removes a space there: an existing space
is glued (`20 °C` → `20⍽°C`), an attached sign stays attached. The other `PercentSpacing` values
change this for `%` and `‰` only (section 8.7); `°C` is never affected.

### 7.4 COMMON

`Language::COMMON` (tag `und`) is for an unknown language that the caller chose to treat with common
rules only. Its values:

| Field                                                                          | Value                                                                                                                                               | Source                              |
| ------------------------------------------------------------------------------ | --------------------------------------------------------------------------------------------------------------------------------------------------- | ----------------------------------- |
| `tag`, `cldr_version`, `script`                                                | `und`, `49`, Other                                                                                                                                  |                                     |
| `quotes`                                                                       | None                                                                                                                                                |                                     |
| `apostrophe`                                                                   | `’` U+2019                                                                                                                                          |                                     |
| `elisions`, `short_words`, `particles`, `abbreviations`, `atom_exceptions`     | empty                                                                                                                                               |                                     |
| `months`, `year_words`, `century_words`, `range_lead_words`, `ambiguous_units` | empty                                                                                                                                               |                                     |
| `decade_apostrophe`, `trailing_possessive`                                     | false, false                                                                                                                                        |                                     |
| `dash`                                                                         | None                                                                                                                                                |                                     |
| `ellipsis`                                                                     | `?…`, `!…`, `?!…`                                                                                                                                   |                                     |
| `short_word_length`                                                            | 0                                                                                                                                                   |                                     |
| `month_position`                                                               | AfterDay (not read while `months` is empty)                                                                                                         |                                     |
| `units`                                                                        | B kB KB MB GB TB KiB MiB GiB TiB bit ns ms s min h mg g kg mm cm m km L mL W kW V A Hz kHz MHz GHz °C °F K px (language-neutral SI and IEC symbols) |                                     |
| `signs`                                                                        | № § ¶                                                                                                                                               |                                     |
| `numbers.decimal`                                                              | `.`                                                                                                                                                 |                                     |
| `numbers.group`                                                                | U+00A0 (not used while grouping is off; a space is the SI-neutral separator)                                                                        |                                     |
| `numbers.min_grouping_digits`                                                  | 0 (never group, section 2.7)                                                                                                                        |                                     |
| `numbers.minus`                                                                | `−` U+2212                                                                                                                                          |                                     |
| `numbers.percent_spaced`                                                       | false                                                                                                                                               | CLDR 49 root percentFormat `#,##0%` |
| `numbers.range_dash`, `range_spaced`                                           | `–` U+2013, no spaces                                                                                                                               |                                     |
| `numbers.nan`, `infinity`                                                      | `NaN`, `∞`                                                                                                                                          |                                     |

Active with COMMON: `symbols.*`, `ellipsis.*`, `range.placeholders`, `range.digits`, `range.time`,
`number.*`, `quotes.apostrophe` (between letters only), `quotes.primes`, `nbsp.units`, `nbsp.signs`,
`nbsp.phone`. Inert: `quotes.pairs`, `dash.*`, `range.context`, `nbsp.abbreviations`,
`nbsp.initials`, `nbsp.dates`, `nbsp.short-words`, `nbsp.short-length`, `nbsp.particles`,
`nbsp.dash`. `Typographer::rules()` minus `active_rules()` lists the inert ones.

### 7.5 Deviations from CLDR 49

| Item                                  | CLDR 49                       | typolish                                                    | Why                                                                            |
| ------------------------------------- | ----------------------------- | ----------------------------------------------------------- | ------------------------------------------------------------------------------ |
| minus sign, ru and en                 | U+002D                        | U+2212                                                      | the task; minus and hyphen are different signs (Kovodstvo §97)                 |
| space between a number and a unit, ru | U+0020 in `unitPattern`       | glued (U+00A0)                                              | the task (`5 МБ`, `30 с`, `12 дн.`); CLDR describes display, not line breaking |
| percent spacing                       | ru `5⍽%`, en `5%`             | as written by default; CLDR with `PercentSpacing::Language` | the task says "by setting"; settled, 15.5 item 11                              |
| ellipsis after `?` and `!`, ru        | no pattern                    | `?..`, `!..`                                                | Russian norm from a secondary source                                           |
| dotted month abbreviations, en        | `Jan`, `Feb` without a period | both forms                                                  | running text writes `Jan. 5`                                                   |

`en-GB` in CLDR 49 has the same quotes and number symbols as `en` (checked during scouting); its
differences are house style (dash spacing, punctuation with quotes), not CLDR data.

### 7.6 Validation

`Language::from_data` and `LanguageBuilder::build` check a table and return
`Error::InvalidLanguageData` naming the field; `LanguageBuilder` normalizes lists first, so its
callers need not sort them. The built-in tables pass the same check in a test, so an error in them
cannot reach a user.

- The tag is a subset of BCP 47 in canonical case: `ll` (two lowercase ASCII letters), `ll-RR`
  (region in uppercase), `ll-DDD` (three digits, a UN M.49 region such as `es-419`) or `und`.
  `Language::from_data` rejects other case with `Error::InvalidLanguageData`;
  `LanguageBuilder::build` normalizes the case first.
- The four glyphs of `QuotePairs` are pairwise distinct; each is a character the lexer reads as
  `Quote` (section 3.5) other than `"`, `'`, `′` and `″` (so output never becomes input again, and
  the wrapper of section 3.6 strips it). The `apostrophe` is such a character too (so not U+02BC,
  which is a letter); it may equal one of the glyphs (`’` in English).
- List entries are non-empty and contain no whitespace. Every list is strictly sorted by code point
  and free of duplicates; `abbreviations` are sorted by their parts (first part, then the next), and
  no two have the same parts. The word lists (`elisions`, `short_words`, `particles`,
  `atom_exceptions`, `year_words`, `century_words`, `range_lead_words`) are lowercase; `months`,
  `abbreviations`, `units` and `ambiguous_units` keep the case they are printed in (lookups: section
  7.1). Units do not start with a digit; ambiguous units are also in `units`.
- A unit equal to a short word or a particle (compared with case: `с` must be listed, `В` need not)
  must be listed in `ambiguous_units`.
- `decimal != group`; `min_grouping_digits <= 4`; `minus`, `group` and signs are not letters or
  digits; `group` is U+00A0, U+202F or not whitespace; `minus` is not whitespace, `"` or `'`;
  ellipsis forms consist of `?`, `!`, `.` and `…` only.

### 7.7 Adding a language

1. A file `src/lang/<tag>.rs` with a `LanguageData` literal. CLDR values (quotes, number symbols,
   months, units) come from cldr-json of a stated version, recorded in `cldr_version` and in a
   comment at each field; every deviation from CLDR carries a comment with its reason.
2. A line in the registry `src/lang/mod.rs` and a constant `Language::<TAG>`.
3. Test tables for the language (section 13.2), a line in the data test, a corpus file.
4. Behavior that the format cannot express becomes a new field first (default: off) and at most one
   generic rule that reads it. No rule ever checks a language tag. A new rule is inert for languages
   without the field set.
5. The language ships in a release that raises the breaking number: its tag stops returning
   `Error::UnknownLanguage` (section 15.1).

Example: British English needs no code. In a downstream crate:

```rust
use typolish::{DashStyle, Language, LanguageData, RuleSet, Typographer};

/// British English: CLDR 49 en-GB quotes and numbers equal en; spaced dashes by house style.
const EN_GB: LanguageData = LanguageData::EN_US.with_tag("en-GB").with_dash_style(DashStyle::Spaced);
static EN_GB_DATA: LanguageData = EN_GB;

let t = Typographer::builder(Language::from_data(&EN_GB_DATA)?, RuleSet::PROSE).build()?;
# Ok::<(), typolish::Error>(())
```

For a true `static` typographer, `Language::from_static(&EN_GB_DATA)` skips validation, and a test
of the downstream crate checks `Language::from_data(&EN_GB_DATA).is_ok()` once (section 2.2).

The same language can be built at run time with
`LanguageBuilder::from_language(&Language::EN_US, "en-GB").dash_style(DashStyle::Spaced).build()`.
Inside the crate the table becomes `src/lang/en_gb.rs` once the maintainer settles its style (single
outer quotes, en dash or em dash); that is outside version 1.
