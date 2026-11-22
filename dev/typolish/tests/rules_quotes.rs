//! The tables of `quotes.apostrophe`, `quotes.pairs` and `quotes.primes` (design sections 6, 8.3
//! and 13.2).
//!
//! A row that names no configuration runs with the three quote rules (section 13.2), and the rows
//! marked as set-level also run inside Prose and Interface without the `nbsp.*` rules (stage 2 of
//! section 15.4; stage 3 switches those tables to the full sets). As in section 6.12 the outputs
//! show only the edits of the rules that run. Rows of 6.12 that sit next to another stage 2 rule
//! (an ellipsis, a spaced hyphen) run with the three quote rules only.

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

use common::{assert_changed_by, check_in, check_unchanged, configs, interface, only, prose, show};
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

/// A built-in set without the `nbsp.*` rules and with a `SingleQuotes` mode.
fn set_with(language: Language, set: RuleSet, mode: SingleQuotes) -> Typographer {
    let mut builder = Typographer::builder(language, set).single_quotes(mode);
    for rule in Rule::ALL.iter().filter(|r| r.id().starts_with("nbsp.")) {
        builder = builder.disable(*rule);
    }
    builder.build().unwrap()
}

/// The three typographers of a language that a set-level row runs with: the quote rules alone,
/// Prose and Interface (both without `nbsp.*`).
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

// Section 6.12, Russian; each row holds with the quote rules alone and in both sets.
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
    // Section 6.7: typed pairs count in the levels.
    ("«Он сказал \"да\"»", "«Он сказал „да“»"),
    ("\"a \"b \"c\" d\" e\"", "«a „b «c» d“ e»"),
    // Section 6.5: a weak quote pairs with an opener that pass 1 left unpaired.
    ("Он сказал \"диагональ 27\".", "Он сказал «диагональ 27»."),
    // Section 13.2: an empty isolate pair is a sign-only pair; neutral glyphs never change.
    ("Имя: \"\u{2068}\u{2069}\"", "Имя: «\u{2068}\u{2069}»"),
    ("Он сказал ”привет” и ушёл.", "Он сказал ”привет” и ушёл."),
    // Section 6.10: the scope is the window.
    ("\"a\" b \"c\n\n\"d\"", "\"a\" b \"c\n\n«d»"),
    ("\"a\n\n\"b\" c", "\"a\n\n«b» c"),
    // Section 6.6: an unpaired straight single quote stays.
    ("Он сказал 'привет", "Он сказал 'привет"),
];

#[test]
fn russian_rows_of_section_6_12() {
    rows(&Language::RU_RU, RU_ROWS);
}

// Section 6.12, English, `NestedOnly`; the rows hold with the quote rules alone and in both sets.
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
    // Section 13.2.
    ("the 90's", "the 90’s"),
    ("1'000'000", "1'000'000"),
    // Section 6.2, step 1.
    ("O'Brien", "O’Brien"),
    ("Д'Артаньян", "Д’Артаньян"),
    ("rock'n'roll", "rock’n’roll"),
    ("'tis the season", "’tis the season"),
    ("Give 'em a hand", "Give ’em a hand"),
    ("Do it 'cause I said", "Do it ’cause I said"),
    ("It's already typeset: don’t", "It’s already typeset: don’t"),
    // Section 6.6: the trailing possessive and the decade.
    ("the Joneses' house", "the Joneses’ house"),
    ("the dogs' bowls", "the dogs’ bowls"),
    ("He said 'hi", "He said 'hi"),
    // Section 6.4: a closer covered only by straight single quotes unpairs them.
    ("\"a 'b\" c'", "“a 'b” c'"),
    // Section 6.10.
    ("\"a\" b \"c\n\n\"d\"", "\"a\" b \"c\n\n“d”"),
    // Section 6.6: the edits of step 1 stay in a window that does not converge.
    ("Don't \"x", "Don’t \"x"),
    ("\"x\" and \"y don't", "\"x\" and \"y don’t"),
    // Section 6.1: a typed glyph of another language is neutral.
    ("He said «hi» there", "He said «hi» there"),
];

#[test]
fn english_rows_of_section_6_12() {
    rows(&Language::EN_US, EN_ROWS);
}

// The rows of 6.12 that need `quotes.primes`: Prose (section 6.5), and what Interface does.
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
    // own edits only (section 6).
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
    // `quotes.apostrophe` disabled: `don't` still does not take part in pairing (section 6).
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
    // The example of section 2.9: the unmatched quote is the one at byte 18.
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
    // The covering entry of `«a "b» c"` is the straight quote (section 6.4).
    let d = diagnostics_of(&t, "«a \"b» c\"");
    assert_eq!(d[0].0, DiagnosticKind::CrossingQuotes);
    assert_eq!(d[0].4, 4..5);
    // A quote without a role is the diagnostic's range.
    let d = diagnostics_of(&t, "Слово \" в тексте.");
    assert_eq!(&"Слово \" в тексте."[d[0].4.clone()], "\"");
}

#[test]
fn protected_spans_of_quote_rows_are_empty() {
    // Section 13.2: the quotes in these rows belong to no frozen fragment.
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
    // Section 6.8. `NestedOnly` is the default.
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
    // Section 6.2 and the table of section 8.1: elisions, the decade and the trailing possessive
    // are English data; `don't` between letters is an apostrophe in every language.
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
    // A decade without `s` waits for pairing (section 6.2) and is an apostrophe only unpaired.
    check_unchanged("en", &en, "Press '42' to confirm");
    // A digit before a letter: the possessive of a number.
    check_in("en", &en, ("the 1990's", "the 1990’s"));
}

#[test]
fn primes_follow_a_number() {
    // Section 6.2: one to three digits on the left, one or two on the right ending the core.
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
fn common_has_no_pairs() {
    // `Language::COMMON` has no quote pairs: `quotes.pairs` is inert (section 7.4).
    let t = Typographer::new(Language::COMMON, RuleSet::ALL);
    let off = common::without_nbsp(Language::COMMON, RuleSet::ALL);
    for t in [&t, &off] {
        let got = t.typeset("He said \"hi\" at 5'10\"", Output::Unicode);
        assert!(got.contains("\"hi\""), "{got}");
        assert!(got.contains("5′10″"), "{got}");
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
    // Section 6.9: a slot is of class `Word`; the glyphs go into the neighboring texts.
    for (what, t) in all_three(&Language::RU_RU) {
        let out = t.typeset_template(
            [
                Piece::Text("Файл \""),
                Piece::Slot("name", Slot::WORD),
                Piece::Text("\" не найден"),
            ],
            Output::Unicode,
        );
        assert_eq!(out.texts(), ["Файл «", "» не найден"], "{what}");
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
        assert_eq!(out.texts(), ["«", "» и «", "»"], "{what}");
    }
}

#[test]
fn rule_tables_run_with_the_configurations_of_section_13_2() {
    // The helper `configs` gives the rule alone and each built-in set that holds it.
    for rule in QUOTES {
        let names: Vec<_> = configs(&Language::EN_US, rule)
            .into_iter()
            .map(|(name, _)| name)
            .collect();
        let expected = if rule == Rule::QuotesPrimes { 2 } else { 3 };
        assert_eq!(names.len(), expected, "{rule}: {names:?}");
    }
}
