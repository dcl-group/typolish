//! Rules, their execution order and rule sets (design sections 2.3, 4.1 and 9).

use core::fmt;
use core::str::FromStr;

use crate::Error;

/// A typographic rule. The string id is stable across versions.
///
/// Variants are declared in the execution order of design section 4.1; [`Rule::ALL`] lists them
/// in that order. The order is not part of the API contract: a rule added later may go into the
/// middle.
///
/// ```
/// use typolish::Rule;
///
/// assert_eq!(Rule::NbspShortWords.id(), "nbsp.short-words");
/// assert_eq!("nbsp.short-words".parse::<Rule>(), Ok(Rule::NbspShortWords));
/// assert_eq!(Rule::NbspShortWords.to_string(), "nbsp.short-words");
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[non_exhaustive]
pub enum Rule {
    /// `symbols.copyright`: `(c)` next to a year, `(r)` and `(tm)` after a brand name.
    SymbolsCopyright,
    /// `symbols.arrows`: a lone `->`, `<-`, `<->` between spaces.
    SymbolsArrows,
    /// `symbols.math`: a lone `>=`, `<=`, `!=`, and `+-`.
    SymbolsMath,
    /// `ellipsis.dots`: `...` at a word end or alone, and after `?` and `!`.
    EllipsisDots,
    /// `ellipsis.inner`: `...` at a word start or between words.
    EllipsisInner,
    /// `dash.dialogue`: a hyphen at a line start that opens a line of dialogue.
    DashDialogue,
    /// `dash.double-hyphen`: a lone `--` or `---` between operands.
    DashDoubleHyphen,
    /// `dash.spaced-hyphen`: a lone `-` between operands.
    DashSpacedHyphen,
    /// `range.placeholders`: a hyphen between two number slots or two date slots.
    RangePlaceholders,
    /// `range.digits`: a hyphen between two ascending numbers.
    RangeDigits,
    /// `range.context`: a hyphen between numbers next to a unit, year, century or page word.
    RangeContext,
    /// `range.time`: a hyphen between two times of day.
    RangeTime,
    /// `number.minus`: a hyphen before a number that stands alone.
    NumberMinus,
    /// `number.times`: `x` between two numbers.
    NumberTimes,
    /// `quotes.apostrophe`: apostrophes inside and around words.
    QuotesApostrophe,
    /// `quotes.pairs`: straight quotes to the language's pairs by nesting level.
    QuotesPairs,
    /// `quotes.primes`: a quote after a number that is not a closing quote.
    QuotesPrimes,
    /// `nbsp.abbreviations`: glue inside and after listed abbreviations.
    NbspAbbreviations,
    /// `nbsp.initials`: glue between initials and a surname.
    NbspInitials,
    /// `nbsp.dates`: glue between a day and a month, and after a year or a century.
    NbspDates,
    /// `nbsp.units`: glue between a number and a unit.
    NbspUnits,
    /// `nbsp.signs`: glue after `№`, `§` and `¶`.
    NbspSigns,
    /// `nbsp.phone`: glue inside a telephone number.
    NbspPhone,
    /// `nbsp.short-words`: glue after a short word of the language's list.
    NbspShortWords,
    /// `nbsp.short-length`: glue after any short word of the language's script.
    NbspShortLength,
    /// `nbsp.particles`: glue before a particle of the language's list.
    NbspParticles,
    /// `nbsp.dash`: glue before a spaced dash.
    NbspDash,
}

/// The phase a rule runs in (design section 3.8).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub(crate) enum Phase {
    Symbols,
    Ellipsis,
    Dash,
    Range,
    Number,
    Quotes,
    Nbsp,
}

/// The execution order: the table of design section 4.1.
///
/// The engine walks this table; [`Rule::ALL`] lists the same rules in the same order.
pub(crate) const ORDER: [(Rule, Phase); 27] = [
    (Rule::SymbolsCopyright, Phase::Symbols),
    (Rule::SymbolsArrows, Phase::Symbols),
    (Rule::SymbolsMath, Phase::Symbols),
    (Rule::EllipsisDots, Phase::Ellipsis),
    (Rule::EllipsisInner, Phase::Ellipsis),
    (Rule::DashDialogue, Phase::Dash),
    (Rule::DashDoubleHyphen, Phase::Dash),
    (Rule::DashSpacedHyphen, Phase::Dash),
    (Rule::RangePlaceholders, Phase::Range),
    (Rule::RangeDigits, Phase::Range),
    (Rule::RangeContext, Phase::Range),
    (Rule::RangeTime, Phase::Range),
    (Rule::NumberMinus, Phase::Number),
    (Rule::NumberTimes, Phase::Number),
    (Rule::QuotesApostrophe, Phase::Quotes),
    (Rule::QuotesPairs, Phase::Quotes),
    (Rule::QuotesPrimes, Phase::Quotes),
    (Rule::NbspAbbreviations, Phase::Nbsp),
    (Rule::NbspInitials, Phase::Nbsp),
    (Rule::NbspDates, Phase::Nbsp),
    (Rule::NbspUnits, Phase::Nbsp),
    (Rule::NbspSigns, Phase::Nbsp),
    (Rule::NbspPhone, Phase::Nbsp),
    (Rule::NbspShortWords, Phase::Nbsp),
    (Rule::NbspShortLength, Phase::Nbsp),
    (Rule::NbspParticles, Phase::Nbsp),
    (Rule::NbspDash, Phase::Nbsp),
];

impl Rule {
    /// Every rule, in execution order (design section 4.1).
    ///
    /// ```
    /// use typolish::Rule;
    ///
    /// assert_eq!(Rule::ALL.len(), 27);
    /// assert_eq!(Rule::ALL[0], Rule::SymbolsCopyright);
    /// ```
    pub const ALL: &'static [Rule] = &[
        Rule::SymbolsCopyright,
        Rule::SymbolsArrows,
        Rule::SymbolsMath,
        Rule::EllipsisDots,
        Rule::EllipsisInner,
        Rule::DashDialogue,
        Rule::DashDoubleHyphen,
        Rule::DashSpacedHyphen,
        Rule::RangePlaceholders,
        Rule::RangeDigits,
        Rule::RangeContext,
        Rule::RangeTime,
        Rule::NumberMinus,
        Rule::NumberTimes,
        Rule::QuotesApostrophe,
        Rule::QuotesPairs,
        Rule::QuotesPrimes,
        Rule::NbspAbbreviations,
        Rule::NbspInitials,
        Rule::NbspDates,
        Rule::NbspUnits,
        Rule::NbspSigns,
        Rule::NbspPhone,
        Rule::NbspShortWords,
        Rule::NbspShortLength,
        Rule::NbspParticles,
        Rule::NbspDash,
    ];

    /// The stable id, for example `"nbsp.short-words"`.
    ///
    /// ```
    /// use typolish::Rule;
    ///
    /// assert_eq!(Rule::QuotesPairs.id(), "quotes.pairs");
    /// ```
    #[must_use]
    pub const fn id(self) -> &'static str {
        match self {
            Rule::SymbolsCopyright => "symbols.copyright",
            Rule::SymbolsArrows => "symbols.arrows",
            Rule::SymbolsMath => "symbols.math",
            Rule::EllipsisDots => "ellipsis.dots",
            Rule::EllipsisInner => "ellipsis.inner",
            Rule::DashDialogue => "dash.dialogue",
            Rule::DashDoubleHyphen => "dash.double-hyphen",
            Rule::DashSpacedHyphen => "dash.spaced-hyphen",
            Rule::RangePlaceholders => "range.placeholders",
            Rule::RangeDigits => "range.digits",
            Rule::RangeContext => "range.context",
            Rule::RangeTime => "range.time",
            Rule::NumberMinus => "number.minus",
            Rule::NumberTimes => "number.times",
            Rule::QuotesApostrophe => "quotes.apostrophe",
            Rule::QuotesPairs => "quotes.pairs",
            Rule::QuotesPrimes => "quotes.primes",
            Rule::NbspAbbreviations => "nbsp.abbreviations",
            Rule::NbspInitials => "nbsp.initials",
            Rule::NbspDates => "nbsp.dates",
            Rule::NbspUnits => "nbsp.units",
            Rule::NbspSigns => "nbsp.signs",
            Rule::NbspPhone => "nbsp.phone",
            Rule::NbspShortWords => "nbsp.short-words",
            Rule::NbspShortLength => "nbsp.short-length",
            Rule::NbspParticles => "nbsp.particles",
            Rule::NbspDash => "nbsp.dash",
        }
    }

    /// Looks a rule up by id; also accepts the retired ids of renamed rules (version 1 has none).
    ///
    /// ```
    /// use typolish::Rule;
    ///
    /// assert_eq!(Rule::from_id("dash.dialogue"), Some(Rule::DashDialogue));
    /// assert_eq!(Rule::from_id("dash.Dialogue"), None);
    /// ```
    #[must_use]
    pub fn from_id(id: &str) -> Option<Rule> {
        Rule::ALL.iter().copied().find(|rule| rule.id() == id)
    }

    /// One English line for logs and reports. The text is not part of the API contract.
    ///
    /// ```
    /// use typolish::Rule;
    ///
    /// assert!(!Rule::NbspDash.summary().is_empty());
    /// ```
    #[must_use]
    pub const fn summary(self) -> &'static str {
        match self {
            Rule::SymbolsCopyright => {
                "(c) next to a year to ©; (r), (tm) after a brand name to ®, ™"
            }
            Rule::SymbolsArrows => "a lone ->, <-, <-> between spaces to an arrow",
            Rule::SymbolsMath => "a lone >=, <=, != to ≥, ≤, ≠; +- to ±",
            Rule::EllipsisDots => {
                "... at a word end or alone to an ellipsis; ?... and !... by the language"
            }
            Rule::EllipsisInner => "... at a word start or between words to an ellipsis",
            Rule::DashDialogue => {
                "a hyphen at a line start before a word to a dash with glue after it"
            }
            Rule::DashDoubleHyphen => {
                "a lone -- or --- between operands to a dash in the language's style"
            }
            Rule::DashSpacedHyphen => "a lone - between operands to a dash in the language's style",
            Rule::RangePlaceholders => {
                "a hyphen between two number slots or two date slots to a range dash"
            }
            Rule::RangeDigits => "a hyphen between two ascending numbers to a range dash",
            Rule::RangeContext => {
                "a hyphen between numbers next to a unit, year, century or page word to a range dash"
            }
            Rule::RangeTime => "a hyphen between two times of day to a range dash",
            Rule::NumberMinus => "a hyphen before a number that stands alone to a minus sign",
            Rule::NumberTimes => "x between two numbers to ×",
            Rule::QuotesApostrophe => "a straight quote inside or around a word to an apostrophe",
            Rule::QuotesPairs => "straight quotes to the language's pairs by nesting level",
            Rule::QuotesPrimes => "a quote after a number that is not a closing quote to a prime",
            Rule::NbspAbbreviations => "glue inside and after listed abbreviations",
            Rule::NbspInitials => "glue between initials and a surname",
            Rule::NbspDates => "glue between a day and a month, after a year and after a century",
            Rule::NbspUnits => "glue between a number and a unit; percent by setting",
            Rule::NbspSigns => "glue after №, § and ¶",
            Rule::NbspPhone => "glue inside a telephone number",
            Rule::NbspShortWords => "glue after a short word of the language's list",
            Rule::NbspShortLength => "glue after any short word of the language's script",
            Rule::NbspParticles => "glue before a particle of the language's list",
            Rule::NbspDash => "glue before a spaced dash between operands",
        }
    }

    /// The bit of the rule in a [`RuleSet`].
    const fn bit(self) -> u128 {
        1u128 << (self as u8)
    }
}

/// Reads a rule id; the same as [`Rule::from_id`] with an error.
impl FromStr for Rule {
    type Err = Error;

    fn from_str(s: &str) -> Result<Rule, Error> {
        Rule::from_id(s).ok_or_else(|| Error::UnknownRule { id: s.to_owned() })
    }
}

/// Writes the id.
impl fmt::Display for Rule {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.id())
    }
}

/// A set of rules. Execution order never depends on how the set was built.
///
/// A set is built in code (usable in `const`), from a string, or through
/// [`TypographerBuilder::enable`](crate::TypographerBuilder::enable) and
/// [`disable`](crate::TypographerBuilder::disable).
///
/// ```
/// use typolish::{Rule, RuleSet};
///
/// const UI: RuleSet = RuleSet::INTERFACE.with(Rule::NumberTimes);
/// assert!(UI.contains(Rule::NumberTimes));
///
/// let parsed: RuleSet = "interface,+number.times".parse()?;
/// assert_eq!(parsed, UI);
/// assert_eq!(parsed.to_string().parse::<RuleSet>()?, parsed);
/// # Ok::<(), typolish::Error>(())
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
pub struct RuleSet {
    bits: u128,
}

impl RuleSet {
    /// No rule.
    ///
    /// ```
    /// use typolish::RuleSet;
    ///
    /// assert!(RuleSet::EMPTY.is_empty());
    /// assert_eq!(RuleSet::EMPTY.to_string(), "empty");
    /// ```
    pub const EMPTY: RuleSet = RuleSet { bits: 0 };

    /// Every rule of the running version.
    ///
    /// A rule added in a later release joins this set, so its output may change with
    /// `cargo update`; callers who need stable output use [`RuleSet::PROSE`],
    /// [`RuleSet::INTERFACE`] or an explicit list of rules (design section 15.1).
    ///
    /// ```
    /// use typolish::{Rule, RuleSet};
    ///
    /// assert_eq!(RuleSet::ALL.len(), Rule::ALL.len());
    /// ```
    pub const ALL: RuleSet = RuleSet::from_slice(Rule::ALL);

    /// Running text written for reading: every rule of version 1 (design section 9).
    ///
    /// Rules: `symbols.copyright`, `symbols.arrows`, `symbols.math`, `ellipsis.dots`,
    /// `ellipsis.inner`, `dash.dialogue`, `dash.double-hyphen`, `dash.spaced-hyphen`,
    /// `range.placeholders`, `range.digits`, `range.context`, `range.time`, `number.minus`,
    /// `number.times`, `quotes.apostrophe`, `quotes.pairs`, `quotes.primes`,
    /// `nbsp.abbreviations`, `nbsp.initials`, `nbsp.dates`, `nbsp.units`, `nbsp.signs`,
    /// `nbsp.phone`, `nbsp.short-words`, `nbsp.short-length`, `nbsp.particles`, `nbsp.dash`.
    ///
    /// The composition changes only in a release that raises the breaking number.
    ///
    /// ```
    /// use typolish::RuleSet;
    ///
    /// assert_eq!(RuleSet::PROSE.len(), 27);
    /// ```
    pub const PROSE: RuleSet = RuleSet::from_slice(&[
        Rule::SymbolsCopyright,
        Rule::SymbolsArrows,
        Rule::SymbolsMath,
        Rule::EllipsisDots,
        Rule::EllipsisInner,
        Rule::DashDialogue,
        Rule::DashDoubleHyphen,
        Rule::DashSpacedHyphen,
        Rule::RangePlaceholders,
        Rule::RangeDigits,
        Rule::RangeContext,
        Rule::RangeTime,
        Rule::NumberMinus,
        Rule::NumberTimes,
        Rule::QuotesApostrophe,
        Rule::QuotesPairs,
        Rule::QuotesPrimes,
        Rule::NbspAbbreviations,
        Rule::NbspInitials,
        Rule::NbspDates,
        Rule::NbspUnits,
        Rule::NbspSigns,
        Rule::NbspPhone,
        Rule::NbspShortWords,
        Rule::NbspShortLength,
        Rule::NbspParticles,
        Rule::NbspDash,
    ]);

    /// UI strings, error messages and technical text: the rules that cannot alter a technical
    /// token standing as its own fragment (design section 9).
    ///
    /// Rules: `ellipsis.dots`, `dash.spaced-hyphen`, `range.placeholders`, `quotes.apostrophe`,
    /// `quotes.pairs`, `nbsp.abbreviations`, `nbsp.initials`, `nbsp.dates`, `nbsp.units`,
    /// `nbsp.signs`, `nbsp.short-words`, `nbsp.particles`, `nbsp.dash`.
    ///
    /// The composition changes only in a release that raises the breaking number.
    ///
    /// ```
    /// use typolish::{Rule, RuleSet};
    ///
    /// assert_eq!(RuleSet::INTERFACE.len(), 13);
    /// assert!(!RuleSet::INTERFACE.contains(Rule::DashDoubleHyphen));
    /// ```
    pub const INTERFACE: RuleSet = RuleSet::from_slice(&[
        Rule::EllipsisDots,
        Rule::DashSpacedHyphen,
        Rule::RangePlaceholders,
        Rule::QuotesApostrophe,
        Rule::QuotesPairs,
        Rule::NbspAbbreviations,
        Rule::NbspInitials,
        Rule::NbspDates,
        Rule::NbspUnits,
        Rule::NbspSigns,
        Rule::NbspShortWords,
        Rule::NbspParticles,
        Rule::NbspDash,
    ]);

    /// The base names that [`RuleSet::from_str`] accepts as the first item.
    const BASES: [(&'static str, RuleSet); 4] = [
        ("prose", RuleSet::PROSE),
        ("interface", RuleSet::INTERFACE),
        ("empty", RuleSet::EMPTY),
        ("all", RuleSet::ALL),
    ];

    const fn from_slice(rules: &[Rule]) -> RuleSet {
        let mut set = RuleSet::EMPTY;
        let mut rest = rules;
        while let [first, tail @ ..] = rest {
            set = set.with(*first);
            rest = tail;
        }
        set
    }

    /// The set with `rule` added.
    ///
    /// ```
    /// use typolish::{Rule, RuleSet};
    ///
    /// assert!(RuleSet::EMPTY.with(Rule::NbspDash).contains(Rule::NbspDash));
    /// ```
    #[must_use]
    pub const fn with(self, rule: Rule) -> RuleSet {
        RuleSet {
            bits: self.bits | rule.bit(),
        }
    }

    /// The set with `rule` removed.
    ///
    /// ```
    /// use typolish::{Rule, RuleSet};
    ///
    /// assert!(!RuleSet::PROSE.without(Rule::NbspDash).contains(Rule::NbspDash));
    /// ```
    #[must_use]
    pub const fn without(self, rule: Rule) -> RuleSet {
        RuleSet {
            bits: self.bits & !rule.bit(),
        }
    }

    /// Whether `rule` is in the set.
    ///
    /// ```
    /// use typolish::{Rule, RuleSet};
    ///
    /// assert!(RuleSet::INTERFACE.contains(Rule::QuotesPairs));
    /// ```
    #[must_use]
    pub const fn contains(self, rule: Rule) -> bool {
        self.bits & rule.bit() != 0
    }

    /// Every rule of either set.
    ///
    /// ```
    /// use typolish::RuleSet;
    ///
    /// assert_eq!(RuleSet::INTERFACE.union(RuleSet::PROSE), RuleSet::PROSE);
    /// ```
    #[must_use]
    pub const fn union(self, other: RuleSet) -> RuleSet {
        RuleSet {
            bits: self.bits | other.bits,
        }
    }

    /// The rules of `self` that are not in `other`.
    ///
    /// ```
    /// use typolish::RuleSet;
    ///
    /// assert_eq!(RuleSet::PROSE.difference(RuleSet::PROSE), RuleSet::EMPTY);
    /// ```
    #[must_use]
    pub const fn difference(self, other: RuleSet) -> RuleSet {
        RuleSet {
            bits: self.bits & !other.bits,
        }
    }

    /// The rules of both sets.
    pub(crate) const fn intersection(self, other: RuleSet) -> RuleSet {
        RuleSet {
            bits: self.bits & other.bits,
        }
    }

    /// The number of rules in the set.
    ///
    /// ```
    /// use typolish::{Rule, RuleSet};
    ///
    /// assert_eq!(RuleSet::EMPTY.with(Rule::NbspDash).len(), 1);
    /// ```
    #[must_use]
    pub const fn len(self) -> usize {
        self.bits.count_ones() as usize
    }

    /// Whether the set has no rule.
    ///
    /// ```
    /// use typolish::RuleSet;
    ///
    /// assert!(!RuleSet::INTERFACE.is_empty());
    /// ```
    #[must_use]
    pub const fn is_empty(self) -> bool {
        self.bits == 0
    }

    /// The rules of the set in execution order.
    ///
    /// ```
    /// use typolish::{Rule, RuleSet};
    ///
    /// let set = RuleSet::EMPTY.with(Rule::NbspDash).with(Rule::EllipsisDots);
    /// assert_eq!(set.iter().collect::<Vec<_>>(), [Rule::EllipsisDots, Rule::NbspDash]);
    /// ```
    pub fn iter(self) -> impl Iterator<Item = Rule> {
        Rule::ALL
            .iter()
            .copied()
            .filter(move |rule| self.contains(*rule))
    }
}

impl FromIterator<Rule> for RuleSet {
    fn from_iter<I: IntoIterator<Item = Rule>>(iter: I) -> RuleSet {
        let mut set = RuleSet::EMPTY;
        set.extend(iter);
        set
    }
}

impl Extend<Rule> for RuleSet {
    fn extend<I: IntoIterator<Item = Rule>>(&mut self, iter: I) {
        for rule in iter {
            *self = self.with(rule);
        }
    }
}

/// Reads `"interface,+number.times,-nbsp.initials"`: an optional base set name first (`prose`,
/// `interface`, `empty`, `all`; `empty` when absent), then ids, each optionally prefixed by `+`
/// (add) or `-` (remove). The result is `(base ∪ added) \ removed` (design section 2.3).
impl FromStr for RuleSet {
    type Err = Error;

    fn from_str(s: &str) -> Result<RuleSet, Error> {
        let invalid = |reason: &'static str| Error::InvalidRuleSet {
            input: s.to_owned(),
            reason,
        };
        let mut base = RuleSet::EMPTY;
        let mut added = RuleSet::EMPTY;
        let mut removed = RuleSet::EMPTY;
        for (index, raw) in s.split(',').enumerate() {
            let item = raw.trim();
            let (sign, name) = match item.as_bytes().first() {
                Some(b'+') => (Some(true), item.get(1..).unwrap_or("")),
                Some(b'-') => (Some(false), item.get(1..).unwrap_or("")),
                _ => (None, item),
            };
            if let Some((_, set)) = RuleSet::BASES.iter().find(|(base, _)| *base == name) {
                if index > 0 {
                    return Err(invalid("base set name after the first item"));
                }
                if sign.is_some() {
                    return Err(invalid("sign before a base set name"));
                }
                base = *set;
                continue;
            }
            if name.is_empty() {
                return Err(invalid("empty item"));
            }
            let rule = name.parse::<Rule>()?;
            if sign == Some(false) {
                removed = removed.with(rule);
            } else {
                added = added.with(rule);
            }
        }
        if !added.intersection(removed).is_empty() {
            return Err(invalid("rule both added and removed"));
        }
        Ok(base.union(added).difference(removed))
    }
}

/// Writes the ids in execution order, comma-separated, or `empty` for the empty set.
impl fmt::Display for RuleSet {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.is_empty() {
            return f.write_str("empty");
        }
        for (i, rule) in self.iter().enumerate() {
            if i > 0 {
                f.write_str(",")?;
            }
            f.write_str(rule.id())?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn order_and_all_agree() {
        assert_eq!(ORDER.len(), Rule::ALL.len());
        for (i, (rule, _)) in ORDER.iter().enumerate() {
            assert_eq!(Rule::ALL[i], *rule);
            assert_eq!(*rule as usize, i, "declaration order is execution order");
        }
        let mut phases: Vec<Phase> = ORDER.iter().map(|(_, phase)| *phase).collect();
        let sorted = {
            let mut p = phases.clone();
            p.sort();
            p
        };
        assert_eq!(phases, sorted, "phases are contiguous and in order");
        phases.dedup();
        assert_eq!(phases.len(), 7);
    }

    #[test]
    fn bits_fit() {
        assert!(Rule::ALL.iter().all(|rule| (*rule as u32) < u128::BITS));
        assert_eq!(RuleSet::ALL.len(), Rule::ALL.len());
    }
}
