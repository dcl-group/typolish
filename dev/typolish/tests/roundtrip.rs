//! The reverse transformation and the round trip (design sections 2.10, 12.1, 12.2, 13.5 property
//! 13 and 13.7): the tables of `to_plain` and `to_plain_template`, the table test of
//! `tests/corpus/roundtrip-known.tsv`, P1 and P2 on generated strings and templates and on the
//! corpus. Everything goes through the public API.
//!
//! Rows of the tables are written with the markers of `vis`: `⍽` is U+00A0 and `⎵` is U+202F. The
//! first column of a `to_plain` row is the typeset text, the second the plain form.

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

use common::canonical::{Setup, canonical_template, canonical_text, plain_with_setup, setup};
use common::corpus::{
    FILES, LANGS, SETS, known_rows, messages, read, settings_typographer, split_template,
    template_pieces, text_lines, texts_and_slots, typographer,
};
use common::{configs, interface, parse_tmpl, prose, show, vis, without_rule};
use proptest::prelude::*;
use typolish::{
    DashStyle, Language, LanguageBuilder, LanguageData, NumberData, Output, PercentSpacing, Piece,
    QuotePairs, Rule, RuleSet, SingleQuotes, Slot, SlotKind, Template, Typographer,
};

fn ru() -> Language {
    Language::RU_RU
}

fn en() -> Language {
    Language::EN_US
}

// ---------------------------------------------------------------------------------------------
// The tables of `to_plain` (design section 12.1)
// ---------------------------------------------------------------------------------------------

/// Checks one row: `to_plain` of the typeset text gives the plain form, the result is borrowed
/// exactly when it equals the input, and the plain form is a fixed point.
#[track_caller]
fn check_plain(what: &str, t: &Typographer, row: (&str, &str)) {
    let typeset = vis(row.0);
    let expected = vis(row.1);
    let got = t.to_plain(&typeset);
    assert!(
        got == expected,
        "{what}\ntypeset  {}\nexpected {}\ngot      {}",
        show(&typeset),
        show(&expected),
        show(&got)
    );
    assert_eq!(
        matches!(got, Cow::Borrowed(_)),
        typeset == expected,
        "{what}: Cow::Borrowed exactly when the output equals the input: {}",
        show(&typeset)
    );
    let again = t.to_plain(&expected);
    assert!(
        again == expected,
        "{what}: to_plain(to_plain(x)) == to_plain(x)\nplain {}\nagain {}",
        show(&expected),
        show(&again)
    );
}

#[track_caller]
fn run(typographers: &[(String, Typographer)], rows: &[(&str, &str)]) {
    for (what, t) in typographers {
        for row in rows {
            check_plain(what, t, *row);
        }
    }
}

/// The two built-in sets of a language, with names.
fn sets_of(language: &Language) -> Vec<(String, Typographer)> {
    vec![
        (format!("{language} Prose"), prose(language.clone())),
        (format!("{language} Interface"), interface(language.clone())),
    ]
}

/// A named typographer with exactly these rules.
fn with_rules(language: &Language, rules: &[Rule]) -> (String, Typographer) {
    let set: RuleSet = rules.iter().copied().collect();
    (
        format!(
            "{language} {}",
            rules.iter().map(|r| r.id()).collect::<Vec<_>>().join("+")
        ),
        Typographer::new(language.clone(), set),
    )
}

#[test]
fn the_example_of_section_2_10() {
    static RU: Typographer = Typographer::new(Language::RU_RU, RuleSet::INTERFACE);
    assert_eq!(
        RU.to_plain("Файл «report.pdf» не\u{a0}найден\u{a0}— проверьте"),
        "Файл \"report.pdf\" не найден - проверьте",
    );
}

#[test]
fn quotes_pairs_ru() {
    run(
        &configs(&ru(), Rule::QuotesPairs),
        &[
            ("«Привет»", "\"Привет\""),
            (
                "Он сказал: «Читай „это“ внимательно».",
                "Он сказал: \"Читай \"это\" внимательно\".",
            ),
            ("„Привет“", "\"Привет\""),
            ("«?» и «»", "\"?\" и \"\""),
            // `”` is no glyph of the Russian pairs: neutral, untouched (section 6.1).
            ("Он ”привет” сказал", "Он ”привет” сказал"),
            ("Без кавычек", "Без кавычек"),
            ("", ""),
        ],
    );
}

#[test]
fn quotes_pairs_en() {
    run(
        &configs(&en(), Rule::QuotesPairs),
        &[
            ("“Hi”", "\"Hi\""),
            ("He said “yes”.", "He said \"yes\"."),
            ("“a ‘b’ c”", "\"a \"b\" c\""),
            ("He said ‘yes’.", "He said \"yes\"."),
            ("“?” and “”", "\"?\" and \"\""),
            // `«` and `»` are no glyphs of the English pairs: neutral, untouched.
            ("an «x» here", "an «x» here"),
        ],
    );
}

#[test]
fn quotes_apostrophe() {
    let rows_en = [
        ("don’t", "don't"),
        ("rock’n’roll", "rock'n'roll"),
        ("the dogs’ bowl", "the dogs' bowl"),
        ("James’s coat", "James's coat"),
        ("’tis the season", "'tis the season"),
        ("the ’90s", "the '90s"),
        ("the 90’s", "the 90's"),
        ("in ’26", "in '26"),
        ("O’Brien", "O'Brien"),
    ];
    run(&configs(&en(), Rule::QuotesApostrophe), &rows_en);
    run(
        &configs(&ru(), Rule::QuotesApostrophe),
        &[("Д’Артаньян", "Д'Артаньян"), ("rock’n’roll", "rock'n'roll")],
    );
}

#[test]
fn quotes_pairs_and_apostrophe_together() {
    let mut typographers = sets_of(&en());
    typographers.push(with_rules(
        &en(),
        &[Rule::QuotesPairs, Rule::QuotesApostrophe],
    ));
    run(
        &typographers,
        &[
            ("“don’t”", "\"don't\""),
            ("“a ‘don’t’ b”", "\"a \"don't\" b\""),
            // The examples of section 12.1.
            (
                "“He said ‘the dogs’ bowl is empty’”",
                "\"He said \"the dogs' bowl is empty\"\"",
            ),
            (
                "“‘Hi,’ said the dogs’ owner”",
                "\"\"Hi,\" said the dogs' owner\"",
            ),
            // Every candidate follows `s`: the first one closes (`NestedPossessive`).
            ("“a ‘the dogs’ bowls’ b”", "\"a \"the dogs\" bowls' b\""),
            ("the dogs’ bowl and James’s", "the dogs' bowl and James's"),
            // The forward rule puts a trailing possessive only after a lowercase `s` (design
            // section 6.5).
            (
                "“He said ‘THE DOGS’ bowl is empty’”",
                "\"He said \"THE DOGS\" bowl is empty'\"",
            ),
            ("“‘DOGS’ bowl’”", "\"\"DOGS\" bowl'\""),
        ],
    );
    // Embeddings and overrides are format characters too (design section 5.4).
    for f in [
        '\u{200e}', '\u{202a}', '\u{202b}', '\u{202c}', '\u{202d}', '\u{202e}',
    ] {
        let typeset = format!("“He said ‘the dogs{f}’ bowl is empty’”");
        let plain = format!("\"He said \"the dogs{f}' bowl is empty\"\"");
        run(&typographers, &[(&typeset, &plain)]);
    }
    let mut ru_typographers = sets_of(&ru());
    ru_typographers.push(with_rules(
        &ru(),
        &[Rule::QuotesPairs, Rule::QuotesApostrophe],
    ));
    run(
        &ru_typographers,
        &[("«Д’Артаньян сказал „нет“»", "\"Д'Артаньян сказал \"нет\"\"")],
    );
}

/// The two built-in sets and `RuleSet::ALL` of a language, with names.
fn sets_and_all(language: &Language) -> Vec<(String, Typographer)> {
    let mut typographers = sets_of(language);
    typographers.push((
        format!("{language} All"),
        Typographer::new(language.clone(), RuleSet::ALL),
    ));
    typographers
}

#[test]
fn nested_pairs_of_signs_and_possessives_round_trip() {
    // Section 12.1: a `’` after a `‘` with nothing but signs between them closes it, the mirror of
    // the sign-only pair of section 6.3; a possessive `’` inside a deeper pair does not end the
    // search for the closer of its own pair.
    let canonical_en = [
        "\"a \"\" b\"",
        "\"a \"-\" b\"",
        "\"a \"\", b\"",
        "\"a \"?\" b\"",
        "\"a \"...\" b\"",
        "\"a \"the dogs' bowl\" b\"",
        "\"\"a dogs' b \"c dogs' d\" e\" f\"",
        "\"\"house the dogs' bowl \"the dogs' bowl\" house\" two words\"",
        // A neighbor inside a frozen core is of class `Word` (section 6.3).
        "\"x \"y {\"\"",
        "\"x \"y 2026{\"\"",
        "\"x \"y a/\"\"",
        "\"\"\"house the dogs' bowl in 2026{\"\"\"",
    ];
    let canonical_ru = [
        "\"a \"\" b\"",
        "\"a \"-\" b\"",
        "\"a \"\", b\"",
        "\"a \"?\" b\"",
        "\"x \"y {\"\"",
    ];
    // Section 12.1: where the double quotes of a window do not pair again, the nested pair reads
    // as `'`; these inputs are not canonical, so only P2 holds.
    let plain = ["\"x ''.\"", "\"May '+-'':\"", "\"x '-'.\""];
    let mut failures = Vec::new();
    for (language, canonical) in [(en(), &canonical_en[..]), (ru(), &canonical_ru[..])] {
        for (what, t) in sets_and_all(&language) {
            for x in canonical {
                if let Err(e) = p1(&t, x) {
                    failures.push(format!("{what}: {e}"));
                }
            }
            for x in canonical.iter().chain(&plain) {
                if let Err(e) = p2(&t, x) {
                    failures.push(format!("{what}: {e}"));
                }
            }
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
    for (what, t) in sets_of(&en()) {
        check_plain(&what, &t, ("“x ‘’.”", "\"x ''.\""));
        check_plain(&what, &t, ("“a⍽‘’ b”", "\"a \"\" b\""));
    }
    for (what, t) in sets_of(&ru()) {
        check_plain(&what, &t, ("«x „“.»", "\"x ''.\""));
    }
}

#[test]
fn each_of_the_two_quote_rules_undoes_only_its_own_character() {
    // `quotes.pairs` alone: the apostrophe stays.
    let (what, pairs) = with_rules(&en(), &[Rule::QuotesPairs]);
    check_plain(&what, &pairs, ("“don’t”", "\"don’t\""));
    // `quotes.apostrophe` alone: the pair glyphs stay.
    let (what, apostrophe) = with_rules(&en(), &[Rule::QuotesApostrophe]);
    check_plain(&what, &apostrophe, ("“don’t”", "“don't”"));
    // A set without `quotes.pairs` leaves the glyphs; without `quotes.apostrophe` the apostrophe.
    let no_pairs = without_rule(en(), RuleSet::PROSE, Rule::QuotesPairs);
    check_plain(
        "Prose without quotes.pairs",
        &no_pairs,
        ("“don’t”", "“don't”"),
    );
    let no_apostrophe = without_rule(en(), RuleSet::PROSE, Rule::QuotesApostrophe);
    check_plain(
        "Prose without quotes.apostrophe",
        &no_apostrophe,
        ("“don’t”", "\"don’t\""),
    );
}

#[test]
fn quotes_primes() {
    for language in [ru(), en()] {
        run(
            &configs(&language, Rule::QuotesPrimes),
            &[
                ("5′10″", "5'10\""),
                ("30″", "30\""),
                ("He is 5′10″ tall.", "He is 5'10\" tall."),
                ("Угол 30′ 15″", "Угол 30' 15\""),
            ],
        );
    }
}

#[test]
fn dash_without_glue() {
    let ru_rows: &[(&str, &str)] = &[
        ("Москва — столица", "Москва - столица"),
        ("Привет, — сказал он.", "Привет, - сказал он."),
        ("кто-то — что-то", "кто-то - что-то"),
    ];
    let en_rows: &[(&str, &str)] = &[
        // The closed form of the English style becomes a spaced hyphen.
        ("Moscow—the capital", "Moscow - the capital"),
        ("Open host:8080 — now", "Open host:8080 - now"),
        ("Hello, — he said", "Hello, - he said"),
        (
            "mother-in-law — father-in-law",
            "mother-in-law - father-in-law",
        ),
    ];
    for rule in [Rule::DashSpacedHyphen, Rule::DashDoubleHyphen] {
        for (language, rows) in [(ru(), ru_rows), (en(), en_rows)] {
            let mut typographers = configs(&language, rule);
            if rule == Rule::DashDoubleHyphen {
                // `dash.double-hyphen` alone turns only `--` into a dash, so the dash goes back
                // to `--`: with `-` the round trip of section 12.2 would not hold for this rule
                // set on its own (section 12.1).
                let (what, alone) = typographers.remove(0);
                for &(typeset, plain) in rows {
                    check_plain(&what, &alone, (typeset, &plain.replace(" - ", " -- ")));
                }
            }
            run(&typographers, rows);
        }
    }
}

#[test]
fn dash_with_glue_in_the_built_in_sets() {
    run(
        &sets_of(&ru()),
        &[
            ("Москва⍽— столица", "Москва - столица"),
            ("Привет,⍽— сказал он.", "Привет, - сказал он."),
            ("Слово⍽— «слово»", "Слово - \"слово\""),
        ],
    );
    run(
        &sets_of(&en()),
        &[
            ("He said — “a cat”", "He said - \"a cat\""),
            (
                "See https://example.com/docs — the guide",
                "See https://example.com/docs - the guide",
            ),
        ],
    );
}

#[test]
fn dash_style_of_the_builder() {
    let closed = Typographer::builder(ru(), RuleSet::PROSE)
        .dash_style(DashStyle::Closed)
        .build()
        .unwrap();
    check_plain("ru Closed", &closed, ("слово—слово", "слово - слово"));
    let spaced = Typographer::builder(en(), RuleSet::PROSE)
        .dash_style(DashStyle::Spaced)
        .build()
        .unwrap();
    check_plain(
        "en Spaced",
        &spaced,
        ("Moscow — the capital", "Moscow - the capital"),
    );
    check_plain(
        "en Spaced",
        &spaced,
        ("Moscow—the capital", "Moscow - the capital"),
    );
}

#[test]
fn a_closed_dash_next_to_a_non_letter_stays() {
    // Design sections 8.4 and 12.1: a closed `—` next to a number, a sign or a protected span is
    // no product of the dash rules, so it has no reading with spaces and stays.
    run(
        &sets_of(&en()),
        &[
            ("word—5", "word—5"),
            ("in 2020—the year", "in 2020—the year"),
            ("Run `ls`—now", "Run `ls`—now"),
        ],
    );
    run(
        &sets_of(&ru()),
        &[("слово—5", "слово—5"), ("в 2020—году", "в 2020—году")],
    );
    // A letter with a combining mark is a letter of a word.
    check_plain(
        "en Prose",
        &prose(en()),
        ("cafe\u{301}—word", "cafe\u{301} - word"),
    );
    // Before a slot: no reading with spaces either.
    let t = prose(en());
    let back = t.to_plain_template([Piece::Text("total—"), Piece::Slot("n", Slot::NUMBER)]);
    assert_eq!(back.texts(), ["total—", ""]);
    assert!(!back.changed());
    let back = t.to_plain_template([Piece::Text("word—"), Piece::Slot("w", Slot::WORD)]);
    assert_eq!(back.texts(), ["word—", ""]);
    assert!(!back.changed());
}

#[test]
fn dash_dialogue() {
    run(
        &configs(&ru(), Rule::DashDialogue),
        &[
            ("—⍽Привет!", "- Привет!"),
            ("—⍽Привет!\n—⍽Пока.", "- Привет!\n- Пока."),
        ],
    );
    // In Prose the quotes of the line go back too.
    check_plain(
        "ru Prose",
        &prose(ru()),
        ("—⍽«Привет!»\n—⍽Пока.", "- \"Привет!\"\n- Пока."),
    );
    // Interface has no `dash.dialogue`: a dash at a line start has no left operand and stays.
    check_plain(
        "ru Interface",
        &interface(ru()),
        ("— Привет!\n— Пока.", "— Привет!\n— Пока."),
    );
}

#[test]
fn ranges() {
    run(
        &configs(&ru(), Rule::RangeDigits),
        &[
            ("5–10", "5-10"),
            ("Страницы 10–20.", "Страницы 10-20."),
            ("1941–1945 гг.", "1941-1945 гг."),
        ],
    );
    run(
        &configs(&ru(), Rule::RangeContext),
        &[
            ("XIX–XX вв.", "XIX-XX вв."),
            ("1,5–2,5 кг", "1,5-2,5 кг"),
            ("стр. 9–12", "стр. 9-12"),
            ("в 2010–11 гг.", "в 2010-11 гг."),
        ],
    );
    run(
        &configs(&ru(), Rule::RangeTime),
        &[
            ("10:00–12:30", "10:00-12:30"),
            ("10:00:15–12:30:45", "10:00:15-12:30:45"),
            ("с 9:00–17:00 ежедневно", "с 9:00-17:00 ежедневно"),
        ],
    );
    run(
        &configs(&en(), Rule::RangeDigits),
        &[("1941–1945", "1941-1945"), ("pages 10–20", "pages 10-20")],
    );
    // Design sections 7.6 and 8.5: a language whose range dash is `—` gets it back as `-`.
    static EM_RANGE: LanguageData =
        LanguageData::new("xx").with_numbers(NumberData::new().with_range_dash('\u{2014}'));
    let t = Typographer::new(Language::from_data(&EM_RANGE).unwrap(), RuleSet::ALL);
    check_plain("xx em-dash range", &t, ("5—7", "5-7"));
    check_plain("xx em-dash range", &t, ("pages 10—20", "pages 10-20"));
    let back = t.to_plain_template([
        Piece::Slot("a", Slot::NUMBER),
        Piece::Text("—"),
        Piece::Slot("b", Slot::NUMBER),
    ]);
    assert_eq!(back.texts(), ["", "-", ""]);
    // Between Roman numerals the range dash `—` has letters on both sides and still reads `-`.
    let xx = LanguageBuilder::from_language(&Language::RU_RU, "xx")
        .numbers(NumberData::new().with_range_dash('\u{2014}'))
        .build()
        .unwrap();
    let t = Typographer::new(xx, RuleSet::PROSE);
    assert_eq!(t.typeset("XIX-XX века", Output::Unicode), "XIX—XX века");
    check_plain("xx Roman range", &t, ("XIX—XX века", "XIX-XX века"));
    check_plain("xx Roman range", &t, ("в XI—XX веках", "в XI-XX веках"));
    assert_eq!(t.lint("XIX—XX века").len(), 1);
    // Interface has no range rule for literal numbers.
    check_plain(
        "ru Interface",
        &interface(ru()),
        (
            "5–10 кг, 10:00–12:30, XIX–XX вв.",
            "5–10 кг, 10:00–12:30, XIX–XX вв.",
        ),
    );
}

#[test]
fn a_typed_en_dash_with_spaces_stays() {
    // Only the glue that `nbsp.dash` set before it becomes a space (design section 12.1).
    let mut typographers = sets_of(&ru());
    typographers.push(with_rules(&ru(), &[Rule::NbspDash]));
    run(
        &typographers,
        &[
            ("Москва⍽– столица", "Москва – столица"),
            ("10⍽– 20", "10 – 20"),
        ],
    );
    let mut all = typographers;
    all.extend(configs(&ru(), Rule::RangeDigits));
    run(
        &all,
        &[
            ("Москва – столица", "Москва – столица"),
            ("10 – 20", "10 – 20"),
        ],
    );
}

#[test]
fn ellipsis() {
    run(
        &configs(&ru(), Rule::EllipsisDots),
        &[
            ("Подождите…", "Подождите..."),
            ("… и тогда", "... и тогда"),
            ("Подождите…\nДальше", "Подождите...\nДальше"),
            // Plain forms stay: `?..` and `!..` are plain (section 12.1).
            ("Что?..", "Что?.."),
            ("Да!..", "Да!.."),
            ("Что?!..", "Что?!.."),
        ],
    );
    run(
        &configs(&en(), Rule::EllipsisDots),
        &[
            ("Wait…", "Wait..."),
            ("What?…", "What?..."),
            ("… and then", "... and then"),
        ],
    );
    run(
        &configs(&ru(), Rule::EllipsisInner),
        &[("…и тогда", "...и тогда"), ("да…нет", "да...нет")],
    );
}

/// A language, a rule and the rows of the rule: typeset and plain.
type NbspCase = (Language, Rule, &'static [(&'static str, &'static str)]);

#[test]
fn nbsp_rules_give_their_glue_back_as_spaces() {
    let cases: [NbspCase; 16] = [
        (
            ru(),
            Rule::NbspAbbreviations,
            &[
                ("т.⍽е.", "т. е."),
                ("и⍽т.⍽д.", "и т. д."),
                ("и⍽т.⍽п.", "и т. п."),
                ("т.⎵д.", "т. д."),
                ("см.⍽рис.⍽5", "см. рис. 5"),
                ("ул.⍽Ленина", "ул. Ленина"),
            ],
        ),
        (
            ru(),
            Rule::NbspInitials,
            &[
                ("А.⍽С.⍽Пушкин", "А. С. Пушкин"),
                ("Пушкин А.⍽С.", "Пушкин А. С."),
            ],
        ),
        (
            ru(),
            Rule::NbspDates,
            &[
                ("21⍽ноября 2026⍽г.", "21 ноября 2026 г."),
                ("XIX⍽в.", "XIX в."),
            ],
        ),
        (
            ru(),
            Rule::NbspUnits,
            &[
                ("5⍽МБ", "5 МБ"),
                ("30⍽с.", "30 с."),
                ("12⍽дн.", "12 дн."),
                ("60⍽км/ч", "60 км/ч"),
                ("100⍽%", "100 %"),
                ("100%", "100%"),
            ],
        ),
        (
            ru(),
            Rule::NbspSigns,
            &[
                ("Документ №⍽5", "Документ № 5"),
                ("§⍽12", "§ 12"),
                ("№⎵5", "№ 5"),
            ],
        ),
        (
            ru(),
            Rule::NbspPhone,
            &[("+7⍽495⍽123-45-67", "+7 495 123-45-67")],
        ),
        (
            ru(),
            Rule::NbspShortWords,
            &[("в⍽2026 году", "в 2026 году"), ("и⍽в⍽Москве", "и в Москве")],
        ),
        (
            ru(),
            Rule::NbspShortLength,
            &[("Он⍽пошёл", "Он пошёл"), ("Мы⍽и⍽они", "Мы и они")],
        ),
        (
            ru(),
            Rule::NbspParticles,
            &[("сказал⍽бы", "сказал бы"), ("Знаешь⍽ли", "Знаешь ли")],
        ),
        (en(), Rule::NbspAbbreviations, &[("Mr.⍽Smith", "Mr. Smith")]),
        (
            en(),
            Rule::NbspInitials,
            &[("J.⍽R.⍽R.⍽Tolkien", "J. R. R. Tolkien")],
        ),
        (
            en(),
            Rule::NbspDates,
            // The glue after `On` is set by `nbsp.short-words`, not by this rule; Prose undoes both.
            &[
                ("November⍽21", "November 21"),
                ("On November⍽21", "On November 21"),
            ],
        ),
        (en(), Rule::NbspUnits, &[("5⍽MB", "5 MB"), ("100%", "100%")]),
        (en(), Rule::NbspSigns, &[("№⍽5", "№ 5")]),
        (
            en(),
            Rule::NbspShortWords,
            &[("in⍽the⍽house", "in the house")],
        ),
        (
            en(),
            Rule::NbspPhone,
            &[("+1⍽212⍽555-0100", "+1 212 555-0100")],
        ),
    ];
    for (language, rule, rows) in cases {
        run(&configs(&language, rule), rows);
    }
    // `nbsp.dash` alone gives back the glue before a dash and leaves the dash.
    let (what, dash) = with_rules(&ru(), &[Rule::NbspDash]);
    check_plain(&what, &dash, ("Москва⍽— столица", "Москва — столица"));
}

#[test]
fn glue_that_no_enabled_rule_sets_stays() {
    // Only `nbsp.units` runs: the glue after the short word is not its product (section 12.1:
    // `to_plain` undoes exactly the rules that are enabled).
    let (what, units) = with_rules(&ru(), &[Rule::NbspUnits]);
    check_plain(&what, &units, ("5⍽МБ в⍽доме", "5 МБ в⍽доме"));
    let (what, short) = with_rules(&ru(), &[Rule::NbspShortWords]);
    check_plain(&what, &short, ("5⍽МБ в⍽доме", "5⍽МБ в доме"));
}

#[test]
fn number_rules() {
    run(
        &configs(&ru(), Rule::NumberMinus),
        &[
            ("Минус −5 градусов", "Минус -5 градусов"),
            ("(−5)", "(-5)"),
            ("−5%", "-5%"),
            ("−1,5", "-1,5"),
        ],
    );
    run(
        &configs(&en(), Rule::NumberMinus),
        &[("It is −5 degrees", "It is -5 degrees")],
    );
    for language in [ru(), en()] {
        run(
            &configs(&language, Rule::NumberTimes),
            &[
                ("1920×1080", "1920x1080"),
                ("10 × 5", "10 x 5"),
                ("Размер 10 × 5.", "Размер 10 x 5."),
            ],
        );
    }
}

#[test]
fn symbols_rules() {
    for language in [ru(), en()] {
        run(
            &configs(&language, Rule::SymbolsCopyright),
            &[
                ("© 2026", "(c) 2026"),
                ("©2026", "(c)2026"),
                ("2026 ©", "2026 (c)"),
                ("© 2020-2026", "(c) 2020-2026"),
                ("2020-2026 ©", "2020-2026 (c)"),
                ("Brand™", "Brand(tm)"),
                ("Brand®", "Brand(r)"),
                ("Brand ®", "Brand (r)"),
                // Not products of the rule: a lone sign, a label after a number, a short word.
                ("© Example Corp", "© Example Corp"),
                ("® знак", "® знак"),
                ("Section 1001©", "Section 1001©"),
                ("2026©", "2026©"),
                ("brand™", "brand™"),
                ("A®", "A®"),
            ],
        );
        run(
            &configs(&language, Rule::SymbolsArrows),
            &[
                ("x → y", "x -> y"),
                ("a ← b", "a <- b"),
                ("a ↔ b", "a <-> b"),
            ],
        );
        run(
            &configs(&language, Rule::SymbolsMath),
            &[
                ("x ≥ 5", "x >= 5"),
                ("a ≤ b", "a <= b"),
                ("a ≠ b", "a != b"),
                ("a ± b", "a +- b"),
                ("±5", "+-5"),
            ],
        );
    }
}

#[test]
fn rules_that_are_off_leave_their_characters() {
    // `symbols.*` are off in Interface: nothing of this string is a product of an enabled rule.
    let text = "© 2026 Brand™ x → y x ≥ 5 a ± b 1920×1080 −5 5–10 кг 5′10″";
    for language in [ru(), en()] {
        check_plain("Interface", &interface(language.clone()), (text, text));
    }
    let no_copyright = without_rule(ru(), RuleSet::PROSE, Rule::SymbolsCopyright);
    check_plain(
        "Prose without symbols.copyright",
        &no_copyright,
        ("© 2026 и x → y", "© 2026 и x -> y"),
    );
    let no_primes = without_rule(en(), RuleSet::PROSE, Rule::QuotesPrimes);
    check_plain(
        "Prose without quotes.primes",
        &no_primes,
        ("5′10″", "5′10″"),
    );
    let no_minus = without_rule(ru(), RuleSet::PROSE, Rule::NumberMinus);
    check_plain("Prose without number.minus", &no_minus, ("−5", "−5"));
}

#[test]
fn plain_forms_are_not_normalized() {
    for language in [ru(), en()] {
        let t = prose(language);
        for text in [
            "a -- b",
            "т.д.",
            "?...",
            "(C)",
            "(c)",
            "Он - \"врач\"",
            "a - b",
            "5-10",
            "...",
            "'x'",
            "1920x1080",
            "5 МБ",
            "№ 5",
            "",
            "plain text",
        ] {
            let got = t.to_plain(text);
            assert!(
                matches!(&got, Cow::Borrowed(s) if *s == text),
                "{text:?} must come back as Cow::Borrowed, got {got:?}"
            );
        }
    }
}

#[test]
fn the_settings_of_the_builder_reach_to_plain() {
    // Own quote pairs (the language data with the builder's overrides, section 12.1).
    let own = Typographer::builder(en(), RuleSet::PROSE)
        .quote_pairs(QuotePairs::new(('«', '»'), ('‹', '›')))
        .build()
        .unwrap();
    check_plain("own pairs", &own, ("«x ‹y› z»", "\"x \"y\" z\""));
    check_plain("own pairs", &own, ("“x”", "“x”"));
    // The single quote mode does not change what `to_plain` writes for the pairs.
    for mode in [
        SingleQuotes::NestedOnly,
        SingleQuotes::Everywhere,
        SingleQuotes::Never,
    ] {
        let t = Typographer::builder(en(), RuleSet::PROSE)
            .single_quotes(mode)
            .build()
            .unwrap();
        check_plain(&format!("{mode:?}"), &t, ("“x”", "\"x\""));
    }
}

#[test]
fn protected_text_is_not_touched() {
    let t = prose(ru());
    for (typeset, plain) in [
        // Backticks (the default delimiters).
        ("`«x»` и «y»", "`«x»` и \"y\""),
        ("`a — b` — «y»", "`a — b` - \"y\""),
        (
            "Команда `ls …` — «готово»…",
            "Команда `ls …` - \"готово\"...",
        ),
        // An opening delimiter without its closing one is text (section 10.1); glued to the
        // quote and the word after it, it makes the frozen fragment "`«x"
        // (`Technical(Unclassified)`), a protected span that stays (sections 10.5, 12.1).
        ("`«x» и «y»", "`«x\" и \"y\""),
    ] {
        check_plain("ru Prose", &t, (typeset, plain));
    }
    // The delimiters of the builder work as in `typeset`.
    let own = Typographer::builder(ru(), RuleSet::PROSE)
        .protect_between("{{", "}}")
        .build()
        .unwrap();
    check_plain("own delimiters", &own, ("{{«x»}} и «y»", "{{«x»}} и \"y\""));
    let no_backticks = Typographer::builder(ru(), RuleSet::PROSE)
        .protect_backticks(false)
        .build()
        .unwrap();
    check_plain(
        "no backticks",
        &no_backticks,
        // Without backtick protection, "`«x»`" is one frozen fragment (`Technical(Unclassified)`):
        // a protected span that stays (sections 10.5, 12.1).
        ("`«x»` и «y»", "`«x»` и \"y\""),
    );
}

// ---------------------------------------------------------------------------------------------
// `to_plain_template` (design section 2.10)
// ---------------------------------------------------------------------------------------------

fn texts(template: &Template<'_, &str>) -> Vec<String> {
    template.texts().iter().map(Cow::to_string).collect()
}

fn names(template: &Template<'_, &str>) -> Vec<(String, SlotKind)> {
    template
        .slots()
        .iter()
        .map(|(n, s)| ((*n).to_owned(), s.kind()))
        .collect()
}

#[test]
fn a_template_keeps_its_slots_and_gives_plain_texts() {
    let t = prose(ru());
    let back = t.to_plain_template([
        Piece::Text("«"),
        Piece::Slot("name", Slot::WORD),
        Piece::Text("» нашёл "),
        Piece::Slot("n", Slot::NUMBER),
        Piece::Text("\u{a0}МБ."),
    ]);
    assert_eq!(texts(&back), ["\"", "\" нашёл ", " МБ."]);
    assert_eq!(
        names(&back),
        [
            ("name".to_owned(), SlotKind::Word),
            ("n".to_owned(), SlotKind::Number)
        ]
    );
    assert!(back.changed());
}

#[test]
fn a_template_that_is_plain_is_unchanged() {
    let t = interface(en());
    let back = t.to_plain_template([
        Piece::Text("Hello, "),
        Piece::Slot(7u32, Slot::WORD),
        Piece::Text("! You have "),
        Piece::Slot(8u32, Slot::NUMBER),
        Piece::Text(" files."),
    ]);
    assert_eq!(back.texts(), ["Hello, ", "! You have ", " files."]);
    assert!(!back.changed());
    assert_eq!(back.slots().len(), 2);
    assert_eq!(back.slots()[0].0, 7);
    assert_eq!(back.slots()[1].0, 8);
}

#[test]
fn the_kinds_and_payloads_of_slots_are_kept_in_order() {
    let t = prose(en());
    let back = t.to_plain_template([
        Piece::Slot(0u8, Slot::NUMBER),
        Piece::Text("“"),
        Piece::Slot(1u8, Slot::WORD),
        Piece::Text("”"),
        Piece::Slot(2u8, Slot::DATE),
        Piece::Slot(3u8, Slot::OTHER),
    ]);
    let kinds: Vec<(u8, SlotKind)> = back.slots().iter().map(|(p, s)| (*p, s.kind())).collect();
    assert_eq!(
        kinds,
        [
            (0, SlotKind::Number),
            (1, SlotKind::Word),
            (2, SlotKind::Date),
            (3, SlotKind::Other)
        ]
    );
    assert_eq!(back.texts().len(), back.slots().len() + 1);
    // A quote glued between two slots is a frozen fragment (`Technical(Unclassified)`), a
    // protected span of section 12.5 that `to_plain` does not touch (section 12.1).
    assert_eq!(back.texts(), ["", "“", "”", "", ""]);
    assert!(!back.changed());
}

#[test]
fn quotes_pair_across_the_pieces_of_a_template() {
    let t = prose(ru());
    let back = t.to_plain_template([
        Piece::Text("Он сказал «"),
        Piece::Slot("a", Slot::WORD),
        Piece::Text("» и „"),
        Piece::Slot("b", Slot::OTHER),
        Piece::Text("“."),
    ]);
    assert_eq!(texts(&back), ["Он сказал \"", "\" и \"", "\"."]);
}

#[test]
fn markup_and_protected_pieces_are_not_touched() {
    let t = prose(ru());
    let back = t.to_plain_template::<&str>([
        Piece::Text("«a» "),
        Piece::Markup("<b>«</b>"),
        Piece::Text(" «b» "),
        Piece::Protected("«c» — d…"),
        Piece::Text(" «e»"),
    ]);
    assert_eq!(texts(&back), ["\"a\" <b>«</b> \"b\" «c» — d… \"e\""]);
}

#[test]
fn an_empty_template_stays_empty() {
    let t = prose(en());
    let back = t.to_plain_template::<u8>([]);
    assert_eq!(back.texts(), [""]);
    assert!(back.slots().is_empty());
    assert!(!back.changed());
}

#[test]
fn a_typeset_template_comes_back_plain() {
    // The design examples of section 1: `{n} дн.`, `{a}-{b}` and `"{name}"`.
    let t = prose(ru());
    let pieces = || {
        vec![
            Piece::Text("Осталось "),
            Piece::Slot("n", Slot::NUMBER),
            Piece::Text(" дн., "),
            Piece::Slot("a", Slot::NUMBER),
            Piece::Text("-"),
            Piece::Slot("b", Slot::NUMBER),
            Piece::Text(" и \""),
            Piece::Slot("name", Slot::WORD),
            Piece::Text("\"."),
        ]
    };
    let typeset = t.typeset_template(pieces(), Output::Unicode);
    assert!(
        typeset.changed(),
        "the template is not typeset: {:?}",
        texts(&typeset)
    );
    assert_eq!(typeset.texts()[2], "–", "the range dash of {{a}}-{{b}}");
    let back = t.to_plain_template(template_pieces(&typeset));
    let original =
        parse_tmpl("m = Осталось {n:number} дн., {a:number}-{b:number} и \"{name:word}\".");
    let (want_texts, want_slots) = texts_and_slots(&original[0]);
    assert_eq!(texts(&back), want_texts);
    assert_eq!(names(&back), want_slots);
}

// ---------------------------------------------------------------------------------------------
// Protected spans (design sections 12.1 and 12.5)
// ---------------------------------------------------------------------------------------------

/// The text of every non-empty protected span of `y`, in order of position.
fn protected_texts(t: &Typographer, y: &str) -> Vec<String> {
    let report = t.report(y, Output::Unicode);
    let mut spans: Vec<std::ops::Range<usize>> = report
        .protected()
        .iter()
        .filter(|s| s.piece() == 0 && !s.range().is_empty())
        .map(|s| s.range())
        .collect();
    spans.sort_by_key(|r| (r.start, r.end));
    spans.into_iter().map(|r| y[r].to_owned()).collect()
}

/// `to_plain` leaves the spans of `protected()` byte for byte in place: each of them is found in
/// the output, in order.
fn check_protected(t: &Typographer, y: &str) -> Result<(), String> {
    let plain = t.to_plain(y);
    let mut cursor = 0;
    for span in protected_texts(t, y) {
        match plain[cursor..].find(&span) {
            Some(at) => cursor += at + span.len(),
            None => {
                return Err(format!(
                    "a protected span was changed by to_plain\n  y        {}\n  to_plain {}\n  span     {}",
                    show(y),
                    show(&plain),
                    show(&span)
                ));
            }
        }
    }
    Ok(())
}

#[test]
fn protected_spans_of_the_corpus_stay() {
    let mut failures = Vec::new();
    for lang in LANGS {
        for (set_name, _) in SETS {
            let t = typographer(lang, set_name);
            for file in ["prose.txt", "tech.txt"] {
                for line in text_lines(lang, file) {
                    let y = t.typeset(&line, Output::Unicode);
                    if let Err(error) = check_protected(&t, &y) {
                        failures.push(format!("{lang}/{file} {set_name}: {error}"));
                    }
                }
            }
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(400))]

    /// The protected spans of a typeset string stay in `to_plain` (section 12.1).
    #[test]
    fn protected_spans_stay_in_generated_text(
        x in common::strategies::any_text(),
        lang in 0usize..2,
        set in prop_oneof![Just(RuleSet::PROSE), Just(RuleSet::INTERFACE), Just(RuleSet::ALL)],
    ) {
        let t = Typographer::new(if lang == 0 { ru() } else { en() }, set);
        let y = t.typeset(&x, Output::Unicode);
        if let Err(error) = check_protected(&t, &y) {
            prop_assert!(false, "{}", error);
        }
    }
}

// ---------------------------------------------------------------------------------------------
// The known losses (design section 12.2)
// ---------------------------------------------------------------------------------------------

/// The codes of the table of section 12.2.
const KNOWN_CODES: [&str; 13] = [
    "DoubleHyphen",
    "CompactAbbr",
    "AttachedSign",
    "TerminalEllipsis",
    "SingleQuote",
    "NestedSingle",
    "CyrillicTimes",
    "PercentInserted",
    "PercentRemoved",
    "LegalMarkSpelling",
    "TimesSpelling",
    "AuthorGlue",
    "NestedPossessive",
];

#[test]
fn the_table_of_known_losses_holds() {
    let rows = known_rows();
    let mut failures: Vec<String> = Vec::new();
    for row in &rows {
        let t = settings_typographer(&row.lang, &row.settings);
        let typeset = t.typeset(&row.input, Output::Unicode).into_owned();
        let plain = t.to_plain(&typeset).into_owned();
        let mut problems: Vec<String> = Vec::new();
        if typeset != row.typeset {
            problems.push(format!("typeset is {}", show(&typeset)));
        }
        if plain != row.plain {
            problems.push(format!("to_plain is {}", show(&plain)));
        }
        if plain == row.input {
            problems.push("the loss vanished: to_plain(typeset(input)) == input".to_owned());
        }
        // P2 holds for every row (section 12.2).
        let again = t.typeset(&row.plain, Output::Unicode);
        if again != row.typeset {
            problems.push(format!(
                "P2: typeset(to_plain) is {}, not {}",
                show(&again),
                show(&row.typeset)
            ));
        }
        if !problems.is_empty() {
            failures.push(format!(
                "line {} {} {} {} [{}]: {}",
                row.line,
                row.lang,
                row.settings,
                show(&row.input),
                row.code,
                problems.join("; ")
            ));
        }
    }
    let listed: Vec<String> = rows
        .iter()
        .map(|r| {
            format!(
                "{} {} {:?} -> {:?} -> {:?} [{}]",
                r.lang,
                r.settings,
                show(&r.input),
                show(&r.typeset),
                show(&r.plain),
                r.code
            )
        })
        .collect();
    assert!(
        failures.is_empty(),
        "rows of roundtrip-known.tsv that no longer hold:\n{}\n\nthe rows:\n{}",
        failures.join("\n"),
        listed.join("\n")
    );
    for code in KNOWN_CODES {
        assert!(
            rows.iter().any(|r| r.code == code),
            "roundtrip-known.tsv has no row with the code {code} (section 12.2)"
        );
    }
    for row in &rows {
        assert!(
            KNOWN_CODES.contains(&row.code.as_str()),
            "line {}: unknown code {}",
            row.line,
            row.code
        );
    }
}

// ---------------------------------------------------------------------------------------------
// P1 and P2 (design section 12.2)
// ---------------------------------------------------------------------------------------------

/// P1 for one string: `to_plain(typeset(x)) == x`.
fn p1(t: &Typographer, x: &str) -> Result<(), String> {
    let typeset = t.typeset(x, Output::Unicode);
    let back = t.to_plain(&typeset);
    if back == x {
        Ok(())
    } else {
        Err(format!(
            "P1\n  x        {}\n  typeset  {}\n  to_plain {}",
            show(x),
            show(&typeset),
            show(&back)
        ))
    }
}

/// P2 for one string: `typeset(to_plain(y)) == y` for `y = typeset(x)`.
fn p2(t: &Typographer, x: &str) -> Result<(), String> {
    let y = t.typeset(x, Output::Unicode).into_owned();
    let plain = t.to_plain(&y).into_owned();
    let again = t.typeset(&plain, Output::Unicode);
    if again == y {
        Ok(())
    } else {
        Err(format!(
            "P2\n  x        {}\n  y        {}\n  to_plain {}\n  typeset  {}",
            show(x),
            show(&y),
            show(&plain),
            show(&again)
        ))
    }
}

/// P1 for a template: the texts and the slots come back as they went in.
fn p1_template(t: &Typographer, message: &common::TmplMessage) -> Result<(), String> {
    let typeset = t.typeset_template(message.pieces(), Output::Unicode);
    let back = t.to_plain_template(template_pieces(&typeset));
    let want = texts_and_slots(message);
    let got = split_template(&back);
    if got == want {
        Ok(())
    } else {
        Err(format!(
            "P1 {}\n  x        {:?}\n  typeset  {:?}\n  to_plain {:?}",
            message.id,
            want.0.iter().map(|s| show(s)).collect::<Vec<_>>(),
            texts(&typeset).iter().map(|s| show(s)).collect::<Vec<_>>(),
            got.0.iter().map(|s| show(s)).collect::<Vec<_>>(),
        ))
    }
}

/// P2 for a template.
fn p2_template(t: &Typographer, message: &common::TmplMessage) -> Result<(), String> {
    let y = t.typeset_template(message.pieces(), Output::Unicode);
    let plain = t.to_plain_template(template_pieces(&y));
    let again = t.typeset_template(template_pieces(&plain), Output::Unicode);
    if split_template(&again) == split_template(&y) {
        Ok(())
    } else {
        Err(format!(
            "P2 {}\n  x        {}\n  y        {:?}\n  to_plain {:?}\n  typeset  {:?}",
            message.id,
            message.flat(),
            texts(&y).iter().map(|s| show(s)).collect::<Vec<_>>(),
            texts(&plain).iter().map(|s| show(s)).collect::<Vec<_>>(),
            texts(&again).iter().map(|s| show(s)).collect::<Vec<_>>(),
        ))
    }
}

#[test]
fn p2_on_percentage_ranges_under_every_percent_setting() {
    // A hyphen range reads as a percentage range with its sign attached or after spaces, so the
    // space that the percent setting removes does not make a dash rule fire (section 8.2).
    let mut failures = Vec::new();
    for language in [ru(), en()] {
        for set in [RuleSet::PROSE, RuleSet::INTERFACE, RuleSet::ALL] {
            for percent in [
                PercentSpacing::AsWritten,
                PercentSpacing::Language,
                PercentSpacing::Attached,
                PercentSpacing::Spaced,
            ] {
                let t = Typographer::builder(language.clone(), set)
                    .percent(percent)
                    .build()
                    .unwrap();
                for x in ["5 - 10-20 %", "5 - 10-20%", "10-20 % - 5", "10-20% - 5"] {
                    if let Err(e) = p2(&t, x) {
                        failures.push(format!("{language} {set} {percent:?}: {e}"));
                    }
                }
            }
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

fn setup_and_text() -> impl Strategy<Value = (Setup, String)> {
    setup().prop_flat_map(|s| {
        let text = canonical_text(&s);
        (Just(s), text)
    })
}

fn setup_and_template() -> impl Strategy<Value = (Setup, String)> {
    setup().prop_flat_map(|s| {
        let text = canonical_template(&s);
        (Just(s), text)
    })
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(600))]

    /// P1 on generated canonical strings (section 13.5, property 13).
    #[test]
    fn p1_on_generated_canonical_strings(case in setup_and_text()) {
        let (setup, x) = case;
        let t = setup.typographer();
        if let Err(message) = p1(&t, &x) {
            prop_assert!(false, "{}\n  setup    {}", message, setup.describe());
        }
    }

    /// P1 on generated canonical templates.
    #[test]
    fn p1_on_generated_canonical_templates(case in setup_and_template()) {
        let (setup, text) = case;
        let t = setup.typographer();
        let messages = parse_tmpl(&format!("generated = {text}"));
        for message in &messages {
            if let Err(error) = p1_template(&t, message) {
                prop_assert!(false, "{}\n  setup    {}", error, setup.describe());
            }
        }
    }

    /// P2 on generated plain strings.
    #[test]
    fn p2_on_generated_plain_strings(case in plain_with_setup()) {
        let (setup, x) = case;
        let t = setup.typographer();
        if let Err(message) = p2(&t, &x) {
            prop_assert!(false, "{}\n  setup    {}", message, setup.describe());
        }
    }

    /// P2 on generated canonical strings as well: they are plain.
    #[test]
    fn p2_on_generated_canonical_strings(case in setup_and_text()) {
        let (setup, x) = case;
        let t = setup.typographer();
        if let Err(message) = p2(&t, &x) {
            prop_assert!(false, "{}\n  setup    {}", message, setup.describe());
        }
    }

    /// P2 on generated templates.
    #[test]
    fn p2_on_generated_templates(case in setup_and_template()) {
        let (setup, text) = case;
        let t = setup.typographer();
        let messages = parse_tmpl(&format!("generated = {text}"));
        for message in &messages {
            if let Err(error) = p2_template(&t, message) {
                prop_assert!(false, "{}\n  setup    {}", error, setup.describe());
            }
        }
    }
}

// ---------------------------------------------------------------------------------------------
// The corpus (design section 13.7)
// ---------------------------------------------------------------------------------------------

/// The inputs of `roundtrip-known.tsv` of a language: a corpus line listed there may diverge.
fn known_inputs(lang: &str) -> Vec<String> {
    known_rows()
        .into_iter()
        .filter(|r| r.lang == lang)
        .map(|r| r.input)
        .collect()
}

#[test]
fn the_corpus_has_its_size() {
    for lang in LANGS {
        let prose_lines = text_lines(lang, "prose.txt").len();
        let tech_lines = text_lines(lang, "tech.txt").len();
        let messages = messages(lang).len();
        assert!(
            prose_lines >= 140,
            "{lang}/prose.txt has {prose_lines} lines, about 150 expected"
        );
        assert!(
            messages >= 140,
            "{lang}/ui.tmpl has {messages} messages, about 150 expected"
        );
        assert!(
            tech_lines >= 100,
            "{lang}/tech.txt has {tech_lines} lines, about 100 expected"
        );
    }
}

#[test]
fn the_corpus_holds_no_marker_and_no_typographic_character() {
    // The corpus is in the plain form: no glue, no typographic glyph of the rules.
    for lang in LANGS {
        for file in FILES {
            let text = read(lang, file);
            for c in text.chars() {
                assert!(
                    !matches!(
                        c,
                        '\u{a0}'
                            | '\u{202f}'
                            | '⍽'
                            | '⎵'
                            | '«'
                            | '»'
                            | '„'
                            | '“'
                            | '”'
                            | '‘'
                            | '’'
                            | '″'
                            | '′'
                            | '—'
                            | '–'
                            | '…'
                            | '×'
                            | '−'
                            | '©'
                            | '®'
                            | '™'
                            | '→'
                            | '←'
                            | '↔'
                            | '≥'
                            | '≤'
                            | '≠'
                            | '±'
                    ),
                    "{lang}/{file}: the character {c:?} is not plain"
                );
            }
        }
    }
}

#[test]
fn p1_on_the_corpus() {
    let mut failures: Vec<String> = Vec::new();
    for lang in LANGS {
        let known = known_inputs(lang);
        // `prose.txt` and `ui.tmpl` with both sets, `tech.txt` with Interface only (section 12.2).
        for (set_name, _) in SETS {
            let t = typographer(lang, set_name);
            for file in ["prose.txt", "tech.txt"] {
                if file == "tech.txt" && set_name != "interface" {
                    continue;
                }
                for line in text_lines(lang, file) {
                    if let Err(error) = p1(&t, &line)
                        && !known.contains(&line)
                    {
                        failures.push(format!("{lang}/{file} {set_name}: {error}"));
                    }
                }
            }
            for message in messages(lang) {
                if let Err(error) = p1_template(&t, &message) {
                    failures.push(format!("{lang}/ui.tmpl {set_name}: {error}"));
                }
            }
        }
    }
    assert!(
        failures.is_empty(),
        "{} corpus lines break P1 and are not in roundtrip-known.tsv:\n{}",
        failures.len(),
        failures.join("\n")
    );
}

#[test]
fn p2_on_the_corpus() {
    let mut failures: Vec<String> = Vec::new();
    for lang in LANGS {
        for (set_name, _) in SETS {
            let t = typographer(lang, set_name);
            for file in ["prose.txt", "tech.txt"] {
                for line in text_lines(lang, file) {
                    if let Err(error) = p2(&t, &line) {
                        failures.push(format!("{lang}/{file} {set_name}: {error}"));
                    }
                }
            }
            for message in messages(lang) {
                if let Err(error) = p2_template(&t, &message) {
                    failures.push(format!("{lang}/ui.tmpl {set_name}: {error}"));
                }
            }
        }
    }
    assert!(
        failures.is_empty(),
        "{} corpus lines break P2:\n{}",
        failures.len(),
        failures.join("\n")
    );
}

#[test]
fn the_whole_corpus_files_round_trip_too() {
    // A file as one string: line starts after line breaks are the starts of dialogue lines.
    let mut failures: Vec<String> = Vec::new();
    for lang in LANGS {
        for (set_name, _) in SETS {
            let t = typographer(lang, set_name);
            for file in ["prose.txt", "tech.txt"] {
                if file == "tech.txt" && set_name != "interface" {
                    continue;
                }
                let text = read(lang, file);
                if let Err(error) = p1(&t, &text) {
                    failures.push(format!("{lang}/{file} {set_name}: {}", first_lines(&error)));
                }
            }
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

fn first_lines(text: &str) -> String {
    text.lines().take(4).collect::<Vec<_>>().join("\n")
}
