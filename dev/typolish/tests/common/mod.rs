//! Helpers shared by the integration tests (design sections 13.1 and 13.7).
//!
//! `vis` writes invisible characters as visible markers in test tables, `assert_no_markers`
//! guards against a forgotten `vis`, `show` prints invisible characters as markers on failure, and
//! `parse_tmpl` reads the test-only `.tmpl` syntax.

#![allow(dead_code)]

pub mod canonical;
pub mod corpus;
pub mod decode;
pub mod set_glue;
pub mod strategies;

use std::borrow::Cow;

use typolish::{Language, Output, Piece, Rule, RuleSet, Slot, SlotKind, Typographer};

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

/// The expectation of a table row in a full built-in set. The stage 2 tables give the output of
/// the quote, dash, range and ellipsis rules; in the full `PROSE` and `INTERFACE` the `nbsp.*`
/// rules add their glue (design section 15.4), which `set_glue::TABLE` lists for the rows it
/// touches. A typographer without the short-word rule, and a row that the table does not list, keep
/// the expectation as written.
pub fn expected_in_set(t: &Typographer, source: &str, alone: String) -> String {
    let kind = if t.rules().contains(Rule::NbspShortLength) {
        "prose"
    } else if t.rules().contains(Rule::NbspShortWords) {
        "interface"
    } else {
        return alone;
    };
    let tag = t.language().tag();
    for (lang, set, input, plain, full) in set_glue::TABLE {
        if *lang == tag && *set == kind && vis(input) == source && vis(plain) == alone {
            return vis(full);
        }
    }
    alone
}

/// Typesets a table row (input and expectation written with markers) as Unicode and compares.
#[track_caller]
pub fn check(typographer: &Typographer, row: (&str, &str)) {
    let source = input(row.0);
    let expected = expected_in_set(typographer, &source, vis(row.1));
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

/// A typographer with the rules of `set` except the `nbsp.*` rules. Stage 2 ran its set-level
/// tables with it (design section 15.4); stage 3 runs them in the full sets (`prose`, `interface`),
/// and this helper stays for the tests that look at the other rules of a set on their own.
pub fn without_nbsp(language: Language, set: RuleSet) -> Typographer {
    let mut builder = Typographer::builder(language, set);
    for rule in Rule::ALL.iter().filter(|r| r.id().starts_with("nbsp.")) {
        builder = builder.disable(*rule);
    }
    builder.build().unwrap()
}

/// The reason codes of design section 8.12: why our output differs from the recorded output of the
/// reference implementation on a `DIVERGE` row (design section 13.4).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Reason {
    /// See design section 8.12.
    NoGuess,
    /// See design section 8.12.
    LookAlike,
    /// See design section 8.12.
    NoSpaceEdit,
    /// See design section 8.12.
    NoDateRewrite,
    /// See design section 8.12.
    AmbiguousHyphen,
    /// See design section 8.12.
    PercentAsWritten,
    /// See design section 8.12.
    Technical,
    /// See design section 8.12.
    AuthorPunctuation,
    /// See design section 8.12.
    DashStyleEn,
    /// See design section 8.12.
    RangeLiteral,
    /// See design section 8.12.
    NoLastShortWord,
    /// See design section 8.12.
    Slots,
    /// See design section 8.12.
    NoSpelling,
    /// See design section 8.12.
    Apostrophe,
    /// See design section 8.12.
    NestedSingle,
    /// See design section 8.12.
    EnNbsp,
    /// See design section 8.12.
    RuNbsp,
    /// See design section 8.12.
    LegalMarkContext,
    /// See design section 8.12.
    Primes,
    /// See design section 8.12.
    WideGlue,
    /// See design section 8.12.
    NoDigitChange,
    /// See design section 8.12.
    MinusSign,
    /// See design section 8.12.
    AlreadyTypeset,
    /// See design section 8.12.
    NoTrim,
    /// See design section 8.12.
    KeepSpaces,
    /// See design section 8.12.
    EllipsisDigits,
}

impl Reason {
    /// Every reason, in the order of the table of design section 8.12.
    pub const ALL: [Reason; 26] = [
        Reason::NoGuess,
        Reason::LookAlike,
        Reason::NoSpaceEdit,
        Reason::NoDateRewrite,
        Reason::AmbiguousHyphen,
        Reason::PercentAsWritten,
        Reason::Technical,
        Reason::AuthorPunctuation,
        Reason::DashStyleEn,
        Reason::RangeLiteral,
        Reason::NoLastShortWord,
        Reason::Slots,
        Reason::NoSpelling,
        Reason::Apostrophe,
        Reason::NestedSingle,
        Reason::EnNbsp,
        Reason::RuNbsp,
        Reason::LegalMarkContext,
        Reason::Primes,
        Reason::WideGlue,
        Reason::NoDigitChange,
        Reason::MinusSign,
        Reason::AlreadyTypeset,
        Reason::NoTrim,
        Reason::KeepSpaces,
        Reason::EllipsisDigits,
    ];
}

/// The group of a reference row (design section 13.4).
#[derive(Clone, Copy, Debug)]
pub enum Group {
    /// Our output equals the recorded one.
    Agree,
    /// Our output differs: it is written out, with the reason codes of design section 8.12.
    Diverge {
        /// Our output for the input (a template input is written with its `{name}` slots).
        ours: &'static str,
        /// One or more reasons.
        reasons: &'static [Reason],
    },
}

/// A row of the generated reference tables `tests/reference/{ru,en_us}.rs` (design section 13.4):
/// the case id, the input, the output the reference implementation recorded and the group.
#[derive(Clone, Copy, Debug)]
pub struct RefRow {
    /// The case id: the corpus group, the language and the number (`neg.list.ctx.ru.519`).
    pub id: &'static str,
    /// The input, as typed.
    pub input: &'static str,
    /// The recorded output.
    pub recorded: &'static str,
    /// `AGREE` or `DIVERGE` with the reasons.
    pub group: Group,
}

/// An `AGREE` row of a reference table.
pub const fn agree(id: &'static str, input: &'static str, recorded: &'static str) -> RefRow {
    RefRow {
        id,
        input,
        recorded,
        group: Group::Agree,
    }
}

/// A `DIVERGE` row of a reference table.
pub const fn diverge(
    id: &'static str,
    input: &'static str,
    recorded: &'static str,
    ours: &'static str,
    reasons: &'static [Reason],
) -> RefRow {
    RefRow {
        id,
        input,
        recorded,
        group: Group::Diverge { ours, reasons },
    }
}

/// A typographer that runs exactly `rules`, the rest being off.
pub fn only(language: Language, rules: &[Rule]) -> Typographer {
    Typographer::new(language, rules.iter().copied().collect())
}

/// Prose in full, `nbsp.*` rules included (stage 3, design section 15.4).
pub fn prose(language: Language) -> Typographer {
    Typographer::new(language, RuleSet::PROSE)
}

/// Interface in full, `nbsp.*` rules included (stage 3, design section 15.4).
pub fn interface(language: Language) -> Typographer {
    Typographer::new(language, RuleSet::INTERFACE)
}

/// A built-in set without `rule`: the rule disabled in a set.
pub fn without_rule(language: Language, set: RuleSet, rule: Rule) -> Typographer {
    Typographer::builder(language, set)
        .disable(rule)
        .build()
        .unwrap()
}

/// The typographers a rule table runs with (design section 13.2): the rule alone, and each
/// built-in set that holds the rule, in full. The name says which one failed.
pub fn configs(language: &Language, rule: Rule) -> Vec<(String, Typographer)> {
    let mut out = vec![(
        format!("{language} {} alone", rule.id()),
        Typographer::new(language.clone(), RuleSet::EMPTY.with(rule)),
    )];
    for (name, set) in [("Prose", RuleSet::PROSE), ("Interface", RuleSet::INTERFACE)] {
        if set.contains(rule) {
            out.push((
                format!("{language} {name}"),
                Typographer::new(language.clone(), set),
            ));
        }
    }
    out
}

/// Typesets a table row (written with markers) with `t` and compares, naming `what` on failure;
/// the result is borrowed exactly when it equals the input.
#[track_caller]
pub fn check_exact(what: &str, t: &Typographer, row: (&str, &str)) {
    let source = input(row.0);
    let expected = vis(row.1);
    let got = t.typeset(&source, Output::Unicode);
    assert!(
        got == expected,
        "{what}\ninput    {}\nexpected {}\ngot      {}",
        show(&source),
        show(&expected),
        show(&got)
    );
    assert_eq!(
        matches!(got, Cow::Borrowed(_)),
        source == expected,
        "{what}: Cow::Borrowed exactly when the output equals the input: {}",
        show(&source)
    );
}

/// Typesets a table row of a stage 2 table (written with markers) with `t` and compares, naming
/// `what` on failure. In a full built-in set the expectation of the row is completed with the glue
/// of the `nbsp.*` rules (`expected_in_set`); everywhere else it is `check_exact`.
#[track_caller]
pub fn check_in(what: &str, t: &Typographer, row: (&str, &str)) {
    let source = input(row.0);
    let expected = expected_in_set(t, &source, vis(row.1));
    check_exact(what, t, (row.0, &expected));
}

/// Runs every row with every typographer.
#[track_caller]
pub fn check_rows(typographers: &[(String, Typographer)], rows: &[(&str, &str)]) {
    for (what, t) in typographers {
        for row in rows {
            check_in(what, t, *row);
        }
    }
}

/// Asserts that a row (in markers) is typeset to itself and borrowed.
#[track_caller]
pub fn check_unchanged(what: &str, t: &Typographer, text: &str) {
    check_in(what, t, (text, text));
}

/// The rules of the edits that `report` lists for a string written with markers, in order.
pub fn change_rules(t: &Typographer, text: &str) -> Vec<Rule> {
    let source = vis(text);
    t.report(&source, Output::Unicode)
        .changes()
        .iter()
        .map(|c| c.rule())
        .collect()
}

/// Asserts that every edit of `report` for the string is by `rule` and that there is at least
/// one (design section 13.2: "the `Change` record with the right rule").
#[track_caller]
pub fn assert_changed_by(what: &str, t: &Typographer, text: &str, rule: Rule) {
    let rules = change_rules(t, text);
    assert!(
        !rules.is_empty() && rules.iter().all(|r| *r == rule),
        "{what}: edits of {} should all be by {}, got {:?}",
        show(text),
        rule.id(),
        rules.iter().map(|r| r.id()).collect::<Vec<_>>()
    );
}

/// Asserts that `report` lists an edit by `rule` for the string and that every other edit is a
/// glue mark of an `nbsp.*` rule: in a full built-in set the no-break rules add their glue to the
/// edits of the rule a row exercises (design sections 8.4 and 8.7).
#[track_caller]
pub fn assert_changed_by_with_glue(what: &str, t: &Typographer, text: &str, rule: Rule) {
    let source = vis(text);
    let report = t.report(&source, Output::Unicode);
    let changes = report.changes();
    let ok = changes.iter().any(|c| c.rule() == rule)
        && changes.iter().all(|c| {
            c.rule() == rule
                || (c.rule().id().starts_with("nbsp.")
                    && matches!(c.replacement(), "\u{a0}" | "\u{202f}"))
        });
    assert!(
        ok,
        "{what}: edits of {} should be by {} and glue of nbsp rules, got {:?}",
        show(text),
        rule.id(),
        changes.iter().map(|c| c.rule().id()).collect::<Vec<_>>()
    );
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
