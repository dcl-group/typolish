//! The phases (design section 3.8) and freezing with edit refusal (design section 3.7).
//!
//! A rule gets no mutable access to tokens: it reads the input of its phase through `Cx` and
//! returns a proposal, and the engine applies it only if no affected token is `FROZEN` and no
//! insertion falls inside a frozen fragment. The check sits here, in one place, so protection does
//! not depend on the care of each rule. Every phase is one pass over a snapshot: its rules read the
//! input tokens of the phase and the classification of the pass, and write a new token vector.

use super::classify::{Frag, FragKind};
use super::lex::{EllipsisForm, Flags, Glue, Kind, NO_FRAG, NO_PIECE, Repl, Tok};
use super::record::{Loc, Recorder, Sink};
use crate::lang::data::{DashData, QuotePairs, Table};
use crate::rule::{ORDER, Phase};
use crate::rules::{ANCHORS, RULES};
use crate::{DiagnosticKind, Rule, Severity, SingleQuotes, Typographer};

/// The most edits one proposal holds; `nbsp.phone` glues up to five spaces, and an entry of
/// `nbsp.abbreviations` that needs more makes no edit (design section 8.7).
pub(crate) const MAX_EDITS: usize = 6;

/// A proposal of a rule at an anchor token.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Match {
    /// The tokens `anchor..=anchor + consume` are taken: closed to the other rules of the phase.
    pub(crate) consume: u8,
    /// At most `MAX_EDITS` edits inside `anchor..=anchor + consume`.
    pub(crate) edits: Edits,
    /// A refusal the rule reports (`AmbiguousHyphen`) at the token with this offset; the match then
    /// has no edits.
    pub(crate) note: Option<u8>,
}

impl Match {
    /// A match with no edits: the pattern is already in its target form (design section 3.8,
    /// item 3).
    pub(crate) const fn done(consume: u8) -> Match {
        Match {
            consume,
            edits: Edits::NONE,
            note: None,
        }
    }

    /// A match with edits.
    pub(crate) const fn with(consume: u8, edits: Edits) -> Match {
        Match {
            consume,
            edits,
            note: None,
        }
    }

    /// A refusal reported as `AmbiguousHyphen` at the token with offset `at`.
    pub(crate) const fn refused(consume: u8, at: u8) -> Match {
        Match {
            consume,
            edits: Edits::NONE,
            note: Some(at),
        }
    }
}

/// One edit, by token offset from the anchor.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Edit {
    /// New text of a token; the kind stays.
    Replace { at: u8, repl: Repl },
    /// Set the glue mark of a `Space` token.
    Glue { at: u8, glue: Glue },
    /// Insert a `Space` token carrying glue before the token at this offset.
    Insert { before: u8, glue: Glue },
    /// Remove a `Space` token.
    Remove { at: u8 },
}

/// The edits of a proposal, without allocation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Edits {
    len: u8,
    items: [Edit; MAX_EDITS],
}

impl Edits {
    /// No edit: a pattern already in its target form (design section 3.8, item 3).
    pub(crate) const NONE: Edits = Edits {
        len: 0,
        items: [Edit::Remove { at: 0 }; MAX_EDITS],
    };

    /// One edit.
    pub(crate) const fn one(edit: Edit) -> Edits {
        let mut items = Edits::NONE.items;
        let [first, ..] = &mut items;
        *first = edit;
        Edits { len: 1, items }
    }

    /// The edits with one more, or `None` when they already hold `MAX_EDITS`: a rule whose
    /// proposal would need more then proposes none of its edits, so it never applies part of a
    /// pattern that a second pass would complete (design section 4.4).
    pub(crate) fn try_with(mut self, edit: Edit) -> Option<Edits> {
        let slot = self.items.get_mut(self.len as usize)?;
        *slot = edit;
        self.len += 1;
        Some(self)
    }

    /// The edits in order.
    pub(crate) fn iter(&self) -> impl Iterator<Item = &Edit> {
        self.items.iter().take(self.len as usize)
    }
}

/// Whether the engine applies a proposal at `anchor` (design section 3.7): every affected token
/// exists, lies within the taken range and is not `FROZEN`, and no insertion stands next to a
/// frozen token of the call's text (the inside of a frozen fragment, the hyphen of a compound
/// word or a frozen space). An insertion next to a slot or protected text, which are frozen but
/// opaque, is allowed: it falls outside them. Glue and removal apply only to `Space` tokens, and
/// no edit replaces a `Format` or `Markup` token (invariant I1).
pub(crate) fn edits_allowed(tokens: &[Tok], anchor: usize, m: &Match) -> bool {
    let last = anchor + m.consume as usize;
    let editable = |at: u8| {
        let i = anchor + at as usize;
        i <= last && tokens.get(i).is_some_and(|t| !t.is_frozen())
    };
    let open_side = |i: Option<usize>| match i.and_then(|i| tokens.get(i)) {
        Some(t) => !t.is_frozen() || t.kind.is_opaque(),
        None => true,
    };
    if tokens.get(anchor).is_none_or(Tok::is_frozen) {
        return false;
    }
    m.edits.iter().all(|edit| match *edit {
        // Glue and removal apply only to `Space` tokens, and no edit touches markup or a format
        // character (invariant I1 of design section 4.3, design section 3.4).
        Edit::Glue { at, .. } | Edit::Remove { at } => {
            editable(at)
                && tokens
                    .get(anchor + at as usize)
                    .is_some_and(|t| t.kind == Kind::Space)
        }
        Edit::Replace { at, .. } => {
            editable(at)
                && tokens
                    .get(anchor + at as usize)
                    .is_some_and(|t| !t.kind.is_transparent())
        }
        Edit::Insert { before, .. } => {
            let right = anchor + before as usize;
            right <= last + 1
                && open_side((anchor + before as usize).checked_sub(1))
                && open_side(Some(right))
        }
    })
}

/// Read-only access of rules to the input of a phase (design section 3.8): the tokens with their
/// current text, neighbors without `Format` and `Markup` tokens, the fragments of the pass with
/// their kinds and flags, the language data and the settings.
pub(crate) struct Cx<'a> {
    pub(crate) t: &'a Typographer,
    pub(crate) data: Table<'a>,
    /// The call's text (design section 3.4).
    pub(crate) text: &'a str,
    pub(crate) toks: &'a [Tok],
    pub(crate) frags: &'a [Frag],
    /// The decisions of the quote phase, one per token (design section 6); empty in other phases.
    pub(crate) quotes: &'a [crate::rules::quotes::Mark],
    /// The text of quote tokens as a second pass of the quote phase reads it, by token index in
    /// order (design section 6.6); empty outside that pass.
    pub(crate) overlay: &'a [(u32, char)],
    /// The place in `overlay` of the last quote read, where the next read usually is.
    pub(crate) hint: core::cell::Cell<usize>,
    pub(crate) pairs: Option<QuotePairs>,
    pub(crate) single_quotes: SingleQuotes,
    pub(crate) dash: Option<DashData>,
    /// The last space whose sides `rules::nbsp` computed, with the result: the rules of a phase
    /// read one input, so it holds for the whole phase.
    pub(crate) sides: core::cell::Cell<Option<(usize, Sides)>>,
}

/// The tokens on both sides of a pattern space, if the rules of design section 8.2 may glue it.
pub(crate) type Sides = Option<(usize, usize)>;

impl<'a> Cx<'a> {
    pub(crate) fn tok(&self, i: usize) -> Option<&'a Tok> {
        self.toks.get(i)
    }

    /// The source text of a token: the slice of the call's text at its range.
    pub(crate) fn src(&self, i: usize) -> &'a str {
        self.toks
            .get(i)
            .and_then(|t| self.text.get(t.src.start as usize..t.src.end as usize))
            .unwrap_or("")
    }

    /// The first character of the current text of a token.
    pub(crate) fn chr(&self, i: usize) -> Option<char> {
        let t = self.toks.get(i)?;
        if !self.overlay.is_empty() && t.kind == Kind::Quote {
            // The quotes are read mostly in order: the place of the last read, or one next to it.
            let h = self.hint.get();
            let near = [h, h.saturating_add(1), h.saturating_sub(1)]
                .into_iter()
                .find(|&k| self.overlay.get(k).is_some_and(|&(at, _)| at as usize == i));
            let found = near.or_else(|| {
                self.overlay
                    .binary_search_by_key(&i, |&(at, _)| at as usize)
                    .ok()
            });
            if let Some(k) = found {
                self.hint.set(k);
                return self.overlay.get(k).map(|&(_, c)| c);
            }
        }
        match t.repl {
            Repl::Source => self.src(i).chars().next(),
            Repl::Char(c) => Some(c),
            Repl::Ellipsis(form) => ellipsis_tail(&self.data, form).chars().next(),
        }
    }

    /// Whether the current text of a token is exactly `s`.
    pub(crate) fn is(&self, i: usize, s: &str) -> bool {
        match self.toks.get(i).map(|t| t.repl) {
            Some(Repl::Source) => self.src(i) == s,
            Some(Repl::Char(c)) => {
                let mut chars = s.chars();
                chars.next() == Some(c) && chars.next().is_none()
            }
            Some(Repl::Ellipsis(form)) => ellipsis_tail(&self.data, form) == s,
            None => false,
        }
    }

    /// The token before `i`, `Format` and `Markup` tokens skipped.
    pub(crate) fn prev(&self, i: usize) -> Option<usize> {
        let mut j = i;
        while j > 0 {
            j -= 1;
            if !self.toks.get(j)?.kind.is_transparent() {
                return Some(j);
            }
        }
        None
    }

    /// The token after `i`, `Format` and `Markup` tokens skipped.
    pub(crate) fn next(&self, i: usize) -> Option<usize> {
        let mut j = i + 1;
        while let Some(t) = self.toks.get(j) {
            if !t.kind.is_transparent() {
                return Some(j);
            }
            j += 1;
        }
        None
    }

    /// The kind of a token, if it exists.
    pub(crate) fn kind(&self, i: usize) -> Option<Kind> {
        self.toks.get(i).map(|t| t.kind)
    }

    /// The fragment of a token.
    pub(crate) fn frag_of(&self, i: usize) -> Option<&'a Frag> {
        let f = self.toks.get(i)?.frag;
        if f == NO_FRAG {
            return None;
        }
        self.frags.get(f as usize)
    }

    /// Whether a token belongs to the core of its fragment.
    pub(crate) fn in_core(&self, i: usize) -> bool {
        self.toks
            .get(i)
            .is_some_and(|t| t.flags.contains(Flags::CORE))
    }

    /// The indices of the tokens of a core, `Format` and `Markup` tokens skipped.
    pub(crate) fn core(&self, frag: &Frag) -> impl Iterator<Item = usize> + '_ {
        (frag.core_start as usize..frag.core_end as usize)
            .filter(|&i| self.toks.get(i).is_some_and(|t| !t.kind.is_transparent()))
    }

    /// Whether `b` is the token after `a`, `Format` and `Markup` tokens skipped, and both are
    /// `Number` tokens of one fragment: one number run.
    fn run_continues(&self, a: usize, b: usize) -> bool {
        match (self.toks.get(a), self.toks.get(b)) {
            (Some(x), Some(y)) => {
                x.kind == Kind::Number
                    && y.kind == Kind::Number
                    && x.frag != NO_FRAG
                    && x.frag == y.frag
                    && self.next(a) == Some(b)
            }
            _ => false,
        }
    }

    /// The last token of the number run of a `Number` token: the `Number` tokens of its fragment
    /// with only `Format` and `Markup` tokens between them. For any other token, the token itself.
    /// A `Number` run split by `Format` or `Markup` tokens reads as one number (design section
    /// 3.6).
    pub(crate) fn number_end(&self, j: usize) -> usize {
        let mut last = j;
        while let Some(n) = self.next(last)
            && self.run_continues(last, n)
        {
            last = n;
        }
        last
    }

    /// The first token of the number run of a `Number` token; for any other token, the token
    /// itself. A `Number` run split by `Format` or `Markup` tokens reads as one number (design
    /// section 3.6).
    pub(crate) fn number_start(&self, j: usize) -> usize {
        let mut first = j;
        while let Some(p) = self.prev(first)
            && self.run_continues(p, first)
        {
            first = p;
        }
        first
    }

    /// Tokens in order, `Format` and `Markup` tokens skipped, as units of the first and the last
    /// token: a number run is one unit, any other token a unit of its own; at most `N` units,
    /// `None` when there are more. A `Number` run split by `Format` or `Markup` tokens reads as
    /// one number (design section 3.6).
    pub(crate) fn fold_runs<const N: usize>(
        &self,
        tokens: impl IntoIterator<Item = usize>,
    ) -> Option<([(usize, usize); N], usize)> {
        let mut out = [(0usize, 0usize); N];
        let mut len = 0usize;
        for j in tokens {
            if let Some(unit) = len.checked_sub(1).and_then(|k| out.get_mut(k))
                && self.run_continues(unit.1, j)
            {
                unit.1 = j;
                continue;
            }
            *out.get_mut(len)? = (j, j);
            len += 1;
        }
        Some((out, len))
    }

    /// The units of a core (`fold_runs`), at most `N`. A `Number` run split by `Format` or
    /// `Markup` tokens reads as one number (design section 3.6).
    pub(crate) fn core_units<const N: usize>(
        &self,
        frag: &Frag,
    ) -> Option<([(usize, usize); N], usize)> {
        self.fold_runs(self.core(frag))
    }

    /// The number of visible characters of a range of tokens, the digits of a number run. A
    /// `Number` run split by `Format` or `Markup` tokens reads as one number (design section
    /// 3.6).
    pub(crate) fn digits_len(&self, first: usize, last: usize) -> usize {
        self.visible(first, last).count()
    }

    /// The visible characters of a range of tokens: their source text without format characters,
    /// ZWNJ and ZWJ (design section 3.6).
    pub(crate) fn visible(
        &self,
        first: usize,
        last: usize,
    ) -> impl Iterator<Item = char> + Clone + 'a {
        let start = self.toks.get(first).map_or(0, |t| t.src.start) as usize;
        let end = self.toks.get(last).map_or(0, |t| t.src.end) as usize;
        self.visible_bytes(start, end)
    }

    /// The visible characters of the source bytes `start..end` (none when `end <= start`).
    fn visible_bytes(&self, start: usize, end: usize) -> impl Iterator<Item = char> + Clone + 'a {
        self.text
            .get(start..end.max(start))
            .unwrap_or("")
            .chars()
            .filter(|c| !super::lex::is_joiner(*c) && !super::lex::is_format_char(*c))
    }

    /// The visible characters of a core.
    pub(crate) fn core_chars(&self, frag: &Frag) -> impl Iterator<Item = char> + Clone + 'a {
        // An empty core has no characters, also in a window of one token, where `visible(1, 0)`
        // would read the whole of token 0.
        if frag.core_start >= frag.core_end {
            return self.visible_bytes(0, 0);
        }
        let start = self
            .toks
            .get(frag.core_start as usize)
            .map_or(0, |t| t.src.start) as usize;
        let end = self
            .toks
            .get(frag.core_end as usize - 1)
            .map_or(0, |t| t.src.end) as usize;
        self.visible_bytes(start, end)
    }

    /// A pattern space (design section 8.2): a `Space` token of exactly one character, U+0020 or a
    /// glued U+00A0 or U+202F, not frozen.
    pub(crate) fn pattern_space(&self, i: usize) -> bool {
        self.toks.get(i).is_some_and(|t| {
            t.kind == Kind::Space && !t.is_frozen() && (t.glue != Glue::None || self.src(i) == " ")
        })
    }

    /// A pattern space that no other `Space` token touches through transparent tokens: a run of
    /// spaces cut by a `Markup` token is a run, not a single space (design section 8.2).
    pub(crate) fn lone_space(&self, i: usize) -> bool {
        let space = |j: Option<usize>| {
            j.is_some_and(|j| self.toks.get(j).is_some_and(|t| t.kind == Kind::Space))
        };
        self.pattern_space(i) && !space(self.prev(i)) && !space(self.next(i))
    }

    /// Whether a token is a single U+0020 without glue.
    pub(crate) fn plain_space(&self, i: usize) -> bool {
        self.toks.get(i).is_some_and(|t| {
            t.kind == Kind::Space && !t.is_frozen() && t.glue == Glue::None && self.src(i) == " "
        })
    }

    /// Whether the token starts a line: only horizontal whitespace (and `Format` and `Markup`
    /// tokens) stands between it and a line break or the window start.
    pub(crate) fn line_start(&self, i: usize) -> bool {
        let mut j = i;
        while let Some(p) = self.prev(j) {
            match self.kind(p) {
                Some(Kind::Space) => j = p,
                Some(Kind::Newline) => return true,
                _ => return false,
            }
        }
        true
    }

    /// The fragment kind as the rules read it: a one-word `Dictionary` core counts as `Word`
    /// (design section 8.2).
    pub(crate) fn kind_of(&self, frag: &Frag) -> FragKind {
        if frag.kind == FragKind::Dictionary && self.one_word(frag) {
            FragKind::Word
        } else {
            frag.kind
        }
    }

    /// Whether the core is one `Word` token.
    pub(crate) fn one_word(&self, frag: &Frag) -> bool {
        let mut core = self.core(frag);
        matches!(
            (core.next().and_then(|i| self.kind(i)), core.next()),
            (Some(Kind::Word(_)), None)
        )
    }
}

/// The text a form of the language's ellipsis data puts in place of the dots: the form without
/// its leading `?` and `!`, which stay in their own tokens (`?..` gives `..`).
pub(crate) fn ellipsis_tail(data: &Table<'_>, form: EllipsisForm) -> &'static str {
    let forms = &data.ellipsis;
    let whole = match form {
        EllipsisForm::Question => forms.question,
        EllipsisForm::Exclamation => forms.exclamation,
        EllipsisForm::Both => forms.both,
    };
    whole.trim_start_matches(['?', '!'])
}

/// The most rules that one phase has in `ORDER`: the capacity of `PhaseRules`, so a rule added
/// to a phase can never be dropped for want of room.
const PHASE_CAPACITY: usize = {
    let mut max = 0;
    let mut outer: &[(Rule, Phase)] = &ORDER;
    while let [(_, phase), rest @ ..] = outer {
        let mut n = 0;
        let mut inner: &[(Rule, Phase)] = &ORDER;
        while let [(_, other), tail @ ..] = inner {
            if *other as u8 == *phase as u8 {
                n += 1;
            }
            inner = tail;
        }
        if n > max {
            max = n;
        }
        outer = rest;
    }
    max
};

/// The active rules of one phase, in the order of design section 4.1.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct PhaseRules {
    len: usize,
    rules: [Rule; PHASE_CAPACITY],
    /// The kinds at which some of the rules can match (`ANCHORS`).
    anchors: u32,
}

impl PhaseRules {
    /// The rules of `phase` that are in `active`.
    pub(crate) fn of(phase: Phase, active: crate::RuleSet) -> PhaseRules {
        let mut out = PhaseRules {
            len: 0,
            rules: [Rule::SymbolsCopyright; PHASE_CAPACITY],
            anchors: 0,
        };
        for (rule, p) in ORDER {
            if p == phase
                && active.contains(rule)
                && let Some(slot) = out.rules.get_mut(out.len)
            {
                *slot = rule;
                out.len += 1;
                out.anchors |= ANCHORS.get(rule as usize).copied().unwrap_or(!0);
            }
        }
        out
    }

    /// The kinds at which some rule of the phase can match, as bits of `Kind::bit`.
    pub(crate) const fn anchors(&self) -> u32 {
        self.anchors
    }

    pub(crate) fn as_slice(&self) -> &[Rule] {
        self.rules.get(..self.len).unwrap_or_default()
    }
}

/// What a phase did.
#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct Outcome {
    /// The output tokens were written to `out`. Until the first match with edits nothing is
    /// copied; a phase without one leaves `out` empty and its input stands as its output.
    pub(crate) written: bool,
    /// An edit was applied.
    pub(crate) changed: bool,
    /// A `Space` token was inserted or removed, so the token indices of the fragments moved.
    pub(crate) moved: bool,
}

/// One phase (design section 3.8): at each position the active rules of the phase are tried in
/// the order of design section 4.1; the first that matches takes the anchor and `consume` tokens,
/// which are not considered again. The output goes to `out` once a match has edits (`written`); the
/// source ranges of removed tokens go to `removed`.
pub(crate) fn run_phase<R: Recorder>(
    cx: &Cx<'_>,
    phase: &PhaseRules,
    out: &mut Vec<Tok>,
    removed: &mut Vec<(u32, u32)>,
    sink: &mut Sink<'_, R>,
) -> Outcome {
    out.clear();
    let input = cx.toks;
    let mut outcome = Outcome::default();
    let rules = phase.as_slice();
    let anchors = phase.anchors();
    let mut i = 0;
    while let Some(tok) = input.get(i) {
        let bit = tok.kind.bit();
        let found = if tok.is_frozen() || anchors & bit == 0 {
            None
        } else {
            rules.iter().find_map(|&rule| {
                if ANCHORS.get(rule as usize).is_some_and(|set| set & bit == 0) {
                    return None;
                }
                RULES
                    .get(rule as usize)
                    .and_then(|f| f(cx, i))
                    .map(|m| (rule, m))
            })
        };
        #[cfg(test)]
        if !tok.is_frozen() {
            // `ANCHORS` holds every kind at which a rule matches.
            for &rule in rules {
                if ANCHORS.get(rule as usize).is_some_and(|set| set & bit == 0) {
                    let f = RULES.get(rule as usize);
                    assert!(
                        f.and_then(|f| f(cx, i)).is_none(),
                        "{rule:?} at {:?}",
                        tok.kind
                    );
                }
            }
        }
        match found {
            Some((rule, m)) if edits_allowed(input, i, &m) => {
                if R::TRACE {
                    let end = input
                        .get(i + m.consume as usize)
                        .map_or(tok.src.end, |t| t.src.end);
                    let edits = m.edits.iter().next().is_some();
                    sink.rec.matched(rule, tok.src.start..end, m.consume, edits);
                }
                if let Some(at) = m.note
                    && let Some(t) = input.get(i + at as usize)
                {
                    sink.diagnostic_of(
                        DiagnosticKind::AmbiguousHyphen,
                        Severity::Info,
                        Some(rule),
                        loc(t),
                    );
                }
                let next = i + m.consume as usize + 1;
                if m.edits.iter().next().is_none() {
                    // A match without edits leaves its tokens as they are.
                    if outcome.written {
                        out.extend_from_slice(input.get(i..next).unwrap_or_default());
                    }
                } else {
                    if !outcome.written {
                        out.extend_from_slice(input.get(..i).unwrap_or_default());
                        outcome.written = true;
                    }
                    emit(cx, input, i, rule, &m, out, removed, sink, &mut outcome);
                }
                i = next;
            }
            _ => {
                if outcome.written {
                    out.push(*tok);
                }
                i += 1;
            }
        }
    }
    outcome
}

/// Where a token lies, for a record.
pub(crate) fn loc(t: &Tok) -> Loc {
    if t.piece == NO_PIECE {
        Loc::Flat(t.src.start, t.src.end)
    } else {
        Loc::Piece(t.piece)
    }
}

/// The text of a glue mark.
pub(crate) const fn glue_char(glue: Glue) -> char {
    match glue {
        Glue::None => ' ',
        Glue::NoBreak => '\u{a0}',
        Glue::Narrow => '\u{202f}',
    }
}

/// Writes the taken tokens of a match with its edits applied, and records them.
#[expect(clippy::too_many_arguments, reason = "the parts of one phase")]
fn emit<R: Recorder>(
    cx: &Cx<'_>,
    input: &[Tok],
    anchor: usize,
    rule: Rule,
    m: &Match,
    out: &mut Vec<Tok>,
    removed: &mut Vec<(u32, u32)>,
    sink: &mut Sink<'_, R>,
    outcome: &mut Outcome,
) {
    let mut buf = [0u8; 4];
    let insert = |before: u8, out: &mut Vec<Tok>, sink: &mut Sink<'_, R>, outcome: &mut Outcome| {
        for edit in m.edits.iter() {
            let Edit::Insert { before: b, glue } = *edit else {
                continue;
            };
            if b != before {
                continue;
            }
            let at = input
                .get(anchor + b as usize)
                .map(|t| t.src.start)
                .or_else(|| out.last().map(|t| t.src.end))
                .unwrap_or(0);
            // The text on the left owns the insertion when no text token follows it (design
            // section 3.11), as in `collect` of the renderer.
            let next_text = input
                .get(anchor + b as usize)
                .is_some_and(|t| t.piece == NO_PIECE);
            let left = out.last().is_some_and(|t| t.piece == NO_PIECE) || !next_text;
            let mut text = [0u8; 4];
            sink.change(rule, at, at, left, glue_char(glue).encode_utf8(&mut text));
            out.push(Tok::inserted(at, glue));
            outcome.changed = true;
            outcome.moved = true;
        }
    };
    for k in 0..=m.consume {
        let Some(&original) = input.get(anchor + k as usize) else {
            break;
        };
        insert(k, out, sink, outcome);
        let mut tok = original;
        let mut keep = true;
        for edit in m.edits.iter() {
            match *edit {
                Edit::Replace { at, repl } if at == k => {
                    if tok.repl == repl {
                        continue;
                    }
                    tok.repl = repl;
                    tok.flags = tok.flags.with(Flags::CHANGED);
                    let text: &str = match repl {
                        Repl::Source => cx.src(anchor + k as usize),
                        Repl::Char(c) => c.encode_utf8(&mut buf),
                        Repl::Ellipsis(form) => ellipsis_tail(&cx.data, form),
                    };
                    sink.change(rule, tok.src.start, tok.src.end, true, text);
                    outcome.changed = true;
                }
                Edit::Glue { at, glue } if at == k => {
                    if tok.glue == glue {
                        continue;
                    }
                    tok.glue = glue;
                    tok.flags = tok.flags.with(Flags::CHANGED);
                    let mut text = [0u8; 4];
                    sink.change(
                        rule,
                        tok.src.start,
                        tok.src.end,
                        true,
                        glue_char(glue).encode_utf8(&mut text),
                    );
                    outcome.changed = true;
                }
                Edit::Remove { at } if at == k => keep = false,
                _ => {}
            }
        }
        if keep {
            out.push(tok);
        } else {
            sink.change(rule, tok.src.start, tok.src.end, true, "");
            removed.push((tok.src.start, tok.src.end));
            outcome.changed = true;
            outcome.moved = true;
        }
    }
    // An insertion after the last taken token; with 256 taken tokens no offset names that place.
    if let Some(after) = m.consume.checked_add(1) {
        insert(after, out, sink, outcome);
    }
}

/// Moves the token ranges of the fragments to their tokens after a phase inserted or removed a
/// `Space` token. The fragments, their kinds and flags stay those of the pass (design section
/// 3.6); only their positions follow the tokens.
pub(crate) fn remap(tokens: &[Tok], frags: &mut [Frag]) {
    let mut cur: Option<usize> = None;
    let mut seen_core = false;
    for (j, tok) in tokens.iter().enumerate() {
        // `j + 1` below must not overflow: a window of 2^32 tokens stops here (design section
        // 3.4 bounds bytes and pieces, not tokens).
        let Some(j) = u32::try_from(j).ok().filter(|j| *j < u32::MAX) else {
            return;
        };
        if tok.frag == NO_FRAG {
            continue;
        }
        let f = tok.frag as usize;
        let Some(frag) = frags.get_mut(f) else {
            continue;
        };
        if cur != Some(f) {
            cur = Some(f);
            seen_core = false;
            let empty_core = frag.core_start >= frag.core_end;
            let offset = frag.core_start.saturating_sub(frag.start);
            frag.start = j;
            if empty_core {
                frag.core_start = j.saturating_add(offset);
                frag.core_end = j.saturating_add(offset);
            }
        }
        frag.end = j + 1;
        if tok.flags.contains(Flags::CORE) {
            if !seen_core {
                frag.core_start = j;
                seen_core = true;
            }
            frag.core_end = j + 1;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::super::ingest::Atom;
    use super::super::lex::{Flags, Kind, lex_window};
    use super::*;

    fn tokens(text: &str, frozen: &[usize]) -> Vec<Tok> {
        let atoms = [Atom::Text {
            start: 0,
            end: u32::try_from(text.len()).unwrap(),
        }];
        let mut buf = Vec::new();
        let mut out = Vec::new();
        lex_window(text, &atoms, &mut buf, &mut out);
        for i in frozen {
            out[*i].flags = out[*i].flags.with(Flags::FROZEN);
        }
        out
    }

    #[test]
    fn an_empty_core_has_no_characters_in_a_window_of_one_token() {
        let t = Typographer::new(crate::Language::COMMON, crate::RuleSet::EMPTY);
        let text = "word";
        let toks = tokens(text, &[]);
        assert_eq!(toks.len(), 1);
        let cx = Cx {
            t: &t,
            data: t.language().data(),
            text,
            toks: &toks,
            frags: &[],
            quotes: &[],
            pairs: None,
            single_quotes: SingleQuotes::NestedOnly,
            dash: None,
            overlay: &[],
            hint: core::cell::Cell::new(0),
            sides: core::cell::Cell::new(None),
        };
        let empty = Frag {
            start: 0,
            end: 1,
            core_start: 0,
            core_end: 0,
            kind: FragKind::Punctuation,
            stop: false,
            label: false,
            other_slot: false,
        };
        assert_eq!(cx.core_chars(&empty).count(), 0);
        let whole = Frag {
            core_end: 1,
            kind: FragKind::Word,
            ..empty
        };
        assert_eq!(cx.core_chars(&whole).collect::<String>(), "word");
    }

    fn one(edit: Edit, consume: u8) -> Match {
        Match::with(consume, Edits::one(edit))
    }

    #[test]
    fn a_proposal_holds_at_most_max_edits() {
        let edit = |at| Edit::Glue {
            at,
            glue: Glue::NoBreak,
        };
        let mut edits = Edits::NONE;
        for at in 0..MAX_EDITS {
            edits = edits.try_with(edit(u8::try_from(at).unwrap())).unwrap();
        }
        assert_eq!(edits.iter().count(), MAX_EDITS);
        assert_eq!(edits.try_with(edit(9)), None);
        assert_eq!(
            Edits::one(edit(3)).iter().copied().collect::<Vec<_>>(),
            [edit(3)]
        );
    }

    #[test]
    fn every_rule_of_a_phase_has_room() {
        for (_, phase) in ORDER {
            let expected = ORDER.iter().filter(|(_, p)| *p == phase).count();
            assert_eq!(
                PhaseRules::of(phase, crate::RuleSet::ALL).as_slice().len(),
                expected,
                "{phase:?}"
            );
        }
    }

    #[test]
    fn edits_keep_the_contract_of_edit() {
        // Invariant I1 of design section 4.3: glue and removal only on `Space` tokens, no edit of
        // a `Format` or `Markup` token.
        let t = tokens("a b", &[]);
        let glue = |at| Edit::Glue {
            at,
            glue: Glue::NoBreak,
        };
        assert!(!edits_allowed(&t, 0, &one(glue(0), 1)));
        assert!(!edits_allowed(&t, 0, &one(Edit::Remove { at: 0 }, 1)));
        assert!(edits_allowed(&t, 0, &one(glue(1), 1)));
        assert!(edits_allowed(&t, 0, &one(Edit::Remove { at: 1 }, 1)));
        let t = tokens("a\u{AD}b", &[]);
        assert_eq!(t[1].kind, Kind::Format);
        let replace = Edit::Replace {
            at: 1,
            repl: Repl::Char('x'),
        };
        assert!(!edits_allowed(&t, 0, &one(replace, 1)));
    }

    #[test]
    fn edits_of_frozen_tokens_are_refused() {
        // `в 10.0.0.0/8`: the space may be glued, the fragment may not change.
        let t = tokens("в 10.0.0.0/8", &[2, 3, 4, 5, 6, 7, 8, 9, 10]);
        assert!(edits_allowed(
            &t,
            0,
            &one(
                Edit::Glue {
                    at: 1,
                    glue: Glue::NoBreak
                },
                1
            )
        ));
        assert!(!edits_allowed(
            &t,
            0,
            &one(
                Edit::Replace {
                    at: 2,
                    repl: Repl::Char('x')
                },
                2
            )
        ));
        assert!(!edits_allowed(&t, 2, &Match::done(0)));
        // An edit outside the taken range is refused.
        assert!(!edits_allowed(
            &t,
            0,
            &one(
                Edit::Glue {
                    at: 1,
                    glue: Glue::NoBreak
                },
                0
            )
        ));
        // A whole proposal is dropped when one of its edits is refused.
        let both = Match::with(
            2,
            Edits::one(Edit::Glue {
                at: 1,
                glue: Glue::NoBreak,
            })
            .try_with(Edit::Remove { at: 2 })
            .unwrap(),
        );
        assert!(!edits_allowed(&t, 0, &both));
    }

    #[test]
    fn insertions_inside_a_frozen_fragment_are_refused() {
        let t = tokens("a.b", &[0, 1, 2]);
        assert!(!edits_allowed(
            &t,
            0,
            &one(
                Edit::Insert {
                    before: 1,
                    glue: Glue::NoBreak
                },
                1
            )
        ));
        // `т.е` unfrozen: the insertion is allowed.
        let t = tokens("т.е", &[]);
        assert!(edits_allowed(
            &t,
            0,
            &one(
                Edit::Insert {
                    before: 2,
                    glue: Glue::NoBreak
                },
                2
            )
        ));
        // Next to an opaque token the insertion falls outside it.
        let mut t = tokens("№", &[]);
        let mut slot = t[0];
        slot.kind = Kind::Slot(crate::SlotKind::Number);
        slot.flags = Flags::FROZEN;
        t.push(slot);
        assert!(edits_allowed(
            &t,
            0,
            &one(
                Edit::Insert {
                    before: 1,
                    glue: Glue::NoBreak
                },
                1
            )
        ));
        assert_eq!(Edits::one(Edit::Remove { at: 0 }).iter().count(), 1);
    }
}
