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
    TmplPiece, assert_no_markers, configs, interface, only, parse_tmpl, prose, row, show, vis,
    without_rule,
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
    let r = row("neg.flags.ru.395", "--force", "--force");
    assert_eq!(
        (r.id, r.input, r.recorded),
        ("neg.flags.ru.395", "--force", "--force")
    );
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
fn the_set_helpers_drop_the_nbsp_rules_and_the_named_rule() {
    for t in [prose(Language::RU_RU), interface(Language::RU_RU)] {
        assert!(t.rules().iter().all(|r| !r.id().starts_with("nbsp.")));
    }
    assert!(prose(Language::EN_US).rules().contains(Rule::QuotesPairs));
    assert!(
        !interface(Language::EN_US)
            .rules()
            .contains(Rule::RangeDigits)
    );
    let t = without_rule(Language::RU_RU, RuleSet::PROSE, Rule::DashDoubleHyphen);
    assert!(!t.rules().contains(Rule::DashDoubleHyphen));
    assert!(t.rules().contains(Rule::DashSpacedHyphen));
    assert!(t.rules().iter().all(|r| !r.id().starts_with("nbsp.")));
}

#[test]
fn configs_name_the_rule_alone_and_the_sets_that_hold_it() {
    // `ellipsis.dots` is in both sets, `range.digits` in Prose only, `nbsp.dash` is removed.
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
