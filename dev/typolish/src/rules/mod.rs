//! The rules (design section 8). So far each rule file declares the trigger bytes of its rules
//! (design section 3.2); the rule functions arrive with the phases of stage 2.

pub(crate) mod dash;
pub(crate) mod ellipsis;
pub(crate) mod nbsp;
pub(crate) mod number;
pub(crate) mod quotes;
pub(crate) mod range;
pub(crate) mod symbols;

use crate::Rule;
use crate::lang::data::LanguageData;

/// A set of bytes: the trigger bytes of rules and the bytes an output kind encodes (design
/// section 3.2).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub(crate) struct Triggers([u64; 4]);

impl Triggers {
    /// The empty set.
    pub(crate) const NONE: Triggers = Triggers([0; 4]);
    /// The ASCII digits.
    pub(crate) const DIGITS: Triggers = Triggers::of(b"0123456789");
    /// The lead bytes of the typographic characters that rules use as context: U+00A0, `«`, `»`
    /// (0xC2) and U+2000 to U+2FFF with the dashes, quotes, `…` and U+202F (0xE2).
    pub(crate) const TYPOGRAPHIC: Triggers = Triggers::of(&[0xc2, 0xe2]);

    /// The set of the given bytes.
    pub(crate) const fn of(bytes: &[u8]) -> Triggers {
        let mut set = Triggers::NONE;
        let mut rest = bytes;
        while let [first, tail @ ..] = rest {
            set = set.with(*first);
            rest = tail;
        }
        set
    }

    /// The set with one more byte.
    pub(crate) const fn with(self, byte: u8) -> Triggers {
        let bit = 1u64 << (byte & 63);
        let [a, b, c, d] = self.0;
        Triggers(match byte >> 6 {
            0 => [a | bit, b, c, d],
            1 => [a, b | bit, c, d],
            2 => [a, b, c | bit, d],
            _ => [a, b, c, d | bit],
        })
    }

    /// The set with the lead byte of the UTF-8 form of `c`.
    pub(crate) const fn with_lead(self, c: char) -> Triggers {
        let code = c as u32;
        // The lead byte of a UTF-8 sequence always fits a `u8`.
        #[expect(clippy::cast_possible_truncation, reason = "each value is below 256")]
        let lead = match code {
            0..0x80 => code as u8,
            0x80..0x800 => (0xc0 | (code >> 6)) as u8,
            0x800..0x10000 => (0xe0 | (code >> 12)) as u8,
            _ => (0xf0 | (code >> 18)) as u8,
        };
        self.with(lead)
    }

    /// The union of two sets.
    pub(crate) const fn union(self, other: Triggers) -> Triggers {
        let [a, b, c, d] = self.0;
        let [e, f, g, h] = other.0;
        Triggers([a | e, b | f, c | g, d | h])
    }

    /// Whether the set holds `byte`.
    pub(crate) const fn contains(self, byte: u8) -> bool {
        let [a, b, c, d] = self.0;
        let word = match byte >> 6 {
            0 => a,
            1 => b,
            2 => c,
            _ => d,
        };
        word & (1 << (byte & 63)) != 0
    }

    /// Whether any byte of `bytes` is in the set; a linear scan without allocation.
    pub(crate) fn any_in(self, bytes: &[u8]) -> bool {
        bytes.iter().any(|b| self.contains(*b))
    }
}

/// The trigger bytes of a rule for a language and the percent setting (design section 3.2).
pub(crate) fn triggers(rule: Rule, data: &LanguageData, percent_edits: bool) -> Triggers {
    match rule {
        Rule::SymbolsCopyright => symbols::COPYRIGHT,
        Rule::SymbolsArrows => symbols::ARROWS,
        Rule::SymbolsMath => symbols::MATH,
        Rule::EllipsisDots => ellipsis::DOTS,
        Rule::EllipsisInner => ellipsis::INNER,
        Rule::DashDialogue => dash::DIALOGUE,
        Rule::DashDoubleHyphen => dash::DOUBLE_HYPHEN,
        Rule::DashSpacedHyphen => dash::SPACED_HYPHEN,
        Rule::RangePlaceholders => range::PLACEHOLDERS,
        Rule::RangeDigits => range::DIGITS,
        Rule::RangeContext => range::CONTEXT,
        Rule::RangeTime => range::TIME,
        Rule::NumberMinus => number::MINUS,
        Rule::NumberTimes => number::TIMES,
        Rule::QuotesApostrophe => quotes::APOSTROPHE,
        Rule::QuotesPairs => quotes::PAIRS,
        Rule::QuotesPrimes => quotes::PRIMES,
        Rule::NbspAbbreviations => nbsp::ABBREVIATIONS,
        Rule::NbspInitials => nbsp::INITIALS,
        Rule::NbspDates => nbsp::DATES,
        Rule::NbspUnits if percent_edits => nbsp::UNITS.union(nbsp::PERCENT),
        Rule::NbspUnits => nbsp::UNITS,
        Rule::NbspSigns => data
            .signs
            .iter()
            .fold(nbsp::SIGNS, |set, sign| set.with_lead(*sign)),
        Rule::NbspPhone => nbsp::PHONE,
        Rule::NbspShortWords => nbsp::SHORT_WORDS,
        Rule::NbspShortLength => nbsp::SHORT_LENGTH,
        Rule::NbspParticles => nbsp::PARTICLES,
        Rule::NbspDash => nbsp::DASH,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sets_hold_their_bytes() {
        let set = Triggers::of(b"\"-").with_lead('№');
        assert_eq!(Triggers::NONE.with_lead('я'), Triggers::of(&[0xd1]));
        assert_eq!(Triggers::NONE.with_lead('😀'), Triggers::of(&[0xf0]));
        assert_eq!(Triggers::NONE.with_lead('«'), Triggers::of(&[0xc2]));
        assert!(set.contains(b'"') && set.contains(b'-') && set.contains(0xe2));
        assert!(!set.contains(b'a') && !set.contains(0xd0));
        assert!(set.any_in("a-b".as_bytes()));
        assert!(!set.any_in("Москва".as_bytes()));
        assert_eq!(Triggers::NONE.union(set), set);
        assert!(Triggers::of(&[255, 0, 64, 128]).contains(255));
    }
}
