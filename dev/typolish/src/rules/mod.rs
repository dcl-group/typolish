//! The rules (design section 8): `RULES`, the table of rule functions in the order of design
//! section 4.1, the trigger bytes of every rule (design section 3.2), and the conditions that
//! several rules share (design section 8.2). The rules `symbols.*`, `number.minus` and `nbsp.*`
//! declare their trigger bytes and match nothing yet.

pub(crate) mod dash;
pub(crate) mod ellipsis;
pub(crate) mod nbsp;
pub(crate) mod number;
pub(crate) mod quotes;
pub(crate) mod range;
pub(crate) mod symbols;

use crate::Rule;
pub(crate) use crate::engine::classify::{Frag, FragKind};
use crate::engine::lex::{Kind, script_bit};
use crate::engine::phase::{Cx, Match};
use crate::lang::data::{List, Script, Table, fold_char};

/// A rule: reads the input of its phase at an anchor token and proposes a match (design section
/// 3.8).
pub(crate) type RuleFn = fn(&Cx<'_>, usize) -> Option<Match>;

/// A rule that matches nothing yet (`symbols.*`, `number.minus`, `nbsp.*`).
fn later(_cx: &Cx<'_>, _i: usize) -> Option<Match> {
    None
}

/// The rule functions, indexed by `Rule as usize`, in the order of `ORDER` (design section 4.1).
pub(crate) const RULES: [RuleFn; 27] = [
    later,
    later,
    later,
    ellipsis::dots,
    ellipsis::inner,
    dash::dialogue,
    dash::double_hyphen,
    dash::spaced_hyphen,
    range::placeholders,
    range::digits,
    range::context,
    range::time,
    later,
    number::times,
    quotes::apostrophe,
    quotes::pairs,
    quotes::primes,
    later,
    later,
    later,
    later,
    later,
    later,
    later,
    later,
    later,
    later,
];

/// A word (design section 8.2): a `Word` core, a one-word `Dictionary` core or a compound word.
pub(crate) fn is_word(cx: &Cx<'_>, f: &Frag) -> bool {
    matches!(cx.kind_of(f), FragKind::Word | FragKind::Compound)
}

/// A number: a `Number` fragment or a `Number` slot (labels and percentages included).
pub(crate) fn is_number(f: &Frag) -> bool {
    f.kind == FragKind::Number
}

/// A range: a `Range` or `YearMonth` core.
pub(crate) fn is_range(f: &Frag) -> bool {
    matches!(f.kind, FragKind::Range | FragKind::YearMonth)
}

/// A technical fragment (design section 3.6): its core is frozen, whole or in part, and it is not
/// a compound word.
pub(crate) fn is_technical(f: &Frag) -> bool {
    f.kind.tech().is_some() && f.kind != FragKind::Compound
}

/// An operand (design section 8.2): a word, a number, a range, a slot, a protected span or a
/// technical fragment. A slot or a protected span alone is a `Word` or `Number` core.
pub(crate) fn is_operand(cx: &Cx<'_>, f: &Frag) -> bool {
    is_word(cx, f) || is_number(f) || is_range(f) || is_technical(f)
}

/// A word of one letter (`a`, `x`); a slot or a compound word is not one.
pub(crate) fn single_letter(cx: &Cx<'_>, f: &Frag) -> bool {
    if cx.kind_of(f) != FragKind::Word {
        return false;
    }
    let mut core = cx.core(f);
    match (core.next(), core.next()) {
        (Some(i), None) if matches!(cx.kind(i), Some(Kind::Word(_))) => {
            cx.visible(i, i).count() == 1
        }
        _ => false,
    }
}

/// The scripts of the letters of a core, as bits of `script_bit`.
fn scripts(cx: &Cx<'_>, f: &Frag) -> u8 {
    cx.core(f)
        .filter(|&i| matches!(cx.kind(i), Some(Kind::Word(_))))
        .flat_map(|i| cx.visible(i, i))
        .filter(|c| c.is_alphabetic())
        .fold(0, |mask, c| mask | script_bit(c))
}

/// The bit of the language's script; `None` for `Script::Other`.
fn language_script(data: &Table<'_>) -> Option<u8> {
    match data.script {
        Script::Cyrillic => Some(1),
        Script::Latin => Some(2),
        Script::Greek => Some(4),
        _ => None,
    }
}

/// A word outside the language's script (design section 8.2): none of its letters is of the
/// script; a compound word without letters counts as one in every language. A word of two
/// scripts is a word of each.
pub(crate) fn foreign(cx: &Cx<'_>, f: &Frag) -> bool {
    if !is_word(cx, f) {
        return false;
    }
    let mask = scripts(cx, f);
    if mask == 0 {
        return f.kind == FragKind::Compound
            && !cx
                .core(f)
                .any(|i| matches!(cx.kind(i), Some(Kind::Word(_))));
    }
    language_script(&cx.data).is_some_and(|bit| mask & bit == 0)
}

/// Whether a lowercase word list holds the characters of `key`, compared without case (design
/// section 7.1).
pub(crate) fn folded_in(list: List<'_>, key: impl Iterator<Item = char> + Clone) -> bool {
    list.binary_search_by(|entry| entry.chars().cmp(key.clone().map(fold_char)))
        .is_ok()
}

/// Calls `f` with the visible text of a core, followed by a full stop when `stop` is set, without
/// allocation; a core longer than any list entry gives `false`.
pub(crate) fn with_key(
    chars: impl Iterator<Item = char>,
    stop: bool,
    f: impl FnOnce(&str) -> bool,
) -> bool {
    let mut buf = [0u8; 96];
    let mut len = 0;
    for c in chars.chain(stop.then_some('.')) {
        let Some(slot) = buf.get_mut(len..len + c.len_utf8()) else {
            return false;
        };
        c.encode_utf8(slot);
        len += c.len_utf8();
    }
    match buf.get(..len).map(core::str::from_utf8) {
        Some(Ok(key)) => f(key),
        _ => false,
    }
}

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
pub(crate) fn triggers(rule: Rule, data: &Table<'_>, percent_edits: bool) -> Triggers {
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
    fn the_table_follows_the_order() {
        // `RULES` is indexed by `Rule as usize`, which follows `ORDER` (design section 4.1).
        for (k, (rule, _)) in crate::rule::ORDER.iter().enumerate() {
            assert_eq!(*rule as usize, k);
        }
        assert_eq!(RULES.len(), crate::rule::ORDER.len());
    }

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
