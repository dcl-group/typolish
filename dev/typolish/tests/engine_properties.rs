//! Properties of the lexer, the classifier and protection through `report` (design sections
//! 3.2 to 3.7, 10.3, 13.5): no panic on arbitrary strings, well-formed spans and diagnostics,
//! protection independent of the rule set, `report` equal to `typeset` (the prefilter changes
//! nothing), protected and frozen text intact in the output, graphemes, and the pathological
//! inputs of section 3.12 at a size that a normal run affords.
//!
//! `report` always runs the whole pipeline, so unlike `typeset` it reaches the lexer and the
//! classifier on strings that carry no trigger byte (section 3.2).

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
#[path = "common/probe.rs"]
mod probe;

use std::borrow::Cow;

use common::show;
use common::strategies::{
    PROTECTION_CONFIGS, any_text, delimiter_text, protection_typographer, risky_text, rule_set,
};
use probe::{SETS, check_clusters, spans_of};
use proptest::prelude::*;
use typolish::{DiagnosticKind, Language, Output, ProtectedKind, RuleSet, Typographer};

const LANGUAGES: [Language; 3] = [Language::RU_RU, Language::EN_US, Language::COMMON];

/// Compound words of design section 8.2: their hyphens and letters stand byte for byte (design
/// section 13.5, property 5).
const COMPOUNDS: &[&str] = &[
    "кто-то",
    "tls-server",
    "agent-ca",
    "pre-commit",
    "db-01",
    "25-й",
    "123-45-67",
    "mother-in-law",
    "a-b-c",
    "IT-отдел",
];

fn typographer(lang: usize, set: RuleSet) -> Typographer {
    Typographer::new(LANGUAGES[lang % 3].clone(), set)
}

/// A string over ASCII and the characters that glue to a neighbor (design section 13.6): no
/// backtick, so that no delimiter can stand inside a cluster.
fn cluster_text() -> impl Strategy<Value = String> {
    const PARTS: &[&str] = &[
        "a",
        "b",
        "1",
        "2",
        "\"",
        "'",
        "-",
        "--",
        ".",
        "...",
        " ",
        "  ",
        "\r\n",
        "\n",
        "\r",
        ":",
        "/",
        "@",
        "=",
        "<",
        ">",
        ",",
        ";",
        "(",
        ")",
        "x",
        "я",
        "Ж",
        "\u{600}",
        "\u{6dd}",
        "\u{301}",
        "\u{300}",
        "\u{200d}",
        "\u{200c}",
        "\u{903}",
        "\u{fe0f}",
        "\u{20e3}",
        "\u{1f600}",
        "\u{1f1f7}",
        "\u{1f1fa}",
        "\u{1100}",
        "\u{1161}",
        "\u{11a8}",
        "\u{ad}",
        "\u{85}",
        "\u{2028}",
        "\u{2029}",
        "\u{a0}",
        "\u{202f}",
        "\u{2068}",
        "\u{2069}",
        "(c)",
        "№",
        "§",
        "\u{3000}",
        "\u{0}",
        "\u{7f}",
        "е\u{301}",
        "\u{1f468}\u{200d}\u{1f469}",
    ];
    prop::collection::vec(prop::sample::select(PARTS), 0..16).prop_map(|v| v.concat())
}

/// Finds the texts of `needles` in `haystack`, in order and without overlap.
fn in_order(haystack: &str, needles: &[&str]) -> bool {
    let mut at = 0;
    for needle in needles {
        match haystack[at..].find(needle) {
            Some(i) => at += i + needle.len(),
            None => return false,
        }
    }
    true
}

/// The structure of a report of a string: the spans and the diagnostics are slices of the input,
/// in order, and the result equals what `typeset` returns.
fn assert_report(t: &Typographer, text: &str) -> Result<(), TestCaseError> {
    let report = t.report(text, Output::Unicode);

    // Section 9 of 13.5: `report` and `typeset` are one engine.
    let typeset = t.typeset(text, Output::Unicode);
    prop_assert_eq!(report.result().as_ref(), typeset.as_ref());
    prop_assert_eq!(
        matches!(report.result(), Cow::Borrowed(_)),
        report.result() == text,
        "Cow"
    );

    let mut end = 0;
    let mut texts: Vec<&str> = Vec::new();
    for span in report.protected() {
        let range = span.range();
        prop_assert_eq!(span.piece(), 0);
        prop_assert!(
            text.get(range.clone()).is_some(),
            "span {:?} of {:?}",
            range,
            text
        );
        prop_assert!(
            range.start >= end,
            "spans overlap in {:?}: {:?}",
            text,
            report.protected()
        );
        end = range.end;
        texts.push(&text[range]);
    }

    // Property 4 and 5: protected spans and frozen fragments stand in the output byte for byte,
    // in order.
    prop_assert!(
        in_order(report.result(), &texts),
        "protected text changed: {:?} -> {:?}, {:?}",
        text,
        report.result(),
        texts
    );

    let mut last = (0, 0, 0);
    for d in report.diagnostics() {
        let range = d.range();
        prop_assert_eq!(d.piece(), 0);
        prop_assert!(
            text.get(range.clone()).is_some(),
            "diagnostic {:?} of {:?}",
            range,
            text
        );
        let key = (d.piece(), range.start, range.end);
        prop_assert!(key >= last, "diagnostics out of order in {:?}", text);
        last = key;
    }

    for change in report.changes() {
        let range = change.range();
        prop_assert!(
            text.get(range.clone()).is_some(),
            "change {:?} of {:?}",
            range,
            text
        );
    }
    Ok(())
}

/// A string for property 14: arbitrary Unicode, risky pieces, clusters, control characters, and
/// the delimiters of own protection pairs.
fn no_panic_text() -> impl Strategy<Value = String> {
    prop_oneof![
        any_text(),
        "\\PC*|.*|\\p{Cc}*",
        cluster_text(),
        delimiter_text(),
    ]
}

/// A typographer for property 14: the default protection or one of the own-pair
/// configurations.
fn no_panic_typographer(config: usize, lang: usize, set: RuleSet) -> Typographer {
    if config == PROTECTION_CONFIGS {
        typographer(lang, set)
    } else {
        protection_typographer(config, LANGUAGES[lang % 3].clone(), set)
    }
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(10_000))]

    /// 14. No panic in the lexer and the classifier on arbitrary strings, with well-formed
    /// results: 10 000 cases in a normal run (design section 13.5).
    #[test]
    fn report_never_panics_and_is_well_formed(
        text in any_text(), lang in 0usize..3, set in rule_set(),
    ) {
        assert_report(&typographer(lang, set), &text)?;
    }

    #[test]
    fn report_never_panics_on_arbitrary_unicode(text in "\\PC*|.*|\\p{Cc}*", lang in 0usize..3) {
        assert_report(&typographer(lang, RuleSet::ALL), &text)?;
    }

    /// Own protection pairs and backticks off (design section 10.1): the delimiter matching of
    /// `protect_between` and `protect_backticks(false)` on generated input.
    #[test]
    fn report_never_panics_with_own_protection_pairs(
        text in prop_oneof![delimiter_text(), any_text()],
        config in 0usize..PROTECTION_CONFIGS,
        lang in 0usize..3,
        set in rule_set(),
    ) {
        assert_report(&protection_typographer(config, LANGUAGES[lang].clone(), set), &text)?;
    }
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(1_000_000))]

    /// 14. The long run of design section 13.5: 1 000 000 strings through `report`, so the lexer
    /// and the classifier run whatever the trigger bytes. Run with `cargo test -- --ignored`.
    #[test]
    #[ignore = "long run of design section 13.5, property 14"]
    fn no_panic_on_arbitrary_strings_long(
        text in no_panic_text(),
        config in 0usize..=PROTECTION_CONFIGS,
        lang in 0usize..3,
        set in rule_set(),
    ) {
        assert_report(&no_panic_typographer(config, lang, set), &text)?;
    }
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(4000))]

    #[test]
    fn report_never_panics_on_cluster_text(
        text in cluster_text(), lang in 0usize..3, set in rule_set(),
    ) {
        assert_report(&typographer(lang, set), &text)?;
    }

    /// The empty set has no rule: nothing changes, and the output is borrowed.
    #[test]
    fn the_empty_set_changes_nothing(text in any_text(), lang in 0usize..3) {
        let t = typographer(lang, RuleSet::EMPTY);
        let report = t.report(&text, Output::Unicode);
        prop_assert_eq!(report.result().as_ref(), text.as_str());
        prop_assert!(matches!(report.result(), Cow::Borrowed(_)));
        prop_assert!(report.changes().is_empty());
    }

    /// Section 10.2: protection does not depend on the set.
    #[test]
    fn protection_does_not_depend_on_the_set(text in any_text(), lang in 0usize..3) {
        let first = spans_of(&typographer(lang, SETS[0]), &text);
        for set in &SETS[1..] {
            prop_assert_eq!(&spans_of(&typographer(lang, *set), &text), &first, "{}", set);
        }
    }

    /// 7. Graphemes: protected fragments stand on cluster boundaries, edits too, and untouched
    /// clusters are written back.
    #[test]
    fn spans_and_edits_are_on_cluster_boundaries(
        text in cluster_text(), lang in 0usize..3, set in rule_set(),
    ) {
        check_clusters(&typographer(lang, set), "property", &text);
    }

    /// 5. Compound words (design section 8.2) stand in the output byte for byte, between any
    /// risky texts, in every language and set.
    #[test]
    fn compound_words_stand_byte_for_byte(
        before in risky_text(), after in risky_text(), w in prop::sample::select(COMPOUNDS),
        lang in 0usize..3, set in rule_set(),
    ) {
        let text = format!("{before} {w} {after}");
        let out = typographer(lang, set).typeset(&text, Output::Unicode);
        prop_assert!(
            out.matches(w).count() >= text.matches(w).count(),
            "{} -> {}",
            show(&text),
            show(&out)
        );
    }

    /// Diagnostics of the classifier and of protection come from `report`, never from a trigger
    /// byte: the diagnostics of a string do not depend on the set.
    #[test]
    fn diagnostics_of_the_classifier_do_not_depend_on_the_set(text in risky_text(), lang in 0usize..3) {
        let kinds = |set: RuleSet| -> Vec<(DiagnosticKind, std::ops::Range<usize>)> {
            typographer(lang, set)
                .report(&text, Output::Unicode)
                .diagnostics()
                .iter()
                .filter(|d| {
                    matches!(
                        d.kind(),
                        DiagnosticKind::MixedScript | DiagnosticKind::UnclosedDelimiter | DiagnosticKind::SlotGlue
                    )
                })
                .map(|d| (d.kind().clone(), d.range()))
                .collect()
        };
        let first = kinds(RuleSet::EMPTY);
        for set in [RuleSet::INTERFACE, RuleSet::PROSE, RuleSet::ALL] {
            prop_assert_eq!(kinds(set), first.clone());
        }
    }

    /// A frame around a string changes the classification of the string only through its
    /// neighbors: a space on both sides makes fragments of their own, so the spans of a string
    /// without spaces and line breaks are the spans it has alone, shifted.
    #[test]
    fn a_fragment_keeps_its_spans_between_spaces(
        text in "[a-zA-Z0-9:/@._,;=<>#$%+*&|^~!?\\\\-]{0,24}", lang in 0usize..3,
    ) {
        let t = typographer(lang, RuleSet::INTERFACE);
        let alone = spans_of(&t, &text);
        let framed_text = format!("a {text} b");
        let framed = spans_of(&t, &framed_text);
        // The frame words `a` and `b` are words, never frozen.
        let shifted: Vec<_> = alone.iter().map(|(k, p, r)| (*k, *p, r.start + 2..r.end + 2)).collect();
        prop_assert_eq!(framed, shifted, "{}", show(&framed_text));
    }
}

// Pathological inputs of section 3.12 (the load tests of section 13.8 measure them; here they
// must finish, with a sane report).

fn big(unit: &str, bytes: usize) -> String {
    unit.repeat(bytes / unit.len().max(1))
}

fn all_typographers() -> Vec<Typographer> {
    let mut out = Vec::new();
    for language in LANGUAGES {
        for set in [RuleSet::EMPTY, RuleSet::INTERFACE, RuleSet::PROSE] {
            out.push(Typographer::new(language.clone(), set));
        }
    }
    out
}

#[test]
fn pathological_runs_finish() {
    let inputs: Vec<(&str, String)> = vec![
        ("quotes", big("\"", 100_000)),
        ("apostrophes", big("'", 100_000)),
        ("backticks", big("`", 100_000)),
        ("backtick pairs", big("`a", 100_000)),
        ("hyphens", big("-", 100_000)),
        ("dots", big(".", 100_000)),
        ("open parens", big("(", 100_000)),
        ("close parens", big(")", 100_000)),
        ("alternating quotes", big("\" ", 100_000)),
        ("alternating scripts", big("aя", 100_000)),
        (
            "zwj emoji",
            big("\u{1f468}\u{200d}\u{1f469}\u{200d}\u{1f467}", 100_000),
        ),
        ("combining marks", format!("a{}", big("\u{301}", 100_000))),
        ("prepend", big("\u{600}", 100_000)),
        ("one fragment", big("a:b/", 400_000)),
        ("one word", big("слово", 400_000)),
        ("many spaces", big(" ", 100_000)),
        ("many newlines", big("\n", 100_000)),
        ("blank lines", big("a\n\n", 100_000)),
        ("many paragraph separators", big("\u{2029}", 100_000)),
        ("operators", big("->", 100_000)),
        ("legal marks", big("(c)", 100_000)),
        ("digits", big("1234567890", 100_000)),
        ("comma list", big("a,", 100_000)),
        ("fences", big("```\na\n```\n", 100_000)),
        // A long run of transparent tokens before many dashes of one fragment each.
        (
            "zero-width prefix before dashes",
            big("\u{200b}", 100_000) + &big("-\u{200b}", 100_000),
        ),
        (
            "joiner prefix before dashes",
            big("\u{2060}\u{200c}", 100_000) + &big("-\u{200b}", 100_000),
        ),
        // Many straight quotes in the wrapper of one technical fragment.
        (
            "quotes around a path",
            big("\"", 100_000) + &big("a:b/", 100_000) + &big("\"", 100_000),
        ),
    ];
    for t in all_typographers() {
        for (name, input) in &inputs {
            let report = t.report(input, Output::Unicode);
            assert_eq!(
                report
                    .protected()
                    .windows(2)
                    .filter(
                        |w| w[0].range().end > w[1].range().start && w[0].piece() == w[1].piece()
                    )
                    .count(),
                0,
                "{name}"
            );
            let out = t.typeset(input, Output::Unicode);
            assert_eq!(report.result().as_ref(), out.as_ref(), "{name}");
        }
    }
}

#[test]
fn unclosed_delimiter_runs_of_growing_length_finish() {
    // Section 3.12: if the closing delimiter of a pair is not found from position p, it is not
    // found further on either; the search stops. Runs of every length from 1 to 440 are all
    // unclosed, and a quadratic search would not finish.
    let mut input = String::new();
    for n in 1..=440 {
        input.push_str(&"`".repeat(n));
        input.push(' ');
    }
    let t = Typographer::new(Language::EN_US, RuleSet::INTERFACE);
    let report = t.report(&input, Output::Unicode);
    assert!(
        report
            .diagnostics()
            .iter()
            .all(|d| d.kind() == &DiagnosticKind::UnclosedDelimiter)
    );
    assert!(!report.diagnostics().is_empty());
    assert!(
        report
            .protected()
            .iter()
            .all(|s| s.kind() != ProtectedKind::Delimited)
    );
    // The same with an own pair whose opening is never closed.
    let own = Typographer::builder(Language::EN_US, RuleSet::INTERFACE)
        .protect_between("{{", "}}")
        .build()
        .unwrap();
    let input = "{{ a ".repeat(50_000);
    let report = own.report(&input, Output::Unicode);
    assert!(
        report
            .protected()
            .iter()
            .all(|s| s.kind() != ProtectedKind::Delimited)
    );
    assert_eq!(report.result().as_ref(), input.as_str());
}

#[test]
fn deeply_nested_input_finishes() {
    // Quote nesting 10 000 deep and a long run of open brackets (section 3.12).
    let input = format!("{}x{}", "\"a ".repeat(10_000), "\" ".repeat(10_000));
    for t in all_typographers() {
        assert_report(&t, &input).unwrap();
    }
    let input = format!("{}x{}", "(".repeat(50_000), ")".repeat(50_000));
    for t in all_typographers() {
        assert_report(&t, &input).unwrap();
    }
}

#[test]
fn a_one_megabyte_line_without_spaces_is_one_fragment() {
    // Section 3.12: the classifier is linear, freezing is one pass.
    let t = Typographer::new(Language::RU_RU, RuleSet::INTERFACE);
    let input = big("a:b/c", 1 << 20);
    let spans = spans_of(&t, &input);
    assert_eq!(spans.len(), 1);
    assert_eq!(spans[0].2, 0..input.len());
    // And in pieces of an ASCII word.
    let input = big("abcdefgh", 1 << 20);
    assert!(spans_of(&t, &input).is_empty());
}

#[test]
fn many_short_fragments_finish() {
    let t = Typographer::new(Language::EN_US, RuleSet::PROSE);
    let unit = "a:b c:d -x y,z 1.5 ";
    let input = big(unit, 1 << 20);
    let spans = spans_of(&t, &input);
    // Four frozen fragments in each unit: two colons, a flag and a list; `1.5` is a number.
    assert_eq!(spans.len(), input.len() / unit.len() * 4);
}
