//! Helpers of the stage 2 tests that read `Report::protected()` and `Report::diagnostics()`
//! (design sections 2.9 and 12.5): the classifier, protection, window and lexer tables observe the
//! engine only through them.
//!
//! The module is not declared in `tests/common/mod.rs`, so that the test targets that need no
//! `report` keep building while the API is incomplete. A test file that needs it declares
//! `#[path = "common/probe.rs"] mod probe;` after `mod common;`.

#![allow(dead_code)]

use std::collections::BTreeSet;
use std::ops::Range;

use unicode_segmentation::UnicodeSegmentation;

use typolish::{
    Diagnostic, DiagnosticKind, Language, Output, ProtectedKind, ProtectedSpan, Rule, RuleSet,
    Severity, TechKind, Typographer,
};

use crate::common::show;

/// The rule sets under which the classification of a fragment must not change: protection does
/// not depend on the set (design section 10.2).
pub const SETS: [RuleSet; 4] = [
    RuleSet::EMPTY,
    RuleSet::INTERFACE,
    RuleSet::PROSE,
    RuleSet::ALL,
];

/// The built-in languages and `COMMON`.
pub fn all_languages() -> Vec<Language> {
    vec![Language::RU_RU, Language::EN_US, Language::COMMON]
}

/// A technical span of the given kind.
pub const fn tech(kind: TechKind) -> ProtectedKind {
    ProtectedKind::Technical(kind)
}

/// A span with its kind, its piece and its byte range.
pub type Span = (ProtectedKind, usize, Range<usize>);

/// Every protected span of a string, in the order of `protected()`, with the kind and the piece.
/// Checks the structure on the way: ranges are on character boundaries, inside the input, and the
/// records of one piece neither overlap nor run backwards.
#[track_caller]
pub fn spans_of(t: &Typographer, text: &str) -> Vec<Span> {
    let report = t.report(text, Output::Unicode);
    let mut out = Vec::new();
    let mut end = 0;
    for span in report.protected() {
        let range = span.range();
        assert_eq!(
            span.piece(),
            0,
            "string input has one piece: {}",
            show(text)
        );
        assert!(
            text.get(range.clone()).is_some(),
            "span {range:?} is not a slice of {}",
            show(text)
        );
        assert!(
            range.start >= end && range.start <= range.end,
            "spans overlap or run backwards in {}: {:?}",
            show(text),
            report.protected()
        );
        end = range.end;
        out.push((span.kind(), span.piece(), range));
    }
    out
}

/// The protected spans of a string as `(kind, text)`, in order.
#[track_caller]
pub fn span_texts<'a>(t: &Typographer, text: &'a str) -> Vec<(ProtectedKind, &'a str)> {
    spans_of(t, text)
        .into_iter()
        .map(|(kind, _, range)| (kind, &text[range]))
        .collect()
}

/// A diagnostic without its `Display`: kind, severity, piece and range.
pub type Diag = (DiagnosticKind, Severity, Option<Rule>, usize, Range<usize>);

fn diag_of(d: &Diagnostic) -> Diag {
    (
        d.kind().clone(),
        d.severity(),
        d.rule(),
        d.piece(),
        d.range(),
    )
}

/// Every diagnostic of a string.
pub fn diagnostics_of(t: &Typographer, text: &str) -> Vec<Diag> {
    t.report(text, Output::Unicode)
        .diagnostics()
        .iter()
        .map(diag_of)
        .collect()
}

/// The ranges of the diagnostics of one kind in a string.
pub fn diag_ranges(t: &Typographer, text: &str, kind: &DiagnosticKind) -> Vec<Range<usize>> {
    diagnostics_of(t, text)
        .into_iter()
        .filter(|d| &d.0 == kind)
        .map(|d| d.4)
        .collect()
}

/// The texts of the diagnostics of one kind in a string.
pub fn diag_texts<'a>(t: &Typographer, text: &'a str, kind: &DiagnosticKind) -> Vec<&'a str> {
    diag_ranges(t, text, kind)
        .into_iter()
        .map(|r| &text[r])
        .collect()
}

/// The range of the `n`-th (from 0) occurrence of `needle` in `text`.
#[track_caller]
pub fn nth(text: &str, needle: &str, n: usize) -> Range<usize> {
    let start = text
        .match_indices(needle)
        .nth(n)
        .unwrap_or_else(|| panic!("no occurrence {n} of {needle:?} in {text:?}"))
        .0;
    start..start + needle.len()
}

/// The records of a span list as `(piece, range, kind)`.
pub fn records(spans: &[ProtectedSpan]) -> Vec<(usize, Range<usize>, ProtectedKind)> {
    spans
        .iter()
        .map(|s| (s.piece(), s.range(), s.kind()))
        .collect()
}

/// Typographers for a sweep over languages and sets.
pub fn sweep(languages: &[Language]) -> Vec<(String, Typographer)> {
    let mut out = Vec::new();
    for language in languages {
        for set in SETS {
            out.push((
                format!("{language} {set}"),
                Typographer::new(language.clone(), set),
            ));
        }
    }
    out
}

/// The cluster boundaries of a string by the segmenter.
pub fn boundaries_of(text: &str) -> BTreeSet<usize> {
    let mut set: BTreeSet<usize> = text.grapheme_indices(true).map(|(i, _)| i).collect();
    set.insert(text.len());
    set
}

/// Checks one input through one typographer: edits and protected spans on cluster boundaries,
/// untouched clusters byte for byte in the output, and the output equals the application of the
/// changes.
#[track_caller]
pub fn check_clusters(t: &Typographer, label: &str, input: &str) {
    let report = t.report(input, Output::Unicode);
    let boundaries = boundaries_of(input);
    let what = || format!("{label}: {}", input.escape_unicode());

    let mut edits: Vec<(usize, usize, &str)> = Vec::new();
    for change in report.changes() {
        let range = change.range();
        assert_eq!(change.piece(), 0, "{}", what());
        assert!(
            boundaries.contains(&range.start) && boundaries.contains(&range.end),
            "an edit {range:?} falls inside a cluster: {}",
            what()
        );
        edits.push((range.start, range.end, change.replacement()));
    }
    for span in report.protected() {
        // A delimiter is matched on the text and may stand inside a cluster (a `Prepend`
        // character before a backtick); a frozen fragment is made of whole clusters.
        if !matches!(span.kind(), ProtectedKind::Technical(_)) {
            continue;
        }
        let range = span.range();
        assert!(
            boundaries.contains(&range.start) && boundaries.contains(&range.end),
            "a protected span {range:?} ({:?}) falls inside a cluster: {}",
            span.kind(),
            what()
        );
    }

    // Edits are ordered by position and do not overlap, except insertions at one point.
    let mut rebuilt = String::new();
    let mut at = 0;
    for (start, end, replacement) in &edits {
        assert!(*start >= at, "edits overlap or run backwards: {}", what());
        rebuilt.push_str(&input[at..*start]);
        rebuilt.push_str(replacement);
        at = *end;
    }
    rebuilt.push_str(&input[at..]);
    assert_eq!(
        report.result().as_ref(),
        rebuilt,
        "applying the changes does not give the result: {}",
        what()
    );

    // Every cluster that no replacement or removal covers is in the output byte for byte: the
    // text between two edits is a run of whole clusters, and it was copied unchanged above.
    for (cluster_at, cluster) in input.grapheme_indices(true) {
        let end = cluster_at + cluster.len();
        let covered = edits
            .iter()
            .any(|(s, e, _)| *s < end && *e > cluster_at && s != e);
        if !covered {
            assert!(
                rebuilt.contains(cluster),
                "a cluster {cluster:?} vanished: {}",
                what()
            );
        }
    }
}
