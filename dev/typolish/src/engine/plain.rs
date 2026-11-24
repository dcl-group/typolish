//! The reverse transformation (design sections 2.10 and 12.1): typeset Unicode text back to the
//! canonical plain form of the typographer.
//!
//! The inverse tables of design section 12.1 give a reading (the plain text it came from) for every
//! typographic character outside the spans of `protected()`; a closed or spaced dash has a second
//! reading with `--`. A closed dash reads with spaces only between two letters of words, and then
//! `-` is its last reading (the range dash `—` of a language between Roman numerals); between
//! other non-blank neighbors its only reading is `-`, the range dash of a language whose range dash
//! is `—`. Quote glyphs and apostrophes are undone by the tables and the rule for `’`
//! alone. Every other character is undone only where the enabled rules produce it: the engine runs
//! forward on the text with the readings in place, and a reading that does not come back as its
//! character, or that makes a rule edit the text next to it, is put aside. Readings put aside are
//! retried in batches far apart, each in its other readings, and the whole pass repeats on its own
//! output until nothing changes. So "undoes exactly the enabled rules" is decided by the rules
//! themselves, with no second copy of their patterns.

use std::borrow::Cow;

use unicode_segmentation::UnicodeSegmentation;

use super::Input;
use super::ingest::Atom;
use super::lex::{is_dash_char, is_punct_char};
use super::record::{NoRecord, Recorder};
use super::render::{REdit, merge, write_text};
use super::scratch::Scratch;
use crate::report::{Change, Diagnostic, ProtectedKind, ProtectedSpan};
use crate::{Rule, SingleQuotes, Typographer};

/// How far apart, in bytes, two readings are tried in one forward run: farther than any rule
/// looks (the longest pattern, `nbsp.phone`, spans at most 24 tokens).
const REACH: usize = 256;

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
    /// Whether it is a quote glyph or `’`: read as a quote, it comes back as any glyph of the pairs
    /// or `’`, since the level decides the glyph and not the reading; read as an apostrophe, it
    /// comes back as `’` (`Cand::apostrophe`).
    quote: bool,
    /// For a quote glyph with the second reading `'`, when `settle_quotes` tries that reading: 1
    /// for a glyph of the nested pair, 2 for a glyph of the primary pair with
    /// `SingleQuotes::Everywhere`, 3 for a glyph of the primary pair inside a nested pair (the
    /// third level, `«` of `«„«x»“»`); 0 for any other candidate.
    tier: u8,
}

impl Cand {
    fn plain(&self) -> &'static str {
        self.alts.get(self.alt).copied().unwrap_or("")
    }

    /// Whether it is read as the closed form of a dash, with spaces restored around it.
    fn closed(&self) -> bool {
        self.dash && self.plain().starts_with(' ')
    }

    /// Whether it is a `’` read as an apostrophe (design section 12.1): unlike a quote glyph, it
    /// must come back as itself.
    fn apostrophe(&self) -> bool {
        self.quote && self.alts.first() == Some(&"'")
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
                (Input::Text(s) | Input::Protected(s) | Input::Markup(s) | Input::Block(s), _) => {
                    out.push_str(s)
                }
                (Input::Slot(_), _) => {}
            }
        }
        texts.push(Cow::Owned(out));
    }
    (texts, changed)
}

/// The plain text of every input piece: `Some` for a `Text` piece that changed. The pass repeats
/// on its own output until it changes nothing (design section 13.5, property 16).
fn plain_texts(
    typographer: &Typographer,
    scratch: &mut Scratch,
    input: &[Input<'_>],
) -> Vec<Option<String>> {
    passes(typographer, scratch, input, None)
}

/// Every character of the input that `to_plain` changes, as its piece, its byte range in that
/// piece and the character: what `lint` reports as `TypographicChar` (design section 12.3). It is
/// read from the passes of `to_plain` themselves, so the two cannot disagree (design section
/// 13.5, property 15).
pub(crate) fn changed_chars(
    typographer: &Typographer,
    scratch: &mut Scratch,
    input: &[Input<'_>],
) -> Vec<(usize, core::ops::Range<usize>, char)> {
    let mut found = Vec::new();
    passes(typographer, scratch, input, Some(&mut found));
    found.sort_unstable_by_key(|(piece, range, _)| (*piece, range.start));
    found
}

/// The passes of `to_plain` until nothing changes. With `found`, it also collects every
/// character of the input that a pass undid, at its place in the input piece.
fn passes(
    typographer: &Typographer,
    scratch: &mut Scratch,
    input: &[Input<'_>],
    mut found: Option<&mut Vec<(usize, core::ops::Range<usize>, char)>>,
) -> Vec<Option<String>> {
    let mut owned: Vec<Option<String>> = vec![None; input.len()];
    // For a piece that changed, the input byte of every byte of its current text (`usize::MAX`
    // for a byte that a reading wrote); kept only when `found` asks for it.
    let mut origins: Vec<Option<Vec<usize>>> = vec![None; input.len()];
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
        // text, so the loop ends. On the inputs measured, ordinary prose and long runs of quotes
        // in one paragraph alike, it ends after one or two rounds (design section 12.1).
        let Some((changed, undone)) = round(typographer, scratch, &current) else {
            break;
        };
        if let Some(found) = found.as_deref_mut() {
            for u in &undone {
                let Some(Input::Text(now)) = current.get(u.piece) else {
                    continue;
                };
                let origin = origins.get(u.piece).and_then(Option::as_ref);
                let at = |pos: usize| match origin {
                    Some(map) => map.get(pos).copied().unwrap_or(usize::MAX),
                    None => pos,
                };
                // A candidate is a non-ASCII character and a reading is ASCII text, so the bytes
                // of a candidate always come from the input.
                let (from, last) = (at(u.start), at(u.end.saturating_sub(1)));
                let c = now.get(u.start..u.end).and_then(|s| s.chars().next());
                if let Some(c) = c
                    && from != usize::MAX
                    && last != usize::MAX
                {
                    found.push((u.piece, from..last.saturating_add(1), c));
                }
            }
            for &(index, _) in &changed {
                let Some(Input::Text(now)) = current.get(index) else {
                    continue;
                };
                // `undone` is ordered by piece (see `round`).
                let lo = undone.partition_point(|u| u.piece < index);
                let hi = undone.partition_point(|u| u.piece <= index);
                if let Some(slot) = origins.get_mut(index) {
                    let old = slot.take();
                    *slot = Some(remap(now, old, undone.get(lo..hi).unwrap_or(&[])));
                }
            }
        }
        drop(current);
        for (index, text) in changed {
            if let Some(slot) = owned.get_mut(index) {
                *slot = Some(text);
            }
        }
    }
    owned
}

/// The input byte of every byte of the text of a piece after the candidates `undone` of `now`,
/// all of that piece and in order, were replaced by their readings (`usize::MAX` for a byte of a
/// reading); `old` maps the bytes of `now`, and no map means that `now` is the input piece itself.
fn remap(now: &str, old: Option<Vec<usize>>, undone: &[Undone]) -> Vec<usize> {
    let of = |pos: usize| match &old {
        Some(map) => map.get(pos).copied().unwrap_or(usize::MAX),
        None => pos,
    };
    let mut map = Vec::with_capacity(now.len());
    let mut pos = 0usize;
    for u in undone {
        map.extend((pos..u.start).map(of));
        map.extend(core::iter::repeat_n(usize::MAX, u.written));
        pos = u.end;
    }
    map.extend((pos..now.len()).map(of));
    map
}

/// A candidate that a pass undid: its piece, its byte range in the piece's text of that pass, and
/// the length of the reading written in its place.
#[derive(Clone, Copy, Debug)]
struct Undone {
    piece: usize,
    start: usize,
    end: usize,
    written: usize,
}

/// The end of the chain of links from `n`: a link that leads to itself ends it. The links on
/// the way are shortened, so that a chain is walked about once.
fn find(links: &mut [usize], mut n: usize) -> usize {
    while let Some(&next) = links.get(n) {
        if next == n {
            break;
        }
        let after = links.get(next).copied().unwrap_or(next);
        if let Some(l) = links.get_mut(n) {
            *l = after;
        }
        n = next;
    }
    n
}

/// The trial nearest to the range `start..end` in `window`, as its index in `trial` and its
/// candidate: the distance is the gap between the two ranges, and of two at the same distance the
/// first. `trial` is in text order, and so are its windows, so the nearest one is next to where
/// the range falls among them.
fn nearest_trial<'c>(
    cands: &'c [Cand],
    trial: &[usize],
    window: usize,
    start: usize,
    end: usize,
) -> Option<(usize, &'c Cand)> {
    let at = |n: usize| trial.get(n).and_then(|&k| cands.get(k));
    let lo = trial.partition_point(|&k| cands.get(k).is_some_and(|c| c.window < window));
    let hi = trial.partition_point(|&k| cands.get(k).is_some_and(|c| c.window <= window));
    let span = trial.get(lo..hi)?;
    // Before `p` a trial starts before `end`: its gap shrinks as its end grows, down to 0 from
    // the first one that reaches `start`. From `p` on, the gap grows with the start.
    let p = span.partition_point(|&k| cands.get(k).is_some_and(|c| c.start < end));
    let z = span
        .get(..p)?
        .partition_point(|&k| cands.get(k).is_some_and(|c| c.end < start));
    let left = if z < p {
        Some((lo + z, 0))
    } else {
        p.checked_sub(1)
            .and_then(|n| Some((lo + n, start.saturating_sub(at(lo + n)?.end))))
    };
    let right = (p < span.len())
        .then(|| Some((lo + p, at(lo + p)?.start.saturating_sub(end))))
        .flatten();
    let (n, _) = match (left, right) {
        (Some(l), Some(r)) => {
            if r.1 < l.1 {
                r
            } else {
                l
            }
        }
        (l, r) => l.or(r)?,
    };
    Some((n, at(n)?))
}

/// The result of one pass: the new text of every `Text` piece that changed, by input index, and
/// the candidates it undid, in order.
type Pass = (Vec<(usize, String)>, Vec<Undone>);

/// One pass: the candidates of the text, checked against the forward rules. Returns `None` when
/// nothing changed.
fn round(typographer: &Typographer, scratch: &mut Scratch, input: &[Input<'_>]) -> Option<Pass> {
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
    let mut stops: Vec<(usize, Near)> = Vec::new();
    let mut boundaries: Vec<usize> = Vec::new();
    for atom in &scratch.atoms {
        match *atom {
            Atom::Slot { at, .. } | Atom::Protected { at, .. } => {
                stops.push((at as usize, Near::Wall));
            }
            Atom::Boundary { start, end } => {
                boundaries.push(start as usize);
                // A `BlockMarkup` piece: a boundary without characters, which no neighbor is
                // read across.
                if start == end {
                    stops.push((start as usize, Near::Edge));
                }
            }
            _ => {}
        }
    }
    // The atoms come in the order of the input; a stable sort keeps that order among the stops at
    // one position, which decides what stands next to the text on either side of them.
    stops.sort_by_key(|&(at, _)| at);
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
        stops: &stops,
        frozen: &protected,
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
        reading: settings,
    };
    settle_quotes(&check, scratch, &mut cands);
    // First the candidates that do not come back are put aside, until all that are left come
    // back; one that failed only because of a neighbor's wrong reading is then tried again, one
    // at a time in each window (windows are independent, design section 3.3), in each of its
    // readings. A `’` read as an apostrophe that does not come back as `’` keeps its character
    // for good: its `'` would pair as a quote (`“a ‘dogs’ bowl ’twas’ b”`, whose possessive
    // closes the nested pair, read as `"a "dogs" bowl 'twas' b"` gives `‘twas’`), and it has no
    // other reading.
    let mut parked = vec![false; cands.len()];
    let mut verdict = loop {
        let verdict = check.run(scratch, &cands);
        let mut failed = false;
        for (k, cand) in cands.iter_mut().enumerate() {
            if !cand.on || !verdict.failed.get(k).copied().unwrap_or(true) {
                continue;
            }
            if !cand.fixed {
                cand.on = false;
                if let Some(p) = parked.get_mut(k) {
                    *p = true;
                }
                failed = true;
            } else if cand.apostrophe() {
                cand.on = false;
                failed = true;
            }
        }
        if failed {
            continue;
        }
        // An edit outside every candidate that the text as it was does not get: the plain text
        // of a neighboring candidate made a rule fire, so it is not the plain text the character
        // came from (the round trip of design section 12.2 would break). The nearest candidates on
        // both sides are put aside; one that was innocent comes back below. The spaces of a closed
        // dash are exempt: the closed form joins two words into one fragment, which by design
        // changes the rules around it, and `to_plain` restores them (design section 8.4).
        let mut blamed = false;
        // The candidates that can be blamed, in text order (and so in window order): the nearest
        // on each side of an edit is next to where the edit falls among them. A blamed one is
        // off, so later edits skip it; the skips are links that jump over the blamed ones.
        let suspects: Vec<usize> = cands
            .iter()
            .enumerate()
            .filter(|(_, c)| c.on && !c.fixed && !c.closed())
            .map(|(k, _)| k)
            .collect();
        // `to_left[n]` leads to the nearest suspect at or before position `n - 1` (0: none);
        // `to_right[n]` to the nearest at or after `n` (`len`: none).
        let mut to_left: Vec<usize> = (0..=suspects.len()).collect();
        let mut to_right: Vec<usize> = (0..=suspects.len()).collect();
        for &(window, ys, yt) in &verdict.stray {
            let in_window = |n: &usize| {
                suspects
                    .get(*n)
                    .and_then(|&k| cands.get(k))
                    .is_some_and(|c| c.window == window)
            };
            let at = suspects.partition_point(|&k| cands.get(k).is_some_and(|c| c.end <= ys));
            let left = find(&mut to_left, at).checked_sub(1).filter(in_window);
            let from = suspects.partition_point(|&k| cands.get(k).is_some_and(|c| c.start < yt));
            let right = Some(find(&mut to_right, from)).filter(in_window);
            for n in left.into_iter().chain(right) {
                if let Some(l) = to_left.get_mut(n.saturating_add(1)) {
                    *l = n;
                }
                if let Some(r) = to_right.get_mut(n) {
                    *r = n.saturating_add(1);
                }
                let Some(&k) = suspects.get(n) else {
                    continue;
                };
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
    // The candidates put aside, in order. One whose context did not change (no other candidate
    // was put aside near it) and that has no other reading would fail again: it is not retried.
    let mut pending: Vec<usize> = parked
        .iter()
        .enumerate()
        .filter_map(|(k, &p)| p.then_some(k))
        .collect();
    // `pending` is in text order and the candidates do not overlap, so a candidate put aside
    // within reach of this one is next to it in `pending`, or none is.
    let near = |c: &Cand, j: Option<&usize>| {
        j.and_then(|&j| cands.get(j)).is_some_and(|d| {
            d.window == c.window
                && d.start < c.end.saturating_add(REACH)
                && c.start < d.end.saturating_add(REACH)
        })
    };
    let retried: Vec<usize> = pending
        .iter()
        .enumerate()
        .filter(|&(n, &k)| {
            cands.get(k).is_some_and(|c| {
                c.alts.len() > 1
                    || near(c, n.checked_sub(1).and_then(|m| pending.get(m)))
                    || near(c, pending.get(n.saturating_add(1)))
            })
        })
        .map(|(_, &k)| k)
        .collect();
    pending = retried;
    for &k in &pending {
        if let Some(cand) = cands.get_mut(k) {
            cand.alt = 0;
        }
    }
    let tried = !pending.is_empty();
    let mut gone = vec![false; cands.len()];
    while !pending.is_empty() {
        // Trials far enough apart that the rules see them separately (their contexts are a few
        // tokens wide): each problem of the run is laid at the nearest trial.
        let mut trial: Vec<usize> = Vec::new();
        for &k in &pending {
            let Some(c) = cands.get(k) else {
                continue;
            };
            let apart = trial
                .last()
                .and_then(|&t| cands.get(t))
                .is_none_or(|t| t.window != c.window || c.start >= t.end.saturating_add(REACH));
            if apart {
                trial.push(k);
            }
        }
        for &k in &trial {
            if let Some(cand) = cands.get_mut(k) {
                cand.on = true;
            }
        }
        let next = check.run(scratch, &cands);
        let mut rejected = vec![false; trial.len()];
        let mut blame = |window: usize, start: usize, end: usize, stray: bool| {
            let nearest = nearest_trial(&cands, &trial, window, start, end);
            if let Some((n, c)) = nearest
                && !(stray && c.closed())
                && let Some(r) = rejected.get_mut(n)
            {
                *r = true;
            }
        };
        for (cand, &failed) in cands.iter().zip(&next.failed) {
            if cand.on && !cand.fixed && failed {
                blame(cand.window, cand.start, cand.end, false);
            }
        }
        let mut known = verdict.stray.clone();
        known.sort_unstable();
        for s in &next.stray {
            if known.binary_search(s).is_err() {
                blame(s.0, s.1, s.2, true);
            }
        }
        for (n, &k) in trial.iter().enumerate() {
            let held = !rejected.get(n).copied().unwrap_or(true);
            let Some(cand) = cands.get_mut(k) else {
                continue;
            };
            let done = if held {
                true
            } else {
                cand.on = false;
                cand.alt = cand.alt.saturating_add(1);
                cand.alt >= cand.alts.len()
            };
            if done && let Some(g) = gone.get_mut(k) {
                *g = true;
            }
        }
        pending.retain(|&j| !gone.get(j).copied().unwrap_or(true));
        // The strays of the new state: those of the run, less those of the rejected trials,
        // which are off again.
        // The rejected trials, in text order: the one that can reach a stray is the first whose
        // end reaches it.
        let off: Vec<&Cand> = trial
            .iter()
            .zip(&rejected)
            .filter(|&(_, &r)| r)
            .filter_map(|(&k, _)| cands.get(k))
            .collect();
        verdict.stray = next
            .stray
            .iter()
            .filter(|s| {
                known.binary_search(s).is_ok() || {
                    let q = off.partition_point(|c| {
                        c.window < s.0 || (c.window == s.0 && c.end.saturating_add(REACH) <= s.1)
                    });
                    !off.get(q).is_some_and(|c| {
                        c.window == s.0
                            && s.1 < c.end.saturating_add(REACH)
                            && c.start < s.2.saturating_add(REACH)
                    })
                }
            })
            .copied()
            .collect();
    }
    // A last check: a candidate that a later trial broke after all is kept.
    let mut again = tried;
    while again {
        let last = check.run(scratch, &cands);
        again = false;
        for (cand, &f) in cands.iter_mut().zip(&last.failed) {
            if cand.on && (!cand.fixed || cand.apostrophe()) && f {
                cand.on = false;
                again = true;
            }
        }
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
    // The candidates and the pieces are both in text order, so one cursor finds the piece of
    // every candidate and `undone` comes out ordered by piece.
    let mut cursor = 0usize;
    let undone: Vec<Undone> = cands
        .iter()
        .filter(|c| c.on)
        .filter_map(|c| {
            while pieces
                .get(cursor)
                .is_some_and(|p| p.start.saturating_add(p.text.len()) <= c.start)
            {
                cursor = cursor.saturating_add(1);
            }
            let piece = pieces.get(cursor).filter(|p| p.start <= c.start)?;
            Some(Undone {
                piece: piece.index,
                start: c.start.saturating_sub(piece.start),
                end: c.end.saturating_sub(piece.start),
                written: c.plain().len(),
            })
        })
        .collect();
    (!changed.is_empty()).then_some((changed, undone))
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
    reading: Reading,
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
                *f = !ok
                    || if cand.apostrophe() {
                        segment != original
                    } else if cand.quote {
                        !self.reading.is_quote_glyph(&segment)
                    } else {
                        !same(cand.dash, &segment, original)
                    };
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

/// The readings of the quote glyphs, checked before the other candidates (they are context for
/// the rules that check those): the engine runs forward with the quote readings in place and the
/// other candidates as they are, and in a window where a quote reading does not come back as a
/// glyph (the double quotes of `«x „“.»` read as `"x "".` do not pair again), the glyphs of the
/// nested pair read as `'` instead, if that brings every quote of the window back (design section
/// 12.1). In a window that this does not bring back, the glyphs of the nested pair and, with
/// `SingleQuotes::Everywhere`, those of the primary pair read as `'` (`“3" 0”` as `'3" 0'`); in a
/// window that neither brings back, the glyphs of the primary pair inside a nested pair read as `'`
/// alone (`«„«6»“»` as `""'6'""`). Each try is kept only where it brings the window back.
fn settle_quotes(check: &Check<'_, '_>, scratch: &mut Scratch, cands: &mut [Cand]) {
    let alternative = |c: &Cand| c.fixed && c.on && c.alts.len() > 1;
    if !cands.iter().any(alternative) {
        return;
    }
    let on: Vec<bool> = cands.iter().map(|c| c.on).collect();
    for cand in cands.iter_mut() {
        if !cand.fixed {
            cand.on = false;
        }
    }
    // Per window, flags rather than lists of windows: every lookup below is constant, so the
    // bookkeeping stays linear however many windows stay broken (design section 12.1). The
    // candidates are in text order, so the last one has the highest window.
    let count = cands.last().map_or(0, |c| c.window.saturating_add(1));
    let is_set = |flags: &[bool], w: usize| flags.get(w).copied().unwrap_or(false);
    let broken = |cands: &[Cand], verdict: &Verdict| -> Vec<bool> {
        let mut windows = vec![false; count];
        for (c, &f) in cands.iter().zip(&verdict.failed) {
            if c.on
                && c.quote
                && c.alts.first() == Some(&"\"")
                && f
                && let Some(w) = windows.get_mut(c.window)
            {
                *w = true;
            }
        }
        windows
    };
    let first = check.run(scratch, cands);
    let mut still = broken(cands, &first);
    // The tiers of `Cand::tier`, tried in turn in the windows that are still broken and hold a
    // glyph of the last tier of the try (a try without it would repeat the one before).
    for (tiers, last) in [(&[1u8][..], 1u8), (&[1, 2], 2), (&[3], 3)] {
        let take = |c: &Cand| alternative(c) && tiers.contains(&c.tier);
        let mut windows = vec![false; count];
        let mut any = false;
        for c in cands.iter() {
            if alternative(c)
                && c.tier == last
                && is_set(&still, c.window)
                && let Some(w) = windows.get_mut(c.window)
            {
                *w = true;
                any = true;
            }
        }
        if !any {
            continue;
        }
        for cand in cands.iter_mut() {
            if cand.fixed && is_set(&windows, cand.window) {
                cand.alt = usize::from(take(cand));
            }
        }
        let verdict = check.run(scratch, cands);
        let now = broken(cands, &verdict);
        for cand in cands.iter_mut() {
            if cand.fixed && is_set(&windows, cand.window) && is_set(&now, cand.window) {
                cand.alt = 0;
            }
        }
        for ((s, &tried), &bad) in still.iter_mut().zip(&windows).zip(&now) {
            if tried {
                *s = bad;
            }
        }
    }
    for (cand, was) in cands.iter_mut().zip(on) {
        cand.on = was;
    }
}

/// Whether the forward run gave the character of a candidate back. Glue of either width counts
/// as glue (design section 12.1: glue, U+00A0 or U+202F, set by a rule becomes a space); for a dash
/// the spaces around it do not count, since the dash style of the typographer decides them;
/// `candidates` gives the reading with spaces only between two letters.
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
    /// Whether single quotes are paired at every level (`SingleQuotes::Everywhere`), so that a
    /// glyph of the primary pair may come from `'` too.
    everywhere: bool,
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
        let everywhere = typographer.single_quotes() == SingleQuotes::Everywhere;
        Reading {
            pairs,
            minus,
            apostrophe,
            everywhere,
        }
    }

    fn is_glyph(&self, c: char) -> bool {
        self.pairs.is_some_and(|g| g.contains(&c))
    }

    /// Whether `s` is one glyph of the pairs or `’`: what a quote reading comes back as, at
    /// whatever level.
    fn is_quote_glyph(&self, s: &str) -> bool {
        let mut chars = s.chars();
        matches!((chars.next(), chars.next()), (Some(c), None) if c == APOSTROPHE || self.is_glyph(c))
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

/// The call's text with the pieces that have no text in it, and the protected spans of the text,
/// in order.
#[derive(Clone, Copy, Debug)]
struct Flat<'s> {
    text: &'s str,
    /// The pieces without text that stop a walk to a neighbor, by position and in the order of the
    /// input among those at one position: a slot or a protected piece is `Near::Wall`; a
    /// `BlockMarkup` piece is `Near::Edge`, since a paragraph ends there and a neighbor read across
    /// it is the edge of the text, as for the forward rules (design section 3.3).
    stops: &'s [(usize, Near)],
    frozen: &'s [(usize, usize)],
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

/// Format characters, ZWNJ and ZWJ, which are transparent to the context (design section 5.4):
/// the same set as the lexer's.
fn is_format(c: char) -> bool {
    super::lex::is_format_char(c) || super::lex::is_joiner(c)
}

impl Flat<'_> {
    /// The last of the stops at `at`, the one next to the character that starts there.
    fn stop_before(&self, at: usize) -> Option<Near> {
        let end = self.stops.partition_point(|&(p, _)| p <= at);
        let (p, near) = *self.stops.get(end.checked_sub(1)?)?;
        (p == at).then_some(near)
    }

    /// The first of the stops at `at`, the one next to the character that ends there.
    fn stop_after(&self, at: usize) -> Option<Near> {
        let start = self.stops.partition_point(|&(p, _)| p < at);
        let &(p, near) = self.stops.get(start)?;
        (p == at).then_some(near)
    }

    /// What stands before `at`, skipping format characters and the characters `skip` accepts.
    fn before(&self, at: usize, skip: impl Fn(char) -> bool) -> Near {
        self.before_at(at, skip).0
    }

    /// `before`, with the start of the character found.
    fn before_at(&self, mut at: usize, skip: impl Fn(char) -> bool) -> (Near, usize) {
        loop {
            if let Some(near) = self.stop_before(at) {
                return (near, at);
            }
            let Some(c) = self.text.get(..at).and_then(|s| s.chars().next_back()) else {
                return (Near::Edge, at);
            };
            at = at.saturating_sub(c.len_utf8());
            if is_format(c) || skip(c) {
                continue;
            }
            return (Near::Char(c), at);
        }
    }

    /// `after`, with the start of the character found.
    fn after_at(&self, mut at: usize, skip: impl Fn(char) -> bool) -> (Near, usize) {
        loop {
            if let Some(near) = self.stop_after(at) {
                return (near, at);
            }
            let Some(c) = self.text.get(at..).and_then(|s| s.chars().next()) else {
                return (Near::Edge, at);
            };
            if is_format(c) || skip(c) {
                at = at.saturating_add(c.len_utf8());
                continue;
            }
            return (Near::Char(c), at);
        }
    }

    /// Whether the character at `at` lies in a protected span.
    fn is_frozen(&self, at: usize) -> bool {
        let next = self.frozen.partition_point(|&(start, _)| start <= at);
        next.checked_sub(1)
            .and_then(|k| self.frozen.get(k))
            .is_some_and(|&(_, end)| at < end)
    }

    /// The class of the left neighbor of the series of quotes that ends at `at` (design section
    /// 6.3), and where the walk to it stopped. A character inside a protected span is of class
    /// `Word`, as a neighbor inside a frozen core or a protected span is in the forward algorithm
    /// (design sections 6.3 and 6.9).
    fn class_before_at(&self, reading: &Reading, at: usize) -> (Class, usize) {
        match self.before_at(at, |q| is_quote(reading, q)) {
            (Near::Char(_), pos) if self.is_frozen(pos) => (Class::Word, pos),
            (near, pos) => (class(near), pos),
        }
    }

    /// The class of the right neighbor of the series of quotes that starts at `at`, and where the
    /// walk to it stopped.
    fn class_after_at(&self, reading: &Reading, at: usize) -> (Class, usize) {
        match self.after_at(at, |q| is_quote(reading, q)) {
            (Near::Char(_), pos) if self.is_frozen(pos) => (Class::Word, pos),
            (near, pos) => (class(near), pos),
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

/// The classes of the neighbors of a quote (design section 6.3).
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
    /// The neighbors of its series allow a closing role (design section 6.3).
    closing: bool,
    /// It follows `s` or `S`, where the forward algorithm puts a trailing possessive.
    after_s: bool,
    /// It is a `’` that closes the nested opener before it as the second quote of a sign-only
    /// pair would (design sections 6.3 and 12.1).
    sign_only: bool,
}

/// A sign between the quotes of a sign-only pair (design section 6.3): punctuation, dots, dashes,
/// operators, brackets and the times sign.
fn is_sign(c: char) -> bool {
    matches!(
        c,
        '-' | '.' | '\u{2026}' | '\u{d7}' | '(' | ')' | '[' | ']' | '{' | '}' | '\u{2190}'
            ..='\u{2194}' | '\u{2260}' | '\u{2264}' | '\u{2265}' | '\u{b1}'
    ) || is_dash_char(c)
        || is_punct_char(c)
}

/// Whether the `’` at `at..end` is the mirror of the second quote of a sign-only pair (design
/// section 6.3): the nearest character before it that is not a sign or a format character is the
/// opener of the pair whose closer is `’` (nested or primary), whose series has L of class
/// `Space`, `OpenP` or `DashC`, and two adjacent quotes are not part of a series of three or more.
/// R of the `’` is the caller's to check. Plain `""`, `"-"` or `"?"` inside a pair is typeset as
/// such a nested pair, whatever the roles of its series.
fn sign_only_close(
    reading: &Reading,
    text: &Flat<'_>,
    at: usize,
    series: &mut Option<Series>,
) -> bool {
    let Some([open1, close1, open2, _]) = reading.pairs else {
        return false;
    };
    if open2 == open1 {
        return false;
    }
    // The opener of the pair whose closer is `’`: the nested one in the built-in languages, the
    // primary one in a language whose primary pair is `‘’` (en-GB).
    let open = if close1 == APOSTROPHE { open1 } else { open2 };
    let mut p = at;
    let mut adjacent = true;
    loop {
        if text.stop_before(p).is_some() {
            return false;
        }
        let Some(c) = text.text.get(..p).and_then(|s| s.chars().next_back()) else {
            return false;
        };
        p = p.saturating_sub(c.len_utf8());
        if is_format(c) {
            continue;
        }
        if is_sign(c) {
            adjacent = false;
            continue;
        }
        if c != open {
            return false;
        }
        break;
    }
    // Two adjacent quotes are in the series of the `’`, whose neighbors and count are found once
    // for the series; an opener before signs is in a series of its own.
    let left = match series.as_mut() {
        Some(s) if adjacent => {
            if s.count(reading, text) >= 3 {
                return false;
            }
            s.left
        }
        _ => text.class_before_at(reading, p).0,
    };
    matches!(left, Class::Space | Class::OpenP | Class::DashC)
}

/// A series of quotes (design section 6.3): the classes of its neighbors and where they stand,
/// and the number of its quotes once it is asked for.
#[derive(Clone, Copy, Debug)]
struct Series {
    /// Where the walk to the left neighbor stopped: the start of that character, or a wall or the
    /// edge of the text.
    left_at: usize,
    /// Where the walk to the right neighbor stopped.
    right_at: usize,
    left: Class,
    right: Class,
    /// The number of quotes in the series, format characters skipped.
    count: Option<usize>,
}

impl Series {
    /// The series of the quote at `at..end`.
    fn of(reading: &Reading, text: &Flat<'_>, at: usize, end: usize) -> Series {
        let (left, left_at) = text.class_before_at(reading, at);
        let (right, right_at) = text.class_after_at(reading, end);
        Series {
            left_at,
            right_at,
            left,
            right,
            count: None,
        }
    }

    /// Whether the quote that starts at `at` belongs to this series: every position between the
    /// two neighbors holds a quote or a format character and no wall, so a walk from such a quote
    /// stops where the walks of the series stopped.
    fn holds(&self, at: usize) -> bool {
        self.left_at < at && at < self.right_at
    }

    /// The number of quotes in the series.
    fn count(&mut self, reading: &Reading, text: &Flat<'_>) -> usize {
        if let Some(n) = self.count {
            return n;
        }
        let n = text
            .text
            .get(self.left_at..self.right_at)
            .map_or(0, |s| s.chars().filter(|&q| is_quote(reading, q)).count());
        self.count = Some(n);
        n
    }
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
    // The series of the last quote met: every quote of a series has its neighbors, so they are
    // found once per series and not once per quote, which would be quadratic in a long series
    // (design section 12.1).
    let mut series: Option<Series> = None;
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
            let s = match series {
                Some(s) if s.holds(at) => s,
                _ => Series::of(reading, text, at, end),
            };
            series = Some(s);
            let (left, right) = (s.left, s.right);
            let closing = !matches!(left, Class::Space | Class::OpenP | Class::DashC)
                && matches!(right, Class::Space | Class::CloseP | Class::DashC);
            let after_s = matches!(text.before(at, |_| false), Near::Char('s' | 'S'));
            let sign_only = c == APOSTROPHE
                && matches!(right, Class::Space | Class::CloseP | Class::DashC)
                && sign_only_close(reading, text, at, &mut series);
            quotes.push(QuoteAt {
                c,
                window: boundaries.partition_point(|&b| b <= at),
                closing,
                after_s,
                sign_only,
            });
            // The reading is decided below, with the pairs.
            &[]
        } else if c == '\u{2014}' {
            let (before, left_at) = text.before_at(at, |_| false);
            let (after, right_at) = text.after_at(end, |_| false);
            // A letter of a word (design section 8.4): the first character of its cluster, so
            // that a letter with a combining mark counts, outside the protected spans.
            let letter_before = match before {
                Near::Char(c) => {
                    !text.is_frozen(left_at)
                        && text
                            .text
                            .get(..left_at.saturating_add(c.len_utf8()))
                            .and_then(|s| s.graphemes(true).next_back())
                            .and_then(|g| g.chars().next())
                            .is_some_and(char::is_alphabetic)
                }
                Near::Edge | Near::Wall => false,
            };
            let letter_after =
                matches!(after, Near::Char(c) if c.is_alphabetic()) && !text.is_frozen(right_at);
            // `--` is the reading when only `dash.double-hyphen` produces the dash. Between two
            // letters the last reading is `-`: the range dash of a language whose range dash is
            // `—` stands between Roman numerals (design section 8.5).
            if letter_before && letter_after {
                &[" - ", " -- ", "-"]
            } else if !blank(before) && !blank(after) {
                // The range dash of a language whose range dash is `—` (design section 8.5).
                &["-"]
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
            quote: c == APOSTROPHE || reading.is_glyph(c),
            tier: 0,
        });
    }
    let readings = quote_readings(reading, &quotes);
    let mut next = readings.iter();
    for cand in &mut out {
        if cand.alts.is_empty() {
            match next.next() {
                Some(&(alts, tier)) => {
                    cand.alts = alts;
                    cand.fixed = true;
                    cand.tier = tier;
                }
                None => cand.on = false,
            }
        }
    }
    out.retain(|c| c.on && !c.alts.is_empty());
    out
}

/// The open pairs of `quote_readings`, as their kinds (`false` for the primary pair, `true` for
/// the nested one), with the places of the entries of each kind: the innermost pair of a kind and
/// whether a nested pair is open below a place are found in constant time, so the readings stay
/// linear however deep the pairs are (design section 12.1).
#[derive(Debug, Default)]
struct Opened {
    kinds: Vec<bool>,
    /// The places in `kinds` of the entries of the primary pair and of the nested pair.
    of: [Vec<usize>; 2],
}

impl Opened {
    fn len(&self) -> usize {
        self.kinds.len()
    }

    fn last(&self) -> Option<bool> {
        self.kinds.last().copied()
    }

    fn push(&mut self, nested: bool) {
        if let Some(places) = self.of.get_mut(usize::from(nested)) {
            places.push(self.kinds.len());
        }
        self.kinds.push(nested);
    }

    /// Removes the entries from `level` on.
    fn truncate(&mut self, level: usize) {
        self.kinds.truncate(level);
        for places in &mut self.of {
            while places.last().is_some_and(|&p| p >= level) {
                places.pop();
            }
        }
    }

    fn pop(&mut self) {
        self.truncate(self.len().saturating_sub(1));
    }

    /// The place of the innermost entry of a kind.
    fn innermost(&self, nested: bool) -> Option<usize> {
        self.of
            .get(usize::from(nested))
            .and_then(|places| places.last().copied())
    }

    /// Whether a nested pair is open below `level`.
    fn nested_below(&self, level: usize) -> bool {
        self.of
            .get(1)
            .and_then(|places| places.first())
            .is_some_and(|&p| p < level)
    }

    fn clear(&mut self) {
        self.truncate(0);
    }
}

/// The readings of the quote glyphs and apostrophes, in order: a glyph is `"`; a `’` closes the
/// innermost open pair whose closer it is, or is an apostrophe (design section 12.1). A glyph of
/// the nested pair has the second reading `'`, which `settle_quotes` takes where `"` does not come
/// back; so has a glyph of the primary pair inside a nested pair or with
/// `SingleQuotes::Everywhere`. Each reading comes with its tier (`Cand::tier`).
fn quote_readings(reading: &Reading, quotes: &[QuoteAt]) -> Vec<(&'static [&'static str], u8)> {
    const QUOTE: &[&str] = &["\""];
    const NESTED: &[&str] = &["\"", "'"];
    // An apostrophe is undone only with `quotes.apostrophe`; otherwise it has no reading.
    let apostrophe: &'static [&'static str] = if reading.apostrophe { &["'"] } else { &[] };
    let Some([open1, close1, open2, close2]) = reading.pairs else {
        return quotes.iter().map(|_| (apostrophe, 0)).collect();
    };
    // The readings of a glyph of the pairs and their tier: `nested` for the nested pair, `deep`
    // for the primary pair inside a nested one.
    let glyph = |nested: bool, deep: bool| -> (&'static [&'static str], u8) {
        if nested {
            (NESTED, 1)
        } else if deep {
            (NESTED, 3)
        } else if reading.everywhere {
            (NESTED, 2)
        } else {
            (QUOTE, 0)
        }
    };
    let glyphs = [open1, close1, open2, close2];
    // `’` is the closer of the nested pair in the built-in languages, of the primary one in a
    // language whose primary pair is `‘’` (en-GB).
    let later = later_closers(quotes, glyphs, APOSTROPHE == close2);
    let mut out = Vec::with_capacity(quotes.len());
    let mut stack = Opened::default();
    let mut window = 0usize;
    for (k, q) in quotes.iter().enumerate() {
        if q.window != window {
            window = q.window;
            stack.clear();
        }
        let c = q.c;
        if c == APOSTROPHE && (c == close1 || c == close2) {
            let nested = c == close2;
            let deep = stack.nested_below(stack.len().saturating_sub(1));
            let closer = glyph(nested, deep);
            if q.sign_only && stack.last() == Some(nested) {
                // The mirror of a sign-only pair (design section 6.3): `‘’`, `‘-’`.
                stack.pop();
                out.push(closer);
            } else if q.closing && stack.last() == Some(nested) {
                if q.after_s && later.get(k).copied().unwrap_or(false) {
                    out.push((apostrophe, 0));
                } else {
                    stack.pop();
                    out.push(closer);
                }
            } else {
                out.push((apostrophe, 0));
            }
            continue;
        }
        if c == APOSTROPHE && !reading.is_glyph(c) {
            out.push((apostrophe, 0));
            continue;
        }
        let (nested, deep) = if c == open1 || c == open2 {
            let nested = c == open2 && c != open1;
            let deep = stack.nested_below(stack.len());
            stack.push(nested);
            (nested, deep)
        } else if c == close1 || c == close2 {
            let nested = c == close2 && c != close1;
            let mut deep = stack.nested_below(stack.len());
            if let Some(level) = stack.innermost(nested) {
                deep = stack.nested_below(level);
                stack.truncate(level);
            }
            (nested, deep)
        } else {
            (false, false)
        };
        out.push(glyph(nested, deep));
    }
    out
}

/// For every `’` after `s` or `S` with a closing role: whether a later `’` of the same depth,
/// before the next opener of that depth and before the closer of the enclosing pair, has a closing
/// role and does not follow `s` or `S` (design section 12.1). The pairs opened after the `’` are
/// followed by their kind, as in `quote_readings`: a `’` inside a deeper primary pair closes
/// nothing there and leaves the scan where it is.
///
/// One pass answers every such `’` at once: a pending `’` waits in a group with the others whose
/// own pairs start at the same depth of one stack of the pairs opened since, and the groups behave
/// alike from there on, so the time is linear in the number of quotes.
fn later_closers(quotes: &[QuoteAt], glyphs: [char; 4], nested: bool) -> Vec<bool> {
    let [open1, close1, open2, close2] = glyphs;
    let (own_open, own_close) = if nested {
        (open2, close2)
    } else {
        (open1, close1)
    };
    let mut answers = vec![false; quotes.len()];
    // The pending `’`, as linked lists: `next[k]` is the next member of the group of `k`.
    let mut next: Vec<usize> = vec![usize::MAX; quotes.len()];
    // The groups, innermost last: the depth of `stack` at which their pairs start, and the first
    // and last member.
    let mut groups: Vec<(usize, usize, usize)> = Vec::new();
    let mut stack = Opened::default();
    let mut window = usize::MAX;
    let settle = |answers: &mut Vec<bool>, next: &[usize], first: usize, answer: bool| {
        let mut k = first;
        while let Some(a) = answers.get_mut(k) {
            *a = answer;
            k = next.get(k).copied().unwrap_or(usize::MAX);
        }
    };
    for (k, q) in quotes.iter().enumerate() {
        if q.window != window {
            // A pending `’` finds nothing in its own window.
            groups.clear();
            stack.clear();
            window = q.window;
        }
        let c = q.c;
        let opens = c == open1 || c == open2;
        let closes = (c == close1 || c == close2) && (c != APOSTROPHE || q.closing);
        // The group whose own stack is empty.
        if let Some(&(base, first, _)) = groups.last()
            && base == stack.len()
        {
            let answer = if c == own_close && c == APOSTROPHE {
                (q.closing && !q.after_s).then_some(true)
            } else if c == own_open || closes {
                Some(false)
            } else {
                None
            };
            if let Some(answer) = answer {
                settle(&mut answers, &next, first, answer);
                groups.pop();
            }
        }
        if opens {
            stack.push(c == open2 && c != open1);
        } else if closes {
            let kind = c == close2 && c != close1;
            if c == APOSTROPHE {
                // `’` closes only a pair of its kind on top; otherwise it is an apostrophe.
                if stack.last() == Some(kind)
                    && groups.last().is_none_or(|&(base, _, _)| base < stack.len())
                {
                    stack.pop();
                    // The group whose own stack held only that pair now has an empty one, as
                    // the group that was waiting at the empty depth, if any, still has: they
                    // behave alike from here on.
                    merge_top(&mut groups, &mut next, stack.len());
                } else if stack.last() == Some(kind) {
                    // A group waits at the empty depth: the groups below it pop the pair.
                    let waiting = groups.pop();
                    stack.pop();
                    if let Some((_, first, last)) = waiting {
                        groups.push((stack.len(), first, last));
                    }
                    merge_top(&mut groups, &mut next, stack.len());
                }
            } else {
                // A closer that finds no pair of its kind in the own stack of a group closes a
                // pair opened before: for that group the answer is no.
                let level = stack.innermost(kind);
                while let Some(&(base, first, _)) = groups.last()
                    && level.is_none_or(|l| base > l)
                {
                    settle(&mut answers, &next, first, false);
                    groups.pop();
                }
                if let Some(level) = level {
                    stack.truncate(level);
                }
            }
        }
        if c == APOSTROPHE && (c == close1 || c == close2) && q.after_s && q.closing {
            match groups.last_mut() {
                Some((base, _, last)) if *base == stack.len() => {
                    if let Some(n) = next.get_mut(*last) {
                        *n = k;
                    }
                    *last = k;
                }
                _ => groups.push((stack.len(), k, k)),
            }
        }
    }
    answers
}

/// Merges the two innermost groups when both start at `depth`.
fn merge_top(groups: &mut Vec<(usize, usize, usize)>, next: &mut [usize], depth: usize) {
    let n = groups.len();
    if n < 2 {
        return;
    }
    let (Some(&(b1, f1, l1)), Some(&(b2, f2, l2))) = (groups.get(n - 2), groups.get(n - 1)) else {
        return;
    };
    if b1 == depth && b2 == depth {
        if let Some(x) = next.get_mut(l1) {
            *x = f2;
        }
        groups.truncate(n - 2);
        groups.push((depth, f1, l2));
    }
}
