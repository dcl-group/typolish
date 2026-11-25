//! Recorders: what a call collects besides its result (design section 3.11).
//!
//! The engine is generic over `R: Recorder` and monomorphized: `typeset` uses `NoRecord`, whose
//! `ACTIVE` is false, so the plain path skips the work of building records; `lint` uses
//! `Diagnostics`; `report` uses `Journal`. The engine resolves positions to pieces before it calls a recorder (one record per
//! piece for a change or a span, the first piece for a diagnostic), so a recorder only collects.

use core::ops::Range;

use super::classify::FragKind;
use super::ingest::TextPiece;
use crate::report::{Change, Diagnostic, ProtectedKind, ProtectedSpan, Records};
use crate::{DiagnosticKind, Rule, Severity};

/// A sink for the records of one call.
pub(crate) trait Recorder {
    /// Whether the recorder keeps anything; the engine builds no record when it is false.
    const ACTIVE: bool;

    /// One record of one piece of an applied edit.
    fn change(&mut self, change: Change);

    /// A protected span of one piece.
    fn protected(&mut self, span: ProtectedSpan);

    /// A diagnostic, already reported at its first piece.
    fn diagnostic(&mut self, diagnostic: Diagnostic);

    /// Whether the recorder also traces the classification and the matches of every window (the
    /// invariant tests of design section 13.5, property 3); only a test recorder does.
    const TRACE: bool = false;

    /// One fragment of the classification of a window: its kind, its `LABEL` and `FROZEN` flags
    /// and the byte range of its tokens in the call's text.
    #[inline(always)]
    fn fragment(&mut self, _kind: FragKind, _label: bool, _frozen: bool, _range: Range<u32>) {}

    /// One match of a rule that the engine applied: the byte range from its anchor to its last
    /// consumed token, its `consume`, and whether it has edits.
    #[inline(always)]
    fn matched(&mut self, _rule: Rule, _range: Range<u32>, _consume: u8, _edits: bool) {}
}

/// Records nothing: the path of `typeset`.
pub(crate) struct NoRecord;

impl Recorder for NoRecord {
    const ACTIVE: bool = false;

    #[inline(always)]
    fn change(&mut self, _change: Change) {}

    #[inline(always)]
    fn protected(&mut self, _span: ProtectedSpan) {}

    #[inline(always)]
    fn diagnostic(&mut self, _diagnostic: Diagnostic) {}
}

/// Records diagnostics only: the path of `lint` (design section 12.3).
#[derive(Debug, Default)]
pub(crate) struct Diagnostics {
    diagnostics: Vec<Diagnostic>,
}

impl Diagnostics {
    /// The diagnostics in the order of design section 12.4.
    pub(crate) fn finish(mut self) -> Vec<Diagnostic> {
        self.diagnostics.sort_by_key(Diagnostic::sort_key);
        self.diagnostics
    }
}

impl Recorder for Diagnostics {
    const ACTIVE: bool = true;

    #[inline(always)]
    fn change(&mut self, _change: Change) {}

    #[inline(always)]
    fn protected(&mut self, _span: ProtectedSpan) {}

    fn diagnostic(&mut self, diagnostic: Diagnostic) {
        self.diagnostics.push(diagnostic);
    }
}

/// Records everything: the path of `report`.
#[derive(Debug, Default)]
pub(crate) struct Journal {
    records: Records,
}

impl Journal {
    /// The records in the order of design sections 2.9 and 12.4.
    pub(crate) fn finish(mut self) -> Records {
        self.records.changes.sort_by_key(Change::sort_key);
        self.records
            .protected
            .sort_by_key(|span| (span.piece(), span.range().start, span.range().end));
        self.records.diagnostics.sort_by_key(Diagnostic::sort_key);
        self.records
    }
}

impl Recorder for Journal {
    const ACTIVE: bool = true;

    fn change(&mut self, change: Change) {
        self.records.changes.push(change);
    }

    fn protected(&mut self, span: ProtectedSpan) {
        self.records.protected.push(span);
    }

    fn diagnostic(&mut self, diagnostic: Diagnostic) {
        self.records.diagnostics.push(diagnostic);
    }
}

/// Where a record lies before it is resolved to pieces.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Loc {
    /// A byte range of the call's text.
    Flat(u32, u32),
    /// An input piece that has no extent in the call's text: a slot or a protected piece. A
    /// diagnostic or a span that starts at such a piece is reported at it.
    Piece(u32),
}

/// The `Text` piece that holds an insertion at `at`: the piece that ends there when `left`, the
/// piece that starts there otherwise; a position inside a piece belongs to it either way.
pub(crate) fn insertion_piece(pieces: &[TextPiece], at: u32, left: bool) -> Option<&TextPiece> {
    let holds = |p: &&TextPiece| {
        if left {
            p.start < at && at <= p.end
        } else {
            p.start <= at && at < p.end
        }
    };
    let first = pieces.partition_point(|p| p.end < at);
    pieces.get(first..)?.iter().take(2).find(holds)
}

/// A recorder together with what resolves positions to pieces.
pub(crate) struct Sink<'a, R> {
    pub(crate) rec: &'a mut R,
    /// The `Text` pieces of the input with their ranges in the call's text, in order.
    pub(crate) pieces: &'a [TextPiece],
    /// The length of every input piece, by piece index; a protected piece is not part of the
    /// call's text, so its length may exceed `u32::MAX`.
    pub(crate) lens: &'a [usize],
}

/// The pieces that a range of the call's text covers, with the range in each.
fn split(
    pieces: &[TextPiece],
    start: u32,
    end: u32,
) -> impl Iterator<Item = (usize, Range<usize>)> + '_ {
    let first = pieces.partition_point(|p| p.end <= start);
    pieces
        .get(first..)
        .unwrap_or_default()
        .iter()
        .take_while(move |p| p.start < end)
        .filter_map(move |p| {
            let s = start.max(p.start);
            let e = end.min(p.end);
            (s < e).then(|| {
                (
                    p.piece as usize,
                    (s - p.start) as usize..(e - p.start) as usize,
                )
            })
        })
}

impl<R: Recorder> Sink<'_, R> {
    /// Records an applied edit of `rule` over `start..end` of the call's text: one record per piece
    /// it covers, the first holding the whole replacement and the others an empty one (design
    /// section 3.11). An insertion (an empty range) belongs to the `Text` piece that ends at its
    /// position when `left` is true (the token before it is text), and otherwise to the one that
    /// starts there.
    pub(crate) fn change(&mut self, rule: Rule, start: u32, end: u32, left: bool, text: &str) {
        if !R::ACTIVE {
            return;
        }
        if start == end {
            if let Some(p) = insertion_piece(self.pieces, start, left) {
                let at = (start - p.start) as usize;
                self.rec
                    .change(Change::new(rule, p.piece as usize, at..at, text.to_owned()));
            }
            return;
        }
        // The pieces outlive the borrow of the sink, so the records need no collected copy.
        let pieces = self.pieces;
        for (n, (piece, range)) in split(pieces, start, end).enumerate() {
            let replacement = if n == 0 { text } else { "" };
            self.rec
                .change(Change::new(rule, piece, range, replacement.to_owned()));
        }
    }

    /// Records a protected span: one record per piece it covers (design section 3.11).
    pub(crate) fn protected(&mut self, at: Loc, kind: ProtectedKind) {
        if !R::ACTIVE {
            return;
        }
        match at {
            Loc::Piece(piece) => {
                let len = match kind {
                    ProtectedKind::Slot => 0,
                    _ => self.lens.get(piece as usize).copied().unwrap_or(0),
                };
                self.rec
                    .protected(ProtectedSpan::new(piece as usize, 0..len, kind));
            }
            Loc::Flat(start, end) => {
                let pieces = self.pieces;
                for (piece, range) in split(pieces, start, end) {
                    self.rec.protected(ProtectedSpan::new(piece, range, kind));
                }
            }
        }
    }

    /// Records a diagnostic at its first piece, the range clipped to that piece (design section
    /// 3.11).
    pub(crate) fn diagnostic(&mut self, kind: DiagnosticKind, severity: Severity, at: Loc) {
        self.diagnostic_of(kind, severity, None, at);
    }

    /// Records a diagnostic that a rule raised.
    pub(crate) fn diagnostic_of(
        &mut self,
        kind: DiagnosticKind,
        severity: Severity,
        rule: Option<Rule>,
        at: Loc,
    ) {
        if !R::ACTIVE {
            return;
        }
        let (piece, range) = match at {
            Loc::Piece(piece) => (piece as usize, 0..0),
            Loc::Flat(start, end) => match split(self.pieces, start, end).next() {
                Some(first) => first,
                None => return,
            },
        };
        self.rec
            .diagnostic(Diagnostic::new(kind, severity, rule, piece, range));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_protected_piece_keeps_a_length_past_u32_max() {
        // A `Protected` piece is no part of the call's text, so its length is not bounded by the
        // `u32` limit of design section 3.4; its span covers all of it (design section 3.11).
        let len = usize::try_from(u64::from(u32::MAX) + 7).unwrap_or(usize::MAX);
        let mut rec = Journal::default();
        let mut sink = Sink {
            rec: &mut rec,
            pieces: &[],
            lens: &[3, len],
        };
        sink.protected(Loc::Piece(1), ProtectedKind::ProtectedPiece);
        sink.protected(Loc::Piece(0), ProtectedKind::Slot);
        let records = rec.finish();
        let spans: Vec<(usize, Range<usize>)> = records
            .protected
            .iter()
            .map(|s| (s.piece(), s.range()))
            .collect();
        assert_eq!(spans, [(0, 0..0), (1, 0..len)]);
    }
}
