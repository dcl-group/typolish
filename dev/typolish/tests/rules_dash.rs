//! The tables of `dash.dialogue`, `dash.double-hyphen` and `dash.spaced-hyphen` (design sections
//! 8.1, 8.4, 10.5, 10.7 and 13.2).
//!
//! A row that names no configuration runs with the one rule of its file section, alone
//! (`RuleSet::EMPTY.with(rule)`), and, where a set holds the rule, in Prose and Interface without
//! the `nbsp.*` rules (stage 2 of design section 15.4; stage 3 switches the set-level tables to the
//! full sets, where `nbsp.dash` and `nbsp.short-words` add their glue). So a Russian dash keeps the
//! author's spaces here (`Москва — столица`) and an English one is closed (`Moscow—the capital`).
//! Rows that name Prose or Interface give the output of the whole set.

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
    interface, only, prose, show, vis, without_rule,
};
use probe::{diag_ranges, diagnostics_of};
use typolish::{
    Bind, DashStyle, DiagnosticKind, Language, LanguageBuilder, Output, Piece, Rule, RuleSet, Slot,
    Typographer,
};

const SPACED: Rule = Rule::DashSpacedHyphen;
const DOUBLE: Rule = Rule::DashDoubleHyphen;
const DIALOGUE: Rule = Rule::DashDialogue;

/// The text with each `AmbiguousHyphen` range of `t` in brackets.
fn marked(t: &Typographer, text: &str) -> String {
    let source = vis(text);
    let mut out = String::new();
    let mut at = 0;
    for r in diag_ranges(t, &source, &DiagnosticKind::AmbiguousHyphen) {
        out.push_str(&source[at..r.start]);
        out.push('[');
        out.push_str(&source[r.clone()]);
        out.push(']');
        at = r.end;
    }
    out.push_str(&source[at..]);
    out
}

// dash.spaced-hyphen, Russian (SpacedGlued: the author's spaces stay; `nbsp.dash` is stage 3).
const RU_SPACED: &[(&str, &str)] = &[
    ("Москва - столица", "Москва — столица"),
    ("Привет, - сказал он.", "Привет, — сказал он."),
    ("Да! - сказал он.", "Да! — сказал он."),
    ("слово - (пояснение)", "слово — (пояснение)"),
    ("кто-то - что-то", "кто-то — что-то"),
    ("Москва - столица - город", "Москва — столица — город"),
    ("Москва — столица", "Москва — столица"),
    ("Москва – столица", "Москва – столица"),
    ("Москва⍽- столица", "Москва⍽— столица"),
    ("кто-то, tls-server", "кто-то, tls-server"),
    (
        "Сервис слушает host:8080 - проверьте",
        "Сервис слушает host:8080 — проверьте",
    ),
    ("IT-отдел - что-то", "IT-отдел — что-то"),
    ("Глава 5 - введение", "Глава 5 — введение"),
    ("123-45-67 - звоните", "123-45-67 — звоните"),
    ("1990–2000 - годы", "1990–2000 — годы"),
    // U+2212 makes a flag-like operand only of a number (design section 8.4).
    ("слово - −abc", "слово — −abc"),
    // Refusals: the hyphen stays (and `AmbiguousHyphen`, tested below).
    ("слово - −5", "слово - −5"),
    ("10 - 20 штук", "10 - 20 штук"),
    ("5 - 3 = 2", "5 - 3 = 2"),
    ("5 - x", "5 - x"),
    ("x - 5", "x - 5"),
    ("a - b", "a - b"),
    ("Выполните cat - file", "Выполните cat - file"),
    ("tls-server - agent-ca", "tls-server - agent-ca"),
    ("123-45-67 - 123-45-68", "123-45-67 - 123-45-68"),
    ("cat - 123-45-67", "cat - 123-45-67"),
    ("Приём 10:00 - 12:00", "Приём 10:00 - 12:00"),
    (
        "Выполните diff -u - file.txt",
        "Выполните diff -u - file.txt",
    ),
    ("5–10 - 20", "5–10 - 20"),
    // A hyphen range with its percent sign attached is a percentage range, as with the sign after
    // spaces (design section 8.2).
    ("5 - 10-20%", "5 - 10-20%"),
    ("10-20% - 5", "10-20% - 5"),
    ("5 - 1,5-2,5%", "5 - 1,5-2,5%"),
    // A side of check 17 may be a Roman numeral, with the sign attached too (`XIX-XX %` is a range).
    ("XIX-XX% - 27", "XIX-XX% - 27"),
    ("5 - XIX-XX%", "5 - XIX-XX%"),
    ("Use - --force", "Use - --force"),
    ("tar -cf - -C dir", "tar -cf - -C dir"),
    // Not a pattern: no single space on each side, or a `--` run (design section 8.2).
    ("слово  -  слово", "слово  -  слово"),
    ("слово\t-\tслово", "слово\t-\tслово"),
    ("слово -\nслово", "слово -\nслово"),
    ("слово -", "слово -"),
    ("слово - ", "слово - "),
    // A `Dimensions` core is not an operand: matched by no dash rule, no diagnostic (design section
    // 8.4).
    ("1920x1080 - монитор", "1920x1080 - монитор"),
    ("1920x1080 — монитор", "1920x1080 — монитор"),
];

// Rows that the sets change with other rules of their own: Prose sets `-- ` and `- ` at a line
// start (`dash.double-hyphen`, `dash.dialogue`). `dash.spaced-hyphen` alone and in Interface
// leaves them, and does not match a `--` run, not even with an empty edit (design section 8.4).
const RU_SPACED_NOT_PROSE: &[(&str, &str)] = &[
    ("слово\n- слово", "слово\n- слово"),
    ("- слово", "- слово"),
    ("git checkout -- file", "git checkout -- file"),
    ("Запустите -- сейчас.", "Запустите -- сейчас."),
    ("Запустите --- сейчас.", "Запустите --- сейчас."),
];

const EN_SPACED_NOT_PROSE: &[(&str, &str)] = &[
    ("git checkout -- file", "git checkout -- file"),
    ("Run -- now.", "Run -- now."),
    ("Run --- now.", "Run --- now."),
];

#[test]
fn spaced_hyphen_in_russian() {
    // Alone, in Prose and in Interface: no other stage 2 rule edits these rows. `1920x1080` is
    // left out of the sets, where `number.times` (stage 3) reads it in Prose.
    let all = configs(&Language::RU_RU, SPACED);
    assert_eq!(all.len(), 3);
    let (dimension, plain): (Vec<_>, Vec<_>) =
        RU_SPACED.iter().partition(|r| r.0.contains("1920x"));
    let plain: Vec<(&str, &str)> = plain.into_iter().copied().collect();
    check_rows(&all, &plain);
    let alone = &all[..1];
    check_rows(alone, RU_SPACED_NOT_PROSE);
    check_rows(&all[2..], RU_SPACED_NOT_PROSE);
    let dimension: Vec<(&str, &str)> = dimension.into_iter().copied().collect();
    check_rows(alone, &dimension);
    for (what, t) in &all {
        assert_changed_by_with_glue(what, t, "Москва - столица", SPACED);
        assert_changed_by_with_glue(what, t, "Привет, - сказал он.", SPACED);
    }
}

// dash.spaced-hyphen, English (Closed: two plain words are joined, anything else keeps its spaces).
const EN_SPACED: &[(&str, &str)] = &[
    ("Moscow - the capital", "Moscow—the capital"),
    ("cat - file", "cat—file"),
    ("Moscow — the capital", "Moscow — the capital"),
    ("Moscow – the capital", "Moscow – the capital"),
    ("Moscow—the capital", "Moscow—the capital"),
    ("Moscow—the - capital", "Moscow—the - capital"),
    ("5 - x - Moscow", "5 - x—Moscow"),
    ("5 - x—Moscow", "5 - x—Moscow"),
    ("Hello, - he said", "Hello, — he said"),
    ("Chapter - 5", "Chapter — 5"),
    ("(word) - word", "(word) — word"),
    ("word - (word)", "word — (word)"),
    (
        "See https://example.com/docs - the guide",
        "See https://example.com/docs — the guide",
    ),
    ("Open host:8080 - now", "Open host:8080 — now"),
    (
        "mother-in-law - father-in-law",
        "mother-in-law — father-in-law",
    ),
    (
        "mother-in-law — father-in-law",
        "mother-in-law — father-in-law",
    ),
    ("tls-server - agent-ca", "tls-server — agent-ca"),
    ("word⍽- word", "word⍽— word"),
    ("word -⍽word", "word —⍽word"),
    ("word  -  word", "word  -  word"),
    ("word\t-\tword", "word\t-\tword"),
    ("a - b", "a - b"),
    ("5 - x", "5 - x"),
    ("x - 5", "x - 5"),
    ("10 - 20", "10 - 20"),
    ("5 - 10-20%", "5 - 10-20%"),
    ("10-20% - 5", "10-20% - 5"),
    ("V-X% - 5", "V-X% - 5"),
    ("x - V-X%", "x - V-X%"),
    ("123-45-67 - 123-45-68", "123-45-67 - 123-45-68"),
    ("diff -u - file.txt", "diff -u - file.txt"),
    ("tar -cf - -C dir", "tar -cf - -C dir"),
    ("Use - --force", "Use - --force"),
];

#[test]
fn spaced_hyphen_in_english() {
    let all = configs(&Language::EN_US, SPACED);
    check_rows(&all, EN_SPACED);
    check_rows(&all[..1], EN_SPACED_NOT_PROSE);
    check_rows(&all[2..], EN_SPACED_NOT_PROSE);
    for (what, t) in configs(&Language::EN_US, SPACED) {
        assert_changed_by_with_glue(&what, &t, "Moscow - the capital", SPACED);
    }
}

#[test]
fn spaced_hyphen_with_a_quote_after_it() {
    // Design section 8.4: a quote with an opening role keeps the spaces and the dash is set; a
    // quote without a role is not an opening quote, so there is no dash and no diagnostic.
    let t = only(Language::EN_US, &[SPACED]);
    check_in("alone", &t, ("He said - \"a cat\"", "He said — \"a cat\""));
    for set in [prose(Language::EN_US), interface(Language::EN_US)] {
        check_in("set", &set, ("He said - \"a cat\"", "He said — “a cat”"));
    }
    for (what, t) in configs(&Language::EN_US, SPACED) {
        let text = "He said - \" a cat\"";
        check_unchanged(&what, &t, text);
        assert_eq!(marked(&t, text), text, "{what}");
    }
    for t in [prose(Language::EN_US), interface(Language::EN_US)] {
        let kinds: Vec<_> = diagnostics_of(&t, "He said - \" a cat\"")
            .into_iter()
            .map(|d| d.0)
            .collect();
        assert!(kinds.contains(&DiagnosticKind::AmbiguousQuote), "{kinds:?}");
        assert!(kinds.contains(&DiagnosticKind::UnmatchedQuote), "{kinds:?}");
        assert!(
            !kinds.contains(&DiagnosticKind::AmbiguousHyphen),
            "{kinds:?}"
        );
    }
}

#[test]
fn a_sign_only_pair_is_a_quote_for_the_dash_rules() {
    // Design sections 6.3 and 8.2: the local rule includes the sign-only pair, so the dash rules
    // read `""` as an opening (or closing) quote on the first pass as on the second (design section
    // 4.3, I4).
    let ru_ui = interface(Language::RU_RU);
    let ru_prose = prose(Language::RU_RU);
    check_in("ru Interface", &ru_ui, ("Слово - \"\" да", "Слово — «» да"));
    check_in("ru Prose", &ru_prose, ("Слово - \"\" да", "Слово — «» да"));
    for (what, t) in [("ru Interface", &ru_ui), ("ru Prose", &ru_prose)] {
        check_in(what, t, ("\"\" - 5", "«» — 5"));
    }
    check_in("ru Prose", &ru_prose, ("- \"\" да", "—⍽«» да"));
    check_in("ru Prose", &ru_prose, ("-- \"\"", "—⍽«»"));
    check_in("ru Prose", &ru_prose, ("- \"-\" да", "—⍽«-» да"));
    check_in(
        "en Prose",
        &prose(Language::EN_US),
        ("Word - \"\" yes", "Word — “” yes"),
    );
    // The rule alone reads the same roles from the straight quotes.
    let t = only(Language::RU_RU, &[SPACED]);
    check_in("alone", &t, ("Слово - \"\" да", "Слово — \"\" да"));
    check_in("alone", &t, ("\"\" - 5", "\"\" — 5"));
    // Three quotes in a series have no sign-only pair: no role, no dash.
    check_unchanged("alone", &t, "Слово - \"\"\" да");
}

#[test]
fn spaced_hyphen_next_to_the_apostrophe_rule() {
    // `Sign in - it's free` and `Sign up - it's free` (design section 8.4): the rule alone leaves
    // the apostrophe, the sets also give `it’s`.
    let alone = only(Language::EN_US, &[SPACED]);
    check_in(
        "alone",
        &alone,
        ("Sign in - it's free", "Sign in—it's free"),
    );
    check_in(
        "alone",
        &alone,
        ("Sign up - it's free", "Sign up—it's free"),
    );
    for t in [prose(Language::EN_US), interface(Language::EN_US)] {
        check_in("set", &t, ("Sign in - it's free", "Sign in—it’s free"));
        check_in("set", &t, ("Sign up - it's free", "Sign up—it’s free"));
    }
}

#[test]
fn an_apostrophe_of_step_1_is_no_quote_for_the_dash_rules() {
    // Design sections 6.2, 6.3 and 8.2: `'tis` and `’tis` read alike (design section 4.3, I4).
    let alone = only(Language::EN_US, &[SPACED]);
    for text in [
        "x - 'tis",
        "x - ’tis",
        "x - '90s",
        "x - ’90s",
        "x - 'em now",
    ] {
        check_unchanged("alone", &alone, text);
    }
    for t in [prose(Language::EN_US), interface(Language::EN_US)] {
        check_in("set", &t, ("x - 'tis", "x - ’tis"));
        check_in("set", &t, ("x - '90s", "x - ’90s"));
        check_unchanged("set", &t, "x - ’tis");
        check_unchanged("set", &t, "x - ’90s");
    }
}

#[test]
fn spaced_hyphen_and_the_ellipsis_rule() {
    // Design section 13.2: `Ждите - ...` stays with `dash.spaced-hyphen` alone, with no
    // `AmbiguousHyphen` and no protected span; the sets also set the ellipsis.
    let t = only(Language::RU_RU, &[SPACED]);
    check_unchanged("alone", &t, "Ждите - ...");
    assert_eq!(marked(&t, "Ждите - ..."), "Ждите - ...");
    assert!(
        t.report("Ждите - ...", Output::Unicode)
            .protected()
            .is_empty()
    );
    for t in [prose(Language::RU_RU), interface(Language::RU_RU)] {
        check_in("set", &t, ("Ждите - ...", "Ждите - …"));
        check_unchanged("set", &t, "Ждите - …");
        assert_eq!(marked(&t, "Ждите - ..."), "Ждите - ...");
    }
}

#[test]
fn ambiguous_hyphens_are_reported_at_the_hyphen() {
    // Each row: the text and the text with the reported ranges in brackets.
    let cases: &[(Language, &str, &str)] = &[
        (Language::RU_RU, "10 - 20 штук", "10 [-] 20 штук"),
        (Language::RU_RU, "5 - 3 = 2", "5 [-] 3 = 2"),
        (
            Language::RU_RU,
            "Выполните cat - file",
            "Выполните cat [-] file",
        ),
        (
            Language::RU_RU,
            "tls-server - agent-ca",
            "tls-server [-] agent-ca",
        ),
        (
            Language::RU_RU,
            "Выполните diff -u - file.txt",
            "Выполните diff -u [-] file.txt",
        ),
        (Language::RU_RU, "5–10 - 20", "5–10 [-] 20"),
        (Language::RU_RU, "5 - 10-20%", "5 [-] 10-20%"),
        (Language::EN_US, "10-20% - 5", "10-20% [-] 5"),
        (
            Language::RU_RU,
            "123-45-67 - 123-45-68",
            "123-45-67 [-] 123-45-68",
        ),
        (Language::RU_RU, "cat - 123-45-67", "cat [-] 123-45-67"),
        // Two dictionary signs read as two technical fragments (design section 8.2).
        (Language::RU_RU, "км/ч - м/с", "км/ч [-] м/с"),
        // A time range with spaces: technical fragments on both sides (design sections 8.4, 10.8).
        (
            Language::RU_RU,
            "Приём 10:00 - 12:00",
            "Приём 10:00 [-] 12:00",
        ),
        (Language::EN_US, "x - 5", "x [-] 5"),
        (Language::EN_US, "5 - x", "5 [-] x"),
        (Language::EN_US, "5 - x - Moscow", "5 [-] x - Moscow"),
        (Language::EN_US, "tar -cf - -C dir", "tar -cf [-] -C dir"),
        (Language::EN_US, "Use - --force", "Use [-] --force"),
        (
            Language::EN_US,
            "123-45-67 - 123-45-68",
            "123-45-67 [-] 123-45-68",
        ),
        // No diagnostic: a pattern that does not match is not a refusal.
        (Language::RU_RU, "Москва - столица", "Москва - столица"),
        (Language::RU_RU, "слово  -  слово", "слово  -  слово"),
        (Language::RU_RU, "слово\t-\tслово", "слово\t-\tслово"),
        (Language::RU_RU, "Москва — столица", "Москва — столица"),
        (
            Language::RU_RU,
            "1920x1080 - монитор",
            "1920x1080 - монитор",
        ),
        (Language::EN_US, "5 - x—Moscow", "5 - x—Moscow"),
        (
            Language::EN_US,
            "Moscow—the - capital",
            "Moscow—the - capital",
        ),
        (Language::EN_US, "word  -  word", "word  -  word"),
        (Language::EN_US, "word\t-\tword", "word\t-\tword"),
        (
            Language::EN_US,
            "git checkout -- file",
            "git checkout -- file",
        ),
    ];
    for (language, text, expected) in cases {
        for (what, t) in configs(language, SPACED) {
            assert_eq!(marked(&t, text), *expected, "{what}: {}", show(text));
        }
    }
    // Two refusals, two diagnostics.
    let t = only(Language::RU_RU, &[SPACED]);
    assert_eq!(marked(&t, "5 - x и x - 5"), "5 [-] x и x [-] 5");
    // The diagnostic names the rule that raised it.
    let d = diagnostics_of(&t, "10 - 20 штук");
    assert_eq!(d.len(), 1);
    assert_eq!(d[0].2, Some(SPACED));
    assert_eq!(d[0].4, 3..4);
}

#[test]
fn a_slot_is_an_operand_of_the_spaced_hyphen() {
    // The slot participation table of design section 2.6, last row but one.
    let t = only(Language::RU_RU, &[SPACED]);
    let dash = |a: Slot, tail: &'static str| -> Vec<String> {
        t.typeset_template([Piece::Slot("x", a), Piece::Text(tail)], Output::Unicode)
            .texts()
            .iter()
            .map(|s| s.to_string())
            .collect()
    };
    for slot in [Slot::NUMBER, Slot::WORD, Slot::DATE, Slot::OTHER] {
        assert_eq!(dash(slot, " - слово"), ["", " — слово"], "{slot:?}");
    }
    // Next to a number, a range or a single-letter word: no dash.
    assert_eq!(dash(Slot::NUMBER, " - 5"), ["", " - 5"]);
    assert_eq!(dash(Slot::NUMBER, " - x"), ["", " - x"]);
    let out = t.typeset_template(
        [
            Piece::Slot("a", Slot::NUMBER),
            Piece::Text(" - "),
            Piece::Slot("b", Slot::NUMBER),
        ],
        Output::Unicode,
    );
    assert_eq!(out.texts(), ["", " - ", ""]);
    assert!(!out.changed());
    // `{a} - x` is a refusal; `Word` slots on both sides are words.
    let out = t.typeset_template(
        [Piece::Slot("a", Slot::WORD), Piece::Text(" - x")],
        Output::Unicode,
    );
    assert_eq!(out.texts(), ["", " — x"]);
    let out = t.typeset_template(
        [
            Piece::Slot("a", Slot::WORD),
            Piece::Text(" - "),
            Piece::Slot("b", Slot::WORD),
        ],
        Output::Unicode,
    );
    assert_eq!(out.texts(), ["", " — ", ""]);
    // `{a} - x` with a `Number` slot: ambiguous, reported at the hyphen.
    let r = t.report_template(
        [Piece::Slot("a", Slot::NUMBER), Piece::Text(" - x")],
        Output::Unicode,
    );
    assert!(!r.result().changed());
    let kinds: Vec<_> = r.diagnostics().iter().map(|d| d.kind().clone()).collect();
    assert_eq!(kinds, [DiagnosticKind::AmbiguousHyphen]);
    assert_eq!(r.diagnostics()[0].piece(), 1);
    assert_eq!(r.diagnostics()[0].range(), 1..2);
    // English Closed: a slot is not a plain word, so the spaces stay.
    let en = only(Language::EN_US, &[SPACED]);
    let out = en.typeset_template(
        [
            Piece::Slot("a", Slot::WORD),
            Piece::Text(" - "),
            Piece::Slot("b", Slot::WORD),
        ],
        Output::Unicode,
    );
    assert_eq!(out.texts(), ["", " — ", ""]);
}

// A `Dictionary` core without a full stop (a unit with `/`, `⋅`, `°` or a digit) is an operand, as
// the same text is when no list holds it: `бит/с` is a `Path` core, a technical fragment (design
// section 8.2). One with a full stop inside reads as an `Abbreviation` core, no operand.
const RU_UNIT_OPERANDS: &[(&str, &str)] = &[
    (
        "Скорость 5 кбит/с - это мало",
        "Скорость 5 кбит/с — это мало",
    ),
    (
        "Скорость в кбит/с - это мало",
        "Скорость в кбит/с — это мало",
    ),
    ("Скорость 5 бит/с - это мало", "Скорость 5 бит/с — это мало"),
    (
        "Скорость 60 км/ч - это много",
        "Скорость 60 км/ч — это много",
    ),
    ("Расход 5 кВт⋅ч - это много", "Расход 5 кВт⋅ч — это много"),
    ("Было 5 °C - это мало", "Было 5 °C — это мало"),
    ("Площадь 5 м2 - это мало", "Площадь 5 м2 — это мало"),
    ("это - 5 км/ч", "это — 5 км/ч"),
    ("5 км/ч - 10 км/ч", "5 км/ч — 10 км/ч"),
    // The refusal next to a number reads a unit as no number.
    ("5 - км/ч", "5 — км/ч"),
    // Two of them are refused, as two technical fragments are.
    ("км/ч - м/с", "км/ч - м/с"),
    ("₽ - $", "₽ - $"),
    // A sign of letters and digits only reads as a word, as without the list.
    ("м2 - м2", "м2 — м2"),
];

const EN_UNIT_OPERANDS: &[(&str, &str)] = &[
    // Not plain words, so the spaces stay in the closed style.
    ("Speed 60 km/h - fast", "Speed 60 km/h — fast"),
    ("Rate 5 MB/s - slow", "Rate 5 MB/s — slow"),
    ("It was 5 °F - cold", "It was 5 °F — cold"),
    ("°C - °F", "°C - °F"),
    ("Open at - a.m. hours", "Open at - a.m. hours"),
    ("See - e.g. this", "See - e.g. this"),
];

#[test]
fn a_unit_without_a_full_stop_is_an_operand() {
    // The rule alone; the sets add the glue of `nbsp.units`, which the rows below show.
    check_rows(&configs(&Language::RU_RU, SPACED)[..1], RU_UNIT_OPERANDS);
    check_rows(&configs(&Language::EN_US, SPACED)[..1], EN_UNIT_OPERANDS);
    // In the full sets `nbsp.units` and `nbsp.dash` glue around it, a typed dash included.
    for t in [prose(Language::RU_RU), interface(Language::RU_RU)] {
        for (text, expected) in [
            (
                "Скорость 5 кбит/с - это мало",
                "Скорость 5⍽кбит/с⍽— это мало",
            ),
            (
                "Скорость 5 кбит/с — это мало",
                "Скорость 5⍽кбит/с⍽— это мало",
            ),
            (
                "Скорость в кбит/с - это мало",
                "Скорость в⍽кбит/с⍽— это мало",
            ),
            (
                "Скорость 60 км/ч – это много",
                "Скорость 60⍽км/ч⍽– это много",
            ),
        ] {
            assert_eq!(t.typeset(&vis(text), Output::Unicode), vis(expected));
        }
    }
    for t in [prose(Language::EN_US), interface(Language::EN_US)] {
        assert_eq!(
            t.typeset("Speed 60 km/h - fast", Output::Unicode),
            vis("Speed 60⍽km/h — fast")
        );
    }
    // A unit that a caller adds does not change the dash rules: `бит/с` reads alike as a `Path`
    // core and as a unit.
    let t = Typographer::builder(Language::RU_RU, RuleSet::PROSE)
        .add_units(["бит/с"])
        .build()
        .unwrap();
    assert_eq!(
        t.typeset("Скорость 5 бит/с - это мало", Output::Unicode),
        vis("Скорость 5⍽бит/с⍽— это мало")
    );
}

// An entry of words joined by single hyphens (`г-жа`, `г-н`, a unit `kW-h` of `add_units`) reads
// as the compound word it is without the list, by the script of its letters (design section 8.2).
const RU_COMPOUND_ENTRIES: &[(&str, &str)] = &[
    ("г-жа - г-н", "г-жа — г-н"),
    ("Пришла г-жа - и ушла", "Пришла г-жа — и ушла"),
    ("г-н — это", "г-н — это"),
    ("Это - г-н Иванов", "Это — г-н Иванов"),
    ("г-н - кто-то", "г-н — кто-то"),
    ("г-н - cat", "г-н — cat"),
    ("5 км/ч - г-н", "5 км/ч — г-н"),
];

// `dash.dialogue` reads such an entry, and one of letters and digits (`м2`), as the word it is
// without the list; a unit read as a technical fragment stays, as `- бит/с` does.
const RU_DIALOGUE_ENTRIES: &[(&str, &str)] = &[
    ("- г-н Иванов", "—⍽г-н Иванов"),
    ("-- г-жа Петрова", "—⍽г-жа Петрова"),
    ("- м2 это", "—⍽м2 это"),
    ("- км/ч это", "- км/ч это"),
    ("- бит/с это", "- бит/с это"),
];

#[test]
fn an_entry_of_hyphenated_words_is_a_compound_word() {
    check_rows(&configs(&Language::RU_RU, SPACED)[..1], RU_COMPOUND_ENTRIES);
    check_rows(
        &configs(&Language::RU_RU, DIALOGUE)[..1],
        RU_DIALOGUE_ENTRIES,
    );
    // With the built-in abbreviations replaced, `г-жа` and `г-н` are `Compound` cores: the same
    // dash, and the same glue of `nbsp.dash`.
    let replaced = LanguageBuilder::from_language(&Language::RU_RU, "ru-RU")
        .abbreviations([(&["т.", "е."][..], Bind::Inner)])
        .build()
        .unwrap();
    for set in [RuleSet::PROSE, RuleSet::INTERFACE] {
        let listed = Typographer::new(Language::RU_RU, set);
        let plain = Typographer::new(replaced.clone(), set);
        for (text, expected) in [
            ("г-жа - г-н", "г-жа⍽— г-н"),
            ("Пришла г-жа - и ушла", "Пришла г-жа⍽— и⍽ушла"),
            ("г-н — это", "г-н⍽— это"),
            ("г-н - cat", "г-н⍽— cat"),
        ] {
            assert_eq!(
                listed.typeset(text, Output::Unicode),
                vis(expected),
                "{text}"
            );
            assert_eq!(
                plain.typeset(text, Output::Unicode),
                vis(expected),
                "{text}"
            );
            assert!(
                diagnostics_of(&listed, text).is_empty(),
                "{text}: {:?}",
                diagnostics_of(&listed, text)
            );
        }
    }
    // A unit with a hyphen that a caller adds changes no dash; a Latin one is a foreign compound
    // word in Russian text, as `tls-server` is.
    let rows: &[(&Language, &str, &str, &str)] = &[
        (
            &Language::EN_US,
            "kW-h",
            "Was 5 kW-h - kW-h",
            "Was 5⍽kW-h — kW-h",
        ),
        (
            &Language::RU_RU,
            "кВт-ч",
            "Было 5 кВт-ч - г-н",
            "Было 5⍽кВт-ч⍽— г-н",
        ),
        (
            &Language::RU_RU,
            "kW-h",
            "Было kW-h - kW-h",
            "Было kW-h - kW-h",
        ),
        (
            &Language::RU_RU,
            "kW-h",
            "Было kW-h - это",
            "Было kW-h⍽— это",
        ),
    ];
    for &(language, unit, text, expected) in rows {
        for set in [RuleSet::PROSE, RuleSet::INTERFACE] {
            let listed = Typographer::builder(language.clone(), set)
                .add_units([unit])
                .build()
                .unwrap();
            let plain = Typographer::new(language.clone(), set);
            let out = listed.typeset(text, Output::Unicode);
            assert_eq!(out, vis(expected), "{language} {text}");
            // Without the unit only the glue after a number differs.
            let unglued = plain.typeset(text, Output::Unicode);
            assert_eq!(
                out.replace('\u{a0}', " "),
                unglued.replace('\u{a0}', " "),
                "{language} {text}"
            );
            assert_eq!(
                diagnostics_of(&listed, text).len(),
                diagnostics_of(&plain, text).len(),
                "{language} {text}"
            );
        }
    }
}

#[test]
fn spaced_hyphen_follows_the_dash_style_setting() {
    let build = |language, style| {
        Typographer::builder(language, RuleSet::EMPTY.with(SPACED))
            .dash_style(style)
            .build()
            .unwrap()
    };
    let t = build(Language::RU_RU, DashStyle::Closed);
    check_in("ru closed", &t, ("Москва - столица", "Москва—столица"));
    check_in("ru closed", &t, ("5 - слово", "5 — слово"));
    let t = build(Language::RU_RU, DashStyle::Spaced);
    check_in("ru spaced", &t, ("Москва - столица", "Москва — столица"));
    let t = build(Language::EN_US, DashStyle::Spaced);
    check_in(
        "en spaced",
        &t,
        ("Moscow - the capital", "Moscow — the capital"),
    );
    let t = build(Language::EN_US, DashStyle::SpacedGlued);
    check_in(
        "en glued",
        &t,
        ("Moscow - the capital", "Moscow — the capital"),
    );
}

#[test]
fn the_rule_disabled() {
    for (language, text) in [
        (Language::RU_RU, "Москва - столица"),
        (Language::EN_US, "Moscow - the capital"),
    ] {
        for set in [RuleSet::PROSE, RuleSet::INTERFACE] {
            let t = without_rule(language.clone(), set, SPACED);
            check_unchanged("without dash.spaced-hyphen", &t, text);
        }
    }
    let t = without_rule(Language::RU_RU, RuleSet::PROSE, DOUBLE);
    check_unchanged("without dash.double-hyphen", &t, "Запустите -- сейчас.");
    let t = without_rule(Language::RU_RU, RuleSet::PROSE, DIALOGUE);
    check_unchanged("without dash.dialogue", &t, "- Привет!\n- Пока.");
    let t = Typographer::new(Language::RU_RU, RuleSet::EMPTY);
    check_unchanged("empty", &t, "Москва - столица -- город\n- Привет");
}

// dash.double-hyphen (Prose).
const RU_DOUBLE: &[(&str, &str)] = &[
    ("Запустите -- сейчас.", "Запустите — сейчас."),
    ("Москва -- столица", "Москва — столица"),
    ("Москва --- столица", "Москва — столица"),
    ("Москва⍽-- столица", "Москва⍽— столица"),
    ("кто-то -- что-то", "кто-то — что-то"),
    ("npm test -- --watch", "npm test -- --watch"),
    ("git checkout -- file", "git checkout -- file"),
    ("x -- y", "x -- y"),
    ("5 -- 3", "5 -- 3"),
    ("--", "--"),
    ("слово --", "слово --"),
    ("слово  --  слово", "слово  --  слово"),
    ("слово\t--\tслово", "слово\t--\tслово"),
    ("слово--слово", "слово--слово"),
    ("--force", "--force"),
    ("Москва — столица", "Москва — столица"),
];

const EN_DOUBLE: &[(&str, &str)] = &[
    ("Moscow -- the capital", "Moscow—the capital"),
    ("Moscow --- the capital", "Moscow—the capital"),
    // Design section 10.8: a command that is not protected becomes a dash in English text.
    ("git checkout -- file", "git checkout—file"),
    ("npm test -- --watch", "npm test -- --watch"),
    ("Use -- --force", "Use -- --force"),
    ("x -- y", "x -- y"),
    ("--", "--"),
    ("Moscow  --  the capital", "Moscow  --  the capital"),
    ("Moscow—the capital", "Moscow—the capital"),
];

#[test]
fn double_hyphen() {
    check_rows(&configs(&Language::RU_RU, DOUBLE), RU_DOUBLE);
    check_rows(&configs(&Language::EN_US, DOUBLE), EN_DOUBLE);
    for (what, t) in configs(&Language::RU_RU, DOUBLE) {
        assert_changed_by_with_glue(&what, &t, "Москва -- столица", DOUBLE);
        assert_changed_by_with_glue(&what, &t, "Москва --- столица", DOUBLE);
    }
    for (what, t) in configs(&Language::EN_US, DOUBLE) {
        assert_changed_by_with_glue(&what, &t, "Moscow -- the capital", DOUBLE);
    }
}

#[test]
fn double_hyphen_refusals_are_reported() {
    for (language, text, expected) in [
        (
            Language::RU_RU,
            "npm test -- --watch",
            "npm test [--] --watch",
        ),
        (
            Language::RU_RU,
            "git checkout -- file",
            "git checkout [--] file",
        ),
        (
            Language::EN_US,
            "npm test -- --watch",
            "npm test [--] --watch",
        ),
        (Language::EN_US, "x -- y", "x [--] y"),
        (
            Language::RU_RU,
            "Запустите -- сейчас.",
            "Запустите -- сейчас.",
        ),
        (
            Language::EN_US,
            "Moscow -- the capital",
            "Moscow -- the capital",
        ),
    ] {
        for (what, t) in configs(&language, DOUBLE) {
            assert_eq!(marked(&t, text), expected, "{what}: {}", show(text));
        }
    }
}

#[test]
fn double_hyphen_is_not_in_interface() {
    // Design section 10.5, row 3: in Interface a lone `--` stays, and `dash.spaced-hyphen` does not
    // match it, not even with an empty edit.
    for language in [Language::RU_RU, Language::EN_US] {
        let t = interface(language.clone());
        for text in [
            "Запустите -- сейчас.",
            "git checkout -- file",
            "Run -- now.",
            "--",
            "Москва --- столица",
        ] {
            check_unchanged(&format!("{language} Interface"), &t, text);
        }
    }
    // Interface keeps `dash.spaced-hyphen`.
    check_in(
        "Interface",
        &interface(Language::RU_RU),
        ("Москва - столица", "Москва — столица"),
    );
}

#[test]
fn a_slot_next_to_a_double_hyphen() {
    let t = only(Language::RU_RU, &[DOUBLE]);
    let out = t.typeset_template(
        [Piece::Slot("x", Slot::OTHER), Piece::Text(" -- слово")],
        Output::Unicode,
    );
    assert_eq!(out.texts(), ["", " — слово"]);
    assert!(out.changed());
    let out = t.typeset_template(
        [Piece::Text("слово -- "), Piece::Slot("x", Slot::WORD)],
        Output::Unicode,
    );
    assert_eq!(out.texts(), ["слово — ", ""]);
}

// dash.dialogue (Prose, languages with `dialogue`: ru).
const RU_DIALOGUE: &[(&str, &str)] = &[
    ("- Привет!", "—⍽Привет!"),
    ("- Привет!\n- Пока.", "—⍽Привет!\n—⍽Пока."),
    ("-- file", "—⍽file"),
    ("-- Привет", "—⍽Привет"),
    ("  - Привет", "  —⍽Привет"),
    ("\t- Привет", "\t—⍽Привет"),
    ("Он ушёл.\n\n- Привет", "Он ушёл.\n\n—⍽Привет"),
    ("Он ушёл.\r\n- Привет", "Он ушёл.\r\n—⍽Привет"),
    // A `Joined` core after the dash reads as its first word (design section 4.3, I4): the form
    // that the closed style gives `- река - ООО`, typed.
    ("- река—ООО", "—⍽река—ООО"),
    ("— река—ООО", "—⍽река—ООО"),
    // Not at a line start, no single space, no word after it.
    ("-Привет", "-Привет"),
    ("-  Привет", "-  Привет"),
    ("- 5 штук", "- 5 штук"),
    ("-", "-"),
    ("- ", "- "),
    ("---", "---"),
    // Already in the target form.
    ("—⍽Привет!", "—⍽Привет!"),
];

#[test]
fn dialogue_in_russian() {
    let all = configs(&Language::RU_RU, DIALOGUE);
    assert_eq!(all.len(), 2, "alone and Prose: Interface has no dialogue");
    check_rows(&all, RU_DIALOGUE);
    for (what, t) in &all {
        assert_changed_by(what, t, "- Привет!\n- Пока.", DIALOGUE);
        assert_changed_by(what, t, "-- file", DIALOGUE);
    }
    // Alone, a hyphen after a word is no dialogue.
    let t = only(Language::RU_RU, &[DIALOGUE]);
    check_unchanged("alone", &t, "Он сказал: - Привет");
    check_unchanged("alone", &t, "Привет - пока");
    // With the quote rules of Prose: an opening quote after the dash.
    check_in("alone", &t, ("- \"Привет\"", "—⍽\"Привет\""));
    // A label after the dash is a word (design section 8.4); only a `Joined` core reads as its
    // first word, a label does not read as its sign. Prose also splits the label (`nbsp.signs`).
    check_in("alone", &t, ("- №5 дом", "—⍽№5 дом"));
    for row in [
        ("- №5 дом", "—⍽№⍽5 дом"),
        ("— №5 дом", "—⍽№⍽5 дом"),
        ("- §5 текст", "—⍽§⍽5 текст"),
        ("- ¶IV дом", "—⍽¶⍽IV дом"),
        ("—⍽№⍽5 дом", "—⍽№⍽5 дом"),
    ] {
        check_in("Prose", &prose(Language::RU_RU), row);
    }
    check_in(
        "Prose",
        &prose(Language::RU_RU),
        ("- \"Привет\"", "—⍽«Привет»"),
    );
    check_in(
        "Prose",
        &prose(Language::RU_RU),
        ("- \"Привет!\"\n- \"Пока.\"", "—⍽«Привет!»\n—⍽«Пока.»"),
    );
}

#[test]
fn dialogue_is_not_in_interface_nor_in_english() {
    let t = interface(Language::RU_RU);
    for text in ["- Привет!\n- Пока.", "-- file", "- Привет"] {
        check_unchanged("ru Interface", &t, text);
    }
    for t in [prose(Language::EN_US), interface(Language::EN_US)] {
        for text in ["- Hello!\n- Bye.", "-- file", "- Hello"] {
            check_unchanged("en", &t, text);
        }
    }
    // `-- file` at a line start: ru Prose is a dialogue line, en stays (design section 13.2).
    check_in("ru Prose", &prose(Language::RU_RU), ("-- file", "—⍽file"));
    // `Language::COMMON` has no dash data: the dash rules are inert (design section 7.4).
    for set in [RuleSet::PROSE, RuleSet::INTERFACE] {
        let t = common::without_nbsp(Language::COMMON, set);
        for text in [
            "- Привет!",
            "-- file",
            "Moscow - the capital",
            "Запустите -- сейчас.",
        ] {
            check_unchanged("und", &t, text);
        }
    }
}

#[test]
fn a_typed_dialogue_dash_passes_the_prefilter() {
    // Design sections 3.2 and 8.4: a typed dash at a line start has its space glued; the prefilter
    // lets it through, so `typeset` and `report` agree with `dash.dialogue` alone.
    let t = only(Language::RU_RU, &[DIALOGUE]);
    for (input, expected) in [
        ("— Привет", "—⍽Привет"),
        ("– Привет", "–⍽Привет"),
        ("Текст\n— Привет", "Текст\n—⍽Привет"),
    ] {
        check_in("alone", &t, (input, expected));
        let source = common::input(input);
        let report = t.report(&source, Output::Unicode);
        assert_eq!(
            report.result(),
            &t.typeset(&source, Output::Unicode),
            "{input}"
        );
    }
}

#[test]
fn a_slot_next_to_the_dialogue_dash() {
    let t = only(Language::RU_RU, &[DIALOGUE]);
    let out = t.typeset_template(
        [Piece::Text("- "), Piece::Slot("name", Slot::WORD)],
        Output::Unicode,
    );
    assert_eq!(out.texts(), ["—\u{a0}", ""]);
    assert!(out.changed());
    let out = t.typeset_template(
        [Piece::Slot("name", Slot::WORD), Piece::Text("\n- Привет")],
        Output::Unicode,
    );
    assert_eq!(out.texts(), ["", "\n—\u{a0}Привет"]);
}

#[test]
fn interaction_strings_of_section_10_7_for_the_dash_rules() {
    // `-- file` at a line start; `word  -  word` and `word\t-\tword`; `word⍽- word`; `5 - x`.
    let ru_prose = prose(Language::RU_RU);
    let en_prose = prose(Language::EN_US);
    let ru_ui = interface(Language::RU_RU);
    let en_ui = interface(Language::EN_US);
    check_in("ru Prose", &ru_prose, ("-- file", "—⍽file"));
    check_unchanged("ru Interface", &ru_ui, "-- file");
    check_unchanged("en Prose", &en_prose, "-- file");
    check_unchanged("en Interface", &en_ui, "-- file");
    for t in [&en_prose, &en_ui] {
        check_unchanged("en", t, "word  -  word");
        check_unchanged("en", t, "word\t-\tword");
        check_in("en", t, ("word⍽- word", "word⍽— word"));
        check_unchanged("en", t, "5 - x");
        check_unchanged("en", t, "x - 5");
    }
    for t in [&ru_prose, &ru_ui] {
        check_unchanged("ru", t, "5 - x");
        check_unchanged("ru", t, "x - 5");
        assert_eq!(marked(t, "5 - x"), "5 [-] x");
        assert_eq!(marked(t, "x - 5"), "x [-] 5");
    }
    for t in [&en_prose, &en_ui] {
        assert_eq!(marked(t, "5 - x"), "5 [-] x");
        assert_eq!(marked(t, "x - 5"), "x [-] 5");
        assert_eq!(marked(t, "word  -  word"), "word  -  word");
        assert_eq!(marked(t, "word\t-\tword"), "word\t-\tword");
    }
}

#[test]
fn stress_strings_of_section_10_7_for_the_dash_rules() {
    // Interface: unchanged; Prose: unchanged, except the lone `--` between words of the language.
    for (language, frame) in [
        (Language::RU_RU, "Запустите X сейчас."),
        (Language::EN_US, "Run X now."),
    ] {
        for token in [
            "diff -u - file.txt",
            "tar -cf - -C dir",
            "npm test -- --watch",
            "Use - --force",
            "rm -rf --no-preserve-root /",
            "git commit -m msg --amend",
            "cargo build --release",
            "pre-commit",
            "my_var-name",
            "CamelCase-Mixed_id",
            "nats-server-2.10",
        ] {
            for t in [prose(language.clone()), interface(language.clone())] {
                check_unchanged("stress", &t, token);
                check_unchanged("stress in a frame", &t, &frame.replace('X', token));
            }
        }
    }
}
