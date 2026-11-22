//! The tables of `range.placeholders`, `range.digits`, `range.context` and `range.time` (design
//! sections 8.1, 8.5, 10.6 and 13.2).
//!
//! A row that names no configuration runs with the one rule of its table, alone
//! (`RuleSet::EMPTY.with(rule)`), and in the built-in sets that hold it, in full since stage 3
//! (section 15.4): the glue that the full sets add after a range (`2010–11⍽гг.`) is part of the
//! outputs there (`common::set_glue`); the rows of `ranges_in_the_full_sets` spell it out.

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

use common::{
    assert_changed_by, assert_changed_by_with_glue, check_in, check_rows, check_unchanged, configs,
    interface, only, prose, without_rule,
};
use probe::{diagnostics_of, spans_of, tech};
use typolish::{DiagnosticKind, Language, Output, Piece, Rule, RuleSet, Slot, TechKind};

const PLACEHOLDERS: Rule = Rule::RangePlaceholders;
const DIGITS: Rule = Rule::RangeDigits;
const CONTEXT: Rule = Rule::RangeContext;
const TIME: Rule = Rule::RangeTime;

fn languages() -> [Language; 3] {
    [Language::RU_RU, Language::EN_US, Language::COMMON]
}

// range.digits: a core `D+-D+` with exactly two numbers, no leading zeros, left smaller than right.
// The rows hold in every language, including COMMON (section 7.4), alone and in Prose.
const DIGITS_ROWS: &[(&str, &str)] = &[
    ("1941-1945", "1941–1945"),
    ("10-20", "10–20"),
    ("5-10", "5–10"),
    ("1-2", "1–2"),
    ("8000-8100", "8000–8100"),
    ("ports 8000-8100", "ports 8000–8100"),
    ("Страницы 10-20.", "Страницы 10–20."),
    ("Pages 10-20.", "Pages 10–20."),
    ("(5-10)", "(5–10)"),
    ("5-10, 12-15", "5–10, 12–15"),
    // Not firing at each boundary of the pattern: equal, descending, a leading zero, a third
    // number, a year and month, a spaced or half-spaced hyphen. None of them reports anything.
    ("10-10", "10-10"),
    ("20-10", "20-10"),
    ("01-12", "01-12"),
    ("1-2-3", "1-2-3"),
    ("5-10-15", "5-10-15"),
    ("2026-11", "2026-11"),
    ("2026-11-21", "2026-11-21"),
    ("0.2.27", "0.2.27"),
    ("5- 10", "5- 10"),
    ("5 - 10", "5 - 10"),
    // Shapes that range.digits refuses on purpose (the context path takes them).
    ("XIX-XX", "XIX-XX"),
    ("1941-45", "1941-45"),
    ("x-5", "x-5"),
    ("5-x", "5-x"),
    // Typeset input stays, borrowed; a typed em dash is not corrected (section 4.5).
    ("5–10", "5–10"),
    ("1941–1945", "1941–1945"),
    ("10—20", "10—20"),
];

#[test]
fn digits() {
    for language in languages() {
        let all = configs(&language, DIGITS);
        assert_eq!(
            all.len(),
            2,
            "alone and Prose: Interface has no range.digits"
        );
        check_rows(&all, DIGITS_ROWS);
        for (what, t) in &all {
            assert_changed_by(what, t, "1941-1945", DIGITS);
            // None of the refusals reports a diagnostic.
            for text in ["10-10", "20-10", "01-12", "1-2-3", "2026-11"] {
                let kinds = diagnostics_of(t, text);
                assert!(kinds.is_empty(), "{what}: {text}: {kinds:?}");
            }
        }
    }
}

#[test]
fn digits_refuses_decimals() {
    let ru = configs(&Language::RU_RU, DIGITS);
    check_rows(
        &ru,
        &[("1,5-2,5", "1,5-2,5"), ("в 1,5-2,5 раза", "в 1,5-2,5 раза")],
    );
    let en = configs(&Language::EN_US, DIGITS);
    check_rows(
        &en,
        &[
            ("1.5-2.5", "1.5-2.5"),
            ("about 1.5-2.5 times", "about 1.5-2.5 times"),
        ],
    );
}

#[test]
fn digits_is_not_in_interface() {
    // Section 10.6: Interface has no literal path: a port range never changes there.
    for language in languages() {
        let t = interface(language);
        for text in [
            "8000-8100",
            "ports 8000-8100",
            "Порты 8000-8100 с балансировкой",
            "1941-1945",
            "5-10",
            "1941-1945 гг.",
            "10:00-12:30",
            "XIX-XX вв.",
            "5-10 кг",
        ] {
            check_unchanged("Interface", &t, text);
        }
    }
}

#[test]
fn digits_with_a_slot_next_to_it() {
    for (what, t) in configs(&Language::RU_RU, DIGITS) {
        let out = t.typeset_template(
            [Piece::Text("5-10 "), Piece::Slot("s", Slot::OTHER)],
            Output::Unicode,
        );
        assert_eq!(out.texts(), ["5–10 ", ""], "{what}");
        let out = t.typeset_template(
            [Piece::Slot("s", Slot::WORD), Piece::Text(" 5-10")],
            Output::Unicode,
        );
        assert_eq!(out.texts(), ["", " 5–10"], "{what}");
        // A slot glued to a number is no longer a `D+-D+` core.
        let out = t.typeset_template(
            [Piece::Slot("a", Slot::NUMBER), Piece::Text("-5")],
            Output::Unicode,
        );
        assert_eq!(out.texts(), ["", "-5"], "{what}");
        assert!(!out.changed(), "{what}");
    }
}

#[test]
fn digits_disabled() {
    for language in languages() {
        let t = without_rule(language, RuleSet::PROSE, DIGITS);
        check_unchanged("Prose without range.digits", &t, "1941-1945");
        check_unchanged("Prose without range.digits", &t, "ports 8000-8100");
    }
}

// range.context: `N-N` (integers, decimals, Roman numerals, `YYYY-YY`) with a unit, a year or a
// century word after one pattern space, or a range lead word before it. Russian.
const RU_CONTEXT: &[(&str, &str)] = &[
    ("1941-45 гг.", "1941–45 гг."),
    ("1941-1945 гг.", "1941–1945 гг."),
    ("XIX-XX вв.", "XIX–XX вв."),
    ("IV-VI вв.", "IV–VI вв."),
    ("1,5-2,5 кг", "1,5–2,5 кг"),
    ("5-10 кг", "5–10 кг"),
    ("стр. 9-12", "стр. 9–12"),
    ("с. 9-12", "с. 9–12"),
    ("в 2010-11 гг.", "в 2010–11 гг."),
    ("сезон 2011-12 годов", "сезон 2011–12 годов"),
    ("Ждать 1,5-2,5 с.", "Ждать 1,5–2,5 с."),
    // The tail is not greater than the last two digits of the year; no year word; no context.
    ("2026-11 гг.", "2026-11 гг."),
    ("2010-11", "2010-11"),
    ("в 2010-11", "в 2010-11"),
    ("XX-XIX вв.", "XX-XIX вв."),
    ("xix-xx вв.", "xix-xx вв."),
    ("20-10 кг", "20-10 кг"),
    ("стр. 010-020", "стр. 010-020"),
    ("1,5-2,5", "1,5-2,5"),
    ("XIX-XX", "XIX-XX"),
    // An ambiguous unit counts only where `nbsp.units` would glue it (section 8.5).
    ("Ждать 1,5-2,5 с друзьями", "Ждать 1,5-2,5 с друзьями"),
    // One pattern space: a run of spaces or a tab breaks the context.
    ("1941-45  гг.", "1941-45  гг."),
    ("1941-45\tгг.", "1941-45\tгг."),
    ("1,5-2,5\tкг", "1,5-2,5\tкг"),
    // Typeset input stays.
    ("1941–45 гг.", "1941–45 гг."),
    ("XIX–XX вв.", "XIX–XX вв."),
];

// English: units and the page words `p.` and `pp.`; no year or century words.
const EN_CONTEXT: &[(&str, &str)] = &[
    ("5-10 kg", "5–10 kg"),
    ("5-10 MB", "5–10 MB"),
    ("1.5-2.5 kg", "1.5–2.5 kg"),
    ("p. 9-12", "p. 9–12"),
    ("pp. 9-12", "pp. 9–12"),
    ("See p. 9-12.", "See p. 9–12."),
    ("20-10 kg", "20-10 kg"),
    ("XIX-XX", "XIX-XX"),
    ("1941-45", "1941-45"),
    ("1941-45 years", "1941-45 years"),
    ("1.5-2.5", "1.5-2.5"),
    ("1.5-2.5  kg", "1.5-2.5  kg"),
    ("5–10 kg", "5–10 kg"),
];

#[test]
fn context() {
    let ru = configs(&Language::RU_RU, CONTEXT);
    assert_eq!(ru.len(), 2);
    check_rows(&ru, RU_CONTEXT);
    let en = configs(&Language::EN_US, CONTEXT);
    check_rows(&en, EN_CONTEXT);
}

#[test]
fn context_names_its_rule_and_digits_wins_where_both_match() {
    // Section 8.5: where both rules match (`1941-1945 гг.`) the edits are equal and the report
    // names `range.digits`, the earlier rule; the shapes that only the context takes name
    // `range.context`.
    let alone = only(Language::RU_RU, &[CONTEXT]);
    assert_changed_by("alone", &alone, "1941-1945 гг.", CONTEXT);
    let both = only(Language::RU_RU, &[DIGITS, CONTEXT]);
    assert_changed_by("both", &both, "1941-1945 гг.", DIGITS);
    assert_changed_by("both", &both, "XIX-XX вв.", CONTEXT);
    assert_changed_by("both", &both, "1941-45 гг.", CONTEXT);
    let p = prose(Language::RU_RU);
    assert_changed_by_with_glue("Prose", &p, "1941-1945 гг.", DIGITS);
    assert_changed_by_with_glue("Prose", &p, "в 2010-11 гг.", CONTEXT);
    assert_changed_by_with_glue("Prose", &p, "стр. 9-12", DIGITS);
    // The output does not depend on which rule matched.
    check_in("both", &both, ("1941-1945 гг.", "1941–1945 гг."));
}

#[test]
fn context_leaves_a_label_range_alone() {
    // A label range takes no context (section 8.5): `§§ 5-7 г. Москвы`.
    let t = only(Language::RU_RU, &[CONTEXT]);
    check_unchanged("alone", &t, "§§ 5-7 г. Москвы");
    check_unchanged("alone", &t, "№ 5-7 кг");
}

#[test]
fn a_cyrillic_look_alike_is_not_a_roman_numeral() {
    // `ХIХ` with Cyrillic `Х` (U+0425) is a mixed word: the range stays, with `MixedScript`.
    let text = "\u{425}I\u{425}-\u{425}\u{425} вв.";
    for (what, t) in configs(&Language::RU_RU, CONTEXT) {
        check_unchanged(&what, &t, text);
        let kinds: Vec<_> = diagnostics_of(&t, text).into_iter().map(|d| d.0).collect();
        assert!(
            kinds.contains(&DiagnosticKind::MixedScript),
            "{what}: {kinds:?}"
        );
    }
    // The same in a set that holds every rule of the stage.
    let t = prose(Language::RU_RU);
    check_unchanged("Prose", &t, text);
}

#[test]
fn context_is_inert_for_common() {
    // Units alone do not activate `range.context` (section 7.4).
    let t = only(Language::COMMON, &[CONTEXT]);
    for text in ["5-10 kg", "1,5-2,5 кг", "p. 9-12", "XIX-XX вв."] {
        check_unchanged("und", &t, text);
    }
    // Prose with COMMON: only range.digits works.
    let t = prose(Language::COMMON);
    check_in("und Prose", &t, ("5-10 kg", "5–10 kg"));
    check_unchanged("und Prose", &t, "1941-45 гг.");
}

#[test]
fn context_follows_the_unit_settings() {
    // `add_units` and `remove_units` change what counts as a unit (section 8.1).
    let added = typolish::Typographer::builder(Language::EN_US, RuleSet::EMPTY.with(CONTEXT))
        .add_units(["widgets"])
        .build()
        .unwrap();
    check_in("added", &added, ("1.5-2.5 widgets", "1.5–2.5 widgets"));
    let removed = typolish::Typographer::builder(Language::EN_US, RuleSet::EMPTY.with(CONTEXT))
        .remove_units(["kg"])
        .build()
        .unwrap();
    check_unchanged("removed", &removed, "1.5-2.5 kg");
}

#[test]
fn context_disabled() {
    let t = without_rule(Language::RU_RU, RuleSet::PROSE, CONTEXT);
    check_unchanged("Prose without range.context", &t, "1941-45 гг.");
    check_unchanged("Prose without range.context", &t, "XIX-XX вв.");
    // The digits path of the same set stays.
    check_in(
        "Prose without range.context",
        &t,
        ("1941-1945 гг.", "1941–1945 гг."),
    );
}

#[test]
fn a_slot_next_to_a_context_range() {
    let t = only(Language::RU_RU, &[CONTEXT]);
    let out = t.typeset_template(
        [Piece::Slot("a", Slot::OTHER), Piece::Text(" 1,5-2,5 кг")],
        Output::Unicode,
    );
    assert_eq!(out.texts(), ["", " 1,5–2,5 кг"]);
    let out = t.typeset_template(
        [Piece::Text("1,5-2,5 "), Piece::Slot("u", Slot::WORD)],
        Output::Unicode,
    );
    assert_eq!(out.texts(), ["1,5-2,5 ", ""]);
}

// range.time: `HH:MM[:SS]-HH:MM[:SS]`; hours 0 to 23 with one or two digits, minutes and seconds
// two digits (check 5).
const TIME_ROWS: &[(&str, &str)] = &[
    ("10:00-12:30", "10:00–12:30"),
    ("10:00:15-12:30:45", "10:00:15–12:30:45"),
    ("9:00-18:00", "9:00–18:00"),
    ("22:00-06:00", "22:00–06:00"),
    ("Приём 10:00-12:30.", "Приём 10:00–12:30."),
    ("Open 10:00-12:30 daily", "Open 10:00–12:30 daily"),
    ("(10:00-12:30)", "(10:00–12:30)"),
    // Not a time range: an hour above 23, a minute above 59, one side without minutes, spaces.
    ("25:00-26:00", "25:00-26:00"),
    ("10:60-11:00", "10:60-11:00"),
    ("10:00-12", "10:00-12"),
    ("10:00 - 12:30", "10:00 - 12:30"),
    ("2026-11-21T10:20:30Z", "2026-11-21T10:20:30Z"),
    // Typeset input.
    ("10:00–12:30", "10:00–12:30"),
];

#[test]
fn time() {
    for language in languages() {
        let all = configs(&language, TIME);
        assert_eq!(all.len(), 2);
        check_rows(&all, TIME_ROWS);
        for (what, t) in &all {
            assert_changed_by(what, t, "10:00-12:30", TIME);
        }
        check_unchanged("Interface", &interface(language), "10:00-12:30");
    }
}

#[test]
fn the_times_are_frozen_and_the_separator_is_not() {
    // TechKind::TimeRange: "the times; the separator stays editable" (section 3.6, check 5).
    for (what, t) in configs(&Language::RU_RU, TIME) {
        let spans = spans_of(&t, "10:00-12:30");
        let got: Vec<_> = spans.iter().map(|s| (s.0, s.2.clone())).collect();
        assert_eq!(
            got,
            [
                (tech(TechKind::TimeRange), 0..5),
                (tech(TechKind::TimeRange), 6..11)
            ],
            "{what}"
        );
        let r = t.report("10:00-12:30", Output::Unicode);
        assert_eq!(r.changes().len(), 1, "{what}");
        assert_eq!(r.changes()[0].range(), 5..6, "{what}");
        assert_eq!(r.changes()[0].replacement(), "–", "{what}");
    }
}

#[test]
fn time_disabled() {
    let t = without_rule(Language::EN_US, RuleSet::PROSE, TIME);
    check_unchanged("Prose without range.time", &t, "10:00-12:30");
}

// range.placeholders: `{Number}-{Number}` and `{Date}-{Date}` without spaces (section 8.5).
#[test]
fn placeholders() {
    for language in languages() {
        let all = configs(&language, PLACEHOLDERS);
        assert_eq!(all.len(), 3, "alone, Prose and Interface");
        for (what, t) in &all {
            let join = |a: Slot, mid: &'static str, b: Slot| -> (Vec<String>, bool) {
                let out = t.typeset_template(
                    [Piece::Slot("a", a), Piece::Text(mid), Piece::Slot("b", b)],
                    Output::Unicode,
                );
                (
                    out.texts().iter().map(|s| s.to_string()).collect(),
                    out.changed(),
                )
            };
            for (a, b) in [(Slot::NUMBER, Slot::NUMBER), (Slot::DATE, Slot::DATE)] {
                assert_eq!(
                    join(a, "-", b),
                    (vec!["".to_owned(), "–".to_owned(), "".to_owned()], true),
                    "{what}: {a:?}"
                );
            }
            // The other pairs and the typed dash stay. `{a} - {b}` is a subtraction for
            // `Number` slots (the dash rules refuse it, section 8.4).
            for (a, mid, b) in [
                (Slot::NUMBER, "-", Slot::DATE),
                (Slot::DATE, "-", Slot::NUMBER),
                (Slot::WORD, "-", Slot::WORD),
                (Slot::OTHER, "-", Slot::OTHER),
                (Slot::NUMBER, "-", Slot::WORD),
                (Slot::NUMBER, " - ", Slot::NUMBER),
                (Slot::NUMBER, "–", Slot::NUMBER),
                (Slot::DATE, "–", Slot::DATE),
            ] {
                let (texts, changed) = join(a, mid, b);
                assert_eq!(texts, ["", mid, ""], "{what}: {a:?} {mid:?} {b:?}");
                assert!(!changed, "{what}: {a:?} {mid:?} {b:?}");
            }
        }
    }
}

#[test]
fn placeholders_next_to_literals_and_text() {
    for language in languages() {
        for (what, t) in configs(&language, PLACEHOLDERS) {
            // A slot next to a literal is not edited by any range rule.
            let out = t.typeset_template(
                [Piece::Slot("a", Slot::NUMBER), Piece::Text("-5")],
                Output::Unicode,
            );
            assert_eq!(out.texts(), ["", "-5"], "{what}");
            let out = t.typeset_template(
                [Piece::Text("5-"), Piece::Slot("b", Slot::NUMBER)],
                Output::Unicode,
            );
            assert_eq!(out.texts(), ["5-", ""], "{what}");
            // With text around the range.
            let out = t.typeset_template(
                [
                    Piece::Text("Дни "),
                    Piece::Slot("a", Slot::NUMBER),
                    Piece::Text("-"),
                    Piece::Slot("b", Slot::NUMBER),
                    Piece::Text(" дн."),
                ],
                Output::Unicode,
            );
            // In a full set `nbsp.units` glues the Russian unit after the range (section 2.6).
            let glued = language == Language::RU_RU && t.rules().contains(Rule::NbspUnits);
            let unit = if glued { "\u{a0}дн." } else { " дн." };
            assert_eq!(out.texts(), ["Дни ", "–", unit], "{what}");
            // Dates in words and digits: two `Date` slots.
            let out = t.typeset_template(
                [
                    Piece::Slot("d1", Slot::DATE),
                    Piece::Text("-"),
                    Piece::Slot("d2", Slot::DATE),
                    Piece::Text(" г."),
                ],
                Output::Unicode,
            );
            // `nbsp.dates` glues a year word after a range of `Date` slots (section 2.6).
            let glued = language == Language::RU_RU && t.rules().contains(Rule::NbspDates);
            let year = if glued { "\u{a0}г." } else { " г." };
            assert_eq!(out.texts(), ["", "–", year], "{what}");
        }
    }
}

#[test]
fn a_placeholder_range_is_recorded_by_its_rule() {
    for (what, t) in configs(&Language::RU_RU, PLACEHOLDERS) {
        let r = t.report_template(
            [
                Piece::Slot("a", Slot::NUMBER),
                Piece::Text("-"),
                Piece::Slot("b", Slot::NUMBER),
            ],
            Output::Unicode,
        );
        assert_eq!(r.changes().len(), 1, "{what}");
        let change = &r.changes()[0];
        assert_eq!(change.rule(), PLACEHOLDERS, "{what}");
        assert_eq!((change.piece(), change.range()), (1, 0..1), "{what}");
        assert_eq!(change.replacement(), "–", "{what}");
        assert_eq!(r.original(change), "-", "{what}");
    }
}

#[test]
fn a_spaced_placeholder_pair_stays() {
    // `{a} - {b}` may be a subtraction: nothing changes (section 8.5); the dash rules refuse it
    // and say so in the sets.
    for (what, t) in configs(&Language::RU_RU, PLACEHOLDERS) {
        let r = t.report_template(
            [
                Piece::Slot("a", Slot::NUMBER),
                Piece::Text(" - "),
                Piece::Slot("b", Slot::NUMBER),
            ],
            Output::Unicode,
        );
        assert!(!r.result().changed(), "{what}");
        let ambiguous = r
            .diagnostics()
            .iter()
            .filter(|d| d.kind() == &DiagnosticKind::AmbiguousHyphen)
            .count();
        let expected = usize::from(!what.contains("alone"));
        assert_eq!(ambiguous, expected, "{what}");
    }
}

#[test]
fn placeholders_disabled() {
    for set in [RuleSet::PROSE, RuleSet::INTERFACE] {
        let t = without_rule(Language::RU_RU, set, PLACEHOLDERS);
        let out = t.typeset_template(
            [
                Piece::Slot("a", Slot::NUMBER),
                Piece::Text("-"),
                Piece::Slot("b", Slot::NUMBER),
            ],
            Output::Unicode,
        );
        assert_eq!(out.texts(), ["", "-", ""]);
        assert!(!out.changed());
    }
}

#[test]
fn ranges_in_the_full_sets() {
    // Prose: the literal paths; Interface: only the placeholder path (section 10.6). The glue of
    // the `nbsp.*` rules is part of the expectations (sections 10.6 and 10.7).
    let p = prose(Language::RU_RU);
    let ui = interface(Language::RU_RU);
    let rows: &[(&str, &str, &str)] = &[
        ("8000-8100", "8000–8100", "8000-8100"),
        ("ports 8000-8100", "ports 8000–8100", "ports 8000-8100"),
        (
            "Порты 8000-8100 с балансировкой",
            "Порты 8000–8100 с⍽балансировкой",
            "Порты 8000-8100 с⍽балансировкой",
        ),
        ("в 2010-11 гг.", "в⍽2010–11⍽гг.", "в⍽2010-11⍽гг."),
        ("2010-11 гг.", "2010–11⍽гг.", "2010-11⍽гг."),
        ("2026-11 гг.", "2026-11⍽гг.", "2026-11⍽гг."),
        ("2026-11", "2026-11", "2026-11"),
        ("123-4567", "123–4567", "123-4567"),
        ("+7 495 123-4567", "+7⍽495⍽123–4567", "+7 495 123-4567"),
        ("+7 495 123-45-67", "+7⍽495⍽123-45-67", "+7 495 123-45-67"),
        (
            "+1 300 200 100 50",
            "+1⍽300⍽200⍽100⍽50",
            "+1 300 200 100 50",
        ),
        (
            "Приём 10:00-12:30",
            "Приём 10:00–12:30",
            "Приём 10:00-12:30",
        ),
    ];
    for (input, prose_out, ui_out) in rows {
        common::check_exact("Prose", &p, (input, prose_out));
        common::check_exact("Interface", &ui, (input, ui_out));
    }
}
