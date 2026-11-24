//! The application methods of design sections 2.5, 2.6 and 2.12 as far as they hold whatever rules
//! are implemented: no rule means no change, `Cow::Borrowed` exactly when nothing changed, slots
//! on their places, `typeset_with`, `typeset_into` and the thread-local scratch.

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
use std::thread;

use common::{check, vis};
use typolish::{
    DiagnosticKind, EscapeStyle, HtmlStyle, Language, Output, Piece, RuleSet, Scratch, Slot,
    SlotKind, Template, Typographer,
};

static RU_UI: Typographer = Typographer::new(Language::RU_RU, RuleSet::INTERFACE);
static EN_PROSE: Typographer = Typographer::new(Language::EN_US, RuleSet::PROSE);
static NOTHING: Typographer = Typographer::new(Language::RU_RU, RuleSet::EMPTY);

const OUTPUTS: [Output; 4] = [
    Output::Unicode,
    Output::Html(HtmlStyle::NAMED),
    Output::Escaped(EscapeStyle::JS),
    Output::Escaped(EscapeStyle::CSS),
];

#[test]
fn an_empty_rule_set_changes_nothing() {
    for text in [
        "",
        "plain",
        "Файл \"report.pdf\" не найден - проверьте путь...",
        "\"\" '' ... -- (c) -> 5'10\"",
        "a\u{0}b\n\r\n\u{2028}\u{85}",
        "👨\u{200d}👩\u{200d}👧 e\u{301} 🇷🇺",
    ] {
        let out = NOTHING.typeset(text, Output::Unicode);
        assert_eq!(out, text);
        assert!(matches!(out, Cow::Borrowed(_)), "{text:?}");
    }
}

#[test]
fn text_with_nothing_to_encode_is_borrowed_in_every_output() {
    for output in OUTPUTS {
        for text in ["", "plain", "abc 123", "Москва"] {
            let out = NOTHING.typeset(text, output);
            assert!(
                matches!(out, Cow::Borrowed(t) if t == text),
                "{output:?} {text:?}"
            );
        }
    }
}

#[test]
fn technical_text_is_neither_changed_nor_copied() {
    for text in [
        "host:8080",
        "::1",
        "10.0.0.0/8",
        "a:b",
        "tls-server",
        "kv_lang",
        "--force",
    ] {
        let out = RU_UI.typeset(text, Output::Unicode);
        assert!(matches!(out, Cow::Borrowed(t) if t == text), "{text}");
    }
}

#[test]
fn typeset_text_comes_back_borrowed() {
    let s = "Файл «report.pdf» не\u{a0}найден\u{a0}— проверьте путь в\u{a0}`--config`.";
    assert!(matches!(RU_UI.typeset(s, Output::Unicode), Cow::Borrowed(t) if t == s));
}

#[test]
fn typeset_with_equals_typeset_and_reuses_the_scratch() {
    let mut scratch = Scratch::new();
    for text in ["", "a", "Файл \"x\" - y...", "5'10\"", "(c) 2026"] {
        let owned = String::from(text);
        let a = RU_UI
            .typeset_with(&mut scratch, &owned, Output::Unicode)
            .into_owned();
        drop(owned);
        assert_eq!(a, RU_UI.typeset(text, Output::Unicode));
    }
    let mut scratch = Scratch::default();
    let b = EN_PROSE.typeset_with(&mut scratch, "He said \"hi\"", Output::Unicode);
    assert_eq!(b, EN_PROSE.typeset("He said \"hi\"", Output::Unicode));
}

#[test]
fn typeset_into_appends() {
    let mut buf = String::from("[");
    RU_UI.typeset_into("host:8080", Output::Unicode, &mut buf);
    assert_eq!(buf, "[host:8080");
    EN_PROSE.typeset_into("", Output::Unicode, &mut buf);
    assert_eq!(buf, "[host:8080");
    RU_UI.typeset_into("abc", Output::Unicode, &mut buf);
    assert_eq!(buf, "[host:8080abc");
    let mut empty = String::new();
    let text = "Файл \"report.pdf\" не найден - проверьте путь.";
    RU_UI.typeset_into(text, Output::Unicode, &mut empty);
    assert_eq!(empty, RU_UI.typeset(text, Output::Unicode));
}

#[test]
fn typeset_into_keeps_the_capacity_it_has() {
    let mut buf = String::with_capacity(256);
    let ptr = buf.as_ptr();
    NOTHING.typeset_into("some text", Output::Unicode, &mut buf);
    assert_eq!(buf, "some text");
    assert_eq!(buf.as_ptr(), ptr);
    assert!(buf.capacity() >= 256);
}

#[test]
fn a_static_typographer_serves_many_threads() {
    let expected = RU_UI
        .typeset("Файл \"x\" - y", Output::Unicode)
        .into_owned();
    let handles: Vec<_> = (0..4)
        .map(|_| {
            thread::spawn(|| {
                (0..50)
                    .map(|_| {
                        RU_UI
                            .typeset("Файл \"x\" - y", Output::Unicode)
                            .into_owned()
                    })
                    .collect::<Vec<_>>()
            })
        })
        .collect();
    for handle in handles {
        for got in handle.join().unwrap() {
            assert_eq!(got, expected);
        }
    }
}

#[test]
fn a_scratch_moves_between_threads() {
    let mut scratch = Scratch::new();
    let first = RU_UI
        .typeset_with(&mut scratch, "a - b", Output::Unicode)
        .into_owned();
    let (scratch, second) = thread::spawn(move || {
        let out = RU_UI
            .typeset_with(&mut scratch, "a - b", Output::Unicode)
            .into_owned();
        (scratch, out)
    })
    .join()
    .unwrap();
    assert_eq!(first, second);
    drop(scratch);
}

#[test]
fn a_large_input_does_not_break_the_next_call() {
    // The thread-local scratch is replaced after a call that grew it past 4 MiB (design section
    // 2.12).
    let big = "Слово \"слово\" - и ещё... ".repeat(20_000);
    let first = RU_UI.typeset(&big, Output::Unicode).into_owned();
    assert!(first.len() >= big.len() / 2);
    let small = RU_UI.typeset("host:8080", Output::Unicode);
    assert_eq!(small, "host:8080");
    assert_eq!(RU_UI.typeset(&big, Output::Unicode), first);
}

#[test]
fn table_rows_go_through_vis() {
    // A row without a rule's effect: the helper compares and checks `Cow` as the design says.
    let t = Typographer::new(Language::EN_US, RuleSet::EMPTY);
    check(&t, ("5⍽МБ", "5⍽МБ"));
    check(&t, ("5⎵МБ", "5⎵МБ"));
    assert_eq!(vis("5⍽МБ"), "5\u{a0}МБ");
}

// Templates.

fn slots_of<S: Copy>(t: &Template<'_, S>) -> Vec<(S, SlotKind)> {
    t.slots()
        .iter()
        .map(|(s, slot)| (*s, slot.kind()))
        .collect()
}

#[test]
fn the_n_plus_one_form() {
    let pieces = [
        Piece::Slot(0u8, Slot::NUMBER),
        Piece::Slot(1, Slot::WORD),
        Piece::Text("x"),
        Piece::Slot(2, Slot::DATE),
    ];
    let t = NOTHING.typeset_template(pieces, Output::Unicode);
    assert_eq!(t.texts().len(), t.slots().len() + 1);
    assert_eq!(t.texts(), ["", "", "x", ""]);
    assert_eq!(
        slots_of(&t),
        [
            (0, SlotKind::Number),
            (1, SlotKind::Word),
            (2, SlotKind::Date)
        ]
    );
    assert!(!t.changed());
}

#[test]
fn adjacent_pieces_merge_into_one_text() {
    let t = NOTHING.typeset_template::<()>(
        [
            Piece::Text("a"),
            Piece::Protected("`b`"),
            Piece::Markup("<i>"),
            Piece::Text("c"),
        ],
        Output::Unicode,
    );
    assert_eq!(t.texts(), ["a`b`<i>c"]);
    assert!(!t.changed(), "merging is not a change");
}

#[test]
fn a_text_of_one_unchanged_piece_is_borrowed() {
    let t = RU_UI.typeset_template(
        [
            Piece::Slot(1u8, Slot::OTHER),
            Piece::Text("host:8080"),
            Piece::Slot(2, Slot::OTHER),
        ],
        Output::Unicode,
    );
    assert!(matches!(t.texts()[1], Cow::Borrowed("host:8080")));
    let t = NOTHING.typeset_template::<()>([Piece::Protected("`x`")], Output::Unicode);
    assert!(matches!(t.texts()[0], Cow::Borrowed("`x`")));
    let t = NOTHING.typeset_template::<()>([Piece::Markup("<b>")], Output::Unicode);
    assert_eq!(t.texts(), ["<b>"]);
}

#[test]
fn an_empty_template() {
    let t = RU_UI.typeset_template::<u8>(std::iter::empty(), Output::Unicode);
    assert_eq!(t.texts(), [""]);
    assert!(matches!(t.texts()[0], Cow::Borrowed("")));
    assert!(t.slots().is_empty());
    assert!(!t.changed());
    assert_eq!(t.iter().count(), 0);
    assert_eq!(t.fill(|_| Cow::Borrowed("never")), "");
}

#[test]
fn slots_keep_their_payloads_in_order() {
    let names = ["name", "size", "name", "x"];
    let pieces: Vec<Piece<'_, &str>> = vec![
        Piece::Text("Файл \""),
        Piece::Slot(names[0], Slot::WORD),
        Piece::Text("\" весит "),
        Piece::Slot(names[1], Slot::NUMBER),
        Piece::Slot(names[2], Slot::OTHER),
        Piece::Text(" МБ - "),
        Piece::Slot(names[3], Slot::DATE),
    ];
    for typographer in [&RU_UI, &EN_PROSE, &NOTHING] {
        let t = typographer.typeset_template(pieces.iter().copied(), Output::Unicode);
        assert_eq!(t.slots().len(), 4);
        assert_eq!(t.texts().len(), 5);
        assert_eq!(
            slots_of(&t),
            [
                ("name", SlotKind::Word),
                ("size", SlotKind::Number),
                ("name", SlotKind::Other),
                ("x", SlotKind::Date)
            ]
        );
    }
}

#[test]
fn fill_inserts_values_unchanged() {
    let t = NOTHING.typeset_template(
        [
            Piece::Text("<"),
            Piece::Slot("a", Slot::WORD),
            Piece::Text(">"),
            Piece::Slot("b", Slot::OTHER),
        ],
        Output::Unicode,
    );
    assert_eq!(
        t.fill(|name| Cow::Owned(format!("[{name}&<b>]"))),
        "<[a&<b>]>[b&<b>]"
    );
    let html = NOTHING.typeset_template(
        [Piece::Slot("a", Slot::WORD)],
        Output::Html(HtmlStyle::NAMED),
    );
    assert_eq!(html.fill(|_| Cow::Borrowed("<b>")), "<b>");
}

#[test]
fn iter_skips_empty_texts() {
    use typolish::TemplatePart;
    let t = NOTHING.typeset_template(
        [
            Piece::Slot(1u8, Slot::NUMBER),
            Piece::Slot(2, Slot::NUMBER),
            Piece::Text("x"),
        ],
        Output::Unicode,
    );
    assert_eq!(
        t.iter().collect::<Vec<_>>(),
        [
            TemplatePart::Slot(&1, Slot::NUMBER),
            TemplatePart::Slot(&2, Slot::NUMBER),
            TemplatePart::Text("x"),
        ]
    );
    let (texts, slots) = t.into_parts();
    assert_eq!(texts.len(), 3);
    assert_eq!(slots.len(), 2);
}

#[test]
fn typeset_template_with_a_scratch_equals_typeset_template() {
    let pieces = [
        Piece::Text("a - "),
        Piece::Slot(0u8, Slot::WORD),
        Piece::Text("..."),
    ];
    let mut scratch = Scratch::new();
    let a = EN_PROSE.typeset_template_with(&mut scratch, pieces, Output::Unicode);
    let b = EN_PROSE.typeset_template(pieces, Output::Unicode);
    assert_eq!(a, b);
}

#[test]
fn text_pieces_equal_their_concatenation() {
    let whole = "Ждём... Файл \"x\" - y; `code` (c) 5'10\"";
    for typographer in [&RU_UI, &EN_PROSE] {
        let expected = typographer.typeset(whole, Output::Unicode).into_owned();
        for cut in whole.char_indices().map(|(i, _)| i).chain([whole.len()]) {
            let (a, b) = whole.split_at(cut);
            let t = typographer
                .typeset_template::<()>([Piece::Text(a), Piece::Text(b)], Output::Unicode);
            assert_eq!(t.texts().len(), 1);
            assert_eq!(t.texts()[0], expected, "cut at {cut}");
        }
    }
}

#[test]
fn an_iterator_may_call_the_typographer_and_a_template_may_nest() {
    let t = EN_PROSE.typeset_template(
        ["a", "b"].into_iter().map(|s| {
            let inner = EN_PROSE.typeset_template::<()>([Piece::Text(s)], Output::Unicode);
            assert_eq!(inner.texts()[0], s);
            Piece::<()>::Text(s)
        }),
        Output::Unicode,
    );
    assert_eq!(t.texts(), ["ab"]);
}

#[test]
fn the_template_is_a_value() {
    let a = NOTHING.typeset_template(
        [Piece::Text("a"), Piece::Slot(1u8, Slot::WORD)],
        Output::Unicode,
    );
    let b = a.clone();
    assert_eq!(a, b);
    assert!(!format!("{a:?}").is_empty());
}

#[test]
fn a_rejected_template_after_a_changed_one_reuses_no_state() {
    // Design sections 2.12 and 3.2: a template that the prefilter rejects is rendered from its
    // pieces alone, never from the working memory of the previous call.
    let t = Typographer::new(Language::EN_US, RuleSet::INTERFACE);
    let mut scratch = Scratch::new();
    let out = t.typeset_template_with(
        &mut scratch,
        [
            Piece::Text("He said \"hi\" - ok"),
            Piece::Slot(0u8, Slot::OTHER),
            Piece::Text("x \"y\""),
        ],
        Output::Unicode,
    );
    assert_eq!(out.texts(), ["He said “hi” — ok", "x “y”"]);
    assert!(out.changed());
    let plain = || {
        [
            Piece::Text("plain"),
            Piece::Slot(0u8, Slot::OTHER),
            Piece::Text("text"),
        ]
    };
    let out = t.typeset_template_with(&mut scratch, plain(), Output::Unicode);
    assert_eq!(out.texts(), ["plain", "text"]);
    assert!(!out.changed());
    let fresh = t.typeset_template_with(&mut Scratch::new(), plain(), Output::Unicode);
    assert_eq!(out.texts(), fresh.texts());
}

// `Piece::BlockMarkup` (design sections 2.6 and 3.3): markup that ends the paragraph.

#[test]
fn block_markup_is_written_like_markup_in_every_output_kind() {
    // Written as is in Unicode and HTML output, escaped like text in escaped output; the text after
    // it is a paragraph of its own, so `в` gets no glue across it.
    let t = Typographer::new(Language::RU_RU, RuleSet::PROSE);
    let pieces = [
        Piece::<()>::Text("в "),
        Piece::BlockMarkup("</p>\n<p>"),
        Piece::Text("доме - \"да\""),
    ];
    let cases = [
        (Output::Unicode, "в </p>\n<p>доме\u{a0}— «да»"),
        (
            Output::Html(HtmlStyle::NAMED),
            "в </p>\n<p>доме&nbsp;&mdash; &laquo;да&raquo;",
        ),
        (
            Output::Html(HtmlStyle::NAMED.nowrap_class("nw")),
            "в </p>\n<p><span class=\"nw\">доме &mdash;</span> &laquo;да&raquo;",
        ),
        (
            Output::Escaped(EscapeStyle::JS),
            "в \\u003C/p\\u003E\\n\\u003Cp\\u003Eдоме\\u00A0\\u2014 \\u00ABда\\u00BB",
        ),
        (
            Output::Escaped(EscapeStyle::RUST),
            "в </p>\\n<p>доме\\u{a0}\\u{2014} \\u{ab}да\\u{bb}",
        ),
        (
            Output::Escaped(EscapeStyle::CSS),
            "в \\3c /p>\\a \\3c p>доме\\a0 \\2014  \\ab да\\bb ",
        ),
    ];
    for (output, expected) in cases {
        let template = t.typeset_template(pieces, output);
        assert_eq!(template.texts(), [expected], "{output:?}");
        assert!(template.changed());
    }
}

#[test]
fn block_markup_ends_quotes_glue_and_nowrap_groups() {
    let t = Typographer::new(Language::RU_RU, RuleSet::PROSE);
    let nowrap = Output::Html(HtmlStyle::NAMED.nowrap_class("nw"));
    // Across `Markup` the short word is glued and the group split at the markup is reported;
    // across `BlockMarkup` nothing is glued and nothing is reported.
    let markup = [
        Piece::<()>::Text("в "),
        Piece::Markup("<br>"),
        Piece::Text("доме"),
    ];
    let r = t.report_template(markup, nowrap);
    assert_eq!(r.result().texts(), ["в&nbsp;<br>доме"]);
    assert_eq!(r.diagnostics().len(), 1);
    let block = [
        Piece::<()>::Text("в "),
        Piece::BlockMarkup("<br>"),
        Piece::Text("доме"),
    ];
    let r = t.report_template(block, nowrap);
    assert_eq!(r.result().texts(), ["в <br>доме"]);
    assert!(r.changes().is_empty() && r.diagnostics().is_empty());
    assert!(t.lint_template(block).is_empty());
    // A quote on each side stays unpaired, and each is reported in its own piece.
    let pieces = [
        Piece::<()>::Text("Он сказал \"да"),
        Piece::BlockMarkup("</p><p>"),
        Piece::Text("Нет\" - ответила она."),
    ];
    let linted: Vec<(DiagnosticKind, usize, std::ops::Range<usize>)> = t
        .lint_template(pieces)
        .iter()
        .map(|d| (d.kind().clone(), d.piece(), d.range()))
        .collect();
    assert_eq!(
        linted,
        [
            (DiagnosticKind::UnmatchedQuote, 0, 18..19),
            (DiagnosticKind::UnmatchedQuote, 2, 6..7)
        ]
    );
    let r = t.report_template(pieces, Output::Unicode);
    let changes: Vec<(&str, usize)> = r
        .changes()
        .iter()
        .map(|c| (c.rule().id(), c.piece()))
        .collect();
    assert_eq!(
        changes,
        [
            ("nbsp.short-length", 0),
            ("nbsp.dash", 2),
            ("dash.spaced-hyphen", 2)
        ]
    );
}

#[test]
fn block_markup_starts_a_line_and_keeps_the_template_contract() {
    let t = Typographer::new(Language::RU_RU, RuleSet::PROSE);
    // An empty piece is a boundary too: the dialogue dash after it is placed.
    let pieces = [
        Piece::<()>::Text("Текст."),
        Piece::BlockMarkup(""),
        Piece::Text("- Да, - сказал он."),
    ];
    assert_eq!(
        t.typeset_template(pieces, Output::Unicode).texts(),
        ["Текст.—\u{a0}Да,\u{a0}— сказал он."]
    );
    // A slot before it is no neighbor of the text after it: no glue to a unit.
    let pieces = [
        Piece::Slot("n", Slot::NUMBER),
        Piece::BlockMarkup("<br><br>"),
        Piece::Text("МБ - \"x\""),
    ];
    let template = t.typeset_template(pieces, Output::Unicode);
    assert_eq!(template.texts(), ["", "<br><br>МБ\u{a0}— «x»"]);
    assert_eq!(slots_of(&template), [("n", SlotKind::Number)]);
    // Two slots with a block boundary between them are not glued together.
    let glued = |middle| {
        t.lint_template([
            Piece::Slot("a", Slot::WORD),
            middle,
            Piece::Slot("b", Slot::WORD),
        ])
        .iter()
        .map(|d| d.kind().clone())
        .collect::<Vec<_>>()
    };
    assert_eq!(glued(Piece::Markup("<br>")), [DiagnosticKind::SlotGlue]);
    assert!(glued(Piece::BlockMarkup("<br>")).is_empty());
    // Nothing to change: the merged text is no change.
    let pieces = [
        Piece::<()>::Text("Готово"),
        Piece::BlockMarkup("<br>"),
        Piece::Text("Да"),
    ];
    let template = t.typeset_template(pieces, Output::Unicode);
    assert_eq!(template.texts(), ["Готово<br>Да"]);
    assert!(!template.changed());
    let one = [Piece::<()>::BlockMarkup("<hr>")];
    let template = t.typeset_template(one, Output::Unicode);
    assert!(matches!(template.texts()[0], Cow::Borrowed("<hr>")));
}

#[test]
fn to_plain_template_reads_no_neighbor_across_block_markup() {
    // The `’` after the block starts its paragraph: with the `‘` before the block read as its
    // neighbor it would be taken for the closer of a sign-only pair and kept.
    for set in [RuleSet::PROSE, RuleSet::INTERFACE, RuleSet::ALL] {
        let t = Typographer::new(Language::EN_US, set);
        let template = t.to_plain_template([
            Piece::<()>::Text("‘’"),
            Piece::BlockMarkup("|"),
            Piece::Text("'a.-"),
        ]);
        assert_eq!(template.texts(), ["\"\"|'a.-"], "{set}");
    }
}

#[test]
fn a_fenced_block_runs_across_block_markup() {
    // Design section 10.1: a fenced block may hold paragraph boundaries, a `BlockMarkup` piece
    // among them; the text after the closing run is typeset.
    let t = Typographer::new(Language::EN_US, RuleSet::PROSE);
    let pieces = [
        Piece::<()>::Text("```\n\"a\" - b"),
        Piece::BlockMarkup("</p><p>"),
        Piece::Text("\"c\" - d\n```\n\"e\""),
    ];
    assert_eq!(
        t.typeset_template(pieces, Output::Unicode).texts(),
        ["```\n\"a\" - b</p><p>\"c\" - d\n```\n“e”"]
    );
}
