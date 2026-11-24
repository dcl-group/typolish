//! Unicode conformance of the lexer (design sections 5.1, 13.6): every line of the Unicode
//! `GraphemeBreakTest.txt` of the version of `unicode-segmentation` goes through both rule sets,
//! alone and inside sentence frames, and `report` shows that every edit starts and ends at a
//! cluster boundary of the input and that every cluster that no edit covers appears byte for byte
//! in the output.
//!
//! The file is kept verbatim with its Unicode license header in `tests/fixtures/unicode/` and is
//! excluded from the published package; the tests that read it are skipped when it is absent.
//!
//! The fixture is also compared with the segmenter itself, which pins the dependency to the
//! file; the spans of `protected()` are checked at cluster boundaries too, since a frozen
//! fragment is made of whole tokens and tokens of whole clusters.

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

use std::collections::BTreeSet;
use std::fs;
use std::path::PathBuf;

use probe::check_clusters as check;
use typolish::{Language, Output, RuleSet, Typographer};
use unicode_segmentation::UnicodeSegmentation;

fn fixture() -> Option<String> {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/unicode/GraphemeBreakTest.txt");
    fs::read_to_string(path).ok()
}

/// A line of the file: the string and the byte offsets of its cluster boundaries (including 0 and
/// the end).
struct Case {
    line: usize,
    text: String,
    boundaries: BTreeSet<usize>,
}

fn parse(source: &str) -> Vec<Case> {
    let mut cases = Vec::new();
    for (n, line) in source.lines().enumerate() {
        let data = line.split('#').next().unwrap_or("").trim();
        if data.is_empty() {
            continue;
        }
        let mut text = String::new();
        let mut boundaries = BTreeSet::new();
        for item in data.split_whitespace() {
            match item {
                "÷" => {
                    boundaries.insert(text.len());
                }
                "×" => {}
                hex => {
                    let code = u32::from_str_radix(hex, 16)
                        .unwrap_or_else(|_| panic!("line {}: {hex:?}", n + 1));
                    text.push(char::from_u32(code).unwrap_or_else(|| panic!("line {}", n + 1)));
                }
            }
        }
        cases.push(Case {
            line: n + 1,
            text,
            boundaries,
        });
    }
    cases
}

#[test]
fn the_fixture_is_for_the_unicode_version_of_the_dependency() {
    // The version of the fixture and of `unicode-segmentation` 1.13.3 (design section 13.6).
    assert_eq!(unicode_segmentation::UNICODE_VERSION, (17, 0, 0));
    if let Some(source) = fixture() {
        let first = source.lines().next().unwrap_or("");
        assert!(
            first.contains("GraphemeBreakTest-17.0.0"),
            "the fixture is not the file of Unicode 17.0.0: {first:?}"
        );
        assert!(
            source.contains("Unicode"),
            "the license header of the file must stay"
        );
        let lines = source
            .lines()
            .filter(|l| !l.starts_with('#') && !l.is_empty());
        assert!(lines.count() > 700, "the file is cut");
    } else {
        eprintln!("tests/fixtures/unicode/GraphemeBreakTest.txt is absent: skipped");
    }
}

#[test]
fn the_segmenter_agrees_with_the_fixture() {
    let Some(source) = fixture() else {
        eprintln!("GraphemeBreakTest.txt is absent: skipped");
        return;
    };
    let cases = parse(&source);
    assert!(cases.len() > 700);
    for case in cases {
        let mut got: BTreeSet<usize> = case.text.grapheme_indices(true).map(|(i, _)| i).collect();
        got.insert(case.text.len());
        assert_eq!(
            got,
            case.boundaries,
            "line {}: {}",
            case.line,
            case.text.escape_unicode()
        );
    }
}

const FRAMES: &[&str] = &[
    "X",
    "Запустите X сейчас.",
    "Значение: X, затем далее.",
    "Run X now.",
    "Value: X, then more.",
    "a\"X\"b",
    "1 X 2",
    "X\n\nX",
];

fn typographers() -> Vec<(String, Typographer)> {
    let mut out = Vec::new();
    for language in [Language::RU_RU, Language::EN_US, Language::COMMON] {
        for set in [RuleSet::PROSE, RuleSet::INTERFACE] {
            out.push((
                format!("{language} {set}"),
                Typographer::new(language.clone(), set),
            ));
        }
    }
    out
}

#[test]
fn every_line_of_the_fixture_alone() {
    let Some(source) = fixture() else {
        eprintln!("GraphemeBreakTest.txt is absent: skipped");
        return;
    };
    let cases = parse(&source);
    assert!(cases.len() > 700);
    for (name, t) in typographers() {
        for case in &cases {
            check(&t, &format!("{name} line {}", case.line), &case.text);
        }
    }
}

#[test]
fn every_line_of_the_fixture_inside_sentence_frames() {
    let Some(source) = fixture() else {
        eprintln!("GraphemeBreakTest.txt is absent: skipped");
        return;
    };
    let cases = parse(&source);
    assert!(cases.len() > 700);
    let ru = Typographer::new(Language::RU_RU, RuleSet::PROSE);
    let en = Typographer::new(Language::EN_US, RuleSet::INTERFACE);
    for case in &cases {
        for frame in FRAMES {
            let input = frame.replace('X', &case.text);
            check(
                &ru,
                &format!("ru Prose line {} frame {frame:?}", case.line),
                &input,
            );
            check(
                &en,
                &format!("en Interface line {} frame {frame:?}", case.line),
                &input,
            );
        }
    }
}

#[test]
fn every_line_of_the_fixture_as_a_part_of_ascii_text() {
    // The ASCII fast path of design section 3.5 hands the ASCII bytes on both sides of a non-ASCII
    // character to the segmenter: a mark after `"` or a digit, a `Prepend` before a letter.
    let Some(source) = fixture() else {
        eprintln!("GraphemeBreakTest.txt is absent: skipped");
        return;
    };
    let cases = parse(&source);
    assert!(cases.len() > 700);
    let t = Typographer::new(Language::EN_US, RuleSet::PROSE);
    for case in &cases {
        for (before, after) in [
            ("\"", "\""),
            ("1", "2"),
            ("a", "b"),
            (" - ", " - "),
            ("...", "..."),
            ("(c)", "5"),
        ] {
            let input = format!("{before}{}{after}", case.text);
            check(
                &t,
                &format!("line {} between {before:?} and {after:?}", case.line),
                &input,
            );
        }
    }
}

// The tables of design section 13.6 for clusters led by a non-letter: they come out unchanged and
// are neither numbers, signs nor quotes. A frozen `Symbol` fragment is `Unclassified` (check 24 of
// design section 3.6); a number would not be listed by `protected()`.

const CLUSTERS_LED_BY_A_NON_LETTER: &[&str] = &[
    "1\u{fe0f}\u{20e3}",
    "#\u{fe0f}\u{20e3}",
    "\"\u{301}",
    "\"\u{200c}",
    "1\u{200d}2",
    "Шаги 1\u{fe0f}\u{20e3}-3\u{fe0f}\u{20e3}",
    "a\"\u{301}b",
    "\u{600}123",
    "\u{6dd} ",
    "a\u{600}b",
    "x\u{600}",
    "\"\u{301}host:8080",
];

#[test]
fn clusters_led_by_a_non_letter_come_out_unchanged() {
    for (name, t) in typographers() {
        for input in CLUSTERS_LED_BY_A_NON_LETTER {
            check(&t, &name, input);
            let out = t.typeset(input, Output::Unicode);
            assert_eq!(out, *input, "{name}: {}", input.escape_unicode());
            assert!(
                matches!(out, std::borrow::Cow::Borrowed(_)),
                "{name}: {}",
                input.escape_unicode()
            );
        }
    }
}

/// Sequences of characters assigned long before Unicode 17.0.0 whose cluster boundaries differ
/// between versions of `unicode-segmentation` (design sections 5.6 and 15.1): the clusters with
/// 1.13.3 (Unicode 17.0.0), the version of the manifest, and as measured with 1.12.0 (Unicode
/// 16.0.0) and 1.10.1 (Unicode 15.0.0). Clusters are separated by `|`.
const CHANGED_BOUNDARIES: &[(&str, &str, &str, &str)] = &[
    // Rule GB9c (Unicode 15.1): an Indic conjunct is one cluster.
    ("\u{915}\u{94d}\u{937}", "क्ष", "क्ष", "क्|ष"),
    (
        "\u{915}\u{94d}\u{200d}\u{937}",
        "क्\u{200d}ष",
        "क्\u{200d}ष",
        "क्\u{200d}|ष",
    ),
    ("\u{995}\u{9cd}\u{9b7}", "ক্ষ", "ক্ষ", "ক্|ষ"),
    ("\u{d15}\u{d4d}\u{d37}", "ക്ഷ", "ക്ഷ", "ക്|ഷ"),
    // GB9c extended to Myanmar and Khmer (Unicode 17.0.0).
    ("\u{1000}\u{1039}\u{1000}", "က္က", "က္|က", "က္|က"),
    ("\u{1780}\u{17d2}\u{1780}", "ក្ក", "ក្|ក", "ក្|ក"),
    // `Extended_Pictographic` no longer holds for U+2605 BLACK STAR and many other symbols of
    // Unicode 1.1, so rule GB11 no longer joins them after an emoji and a ZWJ.
    (
        "\u{1f600}\u{200d}\u{2605}",
        "😀\u{200d}|★",
        "😀\u{200d}★",
        "😀\u{200d}★",
    ),
    (
        "\u{1f600}\u{200d}\u{2764}",
        "😀\u{200d}❤",
        "😀\u{200d}❤",
        "😀\u{200d}❤",
    ),
    // U+11A3A ZANABAZAR SQUARE CLUSTER-INITIAL LETTER RA (Unicode 10.0) is no longer `Prepend`, so
    // rule GB9b no longer joins it with the character after it, a quote or a space included.
    ("\u{11a3a}\"", "𑨺|\"", "𑨺\"", "𑨺\""),
    ("\u{11a3a} ", "𑨺| ", "𑨺 ", "𑨺 "),
    ("\u{11a3a}a", "𑨺|a", "𑨺a", "𑨺a"),
    // Unchanged: U+0600 ARABIC NUMBER SIGN stays `Prepend`.
    ("\u{600}\"", "\u{600}\"", "\u{600}\"", "\u{600}\""),
];

#[test]
fn boundaries_that_changed_between_versions_of_the_dependency() {
    // Design section 5.6: a newer `unicode-segmentation` changes cluster boundaries of characters
    // assigned long before, not only of new ones. The table pins the boundaries of the manifest's
    // version; the other two columns record the measured clusters of older versions.
    assert_eq!(unicode_segmentation::UNICODE_VERSION, (17, 0, 0));
    for &(text, now, v1_12, v1_10) in CHANGED_BOUNDARIES {
        let got: Vec<&str> = text.graphemes(true).collect();
        assert_eq!(got.join("|"), now, "{}", text.escape_unicode());
        for old in [v1_12, v1_10] {
            assert_eq!(old.replace('|', ""), text, "{}", text.escape_unicode());
        }
    }
}

#[test]
fn output_that_changed_between_versions_of_the_dependency() {
    // Design sections 5.6 and 15.1: with `unicode-segmentation` 1.12.0 U+11A3A joined the quote,
    // dash or full stop after it into one cluster, which the rules leave alone; with 1.13.3 the
    // quote, the dash and the ellipsis after it are typeset. The third column is the output
    // measured with 1.12.0 (the same with 1.10.1).
    let ru = Typographer::new(Language::RU_RU, RuleSet::PROSE);
    let en = Typographer::new(Language::EN_US, RuleSet::PROSE);
    let rows: &[(&Typographer, &str, &str, &str)] = &[
        (&ru, "\"\u{11a3a}\"", "«\u{11a3a}»", "\"\u{11a3a}\""),
        (
            &ru,
            "\u{11a3a} - да",
            "\u{11a3a}\u{a0}— да",
            "\u{11a3a} - да",
        ),
        (
            &ru,
            "\"да\" \u{11a3a}\"",
            "\"да\" \u{11a3a}\"",
            "«да» \u{11a3a}\"",
        ),
        (&en, "\u{11a3a}...", "\u{11a3a}…", "\u{11a3a}..."),
        (&en, "\u{11a3a}'s", "\u{11a3a}’s", "\u{11a3a}'s"),
        (
            &en,
            "x (\u{11a3a}) - y",
            "x (\u{11a3a}) - y",
            "x (\u{11a3a})—y",
        ),
        // GB9c and the emoji change move boundaries inside a run of letters or symbols only, so
        // the output stays the same.
        (&ru, "\"क्ष\" - да", "«क्ष»\u{a0}— да", "«क्ष»\u{a0}— да"),
        (&en, "\"😀\u{200d}★\"", "“😀\u{200d}★”", "“😀\u{200d}★”"),
    ];
    for &(t, input, now, v1_12) in rows {
        assert_eq!(
            t.typeset(input, Output::Unicode),
            now,
            "{}",
            input.escape_unicode()
        );
        assert_eq!(
            now != v1_12,
            input.contains('\u{11a3a}'),
            "{}",
            input.escape_unicode()
        );
    }
}
