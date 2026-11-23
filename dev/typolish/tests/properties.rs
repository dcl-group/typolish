//! The properties of design section 13.5 that hold at every stage and need only the public API:
//! idempotence, the fixed-point oracle, slots and protected spans, letters and digits, `Cow`,
//! pieces and strings, determinism of order and no panic; and, through `report`, the invariants I1,
//! I2, I4 and I5 of design section 4.3 as far as the public API shows them. The properties that
//! need `lint` join when it exists; those of `to_plain` (14 and 16) are at the end. Stage 3 adds the decoder properties (12),
//! the safe alphabet (17), I3, and the properties of `format_number`.

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
use common::strategies::{self, GenPiece, any_text, borrow, pieces, risky_text, rule_set};
use proptest::prelude::*;
use typolish::{
    DiagnosticKind, EscapeStyle, HtmlStyle, Language, LanguageBuilder, Output, PercentSpacing,
    ProtectedKind, QuotePairs, Report, Rule, RuleSet, Severity, SingleQuotes, Typographer,
    TypographerBuilder,
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

/// The seeds of the idempotence property of section 13.5 that need no `LanguageBuilder`.
const SEEDS: &[&str] = &[
    "Школа №5 г. Москвы",
    "§5 ч. 2",
    "т.е.",
    "и т.д...",
    // A compact abbreviation next to a dash reads as a word in both forms (section 8.2).
    "Он пришёл - т.е. опоздал",
    "Слово - т.е.",
    "Москва – т.е. столица",
    "Слово -- т.д.",
    "5 - т.е.",
    "т.е. - 5",
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
    // A percentage range and a percent sign after spaces next to a dash (section 8.2).
    "5–10%",
    "5–10 %",
    "100 % - норма",
    "5–10% - норма",
    "Рост - 5–10%",
    // A label number owns its percent sign after spaces too (section 8.2).
    "№5 % — т.е.",
    "№10 % — -5% +",
    "№5 %\u{a0}-- `x`",
    "§5 % - x",
    // A percentage after spaces as the left factor of a lone `x` and next to `(c)` (8.8, 8.9).
    "10 % x 5",
    "10 %... X\u{a0}10 %",
    "(c) 2026 %",
    "(c) 2026%",
    "№2026 (c)",
    "word\u{a0}- word",
    "Sign in - it's free",
    "Подождите...\nДальше",
    "Экран \u{2068}27\u{2069}\" и \"ручка\"",
    // A sign-only pair next to a dash (sections 6.3 and 8.2).
    "Слово - \"\" да",
    "\"\" - 5",
    "- \"\" да",
    "-- \"\"",
    "Введите \"-\" или \"—\"",
    // A series of three or more quotes of any form has no sign-only pair inside it (section 6.3).
    "\"--\"\"\"",
    // A neutral typed closer inside a new pair of its family (section 6.6).
    "x '\"»b 5' c\"",
    "'\"»90'\"",
    "’'\"”»90'„\"",
    "\"a » b\"",
    "''''- a' b'",
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

#[test]
fn idempotence_of_the_seed_with_a_built_language() {
    // The seed `‚Wort‘` of section 13.5, with a `LanguageBuilder` language whose pairs are `„“` and
    // `‚‘`: a typed nested pair, and around it straight quotes that take the language's glyphs.
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
    let t = Typographer::new(de, RuleSet::INTERFACE);
    assert_eq!(t.typeset("‚Wort‘", Output::Unicode), "‚Wort‘");
    assert_eq!(
        t.typeset("\"Er sagte \"Wort\"\"", Output::Unicode),
        "„Er sagte ‚Wort‘“"
    );
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
        prop_assert_eq!(&once, &twice, "{:?} -> {:?}", text, once);
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

// Stage 3 (design section 15.4): the output kinds, I3 and the numbers.

fn is_css(output: Output) -> bool {
    matches!(output, Output::Escaped(s) if s == EscapeStyle::CSS || s == EscapeStyle::CSS.non_ascii())
}

fn is_html(output: Output) -> bool {
    matches!(output, Output::Html(_))
}

/// Section 11.8: for `CSS` every U+0000 of the generated text is replaced by U+FFFD before both
/// equations.
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
    /// and close in another, section 11.5). `Markup` is written as it is in HTML, so the HTML kinds
    /// run on templates without it.
    #[test]
    fn template_texts_decode_to_the_unicode_texts(
        parts in pieces(), lang in 0usize..3, set in rule_set(),
    ) {
        let t = typographer(lang, set, 0, 0);
        let input = borrow(&parts);
        let has_markup = parts.iter().any(|p| matches!(p, GenPiece::Markup(_)));
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

    /// 8. Cow for templates: a text maps to an unchanged input piece exactly when it is borrowed,
    /// and `changed()` says whether any text differs from its input, in every kind.
    #[test]
    fn template_changed_agrees_with_the_texts(
        parts in pieces(), lang in 0usize..3, set in rule_set(),
    ) {
        let t = typographer(lang, set, 0, 0);
        let input = borrow(&parts);
        for output in all_outputs() {
            let template = t.typeset_template(input.iter().copied(), output);
            // A template of one text piece is borrowed exactly when unchanged.
            if let [typolish::Piece::Text(only)] = input.as_slice() {
                prop_assert_eq!(
                    matches!(template.texts()[0], Cow::Borrowed(_)),
                    template.texts()[0] == *only,
                    "{:?} {:?}", output, only
                );
                prop_assert_eq!(template.changed(), template.texts()[0] != *only);
            }
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
/// grouped by the symbols: an independent model of section 2.7.
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
        lang in 0usize..3, digits in 0u8..5, i in any::<i128>(), u in any::<u128>(), f in any::<f64>(),
        g in any::<f32>(), d in "[+-]?[0-9]{1,40}(\\.[0-9]{1,20})?", places in 0u8..12,
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
        }
        if g.is_finite() {
            let zero = g == 0.0;
            prop_assert_eq!(show(Number::from(g)), model(&format!("{g}"), lang, digits, zero));
        }
        let parsed = Number::decimal(&d).unwrap();
        let zero = d.trim_start_matches(['+', '-']).chars().all(|c| matches!(c, '0' | '.'));
        prop_assert_eq!(show(parsed), model(&d, lang, digits, zero));
    }

    /// Output kinds of numbers: `decode(format_number(n, out))` is the Unicode number, and
    /// `format_range` is the two numbers around the range dash.
    #[test]
    fn numbers_decode_in_every_kind(
        lang in 0usize..3, digits in 0u8..5, i in any::<i64>(), f in any::<f64>(), j in any::<i128>(),
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
