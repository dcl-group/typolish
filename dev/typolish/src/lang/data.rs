//! The format of language data (design section 7.1) and its validation (section 7.6).
//!
//! Fields are `pub(crate)`: the tables of `src/lang/` are struct literals and the engine reads
//! them, while downstream crates see private fields and use the `const fn` constructors and
//! setters.

/// The script of a language's words; word lists and short-word rules are filtered by it.
///
/// ```
/// use typolish::{LanguageData, Script};
///
/// const XX: LanguageData = LanguageData::new("xx").with_script(Script::Latin);
/// assert_eq!(XX.tag(), "xx");
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum Script {
    /// Cyrillic letters.
    Cyrillic,
    /// Latin letters.
    Latin,
    /// Greek letters.
    Greek,
    /// Any other script, or none (`Language::COMMON`).
    Other,
}

/// The spacing of an em dash between words (rules `dash.spaced-hyphen`, `dash.double-hyphen`
/// and `nbsp.dash`, design section 8.4).
///
/// ```
/// use typolish::{DashStyle, LanguageData};
///
/// const EN_GB: LanguageData = LanguageData::EN_US.with_tag("en-GB").with_dash_style(DashStyle::Spaced);
/// assert_eq!(EN_GB.tag(), "en-GB");
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum DashStyle {
    /// No spaces around the dash between two plain words: `word—word` (en-US).
    Closed,
    /// The author's spaces stay: `word — word`.
    Spaced,
    /// The author's spaces stay, and the one before the dash is glued: `слово⍽— слово` (ru-RU).
    SpacedGlued,
}

/// How an abbreviation binds to its neighbours (rule `nbsp.abbreviations`, design section 8.7).
///
/// ```
/// use typolish::{Abbreviation, Bind};
///
/// const MR: Abbreviation = Abbreviation::new(&["Mr."], Bind::Next);
/// # let _ = MR;
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum Bind {
    /// Glue only between the parts of the entry (`т.⍽е.`).
    Inner,
    /// Glue also to the next word (`ул.⍽Ленина`, `Mr.⍽Smith`).
    Next,
    /// Glue to the next word only when it is capitalized (`г.⍽Москва`).
    NextCapitalized,
}

/// Where a month name stands relative to the day number (rule `nbsp.dates`).
///
/// ```
/// use typolish::{LanguageData, MonthPosition};
///
/// const XX: LanguageData = LanguageData::new("xx").with_month_position(MonthPosition::Both);
/// # let _ = XX;
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum MonthPosition {
    /// After the day: `21⍽ноября`.
    AfterDay,
    /// Before the day: `November⍽21`.
    BeforeDay,
    /// Either.
    Both,
}

/// The primary and nested quote pairs of a language, each as `(open, close)`.
///
/// The constructor does not validate; [`Language::from_data`](crate::Language::from_data) and
/// [`TypographerBuilder::build`](crate::TypographerBuilder::build) do (design section 7.6).
///
/// ```
/// use typolish::QuotePairs;
///
/// const RU: QuotePairs = QuotePairs::new(('«', '»'), ('„', '“'));
/// # let _ = RU;
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct QuotePairs {
    pub(crate) primary: (char, char),
    pub(crate) nested: (char, char),
}

impl QuotePairs {
    /// Quote pairs from the primary and the nested `(open, close)` pair.
    #[must_use]
    pub const fn new(primary: (char, char), nested: (char, char)) -> QuotePairs {
        QuotePairs { primary, nested }
    }

    /// The four glyphs: primary open and close, nested open and close.
    pub(crate) const fn glyphs(self) -> [char; 4] {
        [self.primary.0, self.primary.1, self.nested.0, self.nested.1]
    }
}

/// The dash data of a language: the style of an em dash between words and whether a hyphen at a
/// line start opens a line of dialogue (rule `dash.dialogue`).
///
/// ```
/// use typolish::{DashData, DashStyle};
///
/// const RU: DashData = DashData::new(DashStyle::SpacedGlued, true);
/// # let _ = RU;
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct DashData {
    pub(crate) style: DashStyle,
    pub(crate) dialogue: bool,
}

impl DashData {
    /// Dash data from a style and the dialogue flag.
    #[must_use]
    pub const fn new(style: DashStyle, dialogue: bool) -> DashData {
        DashData { style, dialogue }
    }
}

/// The forms of an ellipsis after `?`, after `!` and after `?!` (rule `ellipsis.dots`).
///
/// ```
/// use typolish::EllipsisForms;
///
/// const RU: EllipsisForms = EllipsisForms::new("?..", "!..", "?!..");
/// # let _ = RU;
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct EllipsisForms {
    pub(crate) question: &'static str,
    pub(crate) exclamation: &'static str,
    pub(crate) both: &'static str,
}

impl EllipsisForms {
    /// Ellipsis forms after `?`, `!` and `?!`.
    #[must_use]
    pub const fn new(
        question: &'static str,
        exclamation: &'static str,
        both: &'static str,
    ) -> EllipsisForms {
        EllipsisForms {
            question,
            exclamation,
            both,
        }
    }
}

/// An abbreviation: its parts as printed (`["т.", "е."]`) and how it binds.
///
/// ```
/// use typolish::{Abbreviation, Bind};
///
/// const TE: Abbreviation = Abbreviation::new(&["т.", "е."], Bind::Inner);
/// # let _ = TE;
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Abbreviation {
    pub(crate) parts: &'static [&'static str],
    pub(crate) bind: Bind,
}

impl Abbreviation {
    /// An abbreviation from its parts and its binding.
    #[must_use]
    pub const fn new(parts: &'static [&'static str], bind: Bind) -> Abbreviation {
        Abbreviation { parts, bind }
    }
}

/// The number symbols and patterns of a language (CLDR `numbers`, design section 7.1).
///
/// ```
/// use typolish::NumberData;
///
/// const DE: NumberData = NumberData::new().with_decimal(',').with_group('.').with_min_grouping_digits(1);
/// # let _ = DE;
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct NumberData {
    pub(crate) decimal: char,
    pub(crate) group: char,
    pub(crate) min_grouping_digits: u8,
    pub(crate) minus: char,
    pub(crate) percent_spaced: bool,
    pub(crate) range_dash: char,
    pub(crate) range_spaced: bool,
    pub(crate) nan: &'static str,
    pub(crate) infinity: &'static str,
}

impl NumberData {
    /// The values of `Language::COMMON` (design section 7.4): decimal `.`, group U+00A0, no
    /// grouping, minus U+2212, percent attached, range `–` without spaces, `NaN`, `∞`.
    #[must_use]
    pub const fn new() -> NumberData {
        NumberData {
            decimal: '.',
            group: '\u{a0}',
            min_grouping_digits: 0,
            minus: '\u{2212}',
            percent_spaced: false,
            range_dash: '\u{2013}',
            range_spaced: false,
            nan: "NaN",
            infinity: "∞",
        }
    }

    /// Sets the decimal separator.
    #[must_use]
    pub const fn with_decimal(mut self, decimal: char) -> NumberData {
        self.decimal = decimal;
        self
    }

    /// Sets the group separator.
    #[must_use]
    pub const fn with_group(mut self, group: char) -> NumberData {
        self.group = group;
        self
    }

    /// Sets the minimum grouping digits: 1 to 4 in the CLDR meaning (group from `3 + n` digits),
    /// 0 never groups.
    #[must_use]
    pub const fn with_min_grouping_digits(mut self, digits: u8) -> NumberData {
        self.min_grouping_digits = digits;
        self
    }

    /// Sets the minus sign: `-` or a dash, U+2010 to U+2015 or U+2212 (design section 7.6).
    #[must_use]
    pub const fn with_minus(mut self, minus: char) -> NumberData {
        self.minus = minus;
        self
    }

    /// Sets whether the CLDR percent pattern has a space before `%`.
    #[must_use]
    pub const fn with_percent_spaced(mut self, spaced: bool) -> NumberData {
        self.percent_spaced = spaced;
        self
    }

    /// Sets the dash of the CLDR range pattern.
    #[must_use]
    pub const fn with_range_dash(mut self, dash: char) -> NumberData {
        self.range_dash = dash;
        self
    }

    /// Sets whether the CLDR range pattern has spaces around the dash.
    #[must_use]
    pub const fn with_range_spaced(mut self, spaced: bool) -> NumberData {
        self.range_spaced = spaced;
        self
    }

    /// Sets the symbol of not-a-number.
    #[must_use]
    pub const fn with_nan(mut self, nan: &'static str) -> NumberData {
        self.nan = nan;
        self
    }

    /// Sets the symbol of infinity.
    #[must_use]
    pub const fn with_infinity(mut self, infinity: &'static str) -> NumberData {
        self.infinity = infinity;
        self
    }
}

impl Default for NumberData {
    /// The same as [`NumberData::new`].
    fn default() -> NumberData {
        NumberData::new()
    }
}

/// The data table of a language: a table of fixed structure with no logic (design section 7).
///
/// Built-in tables are [`LanguageData::RU_RU`], [`LanguageData::EN_US`] and
/// [`LanguageData::COMMON`]. A caller declares its own language as a `const` built with the
/// `const fn` setters and wraps it with [`Language::from_data`](crate::Language::from_data)
/// (validated) or [`Language::from_static`](crate::Language::from_static) (for a `static`
/// typographer).
///
/// ```
/// use typolish::{DashStyle, Language, LanguageData};
///
/// const EN_GB: LanguageData = LanguageData::EN_US.with_tag("en-GB").with_dash_style(DashStyle::Spaced);
/// static EN_GB_DATA: LanguageData = EN_GB;
///
/// let lang = Language::from_data(&EN_GB_DATA)?;
/// assert_eq!(lang.tag(), "en-GB");
/// # Ok::<(), typolish::Error>(())
/// ```
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct LanguageData {
    pub(crate) tag: &'static str,
    pub(crate) cldr_version: &'static str,
    pub(crate) script: Script,
    pub(crate) quotes: Option<QuotePairs>,
    pub(crate) elisions: &'static [&'static str],
    pub(crate) decade_apostrophe: bool,
    pub(crate) trailing_possessive: bool,
    pub(crate) dash: Option<DashData>,
    pub(crate) ellipsis: EllipsisForms,
    pub(crate) short_words: &'static [&'static str],
    pub(crate) short_word_length: u8,
    pub(crate) particles: &'static [&'static str],
    pub(crate) abbreviations: &'static [Abbreviation],
    pub(crate) atom_exceptions: &'static [&'static str],
    pub(crate) units: &'static [&'static str],
    pub(crate) ambiguous_units: &'static [&'static str],
    pub(crate) months: &'static [&'static str],
    pub(crate) month_position: MonthPosition,
    pub(crate) year_words: &'static [&'static str],
    pub(crate) century_words: &'static [&'static str],
    pub(crate) range_lead_words: &'static [&'static str],
    pub(crate) signs: &'static [char],
    pub(crate) numbers: NumberData,
}

impl LanguageData {
    /// Russian (Russia), design section 7.2.
    pub const RU_RU: LanguageData = super::ru_ru::RU_RU;
    /// English (United States), design section 7.3.
    pub const EN_US: LanguageData = super::en_us::EN_US;
    /// No language: the values of design section 7.4, tag `und`.
    pub const COMMON: LanguageData = super::common::COMMON;

    /// An empty table with the given tag: the values of `COMMON` (design section 7.4) with every
    /// list empty, `units` and `signs` included, and `cldr_version` `""`.
    ///
    /// ```
    /// use typolish::LanguageData;
    ///
    /// assert_eq!(LanguageData::new("xx").tag(), "xx");
    /// ```
    #[must_use]
    pub const fn new(tag: &'static str) -> LanguageData {
        LanguageData {
            tag,
            cldr_version: "",
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
            units: &[],
            ambiguous_units: &[],
            months: &[],
            month_position: MonthPosition::AfterDay,
            year_words: &[],
            century_words: &[],
            range_lead_words: &[],
            signs: &[],
            numbers: NumberData::new(),
        }
    }

    /// The tag, for example `"ru-RU"`; `"und"` for `COMMON`.
    ///
    /// ```
    /// use typolish::LanguageData;
    ///
    /// assert_eq!(LanguageData::RU_RU.tag(), "ru-RU");
    /// ```
    #[must_use]
    pub const fn tag(&self) -> &'static str {
        self.tag
    }

    /// Sets the tag; it must be canonical (`ll`, `ll-RR`, `ll-DDD` or `und`, design section 7.6).
    #[must_use]
    pub const fn with_tag(mut self, tag: &'static str) -> LanguageData {
        self.tag = tag;
        self
    }

    /// Sets the CLDR version the values were checked against.
    #[must_use]
    pub const fn with_cldr_version(mut self, version: &'static str) -> LanguageData {
        self.cldr_version = version;
        self
    }

    /// Sets the script of the language's words.
    #[must_use]
    pub const fn with_script(mut self, script: Script) -> LanguageData {
        self.script = script;
        self
    }

    /// Sets the quote pairs; `None` makes `quotes.pairs` inert.
    #[must_use]
    pub const fn with_quotes(mut self, quotes: Option<QuotePairs>) -> LanguageData {
        self.quotes = quotes;
        self
    }

    /// Sets the elisions (`'cause`): sorted by code point, lowercase.
    #[must_use]
    pub const fn with_elisions(mut self, elisions: &'static [&'static str]) -> LanguageData {
        self.elisions = elisions;
        self
    }

    /// Sets whether `'90s` takes an apostrophe.
    #[must_use]
    pub const fn with_decade_apostrophe(mut self, on: bool) -> LanguageData {
        self.decade_apostrophe = on;
        self
    }

    /// Sets whether a trailing possessive (`dogs'`) takes an apostrophe.
    #[must_use]
    pub const fn with_trailing_possessive(mut self, on: bool) -> LanguageData {
        self.trailing_possessive = on;
        self
    }

    /// Sets the dash data; `None` makes the `dash.*` rules and `nbsp.dash` inert.
    #[must_use]
    pub const fn with_dash(mut self, dash: Option<DashData>) -> LanguageData {
        self.dash = dash;
        self
    }

    /// Sets the dash style; on a table whose dash data is `None` it makes it
    /// `Some(DashData::new(style, false))`.
    #[must_use]
    pub const fn with_dash_style(mut self, style: DashStyle) -> LanguageData {
        self.dash = match self.dash {
            Some(dash) => Some(DashData::new(style, dash.dialogue)),
            None => Some(DashData::new(style, false)),
        };
        self
    }

    /// Sets the ellipsis forms after `?` and `!`.
    #[must_use]
    pub const fn with_ellipsis(mut self, ellipsis: EllipsisForms) -> LanguageData {
        self.ellipsis = ellipsis;
        self
    }

    /// Sets the short words: sorted by code point, lowercase.
    #[must_use]
    pub const fn with_short_words(mut self, words: &'static [&'static str]) -> LanguageData {
        self.short_words = words;
        self
    }

    /// Sets the length up to which any word of the language's script is short; 0 makes
    /// `nbsp.short-length` inert.
    #[must_use]
    pub const fn with_short_word_length(mut self, max_chars: u8) -> LanguageData {
        self.short_word_length = max_chars;
        self
    }

    /// Sets the particles: sorted by code point, lowercase.
    #[must_use]
    pub const fn with_particles(mut self, particles: &'static [&'static str]) -> LanguageData {
        self.particles = particles;
        self
    }

    /// Sets the abbreviations: sorted by their parts, as printed.
    #[must_use]
    pub const fn with_abbreviations(
        mut self,
        abbreviations: &'static [Abbreviation],
    ) -> LanguageData {
        self.abbreviations = abbreviations;
        self
    }

    /// Sets the dotted words that are not technical (`e.g.`): sorted by code point, lowercase.
    #[must_use]
    pub const fn with_atom_exceptions(mut self, words: &'static [&'static str]) -> LanguageData {
        self.atom_exceptions = words;
        self
    }

    /// Sets the units: sorted by code point, compared with case.
    #[must_use]
    pub const fn with_units(mut self, units: &'static [&'static str]) -> LanguageData {
        self.units = units;
        self
    }

    /// Sets the units that are also words: sorted by code point, each also in the units.
    #[must_use]
    pub const fn with_ambiguous_units(mut self, units: &'static [&'static str]) -> LanguageData {
        self.ambiguous_units = units;
        self
    }

    /// Sets the month forms used next to a day number: sorted by code point, as printed.
    #[must_use]
    pub const fn with_months(mut self, months: &'static [&'static str]) -> LanguageData {
        self.months = months;
        self
    }

    /// Sets where a month name stands relative to the day number.
    #[must_use]
    pub const fn with_month_position(mut self, position: MonthPosition) -> LanguageData {
        self.month_position = position;
        self
    }

    /// Sets the words after a year: sorted by code point, lowercase.
    #[must_use]
    pub const fn with_year_words(mut self, words: &'static [&'static str]) -> LanguageData {
        self.year_words = words;
        self
    }

    /// Sets the words after a century: sorted by code point, lowercase.
    #[must_use]
    pub const fn with_century_words(mut self, words: &'static [&'static str]) -> LanguageData {
        self.century_words = words;
        self
    }

    /// Sets the words before a range (`стр.`, `pp.`): sorted by code point, lowercase.
    #[must_use]
    pub const fn with_range_lead_words(mut self, words: &'static [&'static str]) -> LanguageData {
        self.range_lead_words = words;
        self
    }

    /// Sets the signs that name a number (`№`, `§`, `¶`): sorted by code point.
    #[must_use]
    pub const fn with_signs(mut self, signs: &'static [char]) -> LanguageData {
        self.signs = signs;
        self
    }

    /// Sets the number symbols and patterns.
    #[must_use]
    pub const fn with_numbers(mut self, numbers: NumberData) -> LanguageData {
        self.numbers = numbers;
        self
    }

    /// Runs the checks of design section 7.6; the error names the field.
    pub(crate) fn validate(&self) -> Result<(), &'static str> {
        if !is_canonical_tag(self.tag) {
            return Err("tag: not a canonical tag of the form ll, ll-RR, ll-DDD or und");
        }
        self.table().validate()
    }

    /// The view of the table that the engine reads (design section 7.1).
    pub(crate) const fn table(&self) -> Table<'static> {
        Table {
            cldr_version: self.cldr_version,
            script: self.script,
            quotes: self.quotes,
            elisions: List::Static(self.elisions),
            decade_apostrophe: self.decade_apostrophe,
            trailing_possessive: self.trailing_possessive,
            dash: self.dash,
            ellipsis: self.ellipsis,
            short_words: List::Static(self.short_words),
            short_word_length: self.short_word_length,
            particles: List::Static(self.particles),
            abbreviations: Abbreviations::Static(self.abbreviations),
            atom_exceptions: List::Static(self.atom_exceptions),
            units: List::Static(self.units),
            ambiguous_units: List::Static(self.ambiguous_units),
            months: List::Static(self.months),
            month_position: self.month_position,
            year_words: List::Static(self.year_words),
            century_words: List::Static(self.century_words),
            range_lead_words: List::Static(self.range_lead_words),
            signs: self.signs,
            numbers: self.numbers,
        }
    }
}

/// A list of language data: a list of a `const` table, or an owned list of a language built at
/// run time (`LanguageBuilder`). The engine reads both through this view and does not know where
/// the data came from (design section 7.1). Equality compares the entries.
#[derive(Clone, Copy, Debug)]
pub(crate) enum List<'a> {
    Static(&'static [&'static str]),
    Owned(&'a [Box<str>]),
}

impl<'a> List<'a> {
    /// The number of entries.
    pub(crate) fn len(self) -> usize {
        match self {
            List::Static(list) => list.len(),
            List::Owned(list) => list.len(),
        }
    }

    /// Whether the list has no entry.
    pub(crate) fn is_empty(self) -> bool {
        self.len() == 0
    }

    /// The entry at `i`.
    pub(crate) fn get(self, i: usize) -> Option<&'a str> {
        match self {
            List::Static(list) => list.get(i).copied(),
            List::Owned(list) => list.get(i).map(|entry| &**entry),
        }
    }

    /// The entries in order.
    pub(crate) fn iter(self) -> impl Iterator<Item = &'a str> + Clone {
        (0..self.len()).filter_map(move |i| self.get(i))
    }

    /// Binary search with a comparator of entries, as `slice::binary_search_by`.
    pub(crate) fn binary_search_by(
        self,
        mut f: impl FnMut(&str) -> core::cmp::Ordering,
    ) -> Result<usize, usize> {
        match self {
            List::Static(list) => list.binary_search_by(|entry| f(entry)),
            List::Owned(list) => list.binary_search_by(|entry| f(entry)),
        }
    }

    /// Whether the list holds `key`, compared with case (the list is sorted by code point).
    pub(crate) fn contains(self, key: &str) -> bool {
        self.binary_search_by(|entry| entry.cmp(key)).is_ok()
    }
}

/// Calls `found` with every entry of a list sorted by code point (`len` entries, read through
/// `entry`) that is a prefix of `text`, and its length in characters, shortest first. The entries
/// that start with the characters read so far are a range of the list, narrowed by one binary
/// search per character; reading stops when the range is empty, so a long `text` costs no more
/// than the longest entry.
pub(crate) fn prefix_entries<'l>(
    len: usize,
    entry: impl Fn(usize) -> Option<&'l str>,
    text: impl Iterator<Item = char>,
    mut found: impl FnMut(usize, &'l str),
) {
    let (mut lo, mut hi) = (0, len);
    // Every entry in `lo..hi` starts with the first `bytes` bytes of `text`.
    let mut bytes = 0;
    for (k, c) in text.enumerate() {
        // After the common prefix, an entry that ends there comes first, then the entries by
        // their next character.
        let next = |i: usize| {
            entry(i)
                .and_then(|e| e.get(bytes..))
                .and_then(|rest| rest.chars().next())
        };
        lo = partition(lo, hi, |i| next(i) < Some(c));
        hi = partition(lo, hi, |i| next(i) <= Some(c));
        if lo >= hi {
            return;
        }
        bytes += c.len_utf8();
        if let Some(e) = entry(lo).filter(|e| e.len() == bytes) {
            found(k + 1, e);
        }
    }
}

/// The first index of `lo..hi` where `pred` is false, `pred` being true on a prefix of the range.
fn partition(mut lo: usize, mut hi: usize, pred: impl Fn(usize) -> bool) -> usize {
    while lo < hi {
        let mid = lo + (hi - lo) / 2;
        if pred(mid) {
            lo = mid + 1;
        } else {
            hi = mid;
        }
    }
    lo
}

impl PartialEq for List<'_> {
    fn eq(&self, other: &List<'_>) -> bool {
        self.len() == other.len() && self.iter().eq(other.iter())
    }
}

impl Eq for List<'_> {}

/// An abbreviation of a language built at run time: its owned parts and its binding.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct OwnedAbbreviation {
    pub(crate) parts: Vec<Box<str>>,
    pub(crate) bind: BindKey,
}

/// `Bind` with an order, so that owned abbreviations sort and deduplicate.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct BindKey(u8);

impl BindKey {
    pub(crate) const fn of(bind: Bind) -> BindKey {
        BindKey(match bind {
            Bind::Inner => 0,
            Bind::Next => 1,
            Bind::NextCapitalized => 2,
        })
    }

    pub(crate) const fn bind(self) -> Bind {
        match self.0 {
            0 => Bind::Inner,
            1 => Bind::Next,
            _ => Bind::NextCapitalized,
        }
    }
}

/// The abbreviations of a table, static or owned, as `List` is for strings.
#[derive(Clone, Copy, Debug)]
pub(crate) enum Abbreviations<'a> {
    Static(&'static [Abbreviation]),
    Owned(&'a [OwnedAbbreviation]),
}

impl<'a> Abbreviations<'a> {
    /// The number of entries.
    pub(crate) fn len(self) -> usize {
        match self {
            Abbreviations::Static(list) => list.len(),
            Abbreviations::Owned(list) => list.len(),
        }
    }

    /// Whether the list has no entry.
    pub(crate) fn is_empty(self) -> bool {
        self.len() == 0
    }

    /// The entry at `i`: its parts and its binding.
    pub(crate) fn get(self, i: usize) -> Option<(List<'a>, Bind)> {
        match self {
            Abbreviations::Static(list) => list.get(i).map(|a| (List::Static(a.parts), a.bind)),
            Abbreviations::Owned(list) => {
                list.get(i).map(|a| (List::Owned(&a.parts), a.bind.bind()))
            }
        }
    }

    /// The entries in order.
    pub(crate) fn iter(self) -> impl Iterator<Item = (List<'a>, Bind)> + Clone {
        (0..self.len()).filter_map(move |i| self.get(i))
    }

    /// The indices of the entries whose first part starts with `c`. The entries are sorted by
    /// their parts (design section 7.6), so they form one range, found by binary search.
    pub(crate) fn starting_with(self, c: char) -> core::ops::Range<usize> {
        let first = |i: usize| {
            self.get(i)
                .and_then(|(parts, _)| parts.get(0))
                .and_then(|part| part.chars().next())
        };
        let lo = partition(0, self.len(), |i| first(i) < Some(c));
        let hi = partition(lo, self.len(), |i| first(i) <= Some(c));
        lo..hi
    }
}

impl PartialEq for Abbreviations<'_> {
    fn eq(&self, other: &Abbreviations<'_>) -> bool {
        self.len() == other.len() && self.iter().eq(other.iter())
    }
}

impl Eq for Abbreviations<'_> {}

/// The view of a language's data that the engine and the validation read: the fields of
/// `LanguageData` other than the tag, with lists that may be static or owned (design section
/// 7.1).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Table<'a> {
    pub(crate) cldr_version: &'a str,
    pub(crate) script: Script,
    pub(crate) quotes: Option<QuotePairs>,
    pub(crate) elisions: List<'a>,
    pub(crate) decade_apostrophe: bool,
    pub(crate) trailing_possessive: bool,
    pub(crate) dash: Option<DashData>,
    pub(crate) ellipsis: EllipsisForms,
    pub(crate) short_words: List<'a>,
    pub(crate) short_word_length: u8,
    pub(crate) particles: List<'a>,
    pub(crate) abbreviations: Abbreviations<'a>,
    pub(crate) atom_exceptions: List<'a>,
    pub(crate) units: List<'a>,
    pub(crate) ambiguous_units: List<'a>,
    pub(crate) months: List<'a>,
    pub(crate) month_position: MonthPosition,
    pub(crate) year_words: List<'a>,
    pub(crate) century_words: List<'a>,
    pub(crate) range_lead_words: List<'a>,
    pub(crate) signs: &'a [char],
    pub(crate) numbers: NumberData,
}

/// The data of a language built at run time: every list owned (design section 7.1).
#[derive(Clone, Debug)]
pub(crate) struct OwnedTable {
    pub(crate) cldr_version: String,
    pub(crate) script: Script,
    pub(crate) quotes: Option<QuotePairs>,
    pub(crate) elisions: Vec<Box<str>>,
    pub(crate) decade_apostrophe: bool,
    pub(crate) trailing_possessive: bool,
    pub(crate) dash: Option<DashData>,
    pub(crate) ellipsis: EllipsisForms,
    pub(crate) short_words: Vec<Box<str>>,
    pub(crate) short_word_length: u8,
    pub(crate) particles: Vec<Box<str>>,
    pub(crate) abbreviations: Vec<OwnedAbbreviation>,
    pub(crate) atom_exceptions: Vec<Box<str>>,
    pub(crate) units: Vec<Box<str>>,
    pub(crate) ambiguous_units: Vec<Box<str>>,
    pub(crate) months: Vec<Box<str>>,
    pub(crate) month_position: MonthPosition,
    pub(crate) year_words: Vec<Box<str>>,
    pub(crate) century_words: Vec<Box<str>>,
    pub(crate) range_lead_words: Vec<Box<str>>,
    pub(crate) signs: Vec<char>,
    pub(crate) numbers: NumberData,
}

/// The entries of a list as owned strings.
fn owned(list: List<'_>) -> Vec<Box<str>> {
    list.iter().map(Box::from).collect()
}

impl OwnedTable {
    /// An owned copy of a table.
    pub(crate) fn of(t: Table<'_>) -> OwnedTable {
        OwnedTable {
            cldr_version: t.cldr_version.to_owned(),
            script: t.script,
            quotes: t.quotes,
            elisions: owned(t.elisions),
            decade_apostrophe: t.decade_apostrophe,
            trailing_possessive: t.trailing_possessive,
            dash: t.dash,
            ellipsis: t.ellipsis,
            short_words: owned(t.short_words),
            short_word_length: t.short_word_length,
            particles: owned(t.particles),
            abbreviations: t
                .abbreviations
                .iter()
                .map(|(parts, bind)| OwnedAbbreviation {
                    parts: owned(parts),
                    bind: BindKey::of(bind),
                })
                .collect(),
            atom_exceptions: owned(t.atom_exceptions),
            units: owned(t.units),
            ambiguous_units: owned(t.ambiguous_units),
            months: owned(t.months),
            month_position: t.month_position,
            year_words: owned(t.year_words),
            century_words: owned(t.century_words),
            range_lead_words: owned(t.range_lead_words),
            signs: t.signs.to_vec(),
            numbers: t.numbers,
        }
    }

    /// The view of the owned data.
    pub(crate) fn table(&self) -> Table<'_> {
        Table {
            cldr_version: &self.cldr_version,
            script: self.script,
            quotes: self.quotes,
            elisions: List::Owned(&self.elisions),
            decade_apostrophe: self.decade_apostrophe,
            trailing_possessive: self.trailing_possessive,
            dash: self.dash,
            ellipsis: self.ellipsis,
            short_words: List::Owned(&self.short_words),
            short_word_length: self.short_word_length,
            particles: List::Owned(&self.particles),
            abbreviations: Abbreviations::Owned(&self.abbreviations),
            atom_exceptions: List::Owned(&self.atom_exceptions),
            units: List::Owned(&self.units),
            ambiguous_units: List::Owned(&self.ambiguous_units),
            months: List::Owned(&self.months),
            month_position: self.month_position,
            year_words: List::Owned(&self.year_words),
            century_words: List::Owned(&self.century_words),
            range_lead_words: List::Owned(&self.range_lead_words),
            signs: &self.signs,
            numbers: self.numbers,
        }
    }

    /// The normalization of `LanguageBuilder::build` (design section 7.1): the word lists of
    /// section 7.6 lowercased; every list sorted by code point and free of duplicates.
    pub(crate) fn normalize(&mut self) {
        for list in [
            &mut self.elisions,
            &mut self.short_words,
            &mut self.particles,
            &mut self.atom_exceptions,
            &mut self.year_words,
            &mut self.century_words,
            &mut self.range_lead_words,
        ] {
            for entry in list.iter_mut() {
                if !is_lowercase(entry) {
                    *entry = entry.chars().map(fold_char).collect::<String>().into();
                }
            }
        }
        for list in [
            &mut self.elisions,
            &mut self.short_words,
            &mut self.particles,
            &mut self.atom_exceptions,
            &mut self.year_words,
            &mut self.century_words,
            &mut self.range_lead_words,
            &mut self.units,
            &mut self.ambiguous_units,
            &mut self.months,
        ] {
            list.sort_unstable();
            list.dedup();
        }
        self.abbreviations.sort();
        self.abbreviations.dedup();
        self.signs.sort_unstable();
        self.signs.dedup();
    }
}

impl Table<'_> {
    /// The checks of design section 7.6 other than the tag; the error names the field.
    pub(crate) fn validate(&self) -> Result<(), &'static str> {
        if let Some(quotes) = self.quotes {
            check_quote_pairs(quotes).map_err(
                |_| "quotes: the four glyphs must be distinct quote characters other than \" ' ′ ″",
            )?;
        }
        let word_lists: [(List<'_>, &'static str); 7] = [
            (
                self.elisions,
                "elisions: entries must be non-empty, without whitespace, lowercase, sorted and unique",
            ),
            (
                self.short_words,
                "short_words: entries must be non-empty, without whitespace, lowercase, sorted and unique",
            ),
            (
                self.particles,
                "particles: entries must be non-empty, without whitespace, lowercase, sorted and unique",
            ),
            (
                self.atom_exceptions,
                "atom_exceptions: entries must be non-empty, without whitespace, lowercase, sorted and unique",
            ),
            (
                self.year_words,
                "year_words: entries must be non-empty, without whitespace, lowercase, sorted and unique",
            ),
            (
                self.century_words,
                "century_words: entries must be non-empty, without whitespace, lowercase, sorted and unique",
            ),
            (
                self.range_lead_words,
                "range_lead_words: entries must be non-empty, without whitespace, lowercase, sorted and unique",
            ),
        ];
        for (list, reason) in word_lists {
            if !is_sorted_list(list) || !list.iter().all(is_lowercase) {
                return Err(reason);
            }
        }
        let cased_lists: [(List<'_>, &'static str); 3] = [
            (
                self.months,
                "months: entries must be non-empty, without whitespace, sorted and unique",
            ),
            (
                self.units,
                "units: entries must be non-empty, without whitespace, sorted and unique",
            ),
            (
                self.ambiguous_units,
                "ambiguous_units: entries must be non-empty, without whitespace, sorted and unique",
            ),
        ];
        for (list, reason) in cased_lists {
            if !is_sorted_list(list) {
                return Err(reason);
            }
        }
        let abbreviations = self.abbreviations;
        let sorted = abbreviations
            .iter()
            .zip(abbreviations.iter().skip(1))
            .all(|((a, _), (b, _))| a.iter().lt(b.iter()));
        if !abbreviations.iter().all(|(parts, _)| is_valid_parts(parts)) || !sorted {
            return Err(
                "abbreviations: parts must be non-empty, without whitespace, sorted by parts and unique",
            );
        }
        if self
            .units
            .iter()
            .any(|unit| unit.starts_with(|c: char| c.is_ascii_digit()))
        {
            return Err("units: a unit must not start with an ASCII digit");
        }
        if !self
            .ambiguous_units
            .iter()
            .all(|unit| self.units.contains(unit))
        {
            return Err("ambiguous_units: every ambiguous unit must also be in units");
        }
        let unlisted = self.units.iter().any(|unit| {
            (self.short_words.contains(unit) || self.particles.contains(unit))
                && !self.ambiguous_units.contains(unit)
        });
        if unlisted {
            return Err(
                "ambiguous_units: a unit equal to a short word or a particle must be listed",
            );
        }
        if !self
            .signs
            .windows(2)
            .all(|pair| matches!(pair, [a, b] if a < b))
        {
            return Err("signs: must be sorted and unique");
        }
        if self
            .signs
            .iter()
            .any(|c| c.is_alphanumeric() || c.is_whitespace())
        {
            return Err("signs: a sign must not be a letter, a digit or whitespace");
        }
        let n = &self.numbers;
        if n.decimal == n.group {
            return Err("numbers: decimal and group separators must differ");
        }
        if n.min_grouping_digits > 4 {
            return Err("numbers: min_grouping_digits must be at most 4");
        }
        if !is_valid_group(n.group) {
            return Err(
                "numbers: the group separator must not be a letter, a digit or whitespace other than U+00A0 and U+202F",
            );
        }
        if !is_valid_minus(n.minus) {
            return Err("numbers: the minus must be `-` or a dash, U+2010 to U+2015 or U+2212");
        }
        let e = &self.ellipsis;
        if [e.question, e.exclamation, e.both].iter().any(|form| {
            form.is_empty() || !form.chars().all(|c| matches!(c, '?' | '!' | '.' | '…'))
        }) {
            return Err("ellipsis: forms must consist of ?, !, . and … only");
        }
        Ok(())
    }
}

/// The quote glyphs a language may use: the `Quote` characters of the lexer other than `"`, `'`,
/// `′` and `″` (design sections 3.5 and 7.6).
pub(crate) const QUOTE_GLYPHS: [char; 10] = ['«', '»', '„', '“', '”', '‘', '’', '‚', '‹', '›'];

/// Checks the four glyphs of quote pairs (design section 7.6): pairwise distinct quote glyphs.
pub(crate) fn check_quote_pairs(quotes: QuotePairs) -> Result<(), char> {
    let glyphs = quotes.glyphs();
    for (i, c) in glyphs.iter().enumerate() {
        if !QUOTE_GLYPHS.contains(c) || glyphs.iter().skip(i + 1).any(|other| other == c) {
            return Err(*c);
        }
    }
    Ok(())
}

/// A canonical tag of design section 7.6: `ll`, `ll-RR`, `ll-DDD` or `und`.
pub(crate) fn is_canonical_tag(tag: &str) -> bool {
    let lower = |b: &u8| b.is_ascii_lowercase();
    match tag.as_bytes() {
        b"und" => true,
        [a, b] => lower(a) && lower(b),
        [a, b, b'-', rest @ ..] if lower(a) && lower(b) => match rest {
            [r1, r2] => r1.is_ascii_uppercase() && r2.is_ascii_uppercase(),
            [d1, d2, d3] => d1.is_ascii_digit() && d2.is_ascii_digit() && d3.is_ascii_digit(),
            _ => false,
        },
        _ => false,
    }
}

/// Non-empty entries without whitespace, strictly sorted by code point.
fn is_sorted_list(list: List<'_>) -> bool {
    list.iter().all(is_valid_entry) && list.iter().zip(list.iter().skip(1)).all(|(a, b)| a < b)
}

fn is_valid_entry(entry: &str) -> bool {
    !entry.is_empty() && !entry.chars().any(char::is_whitespace)
}

fn is_valid_parts(parts: List<'_>) -> bool {
    !parts.is_empty() && parts.iter().all(is_valid_entry)
}

/// The word lists that `Signatures` covers.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum WordList {
    /// The entries of check 0 of the classifier (design section 3.6): units (the builder's
    /// additions included), month forms, atom exceptions, elisions, and every abbreviation part or
    /// run of parts that check 0 matches written together.
    Dictionary,
    /// The short words, the builder's additions included.
    ShortWords,
    /// The particles, the builder's additions included.
    Particles,
}

/// The signatures of the entries of some word lists of a typographer: a Bloom filter of 2048
/// bits over the list, the length in characters and the folded first and last characters of
/// every entry. Every comparison of design section 7.1 (with case, with the first letter folded,
/// or without case) keeps the signature, because folding is idempotent, so a key whose signature
/// is absent equals no entry of the list and its lookups can be skipped. A present signature
/// proves nothing. Computed once per typographer, with the active rules, without allocation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Signatures([u64; 32]);

impl Signatures {
    /// No signature.
    pub(crate) const NONE: Signatures = Signatures([0; 32]);

    /// The two bits of the signature of `chars` in `list`.
    fn bits(list: WordList, mut chars: impl Iterator<Item = char>) -> [usize; 2] {
        let first = chars.next();
        let mut last = first;
        let mut len = u64::from(first.is_some());
        for c in chars {
            last = Some(c);
            len += 1;
        }
        let code = |c: Option<char>| u64::from(c.map_or(0, |c| u32::from(fold_char(c))));
        let mut h = len ^ (code(first) << 8) ^ (code(last) << 32) ^ ((list as u64) << 60);
        // The finalizer of SplitMix64.
        h = (h ^ (h >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
        h = (h ^ (h >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
        h ^= h >> 31;
        [(h & 0x7ff) as usize, ((h >> 11) & 0x7ff) as usize]
    }

    /// Adds the signature of an entry of `list`.
    pub(crate) fn add(&mut self, list: WordList, chars: impl Iterator<Item = char>) {
        for bit in Signatures::bits(list, chars) {
            if let Some(word) = self.0.get_mut(bit / 64) {
                *word |= 1 << (bit % 64);
            }
        }
    }

    /// Whether some entry of `list` may have the characters `chars`; `false` means that none has.
    pub(crate) fn may_hold(&self, list: WordList, chars: impl Iterator<Item = char>) -> bool {
        Signatures::bits(list, chars).iter().all(|&bit| {
            self.0
                .get(bit / 64)
                .is_some_and(|word| word & (1 << (bit % 64)) != 0)
        })
    }

    /// Adds the entries of a language's lists: check 0, short words and particles.
    pub(crate) fn add_table(&mut self, data: &Table<'_>) {
        let lists = [data.units, data.months, data.atom_exceptions, data.elisions];
        for entry in lists.iter().flat_map(|list| list.iter()) {
            self.add(WordList::Dictionary, entry.chars());
        }
        // The parts of an abbreviation as check 0 reads them: one part, or a run of parts that
        // all end in a full stop.
        for (parts, _) in data.abbreviations.iter() {
            for i in 0..parts.len() {
                for (n, part) in parts.iter().enumerate().skip(i) {
                    if n > i
                        && !(part.ends_with('.')
                            && parts.get(n - 1).is_some_and(|p| p.ends_with('.')))
                    {
                        break;
                    }
                    let run = parts.iter().skip(i).take(n + 1 - i).flat_map(str::chars);
                    self.add(WordList::Dictionary, run);
                }
            }
        }
        for word in data.short_words.iter() {
            self.add(WordList::ShortWords, word.chars());
        }
        for word in data.particles.iter() {
            self.add(WordList::Particles, word.chars());
        }
    }
}

/// The case folding of word lists (design section 7.1): the first character of
/// `char::to_lowercase`. ASCII and the basic Cyrillic letters are folded without the tables of the
/// standard library, which are slow to search.
pub(crate) fn fold_char(c: char) -> char {
    let shifted = |by: u32| char::from_u32(u32::from(c) + by).unwrap_or(c);
    match c {
        'A'..='Z' => c.to_ascii_lowercase(),
        '\u{410}'..='\u{42f}' => shifted(0x20),
        '\u{400}'..='\u{40f}' => shifted(0x50),
        '\0'..='\x7f' | '\u{430}'..='\u{45f}' => c,
        _ => c.to_lowercase().next().unwrap_or(c),
    }
}

/// Whether each character equals its folding.
fn is_lowercase(word: &str) -> bool {
    word.chars().all(|c| fold_char(c) == c)
}

/// A group separator of design section 7.6.
pub(crate) fn is_valid_group(c: char) -> bool {
    !c.is_alphanumeric() && (!c.is_whitespace() || c == '\u{a0}' || c == '\u{202f}')
}

/// A minus sign of design section 7.6: `-` or a dash character of the lexer (U+2010 to U+2015,
/// U+2212), the signs that check 15 of design section 3.6 reads as the sign of a number, so that
/// the minus that `number.minus` writes changes nothing for the second pass (invariant I4).
pub(crate) fn is_valid_minus(c: char) -> bool {
    c == '-' || crate::engine::lex::is_dash_char(c)
}
