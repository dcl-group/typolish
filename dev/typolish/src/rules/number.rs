//! `number.*` (design section 8.8): `number.minus` and `number.times`.

use super::{FragKind, Triggers};
use crate::engine::lex::{Kind, Repl};
use crate::engine::phase::{Cx, Edit, Edits, Match};

/// `number.minus`: a hyphen before a number.
pub(crate) const MINUS: Triggers = Triggers::of(b"-").union(Triggers::DIGITS);
/// `number.times`: `x` between numbers, with the lead bytes of the Cyrillic `х` and `Х`, which
/// stand alone between two `Number` slots in a template whose texts have no digit
/// (`{w} х {h}`, design section 3.2).
pub(crate) const TIMES: Triggers = Triggers::of(b"xX")
    .union(Triggers::DIGITS)
    .with_lead('\u{445}')
    .with_lead('\u{425}');

const TIMES_SIGN: char = '\u{d7}';

/// `number.minus` (Prose): the hyphen of a core `-D+` or `-D+[.,]D+`, with an optional `%` or `‰`
/// attached, whose fragment starts after a pattern space, a line start or `(` becomes the minus
/// sign of the settings (design section 8.8). A percentage reads alike in both of its forms
/// (section 8.2), so the space that the percent setting inserts changes nothing for the rule. A
/// fragment that already has the sign, typed or produced, matches with an empty edit.
pub(crate) fn minus(cx: &Cx<'_>, i: usize) -> Option<Match> {
    if cx.kind(i) != Some(Kind::Dash) {
        return None;
    }
    let f = cx.frag_of(i)?;
    if f.kind != FragKind::Number || f.core_start as usize != i {
        return None;
    }
    // The shape of the core: the sign, digits, an optional separator and digits, and an optional
    // percent sign.
    let mut core = cx.core(f).peekable();
    if core.next() != Some(i) {
        return None;
    }
    let digits = |j: Option<usize>| j.is_some_and(|j| cx.kind(j) == Some(Kind::Number));
    if !digits(core.next()) {
        return None;
    }
    let percent = |j: usize| matches!(cx.kind(j), Some(Kind::Punct('%' | '‰')));
    if let Some(&sep) = core.peek()
        && !percent(sep)
    {
        let separator = cx.kind(sep) == Some(Kind::Punct(','))
            || (cx.kind(sep) == Some(Kind::Dots) && cx.is(sep, "."));
        core.next();
        if !separator || !digits(core.next()) {
            return None;
        }
    }
    match (core.next(), core.next()) {
        (None, _) => {}
        (Some(sign), None) if percent(sign) => {}
        _ => return None,
    }
    // The fragment starts after a pattern space or a line start, with `(` or nothing before the
    // sign.
    let first = (f.start as usize..i).find(|&j| cx.kind(j).is_some_and(|k| !k.is_transparent()));
    let start = match first {
        None => i,
        Some(j) if cx.kind(j) == Some(Kind::Open('(')) && cx.next(j) == Some(i) => j,
        Some(_) => return None,
    };
    let after_space = cx.prev(start).is_some_and(|p| cx.pattern_space(p));
    if !after_space && !cx.line_start(start) {
        return None;
    }
    let sign = cx.t.minus_sign();
    let mut buf = [0u8; 4];
    if cx.is(i, sign.encode_utf8(&mut buf)) {
        return Some(Match::done(0));
    }
    // A typed sign of another form stays: rules never rewrite a typographic character.
    if !cx.is(i, "-") {
        return None;
    }
    Some(Match::with(
        0,
        Edits::NONE.with(Edit::Replace {
            at: 0,
            repl: Repl::Char(sign),
        }),
    ))
}

/// `number.times` (Prose): the letter of a `Dimensions` core (check 16), or a lone `x`, `X`,
/// `х`, `Х` fragment between two number fragments whose left one has no leading zero, becomes
/// `×`; the author's spaces stay. The classifier gives such a letter the token kind `Times`
/// (design section 3.6); a typed `×` matches with an empty edit.
pub(crate) fn times(cx: &Cx<'_>, i: usize) -> Option<Match> {
    if cx.kind(i) != Some(Kind::Times) {
        return None;
    }
    let f = cx.frag_of(i)?;
    match f.kind {
        FragKind::Dimensions => {}
        FragKind::Times => {
            // The left factor of a lone sign, as the classifier found it (a number fragment, or
            // a label number attached to its sign, `№10`), without a leading zero; a `Number`
            // slot is a number (design section 8.2) whose digits are unknown.
            let left = cx.prev(f.start as usize).and_then(|s| cx.prev(s))?;
            let lf = cx.frag_of(left)?;
            if !matches!(lf.kind, FragKind::Number | FragKind::Word) {
                return None;
            }
            let first = cx.core(lf).find(|&j| {
                matches!(
                    cx.kind(j),
                    Some(Kind::Number | Kind::Slot(crate::SlotKind::Number))
                )
            })?;
            let digits = cx.src(first);
            if cx.kind(first) == Some(Kind::Number) && digits.len() > 1 && digits.starts_with('0') {
                return None;
            }
        }
        _ => return None,
    }
    if cx.is(i, "\u{d7}") {
        return Some(Match::done(0));
    }
    Some(Match::with(
        0,
        Edits::NONE.with(Edit::Replace {
            at: 0,
            repl: Repl::Char(TIMES_SIGN),
        }),
    ))
}
