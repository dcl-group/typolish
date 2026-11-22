//! Rendering (design section 3.10): the edits of the changed windows applied to the input.
//!
//! After the phases of a changed window, its tokens give one edit per replaced token, per glued
//! space and per inserted space, and the phases list the removed spaces. Each edit replaces a
//! range of the call's text. A string comes back borrowed when no window changed; otherwise the
//! output is built once, at `len + len / 8` bytes, with the unchanged parts copied. A template
//! text that no edit touches is the merge of its pieces, as without rules. Only
//! `Output::Unicode` is rendered so far: `Html` and `Escaped` are written as Unicode until the
//! encoders of stage 3.

use std::borrow::Cow;

use super::Input;
use super::ingest::TextPiece;
use super::lex::{EllipsisForm, Flags, Kind, NO_PIECE, Repl, Tok};
use super::phase::{ellipsis_tail, glue_char};
use crate::lang::data::LanguageData;

/// The new text of an edit.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum RText {
    /// A deletion.
    Empty,
    Char(char),
    Ellipsis(EllipsisForm),
}

/// One edit of the call's text.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct REdit {
    pub(crate) start: u32,
    pub(crate) end: u32,
    pub(crate) text: RText,
    /// For an insertion: the token before it is text, so it belongs to the piece that ends at
    /// its position.
    pub(crate) left: bool,
}

/// Appends the edits of one changed window, in position order.
pub(crate) fn collect(tokens: &[Tok], removed: &[(u32, u32)], out: &mut Vec<REdit>) {
    let base = out.len();
    let mut prev_text = false;
    for tok in tokens {
        if tok.flags.contains(Flags::INSERTED) {
            out.push(REdit {
                start: tok.src.start,
                end: tok.src.start,
                text: RText::Char(glue_char(tok.glue)),
                left: prev_text,
            });
        } else if tok.flags.contains(Flags::CHANGED) {
            let text = match tok.repl {
                Repl::Char(c) => RText::Char(c),
                Repl::Ellipsis(form) => RText::Ellipsis(form),
                Repl::Source if tok.kind == Kind::Space => RText::Char(glue_char(tok.glue)),
                Repl::Source => continue,
            };
            out.push(REdit {
                start: tok.src.start,
                end: tok.src.end,
                text,
                left: true,
            });
        }
        prev_text = tok.piece == NO_PIECE;
    }
    out.extend(removed.iter().map(|&(start, end)| REdit {
        start,
        end,
        text: RText::Empty,
        left: true,
    }));
    if let Some(window) = out.get_mut(base..) {
        window.sort_unstable_by_key(|e| (e.start, e.end));
    }
}

fn write(out: &mut String, text: RText, data: &LanguageData) {
    match text {
        RText::Empty => {}
        RText::Char(c) => out.push(c),
        RText::Ellipsis(form) => out.push_str(ellipsis_tail(data, form)),
    }
}

/// The result for string input: borrowed when there is no edit.
pub(crate) fn render_text<'a>(text: &'a str, edits: &[REdit], data: &LanguageData) -> Cow<'a, str> {
    if edits.is_empty() {
        return Cow::Borrowed(text);
    }
    let mut out = String::with_capacity(text.len() + text.len() / 8);
    let mut pos = 0usize;
    for e in edits {
        let (start, end) = (e.start as usize, e.end as usize);
        out.push_str(text.get(pos..start).unwrap_or(""));
        write(&mut out, e.text, data);
        pos = end.max(pos);
    }
    out.push_str(text.get(pos..).unwrap_or(""));
    Cow::Owned(out)
}

/// Whether an edit belongs to a piece: a range that overlaps it, or an insertion at a position
/// it holds (the piece that ends there when `left`, the one that starts there otherwise).
fn touches(e: &REdit, p: &TextPiece) -> bool {
    if e.start == e.end {
        if e.left {
            p.start < e.start && e.start <= p.end
        } else {
            p.start <= e.start && e.start < p.end
        }
    } else {
        e.start < p.end && e.end > p.start
    }
}

/// The edits that belong to a piece.
fn edits_of<'e>(edits: &'e [REdit], p: &'e TextPiece) -> impl Iterator<Item = &'e REdit> + 'e {
    let first = edits.partition_point(|e| e.end < p.start);
    edits
        .get(first..)
        .unwrap_or_default()
        .iter()
        .take_while(move |e| e.start <= p.end)
        .filter(move |e| touches(e, p))
}

/// Appends the text of one piece with its edits; an edit that starts in an earlier piece only
/// deletes its part here (design section 3.11).
fn write_piece(out: &mut String, text: &str, p: &TextPiece, edits: &[REdit], data: &LanguageData) {
    let mut pos = p.start;
    for e in edits_of(edits, p) {
        let start = e.start.max(p.start);
        let end = e.end.min(p.end);
        out.push_str(
            text.get((pos - p.start) as usize..(start.max(pos) - p.start) as usize)
                .unwrap_or(""),
        );
        if e.start >= p.start {
            write(out, e.text, data);
        }
        pos = pos.max(end);
    }
    out.push_str(text.get((pos - p.start) as usize..).unwrap_or(""));
}

/// The texts of a template in the n + 1 form, and whether any of them changed (design section
/// 2.6): a text that no edit touches borrows its piece or merges its pieces into one string.
pub(crate) fn render_template<'t>(
    input: &[Input<'t>],
    pieces: &[TextPiece],
    edits: &[REdit],
    slot_count: usize,
    data: &LanguageData,
) -> (Vec<Cow<'t, str>>, bool) {
    let mut texts = Vec::with_capacity(slot_count.saturating_add(1));
    let mut changed = false;
    let piece_of = |index: usize| {
        pieces
            .binary_search_by_key(&index, |p| p.piece as usize)
            .ok()
            .and_then(|k| pieces.get(k))
    };
    let mut offset = 0usize;
    for group in input.split(|item| matches!(item, Input::Slot(_))) {
        let base = offset;
        offset += group.len() + 1;
        let touched = !edits.is_empty()
            && group.iter().enumerate().any(|(k, item)| {
                matches!(item, Input::Text(_))
                    && piece_of(base + k).is_some_and(|p| edits_of(edits, p).next().is_some())
            });
        if !touched {
            texts.push(merge(group));
            continue;
        }
        changed = true;
        let len = group.iter().fold(0usize, |len, item| match item {
            Input::Text(s) | Input::Protected(s) | Input::Markup(s) => len.saturating_add(s.len()),
            Input::Slot(_) => len,
        });
        let mut out = String::with_capacity(len + len / 8);
        for (k, item) in group.iter().enumerate() {
            match *item {
                Input::Text(s) => match piece_of(base + k) {
                    Some(p) => write_piece(&mut out, s, p, edits, data),
                    None => out.push_str(s),
                },
                Input::Protected(s) | Input::Markup(s) => out.push_str(s),
                Input::Slot(_) => {}
            }
        }
        texts.push(Cow::Owned(out));
    }
    (texts, changed)
}

/// The text of the pieces between two slots: borrowed while it maps to at most one non-empty
/// piece, otherwise one string allocated at its final length (design section 2.6, output
/// contract item 2).
fn merge<'t>(group: &[Input<'t>]) -> Cow<'t, str> {
    let mut parts = group.iter().filter_map(|item| match *item {
        Input::Text(text) | Input::Protected(text) | Input::Markup(text) if !text.is_empty() => {
            Some(text)
        }
        Input::Text(_) | Input::Protected(_) | Input::Markup(_) | Input::Slot(_) => None,
    });
    let Some(first) = parts.next() else {
        return Cow::Borrowed("");
    };
    if parts.clone().next().is_none() {
        return Cow::Borrowed(first);
    }
    let len = parts
        .clone()
        .fold(first.len(), |len, text| len.saturating_add(text.len()));
    let mut all = String::with_capacity(len);
    all.push_str(first);
    parts.for_each(|text| all.push_str(text));
    Cow::Owned(all)
}
