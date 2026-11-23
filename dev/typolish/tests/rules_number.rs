//! The tables of `number.minus` and `number.times` (design sections 8.1, 8.8, 4.3 and 13.2).
//!
//! A row that names no configuration runs with the one rule of its table, alone
//! (`RuleSet::EMPTY.with(rule)`), and in Prose, the only built-in set that holds either rule
//! (section 9.2: Interface has neither). A row that names a set gives the output of the whole set.

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
    assert_changed_by, check_exact, check_in, check_rows, check_unchanged, configs, interface,
    only, prose, vis, without_rule,
};
use probe::{span_texts, tech};
use typolish::{Language, Output, Piece, Rule, RuleSet, Slot, TechKind, Typographer};

const MINUS: Rule = Rule::NumberMinus;
const TIMES: Rule = Rule::NumberTimes;

fn languages() -> [Language; 3] {
    [Language::RU_RU, Language::EN_US, Language::COMMON]
}

// number.minus: a core `-D+` or `-D+[.,]D+` whose fragment starts after a pattern space, a line
// start or `(`. The rows hold in every language, including COMMON (section 7.4).
const MINUS_ROWS: &[(&str, &str)] = &[
    ("Минус -5 градусов", "Минус −5 градусов"),
    ("Minus -5 degrees", "Minus −5 degrees"),
    ("-5", "−5"),
    ("-12", "−12"),
    ("-1,5", "−1,5"),
    ("-1.5", "−1.5"),
    ("(-5)", "(−5)"),
    ("(-1,5 и -2)", "(−1,5 и −2)"),
    ("tail -5", "tail −5"),
    ("head -1", "head −1"),
    ("-5\n-6", "−5\n−6"),
    ("Строка\n-5", "Строка\n−5"),
    ("see -5 now", "see −5 now"),
    ("5 -3", "5 −3"),
    ("-007", "−007"),
    // Not firing at each boundary of the pattern: no space, a hyphen between two numbers, a
    // second sign, a flag, a space after the sign, an operator, a typed minus or plus.
    ("5-3", "5-3"),
    ("a-5", "a-5"),
    ("x-5", "x-5"),
    ("--5", "--5"),
    ("-x", "-x"),
    ("-a5", "-a5"),
    ("5 - 3", "5 - 3"),
    ("-", "-"),
    ("+5", "+5"),
    ("−5", "−5"),
    ("5", "5"),
    ("-5...5", "-5...5"),
    ("v1.2-3", "v1.2-3"),
    ("a\t-5", "a\t-5"),
    ("a  -5", "a  -5"),
    // Typeset input stays, borrowed.
    ("Минус −5 градусов", "Минус −5 градусов"),
    ("(−5)", "(−5)"),
];

#[test]
fn minus() {
    for language in languages() {
        let all = configs(&language, MINUS);
        assert_eq!(
            all.len(),
            2,
            "alone and Prose: Interface has no number.minus"
        );
        check_rows(&all, MINUS_ROWS);
        for (what, t) in &all {
            assert_changed_by(what, t, "Минус -5 градусов", MINUS);
            assert_changed_by(what, t, "tail -5", MINUS);
            let report = t.report("Минус -5 градусов", Output::Unicode);
            let change = &report.changes()[0];
            assert_eq!(change.replacement(), "\u{2212}");
            assert_eq!(report.original(change), "-");
            assert!(report.protected().is_empty(), "{what}");
            assert!(report.diagnostics().is_empty(), "{what}");
        }
    }
}

#[test]
fn minus_of_a_percentage() {
    // A percentage is a number in both of its forms (sections 8.2 and 8.8): the space that the
    // percent setting inserts changes nothing for `number.minus`.
    for language in languages() {
        let t = only(language, &[MINUS]);
        check_exact("alone", &t, ("-5%", "−5%"));
        check_exact("alone", &t, ("-1,5‰", "−1,5‰"));
        check_exact("alone", &t, ("-5 %", "−5 %"));
        check_unchanged("alone", &t, "-5%%");
    }
    let t = Typographer::builder(Language::RU_RU, RuleSet::PROSE)
        .percent(typolish::PercentSpacing::Language)
        .build()
        .unwrap();
    check_exact("ru Prose, Language", &t, ("-1,5100%.", "−1,5100⍽%."));
    check_exact("ru Prose, Language", &t, ("−1,5100⍽%.", "−1,5100⍽%."));
}

#[test]
fn minus_alone_leaves_a_spaced_hyphen() {
    // In Prose `- 5` at a line start is a dialogue dash (`dash.dialogue`), so the row is alone.
    for language in languages() {
        let t = only(language, &[MINUS]);
        check_unchanged("alone", &t, "- 5");
        check_unchanged("alone", &t, "5 - 3");
        check_unchanged("alone", &t, "1990-2000");
    }
}

#[test]
fn minus_in_prose_next_to_a_unit() {
    // Design section 8.8: `-12 °C` → `−12⍽°C` (the unit glue of `nbsp.units`).
    let t = prose(Language::RU_RU);
    check_exact("ru Prose", &t, ("Температура -12 °C", "Температура −12⍽°C"));
    check_exact("ru Prose", &t, ("-12 °C", "−12⍽°C"));
    let t = only(Language::RU_RU, &[MINUS]);
    check_exact("ru alone", &t, ("-12 °C", "−12 °C"));
}

#[test]
fn minus_is_not_in_interface() {
    // Section 9.2 and 10.6: commands such as `head -1` and `ls -1` never change in Interface.
    for language in languages() {
        let t = interface(language);
        for text in [
            "head -1",
            "ls -1",
            "tail -5",
            "Минус -5 градусов",
            "-5",
            "(-5)",
            "x -1,5",
        ] {
            check_unchanged("Interface", &t, text);
        }
    }
    // With the rule enabled on top of Interface it fires, so the set is what keeps it out.
    let t = Typographer::builder(Language::EN_US, RuleSet::INTERFACE)
        .enable(MINUS)
        .build()
        .unwrap();
    assert_eq!(t.typeset("head -1", Output::Unicode), "head \u{2212}1");
}

#[test]
fn minus_follows_the_setting() {
    let build = |minus: char| {
        Typographer::builder(Language::RU_RU, RuleSet::EMPTY.with(MINUS))
            .minus_sign(minus)
            .build()
            .unwrap()
    };
    // The hyphen-minus as the sign: the text is already in the target form.
    check_unchanged("minus_sign('-')", &build('-'), "Минус -5 градусов");
    check_exact(
        "minus_sign('–')",
        &build('\u{2013}'),
        ("Минус -5 градусов", "Минус –5 градусов"),
    );
    check_exact(
        "minus_sign('−')",
        &build('\u{2212}'),
        ("Минус -5 градусов", "Минус −5 градусов"),
    );
}

/// Every minus sign the builder accepts is read as the sign of a number by the second pass, so a
/// dash next to the signed number is refused on both passes and the output is a fixed point
/// (design sections 2.11, 3.6 check 15, 4.3 I4 and 8.4).
#[test]
fn a_minus_of_the_settings_reads_as_a_sign_on_the_second_pass() {
    const SIGNS: [char; 8] = [
        '-', '\u{2010}', '\u{2011}', '\u{2012}', '\u{2013}', '\u{2014}', '\u{2015}', '\u{2212}',
    ];
    for minus in SIGNS {
        for language in [Language::RU_RU, Language::EN_US] {
            let t = Typographer::builder(language.clone(), RuleSet::PROSE)
                .minus_sign(minus)
                .build()
                .unwrap();
            let rows = [
                ("Было -5 - стало", format!("Было {minus}5 - стало")),
                ("range 5 - -5", format!("range 5 - {minus}5")),
                ("a - -5", format!("a - {minus}5")),
            ];
            for (source, expected) in rows {
                let once = t.typeset(source, Output::Unicode).into_owned();
                assert_eq!(once, expected, "{minus:?} {language:?}: {source:?}");
                let twice = t.typeset(&once, Output::Unicode).into_owned();
                assert_eq!(
                    twice, once,
                    "{minus:?} {language:?}: second pass of {source:?}"
                );
            }
            for source in [
                "Было -5 - 3",
                "a -5 -- b",
                "в -5 °C",
                "c -5 до -10",
                "до -5 - до 3",
                "Температура -5 и (-3), -1,5 и -5% в 10 x -5",
            ] {
                let once = t.typeset(source, Output::Unicode).into_owned();
                let twice = t.typeset(&once, Output::Unicode).into_owned();
                assert_eq!(
                    twice, once,
                    "{minus:?} {language:?}: second pass of {source:?}"
                );
            }
        }
    }
}

#[test]
fn minus_disabled() {
    for language in languages() {
        let t = without_rule(language, RuleSet::PROSE, MINUS);
        check_unchanged("Prose without number.minus", &t, "Минус -5 градусов");
        check_unchanged("Prose without number.minus", &t, "tail -5");
    }
}

#[test]
fn minus_with_a_slot_next_to_it() {
    for (what, t) in configs(&Language::RU_RU, MINUS) {
        let out = t.typeset_template(
            [Piece::Text("Минус -5 "), Piece::Slot("s", Slot::WORD)],
            Output::Unicode,
        );
        assert_eq!(out.texts(), ["Минус −5 ", ""], "{what}");
        let out = t.typeset_template(
            [Piece::Slot("s", Slot::WORD), Piece::Text(" -5")],
            Output::Unicode,
        );
        assert_eq!(out.texts(), ["", " −5"], "{what}");
        // A slot glued to a sign is not a `-D+` core.
        let out = t.typeset_template(
            [Piece::Slot("s", Slot::WORD), Piece::Text("-5")],
            Output::Unicode,
        );
        assert_eq!(out.texts(), ["", "-5"], "{what}");
        assert!(!out.changed(), "{what}");
    }
}

#[test]
fn minus_in_every_output_kind() {
    use typolish::{EscapeStyle, HtmlStyle};
    let t = only(Language::RU_RU, &[MINUS]);
    for (output, expected) in [
        (Output::Unicode, "Минус −5"),
        (Output::Html(HtmlStyle::NAMED), "Минус &minus;5"),
        (Output::Html(HtmlStyle::DECIMAL), "Минус &#8722;5"),
        (Output::Html(HtmlStyle::HEX), "Минус &#x2212;5"),
        (Output::Html(HtmlStyle::CHARACTERS), "Минус −5"),
        (Output::Escaped(EscapeStyle::JS), "Минус \\u22125"),
        (Output::Escaped(EscapeStyle::RUST), "Минус \\u{2212}5"),
        (Output::Escaped(EscapeStyle::CSS), "Минус \\2212 5"),
    ] {
        assert_eq!(t.typeset("Минус -5", output), expected, "{output:?}");
    }
}

// number.times: a `Dimensions` core (check 16) → `×` in place of the letter; a lone `x`, `X`, `х`
// or `Х` fragment between two number fragments, the left one without a leading zero → `×` with
// the author's spaces kept. The rows hold in every language, alone and in Prose.
const TIMES_ROWS: &[(&str, &str)] = &[
    ("1920x1080", "1920×1080"),
    ("1920X1080", "1920×1080"),
    ("1920х1080", "1920×1080"),
    ("1920Х1080", "1920×1080"),
    ("5x5", "5×5"),
    ("Монитор 1920x1080.", "Монитор 1920×1080."),
    ("Screen 1920x1080.", "Screen 1920×1080."),
    ("(1920x1080)", "(1920×1080)"),
    ("1920x0108", "1920×0108"),
    ("10 x 5", "10 × 5"),
    ("10 X 5", "10 × 5"),
    ("10 х 5", "10 × 5"),
    ("10 Х 5", "10 × 5"),
    ("10 x 5 x 3", "10 × 5 × 3"),
    // The author's spaces are kept (a run of spaces is no pattern space for this rule, section 8.2).
    ("10  x 5", "10  × 5"),
    ("10 x  5", "10 ×  5"),
    ("1920x1080\n10 x 5", "1920×1080\n10 × 5"),
    // Not firing: hex, identifiers, a leading zero in the left factor, a missing number, a
    // run of spaces or a tab, a second letter, three factors in one core.
    ("0x1F", "0x1F"),
    ("0x10", "0x10"),
    ("x86", "x86"),
    ("2x", "2x"),
    ("10x", "10x"),
    ("x5", "x5"),
    ("01x5", "01x5"),
    ("010 x 5", "010 x 5"),
    ("x 5", "x 5"),
    ("5 x", "5 x"),
    ("foo x 5", "foo x 5"),
    ("5 x foo", "5 x foo"),
    ("5 x b 5", "5 x b 5"),
    ("10 xx 5", "10 xx 5"),
    ("1920x1080x32", "1920x1080x32"),
    // Typeset input stays, borrowed.
    ("1920×1080", "1920×1080"),
    ("10 × 5", "10 × 5"),
];

#[test]
fn times() {
    for language in languages() {
        let all = configs(&language, TIMES);
        assert_eq!(
            all.len(),
            2,
            "alone and Prose: Interface has no number.times"
        );
        check_rows(&all, TIMES_ROWS);
        for (what, t) in &all {
            assert_changed_by(what, t, "1920x1080", TIMES);
            assert_changed_by(what, t, "10 x 5", TIMES);
            let report = t.report("1920x1080", Output::Unicode);
            let change = &report.changes()[0];
            assert_eq!(change.replacement(), "\u{d7}");
            assert_eq!(report.original(change), "x");
            assert!(report.diagnostics().is_empty(), "{what}");
            // The Cyrillic letters are replaced too.
            let source = vis("10 Х 5");
            let report = t.report(&source, Output::Unicode);
            assert_eq!(report.original(&report.changes()[0]), "Х");
        }
    }
}

#[test]
fn times_next_to_a_unit_that_is_a_letter() {
    // Design section 13.2: `10 x 5` with `add_units(["x"])` → `10 × 5` (a dictionary entry is
    // read as a word, so it is still a lone fragment between two numbers).
    for language in [Language::RU_RU, Language::EN_US] {
        let t = Typographer::builder(language, RuleSet::EMPTY.with(TIMES))
            .add_units(["x"])
            .build()
            .unwrap();
        check_exact("add_units(x)", &t, ("10 x 5", "10 × 5"));
        check_exact("add_units(x)", &t, ("1920x1080", "1920×1080"));
        check_exact("add_units(x)", &t, ("010 x 5", "010 x 5"));
    }
}

#[test]
fn times_leaves_the_typed_sign_a_technical_fragment() {
    // `a × b` stays: the typed sign is an unclassified fragment, frozen whole (section 13.2).
    for (what, t) in configs(&Language::RU_RU, TIMES) {
        check_in(&what, &t, ("a × b", "a × b"));
        assert_eq!(
            span_texts(&t, "a × b"),
            [(tech(TechKind::Unclassified), "×")],
            "{what}"
        );
    }
}

#[test]
fn times_is_not_in_interface_and_is_an_opt_in() {
    for language in languages() {
        let t = interface(language);
        for text in ["1920x1080", "10 x 5", "0x1F", "x86"] {
            check_unchanged("Interface", &t, text);
        }
    }
    // The task makes the sign optional: a switch of the rule set, not a setting.
    let t = Typographer::new(Language::EN_US, RuleSet::INTERFACE.with(TIMES));
    check_exact(
        "Interface with number.times",
        &t,
        ("Screen 1920x1080.", "Screen 1920×1080."),
    );
    check_exact("Interface with number.times", &t, ("10 x 5", "10 × 5"));
}

#[test]
fn times_after_a_label_number() {
    // A label number counts as a number for `number.times` (section 8.2) in both of its forms, so
    // the space that `nbsp.signs` inserts into `№10` changes nothing for the rule (I2 of section
    // 4.3): the sign comes in the same pass as the glue.
    let t = only(Language::RU_RU, &[TIMES]);
    check_exact("alone", &t, ("№10 x 5", "№10 × 5"));
    check_exact("alone", &t, ("№ 10 x 5", "№ 10 × 5"));
    check_exact("alone", &t, ("№010 x 5", "№010 x 5"));
    let t = prose(Language::RU_RU);
    check_exact("ru Prose", &t, ("№10 x 5", "№⍽10 × 5"));
    check_exact("ru Prose", &t, ("№ 10 x 5", "№⍽10 × 5"));
    check_exact("ru Prose", &t, ("№⍽10 × 5", "№⍽10 × 5"));
}

#[test]
fn times_disabled() {
    for language in languages() {
        let t = without_rule(language, RuleSet::PROSE.with(TIMES), TIMES);
        check_unchanged("without number.times", &t, "1920x1080");
        check_unchanged("without number.times", &t, "10 x 5");
    }
}

#[test]
fn times_with_slots() {
    // The slot participation table of section 2.6: `{w}x{h}` in Prose, `Number` slots only.
    for (what, t) in configs(&Language::RU_RU, TIMES) {
        let number = |a: Slot, middle: &'static str, b: Slot| {
            t.typeset_template(
                [
                    Piece::Slot("w", a),
                    Piece::Text(middle),
                    Piece::Slot("h", b),
                ],
                Output::Unicode,
            )
        };
        let out = number(Slot::NUMBER, "x", Slot::NUMBER);
        assert_eq!(out.texts(), ["", "×", ""], "{what}");
        assert!(out.changed(), "{what}");
        for (a, b) in [
            (Slot::WORD, Slot::WORD),
            (Slot::OTHER, Slot::OTHER),
            (Slot::DATE, Slot::DATE),
            (Slot::NUMBER, Slot::WORD),
            (Slot::WORD, Slot::NUMBER),
        ] {
            for middle in ["x", " x "] {
                let out = number(a, middle, b);
                assert_eq!(
                    out.texts(),
                    ["", middle, ""],
                    "{what}: {a:?} {middle:?} {b:?}"
                );
                assert!(!out.changed(), "{what}");
            }
        }
    }
}

#[test]
fn times_between_number_slots_with_spaces() {
    // A number is a `Number` fragment or a `Number` slot (section 8.2), so `{w} x {h}` is as much
    // a lone letter between two numbers as `10 x 5` is.
    for (what, t) in configs(&Language::RU_RU, TIMES) {
        for (middle, expected) in [(" x ", " × "), (" Х ", " × "), (" X ", " × ")] {
            let out = t.typeset_template(
                [
                    Piece::Slot("w", Slot::NUMBER),
                    Piece::Text(middle),
                    Piece::Slot("h", Slot::NUMBER),
                ],
                Output::Unicode,
            );
            assert_eq!(out.texts(), ["", expected, ""], "{what}: {middle:?}");
        }
    }
}

#[test]
fn times_in_every_output_kind() {
    use typolish::{EscapeStyle, HtmlStyle};
    let t = only(Language::RU_RU, &[TIMES]);
    for (output, expected) in [
        (Output::Unicode, "1920×1080"),
        (Output::Html(HtmlStyle::NAMED), "1920&times;1080"),
        (Output::Html(HtmlStyle::DECIMAL), "1920&#215;1080"),
        (Output::Html(HtmlStyle::HEX), "1920&#xD7;1080"),
        (Output::Escaped(EscapeStyle::JS), "1920\\u00D71080"),
        (Output::Escaped(EscapeStyle::JS_HEX), "1920\\xD71080"),
        (Output::Escaped(EscapeStyle::RUST), "1920\\u{d7}1080"),
        (Output::Escaped(EscapeStyle::CSS), "1920\\d7 1080"),
    ] {
        assert_eq!(t.typeset("1920x1080", output), expected, "{output:?}");
    }
}
