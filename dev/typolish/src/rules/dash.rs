//! `dash.dialogue`, `dash.double-hyphen` and `dash.spaced-hyphen` (design section 8.4).
//!
//! `dash.dialogue` anchors at the dash at a line start. The other two anchor at the space before a
//! dash fragment, so that the closed style can remove it; they take the space, the dash and the
//! space after it.

use super::nbsp::holds_other;
use super::quotes::{Role, local_role};
use super::{
    Frag, FragKind, Triggers, foreign, is_number, is_operand, is_range, is_technical,
    percent_owner, plain_side,
};
use super::{is_word, single_letter, technical_sign};
use crate::DashStyle;
use crate::SlotKind;
use crate::engine::lex::{Glue, Kind, NO_FRAG, Repl};
use crate::engine::phase::{Cx, Edit, Edits, Match};

/// `dash.dialogue`: a hyphen at a line start; a typed dash there is matched too, and its space
/// glued (design section 4.3, I4).
pub(crate) const DIALOGUE: Triggers = Triggers::of(b"-").union(Triggers::TYPOGRAPHIC);
/// `dash.double-hyphen`: `--` and `---`.
pub(crate) const DOUBLE_HYPHEN: Triggers = Triggers::of(b"-");
/// `dash.spaced-hyphen`: a lone hyphen; typed dashes are matched with an empty edit.
pub(crate) const SPACED_HYPHEN: Triggers = Triggers::of(b"-").union(Triggers::TYPOGRAPHIC);

const EM_DASH: char = '\u{2014}';

/// The offset of token `j` from the anchor `i`.
fn offset(i: usize, j: usize) -> Option<u8> {
    u8::try_from(j.checked_sub(i)?).ok()
}

/// Whether the token at `d` is a dash fragment on its own: its fragment holds no other token than
/// `Format` and `Markup` tokens (no wrapper either).
///
/// A fragment is a contiguous run of tokens, so it is enough that neither non-transparent neighbor
/// belongs to it; the fragment itself is not scanned, which keeps the phase linear when a long run
/// of transparent tokens precedes many dashes (design section 3.12).
pub(crate) fn lone_dash(cx: &Cx<'_>, d: usize) -> bool {
    let Some(f) = cx.tok(d).map(|t| t.frag) else {
        return false;
    };
    let same = |j: Option<usize>| j.is_some_and(|j| cx.tok(j).is_some_and(|t| t.frag == f));
    f != NO_FRAG
        && cx
            .frag_of(d)
            .is_some_and(|fr| fr.kind == FragKind::OperatorRun)
        && !same(cx.prev(d))
        && !same(cx.next(d))
}

/// `dash.dialogue` (design section 8.4): at the start of the text or of a line, after optional
/// indentation, `-` or `--`, one space, then a word or an opening quote → `—` and glue after it.
pub(crate) fn dialogue(cx: &Cx<'_>, i: usize) -> Option<Match> {
    if cx.kind(i) != Some(Kind::Dash) {
        return None;
    }
    let plain = cx.is(i, "-") || cx.is(i, "--");
    let typed = cx.is(i, "\u{2014}") || cx.is(i, "\u{2013}");
    if !(plain || typed) || !lone_dash(cx, i) || !cx.line_start(i) {
        return None;
    }
    let s = cx.next(i)?;
    if !cx.pattern_space(s) {
        return None;
    }
    let r = cx.next(s)?;
    // `Word` and `Other` slots count as words (design section 8.10), a protected span acts as one
    // word (design section 2.6).
    let opaque = cx.kind(r)?;
    let word = match opaque {
        Kind::Slot(kind) => matches!(kind, SlotKind::Word | SlotKind::Other),
        Kind::Protected => true,
        _ => {
            cx.in_core(r)
                && cx
                    .frag_of(r)
                    .is_some_and(|f| is_word(cx, f) && cx.core(f).next() == Some(r))
        }
    };
    if !word && local_role(cx, r) != Some(Role::Open) {
        return None;
    }
    let at = offset(i, s)?;
    let mut edits = Edits::NONE;
    if plain {
        edits = edits.with(Edit::Replace {
            at: 0,
            repl: Repl::Char(EM_DASH),
        });
    }
    // No glue sticks to an `Other` slot or a fragment that holds one (design sections 2.6 and 8.2):
    // the dash is set, the space stays.
    if cx.plain_space(s) && !holds_other(cx, r) {
        edits = edits.with(Edit::Glue {
            at,
            glue: Glue::NoBreak,
        });
    }
    Some(Match::with(at, edits))
}

/// `dash.double-hyphen` (Prose): a lone `--` or `---` between operands → a dash in the language's
/// style.
pub(crate) fn double_hyphen(cx: &Cx<'_>, i: usize) -> Option<Match> {
    between(cx, i, true)
}

/// `dash.spaced-hyphen`: a lone `-` between operands → a dash in the language's style; a typed
/// `—` or `–` is matched with an empty edit.
pub(crate) fn spaced_hyphen(cx: &Cx<'_>, i: usize) -> Option<Match> {
    between(cx, i, false)
}

/// Closing punctuation on the left of a dash: `, . ! ? … )`, a closing quote, or a prime `′` `″`,
/// which `quotes.primes` writes after a number in place of a straight quote that has the closing
/// role there (`5'10" - x`), so the prime reads as the quote it was typed as (design sections 4.3,
/// I4, and 8.4).
pub(crate) fn closing_punct(cx: &Cx<'_>, l: usize) -> bool {
    matches!(
        cx.kind(l),
        Some(Kind::Punct(',' | '!' | '?') | Kind::Dots | Kind::Close(')'))
    ) || local_role(cx, l) == Some(Role::Close)
        || (cx.kind(l) == Some(Kind::Quote) && matches!(cx.chr(l), Some('\u{2032}' | '\u{2033}')))
}

/// A number, a range or a single-letter word: one side of the first refusal.
fn small(cx: &Cx<'_>, f: &Frag, left: bool) -> bool {
    is_number(f) || is_range(cx, f) || single_letter(cx, f, left)
}

/// A flag or another fragment that starts with `-`; a signed number also with `−` or with the
/// minus sign of the settings, the form that `number.minus` gives its sign, so that the minus
/// changes nothing the dash rules read (design sections 4.3, I4, and 8.4).
fn flag_like(cx: &Cx<'_>, f: &Frag) -> bool {
    f.kind == FragKind::Flag
        || (f.start as usize..f.end as usize)
            .find(|&j| !cx.kind(j).is_some_and(Kind::is_transparent))
            .is_some_and(|j| {
                let src = cx.src(j);
                cx.kind(j) == Some(Kind::Dash)
                    && (src.starts_with('-')
                        || (f.kind == FragKind::Number
                            && (src.starts_with('\u{2212}') || src.starts_with(cx.t.minus_sign()))))
            })
}

/// A word of letters for the closed style (design section 8.4): a `Word` core or a one-word
/// `Dictionary` core of letters with their marks and inner apostrophes only.
fn plain_word(cx: &Cx<'_>, f: &Frag) -> bool {
    if cx.kind_of(f) != FragKind::Word {
        return false;
    }
    let mut words = 0;
    for j in cx.core(f) {
        match cx.kind(j) {
            Some(Kind::Word(_)) => words += 1,
            Some(Kind::Quote) if matches!(cx.chr(j), Some('\'' | '\u{2019}')) => {
                let word = |k: Option<usize>| {
                    k.is_some_and(|k| cx.in_core(k) && matches!(cx.kind(k), Some(Kind::Word(_))))
                };
                if !(word(cx.prev(j)) && word(cx.next(j))) {
                    return false;
                }
            }
            _ => return false,
        }
    }
    words > 0
}

/// The two rules that put a dash between operands; `i` is the space before the dash.
fn between(cx: &Cx<'_>, i: usize, double: bool) -> Option<Match> {
    if !cx.pattern_space(i) {
        return None;
    }
    let d = cx.next(i)?;
    if cx.kind(d) != Some(Kind::Dash) {
        return None;
    }
    let typed = !double && (cx.is(d, "\u{2014}") || cx.is(d, "\u{2013}"));
    let shape = if double {
        cx.is(d, "--") || cx.is(d, "---")
    } else {
        typed || cx.is(d, "-")
    };
    if !shape || !lone_dash(cx, d) {
        return None;
    }
    let s2 = cx.next(d)?;
    if !cx.pattern_space(s2) {
        return None;
    }
    let l = cx.prev(i)?;
    let r = cx.next(s2)?;
    // A percent sign after spaces is read with its number (design section 8.2).
    let fl = percent_owner(cx, cx.frag_of(l)?);
    let fr = cx.frag_of(r)?;
    let left = if cx.in_core(l) {
        is_operand(cx, fl)
    } else {
        closing_punct(cx, l)
    };
    let right = (cx.in_core(r) && is_operand(cx, fr) && cx.core(fr).next() == Some(r))
        || matches!(cx.kind(r), Some(Kind::Open(_)))
        || local_role(cx, r) == Some(Role::Open);
    if !left || !right {
        return None;
    }
    let od = offset(i, d)?;
    let os2 = offset(i, s2)?;
    if typed {
        return Some(Match::done(os2));
    }
    // The refusals read the part of each side next to the dash, as it stood before a merge or a
    // split of the first pass: across a wrapper, `Moscow—a,` reads as `a,` (design sections 4.3,
    // I4, and 8.4).
    let (pl, pr) = (plain_side(cx, fl, true), plain_side(cx, fr, false));
    let foreign_or_technical =
        |f: &Frag| foreign(cx, f) || is_technical(f) || technical_sign(cx, f);
    let refused = (small(cx, &pl, true) && small(cx, &pr, false))
        || flag_like(cx, &pl)
        || flag_like(cx, &pr)
        || (foreign_or_technical(&pl) && foreign_or_technical(&pr));
    if refused {
        return Some(Match::refused(od, od));
    }
    let dash = Edit::Replace {
        at: od,
        repl: Repl::Char(EM_DASH),
    };
    let closed = cx.dash.is_some_and(|d| d.style == DashStyle::Closed)
        && cx.plain_space(i)
        && cx.plain_space(s2)
        && cx.in_core(l)
        && cx.in_core(r)
        && plain_word(cx, fl)
        && plain_word(cx, fr);
    let edits = if closed {
        Edits::NONE
            .with(Edit::Remove { at: 0 })
            .with(dash)
            .with(Edit::Remove { at: os2 })
    } else {
        Edits::NONE.with(dash)
    };
    Some(Match::with(os2, edits))
}
