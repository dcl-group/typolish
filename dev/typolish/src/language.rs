//! `Language`: a language as built-in constants or caller data (design section 2.2).

use core::fmt;
use core::hash::{Hash, Hasher};
use core::str::FromStr;
use std::sync::Arc;

use crate::lang::data::{
    Bind, BindKey, DashData, DashStyle, EllipsisForms, LanguageData, MonthPosition, NumberData,
    OwnedAbbreviation, OwnedTable, QuotePairs, Script, Table, is_canonical_tag,
};
use crate::lang::{BUILTIN, common, en_us, ru_ru};
use crate::{Error, Rule};

/// A language: typographic data plus its tag.
///
/// Languages are data, not code: rules never branch on a tag, they read the data table
/// ([`LanguageData`]). Built-in languages are constants; an unknown tag is an explicit error,
/// and a caller who prefers the common rules falls back to [`Language::COMMON`].
///
/// ```
/// use typolish::{Error, Language};
///
/// // An explicit error.
/// assert!(matches!(Language::from_tag("de-DE"), Err(Error::UnknownLanguage { .. })));
/// // Common rules only.
/// let lang = Language::from_tag("de-DE").unwrap_or(Language::COMMON);
/// assert_eq!(lang.tag(), "und");
/// // A bare subtag resolves to the default region; a different region is not substituted.
/// assert_eq!(Language::from_tag("RU")?.tag(), "ru-RU");
/// assert!(Language::from_tag("en-GB").is_err());
/// # Ok::<(), typolish::Error>(())
/// ```
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Language {
    repr: Repr,
}

/// Where the data of a language lives. Equality compares the tag and the data table, the hash
/// uses the tag only; this is consistent, because equal languages have equal tags.
#[derive(Clone, Debug)]
enum Repr {
    Static(&'static LanguageData),
    /// A language of `LanguageBuilder` (design section 7.1).
    Owned(Arc<Owned>),
}

/// The data of a language built at run time: the owned tag and the owned table.
#[derive(Debug)]
struct Owned {
    tag: String,
    data: OwnedTable,
}

impl PartialEq for Repr {
    fn eq(&self, other: &Repr) -> bool {
        self.tag() == other.tag() && self.data() == other.data()
    }
}

impl Eq for Repr {}

impl Hash for Repr {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.tag().hash(state);
    }
}

impl Repr {
    #[inline]
    fn data(&self) -> Table<'_> {
        match self {
            Repr::Static(data) => data.table(),
            Repr::Owned(owned) => owned.data.table(),
        }
    }

    fn tag(&self) -> &str {
        match self {
            Repr::Static(data) => data.tag,
            Repr::Owned(owned) => &owned.tag,
        }
    }
}

/// Bare language subtags and the built-in language each resolves to (the CLDR likely-subtags
/// rule, design section 2.2).
const DEFAULT_REGIONS: [(&str, &LanguageData); 2] = [("en", &en_us::EN_US), ("ru", &ru_ru::RU_RU)];

impl Language {
    /// Russian (Russia), tag `ru-RU`.
    ///
    /// ```
    /// use typolish::Language;
    ///
    /// assert_eq!(Language::RU_RU.tag(), "ru-RU");
    /// ```
    pub const RU_RU: Language = Language::from_static(&ru_ru::RU_RU);

    /// English (United States), tag `en-US`.
    ///
    /// ```
    /// use typolish::Language;
    ///
    /// assert_eq!(Language::EN_US.tag(), "en-US");
    /// ```
    pub const EN_US: Language = Language::from_static(&en_us::EN_US);

    /// No language data: only rules that need none are active (values: design section 7.4).
    ///
    /// ```
    /// use typolish::{Language, Rule};
    ///
    /// assert_eq!(Language::COMMON.tag(), "und");
    /// assert!(!Language::COMMON.supports(Rule::QuotesPairs));
    /// ```
    pub const COMMON: Language = Language::from_static(&common::COMMON);

    /// Resolves a BCP 47 tag: ASCII case-insensitive, `_` read as `-`.
    ///
    /// An exact tag matches; a bare language subtag matches its registered default region
    /// (`ru` → `ru-RU`, `en` → `en-US`). `und` resolves to [`Language::COMMON`], so
    /// `from_tag(l.tag())` gives `l` back for every built-in language and for `COMMON`. Anything
    /// else is [`Error::UnknownLanguage`].
    ///
    /// ```
    /// use typolish::Language;
    ///
    /// assert_eq!(Language::from_tag("en_us")?, Language::EN_US);
    /// assert_eq!(Language::from_tag("UND")?, Language::COMMON);
    /// # Ok::<(), typolish::Error>(())
    /// ```
    pub fn from_tag(tag: &str) -> Result<Language, Error> {
        let same = |known: &str| {
            known.len() == tag.len()
                && known.bytes().zip(tag.bytes()).all(|(k, t)| {
                    let t = if t == b'_' { b'-' } else { t };
                    k.eq_ignore_ascii_case(&t)
                })
        };
        if same(common::COMMON.tag) {
            return Ok(Language::COMMON);
        }
        if let Some(data) = BUILTIN.iter().find(|data| same(data.tag)) {
            return Ok(Language::from_static(data));
        }
        if let Some((_, data)) = DEFAULT_REGIONS.iter().find(|(bare, _)| same(bare)) {
            return Ok(Language::from_static(data));
        }
        Err(Error::UnknownLanguage {
            tag: tag.to_owned(),
        })
    }

    /// A language from caller data built at compile time, validated (design section 7.6).
    ///
    /// ```
    /// use typolish::{Error, Language, LanguageData};
    ///
    /// static EN_GB: LanguageData = LanguageData::EN_US.with_tag("en-GB");
    /// assert_eq!(Language::from_data(&EN_GB)?.tag(), "en-GB");
    ///
    /// static BAD: LanguageData = LanguageData::EN_US.with_tag("EN-gb");
    /// assert!(matches!(Language::from_data(&BAD), Err(Error::InvalidLanguageData { .. })));
    /// # Ok::<(), typolish::Error>(())
    /// ```
    pub fn from_data(data: &'static LanguageData) -> Result<Language, Error> {
        data.validate()
            .map_err(|reason| Error::InvalidLanguageData {
                tag: data.tag.to_owned(),
                reason,
            })?;
        Ok(Language::from_static(data))
    }

    /// The same as [`Language::from_data`] without validation, for a `const` or `static`
    /// typographer. Invalid data never panics, but rules may misbehave; validate the table once in
    /// a test with `from_data`.
    ///
    /// ```
    /// use typolish::{Language, LanguageData, RuleSet, Typographer};
    ///
    /// static EN_GB_DATA: LanguageData = LanguageData::EN_US.with_tag("en-GB");
    /// static EN_GB_PROSE: Typographer =
    ///     Typographer::new(Language::from_static(&EN_GB_DATA), RuleSet::PROSE);
    /// assert!(Language::from_data(&EN_GB_DATA).is_ok());
    /// assert_eq!(EN_GB_PROSE.language().tag(), "en-GB");
    /// ```
    #[must_use]
    pub const fn from_static(data: &'static LanguageData) -> Language {
        Language {
            repr: Repr::Static(data),
        }
    }

    /// The canonical tag, for example `"ru-RU"`; `"und"` for [`Language::COMMON`].
    ///
    /// ```
    /// use typolish::Language;
    ///
    /// assert_eq!(Language::EN_US.tag(), "en-US");
    /// ```
    #[must_use]
    pub fn tag(&self) -> &str {
        self.repr.tag()
    }

    /// The built-in languages in tag order, [`Language::COMMON`] excluded.
    ///
    /// ```
    /// use typolish::Language;
    ///
    /// let tags: Vec<String> = Language::builtin().map(|l| l.to_string()).collect();
    /// assert_eq!(tags, ["en-US", "ru-RU"]);
    /// ```
    pub fn builtin() -> impl Iterator<Item = Language> {
        BUILTIN.into_iter().map(Language::from_static)
    }

    /// Whether the language has the data a rule needs; a rule without its data is inert
    /// (design section 7.4 lists the data each rule reads).
    ///
    /// ```
    /// use typolish::{Language, Rule};
    ///
    /// assert!(Language::RU_RU.supports(Rule::NbspParticles));
    /// assert!(!Language::EN_US.supports(Rule::NbspParticles));
    /// assert!(Language::COMMON.supports(Rule::EllipsisDots));
    /// ```
    #[must_use]
    pub fn supports(&self, rule: Rule) -> bool {
        supports(self.data(), rule)
    }

    /// The data table.
    #[inline]
    pub(crate) fn data(&self) -> Table<'_> {
        self.repr.data()
    }
}

/// The data each rule needs (design section 7.4).
fn supports(data: Table<'_>, rule: Rule) -> bool {
    match rule {
        Rule::SymbolsCopyright
        | Rule::SymbolsArrows
        | Rule::SymbolsMath
        | Rule::EllipsisDots
        | Rule::EllipsisInner
        | Rule::RangePlaceholders
        | Rule::RangeDigits
        | Rule::RangeTime
        | Rule::NumberMinus
        | Rule::NumberTimes
        | Rule::QuotesApostrophe
        | Rule::QuotesPrimes
        | Rule::NbspPhone => true,
        Rule::DashDialogue => data.dash.is_some_and(|dash| dash.dialogue),
        Rule::DashDoubleHyphen | Rule::DashSpacedHyphen => data.dash.is_some(),
        Rule::NbspDash => data
            .dash
            .is_some_and(|dash| dash.style == DashStyle::SpacedGlued),
        Rule::RangeContext => {
            !data.year_words.is_empty()
                || !data.century_words.is_empty()
                || !data.range_lead_words.is_empty()
        }
        Rule::QuotesPairs => data.quotes.is_some(),
        Rule::NbspAbbreviations => !data.abbreviations.is_empty(),
        Rule::NbspInitials => data.script != Script::Other,
        Rule::NbspDates => {
            !data.months.is_empty() || !data.year_words.is_empty() || !data.century_words.is_empty()
        }
        Rule::NbspUnits => !data.units.is_empty(),
        Rule::NbspSigns => !data.signs.is_empty(),
        Rule::NbspShortWords => !data.short_words.is_empty(),
        Rule::NbspShortLength => data.short_word_length > 0,
        Rule::NbspParticles => !data.particles.is_empty(),
    }
}

/// A language built at run time (design section 7.1): it starts from an existing language or from
/// the values of [`LanguageData::new`] under a new tag, changes fields, and
/// [`build`](LanguageBuilder::build) normalizes the lists and the case of the tag, validates the
/// table (design section 7.6) and returns a [`Language`] that holds its data in an `Arc`.
///
/// There is one method per field of [`LanguageData`]. A list method replaces the whole list; the
/// lists are owned, so they can come from a caller's configuration file.
///
/// ```
/// use typolish::{Language, LanguageBuilder, Output, QuotePairs, Rule, RuleSet, Typographer};
///
/// let de = LanguageBuilder::from_language(&Language::EN_US, "de-DE")
///     .quotes(Some(QuotePairs::new(('„', '“'), ('‚', '‘'))))
///     .short_words(["und", "in", "zu"])
///     .build()?;
/// assert_eq!(de.tag(), "de-DE");
/// assert!(de.supports(Rule::NbspShortWords));
/// let t = Typographer::new(de, RuleSet::EMPTY.with(Rule::QuotesPairs));
/// assert_eq!(t.typeset("\"a 'b' und c\"", Output::Unicode), "„a ‚b‘ und c“");
/// # Ok::<(), typolish::Error>(())
/// ```
#[derive(Clone, Debug)]
#[must_use]
pub struct LanguageBuilder {
    tag: String,
    data: OwnedTable,
}

/// The entries of a list argument as owned strings.
fn owned_list<I>(items: I) -> Vec<Box<str>>
where
    I: IntoIterator,
    I::Item: AsRef<str>,
{
    items
        .into_iter()
        .map(|item| Box::from(item.as_ref()))
        .collect()
}

impl LanguageBuilder {
    /// Starts from an existing language's data under a new tag.
    ///
    /// ```
    /// use typolish::{Language, LanguageBuilder};
    ///
    /// let kz = LanguageBuilder::from_language(&Language::RU_RU, "ru-KZ").build()?;
    /// assert_eq!(kz.to_string(), "ru-KZ");
    /// assert_ne!(kz, Language::RU_RU);
    /// # Ok::<(), typolish::Error>(())
    /// ```
    pub fn from_language(base: &Language, tag: &str) -> LanguageBuilder {
        LanguageBuilder {
            tag: tag.to_owned(),
            data: OwnedTable::of(base.data()),
        }
    }

    /// Starts from the values of [`LanguageData::new`] under a tag.
    ///
    /// ```
    /// use typolish::LanguageBuilder;
    ///
    /// assert_eq!(LanguageBuilder::new("EN-gb").build()?.tag(), "en-GB");
    /// # Ok::<(), typolish::Error>(())
    /// ```
    pub fn new(tag: &str) -> LanguageBuilder {
        LanguageBuilder {
            tag: tag.to_owned(),
            data: OwnedTable::of(LanguageData::new("und").table()),
        }
    }

    /// Sets the CLDR version the values were checked against.
    ///
    /// ```
    /// use typolish::{Language, LanguageBuilder};
    ///
    /// let lang = LanguageBuilder::from_language(&Language::RU_RU, "ru-KZ")
    ///     .cldr_version("49")
    ///     .build()?;
    /// assert_eq!(lang.tag(), "ru-KZ");
    /// # Ok::<(), typolish::Error>(())
    /// ```
    pub fn cldr_version(mut self, version: &str) -> Self {
        version.clone_into(&mut self.data.cldr_version);
        self
    }

    /// Sets the script of the language's words.
    ///
    /// ```
    /// use typolish::{LanguageBuilder, Output, Rule, RuleSet, Script, Typographer};
    ///
    /// let lang = LanguageBuilder::new("xx").script(Script::Latin).short_words(["a"]).build()?;
    /// let t = Typographer::new(lang, RuleSet::EMPTY.with(Rule::NbspShortWords));
    /// assert_eq!(t.typeset("a cat", Output::Unicode), "a\u{a0}cat");
    /// # Ok::<(), typolish::Error>(())
    /// ```
    pub fn script(mut self, script: Script) -> Self {
        self.data.script = script;
        self
    }

    /// Sets the quote pairs; `None` makes `quotes.pairs` inert.
    ///
    /// ```
    /// use typolish::{Language, LanguageBuilder, Rule};
    ///
    /// let none = LanguageBuilder::from_language(&Language::RU_RU, "ru").quotes(None).build()?;
    /// assert!(!none.supports(Rule::QuotesPairs));
    /// # Ok::<(), typolish::Error>(())
    /// ```
    pub fn quotes(mut self, quotes: Option<QuotePairs>) -> Self {
        self.data.quotes = quotes;
        self
    }

    /// Sets the elisions (`'cause`); `build` lowercases and sorts them.
    ///
    /// ```
    /// use typolish::{Language, LanguageBuilder, Output, Rule, RuleSet, Typographer};
    ///
    /// let lang = LanguageBuilder::from_language(&Language::EN_US, "en-GB")
    ///     .elisions(["'CAUSE"])
    ///     .build()?;
    /// let t = Typographer::new(lang, RuleSet::EMPTY.with(Rule::QuotesApostrophe));
    /// assert_eq!(
    ///     t.typeset("'cause I said so", Output::Unicode),
    ///     "’cause I said so"
    /// );
    /// # Ok::<(), typolish::Error>(())
    /// ```
    pub fn elisions<I>(mut self, elisions: I) -> Self
    where
        I: IntoIterator,
        I::Item: AsRef<str>,
    {
        self.data.elisions = owned_list(elisions);
        self
    }

    /// Sets whether `'90s` takes an apostrophe.
    ///
    /// ```
    /// use typolish::{Language, LanguageBuilder, Output, Rule, RuleSet, Typographer};
    ///
    /// let lang = LanguageBuilder::from_language(&Language::EN_US, "en-GB")
    ///     .decade_apostrophe(false)
    ///     .build()?;
    /// let t = Typographer::new(lang, RuleSet::EMPTY.with(Rule::QuotesApostrophe));
    /// assert_eq!(t.typeset("the '90s", Output::Unicode), "the '90s");
    /// # Ok::<(), typolish::Error>(())
    /// ```
    pub fn decade_apostrophe(mut self, on: bool) -> Self {
        self.data.decade_apostrophe = on;
        self
    }

    /// Sets whether a trailing possessive (`dogs'`) takes an apostrophe.
    ///
    /// ```
    /// use typolish::{Language, LanguageBuilder, Output, Rule, RuleSet, Typographer};
    ///
    /// let lang = LanguageBuilder::from_language(&Language::EN_US, "en-GB")
    ///     .trailing_possessive(false)
    ///     .build()?;
    /// let t = Typographer::new(lang, RuleSet::EMPTY.with(Rule::QuotesApostrophe));
    /// assert_eq!(
    ///     t.typeset("the dogs' toys", Output::Unicode),
    ///     "the dogs' toys"
    /// );
    /// # Ok::<(), typolish::Error>(())
    /// ```
    pub fn trailing_possessive(mut self, on: bool) -> Self {
        self.data.trailing_possessive = on;
        self
    }

    /// Sets the dash data; `None` makes the `dash.*` rules and `nbsp.dash` inert.
    ///
    /// ```
    /// use typolish::{DashData, DashStyle, Language, LanguageBuilder, Rule};
    ///
    /// let none = LanguageBuilder::from_language(&Language::RU_RU, "ru-KZ")
    ///     .dash(None)
    ///     .build()?;
    /// assert!(!none.supports(Rule::NbspDash));
    /// let spaced = LanguageBuilder::new("xx")
    ///     .dash(Some(DashData::new(DashStyle::Spaced, true)))
    ///     .build()?;
    /// assert!(spaced.supports(Rule::DashDialogue));
    /// # Ok::<(), typolish::Error>(())
    /// ```
    pub fn dash(mut self, dash: Option<DashData>) -> Self {
        self.data.dash = dash;
        self
    }

    /// Sets the dash style, as [`LanguageData::with_dash_style`] does.
    ///
    /// ```
    /// use typolish::{DashStyle, Language, LanguageBuilder, Output, Rule, RuleSet, Typographer};
    ///
    /// let gb = LanguageBuilder::from_language(&Language::EN_US, "en-GB")
    ///     .dash_style(DashStyle::Spaced)
    ///     .build()?;
    /// let t = Typographer::new(gb, RuleSet::EMPTY.with(Rule::DashSpacedHyphen));
    /// assert_eq!(t.typeset("Moscow - the capital", Output::Unicode), "Moscow — the capital");
    /// # Ok::<(), typolish::Error>(())
    /// ```
    pub fn dash_style(mut self, style: DashStyle) -> Self {
        self.data.dash = Some(DashData::new(
            style,
            self.data.dash.is_some_and(|dash| dash.dialogue),
        ));
        self
    }

    /// Sets the ellipsis forms after `?` and `!`.
    ///
    /// ```
    /// use typolish::{
    ///     EllipsisForms, Language, LanguageBuilder, Output, Rule, RuleSet, Typographer,
    /// };
    ///
    /// let lang = LanguageBuilder::from_language(&Language::EN_US, "en-GB")
    ///     .ellipsis(EllipsisForms::new("?..", "!..", "?!.."))
    ///     .build()?;
    /// let t = Typographer::new(lang, RuleSet::EMPTY.with(Rule::EllipsisDots));
    /// assert_eq!(t.typeset("Stop!...", Output::Unicode), "Stop!..");
    /// # Ok::<(), typolish::Error>(())
    /// ```
    pub fn ellipsis(mut self, ellipsis: EllipsisForms) -> Self {
        self.data.ellipsis = ellipsis;
        self
    }

    /// Sets the short words (rule `nbsp.short-words`); `build` lowercases and sorts them.
    ///
    /// ```
    /// use typolish::{Language, LanguageBuilder};
    ///
    /// let lang = LanguageBuilder::from_language(&Language::RU_RU, "ru-KZ")
    ///     .short_words(["а", "в", "и", "к", "о", "с", "у"])
    ///     .build()?;
    /// # let _ = lang;
    /// # Ok::<(), typolish::Error>(())
    /// ```
    pub fn short_words<I>(mut self, words: I) -> Self
    where
        I: IntoIterator,
        I::Item: AsRef<str>,
    {
        self.data.short_words = owned_list(words);
        self
    }

    /// Sets the length up to which any word of the language's script is short; 0 makes
    /// `nbsp.short-length` inert.
    ///
    /// ```
    /// use typolish::{Language, LanguageBuilder, Rule};
    ///
    /// let lang = LanguageBuilder::from_language(&Language::RU_RU, "ru-KZ")
    ///     .short_word_length(0)
    ///     .build()?;
    /// assert!(!lang.supports(Rule::NbspShortLength));
    /// # Ok::<(), typolish::Error>(())
    /// ```
    pub fn short_word_length(mut self, max_chars: u8) -> Self {
        self.data.short_word_length = max_chars;
        self
    }

    /// Sets the particles; `build` lowercases and sorts them.
    ///
    /// The rule `nbsp.particles` glues a particle to the word or slot before it, and a particle
    /// so placed is never the trigger word of `nbsp.short-words` or `nbsp.short-length`, which
    /// glue nothing after it (design section 8.7).
    ///
    /// ```
    /// use typolish::{Language, LanguageBuilder, Output, RuleSet, Typographer};
    ///
    /// let ru = |particles: &[&str]| -> Result<Typographer, typolish::Error> {
    ///     let lang = LanguageBuilder::from_language(&Language::RU_RU, "ru-KZ")
    ///         .particles(particles)
    ///         .short_word_length(2)
    ///         .build()?;
    ///     Ok(Typographer::new(lang, RuleSet::PROSE))
    /// };
    /// let t = ru(&["ли"])?;
    /// assert_eq!(t.typeset("Знаешь ли ты", Output::Unicode), "Знаешь\u{a0}ли ты");
    /// // Without the particle, `ли` is an ordinary short word.
    /// let t = ru(&[])?;
    /// assert_eq!(t.typeset("Знаешь ли ты", Output::Unicode), "Знаешь ли\u{a0}ты");
    /// # Ok::<(), typolish::Error>(())
    /// ```
    pub fn particles<I>(mut self, particles: I) -> Self
    where
        I: IntoIterator,
        I::Item: AsRef<str>,
    {
        self.data.particles = owned_list(particles);
        self
    }

    /// Sets the abbreviations: each entry is its parts as printed and its binding (rule
    /// `nbsp.abbreviations`); `build` sorts them by their parts.
    ///
    /// ```
    /// use typolish::{Bind, Language, LanguageBuilder, Rule};
    ///
    /// let lang = LanguageBuilder::from_language(&Language::RU_RU, "ru-KZ")
    ///     .abbreviations([(&["ул."][..], Bind::Next), (&["т.", "е."][..], Bind::Inner)])
    ///     .build()?;
    /// assert!(lang.supports(Rule::NbspAbbreviations));
    /// let none = LanguageBuilder::new("kk").build()?;
    /// assert!(!none.supports(Rule::NbspAbbreviations));
    /// # Ok::<(), typolish::Error>(())
    /// ```
    pub fn abbreviations<I, P>(mut self, entries: I) -> Self
    where
        I: IntoIterator<Item = (P, Bind)>,
        P: IntoIterator,
        P::Item: AsRef<str>,
    {
        self.data.abbreviations = entries
            .into_iter()
            .map(|(parts, bind)| OwnedAbbreviation {
                parts: owned_list(parts),
                bind: BindKey::of(bind),
            })
            .collect();
        self
    }

    /// Sets the dotted words that are not technical (`e.g.`); `build` lowercases and sorts them.
    ///
    /// ```
    /// use typolish::{Language, LanguageBuilder, Output, RuleSet, Typographer};
    ///
    /// let gb = LanguageBuilder::from_language(&Language::EN_US, "en-GB")
    ///     .atom_exceptions(["Ph.D"])
    ///     .build()?;
    /// // Without the exception `Ph.D` is a dotted technical fragment, like a host name.
    /// let us = Typographer::new(Language::EN_US, RuleSet::PROSE);
    /// let gb = Typographer::new(gb, RuleSet::PROSE);
    /// assert_eq!(us.report("a Ph.D student", Output::Unicode).protected().len(), 1);
    /// assert!(gb.report("a Ph.D student", Output::Unicode).protected().is_empty());
    /// # Ok::<(), typolish::Error>(())
    /// ```
    pub fn atom_exceptions<I>(mut self, words: I) -> Self
    where
        I: IntoIterator,
        I::Item: AsRef<str>,
    {
        self.data.atom_exceptions = owned_list(words);
        self
    }

    /// Sets the units, compared with case (rules `nbsp.units` and `range.context`); `build`
    /// sorts them and keeps their case.
    ///
    /// ```
    /// use typolish::{LanguageBuilder, Output, Rule, RuleSet, Typographer};
    ///
    /// let lang = LanguageBuilder::new("xx").units(["kB", "MB"]).build()?;
    /// let t = Typographer::new(lang, RuleSet::EMPTY.with(Rule::NbspUnits));
    /// assert_eq!(t.typeset("5 MB", Output::Unicode), "5\u{a0}MB");
    /// # Ok::<(), typolish::Error>(())
    /// ```
    pub fn units<I>(mut self, units: I) -> Self
    where
        I: IntoIterator,
        I::Item: AsRef<str>,
    {
        self.data.units = owned_list(units);
        self
    }

    /// Sets the units that are also words; each must also be a unit.
    ///
    /// ```
    /// use typolish::LanguageBuilder;
    ///
    /// assert!(LanguageBuilder::new("xx").units(["in"]).ambiguous_units(["in"]).build().is_ok());
    /// assert!(LanguageBuilder::new("xx").ambiguous_units(["in"]).build().is_err());
    /// ```
    pub fn ambiguous_units<I>(mut self, units: I) -> Self
    where
        I: IntoIterator,
        I::Item: AsRef<str>,
    {
        self.data.ambiguous_units = owned_list(units);
        self
    }

    /// Sets the month forms used next to a day number, as printed (rule `nbsp.dates`).
    ///
    /// ```
    /// use typolish::{LanguageBuilder, Output, Rule, RuleSet, Typographer};
    ///
    /// let lang = LanguageBuilder::new("xx").months(["Mai", "Juni"]).build()?;
    /// let t = Typographer::new(lang, RuleSet::EMPTY.with(Rule::NbspDates));
    /// assert_eq!(t.typeset("am 21 Mai", Output::Unicode), "am 21\u{a0}Mai");
    /// # Ok::<(), typolish::Error>(())
    /// ```
    pub fn months<I>(mut self, months: I) -> Self
    where
        I: IntoIterator,
        I::Item: AsRef<str>,
    {
        self.data.months = owned_list(months);
        self
    }

    /// Sets where a month name stands relative to the day number.
    ///
    /// ```
    /// use typolish::{LanguageBuilder, MonthPosition, Output, Rule, RuleSet, Typographer};
    ///
    /// let lang = LanguageBuilder::new("xx")
    ///     .months(["May"])
    ///     .month_position(MonthPosition::BeforeDay)
    ///     .build()?;
    /// let t = Typographer::new(lang, RuleSet::EMPTY.with(Rule::NbspDates));
    /// assert_eq!(t.typeset("on May 21", Output::Unicode), "on May\u{a0}21");
    /// # Ok::<(), typolish::Error>(())
    /// ```
    pub fn month_position(mut self, position: MonthPosition) -> Self {
        self.data.month_position = position;
        self
    }

    /// Sets the words after a year; `build` lowercases and sorts them.
    ///
    /// ```
    /// use typolish::{LanguageBuilder, Output, Rule, RuleSet, Typographer};
    ///
    /// let lang = LanguageBuilder::new("xx").year_words(["R."]).build()?;
    /// let t = Typographer::new(lang, RuleSet::EMPTY.with(Rule::NbspDates));
    /// assert_eq!(t.typeset("w 2026 r.", Output::Unicode), "w 2026\u{a0}r.");
    /// # Ok::<(), typolish::Error>(())
    /// ```
    pub fn year_words<I>(mut self, words: I) -> Self
    where
        I: IntoIterator,
        I::Item: AsRef<str>,
    {
        self.data.year_words = owned_list(words);
        self
    }

    /// Sets the words after a century; `build` lowercases and sorts them.
    ///
    /// ```
    /// use typolish::{LanguageBuilder, Output, Rule, RuleSet, Typographer};
    ///
    /// let lang = LanguageBuilder::new("xx").century_words(["w."]).build()?;
    /// let t = Typographer::new(lang, RuleSet::EMPTY.with(Rule::NbspDates));
    /// assert_eq!(t.typeset("XX w.", Output::Unicode), "XX\u{a0}w.");
    /// # Ok::<(), typolish::Error>(())
    /// ```
    pub fn century_words<I>(mut self, words: I) -> Self
    where
        I: IntoIterator,
        I::Item: AsRef<str>,
    {
        self.data.century_words = owned_list(words);
        self
    }

    /// Sets the words before a range (`стр.`, `pp.`); `build` lowercases and sorts them.
    ///
    /// ```
    /// use typolish::{LanguageBuilder, Output, Rule, RuleSet, Typographer};
    ///
    /// let lang = LanguageBuilder::new("xx").range_lead_words(["s."]).build()?;
    /// let t = Typographer::new(lang, RuleSet::EMPTY.with(Rule::RangeContext));
    /// assert_eq!(t.typeset("s. 10-20", Output::Unicode), "s. 10–20");
    /// # Ok::<(), typolish::Error>(())
    /// ```
    pub fn range_lead_words<I>(mut self, words: I) -> Self
    where
        I: IntoIterator,
        I::Item: AsRef<str>,
    {
        self.data.range_lead_words = owned_list(words);
        self
    }

    /// Sets the signs that name a number (`№`, `§`, `¶`); `build` sorts them. Each is a
    /// punctuation character of the lexer (`#`, `№`, `§`, `¶`, …; design sections 3.5 and 7.6).
    ///
    /// ```
    /// use typolish::{LanguageBuilder, Output, Rule, RuleSet, Typographer};
    ///
    /// let lang = LanguageBuilder::new("xx").signs(['№', '#']).build()?;
    /// let t = Typographer::new(lang, RuleSet::EMPTY.with(Rule::NbspSigns));
    /// assert_eq!(t.typeset("# 5", Output::Unicode), "#\u{a0}5");
    /// # Ok::<(), typolish::Error>(())
    /// ```
    pub fn signs<I>(mut self, signs: I) -> Self
    where
        I: IntoIterator<Item = char>,
    {
        self.data.signs = signs.into_iter().collect();
        self
    }

    /// Sets the number symbols and patterns.
    ///
    /// ```
    /// use typolish::{Language, LanguageBuilder, NumberData, Output, RuleSet, Typographer};
    ///
    /// let de = LanguageBuilder::from_language(&Language::EN_US, "de-DE")
    ///     .numbers(
    ///         NumberData::new()
    ///             .with_decimal(',')
    ///             .with_group('.')
    ///             .with_min_grouping_digits(1),
    ///     )
    ///     .build()?;
    /// let t = Typographer::new(de, RuleSet::EMPTY);
    /// assert_eq!(
    ///     t.format_number(1234.5, Output::Unicode).to_string(),
    ///     "1.234,5"
    /// );
    /// # Ok::<(), typolish::Error>(())
    /// ```
    pub fn numbers(mut self, numbers: NumberData) -> Self {
        self.data.numbers = numbers;
        self
    }

    /// Normalizes the lists (the word lists of design section 7.6 lowercased; every list sorted
    /// by code point and free of duplicates) and the case of the tag (language subtag lowercase,
    /// region uppercase), validates the table (design section 7.6) and returns the language.
    ///
    /// # Errors
    ///
    /// [`Error::InvalidLanguageData`] when the tag is not of the form `ll`, `ll-RR`, `ll-DDD` or
    /// `und`, or when a check of the table fails, naming the field.
    ///
    /// ```
    /// use typolish::{Error, Language, LanguageBuilder, QuotePairs};
    ///
    /// let bad = LanguageBuilder::from_language(&Language::RU_RU, "ru")
    ///     .quotes(Some(QuotePairs::new(('«', '»'), ('«', '„'))))
    ///     .build();
    /// assert!(matches!(bad, Err(Error::InvalidLanguageData { .. })));
    /// assert!(LanguageBuilder::new("russian").build().is_err());
    /// ```
    pub fn build(mut self) -> Result<Language, Error> {
        let tag = normalize_tag(&self.tag);
        let invalid = |reason| Error::InvalidLanguageData {
            tag: tag.clone(),
            reason,
        };
        if !is_canonical_tag(&tag) {
            return Err(invalid(
                "tag: not a canonical tag of the form ll, ll-RR, ll-DDD or und",
            ));
        }
        self.data.normalize();
        self.data.table().validate().map_err(invalid)?;
        Ok(Language {
            repr: Repr::Owned(Arc::new(Owned {
                tag,
                data: self.data,
            })),
        })
    }
}

/// The case of a tag as design section 7.6 prints it: the language subtag in lowercase, a region
/// subtag in uppercase; `und` in lowercase.
fn normalize_tag(tag: &str) -> String {
    let mut out = String::with_capacity(tag.len());
    for (i, part) in tag.split('-').enumerate() {
        if i > 0 {
            out.push('-');
            out.extend(part.chars().map(|c| c.to_ascii_uppercase()));
        } else {
            out.extend(part.chars().map(|c| c.to_ascii_lowercase()));
        }
    }
    out
}

/// The same as [`Language::from_tag`].
impl FromStr for Language {
    type Err = Error;

    fn from_str(s: &str) -> Result<Language, Error> {
        Language::from_tag(s)
    }
}

/// Writes the tag.
impl fmt::Display for Language {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.tag())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::hash::{BuildHasher, RandomState};

    #[test]
    fn a_built_copy_equals_its_base() {
        // Design section 2.2: equality compares the whole table, the hash the tag only.
        let copy = LanguageBuilder::from_language(&Language::RU_RU, "ru-RU")
            .build()
            .unwrap();
        assert_eq!(copy, Language::RU_RU);
        let hasher = RandomState::new();
        assert_eq!(hasher.hash_one(&copy), hasher.hash_one(&Language::RU_RU));
        let other = LanguageBuilder::from_language(&Language::RU_RU, "ru-RU")
            .quotes(None)
            .build()
            .unwrap();
        assert_ne!(other, Language::RU_RU);
        let kz = LanguageBuilder::from_language(&Language::RU_RU, "ru-kz")
            .build()
            .unwrap();
        assert_eq!(kz.tag(), "ru-KZ");
        assert_ne!(kz, Language::RU_RU);
        assert_eq!(LanguageBuilder::new("UND").build().unwrap().tag(), "und");
    }
}
