//! Self-tests of the helpers of `tests/common/` (design sections 13.1 and 13.7).

#![allow(clippy::unwrap_used, clippy::panic, clippy::indexing_slicing)]

mod common;

use common::{TmplPiece, assert_no_markers, parse_tmpl, show, vis};
use typolish::SlotKind;

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
