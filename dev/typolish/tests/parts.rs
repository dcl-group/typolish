//! Templates: pieces, slots and the output contract of design sections 2.6 and 2.14, as far as the
//! stage 2 rules (`quotes.*`, `dash.*`, `range.*`, `ellipsis.*`) reach them (section 13.2).
//!
//! The rows run in Prose and Interface without the `nbsp.*` rules (stage 2 of section 15.4). The
//! rows that need later parts join with them: the `nbsp.*` rules (the slot participation rows of
//! `nbsp.units`, `nbsp.dates`, glue before and after a slot, abbreviations after a slot, labels
//! and percent next to slots, `{a}-5 кг`, `{d} г. Затем`), `number.times` (`{w}x{h}`),
//! `format_number` and the escaped and HTML outputs (the flow of section 2.14 for
//! `EscapeStyle::RUST` and `EscapeStyle::JSON`, and with `HtmlStyle::NAMED.nowrap_class`), and
//! nowrap groups at slots.

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

use common::{TmplPiece, interface, parse_tmpl, prose};
use typolish::{
    DiagnosticKind, HtmlStyle, Language, Output, Piece, Rule, Scratch, Severity, Slot, SlotKind,
    Template, Typographer,
};

fn sets(language: &Language) -> Vec<(String, Typographer)> {
    vec![
        (format!("{language} Prose"), prose(language.clone())),
        (format!("{language} Interface"), interface(language.clone())),
    ]
}

fn texts<S>(t: &Template<'_, S>) -> Vec<String> {
    t.texts().iter().map(|s| s.to_string()).collect()
}

const KINDS: [(Slot, &str); 4] = [
    (Slot::NUMBER, "Number"),
    (Slot::WORD, "Word"),
    (Slot::DATE, "Date"),
    (Slot::OTHER, "Other"),
];

// The output contract of section 2.6.

#[test]
fn an_empty_list_of_pieces() {
    for (what, t) in sets(&Language::RU_RU) {
        let out = t.typeset_template::<()>([], Output::Unicode);
        assert_eq!(out.texts(), [""], "{what}");
        assert!(matches!(out.texts()[0], Cow::Borrowed(_)), "{what}");
        assert!(out.slots().is_empty(), "{what}");
        assert!(!out.changed(), "{what}");
        let r = t.report_template::<()>([], Output::Unicode);
        assert!(r.changes().is_empty() && r.protected().is_empty() && r.diagnostics().is_empty());
    }
}

#[test]
fn slots_come_out_in_order_with_their_payloads() {
    for (what, t) in sets(&Language::RU_RU) {
        let out = t.typeset_template(
            [
                Piece::Slot(10u32, Slot::NUMBER),
                Piece::Slot(20, Slot::WORD),
                Piece::Text(" и "),
                Piece::Slot(30, Slot::DATE),
                Piece::Slot(40, Slot::OTHER),
            ],
            Output::Unicode,
        );
        let payloads: Vec<_> = out.slots().iter().map(|(p, s)| (*p, s.kind())).collect();
        assert_eq!(
            payloads,
            [
                (10, SlotKind::Number),
                (20, SlotKind::Word),
                (30, SlotKind::Date),
                (40, SlotKind::Other)
            ],
            "{what}"
        );
        assert_eq!(out.texts().len(), 5, "{what}");
        assert_eq!(texts(&out), ["", "", " и ", "", ""], "{what}");
    }
}

#[test]
fn adjacent_pieces_merge_into_one_text() {
    for (what, t) in sets(&Language::EN_US) {
        let out = t.typeset_template::<()>(
            [
                Piece::Text("Run "),
                Piece::Protected("git checkout -- file"),
                Piece::Text(" now."),
            ],
            Output::Unicode,
        );
        assert_eq!(out.texts(), ["Run git checkout -- file now."], "{what}");
        // Merging is not a change; the merged text is not borrowed.
        assert!(!out.changed(), "{what}");
        assert!(matches!(out.texts()[0], Cow::Owned(_)), "{what}");
        // A text that maps to one unchanged piece borrows it.
        let input = "Run now.";
        let out = t.typeset_template::<()>([Piece::Text(input)], Output::Unicode);
        assert!(
            matches!(&out.texts()[0], Cow::Borrowed(s) if std::ptr::eq(*s, input)),
            "{what}"
        );
        let out = t.typeset_template::<()>(
            [Piece::Text(""), Piece::Text("Run now."), Piece::Markup("")],
            Output::Unicode,
        );
        assert_eq!(out.texts(), ["Run now."], "{what}");
        assert!(!out.changed(), "{what}");
    }
}

#[test]
fn a_changed_text_is_marked_changed() {
    for (what, t) in sets(&Language::RU_RU) {
        let out = t.typeset_template(
            [
                Piece::Text("Он сказал \"да\" "),
                Piece::Slot("a", Slot::WORD),
            ],
            Output::Unicode,
        );
        assert_eq!(texts(&out), ["Он сказал «да» ", ""], "{what}");
        assert!(out.changed(), "{what}");
    }
}

#[test]
fn typeset_template_with_equals_typeset_template() {
    let mut scratch = Scratch::new();
    for (what, t) in sets(&Language::RU_RU) {
        let pieces = || {
            [
                Piece::Text("Файл \""),
                Piece::Slot("name", Slot::WORD),
                Piece::Text("\" не найден..."),
            ]
        };
        let a = t.typeset_template(pieces(), Output::Unicode);
        let b = t.typeset_template_with(&mut scratch, pieces(), Output::Unicode);
        assert_eq!(a, b, "{what}");
        assert_eq!(texts(&a), ["Файл «", "» не найден…"], "{what}");
    }
}

// Section 6.9 and 6.12: a quote opened in one text closes in another.

#[test]
fn a_quote_pairs_across_a_slot() {
    for (what, t) in sets(&Language::RU_RU) {
        let out = t.typeset_template(
            [
                Piece::Text("Файл \""),
                Piece::Slot("name", Slot::WORD),
                Piece::Text("\" не найден"),
            ],
            Output::Unicode,
        );
        assert_eq!(texts(&out), ["Файл «", "» не найден"], "{what}");
        assert!(out.changed(), "{what}");
    }
    for (what, t) in sets(&Language::EN_US) {
        let out = t.typeset_template(
            [
                Piece::Text("Open \""),
                Piece::Slot("name", Slot::WORD),
                Piece::Text("\" and \""),
                Piece::Slot("other", Slot::OTHER),
                Piece::Text("\"."),
            ],
            Output::Unicode,
        );
        assert_eq!(texts(&out), ["Open “", "” and “", "”."], "{what}");
    }
}

#[test]
fn one_window_across_a_slot_and_line_breaks() {
    // Section 3.3: `Text("a\n")`, `Slot`, `Text("\nb")` is one window.
    for (what, t) in sets(&Language::RU_RU) {
        let out = t.typeset_template(
            [
                Piece::Text("Скажи \"раз\n"),
                Piece::Slot("s", Slot::OTHER),
                Piece::Text("\nдва\""),
            ],
            Output::Unicode,
        );
        assert_eq!(texts(&out), ["Скажи «раз\n", "\nдва»"], "{what}");
        // A blank line inside a text is a boundary: the first quote has no partner.
        let out = t.typeset_template(
            [
                Piece::Text("\"a\n\n"),
                Piece::Slot("s", Slot::OTHER),
                Piece::Text("\n\n\"b\""),
            ],
            Output::Unicode,
        );
        assert_eq!(texts(&out), ["\"a\n\n", "\n\n«b»"], "{what}");
    }
}

#[test]
fn markup_is_transparent_and_a_protected_piece_is_a_word() {
    for (what, t) in sets(&Language::RU_RU) {
        let out = t.typeset_template::<()>(
            [
                Piece::Text("Он сказал \""),
                Piece::Markup("<b>"),
                Piece::Text("да"),
                Piece::Markup("</b>"),
                Piece::Text("\""),
            ],
            Output::Unicode,
        );
        assert_eq!(texts(&out), ["Он сказал «<b>да</b>»"], "{what}");
        let out = t.typeset_template::<()>(
            [
                Piece::Text("\""),
                Piece::Protected("--force"),
                Piece::Text("\" и \""),
                Piece::Protected("`ls`"),
                Piece::Text("\""),
            ],
            Output::Unicode,
        );
        assert_eq!(texts(&out), ["«--force» и «`ls`»"], "{what}");
    }
}

// Tokens across pieces (section 3.4).

#[test]
fn tokens_span_adjacent_text_pieces() {
    for (what, t) in sets(&Language::RU_RU) {
        let r = t.report_template::<()>([Piece::Text("Ждём."), Piece::Text("..")], Output::Unicode);
        assert_eq!(texts(r.result()), ["Ждём…"], "{what}");
        // Two records: the first holds the replacement, the second deletes the rest.
        let records: Vec<_> = r
            .changes()
            .iter()
            .map(|c| (c.rule(), c.piece(), c.range(), c.replacement().to_owned()))
            .collect();
        assert_eq!(
            records,
            [
                (Rule::EllipsisDots, 0, 8..9, "…".to_owned()),
                (Rule::EllipsisDots, 1, 0..2, String::new()),
            ],
            "{what}"
        );
        assert_eq!(r.original(&r.changes()[0]), ".");
        assert_eq!(r.original(&r.changes()[1]), "..");
        // A markup piece ends a token: `.` and `..` are two tokens and no ellipsis.
        let out = t.typeset_template::<()>(
            [
                Piece::Text("Ждём."),
                Piece::Markup("<b>"),
                Piece::Text(".."),
            ],
            Output::Unicode,
        );
        assert_eq!(texts(&out), ["Ждём.<b>.."], "{what}");
        assert!(!out.changed(), "{what}");
        // So does a slot.
        let out = t.typeset_template(
            [
                Piece::Text("Ждём."),
                Piece::Slot("s", Slot::OTHER),
                Piece::Text(".."),
            ],
            Output::Unicode,
        );
        assert_eq!(texts(&out), ["Ждём.", ".."], "{what}");
        assert!(!out.changed(), "{what}");
    }
}

#[test]
fn backticks_protect_across_pieces() {
    // ``Run `git checkout -- { $b }` to reset.``: the command, with its slot, is protected.
    for (what, t) in sets(&Language::EN_US) {
        let out = t.typeset_template(
            [
                Piece::Text("Run `git checkout -- "),
                Piece::Slot("b", Slot::WORD),
                Piece::Text("` to reset."),
            ],
            Output::Unicode,
        );
        assert_eq!(
            texts(&out),
            ["Run `git checkout -- ", "` to reset."],
            "{what}"
        );
        assert!(!out.changed(), "{what}");
    }
    // The same words without protection are typeset in Prose (section 10.8).
    let t = prose(Language::EN_US);
    let out = t.typeset_template(
        [
            Piece::Text("Run git checkout -- "),
            Piece::Slot("b", Slot::WORD),
            Piece::Text(" to reset."),
        ],
        Output::Unicode,
    );
    assert!(out.changed());
}

// The slot participation table of section 2.6, row by row, as far as the stage 2 rules go.

#[test]
fn row_quotes_around_a_slot() {
    // `"{x}"`: a slot of every kind is a word for the roles of quotes (sections 6.9 and 2.6).
    for language in [Language::RU_RU, Language::EN_US] {
        let expected = if language == Language::RU_RU {
            ["«", "»"]
        } else {
            ["“", "”"]
        };
        for (what, t) in sets(&language) {
            for (slot, name) in KINDS {
                let out = t.typeset_template(
                    [Piece::Text("\""), Piece::Slot("x", slot), Piece::Text("\"")],
                    Output::Unicode,
                );
                assert_eq!(texts(&out), expected, "{what} {name}");
            }
        }
    }
}

#[test]
fn row_apostrophe_after_a_slot() {
    // `{x}'s`: yes after `Number` and `Word`, no after `Date` and `Other` (section 6.2).
    for language in [Language::EN_US, Language::RU_RU] {
        for (what, t) in sets(&language) {
            for (slot, name) in KINDS {
                let out = t
                    .typeset_template([Piece::Slot("x", slot), Piece::Text("'s")], Output::Unicode);
                let expected = match slot.kind() {
                    SlotKind::Number | SlotKind::Word => "’s",
                    _ => "'s",
                };
                assert_eq!(texts(&out), ["", expected], "{what} {name}");
            }
        }
    }
}

#[test]
fn row_prime_after_a_slot() {
    // `{n}"` in Prose: a prime after a `Number` slot if the quote is unpaired; no prime after
    // the other kinds (a closing quote without an opener stays, with `UnmatchedQuote`).
    let t = prose(Language::RU_RU);
    for (slot, name) in KINDS {
        let out = t.typeset_template(
            [
                Piece::Text("Экран "),
                Piece::Slot("x", slot),
                Piece::Text("\" и всё"),
            ],
            Output::Unicode,
        );
        let expected = if slot.kind() == SlotKind::Number {
            "″ и всё"
        } else {
            "\" и всё"
        };
        assert_eq!(texts(&out), ["Экран ", expected], "{name}");
    }
    // `{n}'` too, and not in Interface, where `quotes.primes` is not.
    let out = t.typeset_template(
        [Piece::Slot("n", Slot::NUMBER), Piece::Text("' рост")],
        Output::Unicode,
    );
    assert_eq!(texts(&out), ["", "′ рост"]);
    let ui = interface(Language::RU_RU);
    let out = ui.typeset_template(
        [
            Piece::Text("Экран "),
            Piece::Slot("n", Slot::NUMBER),
            Piece::Text("\" и всё"),
        ],
        Output::Unicode,
    );
    assert_eq!(texts(&out), ["Экран ", "\" и всё"]);
    assert!(!out.changed());
    // A paired quote after a `Number` slot is not a prime: `"{n}"` → `«{n}»` (section 6.9).
    for (what, t) in sets(&Language::RU_RU) {
        let out = t.typeset_template(
            [
                Piece::Text("\""),
                Piece::Slot("n", Slot::NUMBER),
                Piece::Text("\""),
            ],
            Output::Unicode,
        );
        assert_eq!(texts(&out), ["«", "»"], "{what}");
        let out = t.typeset_template(
            [
                Piece::Text("Он сказал \"диагональ "),
                Piece::Slot("n", Slot::NUMBER),
                Piece::Text("\"."),
            ],
            Output::Unicode,
        );
        assert_eq!(texts(&out), ["Он сказал «диагональ ", "»."], "{what}");
    }
}

#[test]
fn row_range_placeholders() {
    // `{a}-{b}`: `Number` with another `Number`, `Date` with another `Date` (both sets).
    for (what, t) in sets(&Language::RU_RU) {
        let join = |a: Slot, b: Slot| {
            let out = t.typeset_template(
                [Piece::Slot("a", a), Piece::Text("-"), Piece::Slot("b", b)],
                Output::Unicode,
            );
            texts(&out)
        };
        assert_eq!(join(Slot::NUMBER, Slot::NUMBER), ["", "–", ""], "{what}");
        assert_eq!(join(Slot::DATE, Slot::DATE), ["", "–", ""], "{what}");
        // `{Number}-{Date}` unchanged.
        assert_eq!(join(Slot::NUMBER, Slot::DATE), ["", "-", ""], "{what}");
        assert_eq!(join(Slot::DATE, Slot::NUMBER), ["", "-", ""], "{what}");
        assert_eq!(join(Slot::WORD, Slot::WORD), ["", "-", ""], "{what}");
        assert_eq!(join(Slot::OTHER, Slot::OTHER), ["", "-", ""], "{what}");
    }
}

#[test]
fn row_operand_of_the_spaced_hyphen() {
    // `{x} - слово`: an operand of every kind. Next to a number, a range or a single-letter word
    // a `Number` slot gets no dash.
    for (what, t) in sets(&Language::RU_RU) {
        for (slot, name) in KINDS {
            let out = t.typeset_template(
                [Piece::Slot("x", slot), Piece::Text(" - слово")],
                Output::Unicode,
            );
            assert_eq!(texts(&out), ["", " — слово"], "{what} {name}");
            let out = t.typeset_template(
                [Piece::Text("слово - "), Piece::Slot("x", slot)],
                Output::Unicode,
            );
            assert_eq!(texts(&out), ["слово — ", ""], "{what} {name}");
        }
        for tail in [" - 5", " - x", " - 5–10"] {
            let out = t.typeset_template(
                [Piece::Slot("n", Slot::NUMBER), Piece::Text(tail)],
                Output::Unicode,
            );
            assert_eq!(texts(&out), ["", tail], "{what} {tail}");
            assert!(!out.changed(), "{what} {tail}");
        }
        let out = t.typeset_template(
            [
                Piece::Slot("a", Slot::NUMBER),
                Piece::Text(" - "),
                Piece::Slot("b", Slot::NUMBER),
            ],
            Output::Unicode,
        );
        assert!(!out.changed(), "{what}");
        let out = t.typeset_template(
            [Piece::Slot("a", Slot::WORD), Piece::Text(" - x")],
            Output::Unicode,
        );
        assert_eq!(texts(&out), ["", " — x"], "{what}");
    }
}

#[test]
fn row_slot_glue_has_no_effect_on_the_output() {
    // The `SlotGlue` diagnostic itself belongs to stage 4; here only that a slot glued to letters
    // or to another slot is left as it is.
    for (what, t) in sets(&Language::RU_RU) {
        let out = t.typeset_template(
            [Piece::Slot("n", Slot::NUMBER), Piece::Text("МБ")],
            Output::Unicode,
        );
        assert_eq!(texts(&out), ["", "МБ"], "{what}");
        let out = t.typeset_template(
            [Piece::Slot("a", Slot::WORD), Piece::Slot("b", Slot::WORD)],
            Output::Unicode,
        );
        assert_eq!(texts(&out), ["", "", ""], "{what}");
    }
}

// The Fluent mapping of section 2.14, each row as a piece list.

#[test]
fn fluent_mapping_table() {
    let ru = interface(Language::RU_RU);
    // TextElement → Text.
    let out = ru.typeset_template::<()>([Piece::Text("Файл \"отчёт\"...")], Output::Unicode);
    assert_eq!(texts(&out), ["Файл «отчёт»…"]);
    // `{ "…" }` StringLiteral → Text with the unescaped value: a glue that is already in place
    // stays, and the literal takes part in the rules like any text.
    let out = ru.typeset_template::<()>(
        [
            Piece::Text("Нажмите "),
            Piece::Text("\u{a0}"),
            Piece::Text("\"ОК\""),
        ],
        Output::Unicode,
    );
    assert_eq!(texts(&out), ["Нажмите \u{a0}«ОК»"]);
    let out = ru.typeset_template::<()>([Piece::Text("5\u{a0}МБ")], Output::Unicode);
    assert!(matches!(out.texts()[0], Cow::Borrowed("5\u{a0}МБ")));
    // `{ 42 }` NumberLiteral → Text("42"): it takes part in rules, a range for one.
    let p = prose(Language::RU_RU);
    let out = p.typeset_template::<()>(
        [
            Piece::Text("Стр. "),
            Piece::Text("9"),
            Piece::Text("-"),
            Piece::Text("12"),
        ],
        Output::Unicode,
    );
    assert_eq!(texts(&out), ["Стр. 9–12"]);
    // `{ NUMBER($n) }`, `{ DATETIME($d) }`, `{ -brand }`, `{ $var }`: slots of the kinds.
    let out = ru.typeset_template(
        [
            Piece::Text("Файл \""),
            Piece::Slot("name", Slot::WORD),
            Piece::Text("\" весит "),
            Piece::Slot("size", Slot::NUMBER),
            Piece::Text(" МБ - удалить?"),
            Piece::Slot("when", Slot::DATE),
            Piece::Slot("var", Slot::OTHER),
        ],
        Output::Unicode,
    );
    assert_eq!(
        texts(&out)[..4].to_vec(),
        ["Файл «", "» весит ", " МБ — удалить?", ""]
    );
    let kinds: Vec<_> = out.slots().iter().map(|(n, s)| (*n, s.kind())).collect();
    assert_eq!(
        kinds,
        [
            ("name", SlotKind::Word),
            ("size", SlotKind::Number),
            ("when", SlotKind::Date),
            ("var", SlotKind::Other)
        ]
    );
}

#[test]
fn select_variants_are_typeset_as_templates_of_their_own() {
    // `{ $n -> [one] … *[other] … }`: one `Word` slot in the outer template, every variant its
    // own template. A quote opened outside and closed inside a variant produces `UnmatchedQuote`
    // in both places (section 2.14), and nothing is paired.
    let t = interface(Language::RU_RU);
    let outer = t.report_template(
        [Piece::Text("Он сказал \""), Piece::Slot("n", Slot::WORD)],
        Output::Unicode,
    );
    let variant_one = t.report_template::<()>([Piece::Text("привет\" и ушёл")], Output::Unicode);
    let variant_other = t.report_template::<()>([Piece::Text("привет\" и ушли")], Output::Unicode);
    for r in [
        &outer.diagnostics(),
        &variant_one.diagnostics(),
        &variant_other.diagnostics(),
    ] {
        let kinds: Vec<_> = r.iter().map(|d| (d.kind().clone(), d.severity())).collect();
        assert_eq!(kinds, [(DiagnosticKind::UnmatchedQuote, Severity::Warning)]);
    }
    assert!(!outer.result().changed());
    assert!(!variant_one.result().changed());
    // Each variant that is complete in itself is typeset.
    let out = t.typeset_template::<()>([Piece::Text("«да» \"нет\" и ушёл")], Output::Unicode);
    assert_eq!(texts(&out), ["«да» «нет» и ушёл"]);
}

#[test]
fn other_slots_are_words_for_quotes_and_dashes() {
    for (what, t) in sets(&Language::EN_US) {
        let out = t.typeset_template(
            [
                Piece::Text("Path \""),
                Piece::Slot("p", Slot::OTHER),
                Piece::Text("\" is bad"),
            ],
            Output::Unicode,
        );
        assert_eq!(texts(&out), ["Path “", "” is bad"], "{what}");
        let out = t.typeset_template(
            [Piece::Slot("p", Slot::OTHER), Piece::Text(" - is bad")],
            Output::Unicode,
        );
        assert_eq!(texts(&out), ["", " — is bad"], "{what}");
    }
}

#[test]
fn word_and_other_slots_and_protected_pieces_are_words_for_the_dialogue_dash() {
    // Sections 8.4, 8.10 and 2.6: `Word` and `Other` slots count as words, a protected piece acts
    // as one word; no glue sticks to an `Other` slot (section 8.2), so there the space stays.
    let t = prose(Language::RU_RU);
    let out = t.typeset_template(
        [
            Piece::Text("- "),
            Piece::Slot("w", Slot::WORD),
            Piece::Text(" пришёл"),
        ],
        Output::Unicode,
    );
    assert_eq!(texts(&out), ["—\u{a0}", " пришёл"]);
    let out = t.typeset_template(
        [
            Piece::Text("- "),
            Piece::Slot("o", Slot::OTHER),
            Piece::Text(" пришёл"),
        ],
        Output::Unicode,
    );
    assert_eq!(texts(&out), ["— ", " пришёл"]);
    let out = t.typeset_template::<()>(
        [
            Piece::Text("- "),
            Piece::Protected("ls"),
            Piece::Text(" пришёл"),
        ],
        Output::Unicode,
    );
    assert_eq!(texts(&out), ["—\u{a0}ls пришёл"]);
    // A `Number` slot is no word here: the line stays.
    let out = t.typeset_template(
        [
            Piece::Text("- "),
            Piece::Slot("n", Slot::NUMBER),
            Piece::Text(" штук"),
        ],
        Output::Unicode,
    );
    assert_eq!(texts(&out), ["- ", " штук"]);
}

#[test]
fn word_and_other_slots_and_protected_pieces_are_words_for_the_inner_ellipsis() {
    // Sections 8.6, 8.10 and 2.6 (Prose only: `ellipsis.inner` is not in Interface).
    let t = prose(Language::EN_US);
    for slot in [Slot::WORD, Slot::OTHER] {
        let out = t.typeset_template(
            [
                Piece::Slot("a", slot),
                Piece::Text("..."),
                Piece::Slot("b", slot),
            ],
            Output::Unicode,
        );
        assert_eq!(texts(&out), ["", "…", ""], "{slot:?}");
        let out = t.typeset_template(
            [Piece::Text("a ..."), Piece::Slot("b", slot)],
            Output::Unicode,
        );
        assert_eq!(texts(&out), ["a …", ""], "{slot:?}");
    }
    let out = t.typeset_template::<()>(
        [Piece::Text("a ..."), Piece::Protected("ls")],
        Output::Unicode,
    );
    assert_eq!(texts(&out), ["a …ls"]);
    let t = interface(Language::EN_US);
    let out = t.typeset_template(
        [Piece::Text("a ..."), Piece::Slot("b", Slot::WORD)],
        Output::Unicode,
    );
    assert_eq!(texts(&out), ["a ...", ""]);
}

#[test]
fn fill_inserts_values_unchanged() {
    // Section 11.1: values are inserted as they are, also with `Output::Html`: the caller escapes
    // them.
    for output in [Output::Unicode, Output::Html(HtmlStyle::NAMED)] {
        let t = interface(Language::RU_RU);
        let out = t.typeset_template(
            [
                Piece::Text("Файл \""),
                Piece::Slot("name", Slot::WORD),
                Piece::Text("\" найден"),
            ],
            output,
        );
        let filled = out.fill(|_| Cow::Borrowed("<b>"));
        assert_eq!(filled, format!("{}<b>{}", out.texts()[0], out.texts()[1]));
        assert!(filled.contains("<b>"));
    }
    let t = interface(Language::RU_RU);
    let out = t.typeset_template(
        [
            Piece::Slot("a", Slot::WORD),
            Piece::Text("-"),
            Piece::Slot("b", Slot::WORD),
        ],
        Output::Unicode,
    );
    assert_eq!(out.fill(|n| Cow::Owned(n.to_string())), "a-b");
    let empty = t.typeset_template::<()>([], Output::Unicode);
    assert_eq!(empty.fill(|_| Cow::Borrowed("x")), "");
}

#[test]
fn an_empty_isolate_pair_is_a_value_that_is_quoted() {
    // Section 5.4 and `tests/fluent_isolation.rs`: isolates around a value are transparent.
    for (what, t) in sets(&Language::RU_RU) {
        let out =
            t.typeset_template::<()>([Piece::Text("Имя: \"\u{2068}\u{2069}\"")], Output::Unicode);
        assert_eq!(texts(&out), ["Имя: «\u{2068}\u{2069}»"], "{what}");
    }
}

#[test]
fn the_tmpl_helper_reads_what_the_tests_feed_it() {
    // The test-only syntax of section 13.7: a template with every kind of slot runs through both
    // sets without touching its slots.
    let source = "ui.size = Файл \"{name:word}\" весит {n:number} МБ - {when:date} {x}...\n";
    let messages = parse_tmpl(source);
    assert_eq!(messages.len(), 1);
    let m = &messages[0];
    assert_eq!(m.id, "ui.size");
    assert!(matches!(m.pieces[0], TmplPiece::Text(_)));
    for (what, t) in sets(&Language::RU_RU) {
        let out = t.typeset_template(m.pieces(), Output::Unicode);
        let names: Vec<_> = out.slots().iter().map(|(n, s)| (*n, s.kind())).collect();
        assert_eq!(
            names,
            [
                ("name", SlotKind::Word),
                ("n", SlotKind::Number),
                ("when", SlotKind::Date),
                ("x", SlotKind::Other)
            ],
            "{what}"
        );
        assert_eq!(out.texts().len(), 5, "{what}");
        assert_eq!(out.texts()[0], "Файл «", "{what}");
        assert_eq!(out.texts()[1], "» весит ", "{what}");
    }
}
