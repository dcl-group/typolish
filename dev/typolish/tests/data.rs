//! Built-in language tables against design sections 7.2 to 7.4 (design section 13.9, `tests/data.rs`).
//!
//! `LanguageData` has no accessors except `tag`, so the stored values are read from its derived
//! `Debug` output (the field names are those of section 7.1). The expected lists below were
//! generated from the tables of the design by a script kept outside the repository. The check of
//! the CLDR values against an extract of cldr-json and the validation rows of section 7.6 join in
//! stage 3 (section 15.4).

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

use typolish::{
    Abbreviation, Bind, DashData, DashStyle, EllipsisForms, Language, LanguageData, MonthPosition,
    NumberData, QuotePairs, Script,
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
