//! Helpers shared by the integration tests (design sections 13.1 and 13.7).
//!
//! `vis` writes invisible characters as visible markers in test tables, `assert_no_markers`
//! guards against a forgotten `vis`, `show` prints invisible characters as markers on failure, and
//! `parse_tmpl` reads the test-only `.tmpl` syntax.

#![allow(
    dead_code,
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    clippy::print_stdout,
    clippy::print_stderr,
    clippy::dbg_macro
)]

pub mod strategies;

use std::borrow::Cow;

use typolish::{Output, Piece, Rule, RuleSet, Slot, SlotKind, Typographer};

/// The marker of U+00A0 in test tables.
pub const NBSP_MARK: char = '⍽';
/// The marker of U+202F in test tables.
pub const NARROW_MARK: char = '⎵';

/// Replaces `⍽` with U+00A0 and `⎵` with U+202F.
pub fn vis(text: &str) -> String {
    text.replace(NBSP_MARK, "\u{a0}")
        .replace(NARROW_MARK, "\u{202f}")
}

/// The inverse of `vis`, for messages: U+00A0 as `⍽` and U+202F as `⎵`.
pub fn show(text: &str) -> String {
    text.replace('\u{a0}', "⍽").replace('\u{202f}', "⎵")
}

/// Asserts that a string handed to the typographer holds no marker character: a table row was
/// passed without `vis`.
#[track_caller]
pub fn assert_no_markers(text: &str) {
    assert!(
        !text.contains(NBSP_MARK) && !text.contains(NARROW_MARK),
        "marker character in a string passed to the typographer, `vis` was forgotten: {text:?}"
    );
}

/// `vis` followed by `assert_no_markers`: the input of a table row.
#[track_caller]
pub fn input(text: &str) -> String {
    let text = vis(text);
    assert_no_markers(&text);
    text
}

/// Typesets a table row (input and expectation written with markers) as Unicode and compares.
#[track_caller]
pub fn check(typographer: &Typographer, row: (&str, &str)) {
    let source = input(row.0);
    let expected = vis(row.1);
    let got = typographer.typeset(&source, Output::Unicode);
    assert!(
        got == expected,
        "input    {}\nexpected {}\ngot      {}",
        show(&source),
        show(&expected),
        show(&got)
    );
    assert_eq!(
        matches!(got, Cow::Borrowed(_)),
        source == expected,
        "Cow::Borrowed exactly when the output equals the input: {}",
        show(&source)
    );
}

/// A typographer with the rules of `set` except the `nbsp.*` rules (stage 2 runs set-level tables
/// without them, section 15.4).
pub fn without_nbsp(language: typolish::Language, set: RuleSet) -> Typographer {
    let mut builder = Typographer::builder(language, set);
    for rule in Rule::ALL.iter().filter(|r| r.id().starts_with("nbsp.")) {
        builder = builder.disable(*rule);
    }
    builder.build().unwrap()
}

/// One piece of a parsed `.tmpl` message.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TmplPiece {
    /// Text.
    Text(String),
    /// A slot: its name and kind.
    Slot(String, SlotKind),
}

/// One message of a `.tmpl` file.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TmplMessage {
    /// The id before `=`.
    pub id: String,
    /// The pieces in order.
    pub pieces: Vec<TmplPiece>,
}

impl TmplMessage {
    /// The message as pieces for `typeset_template`, the slot payload being its name.
    pub fn pieces(&self) -> Vec<Piece<'_, &str>> {
        self.pieces
            .iter()
            .map(|p| match p {
                TmplPiece::Text(t) => Piece::Text(t),
                TmplPiece::Slot(name, kind) => Piece::Slot(name.as_str(), Slot::new(*kind)),
            })
            .collect()
    }

    /// The plain text with every slot written as `{name}`.
    pub fn flat(&self) -> String {
        self.pieces
            .iter()
            .map(|p| match p {
                TmplPiece::Text(t) => t.clone(),
                TmplPiece::Slot(name, _) => format!("{{{name}}}"),
            })
            .collect()
    }
}

/// Parses the `.tmpl` syntax: one message per line, `id = text {n:number} {name:word} {d:date}
/// {x}`; `{x}` without a kind is an `other` slot; empty lines and lines starting with `#` are
/// skipped; markers of `vis` are applied to the text.
pub fn parse_tmpl(source: &str) -> Vec<TmplMessage> {
    let mut messages = Vec::new();
    for line in source.lines() {
        if line.trim().is_empty() || line.starts_with('#') {
            continue;
        }
        let (id, text) = line
            .split_once(" = ")
            .unwrap_or_else(|| panic!("no ` = `: {line:?}"));
        let mut pieces = Vec::new();
        let mut rest = text;
        let mut buffer = String::new();
        while let Some(open) = rest.find('{') {
            buffer.push_str(&rest[..open]);
            let close = rest[open..]
                .find('}')
                .unwrap_or_else(|| panic!("unclosed slot: {line:?}"))
                + open;
            let body = &rest[open + 1..close];
            let (name, kind) = match body.split_once(':') {
                Some((name, "number")) => (name, SlotKind::Number),
                Some((name, "word")) => (name, SlotKind::Word),
                Some((name, "date")) => (name, SlotKind::Date),
                Some((name, "other")) => (name, SlotKind::Other),
                Some((_, kind)) => panic!("unknown slot kind {kind:?}: {line:?}"),
                None => (body, SlotKind::Other),
            };
            if !buffer.is_empty() {
                pieces.push(TmplPiece::Text(vis(&std::mem::take(&mut buffer))));
            }
            pieces.push(TmplPiece::Slot(name.to_owned(), kind));
            rest = &rest[close + 1..];
        }
        buffer.push_str(rest);
        if !buffer.is_empty() {
            pieces.push(TmplPiece::Text(vis(&buffer)));
        }
        messages.push(TmplMessage {
            id: id.trim().to_owned(),
            pieces,
        });
    }
    messages
}
