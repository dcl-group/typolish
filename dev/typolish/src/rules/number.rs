//! `number.*` (design section 8.8). `number.times` is implemented; `number.minus` declares its
//! trigger bytes and matches nothing yet.

use super::{FragKind, Triggers};
use crate::engine::lex::{Kind, Repl};
use crate::engine::phase::{Cx, Edit, Edits, Match};

/// `number.minus`: a hyphen before a number.
pub(crate) const MINUS: Triggers = Triggers::of(b"-").union(Triggers::DIGITS);
/// `number.times`: `x` between numbers (the Cyrillic letters are found through the digits).
pub(crate) const TIMES: Triggers = Triggers::of(b"xX").union(Triggers::DIGITS);

const TIMES_SIGN: char = '\u{d7}';

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
            // The left factor of a lone sign: a number fragment without a leading zero.
            let left = cx.prev(f.start as usize).and_then(|s| cx.prev(s))?;
            let lf = cx.frag_of(left)?;
            let first = cx.core(lf).find(|&j| cx.kind(j) == Some(Kind::Number))?;
            let digits = cx.src(first);
            if lf.kind != FragKind::Number || (digits.len() > 1 && digits.starts_with('0')) {
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
