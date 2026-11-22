//! The mandatory negative cases of design section 10.5 and the stress strings of section 10.7 at
//! the level of `typeset`: in the full Interface every case comes back unchanged and borrowed,
//! alone, in the two sentence frames of each language and as a `Piece::Text` next to slots, with
//! the two exceptions that section 10.5 names (row 24's `if a<b and c>d` in English gets its glue,
//! `if⍽a<b and⍽c>d`; row 25 is checked in English with short Cyrillic words); in the full Prose
//! the rows that Prose leaves unchanged do the same, and rows 3 and 10 give the output that
//! section 10.5 names for Prose. The interaction strings of section 10.7, fenced blocks, Swiss
//! grouping and the 405 negative cases of the reference tables (`tests/reference/{ru,en_us}.rs`,
//! section 13.4) are checked here too, through `report`: the token of a case stays byte for byte
//! and the only other edits are the glue of `nbsp.*` rules outside it.

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
#[path = "reference/en_us.rs"]
mod en_us;
#[path = "common/probe.rs"]
mod probe;
#[path = "reference/ru.rs"]
mod ru;

use std::borrow::Cow;
use std::ops::Range;

use common::{RefRow, check_exact, check_in, check_unchanged, interface, prose, show, vis};
use probe::{diagnostics_of, spans_of};
use typolish::{
    DiagnosticKind, Language, Output, Piece, ProtectedKind, Rule, RuleSet, Slot, TechKind,
    Typographer,
};

/// A case of section 10.5: the row number, the texts for both languages and whether Prose leaves
/// it unchanged.
struct Row {
    no: u8,
    name: &'static str,
    texts: &'static [&'static str],
    prose_unchanged: bool,
}

const fn row(no: u8, name: &'static str, texts: &'static [&'static str]) -> Row {
    Row {
        no,
        name,
        texts,
        prose_unchanged: true,
    }
}

const ROWS: &[Row] = &[
    row(1, "flag_force", &["--force", "--long-opt=value"]),
    row(2, "ls_la", &["ls -la"]),
    Row {
        no: 3,
        name: "lone_double_hyphen",
        texts: &["--", "git checkout -- file"],
        prose_unchanged: false,
    },
    row(4, "shift_operator", &["a>>b"]),
    row(5, "fat_arrow", &["=>"]),
    row(6, "host_port", &["host:8080"]),
    row(7, "ipv6", &["::1", "fe80::1%eth0", "[::1]:8080"]),
    row(8, "cidr", &["10.0.0.0/8"]),
    row(9, "colon_word", &["a:b"]),
    Row {
        no: 10,
        name: "port_range",
        texts: &["8000-8100"],
        prose_unchanged: false,
    },
    row(11, "iso_date", &["2026-11-21", "2026-11-21T10:20:30Z"]),
    row(12, "version", &["0.2.27"]),
    row(13, "version_pre", &["v1.2.3-rc.1", "1.2.3-beta.2+build.5"]),
    row(
        14,
        "hex",
        &[
            "0x1F",
            "deadbeef0123",
            "d41d8cd98f00b204e9800998ecf8427e",
            "deadbeef",
        ],
    ),
    row(15, "uuid", &["550e8400-e29b-41d4-a716-446655440000"]),
    row(
        16,
        "fingerprint",
        &[
            "SHA256:47DEQpj8HBSa+/TImW+5JCeuQeRkm5NMpJWZG3hSuFU",
            "43:51:43:a1:b5:fc:8b:b7",
        ],
    ),
    row(17, "base64", &["SGVsbG8gd29ybGQ=", "a+b/c=="]),
    row(
        18,
        "path",
        &["/etc/hosts", "C:\\Users\\x", "~/.config/app", "../x"],
    ),
    row(19, "url", &["https://example.com/a?b=c", "www.example.com"]),
    row(
        20,
        "email",
        &["user@example.com", "git@example.com:org/repo.git"],
    ),
    row(
        21,
        "host_name",
        &["db.example.internal", "db-01.example.com", "localhost"],
    ),
    row(22, "list", &["a,b,c", "x,y;z", "1,2,3", "red,green,blue"]),
    // Latin letters next to Cyrillic: Latin `o` in `Москва`, Latin `C` in `Сервер`, Latin `x` in `хлеб`.
    row(
        23,
        "lookalike",
        &[
            "\u{41c}o\u{441}\u{43a}\u{432}\u{430}",
            "C\u{435}\u{440}\u{432}\u{435}\u{440}",
            "x\u{43b}\u{435}\u{431}",
        ],
    ),
    row(
        24,
        "angle",
        &["<word>", "vec<int>", "a <b> c", "if a<b and c>d"],
    ),
    row(26, "hyphen_identifier", &["tls-server", "agent-ca"]),
    row(27, "underscore_identifier", &["kv_lang"]),
];

/// Row 25: short Latin words in Russian text; Cyrillic short words in English text.
const SHORT_LATIN_RU: &[&str] = &["ls", "a", "go", "db", "to"];
const SHORT_CYRILLIC_EN: &[&str] = &["\u{438}", "\u{432}", "\u{43a}"];

const FRAMES_RU: [&str; 2] = ["Запустите X сейчас.", "Значение: X, затем далее."];
const FRAMES_EN: [&str; 2] = ["Run X now.", "Value: X, then more."];

/// Stress strings of section 10.7 that stay unchanged in Interface.
const STRESS: &[&str] = &[
    "rm -rf --no-preserve-root /",
    "git commit -m msg --amend",
    "cargo build --release",
    "a=>b",
    "x -> y; z => w",
    "192.168.1.1:443",
    "12:30:45",
    "21.11.2026",
    "nats-server-2.10",
    "my_var-name",
    "CamelCase-Mixed_id",
    "pre-commit",
    "map<k,v>",
    "C#",
    "AT&T",
    "50%-ный",
    "key=\"value\"",
    "f(c)",
    "f(r)",
    "S(r) = πr²",
    "x86",
    "2x",
    "0x10",
    "1/2/2026",
    "a <-> b",
    "-->",
    "<-->",
    ">>=",
    "x == y",
    "diff -u - file.txt",
    "tar -cf - -C dir",
    "npm test -- --watch",
    "Use - --force",
    "@user",
    "@here",
    "1'000'000",
    "+7 495 123-4567",
    "123-4567",
    "+1 300 200 100 50",
    "2026-11",
    "5 - x",
    "x - 5",
];

fn languages() -> [(Language, [&'static str; 2]); 2] {
    [(Language::RU_RU, FRAMES_RU), (Language::EN_US, FRAMES_EN)]
}

fn unchanged(t: &typolish::Typographer, text: &str, what: &str) {
    let out = t.typeset(text, Output::Unicode);
    assert!(
        matches!(out, Cow::Borrowed(s) if s == text),
        "{what}: {} came back as {}",
        show(text),
        show(&out)
    );
}

/// What the full sets give for a case (section 10.5): the case itself, except `if a<b and c>d` in
/// English, where `if` and `and` are listed short words and the glue goes outside the frozen
/// fragments.
fn expected_of(english: bool, text: &str) -> String {
    if english {
        text.replace("if a<b and c>d", "if\u{a0}a<b and\u{a0}c>d")
    } else {
        text.to_owned()
    }
}

/// Checks that `t` gives the expected output for `text` and that the result is borrowed exactly
/// when it is the text.
#[track_caller]
fn expect(t: &typolish::Typographer, text: &str, english: bool, what: &str) {
    let expected = expected_of(english, text);
    if expected == text {
        unchanged(t, text, what);
    } else {
        let out = t.typeset(text, Output::Unicode);
        assert!(
            matches!(out, Cow::Owned(ref s) if *s == expected),
            "{what}: {} came back as {}, expected {}",
            show(text),
            show(&out),
            show(&expected)
        );
    }
}

fn run_row(t: &typolish::Typographer, frames: &[&str; 2], text: &str, what: &str) {
    let english = frames[0].starts_with("Run");
    expect(t, text, english, what);
    for frame in frames {
        expect(
            t,
            &frame.replace('X', text),
            english,
            &format!("{what} in a frame"),
        );
    }
    let spaced = format!(" {text} ");
    let pieces = [
        Piece::Slot("a", Slot::WORD),
        Piece::Text(&spaced),
        Piece::Slot("b", Slot::WORD),
    ];
    let out = t.typeset_template(pieces, Output::Unicode);
    assert_eq!(out.texts().len(), 3, "{what}");
    let expected = expected_of(english, &spaced);
    if expected == spaced {
        assert!(
            matches!(&out.texts()[1], Cow::Borrowed(s) if *s == spaced),
            "{what}: next to slots {} came back as {}",
            show(&spaced),
            show(&out.texts()[1])
        );
        assert!(!out.changed(), "{what}");
    } else {
        assert_eq!(out.texts()[1], expected, "{what}: next to slots");
        assert!(out.changed(), "{what}");
    }
    let glued = [
        Piece::Text(frames[0].split('X').next().unwrap()),
        Piece::Text(text),
        Piece::Slot("n", Slot::NUMBER),
    ];
    let out = t.typeset_template(glued, Output::Unicode);
    assert_eq!(out.texts().len(), 2, "{what}");
}

#[test]
fn interface_leaves_every_row_unchanged_except_the_two_that_section_10_5_names() {
    for (lang, frames) in languages() {
        let t = interface(lang.clone());
        for r in ROWS {
            for text in r.texts {
                run_row(
                    &t,
                    &frames,
                    text,
                    &format!("{lang} row {} {} {text:?}", r.no, r.name),
                );
            }
        }
        let short: &[&str] = if lang == Language::RU_RU {
            SHORT_LATIN_RU
        } else {
            SHORT_CYRILLIC_EN
        };
        for text in short {
            run_row(&t, &frames, text, &format!("{lang} row 25 {text:?}"));
        }
    }
}

#[test]
fn row_24_gets_its_glue_in_english_and_only_there() {
    // `if` and `and` are listed English short words (section 10.5): the glue goes outside `a<b`
    // and `c>d`, which stay byte for byte. In Russian text `if` and `and` are Latin words.
    let want = "if\u{a0}a<b and\u{a0}c>d";
    for t in [interface(Language::EN_US), prose(Language::EN_US)] {
        check_exact("en", &t, ("if a<b and c>d", want));
        check_exact(
            "en",
            &t,
            ("Run if a<b and c>d now.", &format!("Run {want} now.")),
        );
        check_exact(
            "en",
            &t,
            (
                "Value: if a<b and c>d, then more.",
                &format!("Value: {want}, then more."),
            ),
        );
    }
    for t in [interface(Language::RU_RU), prose(Language::RU_RU)] {
        check_unchanged("ru", &t, "if a<b and c>d");
        check_unchanged("ru", &t, "Запустите if a<b and c>d сейчас.");
    }
    // The other forms of row 24 stay in English: `a` is followed by a fragment that starts with `<`.
    let t = interface(Language::EN_US);
    for text in ["<word>", "vec<int>", "a <b> c"] {
        check_unchanged("en", &t, text);
    }
    // `report` lists the glue as `nbsp.short-words` and nothing else.
    let r = t.report("if a<b and c>d", Output::Unicode);
    let rules: Vec<_> = r.changes().iter().map(|c| c.rule()).collect();
    assert_eq!(rules, [Rule::NbspShortWords, Rule::NbspShortWords]);
}

#[test]
fn row_25_short_words_of_the_other_script_get_no_glue() {
    // ru text: Latin `ls`, `a`, `go`, `db`, `to`; en text: Cyrillic `и`, `в`, `к`, even before
    // a word (section 10.5, row 25; `short_words_any_script(false)`, M5).
    for t in [interface(Language::RU_RU), prose(Language::RU_RU)] {
        for text in SHORT_LATIN_RU {
            check_unchanged("ru", &t, &format!("Запустите {text} сервер сейчас."));
            check_unchanged("ru", &t, &format!("{text} server"));
        }
    }
    for t in [interface(Language::EN_US), prose(Language::EN_US)] {
        for text in SHORT_CYRILLIC_EN {
            check_unchanged("en", &t, &format!("Run {text} server now."));
            check_unchanged("en", &t, &format!("{text} server"));
        }
    }
}

#[test]
fn prose_leaves_the_unchanged_rows_unchanged() {
    for (lang, frames) in languages() {
        let t = prose(lang.clone());
        for r in ROWS.iter().filter(|r| r.prose_unchanged) {
            for text in r.texts {
                run_row(
                    &t,
                    &frames,
                    text,
                    &format!("{lang} prose row {} {} {text:?}", r.no, r.name),
                );
            }
        }
        // Row 25 in Prose: unchanged (M5), the length rule counts only words of the script.
        let short: &[&str] = if lang == Language::RU_RU {
            SHORT_LATIN_RU
        } else {
            SHORT_CYRILLIC_EN
        };
        for text in short {
            run_row(&t, &frames, text, &format!("{lang} prose row 25 {text:?}"));
        }
        // Row 3 in Prose: the lone `--` alone is unchanged.
        unchanged(&t, "--", &format!("{lang} prose row 3"));
    }
}

#[test]
fn interface_leaves_the_stress_strings_unchanged() {
    for (lang, _) in languages() {
        let t = interface(lang.clone());
        for text in STRESS {
            unchanged(&t, text, &format!("{lang} {text:?}"));
        }
    }
}

#[test]
fn interface_leaves_the_whitespace_cases_unchanged() {
    // Interaction strings of section 10.7 that no rule of Interface touches.
    let en = interface(Language::EN_US);
    for text in ["word  -  word", "word\t-\tword"] {
        unchanged(&en, text, text);
    }
    let ru = interface(Language::RU_RU);
    for text in ["т.  е.", "№\t5"] {
        unchanged(&ru, text, text);
    }
}

#[test]
fn the_rows_are_complete() {
    let mut numbers: Vec<u8> = ROWS.iter().map(|r| r.no).chain([25]).collect();
    numbers.sort_unstable();
    assert_eq!(numbers, (1..=27).collect::<Vec<u8>>());
}

// Prose, rows 3 and 10 (section 10.5, last column).

#[test]
fn row_3_in_prose() {
    // `--` alone stays; `Запустите -- сейчас.` becomes a dash; `git checkout -- file` stays in
    // Russian (foreign words) and becomes a dash in English (section 10.8).
    let ru = prose(Language::RU_RU);
    let en = prose(Language::EN_US);
    check_unchanged("ru", &ru, "--");
    check_unchanged("en", &en, "--");
    check_exact("ru", &ru, ("Запустите -- сейчас.", "Запустите⍽— сейчас."));
    check_unchanged("ru", &ru, "Значение: --, затем далее.");
    check_in("en", &en, ("Run -- now.", "Run—now."));
    check_unchanged("en", &en, "Value: --, then more.");
    check_unchanged("ru", &ru, "git checkout -- file");
    check_unchanged("ru", &ru, "Запустите git checkout -- file сейчас.");
    check_in("en", &en, ("git checkout -- file", "git checkout—file"));
    check_in(
        "en",
        &en,
        (
            "Run git checkout -- file now.",
            "Run git checkout—file now.",
        ),
    );
    // `report` names the rule; in Russian `nbsp.dash` also glues the space before the dash.
    let r = ru.report("Запустите -- сейчас.", Output::Unicode);
    let rules: Vec<Rule> = r.changes().iter().map(|c| c.rule()).collect();
    assert!(
        rules.contains(&Rule::DashDoubleHyphen)
            && rules
                .iter()
                .all(|c| matches!(c, Rule::DashDoubleHyphen | Rule::NbspDash)),
        "{rules:?}"
    );
    let r = en.report("git checkout -- file", Output::Unicode);
    assert!(
        !r.changes().is_empty()
            && r.changes()
                .iter()
                .all(|c| c.rule() == Rule::DashDoubleHyphen)
    );
    // Interface leaves all of them.
    for t in [interface(Language::RU_RU), interface(Language::EN_US)] {
        for text in [
            "--",
            "Запустите -- сейчас.",
            "git checkout -- file",
            "Run -- now.",
        ] {
            check_unchanged("Interface", &t, text);
        }
    }
}

#[test]
fn row_10_in_prose() {
    // A port range is an en dash in Prose, deliberately (section 10.6) and never in Interface.
    for (language, frames) in languages() {
        let t = prose(language.clone());
        check_in("Prose", &t, ("8000-8100", "8000–8100"));
        check_in("Prose", &t, ("ports 8000-8100", "ports 8000–8100"));
        for frame in frames {
            let input = frame.replace('X', "8000-8100");
            let output = frame.replace('X', "8000–8100");
            check_in("Prose frame", &t, (&input, &output));
        }
        let r = t.report("8000-8100", Output::Unicode);
        assert!(
            !r.changes().is_empty() && r.changes().iter().all(|c| c.rule() == Rule::RangeDigits),
            "{language}"
        );
        // Next to slots.
        let out = t.typeset_template(
            [
                Piece::Slot("a", Slot::WORD),
                Piece::Text(" 8000-8100 "),
                Piece::Slot("b", Slot::WORD),
            ],
            Output::Unicode,
        );
        assert_eq!(out.texts(), ["", " 8000–8100 ", ""], "{language}");
        assert!(out.changed());
        let ui = interface(language);
        check_unchanged("Interface", &ui, "8000-8100");
        check_unchanged("Interface", &ui, "ports 8000-8100");
    }
}

// The stress strings of section 10.7 in Prose, and the interaction strings of the same section.

#[test]
fn stress_strings_in_prose() {
    // Prose glues and changes the telephone fragments (section 10.8) and sets the arrows (symbols
    // are a rule of the same stage: both outputs are accepted until they exist); every other
    // string stays, borrowed.
    let changed: &[(&str, &[&str])] = &[
        ("+7 495 123-4567", &["+7⍽495⍽123–4567"]),
        ("+1 300 200 100 50", &["+1⍽300⍽200⍽100⍽50"]),
        ("123-4567", &["123–4567"]),
        // `symbols.arrows` is stage 3: unchanged until then.
        ("x -> y; z => w", &["x -> y; z => w", "x → y; z => w"]),
        ("a <-> b", &["a <-> b", "a ↔ b"]),
    ];
    for (language, _) in languages() {
        let t = prose(language.clone());
        for text in STRESS {
            match changed.iter().find(|c| c.0 == *text) {
                None => check_unchanged(&format!("{language} Prose"), &t, text),
                Some((_, allowed)) => {
                    let got = t.typeset(text, Output::Unicode);
                    assert!(
                        allowed.iter().any(|a| vis(a) == *got),
                        "{language} Prose: {} came back as {}",
                        show(text),
                        show(&got)
                    );
                }
            }
        }
    }
}

#[test]
fn interaction_strings_of_section_10_7() {
    // Exact output in both sets, ru and en unless a language is given, glue of the other rules
    // included (the table of section 10.7). Rows: input, Interface, Prose.
    type Table = &'static [(&'static str, &'static str, &'static str)];
    let both: Table = &[
        ("+7 495 123-4567", "+7 495 123-4567", "+7⍽495⍽123–4567"),
        ("123-4567", "123-4567", "123–4567"),
        (
            "+1 300 200 100 50",
            "+1 300 200 100 50",
            "+1⍽300⍽200⍽100⍽50",
        ),
        ("2026-11", "2026-11", "2026-11"),
        ("5 - x", "5 - x", "5 - x"),
        ("x - 5", "x - 5", "x - 5"),
    ];
    let ru_only: Table = &[
        ("2010-11 гг.", "2010-11⍽гг.", "2010–11⍽гг."),
        ("2026-11 гг.", "2026-11⍽гг.", "2026-11⍽гг."),
        (
            "21.11.2026 г. Затем",
            "21.11.2026⍽г. Затем",
            "21.11.2026⍽г. Затем",
        ),
        (
            "21.11.26 г. Затем",
            "21.11.26⍽г. Затем",
            "21.11.26⍽г. Затем",
        ),
        ("-- file", "-- file", "—⍽file"),
        (
            "Подождите...\nДальше",
            "Подождите…\nДальше",
            "Подождите…\nДальше",
        ),
        (
            "Подождите...\tДальше",
            "Подождите…\tДальше",
            "Подождите…\tДальше",
        ),
        ("... слово", "… слово", "… слово"),
        ("т.  е.", "т.  е.", "т.  е."),
        ("№\t5", "№\t5", "№\t5"),
    ];
    let en_only: Table = &[
        ("-- file", "-- file", "-- file"),
        ("word  -  word", "word  -  word", "word  -  word"),
        ("word\t-\tword", "word\t-\tword", "word\t-\tword"),
        ("word⍽- word", "word⍽— word", "word⍽— word"),
    ];
    for (language, table) in [
        (Language::RU_RU, [both, ru_only]),
        (Language::EN_US, [both, en_only]),
    ] {
        let ui = interface(language.clone());
        let pr = prose(language.clone());
        for (input, interface_out, prose_out) in table.iter().flat_map(|t| t.iter()) {
            check_exact(
                &format!("{language} Interface"),
                &ui,
                (input, interface_out),
            );
            check_exact(&format!("{language} Prose"), &pr, (input, prose_out));
        }
    }
    // `5 - x` and `x - 5` report `AmbiguousHyphen` in both sets.
    for (language, _) in languages() {
        for t in [interface(language.clone()), prose(language.clone())] {
            for text in ["5 - x", "x - 5"] {
                let kinds: Vec<_> = diagnostics_of(&t, text).into_iter().map(|d| d.0).collect();
                assert_eq!(
                    kinds,
                    [DiagnosticKind::AmbiguousHyphen],
                    "{language} {text}"
                );
            }
        }
    }
}

#[test]
fn mentions_come_out_unchanged_in_interface_and_are_emails() {
    for (language, _) in languages() {
        let t = interface(language);
        for text in ["@user", "@here"] {
            check_unchanged("Interface", &t, text);
            let spans = spans_of(&t, text);
            assert_eq!(
                spans.iter().map(|s| s.0).collect::<Vec<_>>(),
                [ProtectedKind::Technical(TechKind::Email)],
                "{text}"
            );
        }
    }
}

#[test]
fn fenced_blocks_are_protected_whole() {
    // A fenced block indented by three spaces with an info string, with a quote and dots that no
    // rule may touch; the text around it is typeset (section 10.1).
    let input = "   ```rust\nlet s = \"a...\";\nx - y -- z\n   ```\n\nОн сказал \"да\" и ждал...";
    let expected = "   ```rust\nlet s = \"a...\";\nx - y -- z\n   ```\n\nОн сказал «да» и ждал…";
    for t in [prose(Language::RU_RU), interface(Language::RU_RU)] {
        check_in("fenced", &t, (input, expected));
        let r = t.report(input, Output::Unicode);
        let block = r
            .protected()
            .iter()
            .find(|s| s.kind() == ProtectedKind::Delimited)
            .unwrap();
        assert_eq!(block.piece(), 0);
        // The span runs from the opening run to the closing run; the indentation before the
        // opening run is not part of it (section 10.1).
        assert_eq!(
            &input[block.range()],
            &input[input.find("```").unwrap()..input.rfind("```").unwrap() + 3]
        );
    }
}

#[test]
fn a_longer_closing_run_does_not_close() {
    // The closing run is exactly as long as the opening one (section 10.1): the four-backtick line
    // is content, the later three-backtick line closes.
    let input = "```\nx \"a\" - y...\n````\nz -- w\n```\nОн сказал \"да\".";
    let expected = "```\nx \"a\" - y...\n````\nz -- w\n```\nОн сказал «да».";
    for t in [prose(Language::RU_RU), interface(Language::RU_RU)] {
        check_in("fenced", &t, (input, expected));
        let r = t.report(input, Output::Unicode);
        let spans: Vec<_> = r
            .protected()
            .iter()
            .filter(|s| s.kind() == ProtectedKind::Delimited)
            .map(|s| &input[s.range()])
            .collect();
        assert_eq!(spans, ["```\nx \"a\" - y...\n````\nz -- w\n```"]);
    }
}

#[test]
fn swiss_grouping_is_frozen_in_both_sets() {
    for (language, frames) in languages() {
        for t in [prose(language.clone()), interface(language.clone())] {
            check_unchanged("Swiss", &t, "1'000'000");
            for frame in frames {
                check_unchanged("Swiss in a frame", &t, &frame.replace('X', "1'000'000"));
            }
            let spans = spans_of(&t, "1'000'000");
            assert_eq!(spans.len(), 1, "{language}");
            assert!(matches!(spans[0].0, ProtectedKind::Technical(_)));
        }
    }
}

// The 405 negative cases of the reference tables (section 13.3).

const GROUPS: [&str; 10] = [
    "neg.flags",
    "neg.net",
    "neg.ver",
    "neg.ids",
    "neg.ident",
    "neg.list",
    "neg.html",
    "neg.mixed",
    "neg.shortlat",
    "neg.lookalike",
];

fn is_negative(id: &str) -> bool {
    GROUPS
        .iter()
        .any(|g| id.strip_prefix(g).is_some_and(|rest| rest.starts_with('.')))
}

/// The negative rows of a table.
fn negative_rows(table: &'static [RefRow]) -> Vec<&'static RefRow> {
    table.iter().filter(|r| is_negative(r.id)).collect()
}

/// The token of a case: the part between the words of its frame for a framed case, the whole
/// input for a case without a space, none for a sentence or a command (design section 10.5).
fn token_range(r: &RefRow, frames: &[&str; 2]) -> Option<Range<usize>> {
    if r.id.contains(".ctx.") {
        for frame in frames {
            let (pre, post) = frame.split_once('X').unwrap();
            if r.input.len() >= pre.len() + post.len()
                && r.input.starts_with(pre)
                && r.input.ends_with(post)
            {
                return Some(pre.len()..r.input.len() - post.len());
            }
        }
        panic!("{}: no frame of the language fits {:?}", r.id, r.input);
    }
    (!r.input.contains(' ')).then_some(0..r.input.len())
}

/// The text with glued spaces written as ordinary spaces.
fn unglued(text: &str) -> String {
    text.replace(['\u{a0}', '\u{202f}'], " ")
}

/// The output that Prose gives for a case: unchanged, except where section 10.5 and section 10.7
/// allow the token to change (a port range, the lone `--` between words, an arrow). Several
/// entries mean that the output depends on whether the rule of a later stage exists yet.
fn prose_options(r: &RefRow, ru: bool, token: &Option<Range<usize>>) -> Vec<String> {
    let input = r.input;
    let token_text = token.as_ref().map(|t| &input[t.clone()]);
    let mut out = input.to_owned();
    match token_text.unwrap_or(input) {
        text if text == "8000-8100" || text == "ports 8000-8100" => {
            out = input.replace("8000-8100", "8000–8100");
        }
        "--" if r.id.contains(".ctx.") => {
            // The first frame has the dash between two words; the second has a comma after it.
            let first = if ru {
                "Запустите -- сейчас."
            } else {
                "Run -- now."
            };
            if input == first {
                out = if ru {
                    "Запустите — сейчас.".to_owned()
                } else {
                    "Run—now.".to_owned()
                };
            }
        }
        "x -> y; z => w" => {
            return vec![input.to_owned(), input.replace("x -> y", "x → y")];
        }
        _ => {}
    }
    vec![out]
}

/// Checks a case through `report` with `t`: the result against the allowed ones, and every edit
/// either a glue mark of an `nbsp.*` rule outside the token or an edit that the case names.
#[track_caller]
fn check_case(
    what: &str,
    t: &Typographer,
    r: &RefRow,
    frames: &[&str; 2],
    allowed: &[String],
    edits: &[Rule],
) {
    let report = t.report(r.input, Output::Unicode);
    let token = token_range(r, frames);
    let got = report.result().to_string();
    assert!(
        allowed.iter().any(|a| *a == unglued(&got)),
        "{what}: {}: {}\nexpected one of {:?}\ngot      {}",
        r.id,
        show(r.input),
        allowed,
        show(&got)
    );
    assert_eq!(
        matches!(report.result(), Cow::Borrowed(_)),
        got == r.input,
        "{what}: {}: Cow::Borrowed exactly when unchanged",
        r.id
    );
    for change in report.changes() {
        let range = change.range();
        let glue = matches!(change.replacement(), "\u{a0}" | "\u{202f}")
            && matches!(report.original(change), "" | " ");
        if change.rule().id().starts_with("nbsp.") {
            assert!(
                glue,
                "{what}: {}: an nbsp edit that is not glue: {change:?}",
                r.id
            );
            // A case of several fragments with a listed short word between them gets its glue
            // outside the frozen fragments, as row 24 of section 10.5 (`if⍽a<b and⍽c>d`).
            if let Some(token) = &token {
                let between =
                    change.rule() == Rule::NbspShortWords && report.original(change) == " ";
                assert!(
                    range.end <= token.start || range.start >= token.end || between,
                    "{what}: {}: glue inside the token {token:?}: {change:?}",
                    r.id
                );
            }
            // Never before a fragment that starts with `-`, except `nbsp.dash` before the dash it
            // glues to (sections 8.2 and 8.7).
            assert!(
                change.rule() == Rule::NbspDash || !r.input[range.end..].starts_with('-'),
                "{what}: {}: glue before a hyphen: {change:?}",
                r.id
            );
        } else {
            assert!(
                edits.contains(&change.rule()),
                "{what}: {}: unexpected edit of {}: {change:?}",
                r.id,
                change.rule().id()
            );
        }
    }
    // The text that no edit covers stays: the frozen fragments appear in the output in order.
    let mut at = 0;
    for span in report.protected() {
        if let ProtectedKind::Technical(_) = span.kind() {
            let text = &r.input[span.range()];
            match got[at..].find(text) {
                Some(i) => at += i + text.len(),
                None => panic!(
                    "{what}: {}: the frozen {text:?} is not in {}",
                    r.id,
                    show(&got)
                ),
            }
        }
    }
}

#[test]
fn the_reference_tables_hold_405_negative_cases() {
    let ru = negative_rows(ru::ROWS);
    let en = negative_rows(en_us::ROWS);
    assert_eq!((ru.len(), en.len()), (207, 198));
    assert_eq!(ru::ROWS.len() + en_us::ROWS.len(), 820);
    for rows in [&ru, &en] {
        for group in GROUPS {
            let n = rows.iter().filter(|r| r.id.starts_with(group)).count();
            assert!(n > 0, "{group}");
        }
        let mut ids: Vec<_> = rows.iter().map(|r| r.id).collect();
        ids.sort_unstable();
        ids.dedup();
        assert_eq!(ids.len(), rows.len(), "ids are unique");
    }
    // Language of an id: `.ru.` and `.en-US.`.
    assert!(ru.iter().all(|r| r.id.contains(".ru.")));
    assert!(en.iter().all(|r| r.id.contains(".en-US.")));
    // Host names are neutral: a name under `.com` or `.internal` is a reserved example name
    // (`example.com`, `git.example.internal`) or a single host of `.internal` (`db.internal`).
    for r in ru.iter().chain(en.iter()) {
        for text in [r.input, r.recorded] {
            let names = text
                .split(|c: char| !(c.is_ascii_alphanumeric() || c == '.' || c == '-'))
                .map(|w| w.trim_matches('.'))
                .filter(|w| w.ends_with(".com") || w.ends_with(".internal"));
            for name in names {
                let labels: Vec<&str> = name.split('.').collect();
                let example = labels.len() >= 2 && labels[labels.len() - 2] == "example";
                let single = labels.len() == 2 && labels[1] == "internal";
                assert!(example || single, "{}: {name}", r.id);
            }
        }
    }
}

#[test]
fn corpus_cases_keep_their_token_in_interface() {
    // Section 13.3: in Interface every case keeps its token byte for byte; the only other edits
    // are glue marks of `nbsp.*` rules outside it (some cases sit in sentences with listed short
    // words, `Это a и b в тексте`), and the output, with its glue read as ordinary spaces, is the
    // input.
    for (language, frames, rows) in [
        (Language::RU_RU, FRAMES_RU, negative_rows(ru::ROWS)),
        (Language::EN_US, FRAMES_EN, negative_rows(en_us::ROWS)),
    ] {
        let t = interface(language.clone());
        for r in rows {
            let allowed = [r.input.to_owned()];
            check_case(
                &format!("{language} Interface"),
                &t,
                r,
                &frames,
                &allowed,
                &[],
            );
            let report = t.report(r.input, Output::Unicode);
            assert!(
                report
                    .changes()
                    .iter()
                    .all(|c| c.rule().id().starts_with("nbsp.")),
                "{}: {:?}",
                r.id,
                report.changes()
            );
        }
    }
}

#[test]
fn corpus_cases_keep_their_token_in_prose() {
    // In Prose the token may also change in the groups that section 10.5 allows (the lone `--`
    // between words, literal ranges, arrows), and only as `prose_options` records.
    for (language, frames, rows) in [
        (Language::RU_RU, FRAMES_RU, negative_rows(ru::ROWS)),
        (Language::EN_US, FRAMES_EN, negative_rows(en_us::ROWS)),
    ] {
        let ru = language == Language::RU_RU;
        let t = prose(language.clone());
        let edits = [
            Rule::RangeDigits,
            Rule::RangeContext,
            Rule::DashDoubleHyphen,
            Rule::DashSpacedHyphen,
            Rule::SymbolsArrows,
        ];
        let mut changed = 0;
        for r in rows {
            let token = token_range(r, &frames);
            let allowed = prose_options(r, ru, &token);
            check_case(
                &format!("{language} Prose"),
                &t,
                r,
                &frames,
                &allowed,
                &edits,
            );
            changed += usize::from(allowed.len() == 1 && allowed[0] != r.input);
        }
        // `8000-8100` and `ports 8000-8100` with their frames, and the lone `--` in its first
        // frame: 3 + 3 + 1 rows change (the arrow row is the one that waits for stage 3).
        assert_eq!(changed, 7, "{language}");
    }
}

#[test]
fn corpus_cases_are_unchanged_when_no_rule_runs() {
    // The tables are not trivially satisfied: with every rule off nothing changes either, and
    // the tokens of the cases are found in the inputs.
    let t = Typographer::new(Language::RU_RU, RuleSet::EMPTY);
    for r in negative_rows(ru::ROWS) {
        assert!(vis(r.input) == r.input, "{}", r.id);
        check_unchanged("no rules", &t, r.input);
        let token = token_range(r, &FRAMES_RU).unwrap_or(0..r.input.len());
        assert!(!r.input[token].is_empty(), "{}", r.id);
    }
}
