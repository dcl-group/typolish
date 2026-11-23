//! The lexer: class-invariant tokens over grapheme clusters (design sections 3.5 and 5).
//!
//! A token holds no reference: its text is the source slice at `src` of the call's text, unless a
//! rule replaced it, and then `repl` names the replacement. So `Tok` is `'static`, and the vectors
//! that hold tokens live in a `Scratch` and in a thread-local without `unsafe` (design section
//! 2.12).

use unicode_segmentation::UnicodeSegmentation;

use super::ingest::Atom;
use super::window::{is_horizontal_space, is_line_break};
use crate::SlotKind;

/// One token.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Tok {
    pub(crate) kind: Kind,
    pub(crate) src: SrcSpan,
    /// The input piece of a token made from a slot, a protected piece or markup; `NO_PIECE` for
    /// a token of the call's text, whose pieces come from its range.
    pub(crate) piece: u32,
    pub(crate) repl: Repl,
    pub(crate) glue: Glue,
    pub(crate) flags: Flags,
    /// The index of the token's fragment in the classification of the pass (design section 3.6);
    /// `NO_FRAG` for `Space` and `Newline` tokens, which belong to no fragment.
    pub(crate) frag: u32,
}

/// `Tok::piece` of a token of the call's text.
pub(crate) const NO_PIECE: u32 = u32::MAX;

/// `Tok::frag` of a token outside every fragment.
pub(crate) const NO_FRAG: u32 = u32::MAX;

impl Tok {
    fn text(kind: Kind, start: u32, end: u32) -> Tok {
        Tok {
            kind,
            src: SrcSpan { start, end },
            piece: NO_PIECE,
            repl: Repl::Source,
            glue: Glue::None,
            flags: Flags::default(),
            frag: NO_FRAG,
        }
    }

    /// An inserted `Space` token carrying glue, with an empty range at `at` (design section 3.5).
    pub(crate) fn inserted(at: u32, glue: Glue) -> Tok {
        let mut tok = Tok::text(Kind::Space, at, at);
        tok.glue = glue;
        tok.flags = Flags::INSERTED;
        tok
    }

    /// Whether the token is `FROZEN`.
    pub(crate) fn is_frozen(&self) -> bool {
        self.flags.contains(Flags::FROZEN)
    }
}

/// The source of a token: a byte range of the call's text (design section 3.4). A token may span
/// adjacent `Text` pieces. A token made from a slot, a protected piece or markup has an empty
/// range at its position; an inserted `Space` token has an empty range at the insertion point.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct SrcSpan {
    pub(crate) start: u32,
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

/// Token flags: `FROZEN`, `LABEL`, `CHANGED`, `INSERTED`, and `CORE` for a token of the core of
/// its fragment (design section 3.6).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub(crate) struct Flags(u8);

impl Flags {
    pub(crate) const FROZEN: Flags = Flags(1);
    pub(crate) const LABEL: Flags = Flags(1 << 1);
    pub(crate) const CHANGED: Flags = Flags(1 << 2);
    pub(crate) const INSERTED: Flags = Flags(1 << 3);
    pub(crate) const CORE: Flags = Flags(1 << 4);

    pub(crate) const fn contains(self, other: Flags) -> bool {
        self.0 & other.0 == other.0
    }

    pub(crate) const fn with(self, other: Flags) -> Flags {
        Flags(self.0 | other.0)
    }
}

/// The script of a word token (design section 5.2).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum WordScript {
    Cyrillic,
    Latin,
    Greek,
    Other,
    Mixed,
}

/// The kind of a token; a rule edit never changes it (invariant I1, design section 4.3). The
/// classifier alone turns a `Word` token `x` between numbers into `Times` (design section 3.6).
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
    /// Protected text: a span between delimiters or a protected piece.
    Protected,
    /// Zero-width, transparent markup.
    Markup,
}

impl Kind {
    /// Opaque tokens: slots and protected text (design section 3.4).
    pub(crate) const fn is_opaque(self) -> bool {
        matches!(self, Kind::Slot(_) | Kind::Protected)
    }

    /// Tokens that rules and the classifier look through (design sections 3.4 and 5.4).
    pub(crate) const fn is_transparent(self) -> bool {
        matches!(self, Kind::Format | Kind::Markup)
    }
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

/// The plain forms of operators, lexed whole (design section 3.5).
const PLAIN_OPERATORS: [(&str, Op); 11] = [
    ("->", Op::ArrowRight),
    ("<-", Op::ArrowLeft),
    ("<->", Op::ArrowBoth),
    (">=", Op::GreaterEqual),
    ("<=", Op::LessEqual),
    ("!=", Op::NotEqual),
    ("+-", Op::PlusMinus),
    ("=>", Op::DoubleArrow),
    (">>", Op::ShiftRight),
    ("<<", Op::ShiftLeft),
    ("==", Op::Equal),
];

/// The characters of the plain operators.
const fn is_operator_byte(b: u8) -> bool {
    matches!(b, b'-' | b'<' | b'>' | b'=' | b'!' | b'+')
}

/// Legal marks, the same id for both forms.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Mark {
    Copyright,
    Registered,
    Trademark,
}

/// Format characters that form a cluster of their own (design section 5.4); ZWNJ and ZWJ are
/// handled apart.
pub(crate) const fn is_format_char(c: char) -> bool {
    matches!(
        c,
        '\u{ad}'
            | '\u{200b}'
            | '\u{200e}'
            | '\u{200f}'
            | '\u{202a}'..='\u{202e}'
            | '\u{2060}'..='\u{2064}'
            | '\u{2066}'..='\u{2069}'
            | '\u{feff}'
    )
}

/// ZWNJ or ZWJ.
pub(crate) const fn is_joiner(c: char) -> bool {
    matches!(c, '\u{200c}' | '\u{200d}')
}

/// The characters of `Kind::Quote`.
pub(crate) const fn is_quote_char(c: char) -> bool {
    matches!(
        c,
        '"' | '\'' | '«' | '»' | '„' | '“' | '”' | '‘' | '’' | '‚' | '‹' | '›' | '′' | '″'
    )
}

/// The single-character dashes of `Kind::Dash`.
pub(crate) const fn is_dash_char(c: char) -> bool {
    matches!(
        c,
        '\u{2010}' | '\u{2011}' | '\u{2012}' | '\u{2013}' | '\u{2014}' | '\u{2015}' | '\u{2212}'
    )
}

/// The characters of `Kind::Punct`.
pub(crate) const fn is_punct_char(c: char) -> bool {
    matches!(
        c,
        ',' | ';'
            | ':'
            | '!'
            | '?'
            | '№'
            | '§'
            | '¶'
            | '%'
            | '‰'
            | '°'
            | '/'
            | '\\'
            | '@'
            | '#'
            | '&'
            | '*'
            | '+'
            | '='
            | '<'
            | '>'
            | '|'
            | '~'
            | '^'
            | '$'
            | '_'
            | '`'
    )
}

/// The script of a letter from fixed ranges (design section 5.2), as a bit.
pub(crate) const fn script_bit(c: char) -> u8 {
    match c {
        '\u{400}'..='\u{52f}'
        | '\u{1c80}'..='\u{1c8f}'
        | '\u{2de0}'..='\u{2dff}'
        | '\u{a640}'..='\u{a69f}' => 1,
        'a'..='z'
        | 'A'..='Z'
        | '\u{c0}'..='\u{24f}'
        | '\u{1e00}'..='\u{1eff}'
        | '\u{2c60}'..='\u{2c7f}'
        | '\u{a720}'..='\u{a7ff}' => 2,
        '\u{370}'..='\u{3ff}' | '\u{1f00}'..='\u{1fff}' => 4,
        _ => 8,
    }
}

/// The script of a word from the scripts of its letters.
const fn script_of(mask: u8) -> WordScript {
    match mask {
        1 => WordScript::Cyrillic,
        2 => WordScript::Latin,
        4 => WordScript::Greek,
        8 | 0 => WordScript::Other,
        _ => WordScript::Mixed,
    }
}

/// How a cluster joins its neighbors.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Class {
    Letter,
    Digit,
    Hyphen,
    Dot,
    Space,
    /// A whitespace cluster with a mark, ZWNJ or ZWJ attached (design section 5.3).
    FrozenSpace,
    Single(Kind),
}

/// `char::is_alphabetic`, with the basic Cyrillic letters (U+0400 to U+045F) answered without the
/// tables of the standard library, which are slow to search.
pub(crate) fn is_alphabetic(c: char) -> bool {
    matches!(c, '\u{400}'..='\u{45f}') || c.is_alphabetic()
}

/// Whether a character that starts a cluster makes it a word cluster: a letter, or a digit of a
/// system other than ASCII (any non-ASCII character with `char::is_numeric`, design section 5.2).
fn is_word_char(c: char) -> bool {
    is_alphabetic(c) || (c.is_numeric() && !c.is_ascii())
}

/// The class of one cluster (design section 3.5).
fn class_of(cluster: &str) -> Class {
    let mut chars = cluster.chars();
    let Some(c) = chars.next() else {
        return Class::Single(Kind::Symbol);
    };
    if chars.next().is_some() {
        return if cluster == "\r\n" {
            Class::Single(Kind::Newline)
        } else if cluster.chars().all(is_joiner) {
            Class::Single(Kind::Format)
        } else if is_horizontal_space(c) {
            Class::FrozenSpace
        } else if is_word_char(c) {
            Class::Letter
        } else {
            Class::Single(Kind::Symbol)
        };
    }
    if is_word_char(c) {
        return Class::Letter;
    }
    match c {
        '0'..='9' => Class::Digit,
        '-' => Class::Hyphen,
        '.' => Class::Dot,
        '…' => Class::Single(Kind::Dots),
        '×' => Class::Single(Kind::Times),
        '©' => Class::Single(Kind::LegalMark(Mark::Copyright)),
        '®' => Class::Single(Kind::LegalMark(Mark::Registered)),
        '™' => Class::Single(Kind::LegalMark(Mark::Trademark)),
        '→' => Class::Single(Kind::Operator(Op::ArrowRight)),
        '←' => Class::Single(Kind::Operator(Op::ArrowLeft)),
        '↔' => Class::Single(Kind::Operator(Op::ArrowBoth)),
        '≥' => Class::Single(Kind::Operator(Op::GreaterEqual)),
        '≤' => Class::Single(Kind::Operator(Op::LessEqual)),
        '≠' => Class::Single(Kind::Operator(Op::NotEqual)),
        '±' => Class::Single(Kind::Operator(Op::PlusMinus)),
        '(' | '[' | '{' => Class::Single(Kind::Open(c)),
        ')' | ']' | '}' => Class::Single(Kind::Close(c)),
        _ if is_line_break(c) || c == '\u{2029}' => Class::Single(Kind::Newline),
        _ if is_horizontal_space(c) => Class::Space,
        _ if is_format_char(c) || is_joiner(c) => Class::Single(Kind::Format),
        _ if is_quote_char(c) => Class::Single(Kind::Quote),
        _ if is_dash_char(c) => Class::Single(Kind::Dash),
        _ if is_punct_char(c) => Class::Single(Kind::Punct(c)),
        _ => Class::Single(Kind::Symbol),
    }
}

/// The extended grapheme clusters of `text[start..end]`, as ranges of `text` (design section
/// 3.5). A position between two ASCII bytes is a cluster boundary, except inside `\r\n`; every
/// run of non-ASCII characters goes to the segmenter together with the ASCII byte directly
/// before it (a mark may extend it) and the one directly after it (a `Prepend` character glues
/// it).
pub(crate) fn clusters(text: &str, start: usize, end: usize, out: &mut Vec<(u32, u32)>) {
    out.clear();
    let bytes = text.as_bytes();
    let at = |i: usize| bytes.get(i).copied().unwrap_or(0);
    let mut push = |s: usize, e: usize| {
        if let (Ok(s), Ok(e)) = (u32::try_from(s), u32::try_from(e)) {
            out.push((s, e));
        }
    };
    let mut i = start;
    while i < end {
        let b = at(i);
        let next_complex = i + 1 < end && at(i + 1) >= 0x80;
        if b < 0x80 && !next_complex {
            if b == b'\r' && i + 1 < end && at(i + 1) == b'\n' {
                push(i, i + 2);
                i += 2;
            } else {
                push(i, i + 1);
                i += 1;
            }
            continue;
        }
        // A region of non-ASCII characters with its ASCII neighbors.
        let region_start = i;
        let mut j = if b < 0x80 { i + 1 } else { i };
        loop {
            if j >= end {
                break;
            }
            if at(j) >= 0x80 {
                j += 1;
                continue;
            }
            if j + 1 < end && at(j + 1) >= 0x80 {
                j += 1;
                continue;
            }
            j += if at(j) == b'\r' && j + 1 < end && at(j + 1) == b'\n' {
                2
            } else {
                1
            };
            break;
        }
        let j = j.min(end);
        if let Some(region) = text.get(region_start..j) {
            for (offset, cluster) in region.grapheme_indices(true) {
                let s = region_start + offset;
                push(s, s + cluster.len());
            }
        } else {
            // Not reachable for valid positions; keep the text as one cluster rather than fail.
            push(region_start, j);
        }
        i = j;
    }
}

/// Lexes the atoms of one window into tokens (design section 3.5).
pub(crate) fn lex_window(
    text: &str,
    atoms: &[Atom],
    cluster_buf: &mut Vec<(u32, u32)>,
    out: &mut Vec<Tok>,
) {
    out.clear();
    for atom in atoms {
        match *atom {
            Atom::Text { start, end } => {
                clusters(text, start as usize, end as usize, cluster_buf);
                lex_text(text, cluster_buf, out);
            }
            Atom::Delimited { start, end } => {
                let mut tok = Tok::text(Kind::Protected, start, end);
                tok.flags = Flags::FROZEN;
                out.push(tok);
            }
            Atom::Slot { piece, at, kind } => out.push(opaque(Kind::Slot(kind), piece, at, true)),
            Atom::Protected { piece, at } => out.push(opaque(Kind::Protected, piece, at, true)),
            Atom::Markup { piece, at } => out.push(opaque(Kind::Markup, piece, at, false)),
            Atom::Boundary { .. } => {}
        }
    }
}

fn opaque(kind: Kind, piece: u32, at: u32, frozen: bool) -> Tok {
    let mut tok = Tok::text(kind, at, at);
    tok.piece = piece;
    if frozen {
        tok.flags = Flags::FROZEN;
    }
    tok
}

/// A token being joined from clusters.
struct Pending {
    class: Class,
    start: u32,
    end: u32,
    scripts: u8,
    chars: u32,
    first: char,
}

impl Pending {
    fn finish(&self) -> Tok {
        match self.class {
            Class::Letter => Tok::text(Kind::Word(script_of(self.scripts)), self.start, self.end),
            Class::Digit => Tok::text(Kind::Number, self.start, self.end),
            Class::Hyphen => Tok::text(Kind::Dash, self.start, self.end),
            Class::Dot => Tok::text(Kind::Dots, self.start, self.end),
            Class::Space => {
                let mut tok = Tok::text(Kind::Space, self.start, self.end);
                tok.glue = match (self.chars, self.first) {
                    (1, '\u{a0}') => Glue::NoBreak,
                    (1, '\u{202f}') => Glue::Narrow,
                    _ => Glue::None,
                };
                tok
            }
            Class::FrozenSpace => {
                let mut tok = Tok::text(Kind::Space, self.start, self.end);
                tok.flags = Flags::FROZEN;
                tok
            }
            Class::Single(kind) => Tok::text(kind, self.start, self.end),
        }
    }
}

/// Joins the clusters of one text into tokens.
fn lex_text(text: &str, clusters: &[(u32, u32)], out: &mut Vec<Tok>) {
    let slice = |(s, e): (u32, u32)| text.get(s as usize..e as usize).unwrap_or("");
    let single_byte = |i: usize| -> Option<u8> {
        let (s, e) = *clusters.get(i)?;
        (e == s + 1).then(|| text.as_bytes().get(s as usize).copied())?
    };
    let mut pending: Option<Pending> = None;
    let mut no_operator_until = 0u32;
    let mut i = 0;
    while let Some(&(s, e)) = clusters.get(i) {
        // Multi-character plain forms: whole operators and legal marks (design section 3.5).
        let mut whole: Option<(Kind, usize)> = None;
        if let Some(b) = single_byte(i) {
            if is_operator_byte(b) && s >= no_operator_until {
                let mut j = i;
                while single_byte(j).is_some_and(is_operator_byte) {
                    j += 1;
                }
                let run_end = clusters.get(j - 1).map_or(e, |c| c.1);
                let run = text.get(s as usize..run_end as usize).unwrap_or("");
                match PLAIN_OPERATORS.iter().find(|(form, _)| *form == run) {
                    Some((_, op)) => whole = Some((Kind::Operator(*op), j)),
                    None => no_operator_until = run_end,
                }
            } else if b == b'(' {
                whole = legal_mark(text, clusters, i);
            }
        }
        if let Some((kind, next)) = whole {
            if let Some(p) = pending.take() {
                out.push(p.finish());
            }
            let end = clusters.get(next - 1).map_or(e, |c| c.1);
            out.push(Tok::text(kind, s, end));
            i = next;
            continue;
        }
        let cluster = slice((s, e));
        let class = class_of(cluster);
        let joins = matches!(
            class,
            Class::Letter | Class::Digit | Class::Hyphen | Class::Dot | Class::Space
        );
        let mut scripts = 0;
        if class == Class::Letter {
            for c in cluster.chars().filter(|&c| is_alphabetic(c)) {
                scripts |= script_bit(c);
            }
        }
        let chars = u32::try_from(cluster.chars().count()).unwrap_or(u32::MAX);
        match pending.as_mut() {
            Some(p) if joins && p.class == class => {
                p.end = e;
                p.scripts |= scripts;
                p.chars = p.chars.saturating_add(chars);
            }
            _ => {
                if let Some(p) = pending.take() {
                    out.push(p.finish());
                }
                pending = Some(Pending {
                    class,
                    start: s,
                    end: e,
                    scripts,
                    chars,
                    first: cluster.chars().next().unwrap_or(' '),
                });
                if !joins && let Some(p) = pending.take() {
                    out.push(p.finish());
                }
            }
        }
        i += 1;
    }
    if let Some(p) = pending.take() {
        out.push(p.finish());
    }
}

/// `(c)`, `(C)`, `(с)`, `(С)`, `(r)`, `(R)`, `(tm)`, `(TM)` from cluster `i` on, each character a
/// cluster of its own: the kind and the index of the cluster after the mark.
fn legal_mark(text: &str, clusters: &[(u32, u32)], i: usize) -> Option<(Kind, usize)> {
    let one = |k: usize| -> Option<char> {
        let (s, e) = *clusters.get(k)?;
        let cluster = text.get(s as usize..e as usize)?;
        let mut chars = cluster.chars();
        let c = chars.next()?;
        chars.next().is_none().then_some(c)
    };
    match (one(i + 1)?, one(i + 2)?) {
        ('c' | 'C' | 'с' | 'С', ')') => Some((Kind::LegalMark(Mark::Copyright), i + 3)),
        ('r' | 'R', ')') => Some((Kind::LegalMark(Mark::Registered), i + 3)),
        ('t', 'm') | ('T', 'M') if one(i + 3)? == ')' => {
            Some((Kind::LegalMark(Mark::Trademark), i + 4))
        }
        _ => None,
    }
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
        assert!(!flags.contains(Flags::CHANGED.with(Flags::INSERTED)));
        let _ = Repl::Char('x');
    }

    #[test]
    fn clusters_come_from_the_unicode_17_tables() {
        // Design sections 5.1 and 14.1: extended grapheme clusters of Unicode 17.0.0.
        assert_eq!(unicode_segmentation::UNICODE_VERSION, (17, 0, 0));
        let clusters: Vec<&str> = "е\u{301}ж👍🏽".graphemes(true).collect();
        assert_eq!(clusters, ["е\u{301}", "ж", "👍🏽"]);
    }

    fn fast(text: &str) -> Vec<usize> {
        let mut out = Vec::new();
        clusters(text, 0, text.len(), &mut out);
        out.iter().map(|c| c.0 as usize).collect()
    }

    fn reference(text: &str) -> Vec<usize> {
        text.grapheme_indices(true).map(|(i, _)| i).collect()
    }

    #[test]
    fn the_fast_letter_test_is_that_of_the_standard_library() {
        for c in (0..=0x10_ffff).filter_map(char::from_u32) {
            assert_eq!(
                is_alphabetic(c),
                c.is_alphabetic(),
                "{}",
                c.escape_unicode()
            );
        }
    }

    /// The fast path gives the boundaries of the segmenter (design section 13.6).
    #[test]
    fn the_fast_path_agrees_with_the_segmenter() {
        const ALPHABET: &[&str] = &[
            "a",
            "1",
            "\"",
            " ",
            "\r",
            "\n",
            "\r\n",
            "я",
            "\u{600}",
            "\u{301}",
            "\u{200d}",
            "\u{200c}",
            "\u{903}",
            "\u{fe0f}",
            "\u{20e3}",
            "\u{1f600}",
            "\u{1f1f7}",
            "\u{1100}",
            "\u{1161}",
            "\u{ad}",
            "\u{85}",
            "\t",
        ];
        for a in ALPHABET {
            for b in ALPHABET {
                for c in ALPHABET {
                    for d in ["x", "\u{600}", "\u{301}", "\n", "\r"] {
                        let text = format!("{a}{b}{c}{d}");
                        assert_eq!(fast(&text), reference(&text), "{}", text.escape_unicode());
                    }
                }
            }
        }
    }

    /// The same with one character of every `Grapheme_Cluster_Break` class of UAX #29 and the
    /// Indic conjunct classes, as `GraphemeBreakTest.txt` uses them, between ASCII bytes. The
    /// fixture itself is read by `tests/graphemes.rs`: the library sources read no file, not even
    /// in a unit test (design section 13.9).
    #[test]
    fn the_fast_path_agrees_with_the_segmenter_on_every_break_class() {
        const CLASSES: &[&str] = &[
            "\r",
            "\n",
            "\u{1}",
            "\u{300}",
            "\u{200d}",
            "\u{1f1e6}",
            "\u{600}",
            "\u{903}",
            "\u{1100}",
            "\u{1160}",
            "\u{11a8}",
            "\u{ac00}",
            "\u{ac01}",
            "\u{a9}",
            "\u{1f600}",
            "\u{94d}",
            "\u{915}",
            "\u{378}",
            "a",
        ];
        for a in CLASSES {
            for b in CLASSES {
                for c in CLASSES {
                    for (before, after) in [("", ""), ("a", "b"), ("\"", "1"), (" ", "\r\n")] {
                        let text = format!("{before}{a}{b}{c}{after}");
                        assert_eq!(fast(&text), reference(&text), "{}", text.escape_unicode());
                    }
                }
            }
        }
    }

    fn kinds(text: &str) -> Vec<(Kind, &str)> {
        let atoms = [Atom::Text {
            start: 0,
            end: u32::try_from(text.len()).unwrap(),
        }];
        let mut buf = Vec::new();
        let mut out = Vec::new();
        lex_window(text, &atoms, &mut buf, &mut out);
        out.iter()
            .map(|t| (t.kind, &text[t.src.start as usize..t.src.end as usize]))
            .collect()
    }

    #[test]
    fn plain_forms_are_lexed_whole() {
        assert_eq!(
            kinds("a->b"),
            [
                (Kind::Word(WordScript::Latin), "a"),
                (Kind::Operator(Op::ArrowRight), "->"),
                (Kind::Word(WordScript::Latin), "b"),
            ]
        );
        assert_eq!(kinds("-->"), [(Kind::Dash, "--"), (Kind::Punct('>'), ">")]);
        assert_eq!(
            kinds("<-->"),
            [
                (Kind::Punct('<'), "<"),
                (Kind::Dash, "--"),
                (Kind::Punct('>'), ">")
            ]
        );
        assert_eq!(kinds("(c)"), [(Kind::LegalMark(Mark::Copyright), "(c)")]);
        assert_eq!(
            kinds("(\u{441})"),
            [(Kind::LegalMark(Mark::Copyright), "(\u{441})")]
        );
        assert_eq!(kinds("(TM)"), [(Kind::LegalMark(Mark::Trademark), "(TM)")]);
        assert_eq!(
            kinds("(Tm)"),
            [
                (Kind::Open('('), "("),
                (Kind::Word(WordScript::Latin), "Tm"),
                (Kind::Close(')'), ")"),
            ]
        );
    }

    #[test]
    fn clusters_join_into_tokens() {
        assert_eq!(
            kinds("за\u{301}мок 12...x"),
            [
                (Kind::Word(WordScript::Cyrillic), "за\u{301}мок"),
                (Kind::Space, " "),
                (Kind::Number, "12"),
                (Kind::Dots, "..."),
                (Kind::Word(WordScript::Latin), "x"),
            ]
        );
        assert_eq!(
            kinds("\u{41c}o"),
            [(Kind::Word(WordScript::Mixed), "\u{41c}o")]
        );
        assert_eq!(
            kinds("1\u{fe0f}\u{20e3}"),
            [(Kind::Symbol, "1\u{fe0f}\u{20e3}")]
        );
        // Digits of other systems are word characters without a script (design section 5.2).
        assert_eq!(
            kinds("abc\u{663}"),
            [(Kind::Word(WordScript::Latin), "abc\u{663}")]
        );
        assert_eq!(
            kinds("\u{663}\u{664}"),
            [(Kind::Word(WordScript::Other), "\u{663}\u{664}")]
        );
        assert_eq!(
            kinds("x\u{ff11} 5"),
            [
                (Kind::Word(WordScript::Latin), "x\u{ff11}"),
                (Kind::Space, " "),
                (Kind::Number, "5"),
            ]
        );
        assert_eq!(kinds("\u{200c}a")[0], (Kind::Format, "\u{200c}"));
        assert_eq!(kinds("a\r\nb")[1], (Kind::Newline, "\r\n"));
        assert_eq!(kinds(" \u{301}")[0], (Kind::Space, " \u{301}"));
    }

    #[test]
    fn glue_of_a_space_run() {
        let atoms = |text: &str| {
            [Atom::Text {
                start: 0,
                end: u32::try_from(text.len()).unwrap(),
            }]
        };
        let mut buf = Vec::new();
        let mut out = Vec::new();
        for (text, glue, frozen) in [
            ("\u{a0}", Glue::NoBreak, false),
            ("\u{202f}", Glue::Narrow, false),
            ("\u{a0}\u{a0}", Glue::None, false),
            (" ", Glue::None, false),
            (" \u{301}", Glue::None, true),
        ] {
            lex_window(text, &atoms(text), &mut buf, &mut out);
            assert_eq!(out.len(), 1, "{text:?}");
            assert_eq!(out[0].glue, glue, "{text:?}");
            assert_eq!(out[0].is_frozen(), frozen, "{text:?}");
        }
    }
}
