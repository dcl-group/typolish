//! The table of deliberate differences from the reference implementation (design section 8.12),
//! checked from the "Input" and "typolish (Prose)" columns of the table, and the header and the
//! counts of the generated reference tables (design section 13.4).
//!
//! A cell that the table gives as a fragment is checked as a fragment of the output; a cell marked
//! "glue aside" is compared with the no-break spaces turned into plain spaces.

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

use common::{prose, vis};
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
const EXACT_RU: &[(&str, &str, &str)] = &[
    ("PercentAsWritten", "Скидка 100 %.", "Скидка 100⍽%."),
    ("Technical", "a <-> b", "a ↔ b"),
    ("NoLastShortWord", "Что ж,", "Что ж,"),
    ("RangeLiteral", "Страницы 10-20.", "Страницы 10–20."),
    ("Apostrophe", "don't", "don’t"),
    ("RuNbsp", "Г. Москва", "Г.⍽Москва"),
    ("RuNbsp", "3 мин.", "3⍽мин."),
    ("RuNbsp", "Пушкин А.С.", "Пушкин⍽А.С."),
    ("RuNbsp", "над лесом", "над⍽лесом"),
    ("RuNbsp", "Знаешь ли ты", "Знаешь⍽ли⍽ты"),
    ("WideGlue", "Документ № 5", "Документ №⍽5"),
    ("NoDigitChange", "5 м2", "5⍽м2"),
    ("KeepSpaces", "Размер 10 x 5.", "Размер 10 × 5."),
    ("KeepSpaces", "А.С. Пушкин", "А.С.⍽Пушкин"),
];

const EXACT_EN: &[(&str, &str, &str)] = &[
    ("RangeLiteral", "1941-1945", "1941–1945"),
    ("NoLastShortWord", "Run X now.", "Run X now."),
    (
        "NoLastShortWord",
        "He went to the shop",
        "He went to⍽the⍽shop",
    ),
    ("NoLastShortWord", "a -> b", "a → b"),
    ("Apostrophe", "'90s", "’90s"),
    ("Apostrophe", "dogs'", "dogs’"),
    ("Apostrophe", "rock'n'roll", "rock’n’roll"),
    ("EnNbsp", "Mr. Smith", "Mr.⍽Smith"),
    ("EnNbsp", "J. R. R. Tolkien", "J.⍽R.⍽R.⍽Tolkien"),
    ("EnNbsp", "5 MB", "5⍽MB"),
];

/// Rows the table gives as "unchanged": the output equals the input, glue aside.
const UNCHANGED_RU: &[(&str, &str)] = &[
    ("NoSpaceEdit", "host:8080"),
    ("NoSpaceEdit", "::1"),
    ("NoSpaceEdit", "a,b,c"),
    ("NoSpaceEdit", "x,y;z"),
    ("NoDateRewrite", "2026-11-21"),
    ("Technical", "a>>b"),
    ("Technical", "0x1F"),
    ("Technical", "1/2/2026"),
    ("AuthorPunctuation", "А если.."),
    ("AuthorPunctuation", "Конец...."),
    ("AuthorPunctuation", "Да!!"),
    ("AuthorPunctuation", "Да!?"),
    ("NoSpelling", "кто то"),
    ("NoSpelling", "из за"),
    ("NoSpelling", "25-ый этаж"),
    ("LegalMarkContext", "Вариант (c)"),
    ("LegalMarkContext", "(R) знак"),
    ("NoDigitChange", "20C"),
    ("NoDigitChange", "$100"),
    ("AlreadyTypeset", "Что?…"),
    ("EllipsisDigits", "-5...5"),
    ("NoTrim", "Строка   \nвторая\t\n\n\n\nтретья  "),
];

#[test]
fn exact_rows_of_the_table() {
    let mut failures = Vec::new();
    for (language, rows) in [(Language::RU_RU, EXACT_RU), (Language::EN_US, EXACT_EN)] {
        for (reason, input, expected) in rows {
            let got = set(language.clone(), input);
            if got != vis(expected) {
                failures.push(format!(
                    "{reason}: {input:?} gives {:?}, the table says {expected:?}",
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
        if unglued(&got) != *input {
            failures.push(format!(
                "{reason}: {input:?} gives {:?}, the table says unchanged",
                common::show(&got)
            ));
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

#[test]
fn unmatched_quotes_are_not_guessed() {
    // NoGuess: unchanged, with UnmatchedQuote.
    for input in ["Он сказал \"привет.", "\"Один \"два три\""] {
        let got = set(Language::RU_RU, input);
        assert_eq!(unglued(&got), input, "NoGuess {input:?}");
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
        assert_eq!(unglued(&got), input, "AmbiguousHyphen {input:?}");
        assert!(
            diagnostics(Language::RU_RU, input).contains(&DiagnosticKind::AmbiguousHyphen),
            "AmbiguousHyphen {input:?}: no AmbiguousHyphen"
        );
    }
}

#[test]
fn fragments_of_the_table() {
    // Rows that the table gives as a fragment of the output.
    let cases: &[(Language, &str, &str, &str)] = &[
        (
            Language::EN_US,
            "NestedSingle",
            "He said \"a 'b' c\".",
            "“a⍽‘b’ c”",
        ),
        (Language::RU_RU, "Primes", "Угол 30° 15' 20\"", "15′ 20″"),
        (
            Language::EN_US,
            "DashStyleEn",
            "Moscow - the capital",
            "Moscow—the",
        ),
        (Language::RU_RU, "MinusSign", "Минус -5 градусов", "−5"),
        (Language::EN_US, "MinusSign", "Temp -12 °C", "−12"),
    ];
    for (language, reason, input, fragment) in cases {
        let got = set(language.clone(), input);
        assert!(
            got.contains(&vis(fragment)),
            "{reason}: {input:?} gives {:?}, the table has {fragment:?}",
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

/// Both generated tables start with the provenance comment of design section 13.4.
#[test]
fn the_tables_carry_their_provenance() {
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
    }
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
