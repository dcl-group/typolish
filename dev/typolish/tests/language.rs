//! `Language` and the activity of rules per language (design sections 2.2, 7.4 and 7.7).

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    clippy::print_stdout,
    clippy::print_stderr,
    clippy::dbg_macro
)]

use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

use typolish::{DashStyle, Error, Language, LanguageData, QuotePairs, Rule, RuleSet, Typographer};

fn hash_of<T: Hash>(value: &T) -> u64 {
    let mut hasher = DefaultHasher::new();
    value.hash(&mut hasher);
    hasher.finish()
}

fn ids(set: RuleSet) -> Vec<&'static str> {
    set.iter().map(|r| r.id()).collect()
}

#[test]
fn tags() {
    assert_eq!(Language::RU_RU.tag(), "ru-RU");
    assert_eq!(Language::EN_US.tag(), "en-US");
    assert_eq!(Language::COMMON.tag(), "und");
    assert_eq!(Language::RU_RU.to_string(), "ru-RU");
    assert_eq!(Language::EN_US.to_string(), "en-US");
    assert_eq!(Language::COMMON.to_string(), "und");
}

#[test]
fn display_of_an_unvalidated_tag_stays_on_one_line() {
    // Design section 2.11: the `Display` of a language escapes invisible characters.
    static BAD: LanguageData = LanguageData::new("x\r\nFAKE\u{2028}y");
    let lang = Language::from_static(&BAD);
    assert_eq!(lang.to_string(), "x\\u{d}\\u{a}FAKE\\u{2028}y");
    assert_eq!(lang.tag(), "x\r\nFAKE\u{2028}y");
}

#[test]
fn builtin_languages_are_in_tag_order_without_common() {
    let tags: Vec<String> = Language::builtin().map(|l| l.tag().to_owned()).collect();
    assert_eq!(tags, ["en-US", "ru-RU"]);
}

#[test]
fn from_tag_is_ascii_case_insensitive_and_reads_underscore() {
    for tag in [
        "ru-RU", "ru-ru", "RU-RU", "Ru-rU", "ru_RU", "ru_ru", "ru", "RU", "Ru",
    ] {
        assert_eq!(Language::from_tag(tag), Ok(Language::RU_RU), "{tag}");
        assert_eq!(tag.parse::<Language>(), Ok(Language::RU_RU), "{tag}");
    }
    for tag in ["en-US", "en-us", "EN-US", "en_US", "en", "EN"] {
        assert_eq!(Language::from_tag(tag), Ok(Language::EN_US), "{tag}");
    }
    for tag in ["und", "UND", "Und"] {
        assert_eq!(Language::from_tag(tag), Ok(Language::COMMON), "{tag}");
    }
}

#[test]
fn regions_are_never_substituted() {
    for tag in [
        "en-GB",
        "en-AU",
        "ru-UA",
        "ru-KZ",
        "de",
        "de-DE",
        "fr",
        "",
        " ",
        "ru ",
        " ru",
        "r",
        "rus",
        "ru-",
        "-RU",
        "ru-R",
        "ru-RUS",
        "ru-RU-x",
        "ru-419",
        "und-RU",
        "ru--RU",
        "\u{0}",
        "русский",
        "en-US-u-ca-gregory",
    ] {
        assert!(
            matches!(Language::from_tag(tag), Err(Error::UnknownLanguage { tag: ref t, .. }) if t == tag),
            "{tag:?}"
        );
        assert!(tag.parse::<Language>().is_err(), "{tag:?}");
    }
}

#[test]
fn an_unknown_tag_can_fall_back_to_common() {
    let lang = Language::from_tag("de-DE").unwrap_or(Language::COMMON);
    assert_eq!(lang, Language::COMMON);
    assert_eq!(lang.tag(), "und");
}

#[test]
fn tag_round_trip() {
    for lang in Language::builtin().chain([Language::COMMON]) {
        assert_eq!(Language::from_tag(lang.tag()), Ok(lang.clone()));
        assert_eq!(lang.to_string().parse::<Language>(), Ok(lang));
    }
}

#[test]
fn equality_and_hash() {
    assert_eq!(Language::RU_RU, Language::RU_RU.clone());
    assert_eq!(
        hash_of(&Language::RU_RU),
        hash_of(&Language::from_tag("ru").unwrap())
    );
    assert_ne!(Language::RU_RU, Language::EN_US);
    assert_ne!(Language::RU_RU, Language::COMMON);
    assert_ne!(Language::EN_US, Language::COMMON);
}

#[test]
fn a_clone_equals_its_original() {
    // `==` returns at once for two handles of one table (a clone); the result is that of the
    // full comparison.
    assert_eq!(Language::RU_RU, Language::RU_RU.clone());
    assert_eq!(Language::COMMON, Language::COMMON.clone());
    let built = typolish::LanguageBuilder::from_language(&Language::EN_US, "en-GB")
        .quotes(Some(QuotePairs::new(('‘', '’'), ('“', '”'))))
        .build()
        .unwrap();
    assert_eq!(built, built.clone());
    let again = typolish::LanguageBuilder::from_language(&Language::EN_US, "en-GB")
        .quotes(Some(QuotePairs::new(('‘', '’'), ('“', '”'))))
        .build()
        .unwrap();
    assert_eq!(built, again);
    assert_ne!(built, Language::EN_US);
    let copy = typolish::LanguageBuilder::from_language(&Language::RU_RU, "ru-RU")
        .build()
        .unwrap();
    assert_eq!(copy.clone(), Language::RU_RU);
    assert_eq!(Language::RU_RU, copy);
}

#[test]
fn constants_are_const() {
    const RU: Language = Language::RU_RU;
    static EN: Language = Language::EN_US;
    assert_eq!(RU.tag(), "ru-RU");
    assert_eq!(EN.tag(), "en-US");
}

#[test]
fn debug_names_the_tag() {
    for lang in Language::builtin().chain([Language::COMMON]) {
        let text = format!("{lang:?}");
        assert!(text.contains(lang.tag()), "{text}");
    }
}

/// The rules that the data of each language does not activate, design section 7.4.
fn inert(lang: &Language) -> Vec<&'static str> {
    Rule::ALL
        .iter()
        .filter(|r| !lang.supports(**r))
        .map(|r| r.id())
        .collect()
}

#[test]
fn ru_supports_every_rule() {
    assert_eq!(inert(&Language::RU_RU), Vec::<&str>::new());
}

#[test]
fn en_lacks_dialogue_particles_short_length_and_the_glued_dash() {
    assert_eq!(
        inert(&Language::EN_US),
        [
            "dash.dialogue",
            "nbsp.short-length",
            "nbsp.particles",
            "nbsp.dash"
        ]
    );
}

#[test]
fn common_supports_only_rules_that_need_no_data_and_units_and_signs() {
    assert_eq!(
        inert(&Language::COMMON),
        [
            "dash.dialogue",
            "dash.double-hyphen",
            "dash.spaced-hyphen",
            "range.context",
            "quotes.pairs",
            "nbsp.abbreviations",
            "nbsp.initials",
            "nbsp.dates",
            "nbsp.short-words",
            "nbsp.short-length",
            "nbsp.particles",
            "nbsp.dash",
        ]
    );
    for id in [
        "symbols.copyright",
        "symbols.arrows",
        "symbols.math",
        "ellipsis.dots",
        "ellipsis.inner",
        "range.placeholders",
        "range.digits",
        "range.time",
        "number.minus",
        "number.times",
        "quotes.apostrophe",
        "quotes.primes",
        "nbsp.units",
        "nbsp.signs",
        "nbsp.phone",
    ] {
        assert!(Language::COMMON.supports(id.parse().unwrap()), "{id}");
    }
}

#[test]
fn active_rules_are_the_supported_part_of_the_configured_set() {
    for lang in Language::builtin().chain([Language::COMMON]) {
        for set in [
            RuleSet::EMPTY,
            RuleSet::ALL,
            RuleSet::PROSE,
            RuleSet::INTERFACE,
        ] {
            let t = Typographer::new(lang.clone(), set);
            let expected: RuleSet = set.iter().filter(|r| lang.supports(*r)).collect();
            assert_eq!(t.rules(), set, "{lang} {set}");
            assert_eq!(t.active_rules(), expected, "{lang} {set}");
            assert_eq!(t.language(), &lang);
        }
    }
}

#[test]
fn the_interface_of_common_has_these_inert_rules() {
    let t = Typographer::new(Language::COMMON, RuleSet::INTERFACE);
    assert_eq!(
        ids(t.rules().difference(t.active_rules())),
        [
            "dash.spaced-hyphen",
            "quotes.pairs",
            "nbsp.abbreviations",
            "nbsp.initials",
            "nbsp.dates",
            "nbsp.short-words",
            "nbsp.particles",
            "nbsp.dash",
        ]
    );
}

#[test]
fn settings_change_what_is_active() {
    // `quote_pairs` supplies pairs.
    let t = Typographer::builder(Language::COMMON, RuleSet::PROSE)
        .quote_pairs(QuotePairs::new(('«', '»'), ('‹', '›')))
        .build()
        .unwrap();
    assert!(t.active_rules().contains(Rule::QuotesPairs));

    // `dash_style` on a table without dash data creates it.
    let t = Typographer::builder(Language::COMMON, RuleSet::PROSE)
        .dash_style(DashStyle::SpacedGlued)
        .build()
        .unwrap();
    let active = t.active_rules();
    assert!(active.contains(Rule::DashDoubleHyphen));
    assert!(active.contains(Rule::DashSpacedHyphen));
    assert!(active.contains(Rule::NbspDash));
    assert!(!active.contains(Rule::DashDialogue));
    let t = Typographer::builder(Language::COMMON, RuleSet::PROSE)
        .dash_style(DashStyle::Spaced)
        .build()
        .unwrap();
    assert!(t.active_rules().contains(Rule::DashSpacedHyphen));
    assert!(!t.active_rules().contains(Rule::NbspDash));

    // Added words activate lists.
    let t = Typographer::builder(Language::COMMON, RuleSet::PROSE)
        .add_short_words(["a"])
        .add_particles(["же"])
        .build()
        .unwrap();
    assert!(t.active_rules().contains(Rule::NbspShortWords));
    assert!(t.active_rules().contains(Rule::NbspParticles));

    // The length decides `nbsp.short-length`.
    let t = Typographer::builder(Language::RU_RU, RuleSet::PROSE)
        .short_word_length(0)
        .build()
        .unwrap();
    assert!(!t.active_rules().contains(Rule::NbspShortLength));
    let t = Typographer::builder(Language::EN_US, RuleSet::PROSE)
        .short_word_length(2)
        .build()
        .unwrap();
    assert!(t.active_rules().contains(Rule::NbspShortLength));

    // Removing every entry of a list makes its rule inert.
    let ru_particles = ["б", "бы", "ж", "же", "ли", "ль"];
    let t = Typographer::builder(Language::RU_RU, RuleSet::PROSE)
        .remove_particles(ru_particles)
        .build()
        .unwrap();
    assert!(!t.active_rules().contains(Rule::NbspParticles));
    let t = Typographer::builder(Language::RU_RU, RuleSet::PROSE)
        .remove_particles(&ru_particles[..5])
        .build()
        .unwrap();
    assert!(t.active_rules().contains(Rule::NbspParticles));
}

#[test]
fn disabled_and_enabled_rules_follow_base_union_enabled_minus_disabled() {
    let t = Typographer::builder(Language::RU_RU, RuleSet::INTERFACE)
        .enable(Rule::NumberTimes)
        .disable(Rule::NbspShortWords)
        .build()
        .unwrap();
    assert_eq!(
        t.rules(),
        RuleSet::INTERFACE
            .with(Rule::NumberTimes)
            .without(Rule::NbspShortWords)
    );
    // Order of calls does not matter.
    let u = Typographer::builder(Language::RU_RU, RuleSet::INTERFACE)
        .disable(Rule::NbspShortWords)
        .enable(Rule::NumberTimes)
        .build()
        .unwrap();
    assert_eq!(t.rules(), u.rules());
    // Disabling a rule that is not in the base set, and enabling one that is, are no errors.
    let v = Typographer::builder(Language::RU_RU, RuleSet::INTERFACE)
        .disable(Rule::SymbolsMath)
        .enable(Rule::NbspDash)
        .build()
        .unwrap();
    assert_eq!(v.rules(), RuleSet::INTERFACE);
}
