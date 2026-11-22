//! What the decisions of ingest, windows and the classifier (design sections 3.3 to 3.7, 10.1)
//! look like in the output of the stage 2 rules (`quotes.*`, `dash.*`, `range.*`, `ellipsis.*`).
//!
//! Every row here also needs rules of the same stage, so a failure may come from either side; the
//! tables of `tests/protection.rs`, `tests/classifier.rs` and `tests/windows.rs` check the
//! decisions themselves. The rows run in the full Prose and Interface (stage 3, section 15.4): the
//! expectations show the quotes, dashes, ranges and ellipses and the glue of the `nbsp.*` rules,
//! so a row that holds a listed short word reads differently in the two sets (Prose also glues
//! every Cyrillic word of one or two letters, `Он⍽сказал`).

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

use std::borrow::Cow;

use common::show;
use typolish::{Language, Output, Piece, RuleSet, Slot, Typographer};

fn check(t: &Typographer, input: &str, expected: &str) {
    let expected = common::vis(expected);
    let got = t.typeset(input, Output::Unicode);
    assert!(
        got == expected,
        "input    {}\nexpected {}\ngot      {}",
        show(input),
        show(&expected),
        show(&got)
    );
    assert_eq!(
        matches!(got, Cow::Borrowed(_)),
        input == expected,
        "Cow::Borrowed exactly when unchanged: {}",
        show(input)
    );
}

/// The full Prose and the full Interface of a language, named.
fn sets_of(language: Language) -> [(&'static str, Typographer); 2] {
    [
        ("Prose", Typographer::new(language.clone(), RuleSet::PROSE)),
        ("Interface", Typographer::new(language, RuleSet::INTERFACE)),
    ]
}

fn ru_sets() -> [(&'static str, Typographer); 2] {
    sets_of(Language::RU_RU)
}

fn en_sets() -> [(&'static str, Typographer); 2] {
    sets_of(Language::EN_US)
}

/// Checks a row with both sets: `prose` and `interface` are the expectations of the two.
fn check_both(sets: &[(&'static str, Typographer); 2], input: &str, prose: &str, interface: &str) {
    for (name, t) in sets {
        let expected = if *name == "Prose" { prose } else { interface };
        check(t, input, expected);
    }
}

#[test]
fn tokens_span_adjacent_text_pieces() {
    // Section 3.4: the pieces of `Ждём...` are lexed as one text.
    for (_, t) in ru_sets() {
        let out =
            t.typeset_template::<()>([Piece::Text("Ждём."), Piece::Text("..")], Output::Unicode);
        assert_eq!(out.texts(), ["Ждём…"]);
        assert!(out.changed());
        let out = t.typeset_template::<()>(
            [Piece::Text("Ждём"), Piece::Text("..."), Piece::Text("")],
            Output::Unicode,
        );
        assert_eq!(out.texts(), ["Ждём…"]);
        check(&t, "Ждём...", "Ждём…");
    }
}

#[test]
fn markup_ends_a_token() {
    // Section 3.4: `.` and `..` are two `Dots` tokens and no ellipsis; an edit may not include markup.
    for (_, t) in ru_sets() {
        let out = t.typeset_template::<()>(
            [
                Piece::Text("Ждём."),
                Piece::Markup("<b>"),
                Piece::Text(".."),
            ],
            Output::Unicode,
        );
        assert_eq!(out.texts(), ["Ждём.<b>.."]);
        assert!(!out.changed());
    }
}

#[test]
fn a_slot_does_not_break_the_context_of_quotes() {
    // Section 13.2 (`tests/parts.rs`): one window, quotes paired across the slot.
    for (_, t) in ru_sets() {
        let out = t.typeset_template(
            [
                Piece::Text("Скажи \"раз\n"),
                Piece::Slot("s", Slot::OTHER),
                Piece::Text("\nдва\""),
            ],
            Output::Unicode,
        );
        assert_eq!(out.texts(), ["Скажи «раз\n", "\nдва»"]);
        let out = t.typeset_template(
            [
                Piece::Text("Файл \""),
                Piece::Slot("name", Slot::WORD),
                Piece::Text("\" не найден"),
            ],
            Output::Unicode,
        );
        // `не` is a listed short word: the glue lands at the start of the text after the slot.
        assert_eq!(out.texts(), ["Файл «", "» не\u{a0}найден"]);
    }
}

#[test]
fn a_slot_of_a_range_is_a_number() {
    // Section 3.6: two `Number` slots or two `Date` slots joined by `-` are a `Range`, whose dash
    // `range.placeholders` edits; every other pair is a compound word with frozen hyphens.
    for (_, t) in ru_sets() {
        let dash = |a: Slot, b: Slot| -> Vec<String> {
            t.typeset_template(
                [Piece::Slot("a", a), Piece::Text("-"), Piece::Slot("b", b)],
                Output::Unicode,
            )
            .texts()
            .iter()
            .map(|s| s.to_string())
            .collect()
        };
        assert_eq!(dash(Slot::NUMBER, Slot::NUMBER), ["", "\u{2013}", ""]);
        assert_eq!(dash(Slot::DATE, Slot::DATE), ["", "\u{2013}", ""]);
        assert_eq!(dash(Slot::NUMBER, Slot::DATE), ["", "-", ""]);
        assert_eq!(dash(Slot::WORD, Slot::WORD), ["", "-", ""]);
        // A `Number` slot and a literal number form a range that no rule edits.
        let out = t.typeset_template(
            [Piece::Slot("a", Slot::NUMBER), Piece::Text("-5")],
            Output::Unicode,
        );
        assert_eq!(out.texts(), ["", "-5"]);
        assert!(!out.changed());
        let out = t.typeset_template(
            [Piece::Text("5-"), Piece::Slot("d", Slot::DATE)],
            Output::Unicode,
        );
        assert!(!out.changed());
    }
}

#[test]
fn explicit_protection_keeps_a_command_out_of_the_dash_rules() {
    for (_, t) in en_sets() {
        check(
            &t,
            "Run `git checkout -- file` now.",
            "Run `git checkout -- file` now.",
        );
        let out = t.typeset_template(
            [
                Piece::Text("Run `git checkout -- "),
                Piece::Slot("b", Slot::WORD),
                Piece::Text("` to reset."),
            ],
            Output::Unicode,
        );
        // The command and its slot stay as they are; `to` is a short word, so the text after the
        // slot gets the glue (and is the only edit).
        assert_eq!(out.texts(), ["Run `git checkout -- ", "` to\u{a0}reset."]);
        assert!(out.changed());
        let out = t.typeset_template::<()>(
            [
                Piece::Text("Run `git checkout"),
                Piece::Text(" -- file` now."),
            ],
            Output::Unicode,
        );
        assert!(!out.changed());
        let out = t.typeset_template::<()>(
            [
                Piece::Text("Run "),
                Piece::Protected("git checkout -- file"),
                Piece::Text(" now."),
            ],
            Output::Unicode,
        );
        assert!(!out.changed());
    }
    // The same words without protection are typeset in Prose (section 10.8), so the rows above
    // test protection and not the absence of a rule.
    let prose = Typographer::new(Language::EN_US, RuleSet::PROSE);
    let plain = prose.typeset("Run git checkout -- file now.", Output::Unicode);
    assert_ne!(plain, "Run git checkout -- file now.");
}

#[test]
fn a_fenced_block_is_not_typeset_but_its_paragraphs_around_it_are() {
    // Prose glues `Он` to the word after it (`nbsp.short-length`), Interface does not.
    let sets = ru_sets();
    let input = "Он сказал \"да\".\n\n```\nx - y ... \"q\"\n\nz -- w\n```\n\nОн сказал \"нет\".";
    check_both(
        &sets,
        input,
        "Он⍽сказал «да».\n\n```\nx - y ... \"q\"\n\nz -- w\n```\n\nОн⍽сказал «нет».",
        "Он сказал «да».\n\n```\nx - y ... \"q\"\n\nz -- w\n```\n\nОн сказал «нет».",
    );
    let input = "   ```rust\nlet s = \"a...\";\n   ```\n\nОн сказал \"да\".";
    check_both(
        &sets,
        input,
        "   ```rust\nlet s = \"a...\";\n   ```\n\nОн⍽сказал «да».",
        "   ```rust\nlet s = \"a...\";\n   ```\n\nОн сказал «да».",
    );
}

#[test]
fn stray_backticks_switch_nothing_off() {
    // Section 10.1: `Don`t.\n\nОн сказал "да".\n\nIt`s ok.` gets `«да»`.
    let sets = ru_sets();
    check_both(
        &sets,
        "Don`t.\n\nОн сказал \"да\".\n\nIt`s ok.",
        "Don`t.\n\nОн⍽сказал «да».\n\nIt`s ok.",
        "Don`t.\n\nОн сказал «да».\n\nIt`s ok.",
    );
    // A lone backtick is an ordinary character, so no glue goes before it.
    check_both(
        &sets,
        "Он сказал \"да\" и `x",
        "Он⍽сказал «да» и `x",
        "Он сказал «да» и `x",
    );
}

#[test]
fn an_unpaired_quote_does_not_affect_the_next_paragraph() {
    // Section 3.3: the window is the unit of "all or nothing" in quote pairing.
    let sets = ru_sets();
    check_both(
        &sets,
        "Он сказал \"да.\n\nДругой \"нет\" ответ.",
        "Он⍽сказал \"да.\n\nДругой «нет» ответ.",
        "Он сказал \"да.\n\nДругой «нет» ответ.",
    );
    check_both(
        &sets,
        "Он сказал \"да.\u{2029}Другой \"нет\" ответ.",
        "Он⍽сказал \"да.\u{2029}Другой «нет» ответ.",
        "Он сказал \"да.\u{2029}Другой «нет» ответ.",
    );
    check_both(
        &sets,
        "Раз \"два\" три.\r\n\r\nЧетыре \"пять.\r\n \r\nШесть \"семь\".",
        "Раз «два» три.\r\n\r\nЧетыре \"пять.\r\n \r\nШесть «семь».",
        "Раз «два» три.\r\n\r\nЧетыре \"пять.\r\n \r\nШесть «семь».",
    );
}

#[test]
fn a_single_line_break_is_not_a_window_boundary() {
    // `да` before the line break gets no glue; `и` before `ушёл` does.
    check_both(
        &ru_sets(),
        "Он сказал \"да\nи ушёл\" совсем.",
        "Он⍽сказал «да\nи⍽ушёл» совсем.",
        "Он сказал «да\nи⍽ушёл» совсем.",
    );
}

#[test]
fn the_wrapper_is_editable_and_the_core_is_not() {
    // Section 3.6: the wrapper stays editable (it is sentence punctuation), the core is frozen.
    let sets = ru_sets();
    for (_, t) in &sets {
        check(t, "Адрес \"::1\"", "Адрес «::1»");
        check(t, "Папка \"/etc/\" пуста", "Папка «/etc/» пуста");
        check(
            t,
            "Токен \"SGVsbG8gd29ybGQ=\" и \"имя\".",
            "Токен «SGVsbG8gd29ybGQ=» и⍽«имя».",
        );
        check(
            t,
            "Задайте key=\"\" и нажмите \"OK\".",
            "Задайте key=\"\" и⍽нажмите «OK».",
        );
        check(
            t,
            "Подождите host:8080... дальше",
            "Подождите host:8080… дальше",
        );
    }
    // A quote glued to a punctuation-joined word belongs to a frozen fragment (section 10.8).
    check_both(
        &sets,
        "Он сказал:\"привет\" и ушёл.",
        "Он⍽сказал:\"привет\" и⍽ушёл.",
        "Он сказал:\"привет\" и⍽ушёл.",
    );
    for (_, t) in en_sets() {
        check(&t, "Run \"host:8080\" now.", "Run “host:8080” now.");
        check(&t, "Run \"x->\" now", "Run “x->” now");
        check(&t, "See host:8080... later", "See host:8080… later");
    }
}

#[test]
fn a_frozen_fragment_is_never_edited_inside() {
    // Section 3.7: a frozen fragment is byte for byte intact in both sets, next to quotes,
    // dashes and dots that are edited.
    for (_, t) in ru_sets().into_iter().chain(en_sets()) {
        for core in [
            "host:8080",
            "--force",
            "a,b,c",
            "10.0.0.0/8",
            "0x1F",
            "kv_lang",
            "user@example.com",
            "https://example.com/a?b=c",
            "0.2.27",
            "2026-11-21",
            "a>>b",
            "1'000'000",
        ] {
            let input = format!("Run {core} now.");
            let got = t.typeset(&input, Output::Unicode);
            assert!(
                got.contains(core),
                "{} came back as {}",
                show(&input),
                show(&got)
            );
            let input = format!("Run \"{core}\"...");
            let got = t.typeset(&input, Output::Unicode);
            assert!(
                got.contains(core),
                "{} came back as {}",
                show(&input),
                show(&got)
            );
        }
    }
}
