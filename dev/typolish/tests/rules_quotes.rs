//! The tables of `quotes.apostrophe`, `quotes.pairs` and `quotes.primes` (design sections 6, 8.3
//! and 13.2).
//!
//! A row that names no configuration runs with the three quote rules (design section 13.2), and the
//! rows marked as set-level also run inside the full Prose and Interface (stage 3 of design section
//! 15.4; the stage 2 tables ran them without the `nbsp.*` rules). As in design section 6.12 the
//! outputs show only the edits of the rules that run; in the full sets the glue of the `nbsp.*`
//! rules is added to the rows that it touches (`common::set_glue`, `common::expected_in_set`). Rows
//! of design section 6.12 that sit next to another stage 2 rule (an ellipsis, a spaced hyphen) run
//! with the three quote rules only.

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
    assert_changed_by, check_exact, check_in, check_unchanged, configs, interface, only, prose,
    show,
};
use probe::diagnostics_of;
use typolish::{
    DiagnosticKind, Language, Output, Piece, QuotePairs, Rule, RuleSet, Severity, SingleQuotes,
    Slot, Typographer,
};

const QUOTES: [Rule; 3] = [
    Rule::QuotesApostrophe,
    Rule::QuotesPairs,
    Rule::QuotesPrimes,
];

fn quotes(language: Language) -> Typographer {
    only(language, &QUOTES)
}

fn quotes_with(language: Language, mode: SingleQuotes) -> Typographer {
    Typographer::builder(
        language,
        RuleSet::EMPTY
            .with(QUOTES[0])
            .with(QUOTES[1])
            .with(QUOTES[2]),
    )
    .single_quotes(mode)
    .build()
    .unwrap()
}

/// A built-in set with a `SingleQuotes` mode.
fn set_with(language: Language, set: RuleSet, mode: SingleQuotes) -> Typographer {
    Typographer::builder(language, set)
        .single_quotes(mode)
        .build()
        .unwrap()
}

/// The three typographers of a language that a set-level row runs with: the quote rules alone,
/// Prose and Interface (both in full).
fn all_three(language: &Language) -> Vec<(String, Typographer)> {
    vec![
        (format!("{language} quotes alone"), quotes(language.clone())),
        (format!("{language} Prose"), prose(language.clone())),
        (format!("{language} Interface"), interface(language.clone())),
    ]
}

fn rows(language: &Language, rows: &[(&str, &str)]) {
    for (what, t) in all_three(language) {
        for row in rows {
            check_in(&what, &t, *row);
        }
    }
}

// Design section 6.12, Russian; each row holds with the quote rules alone and in both sets.
const RU_ROWS: &[(&str, &str)] = &[
    ("Он сказал \"привет\" и ушёл.", "Он сказал «привет» и ушёл."),
    ("\"Он сказал \"привет\"\"", "«Он сказал „привет“»"),
    ("Он сказал \"привет.", "Он сказал \"привет."),
    ("\"Один \"два три\"", "\"Один \"два три\""),
    ("Слово \" в тексте.", "Слово \" в тексте."),
    ("«a \"b» c\"", "«a \"b» c\""),
    (
        "Он ответил 'мы сказали \"нет\"'",
        "Он ответил 'мы сказали «нет»'",
    ),
    ("Файл \".env\" не найден", "Файл «.env» не найден"),
    ("Папка \"/etc/\" пуста", "Папка «/etc/» пуста"),
    ("Адрес \"::1\"", "Адрес «::1»"),
    ("Нажмите \"?\" для справки", "Нажмите «?» для справки"),
    // Sign-only pairs whose second quote is in a series of its own.
    ("Введите \"-\" или \"—\"", "Введите «-» или «—»"),
    ("Введите \"-\"", "Введите «-»"),
    // A series of three quotes of any form has no sign-only pair inside it (design section 6.3).
    ("a \"--\"\"\" b", "a «--»\"\" b"),
    ("a «--»\"\" b", "a «--»\"\" b"),
    ("\"?\"", "«?»"),
    ("«»", "«»"),
    ("Он сказал \"\" и ушёл.", "Он сказал «» и ушёл."),
    ("Пустая строка \"\".", "Пустая строка «»."),
    ("Он сказал \"\"\" и ушёл.", "Он сказал \"\"\" и ушёл."),
    ("Цена 27»", "Цена 27»"),
    ("Он сказал «привет „мир“»", "Он сказал «привет „мир“»"),
    (
        "Задайте key=\"value\" в файле.",
        "Задайте key=\"value\" в файле.",
    ),
    (
        "Он сказал:\"привет\" и ушёл.",
        "Он сказал:\"привет\" и ушёл.",
    ),
    (
        "Задайте key=\"\" и нажмите \"OK\".",
        "Задайте key=\"\" и нажмите «OK».",
    ),
    (
        "Поле Значение:\"\" и \"да\".",
        "Поле Значение:\"\" и \"да\".",
    ),
    ("Путь /etc/\"\" и \"да\".", "Путь /etc/\"\" и \"да\"."),
    (
        "Токен \"SGVsbG8gd29ybGQ=\" и \"имя\".",
        "Токен «SGVsbG8gd29ybGQ=» и «имя».",
    ),
    ("a\"\"b", "a\"\"b"),
    // Design section 6.7: typed pairs count in the levels.
    ("«Он сказал \"да\"»", "«Он сказал „да“»"),
    ("\"a \"b \"c\" d\" e\"", "«a „b «c» d“ e»"),
    // Design section 6.5: a weak quote pairs with an opener that pass 1 left unpaired.
    ("Он сказал \"диагональ 27\".", "Он сказал «диагональ 27»."),
    // Design section 13.2: an empty isolate pair is a sign-only pair; neutral glyphs never change.
    ("Имя: \"\u{2068}\u{2069}\"", "Имя: «\u{2068}\u{2069}»"),
    ("Он сказал ”привет” и ушёл.", "Он сказал ”привет” и ушёл."),
    // Design section 6.10: the scope is the window.
    ("\"a\" b \"c\n\n\"d\"", "\"a\" b \"c\n\n«d»"),
    ("\"a\n\n\"b\" c", "\"a\n\n«b» c"),
    // Design section 6.6: an unpaired straight single quote stays.
    ("Он сказал 'привет", "Он сказал 'привет"),
    // Design section 5.1: an emoji is a `Symbol` token that neighboring quotes treat as a word.
    ("\"🙂\"", "«🙂»"),
    ("Ответ: \"🙂\".", "Ответ: «🙂»."),
    // Design section 6.12: apostrophes in any script, look-through past `...`.
    ("Это rock'n'roll и don't.", "Это rock’n’roll и don’t."),
    ("Он писал: \"...и всё\".", "Он писал: «...и всё»."),
];

#[test]
fn russian_rows_of_section_6_12() {
    rows(&Language::RU_RU, RU_ROWS);
}

// Design section 6.12, English, `NestedOnly`; the rows hold with the quote rules alone and in both
// sets.
const EN_ROWS: &[(&str, &str)] = &[
    (
        "He said, \"I read \"War and Peace\"\".",
        "He said, “I read ‘War and Peace’”.",
    ),
    ("\"He said 'hi'\"", "“He said ‘hi’”"),
    ("\"a \"b \"c\" d\" e\"", "“a ‘b “c” d’ e”"),
    ("She said 'hi' to him.", "She said 'hi' to him."),
    (
        "I don't know. It's the '90s.",
        "I don’t know. It’s the ’90s.",
    ),
    (
        "Rock 'n' roll, the dogs' bowls",
        "Rock 'n' roll, the dogs’ bowls",
    ),
    ("She loves 'the dogs'", "She loves 'the dogs'"),
    ("Press '42' to confirm", "Press '42' to confirm"),
    ("Press 'y' or 'n' now", "Press 'y' or 'n' now"),
    ("Set the time to '12:30'", "Set the time to '12:30'"),
    ("\"Enter '42' now\"", "“Enter ‘42’ now”"),
    ("The class of '26", "The class of ’26"),
    ("She said 'he told \"no\"'", "She said 'he told “no”'"),
    ("Run \"x->\" now", "Run “x->” now"),
    ("a \"->\" b", "a “->” b"),
    ("“Say \"hi\" at 5”", "“Say ‘hi’ at 5”"),
    ("He is 5’10 tall.", "He is 5’10 tall."),
    (
        "‘Note: 'beta' builds of the developers' tools",
        "‘Note: 'beta' builds of the developers' tools",
    ),
    ("\"She said \"no.\"\"", "“She said ‘no.’”"),
    ("He said ‘x’(see note)", "He said ‘x’(see note)"),
    // Design section 13.2.
    ("the 90's", "the 90’s"),
    ("1'000'000", "1'000'000"),
    // Design section 6.2, step 1.
    ("O'Brien", "O’Brien"),
    ("Д'Артаньян", "Д’Артаньян"),
    ("rock'n'roll", "rock’n’roll"),
    ("'tis the season", "’tis the season"),
    ("Give 'em a hand", "Give ’em a hand"),
    ("Do it 'cause I said", "Do it ’cause I said"),
    ("It's already typeset: don’t", "It’s already typeset: don’t"),
    // Design section 6.6: the trailing possessive and the decade.
    ("the Joneses' house", "the Joneses’ house"),
    ("the dogs' bowls", "the dogs’ bowls"),
    ("He said 'hi", "He said 'hi"),
    // Design section 6.4: a closer covered only by straight single quotes unpairs them.
    ("\"a 'b\" c'", "“a 'b” c'"),
    // Design section 6.10.
    ("\"a\" b \"c\n\n\"d\"", "\"a\" b \"c\n\n“d”"),
    // Design section 6.6: the edits of step 1 stay in a window that does not converge.
    ("Don't \"x", "Don’t \"x"),
    ("\"x\" and \"y don't", "\"x\" and \"y don’t"),
    // Design section 6.1: a typed glyph of another language is neutral.
    ("He said «hi» there", "He said «hi» there"),
];

#[test]
fn english_rows_of_section_6_12() {
    rows(&Language::EN_US, EN_ROWS);
}

#[test]
fn an_elision_closed_by_a_straight_quote_waits_for_pairing() {
    // Design section 6.2: an elision of the list that a straight `'` closes at once (`'round'`)
    // can open a quoted word, so it waits for pairing, as `'n'` does; an elision that goes on into
    // the text stays an apostrophe. Rows: input, the quote rules alone, Prose and Interface.
    let rows: &[(&str, &str, &str)] = &[
        (
            "Choose the 'round' button",
            "Choose the 'round' button",
            "Choose the⍽'round' button",
        ),
        (
            "\"Say 'twas' or 'tis'\"",
            "“Say ‘twas’ or ‘tis’”",
            "“Say ‘twas’ or⍽‘tis’”",
        ),
        (
            "'Tis 'round' here",
            "’Tis 'round' here",
            "’Tis 'round' here",
        ),
        (
            "the 'round', then",
            "the 'round', then",
            "the⍽'round', then",
        ),
        ("Go 'round'", "Go 'round'", "Go 'round'"),
        (
            "He said 'tis the season and 'twas",
            "He said ’tis the season and ’twas",
            "He said ’tis the⍽season and ’twas",
        ),
        ("'cause it's late", "’cause it’s late", "’cause it’s late"),
        ("'em and 'til now", "’em and ’til now", "’em and ’til now"),
        (
            "'round the corner",
            "’round the corner",
            "’round the⍽corner",
        ),
    ];
    let alone = quotes(Language::EN_US);
    let sets = [prose(Language::EN_US), interface(Language::EN_US)];
    for &(input, quoted, full) in rows {
        check_exact("en quotes alone", &alone, (input, quoted));
        for t in &sets {
            check_exact(&format!("en {}", t.rules()), t, (input, full));
        }
    }
    // Unpaired, it is the elision (design section 6.6): with `Never` every `'` is unpaired, and the
    // output is a fixed point.
    let never = Typographer::builder(Language::EN_US, RuleSet::INTERFACE)
        .single_quotes(SingleQuotes::Never)
        .build()
        .unwrap();
    for (input, expected) in [
        ("\"Say 'twas' or 'tis'\"", "“Say ’twas’ or ’tis’”"),
        ("Go 'round'", "Go ’round'"),
    ] {
        check_exact("en Never", &never, (input, expected));
        check_unchanged("en Never", &never, expected);
    }
}

#[test]
fn an_apostrophe_inside_a_frozen_core_is_not_its_quote() {
    // Design section 6.1: a straight quote in the wrapper of a frozen fragment is a candidate only
    // if the core holds no straight quote, and an apostrophe between two letters is not one.
    for (what, t) in all_three(&Language::EN_US) {
        for row in [
            ("Say \"it's:fine\" now", "Say “it's:fine” now"),
            ("Say \"don't:stop\".", "Say “don't:stop”."),
            ("Say \"host:8080\" now", "Say “host:8080” now"),
            ("key=\"it's\"", "key=\"it's\""),
        ] {
            check_exact(&what, &t, row);
        }
    }
    for (what, t) in all_three(&Language::RU_RU) {
        check_exact(
            &what,
            &t,
            ("Файл \"it's:fine\" тут", "Файл «it's:fine» тут"),
        );
    }
}

#[test]
fn a_trailing_possessive_after_a_capital_s() {
    // Design section 6.6: the trailing possessive goes after the letter `s` or `S`. Rows: input,
    // the quote rules alone, Prose and Interface.
    let rows: &[(&str, &str, &str)] = &[
        ("JONES' BOOK", "JONES’ BOOK", "JONES’ BOOK"),
        (
            "DOGS' and JONES' BOOK",
            "DOGS’ and JONES’ BOOK",
            "DOGS’ and⍽JONES’ BOOK",
        ),
        (
            "DOG'S and DOGS' and JONES' BOOK",
            "DOG’S and DOGS’ and JONES’ BOOK",
            "DOG’S and⍽DOGS’ and⍽JONES’ BOOK",
        ),
        // A pair is not a possessive.
        ("'JONES'", "'JONES'", "'JONES'"),
        ("\"'JONES'\"", "“‘JONES’”", "“‘JONES’”"),
    ];
    let alone = quotes(Language::EN_US);
    let sets = [prose(Language::EN_US), interface(Language::EN_US)];
    for &(input, quoted, full) in rows {
        check_exact("en quotes alone", &alone, (input, quoted));
        for t in &sets {
            check_exact(&format!("en {}", t.rules()), t, (input, full));
        }
    }
}

#[test]
fn a_trailing_possessive_that_would_close_its_pair_stays_straight() {
    // Design section 6.6: inside a pair closed by `’`, a trailing possessive stays straight where a
    // second pass or `to_plain` would read it as the closer of the pair and change what lies
    // between them: a straight `'` opener that stayed unpaired, when a quote that could close it
    // follows the closer; or, when the closer follows `s` or `S` too, such an opener or a quote of
    // a pair.
    let en = quotes(Language::EN_US);
    for row in [
        ("\"\"s' 'x.\"'\"", "“‘s' 'x.’'”"),
        ("“\"s' '.\"'”", "“‘s' '.’'”"),
        ("\"a \"dogs' 'x\" it' b\"", "“a ‘dogs' 'x’ it' b”"),
        ("\"a \"dogs' 's\" b\"", "“a ‘dogs' 's’ b”"),
        ("\"a \"DOGS' 'S\" b\"", "“a ‘DOGS' 'S’ b”"),
        ("\"a \"dogs' 'x' bowls\" b\"", "“a ‘dogs' “x” bowls’ b”"),
        // Only text between them, or a closer that does not follow `s`: the possessive stays.
        (
            "\"He said \"the dogs' bowls\"\"",
            "“He said ‘the dogs’ bowls’”",
        ),
        ("\"a \"dogs' 'x' bowl\" b\"", "“a ‘dogs’ “x” bowl’ b”"),
        // An unpaired opener that nothing after the closer could close: a second pass pairs it
        // with nothing, so the possessive is placed.
        ("\"a \"dogs' 'x\" b\"", "“a ‘dogs’ 'x’ b”"),
        ("\"a \"dogs' 'x\" b\" it'", "“a ‘dogs’ 'x’ b” it'"),
    ] {
        check_exact("en quotes alone", &en, row);
    }
    // A language whose primary pair is `‘’` (en-GB).
    let gb = typolish::LanguageBuilder::from_language(&Language::EN_US, "en-GB")
        .quotes(Some(QuotePairs::new(('‘', '’'), ('“', '”'))))
        .build()
        .unwrap();
    let gb = quotes(gb);
    for row in [
        ("\"S' 's\"", "‘S' 's’"),
        ("\"the dogs' 'x' bowls\"", "‘the dogs' “x” bowls’"),
        ("\"the dogs' bowl\"", "‘the dogs’ bowl’"),
        ("\"the dogs' 'bowls\" today", "‘the dogs' 'bowls’ today"),
        ("\"s' 'x\" it'", "‘s' 'x’ it'"),
        ("\"s' 'x\" 5'", "‘s' 'x’ 5′"),
        ("\"s' 'x\"", "‘s’ 'x’"),
        ("\"s' '.\"", "‘s’ '.’"),
        (
            "\"I read 'Jones' and the dogs' 'house\"",
            "‘I read “Jones” and the dogs’ 'house’",
        ),
        // An opener placed as an apostrophe (the decade of design section 6.6) stays no opener.
        ("\"boss'\n'27\nbowls\"", "‘boss’\n’27\nbowls’"),
        (
            "it) - \"'cause) DOGS' - 'house' - oz.) '90s\" a",
            "it) - ‘’cause) DOGS' - “house” - oz.) ’90s’ a",
        ),
    ] {
        check_exact("en-GB quotes alone", &gb, row);
    }
}

#[test]
fn a_typed_closer_after_its_opener_and_signs_closes_it() {
    // Design section 6.3: `’` closes the typed opener of its pair when only signs (or nothing)
    // stand between them, the opener has L of class `Space`, `OpenP` or `DashC` and the `’` has R
    // of class `Space`, `CloseP` or `DashC`, as the sign-only pair `""` that a first pass typesets
    // as `‘’` (design section 6.11). Two adjacent quotes in a series of three are no pair.
    let en = quotes(Language::EN_US);
    for row in [
        ("\"a ‘’ b\"", "“a ‘’ b”"),
        ("\"a ‘-’ b\"", "“a ‘-’ b”"),
        ("\"a ‘’, b\"", "“a ‘’, b”"),
        ("\"a \"\" b\"", "“a ‘’ b”"),
        ("\"a ‘’’ b\"", "\"a ‘’’ b\""),
    ] {
        check_exact("en quotes alone", &en, row);
    }
    // A language whose primary pair is `‘’` (en-GB): the pair `‘’` is closed, so the opener that
    // pass 1 left unpaired takes the weak quote after the number on every pass (design section
    // 6.5).
    let gb = typolish::LanguageBuilder::from_language(&Language::EN_US, "en-GB")
        .quotes(Some(QuotePairs::new(('‘', '’'), ('“', '”'))))
        .build()
        .unwrap();
    let gb = quotes(gb);
    for row in [
        ("'a \"\"-3'", "'a ‘’-3'"),
        ("'a ‘’-3'", "'a ‘’-3'"),
        ("'a \"\" 3'", "'a ‘’ 3'"),
        ("'a ‘’ 3'", "'a ‘’ 3'"),
    ] {
        check_exact("en-GB quotes alone", &gb, row);
    }
}

#[test]
fn a_typed_sign_only_pair_at_a_foreign_level_is_no_pair() {
    // Design section 6.3: a typed sign-only pair `‘’` that would stand at a level whose glyphs are
    // not `‘’` is no pair, as in 0.1.1; the weak quote after the number is a prime.
    let gb = typolish::LanguageBuilder::from_language(&Language::EN_US, "en-GB")
        .quotes(Some(QuotePairs::new(('‘', '’'), ('“', '”'))))
        .build()
        .unwrap();
    let gb = quotes_with(gb, SingleQuotes::Everywhere);
    for row in [
        ("'a ‘’ 3'", "'a ‘’ 3′"),
        ("'a ‘’-3'", "'a ‘’-3′"),
        ("'‘(’ 0'", "'‘(’ 0′"),
        // The plain sign-only pair stands at its own level.
        ("'a \"\"-3'", "‘a “”-3’"),
    ] {
        check_exact("en-GB quotes alone, Everywhere", &gb, row);
    }
}

#[test]
fn a_window_that_a_later_pass_reads_otherwise_is_settled() {
    // Design section 6.6: the phase runs again over its result and takes what that run adds, so
    // the first pass gives what a second pass would; a window whose result changes the role of a
    // quote that another rule reads keeps its straight quotes, with `CrossingQuotes`.
    let en = quotes(Language::EN_US);
    for row in [
        // Sign-only pairs that share a quote.
        ("\"x ''-'' y\"", "“x ‘’-‘’ y”"),
        // A neutral `‘` that a later pass pairs, and a straight pair that it then encloses.
        ("He said ‘the dogs' \"5\"’", "He said ‘the dogs’ “5”’"),
        ("x ‘a 'b' \"2\"’ y", "x ‘a ‘b’ “2”’ y"),
        // A possessive inside a pair that a placed possessive or a covered `’` closes early.
        ("‘S' ‘s'’ s’", "‘S’ ‘s’’ s’"),
        ("‘S' \"x’\" s’", "‘S’ ‘x’’ s’"),
        // The `’` after `‘-` would close a sign-only pair on a second pass, which the dash reads.
        ("\"a '-’ - x' c\"", "\"a '-’ - x' c\""),
    ] {
        check_exact("en quotes alone", &en, row);
    }
    let gb = typolish::LanguageBuilder::from_language(&Language::EN_US, "en-GB")
        .quotes(Some(QuotePairs::new(('‘', '’'), ('“', '”'))))
        .build()
        .unwrap();
    let gb = quotes(gb);
    for row in [
        // The outer `‘` stays neutral and keeps the possessive straight, as in 0.1.1.
        ("‘\"-\" \"S'’\"", "‘‘-’ ‘S'’’"),
        ("\"s' y’ \"\" s\"", "‘s’ y’ “” s’"),
        ("\"-’ - x\"", "\"-’ - x\""),
    ] {
        check_exact("en-GB quotes alone", &gb, row);
    }
    let crossing = diagnostics_of(&en, "\"a '-’ - x' c\"");
    assert!(
        crossing
            .iter()
            .any(|d| d.0 == DiagnosticKind::CrossingQuotes),
        "{crossing:?}"
    );
}

// The rows of design section 6.12 that need `quotes.primes`: Prose (design section 6.5), and what
// Interface does.
#[test]
fn rows_with_primes_in_prose_and_interface() {
    let ru: &[(&str, &str, &str)] = &[
        (
            "Экран 27\" и \"ручка\"",
            "Экран 27″ и «ручка»",
            "Экран 27\" и «ручка»",
        ),
        (
            "\"Экран 27\" дюймов\"",
            "«Экран 27″ дюймов»",
            "«Экран 27\" дюймов»",
        ),
        (
            "\"Монитор 24\" и ручка\"",
            "«Монитор 24″ и ручка»",
            "«Монитор 24\" и ручка»",
        ),
        (
            "\"Диагональ 27\" и экран 24\"",
            "«Диагональ 27» и экран 24″",
            "«Диагональ 27» и экран 24\"",
        ),
        (
            "Угол 30° 15' 20\" и точка.",
            "Угол 30° 15′ 20″ и точка.",
            "Угол 30° 15' 20\" и точка.",
        ),
    ];
    let en: &[(&str, &str, &str)] = &[
        ("\"a 'b c\" 5'", "“a 'b c” 5′", "“a 'b c” 5'"),
        (
            "He is 5'10\" tall.",
            "He is 5′10″ tall.",
            "He is 5'10\" tall.",
        ),
    ];
    for (language, table) in [(Language::RU_RU, ru), (Language::EN_US, en)] {
        let with = [
            (
                format!("{language} primes alone"),
                only(language.clone(), &QUOTES),
            ),
            (format!("{language} Prose"), prose(language.clone())),
        ];
        for (what, t) in &with {
            for (input, output, _) in table {
                check_in(what, t, (input, output));
            }
        }
        let without = [
            (format!("{language} Interface"), interface(language.clone())),
            (
                format!("{language} pairs and apostrophe"),
                only(
                    language.clone(),
                    &[Rule::QuotesApostrophe, Rule::QuotesPairs],
                ),
            ),
        ];
        for (what, t) in &without {
            for (input, _, output) in table {
                check_in(what, t, (input, output));
            }
        }
    }
}

#[test]
fn the_three_rules_alone() {
    // Classification does not depend on which rules are enabled; a disabled rule suppresses its
    // own edits only (design section 6).
    let source = "\"Say don't\" 5'10\"";
    let cases = [
        // The rule alone, and Prose without it.
        (
            Rule::QuotesApostrophe,
            "\"Say don’t\" 5'10\"",
            "“Say don't” 5′10″",
        ),
        (
            Rule::QuotesPairs,
            "“Say don't” 5'10\"",
            "\"Say don’t\" 5′10″",
        ),
        (
            Rule::QuotesPrimes,
            "\"Say don't\" 5′10″",
            "“Say don’t” 5'10\"",
        ),
    ];
    for (rule, alone, prose_without) in cases {
        let t = only(Language::EN_US, &[rule]);
        check_in(rule.id(), &t, (source, alone));
        assert_changed_by(rule.id(), &t, source, rule);
        let off = common::without_rule(Language::EN_US, RuleSet::PROSE, rule);
        check_in(
            &format!("Prose without {rule}"),
            &off,
            (source, prose_without),
        );
    }
    // `quotes.apostrophe` disabled: `don't` still does not take part in pairing (design section 6).
    let t = only(Language::EN_US, &[Rule::QuotesPairs, Rule::QuotesPrimes]);
    check_in(
        "no apostrophe",
        &t,
        ("She said \"don't\"", "She said “don't”"),
    );
    assert!(diagnostics_of(&t, "She said \"don't\"").is_empty());
    // `quotes.pairs` disabled in Interface: straight quotes stay, apostrophes are set.
    let t = common::without_rule(Language::EN_US, RuleSet::INTERFACE, Rule::QuotesPairs);
    check_in("no pairs", &t, ("He said \"don't\"", "He said \"don’t\""));
    // Every rule off: nothing changes.
    let t = Typographer::new(Language::EN_US, RuleSet::EMPTY);
    check_unchanged("empty", &t, "He said \"don't\" at 5'10\"");
}

#[test]
fn edits_are_attributed_to_the_rule_whose_behavior_they_are() {
    let t = quotes(Language::EN_US);
    let r = t.report("He said \"don't\" at 5'10\".", Output::Unicode);
    let got: Vec<(Rule, String)> = r
        .changes()
        .iter()
        .map(|c| (c.rule(), c.replacement().to_owned()))
        .collect();
    assert_eq!(
        got,
        [
            (Rule::QuotesPairs, "“".to_owned()),
            (Rule::QuotesApostrophe, "’".to_owned()),
            (Rule::QuotesPairs, "”".to_owned()),
            (Rule::QuotesPrimes, "′".to_owned()),
            (Rule::QuotesPrimes, "″".to_owned()),
        ]
    );
    for change in r.changes() {
        assert_eq!(change.piece(), 0);
        assert_eq!(r.original(change).len(), 1);
    }
    let t = quotes(Language::RU_RU);
    assert_changed_by(
        "pairs",
        &t,
        "Он сказал \"привет\" и ушёл.",
        Rule::QuotesPairs,
    );
}

#[test]
fn diagnostics_of_section_6_12() {
    use DiagnosticKind::{AmbiguousQuote, CrossingQuotes, UnmatchedQuote};
    use Severity::{Info, Warning};
    let ru = [
        ("Он сказал \"привет.", vec![(UnmatchedQuote, Warning)]),
        ("\"Один \"два три\"", vec![(UnmatchedQuote, Warning)]),
        ("Слово \" в тексте.", vec![(AmbiguousQuote, Info)]),
        ("«a \"b» c\"", vec![(CrossingQuotes, Warning)]),
        ("Он сказал \"\"\" и ушёл.", vec![(AmbiguousQuote, Info); 3]),
        ("Цена 27»", vec![(UnmatchedQuote, Info)]),
        // Design section 6.6: a neutral closer inside a new pair of its family.
        (
            "Он сказал \"a » b\"",
            vec![(UnmatchedQuote, Info), (CrossingQuotes, Warning)],
        ),
        (
            "Поле Значение:\"\" и \"да\".",
            vec![(UnmatchedQuote, Warning)],
        ),
        ("Путь /etc/\"\" и \"да\".", vec![(UnmatchedQuote, Warning)]),
        // No diagnostic.
        ("a\"\"b", vec![]),
        ("Он сказал:\"привет\" и ушёл.", vec![]),
        ("\"?\"", vec![]),
        ("Он сказал \"\" и ушёл.", vec![]),
        ("Имя: \"\u{2068}\u{2069}\"", vec![]),
        ("Задайте key=\"\" и нажмите \"OK\".", vec![]),
        ("Токен \"SGVsbG8gd29ybGQ=\" и \"имя\".", vec![]),
        ("Экран 27\" и \"ручка\"", vec![]),
        ("Он сказал 'привет", vec![]),
    ];
    let en = [
        ("He said ‘x’(see note)", vec![(UnmatchedQuote, Info)]),
        (
            "‘Note: 'beta' builds of the developers' tools",
            vec![(UnmatchedQuote, Info)],
        ),
        ("Don't \"x", vec![(UnmatchedQuote, Warning)]),
        ("He said 'hi", vec![]),
        ("She said 'hi' to him.", vec![]),
        ("He is 5'10\" tall.", vec![]),
        ("The class of '26", vec![]),
    ];
    for (language, table) in [(Language::RU_RU, &ru[..]), (Language::EN_US, &en[..])] {
        for (what, t) in all_three(&language) {
            for (text, expected) in table {
                let got: Vec<_> = diagnostics_of(&t, text)
                    .into_iter()
                    .filter(|d| matches!(d.0, UnmatchedQuote | CrossingQuotes | AmbiguousQuote))
                    .map(|d| (d.0, d.1))
                    .collect();
                assert_eq!(&got, expected, "{what}: {}", show(text));
            }
        }
    }
}

#[test]
fn diagnostics_name_the_rule_and_the_place() {
    // The example of design section 2.9: the unmatched quote is the one at byte 18.
    let t = interface(Language::RU_RU);
    let d = diagnostics_of(&t, "Он сказал \"привет.");
    assert_eq!(d.len(), 1);
    assert_eq!(d[0].0, DiagnosticKind::UnmatchedQuote);
    assert_eq!(d[0].1, Severity::Warning);
    assert_eq!(d[0].2, Some(Rule::QuotesPairs));
    assert_eq!((d[0].3, d[0].4.clone()), (0, 18..19));
    // The leftover opener of `"Один "два три"` is the first quote.
    let d = diagnostics_of(&t, "\"Один \"два три\"");
    assert_eq!(d[0].4, 0..1);
    // The covering entry of `«a "b» c"` is the straight quote (design section 6.4).
    let d = diagnostics_of(&t, "«a \"b» c\"");
    assert_eq!(d[0].0, DiagnosticKind::CrossingQuotes);
    assert_eq!(d[0].4, 4..5);
    // A quote without a role is the diagnostic's range.
    let d = diagnostics_of(&t, "Слово \" в тексте.");
    assert_eq!(&"Слово \" в тексте."[d[0].4.clone()], "\"");
}

#[test]
fn protected_spans_of_quote_rows_are_empty() {
    // Design section 13.2: the quotes in these rows belong to no frozen fragment.
    let t = prose(Language::RU_RU);
    for text in [
        "\"?\"",
        "«»",
        "Имя: \"\u{2068}\u{2069}\"",
        "Он сказал \"\" и ушёл.",
        "Пустая строка \"\".",
    ] {
        let r = t.report(text, Output::Unicode);
        assert!(
            r.protected().is_empty(),
            "{}: {:?}",
            show(text),
            r.protected()
        );
    }
}

#[test]
fn single_quote_modes() {
    // Design section 6.8. `NestedOnly` is the default.
    let en = |mode| quotes_with(Language::EN_US, mode);
    let ru = |mode| quotes_with(Language::RU_RU, mode);
    let nested = quotes(Language::EN_US);
    let explicit = en(SingleQuotes::NestedOnly);
    for text in ["She said 'hi'", "echo 'hi'", "\"He said 'hi'\""] {
        assert_eq!(
            nested.typeset(text, Output::Unicode),
            explicit.typeset(text, Output::Unicode),
            "{text}"
        );
    }
    check_unchanged("nested only", &explicit, "She said 'hi'");
    check_unchanged("nested only", &explicit, "echo 'hi'");

    let t = en(SingleQuotes::Everywhere);
    let rows: &[(&str, &str)] = &[
        ("'hi'", "“hi”"),
        ("She said 'hi' to him.", "She said “hi” to him."),
        ("She loves 'the dogs'", "She loves “the dogs”"),
        ("Rock 'n' roll", "Rock “n” roll"),
        ("She said 'he told \"no\"'", "She said “he told ‘no’”"),
        ("\"x 'y\" z'", "“x 'y” z'"),
        ("don't 'x'", "don’t “x”"),
    ];
    for row in rows {
        check_in("Everywhere", &t, *row);
    }
    let t = ru(SingleQuotes::Everywhere);
    check_in("Everywhere", &t, ("'hi'", "«hi»"));
    // Design section 6.6: a neutral `»` inside the new pair `"…c"` would pair with its `«` on a
    // second pass and let `5'` close the first `'`; the window does not converge, in every mode.
    for mode in [
        SingleQuotes::Everywhere,
        SingleQuotes::NestedOnly,
        SingleQuotes::Never,
    ] {
        for set in [RuleSet::INTERFACE, RuleSet::PROSE] {
            let t = set_with(Language::RU_RU, set, mode);
            check_unchanged("neutral closer", &t, "x '\"»b 5' c\"");
            check_unchanged("neutral closer", &t, "'\"»90'\"");
        }
        let t = set_with(Language::EN_US, RuleSet::INTERFACE, mode);
        check_unchanged("neutral closer", &t, "’'\"”»90'„\"");
    }
    check_in(
        "Everywhere",
        &t,
        (
            "Он ответил 'мы сказали \"нет\"'",
            "Он ответил «мы сказали „нет“»",
        ),
    );

    // `Never`: every `'` is unpaired; only step 1, the trailing possessive and the decade apply.
    let t = en(SingleQuotes::Never);
    check_in(
        "Never",
        &t,
        ("She loves 'the dogs'", "She loves 'the dogs’"),
    );
    check_in("Never", &t, ("'42'", "’42′"));
    check_in("Never", &t, ("\"He said 'hi'\"", "“He said 'hi'”"));
    let t = set_with(Language::EN_US, RuleSet::INTERFACE, SingleQuotes::Never);
    check_in("Never in Interface", &t, ("'42'", "’42'"));
    let t = set_with(Language::EN_US, RuleSet::PROSE, SingleQuotes::Never);
    check_in("Never in Prose", &t, ("'42'", "’42′"));
    check_in(
        "Never in Prose",
        &t,
        ("She loves 'the dogs'", "She loves 'the dogs’"),
    );
}

#[test]
fn custom_quote_pairs() {
    let pairs = QuotePairs::new(('«', '»'), ('‹', '›'));
    let t = Typographer::builder(Language::EN_US, RuleSet::EMPTY.with(Rule::QuotesPairs))
        .quote_pairs(pairs)
        .build()
        .unwrap();
    check_in("custom pairs", &t, ("\"a \"b\" c\"", "«a ‹b› c»"));
    // Typed glyphs of the custom pairs have a role; the language's own are neutral.
    check_in("custom pairs", &t, ("«a \"b\" c»", "«a ‹b› c»"));
    check_unchanged("custom pairs", &t, "“a” b");
}

#[test]
fn the_apostrophe_follows_the_language_data() {
    // Design section 6.2 and the table of design section 8.1: elisions, the decade and the trailing
    // possessive are English data; `don't` between letters is an apostrophe in every language.
    let apostrophe = |language| only(language, &[Rule::QuotesApostrophe]);
    let ru = apostrophe(Language::RU_RU);
    let common = apostrophe(Language::COMMON);
    let en = apostrophe(Language::EN_US);
    for (t, name) in [(&ru, "ru"), (&common, "und"), (&en, "en")] {
        check_in(name, t, ("don't", "don’t"));
        check_in(name, t, ("Д'Артаньян", "Д’Артаньян"));
        check_in(name, t, ("rock'n'roll", "rock’n’roll"));
        check_in(name, t, ("don’t", "don’t"));
    }
    for (t, name) in [(&ru, "ru"), (&common, "und")] {
        for text in ["'tis the season", "the '90s", "the dogs' bowls", "'em"] {
            check_unchanged(name, t, text);
        }
    }
    check_in("en", &en, ("the 90's", "the 90’s"));
    check_in("en", &en, ("'90s", "’90s"));
    // The decade of step 1 needs a non-letter, non-digit after `s` (design section 6.2).
    check_in("en", &en, ("the '90s-era", "the ’90s-era"));
    check_unchanged("en", &en, "the '90s2");
    check_in(
        "en",
        &en,
        (
            "'tis 'twas 'em 'cause 'bout 'round 'til 'nother",
            "’tis ’twas ’em ’cause ’bout ’round ’til ’nother",
        ),
    );
    // Not in the elision list: `'n'` quotes a letter.
    check_unchanged("en", &en, "Press 'y' or 'n' now");
    // The elision runs on across a `Format` token (design section 3.6).
    for t in [&en, &prose(Language::EN_US)] {
        check_in(
            "en",
            t,
            ("He said 'ca\u{AD}use it", "He said ’ca\u{AD}use it"),
        );
    }
    check_unchanged("en", &en, "He said 'causeway it");
    // A decade without `s` waits for pairing (design section 6.2) and is an apostrophe only
    // unpaired.
    check_unchanged("en", &en, "Press '42' to confirm");
    // A digit before a letter: the possessive of a number.
    check_in("en", &en, ("the 1990's", "the 1990’s"));
}

#[test]
fn primes_follow_a_number() {
    // Design section 6.2: one to three digits on the left, one or two on the right ending the core.
    let primes = |language| only(language, &[Rule::QuotesPrimes]);
    for language in [Language::RU_RU, Language::EN_US, Language::COMMON] {
        let t = primes(language.clone());
        check_in("primes", &t, ("5'10", "5′10"));
        check_in("primes", &t, ("рост 5'10 см", "рост 5′10 см"));
        check_in("primes", &t, ("5'10\"", "5′10″"));
        check_unchanged("primes", &t, "1'000'000");
        check_unchanged("primes", &t, "5’10");
        // A prime is not a candidate: it never changes again.
        check_unchanged("primes", &t, "5′10″");
        check_unchanged("primes", &t, "Say \"hi\"");
    }
    // A weak quote that pairs is not a prime.
    let t = primes(Language::EN_US);
    check_unchanged("primes", &t, "\"Enter 5\"");
    check_in("primes", &t, ("Enter 5\" now", "Enter 5″ now"));
    check_in("primes", &t, ("Enter 5' now", "Enter 5′ now"));
}

#[test]
fn a_number_split_by_a_format_token_is_one_number() {
    // Design section 3.6: a decade and the digits around a prime read through a soft hyphen.
    let t = prose(Language::EN_US);
    check_exact("en Prose", &t, ("in the '9\u{AD}0s", "in⍽the ’9\u{AD}0s"));
    check_exact(
        "en Prose",
        &t,
        ("He is 5'1\u{AD}0 tall", "He is 5′1\u{AD}0 tall"),
    );
    // The number on the left is read as a run too: two digits split make a prime, four do not.
    check_exact(
        "en Prose",
        &t,
        ("He is 1\u{AD}2'5 tall", "He is 1\u{AD}2′5 tall"),
    );
    check_unchanged("en Prose", &t, "He is 1\u{AD}000'1 tall");
}

#[test]
fn common_has_no_pairs() {
    // `Language::COMMON` has no quote pairs: `quotes.pairs` is inert (design section 7.4).
    let t = Typographer::new(Language::COMMON, RuleSet::ALL);
    let off = common::without_nbsp(Language::COMMON, RuleSet::ALL);
    for t in [&t, &off] {
        let got = t.typeset("He said \"hi\" at 5'10\"", Output::Unicode);
        assert_eq!(got, "He said \"hi\" at 5′10″");
    }
}

#[test]
fn a_dash_with_a_space_on_its_other_side_is_of_class_dashc() {
    // Design section 6.3: a dash next to a quote is `DashC`. As R it lets the quote open; as L it
    // keeps the quote from closing, so the pair stays straight where a word or `%` would pair it.
    // The roles are the same for a hyphen and a dash (design section 4.2, item 5).
    use DiagnosticKind::{AmbiguousQuote, UnmatchedQuote};
    for language in [Language::RU_RU, Language::EN_US] {
        let (open, close) = if language == Language::RU_RU {
            ("«", "»")
        } else {
            ("“", "”")
        };
        for (what, t) in all_three(&language) {
            for text in [
                "Type \"— nothing\" here",
                "Type \"nothing %\" here",
                "Введите \"— ничего\" здесь",
            ] {
                let expected = text.replacen('"', open, 1).replacen('"', close, 1);
                check_exact(&what, &t, (text, &expected));
            }
            for text in [
                "Type \"nothing —\" here",
                "Type \"nothing -\" here",
                "Введите \"ничего —\" здесь",
            ] {
                check_unchanged(&what, &t, text);
                let kinds: Vec<_> = diagnostics_of(&t, text)
                    .into_iter()
                    .map(|d| d.0)
                    .filter(|k| matches!(k, AmbiguousQuote | UnmatchedQuote))
                    .collect();
                assert_eq!(
                    kinds,
                    [UnmatchedQuote, AmbiguousQuote],
                    "{what}: {}",
                    show(text)
                );
            }
        }
    }
}

#[test]
fn a_straight_quote_next_to_digits_in_a_frozen_fragment_stays() {
    let t = prose(Language::EN_US);
    for text in ["Run 1'000'000 now", "Run 0b1'0000 now"] {
        check_unchanged("frozen", &t, text);
    }
}

#[test]
fn slots_next_to_quotes() {
    // Design section 6.9: a slot is of class `Word`; the glyphs go into the neighboring texts.
    for (what, t) in all_three(&Language::RU_RU) {
        let out = t.typeset_template(
            [
                Piece::Text("Файл \""),
                Piece::Slot("name", Slot::WORD),
                Piece::Text("\" не найден"),
            ],
            Output::Unicode,
        );
        // In the sets `не` is a short word (`nbsp.short-words`).
        let glued = t.rules().contains(Rule::NbspShortWords);
        let expected = if glued {
            ["Файл «", "» не\u{a0}найден"]
        } else {
            ["Файл «", "» не найден"]
        };
        assert_eq!(out.texts(), expected, "{what}");
        let out = t.typeset_template(
            [
                Piece::Text("\""),
                Piece::Slot("a", Slot::WORD),
                Piece::Text("\" и \""),
                Piece::Slot("b", Slot::WORD),
                Piece::Text("\""),
            ],
            Output::Unicode,
        );
        // `и` is a short word before an opening quote.
        let expected = if glued {
            ["«", "» и\u{a0}«", "»"]
        } else {
            ["«", "» и «", "»"]
        };
        assert_eq!(out.texts(), expected, "{what}");
    }
}

#[test]
fn rule_tables_run_with_the_configurations_of_section_13_2() {
    // The helper `configs` gives the rule alone and each built-in set that holds it: Prose holds
    // every rule, Interface the rules of design section 9.2.
    for rule in QUOTES {
        let names: Vec<String> = configs(&Language::EN_US, rule)
            .into_iter()
            .map(|(name, _)| name)
            .collect();
        let mut expected = vec![
            format!("en-US {} alone", rule.id()),
            "en-US Prose".to_owned(),
        ];
        if RuleSet::INTERFACE.contains(rule) {
            expected.push("en-US Interface".to_owned());
        }
        assert_eq!(names, expected, "{rule}");
    }
}
