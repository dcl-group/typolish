//! The reference pairs of design section 13.4 (`tests/reference/{ru,en_us}.rs`): the 820 cases of
//! the scouting corpus with the output of the reference implementation (version 7.9.1, default
//! settings, recorded outside the repository).
//!
//! Every row is in one of two groups. `AGREE`: the output of the full Prose set equals the recorded
//! one. `DIVERGE`: the output of Prose equals the one written out in the row, differs from the
//! recorded one, and the row names one or more reason codes of design section 8.12. A row whose
//! input holds `{name}` slots is run as a template (the slot kinds are in `slot_kind`) and
//! compared as the template written out with its `{name}` slots. A failure prints the input, our
//! output, the recorded output and the `report` table; the summary per reason code is printed with
//! `--nocapture`.

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
#[path = "reference/en_us.rs"]
mod en_us;
#[path = "reference/ru.rs"]
mod ru;

use std::collections::BTreeMap;

use common::{Group, Reason, RefRow, show};
use typolish::{DiagnosticKind, Language, Output, Piece, RuleSet, Slot, Typographer};

/// The kind of the slot of a corpus template, by its name: counts and amounts are numbers,
/// `from` and `to` are dates, the rest are words or names.
fn slot_kind(name: &str) -> Slot {
    match name {
        "n" | "a" | "b" | "pct" | "price" => Slot::NUMBER,
        "from" | "to" => Slot::DATE,
        _ => Slot::WORD,
    }
}

/// The pieces of a template input: text and `{name}` slots.
fn pieces(input: &str) -> Vec<Piece<'_, &str>> {
    let mut pieces = Vec::new();
    let mut rest = input;
    while let Some(open) = rest.find('{') {
        let close = rest[open..]
            .find('}')
            .unwrap_or_else(|| panic!("unclosed slot in {input:?}"))
            + open;
        if open > 0 {
            pieces.push(Piece::Text(&rest[..open]));
        }
        let name = &rest[open + 1..close];
        pieces.push(Piece::Slot(name, slot_kind(name)));
        rest = &rest[close + 1..];
    }
    if !rest.is_empty() {
        pieces.push(Piece::Text(rest));
    }
    pieces
}

/// Our output for a row in Prose, a template written out with its slots, and a description of the
/// `report` for a failure message, and the kinds of the diagnostics of the report.
fn ours(t: &Typographer, row: &RefRow) -> (String, String, Vec<DiagnosticKind>) {
    if row.input.contains('{') {
        let out = t.typeset_template(pieces(row.input), Output::Unicode);
        let mut text = String::new();
        for (i, part) in out.texts().iter().enumerate() {
            text.push_str(part);
            if let Some((name, _)) = out.slots().get(i) {
                text.push('{');
                text.push_str(name);
                text.push('}');
            }
        }
        let report = t.report_template(pieces(row.input), Output::Unicode);
        let kinds = report
            .diagnostics()
            .iter()
            .map(|d| d.kind().clone())
            .collect();
        (text, report.to_string(), kinds)
    } else {
        let report = t.report(row.input, Output::Unicode);
        let kinds = report
            .diagnostics()
            .iter()
            .map(|d| d.kind().clone())
            .collect();
        (
            t.typeset(row.input, Output::Unicode).into_owned(),
            report.to_string(),
            kinds,
        )
    }
}

/// The diagnostic that design section 8.12 names for a reason, if any.
fn named_diagnostic(reason: Reason) -> Option<DiagnosticKind> {
    match reason {
        Reason::NoGuess => Some(DiagnosticKind::UnmatchedQuote),
        Reason::LookAlike => Some(DiagnosticKind::MixedScript),
        Reason::AmbiguousHyphen => Some(DiagnosticKind::AmbiguousHyphen),
        _ => None,
    }
}

/// Runs the rows of a table; returns the failures and counts the groups and the reasons.
fn run(
    language: Language,
    rows: &[RefRow],
    counts: &mut BTreeMap<Reason, usize>,
    groups: &mut (usize, usize),
) -> Vec<String> {
    let t = Typographer::new(language, RuleSet::PROSE);
    let mut failures = Vec::new();
    for row in rows {
        let (got, report, diagnostics) = ours(&t, row);
        let mut fail = |what: &str| {
            failures.push(format!(
                "{} ({what})\ninput    {}\nours     {}\nrecorded {}\n{report}",
                row.id,
                show(row.input),
                show(&got),
                show(row.recorded)
            ));
        };
        match row.group {
            Group::Agree => {
                groups.0 += 1;
                if got != row.recorded {
                    fail("AGREE: our output differs from the recorded one");
                }
            }
            Group::Diverge { ours, reasons } => {
                groups.1 += 1;
                if reasons.is_empty() {
                    fail("DIVERGE without a reason code");
                }
                if ours == row.recorded {
                    fail(
                        "DIVERGE: the written-out output equals the recorded one, the row is AGREE",
                    );
                }
                if got != ours {
                    fail(&format!(
                        "DIVERGE: our output differs from the written-out one {}",
                        show(ours)
                    ));
                }
                for reason in reasons {
                    *counts.entry(*reason).or_default() += 1;
                    // The diagnostic that design section 8.12 names for a reason of the row.
                    if let Some(kind) = named_diagnostic(*reason)
                        && !diagnostics.contains(&kind)
                    {
                        fail(&format!("DIVERGE: no {kind:?} diagnostic"));
                    }
                }
            }
        }
    }
    failures
}

#[test]
fn reference_rows_agree_or_diverge_with_a_reason() {
    let mut counts = BTreeMap::new();
    let mut groups = (0, 0);
    let mut failures = run(Language::RU_RU, ru::ROWS, &mut counts, &mut groups);
    failures.extend(run(Language::EN_US, en_us::ROWS, &mut counts, &mut groups));
    println!("reference rows: {} AGREE, {} DIVERGE", groups.0, groups.1);
    for reason in Reason::ALL {
        println!(
            "  {:<20}{}",
            format!("{reason:?}"),
            counts.get(&reason).copied().unwrap_or(0)
        );
    }
    assert!(
        failures.is_empty(),
        "{} failures:\n\n{}",
        failures.len(),
        failures.join("\n\n")
    );
}

#[test]
fn the_tables_hold_the_820_cases_once() {
    let mut ids: Vec<&str> = ru::ROWS.iter().chain(en_us::ROWS).map(|r| r.id).collect();
    assert_eq!(ids.len(), 820);
    ids.sort_unstable();
    ids.dedup();
    assert_eq!(ids.len(), 820, "case ids are unique");
    assert!(ru::ROWS.iter().all(|r| r.id.contains(".ru.")));
    assert!(en_us::ROWS.iter().all(|r| r.id.contains(".en-US.")));
    // A `{` marks a template input (`ours`); only the template families of design section 13.4
    // hold one, so a literal `{` elsewhere is not read as a slot by mistake.
    for row in ru::ROWS.iter().chain(en_us::ROWS) {
        if row.input.contains('{') {
            assert!(
                ["quote.subst.", "quote.already.", "mix."]
                    .iter()
                    .any(|p| row.id.starts_with(p)),
                "{}: a slot outside the template families of design section 13.4",
                row.id
            );
        }
    }
}

#[test]
fn every_reason_of_section_8_12_has_a_row() {
    let mut used: Vec<Reason> = Vec::new();
    for row in ru::ROWS.iter().chain(en_us::ROWS) {
        if let Group::Diverge { reasons, .. } = row.group {
            used.extend(reasons);
        }
    }
    for reason in Reason::ALL {
        assert!(used.contains(&reason), "no row for {reason:?}");
    }
}

#[test]
fn glue_only_rows_name_a_glue_reason() {
    // Design section 13.4: a row that differs from the recorded output only in its glue marks names
    // a reason that explains glue: one of the glue codes of design section 8.12, `DashStyleEn` for
    // the space before a spaced dash in en-US, or `Slots` for glue next to a slot.
    let fold = |s: &str| -> String {
        s.chars()
            .map(|c| {
                if matches!(c, '\u{a0}' | '\u{202f}') {
                    ' '
                } else {
                    c
                }
            })
            .collect()
    };
    let mut seen = 0;
    for row in ru::ROWS.iter().chain(en_us::ROWS) {
        let Group::Diverge { ours, reasons } = row.group else {
            continue;
        };
        if fold(row.recorded) != fold(ours) {
            continue;
        }
        seen += 1;
        for reason in reasons {
            assert!(
                matches!(
                    reason,
                    Reason::NoLastShortWord
                        | Reason::EnNbsp
                        | Reason::RuNbsp
                        | Reason::WideGlue
                        | Reason::DashStyleEn
                        | Reason::Slots
                ),
                "{}: differs only in glue, but names {reason:?}",
                row.id
            );
            if *reason == Reason::DashStyleEn {
                assert!(
                    row.id.contains(".en-US.") && row.recorded.contains("\u{a0}—"),
                    "{}: `DashStyleEn` on a glue-only row without a glued dash in en-US",
                    row.id
                );
            }
            if *reason == Reason::Slots {
                assert!(
                    row.input.contains('{'),
                    "{}: `Slots` on a glue-only row without a slot",
                    row.id
                );
            }
        }
    }
    assert!(seen > 0, "no row differs only in glue");
}
