//! Repository facts and API conformance (design section 13.9).
//!
//! Checks that read `docs/design/` skip when `docs/design/` is absent: the published package has
//! no design. A missing file or heading inside the repository fails.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    clippy::print_stdout,
    clippy::print_stderr,
    clippy::dbg_macro
)]

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};

use proptest::prelude::*;
use typolish::{Language, Output, Rule, RuleSet, Scratch, Typographer};

fn crate_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn repo_dir() -> PathBuf {
    crate_dir().join("../..")
}

fn read(path: &Path) -> String {
    fs::read_to_string(path).unwrap_or_else(|e| panic!("{}: {e}", path.display()))
}

/// A file of `docs/design/`, or `None` when the design is not there (the published package). A
/// missing file inside the repository fails.
fn design(file: &str) -> Option<String> {
    let dir = repo_dir().join("docs/design");
    if !dir.is_dir() {
        eprintln!("skipped: {} is absent", dir.display());
        return None;
    }
    Some(read(&dir.join(file)))
}

/// The cells of a Markdown table row, trimmed; `None` for other lines.
fn cells(line: &str) -> Option<Vec<&str>> {
    let line = line.trim();
    let inner = line.strip_prefix('|')?.strip_suffix('|')?;
    Some(inner.split('|').map(str::trim).collect())
}

/// The text between the first pair of backticks of a cell.
fn backticked(cell: &str) -> Vec<&str> {
    cell.split('`').skip(1).step_by(2).collect()
}

fn set_of(ids: &[&str]) -> RuleSet {
    ids.iter()
        .map(|id| Rule::from_id(id).unwrap_or_else(|| panic!("unknown id {id}")))
        .collect()
}

fn all_ids() -> Vec<&'static str> {
    Rule::ALL.iter().map(|r| r.id()).collect()
}

#[test]
fn license_copies_are_identical() {
    for name in ["LICENSE-MIT", "LICENSE-APACHE"] {
        let root = repo_dir().join(name);
        if !root.exists() {
            eprintln!("skipped: {} is absent", root.display());
            continue;
        }
        assert_eq!(
            fs::read(&root).unwrap(),
            fs::read(crate_dir().join(name)).unwrap(),
            "{name}"
        );
    }
}

#[test]
fn crate_readme_is_a_link_to_the_root_readme() {
    let link = crate_dir().join("README.md");
    if !repo_dir().join("docs").exists() {
        eprintln!("skipped: not in the repository");
        return;
    }
    let meta = fs::symlink_metadata(&link).unwrap();
    assert!(meta.file_type().is_symlink());
    assert_eq!(fs::read_link(&link).unwrap(), Path::new("../../README.md"));
}

#[test]
fn auto_traits() {
    fn typographer<T: Send + Sync + Clone + 'static>() {}
    fn scratch<T: Send + Sync + Default + 'static>() {}
    typographer::<Typographer>();
    scratch::<Scratch>();
}

#[test]
fn a_typographer_fits_a_static() {
    static RU: Typographer = Typographer::new(Language::RU_RU, RuleSet::INTERFACE);
    static EN: Typographer = Typographer::new(Language::EN_US, RuleSet::PROSE);
    static UND: Typographer = Typographer::new(Language::COMMON, RuleSet::EMPTY);
    assert_eq!(RU.language(), &Language::RU_RU);
    assert_eq!(EN.rules(), RuleSet::PROSE);
    assert_eq!(UND.active_rules(), RuleSet::EMPTY);
}

#[test]
fn replacement_tables_produce_no_letter_and_no_digit() {
    // Design section 13.9: the characters that rules write contain no letter and no digit. Every
    // replacement table of `symbols.*`, `number.times` and the other rules is reached here.
    const TEXTS: [&str; 12] = [
        "(c) 2026",
        "Brand(tm)",
        "Brand (r)",
        "a -> b",
        "a <- b",
        "a <-> b",
        "a >= b",
        "a <= b",
        "a != b",
        "+-5",
        "5x3",
        "Ну... да?.. -5 и 1000000 \"цитата 'вложенная'\" -- тире 1990-2000 гг.",
    ];
    const SIGNS: [char; 11] = ['©', '™', '®', '→', '←', '↔', '≥', '≤', '≠', '±', '×'];
    for language in Language::builtin().chain([Language::COMMON]) {
        let t = Typographer::new(language.clone(), RuleSet::ALL);
        let mut written = BTreeSet::new();
        for text in TEXTS {
            let report = t.report(text, Output::Unicode);
            assert!(!report.changes().is_empty(), "{language} {text:?}");
            for change in report.changes() {
                assert!(
                    !change.replacement().chars().any(char::is_alphanumeric),
                    "{language} {text:?}: {change:?}"
                );
                written.extend(change.replacement().chars());
            }
        }
        for sign in SIGNS {
            assert!(written.contains(&sign), "{language}: no {sign:?}");
        }
    }
}

#[test]
fn rules_are_unique_and_round_trip() {
    let ids = all_ids();
    assert_eq!(ids.len(), 27);
    assert_eq!(ids.iter().collect::<BTreeSet<_>>().len(), ids.len());
    for rule in Rule::ALL {
        let id = rule.id();
        let (group, name) = id.split_once('.').expect("group.name");
        assert!(
            !group.is_empty() && group.bytes().all(|b| b.is_ascii_lowercase()),
            "{id}"
        );
        assert!(
            !name.is_empty()
                && name
                    .split('-')
                    .all(|w| !w.is_empty() && w.bytes().all(|b| b.is_ascii_lowercase())),
            "{id}"
        );
        assert_eq!(id.parse::<Rule>(), Ok(*rule));
        assert_eq!(rule.to_string(), id);
        assert_eq!(Rule::from_id(id), Some(*rule));
        assert!(!rule.summary().is_empty());
    }
    assert!("nbsp.nothing".parse::<Rule>().is_err());
}

#[test]
fn languages_round_trip() {
    for lang in Language::builtin().chain([Language::COMMON]) {
        assert_eq!(Language::from_tag(lang.tag()), Ok(lang.clone()));
        assert_eq!(lang.to_string().parse::<Language>(), Ok(lang.clone()));
    }
    assert_eq!(Language::from_tag("UND"), Ok(Language::COMMON));
}

#[test]
fn built_in_rule_sets_round_trip() {
    for set in [
        RuleSet::EMPTY,
        RuleSet::ALL,
        RuleSet::PROSE,
        RuleSet::INTERFACE,
    ] {
        assert_eq!(set.to_string().parse::<RuleSet>(), Ok(set));
    }
    assert_eq!(RuleSet::EMPTY.to_string(), "empty");
}

proptest! {
    #[test]
    fn generated_rule_sets_round_trip(mask in any::<u32>()) {
        let set: RuleSet = Rule::ALL
            .iter()
            .enumerate()
            .filter(|(i, _)| mask & (1 << i) != 0)
            .map(|(_, rule)| *rule)
            .collect();
        prop_assert_eq!(set.to_string().parse::<RuleSet>(), Ok(set));
    }

    #[test]
    fn rule_set_strings_follow_the_grammar(
        base in 0usize..5,
        items in prop::collection::vec((any::<bool>(), 0usize..27), 0..10),
    ) {
        let bases = [None, Some(("prose", RuleSet::PROSE)), Some(("interface", RuleSet::INTERFACE)),
            Some(("empty", RuleSet::EMPTY)), Some(("all", RuleSet::ALL))];
        let mut parts = Vec::new();
        let mut expected = RuleSet::EMPTY;
        if let Some((name, set)) = bases[base] {
            parts.push(name.to_owned());
            expected = set;
        }
        let mut added = RuleSet::EMPTY;
        let mut removed = RuleSet::EMPTY;
        for (add, index) in &items {
            let rule = Rule::ALL[*index];
            if *add {
                added = added.with(rule);
                parts.push(format!(" +{} ", rule.id()));
            } else {
                removed = removed.with(rule);
                parts.push(format!("-{}", rule.id()));
            }
        }
        let text = parts.join(",");
        let parsed = text.parse::<RuleSet>();
        if parts.is_empty() {
            prop_assert!(parsed.is_err());
        } else if !added.difference(added.difference(removed)).is_empty() {
            prop_assert!(parsed.is_err());
        } else {
            prop_assert_eq!(parsed, Ok(expected.union(added).difference(removed)));
        }
    }
}

/// The rows of the table of design section 4.1: id, Prose, Interface.
fn order_table() -> Option<Vec<(String, bool, bool)>> {
    let text = design("04-order-idempotence.md")?;
    let start = text
        .find("### 4.1")
        .expect("04-order-idempotence.md: no heading ### 4.1");
    let end = text
        .find("### 4.2")
        .expect("04-order-idempotence.md: no heading ### 4.2");
    let rows = text[start..end]
        .lines()
        .filter_map(cells)
        .filter(|c| c.len() == 6 && c[0].parse::<u32>().is_ok())
        .map(|c| (backticked(c[2])[0].to_owned(), c[3] == "+", c[4] == "+"))
        .collect();
    Some(rows)
}

/// A row of the table of design section 8.1: id, settings, Prose, Interface.
type RuleRow = (String, Vec<String>, bool, bool);

/// The rows of the table of design section 8.1.
fn rule_table() -> Option<Vec<RuleRow>> {
    let text = design("08-rules.md")?;
    let start = text
        .find("### 8.1")
        .expect("08-rules.md: no heading ### 8.1");
    let end = text
        .find("### 8.2")
        .expect("08-rules.md: no heading ### 8.2");
    let rows = text[start..end]
        .lines()
        .filter_map(cells)
        .filter(|c| c.len() == 6 && c[0].starts_with('`'))
        .map(|c| {
            let settings = backticked(c[3]).into_iter().map(str::to_owned).collect();
            (
                backticked(c[0])[0].to_owned(),
                settings,
                c[4] == "+",
                c[5] == "+",
            )
        })
        .collect();
    Some(rows)
}

#[test]
fn order_table_matches_rule_all() {
    let Some(rows) = order_table() else { return };
    let ids: Vec<&str> = rows.iter().map(|(id, _, _)| id.as_str()).collect();
    assert_eq!(ids, all_ids());
    let prose: Vec<&str> = rows.iter().filter(|r| r.1).map(|r| r.0.as_str()).collect();
    let interface: Vec<&str> = rows.iter().filter(|r| r.2).map(|r| r.0.as_str()).collect();
    assert_eq!(set_of(&prose), RuleSet::PROSE);
    assert_eq!(set_of(&interface), RuleSet::INTERFACE);
}

#[test]
fn rule_table_matches_rule_all() {
    let Some(rows) = rule_table() else { return };
    let ids: Vec<&str> = rows.iter().map(|r| r.0.as_str()).collect();
    assert_eq!(ids, all_ids());
    let prose: Vec<&str> = rows.iter().filter(|r| r.2).map(|r| r.0.as_str()).collect();
    let interface: Vec<&str> = rows.iter().filter(|r| r.3).map(|r| r.0.as_str()).collect();
    assert_eq!(set_of(&prose), RuleSet::PROSE);
    assert_eq!(set_of(&interface), RuleSet::INTERFACE);
}

#[test]
fn set_composition_matches_section_9_2() {
    assert_eq!(RuleSet::PROSE.len(), 27);
    assert_eq!(RuleSet::PROSE, RuleSet::ALL, "PROSE equals ALL in 0.1.0");
    assert_eq!(RuleSet::INTERFACE.len(), 13);
    let Some(text) = design("09-rule-sets.md") else {
        return;
    };
    let start = text.find("`INTERFACE = PROSE − {").unwrap();
    let paragraph = &text[start..];
    let paragraph = &paragraph[..paragraph.find("\n\n").unwrap()];
    let open = paragraph.find('{').unwrap();
    let close = paragraph.find('}').unwrap();
    let removed: Vec<&str> = paragraph[open + 1..close]
        .split(',')
        .map(str::trim)
        .collect();
    assert_eq!(
        RuleSet::PROSE.difference(set_of(&removed)),
        RuleSet::INTERFACE
    );
    let listed = &paragraph[paragraph.find("rules):").unwrap()..];
    let listed = backticked(listed);
    assert_eq!(listed.len(), 13);
    assert_eq!(set_of(&listed), RuleSet::INTERFACE);
}

/// The ids in backticks after "Rules:" in the rustdoc of a constant of `src/rule.rs`.
fn rustdoc_rules(src: &str, item: &str) -> Vec<String> {
    let at = src.find(item).unwrap_or_else(|| panic!("{item} not found"));
    let doc: Vec<&str> = src[..at]
        .lines()
        .rev()
        .skip(1)
        .take_while(|l| l.trim_start().starts_with("///"))
        .collect();
    let doc: String = doc
        .into_iter()
        .rev()
        .map(|l| l.trim_start().trim_start_matches("///").trim())
        .collect::<Vec<_>>()
        .join("\n");
    let rules = &doc[doc.find("Rules:").unwrap()..];
    let rules = &rules[..rules.find("\n\n").unwrap_or(rules.len())];
    backticked(rules).into_iter().map(str::to_owned).collect()
}

#[test]
fn set_rustdoc_lists_their_rules() {
    let src = read(&crate_dir().join("src/rule.rs"));
    for (item, set) in [
        ("pub const PROSE", RuleSet::PROSE),
        ("pub const INTERFACE", RuleSet::INTERFACE),
    ] {
        let listed = rustdoc_rules(&src, item);
        let ids: Vec<&str> = listed.iter().map(String::as_str).collect();
        let expected: Vec<&str> = set.iter().map(|r| r.id()).collect();
        assert_eq!(ids, expected, "{item}");
    }
}

/// The rule ids named in the rustdoc of each public method of `TypographerBuilder`.
fn builder_docs() -> BTreeMap<String, BTreeSet<String>> {
    let src = read(&crate_dir().join("src/typographer.rs"));
    let start = src.find("impl TypographerBuilder {").unwrap();
    let mut docs = BTreeMap::new();
    let mut doc = Vec::new();
    for line in src[start..].lines().skip(1) {
        if line == "}" {
            break;
        }
        let line = line.trim();
        if let Some(text) = line.strip_prefix("///") {
            doc.push(text.to_owned());
        } else if let Some(rest) = line.strip_prefix("pub fn ") {
            let name = rest[..rest.find(['(', '<']).unwrap()].to_owned();
            let ids = doc
                .iter()
                .flat_map(|l| {
                    backticked(l)
                        .into_iter()
                        .map(str::to_owned)
                        .collect::<Vec<_>>()
                })
                .filter(|id| Rule::from_id(id).is_some())
                .collect();
            docs.insert(name, ids);
            doc.clear();
        } else if !line.starts_with("#[") {
            doc.clear();
        }
    }
    docs
}

#[test]
fn builder_rustdoc_names_the_rules_of_each_setting() {
    let docs = builder_docs();
    assert!(docs.contains_key("protect_between"), "{docs:?}");
    let Some(rows) = rule_table() else { return };
    let mut expected: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    for (id, settings, _, _) in &rows {
        for setting in settings {
            expected
                .entry(setting.clone())
                .or_default()
                .insert(id.clone());
        }
    }
    for setting in expected.keys() {
        assert!(docs.contains_key(setting), "no builder method {setting}");
    }
    for (method, ids) in &docs {
        if method == "build" {
            continue;
        }
        let want = expected.get(method).cloned().unwrap_or_default();
        assert_eq!(ids, &want, "rustdoc of {method}");
    }
}

/// The identifiers and punctuation of Rust source with comments, strings and character literals
/// replaced by spaces.
fn strip_rust(src: &str) -> String {
    let chars: Vec<char> = src.chars().collect();
    let mut out = String::with_capacity(src.len());
    let mut i = 0;
    let is_ident = |c: char| c.is_alphanumeric() || c == '_';
    while i < chars.len() {
        let c = chars[i];
        let next = chars.get(i + 1).copied();
        if c == '/' && next == Some('/') {
            while i < chars.len() && chars[i] != '\n' {
                i += 1;
            }
        } else if c == '/' && next == Some('*') {
            let mut depth = 0;
            while i < chars.len() {
                if chars[i] == '/' && chars.get(i + 1) == Some(&'*') {
                    depth += 1;
                    i += 2;
                } else if chars[i] == '*' && chars.get(i + 1) == Some(&'/') {
                    depth -= 1;
                    i += 2;
                    if depth == 0 {
                        break;
                    }
                } else {
                    i += 1;
                }
            }
            out.push(' ');
        } else if c == 'r'
            && (i == 0 || !is_ident(chars[i - 1]))
            && matches!(next, Some('"' | '#'))
            && chars[i + 1..].iter().find(|c| **c != '#') == Some(&'"')
        {
            let hashes = chars[i + 1..].iter().take_while(|c| **c == '#').count();
            i += 2 + hashes;
            loop {
                if i >= chars.len() {
                    break;
                }
                if chars[i] == '"'
                    && chars[i + 1..]
                        .iter()
                        .take(hashes)
                        .filter(|c| **c == '#')
                        .count()
                        == hashes
                {
                    i += 1 + hashes;
                    break;
                }
                i += 1;
            }
            out.push(' ');
        } else if c == '"' {
            i += 1;
            while i < chars.len() && chars[i] != '"' {
                if chars[i] == '\\' {
                    i += 1;
                }
                i += 1;
            }
            i += 1;
            out.push(' ');
        } else if c == '\'' {
            if next == Some('\\') {
                i += 2;
                while i < chars.len() && chars[i] != '\'' {
                    i += 1;
                }
                i += 1;
                out.push(' ');
            } else if chars.get(i + 2) == Some(&'\'') {
                i += 3;
                out.push(' ');
            } else {
                // A lifetime: the quote alone.
                i += 1;
                out.push(' ');
            }
        } else {
            out.push(c);
            i += 1;
        }
    }
    out
}

fn identifiers(code: &str) -> BTreeSet<&str> {
    code.split(|c: char| !(c.is_alphanumeric() || c == '_'))
        .filter(|s| !s.is_empty())
        .collect()
}

fn rust_files(dir: &Path, out: &mut Vec<PathBuf>) {
    for entry in fs::read_dir(dir).unwrap() {
        let path = entry.unwrap().path();
        if path.is_dir() {
            rust_files(&path, out);
        } else if path.extension().is_some_and(|e| e == "rs") {
            out.push(path);
        }
    }
}

#[test]
fn stripping_keeps_only_code() {
    let code = strip_rust(
        "let a = \"fn if\"; // impl\n/* match /* nested */ unsafe */ 'x' '\\n' r#\"if\"# x<'a>",
    );
    let ids = identifiers(&code);
    assert!(
        ids.contains("let") && ids.contains("a") && ids.contains("x"),
        "{code}"
    );
    for word in ["fn", "if", "impl", "match", "unsafe", "n"] {
        assert!(!ids.contains(word), "{word} in {code}");
    }
}

#[test]
fn source_reads_no_environment_and_has_no_unsafe() {
    let mut files = Vec::new();
    rust_files(&crate_dir().join("src"), &mut files);
    assert!(files.len() > 5);
    for path in files {
        let code = strip_rust(&read(&path));
        for forbidden in ["std::env", "std::fs", "std::net", "std::time"] {
            assert!(
                !code.contains(forbidden),
                "{forbidden} in {}",
                path.display()
            );
        }
        assert!(
            !identifiers(&code).contains("unsafe"),
            "unsafe in {}",
            path.display()
        );
    }
}

/// The lints that every test target allows (design section 14.5).
const TEST_ALLOWS: [&str; 7] = [
    "clippy::unwrap_used",
    "clippy::expect_used",
    "clippy::panic",
    "clippy::indexing_slicing",
    "clippy::print_stdout",
    "clippy::print_stderr",
    "clippy::dbg_macro",
];

/// The lints of every `#![allow(...)]` of a file stripped by `strip_rust`, in order.
fn inner_allows(src: &str) -> Vec<String> {
    src.split("#![allow(")
        .skip(1)
        .flat_map(|rest| {
            rest.split_once(")]")
                .map_or("", |(list, _)| list)
                .split(',')
        })
        .map(str::trim)
        .filter(|lint| !lint.is_empty())
        .map(String::from)
        .collect()
}

#[test]
fn test_targets_allow_the_lints_of_section_14_5() {
    let mut targets = 0;
    for entry in fs::read_dir(crate_dir().join("tests")).unwrap() {
        let path = entry.unwrap().path();
        if path.extension().is_some_and(|e| e == "rs") {
            targets += 1;
            let mut allows = inner_allows(&strip_rust(&read(&path)));
            // Only the counting allocator may use `unsafe` (design section 14.5).
            if path.file_name().is_some_and(|name| name == "alloc.rs") {
                assert_eq!(allows.first().map(String::as_str), Some("unsafe_code"));
                allows.remove(0);
            }
            assert_eq!(allows, TEST_ALLOWS, "{}", path.display());
        }
    }
    assert!(targets > 5);
    let common = crate_dir().join("tests/common/mod.rs");
    assert_eq!(inner_allows(&strip_rust(&read(&common))), ["dead_code"]);
}

#[test]
fn language_tables_hold_no_logic() {
    for file in ["ru_ru.rs", "en_us.rs", "common.rs"] {
        let path = crate_dir().join("src/lang").join(file);
        let code = strip_rust(&read(&path));
        let ids = identifiers(&code);
        for keyword in ["fn", "impl", "match", "if"] {
            assert!(!ids.contains(keyword), "{keyword} in {file}");
        }
    }
}

#[test]
fn built_in_languages_and_sets_build() {
    for lang in Language::builtin().chain([Language::COMMON]) {
        for set in [
            RuleSet::PROSE,
            RuleSet::INTERFACE,
            RuleSet::EMPTY,
            RuleSet::ALL,
        ] {
            let built = Typographer::builder(lang.clone(), set).build();
            assert!(built.is_ok(), "{lang} {set}: {built:?}");
            assert_eq!(built.unwrap().rules(), set);
        }
    }
}
