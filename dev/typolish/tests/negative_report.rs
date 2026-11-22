//! The mandatory negative cases of design section 10.5 and the stress strings of section 10.7
//! through `report` (design section 13.3): each case gets the expected fragment kind and
//! freezing, listed by `protected()`; its tokens stay byte for byte; and no edit other than glue
//! falls inside a frozen span. `tests/negative.rs` holds the same rows at the level of `typeset`.
//!
//! The rows run alone and in the two sentence frames of each language, in the full sets (stage 3):
//! in Interface no edit occurs, except the glue that row 24's `if a<b and c>d` gets in English
//! (`if` and `and` are listed short words, `if⍽a<b and⍽c>d`, section 10.5); row 25 (short Latin
//! words in Russian text, short Cyrillic words in English text) gets none.
//!
//! The reference tables with the 405 corpus cases are generated outside this change and join
//! `tests/negative.rs` with their stage (design sections 13.3, 13.4).

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

use common::{interface, prose, show};
use probe::{SETS, span_texts, spans_of, tech};
use typolish::{DiagnosticKind, Language, Output, ProtectedKind, RuleSet, TechKind, Typographer};

const FLAG: ProtectedKind = tech(TechKind::Flag);
const PATH: ProtectedKind = tech(TechKind::Path);
const COLON: ProtectedKind = tech(TechKind::Colon);
const OPERATOR: ProtectedKind = tech(TechKind::Operator);
const IDENT: ProtectedKind = tech(TechKind::Identifier);
const ANGLE: ProtectedKind = tech(TechKind::Angle);
const UUID: ProtectedKind = tech(TechKind::Uuid);
const HEX: ProtectedKind = tech(TechKind::Hex);
const ISODATE: ProtectedKind = tech(TechKind::IsoDate);
const VERSION: ProtectedKind = tech(TechKind::Version);
const DOTTED: ProtectedKind = tech(TechKind::Dotted);
const LIST: ProtectedKind = tech(TechKind::List);
const HYPHEN: ProtectedKind = tech(TechKind::CompoundHyphens);
const UNCL: ProtectedKind = tech(TechKind::Unclassified);
const URL: ProtectedKind = tech(TechKind::Url);
const EMAIL: ProtectedKind = tech(TechKind::Email);

type Spans = &'static [(ProtectedKind, &'static str)];

/// A case: row of section 10.5, its text and the spans it is listed with.
struct Case {
    row: u8,
    text: &'static str,
    spans: Spans,
}

const fn case(row: u8, text: &'static str, spans: Spans) -> Case {
    Case { row, text, spans }
}

const CASES: &[Case] = &[
    case(1, "--force", &[(FLAG, "--force")]),
    case(1, "--long-opt=value", &[(FLAG, "--long-opt=value")]),
    case(2, "ls -la", &[(FLAG, "-la")]),
    case(3, "--", &[]),
    case(3, "git checkout -- file", &[]),
    case(4, "a>>b", &[(OPERATOR, "a>>b")]),
    case(5, "=>", &[]),
    case(6, "host:8080", &[(COLON, "host:8080")]),
    case(7, "::1", &[(COLON, "::1")]),
    case(7, "fe80::1%eth0", &[(COLON, "fe80::1%eth0")]),
    case(7, "[::1]:8080", &[(COLON, "[::1]:8080")]),
    case(8, "10.0.0.0/8", &[(PATH, "10.0.0.0/8")]),
    case(9, "a:b", &[(COLON, "a:b")]),
    case(10, "8000-8100", &[]),
    case(11, "2026-11-21", &[(ISODATE, "2026-11-21")]),
    case(
        11,
        "2026-11-21T10:20:30Z",
        &[(COLON, "2026-11-21T10:20:30Z")],
    ),
    case(12, "0.2.27", &[(VERSION, "0.2.27")]),
    case(13, "v1.2.3-rc.1", &[(VERSION, "v1.2.3-rc.1")]),
    case(
        13,
        "1.2.3-beta.2+build.5",
        &[(OPERATOR, "1.2.3-beta.2+build.5")],
    ),
    case(14, "0x1F", &[(HEX, "0x1F")]),
    case(14, "deadbeef0123", &[(HEX, "deadbeef0123")]),
    case(
        14,
        "d41d8cd98f00b204e9800998ecf8427e",
        &[(HEX, "d41d8cd98f00b204e9800998ecf8427e")],
    ),
    case(14, "deadbeef", &[]),
    case(
        15,
        "550e8400-e29b-41d4-a716-446655440000",
        &[(UUID, "550e8400-e29b-41d4-a716-446655440000")],
    ),
    case(
        16,
        "SHA256:47DEQpj8HBSa+/TImW+5JCeuQeRkm5NMpJWZG3hSuFU",
        &[(PATH, "SHA256:47DEQpj8HBSa+/TImW+5JCeuQeRkm5NMpJWZG3hSuFU")],
    ),
    case(
        16,
        "43:51:43:a1:b5:fc:8b:b7",
        &[(COLON, "43:51:43:a1:b5:fc:8b:b7")],
    ),
    case(17, "SGVsbG8gd29ybGQ=", &[(UNCL, "SGVsbG8gd29ybGQ=")]),
    case(17, "a+b/c==", &[(PATH, "a+b/c==")]),
    case(18, "/etc/hosts", &[(PATH, "/etc/hosts")]),
    case(18, "C:\\Users\\x", &[(PATH, "C:\\Users\\x")]),
    case(18, "~/.config/app", &[(PATH, "~/.config/app")]),
    // The run of dots at the start is a wrapper; the core is `/x`.
    case(18, "../x", &[(PATH, "/x")]),
    case(
        19,
        "https://example.com/a?b=c",
        &[(URL, "https://example.com/a?b=c")],
    ),
    case(19, "www.example.com", &[(URL, "www.example.com")]),
    case(20, "user@example.com", &[(EMAIL, "user@example.com")]),
    case(
        20,
        "git@example.com:org/repo.git",
        &[(EMAIL, "git@example.com:org/repo.git")],
    ),
    case(
        21,
        "db.example.internal",
        &[(DOTTED, "db.example.internal")],
    ),
    case(21, "db-01.example.com", &[(DOTTED, "db-01.example.com")]),
    case(21, "localhost", &[]),
    case(22, "a,b,c", &[(LIST, "a,b,c")]),
    case(22, "x,y;z", &[(LIST, "x,y;z")]),
    case(22, "1,2,3", &[(LIST, "1,2,3")]),
    case(22, "red,green,blue", &[(LIST, "red,green,blue")]),
    case(24, "<word>", &[(ANGLE, "<word>")]),
    case(24, "vec<int>", &[(OPERATOR, "vec<int>")]),
    case(24, "a <b> c", &[(ANGLE, "<b>")]),
    case(
        24,
        "if a<b and c>d",
        &[(OPERATOR, "a<b"), (OPERATOR, "c>d")],
    ),
    case(26, "tls-server", &[(HYPHEN, "-")]),
    case(26, "agent-ca", &[(HYPHEN, "-")]),
    case(27, "kv_lang", &[(IDENT, "kv_lang")]),
];

/// Row 23: Latin letters among Cyrillic ones, with the `MixedScript` warning; the word is a
/// word, never frozen.
const LOOKALIKES: &[&str] = &[
    "\u{41c}o\u{441}\u{43a}\u{432}\u{430}",
    "C\u{435}\u{440}\u{432}\u{435}\u{440}",
    "x\u{43b}\u{435}\u{431}",
];

/// Row 25: short Latin words in Russian text, short Cyrillic words in English text.
const SHORT_LATIN: &[&str] = &["ls", "a", "go", "db", "to"];
const SHORT_CYRILLIC: &[&str] = &["\u{438}", "\u{432}", "\u{43a}"];

const FRAMES_RU: [&str; 2] = ["Запустите X сейчас.", "Значение: X, затем далее."];
const FRAMES_EN: [&str; 2] = ["Run X now.", "Value: X, then more."];

fn languages() -> [(Language, [&'static str; 2]); 2] {
    [(Language::RU_RU, FRAMES_RU), (Language::EN_US, FRAMES_EN)]
}

/// Every protected span `(start, end)` of an input, technical and others.
fn ranges(t: &Typographer, text: &str) -> Vec<std::ops::Range<usize>> {
    spans_of(t, text).into_iter().map(|s| s.2).collect()
}

/// No edit falls inside a frozen span: an edit may touch its ends (glue next to it) but neither
/// replace nor insert inside it.
#[track_caller]
fn assert_spans_intact(t: &Typographer, text: &str, what: &str) {
    let report = t.report(text, Output::Unicode);
    let spans = ranges(t, text);
    for change in report.changes() {
        let c = change.range();
        for s in &spans {
            let inside = if c.is_empty() {
                c.start > s.start && c.start < s.end
            } else {
                c.start < s.end && c.end > s.start
            };
            assert!(
                !inside,
                "{what}: {} edit {c:?} ({}) falls inside the span {s:?}",
                show(text),
                change.rule()
            );
        }
    }
}

#[track_caller]
fn run_case(language: &Language, frames: &[&str; 2], c: &Case) {
    let inputs: Vec<String> = std::iter::once(c.text.to_owned())
        .chain(frames.iter().map(|f| f.replace('X', c.text)))
        .collect();
    for input in &inputs {
        let what = format!("{language} row {} {:?}", c.row, c.text);
        // The kind and the freezing do not depend on the set.
        for set in SETS {
            let t = Typographer::new(language.clone(), set);
            assert_eq!(
                span_texts(&t, input),
                c.spans,
                "{what}: {set}: {}",
                show(input)
            );
        }
        for base in [RuleSet::INTERFACE, RuleSet::PROSE] {
            let t = Typographer::new(language.clone(), base);
            assert_spans_intact(&t, input, &what);
        }
        // Interface: unchanged and borrowed, through `typeset` and through `report`, except for
        // the glue of row 24 in English.
        let t = interface(language.clone());
        let report = t.report(input, Output::Unicode);
        let glued = *language == Language::EN_US && c.text.contains("if a<b and c>d");
        if glued {
            assert!(!report.changes().is_empty(), "{what}: {}", show(input));
            for change in report.changes() {
                assert_eq!(change.rule(), typolish::Rule::NbspShortWords, "{what}");
                assert_eq!(change.replacement(), "\u{a0}", "{what}");
            }
            assert_eq!(
                report.result().replace('\u{a0}', " "),
                *input,
                "{what}: only glue differs"
            );
            assert!(matches!(report.result(), Cow::Owned(_)), "{what}");
        } else {
            assert!(
                matches!(report.result(), Cow::Borrowed(s) if *s == input.as_str()),
                "{what}: {} came back as {}",
                show(input),
                show(report.result())
            );
            assert!(report.changes().is_empty(), "{what}: {}", show(input));
            assert!(
                matches!(t.typeset(input, Output::Unicode), Cow::Borrowed(_)),
                "{what}"
            );
        }
    }
}

#[test]
fn every_row_of_10_5_has_its_fragment_kind_and_stays_unchanged_in_interface() {
    for (language, frames) in languages() {
        for c in CASES {
            run_case(&language, &frames, c);
        }
    }
}

#[test]
fn every_row_is_present() {
    let mut rows: Vec<u8> = CASES.iter().map(|c| c.row).chain([23, 25]).collect();
    rows.sort_unstable();
    rows.dedup();
    assert_eq!(rows, (1..=27).collect::<Vec<u8>>());
}

#[test]
fn row_23_lookalikes_are_words_with_a_warning() {
    for (language, frames) in languages() {
        for word in LOOKALIKES {
            let case = Case {
                row: 23,
                text: word,
                spans: &[],
            };
            run_case(&language, &frames, &case);
            for set in SETS {
                let t = Typographer::new(language.clone(), set);
                let report = t.report(word, Output::Unicode);
                let mixed: Vec<_> = report
                    .diagnostics()
                    .iter()
                    .filter(|d| d.kind() == &DiagnosticKind::MixedScript)
                    .collect();
                assert_eq!(
                    mixed.len(),
                    1,
                    "{language} {set}: {}",
                    word.escape_unicode()
                );
                assert_eq!(mixed[0].range(), 0..word.len());
                assert_eq!(mixed[0].severity(), typolish::Severity::Warning);
            }
        }
    }
}

#[test]
fn row_25_short_words_are_words_of_their_script() {
    let (ru, ru_frames) = (Language::RU_RU, FRAMES_RU);
    for word in SHORT_LATIN {
        let case = Case {
            row: 25,
            text: word,
            spans: &[],
        };
        run_case(&ru, &ru_frames, &case);
    }
    let (en, en_frames) = (Language::EN_US, FRAMES_EN);
    for word in SHORT_CYRILLIC {
        let case = Case {
            row: 25,
            text: word,
            spans: &[],
        };
        run_case(&en, &en_frames, &case);
    }
}

/// Stress strings of section 10.7 with the spans they are listed with; `None` where the design
/// does not pin the kind down.
const STRESS: &[(&str, Option<Spans>)] = &[
    (
        "rm -rf --no-preserve-root /",
        Some(&[(FLAG, "-rf"), (FLAG, "--no-preserve-root"), (PATH, "/")]),
    ),
    (
        "git commit -m msg --amend",
        Some(&[(FLAG, "-m"), (FLAG, "--amend")]),
    ),
    ("cargo build --release", Some(&[(FLAG, "--release")])),
    ("a=>b", Some(&[(OPERATOR, "a=>b")])),
    ("x -> y; z => w", Some(&[])),
    ("192.168.1.1:443", Some(&[(COLON, "192.168.1.1:443")])),
    ("12:30:45", Some(&[(COLON, "12:30:45")])),
    ("21.11.2026", Some(&[(VERSION, "21.11.2026")])),
    ("nats-server-2.10", Some(&[(DOTTED, "nats-server-2.10")])),
    ("my_var-name", Some(&[(IDENT, "my_var-name")])),
    ("CamelCase-Mixed_id", Some(&[(IDENT, "CamelCase-Mixed_id")])),
    ("pre-commit", Some(&[(HYPHEN, "-")])),
    ("map<k,v>", Some(&[(OPERATOR, "map<k,v>")])),
    ("C#", Some(&[(UNCL, "C#")])),
    ("AT&T", Some(&[(OPERATOR, "AT&T")])),
    ("50%-ный", Some(&[(OPERATOR, "50%-ный")])),
    ("key=\"value\"", Some(&[(OPERATOR, "key=\"value")])),
    ("f(c)", Some(&[])),
    ("f(r)", Some(&[])),
    ("S(r) = πr²", None),
    ("x86", Some(&[(IDENT, "x86")])),
    ("2x", Some(&[(IDENT, "2x")])),
    ("0x10", Some(&[(HEX, "0x10")])),
    ("1/2/2026", Some(&[(PATH, "1/2/2026")])),
    ("a <-> b", Some(&[])),
    ("-->", Some(&[])),
    ("<-->", Some(&[])),
    (">>=", Some(&[])),
    ("x == y", Some(&[])),
    (
        "diff -u - file.txt",
        Some(&[(FLAG, "-u"), (DOTTED, "file.txt")]),
    ),
    ("tar -cf - -C dir", Some(&[(FLAG, "-cf"), (FLAG, "-C")])),
    ("npm test -- --watch", Some(&[(FLAG, "--watch")])),
    ("Use - --force", Some(&[(FLAG, "--force")])),
    ("@user", Some(&[(EMAIL, "@user")])),
    ("@here", Some(&[(EMAIL, "@here")])),
    ("1'000'000", Some(&[(UNCL, "1'000'000")])),
    ("+7 495 123-4567", Some(&[])),
    ("123-4567", Some(&[])),
    ("+1 300 200 100 50", Some(&[])),
    ("2026-11", Some(&[])),
    ("5 - x", Some(&[])),
    ("x - 5", Some(&[])),
];

#[test]
fn stress_strings_have_their_kinds_and_stay_unchanged_in_interface() {
    for (language, _) in languages() {
        let interface = interface(language.clone());
        for (text, spans) in STRESS {
            if let Some(spans) = spans {
                for set in SETS {
                    let t = Typographer::new(language.clone(), set);
                    assert_eq!(
                        span_texts(&t, text),
                        *spans,
                        "{language} {set}: {}",
                        show(text)
                    );
                }
            }
            let report = interface.report(text, Output::Unicode);
            assert!(
                matches!(report.result(), Cow::Borrowed(s) if s == text),
                "{language}: {} came back as {}",
                show(text),
                show(report.result())
            );
            assert!(report.changes().is_empty());
            assert_spans_intact(&prose(language.clone()), text, "Prose");
        }
    }
}

#[test]
fn mentions_are_listed_as_emails() {
    // Section 10.8: `@user` is frozen whole as an `Email`; `@user` and `@here` come out
    // unchanged in Interface.
    for (language, _) in languages() {
        let t = interface(language);
        for text in ["@user", "@here", "Ping @user now", "Ping @here, please"] {
            let report = t.report(text, Output::Unicode);
            let kinds: Vec<ProtectedKind> = report.protected().iter().map(|s| s.kind()).collect();
            assert_eq!(kinds, [EMAIL], "{}", show(text));
            assert!(matches!(report.result(), Cow::Borrowed(_)));
        }
    }
}

#[test]
fn the_row_texts_are_found_in_every_position_of_a_sentence() {
    // The row text at the start, in the middle and at the end of a sentence, and after a
    // quote, keeps its kind.
    let frames = [
        "X rest",
        "mid X rest",
        "mid X",
        "«X»",
        "(X)",
        "X.",
        "X, X",
        "X\nX",
    ];
    for (language, _) in languages() {
        for c in CASES.iter().filter(|c| !c.text.contains(' ')) {
            for frame in frames {
                let input = frame.replace('X', c.text);
                let n = frame.matches('X').count();
                let t = Typographer::new(language.clone(), RuleSet::INTERFACE);
                let got = span_texts(&t, &input);
                let expected: Vec<(ProtectedKind, &str)> =
                    std::iter::repeat_n(c.spans.iter().copied(), n)
                        .flatten()
                        .collect();
                assert_eq!(got, expected, "{language} row {}: {}", c.row, show(&input));
            }
        }
    }
}
