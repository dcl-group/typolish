//! `LanguageBuilder`: one method per field, normalization and validation (design sections 7.1
//! and 7.6, `tests/language_builder.rs`).
//!
//! The design names `quotes`, `dash_style`, `short_words` and `units` and says "one method per
//! field"; the other methods are read as the field names of design section 7.1 (`elisions`,
//! `decade_apostrophe`, `trailing_possessive`, `ellipsis`, `short_word_length`, `particles`,
//! `atom_exceptions`, `ambiguous_units`, `months`, `month_position`, `year_words`,
//! `century_words`, `range_lead_words`, `signs`, `numbers`, `script`, `cldr_version`). The test
//! target stands apart so that a different spelling of one of them breaks this file only.

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

use common::check_exact as check_in;
use typolish::{
    DashStyle, EllipsisForms, Error, Language, LanguageBuilder, LanguageData, MonthPosition,
    NumberData, Output, QuotePairs, Rule, RuleSet, Script, Typographer,
};

fn leak(data: LanguageData) -> &'static LanguageData {
    Box::leak(Box::new(data))
}

#[track_caller]
fn invalid(result: Result<Language, Error>, what: &str) {
    match result {
        Err(Error::InvalidLanguageData { .. }) => {}
        other => panic!("{what}: expected InvalidLanguageData, got {other:?}"),
    }
}

#[test]
fn a_copy_without_changes_equals_the_original() {
    for base in [Language::RU_RU, Language::EN_US, Language::COMMON] {
        let copy = LanguageBuilder::from_language(&base, base.tag())
            .build()
            .unwrap();
        assert_eq!(copy, base, "{base}");
    }
}

#[test]
fn new_starts_from_the_values_of_language_data_new() {
    let built = LanguageBuilder::new("xx").build().unwrap();
    let data = Language::from_data(leak(LanguageData::new("xx"))).unwrap();
    assert_eq!(built, data);
}

#[test]
fn word_lists_are_lowercased_sorted_and_deduplicated() {
    let built = LanguageBuilder::new("xx")
        .short_words(["Б", "а", "а", "В", "б"])
        .build()
        .unwrap();
    let data = LanguageData::new("xx").with_short_words(&["а", "б", "в"]);
    assert_eq!(built, Language::from_data(leak(data)).unwrap());
    // Every word list of design section 7.6, not `months`, `abbreviations`, `units` or
    // `ambiguous_units`.
    let built = LanguageBuilder::new("xx")
        .elisions(["'Tis", "'em", "'em"])
        .particles(["ЖЕ", "бы"])
        .atom_exceptions(["I.E.", "e.g."])
        .year_words(["Г.", "гг."])
        .century_words(["В.", "вв."])
        .range_lead_words(["Стр.", "с."])
        .build()
        .unwrap();
    let data = LanguageData::new("xx")
        .with_elisions(&["'em", "'tis"])
        .with_particles(&["бы", "же"])
        .with_atom_exceptions(&["e.g.", "i.e."])
        .with_year_words(&["г.", "гг."])
        .with_century_words(&["в.", "вв."])
        .with_range_lead_words(&["с.", "стр."]);
    assert_eq!(built, Language::from_data(leak(data)).unwrap());
}

#[test]
fn units_months_and_ambiguous_units_keep_their_case_but_are_sorted() {
    let built = LanguageBuilder::new("xx")
        .units(["МБ", "mb", "MB", "МБ", "В"])
        .ambiguous_units(["В"])
        .months(["May", "Jan", "May", "апреля"])
        .build()
        .unwrap();
    let data = LanguageData::new("xx")
        .with_units(&["MB", "mb", "В", "МБ"])
        .with_ambiguous_units(&["В"])
        .with_months(&["Jan", "May", "апреля"]);
    assert_eq!(built, Language::from_data(leak(data)).unwrap());
}

#[test]
fn lists_accept_any_iterator_of_string_likes() {
    let owned: Vec<String> = vec!["б".to_owned(), "а".to_owned()];
    let a = LanguageBuilder::new("xx")
        .short_words(owned.clone())
        .build()
        .unwrap();
    let b = LanguageBuilder::new("xx")
        .short_words(owned.iter().map(String::as_str))
        .build()
        .unwrap();
    let c = LanguageBuilder::new("xx")
        .short_words(["а", "б"])
        .build()
        .unwrap();
    let d = LanguageBuilder::new("xx")
        .short_words(&["а", "б"][..])
        .build()
        .unwrap();
    assert_eq!(a, b);
    assert_eq!(b, c);
    assert_eq!(c, d);
}

#[test]
fn every_field_has_a_method() {
    let built = LanguageBuilder::new("xx-YY")
        .cldr_version("49")
        .script(Script::Latin)
        .quotes(Some(QuotePairs::new(('«', '»'), ('‹', '›'))))
        .elisions(["'x"])
        .decade_apostrophe(true)
        .trailing_possessive(true)
        .dash_style(DashStyle::SpacedGlued)
        .ellipsis(EllipsisForms::new("?.", "!.", "?!."))
        .short_words(["a"])
        .short_word_length(3)
        .particles(["p"])
        .atom_exceptions(["e.g."])
        .units(["u", "v"])
        .ambiguous_units(["v"])
        .months(["May"])
        .month_position(MonthPosition::BeforeDay)
        .year_words(["yr."])
        .century_words(["c."])
        .range_lead_words(["p."])
        .signs(['§'])
        .numbers(
            NumberData::new()
                .with_decimal(',')
                .with_group('.')
                .with_min_grouping_digits(2),
        )
        .build()
        .unwrap();
    let data = LanguageData::new("xx-YY")
        .with_cldr_version("49")
        .with_script(Script::Latin)
        .with_quotes(Some(QuotePairs::new(('«', '»'), ('‹', '›'))))
        .with_elisions(&["'x"])
        .with_decade_apostrophe(true)
        .with_trailing_possessive(true)
        .with_dash_style(DashStyle::SpacedGlued)
        .with_ellipsis(EllipsisForms::new("?.", "!.", "?!."))
        .with_short_words(&["a"])
        .with_short_word_length(3)
        .with_particles(&["p"])
        .with_atom_exceptions(&["e.g."])
        .with_units(&["u", "v"])
        .with_ambiguous_units(&["v"])
        .with_months(&["May"])
        .with_month_position(MonthPosition::BeforeDay)
        .with_year_words(&["yr."])
        .with_century_words(&["c."])
        .with_range_lead_words(&["p."])
        .with_signs(&['§'])
        .with_numbers(
            NumberData::new()
                .with_decimal(',')
                .with_group('.')
                .with_min_grouping_digits(2),
        );
    assert_eq!(built, Language::from_data(leak(data)).unwrap());
    assert_eq!(built.tag(), "xx-YY");
}

#[test]
fn dash_style_creates_dash_data_as_with_dash_style_does() {
    let built = LanguageBuilder::new("xx")
        .dash_style(DashStyle::Spaced)
        .build()
        .unwrap();
    let data = LanguageData::new("xx").with_dash_style(DashStyle::Spaced);
    assert_eq!(built, Language::from_data(leak(data)).unwrap());
    // On a table with dash data the dialogue flag stays (ru has `dialogue: true`).
    let built = LanguageBuilder::from_language(&Language::RU_RU, "ru-KZ")
        .dash_style(DashStyle::Closed)
        .build()
        .unwrap();
    let data = LanguageData::RU_RU
        .with_tag("ru-KZ")
        .with_dash_style(DashStyle::Closed);
    assert_eq!(built, Language::from_data(leak(data)).unwrap());
}

#[test]
fn build_validates_like_from_data() {
    invalid(LanguageBuilder::new("").build(), "empty tag");
    invalid(LanguageBuilder::new("english").build(), "tag");
    invalid(
        LanguageBuilder::new("xx")
            .quotes(Some(QuotePairs::new(('「', '」'), ('«', '»'))))
            .build(),
        "quote glyphs",
    );
    invalid(
        LanguageBuilder::new("xx")
            .quotes(Some(QuotePairs::new(('"', '"'), ('«', '»'))))
            .build(),
        "straight quotes",
    );
    invalid(
        LanguageBuilder::new("xx")
            .quotes(Some(QuotePairs::new(('«', '»'), ('«', '„'))))
            .build(),
        "duplicate glyphs",
    );
    // Normalization does not repair an empty entry or one with whitespace.
    invalid(
        LanguageBuilder::new("xx").short_words([""]).build(),
        "empty",
    );
    invalid(
        LanguageBuilder::new("xx").short_words(["a b"]).build(),
        "whitespace",
    );
    invalid(
        LanguageBuilder::new("xx").units(["5px"]).build(),
        "unit with a digit",
    );
    invalid(
        LanguageBuilder::new("xx").ambiguous_units(["x"]).build(),
        "ambiguous unit that is not a unit",
    );
    invalid(
        LanguageBuilder::new("xx")
            .units(["с"])
            .short_words(["с"])
            .build(),
        "a unit that is a short word",
    );
    LanguageBuilder::new("xx")
        .units(["с"])
        .short_words(["с"])
        .ambiguous_units(["с"])
        .build()
        .unwrap();
    invalid(
        LanguageBuilder::new("xx")
            .numbers(NumberData::new().with_min_grouping_digits(5))
            .build(),
        "min_grouping_digits",
    );
    invalid(
        LanguageBuilder::new("xx")
            .numbers(NumberData::new().with_group(' '))
            .build(),
        "group",
    );
    invalid(
        LanguageBuilder::new("xx")
            .numbers(NumberData::new().with_range_dash('a'))
            .build(),
        "range dash that is a letter",
    );
    invalid(
        LanguageBuilder::new("xx")
            .numbers(NumberData::new().with_decimal(',').with_group(','))
            .build(),
        "decimal == group",
    );
    // Design section 7.6: a decimal separator that would hide the number or read as a range.
    for (decimal, what) in [
        ('\u{ad}', "decimal that is a soft hyphen"),
        ('\u{2013}', "decimal equal to the range dash"),
        ('-', "decimal that is a hyphen-minus"),
        ('\u{2212}', "decimal equal to the minus"),
        ('\u{206a}', "decimal that is an invisible format character"),
        ('\u{13430}', "decimal that is a hieroglyph format control"),
        ('\u{ff0d}', "decimal that is a fullwidth hyphen-minus"),
        ('\u{2053}', "decimal that is a swung dash"),
        ('\'', "decimal that the quote rules make a prime"),
    ] {
        invalid(
            LanguageBuilder::from_language(&Language::RU_RU, "xx")
                .numbers(NumberData::new().with_decimal(decimal))
                .build(),
            what,
        );
    }
    invalid(
        LanguageBuilder::new("xx").signs(['a']).build(),
        "sign that is a letter",
    );
    match LanguageBuilder::new("xx").signs(['€']).build() {
        Err(Error::InvalidLanguageData { reason, .. }) => assert!(reason.contains("signs")),
        other => panic!("a sign that is no punctuation: {other:?}"),
    }
    invalid(
        LanguageBuilder::new("xx")
            .ellipsis(EllipsisForms::new("?x", "!…", "?!…"))
            .build(),
        "ellipsis",
    );
}

#[test]
fn the_error_names_the_field() {
    for (builder, field) in [
        (
            LanguageBuilder::new("xx").short_words(["a b"]),
            "short_words",
        ),
        (LanguageBuilder::new("xx").units(["5px"]), "units"),
        (
            LanguageBuilder::new("xx").ambiguous_units(["x"]),
            "ambiguous_units",
        ),
        (
            LanguageBuilder::new("xx").quotes(Some(QuotePairs::new(('「', '」'), ('«', '»')))),
            "quotes",
        ),
    ] {
        match builder.build() {
            Err(Error::InvalidLanguageData { reason, .. }) => {
                assert!(reason.contains(field), "{field}: {reason}")
            }
            other => panic!("{field}: {other:?}"),
        }
    }
}

#[test]
fn a_built_language_replaces_the_list_it_started_from() {
    // `from_language` keeps every field of the base; a list method replaces that list.
    let lang = LanguageBuilder::from_language(&Language::EN_US, "en-XX")
        .short_words(["foo"])
        .build()
        .unwrap();
    let t = Typographer::new(lang, RuleSet::EMPTY.with(Rule::NbspShortWords));
    check_in("custom", &t, ("foo bar", "foo⍽bar"));
    check_in("custom", &t, ("the cat", "the cat"));
    // The other fields are those of en-US: the same language in a set.
    let lang = LanguageBuilder::from_language(&Language::EN_US, "en-XX")
        .short_words(["foo"])
        .build()
        .unwrap();
    let t = Typographer::new(lang, RuleSet::PROSE);
    check_in("custom", &t, ("He said \"hi\" - bye", "He said “hi” — bye"));
    check_in("custom", &t, ("5 MB", "5⍽MB"));
}

#[test]
fn the_design_example_builds_british_english() {
    // Design section 7.7: `LanguageBuilder::from_language(&Language::EN_US, "en-GB")
    //     .dash_style(DashStyle::Spaced).build()`.
    let gb = LanguageBuilder::from_language(&Language::EN_US, "en-GB")
        .dash_style(DashStyle::Spaced)
        .build()
        .unwrap();
    assert_eq!(gb.tag(), "en-GB");
    let t = Typographer::new(gb.clone(), RuleSet::PROSE);
    assert_eq!(
        t.typeset("Moscow - the capital", Output::Unicode),
        "Moscow — the\u{a0}capital"
    );
    // The same language as a `const` table.
    const EN_GB: LanguageData = LanguageData::EN_US
        .with_tag("en-GB")
        .with_dash_style(DashStyle::Spaced);
    assert_eq!(gb, Language::from_data(leak(EN_GB)).unwrap());
}

#[test]
fn a_built_language_has_its_own_tag_and_is_not_resolved_by_from_tag() {
    let kz = LanguageBuilder::from_language(&Language::RU_RU, "ru-KZ")
        .build()
        .unwrap();
    assert_eq!(kz.to_string(), "ru-KZ");
    assert!(matches!(
        "ru-KZ".parse::<Language>(),
        Err(Error::UnknownLanguage { .. })
    ));
}

#[test]
fn the_builder_is_clone_and_debug() {
    let b = LanguageBuilder::new("xx").short_words(["а"]);
    let c = b.clone();
    // `Debug` shows the builder's data: the clone prints the same, and the tag is there.
    assert_eq!(format!("{b:?}"), format!("{c:?}"));
    assert!(format!("{b:?}").contains("\"xx\""), "{b:?}");
    assert_eq!(b.build().unwrap(), c.build().unwrap());
}

#[test]
fn a_custom_language_with_a_month_position_and_units_glues_by_its_data() {
    // The ambiguous units of en-US (`in`) must stay units, or be replaced too (design section 7.6).
    let lang = LanguageBuilder::from_language(&Language::EN_US, "en-XB")
        .month_position(MonthPosition::BeforeDay)
        .units(["zz"])
        .ambiguous_units(Vec::<&str>::new())
        .build()
        .unwrap();
    let t = Typographer::new(
        lang,
        RuleSet::EMPTY.with(Rule::NbspDates).with(Rule::NbspUnits),
    );
    check_in("custom", &t, ("November 21", "November⍽21"));
    check_in("custom", &t, ("21 November", "21 November"));
    check_in("custom", &t, ("5 zz", "5⍽zz"));
    check_in("custom", &t, ("5 MB", "5 MB"));
}
