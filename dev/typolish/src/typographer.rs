//! `Typographer` and its builder (design section 2.4) and the application methods (sections 2.5
//! and 2.6).

use std::borrow::Cow;
use std::fmt;
use std::sync::OnceLock;

use crate::engine::{self, Input, scratch::Scratch};
use crate::lang::data::{
    DashData, DashStyle, QUOTE_GLYPHS, QuotePairs, fold_char, is_valid_group, is_valid_minus,
};
use crate::report::Report;
use crate::rules::{self, Triggers};
use crate::{Error, Language, Output, Piece, Rule, RuleSet, Template};

/// Whether straight single quotes become quotes outside double quotes (rule `quotes.pairs`,
/// design section 6.8).
///
/// ```
/// use typolish::{Language, RuleSet, SingleQuotes, Typographer};
///
/// let t = Typographer::builder(Language::EN_US, RuleSet::PROSE)
///     .single_quotes(SingleQuotes::Everywhere)
///     .build()?;
/// # let _ = t;
/// # Ok::<(), typolish::Error>(())
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum SingleQuotes {
    /// Only inside double quotes (the default).
    NestedOnly,
    /// At every level.
    Everywhere,
    /// Never; straight single quotes stay.
    Never,
}

/// The space between a number and `%` or `‰` (rule `nbsp.units`, design section 8.7).
///
/// ```
/// use typolish::{Language, PercentSpacing, RuleSet, Typographer};
///
/// let t = Typographer::builder(Language::RU_RU, RuleSet::PROSE)
///     .percent(PercentSpacing::Language)
///     .build()?;
/// # let _ = t;
/// # Ok::<(), typolish::Error>(())
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum PercentSpacing {
    /// An existing space is glued, an attached sign stays (the default).
    AsWritten,
    /// The CLDR pattern of the language: `5⍽%` in Russian, `5%` in English.
    Language,
    /// The sign is attached: `5%`.
    Attached,
    /// A glued space before the sign: `5⍽%`.
    Spaced,
}

/// Places where glue is written as U+202F instead of U+00A0 (design section 15.5, item 10). A
/// set: the order of `union` calls does not matter.
///
/// ```
/// use typolish::NarrowGlue;
///
/// let places = NarrowGlue::SIGNS.union(NarrowGlue::INITIALS);
/// assert!(places.contains(NarrowGlue::SIGNS));
/// assert!(!places.contains(NarrowGlue::ABBREVIATIONS));
/// assert!(NarrowGlue::ALL.contains(places));
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct NarrowGlue {
    bits: u8,
}

impl NarrowGlue {
    /// The default: U+00A0 everywhere.
    pub const NONE: NarrowGlue = NarrowGlue { bits: 0 };
    /// `nbsp.abbreviations`: the spaces between the parts of an entry.
    pub const ABBREVIATIONS: NarrowGlue = NarrowGlue { bits: 1 };
    /// `nbsp.initials`: every space it glues.
    pub const INITIALS: NarrowGlue = NarrowGlue { bits: 1 << 1 };
    /// `nbsp.signs`: the space after `№`, `§`, `¶`.
    pub const SIGNS: NarrowGlue = NarrowGlue { bits: 1 << 2 };
    /// Every place.
    pub const ALL: NarrowGlue = NarrowGlue { bits: 0b111 };

    /// The places of both sets.
    #[must_use]
    pub const fn union(self, other: NarrowGlue) -> NarrowGlue {
        NarrowGlue {
            bits: self.bits | other.bits,
        }
    }

    /// Whether every place of `other` is in the set.
    #[must_use]
    pub const fn contains(self, other: NarrowGlue) -> bool {
        self.bits & other.bits == other.bits
    }
}

/// Additions to and removals from a list of the language data.
#[derive(Clone, Debug, PartialEq, Eq)]
struct ListEdit {
    added: Vec<String>,
    removed: Vec<String>,
}

impl ListEdit {
    const NONE: ListEdit = ListEdit {
        added: Vec::new(),
        removed: Vec::new(),
    };

    /// Whether the edited list has any entry. `removed` is sorted: `build()` sorts it, and
    /// `NONE` is empty.
    fn non_empty(&self, base: &[&str]) -> bool {
        !self.added.is_empty()
            || base.iter().any(|entry| {
                self.removed
                    .binary_search_by(|removed| removed.as_str().cmp(entry))
                    .is_err()
            })
    }
}

/// The settings of a typographer (design section 2.4): only overrides, additions and removals,
/// so that the defaults are computable at compile time.
#[derive(Clone, Debug)]
struct Settings {
    quote_pairs: Option<QuotePairs>,
    single_quotes: SingleQuotes,
    dash_style: Option<DashStyle>,
    percent: PercentSpacing,
    units: ListEdit,
    short_words: ListEdit,
    particles: ListEdit,
    short_word_length: Option<u8>,
    short_words_any_script: bool,
    narrow_glue: NarrowGlue,
    min_grouping_digits: Option<u8>,
    group_separator: Option<char>,
    minus_sign: Option<char>,
    protect_backticks: bool,
    protect_between: Vec<(String, String)>,
}

impl Settings {
    const DEFAULT: Settings = Settings {
        quote_pairs: None,
        single_quotes: SingleQuotes::NestedOnly,
        dash_style: None,
        percent: PercentSpacing::AsWritten,
        units: ListEdit::NONE,
        short_words: ListEdit::NONE,
        particles: ListEdit::NONE,
        short_word_length: None,
        short_words_any_script: false,
        narrow_glue: NarrowGlue::NONE,
        min_grouping_digits: None,
        group_separator: None,
        minus_sign: None,
        protect_backticks: true,
        protect_between: Vec::new(),
    };
}

/// An immutable typographer: language, rules and settings fixed at construction.
///
/// A typographer with default settings is built by the `const fn` [`Typographer::new`] and fits
/// a `static`; custom settings go through [`Typographer::builder`]. The output kind is given with
/// every call. `Typographer` is `Send + Sync`: the working memory lives outside it.
///
/// ```
/// use std::borrow::Cow;
/// use typolish::{Language, Output, RuleSet, Typographer};
///
/// static RU_UI: Typographer = Typographer::new(Language::RU_RU, RuleSet::INTERFACE);
///
/// // Technical text is neither changed nor copied.
/// assert!(matches!(RU_UI.typeset("host:8080", Output::Unicode), Cow::Borrowed("host:8080")));
/// ```
#[derive(Clone)]
pub struct Typographer {
    language: Language,
    rules: RuleSet,
    settings: Settings,
    /// The active rules and the trigger set, computed once: by `build()`, or by the first call
    /// for a typographer of the `const fn` `new` (design section 3.2).
    compiled: OnceLock<Compiled>,
}

/// Language, rules and settings; the cache of `compiled` is left out, so the output does not change
/// after the first call (design section 2.4).
impl fmt::Debug for Typographer {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Typographer")
            .field("language", &self.language)
            .field("rules", &self.rules)
            .field("settings", &self.settings)
            .finish_non_exhaustive()
    }
}

/// What a typographer computes once from its language, rules and settings.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Compiled {
    /// The configured rules that the language supports (`active_rules`).
    pub(crate) active: RuleSet,
    /// The union of the trigger bytes of the active rules (design section 3.2).
    pub(crate) triggers: Triggers,
}

impl Typographer {
    /// A typographer with the language's defaults and backtick protection on. Usable in a
    /// `static`.
    ///
    /// ```
    /// use typolish::{Language, RuleSet, Typographer};
    ///
    /// static EN: Typographer = Typographer::new(Language::EN_US, RuleSet::PROSE);
    /// assert_eq!(EN.rules(), RuleSet::PROSE);
    /// ```
    #[must_use]
    pub const fn new(language: Language, rules: RuleSet) -> Typographer {
        Typographer {
            language,
            rules,
            settings: Settings::DEFAULT,
            compiled: OnceLock::new(),
        }
    }

    /// Starts a builder; `rules` is the base set.
    ///
    /// ```
    /// use typolish::{DashStyle, Language, Rule, RuleSet, Typographer};
    ///
    /// let t = Typographer::builder(Language::EN_US, RuleSet::INTERFACE)
    ///     .enable(Rule::NumberTimes)
    ///     .disable(Rule::NbspShortWords)
    ///     .dash_style(DashStyle::Spaced)
    ///     .add_units(["req/s", "vCPU"])
    ///     .protect_between("{{", "}}")
    ///     .build()?;
    /// assert!(t.rules().contains(Rule::NumberTimes));
    /// assert!(!t.rules().contains(Rule::NbspShortWords));
    /// # Ok::<(), typolish::Error>(())
    /// ```
    pub fn builder(language: Language, rules: RuleSet) -> TypographerBuilder {
        TypographerBuilder {
            language,
            base: rules,
            enabled: RuleSet::EMPTY,
            disabled: RuleSet::EMPTY,
            settings: Settings::DEFAULT,
        }
    }

    /// The language.
    ///
    /// ```
    /// use typolish::{Language, RuleSet, Typographer};
    ///
    /// let t = Typographer::new(Language::RU_RU, RuleSet::PROSE);
    /// assert_eq!(t.language(), &Language::RU_RU);
    /// ```
    #[must_use]
    pub fn language(&self) -> &Language {
        &self.language
    }

    /// The rules as configured.
    ///
    /// ```
    /// use typolish::{Language, RuleSet, Typographer};
    ///
    /// let t = Typographer::new(Language::COMMON, RuleSet::PROSE);
    /// assert_eq!(t.rules(), RuleSet::PROSE);
    /// ```
    #[must_use]
    pub fn rules(&self) -> RuleSet {
        self.rules
    }

    /// The configured rules that the language, with the settings of the builder, supports
    /// (design section 7.4); the others are inert.
    ///
    /// ```
    /// use typolish::{Language, Rule, RuleSet, Typographer};
    ///
    /// let t = Typographer::new(Language::COMMON, RuleSet::PROSE);
    /// assert!(t.active_rules().contains(Rule::EllipsisDots));
    /// assert!(!t.active_rules().contains(Rule::QuotesPairs));
    /// ```
    #[must_use]
    pub fn active_rules(&self) -> RuleSet {
        self.compiled().active
    }

    /// The active rules and the trigger set, computed on first use.
    pub(crate) fn compiled(&self) -> &Compiled {
        self.compiled.get_or_init(|| self.compile())
    }

    fn compile(&self) -> Compiled {
        let active: RuleSet = self
            .rules
            .iter()
            .filter(|rule| self.supports(*rule))
            .collect();
        let data = self.language.data();
        let percent = self.settings.percent != PercentSpacing::AsWritten;
        let triggers = active.iter().fold(Triggers::NONE, |set, rule| {
            set.union(rules::triggers(rule, data, percent))
        });
        Compiled { active, triggers }
    }

    /// Whether the text is a unit of the language with the builder's additions and removals,
    /// compared with case (design section 7.1).
    pub(crate) fn has_unit(&self, text: &str) -> bool {
        let units = &self.settings.units;
        let found = |list: &[String]| list.binary_search_by(|u| u.as_str().cmp(text)).is_ok();
        found(&units.added)
            || (self.language.data().units.binary_search(&text).is_ok() && !found(&units.removed))
    }

    /// Whether backticks protect (design section 10.1).
    /// The quote pairs in effect: the setting, or the language's.
    pub(crate) fn quote_pairs(&self) -> Option<QuotePairs> {
        self.settings.quote_pairs.or(self.language.data().quotes)
    }

    /// The `SingleQuotes` setting.
    pub(crate) fn single_quotes(&self) -> SingleQuotes {
        self.settings.single_quotes
    }

    /// The dash data in effect: the language's, with the style of the setting.
    pub(crate) fn dash_data(&self) -> Option<DashData> {
        match (self.language.data().dash, self.settings.dash_style) {
            (Some(dash), Some(style)) => Some(DashData::new(style, dash.dialogue)),
            (None, Some(style)) => Some(DashData::new(style, false)),
            (dash, None) => dash,
        }
    }

    pub(crate) fn protects_backticks(&self) -> bool {
        self.settings.protect_backticks
    }

    /// The own protection pairs (design section 10.1).
    pub(crate) fn protection_pairs(&self) -> &[(String, String)] {
        &self.settings.protect_between
    }

    /// Whether the language, with the settings, has the data a rule needs.
    fn supports(&self, rule: Rule) -> bool {
        let data = self.language.data();
        let s = &self.settings;
        let dash = self.dash_data();
        match rule {
            Rule::QuotesPairs => data.quotes.is_some() || s.quote_pairs.is_some(),
            Rule::DashDialogue => dash.is_some_and(|dash| dash.dialogue),
            Rule::DashDoubleHyphen | Rule::DashSpacedHyphen => dash.is_some(),
            Rule::NbspDash => dash.is_some_and(|dash| dash.style == DashStyle::SpacedGlued),
            Rule::NbspUnits => s.units.non_empty(data.units),
            Rule::NbspShortWords => s.short_words.non_empty(data.short_words),
            Rule::NbspParticles => s.particles.non_empty(data.particles),
            Rule::NbspShortLength => s.short_word_length.unwrap_or(data.short_word_length) > 0,
            _ => self.language.supports(rule),
        }
    }

    /// Typesets plain text. Returns the input borrowed when the output equals it byte for byte.
    ///
    /// Uses a thread-local working buffer; after the first call on a thread an unchanged string
    /// costs no allocation (design section 2.12).
    ///
    /// ```
    /// use std::borrow::Cow;
    /// use typolish::{Language, Output, RuleSet, Typographer};
    ///
    /// static RU_UI: Typographer = Typographer::new(Language::RU_RU, RuleSet::INTERFACE);
    ///
    /// assert!(matches!(RU_UI.typeset("host:8080", Output::Unicode), Cow::Borrowed(_)));
    /// ```
    #[must_use]
    pub fn typeset<'a>(&self, text: &'a str, output: Output) -> Cow<'a, str> {
        engine::scratch::with_thread_scratch(|scratch| engine::typeset(self, scratch, text, output))
    }

    /// The same as [`Typographer::typeset`] with working memory supplied by the caller.
    ///
    /// ```
    /// use typolish::{Language, Output, RuleSet, Scratch, Typographer};
    ///
    /// let t = Typographer::new(Language::EN_US, RuleSet::PROSE);
    /// let mut scratch = Scratch::new();
    /// assert_eq!(t.typeset_with(&mut scratch, "plain", Output::Unicode), "plain");
    /// ```
    #[must_use]
    pub fn typeset_with<'a>(
        &self,
        scratch: &mut Scratch,
        text: &'a str,
        output: Output,
    ) -> Cow<'a, str> {
        engine::typeset(self, scratch, text, output)
    }

    /// Appends the typeset text to `buf`; no allocation when `buf` has room.
    ///
    /// ```
    /// use typolish::{Language, Output, RuleSet, Typographer};
    ///
    /// let t = Typographer::new(Language::EN_US, RuleSet::PROSE);
    /// let mut buf = String::from("> ");
    /// t.typeset_into("plain", Output::Unicode, &mut buf);
    /// assert_eq!(buf, "> plain");
    /// ```
    pub fn typeset_into(&self, text: &str, output: Output, buf: &mut String) {
        engine::scratch::with_thread_scratch(|scratch| {
            engine::typeset_into(self, scratch, text, output, buf);
        });
    }

    /// Typesets the pieces of a message template; slots come out in input order, untouched, in
    /// the n + 1 form of [`Template`] (design section 2.6).
    ///
    /// ```
    /// use typolish::{Language, Output, Piece, RuleSet, Slot, Typographer};
    ///
    /// static RU_UI: Typographer = Typographer::new(Language::RU_RU, RuleSet::INTERFACE);
    ///
    /// let t = RU_UI.typeset_template(
    ///     [Piece::Text("Найдено "), Piece::Slot("n", Slot::NUMBER), Piece::Text(" файлов")],
    ///     Output::Unicode,
    /// );
    /// assert_eq!(t.texts().len(), t.slots().len() + 1);
    /// assert_eq!(t.slots()[0].0, "n");
    /// ```
    ///
    /// Allocates at most three vectors (the piece texts, the texts and the slots) plus one string
    /// per text that is changed or merged from more than one non-empty piece (design section
    /// 2.12). The vectors are sized from the lower bound of the iterator's `size_hint`; an iterator
    /// that reports less than its length makes them reallocate while they grow.
    #[must_use]
    pub fn typeset_template<'t, S>(
        &self,
        pieces: impl IntoIterator<Item = Piece<'t, S>>,
        output: Output,
    ) -> Template<'t, S> {
        // The pieces are the only caller code that runs during a call: they are collected before
        // the thread-local scratch is borrowed (design section 2.12).
        let (input, slots) = split_pieces(pieces);
        let slot_count = slots.len();
        let (texts, changed) = engine::scratch::with_thread_scratch(|scratch| {
            engine::typeset_template(self, scratch, &input, slot_count, output)
        });
        Template::from_parts(texts, slots, changed)
    }

    /// The same as [`Typographer::typeset_template`] with working memory supplied by the caller.
    ///
    /// ```
    /// use typolish::{Language, Output, Piece, RuleSet, Scratch, Slot, Typographer};
    ///
    /// let t = Typographer::new(Language::EN_US, RuleSet::INTERFACE);
    /// let mut scratch = Scratch::new();
    /// let out = t.typeset_template_with(&mut scratch, [Piece::Slot(7u8, Slot::OTHER)], Output::Unicode);
    /// assert_eq!(out.texts(), ["", ""]);
    /// ```
    #[must_use]
    pub fn typeset_template_with<'t, S>(
        &self,
        scratch: &mut Scratch,
        pieces: impl IntoIterator<Item = Piece<'t, S>>,
        output: Output,
    ) -> Template<'t, S> {
        let (input, slots) = split_pieces(pieces);
        let (texts, changed) = engine::typeset_template(self, scratch, &input, slots.len(), output);
        Template::from_parts(texts, slots, changed)
    }

    /// The same result as [`Typographer::typeset`], plus every edit, every protected span and
    /// every diagnostic (design sections 2.9 and 12.5).
    ///
    /// `report` always runs the whole pipeline, even on a string that no rule reacts to, so the
    /// diagnostics of protection and the classifier and the protected spans are complete (design
    /// section 3.2). It uses the thread-local working memory and makes no allocation promise.
    ///
    /// ```
    /// use typolish::{DiagnosticKind, Language, Output, ProtectedKind, RuleSet, TechKind, Typographer};
    ///
    /// static RU: Typographer = Typographer::new(Language::RU_RU, RuleSet::INTERFACE);
    ///
    /// let r = RU.report("Порт host:8080, см. `--config`", Output::Unicode);
    /// let kinds: Vec<_> = r.protected().iter().map(|s| s.kind()).collect();
    /// assert_eq!(kinds, [ProtectedKind::Technical(TechKind::Colon), ProtectedKind::Delimited]);
    ///
    /// let r = RU.report("don`t", Output::Unicode);
    /// assert_eq!(r.diagnostics()[0].kind(), &DiagnosticKind::UnclosedDelimiter);
    /// ```
    pub fn report<'a>(&self, text: &'a str, output: Output) -> Report<'a> {
        let (result, records) = engine::scratch::with_thread_scratch(|scratch| {
            engine::report(self, scratch, text, output)
        });
        Report::new(result, records, vec![text])
    }

    /// The same result as [`Typographer::typeset_template`], plus every edit, every protected
    /// span and every diagnostic; a span over several pieces gives one record per piece, and a
    /// diagnostic is reported at its first piece (design section 3.11).
    ///
    /// ```
    /// use typolish::{Language, Output, Piece, ProtectedKind, RuleSet, Typographer};
    ///
    /// static EN: Typographer = Typographer::new(Language::EN_US, RuleSet::INTERFACE);
    ///
    /// let r = EN.report_template::<()>([Piece::Text("Run `ls"), Piece::Text(" -la`")], Output::Unicode);
    /// let records: Vec<_> = r.protected().iter().map(|s| (s.piece(), s.range())).collect();
    /// assert_eq!(records, [(0, 4..7), (1, 0..5)]);
    /// assert!(r.protected().iter().all(|s| s.kind() == ProtectedKind::Delimited));
    /// ```
    pub fn report_template<'t, S>(
        &self,
        pieces: impl IntoIterator<Item = Piece<'t, S>>,
        output: Output,
    ) -> Report<'t, Template<'t, S>> {
        let (input, slots) = split_pieces(pieces);
        let slot_count = slots.len();
        let (texts, changed, records) = engine::scratch::with_thread_scratch(|scratch| {
            engine::report_template(self, scratch, &input, slot_count, output)
        });
        let inputs = input
            .iter()
            .map(|item| match *item {
                Input::Text(text) | Input::Protected(text) | Input::Markup(text) => text,
                Input::Slot(_) => "",
            })
            .collect();
        Report::new(Template::from_parts(texts, slots, changed), records, inputs)
    }
}

/// Splits pieces into the call-local vector of piece texts and the slots with their payloads.
///
/// Both vectors are sized once from the lower bound of `size_hint` (exact for arrays, vectors and
/// slices): there are never more slots than pieces, so neither reallocates for such an iterator
/// (design section 2.12).
fn split_pieces<'t, S>(
    pieces: impl IntoIterator<Item = Piece<'t, S>>,
) -> (Vec<Input<'t>>, Vec<(S, crate::Slot)>) {
    let pieces = pieces.into_iter();
    let (len, _) = pieces.size_hint();
    let mut input = Vec::with_capacity(len);
    let mut slots = Vec::with_capacity(len);
    for piece in pieces {
        input.push(match piece {
            Piece::Text(text) => Input::Text(text),
            Piece::Protected(text) => Input::Protected(text),
            Piece::Markup(text) => Input::Markup(text),
            Piece::Slot(payload, slot) => {
                slots.push((payload, slot));
                Input::Slot(slot)
            }
        });
    }
    (input, slots)
}

/// Settings of a [`Typographer`]; [`TypographerBuilder::build`] validates them.
///
/// The builder is order-independent: setting methods only record values, the active set is
/// `(base ∪ enabled) \ disabled`, and `build()` checks the values in a fixed order (design
/// section 2.4).
///
/// ```
/// use typolish::{Error, Language, Rule, RuleSet, Typographer};
///
/// let err = Typographer::builder(Language::RU_RU, RuleSet::PROSE)
///     .enable(Rule::NumberTimes)
///     .disable(Rule::NumberTimes)
///     .build()
///     .unwrap_err();
/// assert!(matches!(err, Error::RuleConflict { id: "number.times", .. }));
/// ```
#[derive(Clone, Debug)]
#[must_use]
pub struct TypographerBuilder {
    language: Language,
    base: RuleSet,
    enabled: RuleSet,
    disabled: RuleSet,
    settings: Settings,
}

impl TypographerBuilder {
    /// Adds a rule to the base set.
    pub fn enable(mut self, rule: Rule) -> Self {
        self.enabled = self.enabled.with(rule);
        self
    }

    /// Removes a rule from the base set.
    pub fn disable(mut self, rule: Rule) -> Self {
        self.disabled = self.disabled.with(rule);
        self
    }

    /// Sets the quote pairs instead of the language's, for example
    /// `QuotePairs::new(('«', '»'), ('„', '“'))`.
    ///
    /// Affects the rule `quotes.pairs`.
    pub fn quote_pairs(mut self, pairs: QuotePairs) -> Self {
        self.settings.quote_pairs = Some(pairs);
        self
    }

    /// Sets where straight single quotes become quotes; [`SingleQuotes::NestedOnly`] by default.
    ///
    /// Affects the rule `quotes.pairs`.
    pub fn single_quotes(mut self, mode: SingleQuotes) -> Self {
        self.settings.single_quotes = mode;
        self
    }

    /// Sets the dash style instead of the language's.
    ///
    /// Affects the rules `dash.double-hyphen`, `dash.spaced-hyphen` and `nbsp.dash`.
    pub fn dash_style(mut self, style: DashStyle) -> Self {
        self.settings.dash_style = Some(style);
        self
    }

    /// Sets the space before `%` and `‰`; [`PercentSpacing::AsWritten`] by default.
    ///
    /// Affects the rule `nbsp.units`.
    pub fn percent(mut self, spacing: PercentSpacing) -> Self {
        self.settings.percent = spacing;
        self
    }

    /// Adds units to the language's list (compared with case).
    ///
    /// Affects the rules `range.context` and `nbsp.units`.
    pub fn add_units<I>(mut self, units: I) -> Self
    where
        I: IntoIterator,
        I::Item: AsRef<str>,
    {
        extend(&mut self.settings.units.added, units);
        self
    }

    /// Removes units from the language's list.
    ///
    /// Affects the rules `range.context` and `nbsp.units`.
    pub fn remove_units<I>(mut self, units: I) -> Self
    where
        I: IntoIterator,
        I::Item: AsRef<str>,
    {
        extend(&mut self.settings.units.removed, units);
        self
    }

    /// Adds short words to the language's list (compared without case).
    ///
    /// Affects the rule `nbsp.short-words`.
    pub fn add_short_words<I>(mut self, words: I) -> Self
    where
        I: IntoIterator,
        I::Item: AsRef<str>,
    {
        extend(&mut self.settings.short_words.added, words);
        self
    }

    /// Removes short words from the language's list.
    ///
    /// Affects the rule `nbsp.short-words`.
    pub fn remove_short_words<I>(mut self, words: I) -> Self
    where
        I: IntoIterator,
        I::Item: AsRef<str>,
    {
        extend(&mut self.settings.short_words.removed, words);
        self
    }

    /// Sets the length up to which any word of the language's script is short; 0 makes the rule
    /// inert.
    ///
    /// Affects the rule `nbsp.short-length`.
    pub fn short_word_length(mut self, max_chars: u8) -> Self {
        self.settings.short_word_length = Some(max_chars);
        self
    }

    /// Sets whether short words of other scripts than the language's are glued too.
    ///
    /// Affects the rules `nbsp.short-words` and `nbsp.short-length`.
    pub fn short_words_any_script(mut self, yes: bool) -> Self {
        self.settings.short_words_any_script = yes;
        self
    }

    /// Adds particles to the language's list (compared without case).
    ///
    /// Affects the rule `nbsp.particles`.
    pub fn add_particles<I>(mut self, words: I) -> Self
    where
        I: IntoIterator,
        I::Item: AsRef<str>,
    {
        extend(&mut self.settings.particles.added, words);
        self
    }

    /// Removes particles from the language's list.
    ///
    /// Affects the rule `nbsp.particles`.
    pub fn remove_particles<I>(mut self, words: I) -> Self
    where
        I: IntoIterator,
        I::Item: AsRef<str>,
    {
        extend(&mut self.settings.particles.removed, words);
        self
    }

    /// Sets where glue is written as U+202F instead of U+00A0; [`NarrowGlue::NONE`] by default.
    ///
    /// Affects the rules `nbsp.abbreviations`, `nbsp.initials` and `nbsp.signs`.
    pub fn narrow_glue(mut self, places: NarrowGlue) -> Self {
        self.settings.narrow_glue = places;
        self
    }

    /// Sets the minimum grouping digits of `format_number` and `format_range`: from 1 to 4,
    /// numbers are grouped from `3 + digits` digits; 0 never groups.
    pub fn min_grouping_digits(mut self, digits: u8) -> Self {
        self.settings.min_grouping_digits = Some(digits);
        self
    }

    /// Sets the group separator of `format_number` and `format_range`.
    pub fn group_separator(mut self, separator: char) -> Self {
        self.settings.group_separator = Some(separator);
        self
    }

    /// Sets the minus sign of `format_number` and `format_range`.
    ///
    /// Affects the rule `number.minus`.
    pub fn minus_sign(mut self, minus: char) -> Self {
        self.settings.minus_sign = Some(minus);
        self
    }

    /// Turns protection by backticks on or off; on by default (design section 10.1).
    pub fn protect_backticks(mut self, on: bool) -> Self {
        self.settings.protect_backticks = on;
        self
    }

    /// Adds a pair of protection delimiters, for example `("{{", "}}")` (design section 10.1).
    pub fn protect_between(mut self, open: impl Into<String>, close: impl Into<String>) -> Self {
        self.settings
            .protect_between
            .push((open.into(), close.into()));
        self
    }

    /// Validates every setting (design section 2.11) and builds the typographer.
    ///
    /// The checks run in a fixed order and the first failing one is reported: rule conflicts,
    /// protection delimiters, quote pairs, the lists (units, short words, particles), then the
    /// group separator, the minus sign and the minimum grouping digits. Within a check, the
    /// offending value that is smallest in code point order is reported.
    ///
    /// ```
    /// use typolish::{Error, Language, RuleSet, Typographer};
    ///
    /// let err = Typographer::builder(Language::RU_RU, RuleSet::PROSE)
    ///     .protect_between("a1", "|")
    ///     .build()
    ///     .unwrap_err();
    /// assert!(matches!(err, Error::InvalidSetting { setting: "protect_between", .. }));
    /// ```
    pub fn build(self) -> Result<Typographer, Error> {
        let TypographerBuilder {
            language,
            base,
            enabled,
            disabled,
            mut settings,
        } = self;
        if let Some(rule) = enabled.intersection(disabled).iter().next() {
            return Err(Error::RuleConflict { id: rule.id() });
        }
        check_delimiters(&settings.protect_between)?;
        if let Some(pairs) = settings.quote_pairs {
            check_quotes(pairs)?;
        }
        normalize_list(&mut settings.units, "units", false)?;
        normalize_list(&mut settings.short_words, "short_words", true)?;
        normalize_list(&mut settings.particles, "particles", true)?;
        if let Some(c) = settings.group_separator.filter(|c| !is_valid_group(*c)) {
            return Err(invalid(
                "group_separator",
                format!(
                    "{:?} is a digit, a letter or whitespace other than U+00A0 and U+202F",
                    c
                ),
            ));
        }
        if let Some(c) = settings.minus_sign.filter(|c| !is_valid_minus(*c)) {
            return Err(invalid(
                "minus_sign",
                format!("{c:?} is a digit, a letter, whitespace, \" or '"),
            ));
        }
        if let Some(digits) = settings.min_grouping_digits.filter(|d| *d > 4) {
            return Err(invalid(
                "min_grouping_digits",
                format!("{digits} is above 4"),
            ));
        }
        let typographer = Typographer {
            language,
            rules: base.union(enabled).difference(disabled),
            settings,
            compiled: OnceLock::new(),
        };
        let compiled = typographer.compile();
        Ok(Typographer {
            compiled: OnceLock::from(compiled),
            ..typographer
        })
    }
}

fn extend<I>(list: &mut Vec<String>, items: I)
where
    I: IntoIterator,
    I::Item: AsRef<str>,
{
    list.extend(items.into_iter().map(|item| item.as_ref().to_owned()));
}

fn invalid(setting: &'static str, reason: String) -> Error {
    Error::InvalidSetting { setting, reason }
}

/// Reports the offending value that is smallest in code point order.
fn smallest<T: Ord + Clone>(offending: impl Iterator<Item = (T, String)>) -> Option<(T, String)> {
    offending.min_by(|a, b| a.0.cmp(&b.0))
}

/// Protection delimiters (design section 10.1): non-empty, no whitespace, no letter or digit;
/// not equal to or a prefix of a delimiter of another pair; neither delimiter of a pair a proper
/// prefix of the other.
fn check_delimiters(pairs: &[(String, String)]) -> Result<(), Error> {
    let mut offending = Vec::new();
    for (i, (open, close)) in pairs.iter().enumerate() {
        for (delimiter, other) in [(open, close), (close, open)] {
            let reason = if delimiter.is_empty() {
                Some("is empty")
            } else if delimiter.chars().any(char::is_whitespace) {
                Some("contains whitespace")
            } else if delimiter.chars().any(char::is_alphanumeric) {
                Some("contains a letter or a digit")
            } else if pairs.iter().enumerate().any(|(j, (o, c))| {
                j != i && (o.starts_with(delimiter.as_str()) || c.starts_with(delimiter.as_str()))
            }) {
                Some("is equal to or a prefix of a delimiter of another pair")
            } else if other.len() > delimiter.len() && other.starts_with(delimiter.as_str()) {
                Some("is a proper prefix of the other delimiter of its pair")
            } else {
                None
            };
            if let Some(reason) = reason {
                offending.push((
                    delimiter.clone(),
                    format!("delimiter `{delimiter}` {reason}"),
                ));
            }
        }
    }
    match smallest(offending.into_iter()) {
        Some((_, reason)) => Err(invalid("protect_between", reason)),
        None => Ok(()),
    }
}

/// Quote pairs (design section 7.6): four pairwise distinct quote glyphs.
fn check_quotes(pairs: QuotePairs) -> Result<(), Error> {
    let glyphs = pairs.glyphs();
    let offending = glyphs.iter().filter_map(|c| {
        if !QUOTE_GLYPHS.contains(c) {
            Some((
                *c,
                format!("{c:?} is not a quote character a language may use"),
            ))
        } else if glyphs.iter().filter(|other| *other == c).count() > 1 {
            Some((*c, format!("{c:?} appears more than once")))
        } else {
            None
        }
    });
    match smallest(offending) {
        Some((_, reason)) => Err(invalid("quote_pairs", reason)),
        None => Ok(()),
    }
}

/// Checks a list edit (design section 2.11), then folds the case of word lists, sorts and
/// removes duplicates.
fn normalize_list(edit: &mut ListEdit, setting: &'static str, fold: bool) -> Result<(), Error> {
    let normalize = |word: &String| -> String {
        if fold {
            word.chars().map(fold_char).collect()
        } else {
            word.clone()
        }
    };
    let added: Vec<String> = edit.added.iter().map(normalize).collect();
    let removed: Vec<String> = edit.removed.iter().map(normalize).collect();
    let offending = edit
        .added
        .iter()
        .zip(&added)
        .chain(edit.removed.iter().zip(&removed))
        .filter_map(|(word, normal)| {
            let reason = if word.is_empty() {
                "is empty"
            } else if word.chars().any(char::is_whitespace) {
                "contains whitespace"
            } else if setting == "units" && word.starts_with(|c: char| c.is_ascii_digit()) {
                "starts with an ASCII digit"
            } else if added.contains(normal) && removed.contains(normal) {
                "is both added and removed"
            } else {
                return None;
            };
            Some((word.clone(), format!("`{word}` {reason}")))
        });
    if let Some((_, reason)) = smallest(offending) {
        return Err(invalid(setting, reason));
    }
    edit.added = sorted(added);
    edit.removed = sorted(removed);
    Ok(())
}

fn sorted(mut list: Vec<String>) -> Vec<String> {
    list.sort();
    list.dedup();
    list
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::LanguageData;

    #[test]
    fn debug_output_does_not_show_the_cache() {
        // Design section 2.4: the write-once cache is not observable.
        let t = Typographer::new(Language::RU_RU, RuleSet::INTERFACE);
        let before = format!("{t:?}");
        let _ = t.typeset("a - b", Output::Unicode);
        assert!(t.compiled.get().is_some());
        assert_eq!(format!("{t:?}"), before);
        assert!(
            !before.contains("compiled") && !before.contains("OnceLock"),
            "{before}"
        );
        assert!(before.starts_with("Typographer { language: "), "{before}");
        let built = Typographer::builder(Language::RU_RU, RuleSet::INTERFACE)
            .build()
            .unwrap();
        assert_eq!(format!("{built:?}"), before);
    }

    #[test]
    fn builder_lists_are_normalized() {
        let t = Typographer::builder(Language::RU_RU, RuleSet::PROSE)
            .add_short_words(["Через", "через", "сквозь"])
            .add_units(["МБ/ч"])
            .build()
            .unwrap();
        assert_eq!(t.settings.short_words.added, ["сквозь", "через"]);
        assert_eq!(t.settings.units.added, ["МБ/ч"]);
    }

    #[test]
    fn builder_checks_in_a_fixed_order() {
        let err = Typographer::builder(Language::RU_RU, RuleSet::PROSE)
            .minus_sign('x')
            .add_units([""])
            .build()
            .unwrap_err();
        assert!(matches!(
            err,
            Error::InvalidSetting {
                setting: "units",
                ..
            }
        ));
        let err = Typographer::builder(Language::RU_RU, RuleSet::PROSE)
            .add_short_words(["в"])
            .remove_short_words(["В"])
            .build()
            .unwrap_err();
        assert_eq!(
            err,
            Error::InvalidSetting {
                setting: "short_words",
                reason: "`В` is both added and removed".to_owned()
            }
        );
    }

    #[test]
    fn settings_change_active_rules() {
        let common = Typographer::builder(Language::COMMON, RuleSet::PROSE)
            .dash_style(DashStyle::SpacedGlued)
            .add_short_words(["a"])
            .build()
            .unwrap();
        let active = common.active_rules();
        assert!(active.contains(Rule::DashSpacedHyphen));
        assert!(active.contains(Rule::NbspDash));
        assert!(active.contains(Rule::NbspShortWords));
        assert!(!active.contains(Rule::DashDialogue));
        let ru = Typographer::builder(Language::RU_RU, RuleSet::PROSE)
            .short_word_length(0)
            .build()
            .unwrap();
        assert!(!ru.active_rules().contains(Rule::NbspShortLength));
    }

    #[test]
    fn removing_every_unit_turns_the_rule_off() {
        let all = Typographer::builder(Language::RU_RU, RuleSet::PROSE)
            .remove_units(LanguageData::RU_RU.units.iter().rev())
            .build()
            .unwrap();
        assert!(!all.settings.units.non_empty(LanguageData::RU_RU.units));
        assert!(!all.active_rules().contains(Rule::NbspUnits));
        let some = Typographer::builder(Language::RU_RU, RuleSet::PROSE)
            .remove_units(LanguageData::RU_RU.units.iter().skip(1))
            .build()
            .unwrap();
        assert!(some.active_rules().contains(Rule::NbspUnits));
    }
}
