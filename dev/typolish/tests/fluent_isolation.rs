//! Fluent isolation marks (design sections 5.4 and 13.2, `tests/fluent_isolation.rs`).
//!
//! `fluent-bundle` wraps every placeable in U+2068 and U+2069. The marks are `Format` tokens,
//! transparent for the rules: a run-time string with isolates around the values gets the same
//! typesetting as the string without them, and the isolates come out unchanged where they were.
//! In the tables below `‹` stands for U+2068 and `›` for U+2069 (and `⍽`, `⎵` as everywhere).

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

use common::{check_exact as check_in, interface, prose, show, vis};
use typolish::{Language, Output, Rule, RuleSet, Typographer};

const FSI: char = '\u{2068}';
const PDI: char = '\u{2069}';

/// Writes the isolate markers of a table as U+2068 and U+2069.
fn iso(text: &str) -> String {
    text.replace('‹', "\u{2068}").replace('›', "\u{2069}")
}

/// The same text without the isolates.
fn strip(text: &str) -> String {
    text.replace([FSI, PDI], "")
}

/// A row `(input, expected)` of a table, with the isolate markers written out. The glue markers
/// are handled by `check_in`.
#[track_caller]
fn check_iso(what: &str, t: &Typographer, row: (&str, &str)) {
    let input = iso(row.0);
    let expected = iso(row.1);
    check_in(what, t, (&input, &expected));
}

/// The two properties of design section 5.4 for one input with isolates: the isolates are where
/// they were, and the typesetting equals that of the input without them (the isolates removed from
/// the output).
#[track_caller]
fn same_as_without_isolates(what: &str, t: &Typographer, with_isolates: &str) {
    let input = iso(with_isolates);
    let plain = strip(&input);
    let isolated_out = t.typeset(&input, Output::Unicode).into_owned();
    let plain_out = t.typeset(&plain, Output::Unicode).into_owned();
    assert_eq!(
        strip(&isolated_out),
        plain_out,
        "{what}\ninput with isolates {}\nits output           {}\nplain input          {}\nits output           {}",
        show(&input),
        show(&isolated_out),
        show(&plain),
        show(&plain_out)
    );
    // The isolates stay, in order and in number.
    let marks = |s: &str| {
        s.chars()
            .filter(|c| matches!(*c, FSI | PDI))
            .collect::<String>()
    };
    assert_eq!(
        marks(&isolated_out),
        marks(&input),
        "{what}: {}",
        show(&input)
    );
}

fn sets(language: &Language) -> [(String, Typographer); 2] {
    [
        (format!("{language} Prose"), prose(language.clone())),
        (format!("{language} Interface"), interface(language.clone())),
    ]
}

/// Rows that read the same in both sets.
const RU_ROWS: &[(&str, &str)] = &[
    // Quotes around an isolated word (design section 5.4).
    ("Файл \"‹report›\" не найден", "Файл «‹report›» не⍽найден"),
    // Glue after `в`: the isolate is skipped, the word is seen.
    ("в ‹Москве›", "в⍽‹Москве›"),
    ("Живёт в ‹Москве›.", "Живёт в⍽‹Москве›."),
    // Glue before the unit: the isolate around `5` is skipped, the number is seen.
    ("‹5› МБ", "‹5›⍽МБ"),
    ("Скачано ‹5› МБ", "Скачано ‹5›⍽МБ"),
    ("Ждать ‹30› с.", "Ждать ‹30›⍽с."),
    // Dates and signs.
    ("‹21› ноября", "‹21›⍽ноября"),
    ("№ ‹5›", "№⍽‹5›"),
    // A dash next to an isolated word.
    ("‹Москва› - столица", "‹Москва›⍽— столица"),
    // An empty value: an empty isolate pair is a sign-only pair, as `""` is (design section 13.2).
    ("\"‹›\"", "«‹›»"),
    ("Имя: \"‹›\"", "Имя: «‹›»"),
    ("\"\"", "«»"),
    // Already typeset text with isolates comes back unchanged.
    ("в⍽‹Москве›", "в⍽‹Москве›"),
    ("‹5›⍽МБ", "‹5›⍽МБ"),
    ("Файл «‹report›» не⍽найден", "Файл «‹report›» не⍽найден"),
];

#[test]
fn russian_rows_with_isolates() {
    for (what, t) in sets(&Language::RU_RU) {
        for row in RU_ROWS {
            check_iso(&what, &t, *row);
        }
    }
}

#[test]
fn the_empty_isolate_pair_is_like_the_empty_string() {
    for (what, t) in sets(&Language::RU_RU) {
        check_in(&what, &t, ("\"\u{2068}\u{2069}\"", "«\u{2068}\u{2069}»"));
        check_in(&what, &t, ("\"\"", "«»"));
        // The isolates stay where they were, and the report has no protected span for the pair.
        let r = t.report("\"\u{2068}\u{2069}\"", Output::Unicode);
        assert!(r.protected().is_empty(), "{what}: {:?}", r.protected());
    }
}

#[test]
fn an_inch_mark_after_an_isolated_number() {
    // Prose: the quote after the number is a prime, the later pair is a pair of quotes.
    // `и` is a short word before an opening quote, so the glue joins them (design section 8.7).
    let p = prose(Language::RU_RU);
    check_iso(
        "ru Prose",
        &p,
        ("Экран ‹27›\" и \"ручка\"", "Экран ‹27›″ и⍽«ручка»"),
    );
    // The same string without the isolates.
    check_in(
        "ru Prose plain",
        &p,
        ("Экран 27\" и \"ручка\"", "Экран 27″ и⍽«ручка»"),
    );
    // Interface has no primes (design section 9.3): the quotes are paired as they stand.
    let ui = interface(Language::RU_RU);
    same_as_without_isolates("ru Interface", &ui, "Экран ‹27›\" и \"ручка\"");
}

const EN_ROWS: &[(&str, &str)] = &[
    ("Open \"‹file›\" now", "Open “‹file›” now"),
    ("Saved in ‹5› min", "Saved in⍽‹5›⍽min"),
    ("Size ‹5› MB", "Size ‹5›⍽MB"),
    ("on ‹21› May", "on⍽‹21›⍽May"),
    ("\"‹›\"", "“‹›”"),
];

#[test]
fn english_rows_with_isolates() {
    for (what, t) in sets(&Language::EN_US) {
        for row in EN_ROWS {
            check_iso(&what, &t, *row);
        }
    }
}

/// Inputs with isolates for the property: the typesetting equals that of the input without them.
const SAME_AS_PLAIN: &[&str] = &[
    "Файл \"‹report›\" не найден",
    "в ‹Москве› и в ‹Петербурге›",
    "‹5› МБ и ‹10› ГБ",
    "Сказал ‹Иван›: \"‹привет›\"",
    "Цена ‹5› руб. за ‹1› шт.",
    "‹Москва› - столица",
    "Он сказал бы ‹это›",
    "‹21› ноября ‹2026› г.",
    "Run ‹ls› in ‹the› box",
    "10 ‹x› 5",
    "Open \"‹a›\" and \"‹b›\" now",
    // An empty value between a sign and its number does not hide the sign: the number stays a
    // label, not a quantity (design sections 3.6 and 5.4).
    "№ ‹› ‹5› т. е.",
    "№‹› ‹› ‹100› ‹%›",
];

#[test]
fn an_empty_value_between_a_sign_and_its_number() {
    // The label of design section 3.6 looks past a fragment of isolates only (design section 5.4).
    for (what, t) in sets(&Language::RU_RU) {
        check_iso(&what, &t, ("№ ‹› 5 т.", "№ ‹› 5 т."));
        check_in(&what, &t, ("№  5 т.", "№  5 т."));
    }
}

#[test]
fn isolates_are_transparent_for_every_rule() {
    for language in [Language::RU_RU, Language::EN_US, Language::COMMON] {
        let mut typographers = sets(&language).to_vec();
        typographers.push((
            format!("{language} all"),
            Typographer::new(language.clone(), RuleSet::ALL),
        ));
        for (what, t) in &typographers {
            for text in SAME_AS_PLAIN {
                same_as_without_isolates(what, t, text);
            }
        }
    }
}

#[test]
fn isolates_are_transparent_to_each_nbsp_rule_alone() {
    for rule in Rule::ALL.iter().filter(|r| r.id().starts_with("nbsp.")) {
        let t = Typographer::new(Language::RU_RU, RuleSet::EMPTY.with(*rule));
        for text in SAME_AS_PLAIN {
            same_as_without_isolates(&format!("ru {}", rule.id()), &t, text);
        }
    }
}

#[test]
fn isolates_never_become_part_of_an_edit() {
    // Through `report`: no edit range covers an isolate.
    let input = iso("Файл \"‹report›\" не найден, в ‹Москве›, ‹5› МБ, ‹Москва› - столица.");
    for (what, t) in sets(&Language::RU_RU) {
        let r = t.report(&input, Output::Unicode);
        for change in r.changes() {
            let covered = &input[change.range()];
            assert!(
                !covered.contains([FSI, PDI]),
                "{what}: {change:?} covers an isolate in {}",
                show(&input)
            );
        }
        assert!(!r.changes().is_empty(), "{what}");
    }
}

#[test]
fn already_typeset_isolated_text_is_borrowed() {
    for (what, t) in sets(&Language::RU_RU) {
        for text in ["Файл «‹report›» не⍽найден", "в⍽‹Москве›", "‹5›⍽МБ", "«‹›»"]
        {
            let source = vis(&iso(text));
            let out = t.typeset(&source, Output::Unicode);
            assert!(
                matches!(out, Cow::Borrowed(_)),
                "{what}: {} came back as {}",
                show(&source),
                show(&out)
            );
        }
    }
}
