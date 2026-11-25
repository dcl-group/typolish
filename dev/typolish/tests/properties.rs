//! The properties of design section 13.5 that hold at every stage and need only the public API:
//! idempotence, the fixed-point oracle, slots and protected spans, letters and digits, `Cow`,
//! pieces and strings, determinism of order and no panic; and, through `report`, the invariants I1,
//! I2, I4 and I5 of design section 4.3 as far as the public API shows them. Stage 3 adds the
//! decoder properties (12), the safe alphabet (17), I3, and the properties of `format_number`;
//! stage 4 adds those of `to_plain` (14 and 16) and, at the end, those of `lint` and `report`
//! (9, 10, 14 and 15).

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

use common::decode::{TYPOGRAPHIC, all_outputs, check_safe_alphabet, decode};
use common::show;
use common::strategies::{
    GRAMMAR_LANGUAGES, GenPiece, GrammarConfig, any_text, borrow, grammar_known_limit, pieces,
    risky_text, rule_set, run_grammar,
};
use proptest::prelude::*;
use typolish::{
    Change, DashStyle, Diagnostic, DiagnosticKind, EscapeStyle, HtmlStyle, Language,
    LanguageBuilder, Output, PercentSpacing, Piece, ProtectedKind, QuotePairs, Report, Rule,
    RuleSet, Scratch, Severity, SingleQuotes, Template, Typographer, TypographerBuilder,
};

const LANGUAGES: [Language; 3] = [Language::RU_RU, Language::EN_US, Language::COMMON];

fn typographer(lang: usize, set: RuleSet, quotes: u8, percent: u8) -> Typographer {
    builder(lang, set, quotes, percent).build().unwrap()
}

fn builder(lang: usize, set: RuleSet, quotes: u8, percent: u8) -> TypographerBuilder {
    let mode = match quotes % 3 {
        0 => SingleQuotes::NestedOnly,
        1 => SingleQuotes::Everywhere,
        _ => SingleQuotes::Never,
    };
    Typographer::builder(LANGUAGES[lang % 3].clone(), set)
        .single_quotes(mode)
        .percent(spacing(percent))
}

/// A built-in set by index.
fn base_set(i: usize) -> RuleSet {
    [
        RuleSet::EMPTY,
        RuleSet::PROSE,
        RuleSet::INTERFACE,
        RuleSet::ALL,
    ][i % 4]
}

/// A permutation of all 27 rules.
fn permutation() -> impl Strategy<Value = Vec<Rule>> {
    Just(Rule::ALL.to_vec()).prop_shuffle()
}

fn spacing(percent: u8) -> PercentSpacing {
    match percent % 4 {
        0 => PercentSpacing::AsWritten,
        1 => PercentSpacing::Language,
        2 => PercentSpacing::Attached,
        _ => PercentSpacing::Spaced,
    }
}

fn outputs() -> Vec<Output> {
    vec![
        Output::Unicode,
        Output::Html(HtmlStyle::NAMED),
        Output::Html(HtmlStyle::DECIMAL),
        Output::Html(HtmlStyle::HEX),
        Output::Html(HtmlStyle::CHARACTERS),
        Output::Html(HtmlStyle::NAMED.nowrap_class("nowrap")),
        Output::Html(HtmlStyle::NAMED.nowrap_style()),
        Output::Escaped(EscapeStyle::JS),
        Output::Escaped(EscapeStyle::JS_HEX),
        Output::Escaped(EscapeStyle::JS_CODE_POINT),
        Output::Escaped(EscapeStyle::JSON),
        Output::Escaped(EscapeStyle::RUST),
        Output::Escaped(EscapeStyle::CSS),
        Output::Escaped(EscapeStyle::JS.non_ascii()),
    ]
}

/// The seeds of the idempotence property of design section 13.5 that need no `LanguageBuilder`.
const SEEDS: &[&str] = &[
    "Школа №5 г. Москвы",
    "§5 ч. 2",
    "т.е.",
    "и т.д...",
    // A compact abbreviation next to a dash reads as a word in both forms (design section 8.2).
    "Он пришёл - т.е. опоздал",
    "Слово - т.е.",
    "Москва – т.е. столица",
    "Слово -- т.д.",
    "5 - т.е.",
    "т.е. - 5",
    "Moscow - the capital",
    "See https://example.com/docs - the guide",
    "He said - \"a cat\"",
    // A dash next to a fragment that the closed style merged (design section 4.3, I4).
    "details - a - т.е.sk",
    "5 - x - Moscow",
    // A rule that reads a merged or a split neighbor reads it in its plain form (design section
    // 4.3, I4); the rows of `merged_and_split_fragments_read_as_their_plain_form`.
    "Moscow - a, - b",
    "Plan - A, - B",
    "Moscow - a) - b",
    "A - (b - cc)",
    "a - 'b - c's",
    "it's - B... - c",
    "Hello—world, - he said",
    "Москва - а, - б",
    "Москва - cat, - file",
    "5 in - yes",
    "Up to 5 in - no more",
    "5 kg - yes",
    "5 с - стр.",
    "№5 ли ты",
    "§5 же ты",
    "№V ли ты",
    "ли - ведь же (к)",
    "№5 - x",
    "X - №V",
    "x - '§X",
    "т. н.э.",
    "т. н.э. О",
    "он - т.е.а. в",
    "ООО - т. д. Я",
    "США Т. США - к",
    "ООО А. Пушкин - км (не",
    "See §V. St. Paul",
    // A capitalized first part of an abbreviation next to initials (design section 7.1).
    "Т. е. так",
    "Т. д. так",
    "Буква Т. е. не",
    "Т. Е. Иванов",
    // A compact abbreviation after a surname, split into a part that reads as an initial (design
    // sections 4.3, I2, and 8.7).
    "Москва Т.е.",
    "Пушкин А. Т.е.",
    "Ма Т.н.",
    // An elision closed by a straight quote waits for pairing (design section 6.2).
    "Choose the 'round' button",
    "\"Say 'twas' or 'tis'\"",
    "'Tis 'round' here",
    "5'10\"",
    "90's",
    "'90s",
    "'tis",
    "\"\"",
    "\"\"\"",
    "key=\"\"",
    "\"He said 'hi'\"",
    "\"a 'b c\" 5'",
    "\"/etc/\"",
    "Токен \"SGVsbG8gd29ybGQ=\" и \"имя\".",
    "‘Note: 'beta' builds of the developers' tools",
    "¶IV",
    "100%",
    "100‰",
    "100 %",
    "100\u{a0}%",
    // A percentage range and a percent sign after spaces next to a dash (design section 8.2).
    "5–10%",
    "5–10 %",
    "100 % - норма",
    "5–10% - норма",
    "Рост - 5–10%",
    // A hyphen range with its percent sign attached or after spaces (design section 8.2).
    "5 - 10-20 %",
    "5 - 10-20%",
    "10-20 % - 5",
    "5 - 1,5-2,5 %",
    "5 - 1.5-2.5 %",
    // A range of Roman numerals with its percent sign after spaces, which the percent setting
    // attaches (design section 8.2).
    "XIX-XX % - 27",
    "I-5 % - x",
    // A label number owns its percent sign after spaces too (design section 8.2).
    "№5 % — т.е.",
    "№10 % — -5% +",
    "№5 %\u{a0}-- `x`",
    "§5 % - x",
    // A percentage after spaces as the left factor of a lone `x` and next to `(c)` (design sections
    // 8.8 and 8.9).
    "10 % x 5",
    "10 %... X\u{a0}10 %",
    "(c) 2026 %",
    "(c) 2026%",
    "№2026 (c)",
    "word\u{a0}- word",
    "Sign in - it's free",
    "Подождите...\nДальше",
    "Экран \u{2068}27\u{2069}\" и \"ручка\"",
    // A sign-only pair next to a dash (design sections 6.3 and 8.2).
    "Слово - \"\" да",
    "\"\" - 5",
    "- \"\" да",
    "-- \"\"",
    "Введите \"-\" или \"—\"",
    // A series of three or more quotes of any form has no sign-only pair inside it (design section
    // 6.3).
    "\"--\"\"\"",
    // A neutral typed closer inside a new pair of its family (design section 6.6).
    "x '\"»b 5' c\"",
    "'\"»90'\"",
    "’'\"”»90'„\"",
    "\"a » b\"",
    "''''- a' b'",
    // The trailing possessive after `s` and `S` (design section 6.6).
    "DOGS' and JONES' BOOK",
    "the dogs' bowl and the Joneses' car",
    // A trailing possessive that a second pass or `to_plain` would read as the closer of the pair
    // around it stays straight (design section 6.6).
    "\"\"s' 'x.\"'\"",
    "“\"s' '.\"'”",
    "\"a \"dogs' 's\" b\"",
    "\"a \"DOGS' 'S\" b\"",
    // A possessive placed where nothing after the closer could close the unpaired opener; and one
    // that stays straight because a weak `'` follows, which a second pass would pair with it.
    "\"a \"dogs' 'x\" b\"",
    "\"a \"s' 'x\" 5' b\"",
    // A run of pairs closed by `’`: the possessive closes the inner one early, a second possessive
    // the outer one, and the opener between them meets the `'` after them.
    "“\"S ‘s'\ns'\n't’ x' s\"”",
    // A sign or a unit next to a number with and without a space (design section 8.7): two rules
    // must not read one character.
    "5 %1 and 5% and 5 %",
    "5 $1 and 5$ and 5 $",
    "5 §1 и 5 §",
    "5 °1 и 5° и 5 °",
    // The units of design sections 7.2 and 7.3 that the families and the superscript forms
    // complete, and `a.m.` and `p.m.`, which are atom exceptions too.
    "5 м³ 5 см³ 5 км² 5 см² 5 мм²",
    "5-10 ГВт и 5 Дж, 10 кбит/с - 3 Ом",
    "5 ккал… 5 кОм т.е. 5 Па",
    "5 p.m. and 5 a.m.",
    "9-11 a.m. - 5:30 p.m., 5 P.M.",
    "5 kPa - 5 GW, 5 mW... 5 kJ",
    // An entry of words joined by hyphens reads as a compound word, one of letters and digits as a
    // word (design section 8.2).
    "г-жа - г-н",
    "Пришла г-жа - и ушла",
    "г-н — это",
    "г-н - cat - 5 км/ч - г-жа",
    "- г-н Иванов\n-- м2 это",
    // A context range before a compact abbreviation whose first part is a unit (design sections
    // 4.3, I4, and 8.5): `т.е.` reads as `т.`, the part `nbsp.abbreviations` splits off.
    "1,5-2,5 т.е.",
    "Ждать 1,5-2,5 т.е. и т.д.",
    // A `Joined` core after a dialogue dash or a context range reads as its first word (design
    // section 4.3, I4).
    "- река—ООО",
    "1,5-2,5 ккал—бы",
    "1.5-2.5 mW—rock",
    // A label after a dialogue dash is a word, not its sign (design section 8.4).
    "- №5 дом",
    "— §5 текст",
    "- ¶IV дом",
];

#[test]
fn idempotence_of_the_seeds() {
    for seed in SEEDS {
        for (lang, language) in LANGUAGES.iter().enumerate() {
            for set in [
                RuleSet::EMPTY,
                RuleSet::ALL,
                RuleSet::PROSE,
                RuleSet::INTERFACE,
            ] {
                for quotes in 0..3 {
                    for percent in 0..4 {
                        let t = typographer(lang, set, quotes, percent);
                        let once = t.typeset(seed, Output::Unicode).into_owned();
                        let twice = t.typeset(&once, Output::Unicode).into_owned();
                        assert_eq!(
                            once,
                            twice,
                            "{} {set} {seed:?}: {} then {}",
                            language,
                            show(&once),
                            show(&twice)
                        );
                    }
                }
            }
        }
    }
}

/// Seeds of message templates with `BlockMarkup` pieces (design section 2.6): `¦` stands for the
/// piece `</p><p>` between `Text` pieces.
const BLOCK_SEEDS: &[&str] = &[
    "Он сказал \"да¦Нет\" - ответила она.",
    "Текст.¦- Да, - сказал он.",
    "\"a¦b\" c",
    "в¦доме и т.д...¦- Нет",
    "5¦- 7 км¦10-20",
    "'a¦'s' x",
    "```¦code¦```",
    "¦¦- x - y¦",
];

/// The pieces of a seed of `BLOCK_SEEDS`.
fn block_pieces(texts: &[String]) -> Vec<Piece<'_, ()>> {
    let mut pieces = Vec::new();
    for (i, text) in texts.iter().enumerate() {
        if i > 0 {
            pieces.push(Piece::BlockMarkup("</p><p>"));
        }
        pieces.push(Piece::Text(text));
    }
    pieces
}

#[test]
fn idempotence_of_the_template_seeds_with_a_block_boundary() {
    // Design section 13.5, property 1, for templates: the texts between the `BlockMarkup` pieces
    // of the output, typeset again in the same pieces, stay as they are.
    for seed in BLOCK_SEEDS {
        let texts: Vec<String> = seed.split('¦').map(str::to_owned).collect();
        for lang in 0..3 {
            for set in [RuleSet::ALL, RuleSet::PROSE, RuleSet::INTERFACE] {
                for quotes in 0..3 {
                    let t = typographer(lang, set, quotes, 0);
                    let once = t
                        .typeset_template(block_pieces(&texts), Output::Unicode)
                        .texts()
                        .concat();
                    let parts: Vec<String> = once.split("</p><p>").map(str::to_owned).collect();
                    assert_eq!(parts.len(), texts.len(), "{seed:?}: {once:?}");
                    let twice = t
                        .typeset_template(block_pieces(&parts), Output::Unicode)
                        .texts()
                        .concat();
                    assert_eq!(
                        once,
                        twice,
                        "{set} {seed:?}: {} then {}",
                        show(&once),
                        show(&twice)
                    );
                }
            }
        }
    }
}

#[test]
fn idempotence_of_the_seed_with_a_built_language() {
    // The seed `‚Wort‘` of design section 13.5, with a `LanguageBuilder` language whose pairs are
    // `„“` and `‚‘`: a typed nested pair, and around it straight quotes that take the language's
    // glyphs.
    let de = LanguageBuilder::from_language(&Language::EN_US, "de-DE")
        .quotes(Some(QuotePairs::new(('„', '“'), ('‚', '‘'))))
        .build()
        .unwrap();
    for seed in ["‚Wort‘", "\"Er sagte ‚Wort‘\"", "\"a 'b' c\""] {
        for set in [
            RuleSet::EMPTY,
            RuleSet::ALL,
            RuleSet::PROSE,
            RuleSet::INTERFACE,
        ] {
            for mode in [
                SingleQuotes::NestedOnly,
                SingleQuotes::Everywhere,
                SingleQuotes::Never,
            ] {
                let t = Typographer::builder(de.clone(), set)
                    .single_quotes(mode)
                    .build()
                    .unwrap();
                let once = t.typeset(seed, Output::Unicode).into_owned();
                let twice = t.typeset(&once, Output::Unicode);
                assert_eq!(once, twice, "{set} {mode:?} {seed:?}");
            }
        }
    }
    // A language whose primary pair is `‘’` (en-GB): a trailing possessive inside a pair that `’`
    // closes (design section 6.6).
    let gb = LanguageBuilder::from_language(&Language::EN_US, "en-GB")
        .quotes(Some(QuotePairs::new(('‘', '’'), ('“', '”'))))
        .build()
        .unwrap();
    for seed in [
        "\"S' 's\"",
        "\"s' 'x\" 5' a",
        "\"s' 'x\" a",
        "S\n\"boss'\n'27\nbowls\"",
        "it) - 'cause' - 'cause -- \"'cause) DOGS' - 'house' - it, - oz.) '90s\" a",
        "\"S'. a(\n'3,\".'",
        // A sign-only pair `‘’` before a number inside a straight single pair (design sections 6.3
        // and 6.5).
        "'a \"\"-3'",
        "'a \"\" 3'",
        "\"\" ('s'’ “”«",
    ] {
        for set in [
            RuleSet::EMPTY,
            RuleSet::ALL,
            RuleSet::PROSE,
            RuleSet::INTERFACE,
        ] {
            for mode in [
                SingleQuotes::NestedOnly,
                SingleQuotes::Everywhere,
                SingleQuotes::Never,
            ] {
                let t = Typographer::builder(gb.clone(), set)
                    .single_quotes(mode)
                    .build()
                    .unwrap();
                let once = t.typeset(seed, Output::Unicode).into_owned();
                let twice = t.typeset(&once, Output::Unicode);
                assert_eq!(once, twice, "en-GB {set} {mode:?} {seed:?}");
            }
        }
    }
    let t = Typographer::new(de, RuleSet::INTERFACE);
    assert_eq!(t.typeset("‚Wort‘", Output::Unicode), "‚Wort‘");
    assert_eq!(
        t.typeset("\"Er sagte \"Wort\"\"", Output::Unicode),
        "„Er sagte ‚Wort‘“"
    );
}

#[test]
fn merged_and_split_fragments_read_as_their_plain_form() {
    // I4 of design section 4.3: a fragment that the closed dash merged (`Moscow—a`) or that
    // `nbsp.signs` and `nbsp.abbreviations` split (`№⍽5`, `н.⍽э.`) reads as the parts it stands
    // for, so the first pass gives a fixed point and `to_plain` of it comes back.
    let ru_closed = Typographer::builder(Language::RU_RU, RuleSet::PROSE)
        .dash_style(DashStyle::Closed)
        .build()
        .unwrap();
    let en_p = Typographer::new(Language::EN_US, RuleSet::PROSE);
    let en_i = Typographer::new(Language::EN_US, RuleSet::INTERFACE);
    let ru_p = Typographer::new(Language::RU_RU, RuleSet::PROSE);
    let ru_i = Typographer::new(Language::RU_RU, RuleSet::INTERFACE);
    let rows: &[(&Typographer, &str, &str)] = &[
        // A dash across a wrapper reads the word next to it (design section 8.4).
        (&en_p, "Moscow - a, - b", "Moscow—a, - b"),
        (&en_i, "Plan - A, - B", "Plan—A, - B"),
        (&en_p, "Moscow - a) - b", "Moscow—a) - b"),
        (&en_p, "A - (b - cc)", "A - (b—cc)"),
        (&en_p, "a - 'b - c's", "a - 'b—c’s"),
        (&en_p, "it's - B... - c", "it’s—B… - c"),
        (&en_p, "Hello—world, - he said", "Hello—world, — he said"),
        (&ru_closed, "Москва - а, - б", "Москва—а, - б"),
        (&ru_closed, "Москва - cat, - file", "Москва—cat, - file"),
        // An ambiguous unit before the dash of a merge (design section 8.7).
        (&en_p, "5 in - yes", "5 in—yes"),
        (&en_i, "5 in - yes", "5 in—yes"),
        (
            &en_p,
            "Up to 5 in - no more",
            "Up\u{a0}to\u{a0}5 in—no more",
        ),
        (&en_p, "5 kg - yes", "5\u{a0}kg—yes"),
        (&ru_closed, "5 с - стр.", "5 с—стр."),
        // A particle after a label or after a merge (design section 8.7).
        (&ru_p, "№5 ли ты", "№\u{a0}5 ли\u{a0}ты"),
        (&ru_i, "№5 ли ты", "№\u{a0}5 ли ты"),
        (&ru_p, "§5 же ты", "§\u{a0}5 же\u{a0}ты"),
        (&ru_p, "№V ли ты", "№\u{a0}V\u{a0}ли ты"),
        (&ru_closed, "ли - ведь же (к)", "ли—ведь\u{a0}же (к)"),
        // A dialogue dash and a context range before a merge (design sections 8.4 and 8.5).
        (&ru_closed, "- река - ООО", "—\u{a0}река—ООО"),
        (&ru_closed, "1,5-2,5 ккал -- бы", "1,5–2,5\u{a0}ккал—бы"),
        // A dash next to a label reads its numeral or its sign (design section 8.4).
        (&ru_p, "№5 - x", "№\u{a0}5 - x"),
        (&ru_p, "X - №V", "X\u{a0}— №\u{a0}V"),
        (&ru_i, "x - '§X", "x\u{a0}— '§\u{a0}X"),
        // An abbreviation whose last part starts a compact one, or a part of a merge (design
        // section 8.7).
        (&ru_p, "т. н.э.", "т.\u{a0}н.э."),
        (&ru_i, "т. н.э. О", "т.\u{a0}н.э. О"),
        (&ru_p, "он - т.е.а. в", "он - т.е.а. в"),
        (&ru_closed, "ООО - т. д. Я", "ООО—т.\u{a0}д. Я"),
        // Initials next to a merge or a label (design section 8.7).
        (&ru_closed, "США Т. США - к", "США Т.\u{a0}США—к"),
        (
            &ru_closed,
            "ООО А. Пушкин - км (не",
            "ООО А.\u{a0}Пушкин—км\u{a0}(не",
        ),
        (&en_p, "See §V. St. Paul", "See §\u{a0}V. St.\u{a0}Paul"),
    ];
    for &(t, text, expected) in rows {
        let once = t.typeset(text, Output::Unicode).into_owned();
        assert_eq!(once, expected, "{text:?}: {}", show(&once));
        let twice = t.typeset(&once, Output::Unicode).into_owned();
        assert_eq!(
            twice,
            once,
            "{text:?}: {} then {}",
            show(&once),
            show(&twice)
        );
        let plain = t.to_plain(&once);
        let back = t.typeset(&plain, Output::Unicode).into_owned();
        assert_eq!(
            back,
            once,
            "{text:?}: to_plain {plain:?} gives {}",
            show(&back)
        );
    }
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(512))]

    /// 1. Idempotence and 2. the fixed-point oracle; a difference prints the reports of both
    /// passes (design section 13.5).
    #[test]
    fn idempotence_and_fixed_point(
        text in any_text(), lang in 0usize..3, set in rule_set(), quotes in 0u8..3, percent in 0u8..4,
    ) {
        let t = typographer(lang, set, quotes, percent);
        let once = t.typeset(&text, Output::Unicode).into_owned();
        let twice = t.typeset(&once, Output::Unicode).into_owned();
        prop_assert!(
            once == twice,
            "first pass:\n{}\nsecond pass:\n{}",
            t.report(&text, Output::Unicode),
            t.report(&once, Output::Unicode)
        );
        let mut current = text.clone();
        let mut previous = text.clone();
        for _ in 0..3 {
            let next = t.typeset(&current, Output::Unicode).into_owned();
            if next == current {
                break;
            }
            previous = std::mem::replace(&mut current, next);
        }
        prop_assert!(
            current == once,
            "first pass:\n{}\nlast pass:\n{}",
            t.report(&text, Output::Unicode),
            t.report(&previous, Output::Unicode)
        );
    }

    /// 1. Idempotence with every minus sign the builder accepts (design section 2.11): the sign
    /// that `number.minus` writes is read as the sign of a number by the second pass.
    #[test]
    fn idempotence_with_any_accepted_minus_sign(
        text in any_text(), lang in 0usize..2, set in rule_set(), percent in 0u8..4,
        minus in prop::sample::select(vec![
            '-', '\u{2010}', '\u{2011}', '\u{2012}', '\u{2013}', '\u{2014}', '\u{2015}', '\u{2212}',
        ]),
    ) {
        let t = Typographer::builder(LANGUAGES[lang].clone(), set.with(Rule::NumberMinus))
            .minus_sign(minus)
            .percent(spacing(percent))
            .build()
            .unwrap();
        let once = t.typeset(&text, Output::Unicode).into_owned();
        let twice = t.typeset(&once, Output::Unicode).into_owned();
        prop_assert!(
            once == twice,
            "first pass:\n{}\nsecond pass:\n{}",
            t.report(&text, Output::Unicode),
            t.report(&once, Output::Unicode)
        );
    }

    /// 8. Cow: borrowed exactly when the output equals the input, for every output kind.
    #[test]
    fn cow_is_borrowed_exactly_when_equal(text in any_text(), lang in 0usize..3, set in rule_set()) {
        let t = typographer(lang, set, 0, 0);
        for output in outputs() {
            let out = t.typeset(&text, output);
            prop_assert_eq!(matches!(out, Cow::Borrowed(_)), out == text.as_str(), "{:?}", output);
        }
    }

    /// 4. Slots and protected spans.
    #[test]
    fn slots_and_spans_stay(parts in pieces(), lang in 0usize..3, set in rule_set()) {
        let t = typographer(lang, set, 0, 0);
        let input = borrow(&parts);
        let out = t.typeset_template(input.iter().copied(), Output::Unicode);
        let expected_slots: Vec<(u32, typolish::SlotKind)> = parts
            .iter()
            .filter_map(|p| match p {
                GenPiece::Slot(id, kind) => Some((*id, *kind)),
                _ => None,
            })
            .collect();
        let got: Vec<(u32, typolish::SlotKind)> =
            out.slots().iter().map(|(id, slot)| (*id, slot.kind())).collect();
        prop_assert_eq!(got, expected_slots);
        prop_assert_eq!(out.texts().len(), out.slots().len() + 1);
        // Protected text and markup stand byte for byte, in order, between the slot marks.
        let joined = out.fill(|id| Cow::Owned(format!("\u{e000}{id}\u{e001}")));
        let mut at = 0;
        for p in &parts {
            let needle = match p {
                GenPiece::Slot(id, _) => format!("\u{e000}{id}\u{e001}"),
                GenPiece::Protected(text) | GenPiece::Markup(text) | GenPiece::Block(text) => {
                    text.clone()
                }
                GenPiece::Text(_) => continue,
            };
            match joined[at..].find(&needle) {
                Some(i) => at += i + needle.len(),
                None => prop_assert!(false, "{:?} not found after {} in {:?}", needle, at, joined),
            }
        }
    }

    /// 6. Letters and digits, with `symbols.*` and `number.times` off.
    #[test]
    fn letters_and_digits_stay(text in any_text(), lang in 0usize..3, set in rule_set()) {
        let set: RuleSet = set
            .iter()
            .filter(|r| !r.id().starts_with("symbols.") && *r != Rule::NumberTimes)
            .collect();
        let t = typographer(lang, set, 0, 0);
        let out = t.typeset(&text, Output::Unicode);
        let key = |s: &str| s.chars().filter(|c| c.is_alphanumeric()).collect::<String>();
        prop_assert_eq!(key(&out), key(&text));
    }

    /// 10. A template of `Text` pieces equals the typeset concatenation, cut anywhere.
    #[test]
    fn text_pieces_equal_the_whole(text in risky_text(), lang in 0usize..3, set in rule_set(), cut in any::<prop::sample::Index>()) {
        let t = typographer(lang, set, 0, 0);
        let boundaries: Vec<usize> = text.char_indices().map(|(i, _)| i).chain([text.len()]).collect();
        let at = boundaries[cut.index(boundaries.len())];
        let (a, b) = text.split_at(at);
        let out = t.typeset_template::<()>([typolish::Piece::Text(a), typolish::Piece::Text(b)], Output::Unicode);
        prop_assert_eq!(out.texts().len(), 1);
        let whole = t.typeset(&text, Output::Unicode);
        prop_assert_eq!(out.texts()[0].as_ref(), whole.as_ref());
    }

    /// 11. Determinism of order: a set built in any order gives byte-equal output.
    #[test]
    fn rule_order_is_fixed(text in any_text(), lang in 0usize..3, rules in prop::collection::vec(0usize..27, 0..27), seed in any::<u64>()) {
        let forward: RuleSet = rules.iter().map(|i| Rule::ALL[*i]).collect();
        let mut shuffled = rules.clone();
        // A deterministic shuffle.
        let mut state = seed | 1;
        for i in (1..shuffled.len()).rev() {
            state = state.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
            shuffled.swap(i, (state >> 33) as usize % (i + 1));
        }
        let mut backward = RuleSet::EMPTY;
        for i in &shuffled {
            backward = backward.with(Rule::ALL[*i]);
        }
        prop_assert_eq!(forward, backward);
        let a = typographer(lang, forward, 0, 0).typeset(&text, Output::Unicode).into_owned();
        let b = typographer(lang, backward, 0, 0).typeset(&text, Output::Unicode).into_owned();
        prop_assert_eq!(a, b);
    }
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(10_000))]

    /// 14. No panic on arbitrary strings and pieces, in every output kind.
    #[test]
    fn no_panic_on_strings(text in any_text(), lang in 0usize..3, set in rule_set(), pick in 0usize..14) {
        let t = typographer(lang, set, 0, 0);
        let output = outputs()[pick];
        let _ = t.typeset(&text, output);
        let mut buf = String::new();
        t.typeset_into(&text, output, &mut buf);
    }

    #[test]
    fn no_panic_on_arbitrary_unicode(text in "\\PC*|.*", lang in 0usize..3) {
        let t = typographer(lang, RuleSet::ALL, 0, 0);
        let _ = t.typeset(&text, Output::Unicode);
    }

    #[test]
    fn no_panic_on_pieces(parts in pieces(), lang in 0usize..3, set in rule_set(), pick in 0usize..14) {
        let t = typographer(lang, set, 0, 0);
        let input = borrow(&parts);
        let out = t.typeset_template(input, outputs()[pick]);
        prop_assert_eq!(out.texts().len(), out.slots().len() + 1);
    }
}

#[test]
fn no_panic_on_hand_picked_hard_strings() {
    let t = typographer(0, RuleSet::ALL, 0, 0);
    for text in [
        "\u{301}",
        "\u{301}\u{301}\u{301}",
        "\u{200d}",
        "\"\u{301}",
        "\u{fdd0}\u{ffff}",
        "\u{0}\u{1f}\u{7f}",
        "\"",
        "'",
        "`",
        "``",
        "```",
        "--",
        "...",
        "\r",
        "\r\n\r\n",
        "🇷",
        "👨\u{200d}",
        "№",
        "\u{2068}",
    ] {
        for output in outputs() {
            let _ = t.typeset(text, output);
        }
    }
}

// The invariants of design section 4.3 (properties 3 of design section 13.5), observed through
// `report`.
//
// Before stage 3 the sets run without the `nbsp.*` rules, as the set-level tables do (design
// section 15.4); stage 3 lifts the filter where the invariant allows it (I2 with a percent sign
// that a space splits, design section 4.3, keeps it).

/// A rule set without the `nbsp.*` rules.
fn stage2_set() -> impl Strategy<Value = RuleSet> {
    rule_set().prop_map(|set| {
        set.iter()
            .filter(|rule| !rule.id().starts_with("nbsp."))
            .collect()
    })
}

fn kinds(report: &Report<'_>) -> Vec<ProtectedKind> {
    report.protected().iter().map(|s| s.kind()).collect()
}

fn is_glue_or_space(text: &str) -> bool {
    !text.is_empty()
        && text
            .chars()
            .all(|c| matches!(c, ' ' | '\u{a0}' | '\u{202f}'))
}

/// A string of quotes, dots, dashes, digits, words and spaces with no line break: one window.
fn quote_text() -> impl Strategy<Value = String> {
    const PARTS: &[&str] = &[
        "\"",
        "'",
        "\"\"",
        " ",
        " ",
        " ",
        "a",
        "b",
        "word",
        "5",
        "10",
        "27",
        "don't",
        "\"x\"",
        "'y'",
        "«",
        "»",
        "„",
        "“",
        "”",
        "‘",
        "’",
        "′",
        "″",
        ".",
        "...",
        "…",
        ",",
        "?",
        "-",
        " - ",
        "--",
        "(",
        ")",
        "s'",
        "'90s",
        "5'10\"",
        "host:8080",
        "`",
    ];
    prop::collection::vec(prop::sample::select(PARTS), 0..16).prop_map(|v| v.concat())
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(1024))]

    /// I1: an edit never changes a token kind. Observed: insertions are spaces or glue, removals
    /// are single spaces, and a replacement holds no letter and no digit on either side (the
    /// `symbols.*` rules and `number.times`, which replace letters, are off, as in property 6).
    #[test]
    fn i1_edits_keep_the_kinds(text in any_text(), lang in 0usize..3, set in stage2_set()) {
        let set: RuleSet = set
            .iter()
            .filter(|r| !r.id().starts_with("symbols.") && *r != Rule::NumberTimes)
            .collect();
        let t = typographer(lang, set, 0, 0);
        let report = t.report(&text, Output::Unicode);
        for change in report.changes() {
            let original = report.original(change);
            let replacement = change.replacement();
            if original.is_empty() {
                prop_assert!(is_glue_or_space(replacement), "{:?} inserted {:?}", change.rule(), replacement);
            } else if replacement.is_empty() {
                prop_assert_eq!(original, " ", "{:?} removed {:?}", change.rule(), original);
            } else {
                prop_assert!(
                    !original.chars().any(char::is_alphanumeric)
                        && !replacement.chars().any(char::is_alphanumeric),
                    "{:?} replaced {:?} with {:?}", change.rule(), original, replacement
                );
            }
        }
    }

    /// I2 through the public API: the fragments that the classifier freezes in the output of a pass
    /// are those of its input, so the kinds of the protected spans of `f(x)` equal those of `x`. The
    /// full property, every fragment with its kind, flags and extent, is
    /// `i2_and_i4_hold_on_the_classification_and_the_matches` in `src/engine/mod.rs`.
    #[test]
    fn i2_the_classification_is_stable(text in any_text(), lang in 0usize..3, set in stage2_set(), quotes in 0u8..3) {
        let t = typographer(lang, set, quotes, 0);
        let before = t.report(&text, Output::Unicode);
        let out = before.result().to_string();
        let after = t.report(&out, Output::Unicode);
        prop_assert_eq!(kinds(&before), kinds(&after), "{:?} -> {:?}", text, out);
    }

    /// I2 for templates of `Text` pieces and slots: the kinds of the protected spans of the
    /// output template, written out with its slots, equal those of the input.
    #[test]
    fn i2_the_classification_of_pieces_is_stable(
        parts in pieces_text_and_slots(), lang in 0usize..3, set in stage2_set(),
    ) {
        let t = typographer(lang, set, 0, 0);
        let input = borrow(&parts);
        let before = t.report_template(input.iter().copied(), Output::Unicode);
        let (texts, slots) = before.result().clone().into_parts();
        // Rebuild the pieces from the output: text, slot, text, ...
        let mut out: Vec<typolish::Piece<'_, u32>> = Vec::new();
        for (i, text) in texts.iter().enumerate() {
            out.push(typolish::Piece::Text(text));
            if let Some((id, slot)) = slots.get(i) {
                out.push(typolish::Piece::Slot(*id, *slot));
            }
        }
        let after = t.report_template(out, Output::Unicode);
        let a: Vec<_> = before.protected().iter().filter(|s| s.kind() != ProtectedKind::Slot).map(|s| s.kind()).collect();
        let b: Vec<_> = after.protected().iter().filter(|s| s.kind() != ProtectedKind::Slot).map(|s| s.kind()).collect();
        prop_assert_eq!(a, b);
    }

    /// I4 through the public API: on the output of a pass no rule has an edit left, so the report
    /// of `f(x)` holds no change. The recurrence of every match of `x` on `f(x)`, with its rule and
    /// `consume`, is checked by `i2_and_i4_hold_on_the_classification_and_the_matches` in
    /// `src/engine/mod.rs`.
    #[test]
    fn i4_the_second_pass_has_no_edit(text in any_text(), lang in 0usize..3, set in stage2_set(), quotes in 0u8..3) {
        let t = typographer(lang, set, quotes, 0);
        let out = t.typeset(&text, Output::Unicode).into_owned();
        let second = t.report(&out, Output::Unicode);
        prop_assert!(second.changes().is_empty(), "{:?} -> {:?} -> {:?}", text, out, second.changes());
    }

    /// I5: quotes are all or nothing per window (`i5_check`).
    #[test]
    fn i5_a_window_that_does_not_converge_keeps_its_quotes(
        text in quote_text(), lang in 0usize..2, bits in any::<u8>(), mode in 0u8..3,
    ) {
        i5_check(&text, lang, bits, mode)?;
    }
}

/// I5 for one input: in a window that does not converge (an `UnmatchedQuote` warning or
/// `CrossingQuotes`) no straight `"` changed, and a straight `'` changed only by step 1 (an
/// apostrophe or a prime). The text has no line break: one window. Returns whether the window was
/// blocked.
fn i5_check(text: &str, lang: usize, bits: u8, mode: u8) -> Result<bool, TestCaseError> {
    let mut set = RuleSet::EMPTY.with(Rule::QuotesPairs);
    for (bit, rule) in [
        (1, Rule::QuotesApostrophe),
        (2, Rule::QuotesPrimes),
        (4, Rule::EllipsisDots),
        (8, Rule::DashSpacedHyphen),
    ] {
        if bits & bit != 0 {
            set = set.with(rule);
        }
    }
    let t = typographer(lang, set, mode, 0);
    let report = t.report(text, Output::Unicode);
    let blocked = report.diagnostics().iter().any(|d| {
        (d.kind() == &DiagnosticKind::UnmatchedQuote && d.severity() == Severity::Warning)
            || d.kind() == &DiagnosticKind::CrossingQuotes
    });
    if blocked {
        for change in report.changes() {
            let original = report.original(change);
            prop_assert_ne!(original, "\"", "{:?} -> {:?}", text, report.result());
            if original == "'" {
                prop_assert!(
                    matches!(change.replacement(), "’" | "′"),
                    "{:?} -> {:?}",
                    text,
                    report.result()
                );
            }
        }
    }
    Ok(blocked)
}

#[test]
fn i5_blocked_seeds() {
    // Blocked windows, so that the check of `i5_check` runs: a crossing pair, and an unmatched
    // quote next to an apostrophe that still changes (`'` → `’`), so the loop over changes runs.
    for (text, lang, bits) in [
        ("«a \"b» c\"", 0, 0),
        ("Он сказал \"don't.", 0, 1),
        ("He said \"don't.", 1, 1),
    ] {
        for mode in 0..3 {
            assert!(
                matches!(i5_check(text, lang, bits, mode), Ok(true)),
                "{text:?} mode {mode}"
            );
        }
    }
    // The apostrophe changes in the blocked window: the loop over changes is not empty.
    let t = typographer(
        0,
        RuleSet::EMPTY
            .with(Rule::QuotesPairs)
            .with(Rule::QuotesApostrophe),
        0,
        0,
    );
    assert!(
        !t.report("Он сказал \"don't.", Output::Unicode)
            .changes()
            .is_empty()
    );
}

/// Pieces of text and slots only (no protected or markup pieces), with no two texts side by side,
/// so that the output template can be rebuilt as the same pieces.
fn pieces_text_and_slots() -> impl Strategy<Value = Vec<GenPiece>> {
    pieces().prop_map(|v| {
        let mut out: Vec<GenPiece> = Vec::new();
        let mut n = 0;
        for piece in v {
            match piece {
                GenPiece::Text(t) => match out.last_mut() {
                    Some(GenPiece::Text(last)) => last.push_str(&t),
                    _ => out.push(GenPiece::Text(t)),
                },
                GenPiece::Slot(_, kind) => {
                    out.push(GenPiece::Slot(n, kind));
                    n += 1;
                }
                _ => {}
            }
        }
        out
    })
}

// Stage 3 (design section 15.4): the output kinds, I3 and the numbers.

fn is_css(output: Output) -> bool {
    matches!(output, Output::Escaped(s) if s == EscapeStyle::CSS || s == EscapeStyle::CSS.non_ascii())
}

fn is_html(output: Output) -> bool {
    matches!(output, Output::Html(_))
}

/// Design section 11.8: for `CSS` every U+0000 of the generated text is replaced by U+FFFD before
/// both equations.
fn for_output(output: Output, text: &str) -> String {
    if is_css(output) {
        text.replace('\u{0}', "\u{fffd}")
    } else {
        text.to_owned()
    }
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(768))]

    /// 12. Output kinds: `decode(typeset(x, out)) == typeset(x, Unicode)` and
    /// `typeset(decode(typeset(x, out)), out) == typeset(x, out)`, nowrap styles included.
    #[test]
    fn decoding_gives_the_unicode_result(
        text in any_text(), lang in 0usize..3, set in rule_set(), quotes in 0u8..3, percent in 0u8..4,
    ) {
        let t = typographer(lang, set, quotes, percent);
        for output in all_outputs() {
            let text = for_output(output, &text);
            let unicode = t.typeset(&text, Output::Unicode).into_owned();
            let out = t.typeset(&text, output).into_owned();
            let decoded = match decode(output, &out) {
                Ok(decoded) => decoded,
                Err(e) => return Err(TestCaseError::fail(format!("{output:?} {text:?}: {e}: {out:?}"))),
            };
            prop_assert_eq!(&decoded, &unicode, "{:?} {:?} {:?}", output, text, out);
            let again = t.typeset(&decoded, output).into_owned();
            prop_assert_eq!(&again, &out, "{:?} {:?}", output, text);
        }
    }

    /// 17. Safe alphabet: the output of every kind holds none of the raw characters of its
    /// syntax, every `\` starts an escape that the strict parser accepts, and except with
    /// `HtmlStyle::CHARACTERS` no raw character of the typographic set.
    #[test]
    fn the_output_has_only_its_safe_alphabet(
        text in any_text(), lang in 0usize..3, set in rule_set(),
    ) {
        let t = typographer(lang, set, 0, 0);
        for output in all_outputs() {
            let out = t.typeset(&text, output);
            if let Err(e) = check_safe_alphabet(output, &out) {
                return Err(TestCaseError::fail(format!("{output:?} {text:?}: {e}: {out:?}")));
            }
        }
    }

    /// 12 and 17 for the texts of a template: joined with slot values that need no encoding, the
    /// texts decode to the Unicode template filled the same way (an element may open in one text
    /// and close in another, design section 11.5). `Markup` is written as it is in HTML, so the
    /// HTML kinds run on templates without it.
    #[test]
    fn template_texts_decode_to_the_unicode_texts(
        parts in pieces(), lang in 0usize..3, set in rule_set(),
    ) {
        let t = typographer(lang, set, 0, 0);
        let input = borrow(&parts);
        let has_markup = parts
            .iter()
            .any(|p| matches!(p, GenPiece::Markup(_) | GenPiece::Block(_)));
        let fill = |template: &typolish::Template<'_, u32>| {
            template.fill(|id| Cow::Owned(format!("S{id}E")))
        };
        for output in all_outputs() {
            if is_html(output) && has_markup {
                continue;
            }
            let unicode = fill(&t.typeset_template(input.iter().copied(), Output::Unicode));
            let template = t.typeset_template(input.iter().copied(), output);
            prop_assert_eq!(template.texts().len(), template.slots().len() + 1);
            let joined = fill(&template);
            let decoded = match decode(output, &joined) {
                Ok(decoded) => decoded,
                Err(e) => return Err(TestCaseError::fail(format!("{output:?} {parts:?}: {e}: {joined:?}"))),
            };
            prop_assert_eq!(decoded, for_output(output, &unicode), "{:?} {:?}", output, parts);
            if let Err(e) = check_safe_alphabet(output, &joined) {
                return Err(TestCaseError::fail(format!("{output:?} {parts:?}: {e}: {joined:?}")));
            }
        }
    }

    /// A `BlockMarkup` piece ends the paragraph (design sections 2.6 and 3.3): `Text` pieces
    /// between such pieces typeset as separate calls, and `to_plain_template` reads them as
    /// separate calls too. A fenced block may run across the piece (design section 10.1), so
    /// texts with a run of three backticks are left out.
    #[test]
    fn block_markup_splits_the_text_into_separate_calls(
        texts in prop::collection::vec(risky_text(), 1..4), lang in 0usize..3, set in rule_set(),
        quotes in 0u8..3,
    ) {
        prop_assume!(texts.iter().all(|text| !text.contains("```")));
        let t = typographer(lang, set, quotes, 0);
        let got = t.typeset_template(block_pieces(&texts), Output::Unicode).texts().concat();
        let want: Vec<String> =
            texts.iter().map(|text| t.typeset(text, Output::Unicode).into_owned()).collect();
        prop_assert_eq!(got, want.join("</p><p>"), "{:?}", texts);
        let got = t.to_plain_template(block_pieces(&texts)).texts().concat();
        let want: Vec<String> = texts.iter().map(|text| t.to_plain(text).into_owned()).collect();
        prop_assert_eq!(got, want.join("</p><p>"), "{:?}", texts);
    }

    /// The same for templates with slots, protected pieces and inline markup next to the
    /// `BlockMarkup` pieces: the texts of the template, in Unicode and HTML output and of
    /// `to_plain_template`, are those of a separate call per block, joined at the block. Escaped
    /// output is left out: a `$` at the end of a separate call is escaped, since the next
    /// character is not known there, and a `$` before the block is not (design section 11.6).
    #[test]
    fn block_markup_splits_a_template_into_separate_calls(
        blocks in prop::collection::vec(pieces(), 1..4), lang in 0usize..3, set in rule_set(),
        quotes in 0u8..3, edges in prop::collection::vec((0u8..3, 0u8..3), 4),
    ) {
        // Each block may start and end with a slot or a protected piece, so that one stands next
        // to the `BlockMarkup` piece more often than the generator of pieces makes it.
        let edge = |kind: u8| match kind {
            1 => Some(GenPiece::Slot(0, typolish::SlotKind::Word)),
            2 => Some(GenPiece::Protected("p".to_owned())),
            _ => None,
        };
        let blocks: Vec<Vec<GenPiece>> = blocks
            .into_iter()
            .zip(edges)
            .map(|(block, (start, end))| {
                edge(start)
                    .into_iter()
                    .chain(block.into_iter().filter(|p| !matches!(p, GenPiece::Block(_))))
                    .chain(edge(end))
                    .collect()
            })
            .collect();
        prop_assume!(blocks.iter().flatten().all(|p| match p {
            GenPiece::Text(text) => !text.contains("```"),
            _ => true,
        }));
        let t = typographer(lang, set, quotes, 0);
        let mut all: Vec<GenPiece> = Vec::new();
        for (i, block) in blocks.iter().enumerate() {
            if i > 0 {
                all.push(GenPiece::Block("</p><p>".to_owned()));
            }
            all.extend(block.iter().cloned());
        }
        let all = borrow(&all);
        // The texts of separate calls, the last text of one block and the first of the next
        // joined by `mark`.
        let join = |calls: Vec<Vec<String>>, mark: &str| {
            let mut out: Vec<String> = Vec::new();
            for (i, texts) in calls.into_iter().enumerate() {
                let mut texts = texts.into_iter();
                if i > 0
                    && let (Some(last), Some(first)) = (out.last_mut(), texts.next())
                {
                    last.push_str(mark);
                    last.push_str(&first);
                }
                out.extend(texts);
            }
            out
        };
        let owned = |texts: &[Cow<'_, str>]| -> Vec<String> {
            texts.iter().map(|text| text.to_string()).collect()
        };
        for output in all_outputs().into_iter().filter(|o| !matches!(o, Output::Escaped(_))) {
            let mark = t.typeset_template([Piece::<u32>::BlockMarkup("</p><p>")], output);
            let mark = mark.texts().concat();
            let got = t.typeset_template(all.iter().copied(), output);
            let want = join(
                blocks
                    .iter()
                    .map(|block| owned(t.typeset_template(borrow(block), output).texts()))
                    .collect(),
                &mark,
            );
            prop_assert_eq!(owned(got.texts()), want, "{:?} {:?}", output, all);
        }
        let got = t.to_plain_template(all.iter().copied());
        let want = join(
            blocks
                .iter()
                .map(|block| owned(t.to_plain_template(borrow(block)).texts()))
                .collect(),
            "</p><p>",
        );
        prop_assert_eq!(owned(got.texts()), want, "{:?}", all);
    }

    /// 8. Cow for templates (design section 2.6, output contract items 1 and 2): the text of a gap
    /// between slots with at most one non-empty piece is borrowed exactly when it equals that
    /// piece; a merged text is never borrowed; `changed()` says whether any text differs from the
    /// concatenation of its pieces, in every kind.
    #[test]
    fn template_changed_agrees_with_the_texts(
        parts in pieces(), lang in 0usize..3, set in rule_set(),
    ) {
        let t = typographer(lang, set, 0, 0);
        let input = borrow(&parts);
        // The pieces of each gap between slots.
        let mut gaps: Vec<Vec<&str>> = vec![Vec::new()];
        for piece in &input {
            match *piece {
                typolish::Piece::Slot(..) => gaps.push(Vec::new()),
                typolish::Piece::Text(s)
                | typolish::Piece::Protected(s)
                | typolish::Piece::Markup(s)
                | typolish::Piece::BlockMarkup(s) => {
                    gaps.last_mut().unwrap().push(s);
                }
                _ => {}
            }
        }
        for output in all_outputs() {
            let template = t.typeset_template(input.iter().copied(), output);
            prop_assert_eq!(template.texts().len(), gaps.len());
            let mut any = false;
            for (text, gap) in template.texts().iter().zip(&gaps) {
                let joined: String = gap.concat();
                let borrowed = matches!(text, Cow::Borrowed(_));
                if gap.iter().filter(|s| !s.is_empty()).count() <= 1 {
                    prop_assert_eq!(borrowed, *text == joined, "{:?} {:?}", output, gap);
                } else {
                    prop_assert!(!borrowed, "{:?} {:?}", output, gap);
                }
                any |= *text != joined;
            }
            prop_assert_eq!(template.changed(), any, "{:?} {:?}", output, gaps);
        }
    }

    /// I3: a rule changes only plain forms. Every edit of `report` starts from ASCII (straight
    /// quotes, hyphens, dots, the letters of `(c)` and `x`, spaces) or from the Cyrillic look-alikes
    /// the symbol and times rules read, never from a character of the typographic set; and the
    /// output is a fixed point with no edit left, with all 27 rules.
    #[test]
    fn i3_rules_change_only_plain_forms(
        text in any_text(), lang in 0usize..3, set in rule_set(), quotes in 0u8..3, percent in 0u8..4,
    ) {
        let t = typographer(lang, set, quotes, percent);
        let report = t.report(&text, Output::Unicode);
        for change in report.changes() {
            let original = report.original(change);
            for c in original.chars() {
                prop_assert!(
                    !TYPOGRAPHIC.contains(&c) && (c.is_ascii() || matches!(c, 'с' | 'С' | 'х' | 'Х')),
                    "{:?} changed {:?} in {:?}", change.rule(), original, text
                );
            }
        }
        let out = report.result().to_string();
        let second = t.report(&out, Output::Unicode);
        prop_assert!(second.changes().is_empty(), "{:?} -> {:?} -> {:?}", text, out, second.changes());
    }

    /// 11. Determinism of order with all 27 rules: the full set built along different paths, in
    /// orders drawn from the seed, gives byte-equal output in every output kind. The paths: `with`
    /// from `EMPTY`; `with`, `without` and `with` again from a built-in set; `FromStr` of a shuffled
    /// id list; and `TypographerBuilder::enable` from a built-in set.
    #[test]
    fn all_rules_in_any_order_give_the_same_output(
        text in any_text(), lang in 0usize..3, quotes in 0u8..3,
        first in permutation(), second in permutation(),
        bases in (0usize..4, 0usize..4, 0usize..4), plus in prop::collection::vec(any::<bool>(), 27),
    ) {
        prop_assert_eq!(Rule::ALL.len(), 27);
        let with = first.iter().fold(RuleSet::EMPTY, |set, rule| set.with(*rule));
        let there_and_back = first.iter().fold(base_set(bases.0), |set, rule| set.with(*rule));
        let there_and_back = second.iter().fold(there_and_back, |set, rule| set.without(*rule));
        prop_assert_eq!(there_and_back, RuleSet::EMPTY);
        let there_and_back = first.iter().fold(there_and_back, |set, rule| set.with(*rule));
        let names = ["empty", "prose", "interface", "all"];
        let ids: Vec<String> = second
            .iter()
            .zip(&plus)
            .map(|(rule, plus)| format!("{}{}", if *plus { "+" } else { "" }, rule.id()))
            .collect();
        let parsed: RuleSet = format!("{},{}", names[bases.1], ids.join(","))
            .parse()
            .unwrap();
        for set in [with, there_and_back, parsed] {
            prop_assert_eq!(set, RuleSet::ALL);
        }
        let enabled = first
            .iter()
            .fold(builder(lang, base_set(bases.2), quotes, 0), |b, rule| b.enable(*rule))
            .build()
            .unwrap();
        prop_assert_eq!(enabled.rules(), RuleSet::ALL);
        let typographers = [
            typographer(lang, with, quotes, 0),
            typographer(lang, there_and_back, quotes, 0),
            typographer(lang, parsed, quotes, 0),
            enabled,
        ];
        for output in outputs() {
            let expected = typographers[0].typeset(&text, output).into_owned();
            for t in &typographers[1..] {
                prop_assert_eq!(&t.typeset(&text, output).into_owned(), &expected, "{:?}", output);
            }
        }
    }

    /// 11. Determinism of order with rules removed: a set without a drawn subset of rules, built by
    /// `without` and by `TypographerBuilder::disable` in two orders, gives byte-equal output.
    #[test]
    fn removals_in_any_order_give_the_same_output(
        text in any_text(), lang in 0usize..3,
        first in permutation(), second in permutation(), removed in prop::collection::vec(any::<bool>(), 27),
    ) {
        let drop = |rule: &Rule| removed[Rule::ALL.iter().position(|r| r == rule).unwrap()];
        let a = first.iter().filter(|r| drop(r)).fold(RuleSet::ALL, |set, rule| set.without(*rule));
        let b = second.iter().filter(|r| drop(r)).fold(RuleSet::ALL, |set, rule| set.without(*rule));
        prop_assert_eq!(a, b);
        let disabled = second
            .iter()
            .filter(|r| drop(r))
            .fold(builder(lang, RuleSet::ALL, 0, 0), |b, rule| b.disable(*rule))
            .build()
            .unwrap();
        prop_assert_eq!(disabled.rules(), a);
        let typographers = [typographer(lang, a, 0, 0), typographer(lang, b, 0, 0), disabled];
        for output in outputs() {
            let expected = typographers[0].typeset(&text, output).into_owned();
            for t in &typographers[1..] {
                prop_assert_eq!(&t.typeset(&text, output).into_owned(), &expected, "{:?}", output);
            }
        }
    }
}

fn number_typographer(lang: usize, digits: u8) -> Typographer {
    Typographer::builder(LANGUAGES[lang % 3].clone(), RuleSet::PROSE)
        .min_grouping_digits(digits % 5)
        .build()
        .unwrap()
}

/// The symbols of a built-in language: group, decimal, minus, and where grouping starts.
fn symbols(lang: usize, digits: u8) -> (char, char, char, usize) {
    let digits = digits % 5;
    let group = if lang % 3 == 1 { ',' } else { '\u{a0}' };
    let decimal = if lang.is_multiple_of(3) { ',' } else { '.' };
    (
        group,
        decimal,
        '\u{2212}',
        if digits == 0 {
            usize::MAX
        } else {
            3 + usize::from(digits)
        },
    )
}

/// The number as core wrote it (`text`, an optional `-`, digits, an optional `.` and digits)
/// grouped by the symbols: an independent model of design section 2.7.
fn model(text: &str, lang: usize, digits: u8, zero_unsigned: bool) -> String {
    let (group, decimal, minus, threshold) = symbols(lang, digits);
    let (negative, rest) = match text.strip_prefix('-') {
        Some(rest) => (true, rest),
        None => (false, text.strip_prefix('+').unwrap_or(text)),
    };
    let (int, frac) = match rest.split_once('.') {
        Some((i, f)) => (i, Some(f)),
        None => (rest, None),
    };
    let mut out = String::new();
    if negative && !zero_unsigned {
        out.push(minus);
    }
    for (i, c) in int.chars().enumerate() {
        if int.len() >= threshold && i > 0 && (int.len() - i) % 3 == 0 {
            out.push(group);
        }
        out.push(c);
    }
    if let Some(f) = frac {
        out.push(decimal);
        out.push_str(f);
    }
    out
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(768))]

    /// Numbers: the digits are those of core's `{}`, grouped by three from the threshold, with
    /// the symbols of the language, for every integer type, float and decimal string; NaN and the
    /// infinities are the symbols of the language.
    #[test]
    fn numbers_are_the_digits_grouped(
        lang in 0usize..3, digits in 0u8..5, i in any::<i128>(), u in any::<u128>(),
        f in prop::num::f64::ANY, g in prop::num::f32::ANY, d in "[+-]?[0-9]{1,40}(\\.[0-9]{1,20})?", places in 0u8..12,
    ) {
        use typolish::Number;
        let t = number_typographer(lang, digits);
        let show = |n: Number<'_>| t.format_number(n, Output::Unicode).to_string();
        prop_assert_eq!(show(Number::from(i)), model(&i.to_string(), lang, digits, false));
        prop_assert_eq!(show(Number::from(u)), model(&u.to_string(), lang, digits, false));
        if f.is_finite() {
            let text = format!("{f}");
            let zero = f == 0.0;
            prop_assert_eq!(show(Number::from(f)), model(&text, lang, digits, zero));
            let fixed = format!("{:.*}", usize::from(places), f);
            let zero = fixed.trim_start_matches('-').chars().all(|c| matches!(c, '0' | '.'));
            prop_assert_eq!(show(Number::fixed(f, places)), model(&fixed, lang, digits, zero));
        } else {
            // NaN (whatever its sign) and the infinities are the symbols of the language.
            let symbol = if f.is_nan() {
                if lang.is_multiple_of(3) { "не\u{a0}число" } else { "NaN" }
            } else if f > 0.0 {
                "∞"
            } else {
                "\u{2212}∞"
            };
            prop_assert_eq!(show(Number::from(f)), symbol);
        }
        if g.is_finite() {
            let zero = g == 0.0;
            prop_assert_eq!(show(Number::from(g)), model(&format!("{g}"), lang, digits, zero));
        } else {
            // NaN (whatever its sign) and the infinities are the symbols of the language.
            let symbol = if g.is_nan() {
                if lang.is_multiple_of(3) { "не\u{a0}число" } else { "NaN" }
            } else if g > 0.0 {
                "∞"
            } else {
                "\u{2212}∞"
            };
            prop_assert_eq!(show(Number::from(g)), symbol);
        }
        let parsed = Number::decimal(&d).unwrap();
        let zero = d.trim_start_matches(['+', '-']).chars().all(|c| matches!(c, '0' | '.'));
        prop_assert_eq!(show(parsed), model(&d, lang, digits, zero));
    }

    /// Output kinds of numbers: `decode(format_number(n, out))` is the Unicode number, and
    /// `format_range` is the two numbers around the range dash.
    #[test]
    fn numbers_decode_in_every_kind(
        lang in 0usize..3, digits in 0u8..5, i in any::<i64>(), f in prop::num::f64::ANY,
        j in any::<i128>(),
    ) {
        use typolish::Number;
        let t = number_typographer(lang, digits);
        for output in all_outputs() {
            for n in [Number::from(i), Number::from(f), Number::from(j), Number::fixed(f, 2)] {
                let unicode = t.format_number(n, Output::Unicode).to_string();
                let out = t.format_number(n, output).to_string();
                if let Err(e) = check_safe_alphabet(output, &out) {
                    return Err(TestCaseError::fail(format!("{output:?} {n:?}: {e}: {out:?}")));
                }
                prop_assert_eq!(decode(output, &out), Ok(unicode), "{:?} {:?}", output, n);
            }
            let a = Number::from(i);
            let b = Number::from(f);
            let unicode = t.format_range(a, b, Output::Unicode).to_string();
            let out = t.format_range(a, b, output).to_string();
            prop_assert_eq!(decode(output, &out), Ok(unicode.clone()), "{:?}", output);
            let sides = format!(
                "{}\u{2013}{}",
                t.format_number(a, Output::Unicode),
                t.format_number(b, Output::Unicode)
            );
            prop_assert_eq!(unicode, sides);
        }
    }
}

// ---------------------------------------------------------------------------------------------
// Stage 4: `to_plain` (design sections 2.10 and 12.1; properties 14 and 16). The round trip,
// property 13, lives in `tests/roundtrip.rs`.
// ---------------------------------------------------------------------------------------------

proptest! {
    #![proptest_config(ProptestConfig::with_cases(10_000))]

    /// 14. No panic of `to_plain` on arbitrary strings, whatever the settings.
    #[test]
    fn no_panic_of_to_plain_on_strings(
        text in any_text(), lang in 0usize..3, set in rule_set(), quotes in 0u8..3, percent in 0u8..4,
    ) {
        let t = typographer(lang, set, quotes, percent);
        let _ = t.to_plain(&text);
    }

    #[test]
    fn no_panic_of_to_plain_on_arbitrary_unicode(text in "\\PC*|.*", lang in 0usize..3) {
        let t = typographer(lang, RuleSet::ALL, 0, 0);
        let _ = t.to_plain(&text);
    }

    /// 14. No panic of `to_plain_template` on arbitrary pieces; slots keep their order and kinds.
    #[test]
    fn no_panic_of_to_plain_on_pieces(parts in pieces(), lang in 0usize..3, set in rule_set()) {
        let t = typographer(lang, set, 0, 0);
        let input = borrow(&parts);
        let out = t.to_plain_template(input);
        prop_assert_eq!(out.texts().len(), out.slots().len() + 1);
        let want: Vec<(u32, typolish::SlotKind)> = parts
            .iter()
            .filter_map(|p| match p {
                GenPiece::Slot(id, kind) => Some((*id, *kind)),
                _ => None,
            })
            .collect();
        let got: Vec<(u32, typolish::SlotKind)> =
            out.slots().iter().map(|(id, slot)| (*id, slot.kind())).collect();
        prop_assert_eq!(got, want);
    }
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(1_000_000))]

    /// 14, the long run: no panic of `to_plain` on arbitrary strings, whatever the settings.
    #[test]
    #[ignore = "long run of design section 13.5, property 14"]
    fn no_panic_of_to_plain_on_strings_long(
        text in any_text(), lang in 0usize..3, set in rule_set(), quotes in 0u8..3, percent in 0u8..4,
    ) {
        let t = typographer(lang, set, quotes, percent);
        let _ = t.to_plain(&text);
    }

    /// 14, the long run: no panic of `to_plain` on arbitrary Unicode.
    #[test]
    #[ignore = "long run of design section 13.5, property 14"]
    fn no_panic_of_to_plain_on_arbitrary_unicode_long(text in "\\PC*|.*", lang in 0usize..3) {
        let t = typographer(lang, RuleSet::ALL, 0, 0);
        let _ = t.to_plain(&text);
    }

    /// 14, the long run: no panic of `to_plain_template` on arbitrary pieces; slots keep their
    /// order and kinds.
    #[test]
    #[ignore = "long run of design section 13.5, property 14"]
    fn no_panic_of_to_plain_on_pieces_long(parts in pieces(), lang in 0usize..3, set in rule_set()) {
        let t = typographer(lang, set, 0, 0);
        let input = borrow(&parts);
        let out = t.to_plain_template(input);
        prop_assert_eq!(out.texts().len(), out.slots().len() + 1);
        let want: Vec<(u32, typolish::SlotKind)> = parts
            .iter()
            .filter_map(|p| match p {
                GenPiece::Slot(id, kind) => Some((*id, *kind)),
                _ => None,
            })
            .collect();
        let got: Vec<(u32, typolish::SlotKind)> =
            out.slots().iter().map(|(id, slot)| (*id, slot.kind())).collect();
        prop_assert_eq!(got, want);
    }
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(2_000))]

    /// 16. `to_plain` is stable: `to_plain(to_plain(x)) == to_plain(x)` for {ru, en} x {Prose,
    /// Interface}, and the result is `Borrowed` exactly when it equals the input. The input is a
    /// generated string and the typeset form of one.
    #[test]
    fn to_plain_is_stable(
        text in any_text(), lang in 0usize..2, interface in any::<bool>(), quotes in 0u8..3, percent in 0u8..4,
    ) {
        let set = if interface { RuleSet::INTERFACE } else { RuleSet::PROSE };
        let t = typographer(lang, set, quotes, percent);
        let typeset = t.typeset(&text, Output::Unicode).into_owned();
        for x in [text.as_str(), typeset.as_str()] {
            let once = t.to_plain(x);
            let twice = t.to_plain(&once);
            prop_assert_eq!(&twice, &once, "x = {}", show(x));
            prop_assert_eq!(
                matches!(once, Cow::Borrowed(_)),
                once == x,
                "Borrowed exactly when unchanged: {}",
                show(x)
            );
        }
    }

    /// 16. The same for `to_plain_template`: texts stable, slots equal.
    #[test]
    fn to_plain_template_is_stable(parts in pieces(), lang in 0usize..2, interface in any::<bool>()) {
        // Protected and markup pieces become plain text of the result, which a second pass would
        // read as text: the stability is that of `Text` and `Slot` pieces.
        let parts: Vec<GenPiece> = parts
            .into_iter()
            .filter(|p| matches!(p, GenPiece::Text(_) | GenPiece::Slot(..)))
            .collect();
        let set = if interface { RuleSet::INTERFACE } else { RuleSet::PROSE };
        let t = typographer(lang, set, 0, 0);
        let once = t.to_plain_template(borrow(&parts));
        let again_input: Vec<typolish::Piece<'_, u32>> = once
            .iter()
            .map(|part| match part {
                typolish::TemplatePart::Text(text) => typolish::Piece::Text(text),
                typolish::TemplatePart::Slot(id, slot) => typolish::Piece::Slot(*id, slot),
                _ => panic!("a part of a template that the test does not know"),
            })
            .collect();
        let twice = t.to_plain_template(again_input);
        prop_assert_eq!(twice.texts(), once.texts());
        prop_assert_eq!(twice.slots(), once.slots());
    }
}

/// 16 on the corpus: the plain lines and their typeset forms, with both sets.
#[test]
fn to_plain_is_stable_on_the_corpus() {
    use common::corpus::{
        FILES, LANGS, SETS, messages, text_lines, typographer as corpus_typographer,
    };
    for lang in LANGS {
        for (set_name, _) in SETS {
            let t = corpus_typographer(lang, set_name);
            let mut lines: Vec<String> = Vec::new();
            for file in FILES {
                if file.ends_with(".tmpl") {
                    lines.extend(messages(lang).iter().map(|m| m.flat()));
                } else {
                    lines.extend(text_lines(lang, file));
                }
            }
            for line in lines {
                let typeset = t.typeset(&line, Output::Unicode).into_owned();
                for x in [line.as_str(), typeset.as_str()] {
                    let once = t.to_plain(x);
                    let twice = t.to_plain(&once);
                    assert_eq!(twice, once, "{lang} {set_name}: {}", show(x));
                    assert_eq!(
                        matches!(once, Cow::Borrowed(_)),
                        once == x,
                        "{lang} {set_name}: Borrowed exactly when unchanged: {}",
                        show(x)
                    );
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------------------------
// Stage 4: `lint`, `lint_template` and the recorders (design sections 12.3 to 12.5; properties
// 9, 10, 14 and 15).
// ---------------------------------------------------------------------------------------------

type Key = (
    DiagnosticKind,
    Severity,
    Option<Rule>,
    usize,
    std::ops::Range<usize>,
);

fn key_of(d: &Diagnostic) -> Key {
    (
        d.kind().clone(),
        d.severity(),
        d.rule(),
        d.piece(),
        d.range(),
    )
}

fn is_char(d: &Diagnostic) -> bool {
    matches!(d.kind(), DiagnosticKind::TypographicChar { .. })
}

/// The diagnostics other than `TypographicChar`.
fn sans_chars(diagnostics: &[Diagnostic]) -> Vec<Key> {
    diagnostics
        .iter()
        .filter(|d| !is_char(d))
        .map(key_of)
        .collect()
}

/// The `TypographicChar` entries as `(piece, range, found)`.
fn char_entries(diagnostics: &[Diagnostic]) -> Vec<(usize, std::ops::Range<usize>, char)> {
    diagnostics
        .iter()
        .filter_map(|d| match d.kind() {
            DiagnosticKind::TypographicChar { found, .. } => Some((d.piece(), d.range(), *found)),
            _ => None,
        })
        .collect()
}

/// The place of a kind in the declaration of `DiagnosticKind`.
fn kind_order(kind: &DiagnosticKind) -> usize {
    match kind {
        DiagnosticKind::UnmatchedQuote => 0,
        DiagnosticKind::CrossingQuotes => 1,
        DiagnosticKind::AmbiguousQuote => 2,
        DiagnosticKind::UnclosedDelimiter => 3,
        DiagnosticKind::AmbiguousHyphen => 4,
        DiagnosticKind::MixedScript => 5,
        DiagnosticKind::SlotGlue => 6,
        DiagnosticKind::NowrapFallback => 7,
        DiagnosticKind::TypographicChar { .. } => 8,
        _ => 9,
    }
}

/// Diagnostics are ordered by piece, byte start, byte end, then kind (design section 12.4).
fn assert_ordered(diagnostics: &[Diagnostic]) -> Result<(), TestCaseError> {
    let key = |d: &Diagnostic| {
        let r = d.range();
        (d.piece(), r.start, r.end, kind_order(d.kind()))
    };
    for pair in diagnostics.windows(2) {
        prop_assert!(
            key(&pair[0]) <= key(&pair[1]),
            "out of order: {:?} before {:?}",
            pair[0],
            pair[1]
        );
    }
    Ok(())
}

/// Applies edits of one piece to its text: the edits are ordered by position, an insertion at a
/// point comes before an edit that starts there, and edits do not overlap.
fn apply_edits(text: &str, edits: &[&Change]) -> Result<String, TestCaseError> {
    let mut edits: Vec<(usize, usize, &str)> = edits
        .iter()
        .map(|c| (c.range().start, c.range().end, c.replacement()))
        .collect();
    edits.sort_by_key(|e| (e.0, e.1));
    let mut out = String::new();
    let mut at = 0;
    for (start, end, replacement) in edits {
        prop_assert!(start >= at, "edits overlap in {}", show(text));
        prop_assert!(text.get(at..start).is_some() && text.get(start..end).is_some());
        out.push_str(&text[at..start]);
        out.push_str(replacement);
        at = end;
    }
    out.push_str(&text[at..]);
    Ok(out)
}

/// The records of a report are ordered by piece and position (the rule order breaks ties).
fn assert_changes_ordered(changes: &[Change]) -> Result<(), TestCaseError> {
    for pair in changes.windows(2) {
        let a = (pair[0].piece(), pair[0].range().start);
        let b = (pair[1].piece(), pair[1].range().start);
        prop_assert!(a <= b, "changes out of order: {:?}", changes);
    }
    Ok(())
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(512))]

    /// 9. Recorder independence: `typeset`, `typeset_with` and `report` give the same result in
    /// every output kind; `lint(x)` without its `TypographicChar` entries equals the diagnostics
    /// of `report(x, Output::Unicode)`; applying the changes of `report` to `x` yields the result.
    #[test]
    fn recorders_are_independent(
        text in any_text(), lang in 0usize..3, set in rule_set(), quotes in 0u8..3, percent in 0u8..4,
    ) {
        let t = typographer(lang, set, quotes, percent);
        let mut scratch = Scratch::new();
        for output in outputs() {
            let plain = t.typeset(&text, output);
            let with = t.typeset_with(&mut scratch, &text, output);
            let report = t.report(&text, output);
            prop_assert_eq!(plain.as_ref(), with.as_ref(), "{:?}: {}", output, show(&text));
            prop_assert_eq!(plain.as_ref(), report.result().as_ref(), "{:?}: {}", output, show(&text));
            // The other records do not depend on the output kind either.
            let unicode = t.report(&text, Output::Unicode);
            prop_assert_eq!(report.changes(), unicode.changes());
            prop_assert_eq!(report.protected(), unicode.protected());
            prop_assert_eq!(report.diagnostics(), unicode.diagnostics());
        }
        let report = t.report(&text, Output::Unicode);
        let linted = t.lint(&text);
        prop_assert_eq!(sans_chars(&linted), sans_chars(report.diagnostics()), "{}", show(&text));
        prop_assert!(report.diagnostics().iter().all(|d| !is_char(d)));
        assert_ordered(&linted)?;
        assert_ordered(report.diagnostics())?;
        assert_changes_ordered(report.changes())?;
        let edits: Vec<&Change> = report.changes().iter().collect();
        for change in &edits {
            prop_assert_eq!(change.piece(), 0);
            prop_assert_eq!(report.original(change), &text[change.range()]);
        }
        let applied = apply_edits(&text, &edits)?;
        prop_assert_eq!(applied.as_str(), report.result().as_ref(), "{}", show(&text));
        // `lint` is a pure function of the text: a second call gives the same list.
        prop_assert_eq!(t.lint(&text), linted);
    }

    /// 9 and 10. The same for pieces: `report_template` gives what `typeset_template` gives in
    /// every output kind, `lint_template` without `TypographicChar` gives its diagnostics, and
    /// applying the changes to each input piece yields the texts of the result.
    #[test]
    fn template_recorders_are_independent(
        parts in pieces(), lang in 0usize..3, set in rule_set(), quotes in 0u8..3, percent in 0u8..4,
    ) {
        let t = typographer(lang, set, quotes, percent);
        for output in outputs() {
            let plain = t.typeset_template(borrow(&parts), output);
            let report = t.report_template(borrow(&parts), output);
            prop_assert_eq!(plain.texts(), report.result().texts(), "{:?}", output);
            prop_assert_eq!(plain.slots(), report.result().slots());
            prop_assert_eq!(plain.changed(), report.result().changed());
        }
        let report = t.report_template(borrow(&parts), Output::Unicode);
        let linted = t.lint_template(borrow(&parts));
        prop_assert_eq!(sans_chars(&linted), sans_chars(report.diagnostics()), "{:?}", parts);
        prop_assert!(report.diagnostics().iter().all(|d| !is_char(d)));
        assert_ordered(&linted)?;
        assert_ordered(report.diagnostics())?;
        assert_changes_ordered(report.changes())?;
        assert_changes_apply_piece_by_piece(&parts, &report)?;
        // `lint_template` never gives `NowrapFallback` and its entries sit in `Text` pieces.
        for d in &linted {
            prop_assert!(d.piece() < parts.len().max(1));
            prop_assert!(d.kind() != &DiagnosticKind::NowrapFallback);
            if is_char(d) {
                prop_assert!(matches!(parts[d.piece()], GenPiece::Text(_)), "{:?}", d);
            }
        }
    }
}

/// Property 10: applying `report_template(...).changes()` piece by piece yields the texts of the
/// result in the `n + 1` form.
fn assert_changes_apply_piece_by_piece(
    parts: &[GenPiece],
    report: &Report<'_, Template<'_, u32>>,
) -> Result<(), TestCaseError> {
    let mut texts = vec![String::new()];
    for (i, part) in parts.iter().enumerate() {
        let edits: Vec<&Change> = report.changes().iter().filter(|c| c.piece() == i).collect();
        match part {
            GenPiece::Text(text) => {
                for c in &edits {
                    prop_assert_eq!(report.original(c), &text[c.range()]);
                }
                let applied = apply_edits(text, &edits)?;
                if let Some(last) = texts.last_mut() {
                    last.push_str(&applied);
                }
            }
            GenPiece::Protected(text) | GenPiece::Markup(text) | GenPiece::Block(text) => {
                prop_assert!(
                    edits.is_empty(),
                    "an edit of piece {} that no rule may touch",
                    i
                );
                if let Some(last) = texts.last_mut() {
                    last.push_str(text);
                }
            }
            GenPiece::Slot(..) => {
                prop_assert!(edits.is_empty(), "an edit of the slot at piece {}", i);
                texts.push(String::new());
            }
        }
    }
    let got: Vec<&str> = report.result().texts().iter().map(|t| t.as_ref()).collect();
    let want: Vec<&str> = texts.iter().map(String::as_str).collect();
    prop_assert_eq!(got, want);
    Ok(())
}

/// A text of the n + 1 form of the input pieces: the pieces between two slots merged.
fn merged_texts(parts: &[GenPiece]) -> Vec<String> {
    let mut texts = vec![String::new()];
    for part in parts {
        match part {
            GenPiece::Text(t)
            | GenPiece::Protected(t)
            | GenPiece::Markup(t)
            | GenPiece::Block(t) => {
                if let Some(last) = texts.last_mut() {
                    last.push_str(t);
                }
            }
            GenPiece::Slot(..) => texts.push(String::new()),
        }
    }
    texts
}

/// Characters of `skeleton` appear in `whole` in order (a subsequence).
fn is_subsequence(skeleton: &str, whole: &str) -> bool {
    let mut rest = whole.chars();
    skeleton.chars().all(|c| rest.any(|w| w == c))
}

/// Property 15: the entries of `TypographicChar` of one string: each covers exactly the character it
/// names, they are ordered and disjoint, and the characters that no entry covers stay in the
/// plain form in order.
fn assert_entries_cover_what_to_plain_changes(
    t: &Typographer,
    x: &str,
) -> Result<(), TestCaseError> {
    let linted = t.lint(x);
    let entries = char_entries(&linted);
    let plain = t.to_plain(x);
    prop_assert_eq!(
        !entries.is_empty(),
        plain.as_ref() != x,
        "lint and to_plain disagree on {}",
        show(x)
    );
    let mut skeleton = String::new();
    let mut at = 0;
    for (piece, range, found) in &entries {
        prop_assert_eq!(*piece, 0);
        prop_assert!(range.start >= at, "entries overlap in {}", show(x));
        let named = found.to_string();
        prop_assert_eq!(x.get(range.clone()), Some(named.as_str()));
        prop_assert!(
            !found.is_ascii() && !found.is_alphanumeric(),
            "found {:?}",
            found
        );
        skeleton.push_str(&x[at..range.start]);
        at = range.end;
    }
    skeleton.push_str(&x[at..]);
    prop_assert!(
        is_subsequence(&skeleton, &plain),
        "a character that no entry covers changed: {} -> {}",
        show(x),
        show(&plain)
    );
    Ok(())
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(2_000))]

    /// 15. `lint(x)` contains a `TypographicChar` exactly when `to_plain(x) != x`, and each such
    /// entry covers one character that `to_plain` changes. The input is a generated string and
    /// the typeset form of one, for every language and rule set.
    #[test]
    fn lint_matches_to_plain(
        text in any_text(), lang in 0usize..3, set in rule_set(), quotes in 0u8..3, percent in 0u8..4,
    ) {
        let t = typographer(lang, set, quotes, percent);
        let typeset = t.typeset(&text, Output::Unicode).into_owned();
        for x in [text.as_str(), typeset.as_str()] {
            assert_entries_cover_what_to_plain_changes(&t, x)?;
        }
    }

    /// 15 on risky text in the built-in sets of the plain form: {ru, en} x {Prose, Interface}.
    #[test]
    fn lint_matches_to_plain_in_the_built_in_sets(
        text in risky_text(), lang in 0usize..2, interface in any::<bool>(), quotes in 0u8..3, percent in 0u8..4,
    ) {
        let set = if interface { RuleSet::INTERFACE } else { RuleSet::PROSE };
        let t = typographer(lang, set, quotes, percent);
        let typeset = t.typeset(&text, Output::Unicode).into_owned();
        for x in [text.as_str(), typeset.as_str()] {
            assert_entries_cover_what_to_plain_changes(&t, x)?;
        }
        // The plain form of a typeset text has no entry: `to_plain` is stable (property 16).
        prop_assert!(char_entries(&t.lint(&t.to_plain(&typeset))).is_empty());
    }

    /// 15 for pieces: `lint_template` has a `TypographicChar` exactly when `to_plain_template`
    /// changes a text, and each entry covers its character in a `Text` piece.
    #[test]
    fn lint_template_matches_to_plain_template(parts in pieces(), lang in 0usize..3, set in rule_set()) {
        let t = typographer(lang, set, 0, 0);
        let linted = t.lint_template(borrow(&parts));
        let entries = char_entries(&linted);
        let plain = t.to_plain_template(borrow(&parts));
        let before = merged_texts(&parts);
        let after: Vec<&str> = plain.texts().iter().map(|c| c.as_ref()).collect();
        let before: Vec<&str> = before.iter().map(String::as_str).collect();
        prop_assert_eq!(!entries.is_empty(), after != before, "{:?}", parts);
        for (piece, range, found) in &entries {
            match &parts[*piece] {
                GenPiece::Text(text) => {
                    let named = found.to_string();
                    prop_assert_eq!(text.get(range.clone()), Some(named.as_str()));
                }
                other => prop_assert!(false, "an entry in {:?}", other),
            }
        }
    }

    /// 10 for `lint_template`: a template of `Text` pieces cut anywhere gives the diagnostics of
    /// the whole string (the piece and the range of each are those of its first piece).
    #[test]
    fn lint_of_cut_text_pieces_matches_the_whole(
        text in risky_text(), lang in 0usize..3, set in rule_set(), cut in any::<prop::sample::Index>(),
    ) {
        let t = typographer(lang, set, 0, 0);
        let boundaries: Vec<usize> = text.char_indices().map(|(i, _)| i).chain([text.len()]).collect();
        let at = boundaries[cut.index(boundaries.len())];
        let (a, b) = text.split_at(at);
        let whole = t.lint(&text);
        let parts = t.lint_template::<()>([Piece::Text(a), Piece::Text(b)]);
        let mut whole_kinds: Vec<String> = whole.iter().map(|d| format!("{:?} {:?}", d.kind(), d.severity())).collect();
        let mut part_kinds: Vec<String> = parts.iter().map(|d| format!("{:?} {:?}", d.kind(), d.severity())).collect();
        whole_kinds.sort();
        part_kinds.sort();
        prop_assert_eq!(part_kinds, whole_kinds, "cut {} of {}", at, show(&text));
        // Every range lies in its piece.
        for d in &parts {
            let piece = [a, b][d.piece().min(1)];
            prop_assert!(d.piece() < 2);
            prop_assert!(piece.get(d.range()).is_some(), "{:?}", d);
        }
    }
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(10_000))]

    /// 14. No panic of `lint` on arbitrary strings, whatever the settings.
    #[test]
    fn no_panic_of_lint_on_strings(
        text in any_text(), lang in 0usize..3, set in rule_set(), quotes in 0u8..3, percent in 0u8..4,
    ) {
        let t = typographer(lang, set, quotes, percent);
        let linted = t.lint(&text);
        for d in &linted {
            prop_assert_eq!(d.piece(), 0);
            prop_assert!(text.get(d.range()).is_some(), "{:?} in {}", d, show(&text));
        }
    }

    #[test]
    fn no_panic_of_lint_on_arbitrary_unicode(text in "\\PC*|.*", lang in 0usize..3) {
        let t = typographer(lang, RuleSet::ALL, 0, 0);
        let _ = t.lint(&text);
    }

    /// 14. No panic of `lint_template` on arbitrary pieces; every diagnostic names a piece and
    /// a range inside it.
    #[test]
    fn no_panic_of_lint_on_pieces(parts in pieces(), lang in 0usize..3, set in rule_set()) {
        let t = typographer(lang, set, 0, 0);
        assert_lint_template_is_well_formed(&t, &parts)?;
    }
}

fn assert_lint_template_is_well_formed(
    t: &Typographer,
    parts: &[GenPiece],
) -> Result<(), TestCaseError> {
    let linted = t.lint_template(borrow(parts));
    for d in &linted {
        prop_assert!(d.piece() < parts.len().max(1), "{:?}", d);
        let range = d.range();
        match parts.get(d.piece()) {
            Some(
                GenPiece::Text(text)
                | GenPiece::Protected(text)
                | GenPiece::Markup(text)
                | GenPiece::Block(text),
            ) => {
                prop_assert!(text.get(range).is_some(), "{:?}", d);
            }
            Some(GenPiece::Slot(..)) => prop_assert_eq!(range, 0..0),
            None => prop_assert_eq!(range, 0..0),
        }
    }
    Ok(())
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(1_000_000))]

    /// 14, the long run: no panic of `lint` on arbitrary strings, whatever the settings.
    #[test]
    #[ignore = "long run of design section 13.5, property 14"]
    fn no_panic_of_lint_on_strings_long(
        text in any_text(), lang in 0usize..3, set in rule_set(), quotes in 0u8..3, percent in 0u8..4,
    ) {
        let t = typographer(lang, set, quotes, percent);
        let _ = t.lint(&text);
    }

    /// 14, the long run: no panic of `lint` on arbitrary Unicode.
    #[test]
    #[ignore = "long run of design section 13.5, property 14"]
    fn no_panic_of_lint_on_arbitrary_unicode_long(text in "\\PC*|.*", lang in 0usize..3) {
        let t = typographer(lang, RuleSet::ALL, 0, 0);
        let _ = t.lint(&text);
    }

    /// 14, the long run: no panic of `lint_template` on arbitrary pieces.
    #[test]
    #[ignore = "long run of design section 13.5, property 14"]
    fn no_panic_of_lint_on_pieces_long(parts in pieces(), lang in 0usize..3, set in rule_set()) {
        let t = typographer(lang, set, 0, 0);
        assert_lint_template_is_well_formed(&t, &parts)?;
    }
}

// ---------------------------------------------------------------------------------------------
// The structured grammar (design section 13.5, properties 1 to 3)
// ---------------------------------------------------------------------------------------------

/// The seed of the default run of the grammar properties; the language index is added to it.
const GRAMMAR_SEED: u64 = 0x7970_6f6c_6973_6801;
/// The seed of the long run.
const GRAMMAR_SEED_LONG: u64 = 0x7970_6f6c_6973_6802;

/// Whether a change inserts or removes a space: a split or a merge of design section 4.3 (I2),
/// after which the fragments of the two passes may differ.
fn structural(report: &Report<'_>, change: &Change) -> bool {
    let spaces = |s: &str| {
        s.chars()
            .filter(|c| matches!(c, ' ' | '\u{a0}' | '\u{202f}'))
            .count()
    };
    spaces(report.original(change)) != spaces(change.replacement())
}

/// Idempotence (1), the fixed point (2) and the invariants I1 to I5 (3) of design section 13.5 for
/// one sentence of the grammar, as far as the public API shows them: the second pass changes
/// nothing; a change that is no symbol or times rule inserts a space or glue, removes one space, or
/// replaces no letter and no digit (I1); the kinds of the protected spans of both passes are equal
/// where no change inserted or removed a space (I2); every change starts from a plain form (I3);
/// the report of the second pass holds no change (I4); and a window without a line break that does
/// not converge changes no straight `"` and a straight `'` only into `’` or `′` (I5). A sentence of
/// a known limit is skipped (`grammar_known_limit`).
fn grammar_properties(config: &GrammarConfig, text: &str) -> Result<(), TestCaseError> {
    if grammar_known_limit(config.lang, text).is_some() {
        return Ok(());
    }
    let t = config.typographer();
    let report = t.report(text, Output::Unicode);
    let once = report.result().to_string();
    let second = t.report(&once, Output::Unicode);
    prop_assert!(
        second.result() == &once && second.changes().is_empty(),
        "idempotence and I4: {:?}\nfirst pass:\n{}\nsecond pass:\n{}",
        text,
        report,
        second
    );
    let mut split = false;
    for change in report.changes() {
        let original = report.original(change);
        let replacement = change.replacement();
        let rule = change.rule();
        split |= structural(&report, change);
        if !rule.id().starts_with("symbols.") && rule != Rule::NumberTimes {
            let kept = if original.is_empty() {
                is_glue_or_space(replacement)
            } else if replacement.is_empty() {
                original == " "
            } else {
                !original.chars().any(char::is_alphanumeric)
                    && !replacement.chars().any(char::is_alphanumeric)
            };
            prop_assert!(
                kept,
                "I1: {rule:?} {original:?} -> {replacement:?} in {text:?}"
            );
        }
        for c in original.chars() {
            prop_assert!(
                !TYPOGRAPHIC.contains(&c) && (c.is_ascii() || matches!(c, 'с' | 'С' | 'х' | 'Х')),
                "I3: {rule:?} changed {original:?} in {text:?}"
            );
        }
    }
    if !split {
        prop_assert_eq!(
            kinds(&report),
            kinds(&second),
            "I2: {:?} -> {:?}",
            text,
            once
        );
    }
    let blocked = report.diagnostics().iter().any(|d| {
        (d.kind() == &DiagnosticKind::UnmatchedQuote && d.severity() == Severity::Warning)
            || d.kind() == &DiagnosticKind::CrossingQuotes
    });
    if blocked && !text.contains(['\n', '\r', '\u{85}', '\u{2028}', '\u{2029}']) {
        for change in report.changes() {
            let original = report.original(change);
            prop_assert!(
                original != "\"" && (original != "'" || matches!(change.replacement(), "’" | "′")),
                "I5: {:?} changed {:?} in {:?} -> {:?}",
                change.rule(),
                original,
                text,
                once
            );
        }
    }
    Ok(())
}

#[test]
fn grammar_properties_hold() {
    // Design section 13.5: properties 1 to 3 over the structured grammar, in `ru-RU`, `en-US`,
    // `COMMON` and the two declared languages of `grammar_language`, with fixed seeds.
    for lang in 0..GRAMMAR_LANGUAGES {
        run_grammar(lang, 512, GRAMMAR_SEED + lang as u64, grammar_properties);
    }
}

#[test]
#[ignore = "long run of design section 13.5, properties 1 to 3 over the grammar"]
fn grammar_properties_hold_long() {
    std::thread::scope(|scope| {
        for lang in 0..GRAMMAR_LANGUAGES {
            scope.spawn(move || {
                run_grammar(
                    lang,
                    1_000_000,
                    GRAMMAR_SEED_LONG + lang as u64,
                    grammar_properties,
                );
            });
        }
    });
}

#[test]
fn the_grammar_filters_name_their_limits() {
    // The filters of `grammar_known_limit` match the inputs of their limits (design section 10.8)
    // and leave the neighbors of those inputs to the properties.
    for (lang, text, limit) in [
        (0, "''-''", true),
        (1, "'('')'", true),
        (0, "x ''—'' y", true),
        (0, "\"--\"\"\"", false),
        (0, "\"\"\"", false),
        (0, "\"a\" - \"b\"", false),
        (1, "‘s' \"0\"’", true),
        (3, "‘S' ‘s'’ s’", true),
        (0, "‘s' \"0\"’", false),
        (1, "dogs' ‘a’", false),
        (1, "x ‘a 'b' \"2\"’ y", true),
        (1, "\"a '-’ - x' c\"", true),
        (3, "\"-’ - x\"", true),
        (1, "\"a '-” - x' c\"", false),
        (0, "\"a '-’ - x' c\"", false),
    ] {
        assert_eq!(
            grammar_known_limit(lang, text).is_some(),
            limit,
            "{lang} {text:?}"
        );
    }
    // A known limit that the filter skips still fails (when it does not, the limit is fixed and its
    // filter goes, with its entry in design section 10.8).
    let gb = GrammarConfig {
        lang: 3,
        set: RuleSet::PROSE,
        dash: None,
        single: SingleQuotes::NestedOnly,
        percent: PercentSpacing::AsWritten,
    }
    .typographer();
    let once = gb.typeset("‘s' \"0\"’", Output::Unicode).into_owned();
    assert_ne!(gb.typeset(&once, Output::Unicode), once);
}
