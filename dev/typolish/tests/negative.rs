//! The mandatory negative cases of design section 10.5 and the stress strings of section 10.7 at
//! the level of `typeset`: in Interface without the `nbsp.*` rules every case comes back unchanged
//! and borrowed, alone, in the two sentence frames of each language and as a `Piece::Text` next to
//! slots; in Prose (again without `nbsp.*`) the rows that Prose leaves unchanged do the same.
//!
//! The checks through `report` (tokens byte for byte, expected `TechKind`), the `nbsp.*` glue and
//! the 405 negative cases of the reference tables join with the stages that deliver them (design
//! sections 13.3 and 15.4); rows 3 and 10 in Prose join with their rules.

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

use common::{show, without_nbsp};
use typolish::{Language, Output, Piece, RuleSet, Slot};

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

fn run_row(t: &typolish::Typographer, frames: &[&str; 2], text: &str, what: &str) {
    unchanged(t, text, what);
    for frame in frames {
        unchanged(t, &frame.replace('X', text), &format!("{what} in a frame"));
    }
    let spaced = format!(" {text} ");
    let pieces = [
        Piece::Slot("a", Slot::WORD),
        Piece::Text(&spaced),
        Piece::Slot("b", Slot::WORD),
    ];
    let out = t.typeset_template(pieces, Output::Unicode);
    assert_eq!(out.texts().len(), 3, "{what}");
    assert!(
        matches!(&out.texts()[1], Cow::Borrowed(s) if *s == spaced),
        "{what}: next to slots {} came back as {}",
        show(&spaced),
        show(&out.texts()[1])
    );
    assert!(!out.changed(), "{what}");
    let glued = [
        Piece::Text(frames[0].split('X').next().unwrap()),
        Piece::Text(text),
        Piece::Slot("n", Slot::NUMBER),
    ];
    let out = t.typeset_template(glued, Output::Unicode);
    assert_eq!(out.texts().len(), 2, "{what}");
}

#[test]
fn interface_without_nbsp_leaves_every_row_unchanged() {
    for (lang, frames) in languages() {
        let t = without_nbsp(lang.clone(), RuleSet::INTERFACE);
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
fn prose_without_nbsp_leaves_the_unchanged_rows_unchanged() {
    for (lang, frames) in languages() {
        let t = without_nbsp(lang.clone(), RuleSet::PROSE);
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
        // Row 3 in Prose: the lone `--` alone is unchanged.
        unchanged(&t, "--", &format!("{lang} prose row 3"));
    }
}

#[test]
fn interface_without_nbsp_leaves_the_stress_strings_unchanged() {
    for (lang, _) in languages() {
        let t = without_nbsp(lang.clone(), RuleSet::INTERFACE);
        for text in STRESS {
            unchanged(&t, text, &format!("{lang} {text:?}"));
        }
    }
}

#[test]
fn interface_without_nbsp_leaves_the_whitespace_cases_unchanged() {
    // Interaction strings of section 10.7 that no rule of Interface touches.
    let en = without_nbsp(Language::EN_US, RuleSet::INTERFACE);
    for text in ["word  -  word", "word\t-\tword"] {
        unchanged(&en, text, text);
    }
    let ru = without_nbsp(Language::RU_RU, RuleSet::INTERFACE);
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
