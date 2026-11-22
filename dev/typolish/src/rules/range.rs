//! `range.placeholders`, `range.digits`, `range.context` and `range.time` (design section 8.5).
//!
//! Every range rule anchors at the dash of the range and writes the range dash of the language
//! (`NumberData::range_dash`). A typed range dash is matched with an empty edit.

use core::cmp::Ordering;

use super::{FragKind, Triggers, folded_in, with_key};
use crate::SlotKind;
use crate::engine::lex::{Kind, Repl};
use crate::engine::phase::{Cx, Edit, Edits, Match};

/// `range.placeholders`: a hyphen between two slots.
pub(crate) const PLACEHOLDERS: Triggers = Triggers::of(b"-");
/// `range.digits`: a hyphen between two numbers.
pub(crate) const DIGITS: Triggers = Triggers::of(b"-").union(Triggers::DIGITS);
/// `range.context`: a hyphen in a range with a context word.
pub(crate) const CONTEXT: Triggers = Triggers::of(b"-").union(Triggers::DIGITS);
/// `range.time`: a hyphen between two times.
pub(crate) const TIME: Triggers = Triggers::of(b"-").union(Triggers::DIGITS);

/// The edit of a range rule at its dash: the range dash for a hyphen, nothing for a typed range
/// dash; `None` for any other dash.
fn dash_edit(cx: &Cx<'_>, i: usize) -> Option<Match> {
    let dash = cx.data.numbers.range_dash;
    let mut buf = [0u8; 4];
    if cx.is(i, dash.encode_utf8(&mut buf)) || cx.is(i, "\u{2013}") {
        return Some(Match::done(0));
    }
    cx.is(i, "-").then(|| {
        Match::with(
            0,
            Edits::NONE.with(Edit::Replace {
                at: 0,
                repl: Repl::Char(dash),
            }),
        )
    })
}

/// `range.placeholders`: a hyphen without spaces between two `Number` slots or two `Date` slots.
pub(crate) fn placeholders(cx: &Cx<'_>, i: usize) -> Option<Match> {
    if cx.kind(i) != Some(Kind::Dash) {
        return None;
    }
    let a = cx.prev(i).and_then(|p| cx.kind(p))?;
    let b = cx.next(i).and_then(|n| cx.kind(n))?;
    match (a, b) {
        (Kind::Slot(SlotKind::Number), Kind::Slot(SlotKind::Number))
        | (Kind::Slot(SlotKind::Date), Kind::Slot(SlotKind::Date)) => dash_edit(cx, i),
        _ => None,
    }
}

/// The core tokens of the fragment of the dash at `i`, `Format` and `Markup` tokens skipped, when
/// it is a `Range` or `YearMonth` core and `i` is in it; at most eight.
fn range_core(cx: &Cx<'_>, i: usize, kinds: &[FragKind]) -> Option<([usize; 8], usize)> {
    let f = cx.frag_of(i)?;
    if !kinds.contains(&f.kind) || !cx.in_core(i) {
        return None;
    }
    let mut out = [0usize; 8];
    let mut len = 0;
    for j in cx.core(f) {
        *out.get_mut(len)? = j;
        len += 1;
    }
    Some((out, len))
}

/// Digits without a leading zero.
fn no_leading_zero(s: &str) -> bool {
    !(s.len() > 1 && s.starts_with('0'))
}

/// Compares two digit strings without leading zeros by value.
fn cmp_digits(a: &str, b: &str) -> Ordering {
    a.len().cmp(&b.len()).then_with(|| a.cmp(b))
}

/// `range.digits` (Prose): a core `D+-D+` with exactly two numbers, both without leading zeros,
/// the left smaller than the right.
pub(crate) fn digits(cx: &Cx<'_>, i: usize) -> Option<Match> {
    if cx.kind(i) != Some(Kind::Dash) {
        return None;
    }
    let (core, len) = range_core(cx, i, &[FragKind::Range])?;
    let [a, d, b, ..] = core;
    if len != 3 || d != i {
        return None;
    }
    if cx.kind(a) != Some(Kind::Number) || cx.kind(b) != Some(Kind::Number) {
        return None;
    }
    let (sa, sb) = (cx.src(a), cx.src(b));
    if !no_leading_zero(sa) || !no_leading_zero(sb) || cmp_digits(sa, sb) != Ordering::Less {
        return None;
    }
    dash_edit(cx, i)
}

/// One side of `range.context`.
#[derive(Clone, Copy, Debug)]
enum Side<'a> {
    /// An unsigned integer or a decimal: the integer digits and the fraction digits.
    Number(&'a str, &'a str),
    Roman(u32),
}

/// Reads one side of a context range from its core tokens.
fn side<'a>(cx: &Cx<'a>, tokens: &[usize]) -> Option<Side<'a>> {
    match *tokens {
        [n] if cx.kind(n) == Some(Kind::Number) => Some(Side::Number(cx.src(n), "")),
        [w] if matches!(cx.kind(w), Some(Kind::Word(_))) => {
            roman_value(cx.visible(w, w)).map(Side::Roman)
        }
        [a, sep, b] if cx.kind(a) == Some(Kind::Number) && cx.kind(b) == Some(Kind::Number) => {
            let decimal = cx.data.numbers.decimal;
            let ok = match decimal {
                '.' => cx.kind(sep) == Some(Kind::Dots) && cx.is(sep, "."),
                c => cx.kind(sep) == Some(Kind::Punct(c)),
            };
            ok.then(|| Side::Number(cx.src(a), cx.src(b)))
        }
        _ => None,
    }
}

/// Compares two decimals given as integer and fraction digits, without leading zeros.
fn cmp_decimal(a: (&str, &str), b: (&str, &str)) -> Ordering {
    cmp_digits(a.0, b.0).then_with(|| {
        let mut fa = a.1.chars();
        let mut fb = b.1.chars();
        loop {
            match (fa.next(), fb.next()) {
                (None, None) => return Ordering::Equal,
                (x, y) => match x.unwrap_or('0').cmp(&y.unwrap_or('0')) {
                    Ordering::Equal => {}
                    other => return other,
                },
            }
        }
    })
}

/// The value of a Roman numeral in the standard form, 1 to 3999 (design section 3.6): the
/// numeral read greedily and written back canonically must give the same text.
fn roman_value(chars: impl Iterator<Item = char>) -> Option<u32> {
    const TABLE: [(u32, &str); 13] = [
        (1000, "M"),
        (900, "CM"),
        (500, "D"),
        (400, "CD"),
        (100, "C"),
        (90, "XC"),
        (50, "L"),
        (40, "XL"),
        (10, "X"),
        (9, "IX"),
        (5, "V"),
        (4, "IV"),
        (1, "I"),
    ];
    let mut buf = [0u8; 16];
    let mut len = 0;
    for c in chars {
        if !c.is_ascii() {
            return None;
        }
        *buf.get_mut(len)? = c as u8;
        len += 1;
    }
    let text = core::str::from_utf8(buf.get(..len)?).ok()?;
    let mut rest = text;
    let mut total = 0u32;
    for (value, form) in TABLE {
        while let Some(r) = rest.strip_prefix(form) {
            rest = r;
            total += value;
        }
    }
    if !rest.is_empty() || !(1..=3999).contains(&total) {
        return None;
    }
    let mut canonical = text;
    let mut v = total;
    for (value, form) in TABLE {
        while v >= value {
            canonical = canonical.strip_prefix(form)?;
            v -= value;
        }
    }
    canonical.is_empty().then_some(total)
}

/// Whether the shape of a context range holds: left smaller than right for two numbers or two
/// Roman numerals, or `YYYY-YY` with the tail greater than the last two digits of the year.
fn ordered(a: Side<'_>, b: Side<'_>) -> bool {
    match (a, b) {
        (Side::Roman(x), Side::Roman(y)) => x < y,
        (Side::Number(ai, af), Side::Number(bi, bf)) => {
            if !no_leading_zero(ai) {
                return false;
            }
            if af.is_empty() && bf.is_empty() && ai.len() == 4 && bi.len() == 2 {
                let tail = ai.get(2..).unwrap_or("");
                return bi > tail;
            }
            no_leading_zero(bi) && cmp_decimal((ai, af), (bi, bf)) == Ordering::Less
        }
        _ => false,
    }
}

/// `range.context` (Prose): `N-N` with a unit, a year word or a century word as the next
/// fragment, or a range lead word as the previous one, one pattern space between them.
pub(crate) fn context(cx: &Cx<'_>, i: usize) -> Option<Match> {
    if cx.kind(i) != Some(Kind::Dash) {
        return None;
    }
    let f = cx.frag_of(i)?;
    if f.label {
        return None;
    }
    let (core, len) = range_core(cx, i, &[FragKind::Range, FragKind::YearMonth])?;
    let core = core.get(..len)?;
    let at = core.iter().position(|&j| j == i)?;
    let a = side(cx, core.get(..at)?)?;
    let b = side(cx, core.get(at + 1..)?)?;
    if !ordered(a, b) {
        return None;
    }
    let first = *core.first()?;
    let last = *core.last()?;
    let after = context_after(cx, last);
    let before = || context_before(cx, first);
    if !(after || before()) {
        return None;
    }
    dash_edit(cx, i)
}

/// A unit, a year word or a century word after the range.
fn context_after(cx: &Cx<'_>, last: usize) -> bool {
    let Some(s) = cx.next(last) else {
        return false;
    };
    if !cx.pattern_space(s) {
        return false;
    }
    let Some(n) = cx.next(s) else {
        return false;
    };
    let Some(f) = cx.frag_of(n) else {
        return false;
    };
    if f.core_start >= f.core_end || cx.core(f).next() != Some(n) {
        return false;
    }
    let data = cx.data;
    let chars = cx.core_chars(f);
    let unit = |key: &str| cx.t.has_unit(key);
    let found_unit = with_key(chars.clone(), false, unit)
        .then_some(false)
        .or_else(|| (f.stop && with_key(chars.clone(), true, unit)).then_some(true));
    if let Some(with_stop) = found_unit {
        let ambiguous = with_key(chars.clone(), with_stop, |key| {
            data.ambiguous_units.contains(key)
        });
        if !ambiguous {
            return true;
        }
        // An ambiguous unit counts only before punctuation, a line break or the end.
        let wrapped = (f.core_end as usize..f.end as usize)
            .any(|j| !cx.kind(j).is_some_and(Kind::is_transparent));
        let after = cx.next(f.end as usize - 1);
        let end = after.is_none_or(|j| cx.kind(j) == Some(Kind::Newline));
        return wrapped || end;
    }
    let words = |list: crate::lang::data::List<'_>| {
        folded_in(list, chars.clone()) || (f.stop && folded_in(list, chars.clone().chain(['.'])))
    };
    words(data.year_words) || words(data.century_words)
}

/// A range lead word before the range.
fn context_before(cx: &Cx<'_>, first: usize) -> bool {
    let Some(s) = cx.prev(first) else {
        return false;
    };
    if !cx.pattern_space(s) {
        return false;
    }
    let Some(p) = cx.prev(s) else {
        return false;
    };
    let Some(f) = cx.frag_of(p) else {
        return false;
    };
    if f.core_start >= f.core_end {
        return false;
    }
    let chars = cx.core_chars(f);
    let list = cx.data.range_lead_words;
    folded_in(list, chars.clone()) || (f.stop && folded_in(list, chars.chain(['.'])))
}

/// `range.time` (Prose): `HH:MM[:SS]-HH:MM[:SS]` → the two times with the range dash.
pub(crate) fn time(cx: &Cx<'_>, i: usize) -> Option<Match> {
    if cx.kind(i) != Some(Kind::Dash) {
        return None;
    }
    let f = cx.frag_of(i)?;
    if f.kind != FragKind::TimeRange || !cx.in_core(i) {
        return None;
    }
    dash_edit(cx, i)
}
