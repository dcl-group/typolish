//! Documentation of the package (design sections 13.10 and 15.4, stage 5): the rustdoc example on
//! every public item, the README and its known limits, and the CHANGELOG.
//!
//! The README's code blocks run as doctests through the `#[cfg(doctest)]` item of `lib.rs`; this
//! target checks what a doctest cannot: that every block is run, that the README and the CHANGELOG
//! are English and agree with the code on the numbers they state, that every link points at a file,
//! and that the known limits of design section 10.8 are what the crate does.
//!
//! The README is read through the crate's own `README.md`, which the package holds. The checks
//! that read files only the repository has (the CHANGELOG, `docs/design/`, the targets of links)
//! skip outside it, so that `cargo test` passes in the unpacked package too (design section 13.10).

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

use std::borrow::Cow;
use std::fs;
use std::path::{Path, PathBuf};

use common::{interface, prose, show, vis};
use typolish::{
    DiagnosticKind, HtmlStyle, Language, Output, Piece, Rule, RuleSet, Severity, Typographer,
};

fn crate_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn repo_dir() -> PathBuf {
    crate_dir().join("../..")
}

fn read(path: &Path) -> String {
    fs::read_to_string(path).unwrap_or_else(|e| panic!("{}: {e}", path.display()))
}

/// The root of the repository, or `None` (with a note) when the crate is not inside it, as in the
/// unpacked package: the crate is the repository's `dev/typolish/` and the design is there.
fn repository() -> Option<PathBuf> {
    let repo = repo_dir();
    let inside = repo.join("docs/design/README.md").is_file()
        && repo.join("dev/typolish").canonicalize().ok() == crate_dir().canonicalize().ok();
    if inside {
        Some(repo)
    } else {
        eprintln!(
            "skipped: {} is not in the repository",
            crate_dir().display()
        );
        None
    }
}

/// The README of the crate: a link to the root README in the repository, a copy in the package.
fn readme() -> String {
    read(&crate_dir().join("README.md"))
}

/// The CHANGELOG, which only the repository has.
fn changelog() -> Option<String> {
    repository().map(|repo| read(&repo.join("CHANGELOG.md")))
}

/// A file of `docs/design/`, or `None` outside the repository.
fn design(file: &str) -> Option<String> {
    repository().map(|repo| read(&repo.join("docs/design").join(file)))
}

/// The text of a Markdown section: from the heading line that starts with `heading` to the next
/// heading of the same or a higher level.
fn section(text: &str, heading: &str) -> String {
    let level = heading.chars().take_while(|c| *c == '#').count();
    let mut out = String::new();
    let mut inside = false;
    let mut fenced = false;
    for line in text.lines() {
        if line.starts_with("```") {
            fenced = !fenced;
        }
        let hashes = line.chars().take_while(|c| *c == '#').count();
        let is_heading = !fenced && hashes > 0 && line[hashes..].starts_with(' ');
        if is_heading && line.starts_with(heading) {
            inside = true;
            continue;
        }
        if inside && is_heading && hashes <= level {
            break;
        }
        if inside {
            out.push_str(line);
            out.push('\n');
        }
    }
    assert!(inside, "no section {heading:?}");
    out
}

/// The spans between single backticks of a Markdown text (double-backtick spans are skipped).
fn code_spans(text: &str) -> Vec<String> {
    let mut spans = Vec::new();
    let mut fenced = false;
    for line in text.lines() {
        if line.trim_start().starts_with("```") {
            fenced = !fenced;
            continue;
        }
        if fenced || line.contains("``") {
            continue;
        }
        spans.extend(line.split('`').skip(1).step_by(2).map(str::to_string));
    }
    spans
}

/// The text of a Markdown file without fenced blocks and inline code.
fn prose_text(text: &str) -> String {
    let mut out = String::new();
    let mut fenced = false;
    for line in text.lines() {
        if line.trim_start().starts_with("```") {
            fenced = !fenced;
            continue;
        }
        if fenced {
            continue;
        }
        let mut in_code = false;
        for c in line.chars() {
            if c == '`' {
                in_code = !in_code;
            } else if !in_code {
                out.push(c);
            }
        }
        out.push('\n');
    }
    out
}

fn is_cyrillic(c: char) -> bool {
    ('\u{400}'..='\u{52f}').contains(&c)
}

// ---------------------------------------------------------------------------------------------
// Rustdoc: an example on every public item (design section 13.10).

fn rust_files(dir: &Path, out: &mut Vec<PathBuf>) {
    let mut entries: Vec<_> = fs::read_dir(dir)
        .unwrap()
        .map(|e| e.unwrap().path())
        .collect();
    entries.sort();
    for path in entries {
        if path.is_dir() {
            rust_files(&path, out);
        } else if path.extension().is_some_and(|e| e == "rs") {
            out.push(path);
        }
    }
}

/// The `pub` items of a source file (types, functions, methods, constants, associated constants)
/// with their doc blocks. Enum variants and struct fields do not start with `pub` plus an item
/// keyword, so they are not items here; `public_variants` reads the variants of public enums.
struct Item {
    file: String,
    line: usize,
    kind: &'static str,
    name: String,
    doc: Vec<String>,
}

fn public_items() -> Vec<Item> {
    let mut files = Vec::new();
    rust_files(&crate_dir().join("src"), &mut files);
    let mut items = Vec::new();
    for file in files {
        let src = read(&file);
        let mut doc: Vec<String> = Vec::new();
        for (i, line) in src.lines().enumerate() {
            let t = line.trim();
            if t.starts_with("///") {
                doc.push(t.trim_start_matches('/').to_string());
                continue;
            }
            if t.starts_with("#[") {
                continue;
            }
            let rest = t.strip_prefix("pub ");
            if let Some(rest) = rest {
                let rest = rest.strip_prefix("const ").unwrap_or(rest);
                let rest = rest.strip_prefix("unsafe ").unwrap_or(rest);
                for kind in ["fn", "const", "struct", "enum", "trait", "type", "static"] {
                    if let Some(after) = rest.strip_prefix(kind)
                        && after.starts_with(' ')
                    {
                        let name: String = after
                            .trim_start()
                            .chars()
                            .take_while(|c| c.is_alphanumeric() || *c == '_')
                            .collect();
                        items.push(Item {
                            file: file
                                .strip_prefix(crate_dir())
                                .unwrap()
                                .display()
                                .to_string(),
                            line: i + 1,
                            kind,
                            name,
                            doc: std::mem::take(&mut doc),
                        });
                        break;
                    }
                }
            }
            doc.clear();
        }
    }
    items
}

#[test]
fn every_public_item_has_an_example() {
    let items = public_items();
    assert!(items.len() > 150, "only {} public items found", items.len());
    let missing: Vec<String> = items
        .iter()
        .filter(|item| !item.doc.iter().any(|l| l.trim_start().starts_with("```")))
        .map(|item| format!("{}:{} {} {}", item.file, item.line, item.kind, item.name))
        .collect();
    assert!(
        missing.is_empty(),
        "public items without an example:\n{}",
        missing.join("\n")
    );
}

/// The variants of the `pub enum`s of `src/` with their doc blocks. A variant is a line of code at
/// brace depth 1 of the enum's body; doc lines, comments and attributes do not count braces, so
/// `{n}` in a doc or `{}` in an `#[error]` string does not shift the depth.
fn public_variants() -> Vec<Item> {
    let mut files = Vec::new();
    rust_files(&crate_dir().join("src"), &mut files);
    let mut variants = Vec::new();
    for file in files {
        let src = read(&file);
        let name = file
            .strip_prefix(crate_dir())
            .unwrap()
            .display()
            .to_string();
        let mut depth: Option<usize> = None;
        let mut doc: Vec<String> = Vec::new();
        for (i, line) in src.lines().enumerate() {
            let t = line.trim();
            let Some(d) = depth else {
                if t.starts_with("pub enum ") && t.ends_with('{') {
                    depth = Some(1);
                    doc.clear();
                }
                continue;
            };
            if let Some(text) = t.strip_prefix("///") {
                doc.push(text.to_string());
                continue;
            }
            if t.is_empty() || t.starts_with("//") || t.starts_with("#[") {
                continue;
            }
            if d == 1 && t.starts_with(|c: char| c.is_ascii_uppercase()) {
                variants.push(Item {
                    file: name.clone(),
                    line: i + 1,
                    kind: "variant",
                    name: t
                        .chars()
                        .take_while(|c| c.is_alphanumeric() || *c == '_')
                        .collect(),
                    doc: std::mem::take(&mut doc),
                });
            }
            doc.clear();
            let opens = t.matches('{').count();
            let closes = t.matches('}').count();
            depth = (d + opens).checked_sub(closes).filter(|&d| d > 0);
        }
    }
    variants
}

#[test]
fn every_public_enum_variant_has_an_example() {
    // Design sections 13.10 and 15.2, item 29: every variant of a public enum has an example of
    // its own.
    let variants = public_variants();
    assert!(
        variants.len() >= 99,
        "only {} public enum variants found",
        variants.len()
    );
    let missing: Vec<String> = variants
        .iter()
        .filter(|v| !v.doc.iter().any(|l| l.trim_start().starts_with("```")))
        .map(|v| format!("{}:{} {} {}", v.file, v.line, v.kind, v.name))
        .collect();
    assert!(
        missing.is_empty(),
        "public enum variants without an example:\n{}",
        missing.join("\n")
    );
}

#[test]
fn every_example_runs_as_a_doctest() {
    // An example marked `ignore`, `no_run`, `compile_fail` or with another language is not run.
    let mut files = Vec::new();
    rust_files(&crate_dir().join("src"), &mut files);
    let mut bad = Vec::new();
    for file in files {
        for (i, line) in read(&file).lines().enumerate() {
            let t = line.trim();
            let doc = t.strip_prefix("//!").or_else(|| t.strip_prefix("///"));
            if let Some(doc) = doc
                && let Some(info) = doc.trim_start().strip_prefix("```")
                && !matches!(info.trim(), "" | "rust")
            {
                bad.push(format!("{}:{}: ```{}", file.display(), i + 1, info));
            }
        }
    }
    assert!(
        bad.is_empty(),
        "examples that are not run:\n{}",
        bad.join("\n")
    );
}

#[test]
fn the_lazy_displays_and_the_reports_are_must_use() {
    // Design sections 2.7 and 2.9: discarding a lazy display or a report is a warning. The
    // attribute stands among the attributes right above the item; an example marked
    // `compile_fail` would not run (see above), so the source is read.
    for (file, item) in [
        ("number.rs", "pub struct NumberDisplay<"),
        ("number.rs", "pub struct RangeDisplay<"),
        ("typographer.rs", "pub fn report<"),
        ("typographer.rs", "pub fn report_template<"),
    ] {
        let source = read(&crate_dir().join("src").join(file));
        let lines: Vec<&str> = source.lines().map(str::trim).collect();
        let at = lines
            .iter()
            .position(|l| l.starts_with(item))
            .unwrap_or_else(|| panic!("{file}: no `{item}`"));
        let marked = lines[..at]
            .iter()
            .rev()
            .take_while(|l| l.starts_with("#["))
            .any(|l| l.starts_with("#[must_use"));
        assert!(marked, "{file}: `{item}` is not `#[must_use]`");
    }
}

#[test]
fn the_crate_has_its_own_example_and_the_readme_is_run() {
    let lib = read(&crate_dir().join("src/lib.rs"));
    assert!(
        lib.lines()
            .filter(|l| l.starts_with("//!"))
            .any(|l| l.contains("```")),
        "lib.rs has no example in its crate docs"
    );
    assert!(lib.contains("#![forbid(unsafe_code)]"));
    // Design section 5.5: the crate docs say that the crate does not normalize Unicode.
    assert!(
        lib.lines()
            .filter(|l| l.starts_with("//!"))
            .any(|l| l.contains("does not normalize Unicode")),
        "the crate docs of lib.rs do not state that there is no normalization"
    );
    assert!(
        !lib.contains("gets no no-break space; nothing else"),
        "the crate docs of lib.rs again say that an NFD word gets no glue at all (design section 5.5)"
    );
    // Design section 13.10: the code blocks of the README run through a `#[cfg(doctest)]` item.
    let compact: String = lib.split_whitespace().collect();
    assert!(
        compact.contains("#[cfg(doctest)]#[doc=include_str!(\"../README.md\")]"),
        "lib.rs does not include the README under cfg(doctest)"
    );
}

// ---------------------------------------------------------------------------------------------
// README (design section 13.10).

#[test]
fn readme_blocks_are_run_and_there_are_some() {
    let text = readme();
    let mut fences = Vec::new();
    for line in text.lines() {
        if let Some(info) = line.trim_start().strip_prefix("```") {
            fences.push(info.trim().to_string());
        }
    }
    assert_eq!(fences.len() % 2, 0, "an unclosed code block");
    let openings: Vec<&String> = fences.iter().step_by(2).collect();
    assert!(openings.len() >= 5, "the README shows too little code");
    for info in openings {
        assert_eq!(
            info, "rust",
            "a README block that doctests do not run: ```{info}"
        );
    }
}

#[test]
fn readme_states_what_section_13_10_requires() {
    let text = readme();
    assert!(text.starts_with("# typolish\n"));
    // The known limits of design section 10.8, and how to protect commands.
    let limits = section(&text, "## Known limits");
    assert!(limits.lines().filter(|l| l.starts_with("- ")).count() >= 8);
    assert!(
        limits.contains("10.8"),
        "the limits do not point at design section 10.8"
    );
    let protect = section(&text, "## Protecting commands and code");
    for word in [
        "backtick",
        "protect_between",
        "Piece::Protected",
        "protect_backticks",
    ] {
        assert!(
            protect.contains(word),
            "the protection section lacks {word}"
        );
    }
    assert!(
        limits.contains("protected with backticks or") && limits.contains("Piece::Protected"),
        "the first limit does not say how to protect commands"
    );
    // The license line names both licenses and links both files.
    let license = section(&text, "## License");
    assert!(license.contains("MIT") && license.contains("Apache-2.0"));
    assert!(license.contains("./LICENSE-MIT") && license.contains("./LICENSE-APACHE"));
}

#[test]
fn readme_numbers_agree_with_the_code() {
    let text = readme();
    // "27 rules": the number of rules the crate has.
    assert!(
        text.contains(&format!("- {} rules with stable ids", Rule::ALL.len())),
        "the README does not state the rule count {}",
        Rule::ALL.len()
    );
    assert_eq!(Rule::ALL.len(), 27);
    // The version and the minimum supported Rust version.
    let version = env!("CARGO_PKG_VERSION");
    assert!(text.contains(&format!("Version {version}")));
    let msrv = env!("CARGO_PKG_RUST_VERSION");
    assert_eq!(msrv, "1.100");
    assert!(text.contains(&format!("version is {msrv}")));
    // Prose and Interface counts of the changelog (design section 9.2).
    assert_eq!(RuleSet::PROSE.len(), 27);
    assert_eq!(RuleSet::INTERFACE.len(), 13);
}

#[test]
fn links_point_at_files_of_the_repository() {
    let (Some(repo), Some(changelog)) = (repository(), changelog()) else {
        return;
    };
    let prefix = "https://github.com/dcl-group/typolish/";
    for (name, text) in [("README.md", readme()), ("CHANGELOG.md", changelog)] {
        let mut links = Vec::new();
        let mut rest = text.as_str();
        while let Some(at) = rest.find("](") {
            let tail = &rest[at + 2..];
            let end = tail.find(')').unwrap();
            links.push(tail[..end].to_string());
            rest = &tail[end..];
        }
        for link in links {
            let path = if let Some(p) = link.strip_prefix("./") {
                p.to_string()
            } else if let Some(p) = link.strip_prefix(prefix).and_then(|p| {
                p.strip_prefix("blob/main/")
                    .or_else(|| p.strip_prefix("tree/main/"))
            }) {
                p.to_string()
            } else {
                continue;
            };
            let path = path.split('#').next().unwrap();
            assert!(
                repo.join(path).exists(),
                "{name}: the link {link} points at a missing path"
            );
        }
    }
}

#[test]
fn readme_and_changelog_are_english() {
    // Russian appears in examples only: inside code blocks and code spans.
    let mut texts = vec![("README.md", readme())];
    texts.extend(changelog().map(|text| ("CHANGELOG.md", text)));
    for (name, text) in texts {
        for (i, line) in prose_text(&text).lines().enumerate() {
            assert!(
                !line.chars().any(is_cyrillic),
                "{name}:{}: Cyrillic outside code: {line}",
                i + 1
            );
        }
    }
}

#[test]
fn texts_are_american_english() {
    // AGENTS.md: README, rustdoc, comments and the design are in American English.
    let british = [
        "behaviour",
        "colour",
        "licence",
        "normalis",
        "initialis",
        "recognis",
        "organis",
        "serialis",
        "optimis",
        "minimis",
        "analyse",
        "neighbour",
        "favour",
        "honour",
        "centre ",
        "catalogue",
        "artefact",
    ];
    let mut files = vec![crate_dir().join("README.md")];
    if let Some(repo) = repository() {
        files.push(repo.join("CHANGELOG.md"));
        let mut design: Vec<PathBuf> = fs::read_dir(repo.join("docs/design"))
            .unwrap()
            .map(|e| e.unwrap().path())
            .collect();
        assert!(design.len() > 10, "only {} design files", design.len());
        design.sort();
        files.extend(design);
    }
    // Every Rust file of the crate; this file holds the list of British words and is skipped.
    let mut sources = Vec::new();
    for dir in ["src", "tests", "examples", "benches"] {
        rust_files(&crate_dir().join(dir), &mut sources);
    }
    let this = crate_dir().join("tests/documentation.rs");
    assert!(sources.len() > 80, "only {} Rust files", sources.len());
    files.extend(sources.into_iter().filter(|p| *p != this));
    let mut found = Vec::new();
    for file in files {
        for (i, line) in read(&file).lines().enumerate() {
            let lower = line.to_lowercase();
            for word in british {
                if lower.contains(word) {
                    found.push(format!("{}:{}: {word:?}", file.display(), i + 1));
                }
            }
        }
    }
    assert!(found.is_empty(), "British spelling:\n{}", found.join("\n"));
}

/// Whether the character at `i` starts an identifier-like token, or follows the `b` prefix of a
/// byte string that does.
fn starts_token(chars: &[char], i: usize) -> bool {
    let ident = |k: usize| chars[k].is_alphanumeric() || chars[k] == '_';
    i == 0 || !ident(i - 1) || (chars[i - 1] == 'b' && (i == 1 || !ident(i - 2)))
}

/// The comments of a Rust source as `(line, text)`, one entry per comment: the text after `//`
/// (and after the `/` or `!` of a doc comment) of each line, with the lines of a run of whole-line
/// comments joined into one entry that carries the line number of each word. `//` inside a string
/// literal, a raw string literal or a character literal does not start a comment.
fn comments(source: &str) -> Vec<Vec<(usize, String)>> {
    let mut out: Vec<Vec<(usize, String)>> = Vec::new();
    // `Some(hashes)` inside a raw string closed by `"` and that many `#`; `Some(usize::MAX)`
    // inside an ordinary string.
    let mut string: Option<usize> = None;
    let mut run_open = false;
    for (index, line) in source.lines().enumerate() {
        let chars: Vec<char> = line.chars().collect();
        let mut comment = None;
        let mut i = 0;
        while i < chars.len() {
            let c = chars[i];
            match string {
                Some(usize::MAX) => {
                    if c == '\\' {
                        i += 1;
                    } else if c == '"' {
                        string = None;
                    }
                }
                Some(hashes) => {
                    if c == '"'
                        && chars[i + 1..]
                            .iter()
                            .take(hashes)
                            .filter(|h| **h == '#')
                            .count()
                            == hashes
                    {
                        string = None;
                        i += hashes;
                    }
                }
                None => {
                    if c == '/' && chars.get(i + 1) == Some(&'/') {
                        comment = Some(i);
                        break;
                    } else if c == '"' {
                        string = Some(usize::MAX);
                    } else if c == 'r' && starts_token(&chars, i) {
                        let hashes = chars[i + 1..].iter().take_while(|h| **h == '#').count();
                        if chars.get(i + 1 + hashes) == Some(&'"') {
                            string = Some(hashes);
                            i += 1 + hashes;
                        }
                    } else if c == '\'' {
                        // A character literal such as `'"'` or `'\''`; a lifetime has no closing quote.
                        if chars.get(i + 1) == Some(&'\\') {
                            if let Some(end) = chars[i + 2..].iter().position(|q| *q == '\'') {
                                i += 2 + end;
                            }
                        } else if chars.get(i + 2) == Some(&'\'') {
                            i += 2;
                        }
                    }
                }
            }
            i += 1;
        }
        let Some(start) = comment else {
            run_open = false;
            continue;
        };
        let whole_line = chars[..start].iter().all(|c| c.is_whitespace());
        let mut text: String = chars[start + 2..].iter().collect();
        if text.starts_with('/') || text.starts_with('!') {
            text.remove(0);
        }
        let words: Vec<(usize, String)> = text
            .split_whitespace()
            .map(|w| (index + 1, w.to_string()))
            .collect();
        match out.last_mut() {
            Some(run) if run_open && whole_line => run.extend(words),
            _ => out.push(words),
        }
        run_open = whole_line;
    }
    out
}

#[test]
fn design_references_say_design() {
    // AGENTS.md: code and tests refer to a place in the design as "design section N.M". In every
    // comment of the crate's Rust files, the word `section` or `sections` (in any case, after an
    // opening parenthesis too) that a number follows comes right after the word `design`, also when
    // a comment line ends between the two words. String literals and code are not read.
    if repository().is_none() {
        return;
    }
    let mut sources = Vec::new();
    for dir in ["src", "tests", "examples", "benches"] {
        rust_files(&crate_dir().join(dir), &mut sources);
    }
    assert!(sources.len() > 80, "only {} Rust files", sources.len());
    let bare = |word: &str| -> String {
        word.trim_start_matches(['(', '[', '`', '"'])
            .trim_end_matches(|c: char| !c.is_alphanumeric())
            .to_lowercase()
    };
    let mut found = Vec::new();
    let mut checked = 0;
    for file in sources {
        for run in comments(&read(&file)) {
            for (k, (line, word)) in run.iter().enumerate() {
                let word = bare(word);
                if word != "section" && word != "sections" {
                    continue;
                }
                let Some((_, next)) = run.get(k + 1) else {
                    continue;
                };
                if !next.starts_with(|c: char| c.is_ascii_digit()) {
                    continue;
                }
                checked += 1;
                let after_design = k > 0 && bare(&run[k - 1].1) == "design";
                if !after_design {
                    found.push(format!("{}:{line}", file.display()));
                }
            }
        }
    }
    assert!(checked > 500, "only {checked} design references");
    assert!(
        found.is_empty(),
        "references to the design without the word `design`:\n{}",
        found.join("\n")
    );
}

// ---------------------------------------------------------------------------------------------
// CHANGELOG (design sections 15.2 and 15.4: a prepared entry of the version of `Cargo.toml` first,
// and the 0.1.1 entry, the first one, naming every rule).

fn changelog_entry(text: &str, version: &str) -> String {
    let mut out = String::new();
    let mut inside = false;
    for line in text.lines() {
        if line.starts_with("## ") {
            if inside {
                break;
            }
            inside = line.starts_with(&format!("## {version}"));
            continue;
        }
        if inside {
            out.push_str(line);
            out.push('\n');
        }
    }
    assert!(inside, "no entry for {version}");
    out
}

#[test]
fn changelog_has_the_prepared_entry() {
    let Some(text) = changelog() else {
        return;
    };
    assert!(text.starts_with("# Changelog\n"));
    assert!(text.contains("keepachangelog.com"));
    assert!(text.contains("semver.org"));
    let headings: Vec<&str> = text.lines().filter(|l| l.starts_with("## ")).collect();
    let version = env!("CARGO_PKG_VERSION");
    assert_eq!(version, "0.1.2");
    assert_eq!(
        headings.first().copied(),
        Some(format!("## {version} - unreleased").as_str()),
        "the first entry is the version of Cargo.toml, prepared and not released"
    );
    assert_eq!(
        headings.get(1).copied(),
        Some("## 0.1.1 - unreleased"),
        "the second entry is the first version"
    );
    assert_eq!(headings.len(), 2, "no other versions: {headings:?}");
    for v in [version, "0.1.1"] {
        let entry = changelog_entry(&text, v);
        assert!(entry.contains("### Added"), "the {v} entry has Added");
        assert!(
            entry.contains("1.100"),
            "the {v} entry names the minimum Rust version"
        );
        assert!(
            entry.contains("not published"),
            "a prepared entry says that nothing is published yet"
        );
    }
}

#[test]
fn changelog_names_every_rule_and_its_counts() {
    let Some(text) = changelog() else {
        return;
    };
    // The first version introduces every rule; later entries list only what changed.
    let entry = changelog_entry(&text, "0.1.1");
    for rule in Rule::ALL {
        assert!(
            entry.contains(&format!("`{}`", rule.id())),
            "the 0.1.1 entry does not name the rule {}",
            rule.id()
        );
    }
    assert!(entry.contains(&format!("{} rules", Rule::ALL.len())));
    assert!(entry.contains(&format!(
        "{} rules that cannot alter",
        RuleSet::INTERFACE.len()
    )));
    // The entry names the public entry points of the three inputs and the three outputs.
    for name in [
        "Typographer",
        "typeset",
        "typeset_template",
        "format_number",
        "format_range",
        "report",
        "lint",
        "to_plain",
        "Output::Unicode",
        "Output::Html",
        "Output::Escaped",
        "LanguageBuilder",
        "RuleSet::PROSE",
        "RuleSet::INTERFACE",
        "RuleSet::ALL",
        "RuleSet::EMPTY",
        "Language::from_data",
        "Language::from_static",
    ] {
        assert!(
            entry.contains(name),
            "the 0.1.1 entry does not mention {name}"
        );
    }
}

// ---------------------------------------------------------------------------------------------
// The known limits of design section 10.8: what the README says is what the crate does.

#[derive(Clone, Copy)]
enum Sets {
    Both,
    ProseOnly,
}

struct Limit {
    lang: &'static Language,
    sets: Sets,
    input: &'static str,
    expected: &'static str,
}

const fn limit(
    lang: &'static Language,
    sets: Sets,
    input: &'static str,
    expected: &'static str,
) -> Limit {
    Limit {
        lang,
        sets,
        input,
        expected,
    }
}

const RU: &Language = &Language::RU_RU;
const EN: &Language = &Language::EN_US;

fn limits() -> Vec<Limit> {
    use Sets::{Both, ProseOnly};
    vec![
        // A lone `-` between two words of the language is a dash in both sets (en), and stays
        // between foreign words (ru).
        limit(EN, Both, "cat - file", "cat—file"),
        limit(EN, Both, "tls-server - agent-ca", "tls-server — agent-ca"),
        limit(RU, Both, "cat - file", "cat - file"),
        limit(RU, Both, "tls-server - agent-ca", "tls-server - agent-ca"),
        limit(EN, Both, "word - word", "word—word"),
        // A capital letter with a full stop at a sentence end.
        limit(RU, Both, "вариант А. После", "вариант А.⍽После"),
        limit(EN, Both, "plan B. Then", "plan B.⍽Then"),
        limit(EN, Both, "Neither did I. Then", "Neither did I.⍽Then"),
        limit(RU, Both, "220 В. Затем", "220⍽В.⍽Затем"),
        // Single quotes.
        limit(EN, Both, "rock 'n' roll", "rock 'n' roll"),
        limit(EN, Both, "rock'n'roll", "rock’n’roll"),
        // Prose assumes commands are protected; Interface has none of these rules.
        limit(EN, ProseOnly, "tail -5", "tail −5"),
        limit(EN, ProseOnly, "123-4567", "123–4567"),
        limit(EN, ProseOnly, "...args", "…args"),
        // A mention is frozen whole.
        limit(EN, Both, "@user", "@user"),
        limit(EN, Both, "Ask @user", "Ask @user"),
        // Swiss grouping and digit separators keep the apostrophe.
        limit(EN, Both, "Run 1'000'000 now", "Run 1'000'000 now"),
        limit(EN, Both, "Run 0b1'0000 now", "Run 0b1'0000 now"),
        // A number grouped with commas is a number only with one group (design section 10.8).
        limit(EN, Both, "1,234 kg", "1,234⍽kg"),
        limit(EN, Both, "1,234,567 kg", "1,234,567 kg"),
        limit(EN, Both, "1,000.5 kg", "1,000.5 kg"),
        limit(EN, Both, "-1,234,567 kg", "-1,234,567 kg"),
        // Frozen fragments are left alone.
        limit(RU, Both, "50%-ный", "50%-ный"),
        limit(EN, Both, "C#", "C#"),
        limit(EN, Both, "AT&T", "AT&T"),
        // A quote glued to a word by punctuation, and with a space.
        limit(RU, Both, "сказал:\"привет\"", "сказал:\"привет\""),
        limit(RU, Both, "сказал,\"привет\"", "сказал,\"привет\""),
        limit(RU, Both, "сказал: \"привет\"", "сказал: «привет»"),
        limit(RU, Both, "key: \"\"", "key: «»"),
        limit(RU, Both, "key=\"\"", "key=\"\""),
        // Sign-only pairs that share a quote: a defect, not idempotent.
        limit(RU, Both, "\"x ''-'' y\"", "«x ''-„“ y»"),
        limit(EN, Both, "\"x ''-'' y\"", "“x ''-‘’ y”"),
        limit(EN, Both, "\"\"\"x\n''-''\ne\"\"\"", "“‘“x\n''-‘’\ne”’”"),
        // A typed `‘` that the first pass leaves neutral and a second pass pairs: a defect, not
        // idempotent.
        limit(
            EN,
            Both,
            "He said ‘the dogs' \"5\"’",
            "He said ‘the⍽dogs' “5”’",
        ),
        limit(EN, Both, "x ‘a 'b' \"2\"’ y", "x ‘a⍽'b' “2”’ y"),
        // A new `‘` before a sign and a typed `’` is a sign-only pair on a second pass: a defect,
        // not idempotent.
        limit(EN, Both, "\"a '-’ - x' c\"", "“a⍽‘-’ - x’ c”"),
        // A possessive placed inside a typed pair closes it early on a second pass: a defect, not
        // idempotent.
        limit(EN, Both, "‘S' ‘s'’ s’", "‘S' ‘s’’ s’"),
        // A typed `’` that a straight pair covers on the first pass closes the pair on a second
        // pass: a defect, not idempotent.
        limit(EN, Both, "‘S' \"x’\" s’", "‘S' ‘x’’ s’"),
        // Windows whose plain form does not typeset back: a defect of `to_plain`, P2 fails.
        limit(RU, Both, "\"a '(\" x' \"y'\" b\"", "«a „(\" x“ „y'“ b»"),
        limit(EN, Both, "\"'S '5' 3'\"", "“‘S “5” 3’”"),
        // A quoted value with a space glued to `=`.
        limit(
            EN,
            Both,
            "Set title=\"My App\" for the \"main\" window.",
            "Set title=\"My App\" for⍽the⍽\"main\" window.",
        ),
        // A quote glued to a dash belongs to a frozen core; with a space the quotes are placed.
        limit(RU, Both, "он —\"Да\" и", "он —\"Да\" и"),
        limit(
            EN,
            Both,
            "He said \"no\"—and left.",
            "He said \"no\"—and left.",
        ),
        limit(RU, Both, "он — \"Да\" и", "он⍽— «Да» и"),
        // A unit attached to a number freezes it whole.
        limit(RU, ProseOnly, "Было -15°C, -15 °C", "Было -15°C, −15⍽°C"),
        // A time range with spaces is not a dash.
        limit(RU, Both, "Приём 10:00 - 12:00", "Приём 10:00 - 12:00"),
        limit(RU, ProseOnly, "Приём 10:00-12:00", "Приём 10:00–12:00"),
        // A chain of short words is glued whole.
        limit(RU, ProseOnly, "Он и я и ты и мы", "Он⍽и⍽я⍽и⍽ты⍽и⍽мы"),
        // A unit symbol that is also a word, a name or an initial is read as a unit after a number.
        limit(RU, Both, "Часть 3 В лесу", "Часть 3⍽В⍽лесу"),
        limit(
            RU,
            Both,
            "В 2001 Дж. Буш стал президентом.",
            "В⍽2001⍽Дж. Буш стал президентом.",
        ),
        limit(
            RU,
            ProseOnly,
            "В 1827 Ом открыл свой закон.",
            "В⍽1827⍽Ом⍽открыл свой закон.",
        ),
        limit(RU, Both, "Глава 5 Па-де-де", "Глава 5⍽Па-де-де"),
        limit(
            EN,
            Both,
            "In 2001 GW Bush took office.",
            "In⍽2001⍽GW Bush took office.",
        ),
        // A percentage range with the sign attached keeps its hyphen.
        limit(RU, ProseOnly, "10-20% и 10-20 %", "10-20% и⍽10–20⍽%"),
        // A range of quantities without spaces around the dash glues the first unit only.
        limit(EN, Both, "Lift 5 kg–10 kg daily.", "Lift 5⍽kg–10 kg daily."),
        limit(
            EN,
            Both,
            "Open 9 a.m.–5 p.m. daily.",
            "Open 9⍽a.m.–5 p.m. daily.",
        ),
        limit(
            EN,
            Both,
            "Open 9 a.m. – 5 p.m. daily.",
            "Open 9⍽a.m. – 5⍽p.m. daily.",
        ),
    ]
}

fn typographers(lang: &Language, sets: Sets) -> Vec<(&'static str, Typographer)> {
    let mut out = vec![("Prose", prose(lang.clone()))];
    if matches!(sets, Sets::Both) {
        out.push(("Interface", interface(lang.clone())));
    }
    out
}

#[test]
fn known_limits_hold() {
    for l in limits() {
        for (name, t) in typographers(l.lang, l.sets) {
            let input = vis(l.input);
            let expected = vis(l.expected);
            let got = t.typeset(&input, Output::Unicode);
            assert!(
                got == expected,
                "{} {name}\ninput    {}\nexpected {}\ngot      {}",
                l.lang,
                show(&input),
                show(&expected),
                show(&got)
            );
            assert_eq!(
                matches!(got, Cow::Borrowed(_)),
                input == expected,
                "{} {name}: Cow::Borrowed exactly when unchanged: {}",
                l.lang,
                show(&input)
            );
        }
    }
}

#[test]
fn a_unit_read_from_a_name_can_be_removed() {
    // Design section 10.8: `remove_units` takes a unit that the text uses as a name out of the
    // list.
    let t = Typographer::builder(RU.clone(), RuleSet::PROSE)
        .remove_units(["Дж"])
        .build()
        .unwrap();
    assert_eq!(
        t.typeset("В 2001 Дж. Буш стал президентом.", Output::Unicode),
        vis("В⍽2001 Дж. Буш стал президентом.")
    );
}

#[test]
fn the_closed_style_keeps_the_authors_spaces() {
    // Design section 10.8: next to anything but a plain word the dash keeps the author's spaces.
    // The glue of the other rules (`the⍽guide`) may replace a space, so only the dash and the
    // spaces around it are compared.
    for text in [
        "Hello, — he said",
        "Chapter — 5",
        "See https://example.com — the guide",
        "mother-in-law — father-in-law",
    ] {
        for (name, t) in [
            ("Prose", prose(EN.clone())),
            ("Interface", interface(EN.clone())),
        ] {
            let got = t.typeset(text, Output::Unicode).replace('\u{a0}', " ");
            assert_eq!(got, text, "{name}");
        }
    }
}

#[test]
fn interface_has_none_of_the_prose_only_rules() {
    for (lang, text) in [
        (EN, "git checkout -- file"),
        (EN, "tail -5"),
        (EN, "123-4567"),
        (EN, "...args"),
    ] {
        let t = interface(lang.clone());
        let got = t.typeset(text, Output::Unicode);
        assert!(
            matches!(got, Cow::Borrowed(_)) && got == text,
            "Interface changed {text:?} to {got:?}"
        );
    }
}

#[test]
fn double_hyphen_is_a_dash_in_prose() {
    // Design section 10.8: in Prose `git checkout -- file` becomes a dash in English text.
    let got = prose(EN.clone()).typeset("git checkout -- file", Output::Unicode);
    assert!(got.contains('—') && !got.contains("--"), "{got:?}");
}

#[test]
fn a_flag_is_never_next_to_a_dash() {
    // Design section 10.8: next to a flag a `-` is never a dash, even where one was meant. The text
    // keeps its hyphens; a glue character may replace a space.
    let text = "флаг -v - он включает вывод";
    for (name, t) in [
        ("Prose", prose(RU.clone())),
        ("Interface", interface(RU.clone())),
    ] {
        let got = t.typeset(text, Output::Unicode).replace('\u{a0}', " ");
        assert_eq!(got, text, "{name}");
    }
}

#[test]
fn glued_quotes_before_a_frozen_fragment_have_no_diagnostic() {
    for text in ["сказал:\"привет\"", "сказал,\"привет\""] {
        for (name, t) in [
            ("Prose", prose(RU.clone())),
            ("Interface", interface(RU.clone())),
        ] {
            let r = t.report(text, Output::Unicode);
            assert_eq!(r.result(), text, "{name}: {text}");
            assert!(
                r.diagnostics().is_empty(),
                "{name}: {text}: {:?}",
                r.diagnostics()
            );
        }
    }
}

#[test]
fn an_empty_value_glued_to_a_word_leaves_the_quotes_with_a_warning() {
    // `сказал:""`, `key:""` and `Путь /etc/"" и "да".`: the paragraph keeps its straight quotes
    // with `UnmatchedQuote` (warning, design section 6.1).
    for text in ["сказал:\"\"", "key:\"\"", "Путь /etc/\"\" и \"да\"."] {
        for (name, t) in [
            ("Prose", prose(RU.clone())),
            ("Interface", interface(RU.clone())),
        ] {
            let r = t.report(text, Output::Unicode);
            assert_eq!(
                r.result().replace('\u{a0}', " "),
                text,
                "{name}: the quotes are straight in {text}"
            );
            assert!(
                r.diagnostics().iter().any(|d| {
                    d.kind() == &DiagnosticKind::UnmatchedQuote && d.severity() == Severity::Warning
                }),
                "{name}: {text}: {:?}",
                r.diagnostics()
            );
        }
    }
}

#[test]
fn a_block_tag_passed_as_markup_ends_no_paragraph() {
    // Design section 10.8: `Markup` is inline and transparent, so quotes pair across a block tag
    // and a dialogue dash right after it is not placed; `\n\n` before the tag restores both, and
    // so does the tag passed as `BlockMarkup` (design section 2.6).
    let t = prose(RU.clone());
    let html = Output::Html(HtmlStyle::NAMED);
    let run = |pieces: [Piece<'static, ()>; 3]| t.typeset_template(pieces, html).texts().concat();
    let cases = [
        (
            [
                Piece::Text("Он сказал \"да"),
                Piece::Markup("</p><p>"),
                Piece::Text("Нет\" - ответила она."),
            ],
            "Он&nbsp;сказал &laquo;да</p><p>Нет&raquo;&nbsp;&mdash; ответила она.",
        ),
        (
            [
                Piece::Text("Текст."),
                Piece::Markup("</p><p>"),
                Piece::Text("- Да, - сказал он."),
            ],
            "Текст.</p><p>- Да,&nbsp;&mdash; сказал он.",
        ),
        (
            [
                Piece::Text("Он сказал \"да\n\n"),
                Piece::Markup("</p><p>"),
                Piece::Text("Нет\" - ответила она."),
            ],
            "Он&nbsp;сказал &quot;да\n\n</p><p>Нет&quot;&nbsp;&mdash; ответила она.",
        ),
        (
            [
                Piece::Text("Текст.\n\n"),
                Piece::Markup("</p><p>"),
                Piece::Text("- Да, - сказал он."),
            ],
            "Текст.\n\n</p><p>&mdash;&nbsp;Да,&nbsp;&mdash; сказал он.",
        ),
    ];
    for (pieces, expected) in cases {
        assert_eq!(run(pieces), expected);
    }
    // The same tag passed as `BlockMarkup` ends the paragraph without the line breaks: the
    // results of the cases with `\n\n`.
    let cases = [
        (
            [
                Piece::Text("Он сказал \"да"),
                Piece::BlockMarkup("</p><p>"),
                Piece::Text("Нет\" - ответила она."),
            ],
            "Он&nbsp;сказал &quot;да</p><p>Нет&quot;&nbsp;&mdash; ответила она.",
        ),
        (
            [
                Piece::Text("Текст."),
                Piece::BlockMarkup("</p><p>"),
                Piece::Text("- Да, - сказал он."),
            ],
            "Текст.</p><p>&mdash;&nbsp;Да,&nbsp;&mdash; сказал он.",
        ),
    ];
    for (pieces, expected) in cases {
        assert_eq!(run(pieces), expected);
    }
}

/// The code spans of a section, for the next two tests.
fn spans_of(section_text: &str) -> Vec<String> {
    code_spans(section_text)
}

#[test]
fn readme_limits_use_the_examples_of_the_design() {
    let Some(text) = design("10-protection.md") else {
        return;
    };
    let design_limits = section(&text, "### 10.8 Known limits");
    let design_spans = spans_of(&design_limits);
    let readme_text = readme();
    let readme_limits = section(&readme_text, "## Known limits");
    let mut checked = 0;
    for span in spans_of(&readme_limits) {
        // Rule ids, identifiers and API names are not examples.
        if span.chars().count() < 2
            || (span
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || "._:".contains(c))
                && !span.contains(' '))
        {
            continue;
        }
        checked += 1;
        assert!(
            design_spans.contains(&span),
            "the README limit example {span:?} is not in design section 10.8"
        );
    }
    assert!(checked >= 20, "only {checked} examples compared");
}

#[test]
fn readme_limits_cover_every_limit_of_the_design() {
    let Some(text) = design("10-protection.md") else {
        return;
    };
    let design_limits = section(&text, "### 10.8 Known limits");
    let readme_text = readme();
    let readme_limits = section(&readme_text, "## Known limits");
    // The README may merge two limits into one bullet, but each limit of the design shows in it
    // by its first example.
    let mut bullets = 0;
    for bullet in design_limits.split("\n- ").skip(1) {
        bullets += 1;
        let first = code_spans(bullet)
            .into_iter()
            .find(|s| s.chars().count() > 1 && s != "-")
            .unwrap_or_else(|| panic!("a limit without an example: {bullet}"));
        assert!(
            readme_limits.contains(&format!("`{first}`")),
            "the README limits lack the first example {first:?} of a limit of design section 10.8"
        );
    }
    assert!(bullets >= 10);
}

#[test]
fn the_table_of_limits_is_in_the_readme() {
    // Every example of the table above that the README quotes is quoted as the table writes it.
    let readme_text = readme();
    let limits_text = section(&readme_text, "## Known limits");
    for needle in [
        "`cat - file`",
        "`cat—file`",
        "`tls-server - agent-ca`",
        "`tls-server — agent-ca`",
        "`rock 'n' roll`",
        "`rock'n'roll`",
        "`git checkout -- file`",
        "`tail -5`",
        "`tail −5`",
        "`123-4567`",
        "`...args`",
        "`…args`",
        "`@user`",
        "`1'000'000`",
        "`0b1'0000`",
        "`50%-ный`",
        "`C#`",
        "`AT&T`",
        "`сказал:\"привет\"`",
        "`сказал: \"привет\"`",
        "`key:\"\"`",
        "`key=\"\"`",
        "`UnmatchedQuote`",
    ] {
        assert!(
            limits_text.contains(needle),
            "the README limits lack {needle}"
        );
    }
}

#[test]
fn readme_usage_examples_match_the_crate() {
    // The first block of the README is the one the front page shows; its outputs are the
    // examples of design section 2.1. A doctest runs it; here the same strings are asserted through
    // the public API, so that a wrong README fails this target and not only a doctest.
    static RU_UI: Typographer = Typographer::new(Language::RU_RU, RuleSet::INTERFACE);
    static EN_PROSE: Typographer = Typographer::new(Language::EN_US, RuleSet::PROSE);
    assert_eq!(
        RU_UI.typeset("Он сказал \"привет\" - и ушёл...", Output::Unicode),
        "Он сказал «привет»\u{a0}— и\u{a0}ушёл…"
    );
    assert_eq!(
        EN_PROSE.typeset("\"Don't\" - she said...", Output::Unicode),
        "“Don’t” — she said…"
    );
    let text = readme();
    assert!(text.contains("Он сказал \\\"привет\\\" - и ушёл..."));
    assert!(text.contains("“Don’t” — she said…"));
}
