//! `proptest` generators of design section 13.5: a grammar of risky pieces, lines of the corpus
//! with risky pieces inserted, templates and rule sets, and a structured grammar of sentences with
//! the configurations it runs under.

#![allow(
    dead_code,
    clippy::unwrap_used,
    clippy::indexing_slicing,
    clippy::panic
)]

use std::sync::OnceLock;

use proptest::prelude::*;
use typolish::{
    DashStyle, EllipsisForms, Language, LanguageBuilder, PercentSpacing, QuotePairs, Rule, RuleSet,
    SingleQuotes, Slot, SlotKind, Typographer,
};

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
    /// Markup at a block boundary.
    Block(String),
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
            1 => prop::sample::select(vec!["</p><p>", "<br><br>", "</li>\n<li>", ""]).prop_map(|m| GenPiece::Block(m.to_owned())),
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
            GenPiece::Block(t) => typolish::Piece::BlockMarkup(t),
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

// ---------------------------------------------------------------------------------------------
// The structured grammar (design section 13.5, "a grammar of sentences")
// ---------------------------------------------------------------------------------------------

/// The words a sentence of the grammar is made of: those of Russian, of English, or both.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Vocabulary {
    /// Russian words, abbreviations and units.
    Ru,
    /// English words, abbreviations and units.
    En,
    /// Both, for `Language::COMMON`.
    Both,
}

/// The lists of one vocabulary.
struct Words {
    /// Words of the language, compound words and words with an apostrophe among them.
    words: &'static [&'static str],
    /// Short words and particles (design section 8.7).
    short: &'static [&'static str],
    /// Abbreviations, compact and with spaces, and initials.
    abbreviations: &'static [&'static str],
    /// Units and signs written after a number.
    units: &'static [&'static str],
    /// Numbers, ranges and numerals.
    numbers: &'static [&'static str],
    /// A sign with its number (`№5`, `§V`).
    labels: &'static [&'static str],
}

const RU_WORDS: Words = Words {
    words: &[
        "дом",
        "Москва",
        "слово",
        "кот",
        "река",
        "Пушкин",
        "США",
        "ООО",
        "кто-то",
        "г-жа",
        "г-н",
        "учёный-географ",
        "Д'Артаньян",
        "IT-отдел",
        "Т",
        "А",
    ],
    short: &[
        "в", "на", "и", "не", "с", "к", "а", "я", "он", "из", "по", "до", "ли", "же", "бы", "ведь",
        "ль",
    ],
    abbreviations: &[
        "т. е.",
        "т.е.",
        "и т. д.",
        "и т.д.",
        "и т. п.",
        "т. н.",
        "н. э.",
        "н.э.",
        "т. н.э.",
        "г.",
        "гг.",
        "в.",
        "стр.",
        "рис.",
        "А. С.",
        "А.С.",
        "Т. е.",
        "Т.е.",
        "т. к.",
        "см.",
    ],
    units: &[
        "кг",
        "км",
        "м",
        "м²",
        "см³",
        "км/ч",
        "кбит/с",
        "ГВт",
        "Дж",
        "Ом",
        "Па",
        "В",
        "А",
        "с",
        "%",
        "‰",
        "°",
        "°C",
        "руб.",
        "дн.",
        "МБ",
        "ккал",
        "кОм",
        "г.",
    ],
    numbers: &[
        "5",
        "10",
        "2026",
        "1,5",
        "-5",
        "1000",
        "10-20",
        "5–10",
        "1941-1945",
        "XIX",
        "3",
        "27",
        "100",
        "10 000",
        "1,5-2,5",
    ],
    labels: &["№5", "№ 5", "§V", "§ 12", "№V", "¶IV", "§5", "№10"],
};

const EN_WORDS: Words = Words {
    words: &[
        "word",
        "Moscow",
        "cat",
        "file",
        "Paul",
        "it's",
        "don't",
        "dogs'",
        "Jones'",
        "DOGS'",
        "boss'",
        "'90s",
        "90's",
        "mother-in-law",
        "tls-server",
        "'cause",
        "'tis",
        "rock'n'roll",
        "S",
        "s",
        "B",
    ],
    short: &[
        "a", "an", "the", "to", "of", "in", "I", "on", "is", "or", "up",
    ],
    abbreviations: &[
        "St.", "p.m.", "a.m.", "e.g.", "i.e.", "Mr.", "Dr.", "etc.", "vs.", "No.", "P.M.", "U.S.",
    ],
    units: &[
        "kg", "km", "in", "ft", "GW", "kPa", "mW", "kJ", "%", "°F", "°", "p.m.", "a.m.", "kbit/s",
        "lb", "oz.",
    ],
    numbers: &[
        "5",
        "10",
        "2026",
        "1.5",
        "-5",
        "1000",
        "10-20",
        "5–10",
        "1941-1945",
        "3",
        "27",
        "100",
        "9-11",
        "1.5-2.5",
        "5'10\"",
    ],
    labels: &["№5", "§V", "§ 12", "§5", "¶IV", "№V"],
};

/// Technical fragments of design section 10.5, in every vocabulary.
const TECHNICAL: &[&str] = &[
    "host:8080",
    "https://example.com/a",
    "ls -la",
    "--force",
    "`code`",
    "user@example.com",
    "v1.2.3",
    "10.0.0.0/8",
    "a=b",
    "x <= y",
    "(c) 2026",
    "Brand(tm)",
    "1920x1080",
    "10 x 5",
    "+-5",
];

/// The dashes: a spaced `-`, a spaced `--`, an attached `-`, a spaced `–`, an attached `—`, and a
/// spaced `—`. At the start of a line the dash is a dialogue dash, `- ` or `— `.
const DASHES: &[&str] = &[" - ", " -- ", "-", " – ", "—", " — "];

/// Opening and closing glyphs of a quote pair: straight, typed in every form, and mixed.
const QUOTE_PAIRS: &[(&str, &str)] = &[
    ("\"", "\""),
    ("\"", "\""),
    ("'", "'"),
    ("'", "'"),
    ("«", "»"),
    ("„", "“"),
    ("“", "”"),
    ("‘", "’"),
    ("‹", "›"),
    ("‚", "‘"),
    ("«", "\""),
    ("\"", "»"),
    ("'", "’"),
    ("‘", "'"),
];

/// Ellipses, attached to what precedes them.
const ELLIPSES: &[&str] = &[
    "...", "…", "?..", "!..", "?...", "!...", "?!..", "?!...", "?…",
];

/// Sentence ends and other punctuation, attached to what precedes them.
const ENDS: &[&str] = &[".", "!", "?", ":", ";", "?!", ",", ","];

/// Line breaks.
const BREAKS: &[&str] = &["\n", "\n\n", "\r\n"];

/// Quotes that stand alone, attached to a neighbor or with spaces.
const STRAYS: &[&str] = &["\"", "'", "«", "»", "’", "‘", "”", "“", "„", "\"\"", "''"];

/// One step of the grammar; the numbers pick from the lists of `Words` and of this file.
#[derive(Clone, Copy, Debug)]
pub enum GrammarOp {
    /// A word.
    Word(u16),
    /// A short word or a particle.
    Short(u16),
    /// An abbreviation.
    Abbreviation(u16),
    /// A number and a unit, with a space or attached.
    Quantity(u16, u16, bool),
    /// A number.
    Number(u16),
    /// A label.
    Label(u16),
    /// A technical fragment.
    Technical(u16),
    /// A dash.
    Dash(u16),
    /// An opening quote; its closing glyph is pushed.
    Open(u16),
    /// The closing glyph of the innermost open quote or wrapper.
    Close,
    /// An opening parenthesis.
    Wrap,
    /// An ellipsis.
    Ellipsis(u16),
    /// A sentence end or a punctuation mark.
    End(u16),
    /// A line break; with `true` everything open is closed first.
    Break(u16, bool),
    /// A quote that belongs to no pair, attached to what precedes it or after a space.
    Stray(u16, bool),
}

/// The deepest nesting of quotes and wrappers the grammar writes.
pub const GRAMMAR_DEPTH: usize = 4;

/// One step of the grammar.
fn grammar_op() -> impl Strategy<Value = GrammarOp> {
    prop_oneof![
        8 => any::<u16>().prop_map(GrammarOp::Word),
        3 => any::<u16>().prop_map(GrammarOp::Short),
        2 => any::<u16>().prop_map(GrammarOp::Abbreviation),
        2 => (any::<u16>(), any::<u16>(), any::<bool>()).prop_map(|(n, u, s)| GrammarOp::Quantity(n, u, s)),
        1 => any::<u16>().prop_map(GrammarOp::Number),
        1 => any::<u16>().prop_map(GrammarOp::Label),
        1 => any::<u16>().prop_map(GrammarOp::Technical),
        3 => any::<u16>().prop_map(GrammarOp::Dash),
        3 => any::<u16>().prop_map(GrammarOp::Open),
        3 => Just(GrammarOp::Close),
        1 => Just(GrammarOp::Wrap),
        1 => any::<u16>().prop_map(GrammarOp::Ellipsis),
        2 => any::<u16>().prop_map(GrammarOp::End),
        1 => (any::<u16>(), any::<bool>()).prop_map(|(b, c)| GrammarOp::Break(b, c)),
        1 => (any::<u16>(), any::<bool>()).prop_map(|(q, a)| GrammarOp::Stray(q, a)),
    ]
}

fn pick<'a>(list: &[&'a str], i: u16) -> &'a str {
    list[usize::from(i) % list.len()]
}

/// The lists of a vocabulary; `Both` picks Russian or English by the number.
fn words_of(vocabulary: Vocabulary, i: u16) -> &'static Words {
    match vocabulary {
        Vocabulary::Ru => &RU_WORDS,
        Vocabulary::En => &EN_WORDS,
        Vocabulary::Both if i.is_multiple_of(2) => &RU_WORDS,
        Vocabulary::Both => &EN_WORDS,
    }
}

/// Writes the steps of the grammar as text: items separated by spaces, dashes and punctuation
/// attached or spaced as the step says, quotes and wrappers nested up to `GRAMMAR_DEPTH` and closed
/// at the end.
pub fn write_grammar(vocabulary: Vocabulary, ops: &[GrammarOp]) -> String {
    let mut out = String::new();
    let mut open: Vec<&str> = Vec::new();
    // Whether the next item needs a space before it.
    let mut space = false;
    let item = |out: &mut String, space: &mut bool, text: &str| {
        if *space {
            out.push(' ');
        }
        out.push_str(text);
        *space = true;
    };
    for &op in ops {
        let w = |i: u16| words_of(vocabulary, i);
        match op {
            GrammarOp::Word(i) => item(&mut out, &mut space, pick(w(i).words, i / 2)),
            GrammarOp::Short(i) => item(&mut out, &mut space, pick(w(i).short, i / 2)),
            GrammarOp::Abbreviation(i) => {
                item(&mut out, &mut space, pick(w(i).abbreviations, i / 2));
            }
            GrammarOp::Quantity(n, u, spaced) => {
                let words = w(n);
                let number = pick(words.numbers, n / 2);
                let unit = pick(words.units, u);
                let gap = if spaced { " " } else { "" };
                item(&mut out, &mut space, &format!("{number}{gap}{unit}"));
            }
            GrammarOp::Number(i) => item(&mut out, &mut space, pick(w(i).numbers, i / 2)),
            GrammarOp::Label(i) => item(&mut out, &mut space, pick(w(i).labels, i / 2)),
            GrammarOp::Technical(i) => item(&mut out, &mut space, pick(TECHNICAL, i)),
            GrammarOp::Dash(i) => {
                let dash = pick(DASHES, i);
                if out.is_empty() || out.ends_with('\n') {
                    out.push_str(if i.is_multiple_of(2) { "- " } else { "— " });
                } else {
                    out.push_str(dash);
                }
                space = false;
            }
            GrammarOp::Open(i) if open.len() < GRAMMAR_DEPTH => {
                let (opener, closer) = QUOTE_PAIRS[usize::from(i) % QUOTE_PAIRS.len()];
                if space {
                    out.push(' ');
                }
                out.push_str(opener);
                open.push(closer);
                space = false;
            }
            GrammarOp::Wrap if open.len() < GRAMMAR_DEPTH => {
                if space {
                    out.push(' ');
                }
                out.push('(');
                open.push(")");
                space = false;
            }
            GrammarOp::Open(i) => item(&mut out, &mut space, pick(w(i).words, i / 2)),
            GrammarOp::Wrap => item(&mut out, &mut space, pick(w(0).short, 0)),
            GrammarOp::Close => {
                if let Some(closer) = open.pop() {
                    out.push_str(closer);
                    space = true;
                }
            }
            GrammarOp::Ellipsis(i) => {
                out.push_str(pick(ELLIPSES, i));
                space = true;
            }
            GrammarOp::End(i) => {
                out.push_str(pick(ENDS, i));
                space = true;
            }
            GrammarOp::Break(i, close) => {
                if close {
                    while let Some(closer) = open.pop() {
                        out.push_str(closer);
                    }
                }
                out.push_str(pick(BREAKS, i));
                space = false;
            }
            GrammarOp::Stray(i, attached) => {
                if space && !attached {
                    out.push(' ');
                }
                out.push_str(pick(STRAYS, i));
                space = true;
            }
        }
    }
    while let Some(closer) = open.pop() {
        out.push_str(closer);
    }
    out
}

/// Sentences of the grammar in a vocabulary: words, abbreviations (`т. е.`, `и т. д.`, `St.`,
/// `p.m.`), units after numbers, particles and short words, labels (`№5`, `§V`), dashes in every
/// form, wrappers, ellipses and sentence ends, and quotes of every glyph nested up to a depth of
/// four (design section 13.5).
pub fn grammar(vocabulary: Vocabulary) -> impl Strategy<Value = String> {
    prop::collection::vec(grammar_op(), 1..32).prop_map(move |ops| write_grammar(vocabulary, &ops))
}

/// The number of languages of `grammar_language`.
pub const GRAMMAR_LANGUAGES: usize = 5;

/// The languages the grammar runs in: `ru-RU`, `en-US`, `COMMON`, and two languages declared with
/// `LanguageBuilder`: `en-GB`, whose primary pair is `‘’` with `“”` nested, and `xx`, Russian data
/// with the pairs `«»` and `‹›`, the closed dash style and the ellipsis forms `?…`, `!…`, `?!…`.
pub fn grammar_language(lang: usize) -> Language {
    static DECLARED: OnceLock<[Language; 2]> = OnceLock::new();
    let declared = DECLARED.get_or_init(|| {
        [
            LanguageBuilder::from_language(&Language::EN_US, "en-GB")
                .quotes(Some(QuotePairs::new(('‘', '’'), ('“', '”'))))
                .build()
                .unwrap(),
            LanguageBuilder::from_language(&Language::RU_RU, "xx")
                .quotes(Some(QuotePairs::new(('«', '»'), ('‹', '›'))))
                .dash_style(DashStyle::Closed)
                .ellipsis(EllipsisForms::new("?…", "!…", "?!…"))
                .build()
                .unwrap(),
        ]
    });
    match lang % GRAMMAR_LANGUAGES {
        0 => Language::RU_RU,
        1 => Language::EN_US,
        2 => Language::COMMON,
        3 => declared[0].clone(),
        _ => declared[1].clone(),
    }
}

/// The vocabulary of a language of `grammar_language`.
pub fn grammar_vocabulary(lang: usize) -> Vocabulary {
    match lang % GRAMMAR_LANGUAGES {
        0 | 4 => Vocabulary::Ru,
        1 | 3 => Vocabulary::En,
        _ => Vocabulary::Both,
    }
}

/// The settings a sentence of the grammar runs under.
#[derive(Clone, Debug)]
pub struct GrammarConfig {
    /// The index of the language in `grammar_language`.
    pub lang: usize,
    /// The rule set: Prose, Interface or all rules.
    pub set: RuleSet,
    /// The dash style; `None` keeps the style of the language.
    pub dash: Option<DashStyle>,
    /// The single quote mode.
    pub single: SingleQuotes,
    /// The percent setting.
    pub percent: PercentSpacing,
}

impl GrammarConfig {
    /// The language of the configuration.
    pub fn language(&self) -> Language {
        grammar_language(self.lang)
    }

    /// The typographer of the configuration.
    pub fn typographer(&self) -> Typographer {
        let builder = Typographer::builder(self.language(), self.set)
            .single_quotes(self.single)
            .percent(self.percent);
        match self.dash {
            Some(style) => builder.dash_style(style),
            None => builder,
        }
        .build()
        .unwrap()
    }

    /// The configuration written for a failure message.
    pub fn describe(&self) -> String {
        let set = if self.set == RuleSet::PROSE {
            "Prose".to_owned()
        } else if self.set == RuleSet::INTERFACE {
            "Interface".to_owned()
        } else if self.set == RuleSet::ALL {
            "ALL".to_owned()
        } else {
            self.set.to_string()
        };
        format!(
            "{} {set} dash {:?} {:?} {:?}",
            self.language(),
            self.dash,
            self.single,
            self.percent
        )
    }
}

/// A configuration for a language of `grammar_language`: Prose, Interface or all rules; the
/// language's dash style or `Spaced`, `SpacedGlued` or `Closed`; the three single quote modes; the
/// four percent settings.
pub fn grammar_config(lang: usize) -> impl Strategy<Value = GrammarConfig> {
    (
        prop::sample::select(vec![RuleSet::PROSE, RuleSet::INTERFACE, RuleSet::ALL]),
        prop::sample::select(vec![
            None,
            Some(DashStyle::Spaced),
            Some(DashStyle::SpacedGlued),
            Some(DashStyle::Closed),
        ]),
        prop::sample::select(vec![
            SingleQuotes::NestedOnly,
            SingleQuotes::Everywhere,
            SingleQuotes::Never,
        ]),
        prop::sample::select(vec![
            PercentSpacing::AsWritten,
            PercentSpacing::Language,
            PercentSpacing::Attached,
            PercentSpacing::Spaced,
        ]),
    )
        .prop_map(move |(set, dash, single, percent)| GrammarConfig {
            lang,
            set,
            dash,
            single,
            percent,
        })
}

/// A configuration and a sentence of the grammar in its vocabulary, for one language of
/// `grammar_language`.
pub fn grammar_case(lang: usize) -> impl Strategy<Value = (GrammarConfig, String)> {
    (grammar_config(lang), grammar(grammar_vocabulary(lang)))
}

/// Runs `check` on `cases` sentences of the grammar with their configurations for one language of
/// `grammar_language`, drawn from the fixed `seed`: the same cases on every run, with no clock and
/// no file of failures. A failure is shrunk to a small input and fails the test with it.
pub fn run_grammar(
    lang: usize,
    cases: u32,
    seed: u64,
    check: impl Fn(&GrammarConfig, &str) -> Result<(), TestCaseError>,
) {
    let config = ProptestConfig {
        cases,
        rng_seed: proptest::test_runner::RngSeed::Fixed(seed),
        failure_persistence: None,
        ..ProptestConfig::default()
    };
    let mut runner = proptest::test_runner::TestRunner::new(config);
    if let Err(error) = runner.run(&grammar_case(lang), |(config, text)| {
        check(&config, &text)
            .map_err(|e| TestCaseError::fail(format!("{e}\n  setup    {}", config.describe())))
    }) {
        panic!("{error}");
    }
}

/// The known limit of design section 10.8 that a sentence of the grammar falls under, by its name,
/// or `None`. The properties over the grammar skip such a sentence and test every other one; each
/// filter below matches the shape of the inputs of one limit, and goes when its limit is fixed.
///
/// - Sign-only pairs that share a quote (`todo/quotes-sign-only-chain.md`): three straight quotes
///   in a run of characters that are no letter, digit or space, each next to the one before it or
///   with signs between them, at least one sign in all, in series of one or two quotes (a series of
///   three or more has no sign-only pair): `''-''`, `'('')'`.
/// - In the English languages, `en-US` and `en-GB`, a typed `‘` that the first pass leaves neutral
///   and a second pass pairs (`todo/quotes-neutral-opener-repaired.md`), which then places a
///   trailing possessive (`‘s' "0"’`) or pairs a straight `'` pair that the first pass cancelled
///   at the top level (`x ‘a 'b' "2"’ y`); and a typed pair that a placed possessive closes early on
///   a second pass around a possessive kept straight (`todo/quotes-possessive-run.md`,
///   `‘S' ‘s'’ s’`). The filter matches a `‘` anywhere before a straight `'`.
/// - In `en-US` and `en-GB`, a straight quote that the first pass writes as `‘` before signs, or
///   nothing, and a typed `’`, which the second pass reads as a sign-only pair
///   (`todo/quotes-new-sign-only-pair.md`, `"a '-’ - x' c"`). The filter matches a straight quote,
///   then characters that are no letter, digit, space or quote, then `’`.
pub fn grammar_known_limit(lang: usize, text: &str) -> Option<&'static str> {
    if sign_only_chain(text) {
        return Some("sign-only pairs that share a quote");
    }
    let english = matches!(lang % GRAMMAR_LANGUAGES, 1 | 3);
    if english && typed_opener_before_straight(text) {
        return Some("a typed `‘` that a second pass pairs");
    }
    if english && straight_before_signs_and_closer(text) {
        return Some("a new `‘` that a second pass reads as a sign-only pair");
    }
    None
}

/// Whether the text holds a chain of sign-only pairs that share a quote (`grammar_known_limit`).
fn sign_only_chain(text: &str) -> bool {
    let chars: Vec<char> = text.chars().collect();
    for run in chars.split(|c| c.is_alphanumeric() || c.is_whitespace()) {
        // The quotes of the run with the number of signs before each and the length of its series.
        let mut quotes: Vec<(usize, usize)> = Vec::new();
        let mut signs = 0;
        let mut k = 0;
        while k < run.len() {
            if matches!(run[k], '"' | '\'') {
                let start = k;
                while k < run.len() && matches!(run[k], '"' | '\'') {
                    k += 1;
                }
                let len = k - start;
                for n in 0..len {
                    quotes.push((if n == 0 { signs } else { 0 }, len));
                }
                signs = 0;
            } else {
                signs += 1;
                k += 1;
            }
        }
        if quotes
            .windows(3)
            .any(|w| w.iter().all(|&(_, len)| len <= 2) && w[1].0 + w[2].0 > 0)
        {
            return true;
        }
    }
    false
}

/// Whether a straight quote stands before signs, or nothing, and a typed `’`
/// (`grammar_known_limit`).
fn straight_before_signs_and_closer(text: &str) -> bool {
    let chars: Vec<char> = text.chars().collect();
    let quote = |c: char| {
        matches!(
            c,
            '"' | '\'' | '«' | '»' | '„' | '“' | '”' | '‘' | '’' | '‹' | '›' | '‚'
        )
    };
    chars.iter().enumerate().any(|(k, &c)| {
        matches!(c, '"' | '\'')
            && chars[k + 1..]
                .iter()
                .find(|&&d| d.is_alphanumeric() || d.is_whitespace() || quote(d))
                == Some(&'’')
    })
}

/// Whether a typed `‘` stands before a straight `'` (`grammar_known_limit`).
fn typed_opener_before_straight(text: &str) -> bool {
    text.find('‘').is_some_and(|at| text[at..].contains('\''))
}
