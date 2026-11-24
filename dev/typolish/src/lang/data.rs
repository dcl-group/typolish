//! The format of language data (design section 7.1) and its validation (design section 7.6).
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
    ///
    /// ```
    /// # use typolish::{Language, LanguageData, Output, Rule, RuleSet, Script, Typographer};
    /// static XX: LanguageData = LanguageData::new("xx")
    ///     .with_script(Script::Cyrillic)
    ///     .with_short_words(&["в"]);
    /// let rules = RuleSet::EMPTY.with(Rule::NbspShortWords);
    /// let t = Typographer::new(Language::from_data(&XX)?, rules);
    /// assert_eq!(t.typeset("в дом", Output::Unicode), "в\u{a0}дом");
    /// # Ok::<(), typolish::Error>(())
    /// ```
    Cyrillic,
    /// Latin letters.
    ///
    /// ```
    /// # use typolish::{Language, LanguageData, Output, Rule, RuleSet, Script, Typographer};
    /// static XX: LanguageData = LanguageData::new("xx")
    ///     .with_script(Script::Latin)
    ///     .with_short_words(&["a"]);
    /// let rules = RuleSet::EMPTY.with(Rule::NbspShortWords);
    /// let t = Typographer::new(Language::from_data(&XX)?, rules);
    /// assert_eq!(t.typeset("a cat", Output::Unicode), "a\u{a0}cat");
    /// # Ok::<(), typolish::Error>(())
    /// ```
    Latin,
    /// Greek letters.
    ///
    /// ```
    /// # use typolish::{Language, LanguageData, Output, Rule, RuleSet, Script, Typographer};
    /// static XX: LanguageData = LanguageData::new("xx")
    ///     .with_script(Script::Greek)
    ///     .with_short_words(&["ο"]);
    /// let rules = RuleSet::EMPTY.with(Rule::NbspShortWords);
    /// let t = Typographer::new(Language::from_data(&XX)?, rules);
    /// assert_eq!(t.typeset("ο λόγος", Output::Unicode), "ο\u{a0}λόγος");
    /// # Ok::<(), typolish::Error>(())
    /// ```
    Greek,
    /// Any other script, or none (`Language::COMMON`): the word lists match only words whose
    /// letters are not Cyrillic, Latin or Greek, unless `short_words_any_script` is set.
    ///
    /// ```
    /// # use typolish::{Language, LanguageData, Output, Rule, RuleSet, Script, Typographer};
    /// static XX: LanguageData = LanguageData::new("xx")
    ///     .with_script(Script::Other)
    ///     .with_short_words(&["a", "და"]);
    /// let rules = RuleSet::EMPTY.with(Rule::NbspShortWords);
    /// let t = Typographer::new(Language::from_data(&XX)?, rules);
    /// assert_eq!(t.typeset("და სახლი", Output::Unicode), "და\u{a0}სახლი");
    /// assert_eq!(t.typeset("a cat", Output::Unicode), "a cat");
    /// # Ok::<(), typolish::Error>(())
    /// ```
    Other,
}

/// The spacing of an em dash between words (rules `dash.spaced-hyphen`, `dash.double-hyphen`
/// and `nbsp.dash`, design section 8.4).
///
/// ```
/// use typolish::{DashStyle, LanguageData};
///
/// const EN_GB: LanguageData = LanguageData::EN_US
///     .with_tag("en-GB")
///     .with_dash_style(DashStyle::Spaced);
/// assert_eq!(EN_GB.tag(), "en-GB");
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum DashStyle {
    /// No spaces around the dash between two plain words: `word—word` (en-US).
    ///
    /// ```
    /// # use typolish::{DashStyle, Language, LanguageData, Output, Rule, RuleSet, Typographer};
    /// static XX: LanguageData = LanguageData::EN_US
    ///     .with_tag("xx")
    ///     .with_dash_style(DashStyle::Closed);
    /// let rules = RuleSet::EMPTY.with(Rule::DashSpacedHyphen);
    /// let t = Typographer::new(Language::from_data(&XX)?, rules);
    /// assert_eq!(t.typeset("one - two", Output::Unicode), "one—two");
    /// # Ok::<(), typolish::Error>(())
    /// ```
    Closed,
    /// The author's spaces stay: `word — word`.
    ///
    /// ```
    /// # use typolish::{DashStyle, Language, LanguageData, Output, Rule, RuleSet, Typographer};
    /// static XX: LanguageData = LanguageData::EN_US
    ///     .with_tag("xx")
    ///     .with_dash_style(DashStyle::Spaced);
    /// let rules = RuleSet::EMPTY.with(Rule::DashSpacedHyphen);
    /// let t = Typographer::new(Language::from_data(&XX)?, rules);
    /// assert_eq!(t.typeset("one - two", Output::Unicode), "one — two");
    /// # Ok::<(), typolish::Error>(())
    /// ```
    Spaced,
    /// The author's spaces stay, and the one before the dash is glued: `слово⍽— слово` (ru-RU).
    ///
    /// ```
    /// # use typolish::{DashStyle, Language, LanguageData, Output, Rule, RuleSet, Typographer};
    /// static XX: LanguageData = LanguageData::EN_US
    ///     .with_tag("xx")
    ///     .with_dash_style(DashStyle::SpacedGlued);
    /// let rules = RuleSet::EMPTY.with(Rule::NbspDash);
    /// let t = Typographer::new(Language::from_data(&XX)?, rules);
    /// assert_eq!(t.typeset("one — two", Output::Unicode), "one\u{a0}— two");
    /// # Ok::<(), typolish::Error>(())
    /// ```
    SpacedGlued,
}

/// How an abbreviation binds to its neighbors (rule `nbsp.abbreviations`, design section 8.7).
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
    ///
    /// ```
    /// # use typolish::{Abbreviation, Bind, Language, LanguageData, Output, Rule, RuleSet, Typographer};
    /// static XX: LanguageData = LanguageData::new("xx")
    ///     .with_abbreviations(&[Abbreviation::new(&["i.", "e."], Bind::Inner)]);
    /// let rules = RuleSet::EMPTY.with(Rule::NbspAbbreviations);
    /// let t = Typographer::new(Language::from_data(&XX)?, rules);
    /// assert_eq!(t.typeset("i. e. so", Output::Unicode), "i.\u{a0}e. so");
    /// # Ok::<(), typolish::Error>(())
    /// ```
    Inner,
    /// Glue also to the next word (`ул.⍽Ленина`, `Mr.⍽Smith`).
    ///
    /// ```
    /// # use typolish::{Abbreviation, Bind, Language, LanguageData, Output, Rule, RuleSet, Typographer};
    /// static XX: LanguageData = LanguageData::new("xx")
    ///     .with_abbreviations(&[Abbreviation::new(&["Mr."], Bind::Next)]);
    /// let rules = RuleSet::EMPTY.with(Rule::NbspAbbreviations);
    /// let t = Typographer::new(Language::from_data(&XX)?, rules);
    /// assert_eq!(t.typeset("Mr. smith", Output::Unicode), "Mr.\u{a0}smith");
    /// # Ok::<(), typolish::Error>(())
    /// ```
    Next,
    /// Glue to the next word only when it is capitalized (`г.⍽Москва`).
    ///
    /// ```
    /// # use typolish::{Abbreviation, Bind, Language, LanguageData, Output, Rule, RuleSet, Typographer};
    /// static XX: LanguageData = LanguageData::new("xx")
    ///     .with_abbreviations(&[Abbreviation::new(&["Mr."], Bind::NextCapitalized)]);
    /// let rules = RuleSet::EMPTY.with(Rule::NbspAbbreviations);
    /// let t = Typographer::new(Language::from_data(&XX)?, rules);
    /// assert_eq!(t.typeset("Mr. Smith, Mr. smith", Output::Unicode), "Mr.\u{a0}Smith, Mr. smith");
    /// # Ok::<(), typolish::Error>(())
    /// ```
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
    ///
    /// ```
    /// # use typolish::{Language, LanguageData, MonthPosition, Output, Rule, RuleSet, Typographer};
    /// static XX: LanguageData = LanguageData::new("xx")
    ///     .with_months(&["May"])
    ///     .with_month_position(MonthPosition::AfterDay);
    /// let rules = RuleSet::EMPTY.with(Rule::NbspDates);
    /// let t = Typographer::new(Language::from_data(&XX)?, rules);
    /// assert_eq!(t.typeset("on 21 May and May 21", Output::Unicode), "on 21\u{a0}May and May 21");
    /// # Ok::<(), typolish::Error>(())
    /// ```
    AfterDay,
    /// Before the day: `November⍽21`.
    ///
    /// ```
    /// # use typolish::{Language, LanguageData, MonthPosition, Output, Rule, RuleSet, Typographer};
    /// static XX: LanguageData = LanguageData::new("xx")
    ///     .with_months(&["May"])
    ///     .with_month_position(MonthPosition::BeforeDay);
    /// let rules = RuleSet::EMPTY.with(Rule::NbspDates);
    /// let t = Typographer::new(Language::from_data(&XX)?, rules);
    /// assert_eq!(t.typeset("on 21 May and May 21", Output::Unicode), "on 21 May and May\u{a0}21");
    /// # Ok::<(), typolish::Error>(())
    /// ```
    BeforeDay,
    /// Either.
    ///
    /// ```
    /// # use typolish::{Language, LanguageData, MonthPosition, Output, Rule, RuleSet, Typographer};
    /// static XX: LanguageData = LanguageData::new("xx")
    ///     .with_months(&["May"])
    ///     .with_month_position(MonthPosition::Both);
    /// let rules = RuleSet::EMPTY.with(Rule::NbspDates);
    /// let t = Typographer::new(Language::from_data(&XX)?, rules);
    /// assert_eq!(t.typeset("on 21 May and May 21", Output::Unicode), "on 21\u{a0}May and May\u{a0}21");
    /// # Ok::<(), typolish::Error>(())
    /// ```
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
    ///
    /// ```
    /// use typolish::{Language, LanguageData, Output, QuotePairs, Rule, RuleSet, Typographer};
    ///
    /// const PAIRS: QuotePairs = QuotePairs::new(('„', '“'), ('‚', '‘'));
    /// static DE: LanguageData = LanguageData::EN_US.with_tag("de").with_quotes(Some(PAIRS));
    ///
    /// let t = Typographer::new(Language::from_data(&DE)?, RuleSet::EMPTY.with(Rule::QuotesPairs));
    /// assert_eq!(t.typeset("\"a 'b' c\"", Output::Unicode), "„a ‚b‘ c“");
    /// # Ok::<(), typolish::Error>(())
    /// ```
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
    ///
    /// ```
    /// use typolish::{
    ///     DashData, DashStyle, Language, LanguageData, Output, Rule, RuleSet, Typographer,
    /// };
    ///
    /// const DASH: DashData = DashData::new(DashStyle::Spaced, true);
    /// static XX: LanguageData = LanguageData::EN_US.with_tag("en-GB").with_dash(Some(DASH));
    ///
    /// let t = Typographer::new(
    ///     Language::from_data(&XX)?,
    ///     RuleSet::EMPTY.with(Rule::DashDialogue),
    /// );
    /// assert_eq!(t.typeset("- Hello.", Output::Unicode), "—\u{a0}Hello.");
    /// # Ok::<(), typolish::Error>(())
    /// ```
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
    ///
    /// The rule keeps the signs the author typed and writes the rest of the form in place of the
    /// dots. Validation (design section 7.6) requires each form to be its own signs (none, `?`, `!`,
    /// `?!` or `!?`) followed by a non-empty run of `.` and `…` that does not end in `…` and
    /// exactly three full stops: a sign in the run, or three full stops of their own, would read as
    /// a new ellipsis on a second pass.
    ///
    /// ```
    /// use typolish::{EllipsisForms, Language, LanguageData, Output, Rule, RuleSet, Typographer};
    ///
    /// const FORMS: EllipsisForms = EllipsisForms::new("?..", "!..", "?!..");
    /// static XX: LanguageData = LanguageData::EN_US.with_tag("en-GB").with_ellipsis(FORMS);
    ///
    /// let t = Typographer::new(
    ///     Language::from_data(&XX)?,
    ///     RuleSet::EMPTY.with(Rule::EllipsisDots),
    /// );
    /// assert_eq!(
    ///     t.typeset("Really?... Wait...", Output::Unicode),
    ///     "Really?.. Wait…"
    /// );
    /// # Ok::<(), typolish::Error>(())
    /// ```
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
    ///
    /// ```
    /// use typolish::{
    ///     Abbreviation, Bind, Language, LanguageData, Output, Rule, RuleSet, Typographer,
    /// };
    ///
    /// const ABBREVIATIONS: &[Abbreviation] = &[Abbreviation::new(&["approx."], Bind::Next)];
    /// static XX: LanguageData = LanguageData::EN_US
    ///     .with_tag("en-GB")
    ///     .with_abbreviations(ABBREVIATIONS);
    ///
    /// let t = Typographer::new(
    ///     Language::from_data(&XX)?,
    ///     RuleSet::EMPTY.with(Rule::NbspAbbreviations),
    /// );
    /// assert_eq!(
    ///     t.typeset("approx. ten", Output::Unicode),
    ///     "approx.\u{a0}ten"
    /// );
    /// # Ok::<(), typolish::Error>(())
    /// ```
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
/// const DE: NumberData = NumberData::new()
///     .with_decimal(',')
///     .with_group('.')
///     .with_min_grouping_digits(1);
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
    ///
    /// ```
    /// use typolish::{Language, LanguageData, NumberData, Output, RuleSet, Typographer};
    ///
    /// static XX: LanguageData = LanguageData::new("xx").with_numbers(NumberData::new());
    ///
    /// let t = Typographer::new(Language::from_data(&XX)?, RuleSet::EMPTY);
    /// assert_eq!(t.format_number(-12345.5, Output::Unicode).to_string(), "\u{2212}12345.5");
    /// # Ok::<(), typolish::Error>(())
    /// ```
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

    /// Sets the decimal separator: a visible character of its own that is not a letter, a digit,
    /// whitespace, a quote character or a character with the Unicode `Dash` property (design
    /// section 7.6).
    ///
    /// ```
    /// use typolish::{Language, LanguageData, NumberData, Output, RuleSet, Typographer};
    ///
    /// static XX: LanguageData =
    ///     LanguageData::new("xx").with_numbers(NumberData::new().with_decimal(','));
    ///
    /// let t = Typographer::new(Language::from_data(&XX)?, RuleSet::EMPTY);
    /// assert_eq!(t.format_number(2.5, Output::Unicode).to_string(), "2,5");
    /// # Ok::<(), typolish::Error>(())
    /// ```
    #[must_use]
    pub const fn with_decimal(mut self, decimal: char) -> NumberData {
        self.decimal = decimal;
        self
    }

    /// Sets the group separator: U+00A0, U+202F or a visible character of its own that is not a
    /// letter, a digit, whitespace or a character with the Unicode `Dash` property (design section
    /// 7.6).
    ///
    /// ```
    /// use typolish::{Language, LanguageData, NumberData, Output, RuleSet, Typographer};
    ///
    /// const NUMBERS: NumberData = NumberData::new().with_group(',').with_min_grouping_digits(1);
    /// static XX: LanguageData = LanguageData::new("xx").with_numbers(NUMBERS);
    ///
    /// let t = Typographer::new(Language::from_data(&XX)?, RuleSet::EMPTY);
    /// assert_eq!(t.format_number(1234567, Output::Unicode).to_string(), "1,234,567");
    /// # Ok::<(), typolish::Error>(())
    /// ```
    #[must_use]
    pub const fn with_group(mut self, group: char) -> NumberData {
        self.group = group;
        self
    }

    /// Sets the minimum grouping digits: 1 to 4 in the CLDR meaning (group from `3 + n` digits),
    /// 0 never groups.
    ///
    /// ```
    /// use typolish::{Language, LanguageData, NumberData, Output, RuleSet, Typographer};
    ///
    /// static XX: LanguageData =
    ///     LanguageData::new("xx").with_numbers(NumberData::new().with_min_grouping_digits(2));
    ///
    /// let t = Typographer::new(Language::from_data(&XX)?, RuleSet::EMPTY);
    /// assert_eq!(t.format_number(1234, Output::Unicode).to_string(), "1234");
    /// assert_eq!(t.format_number(12345, Output::Unicode).to_string(), "12\u{a0}345");
    /// # Ok::<(), typolish::Error>(())
    /// ```
    #[must_use]
    pub const fn with_min_grouping_digits(mut self, digits: u8) -> NumberData {
        self.min_grouping_digits = digits;
        self
    }

    /// Sets the minus sign: `-` or a dash, U+2010 to U+2015 or U+2212 (design section 7.6).
    ///
    /// ```
    /// use typolish::{Language, LanguageData, NumberData, Output, RuleSet, Typographer};
    ///
    /// static XX: LanguageData =
    ///     LanguageData::new("xx").with_numbers(NumberData::new().with_minus('-'));
    ///
    /// let t = Typographer::new(Language::from_data(&XX)?, RuleSet::EMPTY);
    /// assert_eq!(t.format_number(-5, Output::Unicode).to_string(), "-5");
    /// # Ok::<(), typolish::Error>(())
    /// ```
    #[must_use]
    pub const fn with_minus(mut self, minus: char) -> NumberData {
        self.minus = minus;
        self
    }

    /// Sets whether the CLDR percent pattern has a space before `%`.
    ///
    /// ```
    /// use typolish::{
    ///     Language, LanguageData, NumberData, Output, PercentSpacing, Rule, RuleSet, Typographer,
    /// };
    ///
    /// static XX: LanguageData = LanguageData::EN_US
    ///     .with_tag("en-GB")
    ///     .with_numbers(NumberData::new().with_percent_spaced(true));
    ///
    /// let t = Typographer::builder(
    ///     Language::from_data(&XX)?,
    ///     RuleSet::EMPTY.with(Rule::NbspUnits),
    /// )
    /// .percent(PercentSpacing::Language)
    /// .build()?;
    /// assert_eq!(t.typeset("5%", Output::Unicode), "5\u{a0}%");
    /// # Ok::<(), typolish::Error>(())
    /// ```
    #[must_use]
    pub const fn with_percent_spaced(mut self, spaced: bool) -> NumberData {
        self.percent_spaced = spaced;
        self
    }

    /// Sets the dash of the CLDR range pattern.
    ///
    /// The dash must be `–` U+2013 or `—` U+2014 (design section 7.6): the range rules write only a
    /// dash that a second pass and `to_plain` read back.
    ///
    /// ```
    /// use typolish::{Language, LanguageData, NumberData, Output, RuleSet, Typographer};
    ///
    /// static XX: LanguageData =
    ///     LanguageData::new("xx").with_numbers(NumberData::new().with_range_dash('\u{2014}'));
    ///
    /// let t = Typographer::new(Language::from_data(&XX)?, RuleSet::EMPTY);
    /// assert_eq!(t.format_range(1, 5, Output::Unicode).to_string(), "1—5");
    /// # Ok::<(), typolish::Error>(())
    /// ```
    #[must_use]
    pub const fn with_range_dash(mut self, dash: char) -> NumberData {
        self.range_dash = dash;
        self
    }

    /// Sets whether the CLDR range pattern has spaces around the dash.
    ///
    /// ```
    /// use typolish::{Language, LanguageData, NumberData, Output, RuleSet, Typographer};
    ///
    /// static XX: LanguageData =
    ///     LanguageData::new("xx").with_numbers(NumberData::new().with_range_spaced(true));
    ///
    /// let t = Typographer::new(Language::from_data(&XX)?, RuleSet::EMPTY);
    /// assert_eq!(t.format_range(1, 5, Output::Unicode).to_string(), "1\u{a0}–\u{a0}5");
    /// # Ok::<(), typolish::Error>(())
    /// ```
    #[must_use]
    pub const fn with_range_spaced(mut self, spaced: bool) -> NumberData {
        self.range_spaced = spaced;
        self
    }

    /// Sets the symbol of not-a-number.
    ///
    /// Validation (design section 7.6) requires a non-empty symbol without digits (no character
    /// with `char::is_numeric`), control, format or other default-ignorable characters, without
    /// whitespace at either end and without a mark at an end that joins the text next to it:
    /// `format_number` writes it as it is, and it must show.
    ///
    /// ```
    /// use typolish::{Language, LanguageData, NumberData, Output, RuleSet, Typographer};
    ///
    /// static XX: LanguageData =
    ///     LanguageData::new("xx").with_numbers(NumberData::new().with_nan("не число"));
    ///
    /// let t = Typographer::new(Language::from_data(&XX)?, RuleSet::EMPTY);
    /// assert_eq!(
    ///     t.format_number(f64::NAN, Output::Unicode).to_string(),
    ///     "не число"
    /// );
    /// # Ok::<(), typolish::Error>(())
    /// ```
    #[must_use]
    pub const fn with_nan(mut self, nan: &'static str) -> NumberData {
        self.nan = nan;
        self
    }

    /// Sets the symbol of infinity.
    ///
    /// Validation (design section 7.6) requires a non-empty symbol without digits (no character
    /// with `char::is_numeric`), control, format or other default-ignorable characters, without
    /// whitespace at either end and without a mark at an end that joins the text next to it:
    /// `format_number` writes it as it is, and it must show.
    ///
    /// ```
    /// use typolish::{Language, LanguageData, NumberData, Output, RuleSet, Typographer};
    ///
    /// static XX: LanguageData =
    ///     LanguageData::new("xx").with_numbers(NumberData::new().with_infinity("inf"));
    ///
    /// let t = Typographer::new(Language::from_data(&XX)?, RuleSet::EMPTY);
    /// assert_eq!(
    ///     t.format_number(f64::NEG_INFINITY, Output::Unicode)
    ///         .to_string(),
    ///     "\u{2212}inf"
    /// );
    /// # Ok::<(), typolish::Error>(())
    /// ```
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
/// const EN_GB: LanguageData = LanguageData::EN_US
///     .with_tag("en-GB")
///     .with_dash_style(DashStyle::Spaced);
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
    ///
    /// ```
    /// use typolish::{Language, LanguageData};
    ///
    /// assert_eq!(LanguageData::RU_RU.tag(), "ru-RU");
    /// assert_eq!(Language::from_static(&LanguageData::RU_RU), Language::RU_RU);
    /// ```
    pub const RU_RU: LanguageData = super::ru_ru::RU_RU;
    /// English (United States), design section 7.3.
    ///
    /// ```
    /// use typolish::{Language, LanguageData};
    ///
    /// assert_eq!(LanguageData::EN_US.tag(), "en-US");
    /// assert_eq!(Language::from_static(&LanguageData::EN_US), Language::EN_US);
    /// ```
    pub const EN_US: LanguageData = super::en_us::EN_US;
    /// No language: the values of design section 7.4, tag `und`.
    ///
    /// ```
    /// use typolish::{Language, LanguageData};
    ///
    /// assert_eq!(LanguageData::COMMON.tag(), "und");
    /// assert_eq!(Language::from_static(&LanguageData::COMMON), Language::COMMON);
    /// ```
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
    ///
    /// ```
    /// use typolish::{Language, LanguageData};
    ///
    /// static EN_GB: LanguageData = LanguageData::EN_US.with_tag("en-GB");
    /// static BAD: LanguageData = LanguageData::EN_US.with_tag("english");
    ///
    /// assert_eq!(Language::from_data(&EN_GB)?.tag(), "en-GB");
    /// assert!(Language::from_data(&BAD).is_err());
    /// # Ok::<(), typolish::Error>(())
    /// ```
    #[must_use]
    pub const fn with_tag(mut self, tag: &'static str) -> LanguageData {
        self.tag = tag;
        self
    }

    /// Sets the CLDR version the values were checked against.
    ///
    /// ```
    /// use typolish::{Language, LanguageData};
    ///
    /// static EN_GB: LanguageData = LanguageData::EN_US.with_tag("en-GB").with_cldr_version("49");
    ///
    /// assert_eq!(Language::from_data(&EN_GB)?.tag(), "en-GB");
    /// # Ok::<(), typolish::Error>(())
    /// ```
    #[must_use]
    pub const fn with_cldr_version(mut self, version: &'static str) -> LanguageData {
        self.cldr_version = version;
        self
    }

    /// Sets the script of the language's words.
    ///
    /// ```
    /// use typolish::{Language, LanguageData, Output, Rule, RuleSet, Script, Typographer};
    ///
    /// static LATIN: LanguageData = LanguageData::new("xx")
    ///     .with_short_words(&["a"])
    ///     .with_script(Script::Latin);
    /// static GREEK: LanguageData = LanguageData::new("xx")
    ///     .with_short_words(&["a"])
    ///     .with_script(Script::Greek);
    ///
    /// let rules = RuleSet::EMPTY.with(Rule::NbspShortWords);
    /// let latin = Typographer::new(Language::from_data(&LATIN)?, rules);
    /// let greek = Typographer::new(Language::from_data(&GREEK)?, rules);
    /// assert_eq!(latin.typeset("a cat", Output::Unicode), "a\u{a0}cat");
    /// // The short words are filtered by the script of the language.
    /// assert_eq!(greek.typeset("a cat", Output::Unicode), "a cat");
    /// # Ok::<(), typolish::Error>(())
    /// ```
    #[must_use]
    pub const fn with_script(mut self, script: Script) -> LanguageData {
        self.script = script;
        self
    }

    /// Sets the quote pairs; `None` makes `quotes.pairs` inert.
    ///
    /// ```
    /// use typolish::{Language, LanguageData, Rule};
    ///
    /// static XX: LanguageData = LanguageData::EN_US.with_tag("en-GB").with_quotes(None);
    ///
    /// assert!(Language::EN_US.supports(Rule::QuotesPairs));
    /// assert!(!Language::from_data(&XX)?.supports(Rule::QuotesPairs));
    /// # Ok::<(), typolish::Error>(())
    /// ```
    #[must_use]
    pub const fn with_quotes(mut self, quotes: Option<QuotePairs>) -> LanguageData {
        self.quotes = quotes;
        self
    }

    /// Sets the elisions (`'cause`): sorted by code point, lowercase.
    ///
    /// ```
    /// use typolish::{Language, LanguageData, Output, Rule, RuleSet, Typographer};
    ///
    /// static XX: LanguageData = LanguageData::EN_US
    ///     .with_tag("en-GB")
    ///     .with_elisions(&["'cause"]);
    ///
    /// let t = Typographer::new(
    ///     Language::from_data(&XX)?,
    ///     RuleSet::EMPTY.with(Rule::QuotesApostrophe),
    /// );
    /// assert_eq!(
    ///     t.typeset("'cause I said so", Output::Unicode),
    ///     "’cause I said so"
    /// );
    /// # Ok::<(), typolish::Error>(())
    /// ```
    #[must_use]
    pub const fn with_elisions(mut self, elisions: &'static [&'static str]) -> LanguageData {
        self.elisions = elisions;
        self
    }

    /// Sets whether `'90s` takes an apostrophe.
    ///
    /// ```
    /// use typolish::{Language, LanguageData, Output, Rule, RuleSet, Typographer};
    ///
    /// static XX: LanguageData = LanguageData::EN_US
    ///     .with_tag("en-GB")
    ///     .with_decade_apostrophe(false);
    ///
    /// let rules = RuleSet::EMPTY.with(Rule::QuotesApostrophe);
    /// let en = Typographer::new(Language::EN_US, rules);
    /// let xx = Typographer::new(Language::from_data(&XX)?, rules);
    /// assert_eq!(en.typeset("the '90s", Output::Unicode), "the ’90s");
    /// assert_eq!(xx.typeset("the '90s", Output::Unicode), "the '90s");
    /// # Ok::<(), typolish::Error>(())
    /// ```
    #[must_use]
    pub const fn with_decade_apostrophe(mut self, on: bool) -> LanguageData {
        self.decade_apostrophe = on;
        self
    }

    /// Sets whether a trailing possessive (`dogs'`) takes an apostrophe.
    ///
    /// ```
    /// use typolish::{Language, LanguageData, Output, Rule, RuleSet, Typographer};
    ///
    /// static XX: LanguageData = LanguageData::EN_US
    ///     .with_tag("en-GB")
    ///     .with_trailing_possessive(false);
    ///
    /// let rules = RuleSet::EMPTY.with(Rule::QuotesApostrophe);
    /// let en = Typographer::new(Language::EN_US, rules);
    /// let xx = Typographer::new(Language::from_data(&XX)?, rules);
    /// assert_eq!(
    ///     en.typeset("the dogs' toys", Output::Unicode),
    ///     "the dogs’ toys"
    /// );
    /// assert_eq!(
    ///     xx.typeset("the dogs' toys", Output::Unicode),
    ///     "the dogs' toys"
    /// );
    /// # Ok::<(), typolish::Error>(())
    /// ```
    #[must_use]
    pub const fn with_trailing_possessive(mut self, on: bool) -> LanguageData {
        self.trailing_possessive = on;
        self
    }

    /// Sets the dash data; `None` makes the `dash.*` rules and `nbsp.dash` inert.
    ///
    /// ```
    /// use typolish::{Language, LanguageData, Rule};
    ///
    /// static XX: LanguageData = LanguageData::EN_US.with_tag("en-GB").with_dash(None);
    ///
    /// assert!(!Language::from_data(&XX)?.supports(Rule::DashSpacedHyphen));
    /// # Ok::<(), typolish::Error>(())
    /// ```
    #[must_use]
    pub const fn with_dash(mut self, dash: Option<DashData>) -> LanguageData {
        self.dash = dash;
        self
    }

    /// Sets the dash style; on a table whose dash data is `None` it makes it
    /// `Some(DashData::new(style, false))`.
    ///
    /// ```
    /// use typolish::{DashStyle, Language, LanguageData, Output, Rule, RuleSet, Typographer};
    ///
    /// static EN_GB: LanguageData = LanguageData::EN_US
    ///     .with_tag("en-GB")
    ///     .with_dash_style(DashStyle::Spaced);
    ///
    /// let rules = RuleSet::EMPTY.with(Rule::DashSpacedHyphen);
    /// let us = Typographer::new(Language::EN_US, rules);
    /// let gb = Typographer::new(Language::from_data(&EN_GB)?, rules);
    /// assert_eq!(
    ///     us.typeset("Moscow - the capital", Output::Unicode),
    ///     "Moscow—the capital"
    /// );
    /// assert_eq!(
    ///     gb.typeset("Moscow - the capital", Output::Unicode),
    ///     "Moscow — the capital"
    /// );
    /// # Ok::<(), typolish::Error>(())
    /// ```
    #[must_use]
    pub const fn with_dash_style(mut self, style: DashStyle) -> LanguageData {
        self.dash = match self.dash {
            Some(dash) => Some(DashData::new(style, dash.dialogue)),
            None => Some(DashData::new(style, false)),
        };
        self
    }

    /// Sets the ellipsis forms after `?` and `!`.
    ///
    /// ```
    /// use typolish::{EllipsisForms, Language, LanguageData, Output, Rule, RuleSet, Typographer};
    ///
    /// static XX: LanguageData = LanguageData::EN_US
    ///     .with_tag("en-GB")
    ///     .with_ellipsis(EllipsisForms::new("?..", "!..", "?!.."));
    ///
    /// let t = Typographer::new(
    ///     Language::from_data(&XX)?,
    ///     RuleSet::EMPTY.with(Rule::EllipsisDots),
    /// );
    /// assert_eq!(t.typeset("Stop!...", Output::Unicode), "Stop!..");
    /// # Ok::<(), typolish::Error>(())
    /// ```
    #[must_use]
    pub const fn with_ellipsis(mut self, ellipsis: EllipsisForms) -> LanguageData {
        self.ellipsis = ellipsis;
        self
    }

    /// Sets the short words: sorted by code point, lowercase.
    ///
    /// ```
    /// use typolish::{Language, LanguageData, Output, Rule, RuleSet, Script, Typographer};
    ///
    /// static XX: LanguageData = LanguageData::new("xx")
    ///     .with_script(Script::Latin)
    ///     .with_short_words(&["a", "of"]);
    ///
    /// let t = Typographer::new(
    ///     Language::from_data(&XX)?,
    ///     RuleSet::EMPTY.with(Rule::NbspShortWords),
    /// );
    /// assert_eq!(
    ///     t.typeset("a cup of tea", Output::Unicode),
    ///     "a\u{a0}cup of\u{a0}tea"
    /// );
    /// # Ok::<(), typolish::Error>(())
    /// ```
    #[must_use]
    pub const fn with_short_words(mut self, words: &'static [&'static str]) -> LanguageData {
        self.short_words = words;
        self
    }

    /// Sets the length up to which any word of the language's script is short; 0 makes
    /// `nbsp.short-length` inert.
    ///
    /// ```
    /// use typolish::{Language, LanguageData, Output, Rule, RuleSet, Script, Typographer};
    ///
    /// static XX: LanguageData = LanguageData::new("xx")
    ///     .with_script(Script::Latin)
    ///     .with_short_word_length(2);
    ///
    /// let t = Typographer::new(
    ///     Language::from_data(&XX)?,
    ///     RuleSet::EMPTY.with(Rule::NbspShortLength),
    /// );
    /// assert_eq!(
    ///     t.typeset("an owl and me", Output::Unicode),
    ///     "an\u{a0}owl and me"
    /// );
    /// # Ok::<(), typolish::Error>(())
    /// ```
    #[must_use]
    pub const fn with_short_word_length(mut self, max_chars: u8) -> LanguageData {
        self.short_word_length = max_chars;
        self
    }

    /// Sets the particles: sorted by code point, lowercase.
    ///
    /// ```
    /// use typolish::{Language, LanguageData, Output, Rule, RuleSet, Typographer};
    ///
    /// static XX: LanguageData = LanguageData::RU_RU
    ///     .with_tag("ru-KZ")
    ///     .with_particles(&["ли"]);
    ///
    /// let t = Typographer::new(
    ///     Language::from_data(&XX)?,
    ///     RuleSet::EMPTY.with(Rule::NbspParticles),
    /// );
    /// assert_eq!(
    ///     t.typeset("Знаешь ли ты", Output::Unicode),
    ///     "Знаешь\u{a0}ли ты"
    /// );
    /// # Ok::<(), typolish::Error>(())
    /// ```
    #[must_use]
    pub const fn with_particles(mut self, particles: &'static [&'static str]) -> LanguageData {
        self.particles = particles;
        self
    }

    /// Sets the abbreviations: sorted by their parts, as printed.
    ///
    /// ```
    /// use typolish::{
    ///     Abbreviation, Bind, Language, LanguageData, Output, Rule, RuleSet, Typographer,
    /// };
    ///
    /// static XX: LanguageData = LanguageData::RU_RU
    ///     .with_tag("ru-KZ")
    ///     .with_abbreviations(&[Abbreviation::new(&["т.", "е."], Bind::Inner)]);
    ///
    /// let t = Typographer::new(
    ///     Language::from_data(&XX)?,
    ///     RuleSet::EMPTY.with(Rule::NbspAbbreviations),
    /// );
    /// assert_eq!(t.typeset("т. е. так", Output::Unicode), "т.\u{a0}е. так");
    /// # Ok::<(), typolish::Error>(())
    /// ```
    #[must_use]
    pub const fn with_abbreviations(
        mut self,
        abbreviations: &'static [Abbreviation],
    ) -> LanguageData {
        self.abbreviations = abbreviations;
        self
    }

    /// Sets the dotted words that are not technical (`e.g.`): sorted by code point, lowercase.
    ///
    /// ```
    /// use typolish::{Language, LanguageData, Output, RuleSet, Typographer};
    ///
    /// static XX: LanguageData = LanguageData::EN_US
    ///     .with_tag("en-GB")
    ///     .with_atom_exceptions(&["ph.d"]);
    ///
    /// // Without the exception `Ph.D` is a dotted technical fragment, like a host name.
    /// let us = Typographer::new(Language::EN_US, RuleSet::PROSE);
    /// let xx = Typographer::new(Language::from_data(&XX)?, RuleSet::PROSE);
    /// assert_eq!(
    ///     us.report("a Ph.D student", Output::Unicode)
    ///         .protected()
    ///         .len(),
    ///     1
    /// );
    /// assert!(
    ///     xx.report("a Ph.D student", Output::Unicode)
    ///         .protected()
    ///         .is_empty()
    /// );
    /// # Ok::<(), typolish::Error>(())
    /// ```
    #[must_use]
    pub const fn with_atom_exceptions(mut self, words: &'static [&'static str]) -> LanguageData {
        self.atom_exceptions = words;
        self
    }

    /// Sets the units: sorted by code point, compared with case.
    ///
    /// ```
    /// use typolish::{Language, LanguageData, Output, Rule, RuleSet, Typographer};
    ///
    /// static XX: LanguageData = LanguageData::new("xx").with_units(&["kB"]);
    ///
    /// let t = Typographer::new(Language::from_data(&XX)?, RuleSet::EMPTY.with(Rule::NbspUnits));
    /// assert_eq!(t.typeset("5 kB", Output::Unicode), "5\u{a0}kB");
    /// // Units are compared with case.
    /// assert_eq!(t.typeset("5 KB", Output::Unicode), "5 KB");
    /// # Ok::<(), typolish::Error>(())
    /// ```
    #[must_use]
    pub const fn with_units(mut self, units: &'static [&'static str]) -> LanguageData {
        self.units = units;
        self
    }

    /// Sets the units that are also words: sorted by code point, each also in the units.
    ///
    /// ```
    /// use typolish::{Language, LanguageData};
    ///
    /// static OK: LanguageData = LanguageData::new("xx")
    ///     .with_units(&["in"])
    ///     .with_ambiguous_units(&["in"]);
    /// static BAD: LanguageData = LanguageData::new("xx").with_ambiguous_units(&["in"]);
    ///
    /// assert!(Language::from_data(&OK).is_ok());
    /// // An ambiguous unit must also be a unit.
    /// assert!(Language::from_data(&BAD).is_err());
    /// ```
    #[must_use]
    pub const fn with_ambiguous_units(mut self, units: &'static [&'static str]) -> LanguageData {
        self.ambiguous_units = units;
        self
    }

    /// Sets the month forms used next to a day number: sorted by code point, as printed.
    ///
    /// ```
    /// use typolish::{Language, LanguageData, Output, Rule, RuleSet, Typographer};
    ///
    /// static XX: LanguageData = LanguageData::new("xx").with_months(&["Mai"]);
    ///
    /// let t = Typographer::new(Language::from_data(&XX)?, RuleSet::EMPTY.with(Rule::NbspDates));
    /// assert_eq!(t.typeset("am 21 Mai", Output::Unicode), "am 21\u{a0}Mai");
    /// # Ok::<(), typolish::Error>(())
    /// ```
    #[must_use]
    pub const fn with_months(mut self, months: &'static [&'static str]) -> LanguageData {
        self.months = months;
        self
    }

    /// Sets where a month name stands relative to the day number.
    ///
    /// ```
    /// use typolish::{Language, LanguageData, MonthPosition, Output, Rule, RuleSet, Typographer};
    ///
    /// static XX: LanguageData = LanguageData::new("xx")
    ///     .with_months(&["May"])
    ///     .with_month_position(MonthPosition::BeforeDay);
    ///
    /// let t = Typographer::new(
    ///     Language::from_data(&XX)?,
    ///     RuleSet::EMPTY.with(Rule::NbspDates),
    /// );
    /// assert_eq!(t.typeset("on May 21", Output::Unicode), "on May\u{a0}21");
    /// # Ok::<(), typolish::Error>(())
    /// ```
    #[must_use]
    pub const fn with_month_position(mut self, position: MonthPosition) -> LanguageData {
        self.month_position = position;
        self
    }

    /// Sets the words after a year: sorted by code point, lowercase.
    ///
    /// ```
    /// use typolish::{Language, LanguageData, Output, Rule, RuleSet, Typographer};
    ///
    /// static XX: LanguageData = LanguageData::new("xx").with_year_words(&["r."]);
    ///
    /// let t = Typographer::new(Language::from_data(&XX)?, RuleSet::EMPTY.with(Rule::NbspDates));
    /// assert_eq!(t.typeset("w 2026 r.", Output::Unicode), "w 2026\u{a0}r.");
    /// # Ok::<(), typolish::Error>(())
    /// ```
    #[must_use]
    pub const fn with_year_words(mut self, words: &'static [&'static str]) -> LanguageData {
        self.year_words = words;
        self
    }

    /// Sets the words after a century: sorted by code point, lowercase.
    ///
    /// ```
    /// use typolish::{Language, LanguageData, Output, Rule, RuleSet, Typographer};
    ///
    /// static XX: LanguageData = LanguageData::new("xx").with_century_words(&["w."]);
    ///
    /// let t = Typographer::new(Language::from_data(&XX)?, RuleSet::EMPTY.with(Rule::NbspDates));
    /// assert_eq!(t.typeset("XX w.", Output::Unicode), "XX\u{a0}w.");
    /// # Ok::<(), typolish::Error>(())
    /// ```
    #[must_use]
    pub const fn with_century_words(mut self, words: &'static [&'static str]) -> LanguageData {
        self.century_words = words;
        self
    }

    /// Sets the words before a range (`стр.`, `pp.`): sorted by code point, lowercase.
    ///
    /// ```
    /// use typolish::{Language, LanguageData, Output, Rule, RuleSet, Typographer};
    ///
    /// static XX: LanguageData = LanguageData::new("xx").with_range_lead_words(&["s."]);
    ///
    /// let t = Typographer::new(
    ///     Language::from_data(&XX)?,
    ///     RuleSet::EMPTY.with(Rule::RangeContext),
    /// );
    /// assert_eq!(t.typeset("s. 10-20", Output::Unicode), "s. 10–20");
    /// # Ok::<(), typolish::Error>(())
    /// ```
    #[must_use]
    pub const fn with_range_lead_words(mut self, words: &'static [&'static str]) -> LanguageData {
        self.range_lead_words = words;
        self
    }

    /// Sets the signs that name a number (`№`, `§`, `¶`): sorted by code point, each a
    /// punctuation character of the lexer (`#`, `№`, `§`, `¶`, …; design sections 3.5 and 7.6)
    /// other than `%` and `‰`, and none equal to a unit. Two rules would read such a sign: the
    /// percent rule and `nbsp.signs` disagree on a second pass, and a sign that is also a unit is
    /// glued both to the number before it and to the number after it (`5⍽$⍽1`).
    ///
    /// ```
    /// use typolish::{Language, LanguageData, Output, Rule, RuleSet, Typographer};
    ///
    /// static XX: LanguageData = LanguageData::new("xx").with_signs(&['#']);
    ///
    /// let t = Typographer::new(Language::from_data(&XX)?, RuleSet::EMPTY.with(Rule::NbspSigns));
    /// assert_eq!(t.typeset("# 5", Output::Unicode), "#\u{a0}5");
    /// # Ok::<(), typolish::Error>(())
    /// ```
    #[must_use]
    pub const fn with_signs(mut self, signs: &'static [char]) -> LanguageData {
        self.signs = signs;
        self
    }

    /// Sets the number symbols and patterns.
    ///
    /// ```
    /// use typolish::{Language, LanguageData, NumberData, Output, RuleSet, Typographer};
    ///
    /// const NUMBERS: NumberData = NumberData::new()
    ///     .with_decimal(',')
    ///     .with_group('.')
    ///     .with_min_grouping_digits(1);
    /// static DE: LanguageData = LanguageData::EN_US.with_tag("de-DE").with_numbers(NUMBERS);
    ///
    /// let t = Typographer::new(Language::from_data(&DE)?, RuleSet::EMPTY);
    /// assert_eq!(
    ///     t.format_number(1234.5, Output::Unicode).to_string(),
    ///     "1.234,5"
    /// );
    /// # Ok::<(), typolish::Error>(())
    /// ```
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
    #[inline]
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

    /// The indices of the entries whose first part starts with the characters of `prefix`. The
    /// entries are sorted by their parts (design section 7.6), so they form one range, narrowed by
    /// one binary search per character; reading stops when the range is empty.
    pub(crate) fn with_prefix(self, prefix: impl Iterator<Item = char>) -> core::ops::Range<usize> {
        match self {
            Abbreviations::Static(list) => prefix_range(
                list.len(),
                |i| list.get(i).and_then(|a| a.parts.first().copied()),
                prefix,
            ),
            Abbreviations::Owned(list) => prefix_range(
                list.len(),
                |i| list.get(i).and_then(|a| a.parts.first()).map(|p| &**p),
                prefix,
            ),
        }
    }
}

/// The indices among `len` entries sorted by their first part (`first`) whose first part starts
/// with the characters of `prefix`, as `Abbreviations::with_prefix` describes.
fn prefix_range<'l>(
    len: usize,
    first: impl Fn(usize) -> Option<&'l str>,
    prefix: impl Iterator<Item = char>,
) -> core::ops::Range<usize> {
    let (mut lo, mut hi) = (0, len);
    // Every entry in `lo..hi` has a first part that starts with the first `bytes` bytes of
    // `prefix`.
    let mut bytes = 0;
    for c in prefix {
        let next = |i: usize| {
            first(i)
                .and_then(|part| part.get(bytes..))
                .and_then(|rest| rest.chars().next())
        };
        lo = partition(lo, hi, |i| next(i) < Some(c));
        hi = partition(lo, hi, |i| next(i) <= Some(c));
        if lo >= hi {
            return lo..lo;
        }
        bytes += c.len_utf8();
    }
    lo..hi
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
    #[inline]
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
    /// design section 7.6 lowercased; every list sorted by code point and free of duplicates.
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
            .any(|c| !crate::engine::lex::is_punct_char(*c))
        {
            return Err("signs: a sign must be a punctuation character of design section 3.5");
        }
        // A sign that is also a percent sign or a unit is read by two rules of design section 8.7:
        // with a percent sign they disagree on a second pass, and a unit is glued to the numbers on
        // both sides.
        if self.signs.iter().any(|c| {
            matches!(c, '%' | '\u{2030}') || self.units.iter().any(|unit| unit.chars().eq([*c]))
        }) {
            return Err("signs: a sign must not be `%`, `\u{2030}` or equal to a unit");
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
                "numbers: the group separator must be U+00A0, U+202F or a visible character of its own, not a letter, a digit, whitespace or a dash",
            );
        }
        if !is_valid_minus(n.minus) {
            return Err("numbers: the minus must be `-` or a dash, U+2010 to U+2015 or U+2212");
        }
        if !matches!(n.range_dash, '\u{2013}' | '\u{2014}') {
            return Err("numbers: the range dash must be U+2013 or U+2014");
        }
        if !is_valid_decimal(n.decimal) {
            return Err(
                "numbers: the decimal separator must be a visible character of its own, not a letter, a digit, whitespace, a quote character or a dash",
            );
        }
        let e = &self.ellipsis;
        if [e.question, e.exclamation, e.both].iter().any(|form| {
            form.is_empty() || !form.chars().all(|c| matches!(c, '?' | '!' | '.' | '…'))
        }) {
            return Err("ellipsis: forms must be non-empty and consist of ?, !, . and … only");
        }
        // The rule keeps the leading signs in their own tokens and writes the rest in place of the
        // dots, so the rest must be dots only (a sign there would read as a new ellipsis on a
        // second pass) and must not end in exactly three full stops after `…` (they would read as
        // an ellipsis of their own), and the leading signs must be the form's own.
        let fits = |form: &str, lead: &[&str]| {
            let tail = form.trim_start_matches(['?', '!']);
            !tail.is_empty()
                && tail.chars().all(|c| matches!(c, '.' | '…'))
                && !tail.ends_with("…...")
                && form
                    .strip_suffix(tail)
                    .is_some_and(|head| lead.contains(&head))
        };
        if !fits(e.question, &["", "?"])
            || !fits(e.exclamation, &["", "!"])
            || !fits(e.both, &["", "?!", "!?"])
        {
            return Err(
                "ellipsis: a form is its own signs (none, `?`, `!`, `?!` or `!?`), then only `.` and `…`, not ending in `…` and three full stops",
            );
        }
        // `format_number` writes these symbols as they are: each must show, and must not read as a
        // number. An invisible character (a default-ignorable code point, such as U+3164 HANGUL
        // FILLER) would print nothing, and a mark at an end (U+0300) would join the text next to
        // the symbol instead of showing on its own.
        let shows = |symbol: &str| {
            !symbol.is_empty()
                && symbol.trim() == symbol
                && !symbol.chars().any(|c| {
                    c.is_numeric()
                        || c.is_control()
                        || is_format_category(c)
                        || is_reserved_default_ignorable(c)
                        || is_assigned_default_ignorable(c)
                })
                && stands_alone(symbol)
        };
        if !shows(n.nan) || !shows(n.infinity) {
            return Err(
                "numbers: nan and infinity must be non-empty, without digits, control, format or default-ignorable characters, without whitespace at the ends and without a mark that joins the text next to them",
            );
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
    /// All the entries of check 0 of the classifier (design section 3.6): the four lists below.
    Dictionary,
    /// The units, the builder's additions included. The only list compared with case alone, so
    /// its signature is not folded.
    Units,
    /// The month forms.
    Months,
    /// Every abbreviation part or run of parts that check 0 matches written together.
    Abbreviations,
    /// The atom exceptions and the elisions.
    Exceptions,
    /// The short words, the builder's additions included.
    ShortWords,
    /// The particles, the builder's additions included.
    Particles,
}

/// What the signature of a key reads: its length in characters and its first and last characters.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Shape {
    len: u64,
    first: Option<char>,
    last: Option<char>,
}

impl Shape {
    /// The shape of the characters `chars`.
    pub(crate) fn of(mut chars: impl Iterator<Item = char>) -> Shape {
        let first = chars.next();
        let mut shape = Shape {
            len: u64::from(first.is_some()),
            first,
            last: first,
        };
        for c in chars {
            shape.last = Some(c);
            shape.len += 1;
        }
        shape
    }

    /// The shape of the characters `chars`, unless there are more than `most` of them: the
    /// characters after those are not read.
    pub(crate) fn of_at_most(mut chars: impl Iterator<Item = char>, most: u64) -> Option<Shape> {
        let first = chars.next();
        let mut shape = Shape {
            len: u64::from(first.is_some()),
            first,
            last: first,
        };
        for c in chars {
            if shape.len >= most {
                return None;
            }
            shape.last = Some(c);
            shape.len += 1;
        }
        (shape.len <= most).then_some(shape)
    }

    /// The shape of the key with `c` appended.
    pub(crate) fn then(self, c: char) -> Shape {
        Shape {
            len: self.len + 1,
            first: self.first.or(Some(c)),
            last: Some(c),
        }
    }

    /// The shape of the key with `c` put before it.
    pub(crate) fn after(self, c: char) -> Shape {
        Shape {
            len: self.len + 1,
            first: Some(c),
            last: self.last.or(Some(c)),
        }
    }
}

/// The signatures of the entries of some word lists of a typographer: a Bloom filter of 4096
/// bits, two per entry, over the list, the length in characters and the first and last characters
/// of every entry, folded except for `WordList::Units`. Every comparison of design section 7.1
/// (with case, with the first letter folded, or without case) keeps the signature, because
/// folding is idempotent, so a key whose signature is absent equals no entry of the list and its
/// lookups can be skipped. A present signature proves nothing. Computed once per typographer,
/// with the active rules, without allocation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Signatures {
    bits: [u64; 64],
    /// The length in characters of the longest entry of each list: a longer key is no entry.
    longest: [u64; 7],
}

impl Signatures {
    /// No signature.
    pub(crate) const NONE: Signatures = Signatures {
        bits: [0; 64],
        longest: [0; 7],
    };

    /// The length in characters of the longest entry of `list`.
    pub(crate) fn longest(&self, list: WordList) -> u64 {
        self.longest.get(list as usize).copied().unwrap_or(u64::MAX)
    }

    /// The two bits of the signature of a key of this shape in `list`.
    fn bits(list: WordList, shape: Shape) -> [usize; 2] {
        let Shape { len, first, last } = shape;
        let fold = list != WordList::Units;
        let code = |c: Option<char>| {
            u64::from(c.map_or(0, |c| u32::from(if fold { fold_char(c) } else { c })))
        };
        let mut h = len ^ (code(first) << 8) ^ (code(last) << 32) ^ ((list as u64) << 60);
        // The finalizer of SplitMix64.
        h = (h ^ (h >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
        h = (h ^ (h >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
        h ^= h >> 31;
        [(h & 0xfff) as usize, ((h >> 12) & 0xfff) as usize]
    }

    /// Adds the signature of an entry of `list`.
    pub(crate) fn add(&mut self, list: WordList, chars: impl Iterator<Item = char>) {
        let shape = Shape::of(chars);
        if let Some(longest) = self.longest.get_mut(list as usize) {
            *longest = (*longest).max(shape.len);
        }
        for bit in Signatures::bits(list, shape) {
            if let Some(word) = self.bits.get_mut(bit / 64) {
                *word |= 1 << (bit % 64);
            }
        }
    }

    /// Whether some entry of `list` may have the characters `chars`; `false` means that none has.
    pub(crate) fn may_hold(&self, list: WordList, chars: impl Iterator<Item = char>) -> bool {
        Shape::of_at_most(chars, self.longest(list))
            .is_some_and(|shape| self.may_hold_shape(list, shape))
    }

    /// `may_hold` for a key of this shape.
    pub(crate) fn may_hold_shape(&self, list: WordList, shape: Shape) -> bool {
        shape.len <= self.longest(list)
            && Signatures::bits(list, shape).iter().all(|&bit| {
                self.bits
                    .get(bit / 64)
                    .is_some_and(|word| word & (1 << (bit % 64)) != 0)
            })
    }

    /// Adds an entry of a list of check 0, to its list and to `Dictionary`.
    pub(crate) fn add_dictionary(
        &mut self,
        list: WordList,
        chars: impl Iterator<Item = char> + Clone,
    ) {
        self.add(WordList::Dictionary, chars.clone());
        self.add(list, chars);
    }

    /// Adds the entries of a language's lists: check 0, short words and particles.
    pub(crate) fn add_table(&mut self, data: &Table<'_>) {
        let lists = [
            (WordList::Units, data.units),
            (WordList::Months, data.months),
            (WordList::Exceptions, data.atom_exceptions),
            (WordList::Exceptions, data.elisions),
        ];
        for (list, entries) in lists {
            for entry in entries.iter() {
                self.add_dictionary(list, entry.chars());
            }
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
                    self.add_dictionary(WordList::Abbreviations, run);
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

/// A separator that shows on its own between two digits and reads neither as a range nor as a
/// minus (design section 7.6).
fn is_visible_separator(c: char) -> bool {
    use unicode_segmentation::UnicodeSegmentation;
    // Between two digits the separator is a cluster of its own.
    let alone = ['0', c, '0']
        .iter()
        .collect::<String>()
        .graphemes(true)
        .count()
        == 3;
    alone
        && !c.is_alphanumeric()
        && !c.is_whitespace()
        && !c.is_control()
        && !is_format_category(c)
        && !is_reserved_default_ignorable(c)
        && !crate::engine::lex::is_joiner(c)
        && !is_dash_property(c)
}

/// A group separator of design section 7.6: U+00A0, U+202F, or a visible separator; a quote
/// character is allowed (`’` in de-CH), since the three digits after it keep it from being read
/// as a prime.
pub(crate) fn is_valid_group(c: char) -> bool {
    matches!(c, '\u{a0}' | '\u{202f}') || is_visible_separator(c)
}

/// A decimal separator of design section 7.6: a visible character of its own that is not a letter,
/// a digit, whitespace, a quote character, or a character with the Unicode `Dash` property. A
/// control or format character, or a mark that joins a digit next to it, would hide the separator
/// (`1\u{ad}5` shows as `15`); a dash would read as a range or a minus (`1–5` is also the range from
/// 1 to 5); a quote character between two digits is rewritten by the quote rules (`1'5` becomes the
/// prime of `1′5`), so the number would not keep its meaning.
pub(crate) fn is_valid_decimal(c: char) -> bool {
    is_visible_separator(c) && !crate::engine::lex::is_quote_char(c)
}

/// Whether `c` is of general category `Cf` (format), all of them invisible, by the Unicode 15.1
/// tables.
pub(crate) const fn is_format_category(c: char) -> bool {
    matches!(
        c,
        '\u{ad}'
            | '\u{600}'..='\u{605}'
            | '\u{61c}'
            | '\u{6dd}'
            | '\u{70f}'
            | '\u{890}'..='\u{891}'
            | '\u{8e2}'
            | '\u{180e}'
            | '\u{200b}'..='\u{200f}'
            | '\u{202a}'..='\u{202e}'
            | '\u{2060}'..='\u{2064}'
            | '\u{2066}'..='\u{206f}'
            | '\u{feff}'
            | '\u{fff9}'..='\u{fffb}'
            | '\u{110bd}'
            | '\u{110cd}'
            | '\u{13430}'..='\u{1343f}'
            | '\u{1bca0}'..='\u{1bca3}'
            | '\u{1d173}'..='\u{1d17a}'
            | '\u{e0001}'
            | '\u{e0020}'..='\u{e007f}'
    )
}

/// Whether `symbol` keeps its own grapheme clusters between two letters: no mark at its start
/// joins the character before it, and nothing at its end joins the character after it.
fn stands_alone(symbol: &str) -> bool {
    use unicode_segmentation::UnicodeSegmentation;
    let framed = format!("x{symbol}x");
    let mut clusters = framed.graphemes(true);
    clusters.next() == Some("x") && clusters.next_back() == Some("x")
}

/// Whether `c` is an assigned default-ignorable code point of Unicode 15.1
/// (`DerivedCoreProperties.txt`) outside general category `Cf`: U+034F COMBINING GRAPHEME JOINER,
/// the Hangul fillers, the Khmer inherent vowels and the variation selectors. They show nothing.
const fn is_assigned_default_ignorable(c: char) -> bool {
    matches!(
        c,
        '\u{34f}'
            | '\u{115f}'..='\u{1160}'
            | '\u{17b4}'..='\u{17b5}'
            | '\u{180b}'..='\u{180d}'
            | '\u{180f}'
            | '\u{3164}'
            | '\u{fe00}'..='\u{fe0f}'
            | '\u{ffa0}'
            | '\u{e0100}'..='\u{e01ef}'
    )
}

/// Whether `c` is a code point that Unicode 15.1 reserves as default ignorable without assigning
/// it (`DerivedCoreProperties.txt`): a later version may assign it as an invisible character.
pub(crate) const fn is_reserved_default_ignorable(c: char) -> bool {
    matches!(
        c,
        '\u{2065}'
            | '\u{fff0}'..='\u{fff8}'
            | '\u{e0000}'
            | '\u{e0002}'..='\u{e001f}'
            | '\u{e0080}'..='\u{e00ff}'
            | '\u{e01f0}'..='\u{e0fff}'
    )
}

/// Whether `c` has the Unicode `Dash` property (`PropList.txt` of Unicode 15.1, and U+10D6E of
/// 16.0): `-`, the dashes of the lexer and their look-alikes such as U+FE63 SMALL HYPHEN-MINUS and
/// U+FF0D FULLWIDTH HYPHEN-MINUS.
const fn is_dash_property(c: char) -> bool {
    matches!(
        c,
        '-' | '\u{58a}'
            | '\u{5be}'
            | '\u{1400}'
            | '\u{1806}'
            | '\u{2010}'..='\u{2015}'
            | '\u{2053}'
            | '\u{207b}'
            | '\u{208b}'
            | '\u{2212}'
            | '\u{2e17}'
            | '\u{2e1a}'
            | '\u{2e3a}'..='\u{2e3b}'
            | '\u{2e40}'
            | '\u{2e5d}'
            | '\u{301c}'
            | '\u{3030}'
            | '\u{30a0}'
            | '\u{fe31}'..='\u{fe32}'
            | '\u{fe58}'
            | '\u{fe63}'
            | '\u{ff0d}'
            | '\u{10d6e}'
            | '\u{10ead}'
    )
}

/// A minus sign of design section 7.6: `-` or a dash character of the lexer (U+2010 to U+2015,
/// U+2212), the signs that check 15 of design section 3.6 reads as the sign of a number, so that
/// the minus that `number.minus` writes changes nothing for the second pass (invariant I4).
pub(crate) fn is_valid_minus(c: char) -> bool {
    c == '-' || crate::engine::lex::is_dash_char(c)
}
