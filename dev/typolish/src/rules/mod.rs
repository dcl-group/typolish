//! The rules (design section 8): `RULES`, the table of rule functions in the order of design
//! section 4.1, the trigger bytes of every rule (design section 3.2), and the conditions that
//! several rules share (design section 8.2).

pub(crate) mod dash;
pub(crate) mod ellipsis;
pub(crate) mod nbsp;
pub(crate) mod number;
pub(crate) mod quotes;
pub(crate) mod range;
pub(crate) mod symbols;

use crate::Rule;
pub(crate) use crate::engine::classify::{Frag, FragKind};
use crate::engine::lex::{Kind, is_alphabetic, script_bit};
use crate::engine::phase::{Cx, Match};
use crate::lang::data::{List, Script, Table, fold_char};

/// A rule: reads the input of its phase at an anchor token and proposes a match (design section
/// 3.8).
pub(crate) type RuleFn = fn(&Cx<'_>, usize) -> Option<Match>;

/// The rule functions, indexed by `Rule as usize`, in the order of `ORDER` (design section 4.1).
pub(crate) const RULES: [RuleFn; 27] = [
    symbols::copyright,
    symbols::arrows,
    symbols::math,
    ellipsis::dots,
    ellipsis::inner,
    dash::dialogue,
    dash::double_hyphen,
    dash::spaced_hyphen,
    range::placeholders,
    range::digits,
    range::context,
    range::time,
    number::minus,
    number::times,
    quotes::apostrophe,
    quotes::pairs,
    quotes::primes,
    nbsp::abbreviations,
    nbsp::initials_rule,
    nbsp::dates,
    nbsp::units,
    nbsp::signs,
    nbsp::phone,
    nbsp::short_words,
    nbsp::short_length,
    nbsp::particles,
    nbsp::dash,
];

/// The kinds of token at which each rule of `RULES` can match, as bits of `Kind::bit`: every rule
/// returns `None` at a token of any other kind, so the phase does not call it there (design
/// section 3.8). The quote rules read the decisions of the quote phase, which only `Quote` tokens
/// get. A unit test of `engine::phase` calls the rules at the other kinds too.
pub(crate) const ANCHORS: [u32; 27] = {
    use crate::engine::lex::{Mark, Op, WordScript};
    let space = Kind::Space.bit();
    let dash = Kind::Dash.bit();
    let operator = Kind::Operator(Op::Equal).bit();
    let punct = Kind::Punct(' ').bit();
    [
        Kind::LegalMark(Mark::Copyright).bit(),
        operator,
        operator,
        Kind::Dots.bit(),
        Kind::Dots.bit(),
        dash,
        space,
        space,
        dash,
        dash,
        dash,
        dash,
        dash,
        Kind::Times.bit(),
        Kind::Quote.bit(),
        Kind::Quote.bit(),
        Kind::Quote.bit(),
        Kind::Word(WordScript::Other).bit(),
        space,
        space,
        space | punct,
        punct,
        space,
        space,
        space,
        space,
        space,
    ]
};

/// A word (design section 8.2): a `Word` core, a one-word `Dictionary` core, a compact
/// abbreviation or a compound word.
pub(crate) fn is_word(cx: &Cx<'_>, f: &Frag) -> bool {
    matches!(cx.kind_of(f), FragKind::Word | FragKind::Compound) || compact_abbreviation(cx, f)
}

/// A compact abbreviation (design section 8.2): a `Dictionary` core of two or more consecutive
/// parts of an abbreviation of the language that all end in a full stop, written together
/// (`т.е.`, `т.д.`, `Т.е.`). `nbsp.abbreviations` splits it into its parts, each a one-word
/// `Dictionary` core, so it reads as a word in both forms (invariant I2 of design section 4.3).
pub(crate) fn compact_abbreviation(cx: &Cx<'_>, f: &Frag) -> bool {
    if f.kind != FragKind::Dictionary || !f.stop {
        return false;
    }
    // The shape: a word, then a single full stop and a word, once or more.
    let mut words = 0;
    let mut last = None;
    for (n, j) in cx.core(f).enumerate() {
        let ok = if n % 2 == 0 {
            matches!(cx.kind(j), Some(Kind::Word(_)))
        } else {
            cx.kind(j) == Some(Kind::Dots) && cx.is(j, ".")
        };
        if !ok {
            return false;
        }
        words += usize::from(n % 2 == 0);
        last = Some(n);
    }
    if words < 2 || last.is_none_or(|n| n % 2 != 0) {
        return false;
    }
    let chars = cx.core_chars(f);
    let mut rest = chars.clone();
    let lowered = rest
        .next()
        .filter(|c| c.is_uppercase())
        .map(|c| core::iter::once(fold_char(c)).chain(rest));
    let dotted = |key: &str| {
        cx.data
            .abbreviations
            .iter()
            .any(|(parts, _)| dotted_parts(parts, key))
    };
    with_key(chars, true, dotted) || lowered.is_some_and(|key| with_key(key, true, dotted))
}

/// Whether `key` is two or more consecutive parts of an abbreviation that all end in a full stop,
/// written together.
fn dotted_parts(parts: List<'_>, key: &str) -> bool {
    (0..parts.len()).any(|start| {
        let mut rest = key;
        let mut count = 0;
        for part in parts.iter().skip(start) {
            if !part.ends_with('.') {
                return false;
            }
            let Some(r) = rest.strip_prefix(part) else {
                return false;
            };
            count += 1;
            rest = r;
            if rest.is_empty() {
                return count >= 2;
            }
        }
        false
    })
}

/// A number: a `Number` fragment or a `Number` slot (labels and percentages included).
pub(crate) fn is_number(f: &Frag) -> bool {
    f.kind == FragKind::Number
}

/// A range: a `Range` or `YearMonth` core, or a percentage range (`5–10%`, `10-20%`, design
/// section 8.2).
pub(crate) fn is_range(cx: &Cx<'_>, f: &Frag) -> bool {
    matches!(f.kind, FragKind::Range | FragKind::YearMonth)
        || percent_range(cx, f)
        || percent_shape(cx, f, "-")
}

/// A percentage range (design section 8.2): a `Joined` core made of the two sides of a range of
/// check 17 joined by `–` and a `%` or `‰` attached to the last side (`5–10%`, `{a}–{b}%`). It is
/// what a range followed by a percent sign becomes when the percent setting removes the space,
/// and it reads as the range does, except where a percentage is not a number.
pub(crate) fn percent_range(cx: &Cx<'_>, f: &Frag) -> bool {
    f.kind == FragKind::Joined && percent_shape(cx, f, "\u{2013}")
}

/// A core made of the two sides of a range of check 17 joined by `joint` and a `%` or `‰`
/// attached to the last side. With `-` (`10-20%`) the core is a technical fragment and no rule
/// edits it, but the dash rules read it as a percentage range, as they read `10-20 %` (design
/// section 8.2), so that the space the percent setting removes changes nothing for them
/// (invariant I2).
fn percent_shape(cx: &Cx<'_>, f: &Frag, joint: &str) -> bool {
    // A number run split by `Format` or `Markup` tokens is one number (design section 3.6).
    let Some((core, len)) = cx.core_units::<8>(f) else {
        return false;
    };
    let Some((&(sign, _), rest)) = core.get(..len).and_then(<[(usize, usize)]>::split_last) else {
        return false;
    };
    if !matches!(cx.kind(sign), Some(Kind::Punct('%' | '‰'))) {
        return false;
    }
    let Some(at) = rest
        .iter()
        .position(|&(j, _)| cx.kind(j) == Some(Kind::Dash) && cx.is(j, joint))
    else {
        return false;
    };
    let side = |part: &[(usize, usize)]| match part {
        [one] => matches!(
            cx.kind(one.0),
            Some(Kind::Number | Kind::Slot(crate::SlotKind::Number))
        ),
        [a, sep, b] => {
            let separator = match cx.data.numbers.decimal {
                '.' => cx.kind(sep.0) == Some(Kind::Dots) && cx.is(sep.0, "."),
                c => cx.kind(sep.0) == Some(Kind::Punct(c)),
            };
            cx.kind(a.0) == Some(Kind::Number) && separator && cx.kind(b.0) == Some(Kind::Number)
        }
        _ => false,
    };
    side(rest.get(..at).unwrap_or_default()) && side(rest.get(at + 1..).unwrap_or_default())
}

/// The fragment that the dash rules read on the left of a dash (design section 8.2: a percentage
/// is one in both forms): for a lone `%` or `‰` after spaces, the number, label number or range
/// before the spaces (`100 %` reads as `100%`, `5–10 %` as `5–10%`, `№⍽5 %` as `№⍽5%`); otherwise
/// the fragment itself. So the space that the percent setting or `nbsp.signs` inserts or removes
/// changes nothing for those rules (invariant I2).
pub(crate) fn percent_owner<'a>(cx: &Cx<'a>, f: &'a Frag) -> &'a Frag {
    match before_percent(cx, f) {
        Some(n)
            if (is_number(n) || n.label || matches!(n.kind, FragKind::Range))
                && !with_sign(cx, n) =>
        {
            n
        }
        _ => f,
    }
}

/// For a fragment that is a lone `%` or `‰` after spaces, the fragment before the spaces when its
/// core ends there (`100 %` gives `100`); otherwise `None`. The caller decides which fragments
/// own the sign (design section 8.2).
pub(crate) fn before_percent<'a>(cx: &Cx<'a>, f: &'a Frag) -> Option<&'a Frag> {
    let mut core = cx.core(f);
    let (Some(sign), None) = (core.next(), core.next()) else {
        return None;
    };
    if !matches!(cx.kind(sign), Some(Kind::Punct('%' | '‰'))) {
        return None;
    }
    let first = (f.start as usize..f.end as usize)
        .find(|&j| cx.kind(j).is_some_and(|k| !k.is_transparent()));
    if first != Some(sign) {
        return None;
    }
    let space = cx.prev(sign).filter(|&s| cx.kind(s) == Some(Kind::Space))?;
    let q = cx.prev(space).filter(|&q| cx.in_core(q))?;
    cx.frag_of(q)
}

/// Whether a core holds `%` or `‰`.
pub(crate) fn with_sign(cx: &Cx<'_>, f: &Frag) -> bool {
    cx.core(f)
        .any(|j| matches!(cx.kind(j), Some(Kind::Punct('%' | '‰'))))
}

/// A technical fragment (design section 3.6): its core is frozen, whole or in part, and it is not
/// a compound word.
pub(crate) fn is_technical(f: &Frag) -> bool {
    f.kind.tech().is_some() && f.kind != FragKind::Compound
}

/// An operand (design section 8.2): a word, a number, a range, a slot, a protected span or a
/// technical fragment. A slot or a protected span alone is a `Word` or `Number` core.
pub(crate) fn is_operand(cx: &Cx<'_>, f: &Frag) -> bool {
    is_word(cx, f) || is_number(f) || is_range(cx, f) || is_technical(f)
}

/// A word of one letter (`a`, `x`); a slot or a compound word is not one. A compact abbreviation
/// reads as the part next to the other side: its last part on the left of a dash, its first part
/// on the right (`т.е.` as `е.` or `т.`, the parts it is split into).
pub(crate) fn single_letter(cx: &Cx<'_>, f: &Frag, left: bool) -> bool {
    if compact_abbreviation(cx, f) {
        let part = if left {
            cx.core(f).last()
        } else {
            cx.core(f).next()
        };
        return part.is_some_and(|i| cx.visible(i, i).count() == 1);
    }
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

/// The fragment `f` as a rule reads it on one side, in the plain form it stands for (design
/// section 4.3, I4); `left` when `f` stands on the left of the place the rule reads.
///
/// - A `Joined` core whose part on that side is a word of letters next to a `—` or `–` reads as
///   that word: `Moscow—a` is the form the closed style gives `Moscow - a` (design section
///   8.4), and `a` was a fragment of its own before the merge.
/// - A label (`№5`, `§V`) reads as its numeral on the left and as its sign on the right: the
///   parts that `nbsp.signs` splits it into (`№⍽5`).
///
/// Any other fragment reads as itself. So the merge or the split of the first pass changes
/// nothing for a rule that reads the part next to it (invariant I2).
pub(crate) fn plain_side(cx: &Cx<'_>, f: &Frag, left: bool) -> Frag {
    let part = match f.kind {
        FragKind::Joined => joined_side(cx, f, left),
        FragKind::Word if f.label => label_side(cx, f, left),
        _ => None,
    };
    part.unwrap_or(*f)
}

/// Whether token `i` starts a part of a `Joined` core: a `—` or `–` of the core stands right
/// before it. Such a part was a fragment of its own before the closed style merged it (design
/// sections 4.3, I4, and 8.4), so a rule that anchors at the start of a core anchors there too.
pub(crate) fn part_start(cx: &Cx<'_>, i: usize) -> bool {
    cx.in_core(i)
        && cx.frag_of(i).is_some_and(|f| f.kind == FragKind::Joined)
        && cx.prev(i).is_some_and(|d| {
            cx.in_core(d)
                && cx.tok(d).map(|t| t.frag) == cx.tok(i).map(|t| t.frag)
                && cx.kind(d) == Some(Kind::Dash)
                && (cx.is(d, "\u{2014}") || cx.is(d, "\u{2013}"))
        })
}

/// The word of letters at one end of a `Joined` core, after its last joint (`left`) or before its
/// first one; `None` when that part is anything else.
fn joined_side(cx: &Cx<'_>, f: &Frag, left: bool) -> Option<Frag> {
    let joint = |j: &usize| {
        cx.kind(*j) == Some(Kind::Dash) && (cx.is(*j, "\u{2014}") || cx.is(*j, "\u{2013}"))
    };
    let at = if left {
        cx.core(f).filter(joint).last()
    } else {
        cx.core(f).find(joint)
    }?;
    let at32 = u32::try_from(at).ok()?;
    let (start, end) = if left {
        (at32.checked_add(1)?, f.core_end)
    } else {
        (f.core_start, at32)
    };
    let part = Frag {
        start: if left { start } else { f.start },
        end: if left { f.end } else { end },
        core_start: start,
        core_end: end,
        kind: FragKind::Word,
        stop: left && f.stop,
        label: false,
        other_slot: false,
    };
    // Letters with their inner apostrophes only, as the closed style merges (design section 8.4).
    let word = |k: Option<usize>| k.is_some_and(|k| matches!(cx.kind(k), Some(Kind::Word(_))));
    let mut words = 0;
    for j in cx.core(&part) {
        match cx.kind(j) {
            Some(Kind::Word(_)) => words += 1,
            Some(Kind::Quote)
                if matches!(cx.chr(j), Some('\'' | '\u{2019}'))
                    && word(cx.prev(j))
                    && word(cx.next(j)) => {}
            _ => return None,
        }
    }
    (words > 0).then_some(part)
}

/// The numeral of a label (`left`) or its sign; `None` when the label has no such part.
fn label_side(cx: &Cx<'_>, f: &Frag, left: bool) -> Option<Frag> {
    let sign = |j: usize| matches!(cx.kind(j), Some(Kind::Punct(c)) if cx.data.signs.contains(&c));
    let first = cx.core(f).find(|&j| !sign(j))?;
    let kind = match cx.kind(first)? {
        Kind::Number | Kind::Slot(crate::SlotKind::Number) => FragKind::Number,
        Kind::Word(_) => FragKind::Word,
        _ => return None,
    };
    let at = u32::try_from(first).ok()?;
    Some(if left {
        Frag {
            start: at,
            core_start: at,
            kind,
            ..*f
        }
    } else {
        Frag {
            end: at,
            core_end: at,
            kind: FragKind::Sign,
            stop: false,
            label: false,
            other_slot: false,
            ..*f
        }
    })
}

/// The scripts of the letters of a core, as bits of `script_bit`.
fn scripts(cx: &Cx<'_>, f: &Frag) -> u8 {
    cx.core(f)
        .filter(|&i| matches!(cx.kind(i), Some(Kind::Word(_))))
        .flat_map(|i| cx.visible(i, i))
        .filter(|&c| is_alphabetic(c))
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
