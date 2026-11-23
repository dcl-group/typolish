//! `Rule` and `RuleSet` against the literal tables of design sections 2.3, 4.1 and 9.2.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    clippy::print_stdout,
    clippy::print_stderr,
    clippy::dbg_macro
)]

use typolish::{Error, Rule, RuleSet};

/// The ids of design section 4.1 in execution order.
const IDS: [&str; 27] = [
    "symbols.copyright",
    "symbols.arrows",
    "symbols.math",
    "ellipsis.dots",
    "ellipsis.inner",
    "dash.dialogue",
    "dash.double-hyphen",
    "dash.spaced-hyphen",
    "range.placeholders",
    "range.digits",
    "range.context",
    "range.time",
    "number.minus",
    "number.times",
    "quotes.apostrophe",
    "quotes.pairs",
    "quotes.primes",
    "nbsp.abbreviations",
    "nbsp.initials",
    "nbsp.dates",
    "nbsp.units",
    "nbsp.signs",
    "nbsp.phone",
    "nbsp.short-words",
    "nbsp.short-length",
    "nbsp.particles",
    "nbsp.dash",
];

const INTERFACE_IDS: [&str; 13] = [
    "ellipsis.dots",
    "dash.spaced-hyphen",
    "range.placeholders",
    "quotes.apostrophe",
    "quotes.pairs",
    "nbsp.abbreviations",
    "nbsp.initials",
    "nbsp.dates",
    "nbsp.units",
    "nbsp.signs",
    "nbsp.short-words",
    "nbsp.particles",
    "nbsp.dash",
];

fn set(ids: &[&str]) -> RuleSet {
    ids.iter().map(|id| id.parse::<Rule>().unwrap()).collect()
}

#[test]
fn rule_all_is_the_order_of_section_4_1() {
    let ids: Vec<&str> = Rule::ALL.iter().map(|r| r.id()).collect();
    assert_eq!(ids, IDS);
}

#[test]
fn rule_variants_are_declared_in_execution_order() {
    assert!(Rule::ALL.windows(2).all(|w| w[0] < w[1]));
    assert!(Rule::SymbolsCopyright < Rule::NbspDash);
    assert_eq!(Rule::ALL.first(), Some(&Rule::SymbolsCopyright));
    assert_eq!(Rule::ALL.last(), Some(&Rule::NbspDash));
}

#[test]
fn variants_have_their_ids() {
    for (rule, id) in [
        (Rule::SymbolsCopyright, "symbols.copyright"),
        (Rule::SymbolsArrows, "symbols.arrows"),
        (Rule::SymbolsMath, "symbols.math"),
        (Rule::EllipsisDots, "ellipsis.dots"),
        (Rule::EllipsisInner, "ellipsis.inner"),
        (Rule::DashDialogue, "dash.dialogue"),
        (Rule::DashDoubleHyphen, "dash.double-hyphen"),
        (Rule::DashSpacedHyphen, "dash.spaced-hyphen"),
        (Rule::RangePlaceholders, "range.placeholders"),
        (Rule::RangeDigits, "range.digits"),
        (Rule::RangeContext, "range.context"),
        (Rule::RangeTime, "range.time"),
        (Rule::NumberMinus, "number.minus"),
        (Rule::NumberTimes, "number.times"),
        (Rule::QuotesApostrophe, "quotes.apostrophe"),
        (Rule::QuotesPairs, "quotes.pairs"),
        (Rule::QuotesPrimes, "quotes.primes"),
        (Rule::NbspAbbreviations, "nbsp.abbreviations"),
        (Rule::NbspInitials, "nbsp.initials"),
        (Rule::NbspDates, "nbsp.dates"),
        (Rule::NbspUnits, "nbsp.units"),
        (Rule::NbspSigns, "nbsp.signs"),
        (Rule::NbspPhone, "nbsp.phone"),
        (Rule::NbspShortWords, "nbsp.short-words"),
        (Rule::NbspShortLength, "nbsp.short-length"),
        (Rule::NbspParticles, "nbsp.particles"),
        (Rule::NbspDash, "nbsp.dash"),
    ] {
        assert_eq!(rule.id(), id);
        assert_eq!(rule.to_string(), id);
        assert_eq!(Rule::from_id(id), Some(rule));
        assert_eq!(id.parse::<Rule>(), Ok(rule));
    }
}

#[test]
fn unknown_ids_are_errors() {
    for bad in [
        "",
        "nbsp",
        "nbsp.",
        "NBSP.DASH",
        "nbsp.dash ",
        " nbsp.dash",
        "nbsp_dash",
        "nbsp.nothing",
    ] {
        assert_eq!(Rule::from_id(bad), None, "{bad:?}");
        assert!(
            matches!(bad.parse::<Rule>(), Err(Error::UnknownRule { id, .. }) if id == bad),
            "{bad:?}"
        );
    }
}

#[test]
fn summaries_are_one_english_line() {
    for rule in Rule::ALL {
        let summary = rule.summary();
        assert!(!summary.trim().is_empty(), "{rule}");
        assert!(!summary.contains('\n'), "{rule}");
    }
}

#[test]
fn rule_set_constants() {
    assert!(RuleSet::EMPTY.is_empty());
    assert_eq!(RuleSet::EMPTY.len(), 0);
    assert_eq!(RuleSet::EMPTY.iter().count(), 0);
    assert_eq!(RuleSet::ALL.len(), 27);
    assert_eq!(RuleSet::PROSE.len(), 27);
    assert_eq!(RuleSet::INTERFACE.len(), 13);
    assert!(!RuleSet::ALL.is_empty());
    assert_eq!(RuleSet::default(), RuleSet::EMPTY);
}

#[test]
fn prose_holds_all_rules_of_version_one() {
    assert_eq!(RuleSet::PROSE, set(&IDS));
    assert_eq!(RuleSet::PROSE, RuleSet::ALL);
    for rule in Rule::ALL {
        assert!(RuleSet::PROSE.contains(*rule), "{rule}");
        assert!(RuleSet::ALL.contains(*rule), "{rule}");
        assert!(!RuleSet::EMPTY.contains(*rule), "{rule}");
    }
}

#[test]
fn interface_is_section_9_2() {
    assert_eq!(RuleSet::INTERFACE, set(&INTERFACE_IDS));
    let removed = [
        "symbols.copyright",
        "symbols.arrows",
        "symbols.math",
        "ellipsis.inner",
        "dash.dialogue",
        "dash.double-hyphen",
        "range.digits",
        "range.context",
        "range.time",
        "number.minus",
        "number.times",
        "quotes.primes",
        "nbsp.phone",
        "nbsp.short-length",
    ];
    assert_eq!(removed.len(), 14);
    assert_eq!(RuleSet::PROSE.difference(set(&removed)), RuleSet::INTERFACE);
    for id in removed {
        assert!(!RuleSet::INTERFACE.contains(id.parse().unwrap()), "{id}");
    }
}

#[test]
fn interface_iterates_in_execution_order() {
    let ids: Vec<&str> = RuleSet::INTERFACE.iter().map(|r| r.id()).collect();
    assert_eq!(ids, INTERFACE_IDS);
    let ids: Vec<&str> = RuleSet::PROSE.iter().map(|r| r.id()).collect();
    assert_eq!(ids, IDS);
}

#[test]
fn rule_sets_are_const() {
    const WITH: RuleSet = RuleSet::INTERFACE.with(Rule::NumberTimes);
    const WITHOUT: RuleSet = RuleSet::PROSE.without(Rule::NbspDash);
    const UNION: RuleSet = RuleSet::EMPTY.union(RuleSet::INTERFACE);
    const DIFF: RuleSet = RuleSet::ALL.difference(RuleSet::INTERFACE);
    const LEN: usize = RuleSet::INTERFACE.len();
    const HAS: bool = RuleSet::INTERFACE.contains(Rule::NbspDash);
    const EMPTY: bool = RuleSet::EMPTY.is_empty();
    assert_eq!(WITH.len(), 14);
    assert!(WITH.contains(Rule::NumberTimes));
    assert_eq!(WITHOUT.len(), 26);
    assert_eq!(UNION, RuleSet::INTERFACE);
    assert_eq!(DIFF.len(), 14);
    assert_eq!(LEN, 13);
    const { assert!(HAS) };
    const { assert!(EMPTY) };
}

#[test]
fn set_algebra() {
    let a = RuleSet::EMPTY.with(Rule::NbspDash).with(Rule::NbspDash);
    assert_eq!(a.len(), 1);
    assert_eq!(a.without(Rule::NbspDash), RuleSet::EMPTY);
    assert_eq!(a.without(Rule::NbspUnits), a);
    let b = RuleSet::EMPTY.with(Rule::NbspUnits);
    assert_eq!(a.union(b), b.union(a));
    assert_eq!(a.union(b).difference(a), b);
    assert_eq!(a.difference(b), a);
    assert_eq!(RuleSet::ALL.difference(RuleSet::ALL), RuleSet::EMPTY);
}

#[test]
fn order_does_not_depend_on_how_a_set_was_built() {
    let forward = RuleSet::EMPTY
        .with(Rule::SymbolsMath)
        .with(Rule::NbspDash)
        .with(Rule::QuotesPairs);
    let backward = RuleSet::EMPTY
        .with(Rule::QuotesPairs)
        .with(Rule::NbspDash)
        .with(Rule::SymbolsMath);
    assert_eq!(forward, backward);
    assert_eq!(forward.to_string(), backward.to_string());
    assert_eq!(forward.to_string(), "symbols.math,quotes.pairs,nbsp.dash");
}

#[test]
fn display_is_ids_in_order_or_empty() {
    assert_eq!(RuleSet::EMPTY.to_string(), "empty");
    assert_eq!(RuleSet::PROSE.to_string(), IDS.join(","));
    assert_eq!(RuleSet::INTERFACE.to_string(), INTERFACE_IDS.join(","));
    assert_eq!(RuleSet::EMPTY.with(Rule::NbspDash).to_string(), "nbsp.dash");
}

#[test]
fn base_names_parse() {
    assert_eq!("prose".parse::<RuleSet>(), Ok(RuleSet::PROSE));
    assert_eq!("interface".parse::<RuleSet>(), Ok(RuleSet::INTERFACE));
    assert_eq!("empty".parse::<RuleSet>(), Ok(RuleSet::EMPTY));
    assert_eq!("all".parse::<RuleSet>(), Ok(RuleSet::ALL));
    assert_eq!(
        "interface,+number.times,-nbsp.initials".parse::<RuleSet>(),
        Ok(RuleSet::INTERFACE
            .with(Rule::NumberTimes)
            .without(Rule::NbspInitials))
    );
}

#[test]
fn from_str_is_independent_of_item_order() {
    let a = "nbsp.dash,-nbsp.dash".parse::<RuleSet>();
    assert!(matches!(a, Err(Error::InvalidRuleSet { .. })));
    let a = "+nbsp.units,quotes.pairs,-nbsp.dash".parse::<RuleSet>();
    let b = "-nbsp.dash,quotes.pairs,+nbsp.units".parse::<RuleSet>();
    assert_eq!(a, b);
    assert_eq!(a, Ok(set(&["nbsp.units", "quotes.pairs"])));
}

#[test]
fn the_first_item_in_error_decides() {
    assert!(matches!(
        "nbsp.nothing,,prose".parse::<RuleSet>(),
        Err(Error::UnknownRule { id, .. }) if id == "nbsp.nothing"
    ));
    assert!(matches!(
        "prose,,nbsp.nothing".parse::<RuleSet>(),
        Err(Error::InvalidRuleSet {
            reason: "empty item",
            ..
        })
    ));
    // The check for an id both added and removed runs after every item has been read.
    assert!(matches!(
        "+nbsp.dash,-nbsp.dash,nbsp.nothing".parse::<RuleSet>(),
        Err(Error::UnknownRule { .. })
    ));
}

#[test]
fn whitespace_is_read_as_str_trim_does() {
    assert_eq!(
        "\tprose\u{a0}, -nbsp.dash \n".parse::<RuleSet>(),
        Ok(RuleSet::PROSE.without(Rule::NbspDash))
    );
}
