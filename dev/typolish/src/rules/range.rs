//! `range.placeholders`, `range.digits`, `range.context` and `range.time` (design section 8.5).
//!
//! Every range rule anchors at the dash of the range and writes the range dash of the language
//! (`NumberData::range_dash`). A typed range dash is matched with an empty edit.

use core::cmp::Ordering;

use super::{Frag, FragKind, Triggers, compact_abbreviation, folded_in, joined_first, with_key};
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
            Edits::one(Edit::Replace {
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

/// A unit of a core: its first and its last token (`Cx::fold_runs`).
type Unit = (usize, usize);

/// The core units of the fragment of the dash at `i`, `Format` and `Markup` tokens skipped and
/// each number run one unit (design section 3.6), when it is a `Range` or `YearMonth` core and
/// `i` is in it; at most eight.
fn range_core(cx: &Cx<'_>, i: usize, kinds: &[FragKind]) -> Option<([Unit; 8], usize)> {
    let f = cx.frag_of(i)?;
    if !kinds.contains(&f.kind) || !cx.in_core(i) {
        return None;
    }
    cx.core_units(f)
}

/// Digits without a leading zero.
fn no_leading_zero(mut digits: impl Iterator<Item = char> + Clone) -> bool {
    !(digits.clone().count() > 1 && digits.next() == Some('0'))
}

/// Compares two digit strings without leading zeros by value.
fn cmp_digits(
    a: impl Iterator<Item = char> + Clone,
    b: impl Iterator<Item = char> + Clone,
) -> Ordering {
    a.clone()
        .count()
        .cmp(&b.clone().count())
        .then_with(|| a.cmp(b))
}

/// `range.digits` (Prose): a core `D+-D+` with exactly two numbers, both without leading zeros,
/// the left smaller than the right.
pub(crate) fn digits(cx: &Cx<'_>, i: usize) -> Option<Match> {
    if cx.kind(i) != Some(Kind::Dash) {
        return None;
    }
    let (core, len) = range_core(cx, i, &[FragKind::Range])?;
    let [a, d, b, ..] = core;
    if len != 3 || d.0 != i {
        return None;
    }
    if cx.kind(a.0) != Some(Kind::Number) || cx.kind(b.0) != Some(Kind::Number) {
        return None;
    }
    let (sa, sb) = (cx.visible(a.0, a.1), cx.visible(b.0, b.1));
    if !no_leading_zero(sa.clone())
        || !no_leading_zero(sb.clone())
        || cmp_digits(sa, sb) != Ordering::Less
    {
        return None;
    }
    dash_edit(cx, i)
}

/// One side of `range.context`.
#[derive(Clone, Copy, Debug)]
enum Side {
    /// An unsigned integer or a decimal: the number runs of the integer and of the fraction.
    Number(Unit, Option<Unit>),
    Roman(u32),
}

/// Reads one side of a context range from its core units.
fn side(cx: &Cx<'_>, units: &[Unit]) -> Option<Side> {
    match *units {
        [n] if cx.kind(n.0) == Some(Kind::Number) => Some(Side::Number(n, None)),
        // A Roman numeral, also split by `Format` and `Markup` tokens (design section 3.6).
        [w, .., last] | [w @ last]
            if units
                .iter()
                .all(|&(t, _)| matches!(cx.kind(t), Some(Kind::Word(_)))) =>
        {
            roman_value(cx.visible(w.0, last.1)).map(Side::Roman)
        }
        [a, sep, b] if cx.kind(a.0) == Some(Kind::Number) && cx.kind(b.0) == Some(Kind::Number) => {
            let decimal = cx.data.numbers.decimal;
            let ok = match decimal {
                '.' => cx.kind(sep.0) == Some(Kind::Dots) && cx.is(sep.0, "."),
                c => cx.kind(sep.0) == Some(Kind::Punct(c)),
            };
            ok.then_some(Side::Number(a, Some(b)))
        }
        _ => None,
    }
}

/// The visible digits of a number run, none for no run.
fn run_digits<'a>(cx: &Cx<'a>, run: Option<Unit>) -> impl Iterator<Item = char> + Clone + 'a {
    run.map(|(first, last)| cx.visible(first, last))
        .into_iter()
        .flatten()
}

/// Compares two decimals given as integer and fraction runs, without leading zeros.
fn cmp_decimal(cx: &Cx<'_>, a: (Unit, Option<Unit>), b: (Unit, Option<Unit>)) -> Ordering {
    cmp_digits(run_digits(cx, Some(a.0)), run_digits(cx, Some(b.0))).then_with(|| {
        let mut fa = run_digits(cx, a.1);
        let mut fb = run_digits(cx, b.1);
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
pub(crate) fn roman_value(chars: impl Iterator<Item = char>) -> Option<u32> {
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
fn ordered(cx: &Cx<'_>, a: Side, b: Side) -> bool {
    match (a, b) {
        (Side::Roman(x), Side::Roman(y)) => x < y,
        (Side::Number(ai, af), Side::Number(bi, bf)) => {
            let (int_a, int_b) = (run_digits(cx, Some(ai)), run_digits(cx, Some(bi)));
            if !no_leading_zero(int_a.clone()) {
                return false;
            }
            if af.is_none()
                && bf.is_none()
                && int_a.clone().count() == 4
                && int_b.clone().count() == 2
            {
                return int_b.cmp(int_a.skip(2)) == Ordering::Greater;
            }
            no_leading_zero(int_b) && cmp_decimal(cx, (ai, af), (bi, bf)) == Ordering::Less
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
    let at = core.iter().position(|&(j, _)| j == i)?;
    let a = side(cx, core.get(..at)?)?;
    let b = side(cx, core.get(at + 1..)?)?;
    if !ordered(cx, a, b) {
        return None;
    }
    let first = core.first()?.0;
    let last = core.last()?.1;
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
    let f = &first_part(cx, &joined_first(cx, f));
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

/// The fragment after a range as `range.context` reads it, after `joined_first` has read a `Joined`
/// core as its first word: a compact abbreviation (`т.е.`) reads as its first part with its full
/// stop (`т.`), the fragment that `nbsp.abbreviations` splits off, so the split of the first pass
/// changes nothing for the rule (design section 4.3, I4). Any other fragment reads as itself.
fn first_part(cx: &Cx<'_>, f: &Frag) -> Frag {
    if !compact_abbreviation(cx, f) {
        return *f;
    }
    let mut core = cx.core(f);
    let part = core
        .next()
        .zip(core.next())
        .and_then(|(word, stop)| Some((u32::try_from(word).ok()?, u32::try_from(stop).ok()?)));
    match part {
        Some((word, stop)) => Frag {
            start: f.start,
            end: stop.saturating_add(1),
            core_start: word,
            core_end: stop,
            kind: FragKind::Dictionary,
            stop: true,
            label: false,
            other_slot: false,
        },
        None => *f,
    }
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
