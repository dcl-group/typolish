//! `proptest` generators of design section 13.5: a grammar of risky pieces, templates and rule
//! sets.

#![allow(dead_code, clippy::unwrap_used, clippy::indexing_slicing)]

use proptest::prelude::*;
use typolish::{Rule, RuleSet, Slot, SlotKind};

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

/// Arbitrary Unicode text, with a bias to risky pieces.
pub fn any_text() -> impl Strategy<Value = String> {
    prop_oneof![risky_text(), ".*", "\\PC*"]
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
    prop::collection::vec(
        prop_oneof![
            4 => risky_text().prop_map(GenPiece::Text),
            2 => slot_kind().prop_map(|k| GenPiece::Slot(0, k)),
            1 => risky_text().prop_map(GenPiece::Protected),
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
