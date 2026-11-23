//! The source check `lint` and `lint_template` and every diagnostic of design section 12.4
//! (design sections 2.9, 12.3, 12.4 and 13.2, `tests/lint.rs`).
//!
//! Every diagnostic kind has a firing and a non-firing row. Each row runs through `lint` and
//! through `report`, which must agree on everything but `TypographicChar` (section 12.3). The
//! rows of `TypographicChar` follow the list of section 13.2. Rows are written with the markers of
//! `vis`: `⍽` is U+00A0 and `⎵` is U+202F.

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
use std::ops::Range;

use common::corpus::{FILES, LANGS, SETS, messages, text_lines, typographer};
use common::{interface, prose, show, vis};
use typolish::{
    Diagnostic, DiagnosticKind, HtmlStyle, Language, Output, Piece, Rule, RuleSet, Severity, Slot,
    Typographer,
};

use DiagnosticKind::{
    AmbiguousHyphen, AmbiguousQuote, CrossingQuotes, MixedScript, NowrapFallback, SlotGlue,
    UnclosedDelimiter, UnmatchedQuote,
};
use Severity::{Info, Warning};

const RU: Language = Language::RU_RU;
const EN: Language = Language::EN_US;

/// A diagnostic of a string without its piece: kind, severity, rule and byte range.
type Exp = (DiagnosticKind, Severity, Option<Rule>, Range<usize>);

/// A diagnostic in full: kind, severity, rule, piece and byte range.
type Full = (DiagnosticKind, Severity, Option<Rule>, usize, Range<usize>);

fn full(d: &Diagnostic) -> Full {
    (
        d.kind().clone(),
        d.severity(),
        d.rule(),
        d.piece(),
        d.range(),
    )
}

fn is_char(kind: &DiagnosticKind) -> bool {
    matches!(kind, DiagnosticKind::TypographicChar { .. })
}

/// The diagnostics other than `TypographicChar`.
fn sans_chars(diagnostics: &[Diagnostic]) -> Vec<Full> {
    diagnostics
        .iter()
        .filter(|d| !is_char(d.kind()))
        .map(full)
        .collect()
}

/// The `TypographicChar` entries as `(piece, range, found)`.
fn chars(diagnostics: &[Diagnostic]) -> Vec<(usize, Range<usize>, char)> {
    diagnostics
        .iter()
        .filter_map(|d| match d.kind() {
            DiagnosticKind::TypographicChar { found, .. } => Some((d.piece(), d.range(), *found)),
            _ => None,
        })
        .collect()
}

/// The `found` characters of a string, in order.
fn found(t: &Typographer, text: &str) -> Vec<char> {
    chars(&t.lint(text)).into_iter().map(|c| c.2).collect()
}

fn string_exp(diagnostics: &[Diagnostic]) -> Vec<Exp> {
    sans_chars(diagnostics)
        .into_iter()
        .map(|(kind, severity, rule, piece, range)| {
            assert_eq!(piece, 0, "string input has one piece");
            (kind, severity, rule, range)
        })
        .collect()
}

/// Checks the diagnostics of a string other than `TypographicChar`, through `lint` and through
/// `report(x, Output::Unicode)`.
#[track_caller]
fn check(what: &str, t: &Typographer, text: &str, expected: &[Exp]) {
    let linted = t.lint(text);
    assert_eq!(
        string_exp(&linted),
        expected,
        "{what}: lint of {}",
        show(text)
    );
    let report = t.report(text, Output::Unicode);
    assert_eq!(
        string_exp(report.diagnostics()),
        expected,
        "{what}: report of {}",
        show(text)
    );
}

/// The first byte range of `needle` in `text`.
#[track_caller]
fn at(text: &str, needle: &str) -> Range<usize> {
    nth(text, needle, 0)
}

#[track_caller]
fn nth(text: &str, needle: &str, n: usize) -> Range<usize> {
    let start = text
        .match_indices(needle)
        .nth(n)
        .unwrap_or_else(|| panic!("no occurrence {n} of {needle:?} in {text:?}"))
        .0;
    start..start + needle.len()
}

/// Typographers of both built-in sets for a language.
fn sets(language: &Language) -> Vec<(String, Typographer)> {
    [("Prose", RuleSet::PROSE), ("Interface", RuleSet::INTERFACE)]
        .into_iter()
        .map(|(name, set)| {
            (
                format!("{language} {name}"),
                Typographer::new(language.clone(), set),
            )
        })
        .collect()
}

/// Typographers of every built-in set for a language, `EMPTY` included.
fn all_sets(language: &Language) -> Vec<(String, Typographer)> {
    let mut out = sets(language);
    for (name, set) in [("Empty", RuleSet::EMPTY), ("All", RuleSet::ALL)] {
        out.push((
            format!("{language} {name}"),
            Typographer::new(language.clone(), set),
        ));
    }
    out
}

#[track_caller]
fn check_in_sets(language: &Language, text: &str, expected: &[Exp]) {
    for (what, t) in sets(language) {
        check(&what, &t, text, expected);
    }
}

// ---------------------------------------------------------------------------------------------
// The diagnostics of section 12.4, one firing and one non-firing row each.
// ---------------------------------------------------------------------------------------------

#[test]
fn unmatched_quote_warning() {
    // Firing (section 12.4, the example of section 2.9): a straight quote without a pair.
    let text = "Он сказал \"привет.";
    check_in_sets(
        &RU,
        text,
        &[(UnmatchedQuote, Warning, Some(Rule::QuotesPairs), 18..19)],
    );
    // The leftover opener of a window that does not converge is the first quote.
    check_in_sets(
        &RU,
        "\"Один \"два три\"",
        &[(UnmatchedQuote, Warning, Some(Rule::QuotesPairs), 0..1)],
    );
    check_in_sets(
        &EN,
        "Don't \"x",
        &[(UnmatchedQuote, Warning, Some(Rule::QuotesPairs), 6..7)],
    );
    // Not firing: the quotes pair.
    check_in_sets(&RU, "Он сказал \"привет\".", &[]);
    check_in_sets(&EN, "He said \"hi\".", &[]);
}

#[test]
fn unmatched_quote_info() {
    // Firing: a typed quote glyph of the language without a partner stays neutral.
    let text = "Цена 27»";
    check_in_sets(
        &RU,
        text,
        &[(UnmatchedQuote, Info, Some(Rule::QuotesPairs), at(text, "»"))],
    );
    let text = "He said ‘x’(see note)";
    check_in_sets(
        &EN,
        text,
        &[(UnmatchedQuote, Info, Some(Rule::QuotesPairs), at(text, "‘"))],
    );
    // Not firing: a typed pair.
    check_in_sets(&RU, "Цена «27»", &[]);
}

#[test]
fn unmatched_quote_needs_its_rule() {
    // Raised by `quotes.pairs` (section 12.4): with the rule off there is no diagnostic.
    for (what, t) in [
        ("empty", Typographer::new(RU, RuleSet::EMPTY)),
        (
            "interface without quotes.pairs",
            Typographer::new(RU, RuleSet::INTERFACE.without(Rule::QuotesPairs)),
        ),
    ] {
        check(what, &t, "Он сказал \"привет.", &[]);
    }
}

#[test]
fn crossing_quotes() {
    // Firing: pairs would cross; the diagnostic sits at the covering entry (section 6.4).
    check_in_sets(
        &RU,
        "«a \"b» c\"",
        &[(CrossingQuotes, Warning, Some(Rule::QuotesPairs), 4..5)],
    );
    // A neutral typed closer inside a new pair of its family (section 6.6).
    let text = "Он сказал \"a » b\"";
    for (what, t) in sets(&RU) {
        let got = string_exp(&t.lint(text));
        assert!(
            got.iter()
                .any(|d| d.0 == CrossingQuotes && d.1 == Warning && d.2 == Some(Rule::QuotesPairs)),
            "{what}: {got:?}"
        );
        assert!(got.iter().any(|d| d.0 == UnmatchedQuote && d.1 == Info));
    }
    // Not firing: a typed pair and a straight pair side by side.
    check_in_sets(&RU, "«a» \"b\" c", &[]);
}

#[test]
fn ambiguous_quote() {
    // Firing: a straight quote with spaces on both sides has no role.
    let text = "Слово \" в тексте.";
    check_in_sets(
        &RU,
        text,
        &[(
            AmbiguousQuote,
            Info,
            Some(Rule::QuotesPairs),
            at(text, "\""),
        )],
    );
    // One of three quotes in a series: no pair inside it.
    let text = "Он сказал \"\"\" и ушёл.";
    let expected: Vec<Exp> = (0..3)
        .map(|n| {
            (
                AmbiguousQuote,
                Info,
                Some(Rule::QuotesPairs),
                nth(text, "\"", n),
            )
        })
        .collect();
    check_in_sets(&RU, text, &expected);
    // Not firing: the quote has a role.
    check_in_sets(&RU, "Слово \"в\" тексте.", &[]);
    // An empty sign-only pair is a pair (section 6.3).
    check_in_sets(&RU, "Он сказал \"\" и ушёл.", &[]);
}

#[test]
fn unclosed_delimiter() {
    // Firing: an opening delimiter without its closing one; the rule is none (protection).
    let text = "don`t";
    for (what, t) in all_sets(&EN).into_iter().chain(all_sets(&RU)) {
        check(
            &what,
            &t,
            text,
            &[(UnclosedDelimiter, Warning, None, at(text, "`"))],
        );
    }
    // Two stray backticks in different paragraphs do not pair (section 10.1).
    let text = "Don`t.\n\nОн сказал \"да\".\n\nIt`s ok.";
    check(
        "ru interface",
        &interface(RU),
        text,
        &[
            (UnclosedDelimiter, Warning, None, nth(text, "`", 0)),
            (UnclosedDelimiter, Warning, None, nth(text, "`", 1)),
        ],
    );
    // Not firing: a pair.
    for (what, t) in all_sets(&RU) {
        check(&what, &t, "`code`", &[]);
        check(&what, &t, "Запустите `ls -la` сейчас", &[]);
    }
    // An own pair works the same way.
    let t = Typographer::builder(RU, RuleSet::INTERFACE)
        .protect_between("{{", "}}")
        .build()
        .unwrap();
    check("own pair", &t, "a {{x}} b", &[]);
    let text = "a {{x b";
    check(
        "own pair, unclosed",
        &t,
        text,
        &[(UnclosedDelimiter, Warning, None, at(text, "{{"))],
    );
}

#[test]
fn ambiguous_hyphen() {
    // Firing: the `-` that section 8.4 refuses, each with its range at the hyphen.
    let rows: &[(&Language, &str, &str)] = &[
        (&RU, "10 - 20 штук", "-"),
        (&RU, "Выполните cat - file", "-"),
        (&RU, "tls-server - agent-ca", "-"),
        (&RU, "Выполните diff -u - file.txt", "-"),
        (&EN, "x - 5", "-"),
        (&EN, "tar -cf - -C dir", "-"),
    ];
    for (language, text, hyphen) in rows {
        for (what, t) in sets(language) {
            let got = string_exp(&t.lint(text));
            let hyphens: Vec<_> = got.iter().filter(|d| d.0 == AmbiguousHyphen).collect();
            assert_eq!(hyphens.len(), 1, "{what}: {}: {got:?}", show(text));
            assert_eq!(hyphens[0].1, Info, "{what}: {text}");
            assert_eq!(hyphens[0].2, Some(Rule::DashSpacedHyphen), "{what}: {text}");
            assert_eq!(&text[hyphens[0].3.clone()], *hyphen, "{what}: {text}");
            check(&what, &t, text, &got);
        }
    }
    // An exact row.
    check_in_sets(
        &RU,
        "10 - 20 штук",
        &[(AmbiguousHyphen, Info, Some(Rule::DashSpacedHyphen), 3..4)],
    );
    // `--` is refused by `dash.double-hyphen` (Prose).
    let text = "npm test -- --watch";
    let got = string_exp(&prose(RU).lint(text));
    let hyphens: Vec<_> = got.iter().filter(|d| d.0 == AmbiguousHyphen).collect();
    assert_eq!(hyphens.len(), 1, "{got:?}");
    assert_eq!(hyphens[0].1, Info);
    assert_eq!(hyphens[0].2, Some(Rule::DashDoubleHyphen));
    assert_eq!(&text[hyphens[0].3.clone()], "--");
    // Not firing: the dash is set.
    check_in_sets(&RU, "Москва - столица", &[]);
    check_in_sets(&EN, "Moscow - the capital", &[]);
    check("ru prose", &prose(RU), "Москва -- столица", &[]);
    // The rule that raises it is off: no diagnostic.
    check(
        "empty",
        &Typographer::new(RU, RuleSet::EMPTY),
        "10 - 20 штук",
        &[],
    );
}

#[test]
fn mixed_script() {
    // Firing: Latin `o` and `C` inside Cyrillic words.
    let moskva = "\u{41c}o\u{441}\u{43a}\u{432}\u{430}";
    let server = "C\u{435}\u{440}\u{432}\u{435}\u{440}";
    for (what, t) in all_sets(&RU).into_iter().chain(all_sets(&EN)) {
        check(
            &what,
            &t,
            moskva,
            &[(MixedScript, Warning, None, 0..moskva.len())],
        );
        check(
            &what,
            &t,
            server,
            &[(MixedScript, Warning, None, 0..server.len())],
        );
        let text = format!("Город {moskva} большой");
        check(
            &what,
            &t,
            &text,
            &[(MixedScript, Warning, None, at(&text, moskva))],
        );
    }
    // Not firing: one script, a word of another script, and a dictionary entry (`μs`).
    for (what, t) in all_sets(&RU) {
        check(&what, &t, "Москва", &[]);
        check(&what, &t, "Привет, world", &[]);
    }
    for (what, t) in all_sets(&EN) {
        check(&what, &t, "Latency 5 \u{3bc}s.", &[]);
        check(&what, &t, "Latency 5 \u{b5}s.", &[]);
    }
}

#[test]
fn inputs_without_a_trigger_byte_keep_their_diagnostics() {
    // Section 3.2, no prefilter: `Мoсква` has no byte that a rule reacts to.
    let moskva = "\u{41c}o\u{441}\u{43a}\u{432}\u{430}";
    let t = interface(RU);
    let linted = t.lint(moskva);
    assert_eq!(linted.len(), 1);
    assert_eq!(linted[0].kind(), &MixedScript);
    let report = t.report(moskva, Output::Unicode);
    assert_eq!(report.diagnostics().len(), 1);
    assert!(matches!(report.result(), Cow::Borrowed(_)));
    // `` `code` `` has one `Delimited` record in `report`, and `lint` reports nothing.
    let report = t.report("`code`", Output::Unicode);
    assert_eq!(report.protected().len(), 1);
    assert_eq!(
        report.protected()[0].kind(),
        typolish::ProtectedKind::Delimited
    );
    assert!(t.lint("`code`").is_empty());
}

#[test]
fn slot_glue() {
    // Firing, warning: a `Number` slot glued to letters; at the slot, with the empty range.
    let t = interface(RU);
    let pieces = || [Piece::Slot("n", Slot::NUMBER), Piece::Text("МБ")];
    let linted = t.lint_template(pieces());
    assert_eq!(
        linted.iter().map(full).collect::<Vec<_>>(),
        [(SlotGlue, Warning, None, 0, 0..0)]
    );
    let report = t.report_template(pieces(), Output::Unicode);
    assert_eq!(sans_chars(report.diagnostics()), sans_chars(&linted));
    // `МБ{n}`: reported at the letters.
    let linted = t.lint_template([Piece::Text("МБ"), Piece::Slot("n", Slot::NUMBER)]);
    assert_eq!(
        linted.iter().map(full).collect::<Vec<_>>(),
        [(SlotGlue, Warning, None, 0, 0..4)]
    );
    // Firing, info: two slots glued together, whatever their kinds; at the first slot.
    for (a, b) in [
        (Slot::WORD, Slot::WORD),
        (Slot::NUMBER, Slot::NUMBER),
        (Slot::DATE, Slot::OTHER),
        (Slot::OTHER, Slot::WORD),
    ] {
        let pieces = || [Piece::Slot("a", a), Piece::Slot("b", b)];
        let linted = t.lint_template(pieces());
        assert_eq!(
            linted.iter().map(full).collect::<Vec<_>>(),
            [(SlotGlue, Info, None, 0, 0..0)],
            "{a:?} {b:?}"
        );
        let report = t.report_template(pieces(), Output::Unicode);
        assert_eq!(sans_chars(report.diagnostics()), sans_chars(&linted));
    }
    // The piece is the slot's piece.
    let linted = t.lint_template([
        Piece::Text("Файл "),
        Piece::Slot("a", Slot::WORD),
        Piece::Slot("b", Slot::WORD),
    ]);
    assert_eq!(
        linted.iter().map(full).collect::<Vec<_>>(),
        [(SlotGlue, Info, None, 1, 0..0)]
    );
    // Not firing: a space between, a `Word` slot next to letters, a slot next to punctuation.
    for pieces in [
        vec![Piece::Slot("n", Slot::NUMBER), Piece::Text(" МБ")],
        vec![Piece::Slot("n", Slot::NUMBER), Piece::Text(" ")],
        vec![
            Piece::Slot("a", Slot::WORD),
            Piece::Text(" "),
            Piece::Slot("b", Slot::WORD),
        ],
        vec![Piece::Slot("w", Slot::WORD), Piece::Text("ов")],
        vec![Piece::Text("ов"), Piece::Slot("w", Slot::WORD)],
        vec![Piece::Slot("n", Slot::NUMBER), Piece::Text(".")],
        vec![Piece::Slot("n", Slot::NUMBER)],
        vec![
            Piece::Slot("a", Slot::WORD),
            Piece::Markup("<b>"),
            Piece::Text(" x"),
        ],
    ] {
        for (what, t) in all_sets(&RU) {
            let linted = t.lint_template(pieces.iter().copied());
            assert!(
                linted.iter().all(|d| d.kind() != &SlotGlue),
                "{what}: {pieces:?}: {linted:?}"
            );
        }
    }
}

#[test]
fn slot_glue_in_every_set() {
    for (what, t) in all_sets(&RU).into_iter().chain(all_sets(&EN)) {
        let linted = t.lint_template([Piece::Slot("n", Slot::NUMBER), Piece::Text("MB")]);
        assert_eq!(
            linted.iter().map(full).collect::<Vec<_>>(),
            [(SlotGlue, Warning, None, 0, 0..0)],
            "{what}"
        );
    }
}

fn nowrap() -> Output {
    Output::Html(HtmlStyle::NAMED.nowrap_class("nowrap"))
}

#[test]
fn nowrap_fallback_comes_from_report_template_only() {
    let t = interface(RU);
    let pieces = || [Piece::Text("в "), Piece::Markup("<b>"), Piece::Text("дом")];
    // Firing: a group split at a `Markup` piece, reported at that piece with its whole range.
    let report = t.report_template::<()>(pieces(), nowrap());
    let got: Vec<Full> = report
        .diagnostics()
        .iter()
        .filter(|d| d.kind() == &NowrapFallback)
        .map(full)
        .collect();
    assert_eq!(got, [(NowrapFallback, Info, None, 1, 0..3)]);
    // `lint` renders nothing: it never gives `NowrapFallback`.
    assert!(
        t.lint_template::<()>(pieces())
            .iter()
            .all(|d| d.kind() != &NowrapFallback)
    );
    // Not firing: another output kind, or markup outside any group.
    for output in [Output::Unicode, Output::Html(HtmlStyle::NAMED)] {
        let report = t.report_template::<()>(pieces(), output);
        assert!(
            report
                .diagnostics()
                .iter()
                .all(|d| d.kind() != &NowrapFallback)
        );
    }
    let report = t.report_template::<()>(
        [
            Piece::Markup("<b>"),
            Piece::Text("в дом"),
            Piece::Markup("</b>"),
        ],
        nowrap(),
    );
    assert!(
        report
            .diagnostics()
            .iter()
            .all(|d| d.kind() != &NowrapFallback)
    );
}

// ---------------------------------------------------------------------------------------------
// TypographicChar (sections 12.3 and 13.2).
// ---------------------------------------------------------------------------------------------

/// The entries of `TypographicChar` of a string with their texts.
fn char_entries<'a>(t: &Typographer, text: &'a str) -> Vec<(char, &'a str)> {
    chars(&t.lint(text))
        .into_iter()
        .map(|(piece, range, c)| {
            assert_eq!(piece, 0);
            (c, &text[range])
        })
        .collect()
}

#[test]
fn a_typographic_character_is_reported_once_at_its_place() {
    let t = prose(RU);
    // `—` between words with the dash rules on.
    let text = "Москва — столица";
    let linted = t.lint(text);
    let entries = chars(&linted);
    assert_eq!(entries, [(0, 13..16, '—')]);
    assert_eq!(linted[0].severity(), Info);
    assert_eq!(linted[0].rule(), None);
    assert_eq!(linted[0].piece(), 0);
    // Quotes: both glyphs of the pair.
    let text = "Он сказал «привет» и ушёл.";
    assert_eq!(char_entries(&interface(RU), text), [('«', "«"), ('»', "»")]);
    // An ellipsis.
    let text = "Подождите…";
    assert_eq!(char_entries(&interface(RU), text), [('…', "…")]);
    // A plain source has none.
    for text in [
        "Он - \"врач\", см. host:8080",
        "Москва - столица",
        "Подождите...",
        "Цена - 5 руб.",
        "",
    ] {
        assert_eq!(found(&t, text), [], "{}", show(text));
    }
}

#[test]
fn the_typed_dash_with_spaces_is_not_reported_but_its_glue_is() {
    // Section 13.2: a typed `–` with spaces is not a product of the range rules, so `to_plain`
    // returns the input and `lint` is silent; only the glue that `nbsp.dash` set before it is
    // reported, once, on the U+00A0.
    let t = prose(RU);
    for text in ["Москва – столица", "10 – 20"] {
        assert_eq!(t.to_plain(text), text);
        assert_eq!(found(&t, text), [], "{}", show(text));
    }
    for (text, glue) in [("Москва⍽– столица", 12..14), ("10⍽– 20", 2..4)] {
        let source = vis(text);
        let entries = chars(&t.lint(&source));
        assert_eq!(entries, [(0, glue, '\u{a0}')], "{text}");
    }
}

#[test]
fn glue_set_by_a_rule_is_reported_where_the_rule_is_on() {
    // Section 12.1: with `nbsp.units` alone, `5⍽МБ в⍽доме` → `5 МБ в⍽доме`.
    let t = Typographer::new(RU, RuleSet::EMPTY.with(Rule::NbspUnits));
    let text = vis("5⍽МБ в⍽доме");
    assert_eq!(t.to_plain(&text), vis("5 МБ в⍽доме"));
    let entries = chars(&t.lint(&text));
    assert_eq!(entries, [(0, 1..3, '\u{a0}')]);
    // Glue that no enabled rule sets stays, and is not reported.
    let none = Typographer::new(RU, RuleSet::EMPTY);
    assert_eq!(found(&none, &text), []);
    // Prose: the units rule and the short-word rule both set glue.
    let text = vis("5⍽МБ в⍽доме");
    let got = found(&prose(RU), &text);
    assert!(got.contains(&'\u{a0}'), "{got:?}");
}

#[test]
fn typographic_characters_that_no_enabled_rule_produces_are_not_reported() {
    // `—` with the dash rules off.
    let text = "Москва — столица";
    assert_eq!(found(&Typographer::new(RU, RuleSet::EMPTY), text), []);
    assert_eq!(
        found(&interface(RU), "Москва — столица"),
        ['—'],
        "ru Interface has `dash.spaced-hyphen`"
    );
    let no_dash = Typographer::builder(RU, RuleSet::PROSE)
        .disable(Rule::DashSpacedHyphen)
        .disable(Rule::DashDoubleHyphen)
        .disable(Rule::DashDialogue)
        .build()
        .unwrap();
    assert_eq!(found(&no_dash, text), []);
    assert_eq!(no_dash.to_plain(text), text);
    // `«»` with `quotes.pairs` off.
    let no_quotes = Typographer::builder(RU, RuleSet::PROSE)
        .disable(Rule::QuotesPairs)
        .build()
        .unwrap();
    assert_eq!(found(&no_quotes, "Он сказал «привет»."), []);
    // A closed `—` next to a number is no product of the dash rules (design section 12.1).
    assert_eq!(found(&prose(EN), "in 2020—the year"), []);
    assert_eq!(found(&prose(RU), "в 2020—году"), []);
    assert_eq!(found(&prose(EN), "word—word"), ['—']);
}

#[test]
fn legal_marks_follow_their_rule_and_their_context() {
    let text = "© 2026";
    // Present for `©` in `© 2026` with `symbols.copyright` on (a lone mark is not frozen).
    for (what, t) in [
        ("ru prose", prose(RU)),
        ("en prose", prose(EN)),
        (
            "ru copyright alone",
            Typographer::new(RU, RuleSet::EMPTY.with(Rule::SymbolsCopyright)),
        ),
    ] {
        let linted = t.lint(text);
        assert_eq!(chars(&linted), [(0, 0..2, '©')], "{what}");
        assert_ne!(t.to_plain(text), text, "{what}");
    }
    // Absent with the rule off.
    for (what, t) in [
        ("ru interface", interface(RU)),
        ("ru empty", Typographer::new(RU, RuleSet::EMPTY)),
        (
            "ru prose without copyright",
            Typographer::builder(RU, RuleSet::PROSE)
                .disable(Rule::SymbolsCopyright)
                .build()
                .unwrap(),
        ),
    ] {
        assert_eq!(found(&t, text), [], "{what}");
        assert_eq!(t.to_plain(text), text, "{what}");
    }
    // Absent for `©` outside the contexts of the rule: `to_plain` returns the input.
    for text in ["© Example Corp", "® знак", "™ знак"] {
        for (what, t) in sets(&RU) {
            assert_eq!(t.to_plain(text), text, "{what}: {text}");
            assert_eq!(found(&t, text), [], "{what}: {text}");
        }
    }
    // The other marks and the arrows of section 12.1.
    let t = prose(EN);
    assert_eq!(found(&t, "Brand™"), ['™']);
    assert_eq!(found(&t, "x → y"), ['→']);
}

#[test]
fn protected_spans_and_markup_hold_no_typographic_character() {
    let t = prose(RU);
    // A frozen fragment.
    assert_eq!(found(&t, "https://example.com/a–b"), []);
    assert_eq!(found(&t, "Адрес https://example.com/a–b — здесь"), ['—']);
    // Backticks.
    assert_eq!(found(&t, "Запустите `a—b` сейчас"), []);
    assert_eq!(found(&t, "`«x» … —`"), []);
    assert_eq!(found(&interface(RU), "Запустите `a—b` сейчас"), []);
    // Own pairs.
    let own = Typographer::builder(RU, RuleSet::PROSE)
        .protect_between("{{", "}}")
        .build()
        .unwrap();
    assert_eq!(found(&own, "a {{«x»—…}} b"), []);
    assert_eq!(found(&own, "a «x» b"), ['«', '»']);
    // Quotes outside a frozen fragment are still reported.
    assert_eq!(found(&t, "«x» https://example.com/a–b"), ['«', '»']);
    // A `Markup` piece and a `Protected` piece.
    for (what, t) in sets(&RU) {
        let linted = t.lint_template::<()>([
            Piece::Text("Москва"),
            Piece::Markup("<i>—«»…</i>"),
            Piece::Text("столица"),
        ]);
        assert!(chars(&linted).is_empty(), "{what}: {linted:?}");
        let linted = t.lint_template::<()>([
            Piece::Text("Запустите "),
            Piece::Protected("a—«»…b"),
            Piece::Text(" сейчас"),
        ]);
        assert!(chars(&linted).is_empty(), "{what}: {linted:?}");
    }
    // A `Text` piece next to them is checked: the piece is its own.
    let linted = prose(RU).lint_template::<()>([
        Piece::Markup("<b>"),
        Piece::Text("Москва — столица"),
        Piece::Markup("</b>"),
    ]);
    assert_eq!(chars(&linted), [(1, 13..16, '—')]);
}

#[test]
fn the_example_of_section_2_10_has_five_entries() {
    // `to_plain` gives `Файл "report.pdf" не найден - проверьте`; every character it changes is
    // an entry.
    let t = interface(RU);
    let text = vis("Файл «report.pdf» не⍽найден⍽— проверьте");
    assert_eq!(
        t.to_plain(&text),
        "Файл \"report.pdf\" не найден - проверьте"
    );
    let entries = chars(&t.lint(&text));
    let letters: Vec<char> = entries.iter().map(|e| e.2).collect();
    assert_eq!(letters, ['«', '»', '\u{a0}', '\u{a0}', '—']);
    for (piece, range, c) in &entries {
        assert_eq!(*piece, 0);
        assert_eq!(&text[range.clone()], c.to_string());
    }
}

#[test]
fn typographic_char_is_reported_for_templates_by_piece() {
    let t = prose(RU);
    let pieces = || {
        [
            Piece::Text("Москва — "),
            Piece::Slot("c", Slot::WORD),
            Piece::Text(" «x»"),
        ]
    };
    let linted = t.lint_template(pieces());
    let entries = chars(&linted);
    assert_eq!(
        entries.iter().map(|e| (e.0, e.2)).collect::<Vec<_>>(),
        [(0, '—'), (2, '«'), (2, '»')]
    );
    assert_eq!(entries[0].1, 13..16);
    // `lint_template` has a `TypographicChar` exactly when `to_plain_template` changes a text.
    let plain = t.to_plain_template(pieces());
    assert_ne!(plain.texts()[0], "Москва — ");
    let texts = t.to_plain_template([Piece::Text("Москва - "), Piece::Slot("c", Slot::WORD)]);
    assert_eq!(texts.texts(), ["Москва - ", ""]);
    assert!(
        chars(&t.lint_template([Piece::Text("Москва - "), Piece::Slot("c", Slot::WORD)]))
            .is_empty()
    );
}

#[test]
fn typographic_char_finds_its_piece_in_a_long_template() {
    // Many `Text` pieces between `Markup` pieces, some empty and some without a candidate: every
    // entry names its own piece and the bytes of its character there, and the plain texts follow
    // (sections 12.1 and 12.4). The lookup of the piece of a candidate walks both lists once
    // (section 3.12), so the length is not a problem.
    let t = prose(RU);
    let mut pieces: Vec<Piece<'_, ()>> = Vec::new();
    for n in 0..3000 {
        pieces.push(match n % 3 {
            0 => Piece::Text("Он «привет» — да "),
            1 => Piece::Text(""),
            _ => Piece::Text("просто текст "),
        });
        pieces.push(Piece::Markup("<b>"));
    }
    let linted = t.lint_template(pieces.iter().copied());
    let entries = chars(&linted);
    assert_eq!(entries.len(), 1000 * 3);
    for (piece, range, c) in &entries {
        assert_eq!(piece % 6, 0, "{piece}");
        let Some(Piece::Text(text)) = pieces.get(*piece) else {
            panic!("{piece}");
        };
        assert_eq!(&text[range.clone()], c.to_string());
    }
    let plain = t.to_plain_template(pieces.iter().copied());
    assert_eq!(
        plain.texts(),
        ["Он \"привет\" - да <b><b>просто текст <b>".repeat(1000)]
    );
}

// ---------------------------------------------------------------------------------------------
// The order, the equality with `report`, `Display`.
// ---------------------------------------------------------------------------------------------

/// The position of a kind in the declaration of `DiagnosticKind` (section 12.4).
fn kind_order(kind: &DiagnosticKind) -> usize {
    match kind {
        UnmatchedQuote => 0,
        CrossingQuotes => 1,
        AmbiguousQuote => 2,
        UnclosedDelimiter => 3,
        AmbiguousHyphen => 4,
        MixedScript => 5,
        SlotGlue => 6,
        NowrapFallback => 7,
        DiagnosticKind::TypographicChar { .. } => 8,
        _ => 9,
    }
}

#[track_caller]
fn assert_ordered(what: &str, diagnostics: &[Diagnostic]) {
    let key = |d: &Diagnostic| {
        let r = d.range();
        (d.piece(), r.start, r.end, kind_order(d.kind()))
    };
    for pair in diagnostics.windows(2) {
        assert!(
            key(&pair[0]) <= key(&pair[1]),
            "{what}: out of order: {:?} before {:?}",
            pair[0],
            pair[1]
        );
    }
}

#[test]
fn diagnostics_are_ordered_by_piece_start_end_and_kind() {
    // A string: the quote, the mixed word, the dash.
    let moskva = "\u{41c}o\u{441}\u{43a}\u{432}\u{430}";
    let text = format!("Он сказал \"привет. {moskva} — да");
    let t = prose(RU);
    let linted = t.lint(&text);
    assert_ordered("string", &linted);
    let kinds: Vec<&DiagnosticKind> = linted.iter().map(|d| d.kind()).collect();
    assert!(kinds.contains(&&UnmatchedQuote), "{kinds:?}");
    assert!(kinds.contains(&&MixedScript), "{kinds:?}");
    let report = t.report(&text, Output::Unicode);
    assert_ordered("report", report.diagnostics());
    assert_eq!(sans_chars(&linted), sans_chars(report.diagnostics()));
    // A template: the mixed word in piece 0, `{n}МБ` at the slot, `{a}{b}` at the first slot.
    let t = interface(RU);
    let pieces = || {
        [
            Piece::Text("\u{41c}o\u{441} "),
            Piece::Slot("n", Slot::NUMBER),
            Piece::Text("МБ "),
            Piece::Slot("a", Slot::WORD),
            Piece::Slot("b", Slot::WORD),
        ]
    };
    let linted = t.lint_template(pieces());
    assert_eq!(
        linted.iter().map(full).collect::<Vec<_>>(),
        [
            (MixedScript, Warning, None, 0, 0..5),
            (SlotGlue, Warning, None, 1, 0..0),
            (SlotGlue, Info, None, 3, 0..0),
        ]
    );
    let report = t.report_template(pieces(), Output::Unicode);
    assert_eq!(
        report.diagnostics().iter().map(full).collect::<Vec<_>>(),
        linted.iter().map(full).collect::<Vec<_>>()
    );
    // Typographic characters come in place by their start, `TypographicChar` last among equals.
    let text = "«a» — «b» …";
    let linted = prose(RU).lint(text);
    assert_ordered("chars", &linted);
    assert!(linted.len() >= 4, "{linted:?}");
}

#[test]
fn display_of_a_diagnostic() {
    // Section 12.4: `warning quotes.pairs 0:10..11: straight quote has no pair; left as is`.
    let t = interface(RU);
    let text = "Он сказал \"привет.";
    let expected = "warning quotes.pairs 0:18..19: straight quote has no pair; left as is";
    let linted = t.lint(text);
    assert_eq!(linted.len(), 1);
    assert_eq!(linted[0].to_string(), expected);
    let report = t.report(text, Output::Unicode);
    assert_eq!(report.diagnostics()[0].to_string(), expected);
    // A ruled diagnostic: severity, rule id, piece and range, then a message.
    let d = &t.lint("10 - 20 штук")[0];
    let line = d.to_string();
    assert!(
        line.starts_with("info dash.spaced-hyphen 0:3..4: "),
        "{line}"
    );
    assert!(
        line.len() > "info dash.spaced-hyphen 0:3..4: ".len(),
        "{line}"
    );
    // Without a rule: severity first, the place after, then a message; one line.
    let moskva = "\u{41c}o\u{441}\u{43a}\u{432}\u{430}";
    let line = t.lint(moskva)[0].to_string();
    assert!(line.starts_with("warning "), "{line}");
    assert!(line.contains(" 0:0..11: "), "{line}");
    assert!(!line.contains('\n'));
    // A piece other than the first.
    let d = &t.lint_template([
        Piece::Text("Файл "),
        Piece::Slot("a", Slot::WORD),
        Piece::Slot("b", Slot::WORD),
    ])[0];
    assert!(d.to_string().starts_with("info "), "{d}");
    assert!(d.to_string().contains(" 1:0..0: "), "{d}");
}

#[test]
fn lint_equals_report_without_typographic_chars_on_every_row() {
    let rows: &[&str] = &[
        "Он сказал \"привет.",
        "\"Один \"два три\"",
        "Слово \" в тексте.",
        "«a \"b» c\"",
        "Он сказал \"\"\" и ушёл.",
        "Цена 27»",
        "Он сказал \"a » b\"",
        "don`t",
        "Don`t.\n\nОн сказал \"да\".\n\nIt`s ok.",
        "10 - 20 штук",
        "Выполните cat - file",
        "npm test -- --watch",
        "\u{41c}o\u{441}\u{43a}\u{432}\u{430}",
        "Москва — столица",
        "Файл «report.pdf» не\u{a0}найден\u{a0}— проверьте",
        "© 2026 Example Corp",
        "a –  b … c",
        "x '\"»b 5' c\"",
        "",
    ];
    for language in [RU, EN, Language::COMMON] {
        for (what, t) in all_sets(&language) {
            for row in rows {
                let linted = t.lint(row);
                let report = t.report(row, Output::Unicode);
                assert_eq!(
                    sans_chars(&linted),
                    sans_chars(report.diagnostics()),
                    "{what}: {}",
                    show(row)
                );
                assert!(
                    report.diagnostics().iter().all(|d| !is_char(d.kind())),
                    "{what}: report has no TypographicChar: {}",
                    show(row)
                );
                assert_ordered(&what, &linted);
                // `lint(x)` has a `TypographicChar` exactly when `to_plain(x) != x`.
                assert_eq!(
                    !chars(&linted).is_empty(),
                    t.to_plain(row) != *row,
                    "{what}: {}",
                    show(row)
                );
                for (piece, range, c) in chars(&linted) {
                    assert_eq!(piece, 0);
                    assert_eq!(row[range].chars().collect::<Vec<_>>(), [c], "{what}");
                }
            }
        }
    }
}

#[test]
fn lint_of_nothing_is_empty() {
    for (what, t) in all_sets(&RU) {
        assert!(t.lint("").is_empty(), "{what}");
        assert!(t.lint_template::<()>([]).is_empty(), "{what}");
        assert!(
            t.lint_template::<()>([Piece::Text("")]).is_empty(),
            "{what}"
        );
        assert!(t.lint("Привет, мир.").is_empty(), "{what}");
    }
}

// ---------------------------------------------------------------------------------------------
// Templates: the same diagnostics in `report_template` and `lint_template`.
// ---------------------------------------------------------------------------------------------

#[test]
fn mixed_script_across_text_pieces() {
    // Section 13.2: `Text("Мо")`, `Text("sква")` → one diagnostic at piece 0, `0..4`, equal in
    // `report_template` and `lint_template`.
    for (what, t) in all_sets(&RU) {
        let pieces = || [Piece::Text("Мо"), Piece::Text("sква")];
        let linted = t.lint_template::<()>(pieces());
        assert_eq!(
            linted.iter().map(full).collect::<Vec<_>>(),
            [(MixedScript, Warning, None, 0, 0..4)],
            "{what}"
        );
        let report = t.report_template::<()>(pieces(), Output::Unicode);
        assert_eq!(
            report.diagnostics().iter().map(full).collect::<Vec<_>>(),
            linted.iter().map(full).collect::<Vec<_>>(),
            "{what}"
        );
    }
}

#[test]
fn a_diagnostic_over_pieces_is_reported_at_its_first_piece() {
    // An unclosed backtick in piece 1: the range is that of the backtick in its piece.
    let t = interface(RU);
    let linted = t.lint_template::<()>([Piece::Text("Run "), Piece::Text("ls `-la")]);
    assert_eq!(
        linted.iter().map(full).collect::<Vec<_>>(),
        [(UnclosedDelimiter, Warning, None, 1, 3..4)]
    );
    // A quote pair across pieces is a pair; a lone quote is reported in its piece.
    let linted = t.lint_template::<()>([Piece::Text("Он сказал "), Piece::Text("\"привет.")]);
    assert_eq!(
        linted.iter().map(full).collect::<Vec<_>>(),
        [(UnmatchedQuote, Warning, Some(Rule::QuotesPairs), 1, 0..1)]
    );
    let linted = t.lint_template([
        Piece::Text("Файл \""),
        Piece::Slot("n", Slot::WORD),
        Piece::Text("\" не найден"),
    ]);
    assert!(linted.is_empty(), "{linted:?}");
}

#[test]
fn lint_template_equals_report_template() {
    let rows: Vec<Vec<Piece<'static, &'static str>>> = vec![
        vec![Piece::Text("Он сказал \""), Piece::Slot("n", Slot::WORD)],
        vec![
            Piece::Text("Москва — "),
            Piece::Slot("c", Slot::WORD),
            Piece::Text(" «x»"),
        ],
        vec![Piece::Slot("n", Slot::NUMBER), Piece::Text("МБ")],
        vec![
            Piece::Slot("a", Slot::WORD),
            Piece::Text(" - "),
            Piece::Slot("b", Slot::WORD),
        ],
        vec![Piece::Slot("a", Slot::NUMBER), Piece::Text(" - x")],
        vec![
            Piece::Text("don`t "),
            Piece::Protected("--force"),
            Piece::Markup("<b>"),
        ],
        vec![Piece::Text("Ждём."), Piece::Text("..")],
    ];
    for language in [RU, EN, Language::COMMON] {
        for (what, t) in all_sets(&language) {
            for row in &rows {
                let linted = t.lint_template(row.iter().copied());
                let report = t.report_template(row.iter().copied(), Output::Unicode);
                assert_eq!(
                    sans_chars(&linted),
                    sans_chars(report.diagnostics()),
                    "{what}: {row:?}"
                );
                assert_ordered(&what, &linted);
                for (piece, range, c) in chars(&linted) {
                    match row[piece] {
                        Piece::Text(text) => {
                            assert_eq!(text[range].chars().collect::<Vec<_>>(), [c]);
                        }
                        other => panic!("{what}: TypographicChar in {other:?}"),
                    }
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------------------------
// The corpus (section 13.7): plain sources have nothing to repair; typeset ones have.
// ---------------------------------------------------------------------------------------------

#[test]
fn the_plain_corpus_has_no_typographic_char_and_the_typeset_one_has() {
    for lang in LANGS {
        for (set_name, _) in SETS {
            let t = typographer(lang, set_name);
            for file in FILES {
                // `tech.txt` is plain for Interface only (section 12.2).
                if file == "tech.txt" && set_name == "prose" {
                    continue;
                }
                let lines: Vec<String> = if file.ends_with(".tmpl") {
                    messages(lang).iter().map(|m| m.flat()).collect()
                } else {
                    text_lines(lang, file)
                };
                for line in &lines {
                    let linted = t.lint(line);
                    assert!(
                        chars(&linted).is_empty(),
                        "{lang} {set_name} {file}: a plain line has an entry: {}",
                        show(line)
                    );
                    assert_eq!(t.to_plain(line), *line, "{lang} {set_name} {file}");
                    // The typeset form: an entry exactly when `to_plain` changes it, and its
                    // diagnostics are those of `report`.
                    let typeset = t.typeset(line, Output::Unicode).into_owned();
                    let linted = t.lint(&typeset);
                    assert_eq!(
                        !chars(&linted).is_empty(),
                        t.to_plain(&typeset) != typeset,
                        "{lang} {set_name} {file}: {}",
                        show(&typeset)
                    );
                    assert_eq!(
                        sans_chars(&linted),
                        sans_chars(t.report(&typeset, Output::Unicode).diagnostics()),
                        "{lang} {set_name} {file}: {}",
                        show(&typeset)
                    );
                    if typeset != *line {
                        assert!(
                            !chars(&linted).is_empty(),
                            "{lang} {set_name} {file}: a typeset line that differs from its plain \
                             form is a source to repair: {}",
                            show(&typeset)
                        );
                    }
                }
            }
        }
    }
}

#[test]
fn the_typeset_corpus_templates_are_checked_piece_by_piece() {
    use common::corpus::{split_template, template_pieces};
    for lang in LANGS {
        for (set_name, _) in SETS {
            let t = typographer(lang, set_name);
            for message in messages(lang) {
                let plain = message.pieces();
                let linted = t.lint_template(plain.iter().copied());
                assert!(
                    chars(&linted).is_empty(),
                    "{lang} {set_name} {}: a plain message has an entry",
                    message.id
                );
                // The typeset template, handed back as pieces.
                let typeset = t.typeset_template(plain.iter().copied(), Output::Unicode);
                let (texts, _) = split_template(&typeset);
                let pieces = template_pieces(&typeset);
                let linted = t.lint_template(pieces.iter().copied());
                let report = t.report_template(pieces.iter().copied(), Output::Unicode);
                assert_eq!(
                    sans_chars(&linted),
                    sans_chars(report.diagnostics()),
                    "{lang} {set_name} {}",
                    message.id
                );
                let back = t.to_plain_template(pieces.iter().copied());
                let changed = back
                    .texts()
                    .iter()
                    .zip(&texts)
                    .any(|(after, before)| after.as_ref() != before);
                assert_eq!(
                    !chars(&linted).is_empty(),
                    changed,
                    "{lang} {set_name} {}: {texts:?}",
                    message.id
                );
            }
        }
    }
}
