//! The tables of `symbols.copyright`, `symbols.arrows` and `symbols.math` (design sections 8.1,
//! 8.9, 10.7 and 13.2).
//!
//! A row that names no configuration runs with the one rule of its table, alone
//! (`RuleSet::EMPTY.with(rule)`), and in Prose, the only built-in set that holds the symbol rules
//! (section 9.2: Interface has none). Rows with a range of years run alone: Prose also turns the
//! hyphen of `2020-2026` into an en dash (`range.digits`), which the last test of the file spells
//! out.

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
    only, prose, without_rule,
};
use probe::{diagnostics_of, span_texts, tech};
use typolish::{
    Language, Output, PercentSpacing, Piece, Rule, RuleSet, Slot, TechKind, Typographer,
};

const COPYRIGHT: Rule = Rule::SymbolsCopyright;
const ARROWS: Rule = Rule::SymbolsArrows;
const MATH: Rule = Rule::SymbolsMath;

fn languages() -> [Language; 3] {
    [Language::RU_RU, Language::EN_US, Language::COMMON]
}

// symbols.copyright: `(c)`, `(C)` and the Cyrillic `(с)`, `(С)` → `©` with a year next to the mark.
const COPYRIGHT_ROWS: &[(&str, &str)] = &[
    ("(c) 2026", "© 2026"),
    ("(C) 2026", "© 2026"),
    ("(с) 2026", "© 2026"),
    ("(С) 2026", "© 2026"),
    ("(c)2026", "©2026"),
    ("2026 (c)", "2026 ©"),
    ("Brand (r)", "Brand ®"),
    ("Brand(r)", "Brand®"),
    ("Brand(tm)", "Brand™"),
    ("Brand(TM)", "Brand™"),
    ("Brand (R)", "Brand ®"),
    ("Brand (tm)", "Brand ™"),
    ("Br(tm)", "Br™"),
    ("Бренд(tm)", "Бренд™"),
    ("Бренд (r)", "Бренд ®"),
    ("(c) 2026 Example Corp", "© 2026 Example Corp"),
    // A word of design section 8.2 whatever its number of tokens, and a compound word in the
    // spaced form (design section 15.2, item 16).
    ("McDonald’s(tm)", "McDonald’s™"),
    ("McDonald’s (r)", "McDonald’s ®"),
    ("Rock’n’Roll (tm)", "Rock’n’Roll ™"),
    ("IT-Systems (tm)", "IT-Systems ™"),
    // Not firing: a list item, a function argument, a lowercase or one-letter brand, an operator
    // after the mark, a mark attached to a number or a range, a mixed-case mark, a short year.
    ("вариант (c)", "вариант (c)"),
    ("f(c)", "f(c)"),
    ("f(c) 2026", "f(c) 2026"),
    ("(R) знак", "(R) знак"),
    ("S(r) = πr²", "S(r) = πr²"),
    ("f(r)", "f(r)"),
    ("V(r)", "V(r)"),
    ("area(r)", "area(r)"),
    ("brand(tm)", "brand(tm)"),
    ("B(tm)", "B(tm)"),
    ("Brand(Tm)", "Brand(Tm)"),
    ("Brand(tM)", "Brand(tM)"),
    ("Brand(tm) = x", "Brand(tm) = x"),
    ("Brand (r) + x", "Brand (r) + x"),
    ("(c) 202", "(c) 202"),
    ("(c) 20266", "(c) 20266"),
    ("(c)", "(c)"),
    ("2026(c)", "2026(c)"),
    ("Section 1001(c) applies.", "Section 1001(c) applies."),
    ("(c) 2026a", "(c) 2026a"),
    // Lowercase, a number first, or an attached compound word (check 24 freezes it).
    ("don’t(tm)", "don’t(tm)"),
    ("90’s (tm)", "90’s (tm)"),
    ("IT-Systems(tm)", "IT-Systems(tm)"),
    // Typeset input stays, borrowed; a lone legal mark is not frozen.
    ("© 2026", "© 2026"),
    ("Brand™", "Brand™"),
    ("Brand ®", "Brand ®"),
    ("© Example Corp", "© Example Corp"),
];

#[test]
fn copyright() {
    for language in languages() {
        let all = configs(&language, COPYRIGHT);
        assert_eq!(
            all.len(),
            2,
            "alone and Prose: Interface has no symbols.copyright"
        );
        check_rows(&all, COPYRIGHT_ROWS);
        for (what, t) in &all {
            assert_changed_by(what, t, "(c) 2026", COPYRIGHT);
            assert_changed_by(what, t, "Brand(tm)", COPYRIGHT);
            let report = t.report("(c) 2026", Output::Unicode);
            assert_eq!(report.changes().len(), 1, "{what}");
            let change = &report.changes()[0];
            assert_eq!(change.replacement(), "\u{a9}");
            assert_eq!(report.original(change), "(c)");
            // `(c) 2026` and a lone `©` are not frozen (section 3.6).
            assert!(report.protected().is_empty(), "{what}");
            for text in ["© 2026", "©", "© Example Corp"] {
                assert!(span_texts(t, text).is_empty(), "{what}: {text}");
            }
        }
    }
}

#[test]
fn copyright_after_a_brand_with_plain_apostrophes_in_prose() {
    // Design section 8.9: the apostrophes of the brand are typeset by `quotes.apostrophe` too.
    for language in [Language::RU_RU, Language::EN_US] {
        let t = prose(language);
        check_in("Prose", &t, ("McDonald's(tm)", "McDonald’s™"));
        check_in("Prose", &t, ("Rock'n'Roll (tm)", "Rock’n’Roll ™"));
    }
}

// A range of years only one space away, on either side; alone, for the reason in the file header.
const RANGE_ROWS: &[(&str, &str)] = &[
    ("2020-2026 (c)", "2020-2026 ©"),
    ("(c) 2020-2026", "© 2020-2026"),
    ("(c) 2020–2026", "© 2020–2026"),
    ("2020–2026 (c)", "2020–2026 ©"),
    // Attached to the range, or to a number: not a word, so not a copyright sign.
    ("2020-2026(c)", "2020-2026(c)"),
    ("(c)2020-2026", "(c)2020-2026"),
    ("2020–2026(c)", "2020–2026(c)"),
];

#[test]
fn copyright_with_a_range_of_years() {
    for language in languages() {
        let t = only(language, &[COPYRIGHT]);
        for row in RANGE_ROWS {
            check_exact("alone", &t, *row);
        }
    }
}

#[test]
fn copyright_after_a_number_is_a_technical_fragment() {
    // Design section 13.2: `report().protected()` lists `Technical(Unclassified)`.
    for language in languages() {
        for (what, t) in configs(&language, COPYRIGHT) {
            for (text, fragment) in [
                ("2026(c)", "2026(c)"),
                ("Section 1001(c) applies.", "1001(c)"),
            ] {
                assert_eq!(
                    span_texts(&t, text),
                    [(tech(TechKind::Unclassified), fragment)],
                    "{what}: {text}"
                );
            }
        }
        let t = only(language, &[COPYRIGHT]);
        for (text, fragment) in [
            ("2020-2026(c)", "2020-2026(c)"),
            ("(c)2020-2026", "(c)2020-2026"),
        ] {
            assert_eq!(
                span_texts(&t, text),
                [(tech(TechKind::Unclassified), fragment)],
                "{text}"
            );
        }
    }
}

#[test]
fn copyright_reads_no_percentage_and_no_label_as_a_year() {
    // A percentage and a label number are not years in either of their forms (sections 8.2 and
    // 8.9), so the space that the percent setting or `nbsp.signs` inserts or removes changes
    // nothing for the rule (I2 of section 4.3).
    for language in languages() {
        let t = only(language, &[COPYRIGHT]);
        for text in [
            "(c) 2026%",
            "(c) 2026 %",
            "(c) 2026⍽%",
            "(c) 2026 ‰",
            "(c)2026 %",
            "(c) 2020-2026 %",
            "(c) 2020–2026⍽%",
            "2026 % (c)",
            "№2026 (c)",
            "№⍽2026 (c)",
            "№ 2026 (c)",
        ] {
            check_unchanged("alone", &t, text);
        }
        // The percent sign of another number does not hide the year.
        check_exact("alone", &t, ("(c) 2026 5 %", "© 2026 5 %"));
    }
    let modes = [
        PercentSpacing::AsWritten,
        PercentSpacing::Language,
        PercentSpacing::Attached,
        PercentSpacing::Spaced,
    ];
    for language in [Language::RU_RU, Language::EN_US] {
        for mode in modes {
            let t = Typographer::builder(language.clone(), RuleSet::PROSE)
                .percent(mode)
                .build()
                .unwrap();
            for text in [
                "(c) 2026%",
                "(c) 2026 %",
                "(c) 2026\u{a0}%",
                "(c) 2020-2026 %",
                "№2026 (c)",
            ] {
                let once = t.typeset(text, Output::Unicode).into_owned();
                assert!(
                    !once.contains('\u{a9}'),
                    "{language:?} {mode:?}: {text:?} -> {once:?}"
                );
                let twice = t.typeset(&once, Output::Unicode).into_owned();
                assert_eq!(
                    twice, once,
                    "{language:?} {mode:?}: second pass of {text:?}"
                );
            }
        }
    }
}

#[test]
fn copyright_in_prose_next_to_a_range() {
    // In Prose `range.digits` takes the hyphen of the range of years too.
    for language in [Language::RU_RU, Language::EN_US] {
        let t = prose(language);
        check_exact("Prose", &t, ("2020-2026 (c)", "2020–2026 ©"));
        check_exact("Prose", &t, ("(c) 2020-2026", "© 2020–2026"));
        // A mark attached after a range makes the fragment unclassified and frozen (sections 3.6
        // and 13.2), so not even the range dash changes.
        check_exact("Prose", &t, ("2020-2026(c)", "2020-2026(c)"));
    }
}

#[test]
fn copyright_is_not_in_interface() {
    for language in languages() {
        let t = interface(language);
        for text in [
            "(c) 2026",
            "(c)2026",
            "2026 (c)",
            "Brand(tm)",
            "Brand (r)",
            "x -> y",
            "x >= 5",
        ] {
            check_unchanged("Interface", &t, text);
        }
    }
}

#[test]
fn copyright_disabled() {
    for language in languages() {
        let t = without_rule(language, RuleSet::PROSE, COPYRIGHT);
        check_unchanged("Prose without symbols.copyright", &t, "(c) 2026");
        check_unchanged("Prose without symbols.copyright", &t, "Brand(tm)");
    }
    // A typed `©` without the rule stays: no rule rewrites it, and it is no diagnostic.
    let t = without_rule(Language::RU_RU, RuleSet::PROSE, COPYRIGHT);
    check_unchanged("Prose without symbols.copyright", &t, "© 2026");
}

#[test]
fn copyright_with_a_slot_next_to_it() {
    let t = only(Language::RU_RU, &[COPYRIGHT]);
    // The year typed in the text, with a slot after it.
    let out = t.typeset_template(
        [Piece::Text("(c) 2026 "), Piece::Slot("n", Slot::WORD)],
        Output::Unicode,
    );
    assert_eq!(out.texts(), ["© 2026 ", ""]);
}

#[test]
fn copyright_and_the_brand_signs_in_every_output_kind() {
    use typolish::{EscapeStyle, HtmlStyle};
    let t = only(Language::RU_RU, &[COPYRIGHT]);
    for (output, expected) in [
        (Output::Unicode, "© 2026 Brand™"),
        (Output::Html(HtmlStyle::NAMED), "&copy; 2026 Brand&trade;"),
        (Output::Html(HtmlStyle::DECIMAL), "&#169; 2026 Brand&#8482;"),
        (Output::Html(HtmlStyle::HEX), "&#xA9; 2026 Brand&#x2122;"),
        (Output::Html(HtmlStyle::CHARACTERS), "© 2026 Brand™"),
        (
            Output::Escaped(EscapeStyle::JS),
            "\\u00A9 2026 Brand\\u2122",
        ),
        (
            Output::Escaped(EscapeStyle::JS_HEX),
            "\\xA9 2026 Brand\\u2122",
        ),
        (
            Output::Escaped(EscapeStyle::RUST),
            "\\u{a9} 2026 Brand\\u{2122}",
        ),
        (Output::Escaped(EscapeStyle::CSS), "\\a9  2026 Brand\\2122 "),
    ] {
        assert_eq!(
            t.typeset("(c) 2026 Brand(tm)", output),
            expected,
            "{output:?}"
        );
    }
    let t = only(Language::RU_RU, &[COPYRIGHT]);
    assert_eq!(
        t.typeset("Brand(r)", Output::Html(HtmlStyle::NAMED)),
        "Brand&reg;"
    );
}

// symbols.arrows: an operator fragment that is exactly `->`, `<-` or `<->`, with a space on each
// side.
const ARROWS_ROWS: &[(&str, &str)] = &[
    ("foo -> bar", "foo → bar"),
    ("foo <- bar", "foo ← bar"),
    ("foo <-> bar", "foo ↔ bar"),
    ("x -> y", "x → y"),
    ("Вход -> выход", "Вход → выход"),
    ("foo -> bar -> baz", "foo → bar → baz"),
    ("a\nfoo -> bar", "a\nfoo → bar"),
    // Not in the table, glued, spaced with a run of spaces or a tab, or without a space on a
    // side: the arrow of the form `->` at a text edge is not an arrow.
    ("a->b", "a->b"),
    ("-->", "-->"),
    ("=>", "=>"),
    ("foo => bar", "foo => bar"),
    ("foo --> bar", "foo --> bar"),
    ("foo <--> bar", "foo <--> bar"),
    ("foo ->> bar", "foo ->> bar"),
    ("foo ->bar", "foo ->bar"),
    ("foo-> bar", "foo-> bar"),
    ("->", "->"),
    ("-> foo", "-> foo"),
    ("foo ->", "foo ->"),
    ("x  ->  y", "x  ->  y"),
    ("x\t->\ty", "x\t->\ty"),
    // Typeset input stays.
    ("foo → bar", "foo → bar"),
    ("foo ← bar", "foo ← bar"),
    ("foo ↔ bar", "foo ↔ bar"),
];

#[test]
fn arrows() {
    for language in languages() {
        let all = configs(&language, ARROWS);
        assert_eq!(
            all.len(),
            2,
            "alone and Prose: Interface has no symbols.arrows"
        );
        check_rows(&all, ARROWS_ROWS);
        for (what, t) in &all {
            assert_changed_by(what, t, "foo -> bar", ARROWS);
            assert_changed_by(what, t, "foo <-> bar", ARROWS);
            // The spaced and the glued forms report nothing: not a mistake, not a pattern.
            for text in ["x  ->  y", "x\t->\ty", "a->b", "foo => bar"] {
                let kinds = diagnostics_of(t, text);
                assert!(kinds.is_empty(), "{what}: {text}: {kinds:?}");
            }
        }
    }
}

// symbols.math: `>=`, `<=`, `!=` with a space on each side; `+-` as such an operator or as the sign
// of a number.
const MATH_ROWS: &[(&str, &str)] = &[
    ("size >= 5", "size ≥ 5"),
    ("size <= 5", "size ≤ 5"),
    ("size != 5", "size ≠ 5"),
    ("a +- b", "a ± b"),
    ("+-5", "±5"),
    ("size +-5", "size ±5"),
    ("size >= 5, size <= 9", "size ≥ 5, size ≤ 9"),
    // Not in the table, glued, spaced with a run of spaces or a tab.
    ("size >=5", "size >=5"),
    ("size>=5", "size>=5"),
    ("size == 5", "size == 5"),
    ("a>>b", "a>>b"),
    ("a >> b", "a >> b"),
    ("a << b", "a << b"),
    ("a >>= b", "a >>= b"),
    ("a ==> b", "a ==> b"),
    ("x  >=  5", "x  >=  5"),
    ("x\t>=\t5", "x\t>=\t5"),
    ("size => 5", "size => 5"),
    // Typeset input stays.
    ("size ≥ 5", "size ≥ 5"),
    ("size ≤ 5", "size ≤ 5"),
    ("size ≠ 5", "size ≠ 5"),
    ("a ± b", "a ± b"),
    ("±5", "±5"),
];

#[test]
fn math() {
    for language in languages() {
        let all = configs(&language, MATH);
        assert_eq!(
            all.len(),
            2,
            "alone and Prose: Interface has no symbols.math"
        );
        check_rows(&all, MATH_ROWS);
        for (what, t) in &all {
            assert_changed_by(what, t, "size >= 5", MATH);
            assert_changed_by(what, t, "+-5", MATH);
            for text in ["x  >=  5", "x\t>=\t5", "a>>b", "size == 5"] {
                let kinds = diagnostics_of(t, text);
                assert!(kinds.is_empty(), "{what}: {text}: {kinds:?}");
            }
        }
    }
}

#[test]
fn arrows_and_math_leave_code_alone_in_the_sets() {
    // Design section 8.9: `a->b` and `a>>b` are frozen, and the rules need spaces on both sides.
    for set in [RuleSet::PROSE, RuleSet::INTERFACE] {
        for language in languages() {
            let t = Typographer::new(language, set);
            for text in ["a->b", "a>>b", "x=>y", "a<-b", "i<=n", "a!=b"] {
                check_in("set", &t, (text, text));
            }
        }
    }
}

#[test]
fn symbols_in_every_output_kind() {
    use typolish::{EscapeStyle, HtmlStyle};
    let t = only(Language::EN_US, &[ARROWS, MATH]);
    let text = "foo -> bar <- baz <-> qux; a >= b, c <= d, e != f, g +- h";
    for (output, expected) in [
        (
            Output::Unicode,
            "foo → bar ← baz ↔ qux; a ≥ b, c ≤ d, e ≠ f, g ± h",
        ),
        (
            Output::Html(HtmlStyle::NAMED),
            "foo &rarr; bar &larr; baz &harr; qux; a &ge; b, c &le; d, e &ne; f, g &plusmn; h",
        ),
        (
            Output::Escaped(EscapeStyle::JS),
            "foo \\u2192 bar \\u2190 baz \\u2194 qux; a \\u2265 b, c \\u2264 d, e \\u2260 f, g \\u00B1 h",
        ),
        (
            Output::Escaped(EscapeStyle::RUST),
            "foo \\u{2192} bar \\u{2190} baz \\u{2194} qux; a \\u{2265} b, c \\u{2264} d, e \\u{2260} f, g \\u{b1} h",
        ),
    ] {
        assert_eq!(t.typeset(text, output), expected, "{output:?}");
    }
}

#[test]
fn symbols_with_a_slot_next_to_them() {
    for (what, t) in configs(&Language::EN_US, ARROWS) {
        let out = t.typeset_template(
            [
                Piece::Slot("a", Slot::WORD),
                Piece::Text(" -> "),
                Piece::Slot("b", Slot::WORD),
            ],
            Output::Unicode,
        );
        assert_eq!(out.texts(), ["", " → ", ""], "{what}");
        assert!(out.changed(), "{what}");
        let out = t.typeset_template(
            [
                Piece::Slot("a", Slot::OTHER),
                Piece::Text("->"),
                Piece::Slot("b", Slot::OTHER),
            ],
            Output::Unicode,
        );
        assert_eq!(out.texts(), ["", "->", ""], "{what}");
        assert!(!out.changed(), "{what}");
    }
}
