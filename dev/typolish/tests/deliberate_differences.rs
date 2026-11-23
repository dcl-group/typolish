//! The table of deliberate differences from the reference implementation (design section 8.12),
//! checked from the "Input" and "typolish (Prose)" columns of the table, and the header and the
//! counts of the generated reference tables (design section 13.4).
//!
//! A cell that the table gives as a fragment is checked as a fragment of the output; a cell marked
//! "unchanged" is compared with the input byte for byte; a cell marked "unchanged (glue aside)" is
//! compared with the no-break spaces turned into plain spaces.

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

use std::collections::BTreeSet;

use common::{Reason, prose, vis};
use typolish::{DiagnosticKind, Language, Output, Piece, Slot};

/// The output with every no-break space written as a plain one: "glue aside".
fn unglued(text: &str) -> String {
    text.replace(['\u{a0}', '\u{202f}'], " ")
}

/// Prose output of a plain string.
fn set(language: Language, input: &str) -> String {
    prose(language).typeset(input, Output::Unicode).into_owned()
}

/// The diagnostics of Prose on a plain string.
fn diagnostics(language: Language, input: &str) -> Vec<DiagnosticKind> {
    prose(language)
        .report(input, Output::Unicode)
        .diagnostics()
        .iter()
        .map(|d| d.kind().clone())
        .collect()
}

/// Rows whose output is the whole string as the table writes it (`⍽` is U+00A0).
const EXACT_RU: &[(Reason, &str, &str)] = &[
    (Reason::PercentAsWritten, "Скидка 100 %.", "Скидка 100⍽%."),
    (Reason::Technical, "a <-> b", "a ↔ b"),
    (
        Reason::NoLastShortWord,
        "-- Привет! -- сказал он.",
        "—⍽Привет!⍽— сказал он.",
    ),
    (
        Reason::NoLastShortWord,
        "Пушкин А. С. написал.",
        "Пушкин⍽А.⍽С. написал.",
    ),
    (Reason::RangeLiteral, "Страницы 10-20.", "Страницы 10–20."),
    (Reason::Apostrophe, "don't", "don’t"),
    (Reason::RuNbsp, "Г. Москва", "Г.⍽Москва"),
    (Reason::RuNbsp, "3 мин.", "3⍽мин."),
    (Reason::RuNbsp, "Пушкин А.С.", "Пушкин⍽А.С."),
    (Reason::RuNbsp, "над лесом", "над⍽лесом"),
    (Reason::RuNbsp, "20 мм и 5 м.", "20⍽мм⍽и⍽5⍽м."),
    (Reason::WideGlue, "Документ № 5", "Документ №⍽5"),
    (Reason::NoDigitChange, "5 м2", "5⍽м2"),
    (Reason::KeepSpaces, "Размер 10 x 5.", "Размер 10 × 5."),
    (Reason::KeepSpaces, "А.С. Пушкин", "А.С.⍽Пушкин"),
];

const EXACT_EN: &[(Reason, &str, &str)] = &[
    (Reason::RangeLiteral, "1941-1945", "1941–1945"),
    (Reason::NoLastShortWord, "Run X now.", "Run X now."),
    (
        Reason::NoLastShortWord,
        "He went to the shop",
        "He went to⍽the⍽shop",
    ),
    (Reason::NoLastShortWord, "a -> b", "a → b"),
    (Reason::Apostrophe, "'90s", "’90s"),
    (Reason::Apostrophe, "dogs'", "dogs’"),
    (Reason::Apostrophe, "rock'n'roll", "rock’n’roll"),
    (Reason::EnNbsp, "Mr. Smith", "Mr.⍽Smith"),
    (Reason::EnNbsp, "J. R. R. Tolkien", "J.⍽R.⍽R.⍽Tolkien"),
    (Reason::EnNbsp, "5 MB", "5⍽MB"),
];

/// Rows the table gives as "unchanged": the output equals the input byte for byte.
const UNCHANGED_RU: &[(Reason, &str)] = &[
    (Reason::NoSpaceEdit, "host:8080"),
    (Reason::NoSpaceEdit, "::1"),
    (Reason::NoSpaceEdit, "a,b,c"),
    (Reason::NoSpaceEdit, "x,y;z"),
    (Reason::NoDateRewrite, "2026-11-21"),
    (Reason::Technical, "a>>b"),
    (Reason::Technical, "0x1F"),
    (Reason::Technical, "1/2/2026"),
    (Reason::LegalMarkContext, "Вариант (c)"),
    (Reason::LegalMarkContext, "(R) знак"),
    (Reason::NoDigitChange, "20C"),
    (Reason::NoDigitChange, "$100"),
    (Reason::AlreadyTypeset, "Что?…"),
    (Reason::EllipsisDigits, "-5...5"),
    (Reason::NoTrim, "Строка   \nвторая\t\n\n\n\nтретья  "),
];

/// Rows the table gives as "unchanged (glue aside)": the output equals the input once its
/// no-break spaces are written as plain ones. The third column is the exact output of Prose, with
/// its glue.
const GLUE_ASIDE_RU: &[(Reason, &str, &str)] = &[
    (Reason::AuthorPunctuation, "А если..", "А⍽если.."),
    (Reason::AuthorPunctuation, "Конец....", "Конец...."),
    (Reason::AuthorPunctuation, "Да!!", "Да!!"),
    (Reason::AuthorPunctuation, "Да!?", "Да!?"),
    (Reason::NoSpelling, "кто то", "кто то"),
    (Reason::NoSpelling, "из за", "из⍽за"),
    (Reason::NoSpelling, "25-ый этаж", "25-ый⍽этаж"),
];

/// The codes of design section 8.12 that a test of their own checks:
/// `unmatched_quotes_are_not_guessed`, `look_alike_letters_are_not_replaced`,
/// `ambiguous_hyphen_is_left_alone_and_reported` and `slots_are_typed_elements`.
const DEDICATED: [Reason; 4] = [
    Reason::NoGuess,
    Reason::LookAlike,
    Reason::AmbiguousHyphen,
    Reason::Slots,
];

/// Rows that the table gives as a fragment of the output.
fn fragments() -> Vec<(Language, Reason, &'static str, &'static str)> {
    vec![
        (
            Language::EN_US,
            Reason::NestedSingle,
            "He said \"a 'b' c\".",
            "“a⍽‘b’ c”",
        ),
        (
            Language::RU_RU,
            Reason::Primes,
            "Угол 30° 15' 20\"",
            "15′ 20″",
        ),
        (
            Language::EN_US,
            Reason::DashStyleEn,
            "Moscow - the capital",
            "Moscow—the",
        ),
        (
            Language::RU_RU,
            Reason::MinusSign,
            "Минус -5 градусов",
            "−5",
        ),
        (Language::EN_US, Reason::MinusSign, "Temp -12 °C", "−12"),
    ]
}

#[test]
fn exact_rows_of_the_table() {
    let mut failures = Vec::new();
    for (language, rows) in [(Language::RU_RU, EXACT_RU), (Language::EN_US, EXACT_EN)] {
        for (reason, input, expected) in rows {
            let got = set(language.clone(), input);
            if got != vis(expected) {
                failures.push(format!(
                    "{reason:?}: {input:?} gives {:?}, the table says {expected:?}",
                    common::show(&got)
                ));
            }
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

#[test]
fn unchanged_rows_of_the_table() {
    let mut failures = Vec::new();
    for (reason, input) in UNCHANGED_RU {
        let got = set(Language::RU_RU, input);
        if got != *input {
            failures.push(format!(
                "{reason:?}: {input:?} gives {:?}, the table says unchanged",
                common::show(&got)
            ));
        }
    }
    for (reason, input, expected) in GLUE_ASIDE_RU {
        let expected = vis(expected);
        assert_eq!(unglued(&expected), *input, "{reason:?}: glue aside");
        let got = set(Language::RU_RU, input);
        if got != expected {
            failures.push(format!(
                "{reason:?}: {input:?} gives {:?}, expected {:?} (unchanged, glue aside)",
                common::show(&got),
                common::show(&expected)
            ));
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

#[test]
fn unmatched_quotes_are_not_guessed() {
    // NoGuess: the quotes stay as written, with UnmatchedQuote; the first row is glued after the
    // short word `Он` (design section 8.7), the second is unchanged.
    for (input, expected) in [
        ("Он сказал \"привет.", "Он⍽сказал \"привет."),
        ("\"Один \"два три\"", "\"Один \"два три\""),
    ] {
        let got = set(Language::RU_RU, input);
        assert_eq!(got, vis(expected), "NoGuess {input:?}");
        assert!(
            diagnostics(Language::RU_RU, input).contains(&DiagnosticKind::UnmatchedQuote),
            "NoGuess {input:?}: no UnmatchedQuote"
        );
    }
}

#[test]
fn look_alike_letters_are_not_replaced() {
    // LookAlike: Latin `o`, `C`, `x` inside Cyrillic words; unchanged, with MixedScript.
    for input in ["М\u{6f}сква", "\u{43}ервер", "\u{78}леб"] {
        let got = set(Language::RU_RU, input);
        assert_eq!(got, input, "LookAlike {input:?}");
        assert!(
            diagnostics(Language::RU_RU, input).contains(&DiagnosticKind::MixedScript),
            "LookAlike {input:?}: no MixedScript"
        );
    }
}

#[test]
fn ambiguous_hyphen_is_left_alone_and_reported() {
    for input in ["10 - 20 штук", "5 - 3 = 2"] {
        let got = set(Language::RU_RU, input);
        assert_eq!(got, input, "AmbiguousHyphen {input:?}");
        assert!(
            diagnostics(Language::RU_RU, input).contains(&DiagnosticKind::AmbiguousHyphen),
            "AmbiguousHyphen {input:?}: no AmbiguousHyphen"
        );
    }
}

#[test]
fn fragments_of_the_table() {
    for (language, reason, input, fragment) in fragments() {
        let got = set(language, input);
        assert!(
            got.contains(&vis(fragment)),
            "{reason:?}: {input:?} gives {:?}, the table has {fragment:?}",
            common::show(&got)
        );
    }
    // Primes: both quote characters after digits.
    assert_eq!(set(Language::EN_US, "5'10\""), "5′10″");
}

#[test]
fn slots_are_typed_elements() {
    // Slots: `{n} дн.`, `{a}-{b}`, `"{name}"`, as the table writes them out.
    let t = prose(Language::RU_RU);
    let write = |pieces: Vec<Piece<'static, &'static str>>| {
        let out = t.typeset_template(pieces, Output::Unicode);
        let mut text = String::new();
        for (i, part) in out.texts().iter().enumerate() {
            text.push_str(part);
            if let Some((name, _)) = out.slots().get(i) {
                text.push('{');
                text.push_str(name);
                text.push('}');
            }
        }
        text
    };
    assert_eq!(
        write(vec![Piece::Slot("n", Slot::NUMBER), Piece::Text(" дн.")]),
        vis("{n}⍽дн.")
    );
    assert_eq!(
        write(vec![
            Piece::Slot("a", Slot::NUMBER),
            Piece::Text("-"),
            Piece::Slot("b", Slot::NUMBER),
        ]),
        "{a}–{b}"
    );
    assert_eq!(
        write(vec![
            Piece::Text("\""),
            Piece::Slot("name", Slot::WORD),
            Piece::Text("\""),
        ]),
        "«{name}»"
    );
}

/// Every code of design section 8.12 has a row in these tables or a test of its own.
#[test]
fn every_reason_of_section_8_12_is_checked() {
    let mut checked: BTreeSet<Reason> = DEDICATED.into_iter().collect();
    checked.extend(EXACT_RU.iter().chain(EXACT_EN).map(|r| r.0));
    checked.extend(UNCHANGED_RU.iter().map(|r| r.0));
    checked.extend(GLUE_ASIDE_RU.iter().map(|r| r.0));
    checked.extend(fragments().into_iter().map(|r| r.1));
    let all: BTreeSet<Reason> = Reason::ALL.into_iter().collect();
    let missing: Vec<_> = all.difference(&checked).collect();
    assert!(
        missing.is_empty(),
        "codes of design section 8.12 without a check: {missing:?}"
    );
    assert_eq!(checked, all);
}

/// Both generated tables start with the same provenance comment of design section 13.4.
#[test]
fn the_tables_carry_their_provenance() {
    let mut pairs = Vec::new();
    for (name, text) in [
        ("ru.rs", include_str!("reference/ru.rs")),
        ("en_us.rs", include_str!("reference/en_us.rs")),
    ] {
        let header: Vec<&str> = text.lines().take_while(|l| l.starts_with("//")).collect();
        let header = header.join("\n");
        assert!(header.contains("7.9.1"), "{name}: version");
        assert!(
            header.contains("2026-11-21"),
            "{name}: date of the recording"
        );
        assert!(header.contains("default settings"), "{name}: settings");
        let hashes: Vec<&str> = header
            .split(|c: char| !c.is_ascii_hexdigit())
            .filter(|w| w.len() == 64)
            .collect();
        assert_eq!(hashes.len(), 2, "{name}: two SHA-256 sums");
        pairs.push((hashes[0].to_owned(), hashes[1].to_owned()));
    }
    assert_eq!(
        pairs[0], pairs[1],
        "ru.rs and en_us.rs name the same corpus and recording"
    );
    assert_ne!(pairs[0].0, pairs[0].1, "input and output sums differ");
}

/// The corpus of 820 cases: 432 in ru, 388 in en-US, 405 of them `neg.*` (design sections 13.3,
/// 13.4).
#[test]
fn the_tables_keep_the_corpus_counts() {
    assert_eq!(ru::ROWS.len(), 432);
    assert_eq!(en_us::ROWS.len(), 388);
    let neg = ru::ROWS
        .iter()
        .chain(en_us::ROWS)
        .filter(|r| r.id.starts_with("neg."))
        .count();
    assert_eq!(neg, 405);
}
