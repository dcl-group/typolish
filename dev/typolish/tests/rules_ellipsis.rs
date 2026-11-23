//! The tables of `ellipsis.dots` and `ellipsis.inner` (design sections 8.1, 8.6 and 13.2).
//!
//! A row that names no configuration runs with the one rule of its table, alone
//! (`RuleSet::EMPTY.with(rule)`), and in the built-in sets that hold it, without the `nbsp.*`
//! rules while stage 3 is not delivered (section 15.4).

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

use common::{
    assert_changed_by, assert_changed_by_with_glue, check_in, check_rows, check_unchanged, configs,
    interface, only, prose, without_rule,
};
use probe::diagnostics_of;
use typolish::{
    DiagnosticKind, Language, Output, Piece, Rule, RuleSet, Severity, Slot, Typographer,
};

const DOTS: Rule = Rule::EllipsisDots;
const INNER: Rule = Rule::EllipsisInner;

// ellipsis.dots, Russian: exactly three full stops attached to what precedes them or alone; `?...`
// gives `?..`.
const RU_DOTS: &[(&str, &str)] = &[
    ("Подождите...", "Подождите…"),
    ("...", "…"),
    ("Ждите - ...", "Ждите - …"),
    ("и т.д...", "и т.д…"),
    ("Подождите...\nДальше", "Подождите…\nДальше"),
    ("Подождите...\tДальше", "Подождите…\tДальше"),
    ("Подождите...  Дальше", "Подождите…  Дальше"),
    ("... слово", "… слово"),
    ("Слово ...", "Слово …"),
    ("Подождите..., потом", "Подождите…, потом"),
    ("(Подождите...)", "(Подождите…)"),
    ("Что?...", "Что?.."),
    ("Что?...\nДа", "Что?..\nДа"),
    ("Что!...", "Что!.."),
    ("Что?!...", "Что?!.."),
    // The quote is closing by its role, so the ellipsis fires (section 6.3).
    ("Подождите...\" дальше", "Подождите…\" дальше"),
    // The dots attach to a number or a sign as well as to a word (design section 8.6).
    ("Осталось 5...", "Осталось 5…"),
    ("10%...", "10%…"),
    // Not three full stops, or digits on both sides, or not at a word end: nothing changes.
    ("А если..", "А если.."),
    ("../x", "../x"),
    ("0..5", "0..5"),
    ("....", "...."),
    ("Конец....", "Конец...."),
    ("-5...5", "-5...5"),
    ("a....b", "a....b"),
    ("a...b", "a...b"),
    ("...args", "...args"),
    ("Слово\t...и тогда", "Слово\t...и тогда"),
    // Typed forms never change.
    ("Подождите…", "Подождите…"),
    ("…", "…"),
    ("Что?…", "Что?…"),
    ("Что?..", "Что?.."),
    ("Что!..", "Что!.."),
    ("Что?!..", "Что?!.."),
];

// Prose changes `...и` and `a...b` with `ellipsis.inner`; those rows are not in the table above
// for the sets. `a...b`, `...args` and `Слово\t...и тогда` hold alone and in Interface only.
const RU_DOTS_NOT_PROSE: &[(&str, &str)] = &[
    ("a...b", "a...b"),
    ("...args", "...args"),
    ("Слово\t...и тогда", "Слово\t...и тогда"),
];

#[test]
fn dots_in_russian() {
    let all = configs(&Language::RU_RU, DOTS);
    assert_eq!(all.len(), 3);
    let (inner, rest): (Vec<_>, Vec<_>) = RU_DOTS
        .iter()
        .partition(|r| RU_DOTS_NOT_PROSE.iter().any(|n| n.0 == r.0));
    let rest: Vec<(&str, &str)> = rest.into_iter().copied().collect();
    let inner: Vec<(&str, &str)> = inner.into_iter().copied().collect();
    check_rows(&all, &rest);
    // Alone and in Interface: `ellipsis.inner` is not there.
    check_rows(&all[..1], &inner);
    check_rows(&all[2..], &inner);
    for (what, t) in &all {
        assert_changed_by(what, t, "Подождите...", DOTS);
        assert_changed_by_with_glue(what, t, "и т.д...", DOTS);
        assert_changed_by(what, t, "Что?...", DOTS);
    }
}

const EN_DOTS: &[(&str, &str)] = &[
    ("Wait...", "Wait…"),
    ("...", "…"),
    ("Wait... what", "Wait… what"),
    ("Wait...\nWhat", "Wait…\nWhat"),
    ("What?...", "What?…"),
    ("What!...", "What!…"),
    ("What?!...", "What?!…"),
    ("Slow ...", "Slow …"),
    ("Wait 5...", "Wait 5…"),
    ("... and then", "… and then"),
    ("Wait..", "Wait.."),
    ("Wait....", "Wait...."),
    ("0..5", "0..5"),
    ("../x", "../x"),
    ("-5...5", "-5...5"),
    ("Wait…", "Wait…"),
    ("What?…", "What?…"),
];

#[test]
fn dots_in_english() {
    check_rows(&configs(&Language::EN_US, DOTS), EN_DOTS);
}

#[test]
fn dots_in_common() {
    // `Language::COMMON`: the forms after `?`, `!` and `?!` are `?…`, `!…` and `?!…` (section 7.4).
    for (what, t) in configs(&Language::COMMON, DOTS) {
        for row in [
            ("Wait...", "Wait…"),
            ("...", "…"),
            ("Что?...", "Что?…"),
            ("Что!...", "Что!…"),
            ("Что?!...", "Что?!…"),
            ("Что?…", "Что?…"),
            ("Wait..", "Wait.."),
            ("Wait....", "Wait...."),
            ("0..5", "0..5"),
        ] {
            check_in(&what, &t, row);
        }
    }
}

// ellipsis.inner (Prose): `...` at the start of a word and between two words.
const INNER_ROWS: &[(&str, &str)] = &[
    ("...и тогда", "…и тогда"),
    ("да...нет", "да…нет"),
    ("Слово\t...и тогда", "Слово\t…и тогда"),
    ("...args", "…args"),
    ("a...b", "a…b"),
    // Not at a word start or between words: the other rule's patterns, or four dots, or two.
    ("a....b", "a....b"),
    ("a..b", "a..b"),
    ("да…нет", "да…нет"),
    ("…и тогда", "…и тогда"),
];

#[test]
fn inner() {
    for language in [Language::RU_RU, Language::EN_US, Language::COMMON] {
        let all = configs(&language, INNER);
        assert_eq!(
            all.len(),
            2,
            "alone and Prose: Interface has no ellipsis.inner"
        );
        check_rows(&all, INNER_ROWS);
        for (what, t) in &all {
            assert_changed_by(what, t, "да...нет", INNER);
            assert_changed_by_with_glue(what, t, "...и тогда", INNER);
        }
    }
    // Alone, `ellipsis.inner` leaves what `ellipsis.dots` takes.
    let t = only(Language::RU_RU, &[INNER]);
    for text in ["Подождите...", "...", "... слово", "Слово ...", "Что?..."] {
        check_unchanged("inner alone", &t, text);
    }
}

#[test]
fn inner_is_not_in_interface() {
    // Section 8.6: `...args` and `a...b` occur in code.
    for language in [Language::RU_RU, Language::EN_US] {
        let t = interface(language);
        for text in [
            "...args",
            "a...b",
            "да...нет",
            "...и тогда",
            "Слово\t...и тогда",
        ] {
            check_unchanged("Interface", &t, text);
        }
    }
}

#[test]
fn prose_runs_both_rules() {
    // `ellipsis.dots` takes the word end, `ellipsis.inner` the rest; the report names each.
    let t = prose(Language::RU_RU);
    check_in(
        "Prose",
        &t,
        ("Подождите... да...нет ...и", "Подождите… да…нет …и"),
    );
    let r = t.report("Подождите... да...нет ...и", Output::Unicode);
    let rules: Vec<_> = r.changes().iter().map(|c| c.rule()).collect();
    assert_eq!(rules, [DOTS, INNER, INNER]);
    for change in r.changes() {
        assert_eq!(r.original(change), "...");
        assert_eq!(change.replacement(), "…");
    }
}

#[test]
fn quotes_and_ellipsis() {
    // Section 13.2: the quote after the dots is closing by its role, so `ellipsis.dots` fires; the
    // quote has no opener, so the sets report `UnmatchedQuote` and keep it straight.
    let text = "Подождите...\" дальше";
    for t in [prose(Language::RU_RU), interface(Language::RU_RU)] {
        check_in("set", &t, (text, "Подождите…\" дальше"));
        let d: Vec<_> = diagnostics_of(&t, text)
            .into_iter()
            .map(|d| (d.0, d.1))
            .collect();
        assert_eq!(d, [(DiagnosticKind::UnmatchedQuote, Severity::Warning)]);
    }
    let t = only(Language::RU_RU, &[DOTS]);
    check_in("alone", &t, (text, "Подождите…\" дальше"));
    assert!(diagnostics_of(&t, text).is_empty());
}

#[test]
fn the_ellipsis_is_not_an_operand() {
    // `Ждите - ...` has no dash (a `Punctuation` fragment is not an operand, section 8.2); the
    // sets set the ellipsis and report no `AmbiguousHyphen` and freeze nothing.
    for t in [prose(Language::RU_RU), interface(Language::RU_RU)] {
        check_in("set", &t, ("Ждите - ...", "Ждите - …"));
        let r = t.report("Ждите - ...", Output::Unicode);
        assert!(r.protected().is_empty());
        assert!(
            r.diagnostics()
                .iter()
                .all(|d| d.kind() != &DiagnosticKind::AmbiguousHyphen)
        );
    }
    let t = only(Language::RU_RU, &[DOTS]);
    check_in("alone", &t, ("...", "…"));
    assert!(t.report("...", Output::Unicode).protected().is_empty());
}

#[test]
fn a_slot_next_to_the_ellipsis() {
    for (what, t) in configs(&Language::RU_RU, DOTS) {
        let out = t.typeset_template(
            [
                Piece::Text("... "),
                Piece::Slot("a", Slot::WORD),
                Piece::Text(" ..."),
            ],
            Output::Unicode,
        );
        assert_eq!(out.texts(), ["… ", " …"], "{what}");
        assert!(out.changed(), "{what}");
        let out = t.typeset_template(
            [Piece::Slot("a", Slot::WORD), Piece::Text("... дальше")],
            Output::Unicode,
        );
        assert_eq!(out.texts(), ["", "… дальше"], "{what}");
        // A `Number` slot after the dots is none of the followers of design section 8.6, and
        // `ellipsis.inner` counts only `Word` and `Other` slots as words: the dots stay.
        let out = t.typeset_template(
            [Piece::Text("Подождите..."), Piece::Slot("n", Slot::NUMBER)],
            Output::Unicode,
        );
        assert_eq!(out.texts(), ["Подождите...", ""], "{what}");
        assert!(!out.changed(), "{what}");
        assert!(matches!(out.texts()[0], Cow::Borrowed(_)), "{what}");
    }
}

#[test]
fn a_report_names_the_ellipsis_rule() {
    for (what, t) in configs(&Language::RU_RU, DOTS) {
        let r = t.report("Подождите...", Output::Unicode);
        assert_eq!(r.changes().len(), 1, "{what}");
        let change = &r.changes()[0];
        assert_eq!(change.rule(), DOTS, "{what}");
        assert_eq!(change.piece(), 0);
        assert_eq!(change.range(), 18..21, "{what}");
        assert_eq!(change.replacement(), "…", "{what}");
        assert_eq!(r.original(change), "...", "{what}");
    }
}

#[test]
fn the_rules_disabled() {
    for set in [RuleSet::PROSE, RuleSet::INTERFACE] {
        let t = without_rule(Language::RU_RU, set, DOTS);
        for text in ["Подождите...", "... слово", "Слово ...", "Что?..."] {
            check_unchanged("without ellipsis.dots", &t, text);
        }
    }
    let t = without_rule(Language::RU_RU, RuleSet::PROSE, INNER);
    for text in ["да...нет", "...и тогда", "...args"] {
        check_unchanged("without ellipsis.inner", &t, text);
    }
    // The other rule of the pair stays.
    check_in(
        "Prose without ellipsis.inner",
        &t,
        ("Подождите...", "Подождите…"),
    );
    let t = Typographer::new(Language::RU_RU, RuleSet::EMPTY);
    check_unchanged("empty", &t, "Подождите... да...нет");
}
