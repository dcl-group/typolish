//! The own corpus of design section 13.7: loading, the settings of `roundtrip-known.tsv`
//! (section 12.2), the output kinds of the snapshots and their rendering.
//!
//! The corpus files are read from `tests/corpus/` at run time.

#![allow(
    dead_code,
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing
)]

use std::borrow::Cow;
use std::fs;
use std::path::PathBuf;

use typolish::{
    EscapeStyle, HtmlStyle, Language, Output, PercentSpacing, Piece, RuleSet, SingleQuotes,
    SlotKind, Template, TemplatePart, Typographer,
};

use super::{TmplMessage, TmplPiece, parse_tmpl, vis};

/// The directories of the corpus, one per language; the directory name is the language tag.
pub const LANGS: [&str; 2] = ["ru-RU", "en-US"];

/// The files of one language directory.
pub const FILES: [&str; 3] = ["prose.txt", "ui.tmpl", "tech.txt"];

/// The built-in sets the corpus runs with.
pub const SETS: [(&str, RuleSet); 2] =
    [("prose", RuleSet::PROSE), ("interface", RuleSet::INTERFACE)];

/// The output kinds of the snapshots in `expected/` (design section 13.7): the name used in the
/// file name and the kind.
pub fn snapshot_outputs() -> Vec<(&'static str, Output)> {
    vec![
        ("unicode", Output::Unicode),
        ("html-named", Output::Html(HtmlStyle::NAMED)),
        (
            "html-characters-nowrap-class",
            Output::Html(HtmlStyle::CHARACTERS.nowrap_class("nowrap")),
        ),
        (
            "html-hex-nowrap-style",
            Output::Html(HtmlStyle::HEX.nowrap_style()),
        ),
        ("js", Output::Escaped(EscapeStyle::JS)),
        ("json", Output::Escaped(EscapeStyle::JSON)),
        ("rust", Output::Escaped(EscapeStyle::RUST)),
        ("css", Output::Escaped(EscapeStyle::CSS)),
    ]
}

/// The directory `tests/corpus/`.
pub fn corpus_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("corpus")
}

/// The text of a corpus file of a language directory.
pub fn read(lang: &str, file: &str) -> String {
    let path = corpus_dir().join(lang).join(file);
    fs::read_to_string(&path).unwrap_or_else(|e| panic!("cannot read {}: {e}", path.display()))
}

/// The built-in language of a corpus directory.
pub fn language(lang: &str) -> Language {
    Language::from_tag(lang).unwrap_or_else(|e| panic!("no language {lang}: {e}"))
}

/// A built-in set by name (`prose`, `interface`).
pub fn set(name: &str) -> RuleSet {
    SETS.iter()
        .find(|(n, _)| *n == name)
        .map(|(_, s)| *s)
        .unwrap_or_else(|| panic!("no set {name}"))
}

/// The typographer a corpus file runs with: the language and the set, default settings.
pub fn typographer(lang: &str, set_name: &str) -> Typographer {
    Typographer::new(language(lang), set(set_name))
}

/// The lines of a `.txt` corpus file; there are no blank lines and no comments in it.
pub fn text_lines(lang: &str, file: &str) -> Vec<String> {
    let text = read(lang, file);
    let lines: Vec<String> = text.lines().map(str::to_owned).collect();
    for line in &lines {
        assert!(!line.trim().is_empty(), "{lang}/{file}: blank line");
        assert_eq!(
            line.trim_end(),
            line,
            "{lang}/{file}: trailing space in {line:?}"
        );
    }
    lines
}

/// The messages of the `ui.tmpl` of a language directory.
pub fn messages(lang: &str) -> Vec<TmplMessage> {
    parse_tmpl(&read(lang, "ui.tmpl"))
}

/// The texts of a message between its slots (always one more than the slots) and the slots as
/// `(name, kind)`.
pub fn texts_and_slots(message: &TmplMessage) -> (Vec<String>, Vec<(String, SlotKind)>) {
    let mut texts = vec![String::new()];
    let mut slots = Vec::new();
    for piece in &message.pieces {
        match piece {
            TmplPiece::Text(t) => texts.last_mut().unwrap().push_str(t),
            TmplPiece::Slot(name, kind) => {
                slots.push((name.clone(), *kind));
                texts.push(String::new());
            }
        }
    }
    (texts, slots)
}

/// The same split of a template: its texts and its slots as `(name, kind)`.
pub fn split_template(template: &Template<'_, &str>) -> (Vec<String>, Vec<(String, SlotKind)>) {
    (
        template.texts().iter().map(Cow::to_string).collect(),
        template
            .slots()
            .iter()
            .map(|(name, slot)| ((*name).to_owned(), slot.kind()))
            .collect(),
    )
}

/// The pieces of a template, to hand a typeset template back to the typographer.
pub fn template_pieces<'a>(template: &'a Template<'_, &'a str>) -> Vec<Piece<'a, &'a str>> {
    template
        .iter()
        .map(|part| match part {
            TemplatePart::Text(t) => Piece::Text(t),
            TemplatePart::Slot(name, slot) => Piece::Slot(*name, slot),
            _ => panic!("a part of a template that the tests do not know"),
        })
        .collect()
}

/// The name of a slot kind in a snapshot.
pub fn kind_name(kind: SlotKind) -> &'static str {
    match kind {
        SlotKind::Number => "number",
        SlotKind::Word => "word",
        SlotKind::Date => "date",
        SlotKind::Other => "other",
        _ => "unknown",
    }
}

/// A message typeset by `t` with `output`, written as in a snapshot: `id = text {name:kind}`.
pub fn render_message(t: &Typographer, message: &TmplMessage, output: Output) -> String {
    let template = t.typeset_template(message.pieces(), output);
    let mut line = format!("{} = ", message.id);
    for part in template.iter() {
        match part {
            TemplatePart::Text(text) => line.push_str(text),
            TemplatePart::Slot(name, slot) => {
                line.push_str(&format!("{{{name}:{}}}", kind_name(slot.kind())));
            }
            _ => panic!("a part of a template that the tests do not know"),
        }
    }
    line
}

/// The snapshot of a corpus file: every line (every message) typeset on its own with `output`,
/// each followed by a line feed.
pub fn render_snapshot(lang: &str, file: &str, set_name: &str, output: Output) -> String {
    let t = typographer(lang, set_name);
    let mut out = String::new();
    if file.ends_with(".tmpl") {
        for message in messages(lang) {
            out.push_str(&render_message(&t, &message, output));
            out.push('\n');
        }
    } else {
        for line in text_lines(lang, file) {
            out.push_str(&t.typeset(&line, output));
            out.push('\n');
        }
    }
    out
}

/// The file name of a snapshot: `<file stem>.<set>.<output>.txt`.
pub fn snapshot_name(file: &str, set_name: &str, output_name: &str) -> String {
    let stem = file.split('.').next().unwrap();
    format!("{stem}.{set_name}.{output_name}.txt")
}

/// The path of a snapshot under `tests/corpus/expected/<language>/`.
pub fn snapshot_path(lang: &str, file: &str, set_name: &str, output_name: &str) -> PathBuf {
    corpus_dir()
        .join("expected")
        .join(lang)
        .join(snapshot_name(file, set_name, output_name))
}

/// The settings of a row of `roundtrip-known.tsv`: `default`, or one setting of the typographer
/// written as `Type::Variant`.
pub fn settings_typographer(lang: &str, settings: &str) -> Typographer {
    let builder = Typographer::builder(language(lang), RuleSet::PROSE);
    let builder = match settings {
        "default" => builder,
        "SingleQuotes::Everywhere" => builder.single_quotes(SingleQuotes::Everywhere),
        "SingleQuotes::NestedOnly" => builder.single_quotes(SingleQuotes::NestedOnly),
        "SingleQuotes::Never" => builder.single_quotes(SingleQuotes::Never),
        "PercentSpacing::AsWritten" => builder.percent(PercentSpacing::AsWritten),
        "PercentSpacing::Language" => builder.percent(PercentSpacing::Language),
        "PercentSpacing::Attached" => builder.percent(PercentSpacing::Attached),
        "PercentSpacing::Spaced" => builder.percent(PercentSpacing::Spaced),
        other => panic!("unknown setting {other:?} in roundtrip-known.tsv"),
    };
    builder
        .build()
        .unwrap_or_else(|e| panic!("settings {settings}: {e}"))
}

/// One row of `tests/corpus/roundtrip-known.tsv` (design section 12.2), the columns after `vis`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct KnownRow {
    /// The language tag.
    pub lang: String,
    /// The settings column.
    pub settings: String,
    /// The input.
    pub input: String,
    /// The expected `typeset(input)`.
    pub typeset: String,
    /// The expected `to_plain(typeset(input))`.
    pub plain: String,
    /// The code of the loss.
    pub code: String,
    /// The line of the file, from 1.
    pub line: usize,
}

/// Reads `tests/corpus/roundtrip-known.tsv`: tab-separated, six columns, `#` starts a comment, the
/// first non-comment line holds the names of the columns.
pub fn known_rows() -> Vec<KnownRow> {
    let path = corpus_dir().join("roundtrip-known.tsv");
    let text =
        fs::read_to_string(&path).unwrap_or_else(|e| panic!("cannot read {}: {e}", path.display()));
    let mut rows = Vec::new();
    let mut header_seen = false;
    for (index, line) in text.lines().enumerate() {
        if line.trim().is_empty() || line.starts_with('#') {
            continue;
        }
        let columns: Vec<&str> = line.split('\t').collect();
        assert_eq!(
            columns.len(),
            6,
            "roundtrip-known.tsv:{}: six tab-separated columns expected: {line:?}",
            index + 1
        );
        if !header_seen {
            assert_eq!(
                columns,
                [
                    "language", "settings", "input", "typeset", "to_plain", "code"
                ],
                "roundtrip-known.tsv: the header line"
            );
            header_seen = true;
            continue;
        }
        rows.push(KnownRow {
            lang: columns[0].to_owned(),
            settings: columns[1].to_owned(),
            input: vis(columns[2]),
            typeset: vis(columns[3]),
            plain: vis(columns[4]),
            code: columns[5].to_owned(),
            line: index + 1,
        });
    }
    rows
}
