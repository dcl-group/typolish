//! Templates: pieces, slots and the output contract of design sections 2.6 and 2.14 (section 13.2).
//!
//! The rows run in the full Prose and Interface (stage 3 of section 15.4): the expectations carry
//! the glue of the `nbsp.*` rules, which also makes the slot participation rows of `nbsp.units`,
//! `nbsp.dates`, glue before and after a slot, abbreviations after a slot, labels and percent next
//! to slots, `{a}-5 кг` and `{d} г. Затем` testable. The row of `number.times` (`{w}x{h}`) and the
//! flow of section 2.14 for `EscapeStyle::RUST`, `EscapeStyle::JSON` and
//! `HtmlStyle::NAMED.nowrap_class` (the last tests of the file) use `format_number` and the
//! encoders; the nowrap groups at slots are in `tests/output.rs`.

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

use common::decode::decode_escaped;
use common::{TmplPiece, interface, parse_tmpl, prose, vis};
use typolish::{
    DiagnosticKind, EscapeStyle, HtmlStyle, Language, Output, Piece, Rule, Scratch, Severity, Slot,
    SlotKind, Template, Typographer,
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
        // `и` is a short word and glue sticks before a `Date` slot, but the fragment after the
        // space is `{d}{o}`, which holds an `Other` slot: no glue sticks to it (sections 2.6 and
        // 8.2), so the space stays.
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
        // Prose glues the two-letter `Он` (`nbsp.short-length`, section 8.7).
        let first = if t.rules().contains(Rule::NbspShortLength) {
            "Он\u{a0}сказал «да» "
        } else {
            "Он сказал «да» "
        };
        assert_eq!(texts(&out), [first, ""], "{what}");
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
        assert_eq!(texts(&a), ["Файл «", "» не\u{a0}найден…"], "{what}");
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
        assert_eq!(texts(&out), ["Файл «", "» не\u{a0}найден"], "{what}");
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
        // `and` is a listed short word before an opening quote, but the fragment after the space
        // is `“{other}”.`, whose core is the `Other` slot: no glue sticks to it (sections 2.6 and
        // 8.2), so the space after `and` stays an ordinary space.
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
        // Prose glues the two-letter `Он` to the word after it (`nbsp.short-length`).
        let expected = if what.contains("Prose") {
            "Он\u{a0}сказал «<b>да</b>»"
        } else {
            "Он сказал «<b>да</b>»"
        };
        assert_eq!(texts(&out), [expected], "{what}");
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
        assert_eq!(texts(&out), ["«--force» и\u{a0}«`ls`»"], "{what}");
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
        // The command and its slot are protected; `to` is a short word, so the text after the
        // slot gets its glue and is the only edit.
        assert_eq!(
            texts(&out),
            ["Run `git checkout -- ", "` to\u{a0}reset."],
            "{what}"
        );
        assert!(out.changed(), "{what}");
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
            "″ и\u{a0}всё"
        } else {
            "\" и\u{a0}всё"
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
    // Interface has no primes; the quote stays, and `и` before `всё` gets its glue.
    assert_eq!(texts(&out), ["Экран ", "\" и\u{a0}всё"]);
    assert!(out.changed());
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
        let expected = if what.contains("Prose") {
            ["Он\u{a0}сказал «диагональ ", "»."]
        } else {
            ["Он сказал «диагональ ", "»."]
        };
        assert_eq!(texts(&out), expected, "{what}");
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
            // `nbsp.dash` glues the space before the dash, but not next to an `Other` slot.
            let expected = if slot.kind() == SlotKind::Other {
                " — слово"
            } else {
                "\u{a0}— слово"
            };
            assert_eq!(texts(&out), ["", expected], "{what} {name}");
            let out = t.typeset_template(
                [Piece::Text("слово - "), Piece::Slot("x", slot)],
                Output::Unicode,
            );
            assert_eq!(texts(&out), ["слово\u{a0}— ", ""], "{what} {name}");
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
        assert_eq!(texts(&out), ["", "\u{a0}— x"], "{what}");
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
    // `Стр.` is a `Next` abbreviation (a capital start matches), glued to the range.
    assert_eq!(texts(&out), ["Стр.\u{a0}9–12"]);
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
    // The example of section 2.6: the glue before the unit and before the dash lands in the
    // text after the slot.
    assert_eq!(
        texts(&out)[..4].to_vec(),
        ["Файл «", "» весит ", "\u{a0}МБ\u{a0}— удалить?", ""]
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
    // Nothing is paired in the variants; `и` before a word gets its glue, which is all.
    assert_eq!(texts(variant_one.result()), ["привет\" и\u{a0}ушёл"]);
    assert_eq!(texts(variant_other.result()), ["привет\" и\u{a0}ушли"]);
    // Each variant that is complete in itself is typeset.
    let out = t.typeset_template::<()>([Piece::Text("«да» \"нет\" и ушёл")], Output::Unicode);
    assert_eq!(texts(&out), ["«да» «нет» и\u{a0}ушёл"]);
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
    // The same for a quoted `Other` slot: the quote opens the fragment whose core is the slot.
    let out = t.typeset_template(
        [
            Piece::Text("- \""),
            Piece::Slot("o", Slot::OTHER),
            Piece::Text("\" пришёл"),
        ],
        Output::Unicode,
    );
    assert_eq!(texts(&out), ["— «", "» пришёл"]);
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

// ---------------------------------------------------------------------------------------------
// The slot participation table of section 2.6: the rows of the `nbsp.*` rules and the glue
// (stage 3). One test per row; each slot kind is a column.
// ---------------------------------------------------------------------------------------------

fn alone(language: Language, rule: Rule) -> Typographer {
    Typographer::new(language, typolish::RuleSet::EMPTY.with(rule))
}

/// The texts of `pieces` typeset with `t`.
fn run<'t>(t: &Typographer, pieces: Vec<Piece<'t, &'static str>>) -> Vec<String> {
    texts(&t.typeset_template(pieces, Output::Unicode))
}

#[test]
fn row_units_on_the_left_of_a_unit() {
    // `{n} МБ`: yes for `Number`; no for the other kinds. Alone and in both sets.
    let mut configs = vec![("alone".to_owned(), alone(Language::RU_RU, Rule::NbspUnits))];
    configs.extend(sets(&Language::RU_RU));
    for (what, t) in &configs {
        for (slot, name) in KINDS {
            let out = run(t, vec![Piece::Slot("x", slot), Piece::Text(" МБ")]);
            let expected = if slot.kind() == SlotKind::Number {
                "\u{a0}МБ"
            } else {
                " МБ"
            };
            assert_eq!(out, ["", expected], "{what} {name}");
        }
        // `{n} дн.`, the example of the task.
        let out = run(t, vec![Piece::Slot("n", Slot::NUMBER), Piece::Text(" дн.")]);
        assert_eq!(out, ["", "\u{a0}дн."], "{what}");
    }
}

#[test]
fn row_dates_with_a_slot() {
    // `{n} мая`: a `Number` slot is a day or a year; `{d} г.`: a `Date` slot is a year only;
    // a `Word` slot and an `Other` slot are neither.
    let mut configs = vec![("alone".to_owned(), alone(Language::RU_RU, Rule::NbspDates))];
    configs.extend(sets(&Language::RU_RU));
    for (what, t) in &configs {
        for (slot, name) in KINDS {
            let out = run(t, vec![Piece::Slot("x", slot), Piece::Text(" мая")]);
            let expected = if slot.kind() == SlotKind::Number {
                "\u{a0}мая"
            } else {
                " мая"
            };
            assert_eq!(out, ["", expected], "{what} {name} day");
            let out = run(t, vec![Piece::Slot("x", slot), Piece::Text(" г.")]);
            let expected = if matches!(slot.kind(), SlotKind::Number | SlotKind::Date) {
                "\u{a0}г."
            } else {
                " г."
            };
            assert_eq!(out, ["", expected], "{what} {name} year");
        }
    }
}

#[test]
fn row_a_next_abbreviation_after_a_slot() {
    // `{n} ч. Потом`, `{d} г. Затем`: after a `Number` or a `Date` slot the abbreviation is a unit
    // or a year with a sentence full stop and does not bind forward; after a `Word` or an `Other`
    // slot it does.
    let t = alone(Language::RU_RU, Rule::NbspAbbreviations);
    for (slot, name) in KINDS {
        for (abbr, tail) in [("ч.", "Потом"), ("г.", "Затем")] {
            let out = run(
                &t,
                vec![
                    Piece::Slot("x", slot),
                    Piece::Text(&format!(" {abbr} {tail}")),
                ],
            );
            let binds = matches!(slot.kind(), SlotKind::Word | SlotKind::Other);
            let expected = if binds {
                format!(" {abbr}\u{a0}{tail}")
            } else {
                format!(" {abbr} {tail}")
            };
            assert_eq!(out, ["", expected.as_str()], "{name} {abbr}");
        }
    }
    // Section 13.2: `{d} г. Затем` gets no glue after `г.` in the sets either.
    for (what, t) in sets(&Language::RU_RU) {
        let out = run(
            &t,
            vec![Piece::Slot("d", Slot::DATE), Piece::Text(" г. Затем")],
        );
        assert_eq!(out, ["", "\u{a0}г. Затем"], "{what}");
    }
}

#[test]
fn row_glue_before_and_after_a_slot() {
    // `в {x}`, `№ {x}`: glue before a `Number`, `Word` or `Date` slot, not before an `Other` one.
    // `{x} же`: glue after the three, not after an `Other` slot.
    let short = alone(Language::RU_RU, Rule::NbspShortWords);
    let signs = alone(Language::RU_RU, Rule::NbspSigns);
    let particles = alone(Language::RU_RU, Rule::NbspParticles);
    for (slot, name) in KINDS {
        let glued = slot.kind() != SlotKind::Other;
        let out = run(&short, vec![Piece::Text("в "), Piece::Slot("x", slot)]);
        assert_eq!(out, [if glued { "в\u{a0}" } else { "в " }, ""], "в {name}");
        let out = run(&signs, vec![Piece::Text("№ "), Piece::Slot("x", slot)]);
        assert_eq!(out, [if glued { "№\u{a0}" } else { "№ " }, ""], "№ {name}");
        let out = run(&particles, vec![Piece::Slot("x", slot), Piece::Text(" же")]);
        assert_eq!(
            out,
            ["", if glued { "\u{a0}же" } else { " же" }],
            "{name} же"
        );
    }
    // `в {x}` and `{x} же` in both sets.
    for (what, t) in sets(&Language::RU_RU) {
        let out = run(&t, vec![Piece::Text("в "), Piece::Slot("x", Slot::WORD)]);
        assert_eq!(out, ["в\u{a0}", ""], "{what}");
        let out = run(&t, vec![Piece::Text("в "), Piece::Slot("x", Slot::OTHER)]);
        assert_eq!(out, ["в ", ""], "{what}");
        let out = run(
            &t,
            vec![Piece::Slot("name", Slot::WORD), Piece::Text(" же")],
        );
        assert_eq!(out, ["", "\u{a0}же"], "{what}");
    }
    // A quoted or bracketed slot is the core of its fragment: the glue before the fragment sticks
    // to the slot, so an `Other` slot gets none there either (sections 2.6 and 8.2).
    for (slot, name) in KINDS {
        let glued = slot.kind() != SlotKind::Other;
        for (open, close) in [("\"", "\""), ("(", ")")] {
            let out = run(
                &short,
                vec![
                    Piece::Text(&format!("в {open}")),
                    Piece::Slot("x", slot),
                    Piece::Text(close),
                ],
            );
            let before = if glued { "в\u{a0}" } else { "в " };
            assert_eq!(
                out,
                [format!("{before}{open}"), close.to_owned()],
                "в {open}{name}"
            );
        }
    }
}

#[test]
fn row_number_times_with_slots() {
    // `{w}x{h}` in Prose: `Number` slots only (`number.times`, section 8.8).
    let t = prose(Language::RU_RU);
    let out = run(
        &t,
        vec![
            Piece::Slot("w", Slot::NUMBER),
            Piece::Text("x"),
            Piece::Slot("h", Slot::NUMBER),
        ],
    );
    assert_eq!(out, ["", "×", ""]);
    for (a, b) in [
        (Slot::WORD, Slot::WORD),
        (Slot::DATE, Slot::DATE),
        (Slot::OTHER, Slot::OTHER),
        (Slot::NUMBER, Slot::WORD),
    ] {
        let out = run(
            &t,
            vec![Piece::Slot("w", a), Piece::Text("x"), Piece::Slot("h", b)],
        );
        assert_eq!(out, ["", "x", ""], "{a:?} {b:?}");
    }
}

#[test]
fn labels_next_to_slots() {
    // Section 8.2: a `Number` slot after a sign is a label: `№ {n}` and `№{n}` are glued to the
    // sign, the unit and date rules leave it alone, and `г.` after it is a city.
    let t = alone(Language::RU_RU, Rule::NbspSigns);
    let out = run(&t, vec![Piece::Text("№ "), Piece::Slot("n", Slot::NUMBER)]);
    assert_eq!(out, ["№\u{a0}", ""]);
    let out = run(&t, vec![Piece::Text("№"), Piece::Slot("n", Slot::NUMBER)]);
    assert_eq!(out, ["№\u{a0}", ""]);
    // `¶{n}` → `¶⍽{n}` (13.2), not borrowed.
    let out = t.typeset_template(
        [Piece::Text("¶"), Piece::Slot("n", Slot::NUMBER)],
        Output::Unicode,
    );
    assert_eq!(texts(&out), ["¶\u{a0}", ""]);
    assert!(out.changed());
    assert!(matches!(out.texts()[0], Cow::Owned(_)));
    // `№{n} г. Москвы` (13.2, both sets): `г.` is a city.
    for (what, t) in sets(&Language::RU_RU) {
        let out = run(
            &t,
            vec![
                Piece::Text("Школа №"),
                Piece::Slot("n", Slot::NUMBER),
                Piece::Text(" г. Москвы"),
            ],
        );
        assert_eq!(out, ["Школа №\u{a0}", " г.\u{a0}Москвы"], "{what}");
    }
    // The unit rule does not take a label: `№ {n} МБ` keeps its plain space before the unit.
    let units = alone(Language::RU_RU, Rule::NbspUnits);
    let out = run(
        &units,
        vec![
            Piece::Text("№ "),
            Piece::Slot("n", Slot::NUMBER),
            Piece::Text(" МБ"),
        ],
    );
    assert_eq!(out, ["№ ", " МБ"]);
}

#[test]
fn percent_next_to_a_slot() {
    use typolish::PercentSpacing::{Language as Lang, Spaced};
    // `{n}%` in ru with `Spaced` and with `Language`: `{n}⍽%`; none of them borrowed.
    for spacing in [Spaced, Lang] {
        let t = Typographer::builder(Language::RU_RU, typolish::RuleSet::PROSE)
            .percent(spacing)
            .build()
            .unwrap();
        let out = t.typeset_template(
            [Piece::Slot("n", Slot::NUMBER), Piece::Text("%")],
            Output::Unicode,
        );
        assert_eq!(texts(&out), ["", "\u{a0}%"], "{spacing:?}");
        assert!(out.changed());
        assert!(matches!(out.texts()[1], Cow::Owned(_)));
    }
    // `{n}‰` with `nbsp.units` alone and `Spaced`.
    let t = Typographer::builder(
        Language::RU_RU,
        typolish::RuleSet::EMPTY.with(Rule::NbspUnits),
    )
    .percent(Spaced)
    .build()
    .unwrap();
    let out = t.typeset_template(
        [Piece::Slot("n", Slot::NUMBER), Piece::Text("‰")],
        Output::Unicode,
    );
    assert_eq!(texts(&out), ["", "\u{a0}‰"]);
    assert!(out.changed());
    // As written (the default): an attached sign stays attached, a spaced one is glued.
    let t = alone(Language::RU_RU, Rule::NbspUnits);
    assert_eq!(
        run(&t, vec![Piece::Slot("n", Slot::NUMBER), Piece::Text("%")]),
        ["", "%"]
    );
    assert_eq!(
        run(&t, vec![Piece::Slot("n", Slot::NUMBER), Piece::Text(" %")]),
        ["", "\u{a0}%"]
    );
}

#[test]
fn ranges_with_a_literal_next_to_a_slot() {
    // A `Number` slot and a literal number form a range for the unit rule, whose dash no rule
    // edits: `{a}-5 кг` → `{a}-5⍽кг`; `5-{d} г.` is not a range (`Compound`): unchanged.
    for (what, t) in sets(&Language::RU_RU) {
        let out = run(
            &t,
            vec![Piece::Slot("a", Slot::NUMBER), Piece::Text("-5 кг")],
        );
        assert_eq!(out, ["", "-5\u{a0}кг"], "{what}");
        let out = t.typeset_template(
            [
                Piece::Text("5-"),
                Piece::Slot("d", Slot::DATE),
                Piece::Text(" г."),
            ],
            Output::Unicode,
        );
        assert_eq!(texts(&out), ["5-", " г."], "{what}");
        assert!(!out.changed(), "{what}");
    }
    let t = alone(Language::RU_RU, Rule::NbspUnits);
    let out = run(
        &t,
        vec![Piece::Slot("a", Slot::NUMBER), Piece::Text("-5 кг")],
    );
    assert_eq!(out, ["", "-5\u{a0}кг"]);
}

#[test]
fn the_flow_of_a_template_with_a_unit_and_a_dash() {
    // The example of section 2.4: `Файл "{name}" весит {size} МБ - удалить?` in Interface.
    let t = interface(Language::RU_RU);
    let out = run(
        &t,
        vec![
            Piece::Text("Файл \""),
            Piece::Slot("name", Slot::WORD),
            Piece::Text("\" весит "),
            Piece::Slot("size", Slot::NUMBER),
            Piece::Text(" МБ - удалить?"),
        ],
    );
    assert_eq!(out, ["Файл «", "» весит ", "\u{a0}МБ\u{a0}— удалить?"]);
    // And the range of the same section: `{a}-{b} дн.`.
    let out = run(
        &t,
        vec![
            Piece::Slot("a", Slot::NUMBER),
            Piece::Text("-"),
            Piece::Slot("b", Slot::NUMBER),
            Piece::Text(" дн."),
        ],
    );
    assert_eq!(out, ["", "\u{2013}", "\u{a0}дн."]);
}

// The flow of section 2.14: texts written by a generator as the bodies of string literals, values
// formatted at run time with `format_number` and `Output::Unicode`.
fn flow_template() -> Vec<Piece<'static, &'static str>> {
    vec![Piece::Slot("n", Slot::NUMBER), Piece::Text(" руб.")]
}

#[test]
fn the_generator_flow_for_rust_and_json_bodies() {
    let t = interface(Language::RU_RU);
    let unicode = t.typeset_template(flow_template(), Output::Unicode);
    let filled = unicode.fill(|_| Cow::Owned(t.format_number(1000, Output::Unicode).to_string()));
    assert_eq!(filled, vis("1⍽000⍽руб."));
    for style in [EscapeStyle::RUST, EscapeStyle::JSON] {
        let template = t.typeset_template(flow_template(), Output::Escaped(style));
        assert!(template.changed(), "{style:?}");
        // Each text is the body of a literal: decoded, joined with the plain value, it is the text
        // of the `Output::Unicode` template filled the same way, with a real U+00A0.
        let values = [t.format_number(1000, Output::Unicode).to_string()];
        let mut joined = String::new();
        for (i, text) in template.texts().iter().enumerate() {
            joined.push_str(&decode_escaped(style, text).unwrap());
            if let Some(value) = values.get(i) {
                joined.push_str(value);
            }
        }
        assert_eq!(joined, filled, "{style:?}");
        assert!(joined.contains('\u{a0}'));
        // The escaped text contains no raw backslash of the value and the filled text has none.
        assert!(!joined.contains('\\'), "{style:?}");
        // The texts of the escaped template are the escapes of the Unicode texts.
        assert_eq!(
            template.texts()[1],
            if style == EscapeStyle::RUST {
                "\\u{a0}руб."
            } else {
                "\\u00a0руб."
            }
        );
    }
}

#[test]
fn the_generator_flow_for_html_texts_as_debug_literals() {
    // With `Output::Html` the texts are HTML, not bodies of literals: each is written as a complete
    // literal with `{t:?}`, and the decoder of `EscapeStyle::RUST` reads the body of every such
    // literal back to the text unchanged (section 2.14).
    let t = interface(Language::RU_RU);
    let output = Output::Html(HtmlStyle::NAMED.nowrap_class("nowrap"));
    let pieces = || {
        vec![
            Piece::Text("Цена - 5 руб. в "),
            Piece::Protected("`C:\\temp`"),
            Piece::Slot("n", Slot::NUMBER),
            Piece::Text(" МБ"),
        ]
    };
    let template = t.typeset_template(pieces(), output);
    assert!(
        template
            .texts()
            .iter()
            .any(|text| text.contains("<span class=\"nowrap\">"))
    );
    assert!(template.texts().iter().any(|text| text.contains('\\')));
    for text in template.texts() {
        let literal = format!("{text:?}");
        let body = &literal[1..literal.len() - 1];
        assert_eq!(
            decode_escaped(EscapeStyle::RUST, body).as_deref(),
            Ok(text.as_ref()),
            "{literal}"
        );
    }
    // The same for a text with line breaks and quotes.
    let template = t.typeset_template::<()>(
        [Piece::Text("a\nb \"c\" \\ <d> \u{a0}")],
        Output::Html(HtmlStyle::HEX.nowrap_style()),
    );
    for text in template.texts() {
        let literal = format!("{text:?}");
        let body = &literal[1..literal.len() - 1];
        assert_eq!(
            decode_escaped(EscapeStyle::RUST, body).as_deref(),
            Ok(text.as_ref())
        );
    }
}

#[test]
fn run_time_numbers_agree_with_the_template() {
    // Section 2.7: one typographer sets the template at build time and formats the numbers at run
    // time, so `{n} МБ` and the value fit together, in the output kind of the template.
    let t = prose(Language::RU_RU);
    for output in [
        Output::Unicode,
        Output::Html(HtmlStyle::NAMED),
        Output::Escaped(EscapeStyle::JS),
    ] {
        let template = t.typeset_template(
            [
                Piece::Slot("n", Slot::NUMBER),
                Piece::Text(" МБ, "),
                Piece::Slot("m", Slot::NUMBER),
            ],
            output,
        );
        let value = t.format_number(12345, output).to_string();
        let filled = template.fill(|_| Cow::Owned(value.clone()));
        let nbsp = match output {
            Output::Unicode => "\u{a0}",
            Output::Html(_) => "&nbsp;",
            _ => "\\u00A0",
        };
        assert_eq!(filled, format!("{value}{nbsp}МБ, {value}"), "{output:?}");
    }
}
