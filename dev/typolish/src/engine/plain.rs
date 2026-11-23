//! The reverse transformation (design sections 2.10 and 12.1): typeset Unicode text back to the
//! canonical plain form of the typographer.
//!
//! The inverse tables of section 12.1 give, for every typographic character outside the spans of
//! `protected()`, the plain text it came from (a candidate; a few characters have a second
//! reading, such as `’`, which is an apostrophe or the closer of a nested pair). A candidate is
//! undone only where the enabled rules produce its character: the engine runs forward on the text
//! with the candidates undone, and a candidate whose plain text does not come back as its
//! character, or whose plain text makes a rule change the text next to it, is tried in its next
//! reading or kept. The check repeats until every remaining candidate holds, and the whole pass
//! repeats on its own output until nothing changes, so the result is a fixed point of
//! `to_plain`. So "undoes exactly the enabled rules" is decided by the rules themselves, with no
//! second copy of their patterns.

use std::borrow::Cow;

use super::Input;
use super::ingest::Atom;
use super::record::{NoRecord, Recorder};
use super::render::{REdit, write_text};
use super::scratch::Scratch;
use crate::report::{Change, Diagnostic, ProtectedKind, ProtectedSpan};
use crate::{Rule, Typographer};

/// The apostrophe, also the English closing single quote (design section 6.2).
const APOSTROPHE: char = '\u{2019}';

/// Collects the protected spans of a run and nothing else.
#[derive(Debug, Default)]
struct Spans {
    spans: Vec<ProtectedSpan>,
}

impl Recorder for Spans {
    const ACTIVE: bool = true;

    fn change(&mut self, _change: Change) {}

    fn protected(&mut self, span: ProtectedSpan) {
        self.spans.push(span);
    }

    fn diagnostic(&mut self, _diagnostic: Diagnostic) {}
}

/// One typographic character of the text and its readings in plain text.
#[derive(Clone, Copy, Debug)]
struct Cand {
    /// The byte range of the character in the call's text.
    start: usize,
    end: usize,
    /// The plain readings, the likeliest first.
    alts: &'static [&'static str],
    /// The reading being tried.
    alt: usize,
    /// Whether the candidate is undone in the text being checked.
    on: bool,
    /// Whether it is undone without the forward check: the quote glyphs and apostrophes of
    /// design section 12.1, whose plain form does not depend on the level the rule would choose.
    fixed: bool,
    /// The window it lies in (design section 3.3).
    window: usize,
    /// Whether it is an em dash, whose spaces the dash style decides.
    dash: bool,
}

impl Cand {
    fn plain(&self) -> &'static str {
        self.alts.get(self.alt).copied().unwrap_or("")
    }

    /// Whether it is read as the closed form of a dash, with spaces restored around it.
    fn closed(&self) -> bool {
        self.dash && self.plain().starts_with(' ')
    }
}

/// A `Text` piece of the input: its index, its start in the call's text, and its text.
#[derive(Clone, Copy, Debug)]
struct TextAt<'s> {
    index: usize,
    start: usize,
    text: &'s str,
}

/// `Typographer::to_plain`.
pub(crate) fn to_plain<'a>(
    typographer: &Typographer,
    scratch: &mut Scratch,
    text: &'a str,
) -> Cow<'a, str> {
    let input = [Input::Text(text)];
    let mut new = plain_texts(typographer, scratch, &input);
    match new.pop().flatten() {
        Some(changed) => Cow::Owned(changed),
        None => Cow::Borrowed(text),
    }
}

/// `Typographer::to_plain_template`: the texts in the n + 1 form and whether any of them changed.
pub(crate) fn to_plain_template<'t>(
    typographer: &Typographer,
    scratch: &mut Scratch,
    input: &[Input<'t>],
    slot_count: usize,
) -> (Vec<Cow<'t, str>>, bool) {
    let new = plain_texts(typographer, scratch, input);
    let mut texts = Vec::with_capacity(slot_count.saturating_add(1));
    let mut changed = false;
    let mut offset = 0usize;
    for group in input.split(|item| matches!(item, Input::Slot(_))) {
        let base = offset;
        offset = offset.saturating_add(group.len()).saturating_add(1);
        let own = |k: usize| new.get(base.saturating_add(k)).and_then(Option::as_deref);
        let touched = (0..group.len()).any(|k| own(k).is_some());
        if !touched {
            texts.push(merge(group));
            continue;
        }
        changed = true;
        let mut out = String::new();
        for (k, item) in group.iter().enumerate() {
            match (*item, own(k)) {
                (Input::Text(_), Some(text)) => out.push_str(text),
                (Input::Text(s) | Input::Protected(s) | Input::Markup(s), _) => out.push_str(s),
                (Input::Slot(_), _) => {}
            }
        }
        texts.push(Cow::Owned(out));
    }
    (texts, changed)
}

/// The text of the pieces between two slots: borrowed while it maps to at most one non-empty
/// piece, as in `typeset_template`.
fn merge<'t>(group: &[Input<'t>]) -> Cow<'t, str> {
    let mut parts = group.iter().filter_map(|item| match *item {
        Input::Text(text) | Input::Protected(text) | Input::Markup(text) if !text.is_empty() => {
            Some(text)
        }
        _ => None,
    });
    let Some(first) = parts.next() else {
        return Cow::Borrowed("");
    };
    if parts.clone().next().is_none() {
        return Cow::Borrowed(first);
    }
    let mut all = String::from(first);
    parts.for_each(|text| all.push_str(text));
    Cow::Owned(all)
}

/// The plain text of every input piece: `Some` for a `Text` piece that changed. The pass repeats
/// on its own output until it changes nothing (design section 13.5, property 16).
fn plain_texts(
    typographer: &Typographer,
    scratch: &mut Scratch,
    input: &[Input<'_>],
) -> Vec<Option<String>> {
    let mut owned: Vec<Option<String>> = vec![None; input.len()];
    loop {
        let current: Vec<Input<'_>> = input
            .iter()
            .zip(&owned)
            .map(|(item, own)| match (item, own) {
                (Input::Text(_), Some(text)) => Input::Text(text.as_str()),
                _ => *item,
            })
            .collect();
        // Every round that changes something turns at least one non-ASCII character into ASCII
        // text, so the loop ends.
        let Some(changed) = round(typographer, scratch, &current) else {
            break;
        };
        drop(current);
        for (index, text) in changed {
            if let Some(slot) = owned.get_mut(index) {
                *slot = Some(text);
            }
        }
    }
    owned
}

/// One pass: the candidates of the text, checked against the forward rules. Returns the new text
/// of every `Text` piece that changed, by input index, or `None` when nothing changed.
fn round(
    typographer: &Typographer,
    scratch: &mut Scratch,
    input: &[Input<'_>],
) -> Option<Vec<(usize, String)>> {
    let settings = Reading::of(typographer);
    let any = input.iter().any(|item| match item {
        Input::Text(text) => text.chars().any(|c| settings.is_candidate(c)),
        _ => false,
    });
    if !any {
        return None;
    }
    let mut pieces: Vec<TextAt<'_>> = Vec::new();
    let mut flat = String::new();
    for (index, item) in input.iter().enumerate() {
        if let Input::Text(text) = item
            && !text.is_empty()
        {
            pieces.push(TextAt {
                index,
                start: flat.len(),
                text,
            });
            flat.push_str(text);
        }
    }
    u32::try_from(flat.len()).ok()?;

    // The forward run on the text as it is: its protected spans, its window boundaries and slots,
    // and the edits it would make anyway.
    let mut spans = Spans::default();
    super::run(typographer, scratch, input, &mut spans);
    let mut base: Vec<REdit> = scratch.edits.clone();
    base.sort_unstable_by_key(|e| (e.start, e.end));
    let mut walls: Vec<usize> = Vec::new();
    let mut boundaries: Vec<usize> = Vec::new();
    for atom in &scratch.atoms {
        match *atom {
            Atom::Slot { at, .. } | Atom::Protected { at, .. } => walls.push(at as usize),
            Atom::Boundary { start, .. } => boundaries.push(start as usize),
            _ => {}
        }
    }
    walls.sort_unstable();
    boundaries.sort_unstable();
    let mut protected: Vec<(usize, usize)> = spans
        .spans
        .iter()
        .filter(|span| {
            !matches!(
                span.kind(),
                ProtectedKind::Slot | ProtectedKind::ProtectedPiece
            )
        })
        .filter_map(|span| {
            let piece = pieces.iter().find(|p| p.index == span.piece())?;
            let range = span.range();
            Some((
                piece.start.saturating_add(range.start),
                piece.start.saturating_add(range.end),
            ))
        })
        .collect();
    protected.sort_unstable();

    let text = Flat {
        text: &flat,
        walls: &walls,
    };
    let mut cands = candidates(&settings, &text, &protected, &boundaries);
    if cands.is_empty() {
        return None;
    }
    let check = Check {
        typographer,
        input,
        pieces: &pieces,
        flat: &flat,
        base: &base,
        boundaries: &boundaries,
    };
    // First the candidates that do not come back are put aside, until all that are left come
    // back; one that failed only because of a neighbour's wrong reading is then tried again, one
    // at a time in each window (windows are independent, design section 3.3), in each of its
    // readings.
    let mut parked = vec![false; cands.len()];
    let mut verdict = loop {
        let verdict = check.run(scratch, &cands);
        let mut failed = false;
        for (k, cand) in cands.iter_mut().enumerate() {
            if cand.on && !cand.fixed && verdict.failed.get(k).copied().unwrap_or(true) {
                cand.on = false;
                if let Some(p) = parked.get_mut(k) {
                    *p = true;
                }
                failed = true;
            }
        }
        if failed {
            continue;
        }
        // An edit outside every candidate that the text as it was does not get: the plain text
        // of a neighbouring candidate made a rule fire, so it is not the plain text the character
        // came from (the round trip of section 12.2 would break). The nearest candidates on both
        // sides are put aside; one that was innocent comes back below. The spaces of a closed
        // dash are exempt: the closed form joins two words into one fragment, which by design
        // changes the rules around it, and `to_plain` restores them (section 8.4).
        let mut blamed = false;
        for &(window, ys, yt) in &verdict.stray {
            let suspect = |c: &Cand| c.on && !c.fixed && !c.closed() && c.window == window;
            let left = cands.iter().rposition(|c| suspect(c) && c.end <= ys);
            let right = cands.iter().position(|c| suspect(c) && c.start >= yt);
            for k in left.into_iter().chain(right) {
                if let (Some(cand), Some(p)) = (cands.get_mut(k), parked.get_mut(k)) {
                    cand.on = false;
                    *p = true;
                    blamed = true;
                }
            }
        }
        if !blamed {
            break verdict;
        }
    };
    let windows = boundaries.len().saturating_add(1);
    let mut queues: Vec<std::collections::VecDeque<usize>> =
        vec![std::collections::VecDeque::new(); windows];
    for k in parked
        .iter()
        .enumerate()
        .filter_map(|(k, &p)| p.then_some(k))
    {
        if let Some(cand) = cands.get_mut(k) {
            cand.alt = 0;
            if let Some(queue) = queues.get_mut(cand.window) {
                queue.push_back(k);
            }
        }
    }
    loop {
        let trial: Vec<usize> = queues.iter().filter_map(|q| q.front().copied()).collect();
        if trial.is_empty() {
            break;
        }
        for &k in &trial {
            if let Some(cand) = cands.get_mut(k) {
                cand.on = true;
            }
        }
        let next = check.run(scratch, &cands);
        let mut broken = vec![false; windows];
        for (cand, &failed) in cands.iter().zip(&next.failed) {
            if cand.on
                && !cand.fixed
                && failed
                && let Some(b) = broken.get_mut(cand.window)
            {
                *b = true;
            }
        }
        for &k in &trial {
            let Some(window) = cands.get(k).map(|c| c.window) else {
                continue;
            };
            let broke = broken.get(window).copied().unwrap_or(true);
            let closed = cands.get(k).is_some_and(Cand::closed);
            let stray = !closed
                && next
                    .stray
                    .iter()
                    .any(|s| s.0 == window && !verdict.stray.contains(s));
            if let Some(cand) = cands.get_mut(k) {
                if !broke && !stray {
                    if let Some(queue) = queues.get_mut(window) {
                        queue.pop_front();
                    }
                    continue;
                }
                cand.on = false;
                cand.alt = cand.alt.saturating_add(1);
                if cand.alt >= cand.alts.len()
                    && let Some(queue) = queues.get_mut(window)
                {
                    queue.pop_front();
                }
            }
        }
        // The strays of the windows whose trial held are those of the new state.
        let held: Vec<usize> = trial
            .iter()
            .filter_map(|&k| cands.get(k).filter(|c| c.on).map(|c| c.window))
            .collect();
        verdict.stray.retain(|s| !held.contains(&s.0));
        verdict
            .stray
            .extend(next.stray.iter().filter(|s| held.contains(&s.0)).copied());
    }
    if !cands.iter().any(|c| c.on) {
        return None;
    }
    let (texts, _) = build(&pieces, &cands);
    let changed: Vec<(usize, String)> = pieces
        .iter()
        .zip(texts)
        .filter(|(p, t)| p.text != t.as_str())
        .map(|(p, t)| (p.index, t))
        .collect();
    (!changed.is_empty()).then_some(changed)
}

/// What one forward run says about the candidates.
#[derive(Debug)]
struct Verdict {
    /// Per candidate: it is on and its plain text does not come back as its character.
    failed: Vec<bool>,
    /// The edits outside every candidate that the text as it was does not get, as the window and
    /// the range in the text as it was.
    stray: Vec<(usize, usize, usize)>,
}

/// The forward check of the candidates of one pass.
struct Check<'c, 'i> {
    typographer: &'c Typographer,
    input: &'c [Input<'i>],
    pieces: &'c [TextAt<'c>],
    flat: &'c str,
    /// The edits of the forward run on the text as it is, in order.
    base: &'c [REdit],
    boundaries: &'c [usize],
}

impl Check<'_, '_> {
    /// Runs the engine on the text with the candidates that are on undone.
    fn run(&self, scratch: &mut Scratch, cands: &[Cand]) -> Verdict {
        let (texts, xranges) = build(self.pieces, cands);
        let mut texts_iter = texts.iter();
        let mut next_piece = self.pieces.iter().peekable();
        let current: Vec<Input<'_>> = self
            .input
            .iter()
            .enumerate()
            .map(|(index, item)| {
                if next_piece.next_if(|p| p.index == index).is_some() {
                    texts_iter
                        .next()
                        .map_or(*item, |text| Input::Text(text.as_str()))
                } else {
                    *item
                }
            })
            .collect();
        super::run(self.typographer, scratch, &current, &mut NoRecord);
        let mut edits: Vec<REdit> = scratch.edits.clone();
        edits.sort_unstable_by_key(|e| (e.start, e.end));
        let data = self.typographer.language().data();
        let mut used = vec![false; edits.len()];
        let mut failed = vec![false; cands.len()];
        let mut segment = String::new();
        for (k, cand) in cands.iter().enumerate() {
            if !cand.on {
                continue;
            }
            let Some(&(a, b)) = xranges.get(k) else {
                continue;
            };
            let first = edits.partition_point(|e| (e.end as usize) <= a);
            let mut ok = true;
            segment.clear();
            let plain = cand.plain();
            let mut pos = a;
            for (n, e) in edits.iter().enumerate().skip(first) {
                let (s, t) = (e.start as usize, e.end as usize);
                if s >= b {
                    break;
                }
                if let Some(u) = used.get_mut(n) {
                    *u = true;
                }
                if s < a || t > b {
                    ok = false;
                    continue;
                }
                segment.push_str(plain.get(pos - a..s - a).unwrap_or(""));
                write_text(&mut segment, e.text, &data);
                pos = pos.max(t);
            }
            segment.push_str(
                plain
                    .get(pos.saturating_sub(a).min(plain.len())..)
                    .unwrap_or(""),
            );
            let original = self.flat.get(cand.start..cand.end).unwrap_or("");
            if let Some(f) = failed.get_mut(k) {
                *f = !ok || !same(cand.dash, &segment, original);
            }
        }
        let mut stray = Vec::new();
        let mut delta = 0isize;
        let mut next_cand = 0usize;
        for (n, e) in edits.iter().enumerate() {
            if used.get(n).copied().unwrap_or(true) {
                continue;
            }
            let (s, t) = (e.start as usize, e.end as usize);
            while let (Some(c), Some(r)) = (cands.get(next_cand), xranges.get(next_cand))
                && r.1 <= s
            {
                delta = delta.saturating_add(growth(c, *r));
                next_cand = next_cand.saturating_add(1);
            }
            let (ys, yt) = (back(s, delta), back(t, delta));
            let first = self
                .base
                .partition_point(|b| (b.start as usize, b.end as usize) < (ys, yt));
            let known = self
                .base
                .get(first..)
                .unwrap_or_default()
                .iter()
                .take_while(|b| (b.start as usize, b.end as usize) == (ys, yt))
                .any(|b| b.text == e.text);
            if !known {
                stray.push((self.boundaries.partition_point(|&b| b <= ys), ys, yt));
            }
        }
        Verdict { failed, stray }
    }
}

/// Whether the forward run gave the character of a candidate back. Glue of either width counts
/// as glue (design section 12.1: glue, U+00A0 or U+202F, set by a rule becomes a space); for a dash
/// the spaces around it do not count, since the dash style of the typographer decides them.
fn same(dash: bool, got: &str, want: &str) -> bool {
    let norm = |c: char| if c == '\u{202f}' { '\u{a0}' } else { c };
    let keep = |c: &char| !(dash && (*c == ' ' || *c == '\u{a0}'));
    got.chars()
        .map(norm)
        .filter(keep)
        .eq(want.chars().map(norm).filter(keep))
}

/// How many bytes longer the text of a candidate is in the text with the candidates undone.
fn growth(cand: &Cand, range: (usize, usize)) -> isize {
    let now = isize::try_from(range.1.saturating_sub(range.0)).unwrap_or(isize::MAX);
    let was = isize::try_from(cand.end.saturating_sub(cand.start)).unwrap_or(isize::MAX);
    now.saturating_sub(was)
}

/// A position of the text with the candidates undone, in the text as it was.
fn back(at: usize, delta: isize) -> usize {
    at.saturating_add_signed(delta.saturating_neg())
}

/// The texts of the `Text` pieces with the candidates that are on undone, and the range of every
/// candidate in the call's text so built (its character, if it is off).
fn build(pieces: &[TextAt<'_>], cands: &[Cand]) -> (Vec<String>, Vec<(usize, usize)>) {
    let mut texts = Vec::with_capacity(pieces.len());
    let mut ranges = Vec::with_capacity(cands.len());
    let mut flat = 0usize;
    let mut next = cands.iter().peekable();
    for piece in pieces {
        let end = piece.start.saturating_add(piece.text.len());
        let mut out = String::with_capacity(piece.text.len());
        let mut pos = piece.start;
        while let Some(cand) = next.next_if(|c| c.start < end) {
            let before = piece
                .text
                .get(pos - piece.start..cand.start.saturating_sub(piece.start))
                .unwrap_or("");
            out.push_str(before);
            let at = flat.saturating_add(out.len());
            if cand.on {
                out.push_str(cand.plain());
            } else {
                out.push_str(
                    piece
                        .text
                        .get(cand.start - piece.start..cand.end - piece.start)
                        .unwrap_or(""),
                );
            }
            ranges.push((at, flat.saturating_add(out.len())));
            pos = cand.end;
        }
        out.push_str(piece.text.get(pos - piece.start..).unwrap_or(""));
        flat = flat.saturating_add(out.len());
        texts.push(out);
    }
    (texts, ranges)
}

/// What the inverse tables read from the typographer.
#[derive(Clone, Copy, Debug)]
struct Reading {
    /// The opening and closing glyphs of the primary and the nested pair, when `quotes.pairs` is
    /// enabled.
    pairs: Option<[char; 4]>,
    /// The minus sign that `number.minus` writes, when it is not the hyphen-minus.
    minus: Option<char>,
    /// Whether `quotes.apostrophe` is enabled.
    apostrophe: bool,
}

impl Reading {
    fn of(typographer: &Typographer) -> Reading {
        let active = typographer.compiled().active;
        let pairs = typographer
            .quote_pairs()
            .filter(|_| active.contains(Rule::QuotesPairs))
            .map(|p| p.glyphs());
        let minus = Some(typographer.minus_sign()).filter(|&c| c != '-');
        let apostrophe = active.contains(Rule::QuotesApostrophe);
        Reading {
            pairs,
            minus,
            apostrophe,
        }
    }

    fn is_glyph(&self, c: char) -> bool {
        self.pairs.is_some_and(|g| g.contains(&c))
    }

    fn is_candidate(&self, c: char) -> bool {
        fixed(c).is_some()
            || matches!(c, '\u{2014}' | APOSTROPHE)
            || self.is_glyph(c)
            || self.minus == Some(c)
    }
}

/// The plain text of a character whose reading does not depend on its context (design section
/// 12.1).
fn fixed(c: char) -> Option<&'static [&'static str]> {
    Some(match c {
        '\u{a0}' | '\u{202f}' => &[" "],
        '\u{2013}' | '\u{2212}' => &["-"],
        '\u{2026}' => &["..."],
        '\u{d7}' => &["x"],
        '\u{a9}' => &["(c)"],
        '\u{ae}' => &["(r)"],
        '\u{2122}' => &["(tm)"],
        '\u{2192}' => &["->"],
        '\u{2190}' => &["<-"],
        '\u{2194}' => &["<->"],
        '\u{2265}' => &[">="],
        '\u{2264}' => &["<="],
        '\u{2260}' => &["!="],
        '\u{b1}' => &["+-"],
        '\u{2032}' => &["'"],
        '\u{2033}' => &["\""],
        _ => return None,
    })
}

/// The call's text with the positions of slots and protected pieces, which have no text.
#[derive(Clone, Copy, Debug)]
struct Flat<'s> {
    text: &'s str,
    walls: &'s [usize],
}

/// What stands next to a character, format characters skipped.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Near {
    /// The start or the end of the text.
    Edge,
    /// A slot or a protected piece.
    Wall,
    Char(char),
}

/// Format characters, which are transparent to the context (design section 5.4).
fn is_format(c: char) -> bool {
    matches!(
        c,
        '\u{ad}' | '\u{200b}'..='\u{200f}' | '\u{2060}'..='\u{2064}' | '\u{2066}'..='\u{2069}' | '\u{feff}'
    )
}

impl Flat<'_> {
    /// What stands before `at`, skipping format characters and the characters `skip` accepts.
    fn before(&self, mut at: usize, skip: impl Fn(char) -> bool) -> Near {
        loop {
            if self.walls.binary_search(&at).is_ok() {
                return Near::Wall;
            }
            let Some(c) = self.text.get(..at).and_then(|s| s.chars().next_back()) else {
                return Near::Edge;
            };
            if is_format(c) || skip(c) {
                at = at.saturating_sub(c.len_utf8());
                continue;
            }
            return Near::Char(c);
        }
    }

    /// What stands after `at`, skipping format characters and the characters `skip` accepts.
    fn after(&self, mut at: usize, skip: impl Fn(char) -> bool) -> Near {
        loop {
            if self.walls.binary_search(&at).is_ok() {
                return Near::Wall;
            }
            let Some(c) = self.text.get(at..).and_then(|s| s.chars().next()) else {
                return Near::Edge;
            };
            if is_format(c) || skip(c) {
                at = at.saturating_add(c.len_utf8());
                continue;
            }
            return Near::Char(c);
        }
    }
}

/// Whitespace for the readings of a dash: a space, glue, a line break, or the edge of the text.
fn blank(near: Near) -> bool {
    match near {
        Near::Edge => true,
        Near::Wall => false,
        Near::Char(c) => c.is_whitespace(),
    }
}

/// The classes of the neighbours of a quote (design section 6.3).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Class {
    Space,
    OpenP,
    CloseP,
    DashC,
    Word,
}

fn class(near: Near) -> Class {
    match near {
        Near::Edge => Class::Space,
        Near::Wall => Class::Word,
        Near::Char(c) if c.is_whitespace() => Class::Space,
        Near::Char('(' | '[' | '{' | '/') => Class::OpenP,
        Near::Char(')' | ']' | '}' | '.' | ',' | ';' | ':' | '!' | '?' | '\u{2026}') => {
            Class::CloseP
        }
        Near::Char('-' | '\u{2010}'..='\u{2015}' | '\u{2212}') => Class::DashC,
        Near::Char(_) => Class::Word,
    }
}

/// Quote characters of any form, which make a series (design section 6.3).
fn is_quote(reading: &Reading, c: char) -> bool {
    matches!(
        c,
        '"' | '\'' | '\u{ab}' | '\u{bb}' | '\u{2018}'
            ..='\u{201f}' | '\u{2039}' | '\u{203a}' | '\u{2032}' | '\u{2033}'
    ) || reading.is_glyph(c)
}

/// A quote glyph or an apostrophe of the text, for the reading of `’` (design section 12.1).
#[derive(Clone, Copy, Debug)]
struct QuoteAt {
    c: char,
    window: usize,
    /// The neighbours of its series allow a closing role (design section 6.3).
    closing: bool,
    /// It follows `s`, where the forward algorithm puts a trailing possessive.
    after_s: bool,
}

/// The candidates of the text in order of position, outside the protected spans.
fn candidates(
    reading: &Reading,
    text: &Flat<'_>,
    protected: &[(usize, usize)],
    boundaries: &[usize],
) -> Vec<Cand> {
    let mut out = Vec::new();
    let mut quotes: Vec<QuoteAt> = Vec::new();
    let mut span = 0usize;
    for (at, c) in text.text.char_indices() {
        while protected.get(span).is_some_and(|&(_, end)| end <= at) {
            span = span.saturating_add(1);
        }
        if protected
            .get(span)
            .is_some_and(|&(start, end)| start <= at && at < end)
        {
            continue;
        }
        let end = at.saturating_add(c.len_utf8());
        let alts: &'static [&'static str] = if c == APOSTROPHE || reading.is_glyph(c) {
            let left = text.before(at, |q| is_quote(reading, q));
            let right = text.after(end, |q| is_quote(reading, q));
            let closing = !matches!(class(left), Class::Space | Class::OpenP | Class::DashC)
                && matches!(class(right), Class::Space | Class::CloseP | Class::DashC);
            let after_s = matches!(text.before(at, |_| false), Near::Char('s' | 'S'));
            quotes.push(QuoteAt {
                c,
                window: boundaries.partition_point(|&b| b <= at),
                closing,
                after_s,
            });
            // The reading is decided below, with the pairs.
            &[]
        } else if c == '\u{2014}' {
            let left = blank(text.before(at, |_| false));
            let right = blank(text.after(end, |_| false));
            // `--` is the reading when only `dash.double-hyphen` produces the dash.
            if !left && !right {
                &[" - ", " -- ", "-"]
            } else {
                &["-", "--"]
            }
        } else if let Some(alts) = fixed(c) {
            alts
        } else if reading.minus == Some(c) {
            &["-"]
        } else {
            continue;
        };
        out.push(Cand {
            start: at,
            end,
            alts,
            alt: 0,
            on: true,
            fixed: false,
            window: boundaries.partition_point(|&b| b <= at),
            dash: c == '\u{2014}',
        });
    }
    let readings = quote_readings(reading, &quotes);
    let mut next = readings.iter();
    for cand in &mut out {
        if cand.alts.is_empty() {
            match next.next() {
                Some(&alts) => {
                    cand.alts = alts;
                    cand.fixed = true;
                }
                None => cand.on = false,
            }
        }
    }
    out.retain(|c| c.on && !c.alts.is_empty());
    out
}

/// The readings of the quote glyphs and apostrophes, in order: a glyph is `"`; a `’` closes the
/// innermost open pair whose closer it is, or is an apostrophe (design section 12.1).
fn quote_readings(reading: &Reading, quotes: &[QuoteAt]) -> Vec<&'static [&'static str]> {
    const QUOTE: &[&str] = &["\""];
    const CLOSER: &[&str] = &["\""];
    // An apostrophe is undone only with `quotes.apostrophe`; otherwise it has no reading.
    let apostrophe: &'static [&'static str] = if reading.apostrophe { &["'"] } else { &[] };
    let Some([open1, close1, open2, close2]) = reading.pairs else {
        return quotes.iter().map(|_| apostrophe).collect();
    };
    let mut out = Vec::with_capacity(quotes.len());
    // The open pairs: `false` for the primary pair, `true` for the nested one.
    let mut stack: Vec<bool> = Vec::new();
    let mut window = 0usize;
    for (k, q) in quotes.iter().enumerate() {
        if q.window != window {
            window = q.window;
            stack.clear();
        }
        let c = q.c;
        if c == APOSTROPHE && (c == close1 || c == close2) {
            let nested = c == close2;
            if q.closing && stack.last() == Some(&nested) {
                if q.after_s && later_closer(quotes, k, [open1, close1, open2, close2], nested) {
                    out.push(apostrophe);
                } else {
                    stack.pop();
                    out.push(CLOSER);
                }
            } else {
                out.push(apostrophe);
            }
            continue;
        }
        if c == APOSTROPHE && !reading.is_glyph(c) {
            out.push(apostrophe);
            continue;
        }
        if c == open1 || c == open2 {
            stack.push(c == open2 && c != open1);
        } else if c == close1 || c == close2 {
            let nested = c == close2 && c != close1;
            if let Some(level) = stack.iter().rposition(|&n| n == nested) {
                stack.truncate(level);
            }
        }
        out.push(QUOTE);
    }
    out
}

/// Whether a later `’` of the same depth, before the next opener of that depth and before the
/// closer of the enclosing pair, has a closing role and does not follow `s` (design section 12.1).
fn later_closer(quotes: &[QuoteAt], k: usize, glyphs: [char; 4], nested: bool) -> bool {
    let [open1, close1, open2, close2] = glyphs;
    let (own_open, own_close) = if nested {
        (open2, close2)
    } else {
        (open1, close1)
    };
    let Some(this) = quotes.get(k) else {
        return false;
    };
    let mut depth = 0usize;
    for q in quotes.iter().skip(k.saturating_add(1)) {
        if q.window != this.window {
            return false;
        }
        let c = q.c;
        let opens = c == open1 || c == open2;
        let closes = (c == close1 || c == close2) && (c != APOSTROPHE || q.closing);
        if depth == 0 {
            if c == own_close && c == APOSTROPHE {
                if q.closing && !q.after_s {
                    return true;
                }
                continue;
            }
            if c == own_open || closes {
                return false;
            }
            if opens {
                depth = depth.saturating_add(1);
            }
        } else if opens {
            depth = depth.saturating_add(1);
        } else if closes {
            depth = depth.saturating_sub(1);
        }
    }
    false
}
