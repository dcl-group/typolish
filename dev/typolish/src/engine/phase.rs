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

/// The most edits one proposal holds; `nbsp.phone` glues up to five spaces.
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

    /// The edits with one more; a seventh edit is dropped, which a rule never proposes.
    pub(crate) fn with(mut self, edit: Edit) -> Edits {
        if let Some(slot) = self.items.get_mut(self.len as usize) {
            *slot = edit;
            self.len += 1;
        }
        self
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
/// opaque, is allowed: it falls outside them.
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
        Edit::Replace { at, .. } | Edit::Glue { at, .. } | Edit::Remove { at } => editable(at),
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
    pub(crate) pairs: Option<QuotePairs>,
    pub(crate) single_quotes: SingleQuotes,
    pub(crate) dash: Option<DashData>,
}

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
        match self.toks.get(i)?.repl {
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

    /// The visible characters of a range of tokens: their source text without format characters,
    /// ZWNJ and ZWJ (design section 3.6).
    pub(crate) fn visible(
        &self,
        first: usize,
        last: usize,
    ) -> impl Iterator<Item = char> + Clone + 'a {
        let start = self.toks.get(first).map_or(0, |t| t.src.start) as usize;
        let end = self.toks.get(last).map_or(0, |t| t.src.end) as usize;
        self.text
            .get(start..end.max(start))
            .unwrap_or("")
            .chars()
            .filter(|c| !super::lex::is_joiner(*c) && !super::lex::is_format_char(*c))
    }

    /// The visible characters of a core.
    pub(crate) fn core_chars(&self, frag: &Frag) -> impl Iterator<Item = char> + Clone + 'a {
        if frag.core_start >= frag.core_end {
            return self.visible(1, 0);
        }
        self.visible(frag.core_start as usize, frag.core_end as usize - 1)
    }

    /// A pattern space (design section 8.2): a `Space` token of exactly one character, U+0020 or a
    /// glued U+00A0 or U+202F, not frozen.
    pub(crate) fn pattern_space(&self, i: usize) -> bool {
        self.toks.get(i).is_some_and(|t| {
            t.kind == Kind::Space && !t.is_frozen() && (t.glue != Glue::None || self.src(i) == " ")
        })
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

/// The active rules of one phase, in the order of design section 4.1.
#[derive(Clone, Copy, Debug)]
pub(crate) struct PhaseRules {
    len: usize,
    rules: [Rule; 10],
}

impl PhaseRules {
    /// The rules of `phase` that are in `active`.
    pub(crate) fn of(phase: Phase, active: crate::RuleSet) -> PhaseRules {
        let mut out = PhaseRules {
            len: 0,
            rules: [Rule::SymbolsCopyright; 10],
        };
        for (rule, p) in ORDER {
            if p == phase
                && active.contains(rule)
                && let Some(slot) = out.rules.get_mut(out.len)
            {
                *slot = rule;
                out.len += 1;
            }
        }
        out
    }

    pub(crate) fn as_slice(&self) -> &[Rule] {
        self.rules.get(..self.len).unwrap_or_default()
    }

    pub(crate) fn is_empty(&self) -> bool {
        self.len == 0
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
/// the order of section 4.1; the first that matches takes the anchor and `consume` tokens, which
/// are not considered again. The output goes to `out` once a match has edits (`written`); the
/// source ranges of removed tokens go to `removed`.
pub(crate) fn run_phase<R: Recorder>(
    cx: &Cx<'_>,
    rules: &[Rule],
    out: &mut Vec<Tok>,
    removed: &mut Vec<(u32, u32)>,
    sink: &mut Sink<'_, R>,
) -> Outcome {
    out.clear();
    let input = cx.toks;
    let mut outcome = Outcome::default();
    // The kinds at which some rule of the phase can match (`ANCHORS`).
    let anchors = rules.iter().fold(0, |set, &rule| {
        set | ANCHORS.get(rule as usize).copied().unwrap_or(!0)
    });
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
            let left = out.last().is_some_and(|t| t.piece == NO_PIECE);
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
    insert(m.consume.saturating_add(1), out, sink, outcome);
}

/// Moves the token ranges of the fragments to their tokens after a phase inserted or removed a
/// `Space` token. The fragments, their kinds and flags stay those of the pass (design section
/// 3.6); only their positions follow the tokens.
pub(crate) fn remap(tokens: &[Tok], frags: &mut [Frag]) {
    let mut cur: Option<usize> = None;
    let mut seen_core = false;
    for (j, tok) in tokens.iter().enumerate() {
        let Ok(j) = u32::try_from(j) else {
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
                frag.core_start = j + offset;
                frag.core_end = j + offset;
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

    fn one(edit: Edit, consume: u8) -> Match {
        Match::with(consume, Edits::NONE.with(edit))
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
            Edits::NONE
                .with(Edit::Glue {
                    at: 1,
                    glue: Glue::NoBreak,
                })
                .with(Edit::Remove { at: 2 }),
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
        assert_eq!(Edits::NONE.with(Edit::Remove { at: 0 }).iter().count(), 1);
    }
}
