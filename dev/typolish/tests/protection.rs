//! Ingest and explicit protection of a whole string (design sections 3.4, 10.1) and windows
//! (section 3.3), observed through `report`: the `Delimited` records of `protected()`, the
//! `UnclosedDelimiter` diagnostics, and the borrowed output.
//!
//! An unclosed delimiter is an ordinary character (section 10.1), so a stray backtick is
//! classified like any other character and may make its fragment a technical span. The tables
//! here therefore compare the `Delimited` records only; the technical spans have their tables in
//! `tests/classifier.rs`.
//!
//! The same behavior for pieces is in `tests/pieces_report.rs`, which needs `report_template`
//! (design section 15.4: stage 4); the rules that read these decisions are in
//! `tests/ingest_rules.rs`.

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
use probe::{SETS, diagnostics_of, nth, span_texts, tech};
use typolish::{
    DiagnosticKind, Language, Output, ProtectedKind, RuleSet, Severity, TechKind, Typographer,
};

const D: ProtectedKind = ProtectedKind::Delimited;
const COLON: ProtectedKind = tech(TechKind::Colon);

fn interface() -> Typographer {
    Typographer::new(Language::RU_RU, RuleSet::INTERFACE)
}

/// The texts of the `Delimited` records.
fn ds<'a>(t: &Typographer, text: &'a str) -> Vec<&'a str> {
    span_texts(t, text)
        .into_iter()
        .filter(|s| s.0 == D)
        .map(|s| s.1)
        .collect()
}

/// A range, spelled so that a one-element array of it is no `single_range_in_vec_init`.
fn at(start: usize, end: usize) -> std::ops::Range<usize> {
    start..end
}

fn unclosed(t: &Typographer, text: &str) -> Vec<std::ops::Range<usize>> {
    probe::diag_ranges(t, text, &DiagnosticKind::UnclosedDelimiter)
}

fn unclosed_texts<'a>(t: &Typographer, text: &'a str) -> Vec<&'a str> {
    probe::diag_texts(t, text, &DiagnosticKind::UnclosedDelimiter)
}

/// An input, the texts of its `Delimited` records and the texts of its `UnclosedDelimiter`
/// diagnostics.
type Row = (
    &'static str,
    &'static [&'static str],
    &'static [&'static str],
);

const INLINE: &[Row] = &[
    // Runs of N backticks, closed by the next run of exactly N (section 10.1).
    ("`a`", &["`a`"], &[]),
    ("x `a b` y `c` z", &["`a b`", "`c`"], &[]),
    ("` `", &["` `"], &[]),
    ("a ` b ` c", &["` b `"], &[]),
    ("``a`b``", &["``a`b``"], &[]),
    ("`a``b`", &["`a``b`"], &[]),
    ("``a` b``", &["``a` b``"], &[]),
    ("```code```", &["```code```"], &[]),
    ("до`и`после", &["`и`"], &[]),
    // A span may cross a single line break, never a blank line.
    ("`a\nb`", &["`a\nb`"], &[]),
    ("`a\r\nb`", &["`a\r\nb`"], &[]),
    ("`a\u{2028}b`", &["`a\u{2028}b`"], &[]),
    ("`a\n\nb`", &[], &["`", "`"]),
    // Pairs are formed left to right; a lone run stays text.
    ("`a`b`", &["`a`"], &["`"]),
    ("don`t", &[], &["`"]),
    ("``", &[], &["``"]),
    ("`", &[], &["`"]),
    ("``a`", &[], &["``", "`"]),
    // Inside a span the content is protected whole.
    ("`host:8080` host:8080", &["`host:8080`"], &[]),
    (
        "Run `git checkout -- file` now.",
        &["`git checkout -- file`"],
        &[],
    ),
    ("в `--config`", &["`--config`"], &[]),
    ("\"`--force`\"", &["`--force`"], &[]),
    // Three or more backticks at a line start with no closing run at a later line start are
    // inline runs.
    ("```\ncode", &[], &["```"]),
    ("```\nx```", &["```\nx```"], &[]),
    ("```a```\nb", &["```a```"], &[]),
    // The closing run of a fenced block must start a later line; here it does not, and the
    // blank line makes two windows.
    ("```\nx\n\ny```", &[], &["```", "```"]),
];

#[test]
fn inline_backticks() {
    for language in [Language::RU_RU, Language::EN_US, Language::COMMON] {
        for set in SETS {
            let t = Typographer::new(language.clone(), set);
            for (input, spans, unclosed) in INLINE {
                assert_eq!(ds(&t, input), *spans, "{language} {set}: {}", show(input));
                assert_eq!(
                    unclosed_texts(&t, input),
                    *unclosed,
                    "{language} {set}: {}",
                    show(input)
                );
            }
        }
    }
}

#[test]
fn unclosed_delimiters_are_warnings_at_the_run() {
    let t = interface();
    let input = "Он сказал: don`t.";
    let diagnostics = diagnostics_of(&t, input);
    assert_eq!(diagnostics.len(), 1);
    let (kind, severity, rule, piece, range) = &diagnostics[0];
    assert_eq!(kind, &DiagnosticKind::UnclosedDelimiter);
    assert_eq!(*severity, Severity::Warning);
    assert_eq!(*rule, None, "protection is no rule (section 12.4)");
    assert_eq!(*piece, 0);
    assert_eq!(range, &nth(input, "`", 0));
}

#[test]
fn an_unclosed_run_is_text_not_protection_to_the_end() {
    // A stray backtick switches nothing off: the rest of the text is classified.
    let t = interface();
    for input in [
        "a ` b host:8080 c:d",
        "don`t, host:8080 c:d",
        "`x host:8080 c:d",
    ] {
        let spans = span_texts(&t, input);
        assert!(spans.contains(&(COLON, "host:8080")), "{spans:?}");
        assert!(spans.contains(&(COLON, "c:d")), "{spans:?}");
        assert!(ds(&t, input).is_empty());
    }
}

#[test]
fn two_stray_backticks_in_different_paragraphs_do_not_pair() {
    // Section 10.1: `Don`t.\n\nОн сказал "да".\n\nIt`s ok.` gets two diagnostics.
    let t = interface();
    let input = "Don`t.\n\nОн сказал \"да\".\n\nIt`s ok.";
    assert!(ds(&t, input).is_empty());
    assert_eq!(
        unclosed(&t, input),
        [nth(input, "`", 0), nth(input, "`", 1)]
    );
}

const FENCED: &[(&str, &[&str])] = &[
    ("```\ncode\n```", &["```\ncode\n```"]),
    (
        "text\n\n```rust\nfn x() {}\n```\n\nmore",
        &["```rust\nfn x() {}\n```"],
    ),
    // A fenced block may contain blank lines.
    ("```\na\n\nb\n```", &["```\na\n\nb\n```"]),
    ("```\na\n\n\n\nb\n```", &["```\na\n\n\n\nb\n```"]),
    ("```\n\n```", &["```\n\n```"]),
    // Indentation of spaces and tabs before the opening and the closing run; the block runs
    // from the opening run to the closing run.
    ("   ```rust\nx\n   ```", &["```rust\nx\n   ```"]),
    ("\t```\nx\n\t```", &["```\nx\n\t```"]),
    ("    ```\nx\n    ```", &["```\nx\n    ```"]),
    // A longer run does not close; the closing run has exactly the length of the opening one.
    ("```\nx\n````\ny\n```", &["```\nx\n````\ny\n```"]),
    ("````\n```\n````", &["````\n```\n````"]),
    ("```\nx\n``\ny\n```", &["```\nx\n``\ny\n```"]),
    // Two blocks and text between them.
    (
        "```\na\n```\nmid\n```\nb\n```",
        &["```\na\n```", "```\nb\n```"],
    ),
    // A paragraph separator inside a block is part of the block.
    ("```\na\u{2029}b\n```", &["```\na\u{2029}b\n```"]),
    // Line breaks of every kind inside.
    ("```\r\na\r\n\r\nb\r\n```", &["```\r\na\r\n\r\nb\r\n```"]),
    // Backticks in the content do not matter.
    ("```\n`a` `\n```", &["```\n`a` `\n```"]),
];

#[test]
fn fenced_blocks_are_protected_whole() {
    for language in [Language::RU_RU, Language::EN_US, Language::COMMON] {
        for set in SETS {
            let t = Typographer::new(language.clone(), set);
            for (input, blocks) in FENCED {
                assert_eq!(ds(&t, input), *blocks, "{language} {set}: {}", show(input));
                // The content is never classified: nothing else is listed.
                assert_eq!(
                    span_texts(&t, input).len(),
                    blocks.len(),
                    "{language} {set}: {}",
                    show(input)
                );
                assert!(
                    unclosed(&t, input).is_empty(),
                    "{language} {set}: {}",
                    show(input)
                );
            }
        }
    }
}

#[test]
fn a_fence_without_a_closing_run_is_an_unclosed_inline_run() {
    let t = interface();
    // The 4-run does not close the 3-run, and itself has no closing run.
    let input = "```\nx\n````";
    assert!(ds(&t, input).is_empty());
    assert_eq!(
        unclosed(&t, input),
        [nth(input, "```", 0), nth(input, "````", 0)]
    );
    // A run of three that is not at a line start opens no block; it pairs as an inline run.
    let input = "a ```b``` c";
    assert_eq!(ds(&t, input), ["```b```"]);
}

#[test]
fn text_around_a_fenced_block_is_classified() {
    let t = interface();
    let input = "a:b\n\n```\nc:d\n```\n\ne:f";
    assert_eq!(
        span_texts(&t, input),
        [(COLON, "a:b"), (D, "```\nc:d\n```"), (COLON, "e:f")]
    );
}

#[test]
fn spans_keep_their_exact_ranges() {
    let t = interface();
    let input = "Run `ls -la` now";
    let report = t.report(input, Output::Unicode);
    assert_eq!(probe::records(report.protected()), [(0, 4..12, D)]);
    let input = "а `б` в";
    let report = t.report(input, Output::Unicode);
    // `а ` is 3 bytes, `` `б` `` is 4.
    assert_eq!(probe::records(report.protected()), [(0, 3..7, D)]);
}

// Own pairs (section 10.1).

fn with_pairs(pairs: &[(&str, &str)]) -> Typographer {
    let mut builder = Typographer::builder(Language::EN_US, RuleSet::INTERFACE);
    for (open, close) in pairs {
        builder = builder.protect_between(*open, *close);
    }
    builder.build().unwrap()
}

#[test]
fn own_pairs() {
    let braces = with_pairs(&[("{{", "}}")]);
    assert_eq!(ds(&braces, "a {{b c}} d"), ["{{b c}}"]);
    assert_eq!(ds(&braces, "{{a}} and {{b}}"), ["{{a}}", "{{b}}"]);
    assert_eq!(ds(&braces, "{{a:b}}"), ["{{a:b}}"]);
    assert_eq!(span_texts(&braces, "{{a:b}}").len(), 1);
    assert_eq!(ds(&braces, "{{a\nb}}"), ["{{a\nb}}"]);
    assert_eq!(unclosed(&braces, "{{a"), [at(0, 2)]);
    assert!(ds(&braces, "{{a").is_empty());
    // A stray closing delimiter is text, not an unclosed opening.
    assert!(ds(&braces, "a }} b").is_empty());
    assert!(unclosed(&braces, "a }} b").is_empty());

    let brackets = with_pairs(&[("[[", "]]")]);
    assert_eq!(ds(&brackets, "x [[y]] z"), ["[[y]]"]);

    let both = with_pairs(&[("{{", "}}"), ("[[", "]]")]);
    assert_eq!(ds(&both, "[[a]] {{b}}"), ["[[a]]", "{{b}}"]);

    // Opening and closing delimiters may be equal.
    let bars = with_pairs(&[("|", "|")]);
    assert_eq!(ds(&bars, "a |b| c"), ["|b|"]);
    assert_eq!(ds(&bars, "|a|b|c|"), ["|a|", "|c|"]);
    assert_eq!(ds(&bars, "|a|b|"), ["|a|"]);
    assert_eq!(unclosed(&bars, "|a|b|"), [at(4, 5)]);
}

#[test]
fn own_pairs_work_next_to_backticks() {
    let t = with_pairs(&[("{{", "}}")]);
    assert_eq!(ds(&t, "`a` {{b}} `c`"), ["`a`", "{{b}}", "`c`"]);
    // Spans do not nest: inside an open span only its closing delimiter is looked for.
    let input = "{{ a `b }} c`";
    assert_eq!(ds(&t, input), ["{{ a `b }}"]);
    assert_eq!(unclosed(&t, input), [nth(input, "`", 1)]);
    let input = "`a {{ b` c }}";
    assert_eq!(ds(&t, input), ["`a {{ b`"]);
    // The stray `}}` is text; it is not an unclosed opening.
    assert!(unclosed(&t, input).is_empty());
}

#[test]
fn backticks_can_be_turned_off() {
    let off = Typographer::builder(Language::EN_US, RuleSet::INTERFACE)
        .protect_backticks(false)
        .build()
        .unwrap();
    for input in ["`a`", "Run `host:8080`", "don`t `a`", "```\nx\n```"] {
        assert!(ds(&off, input).is_empty(), "{}", show(input));
        assert!(unclosed(&off, input).is_empty(), "{}", show(input));
    }
    // The content of an unprotected span is classified: `host:8080` between backticks is one
    // fragment with the backticks, a colon fragment.
    assert_eq!(
        span_texts(&off, "Run `host:8080`"),
        [(COLON, "`host:8080`")]
    );
    let mixed = Typographer::builder(Language::EN_US, RuleSet::INTERFACE)
        .protect_backticks(false)
        .protect_between("{{", "}}")
        .build()
        .unwrap();
    assert_eq!(ds(&mixed, "`a` {{b}}"), ["{{b}}"]);
}

#[test]
fn own_pairs_are_scoped_to_a_window() {
    let t = with_pairs(&[("{{", "}}")]);
    let input = "{{a\n\nb}}";
    assert!(ds(&t, input).is_empty());
    assert_eq!(unclosed(&t, input), [at(0, 2)]);
}

// Windows (section 3.3): observed through the scope of a delimiter pair.

/// Separators that end a window: two or more line breaks with only horizontal whitespace
/// between them, or U+2029.
const BOUNDARIES: &[&str] = &[
    "\n\n",
    "\n\n\n",
    "\n \n",
    "\n\t\n",
    "\n \t \n",
    "\n\u{a0}\n",
    "\n\u{3000}\n",
    "\n\u{2003}\n",
    "\r\n\r\n",
    "\r\n\r\n\r\n",
    "\r\n \r\n",
    "\r\r",
    "\n\r",
    "\u{85}\u{85}",
    "\u{2028}\u{2028}",
    "\n\u{2028}",
    "\u{2029}",
    " \u{2029} ",
    "\u{2029}\u{2029}",
    "\n\n\u{2029}",
    "\n\u{2029}\n",
];

/// Separators that do not: a single line break, alone or among horizontal whitespace, and
/// anything else between two line breaks.
const NOT_BOUNDARIES: &[&str] = &[
    "\n",
    "\r\n",
    "\r",
    "\u{85}",
    "\u{2028}",
    " \n ",
    "\n.\n",
    "\n\u{200b}\n",
    "\n\u{2068}\u{2069}\n",
    "\n\u{301}\n",
    " ",
    "\t",
];

#[test]
fn window_boundaries_end_the_scope_of_a_span() {
    for language in [Language::RU_RU, Language::EN_US] {
        for set in SETS {
            let t = Typographer::new(language.clone(), set);
            for sep in BOUNDARIES {
                let input = format!("`a{sep}b`");
                assert!(
                    ds(&t, &input).is_empty(),
                    "{language} {set}: two windows in {}",
                    show(&input)
                );
                assert_eq!(unclosed(&t, &input).len(), 2, "{}", show(&input));
            }
            for sep in NOT_BOUNDARIES {
                let input = format!("`a{sep}b`");
                assert_eq!(
                    ds(&t, &input),
                    [input.as_str()],
                    "{language} {set}: one window in {}",
                    show(&input)
                );
                assert!(unclosed(&t, &input).is_empty(), "{}", show(&input));
            }
        }
    }
}

#[test]
fn fragments_never_cross_a_separator() {
    let t = interface();
    for sep in BOUNDARIES.iter().chain(NOT_BOUNDARIES) {
        let input = format!("a:b{sep}c:d");
        // A combining mark after a line break is a cluster of its own (UAX #29, rule GB4), so it
        // is a `Symbol` fragment between the line breaks, frozen by check 24 (design sections
        // 3.5, 3.6 and 5.1); it is content of the separator, not part of a neighbor.
        let expected: &[(ProtectedKind, &str)] = if sep.contains('\u{301}') {
            &[
                (COLON, "a:b"),
                (tech(TechKind::Unclassified), "\u{301}"),
                (COLON, "c:d"),
            ]
        } else {
            &[(COLON, "a:b"), (COLON, "c:d")]
        };
        assert_eq!(span_texts(&t, &input), expected, "{}", show(&input));
    }
}

#[test]
fn boundary_characters_are_written_back_unchanged() {
    // The renderer writes the characters of a boundary between the windows (section 3.3).
    for language in [Language::RU_RU, Language::EN_US, Language::COMMON] {
        for set in SETS {
            let t = Typographer::new(language.clone(), set);
            for sep in BOUNDARIES.iter().chain(NOT_BOUNDARIES) {
                let input = format!("a:b{sep}c:d{sep}Ждём");
                let out = t.typeset(&input, Output::Unicode);
                assert!(
                    matches!(&out, Cow::Borrowed(s) if *s == input),
                    "{language} {set}: {} came back as {}",
                    show(&input),
                    show(&out)
                );
                let report = t.report(&input, Output::Unicode);
                assert_eq!(report.result().as_ref(), input.as_str());
                assert!(report.changes().is_empty());
            }
        }
    }
}

#[test]
fn leading_and_trailing_boundaries_are_harmless() {
    let t = interface();
    for input in [
        "\n\n",
        "\n\na:b",
        "a:b\n\n",
        "\n\n\n\n",
        "\u{2029}",
        "\u{2029}a:b\u{2029}",
        " \n\n ",
        "\r\n\r\n",
    ] {
        let report = t.report(input, Output::Unicode);
        assert_eq!(report.result().as_ref(), input, "{}", show(input));
        assert!(matches!(report.result(), Cow::Borrowed(_)));
    }
}

#[test]
fn a_fenced_block_lies_inside_one_window() {
    let t = interface();
    // Fenced blocks are cut before windows are found. The blank lines around the block end the
    // windows of the lone backticks; the blank line inside the block ends nothing.
    let input = "`x\n\n```\na\n\nb\n```\n\ny`";
    assert_eq!(ds(&t, input), ["```\na\n\nb\n```"]);
    assert_eq!(unclosed(&t, input).len(), 2);
}

#[test]
fn an_empty_string_and_odd_strings_are_fine() {
    for t in [
        interface(),
        Typographer::new(Language::EN_US, RuleSet::PROSE),
        Typographer::new(Language::COMMON, RuleSet::ALL),
    ] {
        for input in [
            "", " ", "\n", "`", "``", "```", "\u{301}", "\u{200d}", "\r", "\u{2029}",
        ] {
            let report = t.report(input, Output::Unicode);
            assert_eq!(report.result().as_ref(), input, "{}", show(input));
        }
    }
}
