//! Tokens: class-invariant tokens over grapheme clusters (design section 3.5).
//!
//! A token holds no reference: its text is the source slice at `src`, unless a rule replaced it,
//! and then `repl` names the replacement. So `Tok` is `'static`, and the vectors that hold tokens
//! live in a `Scratch` and in a thread-local without `unsafe` (design section 2.12).

#![expect(dead_code, reason = "the lexer of stage 2 builds tokens")]

use crate::SlotKind;

/// One token.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Tok {
    pub(crate) kind: Kind,
    pub(crate) src: SrcSpan,
    pub(crate) repl: Repl,
    pub(crate) glue: Glue,
    pub(crate) flags: Flags,
}

/// The source of a token: first piece and byte offset, last piece and byte offset. A token may
/// span adjacent `Text` pieces; for string input both pieces are 0. An inserted `Space` token has
/// an empty span at the insertion point.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct SrcSpan {
    pub(crate) start_piece: u32,
    pub(crate) start: u32,
    pub(crate) end_piece: u32,
    pub(crate) end: u32,
}

/// The current text of a token.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Repl {
    /// The source slice at `src`.
    Source,
    /// One character that a rule wrote.
    Char(char),
    /// A form of the language's ellipsis data, which the renderer reads from the language.
    Ellipsis(EllipsisForm),
}

/// The forms of the language's ellipsis data (`?..` or `?…` and the others).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum EllipsisForm {
    Question,
    Exclamation,
    Both,
}

/// The glue mark of a `Space` token (design section 3.9).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Glue {
    None,
    NoBreak,
    Narrow,
}

/// Token flags: `FROZEN`, `LABEL`, `CHANGED`, `INSERTED`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub(crate) struct Flags(u8);

impl Flags {
    pub(crate) const FROZEN: Flags = Flags(1);
    pub(crate) const LABEL: Flags = Flags(1 << 1);
    pub(crate) const CHANGED: Flags = Flags(1 << 2);
    pub(crate) const INSERTED: Flags = Flags(1 << 3);

    pub(crate) const fn contains(self, other: Flags) -> bool {
        self.0 & other.0 == other.0
    }

    pub(crate) const fn with(self, other: Flags) -> Flags {
        Flags(self.0 | other.0)
    }
}

/// The script of a word token.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum WordScript {
    Cyrillic,
    Latin,
    Greek,
    Other,
    Mixed,
}

/// The kind of a token; a rule edit never changes it (invariant I1, design section 4.3).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Kind {
    /// Letters with their combining marks.
    Word(WordScript),
    /// A run of ASCII digits.
    Number,
    /// A run of horizontal whitespace.
    Space,
    /// `\n`, `\r\n`, `\r`, U+0085, U+2028.
    Newline,
    /// Format characters (design section 5.4).
    Format,
    /// `"` `'` `«` `»` `„` `“` `”` `‘` `’` `‚` `‹` `›` `′` `″`.
    Quote,
    /// A run of `-`, and the typographic dashes and the minus.
    Dash,
    /// A run of `.`, and `…`.
    Dots,
    /// An operator in its plain or typographic form.
    Operator(Op),
    /// A legal mark in its plain or typographic form.
    LegalMark(Mark),
    /// `x` between numbers, and `×`.
    Times,
    /// `(`, `[`, `{`.
    Open(char),
    /// `)`, `]`, `}`.
    Close(char),
    /// Other punctuation and signs.
    Punct(char),
    /// Any other cluster.
    Symbol,
    /// An opaque placeholder.
    Slot(SlotKind),
    /// Protected text.
    Protected,
    /// Zero-width, transparent markup.
    Markup,
}

/// Operators, the same id for both forms.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Op {
    ArrowRight,
    ArrowLeft,
    ArrowBoth,
    GreaterEqual,
    LessEqual,
    NotEqual,
    PlusMinus,
    DoubleArrow,
    ShiftRight,
    ShiftLeft,
    Equal,
}

/// Legal marks, the same id for both forms.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Mark {
    Copyright,
    Registered,
    Trademark,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tokens_are_small_and_static() {
        fn is_static<T: 'static + Send + Sync + Copy>() {}
        is_static::<Tok>();
        // About 40 bytes per token (design section 3.12).
        assert!(core::mem::size_of::<Tok>() <= 40);
        let flags = Flags::default().with(Flags::FROZEN);
        assert!(flags.contains(Flags::FROZEN));
        assert!(!flags.contains(Flags::LABEL));
    }

    #[test]
    fn clusters_come_from_the_unicode_17_tables() {
        use unicode_segmentation::UnicodeSegmentation;

        // Design sections 5.1 and 14.1: extended grapheme clusters of Unicode 17.0.0.
        assert_eq!(unicode_segmentation::UNICODE_VERSION, (17, 0, 0));
        let clusters: Vec<&str> = "е\u{301}ж👍🏽".graphemes(true).collect();
        assert_eq!(clusters, ["е\u{301}", "ж", "👍🏽"]);
    }
}
