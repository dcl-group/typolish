//! The no-break rules `nbsp.*` (design section 8.7) with the common conditions of section 8.2.
//!
//! The rules set glue marks (design section 3.9) and never write U+00A0 into the text. Most of
//! them anchor at the space they glue and read the fragments on both sides, so when two rules
//! could glue the same space the first of them in the order table takes it (section 3.8). The
//! rules that may insert a space inside a fragment anchor before it: `nbsp.abbreviations` at the
//! first token of its entry, `nbsp.signs` at the sign, and `nbsp.units` at an attached `%` or `‰`.
//! A rule proposes a glue only for a single U+0020; a space the author already glued (U+00A0 or
//! U+202F) matches with an empty edit and is never rewritten (sections 4.3, I3, and 4.5).

use super::dash::{closing_punct, lone_dash};
use super::quotes::{Role, local_role};
use super::range::roman_value;
use super::{
    Frag, FragKind, Triggers, folded_in, is_operand, is_word, percent_owner, percent_range,
};
use crate::engine::lex::{Glue, Kind, NO_FRAG, script_bit};
use crate::engine::phase::{Cx, Edit, Edits, Match};
use crate::lang::data::{Bind, List, MonthPosition, Script, fold_char};
use crate::{NarrowGlue, PercentSpacing, SlotKind};

const SPACE: Triggers = Triggers::of(b" ");

/// `nbsp.abbreviations`: the space inside `т.е.` is inserted.
pub(crate) const ABBREVIATIONS: Triggers = SPACE.union(Triggers::of(b"."));
/// `nbsp.initials`.
pub(crate) const INITIALS: Triggers = SPACE;
/// `nbsp.dates`.
pub(crate) const DATES: Triggers = SPACE;
/// `nbsp.units`.
pub(crate) const UNITS: Triggers = SPACE;
/// `nbsp.units` when `PercentSpacing` is not `AsWritten`: `%` and the lead byte of `‰`.
pub(crate) const PERCENT: Triggers = Triggers::of(b"%").with_lead('‰');
/// `nbsp.signs`; the lead bytes of the language's signs are added (`№5` gets a space).
pub(crate) const SIGNS: Triggers = SPACE;
/// `nbsp.phone`.
pub(crate) const PHONE: Triggers = SPACE;
/// `nbsp.short-words`.
pub(crate) const SHORT_WORDS: Triggers = SPACE;
/// `nbsp.short-length`.
pub(crate) const SHORT_LENGTH: Triggers = SPACE;
/// `nbsp.particles`.
pub(crate) const PARTICLES: Triggers = SPACE;
/// `nbsp.dash`: the space before a dash; typed dashes are context.
pub(crate) const DASH: Triggers = SPACE.union(Triggers::TYPOGRAPHIC);

/// The offset of token `j` from the anchor `i`.
fn offset(i: usize, j: usize) -> Option<u8> {
    u8::try_from(j.checked_sub(i)?).ok()
}

/// The edits of one match, collected from its anchor.
struct Glues {
    anchor: usize,
    last: usize,
    edits: Edits,
}

impl Glues {
    fn new(anchor: usize) -> Glues {
        Glues {
            anchor,
            last: anchor,
            edits: Edits::NONE,
        }
    }

    /// Takes the tokens up to `j`.
    fn reach(&mut self, j: usize) {
        self.last = self.last.max(j);
    }

    /// Glues the pattern space `s`: an edit for a single U+0020, nothing for a space that is
    /// already glued (section 4.3, I3).
    fn glue(&mut self, cx: &Cx<'_>, s: usize, glue: Glue) -> Option<()> {
        let at = offset(self.anchor, s)?;
        if cx.plain_space(s) {
            self.edits = self.edits.with(Edit::Glue { at, glue });
        }
        self.reach(s);
        Some(())
    }

    /// Inserts a glued space before token `before`.
    fn insert(&mut self, before: usize, glue: Glue) -> Option<()> {
        let at = offset(self.anchor, before)?;
        self.edits = self.edits.with(Edit::Insert { before: at, glue });
        self.reach(before);
        Some(())
    }

    fn done(self) -> Option<Match> {
        Some(Match::with(offset(self.anchor, self.last)?, self.edits))
    }
}

/// A match at the space `s` that glues it alone.
fn glue_one(cx: &Cx<'_>, s: usize, glue: Glue) -> Option<Match> {
    let mut g = Glues::new(s);
    g.glue(cx, s, glue)?;
    g.done()
}

/// The glue of a place of `narrow_glue` (design section 2.4).
fn glue_of(cx: &Cx<'_>, place: NarrowGlue) -> Glue {
    if cx.t.narrow(place) {
        Glue::Narrow
    } else {
        Glue::NoBreak
    }
}

/// Whether a token is a slot of kind `Other` or belongs to a fragment that holds one: no glue
/// sticks to such a fragment, quoted or bracketed alike (`and "{o}"`), because a nowrap group
/// would take the whole fragment, slot included (sections 2.6, 8.2 and 11.5).
pub(crate) fn holds_other(cx: &Cx<'_>, i: usize) -> bool {
    let other = |j: usize| cx.kind(j) == Some(Kind::Slot(SlotKind::Other));
    other(i)
        || cx
            .frag_of(i)
            .is_some_and(|f| (f.start as usize..f.end as usize).any(other))
}

/// Whether a token separates fragments.
fn separator(cx: &Cx<'_>, i: usize) -> bool {
    matches!(cx.kind(i), Some(Kind::Space | Kind::Newline))
}

/// The tokens on each side of the pattern space `s` between two fragments on one line, neither
/// of them holding an `Other` slot (section 8.2).
fn sides(cx: &Cx<'_>, s: usize) -> Option<(usize, usize)> {
    if !cx.pattern_space(s) {
        return None;
    }
    let l = cx.prev(s)?;
    let r = cx.next(s)?;
    if separator(cx, l) || separator(cx, r) || holds_other(cx, l) || holds_other(cx, r) {
        return None;
    }
    Some((l, r))
}

/// The fragment whose core ends with token `l`, the last token before a space: no closing
/// punctuation follows the core (section 8.2: `и,` triggers nothing).
fn tail<'a>(cx: &Cx<'a>, l: usize) -> Option<&'a Frag> {
    cx.in_core(l).then(|| cx.frag_of(l)).flatten()
}

/// The fragment whose core starts with token `r`: no wrapper stands before the core.
fn head<'a>(cx: &Cx<'a>, r: usize) -> Option<&'a Frag> {
    let f = cx.frag_of(r)?;
    (cx.in_core(r) && cx.core(f).next() == Some(r)).then_some(f)
}

/// Whether nothing but `Format` and `Markup` tokens follows the core in its fragment.
fn bare_end(cx: &Cx<'_>, f: &Frag) -> bool {
    (f.core_end..f.end).all(|j| cx.kind(j as usize).is_some_and(Kind::is_transparent))
}

/// The last token of a fragment that is not a `Format` or `Markup` token.
fn last_visible(cx: &Cx<'_>, f: &Frag) -> Option<usize> {
    (f.start as usize..f.end as usize)
        .rev()
        .find(|&j| !cx.kind(j).is_some_and(Kind::is_transparent))
}

/// The tokens of a fragment without `Format` and `Markup` tokens, at most eight.
fn visible_tokens(cx: &Cx<'_>, f: &Frag) -> Option<([usize; 8], usize)> {
    let mut out = [0usize; 8];
    let mut len = 0;
    for j in f.start as usize..f.end as usize {
        if cx.kind(j).is_some_and(Kind::is_transparent) {
            continue;
        }
        *out.get_mut(len)? = j;
        len += 1;
    }
    Some((out, len))
}

/// Whether two tokens belong to one fragment.
fn same_frag(cx: &Cx<'_>, a: usize, b: usize) -> bool {
    match (cx.tok(a), cx.tok(b)) {
        (Some(x), Some(y)) => x.frag != NO_FRAG && x.frag == y.frag,
        _ => false,
    }
}

/// The 8.2 condition on the right side of a glued space: the next fragment starts with a letter,
/// a digit, a slot (not `Other`), a protected span, an opening quote or an opening bracket `(`,
/// or, where the rule names it, a sign of the language; never with `-`.
fn glueable(cx: &Cx<'_>, r: usize, sign: bool) -> bool {
    match cx.kind(r) {
        Some(Kind::Word(_) | Kind::Number | Kind::Protected | Kind::Open('(')) => true,
        Some(Kind::Slot(kind)) => kind != SlotKind::Other,
        Some(Kind::Quote) => local_role(cx, r) == Some(Role::Open),
        Some(Kind::Punct(c)) => sign && cx.data.signs.contains(&c),
        _ => false,
    }
}

/// The bit of the language's script in `script_bit`; `Script::Other` is the bit of the other
/// scripts.
fn language_bit(cx: &Cx<'_>) -> u8 {
    match cx.data.script {
        Script::Cyrillic => 1,
        Script::Latin => 2,
        Script::Greek => 4,
        _ => 8,
    }
}

/// The scripts of the letters of a core and their number.
fn letters(cx: &Cx<'_>, f: &Frag) -> (u8, usize) {
    cx.core(f)
        .filter(|&j| matches!(cx.kind(j), Some(Kind::Word(_))))
        .flat_map(|j| cx.visible(j, j))
        .filter(|c| c.is_alphabetic())
        .fold((0, 0), |(mask, n), c| (mask | script_bit(c), n + 1))
}

/// Whether a word is in the language's script; a word of two scripts is a word of each (section
/// 8.2).
fn in_script(cx: &Cx<'_>, f: &Frag) -> bool {
    letters(cx, f).0 & language_bit(cx) != 0
}

/// Whether a list holds the characters of a core compared with case, with the relaxation of
/// section 7.1: an entry that starts with a lowercase letter also matches the text with that
/// letter in uppercase, unless the text is a single letter with a full stop.
fn cased_in(list: List<'_>, key: impl Iterator<Item = char> + Clone) -> bool {
    if list
        .binary_search_by(|entry| entry.chars().cmp(key.clone()))
        .is_ok()
    {
        return true;
    }
    let mut rest = key.clone();
    let Some(first) = rest.next() else {
        return false;
    };
    let single_with_stop = first.is_alphabetic() && rest.clone().eq(['.']);
    if !first.is_uppercase() || single_with_stop {
        return false;
    }
    let lowered = core::iter::once(fold_char(first)).chain(rest);
    list.binary_search_by(|entry| entry.chars().cmp(lowered.clone()))
        .is_ok()
}

/// A percentage: a number with `%` or `‰` in its core (section 8.2).
fn percentage(cx: &Cx<'_>, f: &Frag) -> bool {
    f.kind == FragKind::Number
        && cx
            .core(f)
            .any(|j| matches!(cx.kind(j), Some(Kind::Punct('%' | '‰'))))
}

/// Whether a lone `%` or `‰` after spaces follows the fragment `f` and is read with it (section
/// 8.2: `100 %` and `100⍽%` are the percentage `100%`).
fn percent_after(cx: &Cx<'_>, f: &Frag) -> bool {
    let Some(last) = (f.start as usize..f.end as usize)
        .rev()
        .find(|&j| cx.kind(j).is_some_and(|k| !k.is_transparent()))
    else {
        return false;
    };
    cx.next(last)
        .filter(|&s| cx.kind(s) == Some(Kind::Space))
        .and_then(|s| cx.next(s))
        .and_then(|j| cx.frag_of(j))
        .and_then(|g| super::before_percent(cx, g))
        .is_some_and(|owner| owner.start == f.start)
}

/// The core tokens of a fragment, `Format` and `Markup` tokens skipped, at most eight.
fn core_tokens(cx: &Cx<'_>, f: &Frag) -> Option<([usize; 8], usize)> {
    let mut out = [0usize; 8];
    let mut len = 0;
    for j in cx.core(f) {
        *out.get_mut(len)? = j;
        len += 1;
    }
    Some((out, len))
}

/// The value of a `Number` token of at most two digits.
fn small_value(cx: &Cx<'_>, j: usize) -> Option<u32> {
    if cx.kind(j) != Some(Kind::Number) {
        return None;
    }
    let text = cx.src(j);
    if text.is_empty() || text.len() > 2 {
        return None;
    }
    text.parse().ok()
}

/// A date core (section 8.2): a `Version` core `D.M.YYYY` or `D.M.YY`, day 1 to 31, month 1 to 12.
fn date_core(cx: &Cx<'_>, f: &Frag) -> bool {
    if f.kind != FragKind::Version {
        return false;
    }
    let Some(([d, s1, m, s2, y, ..], 5)) = core_tokens(cx, f) else {
        return false;
    };
    let dot = |j: usize| cx.kind(j) == Some(Kind::Dots) && cx.is(j, ".");
    let year = cx.kind(y) == Some(Kind::Number) && matches!(cx.src(y).len(), 2 | 4);
    dot(s1)
        && dot(s2)
        && year
        && small_value(cx, d).is_some_and(|v| (1..=31).contains(&v))
        && small_value(cx, m).is_some_and(|v| (1..=12).contains(&v))
}

/// Whether the core is one slot of the kind.
fn slot_core(cx: &Cx<'_>, f: &Frag, kind: SlotKind) -> bool {
    matches!(core_tokens(cx, f), Some(([j, ..], 1)) if cx.kind(j) == Some(Kind::Slot(kind)))
}

/// A Roman numeral core (section 3.6).
fn roman(cx: &Cx<'_>, f: &Frag) -> bool {
    matches!(core_tokens(cx, f), Some(([j, ..], 1))
        if matches!(cx.kind(j), Some(Kind::Word(_))) && roman_value(cx.visible(j, j)).is_some())
}

/// What a year word follows: a number, a range, a `YearMonth` core, a date core or a `Number` or
/// `Date` slot; labels and percentages are not numbers here (section 8.2).
fn year_like(cx: &Cx<'_>, f: &Frag) -> bool {
    if f.label {
        return false;
    }
    match f.kind {
        FragKind::Number => !percentage(cx, f),
        FragKind::Range | FragKind::YearMonth => true,
        _ => date_core(cx, f) || slot_core(cx, f, SlotKind::Date),
    }
}

// ---------------------------------------------------------------------------------------------
// nbsp.abbreviations
// ---------------------------------------------------------------------------------------------

/// Matches one part of an abbreviation from token `j`: the visible text of consecutive tokens of
/// one fragment equals the part, compared with case and the relaxation of section 7.1. The full
/// stop that ends the last part may be the first full stop of a `Dots` token in any form
/// (`и т.д…`). Returns the last token of the part.
fn match_part(cx: &Cx<'_>, start: usize, part: &str, last: bool) -> Option<usize> {
    let mut chars = part.chars();
    let single = chars.next().is_some_and(char::is_alphabetic) && chars.as_str() == ".";
    let frag = cx.tok(start)?.frag;
    let mut rest = part;
    let mut first = true;
    let mut j = start;
    loop {
        if cx.tok(j)?.frag != frag {
            return None;
        }
        match cx.kind(j)? {
            Kind::Dots => {
                return (rest == "." && (cx.is(j, ".") || last)).then_some(j);
            }
            Kind::Word(_) | Kind::Dash => {
                for c in cx.visible(j, j) {
                    let mut tail = rest.chars();
                    let p = tail.next()?;
                    let relaxed = first && !single && p.is_lowercase() && fold_char(c) == p;
                    if c != p && !relaxed {
                        return None;
                    }
                    first = false;
                    rest = tail.as_str();
                }
                if rest.is_empty() {
                    return Some(j);
                }
            }
            _ => return None,
        }
        j = cx.next(j)?;
    }
}

/// Whether the token before the entry starting at `i` is, across one space, a number, a range, a
/// `YearMonth` core, a date core or a `Number` or `Date` slot (section 8.7: the entry is then a
/// unit or a year with a sentence full stop and does not bind forward).
fn after_quantity(cx: &Cx<'_>, i: usize) -> bool {
    let Some(p) = cx.prev(i) else {
        return false;
    };
    if cx.kind(p) != Some(Kind::Space) {
        return false;
    }
    let Some(q) = cx.prev(p) else {
        return false;
    };
    tail(cx, q).is_some_and(|f| year_like(cx, f))
}

/// One entry of the abbreviation list at the token `i`.
fn entry_at(cx: &Cx<'_>, i: usize, parts: List<'_>, bind: Bind) -> Option<Match> {
    let narrow = glue_of(cx, NarrowGlue::ABBREVIATIONS);
    let n = parts.len();
    let mut g = Glues::new(i);
    let mut start = i;
    let mut end = i;
    for k in 0..n {
        let part = parts.get(k)?;
        let last = k + 1 == n;
        end = match_part(cx, start, part, last)?;
        g.reach(end);
        if last {
            break;
        }
        let dotted = part.ends_with('.');
        let both_dotted = dotted && parts.get(k + 1).is_some_and(|p| p.ends_with('.'));
        // `Narrow` goes only between the parts of an entry; after a leading word it is `NoBreak`.
        let glue = if dotted { narrow } else { Glue::NoBreak };
        let after = cx.next(end)?;
        if cx.kind(after) == Some(Kind::Space) {
            let next = cx.next(after)?;
            if !cx.pattern_space(after) || head(cx, next).is_none() {
                return None;
            }
            g.glue(cx, after, glue)?;
            start = next;
        } else if both_dotted && same_frag(cx, after, end) {
            g.insert(after, glue)?;
            start = after;
        } else {
            return None;
        }
    }
    // The entry ends its word: no letter, digit or slot follows in the fragment.
    if let Some(after) = cx.next(end)
        && same_frag(cx, after, end)
        && matches!(
            cx.kind(after),
            Some(Kind::Word(_) | Kind::Number | Kind::Slot(_) | Kind::Protected)
        )
    {
        return None;
    }
    if bind != Bind::Inner && !after_quantity(cx, i) {
        forward(cx, &mut g, end, bind == Bind::NextCapitalized);
    }
    g.done()
}

/// The forward glue of a `Next` or `NextCapitalized` entry ending with token `end`.
fn forward(cx: &Cx<'_>, g: &mut Glues, end: usize, capitalized: bool) {
    let Some(s) = cx.next(end) else {
        return;
    };
    let Some((_, r)) = sides(cx, s) else {
        return;
    };
    if !glueable(cx, r, false) {
        return;
    }
    if capitalized {
        let upper = matches!(cx.kind(r), Some(Kind::Word(_)))
            && cx.visible(r, r).next().is_some_and(char::is_uppercase);
        if !upper {
            return;
        }
    }
    let _ = g.glue(cx, s, Glue::NoBreak);
}

/// `nbsp.abbreviations`: the spaces inside a listed abbreviation are glued and a missing space
/// after a full stop is inserted; `Next` entries glue to the following operand, `NextCapitalized`
/// only before a capitalized word. Anchors at the first token of the entry.
pub(crate) fn abbreviations(cx: &Cx<'_>, i: usize) -> Option<Match> {
    if !matches!(cx.kind(i), Some(Kind::Word(_))) {
        return None;
    }
    head(cx, i)?;
    let mut best: Option<(usize, Match)> = None;
    for (parts, bind) in cx.data.abbreviations.iter() {
        if best.as_ref().is_some_and(|(len, _)| *len >= parts.len()) {
            continue;
        }
        if let Some(m) = entry_at(cx, i, parts, bind) {
            best = Some((parts.len(), m));
        }
    }
    best.map(|(_, m)| m)
}

// ---------------------------------------------------------------------------------------------
// nbsp.initials
// ---------------------------------------------------------------------------------------------

/// The number of initials in a fragment, when it holds only initials: one capital letter of the
/// language's script with a full stop (`А.`), or a compact run of them (`А.С.`). A start wrapper
/// is allowed with `lead`, more closing punctuation after the last full stop with `trail`.
fn initials(cx: &Cx<'_>, f: &Frag, lead: bool, trail: bool) -> Option<usize> {
    let (core, len) = core_tokens(cx, f)?;
    let core = core.get(..len)?;
    let bit = language_bit(cx);
    let initial = |j: usize| {
        matches!(cx.kind(j), Some(Kind::Word(_))) && {
            let mut chars = cx.visible(j, j);
            matches!((chars.next(), chars.next()),
                (Some(c), None) if c.is_uppercase() && script_bit(c) == bit)
        }
    };
    let mut count = 0;
    for (k, &j) in core.iter().enumerate() {
        let ok = if k % 2 == 0 {
            initial(j)
        } else {
            cx.kind(j) == Some(Kind::Dots) && cx.is(j, ".")
        };
        if !ok {
            return None;
        }
        count += usize::from(k % 2 == 0);
    }
    if len % 2 == 0 || !f.stop {
        return None;
    }
    let stop = cx.next(*core.last()?)?;
    if !cx.is(stop, ".") {
        return None;
    }
    if !lead && cx.core(f).next() != first_visible(cx, f) {
        return None;
    }
    if !trail && last_visible(cx, f) != Some(stop) {
        return None;
    }
    Some(count)
}

/// The first token of a fragment that is not a `Format` or `Markup` token.
fn first_visible(cx: &Cx<'_>, f: &Frag) -> Option<usize> {
    (f.start as usize..f.end as usize).find(|&j| !cx.kind(j).is_some_and(Kind::is_transparent))
}

/// A surname: a capitalized word of the language's script of at least two letters.
fn surname(cx: &Cx<'_>, f: &Frag) -> bool {
    if !is_word(cx, f)
        || cx
            .core(f)
            .any(|j| !matches!(cx.kind(j), Some(Kind::Word(_) | Kind::Dash)))
    {
        return false;
    }
    let (mask, n) = letters(cx, f);
    n >= 2 && mask == language_bit(cx) && cx.core_chars(f).next().is_some_and(char::is_uppercase)
}

/// The space after a fragment and the token after that space, when the space is a pattern space.
fn space_after(cx: &Cx<'_>, f: &Frag) -> Option<(usize, usize)> {
    let s = cx.next(last_visible(cx, f)?)?;
    let (_, r) = sides(cx, s)?;
    Some((s, r))
}

/// Whether a run of initials continues before the fragment: an initial ends one pattern space
/// before it.
fn initial_before(cx: &Cx<'_>, f: &Frag) -> bool {
    let Some(p) = first_visible(cx, f).and_then(|j| cx.prev(j)) else {
        return false;
    };
    if !cx.pattern_space(p) {
        return false;
    }
    cx.prev(p)
        .and_then(|q| cx.frag_of(q))
        .is_some_and(|q| initials(cx, q, true, false).is_some())
}

/// `nbsp.initials`: `И. О. Фамилия`, `И. Фамилия`, `Фамилия И. О.`, `Фамилия И.` (at most three
/// initials; a run of four or more matches nowhere). Anchors at the first space of the pattern.
pub(crate) fn initials_rule(cx: &Cx<'_>, s: usize) -> Option<Match> {
    let (l, r) = sides(cx, s)?;
    let fl = cx.frag_of(l)?;
    let glue = glue_of(cx, NarrowGlue::INITIALS);
    let mut g = Glues::new(s);
    if let Some(n) = initials(cx, fl, true, false) {
        // `И. О. Фамилия`: `s` must be the first space of the run.
        if initial_before(cx, fl) {
            return None;
        }
        let mut count = n;
        let mut space = s;
        let mut right = r;
        loop {
            let fr = head(cx, right)?;
            g.glue(cx, space, glue)?;
            if let Some(n) = initials(cx, fr, false, false) {
                count += n;
                if count > 3 {
                    return None;
                }
                (space, right) = space_after(cx, fr)?;
                continue;
            }
            if !surname(cx, fr) {
                return None;
            }
            return g.done();
        }
    }
    // `Фамилия И. О.`: the surname ends at `l`.
    let fl = tail(cx, l)?;
    if !surname(cx, fl) {
        return None;
    }
    let mut count = 0;
    let mut space = s;
    let mut right = r;
    loop {
        let fr = head(cx, right)?;
        let n = initials(cx, fr, false, true)?;
        count += n;
        if count > 3 {
            return None;
        }
        g.glue(cx, space, glue)?;
        if initials(cx, fr, false, false).is_none() {
            break;
        }
        let Some((next_space, next)) = space_after(cx, fr) else {
            break;
        };
        match head(cx, next) {
            Some(f) if initials(cx, f, false, true).is_some() => {
                (space, right) = (next_space, next);
            }
            // Initials before a surname are the forward form, which the next space takes.
            Some(f) if surname(cx, f) => return None,
            _ => break,
        }
    }
    g.done()
}

// ---------------------------------------------------------------------------------------------
// nbsp.dates
// ---------------------------------------------------------------------------------------------

/// A day number 1 to 31 or a `Number` slot, not a label.
fn day(cx: &Cx<'_>, f: &Frag) -> bool {
    if f.kind != FragKind::Number || f.label {
        return false;
    }
    match core_tokens(cx, f) {
        Some(([j, ..], 1)) => {
            cx.kind(j) == Some(Kind::Slot(SlotKind::Number))
                || small_value(cx, j).is_some_and(|v| (1..=31).contains(&v))
        }
        _ => false,
    }
}

/// A month form starting a fragment: its core, or its core with the stop.
fn month_ahead(cx: &Cx<'_>, f: &Frag) -> bool {
    let months = cx.data.months;
    let chars = cx.core_chars(f);
    cased_in(months, chars.clone()) || (f.stop && cased_in(months, chars.chain(['.'])))
}

/// A month form that ends with token `l`: the core without a stop, or the core with a stop that
/// is a single full stop and the last token of the fragment.
fn month_behind(cx: &Cx<'_>, f: &Frag, l: usize) -> bool {
    let months = cx.data.months;
    let chars = cx.core_chars(f);
    if cx.in_core(l) {
        return cased_in(months, chars);
    }
    f.stop
        && cx.is(l, ".")
        && cx.core(f).last().and_then(|j| cx.next(j)) == Some(l)
        && cased_in(months, chars.chain(['.']))
}

/// A word of a list that ends in a full stop, read as the core with its stop.
fn dotted_word(cx: &Cx<'_>, f: &Frag, list: List<'_>) -> bool {
    f.stop && folded_in(list, cx.core_chars(f).chain(['.']))
}

/// What a century word follows: a Roman numeral, a number or a range of them, not a label.
fn century_like(cx: &Cx<'_>, f: &Frag) -> bool {
    !f.label
        && match f.kind {
            FragKind::Number => !percentage(cx, f),
            FragKind::Range => true,
            _ => roman(cx, f),
        }
}

/// `nbsp.dates`: day and month by `month_position`; a year before a year word ending in `.`;
/// a century before a century word ending in `.`. Anchors at the space.
pub(crate) fn dates(cx: &Cx<'_>, s: usize) -> Option<Match> {
    let (l, r) = sides(cx, s)?;
    let fl = cx.frag_of(l)?;
    let fr = head(cx, r)?;
    let data = &cx.data;
    let after_day = matches!(
        data.month_position,
        MonthPosition::AfterDay | MonthPosition::Both
    );
    let before_day = matches!(
        data.month_position,
        MonthPosition::BeforeDay | MonthPosition::Both
    );
    let left_core = cx.in_core(l);
    let found = (after_day && left_core && day(cx, fl) && month_ahead(cx, fr))
        || (before_day && month_behind(cx, fl, l) && day(cx, fr))
        || (left_core && year_like(cx, fl) && dotted_word(cx, fr, data.year_words))
        || (left_core && century_like(cx, fl) && dotted_word(cx, fr, data.century_words));
    found.then(|| glue_one(cx, s, Glue::NoBreak)).flatten()
}

// ---------------------------------------------------------------------------------------------
// nbsp.units
// ---------------------------------------------------------------------------------------------

/// A quantity for `nbsp.units`: a number (not a label or a percentage), a `Number` slot or a
/// range (not a label).
fn quantity(cx: &Cx<'_>, f: &Frag) -> bool {
    !f.label
        && match f.kind {
            FragKind::Number => !percentage(cx, f),
            FragKind::Range => true,
            _ => false,
        }
}

/// The unit that starts a fragment at token `r`: the longest entry of the unit list that is a
/// prefix of its visible text and is followed by a character that is neither a letter nor a
/// digit, or by the end of the fragment; a unit of any length (section 8.7). Returns whether the
/// unit ends the fragment and whether it is an ambiguous unit.
fn unit_at(cx: &Cx<'_>, r: usize, f: &Frag) -> Option<(bool, bool)> {
    let last = last_visible(cx, f)?;
    let text = cx.visible(r, last);
    let mut best: Option<(usize, bool, &str)> = None;
    cx.t.unit_prefixes(text.clone(), |k, unit| {
        let after = text.clone().nth(k);
        if after.is_some_and(char::is_alphanumeric) || best.is_some_and(|(len, ..)| len >= k) {
            return;
        }
        best = Some((k, after.is_none(), unit));
    });
    let (_, ends, unit) = best?;
    Some((ends, cx.data.ambiguous_units.contains(unit)))
}

/// The percent setting in effect: `Language` read as the CLDR pattern of the language.
fn percent_spaced(cx: &Cx<'_>) -> Option<bool> {
    match cx.t.percent() {
        PercentSpacing::AsWritten => None,
        PercentSpacing::Language => Some(cx.data.numbers.percent_spaced),
        PercentSpacing::Spaced => Some(true),
        _ => Some(false),
    }
}

/// `nbsp.units`: a quantity, one space, then a unit of the list → glue; `%` and `‰` follow
/// `PercentSpacing`. Anchors at the space, or at an attached `%` or `‰` where it inserts one.
pub(crate) fn units(cx: &Cx<'_>, i: usize) -> Option<Match> {
    match cx.kind(i)? {
        Kind::Space => units_at_space(cx, i),
        Kind::Punct('%' | '‰') => percent_attached(cx, i),
        _ => None,
    }
}

fn units_at_space(cx: &Cx<'_>, s: usize) -> Option<Match> {
    let (l, r) = sides(cx, s)?;
    let fl = tail(cx, l)?;
    if !quantity(cx, fl) {
        return None;
    }
    let fr = cx.frag_of(r)?;
    if matches!(cx.kind(r), Some(Kind::Punct('%' | '‰'))) {
        // A percentage after a space: the sign alone is the core.
        if head(cx, r).is_none() || !matches!(core_tokens(cx, fr), Some((_, 1))) {
            return None;
        }
        return match percent_spaced(cx) {
            None | Some(true) => glue_one(cx, s, Glue::NoBreak),
            Some(false) if cx.plain_space(s) => {
                Some(Match::with(0, Edits::NONE.with(Edit::Remove { at: 0 })))
            }
            // A U+00A0 or U+202F typed by the author stays (section 5.3).
            Some(false) => Some(Match::done(0)),
        };
    }
    let (ends, ambiguous) = unit_at(cx, r, fr)?;
    if ambiguous && ends {
        // An ambiguous unit is glued only before punctuation, a line break or the end.
        let after = cx.next(last_visible(cx, fr)?);
        if !after.is_none_or(|j| cx.kind(j) == Some(Kind::Newline)) {
            return None;
        }
    }
    glue_one(cx, s, Glue::NoBreak)
}

/// An attached `%` or `‰` of a number or of a percentage range (`5–10%`, section 8.2) with a
/// setting that wants it spaced: a glued space is inserted before it.
fn percent_attached(cx: &Cx<'_>, i: usize) -> Option<Match> {
    if percent_spaced(cx) != Some(true) {
        return None;
    }
    let f = cx.frag_of(i)?;
    let kind = f.kind == FragKind::Number || percent_range(cx, f);
    if !kind || f.label || !cx.in_core(i) {
        return None;
    }
    let p = cx.prev(i)?;
    if !same_frag(cx, p, i)
        || !matches!(
            cx.kind(p),
            Some(Kind::Number | Kind::Slot(SlotKind::Number))
        )
    {
        return None;
    }
    let mut g = Glues::new(i);
    g.insert(i, Glue::NoBreak)?;
    g.done()
}

// ---------------------------------------------------------------------------------------------
// nbsp.signs
// ---------------------------------------------------------------------------------------------

/// `nbsp.signs`: `№`, `§`, `¶` (single or doubled) before a label number, a Roman numeral or a
/// slot (not `Other`): a space is glued, a missing space is inserted. Anchors at the sign.
pub(crate) fn signs(cx: &Cx<'_>, i: usize) -> Option<Match> {
    let Some(Kind::Punct(c)) = cx.kind(i) else {
        return None;
    };
    if !cx.data.signs.contains(&c) {
        return None;
    }
    let f = head(cx, i)?;
    let mut last_sign = i;
    let mut next = cx.next(i)?;
    if cx.kind(next) == Some(Kind::Punct(c)) && same_frag(cx, next, i) && cx.in_core(next) {
        last_sign = next;
        next = cx.next(next)?;
    }
    let glue = glue_of(cx, NarrowGlue::SIGNS);
    let mut g = Glues::new(i);
    g.reach(last_sign);
    match f.kind {
        FragKind::Sign => {
            let (_, r) = sides(cx, next)?;
            let target = match cx.kind(r) {
                Some(Kind::Slot(kind)) => kind != SlotKind::Other,
                _ => head(cx, r).is_some_and(|fr| {
                    fr.label
                        && (matches!(fr.kind, FragKind::Number | FragKind::Range) || roman(cx, fr))
                }),
            };
            // Never before a fragment that starts with `-` (section 8.2): `№ -5` stays.
            if !target || !glueable(cx, r, false) {
                return None;
            }
            g.glue(cx, next, glue)?;
        }
        FragKind::Word if f.label && same_frag(cx, next, i) => {
            g.insert(next, glue)?;
        }
        _ => return None,
    }
    g.done()
}

// ---------------------------------------------------------------------------------------------
// nbsp.phone
// ---------------------------------------------------------------------------------------------

/// The digits of the `Number` tokens of a fragment.
fn digit_count(cx: &Cx<'_>, tokens: &[usize]) -> usize {
    tokens
        .iter()
        .filter(|&&j| cx.kind(j) == Some(Kind::Number))
        .map(|&j| cx.src(j).len())
        .sum()
}

/// A bracketed code `(DDD)`, the whole fragment or its start.
fn bracketed(cx: &Cx<'_>, tokens: &[usize]) -> bool {
    matches!(tokens, [o, n, c, ..]
        if cx.kind(*o) == Some(Kind::Open('('))
            && cx.kind(*n) == Some(Kind::Number)
            && cx.src(*n).len() == 3
            && cx.kind(*c) == Some(Kind::Close(')')))
}

/// A further group of a telephone number: two to four digits, a bracketed code, or a hyphenated
/// group (a `Compound` or `Range` of numbers). Returns its digits and whether closing punctuation
/// follows it, which ends the run.
fn phone_group(cx: &Cx<'_>, f: &Frag) -> Option<(usize, bool)> {
    let (tokens, len) = visible_tokens(cx, f)?;
    let tokens = tokens.get(..len)?;
    if bracketed(cx, tokens) {
        return Some((3, len > 3));
    }
    let (core, n) = core_tokens(cx, f)?;
    let core = core.get(..n)?;
    if first_visible(cx, f) != core.first().copied() {
        return None;
    }
    // A percentage is not a group, in both of its forms (section 8.2): `100 %` and `100⍽%` read as
    // `100%`, so the space that the percent setting inserts changes nothing (invariant I2).
    if percentage(cx, f) || percent_after(cx, f) {
        return None;
    }
    let digits = digit_count(cx, core);
    let ok = match core {
        [one] => cx.kind(*one) == Some(Kind::Number) && (2..=4).contains(&digits),
        _ => {
            matches!(f.kind, FragKind::Compound | FragKind::Range)
                && core.iter().enumerate().all(|(k, &j)| {
                    if k % 2 == 0 {
                        cx.kind(j) == Some(Kind::Number)
                    } else {
                        cx.kind(j) == Some(Kind::Dash) && (cx.is(j, "-") || cx.is(j, "\u{2013}"))
                    }
                })
                && n % 2 == 1
        }
    };
    ok.then_some((digits, !bare_end(cx, f)))
}

/// `nbsp.phone` (Prose): a country code `+D` or a bracketed area code (optionally after a
/// one-digit trunk prefix), then one to four groups, 10 to 15 digits in all, separated by single
/// spaces: every space is glued. Anchors at the first space of the number.
pub(crate) fn phone(cx: &Cx<'_>, s: usize) -> Option<Match> {
    let (l, r) = sides(cx, s)?;
    let fl = cx.frag_of(l)?;
    let (tokens, len) = visible_tokens(cx, fl)?;
    let tokens = tokens.get(..len)?;
    let mut g = Glues::new(s);
    let mut digits;
    let mut space = s;
    let mut right = r;
    match tokens {
        [plus, n] if cx.kind(*plus) == Some(Kind::Punct('+')) => {
            digits = digit_count(cx, &[*n]);
            if cx.kind(*n) != Some(Kind::Number) || !(1..=3).contains(&digits) {
                return None;
            }
        }
        [_, _, _] if bracketed(cx, tokens) => digits = 3,
        [n] if cx.kind(*n) == Some(Kind::Number) && cx.src(*n).len() == 1 => {
            // A trunk prefix before a bracketed code: `8 (800)`.
            let fr = cx.frag_of(r)?;
            let (code, k) = visible_tokens(cx, fr)?;
            if k != 3 || !bracketed(cx, code.get(..k)?) {
                return None;
            }
            g.glue(cx, s, Glue::NoBreak)?;
            digits = 4;
            (space, right) = space_after(cx, fr)?;
        }
        _ => return None,
    }
    let mut groups = 0;
    while let Some(fr) = cx.frag_of(right) {
        let Some((n, closed)) = phone_group(cx, fr) else {
            break;
        };
        groups += 1;
        if groups > 4 {
            return None;
        }
        digits += n;
        g.glue(cx, space, Glue::NoBreak)?;
        if closed {
            break;
        }
        let Some(next) = space_after(cx, fr) else {
            break;
        };
        (space, right) = next;
    }
    (groups > 0 && (10..=15).contains(&digits))
        .then(|| g.done())
        .flatten()
}

// ---------------------------------------------------------------------------------------------
// nbsp.short-words, nbsp.short-length, nbsp.particles
// ---------------------------------------------------------------------------------------------

/// `nbsp.short-words`: a word of the list (no case), one space, then an operand, an opening
/// quote or bracket, or a sign of the language → glue. Anchors at the space.
pub(crate) fn short_words(cx: &Cx<'_>, s: usize) -> Option<Match> {
    let (l, r) = sides(cx, s)?;
    let f = tail(cx, l)?;
    let found = is_word(cx, f)
        && cx.t.has_short_word(cx.core_chars(f))
        && (cx.t.short_words_any_script() || in_script(cx, f))
        && glueable(cx, r, true);
    found.then(|| glue_one(cx, s, Glue::NoBreak)).flatten()
}

/// `nbsp.short-length` (Prose): the same for any word of the language's script of at most
/// `short_word_length` letters. Anchors at the space.
pub(crate) fn short_length(cx: &Cx<'_>, s: usize) -> Option<Match> {
    let (l, r) = sides(cx, s)?;
    let f = tail(cx, l)?;
    // A compound word is a word (section 8.2): `25-й` has one letter.
    if !is_word(cx, f) {
        return None;
    }
    let (_, n) = letters(cx, f);
    let found = n >= 1
        && n <= usize::from(cx.t.short_word_length())
        && (cx.t.short_words_any_script() || in_script(cx, f))
        && glueable(cx, r, true);
    found.then(|| glue_one(cx, s, Glue::NoBreak)).flatten()
}

/// `nbsp.particles`: a word or a slot (not `Other`), one space, then a particle of the list (a
/// whole core, no closing punctuation after it) → glue. Anchors at the space.
pub(crate) fn particles(cx: &Cx<'_>, s: usize) -> Option<Match> {
    let (l, r) = sides(cx, s)?;
    let fl = tail(cx, l)?;
    let left = is_word(cx, fl) || matches!(cx.kind(l), Some(Kind::Slot(_)));
    let fr = head(cx, r)?;
    let found = left && bare_end(cx, fr) && cx.t.has_particle(cx.core_chars(fr));
    found.then(|| glue_one(cx, s, Glue::NoBreak)).flatten()
}

// ---------------------------------------------------------------------------------------------
// nbsp.dash
// ---------------------------------------------------------------------------------------------

/// `nbsp.dash` (style `SpacedGlued`): the space before a lone `—` (produced or typed) or a typed
/// `–` in the context of `dash.spaced-hyphen` is glued; its refusals do not apply. Anchors at
/// the space.
pub(crate) fn dash(cx: &Cx<'_>, s: usize) -> Option<Match> {
    if !cx.pattern_space(s) {
        return None;
    }
    let d = cx.next(s)?;
    let dash = cx.kind(d) == Some(Kind::Dash) && (cx.is(d, "\u{2014}") || cx.is(d, "\u{2013}"));
    if !dash || !lone_dash(cx, d) {
        return None;
    }
    let s2 = cx.next(d)?;
    if !cx.pattern_space(s2) {
        return None;
    }
    let l = cx.prev(s)?;
    let r = cx.next(s2)?;
    if holds_other(cx, l) {
        return None;
    }
    let left = if cx.in_core(l) {
        cx.frag_of(l)
            .is_some_and(|f| is_operand(cx, percent_owner(cx, f)))
    } else {
        closing_punct(cx, l)
    };
    let right = head(cx, r).is_some_and(|f| is_operand(cx, f))
        || matches!(cx.kind(r), Some(Kind::Open(_)))
        || local_role(cx, r) == Some(Role::Open);
    (left && right)
        .then(|| glue_one(cx, s, Glue::NoBreak))
        .flatten()
}
