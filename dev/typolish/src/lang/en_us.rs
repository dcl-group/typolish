//! The en-US table (design section 7.3).
//!
//! One struct literal and nothing else: the lists are stored merged and sorted as design section
//! 7.6 requires; the design lists them for reading.

use super::data::{
    Abbreviation, Bind, DashData, DashStyle, EllipsisForms, LanguageData, MonthPosition,
    NumberData, QuotePairs, Script,
};

/// English (United States). CLDR 49 has no separate `en-US` file; the CLDR values are those of
/// `en`.
pub(crate) const EN_US: LanguageData = LanguageData {
    tag: "en-US",
    cldr_version: "49",
    script: Script::Latin,
    // CLDR 49 `en` `delimiters`.
    quotes: Some(QuotePairs::new(('“', '”'), ('‘', '’'))),
    // Project; `'n'` is left out, since it also quotes a letter (design section 6.2).
    elisions: &[
        "'bout", "'cause", "'em", "'nother", "'round", "'til", "'tis", "'twas",
    ],
    decade_apostrophe: true,
    trailing_possessive: true,
    // Secondary sources (CMOS through Wikipedia); design section 15.5, item 6.
    dash: Some(DashData::new(DashStyle::Closed, false)),
    // CLDR 49 ellipsis character U+2026.
    ellipsis: EllipsisForms::new("?…", "!…", "?!…"),
    // Project (design section 15.5, item 16).
    short_words: &[
        "a", "an", "and", "as", "at", "but", "by", "for", "if", "in", "nor", "not", "of", "off",
        "on", "or", "per", "so", "the", "to", "up", "via", "yet",
    ],
    short_word_length: 0,
    particles: &[],
    // Project.
    abbreviations: &[
        Abbreviation::new(&["Dr."], Bind::Next),
        Abbreviation::new(&["Fig."], Bind::Next),
        Abbreviation::new(&["Figs."], Bind::Next),
        Abbreviation::new(&["Mr."], Bind::Next),
        Abbreviation::new(&["Mrs."], Bind::Next),
        Abbreviation::new(&["Ms."], Bind::Next),
        Abbreviation::new(&["Mt."], Bind::Next),
        Abbreviation::new(&["No."], Bind::Next),
        Abbreviation::new(&["Nos."], Bind::Next),
        Abbreviation::new(&["Prof."], Bind::Next),
        Abbreviation::new(&["St."], Bind::Next),
        Abbreviation::new(&["ch."], Bind::Next),
        Abbreviation::new(&["p."], Bind::Next),
        Abbreviation::new(&["pp."], Bind::Next),
        Abbreviation::new(&["sec."], Bind::Next),
        Abbreviation::new(&["vol."], Bind::Next),
    ],
    atom_exceptions: &["a.m.", "cf.", "e.g.", "etc.", "i.e.", "p.m.", "vs."],
    // CLDR 49 units, short and narrow, merged with the project units (`µs`, `µm` with U+00B5).
    units: &[
        "A", "B", "GB", "GB/s", "GHz", "GW", "Gbit", "Gbps", "GiB", "Hz", "K", "KB", "KiB", "L",
        "MB", "MB/s", "MHz", "MW", "Mbit", "Mbps", "MiB", "TB", "TiB", "V", "W", "a.m.", "bit",
        "cm", "dpi", "ft", "g", "h", "hPa", "ha", "hr", "in", "kB", "kHz", "kJ", "kPa", "kW",
        "kWh", "kbit", "kbps", "kg", "km", "km/h", "lb", "m", "m/s", "mA", "mL", "mW", "mg", "mi",
        "min", "mm", "mph", "ms", "m²", "nm", "ns", "oz", "p.m.", "ppi", "pt", "px", "rpm", "s",
        "sec", "t", "yd", "°", "°C", "°F", "µm", "µs", "μm", "μs",
    ],
    ambiguous_units: &["in"],
    // CLDR 49 gregorian, wide and abbreviated; the dotted forms are the project's (design section
    // 7.5).
    months: &[
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
    ],
    month_position: MonthPosition::Both,
    year_words: &[],
    century_words: &[],
    range_lead_words: &["p.", "pp."],
    // Project.
    signs: &['§', '¶', '№'],
    numbers: NumberData {
        // CLDR 49.
        decimal: '.',
        group: ',',
        min_grouping_digits: 1,
        // The task; CLDR 49 gives U+002D (design section 7.5).
        minus: '\u{2212}',
        // CLDR 49 percentFormat `#,##0%`.
        percent_spaced: false,
        // CLDR 49 miscPatterns range `{0}–{1}`.
        range_dash: '\u{2013}',
        range_spaced: false,
        nan: "NaN",
        infinity: "∞",
    },
};
