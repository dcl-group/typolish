//! `quotes.apostrophe`, `quotes.pairs` and `quotes.primes` (design sections 6 and 8.3).
//!
//! The quote phase has a linear preliminary pass over the window (`analyze`): certain apostrophes
//! and primes (step 1, design section 6.2), roles of series (6.3), families and stack matching
//! (6.4), quotes after numbers in two passes (6.5), the all-or-nothing decision (6.6), levels (6.7)
//! and single quotes at the top level (6.8). It leaves one decision per token; the three rules only
//! read the decisions and propose the edits, each attributed to the rule whose behavior it is.
//! Classification does not depend on which of the three rules are enabled.

use super::Triggers;
use crate::DiagnosticKind;
use crate::Severity;
use crate::SingleQuotes;
use crate::engine::lex::{Kind, Repl, Tok};
use crate::engine::phase::{Cx, Edit, Edits, Match, loc};
use crate::engine::record::{Recorder, Sink};
use crate::lang::data::fold_char;

/// `quotes.apostrophe`: `'` between letters and after digits.
pub(crate) const APOSTROPHE: Triggers = Triggers::of(b"'");
/// `quotes.pairs`: straight quotes; typed quotes are context.
pub(crate) const PAIRS: Triggers = Triggers::of(b"\"'").union(Triggers::TYPOGRAPHIC);
/// `quotes.primes`: `'` and `"` after numbers.
pub(crate) const PRIMES: Triggers = Triggers::of(b"\"'").union(Triggers::DIGITS);

/// The apostrophe, U+2019 for every language (design section 6.2).
const APOSTROPHE_CHAR: char = '\u{2019}';

/// The decision of the quote phase for one token.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub(crate) enum Mark {
    /// No edit.
    #[default]
    None,
    /// `’` (`quotes.apostrophe`).
    Apostrophe,
    /// A glyph of a quote pair (`quotes.pairs`).
    Glyph(char),
    /// `′` or `″` (`quotes.primes`).
    Prime(char),
}

/// `quotes.apostrophe`.
pub(crate) fn apostrophe(cx: &Cx<'_>, i: usize) -> Option<Match> {
    match cx.quotes.get(i) {
        Some(Mark::Apostrophe) => Some(replace(APOSTROPHE_CHAR)),
        _ => None,
    }
}

/// `quotes.pairs`.
pub(crate) fn pairs(cx: &Cx<'_>, i: usize) -> Option<Match> {
    match cx.quotes.get(i) {
        Some(Mark::Glyph(c)) => Some(replace(*c)),
        _ => None,
    }
}

/// `quotes.primes`.
pub(crate) fn primes(cx: &Cx<'_>, i: usize) -> Option<Match> {
    match cx.quotes.get(i) {
        Some(Mark::Prime(c)) => Some(replace(*c)),
        _ => None,
    }
}

fn replace(c: char) -> Match {
    Match::with(
        0,
        Edits::NONE.with(Edit::Replace {
            at: 0,
            repl: Repl::Char(c),
        }),
    )
}

/// The role of a quote.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Role {
    Open,
    Close,
}

/// The class of a neighbor of a series (design section 6.3).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Class {
    Space,
    OpenP,
    CloseP,
    DashC,
    Word,
}

impl Class {
    const fn opens(self) -> bool {
        matches!(self, Class::Space | Class::OpenP | Class::DashC)
    }

    const fn ends(self) -> bool {
        matches!(self, Class::Space | Class::CloseP | Class::DashC)
    }
}

/// The class of a token; `None` is the start or the end of the window.
fn class(cx: &Cx<'_>, j: Option<usize>) -> Class {
    let Some(t) = j.and_then(|j| cx.tok(j)) else {
        return Class::Space;
    };
    // A neighbor inside a frozen core is of class `Word`, whatever its own token.
    if t.is_frozen() && t.kind != Kind::Space {
        return Class::Word;
    }
    match t.kind {
        Kind::Space | Kind::Newline => Class::Space,
        Kind::Open(_) | Kind::Operator(_) | Kind::Punct('/') => Class::OpenP,
        Kind::Close(_) | Kind::Dots | Kind::Punct('.' | ',' | ';' | ':' | '!' | '?') => {
            Class::CloseP
        }
        Kind::Dash => Class::DashC,
        _ => Class::Word,
    }
}

fn is_quote(cx: &Cx<'_>, j: usize) -> bool {
    cx.kind(j) == Some(Kind::Quote)
}

/// L and R of the series that holds the quote at `i`: the nearest tokens before and after the
/// whole run of adjacent quotes.
fn neighbors(cx: &Cx<'_>, i: usize) -> (Option<usize>, Option<usize>) {
    let mut l = cx.prev(i);
    while let Some(j) = l.filter(|&j| is_quote(cx, j)) {
        l = cx.prev(j);
    }
    let mut r = cx.next(i);
    while let Some(j) = r.filter(|&j| is_quote(cx, j)) {
        r = cx.next(j);
    }
    (l, r)
}

/// Whether, from `r` on, a run of `CloseP` tokens within the fragment of `r` is followed, in the
/// same fragment, by a token of class `Word` (`".env"`, `"...и тогда"`).
fn looks_through(cx: &Cx<'_>, r: Option<usize>) -> bool {
    let Some(start) = r else {
        return false;
    };
    let frag = cx.tok(start).map(|t| t.frag);
    let mut k = Some(start);
    while let Some(j) = k {
        if cx.tok(j).map(|t| t.frag) != frag {
            return false;
        }
        match class(cx, Some(j)) {
            Class::CloseP => k = cx.next(j),
            Class::Word => return true,
            _ => return false,
        }
    }
    false
}

/// The role of a straight quote from the classes of its series' neighbors (design section 6.3),
/// without the sign-only pair.
fn straight_role(cx: &Cx<'_>, l: Option<usize>, r: Option<usize>) -> Option<Role> {
    let lc = class(cx, l);
    let rc = class(cx, r);
    if lc.opens() {
        if !matches!(rc, Class::Space | Class::CloseP)
            || (rc == Class::CloseP && looks_through(cx, r))
        {
            return Some(Role::Open);
        }
        return None;
    }
    rc.ends().then_some(Role::Close)
}

/// The role of a quote by the local rule of design section 6.3, as other rules read it (design
/// section 8.2): a typed glyph has the role of its glyph in the language's pairs, except U+2019,
/// which closes only where the closing test of design section 6.3 holds; a straight quote has the
/// role its neighbors give, the sign-only pair included; a quote that step 1 (design section 6.2)
/// makes an apostrophe, plain or typed, has no role. So a glyph has on a second pass the role its
/// plain form had on the first (design section 4.3, I4), except the apostrophes of design section
/// 6.6, which depend on pairing.
pub(crate) fn local_role(cx: &Cx<'_>, i: usize) -> Option<Role> {
    if !is_quote(cx, i) {
        return None;
    }
    let glyph = cx.chr(i)?;
    // A certain apostrophe of step 1 (design section 6.2), plain or typed, is no quote for other
    // rules.
    if matches!(glyph, '\'' | APOSTROPHE_CHAR) && step1(cx, i, glyph) == Some(Mark::Apostrophe) {
        return None;
    }
    match glyph {
        '"' | '\'' => {
            let (l, r) = neighbors(cx, i);
            // As in the quote phase, a later pair overrides an earlier one: the opening role of a
            // pair that starts here wins over the closing role of a pair that ends here.
            if class(cx, l).opens() && sign_only_partner(cx, i, || series_count(cx, i)).is_some() {
                return Some(Role::Open);
            }
            if class(cx, r).ends() && sign_only_opener(cx, i).is_some() {
                return Some(Role::Close);
            }
            straight_role(cx, l, r)
        }
        c => {
            let pairs = cx.pairs?;
            if c == pairs.primary.0 || c == pairs.nested.0 {
                Some(Role::Open)
            } else if c == APOSTROPHE_CHAR && (c == pairs.primary.1 || c == pairs.nested.1) {
                // `’` is also the apostrophe: it closes only where the closing test of design
                // section 6.3 holds, or as the second quote of a sign-only pair.
                let (l, r) = neighbors(cx, i);
                (class(cx, r).ends()
                    && (!class(cx, l).opens() || typed_sign_only(cx, i, class(cx, l))))
                .then_some(Role::Close)
            } else if c == pairs.primary.1 || c == pairs.nested.1 {
                Some(Role::Close)
            } else {
                None
            }
        }
    }
}

/// No index.
const NONE: u32 = u32::MAX;

/// Families (design section 6.4): straight `"`, straight `'`, and the two typed pairs.
const D: u8 = 0;
const S: u8 = 1;
const P: u8 = 2;
const N: u8 = 3;
const NO_FAMILY: u8 = 4;

/// What a quote is for pairing.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Cat {
    /// Takes no part: inside a frozen core, a technical wrapper, a prime, an apostrophe.
    Skip,
    /// A straight quote that is a candidate (with or without a role).
    Straight,
    /// A typed glyph of the language's pairs.
    Typed,
    /// U+2019 as the closer of a pair: it closes only an open partner.
    Cond,
    /// A typed glyph of no pair of the language.
    Neutral,
}

/// One quote of the window.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Q {
    tok: u32,
    glyph: char,
    cat: Cat,
    role: Option<Role>,
    fam: u8,
    weak: bool,
    partner: u32,
    /// Paired in pass 1.
    first: bool,
    /// Resolved as a prime in pass 2.
    prime: bool,
    /// The edit of step 1, kept when the window does not converge.
    step1: Mark,
    /// The pair is cancelled by `SingleQuotes::NestedOnly`.
    cancelled: bool,
}

/// One entry of the matching stack.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Entry {
    q: u32,
    fam: u8,
    /// The next lower entry of the same family.
    below: u32,
    /// The highest entry at or below this one that is not of family `S`.
    non_s: u32,
}

/// Working memory of the quote phase.
#[derive(Debug, Default)]
pub(crate) struct Work {
    qs: Vec<Q>,
    stack: Vec<Entry>,
    /// Per quote, for the trailing possessive (design section 6.6): the number of straight `'`
    /// openers before it that take part and stayed unpaired, and the number of quotes before it
    /// that belong to a pair that is not cancelled.
    before: Vec<[u32; 2]>,
}

impl Work {
    pub(crate) const fn new() -> Work {
        Work {
            qs: Vec::new(),
            stack: Vec::new(),
            before: Vec::new(),
        }
    }

    pub(crate) fn heap_bytes(&self) -> usize {
        self.qs.capacity() * size_of::<Q>()
            + self.stack.capacity() * size_of::<Entry>()
            + self.before.capacity() * size_of::<[u32; 2]>()
    }
}

/// The matching stack with a pointer to the topmost entry of every family, so finding the nearest
/// entry of a family is constant time (design section 6.4).
struct Stack<'a> {
    entries: &'a mut Vec<Entry>,
    top: [u32; 4],
}

impl<'a> Stack<'a> {
    fn new(entries: &'a mut Vec<Entry>) -> Stack<'a> {
        entries.clear();
        Stack {
            entries,
            top: [NONE; 4],
        }
    }

    fn push(&mut self, q: u32, fam: u8) {
        let Ok(index) = u32::try_from(self.entries.len()) else {
            return;
        };
        let below = self.top.get(fam as usize).copied().unwrap_or(NONE);
        let non_s = if fam == S {
            self.entries.last().map_or(NONE, |e| e.non_s)
        } else {
            index
        };
        self.entries.push(Entry {
            q,
            fam,
            below,
            non_s,
        });
        if let Some(t) = self.top.get_mut(fam as usize) {
            *t = index;
        }
    }

    fn pop(&mut self) -> Option<Entry> {
        let e = self.entries.pop()?;
        if let Some(t) = self.top.get_mut(e.fam as usize) {
            *t = e.below;
        }
        Some(e)
    }

    fn top_of(&self, fam: u8) -> u32 {
        self.top.get(fam as usize).copied().unwrap_or(NONE)
    }

    fn last(&self) -> Option<&Entry> {
        self.entries.last()
    }
}

/// Whether a closer of `fam` finds its entry covered by nothing but straight `'` entries.
fn closable(stack: &Stack<'_>, fam: u8) -> bool {
    let p = stack.top_of(fam);
    p != NONE && stack.last().is_none_or(|e| e.non_s == NONE || e.non_s <= p)
}

/// The outcome of matching a closer against the stack.
enum Found {
    Pair(u32),
    Absent,
    Crossing(u32),
}

/// Pass-1 matching of a closer of family `fam` (design section 6.4): pairs with the nearest entry
/// of its family when only straight `'` entries cover it (they are popped unpaired).
fn close(stack: &mut Stack<'_>, fam: u8) -> Found {
    let p = stack.top_of(fam);
    if p == NONE {
        return Found::Absent;
    }
    let len = stack.entries.len();
    if p as usize + 1 != len {
        let h = stack.last().map_or(NONE, |e| e.non_s);
        if h != NONE && h > p {
            return Found::Crossing(stack.entries.get(h as usize).map_or(NONE, |e| e.q));
        }
        while stack.entries.len() > p as usize + 1 {
            stack.pop();
        }
    }
    match stack.pop() {
        Some(e) => Found::Pair(e.q),
        None => Found::Absent,
    }
}

/// The preliminary pass of the quote phase over the window: fills `marks` with one decision per
/// token and reports the diagnostics of `quotes.pairs` when `report` is set.
pub(crate) fn analyze<R: Recorder>(
    cx: &Cx<'_>,
    work: &mut Work,
    marks: &mut Vec<Mark>,
    sink: &mut Sink<'_, R>,
    report: bool,
) {
    marks.clear();
    marks.resize(cx.toks.len(), Mark::None);
    let Work { qs, stack, before } = work;
    qs.clear();
    for (i, t) in cx.toks.iter().enumerate() {
        if t.kind == Kind::Quote
            && let Ok(tok) = u32::try_from(i)
            && let Some(glyph) = cx.chr(i)
        {
            qs.push(Q {
                tok,
                glyph,
                cat: Cat::Skip,
                role: None,
                fam: NO_FAMILY,
                weak: false,
                partner: NONE,
                first: false,
                prime: false,
                step1: Mark::None,
                cancelled: false,
            });
        }
    }
    if qs.is_empty() {
        return;
    }
    let diag = |sink: &mut Sink<'_, R>, kind: DiagnosticKind, severity: Severity, t: &Tok| {
        if report {
            sink.diagnostic_of(kind, severity, Some(crate::Rule::QuotesPairs), loc(t));
        }
    };

    // Candidates and step 1 (design sections 6.1 and 6.2).
    let mut facts = None;
    for q in qs.iter_mut() {
        categorize(cx, q, &mut facts);
    }

    // Roles of series (design section 6.3) and weak quotes (design section 6.5).
    let mut k = 0;
    while k < qs.len() {
        let start = k;
        let (end, l, r) = series(cx, qs, start);
        k = end;
        let role = straight_role(cx, l, r);
        let lc = class(cx, l);
        let rc = class(cx, r);
        for (n, q) in qs
            .get_mut(start..k)
            .unwrap_or_default()
            .iter_mut()
            .enumerate()
        {
            match q.cat {
                Cat::Straight => {
                    q.role = role;
                    q.fam = if q.glyph == '"' { D } else { S };
                    q.weak = n == 0
                        && role == Some(Role::Close)
                        && cx
                            .prev(q.tok as usize)
                            .and_then(|p| cx.kind(p))
                            .is_some_and(|k| {
                                matches!(k, Kind::Number | Kind::Slot(crate::SlotKind::Number))
                            });
                }
                Cat::Cond => {
                    // U+2019 can close only where the neighbors allow a closing role, or as the
                    // second quote of a sign-only pair with the typed opener before it.
                    if !(!lc.opens() && rc.ends())
                        && !(rc.ends() && typed_sign_only(cx, q.tok as usize, lc))
                    {
                        q.cat = Cat::Skip;
                    }
                }
                _ => {}
            }
        }
    }
    // Sign-only pairs (design section 6.3), after every series has its role: the second quote of a
    // pair may stand in a later series, whose role must not overwrite the pair.
    let mut k = 0;
    while k < qs.len() {
        let start = k;
        let (end, l, _) = series(cx, qs, start);
        k = end;
        if class(cx, l).opens() {
            sign_only(cx, qs, start, end);
        }
    }

    // Straight quotes without a role: `AmbiguousQuote`, not candidates.
    for q in qs.iter_mut() {
        if q.cat == Cat::Straight && q.role.is_none() {
            if let Some(t) = cx.tok(q.tok as usize) {
                diag(sink, DiagnosticKind::AmbiguousQuote, Severity::Info, t);
            }
            q.fam = NO_FAMILY;
        }
    }
    let never = cx.single_quotes == SingleQuotes::Never;
    // Whether a candidate takes part in the stack: in `Never` mode straight `'` never pairs.
    let joins = |q: &Q| match q.cat {
        Cat::Straight => q.role.is_some() && !(never && q.fam == S),
        Cat::Typed | Cat::Cond => true,
        _ => false,
    };

    // Pass 1 (design sections 6.4 and 6.5): weak quotes are ignored.
    let mut converges = true;
    {
        let mut st = Stack::new(stack);
        for idx in 0..qs.len() {
            let Some(q) = qs.get(idx).copied() else {
                continue;
            };
            let Ok(index) = u32::try_from(idx) else {
                break;
            };
            if !joins(&q) || q.weak {
                continue;
            }
            let opener = match q.cat {
                Cat::Typed => q.role == Some(Role::Open),
                Cat::Cond => false,
                _ => q.role == Some(Role::Open),
            };
            if opener {
                st.push(index, q.fam);
                continue;
            }
            if q.cat == Cat::Cond && !closable(&st, q.fam) {
                // No open partner that it could close: an apostrophe, which blocks nothing.
                if let Some(q) = qs.get_mut(idx) {
                    q.cat = Cat::Skip;
                }
                continue;
            }
            match close(&mut st, q.fam) {
                Found::Pair(o) => {
                    if let Some(open) = qs.get_mut(o as usize) {
                        open.partner = index;
                        open.first = true;
                    }
                    if let Some(c) = qs.get_mut(idx) {
                        c.partner = o;
                        c.first = true;
                    }
                }
                Found::Crossing(cover) => {
                    if let Some(t) = qs.get(cover as usize).and_then(|c| cx.tok(c.tok as usize)) {
                        diag(sink, DiagnosticKind::CrossingQuotes, Severity::Warning, t);
                    }
                    converges = false;
                    break;
                }
                Found::Absent => match q.cat {
                    Cat::Straight if q.fam == D => {
                        if let Some(t) = cx.tok(q.tok as usize) {
                            diag(sink, DiagnosticKind::UnmatchedQuote, Severity::Warning, t);
                        }
                        converges = false;
                        break;
                    }
                    Cat::Straight => {}
                    _ => {
                        if let Some(t) = cx.tok(q.tok as usize) {
                            diag(sink, DiagnosticKind::UnmatchedQuote, Severity::Info, t);
                        }
                        if let Some(q) = qs.get_mut(idx) {
                            q.cat = Cat::Neutral;
                        }
                    }
                },
            }
        }
    }

    // Pass 2 (design section 6.5): weak quotes pair with an opener that pass 1 left unpaired on top
    // of the stack, or become primes.
    let mut neutral_apostrophe_opener = NONE;
    if converges {
        let mut st = Stack::new(stack);
        for idx in 0..qs.len() {
            let Some(q) = qs.get(idx).copied() else {
                continue;
            };
            let Ok(index) = u32::try_from(idx) else {
                break;
            };
            if !joins(&q) && !(q.weak && q.cat == Cat::Straight) {
                continue;
            }
            if q.cat != Cat::Cond && q.role == Some(Role::Open) && joins(&q) {
                st.push(index, q.fam);
            } else if q.first && q.role != Some(Role::Open) {
                // The closer of a pass-1 pair pops its opener and every entry above it.
                while let Some(e) = st.pop() {
                    if e.q == q.partner {
                        break;
                    }
                }
            } else if q.weak {
                let top = st.last().copied();
                let pairs_with = top.filter(|e| {
                    e.fam == q.fam
                        && joins(&q)
                        && qs.get(e.q as usize).is_some_and(|o| o.partner == NONE)
                });
                match pairs_with {
                    Some(e) => {
                        st.pop();
                        if let Some(open) = qs.get_mut(e.q as usize) {
                            open.partner = index;
                        }
                        if let Some(c) = qs.get_mut(idx) {
                            c.partner = e.q;
                        }
                    }
                    None => {
                        if let Some(c) = qs.get_mut(idx) {
                            c.prime = true;
                        }
                    }
                }
            }
        }
        // The end of the window (design section 6.4): a straight `"` left open does not converge; a
        // straight `'` stays unpaired; a typed opener is neutral.
        let mut first_d = None;
        for e in st.entries.iter() {
            let Some(q) = qs.get(e.q as usize).copied() else {
                continue;
            };
            match q.cat {
                Cat::Straight if q.fam == D => {
                    first_d = first_d.or(Some(q.tok));
                }
                Cat::Typed => {
                    if let Some(t) = cx.tok(q.tok as usize) {
                        diag(sink, DiagnosticKind::UnmatchedQuote, Severity::Info, t);
                    }
                    if cx.pairs.is_some_and(|p| {
                        (q.fam == P && p.primary.1 == APOSTROPHE_CHAR)
                            || (q.fam == N && p.nested.1 == APOSTROPHE_CHAR)
                    }) && neutral_apostrophe_opener == NONE
                    {
                        neutral_apostrophe_opener = q.tok;
                    }
                }
                _ => {}
            }
        }
        if let Some(tok) = first_d {
            if let Some(t) = cx.tok(tok as usize) {
                diag(sink, DiagnosticKind::UnmatchedQuote, Severity::Warning, t);
            }
            converges = false;
        }
    }

    // Step 1 is kept whatever the verdict (design section 6.6).
    for q in qs.iter() {
        if q.step1 != Mark::None
            && let Some(m) = marks.get_mut(q.tok as usize)
        {
            *m = q.step1;
        }
    }
    if !converges {
        return;
    }

    // Single quotes at the top level and levels (design sections 6.7 and 6.8).
    let mut enclosing = 0usize; // pairs of straight `"` and typed pairs
    let mut depth = 0usize; // every pair that counts for levels
    // New pairs open around the current quote whose glyphs are of the primary and of the nested
    // pair, and the first typed closer that stayed neutral inside one of its own family (design
    // section 6.6): a second pass would pair it with the new opener.
    let mut open_new = [0usize; 2];
    let mut repairs = None;
    for idx in 0..qs.len() {
        let Some(q) = qs.get(idx).copied() else {
            continue;
        };
        if q.cat == Cat::Neutral {
            if repairs.is_none()
                && q.role == Some(Role::Close)
                && family_index(q.fam).is_some_and(|f| open_new.get(f).is_some_and(|&n| n > 0))
            {
                repairs = Some(q.tok);
            }
            continue;
        }
        if q.partner == NONE || !matches!(q.cat, Cat::Straight | Cat::Typed | Cat::Cond) {
            continue;
        }
        let opens = (q.partner as usize) > idx;
        let straight_s = q.cat == Cat::Straight && q.fam == S;
        if opens {
            let cancelled =
                straight_s && cx.single_quotes == SingleQuotes::NestedOnly && enclosing == 0;
            let level = depth;
            if !straight_s {
                enclosing += 1;
            }
            if !cancelled {
                depth += 1;
            }
            if let Some(o) = qs.get_mut(idx) {
                o.cancelled = cancelled;
            }
            if let Some(c) = qs.get_mut(q.partner as usize) {
                c.cancelled = cancelled;
            }
            if !cancelled && q.cat == Cat::Straight {
                set_glyphs(cx, qs, idx, level, marks);
                if let Some(n) = level_family(cx, level).and_then(|f| open_new.get_mut(f)) {
                    *n += 1;
                }
            }
        } else {
            if !straight_s {
                enclosing = enclosing.saturating_sub(1);
            }
            if !q.cancelled {
                depth = depth.saturating_sub(1);
                // The level of the pair is the depth outside it.
                if q.cat == Cat::Straight
                    && let Some(n) = level_family(cx, depth).and_then(|f| open_new.get_mut(f))
                {
                    *n = n.saturating_sub(1);
                }
            }
        }
    }
    if let Some(tok) = repairs {
        // The window does not converge: no straight quote changes, step 1 stays.
        if let Some(t) = cx.tok(tok as usize) {
            diag(sink, DiagnosticKind::CrossingQuotes, Severity::Warning, t);
        }
        for q in qs.iter().filter(|q| q.cat == Cat::Straight) {
            if let Some(m) = marks.get_mut(q.tok as usize) {
                *m = q.step1;
            }
        }
        return;
    }

    // What lies between a trailing possessive and the closer of the pair around it (design section
    // 6.6): straight `'` openers that stayed unpaired, and quotes of pairs. Only a language with
    // the trailing possessive needs it.
    let possessives = cx.data.trailing_possessive;
    let mut open_s = 0u32;
    let mut paired = 0u32;
    before.clear();
    for q in qs.iter().filter(|_| possessives) {
        before.push([open_s, paired]);
        if in_pair(q) {
            paired = paired.saturating_add(1);
        } else if q.cat == Cat::Straight
            && q.fam == S
            && q.role == Some(Role::Open)
            && !never
            && !q.prime
        {
            open_s = open_s.saturating_add(1);
        }
    }

    // Primes, the trailing possessive, the decade apostrophe and an elision that waited for
    // pairing (design sections 6.2, 6.5 and 6.6). `stack` holds the openers of the pairs around
    // the current quote.
    let data = cx.data;
    stack.clear();
    for (idx, q) in qs.iter().enumerate() {
        let i = q.tok as usize;
        if in_pair(q) {
            if !possessives {
                continue;
            }
            if (q.partner as usize) > idx {
                if let Ok(index) = u32::try_from(idx) {
                    stack.push(Entry {
                        q: index,
                        fam: q.fam,
                        below: NONE,
                        non_s: NONE,
                    });
                }
            } else {
                while let Some(e) = stack.pop() {
                    if e.q == q.partner {
                        break;
                    }
                }
            }
            continue;
        }
        if q.prime {
            if let Some(m) = marks.get_mut(i) {
                *m = Mark::Prime(if q.glyph == '"' { '″' } else { '′' });
            }
            continue;
        }
        if q.cat != Cat::Straight || q.glyph != '\'' || q.partner != NONE {
            continue;
        }
        let prev = cx.prev(i);
        let next = cx.next(i);
        let possessive = data.trailing_possessive
            && prev.is_some_and(|p| {
                matches!(cx.kind(p), Some(Kind::Word(_))) && cx.src(p).ends_with(['s', 'S'])
            })
            && !next.is_some_and(|n| matches!(cx.kind(n), Some(Kind::Word(_))))
            && (neutral_apostrophe_opener == NONE || neutral_apostrophe_opener > q.tok)
            && !possessive_closes(cx, qs, before, marks, idx, stack.last().map(|e| e.q));
        // A number run split by `Format` or `Markup` tokens is one number (design section 3.6).
        let decade = data.decade_apostrophe
            && word_start(cx, i)
            && next.is_some_and(|n| {
                cx.kind(n) == Some(Kind::Number) && cx.digits_len(n, cx.number_end(n)) == 2
            })
            && !next
                .and_then(|n| cx.next(cx.number_end(n)))
                .is_some_and(|a| matches!(cx.kind(a), Some(Kind::Word(_) | Kind::Number)));
        // An elision that a straight `'` closed waited for pairing (design section 6.2).
        let elided = elision(cx, i) == Some(true);
        if (possessive || decade || elided)
            && let Some(m) = marks.get_mut(i)
        {
            *m = Mark::Apostrophe;
        }
    }
}

/// Whether the quote belongs to a pair that is not cancelled (design sections 6.4 to 6.8).
fn in_pair(q: &Q) -> bool {
    q.partner != NONE && !q.cancelled && matches!(q.cat, Cat::Straight | Cat::Typed | Cat::Cond)
}

/// Whether a trailing possessive at `qs[idx]` would be read as the closer of the pair around it,
/// whose opener is `qs[around]`, and so change what lies between them (design section 6.6).
/// `before` holds the counts of `Work::before`.
///
/// The pair is closed by `’` and the neighbors of `q` allow a closing role (design section 6.3):
/// on a second pass the possessive `’` would close the pair at once, and `to_plain` reads it as
/// the closer when the true closer follows `s` or `S` as well (design section 12.1). That
/// changes nothing when only text lies between them. It changes the result when a straight `'`
/// opener that stayed unpaired lies between, which the closer of the pair popped and which could
/// pair once the pair has closed early; or, for `to_plain`, when the closer follows `s` or `S`
/// and a quote of a pair lies between, whose level would drop by one.
fn possessive_closes(
    cx: &Cx<'_>,
    qs: &[Q],
    before: &[[u32; 2]],
    marks: &[Mark],
    idx: usize,
    around: Option<u32>,
) -> bool {
    let Some(c) = around
        .and_then(|o| qs.get(o as usize))
        .map(|o| o.partner as usize)
    else {
        return false;
    };
    let (Some(q), Some(close), Some(&[open_q, paired_q]), Some(&[open_c, paired_c])) =
        (qs.get(idx), qs.get(c), before.get(idx), before.get(c))
    else {
        return false;
    };
    let glyph = match marks.get(close.tok as usize) {
        Some(Mark::Glyph(g)) => *g,
        _ => close.glyph,
    };
    if glyph != APOSTROPHE_CHAR {
        return false;
    }
    let (l, r) = neighbors(cx, q.tok as usize);
    if class(cx, l).opens() || !class(cx, r).ends() {
        return false;
    }
    if open_c > open_q {
        return true;
    }
    let after_s = cx.prev(close.tok as usize).is_some_and(|p| {
        matches!(cx.kind(p), Some(Kind::Word(_))) && cx.src(p).ends_with(['s', 'S'])
    });
    after_s && paired_c > paired_q
}

/// The series of quotes that starts at `qs[start]`: the index after its last quote, and its L and
/// R (design section 6.3).
fn series(cx: &Cx<'_>, qs: &[Q], start: usize) -> (usize, Option<usize>, Option<usize>) {
    let first_tok = qs.get(start).map_or(0, |q| q.tok as usize);
    let mut last_tok = first_tok;
    let mut k = start + 1;
    while let Some(q) = qs.get(k) {
        if cx.next(last_tok) == Some(q.tok as usize) {
            last_tok = q.tok as usize;
            k += 1;
        } else {
            break;
        }
    }
    (k, cx.prev(first_tok), cx.next(last_tok))
}

/// The index of a typed family in the counters of new pairs: 0 for `P`, 1 for `N`.
fn family_index(fam: u8) -> Option<usize> {
    match fam {
        P => Some(0),
        N => Some(1),
        _ => None,
    }
}

/// The family of the glyphs of a new pair at `level`, as `categorize` reads them on a second
/// pass, as an index of `family_index`.
fn level_family(cx: &Cx<'_>, level: usize) -> Option<usize> {
    let pairs = cx.pairs?;
    let open = if level.is_multiple_of(2) {
        pairs.primary.0
    } else {
        pairs.nested.0
    };
    if open == pairs.primary.0 {
        Some(0)
    } else if open == pairs.nested.0 {
        Some(1)
    } else {
        None
    }
}

/// Writes the glyphs of the pair that opens at `qs[idx]` at `level`.
fn set_glyphs(cx: &Cx<'_>, qs: &[Q], idx: usize, level: usize, marks: &mut [Mark]) {
    let Some(pairs) = cx.pairs else {
        return;
    };
    let (open, close) = if level.is_multiple_of(2) {
        pairs.primary
    } else {
        pairs.nested
    };
    let Some(q) = qs.get(idx) else {
        return;
    };
    if let Some(m) = marks.get_mut(q.tok as usize) {
        *m = Mark::Glyph(open);
    }
    if let Some(m) = qs
        .get(q.partner as usize)
        .and_then(|c| marks.get_mut(c.tok as usize))
    {
        *m = Mark::Glyph(close);
    }
}

/// Whether the quote at `i` stands at the start of a word.
fn word_start(cx: &Cx<'_>, i: usize) -> bool {
    match cx.prev(i).and_then(|p| cx.kind(p)) {
        None => true,
        Some(k) => matches!(
            k,
            Kind::Space | Kind::Newline | Kind::Open(_) | Kind::Dash | Kind::Quote
        ),
    }
}

/// Sets what a quote is for pairing, and the decisions of step 1 (design sections 6.1 and 6.2).
fn categorize(cx: &Cx<'_>, q: &mut Q, facts: &mut Option<WrapperFacts>) {
    let i = q.tok as usize;
    let Some(t) = cx.tok(i) else {
        return;
    };
    if t.is_frozen() {
        return;
    }
    let straight = matches!(q.glyph, '"' | '\'');
    if matches!(q.glyph, '′' | '″') {
        return;
    }
    if straight && !wrapper_candidate(cx, i, facts) {
        return;
    }
    if matches!(q.glyph, '\'' | APOSTROPHE_CHAR)
        && let Some(mark) = step1(cx, i, q.glyph)
    {
        q.step1 = if q.glyph == '\'' { mark } else { Mark::None };
        return;
    }
    if straight {
        q.cat = Cat::Straight;
        return;
    }
    let Some(pairs) = cx.pairs else {
        // No pairs: every typed glyph is neutral, U+2019 an apostrophe.
        q.cat = if q.glyph == APOSTROPHE_CHAR {
            Cat::Skip
        } else {
            Cat::Neutral
        };
        return;
    };
    let (fam, role) = if q.glyph == pairs.primary.0 {
        (P, Role::Open)
    } else if q.glyph == pairs.primary.1 {
        (P, Role::Close)
    } else if q.glyph == pairs.nested.0 {
        (N, Role::Open)
    } else if q.glyph == pairs.nested.1 {
        (N, Role::Close)
    } else {
        q.cat = if q.glyph == APOSTROPHE_CHAR {
            Cat::Skip
        } else {
            Cat::Neutral
        };
        return;
    };
    q.fam = fam;
    q.role = Some(role);
    q.cat = if q.glyph == APOSTROPHE_CHAR && role == Role::Close {
        Cat::Cond
    } else {
        Cat::Typed
    };
}

/// What `wrapper_candidate` reads of one technical fragment, computed once per fragment.
#[derive(Clone, Copy, Debug)]
pub(crate) struct WrapperFacts {
    frag: u32,
    /// The core holds a straight quote.
    core_has: bool,
    /// The leading wrapper holds a straight quote.
    start_has: bool,
    /// The core ends in `=`.
    ends_in_eq: bool,
}

/// Whether a straight quote in the wrapper of a frozen fragment is a candidate (design section
/// 6.1).
///
/// The quotes are visited in token order and the tokens of a fragment are contiguous, so `facts`
/// holds the facts of the last fragment met and each fragment is scanned once (design section
/// 3.12).
fn wrapper_candidate(cx: &Cx<'_>, i: usize, facts: &mut Option<WrapperFacts>) -> bool {
    let Some(frag) = cx.frag_of(i) else {
        return true;
    };
    let technical = frag.kind.tech().is_some() && frag.kind != super::FragKind::Compound;
    if !technical || cx.in_core(i) {
        return true;
    }
    let index = cx.tok(i).map_or(u32::MAX, |t| t.frag);
    let f = match *facts {
        Some(f) if f.frag == index => f,
        _ => {
            let straight = |j: usize| is_quote(cx, j) && matches!(cx.chr(j), Some('"' | '\''));
            let f = WrapperFacts {
                frag: index,
                // An apostrophe between two letters is not a quote of the core (design section
                // 6.1): `"it's:fine"` keeps its wrapper quotes as candidates.
                core_has: (frag.core_start as usize..frag.core_end as usize).any(|j| {
                    straight(j)
                        && !(cx.chr(j) == Some('\'')
                            && cx
                                .prev(j)
                                .is_some_and(|p| matches!(cx.kind(p), Some(Kind::Word(_))))
                            && cx
                                .next(j)
                                .is_some_and(|n| matches!(cx.kind(n), Some(Kind::Word(_)))))
                }),
                start_has: (frag.start as usize..frag.core_start as usize).any(straight),
                ends_in_eq: frag.core_end > frag.core_start
                    && cx.src(frag.core_end as usize - 1).ends_with('='),
            };
            *facts = Some(f);
            f
        }
    };
    if f.core_has {
        return false;
    }
    !(f.ends_in_eq && !f.start_has && i >= frag.core_end as usize)
}

/// Step 1 (design section 6.2): a certain apostrophe or prime.
fn step1(cx: &Cx<'_>, i: usize, glyph: char) -> Option<Mark> {
    let prev = cx.prev(i);
    let next = cx.next(i);
    let pk = prev.and_then(|p| cx.kind(p));
    let nk = next.and_then(|n| cx.kind(n));
    let letter_right = matches!(nk, Some(Kind::Word(_)));
    match pk {
        Some(Kind::Word(_)) if letter_right => return Some(Mark::Apostrophe),
        Some(Kind::Slot(crate::SlotKind::Word | crate::SlotKind::Number)) if letter_right => {
            return Some(Mark::Apostrophe);
        }
        Some(Kind::Number) if letter_right => return Some(Mark::Apostrophe),
        _ => {}
    }
    // A prime between a number of one to three digits and one or two digits ending the core; a
    // number run split by `Format` or `Markup` tokens is one number (design section 3.6).
    if glyph == '\''
        && let (Some(p), Some(n)) = (prev, next)
        && pk == Some(Kind::Number)
        && nk == Some(Kind::Number)
        && (1..=3).contains(&cx.digits_len(cx.number_start(p), p))
        && (1..=2).contains(&cx.digits_len(n, cx.number_end(n)))
        && cx.in_core(n)
        && cx
            .frag_of(n)
            .and_then(|f| cx.core(f).last())
            .is_some_and(|last| last == cx.number_end(n))
    {
        return Some(Mark::Prime('′'));
    }
    if !word_start(cx, i) {
        return None;
    }
    let data = cx.data;
    let n = next?;
    // An elision of the language's list (`'tis`); one closed at once by a straight `'` waits for
    // pairing (design section 6.2).
    if elision(cx, i) == Some(false) {
        return Some(Mark::Apostrophe);
    }
    // A decade: two digits, then `s` (`'90s`).
    if data.decade_apostrophe
        && nk == Some(Kind::Number)
        && cx.digits_len(n, cx.number_end(n)) == 2
        && let Some(s) = cx.next(cx.number_end(n))
        && cx.src(s) == "s"
        && matches!(cx.kind(s), Some(Kind::Word(_)))
        && !cx
            .next(s)
            .is_some_and(|a| matches!(cx.kind(a), Some(Kind::Word(_) | Kind::Number)))
    {
        return Some(Mark::Apostrophe);
    }
    None
}

/// Whether the quote `i` at a word start begins an elision of the language's list (`'tis`), and
/// if so whether a straight `'` closes the word at once (`'round'`): such an elision may open a
/// quoted word, so it waits for pairing, as `'n'` does, and is an apostrophe only unpaired
/// (design sections 6.2 and 6.6). `None` when it is no elision.
fn elision(cx: &Cx<'_>, i: usize) -> Option<bool> {
    let data = cx.data;
    if data.elisions.is_empty() || !word_start(cx, i) {
        return None;
    }
    let n = cx.next(i)?;
    if !matches!(cx.kind(n), Some(Kind::Word(_))) {
        return None;
    }
    // The word runs on across `Format` and `Markup` tokens (design section 3.6).
    let mut last = n;
    while let Some(j) = cx.next(last)
        && j > last + 1
        && matches!(cx.kind(j), Some(Kind::Word(_)))
    {
        last = j;
    }
    let key = core::iter::once('\'').chain(cx.visible(n, last).map(fold_char));
    data.elisions
        .binary_search_by(|e| e.chars().cmp(key.clone()))
        .ok()?;
    Some(cx.next(last).is_some_and(|c| {
        is_quote(cx, c)
            && cx.chr(c) == Some('\'')
            && !cx
                .next(c)
                .is_some_and(|a| matches!(cx.kind(a), Some(Kind::Word(_) | Kind::Number)))
    }))
}

/// Sign-only pairs (design section 6.3) whose first quote is in the series `qs[start..end]`, which
/// has `L` of class `Space`, `OpenP` or `DashC`: the first opens and the second closes.
fn sign_only(cx: &Cx<'_>, qs: &mut [Q], start: usize, end: usize) {
    // Every quote of the series counts for the rule of three, whatever its form: the count is the
    // same on a second pass, where some of the quotes have become glyphs (design section 6.11).
    let count = end - start;
    for a in start..end {
        let Some(q1) = qs.get(a).copied() else {
            continue;
        };
        if q1.cat != Cat::Straight {
            continue;
        }
        let Some(k) = sign_only_partner(cx, q1.tok as usize, || count) else {
            continue;
        };
        let Ok(b) = qs.binary_search_by_key(&k, |q| q.tok as usize) else {
            continue;
        };
        if qs.get(b).is_none_or(|q2| q2.cat != Cat::Straight) {
            continue;
        }
        let fam = if q1.glyph == '"' { D } else { S };
        if let Some(q) = qs.get_mut(a) {
            q.role = Some(Role::Open);
            q.fam = fam;
            q.weak = false;
        }
        if let Some(q) = qs.get_mut(b) {
            q.role = Some(Role::Close);
            q.fam = fam;
            q.weak = false;
        }
    }
}

/// Whether the token at `k` is a sign between the quotes of a sign-only pair (design section 6.3).
fn is_sign(cx: &Cx<'_>, k: usize) -> bool {
    matches!(
        cx.kind(k),
        Some(
            Kind::Punct(_)
                | Kind::Dots
                | Kind::Dash
                | Kind::Operator(_)
                | Kind::Open(_)
                | Kind::Close(_)
                | Kind::Times
        )
    ) && !cx.tok(k).is_some_and(Tok::is_frozen)
}

/// The second quote of a sign-only pair (design section 6.3) whose first quote is the straight
/// quote at `i`: the next token that is not a sign, in the same fragment, is a straight quote of
/// the same character whose series has R of class `Space`, `CloseP` or `DashC`. Two quotes of one
/// series are no pair when the series holds three or more quotes of any form; `count` gives that
/// number for the series of `i`. L of the first quote is the caller's to check.
fn sign_only_partner(cx: &Cx<'_>, i: usize, count: impl FnOnce() -> usize) -> Option<usize> {
    let glyph = cx.chr(i).filter(|c| matches!(c, '"' | '\''))?;
    let frag = cx.tok(i)?.frag;
    let adjacent = cx.next(i);
    let mut j = adjacent;
    while let Some(k) = j {
        if cx.tok(k).map(|t| t.frag) != Some(frag) {
            return None;
        }
        if !is_sign(cx, k) {
            break;
        }
        j = cx.next(k);
    }
    let k = j?;
    if !is_quote(cx, k) || cx.chr(k) != Some(glyph) {
        return None;
    }
    if j == adjacent && count() >= 3 {
        return None;
    }
    let (_, r2) = neighbors(cx, k);
    class(cx, r2).ends().then_some(k)
}

/// The first quote of a sign-only pair whose second quote is the straight quote at `i`, with L of
/// its series of class `Space`, `OpenP` or `DashC`: the mirror of [`sign_only_partner`].
fn sign_only_opener(cx: &Cx<'_>, i: usize) -> Option<usize> {
    let glyph = cx.chr(i)?;
    let frag = cx.tok(i)?.frag;
    let mut j = cx.prev(i);
    while let Some(k) = j {
        if cx.tok(k).map(|t| t.frag) != Some(frag) {
            return None;
        }
        if !is_sign(cx, k) {
            break;
        }
        j = cx.prev(k);
    }
    let a = j?;
    if !is_quote(cx, a) || cx.chr(a) != Some(glyph) {
        return None;
    }
    let (l, _) = neighbors(cx, a);
    (class(cx, l).opens() && sign_only_partner(cx, a, || series_count(cx, a)) == Some(i))
        .then_some(a)
}

/// Whether the `’` at `i` is the second quote of a sign-only pair whose first quote is a typed
/// opener of the pair that `’` closes (design section 6.3): the next token before it that is not a
/// sign, in the same fragment, is that opener, whose series has L of class `Space`, `OpenP` or
/// `DashC`, and two adjacent quotes are no pair in a series of three or more. It is the typed form
/// of the sign-only pair of straight quotes, which a first pass sets as such a pair (`""` → `‘’`),
/// so a second pass reads it as the first did. R of the `’` is the caller's to check, and `left`
/// is the class of L of its series.
fn typed_sign_only(cx: &Cx<'_>, i: usize, left: Class) -> bool {
    let Some(pairs) = cx.pairs else {
        return false;
    };
    // The primary pair comes first, as in `categorize`.
    let open = if pairs.primary.1 == APOSTROPHE_CHAR {
        pairs.primary.0
    } else if pairs.nested.1 == APOSTROPHE_CHAR {
        pairs.nested.0
    } else {
        return false;
    };
    let Some(frag) = cx.tok(i).map(|t| t.frag) else {
        return false;
    };
    let adjacent = cx.prev(i);
    let mut j = adjacent;
    while let Some(k) = j {
        if cx.tok(k).map(|t| t.frag) != Some(frag) {
            return false;
        }
        if !is_sign(cx, k) {
            break;
        }
        j = cx.prev(k);
    }
    let Some(a) = j else {
        return false;
    };
    if !is_quote(cx, a) || cx.chr(a) != Some(open) || cx.tok(a).is_none_or(Tok::is_frozen) {
        return false;
    }
    if j == adjacent {
        // The opener is in the series of the `’`, whose L the caller has.
        return left.opens() && !at_least_three(cx, a);
    }
    let (l, _) = neighbors(cx, a);
    class(cx, l).opens()
}

/// Whether the series that holds the quote at `i` has three or more quotes, of any form. It looks
/// at no more than two quotes on each side, so a call costs the same however long the series is
/// (design section 3.12).
fn at_least_three(cx: &Cx<'_>, i: usize) -> bool {
    let mut count = 1;
    let mut l = cx.prev(i);
    while count < 3
        && let Some(j) = l.filter(|&j| is_quote(cx, j))
    {
        count += 1;
        l = cx.prev(j);
    }
    let mut r = cx.next(i);
    while count < 3
        && let Some(j) = r.filter(|&j| is_quote(cx, j))
    {
        count += 1;
        r = cx.next(j);
    }
    count >= 3
}

/// The number of quotes, of any form, in the series that holds the quote at `i`.
fn series_count(cx: &Cx<'_>, i: usize) -> usize {
    let mut count = 1;
    let mut l = cx.prev(i);
    while let Some(j) = l.filter(|&j| is_quote(cx, j)) {
        count += 1;
        l = cx.prev(j);
    }
    let mut r = cx.next(i);
    while let Some(j) = r.filter(|&j| is_quote(cx, j)) {
        count += 1;
        r = cx.next(j);
    }
    count
}
