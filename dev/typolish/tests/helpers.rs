//! Self-tests of the helpers of `tests/common/` (design sections 13.1 and 13.7).

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

use common::{
    Group, Reason, TmplPiece, agree, assert_no_markers, configs, diverge, expected_in_set,
    interface, only, parse_tmpl, prose, show, vis, without_nbsp, without_rule,
};
use typolish::{Language, Rule, RuleSet, SlotKind};

#[test]
fn vis_writes_glue_characters() {
    assert_eq!(vis("5⍽МБ"), "5\u{a0}МБ");
    assert_eq!(vis("5⎵МБ"), "5\u{202f}МБ");
    assert_eq!(vis("a b"), "a b");
    assert_eq!(vis("⍽⎵⍽"), "\u{a0}\u{202f}\u{a0}");
}

#[test]
fn show_is_the_inverse_of_vis() {
    for text in ["5⍽МБ", "№⎵5", "plain", ""] {
        assert_eq!(show(&vis(text)), text);
    }
}

#[test]
fn markers_are_detected() {
    assert_no_markers("5\u{a0}МБ");
    assert_no_markers(&vis("5⍽МБ"));
    assert!(std::panic::catch_unwind(|| assert_no_markers("5⍽МБ")).is_err());
    assert!(std::panic::catch_unwind(|| assert_no_markers("5⎵МБ")).is_err());
}

#[test]
fn tmpl_lines_become_pieces() {
    let messages = parse_tmpl(
        "# comment\n\nfile.size = Файл «{name:word}» весит {size:number}⍽МБ{x}\nplain = Hello\nonly = {d:date}\n",
    );
    assert_eq!(messages.len(), 3);
    assert_eq!(messages[0].id, "file.size");
    assert_eq!(
        messages[0].pieces,
        [
            TmplPiece::Text("Файл «".to_owned()),
            TmplPiece::Slot("name".to_owned(), SlotKind::Word),
            TmplPiece::Text("» весит ".to_owned()),
            TmplPiece::Slot("size".to_owned(), SlotKind::Number),
            TmplPiece::Text("\u{a0}МБ".to_owned()),
            TmplPiece::Slot("x".to_owned(), SlotKind::Other),
        ]
    );
    assert_eq!(messages[0].flat(), "Файл «{name}» весит {size}\u{a0}МБ{x}");
    assert_eq!(messages[1].pieces, [TmplPiece::Text("Hello".to_owned())]);
    assert_eq!(
        messages[2].pieces,
        [TmplPiece::Slot("d".to_owned(), SlotKind::Date)]
    );
    assert_eq!(messages[0].pieces().len(), 6);
}

#[test]
fn rows_of_the_reference_tables_hold_their_parts() {
    let r = agree("neg.flags.ru.395", "--force", "--force");
    assert_eq!(
        (r.id, r.input, r.recorded),
        ("neg.flags.ru.395", "--force", "--force")
    );
    assert!(matches!(r.group, Group::Agree));
    let r = diverge("host.ru.1", "a:b", "a: b", "a:b", &[Reason::NoSpaceEdit]);
    assert!(matches!(
        r.group,
        Group::Diverge {
            ours: "a:b",
            reasons: [Reason::NoSpaceEdit]
        }
    ));
}

#[test]
fn only_runs_exactly_the_given_rules() {
    let t = only(Language::RU_RU, &[Rule::DashDialogue, Rule::EllipsisDots]);
    assert_eq!(
        t.rules(),
        RuleSet::EMPTY
            .with(Rule::DashDialogue)
            .with(Rule::EllipsisDots)
    );
    assert_eq!(only(Language::EN_US, &[]).rules(), RuleSet::EMPTY);
}

#[test]
fn the_set_helpers_give_the_full_sets_and_drop_the_named_rule() {
    // Stage 3: `prose` and `interface` are the built-in sets in full.
    assert_eq!(prose(Language::RU_RU).rules(), RuleSet::PROSE);
    assert_eq!(interface(Language::RU_RU).rules(), RuleSet::INTERFACE);
    assert!(prose(Language::EN_US).rules().contains(Rule::QuotesPairs));
    assert!(
        prose(Language::EN_US)
            .rules()
            .contains(Rule::NbspShortLength)
    );
    assert!(
        !interface(Language::EN_US)
            .rules()
            .contains(Rule::RangeDigits)
    );
    let t = without_rule(Language::RU_RU, RuleSet::PROSE, Rule::DashDoubleHyphen);
    assert!(!t.rules().contains(Rule::DashDoubleHyphen));
    assert!(t.rules().contains(Rule::DashSpacedHyphen));
    assert!(t.rules().contains(Rule::NbspDash));
    assert_eq!(t.rules(), RuleSet::PROSE.without(Rule::DashDoubleHyphen));
}

#[test]
fn without_nbsp_drops_the_nbsp_rules_only() {
    for set in [RuleSet::PROSE, RuleSet::INTERFACE] {
        let t = without_nbsp(Language::RU_RU, set);
        assert!(t.rules().iter().all(|r| !r.id().starts_with("nbsp.")));
        assert!(t.rules().contains(Rule::QuotesPairs));
    }
}

#[test]
fn the_glue_of_a_set_row_comes_from_the_table_of_set_glue() {
    // A row of a stage 2 table is completed with the glue of the full set (section 15.4).
    let prose = prose(Language::RU_RU);
    let interface = interface(Language::RU_RU);
    let alone = only(Language::RU_RU, &[Rule::QuotesPairs]);
    let source = vis("Он сказал \"привет\" и ушёл.");
    let plain = vis("Он сказал «привет» и ушёл.");
    assert_eq!(
        expected_in_set(&prose, &source, plain.clone()),
        vis("Он⍽сказал «привет» и⍽ушёл.")
    );
    assert_eq!(
        expected_in_set(&interface, &source, plain.clone()),
        vis("Он сказал «привет» и⍽ушёл.")
    );
    // A typographer without the short-word rule and a row that the table does not list keep the
    // expectation as written.
    assert_eq!(expected_in_set(&alone, &source, plain.clone()), plain);
    assert_eq!(expected_in_set(&prose, "x", "y".to_owned()), "y".to_owned());
}

#[test]
fn configs_name_the_rule_alone_and_the_sets_that_hold_it() {
    // `ellipsis.dots` is in both sets, `range.digits` in Prose only.
    assert_eq!(configs(&Language::RU_RU, Rule::EllipsisDots).len(), 3);
    assert_eq!(configs(&Language::RU_RU, Rule::RangeDigits).len(), 2);
    let names: Vec<_> = configs(&Language::EN_US, Rule::RangeDigits)
        .into_iter()
        .map(|c| c.0)
        .collect();
    assert!(
        names[0].contains("alone") && names[1].contains("Prose"),
        "{names:?}"
    );
    let (_, alone) = &configs(&Language::EN_US, Rule::QuotesPairs)[0];
    assert_eq!(alone.rules(), RuleSet::EMPTY.with(Rule::QuotesPairs));
}

// The decoders and the reference encoder of `tests/common/decode.rs` (design sections 11.8 and
// 13.5): they are the oracle of the output tests, so each is checked on rows written from the
// tables of sections 11.2, 11.4 and 11.6.

#[test]
fn the_html_decoder_reads_references_and_elements() {
    use common::decode::decode_html;
    assert_eq!(
        decode_html("a &amp; b &lt;c&gt; &quot;d&quot; &#39;e&#39;").unwrap(),
        "a & b <c> \"d\" 'e'"
    );
    assert_eq!(
        decode_html("5&nbsp;МБ &mdash; &#x202F;&#8212;&#xA0;").unwrap(),
        "5\u{a0}МБ — \u{202f}—\u{a0}"
    );
    assert_eq!(
        decode_html("<span class=\"nowrap\">и т.&#x202F;д.</span> и").unwrap(),
        "и\u{a0}т.\u{202f}д. и"
    );
    assert_eq!(
        decode_html("<span style=\"white-space:nowrap\">a b</span>").unwrap(),
        "a\u{a0}b"
    );
    assert_eq!(
        decode_html("<span class=\"a&quot;b\">x</span>").unwrap(),
        "x"
    );
    for bad in [
        "a < b",
        "a > b",
        "a \" b",
        "a ' b",
        "a & b",
        "&unknown;",
        "&nbsp",
        "<span class=\"n\">a",
        "a</span>",
        "<span class=\"n\"><span class=\"n\">a</span></span>",
        "<b>x</b>",
        "&#xD800;",
    ] {
        assert!(decode_html(bad).is_err(), "{bad}");
    }
}

#[test]
fn the_escape_decoders_read_each_syntax_strictly() {
    use common::decode::decode_escaped;
    use typolish::EscapeStyle;
    let ok = |style: EscapeStyle, body: &str, text: &str| {
        assert_eq!(decode_escaped(style, body).as_deref(), Ok(text), "{body}");
    };
    let err = |style: EscapeStyle, body: &str| {
        assert!(decode_escaped(style, body).is_err(), "{body}");
    };
    ok(
        EscapeStyle::JS,
        "a\\u00A0\\\\\\\"\\u0027\\n\\uD83D\\uDE00",
        "a\u{a0}\\\"'\n😀",
    );
    err(EscapeStyle::JS, "\\x41");
    err(EscapeStyle::JS, "\\u{41}");
    err(EscapeStyle::JS, "\\uD83D");
    err(EscapeStyle::JS, "'");
    err(EscapeStyle::JS, "`");
    err(EscapeStyle::JS, "<");
    err(EscapeStyle::JS, "a${b");
    ok(EscapeStyle::JS, "a$b", "a$b");
    err(EscapeStyle::JS, "\u{2028}");
    err(EscapeStyle::JS, "\u{0}");
    ok(EscapeStyle::JS_HEX, "\\xA0\\u2014\\x00", "\u{a0}—\u{0}");
    ok(
        EscapeStyle::JS_CODE_POINT,
        "\\u{A0}\\u{1F600}\\u{0}",
        "\u{a0}😀\u{0}",
    );
    err(EscapeStyle::JS_CODE_POINT, "\\u00A0");
    err(EscapeStyle::JS_CODE_POINT, "\\u{D800}");
    ok(
        EscapeStyle::JSON,
        "\\u00a0\\\"\\/\\b\\f\\ud83d\\ude00'`$",
        "\u{a0}\"/\u{8}\u{c}😀'`$",
    );
    err(EscapeStyle::JSON, "\\'");
    err(EscapeStyle::JSON, "&");
    ok(
        EscapeStyle::RUST,
        "\\u{a0}\\0\\u{1}\\\"'`&<",
        "\u{a0}\0\u{1}\"'`&<",
    );
    err(EscapeStyle::RUST, "\\u00a0");
    err(EscapeStyle::RUST, "\"");
    err(EscapeStyle::RUST, "\n");
    ok(
        EscapeStyle::CSS,
        "\\a0 \\fffd \\a \\\\\\'\\\"&`",
        "\u{a0}\u{fffd}\n\\'\"&`",
    );
    err(EscapeStyle::CSS, "\\a0");
    err(EscapeStyle::CSS, "\\n");
    err(EscapeStyle::CSS, "'");
    err(EscapeStyle::CSS, "<");
    ok(EscapeStyle::CSS, "\u{85}\u{2028}", "\u{85}\u{2028}");
    err(EscapeStyle::JS.non_ascii(), "Да");
    ok(EscapeStyle::JS.non_ascii(), "\\u0414\\u0430", "Да");
}

#[test]
fn the_reference_encoder_agrees_with_the_decoders() {
    use common::decode::{all_outputs, decode, encode_reference, writes_characters};
    use typolish::Output;
    let text = "a & b <c> \"d\" 'e' `f` ${g} \\ \n\r\t\u{0}\u{1}\u{7f}\u{85}\u{2028}\u{2029} «»„“”‘’‚‹› – — … − × ′ ″ © ® ™ → ← ↔ ≥ ≤ ≠ ± \u{a0}\u{202f}\u{2009}\u{2011} Да 😀";
    for output in all_outputs() {
        if matches!(output, Output::Html(s) if s != typolish::HtmlStyle::NAMED && s != typolish::HtmlStyle::DECIMAL && s != typolish::HtmlStyle::HEX && s != typolish::HtmlStyle::CHARACTERS)
        {
            continue;
        }
        let encoded = encode_reference(output, text);
        let expected = if matches!(output, Output::Escaped(s) if s == typolish::EscapeStyle::CSS || s == typolish::EscapeStyle::CSS.non_ascii())
        {
            text.replace('\u{0}', "\u{fffd}")
        } else {
            text.to_owned()
        };
        assert_eq!(
            decode(output, &encoded).as_deref(),
            Ok(expected.as_str()),
            "{output:?}"
        );
        if let Output::Html(style) = output {
            assert_eq!(
                writes_characters(style),
                style == typolish::HtmlStyle::CHARACTERS
            );
        }
    }
}
