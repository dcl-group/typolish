//! `Language`: a language as built-in constants or caller data (design section 2.2).

use core::fmt;
use core::hash::{Hash, Hasher};
use core::str::FromStr;
use std::sync::Arc;

use crate::lang::data::{DashStyle, LanguageData, QuotePairs, Script, is_canonical_tag};
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

/// The data of a language built at run time: the owned tag, and the table, whose own `tag` field
/// is `und` and is not read.
#[derive(Debug)]
struct Owned {
    tag: String,
    data: LanguageData,
}

impl PartialEq for Repr {
    fn eq(&self, other: &Repr) -> bool {
        // The tag fields of the tables are compared through `tag()`.
        let untagged = |data: &LanguageData| data.clone().with_tag("und");
        self.tag() == other.tag() && untagged(self.data()) == untagged(other.data())
    }
}

impl Eq for Repr {}

impl Hash for Repr {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.tag().hash(state);
    }
}

impl Repr {
    fn data(&self) -> &LanguageData {
        match self {
            Repr::Static(data) => data,
            Repr::Owned(owned) => &owned.data,
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
    pub(crate) fn data(&self) -> &LanguageData {
        self.repr.data()
    }
}

/// The data each rule needs (design section 7.4).
fn supports(data: &LanguageData, rule: Rule) -> bool {
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
/// [`build`](LanguageBuilder::build) normalizes the case of the tag, validates the table (design
/// section 7.6) and returns a [`Language`] that holds its data in an `Arc`.
///
/// The crate is in development: so far the quote pairs and the dash style can be changed; the
/// methods for the other fields and their lists come with the complete language data (design
/// section 15.4, stage 3).
///
/// ```
/// use typolish::{Language, LanguageBuilder, Output, QuotePairs, Rule, RuleSet, Typographer};
///
/// let de = LanguageBuilder::from_language(&Language::EN_US, "de-DE")
///     .quotes(Some(QuotePairs::new(('„', '“'), ('‚', '‘'))))
///     .build()?;
/// assert_eq!(de.tag(), "de-DE");
/// let t = Typographer::new(de, RuleSet::EMPTY.with(Rule::QuotesPairs));
/// assert_eq!(t.typeset("\"a 'b' c\"", Output::Unicode), "„a ‚b‘ c“");
/// # Ok::<(), typolish::Error>(())
/// ```
#[derive(Clone, Debug)]
#[must_use]
pub struct LanguageBuilder {
    tag: String,
    data: LanguageData,
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
            data: base.data().clone(),
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
            data: LanguageData::new("und"),
        }
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
        self.data = self.data.with_quotes(quotes);
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
        self.data = self.data.with_dash_style(style);
        self
    }

    /// Normalizes the case of the tag (language subtag lowercase, region uppercase), validates the
    /// table (design section 7.6) and returns the language.
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
    pub fn build(self) -> Result<Language, Error> {
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
        let data = self.data.with_tag("und");
        data.validate().map_err(invalid)?;
        Ok(Language {
            repr: Repr::Owned(Arc::new(Owned { tag, data })),
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
