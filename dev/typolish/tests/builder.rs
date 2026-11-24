//! `TypographerBuilder` validation (design sections 2.4 and 2.11): which error comes first, which
//! value is reported, which values are accepted.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    clippy::print_stdout,
    clippy::print_stderr,
    clippy::dbg_macro
)]

use typolish::{
    DashStyle, Error, Language, NarrowGlue, Output, PercentSpacing, QuotePairs, Rule, RuleSet,
    SingleQuotes, Typographer, TypographerBuilder,
};

fn base() -> TypographerBuilder {
    Typographer::builder(Language::RU_RU, RuleSet::PROSE)
}

/// The setting name and reason of the `InvalidSetting` that `build` returns.
fn setting_error(builder: TypographerBuilder) -> (&'static str, String) {
    match builder.build() {
        Err(Error::InvalidSetting {
            setting, reason, ..
        }) => (setting, reason),
        other => panic!("expected InvalidSetting, got {other:?}"),
    }
}

fn setting_of(builder: TypographerBuilder) -> &'static str {
    setting_error(builder).0
}

#[test]
fn defaults_build() {
    let t = base().build().unwrap();
    assert_eq!(t.rules(), RuleSet::PROSE);
    assert_eq!(t.language(), &Language::RU_RU);
    assert!(
        Typographer::builder(Language::COMMON, RuleSet::EMPTY)
            .build()
            .is_ok()
    );
}

#[test]
fn every_setting_accepts_a_valid_value() {
    let t = base()
        .enable(Rule::NumberTimes)
        .disable(Rule::NbspDash)
        .quote_pairs(QuotePairs::new(('«', '»'), ('„', '“')))
        .single_quotes(SingleQuotes::Everywhere)
        .dash_style(DashStyle::Spaced)
        .percent(PercentSpacing::Attached)
        .add_units(["req/s", "vCPU"])
        .remove_units(["МБ"])
        .add_short_words(["ну"])
        .remove_short_words(["и"])
        .short_word_length(3)
        .short_words_any_script(true)
        .add_particles(["то"])
        .remove_particles(["ль"])
        .narrow_glue(NarrowGlue::INITIALS.union(NarrowGlue::SIGNS))
        .min_grouping_digits(2)
        .group_separator('\u{202f}')
        .minus_sign('-')
        .protect_backticks(false)
        .protect_between("{{", "}}")
        .protect_between("[[", "]]")
        .build();
    assert!(t.is_ok(), "{t:?}");
}

#[test]
fn single_quote_modes_and_percent_spacings_build() {
    for mode in [
        SingleQuotes::NestedOnly,
        SingleQuotes::Everywhere,
        SingleQuotes::Never,
    ] {
        assert!(base().single_quotes(mode).build().is_ok());
    }
    for spacing in [
        PercentSpacing::AsWritten,
        PercentSpacing::Language,
        PercentSpacing::Attached,
        PercentSpacing::Spaced,
    ] {
        assert!(base().percent(spacing).build().is_ok());
    }
    for style in [DashStyle::Closed, DashStyle::Spaced, DashStyle::SpacedGlued] {
        assert!(base().dash_style(style).build().is_ok());
    }
}

#[test]
fn narrow_glue_is_a_set() {
    assert!(NarrowGlue::NONE.contains(NarrowGlue::NONE));
    assert!(!NarrowGlue::NONE.contains(NarrowGlue::SIGNS));
    for place in [
        NarrowGlue::ABBREVIATIONS,
        NarrowGlue::INITIALS,
        NarrowGlue::SIGNS,
    ] {
        assert!(NarrowGlue::ALL.contains(place));
        assert!(place.contains(place));
        assert_eq!(NarrowGlue::NONE.union(place), place);
        assert_eq!(place.union(place), place);
        assert!(!NarrowGlue::NONE.contains(place));
    }
    assert!(!NarrowGlue::ABBREVIATIONS.contains(NarrowGlue::INITIALS));
    let both = NarrowGlue::ABBREVIATIONS.union(NarrowGlue::INITIALS);
    assert_eq!(both, NarrowGlue::INITIALS.union(NarrowGlue::ABBREVIATIONS));
    assert!(both.contains(NarrowGlue::ABBREVIATIONS));
    assert!(both.contains(NarrowGlue::INITIALS));
    assert!(!both.contains(NarrowGlue::SIGNS));
    assert_eq!(both.union(NarrowGlue::SIGNS), NarrowGlue::ALL);
    assert!(NarrowGlue::ALL.contains(both));
}

#[test]
fn a_rule_both_enabled_and_disabled_is_a_conflict() {
    let err = base()
        .enable(Rule::NbspUnits)
        .disable(Rule::NbspUnits)
        .build()
        .unwrap_err();
    assert!(
        matches!(
            err,
            Error::RuleConflict {
                id: "nbsp.units",
                ..
            }
        ),
        "{err}"
    );
    assert!(err.to_string().contains("nbsp.units"));
    // A conflict beats a later check.
    let err = base()
        .enable(Rule::NbspUnits)
        .disable(Rule::NbspUnits)
        .minus_sign('5')
        .protect_between("a", "b")
        .build()
        .unwrap_err();
    assert!(matches!(err, Error::RuleConflict { .. }), "{err}");
}

#[test]
fn checks_run_in_the_documented_order() {
    let bad_pairs = QuotePairs::new(('"', '"'), ('‘', '’'));
    // (2) delimiters before (3) quote pairs.
    assert_eq!(
        setting_of(base().quote_pairs(bad_pairs).protect_between("", "|")),
        "protect_between"
    );
    assert_eq!(
        setting_of(base().protect_between("", "|").quote_pairs(bad_pairs)),
        "protect_between"
    );
    // (3) quote pairs before (4) lists.
    assert_eq!(
        setting_of(base().add_units(["5G"]).quote_pairs(bad_pairs)),
        "quote_pairs"
    );
    // (4) units, then short words, then particles.
    assert_eq!(
        setting_of(
            base()
                .add_particles([""])
                .add_short_words([""])
                .add_units([""])
        ),
        "units"
    );
    assert_eq!(
        setting_of(base().add_particles([""]).add_short_words([""])),
        "short_words"
    );
    assert_eq!(setting_of(base().add_particles([""])), "particles");
    // (4) lists before (5) numbers.
    assert_eq!(
        setting_of(base().group_separator('5').add_particles([""])),
        "particles"
    );
    // (5) group separator, minus sign, grouping digits.
    assert_eq!(
        setting_of(
            base()
                .min_grouping_digits(9)
                .minus_sign('5')
                .group_separator('5')
        ),
        "group_separator"
    );
    assert_eq!(
        setting_of(base().min_grouping_digits(9).minus_sign('5')),
        "minus_sign"
    );
    assert_eq!(
        setting_of(base().min_grouping_digits(9)),
        "min_grouping_digits"
    );
}

#[test]
fn errors_do_not_depend_on_the_order_of_calls() {
    let a = base()
        .add_particles([""])
        .minus_sign('5')
        .add_units(["5G"])
        .build();
    let b = base()
        .minus_sign('5')
        .add_units(["5G"])
        .add_particles([""])
        .build();
    let c = base()
        .add_units(["5G"])
        .add_particles([""])
        .minus_sign('5')
        .build();
    assert!(a.is_err());
    assert_eq!(a.as_ref().err(), b.as_ref().err());
    assert_eq!(a.as_ref().err(), c.as_ref().err());
}

#[test]
fn the_rule_first_in_rule_all_is_the_conflict() {
    let err = base()
        .enable(Rule::NbspDash)
        .disable(Rule::NbspDash)
        .enable(Rule::SymbolsCopyright)
        .disable(Rule::SymbolsCopyright)
        .build()
        .unwrap_err();
    assert!(
        matches!(
            err,
            Error::RuleConflict {
                id: "symbols.copyright",
                ..
            }
        ),
        "{err}"
    );
}

fn delimiters(pairs: &[(&str, &str)]) -> Result<Typographer, Error> {
    pairs
        .iter()
        .fold(base(), |b, (open, close)| b.protect_between(*open, *close))
        .build()
}

#[test]
fn good_delimiters() {
    for pairs in [
        &[("|", "|")][..],
        &[("{{", "}}")][..],
        &[("<", ">")][..],
        &[("«", "»")][..],
        &[("[[", "]]"), ("{{", "}}")][..],
        &[("{{", "}}"), ("{%", "%}")][..],
        &[("$$", "$$")][..],
    ] {
        assert!(delimiters(pairs).is_ok(), "{pairs:?}");
    }
}

#[test]
fn bad_delimiters() {
    for pairs in [
        &[("", "|")][..],
        &[("|", "")][..],
        &[("a b", "|")][..],
        &[("|", "\t")][..],
        &[("|", "\u{a0}")][..],
        &[("x", "|")][..],
        &[("|", "7")][..],
        &[("é", "|")][..],
        &[("Ж", "|")][..],
        // A delimiter of one pair that is a prefix of a delimiter of another.
        &[("{", "}"), ("{{", "}}")][..],
        &[("{{", "}}"), ("{", "]")][..],
        // The opener a proper prefix of its own closer, and the reverse.
        &[("[", "[[")][..],
        &[("]]", "]")][..],
    ] {
        assert_eq!(
            setting_of(
                pairs
                    .iter()
                    .fold(base(), |b, (o, c)| b.protect_between(*o, *c))
            ),
            "protect_between",
            "{pairs:?}"
        );
    }
}

#[test]
fn equal_delimiters_of_one_pair_are_allowed() {
    assert!(delimiters(&[("|", "|")]).is_ok());
    assert!(delimiters(&[("//", "//")]).is_ok());
}

#[test]
fn the_reason_quotes_the_offending_value() {
    let (setting, reason) = setting_error(base().protect_between("a1", "|"));
    assert_eq!(setting, "protect_between");
    assert!(reason.contains("a1"), "{reason}");
    let (setting, reason) = setting_error(base().add_units(["5G"]));
    assert_eq!(setting, "units");
    assert!(reason.contains("5G"), "{reason}");
    let (setting, reason) = setting_error(base().add_short_words(["a b"]));
    assert_eq!(setting, "short_words");
    assert!(reason.contains("a b"), "{reason}");
    let (setting, reason) = setting_error(base().group_separator('7'));
    assert_eq!(setting, "group_separator");
    assert!(reason.contains('7'), "{reason}");
}

#[test]
fn the_smallest_offending_value_is_reported() {
    let (_, reason) = setting_error(base().add_units(["9x", "1x", "5x"]));
    assert!(reason.contains("1x"), "{reason}");
    let (_, reason) = setting_error(base().add_short_words(["b c", "a b"]));
    assert!(reason.contains("a b"), "{reason}");
    let (_, reason) = setting_error(base().add_particles(["q q", "p p", "r r"]));
    assert!(reason.contains("p p"), "{reason}");
}

#[test]
fn good_and_bad_words() {
    for word in ["", " ", "a b", "a\tb", "a\u{a0}b", "\n"] {
        assert_eq!(
            setting_of(base().add_short_words([word])),
            "short_words",
            "{word:?}"
        );
        assert_eq!(
            setting_of(base().add_particles([word])),
            "particles",
            "{word:?}"
        );
        assert_eq!(setting_of(base().add_units([word])), "units", "{word:?}");
        assert_eq!(
            setting_of(base().remove_short_words([word])),
            "short_words",
            "{word:?}"
        );
    }
    for word in ["ну", "x", "a.", "co-op", "A", "ЖЕ"] {
        assert!(
            base()
                .add_short_words([word])
                .add_particles([word])
                .build()
                .is_ok(),
            "{word}"
        );
    }
}

#[test]
fn a_word_both_added_and_removed_is_an_error() {
    assert_eq!(
        setting_of(base().add_units(["xx"]).remove_units(["xx"])),
        "units"
    );
    assert_eq!(
        setting_of(base().add_short_words(["xx"]).remove_short_words(["xx"])),
        "short_words"
    );
    assert_eq!(
        setting_of(base().add_particles(["xx"]).remove_particles(["xx"])),
        "particles"
    );
    assert!(
        base()
            .add_units(["xx"])
            .remove_units(["yy"])
            .build()
            .is_ok()
    );
}

#[test]
fn units_may_not_start_with_a_digit_but_may_contain_one() {
    assert_eq!(setting_of(base().add_units(["5G"])), "units");
    assert_eq!(setting_of(base().remove_units(["0px"])), "units");
    // The leading digit is an ASCII digit (design sections 2.11, 7.6): a unit that starts with
    // another decimal digit cannot be read as a `Number` token and is accepted.
    assert!(base().add_units(["٣px"]).build().is_ok());
    assert!(
        base()
            .add_units(["m2", "x5", "vCPU", "%", "$", "€"])
            .build()
            .is_ok()
    );
}

/// An added unit equal to a short word or a particle (compared with case) that is not an ambiguous
/// unit of the language, or of one character equal to a sign of the language, is read by two rules
/// and breaks idempotence (design sections 2.11 and 7.6).
#[test]
fn an_added_unit_must_not_be_a_short_word_a_particle_or_a_sign() {
    let (setting, reason) = setting_error(base().add_units(["в"]));
    assert_eq!(setting, "units");
    assert!(reason.contains("`в`"), "{reason}");
    assert_eq!(setting_of(base().add_units(["ли"])), "units");
    assert_eq!(setting_of(base().add_units(["м", "же"])), "units");
    let all = Typographer::builder(Language::RU_RU, RuleSet::ALL);
    let (setting, reason) = setting_error(all.clone().add_units(["§"]));
    assert_eq!(setting, "units");
    assert!(reason.contains("`§`"), "{reason}");
    assert_eq!(
        setting_of(Typographer::builder(Language::EN_US, RuleSet::PROSE).add_units(["№"])),
        "units"
    );
    // Removed from the short words, listed as ambiguous by the language, or differing in case.
    let t = base()
        .add_units(["в"])
        .remove_short_words(["в"])
        .build()
        .unwrap();
    let once = t.typeset("Нас было 5 в лесу", Output::Unicode);
    assert_eq!(once, "Нас было 5\u{a0}в\u{a0}лесу");
    assert_eq!(t.typeset(&once, Output::Unicode), once);
    assert!(base().add_units(["с"]).build().is_ok());
    assert!(base().add_units(["В"]).build().is_ok());
    assert!(all.add_units(["§§", "%", "‰"]).build().is_ok());
}

#[test]
fn group_separators() {
    for ok in ['\u{a0}', '\u{202f}', '.', '\'', '_', '\u{2019}'] {
        assert!(base().group_separator(ok).build().is_ok(), "{ok:?}");
    }
    // The decimal separator of the language is no group separator (`decimal != group`, design
    // sections 2.11 and 7.6): `,` is the decimal of ru-RU, `.` that of en-US.
    assert_eq!(setting_of(base().group_separator(',')), "group_separator");
    let en = || Typographer::builder(Language::EN_US, RuleSet::PROSE);
    assert_eq!(setting_of(en().group_separator('.')), "group_separator");
    assert!(en().group_separator(',').build().is_ok());
    for bad in [
        ' ', '\t', '\n', '\u{2009}', '\u{2002}', '\u{3000}', '5', '٣', 'a', 'я',
    ] {
        assert_eq!(
            setting_of(base().group_separator(bad)),
            "group_separator",
            "{bad:?}"
        );
    }
}

#[test]
fn minus_signs() {
    // `-` and the dashes the second pass reads as the sign of a number (design sections 2.11 and
    // 3.6, check 15).
    for ok in [
        '-', '\u{2010}', '\u{2011}', '\u{2012}', '\u{2013}', '\u{2014}', '\u{2015}', '\u{2212}',
    ] {
        assert!(base().minus_sign(ok).build().is_ok(), "{ok:?}");
    }
    for bad in [
        ' ', '\t', '\u{a0}', '\u{202f}', '5', 'a', 'я', '"', '\'', '~', '*', '_', '+', '(', '.',
        ',', '%', '`', '<', '·', '÷', '\u{ad}', '\u{200b}', '\u{301}', '\u{fe63}', '\u{ff0d}',
    ] {
        assert_eq!(setting_of(base().minus_sign(bad)), "minus_sign", "{bad:?}");
    }
}

#[test]
fn grouping_digits() {
    for ok in 0..=4 {
        assert!(base().min_grouping_digits(ok).build().is_ok(), "{ok}");
    }
    for bad in [5, 6, 100, 255] {
        assert_eq!(
            setting_of(base().min_grouping_digits(bad)),
            "min_grouping_digits",
            "{bad}"
        );
    }
}

#[test]
fn quote_pair_validation() {
    for pairs in [
        QuotePairs::new(('«', '»'), ('„', '“')),
        QuotePairs::new(('“', '”'), ('‘', '’')),
        QuotePairs::new(('„', '“'), ('‚', '‘')),
        QuotePairs::new(('«', '»'), ('‹', '›')),
        QuotePairs::new(('»', '«'), ('›', '‹')),
    ] {
        assert!(base().quote_pairs(pairs).build().is_ok(), "{pairs:?}");
    }
    for pairs in [
        QuotePairs::new(('"', '"'), ('‘', '’')),
        QuotePairs::new(('«', '»'), ('\'', '\'')),
        QuotePairs::new((' ', '»'), ('‘', '’')),
        QuotePairs::new(('a', 'b'), ('‘', '’')),
        QuotePairs::new(('«', '»'), ('я', 'ю')),
        QuotePairs::new(('«', '»'), ('«', '»')),
        QuotePairs::new(('«', '»'), ('„', '»')),
        QuotePairs::new(('«', '»'), ('„', '„')),
        QuotePairs::new(('«', '«'), ('„', '“')),
        QuotePairs::new(('「', '」'), ('『', '』')),
        QuotePairs::new(('(', ')'), ('[', ']')),
    ] {
        assert_eq!(
            setting_of(base().quote_pairs(pairs)),
            "quote_pairs",
            "{pairs:?}"
        );
    }
}

#[test]
fn quote_pair_reasons_name_the_ground_of_design_section_2_11() {
    // Design section 2.11, item (3): a straight quote, a space or a letter would be read as input
    // again, breaking idempotence; other characters are no quote characters.
    for pairs in [
        QuotePairs::new(('"', '»'), ('„', '“')),
        QuotePairs::new(('a', '»'), ('„', '“')),
        QuotePairs::new((' ', '»'), ('„', '“')),
    ] {
        let (setting, reason) = setting_error(base().quote_pairs(pairs));
        assert_eq!(setting, "quote_pairs");
        assert!(reason.contains("idempotence"), "{pairs:?}: {reason}");
    }
    let (_, reason) = setting_error(base().quote_pairs(QuotePairs::new(('(', ')'), ('[', ']'))));
    assert!(reason.contains("is not a quote character"), "{reason}");
}

#[test]
fn quote_pairs_are_a_value_type() {
    let a = QuotePairs::new(('«', '»'), ('„', '“'));
    let b = a;
    assert_eq!(b, QuotePairs::new(('«', '»'), ('„', '“')));
    assert_ne!(a, QuotePairs::new(('“', '”'), ('‘', '’')));
}

#[test]
fn the_builder_is_clone_and_debug() {
    let builder = base().add_units(["x"]);
    let copy = builder.clone();
    assert_eq!(format!("{builder:?}"), format!("{copy:?}"));
    assert!(!format!("{builder:?}").is_empty());
    assert!(copy.build().is_ok());
}

#[test]
fn error_variants_display_their_values() {
    let e = Language::from_tag("de-DE").unwrap_err();
    assert!(e.to_string().contains("de-DE"), "{e}");
    let e = "nbsp.nothing".parse::<Rule>().unwrap_err();
    assert!(e.to_string().contains("nbsp.nothing"), "{e}");
    let e = "prose,,x".parse::<RuleSet>().unwrap_err();
    assert!(
        e.to_string().contains("prose,,x") && e.to_string().contains("empty item"),
        "{e}"
    );
    let e = base().minus_sign('5').build().unwrap_err();
    assert!(e.to_string().contains("minus_sign"), "{e}");
    // `Error` is a std error and comparable.
    let boxed: Box<dyn std::error::Error + Send + Sync> = Box::new(e.clone());
    assert!(!boxed.to_string().is_empty());
    assert_eq!(e.clone(), e);
}
