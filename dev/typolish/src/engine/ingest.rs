//! Ingest and explicit protection (design sections 3.4 and 10.1): input pieces become a stream
//! of atoms, and protection delimiters cut `Delimited` atoms out of the text.
//!
//! The text of all `Text` pieces of a call is read as one string, the call's text: for string
//! input it is the input itself, for pieces the concatenation of their `Text` pieces (copied into
//! the scratch, so a template of `Text` pieces is lexed exactly as their concatenation, design
//! section 13.5, property 10). Atoms and tokens hold byte positions in that text and no
//! references, so the buffers that keep them are reused across calls for inputs of any lifetime
//! (design section 2.12). An atom that is not text (a slot, a protected piece, markup) has no
//! extent in the call's text: it stands at a position between two bytes of it and keeps its piece.

use super::Input;
use super::window::is_line_break;
use crate::SlotKind;

/// A `Text` piece of the input and its range in the call's text.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct TextPiece {
    pub(crate) piece: u32,
    pub(crate) start: u32,
    pub(crate) end: u32,
}

/// One atom of the input.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Atom {
    /// Text to typeset: a range of the call's text that holds adjacent `Text` pieces.
    Text { start: u32, end: u32 },
    /// Text between protection delimiters, the delimiters included (design section 10.1).
    Delimited { start: u32, end: u32 },
    /// An opaque placeholder at a position of the call's text.
    Slot { piece: u32, at: u32, kind: SlotKind },
    /// A `Piece::Protected`.
    Protected { piece: u32, at: u32 },
    /// A `Piece::Markup`.
    Markup { piece: u32, at: u32 },
    /// The characters of a window boundary (design section 3.3).
    Boundary { start: u32, end: u32 },
}

impl Atom {
    /// The position of an atom that has no extent in the call's text.
    fn point(self) -> Option<u32> {
        match self {
            Atom::Slot { at, .. } | Atom::Protected { at, .. } | Atom::Markup { at, .. } => {
                Some(at)
            }
            Atom::Text { .. } | Atom::Delimited { .. } | Atom::Boundary { .. } => None,
        }
    }
}

/// Builds the call's text and the first atoms: adjacent `Text` pieces merge into one `Text` atom,
/// every other non-empty piece and every slot is an atom of its own. Returns `None` when a piece
/// index or the length of the call's text does not fit the `u32` positions of atoms and tokens;
/// the caller then returns the input unchanged (design section 3.4).
pub(crate) fn layout<'x>(
    input: &[Input<'x>],
    buf: &'x mut String,
    pieces: &mut Vec<TextPiece>,
    atoms: &mut Vec<Atom>,
) -> Option<&'x str> {
    pieces.clear();
    atoms.clear();
    buf.clear();
    let mut texts = input.iter().filter_map(|item| match item {
        Input::Text(text) if !text.is_empty() => Some(*text),
        _ => None,
    });
    let first = texts.next();
    let copied = texts.next().is_some();
    let total = input.iter().fold(0usize, |sum, item| match item {
        Input::Text(text) => sum.saturating_add(text.len()),
        _ => sum,
    });
    u32::try_from(total).ok()?;
    let mut pos: u32 = 0;
    for (index, item) in input.iter().enumerate() {
        let piece = u32::try_from(index).ok()?;
        match *item {
            Input::Text(text) => {
                if text.is_empty() {
                    continue;
                }
                let end = pos.checked_add(u32::try_from(text.len()).ok()?)?;
                if copied {
                    buf.push_str(text);
                }
                pieces.push(TextPiece {
                    piece,
                    start: pos,
                    end,
                });
                match atoms.last_mut() {
                    Some(Atom::Text { end: last, .. }) if *last == pos => *last = end,
                    _ => atoms.push(Atom::Text { start: pos, end }),
                }
                pos = end;
            }
            Input::Slot(slot) => atoms.push(Atom::Slot {
                piece,
                at: pos,
                kind: slot.kind(),
            }),
            // An empty protected or markup piece is still an atom: it ends a token and is content
            // (design sections 2.6, 3.3); only empty `Text` pieces are skipped.
            Input::Protected(_) => atoms.push(Atom::Protected { piece, at: pos }),
            Input::Markup(_) => atoms.push(Atom::Markup { piece, at: pos }),
        }
    }
    if copied {
        Some(buf.as_str())
    } else {
        Some(first.unwrap_or(""))
    }
}

/// Working memory of delimiter matching.
#[derive(Debug, Default)]
pub(crate) struct DelimScratch {
    events: Vec<Event>,
    keys: Vec<(u32, u32)>,
    spans: Vec<(u32, u32)>,
    unclosed: Vec<(u32, u32)>,
    closers: Vec<u32>,
}

impl DelimScratch {
    pub(crate) const fn new() -> DelimScratch {
        DelimScratch {
            events: Vec::new(),
            keys: Vec::new(),
            spans: Vec::new(),
            unclosed: Vec::new(),
            closers: Vec::new(),
        }
    }

    pub(crate) fn heap_bytes(&self) -> usize {
        self.events.capacity() * core::mem::size_of::<Event>()
            + (self.keys.capacity() + self.spans.capacity() + self.unclosed.capacity())
                * core::mem::size_of::<(u32, u32)>()
            + self.closers.capacity() * core::mem::size_of::<u32>()
    }

    /// The unclosed delimiters found by the last call of `cut_delimiters`, as ranges of the
    /// call's text.
    pub(crate) fn unclosed(&self) -> &[(u32, u32)] {
        &self.unclosed
    }
}

/// A delimiter occurrence in the text of a window.
#[derive(Clone, Copy, Debug)]
struct Event {
    start: u32,
    end: u32,
    what: What,
    /// The index of the event that closes this one, if any.
    next: u32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum What {
    /// A run of backticks of this length.
    Run(u32),
    /// The opening delimiter of an own pair.
    Open(u32),
    /// The closing delimiter of an own pair.
    Close(u32),
    /// A delimiter that both opens and closes an own pair (`|`, `|`).
    Both(u32),
}

const NONE: u32 = u32::MAX;

/// The length of the run of backticks at `at`.
fn run_length(bytes: &[u8], at: usize, end: usize) -> usize {
    bytes
        .get(at..end)
        .map_or(0, |rest| rest.iter().take_while(|b| **b == b'`').count())
}

/// Step 1 of design section 3.4: cuts fenced backtick blocks (design section 10.1) out of the
/// `Text` atoms over the whole input, before windows are found. A run of three or more backticks
/// at the start of a line, after optional spaces and tabs, opens a block that a run of exactly the
/// same length at the start of a later line closes; a run with no such closing run opens nothing.
/// Atoms other than text are content: a line that starts with one does not start with a run.
pub(crate) fn cut_fences(text: &str, atoms: &[Atom], out: &mut Vec<Atom>, d: &mut DelimScratch) {
    let bytes = text.as_bytes();
    d.events.clear();
    let mut line_start = true;
    for atom in atoms {
        let Atom::Text { start, end } = *atom else {
            line_start = false;
            continue;
        };
        let (start, end) = (start as usize, end as usize);
        let mut i = start;
        let mut indent = line_start;
        while i < end {
            let Some(c) = text.get(i..end).and_then(|rest| rest.chars().next()) else {
                break;
            };
            if is_line_break(c) || c == '\u{2029}' {
                indent = true;
                i += c.len_utf8();
                continue;
            }
            if indent && (c == ' ' || c == '\t') {
                i += 1;
                continue;
            }
            if indent && c == '`' {
                let len = run_length(bytes, i, end);
                if len >= 3
                    && let (Ok(s), Ok(e), Ok(n)) =
                        (u32::try_from(i), u32::try_from(i + len), u32::try_from(len))
                {
                    d.events.push(Event {
                        start: s,
                        end: e,
                        what: What::Run(n),
                        next: NONE,
                    });
                }
                i += len;
                indent = false;
                continue;
            }
            indent = false;
            i += c.len_utf8();
        }
        line_start = indent;
    }
    link_runs(d);
    d.spans.clear();
    let mut i = 0;
    while let Some(event) = d.events.get(i) {
        match d.events.get(event.next as usize) {
            Some(close) if event.next != NONE => {
                d.spans.push((event.start, close.end));
                i = event.next as usize + 1;
            }
            _ => i += 1,
        }
    }
    cut(atoms, &d.spans, out);
}

/// Links every run of backticks to the next run of the same length: one sort, so the time is
/// O(k log k) and the memory O(k) whatever the lengths are.
fn link_runs(d: &mut DelimScratch) {
    let DelimScratch { events, keys, .. } = d;
    keys.clear();
    for (i, event) in events.iter().enumerate() {
        if let (What::Run(n), Ok(i)) = (event.what, u32::try_from(i)) {
            keys.push((n, i));
        }
    }
    keys.sort_unstable();
    for pair in keys.windows(2) {
        if let [(ka, ia), (kb, ib)] = *pair
            && ka == kb
            && let Some(event) = events.get_mut(ia as usize)
        {
            event.next = ib;
        }
    }
}

/// Step 2 of design section 3.4: within each window (the atoms between two `Boundary` atoms),
/// matches the other delimiters over the text of the window, across pieces, and cuts the spans
/// out as `Delimited` atoms; slots, protected pieces and markup inside a span stay what they are.
/// Pairs are formed left to right and do not nest; an opening delimiter without its closing one
/// is ordinary text and is left in `d.unclosed()`.
pub(crate) fn cut_delimiters(
    text: &str,
    atoms: &[Atom],
    backticks: bool,
    pairs: &[(String, String)],
    out: &mut Vec<Atom>,
    d: &mut DelimScratch,
) {
    d.spans.clear();
    d.unclosed.clear();
    if !backticks && pairs.is_empty() {
        out.clear();
        out.extend_from_slice(atoms);
        return;
    }
    let mut from = 0;
    while from < atoms.len() {
        let to = atoms
            .get(from..)
            .and_then(|rest| rest.iter().position(|a| matches!(a, Atom::Boundary { .. })))
            .map_or(atoms.len(), |n| from + n);
        if let Some(window) = atoms.get(from..to) {
            match_window(text, window, backticks, pairs, d);
        }
        from = to + 1;
    }
    cut(atoms, &d.spans, out);
}

/// Collects the delimiter occurrences of one window and pairs them.
fn match_window(
    text: &str,
    window: &[Atom],
    backticks: bool,
    pairs: &[(String, String)],
    d: &mut DelimScratch,
) {
    let bytes = text.as_bytes();
    d.events.clear();
    for atom in window {
        let Atom::Text { start, end } = *atom else {
            continue;
        };
        let (start, end) = (start as usize, end as usize);
        let mut i = start;
        while i < end {
            let rest = bytes.get(i..end).unwrap_or_default();
            let own = pairs.iter().enumerate().find_map(|(k, (open, close))| {
                let k = u32::try_from(k).ok()?;
                if rest.starts_with(open.as_bytes()) {
                    let what = if open == close {
                        What::Both(k)
                    } else {
                        What::Open(k)
                    };
                    Some((what, open.len()))
                } else if rest.starts_with(close.as_bytes()) {
                    Some((What::Close(k), close.len()))
                } else {
                    None
                }
            });
            let found = match own {
                Some(found) => Some(found),
                None if backticks && rest.first() == Some(&b'`') => {
                    let len = run_length(bytes, i, end);
                    u32::try_from(len).ok().map(|n| (What::Run(n), len))
                }
                None => None,
            };
            match found {
                Some((what, len)) => {
                    if let (Ok(s), Ok(e)) = (u32::try_from(i), u32::try_from(i + len)) {
                        d.events.push(Event {
                            start: s,
                            end: e,
                            what,
                            next: NONE,
                        });
                    }
                    i += len.max(1);
                }
                None => {
                    // Delimiters are UTF-8 strings matched at character boundaries, so the next
                    // position is the next character.
                    let step = text
                        .get(i..end)
                        .and_then(|r| r.chars().next())
                        .map_or(1, char::len_utf8);
                    i += step;
                }
            }
        }
    }
    link_runs(d);
    link_pairs(d, pairs.len());
    let mut i = 0;
    while let Some(event) = d.events.get(i) {
        let opens = matches!(event.what, What::Run(_) | What::Open(_) | What::Both(_));
        if !opens {
            // A closing delimiter without an opening one is ordinary text.
            i += 1;
            continue;
        }
        match d.events.get(event.next as usize) {
            Some(close) if event.next != NONE => {
                d.spans.push((event.start, close.end));
                i = event.next as usize + 1;
            }
            _ => {
                d.unclosed.push((event.start, event.end));
                i += 1;
            }
        }
    }
}

/// Links every opening delimiter of an own pair to the next delimiter that closes it.
fn link_pairs(d: &mut DelimScratch, count: usize) {
    if count == 0 {
        return;
    }
    let DelimScratch {
        events, closers, ..
    } = d;
    closers.clear();
    closers.resize(count, NONE);
    for i in (0..events.len()).rev() {
        let Some(event) = events.get_mut(i) else {
            continue;
        };
        let Ok(index) = u32::try_from(i) else {
            continue;
        };
        match event.what {
            What::Open(k) => {
                event.next = closers.get(k as usize).copied().unwrap_or(NONE);
            }
            What::Both(k) => {
                event.next = closers.get(k as usize).copied().unwrap_or(NONE);
                if let Some(slot) = closers.get_mut(k as usize) {
                    *slot = index;
                }
            }
            What::Close(k) => {
                if let Some(slot) = closers.get_mut(k as usize) {
                    *slot = index;
                }
            }
            What::Run(_) => {}
        }
    }
}

/// Rewrites `atoms` into `out` so that the text inside `spans` (sorted ranges of the call's text
/// that do not overlap) becomes `Delimited` atoms. An atom without extent lies inside a span when
/// its position is strictly between the ends of the span; it stays what it is.
fn cut(atoms: &[Atom], spans: &[(u32, u32)], out: &mut Vec<Atom>) {
    out.clear();
    let mut spans = spans.iter().copied().peekable();
    for atom in atoms {
        let Atom::Text { start, end } = *atom else {
            if let Some(at) = atom.point() {
                while spans.next_if(|(_, e)| *e <= at).is_some() {}
            }
            out.push(*atom);
            continue;
        };
        let mut cur = start;
        while cur < end {
            while spans.next_if(|(_, e)| *e <= cur).is_some() {}
            match spans.peek().copied() {
                Some((s, e)) if s < end => {
                    if s > cur {
                        out.push(Atom::Text { start: cur, end: s });
                        cur = s;
                    }
                    let stop = e.min(end);
                    out.push(Atom::Delimited {
                        start: cur,
                        end: stop,
                    });
                    cur = stop;
                }
                _ => {
                    out.push(Atom::Text { start: cur, end });
                    cur = end;
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Slot;

    fn layout_of<'x>(
        input: &[Input<'x>],
        buf: &'x mut String,
    ) -> (String, Vec<TextPiece>, Vec<Atom>) {
        let mut pieces = Vec::new();
        let mut atoms = Vec::new();
        let text = layout(input, buf, &mut pieces, &mut atoms)
            .unwrap()
            .to_owned();
        (text, pieces, atoms)
    }

    #[test]
    fn adjacent_text_pieces_are_one_atom() {
        let input = [
            Input::Text("ab"),
            Input::Text(""),
            Input::Text("c"),
            Input::Slot(Slot::NUMBER),
            Input::Protected("`x`"),
            Input::Markup("<b>"),
            Input::Markup(""),
            Input::Text("d"),
        ];
        let mut buf = String::new();
        let (text, pieces, atoms) = layout_of(&input, &mut buf);
        assert_eq!(text, "abcd");
        assert_eq!(
            pieces,
            [
                TextPiece {
                    piece: 0,
                    start: 0,
                    end: 2
                },
                TextPiece {
                    piece: 2,
                    start: 2,
                    end: 3
                },
                TextPiece {
                    piece: 7,
                    start: 3,
                    end: 4
                },
            ]
        );
        assert_eq!(
            atoms,
            [
                Atom::Text { start: 0, end: 3 },
                Atom::Slot {
                    piece: 3,
                    at: 3,
                    kind: SlotKind::Number
                },
                Atom::Protected { piece: 4, at: 3 },
                Atom::Markup { piece: 5, at: 3 },
                Atom::Text { start: 3, end: 4 },
            ]
        );
    }

    #[test]
    fn one_text_piece_is_not_copied() {
        let input = [Input::Slot(Slot::WORD), Input::Text("abc")];
        let mut buf = String::new();
        let mut pieces = Vec::new();
        let mut atoms = Vec::new();
        let text = layout(&input, &mut buf, &mut pieces, &mut atoms).unwrap();
        assert_eq!(text, "abc");
        assert!(buf.is_empty());
    }

    fn delimited(text: &str, atoms: &[Atom]) -> Vec<String> {
        atoms
            .iter()
            .filter_map(|a| match *a {
                Atom::Delimited { start, end } => {
                    Some(text[start as usize..end as usize].to_owned())
                }
                _ => None,
            })
            .collect()
    }

    #[test]
    fn fences_and_inline_runs() {
        let text = "a\n```\nb\n\nc\n```\n`d` ``e`` `f";
        let atoms = [Atom::Text {
            start: 0,
            end: text.len() as u32,
        }];
        let mut d = DelimScratch::default();
        let mut fenced = Vec::new();
        cut_fences(text, &atoms, &mut fenced, &mut d);
        assert_eq!(delimited(text, &fenced), ["```\nb\n\nc\n```"]);
        let mut out = Vec::new();
        cut_delimiters(text, &fenced, true, &[], &mut out, &mut d);
        assert_eq!(delimited(text, &out), ["```\nb\n\nc\n```", "`d`", "``e``"]);
        assert_eq!(d.unclosed(), [(25, 26)]);
    }

    #[test]
    fn a_span_keeps_the_atoms_inside_it() {
        let text = "`a b`";
        let atoms = [
            Atom::Text { start: 0, end: 2 },
            Atom::Slot {
                piece: 1,
                at: 2,
                kind: SlotKind::Word,
            },
            Atom::Text { start: 2, end: 5 },
        ];
        let mut d = DelimScratch::default();
        let mut out = Vec::new();
        cut_delimiters(text, &atoms, true, &[], &mut out, &mut d);
        assert_eq!(
            out,
            [
                Atom::Delimited { start: 0, end: 2 },
                Atom::Slot {
                    piece: 1,
                    at: 2,
                    kind: SlotKind::Word
                },
                Atom::Delimited { start: 2, end: 5 },
            ]
        );
    }

    #[test]
    fn own_pairs_do_not_nest_and_equal_delimiters_pair() {
        let pairs = [
            ("{{".to_owned(), "}}".to_owned()),
            ("|".to_owned(), "|".to_owned()),
        ];
        let text = "{{ a | b }} |c|d| }}";
        let atoms = [Atom::Text {
            start: 0,
            end: text.len() as u32,
        }];
        let mut d = DelimScratch::default();
        let mut out = Vec::new();
        cut_delimiters(text, &atoms, true, &pairs, &mut out, &mut d);
        assert_eq!(delimited(text, &out), ["{{ a | b }}", "|c|"]);
        assert_eq!(d.unclosed(), [(16, 17)]);
    }
}
