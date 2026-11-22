//! The properties of design section 13.5 that hold at every stage and need only the public API:
//! idempotence, the fixed-point oracle, slots and protected spans, letters and digits, `Cow`,
//! pieces and strings, determinism of order and no panic; and, through `report`, the invariants I1,
//! I2, I4 and I5 of design section 4.3 as far as the public API shows them. The properties that
//! need `lint`, `to_plain`, the decoders of the output kinds or `LanguageBuilder` join when those
//! parts exist.

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

use common::show;
use common::strategies::{self, GenPiece, any_text, borrow, pieces, risky_text, rule_set};
use proptest::prelude::*;
use typolish::{
    DiagnosticKind, EscapeStyle, HtmlStyle, Language, Output, PercentSpacing, ProtectedKind,
    Report, Rule, RuleSet, Severity, SingleQuotes, Typographer,
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

// The invariants of section 4.3 (properties 3 of section 13.5), observed through `report`.
//
// Before stage 3 the sets run without the `nbsp.*` rules, as the set-level tables do (section
// 15.4); stage 3 lifts the filter where the invariant allows it (I2 with a percent sign that a
// space splits, section 4.3, keeps it).

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

    /// I2: the fragments that the classifier freezes in the output of a pass are those of its
    /// input: the kinds of the protected spans of `f(x)` equal those of `x`.
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

    /// I4: matching by class. On the output of a pass no rule has an edit left: the report of
    /// `f(x)` holds no change.
    #[test]
    fn i4_the_second_pass_has_no_edit(text in any_text(), lang in 0usize..3, set in stage2_set(), quotes in 0u8..3) {
        let t = typographer(lang, set, quotes, 0);
        let out = t.typeset(&text, Output::Unicode).into_owned();
        let second = t.report(&out, Output::Unicode);
        prop_assert!(second.changes().is_empty(), "{:?} -> {:?} -> {:?}", text, out, second.changes());
    }

    /// I5: quotes are all or nothing per window. In a window that does not converge (an
    /// `UnmatchedQuote` warning or `CrossingQuotes`) no straight `"` changed, and a straight `'`
    /// changed only by step 1 (an apostrophe or a prime). The text has no line break: one window.
    #[test]
    fn i5_a_window_that_does_not_converge_keeps_its_quotes(
        text in quote_text(), lang in 0usize..2, bits in any::<u8>(), mode in 0u8..3,
    ) {
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
        let report = t.report(&text, Output::Unicode);
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
                        "{:?} -> {:?}", text, report.result()
                    );
                }
            }
        }
    }
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
