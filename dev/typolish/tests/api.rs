//! API contracts of design section 2 that are not output (design section 13.2, `tests/api.rs`).
//!
//! The rows that need `LanguageBuilder`, `report` or `lint` join with those parts; stage 4 adds
//! the accessors and the `Display` of `Report`, `Change`, `ProtectedSpan` and `Diagnostic` and
//! the rows that need `report_template` and `lint_template` (design sections 2.9 and 12.5).

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
    Change, Diagnostic, DiagnosticKind, Error, HtmlStyle, Language, LanguageData, Number, Output,
    Piece, ProtectedKind, ProtectedSpan, QuotePairs, Rule, RuleSet, Scratch, Severity, Slot,
    TechKind, Template, TemplatePart, Typographer,
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

/// An iterator whose `size_hint` lower bound is above its length (a caller bug).
struct LyingPieces(Option<Piece<'static, u8>>, usize);

impl Iterator for LyingPieces {
    type Item = Piece<'static, u8>;
    fn next(&mut self) -> Option<Self::Item> {
        self.0.take()
    }
    fn size_hint(&self) -> (usize, Option<usize>) {
        (self.1, None)
    }
}

#[test]
fn templates_survive_a_lying_size_hint() {
    // Design sections 2.11 and 2.12: a lower bound that cannot be reserved is skipped, so the
    // template methods neither panic nor abort.
    let t = Typographer::new(Language::EN_US, RuleSet::PROSE);
    let honest = || [Piece::<u8>::Text("\"a\"")];
    for hint in [usize::MAX, 1 << 40] {
        let lying = || LyingPieces(Some(Piece::Text("\"a\"")), hint);
        assert_eq!(
            t.typeset_template(lying(), Output::Unicode).texts(),
            ["“a”"]
        );
        let mut scratch = Scratch::new();
        assert_eq!(
            t.typeset_template_with(&mut scratch, lying(), Output::Unicode)
                .texts(),
            ["“a”"]
        );
        assert_eq!(
            t.report_template(lying(), Output::Unicode).result().texts(),
            ["“a”"]
        );
        assert_eq!(t.lint_template(lying()), t.lint_template(honest()));
        assert_eq!(
            t.to_plain_template(lying()).texts(),
            t.to_plain_template(honest()).texts()
        );
    }
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

// Stage 4 (design section 15.4): `Report`, `Change`, `ProtectedSpan`, `Diagnostic`, `lint`.

static UI: Typographer = Typographer::new(Language::RU_RU, RuleSet::INTERFACE);
static PROSE: Typographer = Typographer::new(Language::RU_RU, RuleSet::PROSE);

#[test]
fn the_report_example_of_section_2_9() {
    let r = UI.report("Цена - 5 руб.", Output::Unicode);
    assert_eq!(r.result(), "Цена\u{a0}— 5\u{a0}руб.");
    assert_eq!(
        r.to_string(),
        "\
nbsp.dash           0:8..9    \" \" -> U+00A0
dash.spaced-hyphen  0:9..10   \"-\" -> \"—\"
nbsp.units          0:12..13  \" \" -> U+00A0
"
    );
    let r = UI.report("Он сказал \"привет.", Output::Unicode);
    assert_eq!(r.result(), "Он сказал \"привет.");
    assert_eq!(r.diagnostics()[0].kind(), &DiagnosticKind::UnmatchedQuote);
    assert_eq!(r.diagnostics()[0].range(), 18..19);
}

#[test]
fn the_display_of_section_12_5() {
    // Russian Prose; the result is `Он⍽— «врач», см.⍽host:8080`.
    let r = PROSE.report("Он - \"врач\", см. host:8080", Output::Unicode);
    assert_eq!(r.result(), "Он\u{a0}— «врач», см.\u{a0}host:8080");
    assert_eq!(
        r.to_string(),
        "\
nbsp.dash           0:4..5    \" \" -> U+00A0
dash.spaced-hyphen  0:5..6    \"-\" -> \"—\"
quotes.pairs        0:7..8    \"\\\"\" -> \"«\"
quotes.pairs        0:16..17  \"\\\"\" -> \"»\"
nbsp.abbreviations  0:24..25  \" \" -> U+00A0
protected  Technical(Colon)  0:25..34  host:8080
"
    );
}

#[test]
fn the_display_escapes_every_format_character_of_section_5_4() {
    // Design sections 12.5 and 5.4: a protected text keeps its record on one line and shows
    // every format character escaped, so that an override such as U+202E cannot reorder the
    // printed line.
    let rows = [
        ('\u{ad}', "\\u{ad}"),
        ('\u{200b}', "\\u{200b}"),
        ('\u{200c}', "\\u{200c}"),
        ('\u{200d}', "\\u{200d}"),
        ('\u{200e}', "\\u{200e}"),
        ('\u{200f}', "\\u{200f}"),
        ('\u{202a}', "\\u{202a}"),
        ('\u{202b}', "\\u{202b}"),
        ('\u{202c}', "\\u{202c}"),
        ('\u{202d}', "\\u{202d}"),
        ('\u{202e}', "\\u{202e}"),
        ('\u{2060}', "\\u{2060}"),
        ('\u{2064}', "\\u{2064}"),
        ('\u{2066}', "\\u{2066}"),
        ('\u{2069}', "\\u{2069}"),
        ('\u{feff}', "\\u{feff}"),
    ];
    for (c, escaped) in rows {
        let input = format!("Он - врач `a{c}b - c`");
        let r = UI.report(&input, Output::Unicode);
        let printed = r.to_string();
        let protected: Vec<&str> = printed
            .lines()
            .filter(|line| line.starts_with("protected"))
            .collect();
        let expected = format!("`a{escaped}b - c`");
        assert_eq!(protected.len(), 1, "{c:?}: {printed:?}");
        assert!(
            protected[0].ends_with(&expected),
            "{c:?}: {:?}",
            protected[0]
        );
        assert!(!printed.contains(c), "{c:?}: {printed:?}");
    }
}

#[test]
fn a_report_without_records_prints_nothing() {
    let r = UI.report("Привет, мир.", Output::Unicode);
    assert!(r.changes().is_empty() && r.protected().is_empty() && r.diagnostics().is_empty());
    assert_eq!(r.to_string(), "");
    let r = UI.report_template::<()>([Piece::Text("Привет")], Output::Unicode);
    assert_eq!(r.to_string(), "");
}

#[test]
fn the_display_of_a_template_report_names_the_piece() {
    // One record per edit; the column widths are not fixed here, so compare the fields. The
    // range is in bytes of the piece (section 2.9): `Он ` is five bytes.
    let t = Typographer::new(Language::RU_RU, RuleSet::EMPTY.with(Rule::DashSpacedHyphen));
    let r = t.report_template::<()>(
        [Piece::Markup("<b>"), Piece::Text("Он - врач")],
        Output::Unicode,
    );
    assert_eq!(r.result().texts(), ["<b>Он — врач"]);
    assert_eq!(r.changes().len(), 1);
    let printed = r.to_string();
    let lines: Vec<&str> = printed.lines().collect();
    assert_eq!(lines.len(), 1, "{printed:?}");
    let fields: Vec<&str> = lines[0].split_whitespace().collect();
    assert_eq!(
        fields,
        ["dash.spaced-hyphen", "1:5..6", "\"-\"", "->", "\"—\""]
    );
}

#[test]
fn the_accessors_of_change() {
    let r = UI.report("Цена - 5 руб.", Output::Unicode);
    let changes: Vec<&Change> = r.changes().iter().collect();
    assert_eq!(changes.len(), 3);
    let rows = [
        (Rule::NbspDash, 8..9, "\u{a0}", " "),
        (Rule::DashSpacedHyphen, 9..10, "—", "-"),
        (Rule::NbspUnits, 12..13, "\u{a0}", " "),
    ];
    for (change, (rule, range, replacement, original)) in changes.iter().zip(rows) {
        assert_eq!(change.rule(), rule);
        assert_eq!(change.piece(), 0);
        assert_eq!(change.range(), range);
        assert_eq!(change.replacement(), replacement);
        assert_eq!(r.original(change), original);
    }
    // `Change` is `Clone`, `Debug`, `PartialEq`.
    let first = changes[0].clone();
    assert_eq!(&first, changes[0]);
    assert_ne!(&first, changes[1]);
    assert!(!format!("{first:?}").is_empty());
}

#[test]
fn a_change_holds_unicode_whatever_the_output_kind() {
    // Section 2.9: the replacement is in Unicode, before output encoding.
    let input = "Цена - 5 руб.";
    let unicode = UI.report(input, Output::Unicode);
    for output in [
        Output::Html(HtmlStyle::NAMED),
        Output::Html(HtmlStyle::NAMED.nowrap_class("nowrap")),
        Output::Escaped(typolish::EscapeStyle::JS),
    ] {
        let r = UI.report(input, output);
        assert_eq!(r.changes(), unicode.changes(), "{output:?}");
        assert_eq!(r.protected(), unicode.protected());
        assert_eq!(r.diagnostics(), unicode.diagnostics());
        assert_eq!(r.result().as_ref(), UI.typeset(input, output).as_ref());
    }
    let named = UI.report(input, Output::Html(HtmlStyle::NAMED));
    assert!(named.result().contains("&mdash;"), "{}", named.result());
    assert!(
        named
            .changes()
            .iter()
            .all(|c| !c.replacement().contains('&'))
    );
}

#[test]
fn the_accessors_of_protected_span() {
    // Section 12.5: `kind() == Technical(Colon)`, `piece() == 0`, `range() == 25..34`.
    let r = PROSE.report("Он - \"врач\", см. host:8080", Output::Unicode);
    let spans: &[ProtectedSpan] = r.protected();
    assert_eq!(spans.len(), 1);
    assert_eq!(spans[0].kind(), ProtectedKind::Technical(TechKind::Colon));
    assert_eq!(spans[0].piece(), 0);
    assert_eq!(spans[0].range(), 25..34);
    assert_eq!(spans[0].clone(), spans[0]);
    // A delimited span over two pieces: two `Delimited` records, `0:4..7` and `1:0..5`.
    let r = UI.report_template::<()>(
        [Piece::Text("Run `ls"), Piece::Text(" -la`")],
        Output::Unicode,
    );
    let got: Vec<_> = r
        .protected()
        .iter()
        .map(|s| (s.piece(), s.range(), s.kind()))
        .collect();
    assert_eq!(
        got,
        [
            (0, 4..7, ProtectedKind::Delimited),
            (1, 0..5, ProtectedKind::Delimited)
        ]
    );
    // A slot has the empty range at its piece, a protected piece covers its whole piece.
    let r = UI.report_template(
        [
            Piece::Text("a "),
            Piece::Slot(1u8, Slot::WORD),
            Piece::Protected("--x"),
        ],
        Output::Unicode,
    );
    let got: Vec<_> = r
        .protected()
        .iter()
        .map(|s| (s.piece(), s.range(), s.kind()))
        .collect();
    assert_eq!(
        got,
        [
            (1, 0..0, ProtectedKind::Slot),
            (2, 0..3, ProtectedKind::ProtectedPiece)
        ]
    );
}

#[test]
fn the_accessors_of_diagnostic_and_severity() {
    let r = UI.report("Он сказал \"привет.", Output::Unicode);
    let d: &Diagnostic = &r.diagnostics()[0];
    assert_eq!(d.kind(), &DiagnosticKind::UnmatchedQuote);
    assert_eq!(d.severity(), Severity::Warning);
    assert_eq!(d.rule(), Some(Rule::QuotesPairs));
    assert_eq!(d.piece(), 0);
    assert_eq!(d.range(), 18..19);
    assert_eq!(d.clone(), *d);
    // Severity is `Copy`, ordered, `Info < Warning`.
    assert!(Severity::Info < Severity::Warning);
    assert_eq!(Severity::Info.max(Severity::Warning), Severity::Warning);
    let mut levels = [Severity::Warning, Severity::Info, Severity::Warning];
    levels.sort();
    assert_eq!(
        levels,
        [Severity::Info, Severity::Warning, Severity::Warning]
    );
    // `lint` hands out the same diagnostic.
    let linted: Vec<Diagnostic> = UI.lint("Он сказал \"привет.");
    assert_eq!(linted, r.diagnostics());
    assert_eq!(
        linted[0].to_string(),
        "warning quotes.pairs 0:18..19: straight quote has no pair; left as is"
    );
}

#[test]
fn into_result_gives_what_result_shows() {
    let r = UI.report("Цена - 5 руб.", Output::Unicode);
    let shown = r.result().clone();
    let copy = r.clone();
    assert_eq!(copy.into_result(), shown);
    assert_eq!(r.into_result(), "Цена\u{a0}— 5\u{a0}руб.");
    let r = UI.report_template::<()>([Piece::Text("Цена - 5")], Output::Unicode);
    let texts: Vec<String> = r.result().texts().iter().map(|t| t.to_string()).collect();
    let template = r.into_result();
    assert_eq!(
        template
            .texts()
            .iter()
            .map(|t| t.to_string())
            .collect::<Vec<_>>(),
        texts
    );
}

#[test]
fn original_of_a_template_change_is_the_slice_of_its_piece() {
    // `[Text("Ждём."), Text("..")]` → `Ждём…`: the first record holds the replacement, the other
    // deletes the rest, and `original()` of each returns the slice of its own piece.
    let t = Typographer::new(Language::RU_RU, RuleSet::EMPTY.with(Rule::EllipsisDots));
    let r = t.report_template::<()>([Piece::Text("Ждём."), Piece::Text("..")], Output::Unicode);
    assert_eq!(r.result().texts(), ["Ждём…"]);
    let got: Vec<_> = r
        .changes()
        .iter()
        .map(|c| {
            (
                c.piece(),
                c.range(),
                c.replacement().to_owned(),
                r.original(c),
            )
        })
        .collect();
    assert_eq!(
        got,
        [
            (0, 8..9, "…".to_owned(), "."),
            (1, 0..2, String::new(), "..")
        ]
    );
}

#[test]
fn mixed_script_over_two_pieces_is_one_diagnostic_in_both_calls() {
    // Section 13.2: `Text("Мо")`, `Text("sква")` → one diagnostic, `piece() == 0`, `range() == 0..4`,
    // equal in `report_template` and `lint_template`.
    let pieces = || [Piece::Text("Мо"), Piece::Text("sква")];
    let r = UI.report_template::<()>(pieces(), Output::Unicode);
    let linted = UI.lint_template::<()>(pieces());
    assert_eq!(r.diagnostics().len(), 1);
    assert_eq!(r.diagnostics()[0].kind(), &DiagnosticKind::MixedScript);
    assert_eq!(r.diagnostics()[0].piece(), 0);
    assert_eq!(r.diagnostics()[0].range(), 0..4);
    assert_eq!(linted, r.diagnostics());
}

#[test]
fn lint_and_report_agree_without_typographic_characters() {
    for text in [
        "Он сказал \"привет.",
        "Цена - 5 руб.",
        "Слово \" в тексте.",
        "10 - 20 штук",
        "don`t",
    ] {
        let linted = UI.lint(text);
        let r = UI.report(text, Output::Unicode);
        assert_eq!(linted, r.diagnostics(), "{text}");
    }
}
