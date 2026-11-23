//! Preparation of the package (design sections 14.1, 14.5, 14.6 and 15.4, stage 5): the manifest,
//! the repository layout, the contents of the package, the build from its own tarball, and the
//! examples.
//!
//! Nothing here publishes anything: `cargo package` only builds the tarball and checks it, and the
//! check of the name on crates.io is a single query that the lead makes by hand (section 15.5, item
//! 3), not a test, because tests need no network.
//!
//! The checks of the repository (the manifests as written, `deny.toml`, `Cargo.lock`, the layout,
//! the design) and of the package (`cargo package` refuses to run inside an unpacked package) skip
//! outside the repository, so that `cargo test` passes in the unpacked package too (design section
//! 13.10). The checks of the examples and of the allow lists run in both places.

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

use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use common::show;

fn crate_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn repo_dir() -> PathBuf {
    crate_dir().join("../..")
}

fn dev_dir() -> PathBuf {
    crate_dir().join("..")
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

/// The lines of one table of a TOML file (`[name]`), without comments and blank lines.
fn table(toml: &str, name: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut inside = false;
    for line in toml.lines() {
        let t = line.trim();
        if t.starts_with('[') {
            inside = t == format!("[{name}]");
            continue;
        }
        if inside && !t.is_empty() && !t.starts_with('#') {
            out.push(t.to_string());
        }
    }
    out
}

fn manifest() -> String {
    read(&crate_dir().join("Cargo.toml"))
}

fn run_cargo(args: &[&str]) -> Output {
    let cargo = std::env::var("CARGO").unwrap_or_else(|_| "cargo".to_string());
    Command::new(cargo)
        .args(args)
        .arg("--manifest-path")
        .arg(crate_dir().join("Cargo.toml"))
        .output()
        .unwrap_or_else(|e| panic!("cargo {args:?}: {e}"))
}

fn lossy(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes).into_owned()
}

// ---------------------------------------------------------------------------------------------
// The manifest (section 14.1, 14.5).

#[test]
fn package_manifest_matches_section_14_5() {
    if repository().is_none() {
        return;
    }
    let toml = manifest();
    let package = table(&toml, "package");
    let has = |line: &str| package.iter().any(|l| l == line);
    assert!(has("name = \"typolish\""));
    assert!(has("version = \"0.1.0\""));
    assert!(has("readme = \"README.md\""));
    assert!(has(
        "keywords = [\"typography\", \"typesetting\", \"quotes\", \"nbsp\", \"i18n\"]"
    ));
    assert!(has(
        "categories = [\"text-processing\", \"internationalization\"]"
    ));
    assert!(has("edition.workspace = true"));
    assert!(has("rust-version.workspace = true"));
    assert!(has("license.workspace = true"));
    assert!(has("repository.workspace = true"));
    assert!(has(
        "exclude = [\"tests/fixtures/unicode/\", \"tests/fixtures/whatwg-entities.txt\"]"
    ));
    // A one-line description of the crate, and no `authors` field (the maintainer's decision:
    // license, authors and copyright are copied from zonewright exactly).
    let description = package
        .iter()
        .find_map(|l| l.strip_prefix("description = "))
        .expect("a description");
    assert!(description.starts_with('"') && description.ends_with('"'));
    assert!(description.len() > 20 && description.len() < 200);
    assert!(
        !package.iter().any(|l| l.starts_with("authors")),
        "no authors field"
    );
    assert_eq!(table(&toml, "lints"), ["workspace = true"]);
    assert_eq!(table(&toml, "features"), ["default = []"]);
    // Cargo reads the same values.
    assert_eq!(env!("CARGO_PKG_NAME"), "typolish");
    assert_eq!(env!("CARGO_PKG_VERSION"), "0.1.0");
    assert_eq!(env!("CARGO_PKG_LICENSE"), "MIT OR Apache-2.0");
    assert_eq!(
        env!("CARGO_PKG_REPOSITORY"),
        "https://github.com/dcl-group/typolish"
    );
    assert_eq!(env!("CARGO_PKG_RUST_VERSION"), "1.100");
}

#[test]
fn dependencies_are_those_of_section_14_1() {
    if repository().is_none() {
        return;
    }
    let toml = manifest();
    assert_eq!(
        table(&toml, "dependencies"),
        [
            "thiserror = \"2.0.21\"",
            "unicode-segmentation = \"1.13.3\""
        ]
    );
    assert_eq!(
        table(&toml, "dev-dependencies"),
        ["proptest = { version = \"1.12.0\", default-features = false, features = [\"std\"] }"]
    );
    // The bench harness is the crate's own (section 13.8).
    assert!(toml.contains("[[bench]]\nname = \"load\"\nharness = false"));
}

#[test]
fn workspace_manifest_matches_section_14_5() {
    if repository().is_none() {
        return;
    }
    let toml = read(&dev_dir().join("Cargo.toml"));
    assert_eq!(
        table(&toml, "workspace"),
        ["members = [\"typolish\"]", "resolver = \"3\""]
    );
    assert_eq!(
        table(&toml, "workspace.package"),
        [
            "edition = \"2024\"",
            "rust-version = \"1.100\"",
            "license = \"MIT OR Apache-2.0\"",
            "repository = \"https://github.com/dcl-group/typolish\"",
        ]
    );
    assert_eq!(
        table(&toml, "workspace.lints.rust"),
        ["unsafe_code = \"deny\"", "missing_docs = \"deny\""]
    );
    let clippy = table(&toml, "workspace.lints.clippy");
    for lint in [
        "unwrap_used",
        "expect_used",
        "panic",
        "unreachable",
        "todo",
        "unimplemented",
        "indexing_slicing",
        "dbg_macro",
        "print_stdout",
        "print_stderr",
    ] {
        assert!(
            clippy.contains(&format!("{lint} = \"deny\"")),
            "the workspace does not deny clippy::{lint}"
        );
    }
}

#[test]
fn deny_toml_matches_section_14_5() {
    if repository().is_none() {
        return;
    }
    let toml = read(&dev_dir().join("deny.toml"));
    assert_eq!(table(&toml, "graph"), ["all-features = true"]);
    assert_eq!(
        table(&toml, "licenses"),
        [
            "allow = [\"MIT\", \"Apache-2.0\", \"Unicode-3.0\"]",
            "confidence-threshold = 0.8"
        ]
    );
    let bans = toml.clone();
    for banned in [
        "ring",
        "openssl",
        "openssl-sys",
        "openssl-src",
        "native-tls",
        "regex",
        "regex-lite",
        "regex-automata",
        "fancy-regex",
        "onig",
        "pcre2",
    ] {
        assert!(
            bans.contains(&format!("{{ crate = \"{banned}\" }}")),
            "deny.toml does not ban {banned}"
        );
    }
    // `regex-syntax` is a parser without a matching engine, used by `proptest`: not banned.
    assert!(!bans.contains("crate = \"regex-syntax\""));
    let sources = table(&toml, "sources");
    assert!(sources.contains(&"unknown-registry = \"deny\"".to_string()));
    assert!(sources.contains(&"unknown-git = \"deny\"".to_string()));
    assert!(table(&toml, "bans").contains(&"wildcards = \"deny\"".to_string()));
}

#[test]
fn lockfile_has_the_tree_of_section_14_1() {
    if repository().is_none() {
        return;
    }
    // "23 packages in `Cargo.lock` besides `typolish`": 7 normal ones and the dev tree.
    let lock = read(&dev_dir().join("Cargo.lock"));
    let names: BTreeSet<&str> = lock
        .lines()
        .filter_map(|l| l.strip_prefix("name = \""))
        .filter_map(|l| l.strip_suffix('"'))
        .collect();
    for name in ["thiserror", "unicode-segmentation", "proptest", "typolish"] {
        assert!(names.contains(name), "{name} is not in Cargo.lock");
    }
    for banned in [
        "regex",
        "regex-lite",
        "regex-automata",
        "fancy-regex",
        "ring",
        "openssl",
    ] {
        assert!(!names.contains(banned), "{banned} is in Cargo.lock");
    }
    assert_eq!(names.len() - 1, 23, "packages besides typolish: {names:?}");
}

// ---------------------------------------------------------------------------------------------
// The layout of the repository (section 14.6).

#[test]
fn repository_layout_matches_section_14_6() {
    let Some(repo) = repository() else {
        return;
    };
    for file in [
        "AGENTS.md",
        "CLAUDE.md",
        "README.md",
        "CHANGELOG.md",
        "LICENSE-MIT",
        "LICENSE-APACHE",
        ".gitignore",
        ".gitattributes",
        ".prettierrc.json",
        ".prettierignore",
        "dev/Cargo.toml",
        "dev/Cargo.lock",
        "dev/deny.toml",
        "dev/clippy.toml",
        "dev/typolish/Cargo.toml",
        "dev/typolish/LICENSE-MIT",
        "dev/typolish/LICENSE-APACHE",
        "dev/typolish/README.md",
        "dev/typolish/benches/load.rs",
        "dev/typolish/examples/fluent_generator.rs",
        "dev/typolish/examples/per_request.rs",
        "dev/typolish/examples/numbers.rs",
        "dev/typolish/tests/fixtures/whatwg-entities.txt",
        "dev/typolish/tests/fixtures/unicode/GraphemeBreakTest.txt",
        "dev/typolish/tests/reference/ru.rs",
        "dev/typolish/tests/reference/en_us.rs",
        "dev/typolish/tests/common/mod.rs",
        "dev/typolish/tests/common/strategies.rs",
        "dev/typolish/tests/common/load_inputs.rs",
    ] {
        assert!(repo.join(file).exists(), "{file} is missing");
    }
    let src = [
        "lib.rs",
        "error.rs",
        "typographer.rs",
        "language.rs",
        "rule.rs",
        "piece.rs",
        "number.rs",
        "output.rs",
        "report.rs",
        "lang/mod.rs",
        "lang/data.rs",
        "lang/ru_ru.rs",
        "lang/en_us.rs",
        "lang/common.rs",
        "engine/mod.rs",
        "engine/ingest.rs",
        "engine/window.rs",
        "engine/lex.rs",
        "engine/classify.rs",
        "engine/phase.rs",
        "engine/render.rs",
        "engine/record.rs",
        "engine/scratch.rs",
        "rules/mod.rs",
        "rules/symbols.rs",
        "rules/ellipsis.rs",
        "rules/dash.rs",
        "rules/range.rs",
        "rules/number.rs",
        "rules/quotes.rs",
        "rules/nbsp.rs",
    ];
    for file in src {
        assert!(
            crate_dir().join("src").join(file).exists(),
            "src/{file} is missing"
        );
    }
    let tests = [
        "rules_symbols.rs",
        "rules_ellipsis.rs",
        "rules_dash.rs",
        "rules_range.rs",
        "rules_number.rs",
        "rules_quotes.rs",
        "rules_nbsp.rs",
        "negative.rs",
        "parts.rs",
        "numbers.rs",
        "output.rs",
        "fluent_isolation.rs",
        "reference.rs",
        "deliberate_differences.rs",
        "properties.rs",
        "graphemes.rs",
        "roundtrip.rs",
        "corpus.rs",
        "alloc.rs",
        "load.rs",
        "conformance.rs",
        "data.rs",
        "api.rs",
        "lint.rs",
        "builder.rs",
        "language.rs",
        "traits.rs",
        "typeset_api.rs",
        "rules.rs",
        "helpers.rs",
    ];
    for file in tests {
        assert!(
            crate_dir().join("tests").join(file).exists(),
            "tests/{file} is missing"
        );
    }
    assert!(crate_dir().join("tests/corpus").is_dir());
    // `README.md` of the crate is the root README.
    let link = fs::read_link(crate_dir().join("README.md")).unwrap();
    assert_eq!(link, Path::new("../../README.md"));
}

#[test]
fn claude_md_is_a_pointer_to_agents_md() {
    let Some(repo) = repository() else {
        return;
    };
    let claude = read(&repo.join("CLAUDE.md"));
    assert!(claude.contains("@AGENTS.md"));
    assert!(claude.contains("AGENTS.md"));
    let agents = read(&repo.join("AGENTS.md"));
    // The two hard rules that section 14.6 adds to the skeleton.
    assert!(agents.contains("Co-Authored-By: Claude <noreply@anthropic.com>"));
    assert!(agents.contains("reference implementation"));
}

#[test]
fn license_files_follow_section_14_6() {
    let Some(repo) = repository() else {
        return;
    };
    let mit = read(&repo.join("LICENSE-MIT"));
    assert!(mit.contains("Copyright (c) 2026 Dmitry Bikman"));
    assert!(mit.starts_with("MIT License"));
    let apache = read(&repo.join("LICENSE-APACHE"));
    assert!(apache.contains("Apache License"));
    assert!(apache.contains("Version 2.0, January 2004"));
    // The crate name appears in neither text.
    for text in [&mit, &apache] {
        assert!(!text.to_lowercase().contains("typolish"));
        assert!(!text.to_lowercase().contains("zonewright"));
    }
}

#[test]
fn ignore_files_protect_local_notes_and_keys() {
    // AGENTS.md: `AGENTS.local.md` is ignored by git and never committed; keys never are.
    let Some(repo) = repository() else {
        return;
    };
    let ignore = read(&repo.join(".gitignore"));
    let lines: Vec<&str> = ignore.lines().map(str::trim).collect();
    for pattern in ["target/", ".env", "*.local.*", ".ssh/"] {
        assert!(lines.contains(&pattern), ".gitignore lacks {pattern}");
    }
    let attributes = read(&repo.join(".gitattributes"));
    assert!(attributes.contains("* text=auto eol=lf"));
    assert!(attributes.contains("dev/typolish/tests/fixtures/** -text"));
    assert!(attributes.contains("dev/typolish/tests/corpus/** -text"));
    let prettier = read(&repo.join(".prettierrc.json"));
    assert!(prettier.contains("\"proseWrap\": \"always\""));
    assert!(prettier.contains("\"printWidth\": 100"));
    let prettier_ignore = read(&repo.join(".prettierignore"));
    for pattern in ["target/", "**/tests/fixtures/", "**/tests/corpus/"] {
        assert!(
            prettier_ignore.lines().any(|l| l == pattern),
            ".prettierignore lacks {pattern}"
        );
    }
}

#[test]
fn non_library_targets_carry_the_allow_list_of_section_14_5() {
    let allow = [
        "clippy::print_stdout",
        "clippy::print_stderr",
        "clippy::unwrap_used",
        "clippy::expect_used",
        "clippy::indexing_slicing",
        "clippy::panic",
    ];
    let mut files = vec![crate_dir().join("benches/load.rs")];
    for entry in fs::read_dir(crate_dir().join("examples")).unwrap() {
        files.push(entry.unwrap().path());
    }
    assert_eq!(files.len(), 4);
    for file in files {
        let src = read(&file);
        assert!(
            src.starts_with("//!"),
            "{} starts with a //! line",
            file.display()
        );
        let inner = src
            .split("#![allow(")
            .nth(1)
            .unwrap_or_else(|| panic!("{} has no allow list", file.display()))
            .split(")]")
            .next()
            .unwrap()
            .to_string();
        let found: BTreeSet<&str> = inner
            .split(',')
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .collect();
        let expected: BTreeSet<&str> = allow.iter().copied().collect();
        assert_eq!(found, expected, "{}", file.display());
    }
}

#[test]
fn name_check_is_recorded_in_the_design() {
    // Section 15.4: "the report gives its result"; the record of the one query is kept in the
    // design (the lead made it in stage 5).
    let Some(repo) = repository() else {
        return;
    };
    let text = read(&repo.join("docs/design/history.md"));
    assert!(
        text.contains("Name on crates.io") && text.contains("crates.io/api/v1/crates/typolish"),
        "the history does not record the name check on crates.io"
    );
    assert!(
        text.contains("HTTP 404"),
        "the history gives no result of the name check"
    );
    assert!(text.contains("nothing was published"));
}

// ---------------------------------------------------------------------------------------------
// The contents of the package and the build from its own tarball (section 15.4).

fn package_list() -> Vec<String> {
    let out = run_cargo(&["package", "--list", "--offline", "--allow-dirty"]);
    assert!(
        out.status.success(),
        "cargo package --list failed:\n{}",
        lossy(&out.stderr)
    );
    lossy(&out.stdout).lines().map(str::to_string).collect()
}

#[test]
fn package_lists_what_a_user_needs_and_nothing_else() {
    if repository().is_none() {
        return;
    }
    let list = package_list();
    for file in [
        "Cargo.toml",
        "README.md",
        "LICENSE-MIT",
        "LICENSE-APACHE",
        "src/lib.rs",
        "src/typographer.rs",
        "src/rules/nbsp.rs",
        "src/lang/ru_ru.rs",
        "examples/fluent_generator.rs",
        "examples/per_request.rs",
        "examples/numbers.rs",
        "benches/load.rs",
        "tests/api.rs",
    ] {
        assert!(list.iter().any(|l| l == file), "the package lacks {file}");
    }
    // `exclude` of section 14.1.
    for path in &list {
        assert!(!path.starts_with("tests/fixtures/unicode/"), "{path}");
        assert_ne!(path, "tests/fixtures/whatwg-entities.txt");
    }
    // Only the files of the crate: no design, no local notes, no keys, no build output.
    for path in &list {
        let lower = path.to_lowercase();
        let allowed_root = matches!(
            path.as_str(),
            ".cargo_vcs_info.json"
                | "Cargo.lock"
                | "Cargo.toml"
                | "Cargo.toml.orig"
                | "LICENSE-APACHE"
                | "LICENSE-MIT"
                | "README.md"
        );
        assert!(
            allowed_root
                || ["src/", "tests/", "examples/", "benches/"]
                    .iter()
                    .any(|p| path.starts_with(p)),
            "unexpected file in the package: {path}"
        );
        for bad in [
            ".local.",
            ".env",
            ".ssh",
            "target/",
            "docs/",
            ".claude",
            "node_modules",
        ] {
            assert!(
                !lower.contains(bad),
                "{path} does not belong to the package"
            );
        }
    }
    // Every test target of the package finds its shared files in it.
    for file in [
        "tests/common/mod.rs",
        "tests/common/strategies.rs",
        "tests/reference/ru.rs",
    ] {
        assert!(list.iter().any(|l| l == file), "the package lacks {file}");
    }
}

#[test]
fn package_is_small_enough_for_the_registry() {
    if repository().is_none() {
        return;
    }
    // crates.io accepts a tarball of at most 10 MiB; the uncompressed size is a safe bound.
    let list = package_list();
    let total: u64 = list
        .iter()
        .filter(|p| !p.starts_with(".cargo_vcs_info") && !p.ends_with(".orig"))
        .filter_map(|p| fs::metadata(crate_dir().join(p)).ok())
        .map(|m| m.len())
        .sum();
    assert!(total < 10 * 1024 * 1024, "the package holds {total} bytes");
    assert!(total > 100 * 1024);
}

#[test]
fn package_builds_from_its_own_tarball() {
    if repository().is_none() {
        return;
    }
    // `cargo package` builds the tarball and then compiles it in a clean directory. It does not
    // publish: `cargo publish` is the only command that does, and nothing here runs it.
    let out = run_cargo(&["package", "--offline", "--allow-dirty"]);
    assert!(
        out.status.success(),
        "cargo package failed:\n{}",
        lossy(&out.stderr)
    );
    let stderr = lossy(&out.stderr);
    assert!(stderr.contains("Packaged"), "cargo package: {stderr}");
    assert!(
        stderr.contains("Verifying typolish v0.1.0"),
        "cargo package: {stderr}"
    );
    assert!(
        !stderr.contains("warning:"),
        "cargo package warns:\n{stderr}"
    );
    // The checks of this target and of `tests/documentation.rs` read the repository; in the
    // unpacked package they skip what is not there and pass (design section 13.10). Cargo leaves
    // the unpacked package where it verified it.
    let unpacked = stderr
        .split("Compiling typolish v0.1.0 (")
        .nth(1)
        .and_then(|rest| rest.split(')').next())
        .map(PathBuf::from)
        .unwrap_or_else(|| panic!("no directory of the unpacked package in:\n{stderr}"));
    assert!(unpacked.join("Cargo.toml.orig").is_file(), "{unpacked:?}");
    let cargo = std::env::var("CARGO").unwrap_or_else(|_| "cargo".to_string());
    let out = Command::new(cargo)
        .args([
            "test",
            "--offline",
            "--test",
            "documentation",
            "--test",
            "package",
        ])
        .arg("--manifest-path")
        .arg(unpacked.join("Cargo.toml"))
        .args(["--", "--show-output"])
        .output()
        .unwrap();
    let (stdout, stderr) = (lossy(&out.stdout), lossy(&out.stderr));
    assert!(
        out.status.success(),
        "cargo test in the unpacked package failed:\n{stdout}\n{stderr}"
    );
    assert_eq!(
        stdout.matches("test result: ok.").count(),
        2,
        "two targets run in the unpacked package:\n{stdout}"
    );
    assert!(
        stdout.contains("is not in the repository"),
        "the repository checks did not skip:\n{stdout}"
    );
}

// ---------------------------------------------------------------------------------------------
// The examples run without error (section 15.4, stages 3 and 5).

fn run_example(name: &str) -> (String, String) {
    let out = run_cargo(&["run", "--quiet", "--offline", "--example", name]);
    assert!(
        out.status.success(),
        "example {name} failed:\n{}",
        lossy(&out.stderr)
    );
    (lossy(&out.stdout), lossy(&out.stderr))
}

#[test]
fn example_numbers_runs() {
    let (stdout, _) = run_example("numbers");
    // Section 2.7: the same number by the language.
    assert!(show(&stdout).contains("−1⍽234⍽567,5"), "{stdout}");
    assert!(stdout.contains("−1,234,567.5"), "{stdout}");
    assert!(show(&stdout).contains("1⍽234"), "{stdout}");
    assert!(show(&stdout).contains("не⍽число"), "{stdout}");
    assert!(show(&stdout).contains("8⍽000–8⍽100"), "{stdout}");
    assert!(stdout.contains("&minus;12&nbsp;345"), "{stdout}");
}

#[test]
fn example_per_request_runs() {
    let (stdout, _) = run_example("per_request");
    // Section 2.15: statics for ru and en, a borrowed result for an unchanged string, and a
    // typographer built at run time.
    assert!(stdout.contains("ru-RU"), "{stdout}");
    assert!(stdout.contains("en-US"), "{stdout}");
    assert!(stdout.contains("borrowed"), "{stdout}");
    assert!(stdout.contains("owned"), "{stdout}");
    assert!(stdout.contains("&laquo;привет&raquo;"), "{stdout}");
    assert!(stdout.contains("&ldquo;hello&rdquo;"), "{stdout}");
}

#[test]
fn example_fluent_generator_runs() {
    let (stdout, _) = run_example("fluent_generator");
    // Section 2.14: one line per message, the pieces in a Rust string body, and the number
    // formatting helper.
    assert!(stdout.lines().count() >= 3, "{stdout}");
    assert!(stdout.contains("format_number"), "{stdout}");
    assert!(stdout.contains("\\u{a0}"), "{stdout}");
    assert!(
        stdout.contains("\\u{ab}") || stdout.contains("«"),
        "{stdout}"
    );
}
