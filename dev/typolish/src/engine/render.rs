//! Rendering (design sections 3.10 and 11): the edits of the changed windows applied to the
//! input, written in the output kind.
//!
//! After the phases of a changed window, its tokens give one edit per replaced token, per glued
//! space and per inserted space, and the phases list the removed spaces. Each edit replaces a
//! range of the call's text. In `Output::Unicode` a string comes back borrowed when no window
//! changed; otherwise the output is built once, at the exact length counted from the edits, with
//! the unchanged parts copied. A changed template text is allocated once at its length plus the new text of its
//! edits; a template text that no edit touches is the merge of its pieces, as without rules.
//!
//! The other kinds go through the encoder of `crate::output`: the renderer walks the result as a
//! stream of characters, each marked as text, protected text or markup, with the slots between
//! them, and finds the nowrap groups of design section 11.5 on that stream. A string or a template
//! text is borrowed (or merged) exactly when its encoded form equals the input, which a comparison
//! decides without allocation; a changed string or template text is counted first and then
//! allocated once.

use std::borrow::Cow;

use super::Input;
use super::ingest::{Atom, TextPiece};
use super::lex::{EllipsisForm, Flags, Kind, NO_PIECE, Repl, Tok};
use super::phase::{ellipsis_tail, glue_char};
use super::record::Recorder;
use crate::lang::data::Table;
use crate::output::{self, Count, Encoder, Nowrap, Out, Res, Same};
use crate::report::Diagnostic;
use crate::{DiagnosticKind, Output, Severity};

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

/// The length in bytes of the new text of an edit.
fn written_len(text: RText, data: &Table<'_>) -> usize {
    match text {
        RText::Empty => 0,
        RText::Char(c) => c.len_utf8(),
        RText::Ellipsis(form) => ellipsis_tail(data, form).len(),
    }
}

/// Appends the new text of an edit.
pub(crate) fn write_text(out: &mut String, text: RText, data: &Table<'_>) {
    match text {
        RText::Empty => {}
        RText::Char(c) => out.push(c),
        RText::Ellipsis(form) => out.push_str(ellipsis_tail(data, form)),
    }
}

/// The result for string input: borrowed when there is no edit; otherwise one string of exactly
/// the length of the result (design section 2.12).
pub(crate) fn render_text<'a>(text: &'a str, edits: &[REdit], data: &Table<'_>) -> Cow<'a, str> {
    if edits.is_empty() {
        return Cow::Borrowed(text);
    }
    let mut out = String::with_capacity(rendered_len(text, edits, data));
    render_text_into(text, edits, data, &mut out);
    Cow::Owned(out)
}

/// The length in bytes of what `render_text_into` appends, counted the same way.
fn rendered_len(text: &str, edits: &[REdit], data: &Table<'_>) -> usize {
    let mut pos = 0usize;
    let mut len = 0usize;
    for e in edits {
        let (start, end) = (e.start as usize, e.end as usize);
        len = len.saturating_add(text.get(pos..start).map_or(0, str::len));
        len = len.saturating_add(match e.text {
            RText::Empty => 0,
            RText::Char(c) => c.len_utf8(),
            RText::Ellipsis(form) => ellipsis_tail(data, form).len(),
        });
        pos = end.max(pos);
    }
    len.saturating_add(text.get(pos..).map_or(0, str::len))
}

/// Appends `text` with its edits to `out`, without a string of its own (`typeset_into`, design
/// section 2.12).
pub(crate) fn render_text_into(text: &str, edits: &[REdit], data: &Table<'_>, out: &mut String) {
    let mut pos = 0usize;
    for e in edits {
        let (start, end) = (e.start as usize, e.end as usize);
        out.push_str(text.get(pos..start).unwrap_or(""));
        write_text(out, e.text, data);
        pos = end.max(pos);
    }
    out.push_str(text.get(pos..).unwrap_or(""));
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
fn write_piece(out: &mut String, text: &str, p: &TextPiece, edits: &[REdit], data: &Table<'_>) {
    let mut pos = p.start;
    for e in edits_of(edits, p) {
        let start = e.start.max(p.start);
        let end = e.end.min(p.end);
        out.push_str(
            text.get((pos - p.start) as usize..(start.max(pos) - p.start) as usize)
                .unwrap_or(""),
        );
        if e.start >= p.start {
            write_text(out, e.text, data);
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
    data: &Table<'_>,
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
        // The text and the new text of its edits: a bound on the result, so the string is
        // allocated once (design section 2.12), however much the edits grow a short text (a
        // glued space takes two bytes for one).
        let len = group
            .iter()
            .enumerate()
            .fold(0usize, |len, (k, item)| match item {
                Input::Text(s) => {
                    let grown = piece_of(base + k).map_or(0, |p| {
                        edits_of(edits, p)
                            .filter(|e| e.start >= p.start)
                            .map(|e| written_len(e.text, data))
                            .fold(0usize, usize::saturating_add)
                    });
                    len.saturating_add(s.len()).saturating_add(grown)
                }
                Input::Protected(s) | Input::Markup(s) => len.saturating_add(s.len()),
                Input::Slot(_) => len,
            });
        let mut out = String::with_capacity(len);
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
pub(crate) fn merge<'t>(group: &[Input<'t>]) -> Cow<'t, str> {
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

/// What the encoded renderer reads after a run: the input pieces, the table of `Text` pieces
/// with their ranges in the call's text, the atoms (whose `Delimited` ranges are the protected
/// spans), the edits and the language data.
pub(crate) struct View<'v, 't> {
    pub(crate) input: &'v [Input<'t>],
    pub(crate) pieces: &'v [TextPiece],
    pub(crate) atoms: &'v [Atom],
    pub(crate) edits: &'v [REdit],
    pub(crate) data: &'v Table<'v>,
}

/// A part of the result.
#[derive(Clone, Copy, Debug)]
enum Seg<'a> {
    /// Text that the rules read, unchanged or as an edit wrote it.
    Text(&'a str),
    /// A delimited span or a `Protected` piece: never typeset, still encoded.
    Protected(&'a str),
    /// A `Markup` piece and its index.
    Markup(&'a str, usize),
    /// The one character an edit wrote.
    Char(char),
    /// A slot: the end of a text of the n + 1 form.
    Slot,
}

/// The parts of the result in order: the walk of `write_piece` over every input piece, with the
/// unchanged text split at the bounds of delimited spans. `Copy`, so a lookahead is a copy.
#[derive(Clone, Copy)]
struct Segs<'v, 't> {
    v: &'v View<'v, 't>,
    /// The input piece.
    k: usize,
    /// The `Text` piece `k` is being walked: `piece` is its range, `pos` the position reached.
    walking: bool,
    piece: TextPiece,
    pos: u32,
    /// The next edit to consider, and whether its new text was given out.
    edit: usize,
    wrote: bool,
    /// The next atom to consider for a delimited span.
    atom: usize,
}

impl<'v, 't> Segs<'v, 't> {
    fn new(v: &'v View<'v, 't>) -> Segs<'v, 't> {
        Segs {
            v,
            k: 0,
            walking: false,
            piece: TextPiece {
                piece: 0,
                start: 0,
                end: 0,
            },
            pos: 0,
            edit: 0,
            wrote: false,
            atom: 0,
        }
    }

    /// Whether `pos` lies in a delimited span, and where that answer next changes.
    fn region(&mut self, pos: u32) -> (bool, u32) {
        while let Some(atom) = self.v.atoms.get(self.atom) {
            if let Atom::Delimited { start, end } = *atom
                && end > pos
            {
                return if start <= pos {
                    (true, end)
                } else {
                    (false, start)
                };
            }
            self.atom += 1;
        }
        (false, u32::MAX)
    }

    /// The unchanged text of the current piece from `pos` to `until`, or to the next bound of a
    /// delimited span before it.
    fn raw(&mut self, s: &'t str, until: u32) -> Seg<'t> {
        let (protected, bound) = self.region(self.pos);
        let end = until.min(bound);
        let p = self.piece;
        let slice = s
            .get((self.pos - p.start) as usize..(end.max(self.pos) - p.start) as usize)
            .unwrap_or("");
        self.pos = end.max(self.pos);
        if protected {
            Seg::Protected(slice)
        } else {
            Seg::Text(slice)
        }
    }

    fn next(&mut self) -> Option<Seg<'t>> {
        let edits = self.v.edits;
        loop {
            match *self.v.input.get(self.k)? {
                Input::Slot(_) => {
                    self.k += 1;
                    return Some(Seg::Slot);
                }
                Input::Protected(s) => {
                    self.k += 1;
                    if !s.is_empty() {
                        return Some(Seg::Protected(s));
                    }
                }
                Input::Markup(s) => {
                    self.k += 1;
                    if !s.is_empty() {
                        return Some(Seg::Markup(s, self.k - 1));
                    }
                }
                Input::Text(s) => {
                    if !self.walking {
                        let found = self
                            .v
                            .pieces
                            .binary_search_by_key(&self.k, |p| p.piece as usize)
                            .ok()
                            .and_then(|i| self.v.pieces.get(i));
                        let Some(p) = found else {
                            // No piece: an empty text, or a run that did not lay the input out.
                            self.k += 1;
                            if s.is_empty() {
                                continue;
                            }
                            return Some(Seg::Text(s));
                        };
                        self.walking = true;
                        self.piece = *p;
                        self.pos = p.start;
                        self.edit = edits.partition_point(|e| e.end < p.start);
                        self.wrote = false;
                    }
                    let p = self.piece;
                    while edits
                        .get(self.edit)
                        .is_some_and(|e| e.start <= p.end && !touches(e, &p))
                    {
                        self.edit += 1;
                    }
                    match edits.get(self.edit).filter(|e| e.start <= p.end) {
                        Some(e) => {
                            let start = e.start.max(p.start);
                            if self.pos < start {
                                return Some(self.raw(s, start));
                            }
                            if !self.wrote {
                                self.wrote = true;
                                if e.start >= p.start {
                                    match e.text {
                                        RText::Empty => {}
                                        RText::Char(c) => return Some(Seg::Char(c)),
                                        RText::Ellipsis(form) => {
                                            return Some(Seg::Text(ellipsis_tail(
                                                self.v.data,
                                                form,
                                            )));
                                        }
                                    }
                                }
                            }
                            self.pos = self.pos.max(e.end.min(p.end));
                            self.edit += 1;
                            self.wrote = false;
                        }
                        None if self.pos < p.end => return Some(self.raw(s, p.end)),
                        None => {
                            self.walking = false;
                            self.k += 1;
                        }
                    }
                }
            }
        }
    }
}

/// One element of the result for the encoder.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Item<'a> {
    /// A character, and whether it is protected.
    Char(char, bool),
    /// A `Markup` piece and its index.
    Markup(&'a str, usize),
    Slot,
}

impl Item<'_> {
    /// Material of a nowrap group (design section 11.5): a character of the rules' text that is
    /// not whitespace, or a slot. Whitespace, protected text and markup end a group.
    fn content(self) -> bool {
        match self {
            Item::Char(c, protected) => !protected && !c.is_whitespace(),
            Item::Slot => true,
            Item::Markup(..) => false,
        }
    }

    /// A character that can be glue: U+00A0 or U+202F of the rules' text.
    fn glue(self) -> bool {
        matches!(self, Item::Char('\u{a0}' | '\u{202f}', false))
    }
}

/// The characters of the result. `Copy`, like `Segs`.
#[derive(Clone, Copy)]
struct Items<'v, 't> {
    segs: Segs<'v, 't>,
    rest: &'t str,
    protected: bool,
    one: Option<char>,
}

impl<'v, 't> Items<'v, 't> {
    fn new(v: &'v View<'v, 't>) -> Items<'v, 't> {
        Items {
            segs: Segs::new(v),
            rest: "",
            protected: false,
            one: None,
        }
    }

    fn next(&mut self) -> Option<Item<'t>> {
        loop {
            if let Some(c) = self.one.take() {
                return Some(Item::Char(c, false));
            }
            let mut chars = self.rest.chars();
            if let Some(c) = chars.next() {
                self.rest = chars.as_str();
                return Some(Item::Char(c, self.protected));
            }
            match self.segs.next()? {
                Seg::Text(s) => (self.rest, self.protected) = (s, false),
                Seg::Protected(s) => (self.rest, self.protected) = (s, true),
                Seg::Char(c) => self.one = Some(c),
                Seg::Markup(s, k) => return Some(Item::Markup(s, k)),
                Seg::Slot => return Some(Item::Slot),
            }
        }
    }
}

/// Whether an item is U+2011, a joint wherever it stands in a group (design section 11.5).
fn hyphen(item: Item<'_>) -> usize {
    usize::from(item == Item::Char('\u{2011}', false))
}

/// The extent of the run of group material that starts at the next item: its number of items and
/// of joints (design section 11.5). Glue is a single U+00A0 or U+202F between two items of
/// material, and a joint, as is every U+2011 of the run.
fn scan(mut items: Items<'_, '_>) -> (usize, usize) {
    let (mut len, mut joints) = (0usize, 0usize);
    loop {
        match items.next() {
            Some(item) if item.content() => {
                len += 1;
                joints += hyphen(item);
            }
            Some(item) if item.glue() => match items.next() {
                Some(after) if after.content() => {
                    len += 2;
                    joints += 1 + hyphen(after);
                }
                _ => break,
            },
            _ => break,
        }
    }
    (len, joints)
}

/// The run that starts at the next item when markup is looked through instead of ending it: its
/// number of items (markup included), its joints, and the `Markup` pieces inside it, which split
/// the group it would be. The debugging path of `report_template` only.
fn scan_through_markup(mut items: Items<'_, '_>) -> (usize, usize, Vec<usize>) {
    fn next<'t>(items: &mut Items<'_, 't>, held: &mut Vec<usize>) -> Option<Item<'t>> {
        loop {
            match items.next() {
                Some(Item::Markup(_, k)) => held.push(k),
                other => return other,
            }
        }
    }
    let (mut len, mut joints) = (0usize, 0usize);
    let mut inside = Vec::new();
    loop {
        let mut held = Vec::new();
        match next(&mut items, &mut held) {
            Some(item) if item.content() => {
                len += 1 + held.len();
                joints += hyphen(item);
                inside.append(&mut held);
            }
            Some(item) if item.glue() => {
                let mut after_held = Vec::new();
                match next(&mut items, &mut after_held) {
                    Some(after) if after.content() => {
                        len += 2 + held.len() + after_held.len();
                        joints += 1 + hyphen(after);
                        inside.append(&mut held);
                        inside.append(&mut after_held);
                    }
                    _ => break,
                }
            }
            _ => break,
        }
    }
    (len, joints, inside)
}

/// The encoder of a whole result with the nowrap groups: the state between two texts of a
/// template. `Copy`, so a text can be written once to compare or count and once for real.
#[derive(Clone, Copy)]
struct Writer {
    encoder: Encoder,
    nowrap: Option<Nowrap>,
    /// The items left in the current run of group material, and whether it is a group.
    run: usize,
    group: bool,
    /// The last item written was a slot whose step in the run is still to be taken, so that a
    /// group that ends with the slot closes at the start of the next text.
    after_slot: bool,
}

impl Writer {
    fn new(output: Output) -> Writer {
        Writer {
            encoder: Encoder::new(output),
            nowrap: output::nowrap_of(output),
            run: 0,
            group: false,
            after_slot: false,
        }
    }

    /// One step through the current run; the closing tag after its last item.
    fn step<O: Out>(&mut self, o: &mut O) -> Res {
        if self.run > 0 {
            self.run -= 1;
            if self.run == 0 && self.group {
                self.group = false;
                o.put(output::CLOSE)?;
            }
        }
        Ok(())
    }

    /// Writes the items up to the next slot or the end; returns whether a slot ended the text.
    fn text<O: Out>(&mut self, items: &mut Items<'_, '_>, o: &mut O) -> Result<bool, output::Stop> {
        if self.after_slot {
            self.after_slot = false;
            self.step(o)?;
        }
        loop {
            let ahead = *items;
            let Some(item) = items.next() else {
                self.encoder.flush(o)?;
                return Ok(false);
            };
            if let Some(nowrap) = self.nowrap
                && self.run == 0
                && item.content()
            {
                let (len, joints) = scan(ahead);
                self.run = len;
                self.group = joints > 0;
                if self.group {
                    output::open(o, nowrap)?;
                }
            }
            match item {
                Item::Char(c, protected) => {
                    self.encoder.char(o, c, self.group && !protected)?;
                }
                Item::Markup(s, _) => self.encoder.markup(o, s)?,
                Item::Slot => {
                    self.encoder.flush(o)?;
                    self.after_slot = true;
                    return Ok(true);
                }
            }
            self.step(o)?;
        }
    }
}

/// The result for string input in an encoded output kind: borrowed when the encoded text equals
/// the input; otherwise counted, then allocated once (design section 2.12).
pub(crate) fn render_text_encoded<'a>(
    text: &'a str,
    v: &View<'_, '_>,
    out: Output,
) -> Cow<'a, str> {
    let items = Items::new(v);
    let writer = Writer::new(out);
    if v.edits.is_empty() {
        let (mut i, mut w) = (items, writer);
        let mut same = Same::new(core::iter::once(text));
        if w.text(&mut i, &mut same).is_ok() && same.finished() {
            return Cow::Borrowed(text);
        }
    }
    let mut count = Count(0);
    let (mut i, mut w) = (items, writer);
    let _ = w.text(&mut i, &mut count);
    let mut s = String::with_capacity(count.0);
    let (mut i, mut w) = (items, writer);
    // Writing into a `String` never stops.
    let _ = w.text(&mut i, &mut s);
    Cow::Owned(s)
}

/// Appends the result for string input in an encoded output kind to `buf`.
pub(crate) fn render_into_encoded(v: &View<'_, '_>, out: Output, buf: &mut String) {
    let _ = Writer::new(out).text(&mut Items::new(v), buf);
}

/// The texts of a template in the n + 1 form in an encoded output kind, and whether any of them
/// changed. A text whose encoded form equals the merge of its pieces is that merge; any other is
/// counted, then allocated once. With a nowrap style, every `Markup` piece that splits a group is
/// reported as `NowrapFallback` (design section 11.5).
pub(crate) fn render_template_encoded<'t, R: Recorder>(
    v: &View<'_, 't>,
    out: Output,
    slot_count: usize,
    rec: &mut R,
) -> (Vec<Cow<'t, str>>, bool) {
    if R::ACTIVE && output::nowrap_of(out).is_some() {
        report_fallbacks(v, rec);
    }
    let mut texts = Vec::with_capacity(slot_count.saturating_add(1));
    let mut changed = false;
    let mut items = Items::new(v);
    let mut writer = Writer::new(out);
    for group in v.input.split(|item| matches!(item, Input::Slot(_))) {
        let parts = || {
            group.iter().filter_map(|item| match *item {
                Input::Text(s) | Input::Protected(s) | Input::Markup(s) => Some(s),
                Input::Slot(_) => None,
            })
        };
        let (mut i, mut w) = (items, writer);
        let mut same = Same::new(parts());
        if w.text(&mut i, &mut same).is_ok() && same.finished() {
            texts.push(merge(group));
            (items, writer) = (i, w);
            continue;
        }
        let (mut i, mut w) = (items, writer);
        let mut count = Count(0);
        let _ = w.text(&mut i, &mut count);
        let mut s = String::with_capacity(count.0);
        let _ = writer.text(&mut items, &mut s);
        texts.push(Cow::Owned(s));
        changed = true;
    }
    (texts, changed)
}

/// `NowrapFallback` (info) at every `Markup` piece that lies inside a group with a joint when
/// markup is looked through, and so splits it.
fn report_fallbacks<R: Recorder>(v: &View<'_, '_>, rec: &mut R) {
    let mut items = Items::new(v);
    loop {
        let ahead = items;
        let Some(item) = items.next() else {
            return;
        };
        if !item.content() {
            continue;
        }
        let (len, joints, inside) = scan_through_markup(ahead);
        if joints > 0 {
            for k in inside {
                let len = match v.input.get(k) {
                    Some(Input::Markup(s)) => s.len(),
                    _ => 0,
                };
                rec.diagnostic(Diagnostic::new(
                    DiagnosticKind::NowrapFallback,
                    Severity::Info,
                    None,
                    k,
                    0..len,
                ));
            }
        }
        // Past the run: its items are no start of another.
        items = ahead;
        for _ in 0..len {
            let _ = items.next();
        }
    }
}
