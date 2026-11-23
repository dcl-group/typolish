//! `symbols.*` (design section 8.9): legal marks, arrows and mathematical signs. Each rule
//! replaces one whole token of its own kind (`LegalMark`, `Operator`), so no letter of a word
//! changes (design section 1.4); a typed sign in the same context matches with an empty edit.

use super::{FragKind, Triggers};
use crate::engine::classify::Frag;
use crate::engine::lex::{Kind, Mark, Op, Repl};
use crate::engine::phase::{Cx, Edit, Edits, Match};

/// `symbols.copyright`: `(c)`, `(r)`, `(tm)`.
pub(crate) const COPYRIGHT: Triggers = Triggers::of(b"(");
/// `symbols.arrows`: `->`, `<-`, `<->`.
pub(crate) const ARROWS: Triggers = Triggers::of(b"-<>");
/// `symbols.math`: `>=`, `<=`, `!=`, `+-`.
pub(crate) const MATH: Triggers = Triggers::of(b"<>!+");

/// A replacement of the anchor token by `sign`, or an empty match when it is the sign already.
fn replace(cx: &Cx<'_>, i: usize, sign: char) -> Match {
    let mut buf = [0u8; 4];
    if cx.is(i, sign.encode_utf8(&mut buf)) {
        return Match::done(0);
    }
    Match::with(
        0,
        Edits::NONE.with(Edit::Replace {
            at: 0,
            repl: Repl::Char(sign),
        }),
    )
}

/// The tokens of a core, `Format` and `Markup` tokens skipped, at most `N` of them; `None` when
/// the core is longer.
fn core_of<const N: usize>(cx: &Cx<'_>, f: &Frag) -> Option<([usize; N], usize)> {
    let mut out = [0usize; N];
    let mut len = 0;
    for j in cx.core(f) {
        *out.get_mut(len)? = j;
        len += 1;
    }
    Some((out, len))
}

/// Whether a token is a run of exactly four ASCII digits.
fn four_digits(cx: &Cx<'_>, j: usize) -> bool {
    cx.kind(j) == Some(Kind::Number) && cx.src(j).len() == 4
}

/// A year or a range of years (design section 8.9): a `Number` core of four digits, or a `Range`
/// core of two of them joined by `-` or `–`. A label number and a percentage are not years in
/// either of their forms (design section 8.2): not `№⍽2026`, and not a core that a percent sign
/// follows after spaces (`2026 %`), so the space that `nbsp.signs` or the percent setting inserts
/// or removes changes nothing for the rule (invariant I2).
fn years(cx: &Cx<'_>, f: &Frag, range: bool) -> bool {
    if f.label || percent_follows(cx, f) {
        return false;
    }
    let Some((core, len)) = core_of::<3>(cx, f) else {
        return false;
    };
    match (f.kind, core.get(..len)) {
        (FragKind::Number, Some(&[year])) => four_digits(cx, year),
        (FragKind::Range, Some(&[a, dash, b])) if range => {
            four_digits(cx, a)
                && four_digits(cx, b)
                && cx.kind(dash) == Some(Kind::Dash)
                && (cx.is(dash, "-") || cx.is(dash, "\u{2013}"))
        }
        _ => false,
    }
}

/// The first and the last visible token of a fragment.
fn ends(cx: &Cx<'_>, f: &Frag) -> Option<(usize, usize)> {
    let mut visible = (f.start as usize..f.end as usize)
        .filter(|&j| cx.kind(j).is_some_and(|k| !k.is_transparent()));
    let first = visible.next()?;
    Some((first, visible.next_back().unwrap_or(first)))
}

/// Whether a lone `%` or `‰` after spaces follows the fragment `f` and is read with it (design
/// section 8.2: `2026 %` and `2026⍽%` are the percentage `2026%`).
fn percent_follows(cx: &Cx<'_>, f: &Frag) -> bool {
    let Some((_, last)) = ends(cx, f) else {
        return false;
    };
    cx.next(last)
        .filter(|&s| cx.kind(s) == Some(Kind::Space))
        .and_then(|s| cx.next(s))
        .and_then(|j| cx.frag_of(j))
        .and_then(|g| super::before_percent(cx, g))
        .is_some_and(|owner| owner.start == f.start)
}

/// The fragment one pattern space after the fragment `f`, if its first token is in its core.
fn after<'a>(cx: &Cx<'a>, f: &Frag) -> Option<&'a Frag> {
    let (_, last) = ends(cx, f)?;
    let space = cx.next(last).filter(|&s| cx.pattern_space(s))?;
    let j = cx.next(space)?;
    let g = cx.frag_of(j)?;
    (g.core_start as usize == j).then_some(g)
}

/// The fragment one pattern space before the fragment `f`, if its last token is in its core.
fn before<'a>(cx: &Cx<'a>, f: &Frag) -> Option<&'a Frag> {
    let (first, _) = ends(cx, f)?;
    let space = cx.prev(first).filter(|&s| cx.pattern_space(s))?;
    let j = cx.prev(space)?;
    let g = cx.frag_of(j)?;
    (g.core_end as usize == j + 1 && cx.in_core(j)).then_some(g)
}

/// A brand name (design section 8.9): a word of design section 8.2 (a `Word` core of any number
/// of tokens, apostrophes included, or a compound word), not a label, starting with a capital,
/// with at least two letters. `first` and `last` are its first and last tokens in the fragment
/// `f`.
fn brand(cx: &Cx<'_>, f: &Frag, first: usize, last: usize) -> bool {
    if !matches!(f.kind, FragKind::Word | FragKind::Compound)
        || f.label
        || !matches!(cx.kind(first), Some(Kind::Word(_)))
    {
        return false;
    }
    let mut letters = cx.visible(first, last).filter(|c| c.is_alphabetic());
    letters.next().is_some_and(char::is_uppercase) && letters.next().is_some()
}

/// Whether an operator fragment follows `f` (`Brand(tm) = x`, `Brand (r) + x`): then the mark is
/// a function argument.
fn operator_after(cx: &Cx<'_>, f: &Frag) -> bool {
    let Some((_, last)) = ends(cx, f) else {
        return false;
    };
    let mut j = cx.next(last);
    if let Some(s) = j.filter(|&s| cx.kind(s) == Some(Kind::Space)) {
        j = cx.next(s);
    }
    j.and_then(|j| cx.frag_of(j))
        .is_some_and(|g| matches!(g.kind, FragKind::OperatorRun | FragKind::Operator))
}

/// `symbols.copyright` (Prose): `(c)` next to a year → `©`; `(r)`, `(tm)` after a brand name →
/// `®`, `™` (design section 8.9).
pub(crate) fn copyright(cx: &Cx<'_>, i: usize) -> Option<Match> {
    let Some(Kind::LegalMark(mark)) = cx.kind(i) else {
        return None;
    };
    let f = cx.frag_of(i)?;
    match mark {
        Mark::Copyright => {
            let (core, len) = core_of::<2>(cx, f)?;
            let core = core.get(..len)?;
            let fits = match *core {
                // `(c)2026`: a year attached after the mark.
                [m, year] => {
                    m == i
                        && f.kind == FragKind::Word
                        && four_digits(cx, year)
                        && !percent_follows(cx, f)
                }
                // `(c) 2026`, `2026 (c)`, `(c) 2020-2026`, `2020-2026 (c)`.
                [m] => {
                    m == i
                        && (after(cx, f).is_some_and(|g| years(cx, g, true))
                            || before(cx, f).is_some_and(|g| years(cx, g, true)))
                }
                _ => false,
            };
            fits.then(|| replace(cx, i, '\u{a9}'))
        }
        Mark::Registered | Mark::Trademark => {
            let mut core = cx.core(f);
            let fits = match (core.next(), core.last()) {
                // `Brand (r)`, `IT-Systems (tm)`: the mark alone, the brand before it.
                (Some(m), None) if m == i => before(cx, f).is_some_and(|g| {
                    let mut word = cx.core(g);
                    match (word.next(), word.last()) {
                        (Some(a), b) => brand(cx, g, a, b.unwrap_or(a)),
                        (None, _) => false,
                    }
                }),
                // `Brand(tm)`, `McDonald’s(tm)`: the brand is the rest of the core.
                (Some(a), Some(m)) if m == i && a != i => cx
                    .prev(i)
                    .is_some_and(|b| cx.in_core(b) && brand(cx, f, a, b)),
                _ => false,
            };
            if !fits || operator_after(cx, f) {
                return None;
            }
            let sign = if mark == Mark::Registered {
                '\u{ae}'
            } else {
                '\u{2122}'
            };
            Some(replace(cx, i, sign))
        }
    }
}

/// An operator fragment (design section 8.9): one operator token alone, with a pattern space on
/// each side.
fn spaced_operator(cx: &Cx<'_>, i: usize) -> bool {
    let Some(f) = cx.frag_of(i) else {
        return false;
    };
    f.kind == FragKind::OperatorRun
        && ends(cx, f) == Some((i, i))
        && cx.prev(i).is_some_and(|s| cx.pattern_space(s))
        && cx.next(i).is_some_and(|s| cx.pattern_space(s))
        && cx.prev(i).and_then(|s| cx.prev(s)).is_some()
        && cx.next(i).and_then(|s| cx.next(s)).is_some()
}

/// `symbols.arrows` (Prose): an operator fragment that is exactly `->`, `<-` or `<->`, with a
/// space on each side → `→`, `←`, `↔`.
pub(crate) fn arrows(cx: &Cx<'_>, i: usize) -> Option<Match> {
    let Some(Kind::Operator(op)) = cx.kind(i) else {
        return None;
    };
    let sign = match op {
        Op::ArrowRight => '\u{2192}',
        Op::ArrowLeft => '\u{2190}',
        Op::ArrowBoth => '\u{2194}',
        _ => return None,
    };
    spaced_operator(cx, i).then(|| replace(cx, i, sign))
}

/// `symbols.math` (Prose): an operator fragment that is exactly `>=`, `<=` or `!=`, with a space
/// on each side → `≥`, `≤`, `≠`; `+-` as such a fragment or as the sign of a number → `±`.
pub(crate) fn math(cx: &Cx<'_>, i: usize) -> Option<Match> {
    let Some(Kind::Operator(op)) = cx.kind(i) else {
        return None;
    };
    let sign = match op {
        Op::GreaterEqual => '\u{2265}',
        Op::LessEqual => '\u{2264}',
        Op::NotEqual => '\u{2260}',
        Op::PlusMinus => '\u{b1}',
        _ => return None,
    };
    let signed_number = op == Op::PlusMinus
        && cx
            .frag_of(i)
            .is_some_and(|f| f.kind == FragKind::Number && f.core_start as usize == i);
    (signed_number || spaced_operator(cx, i)).then(|| replace(cx, i, sign))
}
