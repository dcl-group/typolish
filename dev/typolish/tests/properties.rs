//! The properties of design section 13.5 that hold at every stage and need only the public API:
//! idempotence, the fixed-point oracle, slots and protected spans, letters and digits, `Cow`,
//! pieces and strings, determinism of order and no panic. The properties that need `report`,
//! `lint`, `to_plain`, the decoders of the output kinds or `LanguageBuilder` join when those parts
//! exist.

#![allow(clippy::unwrap_used, clippy::panic, clippy::indexing_slicing)]

mod common;

use std::borrow::Cow;

use common::show;
use common::strategies::{self, GenPiece, any_text, borrow, pieces, risky_text, rule_set};
use proptest::prelude::*;
use typolish::{
    EscapeStyle, HtmlStyle, Language, Output, PercentSpacing, Rule, RuleSet, SingleQuotes,
    Typographer,
};

const LANGUAGES: [Language; 3] = [Language::RU_RU, Language::EN_US, Language::COMMON];

fn typographer(lang: usize, set: RuleSet, quotes: u8, percent: u8) -> Typographer {
    let mode = match quotes % 3 {
        0 => SingleQuotes::NestedOnly,
        1 => SingleQuotes::Everywhere,
        _ => SingleQuotes::Never,
    };
    let spacing = match percent % 4 {
        0 => PercentSpacing::AsWritten,
        1 => PercentSpacing::Language,
        2 => PercentSpacing::Attached,
        _ => PercentSpacing::Spaced,
    };
    Typographer::builder(LANGUAGES[lang % 3].clone(), set)
        .single_quotes(mode)
        .percent(spacing)
        .build()
        .unwrap()
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

/// The seeds of the idempotence property of section 13.5 that need no `LanguageBuilder`.
const SEEDS: &[&str] = &[
    "Школа №5 г. Москвы",
    "§5 ч. 2",
    "т.е.",
    "и т.д...",
    "Moscow - the capital",
    "See https://example.com/docs - the guide",
    "He said - \"a cat\"",
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
    "word\u{a0}- word",
    "Sign in - it's free",
    "Подождите...\nДальше",
    "Экран \u{2068}27\u{2069}\" и \"ручка\"",
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

proptest! {
    #![proptest_config(ProptestConfig::with_cases(512))]

    /// 1. Idempotence and 2. the fixed-point oracle.
    #[test]
    fn idempotence_and_fixed_point(
        text in any_text(), lang in 0usize..3, set in rule_set(), quotes in 0u8..3, percent in 0u8..4,
    ) {
        let t = typographer(lang, set, quotes, percent);
        let once = t.typeset(&text, Output::Unicode).into_owned();
        let twice = t.typeset(&once, Output::Unicode).into_owned();
        prop_assert_eq!(&once, &twice);
        let mut current = text.clone();
        for _ in 0..3 {
            let next = t.typeset(&current, Output::Unicode).into_owned();
            if next == current {
                break;
            }
            current = next;
        }
        prop_assert_eq!(&current, &once);
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
                GenPiece::Protected(text) | GenPiece::Markup(text) => text.clone(),
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
    let _ = strategies::RISKY.len();
}
