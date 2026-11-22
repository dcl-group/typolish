//! Atoms, protection, windows and the classifier for pieces (design sections 3.3, 3.4, 3.6,
//! 10.1), observed through `report_template`.
//!
//! `report_template` came with `report` in stage 2 (design section 15.4). The string forms of the
//! same rows are in `tests/protection.rs` and `tests/classifier.rs`; the output of the rules that
//! read these decisions is checked by `tests/ingest_rules.rs`, which comes with those rules.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    clippy::print_stdout,
    clippy::print_stderr,
    clippy::dbg_macro
)]

mod common;
#[path = "common/probe.rs"]
mod probe;

use std::borrow::Cow;
use std::ops::Range;

use common::strategies::{
    GenPiece, PROTECTION_CONFIGS, borrow, delimiter_text, pieces, pieces_of,
    protection_typographer, risky_text, rule_set,
};
use probe::{SETS, records, tech};
use proptest::prelude::*;
use typolish::{
    DiagnosticKind, Language, Output, Piece, ProtectedKind, RuleSet, Severity, Slot, TechKind,
    Typographer,
};

const D: ProtectedKind = ProtectedKind::Delimited;
const SLOT: ProtectedKind = ProtectedKind::Slot;
const COLON: ProtectedKind = tech(TechKind::Colon);
const HYPHEN: ProtectedKind = tech(TechKind::CompoundHyphens);

fn interface() -> Typographer {
    Typographer::new(Language::RU_RU, RuleSet::INTERFACE)
}

type Pieces<'a> = Vec<Piece<'a, &'static str>>;

fn report_records(
    t: &Typographer,
    pieces: Pieces<'_>,
) -> Vec<(usize, Range<usize>, ProtectedKind)> {
    let report = t.report_template(pieces, Output::Unicode);
    records(report.protected())
}

/// The records of one kind.
fn of_kind(
    all: &[(usize, Range<usize>, ProtectedKind)],
    kind: ProtectedKind,
) -> Vec<(usize, Range<usize>)> {
    all.iter()
        .filter(|r| r.2 == kind)
        .map(|r| (r.0, r.1.clone()))
        .collect()
}

#[test]
fn a_delimited_span_over_pieces_gives_one_record_per_piece() {
    // Section 13.2, `tests/api.rs`: two `Delimited` records, `0:4..7` and `1:0..5`.
    let t = interface();
    let got = report_records(&t, vec![Piece::Text("Run `ls"), Piece::Text(" -la`")]);
    assert_eq!(of_kind(&got, D), [(0, 4..7), (1, 0..5)]);
}

#[test]
fn delimiters_are_matched_across_a_slot() {
    // Section 10.1: the command is protected with the slot inside, and the slot stays a slot.
    let t = interface();
    let got = report_records(
        &t,
        vec![
            Piece::Text("Run `git checkout -- "),
            Piece::Slot("b", Slot::WORD),
            Piece::Text("` to reset."),
        ],
    );
    assert_eq!(of_kind(&got, D), [(0, 4..21), (2, 0..1)]);
    assert_eq!(
        got.iter()
            .filter(|r| r.2 == SLOT)
            .map(|r| r.0)
            .collect::<Vec<_>>(),
        [1]
    );
    // No technical span inside the command: it is one protected span.
    assert!(
        got.iter()
            .all(|r| !matches!(r.2, ProtectedKind::Technical(_)))
    );
}

#[test]
fn a_delimiter_split_between_pieces() {
    let t = Typographer::builder(Language::EN_US, RuleSet::INTERFACE)
        .protect_between("{{", "}}")
        .build()
        .unwrap();
    let got = report_records(
        &t,
        vec![Piece::Text("a {"), Piece::Text("{b}"), Piece::Text("} c")],
    );
    assert_eq!(of_kind(&got, D), [(0, 2..3), (1, 0..3), (2, 0..1)]);
    // The two backticks of a double run split between pieces.
    let got = report_records(
        &interface(),
        vec![Piece::Text("a `"), Piece::Text("`b`` c")],
    );
    assert_eq!(of_kind(&got, D), [(0, 2..3), (1, 0..4)]);
}

#[test]
fn markup_inside_a_span_is_not_protected_text() {
    let t = interface();
    let got = report_records(
        &t,
        vec![Piece::Text("`a "), Piece::Markup("<b>"), Piece::Text(" c`")],
    );
    assert_eq!(of_kind(&got, D), [(0, 0..3), (2, 0..3)]);
    assert!(
        got.iter().all(|r| r.0 != 1),
        "markup has no record: {got:?}"
    );
}

#[test]
fn protected_pieces_and_slots_have_records() {
    let t = interface();
    let got = report_records(
        &t,
        vec![
            Piece::Text("a "),
            Piece::Protected("x y"),
            Piece::Text(" b "),
            Piece::Slot("n", Slot::NUMBER),
            Piece::Markup("<br>"),
            Piece::Slot("w", Slot::WORD),
        ],
    );
    assert_eq!(of_kind(&got, ProtectedKind::ProtectedPiece), [(1, 0..3)]);
    let slots: Vec<usize> = got.iter().filter(|r| r.2 == SLOT).map(|r| r.0).collect();
    assert_eq!(slots, [3, 5]);
    assert!(got.iter().all(|r| r.0 != 4));
}

#[test]
fn an_empty_list_of_pieces() {
    let t = interface();
    let report = t.report_template(Vec::<Piece<'_, ()>>::new(), Output::Unicode);
    assert_eq!(report.result().texts(), [""]);
    assert!(matches!(report.result().texts()[0], Cow::Borrowed(_)));
    assert!(report.result().slots().is_empty());
    assert!(!report.result().changed());
    assert!(report.protected().is_empty());
    assert!(report.diagnostics().is_empty());
}

#[test]
fn windows_with_slots() {
    let t = interface();
    // A slot is never a boundary: `Text("a\n")`, `Slot`, `Text("\nb")` is one window.
    let got = report_records(
        &t,
        vec![
            Piece::Text("`a\n"),
            Piece::Slot("s", Slot::OTHER),
            Piece::Text("\nb`"),
        ],
    );
    assert_eq!(of_kind(&got, D), [(0, 0..3), (2, 0..3)]);
    // Two line breaks in one text after a slot are a boundary.
    let got = report_records(
        &t,
        vec![
            Piece::Text("`a"),
            Piece::Slot("s", Slot::OTHER),
            Piece::Text("\n\nb`"),
        ],
    );
    assert!(of_kind(&got, D).is_empty());
    // Adjacent texts are read as one: the line breaks at the end of one and the start of the
    // next make a boundary.
    let got = report_records(&t, vec![Piece::Text("`a\n"), Piece::Text("\nb`")]);
    assert!(of_kind(&got, D).is_empty());
    let got = report_records(&t, vec![Piece::Text("`a\n"), Piece::Text("b`")]);
    assert_eq!(of_kind(&got, D), [(0, 0..3), (1, 0..2)]);
    // Markup and protected pieces are content as well.
    let got = report_records(
        &t,
        vec![
            Piece::Text("`a\n"),
            Piece::Markup("<b>"),
            Piece::Text("\nb`"),
        ],
    );
    assert_eq!(of_kind(&got, D), [(0, 0..3), (2, 0..3)]);
    let got = report_records(
        &t,
        vec![
            Piece::Text("`a\n"),
            Piece::Protected("x"),
            Piece::Text("\nb`"),
        ],
    );
    let spans = of_kind(&got, D);
    assert!(
        spans.contains(&(0, 0..3)) && spans.contains(&(2, 0..3)),
        "{spans:?}"
    );
}

#[test]
fn empty_markup_and_protected_pieces_are_atoms() {
    // Sections 2.6 and 3.3: a markup or protected piece is content and ends a token whether or not
    // its text is empty; only empty `Text` pieces are skipped.
    let t = interface();
    for piece in [Piece::Markup(""), Piece::Protected("")] {
        let got = report_records(&t, vec![Piece::Text("`a\n"), piece, Piece::Text("\nb`")]);
        assert_eq!(of_kind(&got, D), [(0, 0..3), (2, 0..3)], "{piece:?}");
        let report = t.report_template(
            vec![Piece::Text("`a\n"), piece, Piece::Text("\nb`")],
            Output::Unicode,
        );
        assert!(report.diagnostics().is_empty(), "{piece:?}");
    }
    // An empty protected piece has its record, with the empty range of its piece.
    let got = report_records(
        &t,
        vec![Piece::Text("a "), Piece::Protected(""), Piece::Text(" b")],
    );
    assert_eq!(of_kind(&got, ProtectedKind::ProtectedPiece), [(1, 0..0)]);
    // An empty markup piece ends a token: `host` and `:8080` are two fragments, as with `<b>`.
    for markup in ["", "<b>"] {
        let got = report_records(
            &t,
            vec![
                Piece::Text("host"),
                Piece::Markup(markup),
                Piece::Text(":8080"),
            ],
        );
        assert_eq!(
            got,
            report_records(
                &t,
                vec![
                    Piece::Text("host"),
                    Piece::Markup("<i>"),
                    Piece::Text(":8080"),
                ],
            ),
            "{markup:?}"
        );
    }
}

#[test]
fn a_fenced_block_across_a_slot() {
    let t = interface();
    let got = report_records(
        &t,
        vec![
            Piece::Text("```\nx"),
            Piece::Slot("s", Slot::OTHER),
            Piece::Text("\n\ny\n```"),
        ],
    );
    assert_eq!(of_kind(&got, D), [(0, 0..5), (2, 0..7)]);
}

#[test]
fn unclosed_delimiters_are_reported_at_the_first_piece() {
    let t = interface();
    let report = t.report_template(
        vec![
            Piece::<&str>::Text("a `b"),
            Piece::Slot("s", Slot::OTHER),
            Piece::Text("\n\nc`"),
        ],
        Output::Unicode,
    );
    let got: Vec<_> = report
        .diagnostics()
        .iter()
        .filter(|d| d.kind() == &DiagnosticKind::UnclosedDelimiter)
        .map(|d| (d.piece(), d.range()))
        .collect();
    assert_eq!(got, [(0, 2..3), (2, 3..4)]);
}

// Classifier: slots belong to fragments like text (section 3.6).

fn technical(t: &Typographer, pieces: Pieces<'_>) -> Vec<(usize, Range<usize>, ProtectedKind)> {
    report_records(t, pieces)
        .into_iter()
        .filter(|r| matches!(r.2, ProtectedKind::Technical(_)))
        .collect()
}

#[test]
fn a_slot_is_part_of_its_fragment() {
    let t = interface();
    // `{host}:{port}` is one fragment: a colon fragment, frozen whole.
    let got = technical(
        &t,
        vec![
            Piece::Slot("host", Slot::WORD),
            Piece::Text(":"),
            Piece::Slot("port", Slot::NUMBER),
        ],
    );
    assert!(got.contains(&(1, 0..1, COLON)), "{got:?}");
    // A space makes two fragments.
    let got = technical(
        &t,
        vec![
            Piece::Slot("host", Slot::WORD),
            Piece::Text(": "),
            Piece::Slot("port", Slot::NUMBER),
        ],
    );
    assert!(got.is_empty(), "{got:?}");
}

#[test]
fn slot_ranges_and_compounds() {
    let t = interface();
    let dash = |a: Slot, b: Slot| {
        technical(
            &t,
            vec![Piece::Slot("a", a), Piece::Text("-"), Piece::Slot("b", b)],
        )
    };
    // Two `Number` slots and two `Date` slots form a `Range`; nothing is frozen.
    assert!(dash(Slot::NUMBER, Slot::NUMBER).is_empty());
    assert!(dash(Slot::DATE, Slot::DATE).is_empty());
    // Anywhere else a slot counts as letters: a compound word with a frozen hyphen.
    for (a, b) in [
        (Slot::WORD, Slot::WORD),
        (Slot::NUMBER, Slot::DATE),
        (Slot::DATE, Slot::NUMBER),
        (Slot::NUMBER, Slot::WORD),
    ] {
        assert_eq!(
            dash(a, b),
            [(1, 0..1, HYPHEN)],
            "{:?} - {:?}",
            a.kind(),
            b.kind()
        );
    }
    // A `Number` slot and a literal number form a range too.
    assert!(technical(&t, vec![Piece::Slot("a", Slot::NUMBER), Piece::Text("-5")]).is_empty());
    assert!(technical(&t, vec![Piece::Text("5-"), Piece::Slot("b", Slot::NUMBER)]).is_empty());
    // A `Date` slot and a literal number are letters and a number: a compound.
    assert_eq!(
        technical(&t, vec![Piece::Text("5-"), Piece::Slot("d", Slot::DATE)]),
        [(0, 1..2, HYPHEN)]
    );
}

#[test]
fn a_quoted_slot_has_the_core_of_the_slot() {
    let t = interface();
    let got = report_records(
        &t,
        vec![
            Piece::Text("«"),
            Piece::Slot("x", Slot::OTHER),
            Piece::Text("»"),
        ],
    );
    assert_eq!(got.iter().filter(|r| r.2 == SLOT).count(), 1);
    assert!(
        got.iter()
            .all(|r| !matches!(r.2, ProtectedKind::Technical(_)))
    );
}

#[test]
fn label_and_sign_cores_with_slots_are_not_frozen() {
    let t = interface();
    for pieces in [
        vec![Piece::Text("№"), Piece::Slot("n", Slot::NUMBER)],
        vec![Piece::Text("№ "), Piece::Slot("n", Slot::NUMBER)],
        vec![
            Piece::Text("§§ "),
            Piece::Slot("n", Slot::NUMBER),
            Piece::Text("-7"),
        ],
    ] {
        assert!(technical(&t, pieces).is_empty());
    }
}

// Diagnostics of the classifier for pieces (sections 5.7, 12.4).

#[test]
fn mixed_script_across_pieces_is_one_diagnostic_at_the_first_piece() {
    let t = interface();
    let report = t.report_template(
        vec![Piece::<()>::Text("Мо"), Piece::Text("sква")],
        Output::Unicode,
    );
    let got: Vec<_> = report
        .diagnostics()
        .iter()
        .map(|d| (d.kind().clone(), d.severity(), d.piece(), d.range()))
        .collect();
    assert_eq!(
        got,
        [(DiagnosticKind::MixedScript, Severity::Warning, 0, 0..4)]
    );
}

#[test]
fn slot_glue() {
    let t = interface();
    let glue = |pieces: Pieces<'_>| -> Vec<(Severity, usize)> {
        t.report_template(pieces, Output::Unicode)
            .diagnostics()
            .iter()
            .filter(|d| d.kind() == &DiagnosticKind::SlotGlue)
            .map(|d| (d.severity(), d.piece()))
            .collect()
    };
    // A `Number` slot glued to letters: a warning.
    assert_eq!(
        glue(vec![Piece::Slot("n", Slot::NUMBER), Piece::Text("МБ")])
            .iter()
            .map(|g| g.0)
            .collect::<Vec<_>>(),
        [Severity::Warning]
    );
    // Two slots glued together: info.
    assert_eq!(
        glue(vec![
            Piece::Slot("a", Slot::WORD),
            Piece::Slot("b", Slot::WORD)
        ])
        .iter()
        .map(|g| g.0)
        .collect::<Vec<_>>(),
        [Severity::Info]
    );
    // With a space, or with punctuation between them, nothing is glued.
    assert!(glue(vec![Piece::Slot("n", Slot::NUMBER), Piece::Text(" МБ")]).is_empty());
    assert!(
        glue(vec![
            Piece::Slot("a", Slot::WORD),
            Piece::Text(" "),
            Piece::Slot("b", Slot::WORD)
        ])
        .is_empty()
    );
    assert!(
        glue(vec![
            Piece::Slot("a", Slot::WORD),
            Piece::Text(", "),
            Piece::Slot("b", Slot::WORD)
        ])
        .is_empty()
    );
}

// Pieces and strings (property 10 of section 13.5, protection part): the same protected spans as
// those of the concatenation, in absolute offsets.

fn absolute(
    records: &[(usize, Range<usize>, ProtectedKind)],
    offsets: &[usize],
) -> Vec<(ProtectedKind, usize, usize)> {
    // One record per piece of a span: merge adjacent records of one kind into one span.
    let mut out: Vec<(ProtectedKind, usize, usize)> = Vec::new();
    for (piece, range, kind) in records {
        let start = offsets[*piece] + range.start;
        let end = offsets[*piece] + range.end;
        match out.last_mut() {
            Some(last) if last.0 == *kind && last.2 == start => last.2 = end,
            _ => out.push((*kind, start, end)),
        }
    }
    out
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(512))]

    #[test]
    fn text_pieces_protect_like_their_concatenation(
        text in risky_text(), lang in 0usize..3, set in 0usize..4, cut in any::<prop::sample::Index>(),
    ) {
        let language = [Language::RU_RU, Language::EN_US, Language::COMMON][lang].clone();
        let t = Typographer::new(language, SETS[set]);
        let boundaries: Vec<usize> = text.char_indices().map(|(i, _)| i).chain([text.len()]).collect();
        let at = boundaries[cut.index(boundaries.len())];
        let (a, b) = text.split_at(at);
        let whole = t.report(&text, Output::Unicode);
        let parts = t.report_template::<()>([Piece::Text(a), Piece::Text(b)], Output::Unicode);
        let whole_spans: Vec<(ProtectedKind, usize, usize)> = whole
            .protected()
            .iter()
            .map(|s| (s.kind(), s.range().start, s.range().end))
            .collect();
        let part_spans = absolute(&records(parts.protected()), &[0, a.len()]);
        // A span that the cut splits is two records of adjacent pieces; they merge.
        let mut merged: Vec<(ProtectedKind, usize, usize)> = Vec::new();
        for s in whole_spans {
            match merged.last_mut() {
                Some(last) if last.0 == s.0 && last.2 == s.1 => last.2 = s.2,
                _ => merged.push(s),
            }
        }
        prop_assert_eq!(part_spans, merged);
        let whole_diags: Vec<_> = whole.diagnostics().iter().map(|d| d.kind().clone()).collect();
        let part_diags: Vec<_> = parts.diagnostics().iter().map(|d| d.kind().clone()).collect();
        prop_assert_eq!(part_diags, whole_diags);
    }
}

/// The structure of a report of pieces (property 14): the records are slices of their pieces, in
/// order, and `report_template` gives what `typeset_template` gives.
fn assert_template(t: &Typographer, parts: &[GenPiece]) -> Result<(), TestCaseError> {
    let input = borrow(parts);
    let report = t.report_template(input.iter().copied(), Output::Unicode);
    let plain = t.typeset_template(input.iter().copied(), Output::Unicode);
    prop_assert_eq!(report.result().texts(), plain.texts());
    prop_assert_eq!(
        report.result().texts().len(),
        report.result().slots().len() + 1
    );
    let mut last = (0usize, 0usize);
    for span in report.protected() {
        prop_assert!(span.piece() < parts.len(), "{:?}", report.protected());
        let text = match &parts[span.piece()] {
            GenPiece::Text(text) | GenPiece::Protected(text) => Some(text.as_str()),
            GenPiece::Slot(..) => None,
            GenPiece::Markup(_) => {
                prop_assert!(false, "a record for markup: {:?}", report.protected());
                None
            }
        };
        if let Some(text) = text {
            prop_assert!(text.get(span.range()).is_some());
        }
        let key = (span.piece(), span.range().start);
        prop_assert!(
            key >= last,
            "records out of order: {:?}",
            report.protected()
        );
        last = key;
    }
    let slots = parts
        .iter()
        .filter(|p| matches!(p, GenPiece::Slot(..)))
        .count();
    let slot_records = report
        .protected()
        .iter()
        .filter(|s| s.kind() == SLOT)
        .count();
    prop_assert_eq!(slot_records, slots);
    for d in report.diagnostics() {
        prop_assert!(d.piece() < parts.len().max(1));
    }
    Ok(())
}

/// Property 10 for own protection pairs: Text pieces cut anywhere protect like their
/// concatenation.
fn assert_cut(t: &Typographer, text: &str, cut: prop::sample::Index) -> Result<(), TestCaseError> {
    let boundaries: Vec<usize> = text
        .char_indices()
        .map(|(i, _)| i)
        .chain([text.len()])
        .collect();
    let at = boundaries[cut.index(boundaries.len())];
    let (a, b) = text.split_at(at);
    let whole = t.report(text, Output::Unicode);
    let parts = t.report_template::<()>([Piece::Text(a), Piece::Text(b)], Output::Unicode);
    let mut merged: Vec<(ProtectedKind, usize, usize)> = Vec::new();
    for s in whole.protected() {
        let (start, end) = (s.range().start, s.range().end);
        match merged.last_mut() {
            Some(last) if last.0 == s.kind() && last.2 == start => last.2 = end,
            _ => merged.push((s.kind(), start, end)),
        }
    }
    prop_assert_eq!(absolute(&records(parts.protected()), &[0, a.len()]), merged);
    let whole_diags: Vec<_> = whole
        .diagnostics()
        .iter()
        .map(|d| d.kind().clone())
        .collect();
    let part_diags: Vec<_> = parts
        .diagnostics()
        .iter()
        .map(|d| d.kind().clone())
        .collect();
    prop_assert_eq!(part_diags, whole_diags);
    Ok(())
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(2000))]

    /// 14. No panic on arbitrary pieces; the records are slices of their pieces, in order, and
    /// `report_template` gives what `typeset_template` gives.
    #[test]
    fn report_template_is_well_formed(parts in pieces(), lang in 0usize..3, set in rule_set()) {
        let language = [Language::RU_RU, Language::EN_US, Language::COMMON][lang].clone();
        let t = Typographer::new(language, set);
        assert_template(&t, &parts)?;
    }

    /// The same with own protection pairs and backticks off (design section 10.1).
    #[test]
    fn report_template_is_well_formed_with_own_protection_pairs(
        parts in pieces_of(delimiter_text),
        config in 0usize..PROTECTION_CONFIGS,
        lang in 0usize..3,
        set in rule_set(),
    ) {
        let language = [Language::RU_RU, Language::EN_US, Language::COMMON][lang].clone();
        assert_template(&protection_typographer(config, language, set), &parts)?;
    }

    /// Property 10 with own protection pairs: a span over the cut is matched like in the string.
    #[test]
    fn text_pieces_protect_like_their_concatenation_with_own_protection_pairs(
        text in delimiter_text(),
        config in 0usize..PROTECTION_CONFIGS,
        lang in 0usize..3,
        cut in any::<prop::sample::Index>(),
    ) {
        let language = [Language::RU_RU, Language::EN_US, Language::COMMON][lang].clone();
        assert_cut(&protection_typographer(config, language, RuleSet::INTERFACE), &text, cut)?;
    }
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(1_000_000))]

    /// 14. The long run of design section 13.5 for pieces. Run with `cargo test -- --ignored`.
    #[test]
    #[ignore = "long run of design section 13.5, property 14"]
    fn no_panic_on_arbitrary_pieces_long(
        parts in prop_oneof![pieces(), pieces_of(delimiter_text)],
        config in 0usize..=PROTECTION_CONFIGS,
        lang in 0usize..3,
        set in rule_set(),
    ) {
        let language = [Language::RU_RU, Language::EN_US, Language::COMMON][lang].clone();
        let t = if config == PROTECTION_CONFIGS {
            Typographer::new(language, set)
        } else {
            protection_typographer(config, language, set)
        };
        assert_template(&t, &parts)?;
    }
}
