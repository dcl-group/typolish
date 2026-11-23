//! `ellipsis.dots` and `ellipsis.inner` (design section 8.6).
//!
//! Both rules anchor at a `Dots` token. A run of exactly three full stops is replaced as a whole;
//! two and four or more never change. On their products and on typed forms (`…`, the language's
//! `?..` or `?…`) the rules match with an empty edit (design section 3.8, item 3).

use super::Triggers;
use super::quotes::{Role, local_role};
use crate::SlotKind;
use crate::engine::lex::{EllipsisForm, Kind, Repl};
use crate::engine::phase::{Cx, Edit, Edits, Match, ellipsis_tail};

/// `ellipsis.dots`: runs of full stops; typed quotes and dashes are context.
pub(crate) const DOTS: Triggers = Triggers::of(b".").union(Triggers::TYPOGRAPHIC);
/// `ellipsis.inner`: runs of full stops.
pub(crate) const INNER: Triggers = Triggers::of(b".");

/// Whether a token is absent (a window edge), a space or a line break.
fn boundary(cx: &Cx<'_>, j: Option<usize>) -> bool {
    j.and_then(|j| cx.kind(j))
        .is_none_or(|k| matches!(k, Kind::Space | Kind::Newline))
}

/// The form of the language's ellipsis data after `?`, `!` or `?!` before the dots.
fn form_after(cx: &Cx<'_>, prev: Option<usize>) -> Option<EllipsisForm> {
    let p = prev?;
    match cx.kind(p)? {
        Kind::Punct('?') => Some(EllipsisForm::Question),
        Kind::Punct('!') => {
            let before = cx.prev(p).and_then(|b| cx.kind(b));
            Some(if before == Some(Kind::Punct('?')) {
                EllipsisForm::Both
            } else {
                EllipsisForm::Exclamation
            })
        }
        _ => None,
    }
}

/// `ellipsis.dots`: exactly three full stops attached to the preceding token (a word, a number, a
/// closing quote or bracket, a sign; followed by a space, a line break, the end of the window,
/// closing punctuation or a closing quote), or forming a fragment of their own between boundaries
/// → `…`; after `?`, `!` or `?!` the language's form (design section 8.6).
pub(crate) fn dots(cx: &Cx<'_>, i: usize) -> Option<Match> {
    if cx.kind(i) != Some(Kind::Dots) {
        return None;
    }
    let prev = cx.prev(i);
    let next = cx.next(i);
    let form = form_after(cx, prev);
    let three = cx.is(i, "...");
    let typed = cx.is(i, "…") || form.is_some_and(|f| cx.is(i, ellipsis_tail(&cx.data, f)));
    if !three && !typed {
        return None;
    }
    let attached = !boundary(cx, prev);
    let word_end = attached
        && (boundary(cx, next)
            || next.is_some_and(|n| {
                matches!(
                    cx.kind(n),
                    Some(Kind::Punct(',' | ';' | ':' | '!' | '?') | Kind::Close(_))
                ) || local_role(cx, n) == Some(Role::Close)
            }));
    let alone = !attached && boundary(cx, next);
    if !(word_end || alone) {
        return None;
    }
    if typed {
        return Some(Match::done(0));
    }
    let repl = match form {
        Some(f) => Repl::Ellipsis(f),
        None => Repl::Char('…'),
    };
    Some(Match::with(
        0,
        Edits::NONE.with(Edit::Replace { at: 0, repl }),
    ))
}

/// `ellipsis.inner` (Prose): `...` at the start of a word after a space or a line start, or
/// between two words without spaces → `…`.
pub(crate) fn inner(cx: &Cx<'_>, i: usize) -> Option<Match> {
    if cx.kind(i) != Some(Kind::Dots) {
        return None;
    }
    let three = cx.is(i, "...");
    if !three && !cx.is(i, "…") {
        return None;
    }
    let prev = cx.prev(i);
    let next = cx.next(i);
    // `Word` and `Other` slots count as words (section 8.10), a protected span acts as one word
    // (section 2.6).
    let word = |j: Option<usize>| {
        j.is_some_and(|j| {
            matches!(
                cx.kind(j),
                Some(
                    Kind::Word(_) | Kind::Slot(SlotKind::Word | SlotKind::Other) | Kind::Protected
                )
            )
        })
    };
    if !(word(next) && (boundary(cx, prev) || word(prev))) {
        return None;
    }
    if !three {
        return Some(Match::done(0));
    }
    Some(Match::with(
        0,
        Edits::NONE.with(Edit::Replace {
            at: 0,
            repl: Repl::Char('…'),
        }),
    ))
}
