//! The lexer of design sections 3.5, 5.1 and 5.3 to 5.5 as a black box (the tables of section
//! 13.6): which characters separate fragments, which clusters are words, which are `Symbol`s
//! (and so frozen as `Unclassified`), and how format characters and marks behave. The lexer
//! produces tokens, not text, so each row is observed through the fragments they make:
//! `report().protected()`, and through the output, which no mark, space or cluster may change.
//!
//! The grapheme conformance of the lexer is in `tests/graphemes.rs`; the cluster boundaries of
//! the ASCII fast path are checked here against the segmenter on every short string over a small
//! alphabet of ASCII and of `Prepend`, `Extend`, `ZWJ`, `SpacingMark` and other characters.

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
use probe::{all_languages, check_clusters, span_texts, tech};
use typolish::{Language, Output, ProtectedKind, RuleSet, TechKind, Typographer};

const COLON: ProtectedKind = tech(TechKind::Colon);
const UNCL: ProtectedKind = tech(TechKind::Unclassified);

fn sweep() -> Vec<(String, Typographer)> {
    probe::sweep(&all_languages())
}

#[track_caller]
fn expect(rows: &[(&str, Vec<(ProtectedKind, &str)>)]) {
    for (name, t) in sweep() {
        for (input, spans) in rows {
            assert_eq!(
                span_texts(&t, input),
                *spans,
                "{name}: {}",
                input.escape_unicode()
            );
        }
    }
}

/// Every character that `char::is_whitespace` accepts and that is not a line break or U+2029.
const HORIZONTAL: &[char] = &[
    ' ', '\t', '\u{b}', '\u{c}', '\u{a0}', '\u{1680}', '\u{2000}', '\u{2001}', '\u{2002}',
    '\u{2003}', '\u{2004}', '\u{2005}', '\u{2006}', '\u{2007}', '\u{2008}', '\u{2009}', '\u{200a}',
    '\u{202f}', '\u{205f}', '\u{3000}',
];

#[test]
fn horizontal_whitespace_separates_fragments() {
    for (name, t) in sweep() {
        for sp in HORIZONTAL {
            let input = format!("a:b{sp}c:d");
            assert_eq!(
                span_texts(&t, &input),
                [(COLON, "a:b"), (COLON, "c:d")],
                "{name}: {}",
                input.escape_unicode()
            );
            // A run of them too, and mixed.
            let input = format!("a:b{sp}{sp} \t{sp}c:d");
            assert_eq!(span_texts(&t, &input).len(), 2, "{name}");
        }
    }
}

#[test]
fn line_breaks_separate_fragments() {
    for (name, t) in sweep() {
        for br in [
            "\n", "\r\n", "\r", "\u{85}", "\u{2028}", "\u{2029}", "\n\n", "\r\n\r\n",
        ] {
            let input = format!("a:b{br}c:d");
            assert_eq!(
                span_texts(&t, &input),
                [(COLON, "a:b"), (COLON, "c:d")],
                "{name}: {}",
                input.escape_unicode()
            );
        }
    }
}

#[test]
fn a_space_with_a_mark_is_a_frozen_space_that_still_separates() {
    // Design section 5.3: U+0020 + U+0301, U+0020 + U+0903 and U+0020 + U+200D are a frozen
    // `Space`: it separates fragments, is never glued or removed, and the mark stays.
    let rows: Vec<(&str, Vec<(ProtectedKind, &str)>)> = vec![
        ("a:b \u{301}c:d", vec![(COLON, "a:b"), (COLON, "c:d")]),
        ("a:b \u{200d}c:d", vec![(COLON, "a:b"), (COLON, "c:d")]),
        ("a:b \u{903}c:d", vec![(COLON, "a:b"), (COLON, "c:d")]),
        ("a:b \u{200c}c:d", vec![(COLON, "a:b"), (COLON, "c:d")]),
        ("a:b\u{a0}\u{301}c:d", vec![(COLON, "a:b"), (COLON, "c:d")]),
        // A tab is a `Control` character, so the mark after it is a cluster of its own (UAX #29,
        // rules GB4 and GB5), a `Symbol` at the start of the next fragment (design sections 3.5
        // and 5.1): the tab still separates, and the mark belongs to the colon fragment's core.
        (
            "a:b\t\u{301}c:d",
            vec![(COLON, "a:b"), (COLON, "\u{301}c:d")],
        ),
    ];
    expect(&rows);
}

#[test]
fn spaces_with_marks_come_out_unchanged() {
    for (name, t) in sweep() {
        for (input, mark) in [
            ("в \u{301}доме", '\u{301}'),
            ("Он \u{200d}сказал", '\u{200d}'),
            ("5 \u{301}МБ", '\u{301}'),
            ("т. \u{301}е.", '\u{301}'),
            ("Слово - \u{301}слово", '\u{301}'),
            ("а \u{200d}б", '\u{200d}'),
            ("№ \u{301}5", '\u{301}'),
        ] {
            let got = t.typeset(input, Output::Unicode);
            // The mark and the space stay; a rule may glue other spaces, not this one.
            assert!(
                got.contains(&format!(" {mark}")),
                "{name}: {} came back as {}",
                show(input),
                show(&got)
            );
        }
    }
}

#[test]
fn format_characters_do_not_make_a_word_unclassified() {
    // Section 5.4: `Format` tokens inside a fragment, and ZWNJ and ZWJ inside a token, are
    // skipped by every check.
    let none: Vec<(ProtectedKind, &str)> = vec![];
    let rows: Vec<(&str, Vec<(ProtectedKind, &str)>)> = vec![
        ("Мо\u{ad}скве", none.clone()),
        ("ноя\u{ad}бря", none.clone()),
        ("21 ноя\u{ad}бря", none.clone()),
        ("сло\u{200d}во", none.clone()),
        ("ноя\u{200c}бря", none.clone()),
        ("21 ноя\u{200c}бря", none.clone()),
        ("می\u{200c}شود", none.clone()),
        ("\u{200c}a", none.clone()),
        ("\u{200c}\u{200d}a", none.clone()),
        ("\n\u{200c}\u{200d}a", none.clone()),
        ("\u{2068}Москве\u{2069}", none.clone()),
        ("\u{2068}\u{2069}", none.clone()),
        ("\u{200b}слово", none.clone()),
        ("слово\u{200e}", none.clone()),
        ("a\t\u{200d}b", none.clone()),
        ("a\u{85}\u{200d}b", none.clone()),
        ("a\u{ad}\u{200d}b", none.clone()),
    ];
    expect(&rows);
}

#[test]
fn format_characters_come_out_unchanged() {
    // Section 13.6: all of them come out unchanged.
    for (name, t) in sweep() {
        for input in [
            "\u{200c}a",
            "a\t\u{200d}b",
            "a\u{0}\u{200c}b",
            "a\u{7f}\u{200d}b",
            "a\u{85}\u{200d}b",
            "a\u{ad}\u{200d}b",
            "\n\u{200c}\u{200d}a",
            "\"\u{200c}a",
            "1\u{200d}2",
            " \u{200d}",
            "a \u{200d}b",
        ] {
            let got = t.typeset(input, Output::Unicode);
            assert!(
                matches!(&got, Cow::Borrowed(s) if *s == input),
                "{name}: {} came back as {}",
                input.escape_unicode(),
                got.escape_unicode()
            );
        }
    }
}

#[test]
fn a_cluster_led_by_a_non_letter_is_a_symbol_and_is_frozen() {
    // Section 3.5: a cluster of several characters whose first character is not a letter is a
    // `Symbol`, so the fragment is `Unclassified`; it is no number, no sign and no quote (a
    // number or a quote wrapper would not be listed).
    let rows: Vec<(&str, Vec<(ProtectedKind, &str)>)> = vec![
        ("1\u{fe0f}\u{20e3}", vec![(UNCL, "1\u{fe0f}\u{20e3}")]),
        ("#\u{fe0f}\u{20e3}", vec![(UNCL, "#\u{fe0f}\u{20e3}")]),
        ("\"\u{301}", vec![(UNCL, "\"\u{301}")]),
        ("\"\u{200c}a", vec![(UNCL, "\"\u{200c}a")]),
        ("1\u{200d}2", vec![(UNCL, "1\u{200d}2")]),
        (
            "Шаги 1\u{fe0f}\u{20e3}-3\u{fe0f}\u{20e3}",
            vec![(UNCL, "1\u{fe0f}\u{20e3}-3\u{fe0f}\u{20e3}")],
        ),
        ("a\"\u{301}b", vec![(UNCL, "a\"\u{301}b")]),
        ("\u{600}123", vec![(UNCL, "\u{600}123")]),
        ("a\u{600}b", vec![(UNCL, "a\u{600}b")]),
        // A `Prepend` character glues the next character to itself, so no edit and no
        // classification can fall between them.
        ("\u{600}1", vec![(UNCL, "\u{600}1")]),
        // Emoji sequences are single symbols; for quotes they behave as words.
        ("🙂", vec![(UNCL, "🙂")]),
        ("\"🙂\"", vec![(UNCL, "🙂")]),
        (
            "👨\u{200d}👩\u{200d}👧",
            vec![(UNCL, "👨\u{200d}👩\u{200d}👧")],
        ),
        ("🇷🇺", vec![(UNCL, "🇷🇺")]),
        ("👍\u{1f3fd}", vec![(UNCL, "👍\u{1f3fd}")]),
        // CJK punctuation is a symbol; the ideographs are letters.
        ("你好，世界", vec![(UNCL, "你好，世界")]),
        ("你好", vec![]),
    ];
    expect(&rows);
}

#[test]
fn digits_of_other_systems_are_word_characters() {
    // Section 5.2: a non-ASCII digit is a word character, so the fragment is a word that is not
    // frozen, and it adds no script of its own (no `MixedScript`).
    let rows: Vec<(&str, Vec<(ProtectedKind, &str)>)> = vec![
        ("abc\u{663}", vec![]),
        ("\u{663}\u{664}", vec![]),
        ("\u{639}\u{663}", vec![]),
        ("x\u{ff11}", vec![]),
        ("\u{43a}\u{43e}\u{434}\u{966}", vec![]),
    ];
    expect(&rows);
    for (_, t) in sweep() {
        for input in ["abc\u{663}", "\u{43a}\u{43e}\u{434}\u{966}"] {
            let report = t.report(input, typolish::Output::Unicode);
            assert!(
                report.diagnostics().is_empty(),
                "{}",
                input.escape_unicode()
            );
        }
    }
}

#[test]
fn clusters_led_by_a_non_letter_come_out_unchanged_next_to_ascii() {
    // The ASCII fast path hands the ASCII bytes on both sides of a non-ASCII character to the
    // segmenter (section 3.5).
    for (name, t) in sweep() {
        for input in [
            "Шаги 1\u{fe0f}\u{20e3}-3\u{fe0f}\u{20e3}",
            "a\"\u{301}b",
            "x 1\u{fe0f}\u{20e3} y",
            "\u{600}123",
            "a\u{600}b",
            "\u{6dd} ",
            "\"\u{301}host:8080",
            "Run \"\u{301} now.",
            "5\u{301}0",
            "- \u{301}-",
        ] {
            let got = t.typeset(input, Output::Unicode);
            assert!(
                matches!(&got, Cow::Borrowed(s) if *s == input),
                "{name}: {} came back as {}",
                input.escape_unicode(),
                got.escape_unicode()
            );
            check_clusters(&t, &name, input);
        }
    }
}

#[test]
fn a_combining_mark_belongs_to_the_character_before_it() {
    // An ASCII letter with a mark is a letter-led cluster: part of the word, and of the
    // colon fragment's core; the span ends after the mark.
    let rows: Vec<(&str, Vec<(ProtectedKind, &str)>)> = vec![
        ("a:b\u{301}", vec![(COLON, "a:b\u{301}")]),
        ("a:b\u{301} c", vec![(COLON, "a:b\u{301}")]),
        ("за\u{301}мок", vec![]),
        ("e\u{301}cole", vec![]),
        ("\u{438}\u{306}", vec![]),
        ("a\u{301}b\u{301}", vec![]),
    ];
    expect(&rows);
}

#[test]
fn a_line_break_pair_is_one_cluster() {
    // `\r\n` is one `Newline`: the fragments on both sides are separate, and nothing in
    // between is a token of its own that could be edited.
    for (name, t) in sweep() {
        let input = "a:b\r\nc:d\r\n\r\ne:f";
        assert_eq!(
            span_texts(&t, input),
            [(COLON, "a:b"), (COLON, "c:d"), (COLON, "e:f")],
            "{name}"
        );
        check_clusters(&t, &name, input);
        let input = "a:b\r\r\nc:d";
        assert_eq!(span_texts(&t, input).len(), 2, "{name}");
        check_clusters(&t, &name, input);
    }
}

#[test]
fn no_ascii_character_stands_inside_a_cluster_of_ascii_only_text() {
    // On pure ASCII the fast path cuts between every two bytes except inside `\r\n`.
    for (name, t) in sweep() {
        for input in [
            "Run \"x\" -- now... (c) 5-10 'a' 1/2\r\n\r\nNext.",
            "a-b-c d.e.f g,h;i j:k l@m",
            "x86 2FA 0x1F a>>b <T> {x} [y]",
        ] {
            check_clusters(&t, &name, input);
        }
    }
}

/// Characters of the alphabet of the exhaustive check: ASCII that rules read, and the classes of
/// UAX #29 that glue to a neighbor.
const ALPHABET: &[&str] = &[
    "a",
    "1",
    "\"",
    "-",
    ".",
    " ",
    "\r",
    "\n",
    ":",
    "я",
    "\u{600}",   // Prepend
    "\u{301}",   // Extend
    "\u{200d}",  // ZWJ
    "\u{200c}",  // ZWNJ
    "\u{903}",   // SpacingMark
    "\u{fe0f}",  // Extend (variation selector)
    "\u{20e3}",  // Extend (keycap)
    "\u{1f600}", // Extended_Pictographic
    "\u{1f1f7}", // Regional_Indicator
    "\u{1100}",  // Hangul L
    "\u{1161}",  // Hangul V
    "\u{ad}",    // Control (soft hyphen)
    "\u{85}",    // Control
];

#[test]
fn every_short_string_over_the_alphabet_has_edits_and_spans_at_cluster_boundaries() {
    let ts = [
        Typographer::new(Language::RU_RU, RuleSet::PROSE),
        Typographer::new(Language::EN_US, RuleSet::INTERFACE),
        Typographer::new(Language::EN_US, RuleSet::EMPTY),
    ];
    let mut count = 0;
    for a in ALPHABET {
        for b in ALPHABET {
            for c in ALPHABET {
                let input = format!("{a}{b}{c}");
                for (i, t) in ts.iter().enumerate() {
                    check_clusters(t, &format!("t{i}"), &input);
                    // A string that no rule touches stays borrowed; with the empty set this holds
                    // for every string.
                    if i == 2 {
                        let out = t.typeset(&input, Output::Unicode);
                        assert!(
                            matches!(out, Cow::Borrowed(_)),
                            "{}",
                            input.escape_unicode()
                        );
                    }
                }
                count += 1;
            }
        }
    }
    assert_eq!(count, ALPHABET.len().pow(3));
    // Four characters, sparser: a pair of ASCII around a pair of glue-prone characters.
    for a in ["a", "\"", "1", " "] {
        for b in ALPHABET {
            for c in ALPHABET {
                for d in ["b", "\"", "2", " "] {
                    let input = format!("{a}{b}{c}{d}");
                    for (i, t) in ts.iter().enumerate() {
                        check_clusters(t, &format!("t{i}"), &input);
                    }
                }
            }
        }
    }
}

#[test]
fn output_equals_input_for_the_empty_rule_set_on_arbitrary_clusters() {
    let t = Typographer::new(Language::RU_RU, RuleSet::EMPTY);
    for input in [
        "",
        "\u{301}",
        "\u{200d}",
        "\u{600}",
        "\u{600}\u{600}1",
        "🇷🇺🇷",
        "👨\u{200d}👩\u{200d}👧\u{200d}",
        "\u{1100}\u{1161}\u{11a8}",
        "\r\n\u{301}",
        "\"\u{301}\"\u{301}",
    ] {
        let report = t.report(input, Output::Unicode);
        assert_eq!(
            report.result().as_ref(),
            input,
            "{}",
            input.escape_unicode()
        );
        assert!(matches!(report.result(), Cow::Borrowed(_)));
        assert!(report.changes().is_empty());
    }
}

#[test]
fn sets_do_not_change_the_classification_of_a_string_with_clusters() {
    let input = "Run \u{600}1 \"\u{301} a:b\u{301} 🙂 x\u{200d}y 1\u{fe0f}\u{20e3} end.";
    let mut first: Option<Vec<(ProtectedKind, String)>> = None;
    for (name, t) in sweep() {
        let got: Vec<(ProtectedKind, String)> = span_texts(&t, input)
            .into_iter()
            .map(|(k, s)| (k, s.to_owned()))
            .collect();
        match &first {
            None => first = Some(got),
            Some(f) => assert_eq!(&got, f, "{name}"),
        }
    }
}
