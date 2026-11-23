//! API contracts of design section 2 that are not output (design section 13.2, `tests/api.rs`).
//!
//! The rows that need `LanguageBuilder`, `report` or `lint` join with those parts.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    clippy::print_stdout,
    clippy::print_stderr,
    clippy::dbg_macro
)]

use std::borrow::Cow;
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

use typolish::{
    Error, Language, LanguageData, Number, Output, Piece, QuotePairs, Rule, RuleSet, Slot,
    Template, TemplatePart, Typographer,
};

fn invalid_rule_set(input: &str) -> &'static str {
    match input.parse::<RuleSet>() {
        Err(Error::InvalidRuleSet {
            input: echoed,
            reason,
            ..
        }) => {
            assert_eq!(echoed, input);
            reason
        }
        other => panic!("{input:?}: {other:?}"),
    }
}

#[test]
fn rule_set_grammar() {
    assert!(matches!(
        "prose,+nbsp.everything".parse::<RuleSet>(),
        Err(Error::UnknownRule { id, .. }) if id == "nbsp.everything"
    ));
    assert_eq!(
        invalid_rule_set("+nbsp.dash,-nbsp.dash"),
        "rule both added and removed"
    );
    assert_eq!(
        invalid_rule_set("nbsp.dash,-nbsp.dash"),
        "rule both added and removed"
    );
    assert_eq!(
        invalid_rule_set("nbsp.dash,prose"),
        "base set name after the first item"
    );
    assert_eq!(
        invalid_rule_set("nbsp.dash,+prose"),
        "base set name after the first item"
    );
    assert_eq!(
        invalid_rule_set("prose,-interface"),
        "base set name after the first item"
    );
    assert_eq!(invalid_rule_set("+prose"), "sign before a base set name");
    assert_eq!(invalid_rule_set("prose,,nbsp.dash"), "empty item");
    assert_eq!(invalid_rule_set(""), "empty item");
    assert_eq!(invalid_rule_set("   "), "empty item");
    assert_eq!(invalid_rule_set("+"), "empty item");
    assert_eq!(invalid_rule_set("prose,-"), "empty item");
    assert_eq!(
        " prose , +number.times ".parse::<RuleSet>(),
        Ok(RuleSet::PROSE.with(Rule::NumberTimes))
    );
    assert_eq!(
        "+nbsp.dash,+nbsp.dash".parse::<RuleSet>(),
        Ok(RuleSet::EMPTY.with(Rule::NbspDash))
    );
    assert!(matches!(
        "+ nbsp.dash".parse::<RuleSet>(),
        Err(Error::UnknownRule { id, .. }) if id == " nbsp.dash"
    ));
    assert_eq!("-nbsp.dash".parse::<RuleSet>(), Ok(RuleSet::EMPTY));
    assert_eq!(
        "interface,+symbols.arrows,-nbsp.initials".parse::<RuleSet>(),
        Ok(RuleSet::INTERFACE
            .with(Rule::SymbolsArrows)
            .without(Rule::NbspInitials))
    );
    assert_eq!(
        "-nbsp.initials,interface"
            .parse::<RuleSet>()
            .map_err(|_| ()),
        Err(())
    );
    assert_eq!("all".parse::<RuleSet>(), Ok(RuleSet::ALL));
    assert_eq!("empty".parse::<RuleSet>(), Ok(RuleSet::EMPTY));
}

#[test]
fn rule_set_operations() {
    let set = RuleSet::EMPTY
        .with(Rule::NbspDash)
        .with(Rule::SymbolsCopyright);
    assert_eq!(set.len(), 2);
    assert_eq!(set.to_string(), "symbols.copyright,nbsp.dash");
    assert_eq!(
        set.iter().collect::<Vec<_>>(),
        [Rule::SymbolsCopyright, Rule::NbspDash]
    );
    assert_eq!(set.union(RuleSet::EMPTY), set);
    assert_eq!(set.difference(set), RuleSet::EMPTY);
    let collected: RuleSet = [Rule::NbspDash, Rule::SymbolsCopyright]
        .into_iter()
        .collect();
    assert_eq!(collected, set);
    let mut extended = RuleSet::EMPTY;
    extended.extend([Rule::SymbolsCopyright, Rule::NbspDash, Rule::NbspDash]);
    assert_eq!(extended, set);
    assert_eq!(RuleSet::default(), RuleSet::EMPTY);
}

#[test]
fn builder_errors_do_not_depend_on_call_order() {
    let a = Typographer::builder(Language::RU_RU, RuleSet::PROSE)
        .disable(Rule::NumberTimes)
        .protect_between("a", "|")
        .enable(Rule::NumberTimes)
        .build()
        .unwrap_err();
    let b = Typographer::builder(Language::RU_RU, RuleSet::PROSE)
        .enable(Rule::NumberTimes)
        .protect_between("a", "|")
        .disable(Rule::NumberTimes)
        .build()
        .unwrap_err();
    assert_eq!(a, b);
    assert!(matches!(
        a,
        Error::RuleConflict {
            id: "number.times",
            ..
        }
    ));
    // The rule first in `Rule::ALL` is reported.
    let err = Typographer::builder(Language::RU_RU, RuleSet::PROSE)
        .enable(Rule::NbspDash)
        .disable(Rule::NbspDash)
        .enable(Rule::SymbolsMath)
        .disable(Rule::SymbolsMath)
        .build()
        .unwrap_err();
    assert!(matches!(
        err,
        Error::RuleConflict {
            id: "symbols.math",
            ..
        }
    ));
}

fn delimiters(pairs: &[(&str, &str)]) -> Result<Typographer, Error> {
    pairs
        .iter()
        .fold(
            Typographer::builder(Language::EN_US, RuleSet::INTERFACE),
            |b, (open, close)| b.protect_between(*open, *close),
        )
        .build()
}

#[test]
fn protection_delimiters() {
    assert!(delimiters(&[("|", "|")]).is_ok());
    assert!(delimiters(&[("{{", "}}"), ("[[", "]]")]).is_ok());
    for bad in [
        &[("[", "[[")][..],
        &[("{{", "}}"), ("{{", "]]")][..],
        &[("a1", "|")][..],
        &[("", "|")][..],
        &[("< ", ">")][..],
        &[("<", ">"), ("<<", ">>")][..],
    ] {
        assert!(
            matches!(
                delimiters(bad),
                Err(Error::InvalidSetting {
                    setting: "protect_between",
                    ..
                })
            ),
            "{bad:?}"
        );
    }
    // The smallest offending value is reported.
    let err = delimiters(&[("b", "|"), ("a", "|")]).unwrap_err();
    assert!(
        matches!(&err, Error::InvalidSetting { reason, .. } if reason.contains("`a`")),
        "{err}"
    );
}

#[test]
fn quote_pairs_of_the_builder() {
    let ok = Typographer::builder(Language::EN_US, RuleSet::PROSE)
        .quote_pairs(QuotePairs::new(('«', '»'), ('‹', '›')))
        .build();
    assert!(ok.is_ok());
    for pairs in [
        QuotePairs::new(('«', '»'), ('«', '„')),
        QuotePairs::new(('"', '"'), ('‘', '’')),
        QuotePairs::new(('「', '」'), ('‘', '’')),
    ] {
        let err = Typographer::builder(Language::EN_US, RuleSet::PROSE)
            .quote_pairs(pairs)
            .build()
            .unwrap_err();
        assert!(
            matches!(
                err,
                Error::InvalidSetting {
                    setting: "quote_pairs",
                    ..
                }
            ),
            "{err}"
        );
    }
}

#[test]
fn number_settings() {
    let build = |b: typolish::TypographerBuilder| b.build().map(|_| ()).unwrap_err();
    let base = || Typographer::builder(Language::RU_RU, RuleSet::PROSE);
    assert!(
        base()
            .group_separator('\u{202f}')
            .minus_sign('-')
            .min_grouping_digits(0)
            .build()
            .is_ok()
    );
    assert!(matches!(
        build(base().group_separator(' ')),
        Error::InvalidSetting {
            setting: "group_separator",
            ..
        }
    ));
    assert!(matches!(
        build(base().group_separator('5')),
        Error::InvalidSetting {
            setting: "group_separator",
            ..
        }
    ));
    assert!(matches!(
        build(base().minus_sign('\'')),
        Error::InvalidSetting {
            setting: "minus_sign",
            ..
        }
    ));
    assert!(matches!(
        build(base().min_grouping_digits(5)),
        Error::InvalidSetting {
            setting: "min_grouping_digits",
            ..
        }
    ));
    // Lists are checked before numbers.
    assert!(matches!(
        build(base().min_grouping_digits(5).add_particles(["же ли"])),
        Error::InvalidSetting {
            setting: "particles",
            ..
        }
    ));
    assert!(matches!(
        build(base().add_units(["5G"])),
        Error::InvalidSetting {
            setting: "units",
            ..
        }
    ));
}

#[test]
fn language_tags() -> Result<(), Error> {
    assert_eq!(Language::from_tag("ru")?, Language::RU_RU);
    assert_eq!(Language::from_tag("EN")?, Language::EN_US);
    assert_eq!(Language::from_tag("ru_ru")?, Language::RU_RU);
    assert_eq!("und".parse::<Language>()?, Language::COMMON);
    for unknown in ["en-GB", "de", "", "ru-", "russian", "ru-RU-x"] {
        assert!(
            matches!(Language::from_tag(unknown), Err(Error::UnknownLanguage { tag, .. }) if tag == unknown)
        );
    }
    assert_eq!(
        Language::builtin()
            .map(|l| l.tag().to_owned())
            .collect::<Vec<_>>(),
        ["en-US", "ru-RU"]
    );
    Ok(())
}

fn hash_of<T: Hash>(value: &T) -> u64 {
    let mut hasher = DefaultHasher::new();
    value.hash(&mut hasher);
    hasher.finish()
}

#[test]
fn language_equality_compares_data() {
    static RU_COPY: LanguageData = LanguageData::RU_RU;
    static RU_OTHER: LanguageData = LanguageData::RU_RU.with_short_words(&["в"]);
    let copy = Language::from_data(&RU_COPY).unwrap();
    let other = Language::from_data(&RU_OTHER).unwrap();
    assert_eq!(copy, Language::RU_RU);
    assert_eq!(hash_of(&copy), hash_of(&Language::RU_RU));
    assert_ne!(other, Language::RU_RU);
    // A caller's language is printed by its tag and read back against the registry.
    assert_eq!(other.to_string(), "ru-RU");
    assert_eq!(other.to_string().parse::<Language>(), Ok(Language::RU_RU));
    static EN_GB: LanguageData = LanguageData::EN_US.with_tag("en-GB");
    let en_gb = Language::from_static(&EN_GB);
    assert!(matches!(
        en_gb.to_string().parse::<Language>(),
        Err(Error::UnknownLanguage { .. })
    ));
}

#[test]
fn invalid_language_data() {
    static UPPER: LanguageData = LanguageData::EN_US.with_tag("EN-gb");
    static ODD_QUOTES: LanguageData =
        LanguageData::EN_US.with_quotes(Some(QuotePairs::new(('「', '」'), ('‘', '’'))));
    for data in [&UPPER, &ODD_QUOTES] {
        assert!(matches!(
            Language::from_data(data),
            Err(Error::InvalidLanguageData { .. })
        ));
    }
}

#[test]
fn inert_rules_are_not_active() {
    let t = Typographer::new(Language::COMMON, RuleSet::PROSE);
    let inert: Vec<&str> = t
        .rules()
        .difference(t.active_rules())
        .iter()
        .map(|r| r.id())
        .collect();
    assert_eq!(
        inert,
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
    let en = Typographer::new(Language::EN_US, RuleSet::PROSE);
    let inert: Vec<&str> = en
        .rules()
        .difference(en.active_rules())
        .iter()
        .map(|r| r.id())
        .collect();
    assert_eq!(
        inert,
        [
            "dash.dialogue",
            "nbsp.short-length",
            "nbsp.particles",
            "nbsp.dash"
        ]
    );
    let ru = Typographer::new(Language::RU_RU, RuleSet::PROSE);
    assert_eq!(ru.active_rules(), RuleSet::PROSE);
}

#[test]
fn templates_keep_slots_in_place() {
    let t = Typographer::new(Language::RU_RU, RuleSet::INTERFACE);
    let empty: Template<'_, u8> = t.typeset_template([], Output::Unicode);
    assert_eq!(empty.texts(), [""]);
    assert!(matches!(empty.texts()[0], Cow::Borrowed("")));
    assert!(empty.slots().is_empty());
    assert!(!empty.changed());

    let out = t.typeset_template(
        [
            Piece::Slot("a", Slot::NUMBER),
            Piece::Text("Ждём."),
            Piece::Markup("<b>"),
            Piece::Text(".."),
            Piece::Slot("b", Slot::OTHER),
            Piece::Slot("c", Slot::WORD),
            Piece::Protected("`x`"),
        ],
        Output::Unicode,
    );
    assert_eq!(out.texts(), ["", "Ждём.<b>..", "", "`x`"]);
    assert!(matches!(out.texts()[3], Cow::Borrowed(_)));
    assert!(matches!(out.texts()[1], Cow::Owned(_)));
    assert_eq!(
        out.slots().iter().map(|(s, _)| *s).collect::<Vec<_>>(),
        ["a", "b", "c"]
    );
    assert_eq!(
        out.iter().collect::<Vec<_>>(),
        [
            TemplatePart::Slot(&"a", Slot::NUMBER),
            TemplatePart::Text("Ждём.<b>.."),
            TemplatePart::Slot(&"b", Slot::OTHER),
            TemplatePart::Slot(&"c", Slot::WORD),
            TemplatePart::Text("`x`"),
        ]
    );
    assert_eq!(
        out.fill(|s| Cow::Owned(s.to_uppercase())),
        "AЖдём.<b>..BC`x`"
    );
    let (texts, slots) = out.into_parts();
    assert_eq!(texts.len(), slots.len() + 1);
}

#[test]
fn an_iterator_that_calls_the_typographer_meets_a_free_scratch() {
    static EN: Typographer = Typographer::new(Language::EN_US, RuleSet::PROSE);
    let out = EN.typeset_template(
        ["a", "b"].into_iter().map(|s| {
            assert_eq!(EN.typeset(s, Output::Unicode), s);
            Piece::<()>::Text(s)
        }),
        Output::Unicode,
    );
    assert_eq!(out.texts(), ["ab"]);
}

// Stage 3 (design section 15.4): the API contracts that need the numbers and the settings.

#[test]
fn the_builder_example_of_section_2_4_builds_and_works() {
    use std::sync::LazyLock;
    use typolish::DashStyle;
    static EN_DOCS: LazyLock<Typographer> = LazyLock::new(|| {
        Typographer::builder(Language::EN_US, RuleSet::INTERFACE)
            .enable(Rule::NumberTimes)
            .disable(Rule::NbspShortWords)
            .dash_style(DashStyle::Spaced)
            .add_units(["req/s", "vCPU"])
            .protect_between("{{", "}}")
            .build()
            .expect("settings are valid")
    });
    assert!(EN_DOCS.rules().contains(Rule::NumberTimes));
    assert!(!EN_DOCS.rules().contains(Rule::NbspShortWords));
    assert_eq!(
        EN_DOCS.typeset("Screen 1920x1080 at 60 req/s", Output::Unicode),
        "Screen 1920×1080 at 60\u{a0}req/s"
    );
    // A protected pair of the builder stays as typed, and the text is borrowed.
    assert!(matches!(
        EN_DOCS.typeset("Say {{a - b}} now", Output::Unicode),
        Cow::Borrowed("Say {{a - b}} now")
    ));
    // The rules of a caller's configuration file.
    let rules: RuleSet = "interface,+symbols.arrows,-nbsp.initials".parse().unwrap();
    let ui = Typographer::builder(Language::RU_RU, rules)
        .build()
        .unwrap();
    assert!(ui.rules().contains(Rule::SymbolsArrows));
    assert!(!ui.rules().contains(Rule::NbspInitials));
    assert_eq!(ui.typeset("Вход -> выход", Output::Unicode), "Вход → выход");
}

#[test]
fn number_errors_compare_by_value() {
    let a = Number::decimal("1e3").unwrap_err();
    assert_eq!(a, Number::decimal("1e3").unwrap_err());
    assert_ne!(a, Number::decimal("1e4").unwrap_err());
    assert!(matches!(a, Error::InvalidNumber { .. }));
    assert_eq!(a.clone(), a);
}

#[test]
fn numbers_do_not_depend_on_the_rules_of_the_typographer() {
    // Section 2.4: settings do not depend on rule sets, and a language supplies the defaults.
    for language in [Language::RU_RU, Language::EN_US, Language::COMMON] {
        let a = Typographer::new(language.clone(), RuleSet::EMPTY);
        let b = Typographer::new(language.clone(), RuleSet::ALL);
        for n in [0.0, -1234567.5, 1e21, f64::NAN] {
            assert_eq!(
                a.format_number(n, Output::Unicode).to_string(),
                b.format_number(n, Output::Unicode).to_string(),
                "{language}"
            );
        }
    }
}
