//! The table of `Language::COMMON` (design section 7.4).
//!
//! One struct literal and nothing else: the lists are stored merged and sorted as design section
//! 7.6 requires; the design lists them for reading.

use super::data::{EllipsisForms, LanguageData, MonthPosition, NumberData, Script};

/// No language: only rules that need no language data are active.
pub(crate) const COMMON: LanguageData = LanguageData {
    tag: "und",
    cldr_version: "49",
    script: Script::Other,
    quotes: None,
    elisions: &[],
    decade_apostrophe: false,
    trailing_possessive: false,
    dash: None,
    ellipsis: EllipsisForms::new("?…", "!…", "?!…"),
    short_words: &[],
    short_word_length: 0,
    particles: &[],
    abbreviations: &[],
    atom_exceptions: &[],
    // Language-neutral SI and IEC symbols.
    units: &[
        "A", "B", "GB", "GHz", "GiB", "Hz", "K", "KB", "KiB", "L", "MB", "MHz", "MiB", "TB", "TiB",
        "V", "W", "bit", "cm", "g", "h", "kB", "kHz", "kW", "kg", "km", "m", "mL", "mg", "min",
        "mm", "ms", "ns", "px", "s", "°C", "°F",
    ],
    ambiguous_units: &[],
    months: &[],
    // Not read while `months` is empty.
    month_position: MonthPosition::AfterDay,
    year_words: &[],
    century_words: &[],
    range_lead_words: &[],
    signs: &['§', '¶', '№'],
    numbers: NumberData {
        decimal: '.',
        // Not used while grouping is off; a space is the SI-neutral separator.
        group: '\u{a0}',
        // Never group (design section 2.7).
        min_grouping_digits: 0,
        minus: '\u{2212}',
        // CLDR 49 root percentFormat `#,##0%`.
        percent_spaced: false,
        range_dash: '\u{2013}',
        range_spaced: false,
        nan: "NaN",
        infinity: "∞",
    },
};
