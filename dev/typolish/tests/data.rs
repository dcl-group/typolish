//! Built-in language tables against design sections 7.2 to 7.4 (design section 13.9, `tests/data.rs`).
//!
//! `LanguageData` has no accessors except `tag`, so the stored values are read from its derived
//! `Debug` output (the field names are those of section 7.1). The expected lists below were
//! generated from the tables of the design by a script kept outside the repository. The check of
//! the CLDR values against an extract of cldr-json (`tests/common/cldr49.rs`) and the validation
//! rows of section 7.6 joined in stage 3 (section 15.4); they stand at the end of the file.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    clippy::print_stdout,
    clippy::print_stderr,
    clippy::dbg_macro
)]

use std::collections::BTreeMap;

#[path = "common/cldr49.rs"]
mod cldr49;

use typolish::{
    Abbreviation, Bind, DashData, DashStyle, EllipsisForms, Error, Language, LanguageBuilder,
    LanguageData, MonthPosition, NumberData, Output, QuotePairs, RuleSet, Script, Typographer,
};

const RU_UNITS: &[&str] = &[
    "$",
    "K",
    "dpi",
    "px",
    "°",
    "°C",
    "°F",
    "°С",
    "А",
    "Б",
    "В",
    "Вт",
    "ГБ",
    "ГБ/с",
    "ГГц",
    "Гбит",
    "Гбит/с",
    "ГиБ",
    "Гц",
    "КБ",
    "КБ/с",
    "Кбит",
    "Кбит/с",
    "КиБ",
    "МБ",
    "МБ/с",
    "МВт",
    "МГц",
    "Мбит",
    "Мбит/с",
    "МиБ",
    "ТБ",
    "ТиБ",
    "бит",
    "г",
    "гПа",
    "га",
    "дм",
    "дн.",
    "дюйм.",
    "кБ",
    "кВт",
    "кВт⋅ч",
    "кГц",
    "кг",
    "км",
    "км/ч",
    "км2",
    "коп.",
    "л",
    "м",
    "м/с",
    "м2",
    "м3",
    "м²",
    "мА",
    "мг",
    "мес.",
    "мин",
    "мин.",
    "мкм",
    "мкс",
    "мл",
    "млн",
    "млрд",
    "мм",
    "мм2",
    "мс",
    "нед.",
    "нм",
    "нс",
    "об/мин",
    "пкс",
    "руб.",
    "с",
    "сек.",
    "см",
    "см2",
    "см3",
    "сут.",
    "т",
    "трлн",
    "тчк/дюйм",
    "тыс.",
    "унц.",
    "фнт",
    "фт",
    "ц",
    "ч",
    "чел.",
    "шт.",
    "экз.",
    "ярд.",
    "€",
    "₽",
];
const RU_MONTHS: &[&str] = &[
    "авг.",
    "августа",
    "апр.",
    "апреля",
    "дек.",
    "декабря",
    "июл.",
    "июля",
    "июн.",
    "июня",
    "мар.",
    "марта",
    "мая",
    "нояб.",
    "ноября",
    "окт.",
    "октября",
    "сент.",
    "сентября",
    "февр.",
    "февраля",
    "янв.",
    "января",
];
const RU_SHORT_WORDS: &[&str] = &[
    "а", "без", "близ", "в", "вне", "во", "да", "для", "до", "если", "за", "и", "из", "изо", "или",
    "к", "как", "ко", "на", "над", "не", "ни", "но", "о", "об", "обо", "от", "ото", "по", "под",
    "подо", "при", "про", "с", "со", "то", "у",
];
const RU_PARTICLES: &[&str] = &["б", "бы", "ж", "же", "ли", "ль"];
const RU_YEAR_WORDS: &[&str] = &[
    "г.",
    "гг.",
    "год",
    "года",
    "годах",
    "годов",
    "году",
    "годы",
    "лет",
];
const RU_CENTURY_WORDS: &[&str] = &["в.", "вв.", "век", "века", "веках", "веке", "веков"];
const RU_RANGE_LEAD_WORDS: &[&str] = &["с.", "стр."];
const RU_AMBIGUOUS_UNITS: &[&str] = &["с"];
const RU_ABBR_INNER: &[&str] = &[
    "в т. ч.",
    "до н. э.",
    "и др.",
    "и пр.",
    "и т. д.",
    "и т. п.",
    "н. э.",
    "т. д.",
    "т. е.",
    "т. к.",
    "т. н.",
    "т. п.",
    "т. ч.",
];
const RU_ABBR_NEXT: &[&str] = &[
    "г-жа",
    "г-н",
    "гл.",
    "д.",
    "илл.",
    "им.",
    "кв.",
    "корп.",
    "обл.",
    "п.",
    "пер.",
    "пос.",
    "пп.",
    "просп.",
    "разд.",
    "рис.",
    "см.",
    "ср.",
    "ст.",
    "стр.",
    "табл.",
    "тов.",
    "ул.",
    "ч.",
];
const RU_ABBR_NEXT_CAPITALIZED: &[&str] = &["г."];
const EN_ELISIONS: &[&str] = &[
    "'bout", "'cause", "'em", "'nother", "'round", "'til", "'tis", "'twas",
];
const EN_SHORT_WORDS: &[&str] = &[
    "a", "an", "and", "as", "at", "but", "by", "for", "if", "in", "nor", "not", "of", "off", "on",
    "or", "per", "so", "the", "to", "up", "via", "yet",
];
const EN_ATOM_EXCEPTIONS: &[&str] = &["a.m.", "cf.", "e.g.", "etc.", "i.e.", "p.m.", "vs."];
const EN_UNITS: &[&str] = &[
    "A", "B", "GB", "GB/s", "GHz", "Gbit", "Gbps", "GiB", "Hz", "K", "KB", "KiB", "L", "MB",
    "MB/s", "MHz", "MW", "Mbit", "Mbps", "MiB", "TB", "TiB", "V", "W", "bit", "cm", "dpi", "ft",
    "g", "h", "hPa", "ha", "hr", "in", "kB", "kHz", "kW", "kWh", "kbit", "kbps", "kg", "km",
    "km/h", "lb", "m", "m/s", "mA", "mL", "mg", "mi", "min", "mm", "mph", "ms", "m²", "nm", "ns",
    "oz", "ppi", "pt", "px", "rpm", "s", "sec", "t", "yd", "°", "°C", "°F", "µm", "µs", "μm", "μs",
];
const EN_AMBIGUOUS_UNITS: &[&str] = &["in"];
const EN_MONTHS: &[&str] = &[
    "Apr",
    "Apr.",
    "April",
    "Aug",
    "Aug.",
    "August",
    "Dec",
    "Dec.",
    "December",
    "Feb",
    "Feb.",
    "February",
    "Jan",
    "Jan.",
    "January",
    "Jul",
    "July",
    "Jun",
    "June",
    "Mar",
    "Mar.",
    "March",
    "May",
    "Nov",
    "Nov.",
    "November",
    "Oct",
    "Oct.",
    "October",
    "Sep",
    "Sep.",
    "Sept.",
    "September",
];
const EN_ABBR_NEXT: &[&str] = &[
    "Dr.", "Fig.", "Figs.", "Mr.", "Mrs.", "Ms.", "Mt.", "No.", "Nos.", "Prof.", "St.", "ch.",
    "p.", "pp.", "sec.", "vol.",
];
const EN_RANGE_LEAD_WORDS: &[&str] = &["p.", "pp."];
const COMMON_UNITS: &[&str] = &[
    "A", "B", "GB", "GHz", "GiB", "Hz", "K", "KB", "KiB", "L", "MB", "MHz", "MiB", "TB", "TiB",
    "V", "W", "bit", "cm", "g", "h", "kB", "kHz", "kW", "kg", "km", "m", "mL", "mg", "min", "mm",
    "ms", "ns", "px", "s", "°C", "°F",
];

/// The top-level fields of the pretty `Debug` output of a table: the lines between the opening
/// line of a field and its closing line, trimmed.
fn fields(data: &LanguageData) -> BTreeMap<String, Vec<String>> {
    let text = format!("{data:#?}");
    let mut map = BTreeMap::new();
    let mut lines = text.lines().skip(1);
    while let Some(line) = lines.next() {
        let Some(rest) = line.strip_prefix("    ") else {
            break;
        };
        if rest.starts_with(' ') {
            continue;
        }
        let (name, value) = rest
            .split_once(": ")
            .unwrap_or_else(|| panic!("field line {line:?}"));
        let mut body = Vec::new();
        if value.ends_with(['[', '(', '{']) {
            for inner in lines.by_ref() {
                if inner.starts_with("        ")
                    || inner.starts_with("    ") && inner.trim().len() > 2
                {
                    body.push(inner.trim().to_owned());
                } else {
                    break;
                }
            }
        } else {
            body.push(value.trim_end_matches(',').to_owned());
        }
        map.insert(name.to_owned(), body);
    }
    map
}

/// Decodes the escapes that `Debug` writes inside a string or a character literal.
fn unescape(literal: &str) -> String {
    let mut out = String::new();
    let mut chars = literal.chars();
    while let Some(c) = chars.next() {
        if c != '\\' {
            out.push(c);
            continue;
        }
        match chars.next() {
            Some('u') => {
                let hex: String = chars.by_ref().skip(1).take_while(|c| *c != '}').collect();
                out.push(char::from_u32(u32::from_str_radix(&hex, 16).unwrap()).unwrap());
            }
            Some('n') => out.push('\n'),
            Some('t') => out.push('\t'),
            Some(other) => out.push(other),
            None => panic!("dangling escape"),
        }
    }
    out
}

/// The quoted strings of the lines, in order.
fn strings(lines: &[String]) -> Vec<String> {
    lines
        .iter()
        .filter_map(|l| {
            let l = l.trim_end_matches(',');
            let l = if l.starts_with('"') {
                l
            } else {
                l.split_once(": ")?.1
            };
            l.strip_prefix('"')?.strip_suffix('"').map(unescape)
        })
        .collect()
}

/// The character literals of the lines, in order.
fn chars_of(lines: &[String]) -> Vec<char> {
    lines
        .iter()
        .filter_map(|l| {
            let l = l.trim_end_matches(',');
            let inner = l.strip_prefix('\'')?.strip_suffix('\'')?;
            let s = unescape(inner);
            let mut it = s.chars();
            let c = it.next()?;
            it.next().is_none().then_some(c)
        })
        .collect()
}

fn scalar(map: &BTreeMap<String, Vec<String>>, name: &str) -> String {
    map.get(name)
        .unwrap_or_else(|| panic!("no field {name}"))
        .join(" ")
}

fn list(map: &BTreeMap<String, Vec<String>>, name: &str) -> Vec<String> {
    strings(map.get(name).unwrap_or_else(|| panic!("no field {name}")))
}

fn expect_sorted_set(map: &BTreeMap<String, Vec<String>>, name: &str, expected: &[&str]) {
    let stored = list(map, name);
    let mut sorted = stored.clone();
    sorted.sort();
    sorted.dedup();
    assert_eq!(
        stored, sorted,
        "{name} is not strictly sorted by code point"
    );
    let want: Vec<&str> = expected.to_vec();
    let got: Vec<&str> = stored.iter().map(String::as_str).collect();
    assert_eq!(got, want, "{name}");
}

/// The abbreviation entries as (parts, bind).
fn abbreviations(map: &BTreeMap<String, Vec<String>>) -> Vec<(Vec<String>, String)> {
    let lines = &map["abbreviations"];
    let mut out = Vec::new();
    let mut parts = Vec::new();
    let mut in_parts = false;
    for line in lines {
        if line.starts_with("parts: [") {
            in_parts = true;
            parts.clear();
        } else if in_parts && line.starts_with(']') {
            in_parts = false;
        } else if in_parts {
            parts.extend(strings(std::slice::from_ref(line)));
        } else if let Some(bind) = line.strip_prefix("bind: ") {
            out.push((parts.clone(), bind.trim_end_matches(',').to_owned()));
        }
    }
    out
}

fn expect_abbreviations(
    map: &BTreeMap<String, Vec<String>>,
    inner: &[&str],
    next: &[&str],
    capitalized: &[&str],
) {
    let got = abbreviations(map);
    let mut want: Vec<(Vec<String>, String)> = Vec::new();
    for entry in inner {
        want.push((
            entry.split(' ').map(str::to_owned).collect(),
            "Inner".to_owned(),
        ));
    }
    for part in next {
        want.push((vec![(*part).to_owned()], "Next".to_owned()));
    }
    for part in capitalized {
        want.push((vec![(*part).to_owned()], "NextCapitalized".to_owned()));
    }
    // Sorted by parts, first part then the next (section 7.6), and no two with the same parts.
    let keys: Vec<&Vec<String>> = got.iter().map(|(p, _)| p).collect();
    let mut sorted = keys.clone();
    sorted.sort();
    sorted.dedup();
    assert_eq!(
        keys, sorted,
        "abbreviations are not strictly sorted by parts"
    );
    let mut got_sorted = got.clone();
    got_sorted.sort();
    want.sort();
    assert_eq!(got_sorted, want);
}

fn numbers(map: &BTreeMap<String, Vec<String>>) -> BTreeMap<String, String> {
    map["numbers"]
        .iter()
        .filter_map(|l| l.split_once(": "))
        .map(|(k, v)| (k.to_owned(), v.trim_end_matches(',').to_owned()))
        .collect()
}

#[allow(clippy::too_many_arguments)]
fn expect_numbers(
    map: &BTreeMap<String, Vec<String>>,
    decimal: char,
    group: char,
    min_grouping: u8,
    minus: char,
    percent_spaced: bool,
    range_dash: char,
    nan: &str,
    infinity: &str,
) {
    let n = numbers(map);
    let ch = |k: &str| chars_of(std::slice::from_ref(&n[k]))[0];
    let st = |k: &str| strings(std::slice::from_ref(&n[k]))[0].clone();
    assert_eq!(ch("decimal"), decimal);
    assert_eq!(ch("group"), group);
    assert_eq!(n["min_grouping_digits"], min_grouping.to_string());
    assert_eq!(ch("minus"), minus);
    assert_eq!(n["percent_spaced"], percent_spaced.to_string());
    assert_eq!(ch("range_dash"), range_dash);
    assert_eq!(n["range_spaced"], "false");
    assert_eq!(st("nan"), nan);
    assert_eq!(st("infinity"), infinity);
}

fn expect_quotes(map: &BTreeMap<String, Vec<String>>, glyphs: Option<[char; 4]>) {
    match glyphs {
        Some(g) => assert_eq!(chars_of(&map["quotes"]), g),
        None => assert_eq!(scalar(map, "quotes"), "None"),
    }
}

fn expect_ellipsis(map: &BTreeMap<String, Vec<String>>, forms: [&str; 3]) {
    assert_eq!(strings(&map["ellipsis"]), forms);
}

fn expect_empty(map: &BTreeMap<String, Vec<String>>, names: &[&str]) {
    for name in names {
        assert_eq!(list(map, name), Vec::<String>::new(), "{name}");
    }
}

fn expect_signs(map: &BTreeMap<String, Vec<String>>) {
    let mut signs = chars_of(&map["signs"]);
    signs.sort_unstable();
    assert_eq!(signs, ['§', '¶', '№']);
}

#[test]
fn ru_ru_scalars() {
    let map = fields(&LanguageData::RU_RU);
    assert_eq!(scalar(&map, "tag"), "\"ru-RU\"");
    assert_eq!(scalar(&map, "cldr_version"), "\"49\"");
    assert_eq!(scalar(&map, "script"), "Cyrillic");
    expect_quotes(&map, Some(['«', '»', '„', '“']));
    assert_eq!(scalar(&map, "decade_apostrophe"), "false");
    assert_eq!(scalar(&map, "trailing_possessive"), "false");
    let dash = map["dash"].join(" ");
    assert!(
        dash.contains("style: SpacedGlued") && dash.contains("dialogue: true"),
        "{dash}"
    );
    expect_ellipsis(&map, ["?..", "!..", "?!.."]);
    assert_eq!(scalar(&map, "short_word_length"), "2");
    assert_eq!(scalar(&map, "month_position"), "AfterDay");
    expect_signs(&map);
    expect_numbers(
        &map,
        ',',
        '\u{a0}',
        1,
        '\u{2212}',
        true,
        '\u{2013}',
        "не\u{a0}число",
        "∞",
    );
}

#[test]
fn ru_ru_lists() {
    let map = fields(&LanguageData::RU_RU);
    expect_empty(&map, &["elisions", "atom_exceptions"]);
    expect_sorted_set(&map, "short_words", RU_SHORT_WORDS);
    assert_eq!(RU_SHORT_WORDS.len(), 37);
    expect_sorted_set(&map, "particles", RU_PARTICLES);
    expect_sorted_set(&map, "units", RU_UNITS);
    expect_sorted_set(&map, "ambiguous_units", RU_AMBIGUOUS_UNITS);
    expect_sorted_set(&map, "months", RU_MONTHS);
    expect_sorted_set(&map, "year_words", RU_YEAR_WORDS);
    expect_sorted_set(&map, "century_words", RU_CENTURY_WORDS);
    expect_sorted_set(&map, "range_lead_words", RU_RANGE_LEAD_WORDS);
    expect_abbreviations(&map, RU_ABBR_INNER, RU_ABBR_NEXT, RU_ABBR_NEXT_CAPITALIZED);
}

#[test]
fn en_us_scalars() {
    let map = fields(&LanguageData::EN_US);
    assert_eq!(scalar(&map, "tag"), "\"en-US\"");
    assert_eq!(scalar(&map, "cldr_version"), "\"49\"");
    assert_eq!(scalar(&map, "script"), "Latin");
    expect_quotes(&map, Some(['“', '”', '‘', '’']));
    assert_eq!(scalar(&map, "decade_apostrophe"), "true");
    assert_eq!(scalar(&map, "trailing_possessive"), "true");
    let dash = map["dash"].join(" ");
    assert!(
        dash.contains("style: Closed") && dash.contains("dialogue: false"),
        "{dash}"
    );
    expect_ellipsis(&map, ["?…", "!…", "?!…"]);
    assert_eq!(scalar(&map, "short_word_length"), "0");
    assert_eq!(scalar(&map, "month_position"), "Both");
    expect_signs(&map);
    expect_numbers(&map, '.', ',', 1, '\u{2212}', false, '\u{2013}', "NaN", "∞");
}

#[test]
fn en_us_lists() {
    let map = fields(&LanguageData::EN_US);
    expect_empty(&map, &["particles", "year_words", "century_words"]);
    expect_sorted_set(&map, "elisions", EN_ELISIONS);
    expect_sorted_set(&map, "short_words", EN_SHORT_WORDS);
    assert_eq!(EN_SHORT_WORDS.len(), 23);
    expect_sorted_set(&map, "atom_exceptions", EN_ATOM_EXCEPTIONS);
    expect_sorted_set(&map, "units", EN_UNITS);
    expect_sorted_set(&map, "ambiguous_units", EN_AMBIGUOUS_UNITS);
    expect_sorted_set(&map, "months", EN_MONTHS);
    expect_sorted_set(&map, "range_lead_words", EN_RANGE_LEAD_WORDS);
    expect_abbreviations(&map, &[], EN_ABBR_NEXT, &[]);
}

#[test]
fn common_table_is_section_7_4() {
    let map = fields(&LanguageData::COMMON);
    assert_eq!(scalar(&map, "tag"), "\"und\"");
    assert_eq!(scalar(&map, "cldr_version"), "\"49\"");
    assert_eq!(scalar(&map, "script"), "Other");
    expect_quotes(&map, None);
    assert_eq!(scalar(&map, "dash"), "None");
    assert_eq!(scalar(&map, "decade_apostrophe"), "false");
    assert_eq!(scalar(&map, "trailing_possessive"), "false");
    expect_ellipsis(&map, ["?…", "!…", "?!…"]);
    assert_eq!(scalar(&map, "short_word_length"), "0");
    assert_eq!(scalar(&map, "month_position"), "AfterDay");
    expect_empty(
        &map,
        &[
            "elisions",
            "short_words",
            "particles",
            "atom_exceptions",
            "ambiguous_units",
            "months",
            "year_words",
            "century_words",
            "range_lead_words",
        ],
    );
    assert!(abbreviations(&map).is_empty());
    expect_sorted_set(&map, "units", COMMON_UNITS);
    expect_signs(&map);
    expect_numbers(
        &map, '.', '\u{a0}', 0, '\u{2212}', false, '\u{2013}', "NaN", "∞",
    );
}

#[test]
fn constants_and_language_tables_agree() {
    assert_eq!(LanguageData::RU_RU.tag(), "ru-RU");
    assert_eq!(LanguageData::EN_US.tag(), "en-US");
    assert_eq!(LanguageData::COMMON.tag(), "und");
    assert_ne!(LanguageData::RU_RU, LanguageData::EN_US);
    assert_eq!(LanguageData::RU_RU, LanguageData::RU_RU.clone());
}

#[test]
fn an_empty_table_has_the_values_of_common_and_empty_lists() {
    let map = fields(&LanguageData::new("xx"));
    assert_eq!(LanguageData::new("xx").tag(), "xx");
    assert_eq!(scalar(&map, "cldr_version"), "\"\"");
    assert_eq!(scalar(&map, "script"), "Other");
    expect_quotes(&map, None);
    assert_eq!(scalar(&map, "dash"), "None");
    expect_ellipsis(&map, ["?…", "!…", "?!…"]);
    expect_empty(
        &map,
        &["units", "elisions", "short_words", "particles", "months"],
    );
    assert!(chars_of(&map["signs"]).is_empty());
    expect_numbers(
        &map, '.', '\u{a0}', 0, '\u{2212}', false, '\u{2013}', "NaN", "∞",
    );
}

#[test]
fn with_dash_style_creates_dash_data() {
    let map = fields(&LanguageData::new("xx").with_dash_style(DashStyle::Spaced));
    let dash = map["dash"].join(" ");
    assert!(
        dash.contains("style: Spaced") && dash.contains("dialogue: false"),
        "{dash}"
    );
    let map = fields(&LanguageData::RU_RU.with_dash_style(DashStyle::Closed));
    let dash = map["dash"].join(" ");
    assert!(
        dash.contains("style: Closed") && dash.contains("dialogue: true"),
        "{dash}"
    );
}

#[test]
fn setters_are_const_and_replace_their_field() {
    const TABLE: LanguageData = LanguageData::new("xx")
        .with_cldr_version("49")
        .with_script(Script::Latin)
        .with_quotes(Some(QuotePairs::new(('«', '»'), ('‹', '›'))))
        .with_elisions(&["'x"])
        .with_decade_apostrophe(true)
        .with_trailing_possessive(true)
        .with_dash(Some(DashData::new(DashStyle::SpacedGlued, true)))
        .with_ellipsis(EllipsisForms::new("?.", "!.", "?!."))
        .with_short_words(&["a"])
        .with_short_word_length(3)
        .with_particles(&["p"])
        .with_abbreviations(&[Abbreviation::new(&["x."], Bind::Next)])
        .with_atom_exceptions(&["e.g."])
        .with_units(&["u"])
        .with_ambiguous_units(&["u"])
        .with_months(&["May"])
        .with_month_position(MonthPosition::BeforeDay)
        .with_year_words(&["yr"])
        .with_century_words(&["c"])
        .with_range_lead_words(&["p."])
        .with_signs(&['§'])
        .with_numbers(
            NumberData::new()
                .with_decimal(',')
                .with_group('.')
                .with_min_grouping_digits(2),
        );
    let map = fields(&TABLE);
    assert_eq!(scalar(&map, "script"), "Latin");
    expect_quotes(&map, Some(['«', '»', '‹', '›']));
    assert_eq!(list(&map, "elisions"), ["'x"]);
    assert_eq!(scalar(&map, "decade_apostrophe"), "true");
    assert_eq!(scalar(&map, "short_word_length"), "3");
    assert_eq!(list(&map, "short_words"), ["a"]);
    assert_eq!(list(&map, "particles"), ["p"]);
    assert_eq!(list(&map, "atom_exceptions"), ["e.g."]);
    assert_eq!(list(&map, "units"), ["u"]);
    assert_eq!(list(&map, "ambiguous_units"), ["u"]);
    assert_eq!(list(&map, "months"), ["May"]);
    assert_eq!(scalar(&map, "month_position"), "BeforeDay");
    assert_eq!(list(&map, "year_words"), ["yr"]);
    assert_eq!(list(&map, "century_words"), ["c"]);
    assert_eq!(list(&map, "range_lead_words"), ["p."]);
    assert_eq!(chars_of(&map["signs"]), ['§']);
    expect_ellipsis(&map, ["?.", "!.", "?!."]);
    assert_eq!(
        abbreviations(&map),
        [(vec!["x.".to_owned()], "Next".to_owned())]
    );
    let n = numbers(&map);
    assert_eq!(n["min_grouping_digits"], "2");
}

#[test]
fn built_in_tables_pass_validation() {
    static RU: LanguageData = LanguageData::RU_RU;
    static EN: LanguageData = LanguageData::EN_US;
    static COMMON: LanguageData = LanguageData::COMMON;
    for data in [&RU, &EN, &COMMON] {
        let lang = Language::from_data(data).unwrap_or_else(|e| panic!("{}: {e}", data.tag()));
        assert_eq!(lang.tag(), data.tag());
    }
    assert_eq!(Language::from_data(&RU), Ok(Language::RU_RU));
    assert_eq!(Language::from_data(&EN), Ok(Language::EN_US));
    assert_eq!(Language::from_data(&COMMON), Ok(Language::COMMON));
}

// ---------------------------------------------------------------------------------------------
// The CLDR 49 extract (section 13.9): quote pairs, decimal, group, minimum grouping digits,
// percent pattern, range pattern, `nan`, `infinity`, month names and the units marked "CLDR 49".
// ---------------------------------------------------------------------------------------------

/// The built-in tables with the CLDR locale they take their values from (`en-US` inherits `en`,
/// section 7.3).
fn cldr_pairs() -> [(&'static LanguageData, &'static cldr49::Locale); 2] {
    [
        (&LanguageData::RU_RU, &cldr49::RU),
        (&LanguageData::EN_US, &cldr49::EN),
    ]
}

fn first_char(text: &str) -> char {
    let mut it = text.chars();
    let c = it.next().unwrap();
    assert!(it.next().is_none(), "{text:?} is one character");
    c
}

#[test]
fn quote_pairs_are_the_cldr_delimiters() {
    for (data, cldr) in cldr_pairs() {
        let map = fields(data);
        let want = cldr.delimiters.map(first_char);
        assert_eq!(chars_of(&map["quotes"]), want, "{}", data.tag());
    }
}

#[test]
fn numbers_are_the_cldr_symbols() {
    for (data, cldr) in cldr_pairs() {
        let tag = data.tag();
        let n = numbers(&fields(data));
        let ch = |k: &str| chars_of(std::slice::from_ref(&n[k]))[0];
        let st = |k: &str| strings(std::slice::from_ref(&n[k]))[0].clone();
        assert_eq!(ch("decimal"), first_char(cldr.decimal), "{tag} decimal");
        assert_eq!(ch("group"), first_char(cldr.group), "{tag} group");
        assert_eq!(
            n["min_grouping_digits"],
            cldr.minimum_grouping_digits.to_string(),
            "{tag} minimumGroupingDigits"
        );
        assert_eq!(st("nan"), cldr.nan, "{tag} nan");
        assert_eq!(st("infinity"), cldr.infinity, "{tag} infinity");
        // The percent pattern `#,##0 %` (a space, U+00A0 in ru) or `#,##0%`.
        let after_zero = cldr.percent_pattern.rsplit_once('0').unwrap().1;
        let spaced = after_zero.starts_with(char::is_whitespace);
        assert_eq!(
            n["percent_spaced"],
            spaced.to_string(),
            "{tag} percentFormat"
        );
        // The range pattern `{0}–{1}`.
        let middle = cldr
            .range_pattern
            .strip_prefix("{0}")
            .and_then(|r| r.strip_suffix("{1}"))
            .unwrap();
        assert_eq!(ch("range_dash"), first_char(middle.trim()), "{tag} range");
        assert_eq!(
            n["range_spaced"],
            (middle != middle.trim()).to_string(),
            "{tag} range spacing"
        );
    }
}

#[test]
fn the_minus_sign_is_the_documented_deviation_from_cldr() {
    // Section 7.5: CLDR 49 gives U+002D, typolish U+2212 (the task).
    for (data, cldr) in cldr_pairs() {
        assert_eq!(cldr.minus_sign, "-", "{}", data.tag());
        let n = numbers(&fields(data));
        assert_eq!(chars_of(std::slice::from_ref(&n["minus"]))[0], '\u{2212}');
    }
}

fn cldr_months(cldr: &cldr49::Locale) -> Vec<String> {
    let mut all: Vec<String> = cldr
        .months_wide
        .iter()
        .chain(cldr.months_abbreviated.iter())
        .map(|s| (*s).to_owned())
        .collect();
    all.sort();
    all.dedup();
    all
}

#[test]
fn months_are_the_union_of_the_cldr_wide_and_abbreviated_forms() {
    // ru: exactly the union, without duplicates (`мая` once).
    let ru = list(&fields(&LanguageData::RU_RU), "months");
    assert_eq!(ru, cldr_months(&cldr49::RU));
    assert_eq!(ru.len(), 23);
    // en: the union plus the dotted forms of the project (section 7.5).
    let en = list(&fields(&LanguageData::EN_US), "months");
    let union = cldr_months(&cldr49::EN);
    assert_eq!(union.len(), 23);
    for month in &union {
        assert!(en.contains(month), "{month}");
    }
    let extra: Vec<&String> = en.iter().filter(|m| !union.contains(m)).collect();
    assert_eq!(
        extra,
        [
            "Apr.", "Aug.", "Dec.", "Feb.", "Jan.", "Mar.", "Nov.", "Oct.", "Sep.", "Sept."
        ]
    );
}

/// The symbol of a CLDR unit pattern: `{0}` and the spaces around it removed.
fn unit_symbol(pattern: &str) -> String {
    pattern
        .replace("{0}", "")
        .trim_matches([' ', '\u{a0}', '\u{202f}'])
        .to_owned()
}

/// The units that the tables of sections 7.2 and 7.3 mark "project".
const RU_PROJECT_UNITS: &[&str] = &[
    "КБ",
    "КиБ",
    "МиБ",
    "ГиБ",
    "ТиБ",
    "бит",
    "Кбит",
    "Гбит",
    "Кбит/с",
    "Мбит/с",
    "Гбит/с",
    "КБ/с",
    "МБ/с",
    "ГБ/с",
    "сек.",
    "мин.",
    "сут.",
    "дм",
    "ц",
    "°",
    "°С",
    "₽",
    "руб.",
    "коп.",
    "$",
    "€",
    "тыс.",
    "млн",
    "млрд",
    "трлн",
    "шт.",
    "экз.",
    "чел.",
    "об/мин",
    "px",
    "dpi",
    "м2",
    "м3",
    "км2",
    "см2",
    "см3",
    "мм2",
];
const EN_PROJECT_UNITS: &[&str] = &[
    "KB", "KiB", "MiB", "GiB", "TiB", "bit", "kbit", "Mbit", "Gbit", "kbps", "Mbps", "Gbps",
    "MB/s", "GB/s", "µs", "µm", "mph", "rpm", "ppi", "pt", "°",
];

fn sorted(mut items: Vec<String>) -> Vec<String> {
    items.sort();
    items.dedup();
    items
}

#[test]
fn units_marked_cldr_are_the_short_symbols_of_the_extract() {
    for (data, cldr, project) in [
        (&LanguageData::RU_RU, &cldr49::RU, RU_PROJECT_UNITS),
        (&LanguageData::EN_US, &cldr49::EN, EN_PROJECT_UNITS),
    ] {
        let stored = list(&fields(data), "units");
        let derived: Vec<String> = cldr
            .units_short
            .iter()
            .chain(cldr.units_narrow.iter())
            .map(|(_, pattern)| unit_symbol(pattern))
            .collect();
        // Every CLDR symbol is in the table, and the table holds nothing but the CLDR symbols and
        // the project ones.
        for symbol in &derived {
            assert!(stored.contains(symbol), "{}: {symbol}", data.tag());
        }
        let expected = sorted(
            derived
                .iter()
                .cloned()
                .chain(project.iter().map(|s| (*s).to_owned()))
                .collect(),
        );
        assert_eq!(stored, expected, "{}", data.tag());
    }
    // The counts of the design tables: ru 53 CLDR symbols, en 49 short and 3 narrow.
    assert_eq!(cldr49::RU.units_short.len(), 53);
    assert_eq!(cldr49::EN.units_short.len(), 49);
    assert_eq!(cldr49::EN.units_narrow.len(), 3);
}

#[test]
fn the_cldr_units_spell_the_symbols_of_the_design() {
    // A few symbols with their separators: CLDR writes `{0} МБ` with U+0020 in ru (section 7.5,
    // glue is the task's, not CLDR's) and `{0}°C` or `{0} °C` as the extract says.
    let ru: Vec<(&str, String)> = cldr49::RU
        .units_short
        .iter()
        .map(|(id, p)| (*id, unit_symbol(p)))
        .collect();
    for (id, symbol) in [
        ("digital-megabyte", "МБ"),
        ("duration-day", "дн."),
        ("energy-kilowatt-hour", "кВт⋅ч"),
        ("speed-kilometer-per-hour", "км/ч"),
        ("graphics-dot-per-inch", "тчк/дюйм"),
    ] {
        assert!(ru.contains(&(id, symbol.to_owned())), "{id}");
    }
    let en: Vec<(&str, String)> = cldr49::EN
        .units_short
        .iter()
        .chain(cldr49::EN.units_narrow.iter())
        .map(|(id, p)| (*id, unit_symbol(p)))
        .collect();
    for (id, symbol) in [
        ("digital-megabyte", "MB"),
        ("duration-second", "sec"),
        ("duration-second", "s"),
        ("length-inch", "in"),
        ("energy-kilowatt-hour", "kWh"),
        ("digital-byte", "B"),
    ] {
        assert!(en.contains(&(id, symbol.to_owned())), "{id} {symbol}");
    }
}

// ---------------------------------------------------------------------------------------------
// Validation (section 7.6): `Language::from_data`
// ---------------------------------------------------------------------------------------------

/// A table that lives for the whole test run.
fn leak(data: LanguageData) -> &'static LanguageData {
    Box::leak(Box::new(data))
}

/// A valid table with a neutral tag: everything empty, the values of COMMON.
fn base() -> LanguageData {
    LanguageData::new("xx")
}

#[track_caller]
fn accepted(data: LanguageData) -> Language {
    let tag = data.tag();
    Language::from_data(leak(data)).unwrap_or_else(|e| panic!("{tag}: {e}"))
}

/// Asserts `Error::InvalidLanguageData` that names the field, and returns nothing else.
#[track_caller]
fn rejected(data: LanguageData, field: &str) {
    let tag = data.tag();
    match Language::from_data(leak(data)) {
        Err(Error::InvalidLanguageData {
            tag: got, reason, ..
        }) => {
            assert_eq!(got, tag, "the error names the table");
            assert!(
                reason.contains(field),
                "the reason should name the field `{field}`, it says {reason:?}"
            );
        }
        Err(other) => panic!("{tag}: expected InvalidLanguageData, got {other:?}"),
        Ok(_) => panic!("{tag}: a table that breaks section 7.6 was accepted (field {field})"),
    }
}

#[test]
fn a_neutral_table_is_valid() {
    assert_eq!(accepted(base()).tag(), "xx");
    assert_eq!(accepted(LanguageData::COMMON).tag(), "und");
}

#[test]
fn tags_are_canonical() {
    for tag in [
        "ru", "ru-RU", "en-US", "es-419", "und", "xx", "zz-ZZ", "pt-BR",
    ] {
        let lang = Language::from_data(leak(base().with_tag(tag)));
        assert_eq!(lang.as_ref().map(Language::tag), Ok(tag), "{tag}");
    }
    for tag in [
        "EN-gb", "en-gb", "EN", "En-US", "ru-ru", "", "e", "eng", "en-GBR", "es-41", "es-4190",
        "en_US", "en-", "-US", "en-US-x", " en", "en ", "ру",
    ] {
        rejected(base().with_tag(leak_str(tag)), "tag");
    }
}

fn leak_str(text: &str) -> &'static str {
    Box::leak(text.to_owned().into_boxed_str())
}

#[test]
fn the_tag_en_gb_in_wrong_case_is_rejected_by_from_data() {
    // Section 13.9: `Language::from_data` with the tag `EN-gb` gives `InvalidLanguageData`.
    let data: &'static LanguageData = leak(LanguageData::EN_US.with_tag("EN-gb"));
    match Language::from_data(data) {
        Err(Error::InvalidLanguageData { tag, .. }) => assert_eq!(tag, "EN-gb"),
        other => panic!("{other:?}"),
    }
}

#[test]
fn quote_glyphs() {
    // A table whose pairs hold `「」` is rejected (section 13.9).
    rejected(
        base().with_quotes(Some(QuotePairs::new(('「', '」'), ('«', '»')))),
        "quotes",
    );
    rejected(
        base().with_quotes(Some(QuotePairs::new(('«', '»'), ('「', '」')))),
        "quotes",
    );
    // Straight quotes and primes would make the output input again.
    for bad in ['"', '\'', '′', '″', 'a', '5', ' ', '-', '(', '…'] {
        for position in 0..4 {
            let mut glyphs = ['«', '»', '„', '“'];
            glyphs[position] = bad;
            let pairs = QuotePairs::new((glyphs[0], glyphs[1]), (glyphs[2], glyphs[3]));
            rejected(base().with_quotes(Some(pairs)), "quotes");
        }
    }
    // The four glyphs are pairwise distinct.
    for pairs in [
        QuotePairs::new(('«', '»'), ('«', '„')),
        QuotePairs::new(('«', '«'), ('„', '“')),
        QuotePairs::new(('«', '»'), ('„', '„')),
        QuotePairs::new(('«', '»'), ('»', '“')),
        QuotePairs::new(('“', '”'), ('“', '”')),
    ] {
        rejected(base().with_quotes(Some(pairs)), "quotes");
    }
    // The glyphs that the lexer reads as quotes, other than the four above, are valid.
    accepted(base().with_quotes(Some(QuotePairs::new(('«', '»'), ('„', '“')))));
    accepted(base().with_quotes(Some(QuotePairs::new(('“', '”'), ('‘', '’')))));
    accepted(base().with_quotes(Some(QuotePairs::new(('„', '“'), ('‚', '‘')))));
    accepted(base().with_quotes(Some(QuotePairs::new(('‹', '›'), ('«', '»')))));
    accepted(base().with_quotes(None));
}

type Setter = fn(LanguageData, &'static [&'static str]) -> LanguageData;

/// The word lists of section 7.6: lowercase entries.
const WORD_LISTS: &[(&str, Setter)] = &[
    ("elisions", |d, l| d.with_elisions(l)),
    ("short_words", |d, l| d.with_short_words(l)),
    ("particles", |d, l| d.with_particles(l)),
    ("atom_exceptions", |d, l| d.with_atom_exceptions(l)),
    ("year_words", |d, l| d.with_year_words(l)),
    ("century_words", |d, l| d.with_century_words(l)),
    ("range_lead_words", |d, l| d.with_range_lead_words(l)),
];

/// The lists that keep the case they are printed in.
fn case_lists() -> Vec<(&'static str, Setter)> {
    vec![
        ("months", |d, l| d.with_months(l)),
        ("units", |d, l| d.with_units(l)),
    ]
}

#[test]
fn every_list_is_strictly_sorted_free_of_duplicates_and_of_empty_or_spaced_entries() {
    let mut all: Vec<(&str, Setter)> = WORD_LISTS.to_vec();
    all.extend(case_lists());
    for (field, set) in all {
        accepted(set(base(), &[]));
        accepted(set(base(), &["a"]));
        accepted(set(base(), &["a", "b"]));
        accepted(set(base(), &["а", "б", "в"]));
        rejected(set(base(), &["b", "a"]), field);
        rejected(set(base(), &["a", "a"]), field);
        rejected(set(base(), &["a", "b", "b"]), field);
        rejected(set(base(), &[""]), field);
        rejected(set(base(), &["a b"]), field);
        rejected(set(base(), &["a\u{a0}b"]), field);
        rejected(set(base(), &["a\tb"]), field);
        rejected(set(base(), &[" a"]), field);
        rejected(set(base(), &["a "]), field);
    }
}

#[test]
fn sorting_is_by_code_point() {
    // `A` < `a` < `я`: ASCII capitals before small letters before Cyrillic.
    accepted(base().with_units(&["A", "a", "я"]));
    rejected(base().with_units(&["a", "A"]), "units");
    // A longer entry follows its prefix.
    accepted(base().with_units(&["m", "m/s", "m²"]));
    rejected(base().with_units(&["m/s", "m"]), "units");
}

#[test]
fn word_lists_are_lowercase_and_the_others_keep_their_case() {
    for (field, set) in WORD_LISTS {
        rejected(set(base(), &["The"]), field);
        rejected(set(base(), &["ТО"]), field);
        rejected(set(base(), &["aB"]), field);
        accepted(set(base(), &["the"]));
        accepted(set(base(), &["то"]));
    }
    // `months`, `units`, `ambiguous_units` and abbreviations are printed in their own case.
    accepted(base().with_months(&["April", "May", "апреля"]));
    accepted(base().with_units(&["MB", "kB"]));
    accepted(base().with_units(&["MB", "mb"]));
    accepted(base().with_units(&["В", "в"]));
    const MR: &[Abbreviation] = &[Abbreviation::new(&["Mr."], Bind::Next)];
    accepted(base().with_abbreviations(MR));
}

#[test]
fn units_do_not_start_with_an_ascii_digit() {
    rejected(base().with_units(&["5px"]), "units");
    rejected(base().with_units(&["0x"]), "units");
    accepted(base().with_units(&["px5"]));
    accepted(base().with_units(&["m2", "m²"]));
    // A digit of another system is a letter for the lexer (section 2.11): accepted.
    accepted(base().with_units(&["٣px"]));
}

#[test]
fn ambiguous_units_are_also_units() {
    rejected(base().with_ambiguous_units(&["x"]), "ambiguous_units");
    rejected(
        base().with_units(&["m"]).with_ambiguous_units(&["x"]),
        "ambiguous_units",
    );
    accepted(base().with_units(&["x"]).with_ambiguous_units(&["x"]));
    rejected(
        base()
            .with_units(&["x", "y"])
            .with_ambiguous_units(&["y", "x"]),
        "ambiguous_units",
    );
    rejected(
        base().with_units(&["x"]).with_ambiguous_units(&["x", "x"]),
        "ambiguous_units",
    );
}

#[test]
fn a_unit_that_is_a_short_word_or_a_particle_is_listed_as_ambiguous() {
    // Compared with case: `с` must be listed, `В` need not (section 7.6).
    rejected(
        base().with_units(&["с"]).with_short_words(&["с"]),
        "ambiguous_units",
    );
    accepted(
        base()
            .with_units(&["с"])
            .with_short_words(&["с"])
            .with_ambiguous_units(&["с"]),
    );
    accepted(base().with_units(&["В"]).with_short_words(&["в"]));
    rejected(
        base().with_units(&["ж"]).with_particles(&["ж"]),
        "ambiguous_units",
    );
    accepted(
        base()
            .with_units(&["ж"])
            .with_particles(&["ж"])
            .with_ambiguous_units(&["ж"]),
    );
    accepted(base().with_units(&["Ж"]).with_particles(&["ж"]));
}

#[test]
fn abbreviations_are_sorted_by_their_parts_and_free_of_duplicates() {
    const SORTED: &[Abbreviation] = &[
        Abbreviation::new(&["а."], Bind::Next),
        Abbreviation::new(&["б."], Bind::Next),
        Abbreviation::new(&["и", "т.", "д."], Bind::Inner),
        Abbreviation::new(&["и", "т.", "п."], Bind::Inner),
    ];
    const UNSORTED: &[Abbreviation] = &[
        Abbreviation::new(&["б."], Bind::Next),
        Abbreviation::new(&["а."], Bind::Next),
    ];
    const UNSORTED_SECOND_PART: &[Abbreviation] = &[
        Abbreviation::new(&["и", "т.", "п."], Bind::Inner),
        Abbreviation::new(&["и", "т.", "д."], Bind::Inner),
    ];
    const SAME_PARTS: &[Abbreviation] = &[
        Abbreviation::new(&["а."], Bind::Next),
        Abbreviation::new(&["а."], Bind::NextCapitalized),
    ];
    const EMPTY_PART: &[Abbreviation] = &[Abbreviation::new(&[""], Bind::Next)];
    const SPACED_PART: &[Abbreviation] = &[Abbreviation::new(&["т. д."], Bind::Inner)];
    accepted(base().with_abbreviations(&[]));
    accepted(base().with_abbreviations(SORTED));
    rejected(base().with_abbreviations(UNSORTED), "abbreviations");
    rejected(
        base().with_abbreviations(UNSORTED_SECOND_PART),
        "abbreviations",
    );
    rejected(base().with_abbreviations(SAME_PARTS), "abbreviations");
    rejected(base().with_abbreviations(EMPTY_PART), "abbreviations");
    rejected(base().with_abbreviations(SPACED_PART), "abbreviations");
}

#[test]
fn number_data_is_checked() {
    let with = |n: NumberData| base().with_numbers(n);
    accepted(with(NumberData::new()));
    // `decimal != group`.
    rejected(
        with(NumberData::new().with_decimal(',').with_group(',')),
        "numbers",
    );
    rejected(
        with(NumberData::new().with_decimal('.').with_group('.')),
        "numbers",
    );
    // `min_grouping_digits <= 4`.
    for digits in 0..=4 {
        accepted(with(NumberData::new().with_min_grouping_digits(digits)));
    }
    rejected(
        with(NumberData::new().with_min_grouping_digits(5)),
        "numbers",
    );
    rejected(
        with(NumberData::new().with_min_grouping_digits(255)),
        "numbers",
    );
    // The minus sign is `-` or a dash, U+2010 to U+2015 or U+2212 (design section 7.6).
    for bad in [
        'a', 'я', '5', ' ', '\t', '\u{a0}', '"', '\'', '~', '*', '(', '.', '\u{ad}',
    ] {
        rejected(with(NumberData::new().with_minus(bad)), "numbers");
    }
    for good in [
        '-', '\u{2010}', '\u{2011}', '\u{2013}', '\u{2014}', '\u{2015}', '\u{2212}',
    ] {
        accepted(with(NumberData::new().with_minus(good)));
    }
    // The group separator is U+00A0, U+202F or not whitespace; not a letter or a digit.
    for bad in ['a', '5', ' ', '\u{2009}', '\u{3000}', '\t'] {
        rejected(with(NumberData::new().with_group(bad)), "numbers");
    }
    for good in ['\u{a0}', '\u{202f}', ',', '\'', '.', '_'] {
        accepted(with(NumberData::new().with_decimal('!').with_group(good)));
    }
    // The range dash is `–` or `—`, the dashes a second pass and `to_plain` read back; the
    // decimal separator is not a letter, a digit or whitespace (design section 7.6).
    for bad in ['a', '7', ' ', '-', '\u{2012}', '~'] {
        rejected(with(NumberData::new().with_range_dash(bad)), "numbers");
    }
    for good in ['\u{2013}', '\u{2014}'] {
        accepted(with(NumberData::new().with_range_dash(good)));
    }
    for bad in ['a', '7', ' '] {
        rejected(with(NumberData::new().with_decimal(bad)), "numbers");
    }
    // Nor a character that hides it between two digits (`1\u{ad}5` shows as `15`): a control or
    // format character (the whole category `Cf`), a joiner, a mark that joins a digit; nor a
    // character with the Unicode `Dash` property, which would read as a range or a minus (`1–5`,
    // `1－5`); nor a quote character, which the quote rules rewrite between two digits (`1'5` would
    // become `1′5`).
    for bad in [
        '\0',
        '\u{ad}',
        '\u{200b}',
        '\u{200d}',
        '\u{2068}',
        '\u{206a}',
        '\u{206f}',
        '\u{feff}',
        '\u{61c}',
        '\u{13430}',
        '\u{1343f}',
        '\u{e0041}',
        '\u{301}',
        '\u{fe0f}',
        '\u{600}',
        '-',
        '\u{2010}',
        '\u{2013}',
        '\u{2014}',
        '\u{2212}',
        '\u{2053}',
        '\u{2e3a}',
        '\u{2e3b}',
        '\u{fe58}',
        '\u{fe63}',
        '\u{ff0d}',
        '\'',
        '"',
        '’',
        '′',
        '″',
        '«',
    ] {
        rejected(with(NumberData::new().with_decimal(bad)), "numbers");
    }
    for good in [',', '.', '\u{66b}', '\u{b7}', '\u{2396}', '!'] {
        accepted(with(
            NumberData::new().with_decimal(good).with_group('\u{a0}'),
        ));
    }
}

#[test]
fn signs_are_punctuation() {
    // Design section 7.6: a sign is a character of the lexer's `Punct` class (design section
    // 3.5), the only class the readers of a sign match.
    rejected(base().with_signs(&['a']), "signs");
    rejected(base().with_signs(&['я']), "signs");
    rejected(base().with_signs(&['5']), "signs");
    rejected(base().with_signs(&['€']), "signs");
    rejected(base().with_signs(&['©']), "signs");
    rejected(base().with_signs(&['→']), "signs");
    rejected(base().with_signs(&[' ']), "signs");
    rejected(base().with_signs(&['\u{a0}']), "signs");
    rejected(base().with_signs(&['\t']), "signs");
    accepted(base().with_signs(&['§', '¶', '№']));
    accepted(base().with_signs(&['#']));
}

#[test]
fn ellipsis_forms_consist_of_question_exclamation_and_dots_only() {
    accepted(base().with_ellipsis(EllipsisForms::new("?..", "!..", "?!..")));
    accepted(base().with_ellipsis(EllipsisForms::new("?…", "!…", "?!…")));
    accepted(base().with_ellipsis(EllipsisForms::new("?.", "!.", "?!.")));
    for forms in [
        EllipsisForms::new("?x", "!…", "?!…"),
        EllipsisForms::new("?…", "! ", "?!…"),
        EllipsisForms::new("?…", "!…", "?!a"),
        EllipsisForms::new("?-", "!…", "?!…"),
        // An empty form would erase the author's mark (design section 7.6).
        EllipsisForms::new("", "!…", "?!…"),
        EllipsisForms::new("?…", "", "?!…"),
        EllipsisForms::new("?…", "!…", ""),
    ] {
        rejected(base().with_ellipsis(forms), "ellipsis");
    }
}

#[test]
fn from_static_does_not_validate_and_never_panics() {
    // Invalid data never panics, but rules may misbehave (section 2.2).
    let bad = leak(
        base()
            .with_tag("EN-gb")
            .with_short_words(&["б", "а", "а", ""])
            .with_units(&["5x", "x"])
            .with_quotes(Some(QuotePairs::new(('"', '"'), ('"', '"'))))
            .with_numbers(NumberData::new().with_decimal(',').with_group(',')),
    );
    let lang = Language::from_static(bad);
    assert_eq!(lang.tag(), "EN-gb");
    let t = Typographer::new(lang, RuleSet::ALL);
    for text in ["а б \"в\" 5 x - 5x... (c) 1000", "", "\"", "в а а б 5 x"] {
        let _ = t.typeset(text, Output::Unicode);
        let _ = t.report(text, Output::Unicode);
    }
}

#[test]
fn from_static_is_const() {
    static DATA: LanguageData = LanguageData::EN_US.with_tag("en-GB");
    static LANG: Language = Language::from_static(&DATA);
    static T: Typographer = Typographer::new(Language::from_static(&DATA), RuleSet::PROSE);
    assert_eq!(LANG.tag(), "en-GB");
    assert_eq!(T.language().tag(), "en-GB");
    assert_eq!(
        Language::from_data(&DATA).as_ref().map(Language::tag),
        Ok("en-GB")
    );
    // The design example (section 7.7): British English with spaced dashes needs no code.
    static GB: LanguageData = LanguageData::EN_US
        .with_tag("en-GB")
        .with_dash_style(DashStyle::Spaced);
    let t = Typographer::builder(Language::from_data(&GB).unwrap(), RuleSet::PROSE)
        .build()
        .unwrap();
    assert_eq!(
        t.typeset("Moscow - the capital", Output::Unicode),
        "Moscow — the\u{a0}capital"
    );
}

#[test]
fn the_built_in_tables_validate_through_the_typographer_builder_too() {
    // Section 13.9: built-in languages x built-in sets build without error.
    for language in Language::builtin().chain([Language::COMMON]) {
        for set in [
            RuleSet::EMPTY,
            RuleSet::PROSE,
            RuleSet::INTERFACE,
            RuleSet::ALL,
        ] {
            Typographer::builder(language.clone(), set).build().unwrap();
        }
    }
}

#[test]
fn the_quote_pairs_of_a_typographer_builder_are_checked_with_invalid_setting() {
    // Section 13.9: the same pairs that make `LanguageBuilder::build` fail with
    // `InvalidLanguageData` give `InvalidSetting` in the typographer builder.
    let pairs = QuotePairs::new(('«', '»'), ('«', '„'));
    let err = LanguageBuilder::new("xx")
        .quotes(Some(pairs))
        .build()
        .unwrap_err();
    assert!(matches!(err, Error::InvalidLanguageData { .. }), "{err:?}");
    let err = Typographer::builder(Language::RU_RU, RuleSet::PROSE)
        .quote_pairs(pairs)
        .build()
        .unwrap_err();
    assert!(
        matches!(&err, Error::InvalidSetting { setting, .. } if *setting == "quote_pairs"),
        "{err:?}"
    );
    // The pair `(«, »)` and `(«, „)` of section 13.9 for the builder of languages.
    let err = LanguageBuilder::new("xx")
        .quotes(Some(QuotePairs::new(('«', '»'), ('«', '„'))))
        .build()
        .unwrap_err();
    assert!(matches!(err, Error::InvalidLanguageData { .. }));
}

#[test]
fn the_language_builder_normalizes_the_case_of_the_tag() {
    // Section 13.9: `LanguageBuilder::new("EN-gb").build()` gives a language with tag `en-GB`.
    let lang = LanguageBuilder::new("EN-gb").build().unwrap();
    assert_eq!(lang.tag(), "en-GB");
    for (given, canonical) in [
        ("en-gb", "en-GB"),
        ("EN", "en"),
        ("UND", "und"),
        ("Es-419", "es-419"),
        ("RU-ru", "ru-RU"),
        ("zz", "zz"),
    ] {
        assert_eq!(
            LanguageBuilder::new(given).build().unwrap().tag(),
            canonical,
            "{given}"
        );
    }
    // A tag that is malformed beyond case is still an error.
    for bad in ["", "english", "e", "en-GBR", "en_US"] {
        assert!(
            matches!(
                LanguageBuilder::new(bad).build(),
                Err(Error::InvalidLanguageData { .. })
            ),
            "{bad:?}"
        );
    }
}
