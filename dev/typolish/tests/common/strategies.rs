//! `proptest` generators of design section 13.5: a grammar of risky pieces, lines of the corpus
//! with risky pieces inserted, templates and rule sets.

#![allow(dead_code, clippy::unwrap_used, clippy::indexing_slicing)]

use proptest::prelude::*;
use typolish::{Language, Rule, RuleSet, Slot, SlotKind, Typographer};

/// Fragments that rules react to or that output kinds encode.
pub const RISKY: &[&str] = &[
    "\"",
    "'",
    "\"\"",
    "\"\"\"",
    "«",
    "»",
    "„",
    "“",
    "”",
    "‘",
    "’",
    "-",
    "--",
    "—",
    "–",
    ".",
    "...",
    "…",
    "?..",
    "!..",
    " ",
    "  ",
    "\u{a0}",
    "\u{202f}",
    "\n",
    "\r\n",
    "\n\n",
    "\t",
    "\u{2028}",
    "\u{2029}",
    "\u{85}",
    "\r",
    "\u{b}",
    "\u{2003}",
    "\u{2009}",
    "\u{200a}",
    "\u{205f}",
    "\u{3000}",
    "\u{7f}",
    "\u{0}",
    "\u{c}",
    "5",
    "10",
    "2026",
    "1000",
    "0.5",
    "5'10\"",
    "90's",
    "'90s",
    "100%",
    "100‰",
    "№",
    "§",
    "¶",
    "5 МБ",
    "30 с",
    "12 дн.",
    "т. е.",
    "т.е.",
    // A compact abbreviation after a spaced dash (design sections 4.3, I2, and 8.2).
    " - т.е.",
    " – т.д.",
    " -- т.е.",
    "и т.д.",
    "А.С. Пушкин",
    "21 ноября",
    "2026 г.",
    "в",
    "на",
    "не",
    "же",
    "ли",
    "a",
    "the",
    "to",
    "ls -la",
    "--force",
    "host:8080",
    "::1",
    "10.0.0.0/8",
    "a:b",
    "8000-8100",
    "2026-11-21",
    "0.2.27",
    "v1.2.3-rc.1",
    "550e8400-e29b-41d4-a716-446655440000",
    "a,b,c",
    "tls-server",
    "kv_lang",
    "<word>",
    "a>>b",
    "=>",
    "(c)",
    "(tm)",
    "->",
    ">=",
    "!=",
    "1920x1080",
    // Stage 3: the number and symbol rules (design sections 8.8 and 8.9).
    "-5",
    "(-5)",
    "-1,5",
    "tail -5",
    "10 x 5",
    "10 Х 5",
    "(c) 2026",
    "2026 (c)",
    // A percentage after spaces next to the number and symbol rules (design sections 8.2, 8.8,
    // 8.9).
    "10 % x 5",
    "(c) 2026 %",
    " %",
    "%",
    "‰",
    "№10",
    "Brand (r)",
    "Brand(tm)",
    "x <-> y",
    "+-5",
    "a +- b",
    "x <= y",
    "−5",
    "×",
    "©",
    "™",
    "→",
    "≥",
    "https://example.com/a?b=c",
    "user@example.com",
    "/etc/hosts",
    "`code`",
    "`",
    "&",
    "<",
    ">",
    "&nbsp;",
    "\\",
    "$",
    "{",
    "${",
    "</script>",
    "</style>",
    "-->",
    "👨\u{200d}👩\u{200d}👧",
    "e\u{301}",
    "\u{301}",
    "\u{200d}",
    "🇷🇺",
    "1\u{fe0f}\u{20e3}",
    "\u{2068}",
    // Format characters that the lexer skips inside a word or a number (design section 5.4).
    "\u{ad}",
    "\u{200b}",
    "\u{200c}",
    "\u{2060}",
    "\u{feff}",
    "1\u{ad}0",
    "0\u{200b}5 x 3",
    "\u{2069}",
    "\u{fdd0}",
    "Москва",
    "слово",
    "word",
    "Мoсква",
    "😀",
    "𝔘",
];

/// A string built from risky pieces.
pub fn risky_text() -> impl Strategy<Value = String> {
    prop::collection::vec(prop::sample::select(RISKY), 0..14).prop_map(|v| v.concat())
}

/// The `.txt` files of the corpus (design section 13.7).
const CORPUS: [&str; 4] = [
    include_str!("../corpus/ru-RU/prose.txt"),
    include_str!("../corpus/ru-RU/tech.txt"),
    include_str!("../corpus/en-US/prose.txt"),
    include_str!("../corpus/en-US/tech.txt"),
];

/// A line of the corpus with one to three risky pieces inserted at character boundaries (design
/// section 13.5, "strings from the corpus with insertions").
pub fn corpus_with_insertions() -> impl Strategy<Value = String> {
    let lines: Vec<&'static str> = CORPUS.iter().flat_map(|f| f.lines()).collect();
    (
        prop::sample::select(lines),
        prop::collection::vec(
            (any::<prop::sample::Index>(), prop::sample::select(RISKY)),
            1..4,
        ),
    )
        .prop_map(|(line, inserts)| {
            let mut line = line.to_owned();
            for (at, piece) in inserts {
                let bounds: Vec<usize> = line
                    .char_indices()
                    .map(|(i, _)| i)
                    .chain([line.len()])
                    .collect();
                line.insert_str(*at.get(&bounds), piece);
            }
            line
        })
}

/// Arbitrary Unicode text, with a bias to risky pieces and lines of the corpus.
pub fn any_text() -> impl Strategy<Value = String> {
    prop_oneof![risky_text(), corpus_with_insertions(), ".*", "\\PC*"]
}

/// A slot kind.
pub fn slot_kind() -> impl Strategy<Value = SlotKind> {
    prop::sample::select(vec![
        SlotKind::Number,
        SlotKind::Word,
        SlotKind::Date,
        SlotKind::Other,
    ])
}

/// The kinds of a piece of a generated template.
#[derive(Clone, Debug)]
pub enum GenPiece {
    /// Text.
    Text(String),
    /// A slot with its payload.
    Slot(u32, SlotKind),
    /// Protected text.
    Protected(String),
    /// Markup.
    Markup(String),
}

/// A list of generated pieces; slot payloads are their indices.
pub fn pieces() -> impl Strategy<Value = Vec<GenPiece>> {
    pieces_of(risky_text)
}

/// A list of generated pieces whose texts come from `text`; slot payloads are their indices.
pub fn pieces_of<S: Strategy<Value = String> + 'static>(
    text: fn() -> S,
) -> impl Strategy<Value = Vec<GenPiece>> {
    prop::collection::vec(
        prop_oneof![
            4 => text().prop_map(GenPiece::Text),
            2 => slot_kind().prop_map(|k| GenPiece::Slot(0, k)),
            1 => text().prop_map(GenPiece::Protected),
            1 => prop::sample::select(vec!["<b>", "</b>", "<br>", "&amp;", ""]).prop_map(|m| GenPiece::Markup(m.to_owned())),
        ],
        0..8,
    )
    .prop_map(|mut v| {
        let mut n = 0;
        for p in &mut v {
            if let GenPiece::Slot(id, _) = p {
                *id = n;
                n += 1;
            }
        }
        v
    })
}

/// Borrowed pieces of generated ones.
pub fn borrow(pieces: &[GenPiece]) -> Vec<typolish::Piece<'_, u32>> {
    pieces
        .iter()
        .map(|p| match p {
            GenPiece::Text(t) => typolish::Piece::Text(t),
            GenPiece::Slot(id, kind) => typolish::Piece::Slot(*id, Slot::new(*kind)),
            GenPiece::Protected(t) => typolish::Piece::Protected(t),
            GenPiece::Markup(t) => typolish::Piece::Markup(t),
        })
        .collect()
}

/// A rule set: one of the built-in sets or a random subset.
pub fn rule_set() -> impl Strategy<Value = RuleSet> {
    prop_oneof![
        Just(RuleSet::EMPTY),
        Just(RuleSet::ALL),
        Just(RuleSet::PROSE),
        Just(RuleSet::INTERFACE),
        any::<u32>().prop_map(|mask| {
            Rule::ALL
                .iter()
                .enumerate()
                .filter(|(i, _)| mask & (1 << i) != 0)
                .map(|(_, r)| *r)
                .collect()
        }),
    ]
}

/// The delimiters of the protection configurations of `protection_typographer`.
pub const DELIMITERS: &[&str] = &["|", "{{", "}}", "⟦", "⟧", "«", "»", "`", "``", "```"];

/// The number of configurations `protection_typographer` knows.
pub const PROTECTION_CONFIGS: usize = 5;

/// A typographer with own protection pairs or with backtick protection off (design section 10.1):
/// equal delimiters `|`, multi-byte `{{` `}}` and `⟦` `⟧`, `«` `»` with backticks off, and a
/// backtick pair of its own next to the backtick runs.
pub fn protection_typographer(config: usize, language: Language, set: RuleSet) -> Typographer {
    let builder = Typographer::builder(language, set);
    let builder = match config % PROTECTION_CONFIGS {
        0 => builder.protect_between("|", "|"),
        1 => builder.protect_between("{{", "}}"),
        2 => builder.protect_between("⟦", "⟧"),
        3 => builder.protect_backticks(false).protect_between("«", "»"),
        _ => builder.protect_between("`", "`"),
    };
    builder.build().unwrap()
}

/// A string of risky pieces mixed with the delimiters of `protection_typographer`.
pub fn delimiter_text() -> impl Strategy<Value = String> {
    prop::collection::vec(
        prop_oneof![
            2 => prop::sample::select(RISKY),
            1 => prop::sample::select(DELIMITERS),
            1 => prop::sample::select(&["\n", "\n\n", " ", "a", "1"][..]),
        ],
        0..20,
    )
    .prop_map(|v| v.concat())
}
