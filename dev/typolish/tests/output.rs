//! The output kinds (design sections 2.8, 11 and 13.2): the typographic set and its table, HTML
//! text and its styles, the nowrap groups, the escaped string bodies of every syntax and scope,
//! the inputs that no rule touches but the style must encode (never borrowed), templates, and the
//! comparison of the table of design section 11.2 with `tests/fixtures/whatwg-entities.txt`.
//!
//! The expectations come from the tables of design sections 11.2, 11.4 and 11.6 and from the
//! examples of design sections 2.5, 2.8 and 11.5. A fixed set of inputs is run through every kind
//! and compared with the reference encoder of `tests/common/decode.rs`, which is written from the
//! same tables and knows no nowrap element; the nowrap kinds are checked by decoding (design
//! section 11.8).

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

use common::decode::{
    NAMES, TYPOGRAPHIC, all_outputs, check_safe_alphabet, decode, encode_reference, name_of,
};
use common::{show, vis};
use typolish::{
    DiagnosticKind, EscapeStyle, HtmlStyle, Language, NarrowGlue, Output, Piece, QuotePairs, Rule,
    RuleSet, Scratch, Severity, Slot, Typographer,
};

fn empty(language: Language) -> Typographer {
    Typographer::new(language, RuleSet::EMPTY)
}

fn ru_ui() -> Typographer {
    Typographer::new(Language::RU_RU, RuleSet::INTERFACE)
}

fn ru_prose() -> Typographer {
    Typographer::new(Language::RU_RU, RuleSet::PROSE)
}

const NAMED: Output = Output::Html(HtmlStyle::NAMED);
const JS: Output = Output::Escaped(EscapeStyle::JS);
const RUST: Output = Output::Escaped(EscapeStyle::RUST);
const CSS: Output = Output::Escaped(EscapeStyle::CSS);

fn nowrap() -> Output {
    Output::Html(HtmlStyle::NAMED.nowrap_class("nowrap"))
}

#[track_caller]
fn eq(what: &str, got: &str, expected: &str) {
    assert!(
        got == expected,
        "{what}\nexpected {}\ngot      {}",
        show(expected),
        show(got)
    );
}

/// Design section 11.2: the characters of the typographic set and their `NAMED` form.
const SET: &[(char, &str)] = &[
    ('\u{a0}', "&nbsp;"),
    ('\u{202f}', "&#x202F;"),
    ('\u{2009}', "&thinsp;"),
    ('\u{2011}', "&#x2011;"),
    ('–', "&ndash;"),
    ('—', "&mdash;"),
    ('…', "&hellip;"),
    ('«', "&laquo;"),
    ('»', "&raquo;"),
    ('„', "&bdquo;"),
    ('“', "&ldquo;"),
    ('”', "&rdquo;"),
    ('‘', "&lsquo;"),
    ('’', "&rsquo;"),
    ('‚', "&sbquo;"),
    ('‹', "&lsaquo;"),
    ('›', "&rsaquo;"),
    ('−', "&minus;"),
    ('×', "&times;"),
    ('′', "&prime;"),
    ('″', "&Prime;"),
    ('©', "&copy;"),
    ('®', "&reg;"),
    ('™', "&trade;"),
    ('→', "&rarr;"),
    ('←', "&larr;"),
    ('↔', "&harr;"),
    ('≥', "&ge;"),
    ('≤', "&le;"),
    ('≠', "&ne;"),
    ('±', "&plusmn;"),
];

#[test]
fn the_table_lists_the_whole_set() {
    let table: Vec<char> = SET.iter().map(|(c, _)| *c).collect();
    assert_eq!(table, TYPOGRAPHIC);
    for (name, c) in NAMES {
        assert_eq!(
            SET.iter().find(|(ch, _)| ch == c).map(|(_, n)| *n),
            Some(format!("&{name};").as_str())
        );
    }
}

#[test]
fn html_writes_the_typographic_set_in_every_style() {
    let t = empty(Language::RU_RU);
    for (c, named) in SET {
        let text = format!("a{c}b");
        let cp = u32::from(*c);
        eq(
            &format!("NAMED {c:?}"),
            &t.typeset(&text, NAMED),
            &format!("a{named}b"),
        );
        eq(
            &format!("DECIMAL {c:?}"),
            &t.typeset(&text, Output::Html(HtmlStyle::DECIMAL)),
            &format!("a&#{cp};b"),
        );
        eq(
            &format!("HEX {c:?}"),
            &t.typeset(&text, Output::Html(HtmlStyle::HEX)),
            &format!("a&#x{cp:X};b"),
        );
        // `CHARACTERS` writes the characters themselves: nothing to encode, so it is borrowed.
        let out = t.typeset(&text, Output::Html(HtmlStyle::CHARACTERS));
        assert!(matches!(out, Cow::Borrowed(_)), "CHARACTERS {c:?}");
        assert_eq!(out, text);
    }
}

#[test]
fn escaped_writes_the_typographic_set_in_every_syntax() {
    let t = empty(Language::RU_RU);
    for (c, _) in SET {
        let text = format!("a{c}b");
        let cp = u32::from(*c);
        let hex = |digits: usize| format!("{cp:0digits$X}");
        let rows: Vec<(Output, String)> = vec![
            (JS, format!("a\\u{}b", hex(4))),
            (
                Output::Escaped(EscapeStyle::JS_HEX),
                if cp <= 0xff {
                    format!("a\\x{}b", hex(2))
                } else {
                    format!("a\\u{}b", hex(4))
                },
            ),
            (
                Output::Escaped(EscapeStyle::JS_CODE_POINT),
                format!("a\\u{{{cp:X}}}b"),
            ),
            (Output::Escaped(EscapeStyle::JSON), format!("a\\u{cp:04x}b")),
            (RUST, format!("a\\u{{{cp:x}}}b")),
            (CSS, format!("a\\{cp:x} b")),
        ];
        for (output, expected) in rows {
            eq(
                &format!("{output:?} {c:?}"),
                &t.typeset(&text, output),
                &expected,
            );
        }
    }
    // The design's own examples of the table of design section 11.2.
    eq("JS nbsp", &t.typeset("\u{a0}", JS), "\\u00A0");
    eq("JS narrow", &t.typeset("\u{202f}", JS), "\\u202F");
    eq("JS dashes", &t.typeset("–—", JS), "\\u2013\\u2014");
    eq("Rust nbsp", &t.typeset("\u{a0}", RUST), "\\u{a0}");
    eq("CSS nbsp", &t.typeset("\u{a0}", CSS), "\\a0 ");
    eq("CSS em dash", &t.typeset("—", CSS), "\\2014 ");
}

#[test]
fn html_escapes_markup_in_every_style() {
    let t = empty(Language::EN_US);
    for style in [
        HtmlStyle::NAMED,
        HtmlStyle::DECIMAL,
        HtmlStyle::HEX,
        HtmlStyle::CHARACTERS,
    ] {
        let output = Output::Html(style);
        eq(
            "markup",
            &t.typeset("a & b < c > d \"e\" 'f'", output),
            "a &amp; b &lt; c &gt; d &quot;e&quot; &#39;f&#39;",
        );
        // A literal `&nbsp;` in the text is a literal ampersand (design section 11.4).
        eq(
            "literal entity",
            &t.typeset("&nbsp; &amp;", output),
            "&amp;nbsp; &amp;amp;",
        );
        eq(
            "tag",
            &t.typeset("<b>x</b>", output),
            "&lt;b&gt;x&lt;/b&gt;",
        );
        // Cyrillic and other text stay UTF-8 in every style.
        let out = t.typeset("Да, 😀 ё", output);
        assert!(matches!(out, Cow::Borrowed(_)));
        assert_eq!(out, "Да, 😀 ё");
    }
}

#[test]
fn protected_spans_are_encoded_too() {
    // Protection means "not typeset", not "not encoded" (design section 11.1).
    let t = ru_ui();
    let text = "Запустите `a<b && c>d \\ \"x\"` сейчас";
    eq(
        "html",
        &t.typeset(text, NAMED),
        "Запустите `a&lt;b &amp;&amp; c&gt;d \\ &quot;x&quot;` сейчас",
    );
    eq(
        "js",
        &t.typeset(text, JS),
        "Запустите \\u0060a\\u003Cb \\u0026\\u0026 c\\u003Ed \\\\ \\\"x\\\"\\u0060 сейчас",
    );
    eq(
        "rust",
        &t.typeset(text, RUST),
        "Запустите `a<b && c>d \\\\ \\\"x\\\"` сейчас",
    );
    eq(
        "json",
        &t.typeset(text, Output::Escaped(EscapeStyle::JSON)),
        "Запустите `a\\u003cb \\u0026\\u0026 c\\u003ed \\\\ \\\"x\\\"` сейчас",
    );
    eq(
        "css",
        &t.typeset(text, CSS),
        "Запустите `a\\3c b && c>d \\\\ \\\"x\\\"` сейчас",
    );
    // A backslash path in a protected span, the case design section 11.6 names.
    eq(
        "path",
        &t.typeset("Путь `C:\\temp\\new` готов", RUST),
        "Путь `C:\\\\temp\\\\new` готов",
    );
}

// Design section 11.6: every row of the table, for the six syntaxes, with no rule running.
// `[JS, JS_HEX, JS_CODE_POINT, JSON, RUST, CSS]`.
const ESCAPES: &[(&str, [&str; 6])] = &[
    ("\\", ["\\\\", "\\\\", "\\\\", "\\\\", "\\\\", "\\\\"]),
    ("\"", ["\\\"", "\\\"", "\\\"", "\\\"", "\\\"", "\\\""]),
    ("'", ["\\u0027", "\\x27", "\\u{27}", "'", "'", "\\'"]),
    ("`", ["\\u0060", "\\x60", "\\u{60}", "`", "`", "`"]),
    (
        "<",
        ["\\u003C", "\\x3C", "\\u{3C}", "\\u003c", "<", "\\3c "],
    ),
    (">", ["\\u003E", "\\x3E", "\\u{3E}", "\\u003e", ">", ">"]),
    ("&", ["\\u0026", "\\x26", "\\u{26}", "\\u0026", "&", "&"]),
    ("${", ["\\u0024{", "\\x24{", "\\u{24}{", "${", "${", "${"]),
    ("$", ["\\u0024", "\\x24", "\\u{24}", "$", "$", "$"]),
    ("$a", ["$a", "$a", "$a", "$a", "$a", "$a"]),
    ("{", ["{", "{", "{", "{", "{", "{"]),
    ("\n", ["\\n", "\\n", "\\n", "\\n", "\\n", "\\a "]),
    ("\r", ["\\r", "\\r", "\\r", "\\r", "\\r", "\\d "]),
    ("\t", ["\\t", "\\t", "\\t", "\\t", "\\t", "\\9 "]),
    (
        "\r\n",
        ["\\r\\n", "\\r\\n", "\\r\\n", "\\r\\n", "\\r\\n", "\\d \\a "],
    ),
    (
        "\u{0}",
        ["\\u0000", "\\x00", "\\u{0}", "\\u0000", "\\0", "\\fffd "],
    ),
    (
        "\u{1}",
        ["\\u0001", "\\x01", "\\u{1}", "\\u0001", "\\u{1}", "\\1 "],
    ),
    (
        "\u{8}",
        ["\\u0008", "\\x08", "\\u{8}", "\\u0008", "\\u{8}", "\\8 "],
    ),
    (
        "\u{c}",
        ["\\u000C", "\\x0C", "\\u{C}", "\\u000c", "\\u{c}", "\\c "],
    ),
    (
        "\u{1f}",
        ["\\u001F", "\\x1F", "\\u{1F}", "\\u001f", "\\u{1f}", "\\1f "],
    ),
    (
        "\u{7f}",
        ["\\u007F", "\\x7F", "\\u{7F}", "\\u007f", "\\u{7f}", "\\7f "],
    ),
    (
        "\u{2028}",
        [
            "\\u2028",
            "\\u2028",
            "\\u{2028}",
            "\\u2028",
            "\u{2028}",
            "\u{2028}",
        ],
    ),
    (
        "\u{2029}",
        [
            "\\u2029",
            "\\u2029",
            "\\u{2029}",
            "\\u2029",
            "\u{2029}",
            "\u{2029}",
        ],
    ),
    (
        "\u{a0}",
        ["\\u00A0", "\\xA0", "\\u{A0}", "\\u00a0", "\\u{a0}", "\\a0 "],
    ),
    (
        "\u{202f}",
        [
            "\\u202F",
            "\\u202F",
            "\\u{202F}",
            "\\u202f",
            "\\u{202f}",
            "\\202f ",
        ],
    ),
    (
        "—",
        [
            "\\u2014",
            "\\u2014",
            "\\u{2014}",
            "\\u2014",
            "\\u{2014}",
            "\\2014 ",
        ],
    ),
    (
        "«",
        ["\\u00AB", "\\xAB", "\\u{AB}", "\\u00ab", "\\u{ab}", "\\ab "],
    ),
    // The default scope leaves the C1 controls and other text as they are.
    (
        "\u{85}",
        ["\u{85}", "\u{85}", "\u{85}", "\u{85}", "\u{85}", "\u{85}"],
    ),
    ("Да", ["Да", "Да", "Да", "Да", "Да", "Да"]),
    ("ё", ["ё", "ё", "ё", "ё", "ё", "ё"]),
    ("😀", ["😀", "😀", "😀", "😀", "😀", "😀"]),
    (
        "plain text",
        [
            "plain text",
            "plain text",
            "plain text",
            "plain text",
            "plain text",
            "plain text",
        ],
    ),
];

fn syntaxes() -> [EscapeStyle; 6] {
    [
        EscapeStyle::JS,
        EscapeStyle::JS_HEX,
        EscapeStyle::JS_CODE_POINT,
        EscapeStyle::JSON,
        EscapeStyle::RUST,
        EscapeStyle::CSS,
    ]
}

#[test]
fn escaped_bodies_follow_the_table() {
    let t = empty(Language::RU_RU);
    for (input, expected) in ESCAPES {
        for (style, expected) in syntaxes().into_iter().zip(expected) {
            let got = t.typeset(input, Output::Escaped(style));
            eq(&format!("{style:?} {input:?}"), &got, expected);
            // Borrowed exactly when the text needs no escape (design section 2.5).
            assert_eq!(
                matches!(got, Cow::Borrowed(_)),
                got == *input,
                "{style:?} {input:?}"
            );
        }
    }
}

#[test]
fn bidi_controls_are_escaped_in_every_syntax() {
    // Design section 11.6: the bidi controls are escaped in every escaped style, so a Rust literal
    // compiles and no control reorders the code around the literal.
    let bidi = [
        '\u{61c}', '\u{200e}', '\u{200f}', '\u{202a}', '\u{202b}', '\u{202c}', '\u{202d}',
        '\u{202e}', '\u{2066}', '\u{2067}', '\u{2068}', '\u{2069}',
    ];
    let t = empty(Language::COMMON);
    let input = "a\u{202E}b\u{2066}c\u{200F}d\u{61C}e\u{200E}f";
    let all: String = bidi.iter().collect();
    for style in syntaxes() {
        for text in [input, all.as_str()] {
            let output = Output::Escaped(style);
            let got = t.typeset(text, output);
            assert!(matches!(got, Cow::Owned(_)), "{style:?} {text:?}");
            assert!(!got.contains(bidi), "{style:?}: {got:?}");
            assert_eq!(decode(output, &got).unwrap(), text, "{style:?}");
        }
    }
    assert_eq!(
        t.typeset(input, Output::Escaped(EscapeStyle::RUST)),
        "a\\u{202e}b\\u{2066}c\\u{200f}d\\u{61c}e\\u{200e}f"
    );
    assert_eq!(
        t.typeset(input, Output::Escaped(EscapeStyle::JS)),
        "a\\u202Eb\\u2066c\\u200Fd\\u061Ce\\u200Ef"
    );
}

// The scope `non_ascii()` escapes every non-ASCII character; the container is escaped as before.
const NON_ASCII: &[(&str, [&str; 6])] = &[
    (
        "Да",
        [
            "\\u0414\\u0430",
            "\\u0414\\u0430",
            "\\u{414}\\u{430}",
            "\\u0414\\u0430",
            "\\u{414}\\u{430}",
            "\\414 \\430 ",
        ],
    ),
    (
        "é",
        ["\\u00E9", "\\xE9", "\\u{E9}", "\\u00e9", "\\u{e9}", "\\e9 "],
    ),
    (
        "\u{85}",
        ["\\u0085", "\\x85", "\\u{85}", "\\u0085", "\\u{85}", "\\85 "],
    ),
    (
        "\u{2028}",
        [
            "\\u2028",
            "\\u2028",
            "\\u{2028}",
            "\\u2028",
            "\\u{2028}",
            "\\2028 ",
        ],
    ),
    (
        "😀",
        [
            "\\uD83D\\uDE00",
            "\\uD83D\\uDE00",
            "\\u{1F600}",
            "\\ud83d\\ude00",
            "\\u{1f600}",
            "\\1f600 ",
        ],
    ),
    (
        "a—b",
        [
            "a\\u2014b",
            "a\\u2014b",
            "a\\u{2014}b",
            "a\\u2014b",
            "a\\u{2014}b",
            "a\\2014 b",
        ],
    ),
    // The ASCII container does not change with the scope.
    (
        "a\"b\\c'd",
        [
            "a\\\"b\\\\c\\u0027d",
            "a\\\"b\\\\c\\x27d",
            "a\\\"b\\\\c\\u{27}d",
            "a\\\"b\\\\c'd",
            "a\\\"b\\\\c'd",
            "a\\\"b\\\\c\\'d",
        ],
    ),
    (
        "\n\u{0}",
        [
            "\\n\\u0000",
            "\\n\\x00",
            "\\n\\u{0}",
            "\\n\\u0000",
            "\\n\\0",
            "\\a \\fffd ",
        ],
    ),
    (
        "plain",
        ["plain", "plain", "plain", "plain", "plain", "plain"],
    ),
];

#[test]
fn non_ascii_scope_escapes_every_non_ascii_character() {
    let t = empty(Language::RU_RU);
    for (input, expected) in NON_ASCII {
        for (style, expected) in syntaxes().into_iter().zip(expected) {
            let got = t.typeset(input, Output::Escaped(style.non_ascii()));
            eq(&format!("{style:?}.non_ascii() {input:?}"), &got, expected);
        }
    }
}

#[test]
fn css_fixed_rows() {
    // Design section 13.2: the inputs of CSS, which the property tests treat separately.
    let rows = [
        ("a\u{0}b", "a\\fffd b"),
        ("a\u{c}b", "a\\c b"),
        ("a\r\nb", "a\\d \\a b"),
        ("a\u{85}b", "a\u{85}b"),
        ("a\u{2028}b", "a\u{2028}b"),
    ];
    for t in [
        empty(Language::RU_RU),
        ru_ui(),
        ru_prose(),
        Typographer::new(Language::EN_US, RuleSet::PROSE),
    ] {
        for (input, expected) in rows {
            eq(&format!("CSS {input:?}"), &t.typeset(input, CSS), expected);
        }
    }
    // A raw U+FFFD is written as it is, `\fffd ` only with `non_ascii()`.
    let t = empty(Language::RU_RU);
    eq("U+FFFD", &t.typeset("a\u{fffd}b", CSS), "a\u{fffd}b");
    eq(
        "U+FFFD non_ascii",
        &t.typeset("a\u{fffd}b", Output::Escaped(EscapeStyle::CSS.non_ascii())),
        "a\\fffd b",
    );
}

#[test]
fn rows_that_no_rule_touches_but_the_style_encodes_are_never_borrowed() {
    // Design sections 11.6 and 13.2: the prefilter takes the bytes a style encodes from the table.
    let ui = ru_ui();
    let json = Output::Escaped(EscapeStyle::JSON);
    let js_na = Output::Escaped(EscapeStyle::JS.non_ascii());
    let out = ui.typeset("a</script>b", JS);
    assert!(matches!(out, Cow::Owned(_)));
    eq("a</script>b", &out, "a\\u003C/script\\u003Eb");
    let out = ui.typeset("AT&T", json);
    assert!(matches!(out, Cow::Owned(_)));
    eq("AT&T", &out, "AT\\u0026T");
    let out = ui.typeset("Да", js_na);
    assert!(matches!(out, Cow::Owned(_)));
    eq("Да", &out, "\\u0414\\u0430");
    // The same inputs in kinds that do not encode them come back borrowed.
    assert!(matches!(ui.typeset("Да", JS), Cow::Borrowed("Да")));
    assert!(matches!(ui.typeset("AT&T", RUST), Cow::Borrowed("AT&T")));
    assert!(matches!(
        ui.typeset("a</script>b", Output::Unicode),
        Cow::Borrowed(_)
    ));
    assert!(matches!(ui.typeset("a</script>b", RUST), Cow::Borrowed(_)));
    assert!(matches!(ui.typeset("a</script>b", CSS), Cow::Owned(_)));
}

// (input, output, borrowed): typeset with no rule, so only the encoding decides.
#[test]
fn borrowed_exactly_when_nothing_is_encoded() {
    let t = empty(Language::RU_RU);
    let json = Output::Escaped(EscapeStyle::JSON);
    let rows: &[(&str, Output, bool)] = &[
        ("Сохранить", Output::Unicode, true),
        ("Сохранить", NAMED, true),
        ("Сохранить", Output::Html(HtmlStyle::CHARACTERS), true),
        ("Сохранить", nowrap(), true),
        ("Сохранить", JS, true),
        ("Сохранить", json, true),
        ("Сохранить", RUST, true),
        ("Сохранить", CSS, true),
        ("a&b", Output::Unicode, true),
        ("a&b", NAMED, false),
        ("a&b", Output::Html(HtmlStyle::CHARACTERS), false),
        ("a&b", JS, false),
        ("a&b", json, false),
        ("a&b", RUST, true),
        ("a&b", CSS, true),
        ("a<b", CSS, false),
        ("a>b", CSS, true),
        ("a>b", RUST, true),
        ("a>b", json, false),
        ("x\"y", Output::Unicode, true),
        ("x\"y", NAMED, false),
        ("x\"y", Output::Html(HtmlStyle::CHARACTERS), false),
        ("x\"y", RUST, false),
        ("x'y", NAMED, false),
        ("x'y", JS, false),
        ("x'y", json, true),
        ("x'y", RUST, true),
        ("x'y", CSS, false),
        ("x`y", JS, false),
        ("x`y", json, true),
        ("x`y", CSS, true),
        ("a$b", JS, true),
        ("a$", JS, false),
        ("a$", json, true),
        ("a${b", JS, false),
        ("a${b", json, true),
        ("a\\b", RUST, false),
        ("a\\b", CSS, false),
        ("a\\b", NAMED, true),
        ("a\nb", JS, false),
        ("a\nb", NAMED, true),
        ("a\nb", CSS, false),
        ("a\u{7f}b", RUST, false),
        ("a\u{2028}b", JS, false),
        ("a\u{2028}b", RUST, true),
        ("a\u{2028}b", CSS, true),
        ("a\u{85}b", JS, true),
        (
            "a\u{85}b",
            Output::Escaped(EscapeStyle::JS.non_ascii()),
            false,
        ),
        ("a\u{a0}b", Output::Unicode, true),
        ("a\u{a0}b", NAMED, false),
        ("a\u{a0}b", Output::Html(HtmlStyle::CHARACTERS), true),
        ("a\u{a0}b", JS, false),
        ("a—b", NAMED, false),
        ("a—b", Output::Html(HtmlStyle::HEX), false),
        ("a—b", Output::Html(HtmlStyle::CHARACTERS), true),
        ("a—b", JS, false),
        ("a😀b", JS, true),
        (
            "a😀b",
            Output::Escaped(EscapeStyle::RUST.non_ascii()),
            false,
        ),
    ];
    for (input, output, borrowed) in rows {
        let got = t.typeset(input, *output);
        assert_eq!(
            matches!(got, Cow::Borrowed(_)),
            *borrowed,
            "{input:?} in {output:?} gave {got:?}"
        );
        // Borrowed exactly when the output equals the input byte for byte.
        assert_eq!(got == *input, *borrowed, "{input:?} in {output:?}");
    }
}

// The fixed set of inputs: unpaired quotes, backslashes in protected spans, line breaks, U+2028,
// U+2029, blank lines between paragraphs with quotes on both sides, `<script>`, `${`, characters
// above the BMP, controls. It stays as a regression table next to the safe-alphabet property.
const FIXED: &[&str] = &[
    "\"раз",
    "раз\"",
    "\"раз\n\nдва\"",
    "\"раз\"\n\n\"два\"",
    "\"one\"\n\n\"two\"",
    "\"one\n\ntwo\"",
    "He said \"hi",
    "Путь `C:\\temp\\new` готов",
    "Run `C:\\temp\\new` now",
    "a\\b \"c\" 'd'",
    "line1\nline2\r\nline3\n",
    "a\u{2028}b\u{2029}c",
    "<script>alert(\"x\")</script>",
    "a</script>b <style>x</style> -->",
    "${x} $y ${ `z`",
    "😀 \"smile\" 😀 𝔘",
    "AT&T &nbsp; &amp; &#39;",
    "Цена - 5 руб. и т.д... \"да\"",
    "(c) 2026 Brand(tm) x -> y -5 1920x1080",
    "a\u{0}b \u{1}\u{7f}\u{85}",
    "it's a 'test' of \"quotes\" for 5'10\" and '90s",
    "\u{a0}\u{202f}\u{2009}\u{2011} — – … « » „ “ ” ‘ ’ ‚ ‹ ›",
    "− × ′ ″ © ® ™ → ← ↔ ≥ ≤ ≠ ±",
    "Скажи \"раз\", а потом \"два\u{2028}три\"",
    "\"a 'b' c\" и «d „e“ f»",
    "и\u{a0}т.\u{a0}д. +7\u{a0}495\u{a0}123\u{2011}45\u{2011}67",
    "",
];

fn typographers() -> Vec<(&'static str, Typographer)> {
    vec![
        ("ru empty", empty(Language::RU_RU)),
        ("ru Interface", ru_ui()),
        ("ru Prose", ru_prose()),
        (
            "en Interface",
            Typographer::new(Language::EN_US, RuleSet::INTERFACE),
        ),
        (
            "en Prose",
            Typographer::new(Language::EN_US, RuleSet::PROSE),
        ),
        ("en ALL", Typographer::new(Language::EN_US, RuleSet::ALL)),
        (
            "COMMON Prose",
            Typographer::new(Language::COMMON, RuleSet::PROSE),
        ),
    ]
}

fn is_nowrap(output: Output) -> bool {
    matches!(output, Output::Html(s) if s != HtmlStyle::NAMED
        && s != HtmlStyle::DECIMAL && s != HtmlStyle::HEX && s != HtmlStyle::CHARACTERS)
}

fn is_css(output: Output) -> bool {
    matches!(output, Output::Escaped(s) if s == EscapeStyle::CSS || s == EscapeStyle::CSS.non_ascii())
}

#[test]
fn the_fixed_set_in_every_kind() {
    for (name, t) in typographers() {
        for input in FIXED {
            let unicode = t.typeset(input, Output::Unicode).into_owned();
            for output in all_outputs() {
                let what = format!("{name} {output:?} {input:?}");
                let out = t.typeset(input, output).into_owned();
                // The reference encoder, for every kind that has no nowrap element.
                if !is_nowrap(output) {
                    assert_eq!(out, encode_reference(output, &unicode), "{what}");
                }
                // Property 17: the safe alphabet of the kind.
                if let Err(e) = check_safe_alphabet(output, &out) {
                    panic!("{what}: {e}\n{out}");
                }
                // Property 12: decoding gives the Unicode result.
                let expected = if is_css(output) {
                    unicode.replace('\u{0}', "\u{fffd}")
                } else {
                    unicode.clone()
                };
                assert_eq!(
                    decode(output, &out).as_deref(),
                    Ok(expected.as_str()),
                    "{what}: {out}"
                );
                // Property 12, second equation.
                let source = if is_css(output) {
                    input.replace('\u{0}', "\u{fffd}")
                } else {
                    (*input).to_owned()
                };
                let again = t.typeset(&source, output).into_owned();
                let decoded = decode(output, &again).unwrap();
                assert_eq!(t.typeset(&decoded, output), again, "{what}");
            }
        }
    }
}

#[test]
fn borrowed_exactly_when_equal_in_the_fixed_set() {
    for (name, t) in typographers() {
        for input in FIXED {
            for output in all_outputs() {
                let got = t.typeset(input, output);
                assert_eq!(
                    matches!(got, Cow::Borrowed(_)),
                    got == *input,
                    "{name} {output:?} {input:?}"
                );
            }
        }
    }
}

#[test]
fn blank_lines_between_paragraphs_with_quotes_on_both_sides() {
    let t = ru_ui();
    // Paired in each paragraph.
    let text = "\"раз\"\n\n\"два\"";
    eq(
        "unicode",
        &t.typeset(text, Output::Unicode),
        "«раз»\n\n«два»",
    );
    eq(
        "html",
        &t.typeset(text, NAMED),
        "&laquo;раз&raquo;\n\n&laquo;два&raquo;",
    );
    eq(
        "js",
        &t.typeset(text, JS),
        "\\u00ABраз\\u00BB\\n\\n\\u00ABдва\\u00BB",
    );
    eq(
        "rust",
        &t.typeset(text, RUST),
        "\\u{ab}раз\\u{bb}\\n\\n\\u{ab}два\\u{bb}",
    );
    eq(
        "css",
        &t.typeset(text, CSS),
        "\\ab раз\\bb \\a \\a \\ab два\\bb ",
    );
    // One unpaired quote in each paragraph: both stay straight and are escaped as quotes.
    let text = "\"раз\n\nдва\"";
    assert_eq!(t.typeset(text, Output::Unicode), text);
    eq("html", &t.typeset(text, NAMED), "&quot;раз\n\nдва&quot;");
    eq("js", &t.typeset(text, JS), "\\\"раз\\n\\nдва\\\"");
    eq("rust", &t.typeset(text, RUST), "\\\"раз\\n\\nдва\\\"");
    eq("css", &t.typeset(text, CSS), "\\\"раз\\a \\a два\\\"");
}

#[test]
fn script_template_and_the_characters_above_the_bmp() {
    let t = empty(Language::RU_RU);
    eq(
        "script in html",
        &t.typeset("<script>alert(\"x\")</script>", NAMED),
        "&lt;script&gt;alert(&quot;x&quot;)&lt;/script&gt;",
    );
    eq(
        "script in JS",
        &t.typeset("<script>alert(\"x\")</script>", JS),
        "\\u003Cscript\\u003Ealert(\\\"x\\\")\\u003C/script\\u003E",
    );
    eq(
        "template literal",
        &t.typeset("${x} `y`", JS),
        "\\u0024{x} \\u0060y\\u0060",
    );
    eq(
        "template literal JSON",
        &t.typeset("${x}", Output::Escaped(EscapeStyle::JSON)),
        "${x}",
    );
    eq(
        "above the BMP, default scope",
        &t.typeset("😀 𝔘", JS),
        "😀 𝔘",
    );
    eq(
        "above the BMP, non_ascii",
        &t.typeset("😀 𝔘", Output::Escaped(EscapeStyle::JS.non_ascii())),
        "\\uD83D\\uDE00 \\uD835\\uDD18",
    );
    eq(
        "above the BMP, code point",
        &t.typeset(
            "😀 𝔘",
            Output::Escaped(EscapeStyle::JS_CODE_POINT.non_ascii()),
        ),
        "\\u{1F600} \\u{1D518}",
    );
    eq(
        "above the BMP, Rust",
        &t.typeset("😀", Output::Escaped(EscapeStyle::RUST.non_ascii())),
        "\\u{1f600}",
    );
    eq("above the BMP, html", &t.typeset("😀", NAMED), "😀");
}

#[test]
fn quote_pairs_set_by_the_caller_are_encoded_like_the_built_in_ones() {
    // Design section 13.2: en with primary `« »` and nested `‹ ›`.
    let t = Typographer::builder(
        Language::EN_US,
        RuleSet::EMPTY
            .with(Rule::QuotesPairs)
            .with(Rule::QuotesApostrophe)
            .with(Rule::QuotesPrimes),
    )
    .quote_pairs(QuotePairs::new(('«', '»'), ('‹', '›')))
    .build()
    .unwrap();
    let text = "\"a \"b\" c\"";
    eq("unicode", &t.typeset(text, Output::Unicode), "«a ‹b› c»");
    eq(
        "named",
        &t.typeset(text, Output::Html(HtmlStyle::NAMED)),
        "&laquo;a &lsaquo;b&rsaquo; c&raquo;",
    );
    eq(
        "JS",
        &t.typeset(text, Output::Escaped(EscapeStyle::JS)),
        "\\u00ABa \\u2039b\\u203A c\\u00BB",
    );
}

#[test]
fn quote_glyphs_a_caller_may_choose_are_in_the_set() {
    // Design section 11.2: the quotes row holds every glyph that design section 7.6 allows, so a
    // caller's pairs are covered: each of the ten quote glyphs is written by name in `NAMED`, as a
    // reference in `DECIMAL` and `HEX`, and as an escape in every syntax.
    let t = empty(Language::EN_US);
    for c in ['«', '»', '„', '“', '”', '‘', '’', '‚', '‹', '›'] {
        let text = c.to_string();
        assert!(matches!(t.typeset(&text, NAMED), Cow::Owned(_)), "{c:?}");
        assert!(name_of(c).is_some(), "{c:?}");
        assert!(matches!(t.typeset(&text, JS), Cow::Owned(_)), "{c:?}");
    }
}

fn open(class_or_style: &str) -> String {
    format!("<span {class_or_style}>")
}

#[test]
fn nowrap_groups_in_text() {
    let class = nowrap();
    let style = Output::Html(HtmlStyle::NAMED.nowrap_style());
    let c = open("class=\"nowrap\"");
    let s = open("style=\"white-space:nowrap\"");
    // Design section 2.8.
    let t = ru_ui();
    eq(
        "class",
        &t.typeset("Цена - 5 руб.", class),
        &format!("{c}Цена &mdash;</span> {c}5 руб.</span>"),
    );
    eq(
        "style",
        &t.typeset("Цена - 5 руб.", style),
        &format!("{s}Цена &mdash;</span> {s}5 руб.</span>"),
    );
    eq(
        "entities",
        &t.typeset("Цена - 5 руб.", NAMED),
        "Цена&nbsp;&mdash; 5&nbsp;руб.",
    );
    // Design section 11.5: `и⍽т.⍽д.`.
    eq(
        "abbreviation",
        &ru_prose().typeset(&vis("и⍽т.⍽д."), class),
        &format!("{c}и т. д.</span>"),
    );
    eq(
        "abbreviation, style",
        &ru_prose().typeset(&vis("и⍽т.⍽д."), style),
        &format!("{s}и т. д.</span>"),
    );
    // A telephone number glued by `nbsp.phone` (Prose), and the same number typed with U+00A0 and
    // U+2011: a joint of the group, written as a reference.
    eq(
        "phone",
        &ru_prose().typeset("+7 495 123-45-67", class),
        &format!("{c}+7 495 123-45-67</span>"),
    );
    eq(
        "typed phone",
        &ru_prose().typeset("+7\u{a0}495\u{a0}123\u{2011}45\u{2011}67", class),
        &format!("{c}+7 495 123&#x2011;45&#x2011;67</span>"),
    );
    // U+202F glue keeps its width: a reference inside the element.
    let narrow = Typographer::builder(Language::RU_RU, RuleSet::PROSE)
        .narrow_glue(NarrowGlue::ABBREVIATIONS)
        .build()
        .unwrap();
    eq(
        "narrow glue",
        &narrow.typeset("и т. д.", Output::Unicode),
        "и\u{a0}т.\u{202f}д.",
    );
    eq(
        "narrow glue in a group",
        &narrow.typeset("и т. д.", class),
        &format!("{c}и т.&#x202F;д.</span>"),
    );
    eq(
        "narrow glue in a group, characters",
        &narrow.typeset(
            "и т. д.",
            Output::Html(HtmlStyle::CHARACTERS.nowrap_class("nowrap")),
        ),
        &format!("{c}и т.\u{202f}д.</span>"),
    );
    eq(
        "narrow glue in a group, hex",
        &narrow.typeset(
            "и т. д.",
            Output::Html(HtmlStyle::HEX.nowrap_class("nowrap")),
        ),
        &format!("{c}и т.&#x202F;д.</span>"),
    );
}

#[test]
fn nowrap_groups_in_the_design_example() {
    // Design section 2.5: the Unicode result is `Файл «report.pdf» не⍽найден⍽— проверьте путь
    // в⍽`--config`.`; the group of three fragments is one element, and the glue before a protected
    // span is an entity (design section 11.5).
    let t = ru_ui();
    let text = "Файл \"report.pdf\" не найден - проверьте путь в `--config`.";
    eq(
        "unicode",
        &t.typeset(text, Output::Unicode),
        &vis("Файл «report.pdf» не⍽найден⍽— проверьте путь в⍽`--config`."),
    );
    eq(
        "nowrap",
        &t.typeset(text, nowrap()),
        "Файл &laquo;report.pdf&raquo; <span class=\"nowrap\">не найден &mdash;</span> проверьте путь в&nbsp;`--config`.",
    );
    eq(
        "entities",
        &t.typeset(text, NAMED),
        "Файл &laquo;report.pdf&raquo; не&nbsp;найден&nbsp;&mdash; проверьте путь в&nbsp;`--config`.",
    );
}

#[test]
fn nowrap_stops_at_a_protected_span() {
    let t = ru_ui();
    // The glue before the span is an entity, and glue inside the span is an entity too.
    eq(
        "before",
        &t.typeset("Запустите в `--config` сейчас", nowrap()),
        "Запустите в&nbsp;`--config` сейчас",
    );
    eq(
        "inside",
        &t.typeset(&vis("Запустите `a⍽b` сейчас"), nowrap()),
        "Запустите `a&nbsp;b` сейчас",
    );
    // A group that needs no element has none: no joint, no tag.
    eq("none", &t.typeset("Сохранить", nowrap()), "Сохранить");
}

#[test]
fn the_class_is_escaped_as_an_attribute_value() {
    let output = Output::Html(HtmlStyle::NAMED.nowrap_class("a\"b<c&d'e>f"));
    eq(
        "class",
        &ru_prose().typeset(&vis("и⍽т.⍽д."), output),
        "<span class=\"a&quot;b&lt;c&amp;d&#39;e&gt;f\">и т. д.</span>",
    );
}

#[test]
fn nowrap_in_templates() {
    let t = ru_ui();
    let c = open("class=\"nowrap\"");
    // Design section 11.5: `в {city}` → texts `["<span class=\"nowrap\">в ", "</span>"]`.
    let out = t.typeset_template(
        [Piece::Text("в "), Piece::Slot("city", Slot::WORD)],
        nowrap(),
    );
    assert_eq!(out.texts(), [format!("{c}в "), "</span>".to_owned()]);
    assert!(out.changed());
    // A group that starts with slot i opens at the end of texts()[i].
    let out = t.typeset_template(
        [Piece::Slot("n", Slot::NUMBER), Piece::Text(" МБ")],
        nowrap(),
    );
    assert_eq!(out.texts(), [c.clone(), " МБ</span>".to_owned()]);
    // The example of design section 2.6.
    let out = t.typeset_template(
        [
            Piece::Text("Файл \""),
            Piece::Slot("name", Slot::WORD),
            Piece::Text("\" весит "),
            Piece::Slot("size", Slot::NUMBER),
            Piece::Text(" МБ - удалить?"),
        ],
        nowrap(),
    );
    assert_eq!(
        out.texts(),
        [
            "Файл &laquo;".to_owned(),
            format!("&raquo; весит {c}"),
            " МБ &mdash;</span> удалить?".to_owned(),
        ]
    );
    // The entity kinds put the same glue in the texts as references.
    let out = t.typeset_template([Piece::Text("в "), Piece::Slot("city", Slot::WORD)], NAMED);
    assert_eq!(out.texts(), ["в&nbsp;", ""]);
}

#[test]
fn a_group_is_split_at_markup_with_a_diagnostic() {
    let t = ru_ui();
    let pieces = || [Piece::Text("в "), Piece::Markup("<b>"), Piece::Text("дом")];
    let out = t.typeset_template::<()>(pieces(), nowrap());
    assert_eq!(out.texts(), ["в&nbsp;<b>дом"]);
    let report = t.report_template::<()>(pieces(), nowrap());
    let fallback: Vec<_> = report
        .diagnostics()
        .iter()
        .filter(|d| d.kind() == &DiagnosticKind::NowrapFallback)
        .collect();
    assert_eq!(fallback.len(), 1, "{:?}", report.diagnostics());
    assert_eq!(fallback[0].severity(), Severity::Info);
    // At the `Markup` piece, with its whole range (design section 11.5).
    assert_eq!((fallback[0].piece(), fallback[0].range()), (1, 0..3));
    // A group with its joint on the left of the markup only is split there too.
    let pieces_left = [
        Piece::Text("а\u{a0}б"),
        Piece::Markup("<i>"),
        Piece::Text("в"),
    ];
    let report = t.report_template::<()>(pieces_left, nowrap());
    let fallback: Vec<_> = report
        .diagnostics()
        .iter()
        .filter(|d| d.kind() == &DiagnosticKind::NowrapFallback)
        .map(|d| (d.piece(), d.range()))
        .collect();
    assert_eq!(fallback, [(1, 0..3)], "{:?}", report.diagnostics());
    // No fallback where nothing is split, and none in the other kinds.
    let report = t.report_template::<()>(pieces(), NAMED);
    assert!(
        report
            .diagnostics()
            .iter()
            .all(|d| d.kind() != &DiagnosticKind::NowrapFallback)
    );
    let report = t.report("в дом", nowrap());
    assert!(
        report
            .diagnostics()
            .iter()
            .all(|d| d.kind() != &DiagnosticKind::NowrapFallback)
    );
}

#[test]
fn templates_encode_texts_protected_pieces_and_markup() {
    let t = empty(Language::EN_US);
    // Markup: written as is in Unicode and HTML output, escaped like text in escaped output.
    let pieces = || {
        [
            Piece::Text("a & b "),
            Piece::Markup("<b>"),
            Piece::Protected("c<d"),
            Piece::Slot(0u8, Slot::WORD),
            Piece::Text(" \"e\""),
        ]
    };
    let out = t.typeset_template(pieces(), Output::Unicode);
    assert_eq!(out.texts(), ["a & b <b>c<d", " \"e\""]);
    assert!(!out.changed());
    let out = t.typeset_template(pieces(), NAMED);
    assert_eq!(out.texts(), ["a &amp; b <b>c&lt;d", " &quot;e&quot;"]);
    assert!(out.changed());
    let out = t.typeset_template(pieces(), JS);
    assert_eq!(
        out.texts(),
        ["a \\u0026 b \\u003Cb\\u003Ec\\u003Cd", " \\\"e\\\""]
    );
    let out = t.typeset_template(pieces(), RUST);
    assert_eq!(out.texts(), ["a & b <b>c<d", " \\\"e\\\""]);
    // Slots are never touched: they come out with their payload and kind.
    assert_eq!(out.slots()[0].0, 0u8);
}

#[test]
fn a_template_text_is_changed_when_the_encoding_changed_it() {
    let t = empty(Language::RU_RU);
    let out = t.typeset_template::<()>([Piece::Text("a & b")], Output::Unicode);
    assert!(!out.changed());
    assert!(matches!(out.texts()[0], Cow::Borrowed("a & b")));
    let out = t.typeset_template::<()>([Piece::Text("a & b")], NAMED);
    assert!(out.changed());
    assert_eq!(out.texts(), ["a &amp; b"]);
    // Nothing to encode: borrowed with the input lifetime, not changed. With `non_ascii()` the
    // Cyrillic is encoded (design section 11.6), so the text is changed and owned.
    for output in all_outputs() {
        let out = t.typeset_template::<()>([Piece::Text("Сохранить")], output);
        let non_ascii = matches!(output, Output::Escaped(s) if s == s.non_ascii());
        assert_eq!(out.changed(), non_ascii, "{output:?}");
        assert_eq!(
            matches!(out.texts()[0], Cow::Borrowed("Сохранить")),
            !non_ascii,
            "{output:?}"
        );
    }
    // Merging adjacent pieces is not a change; markup in HTML is written as it is.
    let out = t.typeset_template::<()>([Piece::Text("a"), Piece::Markup("<b>")], NAMED);
    assert_eq!(out.texts(), ["a<b>"]);
    assert!(!out.changed());
    let out = t.typeset_template::<()>([Piece::Text("a"), Piece::Markup("<b>")], JS);
    assert_eq!(out.texts(), ["a\\u003Cb\\u003E"]);
    assert!(out.changed());
}

#[test]
fn the_filled_template_is_the_encoded_text() {
    // Design section 11.1: values are inserted as they are, so a value for an HTML template is
    // HTML-escaped by the caller; the number formatted with the same output is already encoded.
    let t = ru_ui();
    let out = t.typeset_template(
        [Piece::Slot("n", Slot::NUMBER), Piece::Text(" руб. & ещё")],
        NAMED,
    );
    let n = t.format_number(-12345, NAMED).to_string();
    let filled = out.fill(|_| Cow::Owned(n.clone()));
    assert_eq!(filled, "&minus;12&nbsp;345&nbsp;руб. &amp; ещё");
    let out = t.typeset_template(
        [Piece::Slot("n", Slot::NUMBER), Piece::Text(" руб.")],
        Output::Escaped(EscapeStyle::RUST),
    );
    let n = t.format_number(1000, Output::Unicode).to_string();
    let filled = out.fill(|_| Cow::Owned(n.clone()));
    assert_eq!(filled, "1\u{a0}000\\u{a0}руб.");
}

#[test]
fn a_dollar_before_a_slot_or_at_the_end_is_escaped() {
    // Design section 11.6: the character after a `$` that ends a text is not known (a slot value
    // or the text after a slot may start with `{`), so the JavaScript styles escape it and no
    // `${` can form across a slot boundary.
    let t = Typographer::new(Language::EN_US, RuleSet::PROSE);
    let js = [
        (EscapeStyle::JS, "price: \\u0024"),
        (EscapeStyle::JS_HEX, "price: \\x24"),
        (EscapeStyle::JS_CODE_POINT, "price: \\u{24}"),
    ];
    for (style, expected) in js {
        let output = Output::Escaped(style);
        let out = t.typeset_template(
            [Piece::Text("price: $"), Piece::Slot(0u8, Slot::OTHER)],
            output,
        );
        assert_eq!(out.texts(), [expected, ""], "{style:?}");
        let filled = out.fill(|_| "{alert(1)}".into());
        assert!(!filled.contains("${"), "{style:?}: {filled}");
        let value = t.typeset("{alert(1)}", output).into_owned();
        assert!(
            !out.fill(|_| value.as_str().into()).contains("${"),
            "{style:?}"
        );
        // The mirror case: the value ends in `$`, the text after the slot starts with `{`.
        let out = t.typeset_template([Piece::Slot(0u8, Slot::OTHER), Piece::Text("{x}")], output);
        let value = t.typeset("$", output).into_owned();
        let filled = out.fill(|_| value.as_str().into());
        assert!(!filled.contains("${"), "{style:?}: {filled}");
    }
    for style in [EscapeStyle::JSON, EscapeStyle::RUST] {
        let out = t.typeset_template(
            [Piece::Text("price: $"), Piece::Slot(0u8, Slot::OTHER)],
            Output::Escaped(style),
        );
        assert_eq!(out.texts()[0], "price: $", "{style:?}");
    }
}

#[test]
fn typeset_into_leaves_the_text_already_in_the_buffer_to_the_caller() {
    // Design section 11.1: a leading `{` stays raw; the caller keeps a raw `$` out of the end of
    // `buf`.
    let t = Typographer::new(Language::EN_US, RuleSet::PROSE);
    let mut buf = String::from("a");
    t.typeset_into("{x}", JS, &mut buf);
    assert_eq!(buf, "a{x}");
}

#[test]
fn typeset_with_and_typeset_into_write_the_kind() {
    let t = ru_ui();
    let mut scratch = Scratch::new();
    let text = "Цена - 5 руб. & <ещё>";
    for output in all_outputs() {
        let a = t.typeset(text, output).into_owned();
        let b = t.typeset_with(&mut scratch, text, output).into_owned();
        let mut buf = String::from("> ");
        t.typeset_into(text, output, &mut buf);
        assert_eq!(a, b, "{output:?}");
        assert_eq!(buf, format!("> {a}"), "{output:?}");
        assert_eq!(t.report(text, output).result().to_string(), a, "{output:?}");
    }
    eq(
        "html",
        &t.typeset(text, NAMED),
        "Цена&nbsp;&mdash; 5&nbsp;руб. &amp; &lt;ещё&gt;",
    );
}

#[test]
fn unicode_never_encodes() {
    let t = empty(Language::RU_RU);
    let text = "a & b < c > \"d\" 'e' \\ ${x} `y` </script> \u{a0}";
    let out = t.typeset(text, Output::Unicode);
    assert!(matches!(out, Cow::Borrowed(_)));
    assert_eq!(out, text);
}

// The typographic set against the extract of the WHATWG list (design sections 11.2 and 13.2). The
// test is skipped when the file is absent: it is excluded from the published package.
const FIXTURE: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/tests/fixtures/whatwg-entities.txt"
);

/// The entries of the extract: `(name, code points)`, names without `&` and `;`.
fn fixture() -> Option<Vec<(String, Vec<u32>)>> {
    let text = std::fs::read_to_string(FIXTURE).ok()?;
    let mut entries = Vec::new();
    for line in text.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let name = line
            .split('"')
            .nth(1)
            .unwrap_or_else(|| panic!("no name: {line}"));
        let name = name.trim_start_matches('&').trim_end_matches(';');
        let list = line
            .split_once("\"codepoints\": [")
            .and_then(|(_, rest)| rest.split_once(']'))
            .map(|(list, _)| list)
            .unwrap_or_else(|| panic!("no code points: {line}"));
        let cps = list.split(',').map(|n| n.trim().parse().unwrap()).collect();
        entries.push((name.to_owned(), cps));
    }
    Some(entries)
}

#[test]
fn the_extract_has_its_header_and_the_entries() {
    let Ok(text) = std::fs::read_to_string(FIXTURE) else {
        return;
    };
    // The header names the source, the date and the license (design section 13.2).
    let header: String = text
        .lines()
        .take_while(|l| l.starts_with('#'))
        .collect::<Vec<_>>()
        .join("\n");
    for needle in [
        "whatwg.org",
        "2026-11-21",
        "CC BY 4.0",
        "Creative Commons Attribution 4.0",
    ] {
        assert!(header.contains(needle), "the header lacks {needle}");
    }
    let entries = fixture().unwrap();
    assert!(entries.len() > 50);
    assert!(entries.iter().any(|(n, c)| n == "nbsp" && c == &[160]));
}

#[test]
fn the_table_agrees_with_the_whatwg_extract() {
    let Some(entries) = fixture() else {
        return;
    };
    let t = empty(Language::RU_RU);
    for (c, named) in SET {
        let cp = u32::from(*c);
        let names: Vec<&str> = entries
            .iter()
            .filter(|(_, cps)| cps == &[cp])
            .map(|(n, _)| n.as_str())
            .collect();
        let written = t.typeset(&c.to_string(), NAMED).into_owned();
        assert_eq!(&written, named, "{c:?}");
        match name_of(*c) {
            Some(name) => {
                // Every name the table writes exists there with that code point.
                assert!(
                    names.contains(&name),
                    "&{name}; is not an entity of U+{cp:04X} (extract: {names:?})"
                );
                assert_eq!(written, format!("&{name};"));
            }
            None => {
                // U+202F and U+2011 have no name, and the extract has none for them.
                assert!(matches!(cp, 0x202f | 0x2011), "{c:?}");
                assert!(
                    names.is_empty(),
                    "U+{cp:04X} has a name in the extract: {names:?}"
                );
                assert!(
                    written.starts_with("&#x") && written.ends_with(';'),
                    "{written}"
                );
            }
        }
        // The test fails when NAMED writes a numeric reference for a character that has a name.
        if !names.is_empty() {
            assert!(
                names.iter().any(|n| written == format!("&{n};")),
                "NAMED writes {written} for U+{cp:04X}, which has the names {names:?}"
            );
        }
    }
    // The references of the markup characters exist too.
    for (c, name) in [('&', "amp"), ('<', "lt"), ('>', "gt"), ('"', "quot")] {
        let cp = u32::from(c);
        assert!(
            entries.iter().any(|(n, cps)| n == name && cps == &[cp]),
            "&{name}; is not in the extract"
        );
        assert_eq!(t.typeset(&c.to_string(), NAMED), format!("&{name};"));
    }
    // `'` is written as a numeric reference, valid in every HTML version.
    assert_eq!(t.typeset("'", NAMED), "&#39;");
}
