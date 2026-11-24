//! The own corpus of design section 13.7: the snapshots of `tests/corpus/expected/` compared byte
//! for byte, idempotence and the output kinds on every corpus file. Round trip (P1, P2) on the
//! corpus is in `tests/roundtrip.rs`.
//!
//! The snapshots are plain files, updated deliberately by hand (no `insta`): after a change that
//! is meant to alter the output, run
//! `cargo test -p typolish --test corpus -- --ignored update_snapshots` from `dev/`, read the diff
//! of `tests/corpus/expected/` and commit it.

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

use common::corpus::{
    FILES, LANGS, SETS, corpus_dir, messages, render_message, render_snapshot, snapshot_name,
    snapshot_outputs, snapshot_path, split_template, template_pieces, text_lines, typographer,
};
use common::decode::decode;
use common::show;
use typolish::Output;

/// Every snapshot: language directory, corpus file, set name, output name and kind.
fn all_snapshots() -> Vec<(
    &'static str,
    &'static str,
    &'static str,
    &'static str,
    Output,
)> {
    let mut all = Vec::new();
    for lang in LANGS {
        for file in FILES {
            for (set_name, _) in SETS {
                for (output_name, output) in snapshot_outputs() {
                    all.push((lang, file, set_name, output_name, output));
                }
            }
        }
    }
    all
}

/// The first line where two texts differ, for a failure message.
fn first_difference(expected: &str, got: &str) -> String {
    let mut expected_lines = expected.split('\n');
    let mut got_lines = got.split('\n');
    let mut number = 1;
    loop {
        match (expected_lines.next(), got_lines.next()) {
            (None, None) => {
                return "no difference in lines (the files differ in their end)".to_owned();
            }
            (a, b) if a == b => number += 1,
            (a, b) => {
                return format!(
                    "line {number}\n    expected {}\n    got      {}",
                    show(a.unwrap_or("<end of file>")),
                    show(b.unwrap_or("<end of file>"))
                );
            }
        }
    }
}

#[test]
fn snapshots_match_byte_for_byte() {
    let mut failures: Vec<String> = Vec::new();
    for (lang, file, set_name, output_name, output) in all_snapshots() {
        let path = snapshot_path(lang, file, set_name, output_name);
        let got = render_snapshot(lang, file, set_name, output);
        match fs::read(&path) {
            Err(_) => failures.push(format!(
                "{lang}/{}: no snapshot file",
                snapshot_name(file, set_name, output_name)
            )),
            Ok(bytes) => {
                if bytes != got.as_bytes() {
                    let expected = String::from_utf8_lossy(&bytes).into_owned();
                    failures.push(format!(
                        "{lang}/{} differs, first difference at {}",
                        snapshot_name(file, set_name, output_name),
                        first_difference(&expected, &got)
                    ));
                }
            }
        }
    }
    assert!(
        failures.is_empty(),
        "{} of {} snapshots differ (update deliberately with \
         `cargo test -p typolish --test corpus -- --ignored update_snapshots`):\n{}",
        failures.len(),
        all_snapshots().len(),
        failures.join("\n")
    );
}

#[test]
fn the_snapshot_directory_has_no_stray_files() {
    let expected: BTreeSet<String> = all_snapshots()
        .into_iter()
        .map(|(lang, file, set_name, output_name, _)| {
            format!("{lang}/{}", snapshot_name(file, set_name, output_name))
        })
        .collect();
    let mut found: BTreeSet<String> = BTreeSet::new();
    let root = corpus_dir().join("expected");
    for lang in LANGS {
        let dir = root.join(lang);
        let entries = fs::read_dir(&dir).unwrap_or_else(|e| panic!("{}: {e}", dir.display()));
        for entry in entries {
            let entry = entry.unwrap();
            found.insert(format!("{lang}/{}", entry.file_name().to_string_lossy()));
        }
    }
    let stray: Vec<&String> = found.difference(&expected).collect();
    assert!(
        stray.is_empty(),
        "files in expected/ that no snapshot owns: {stray:?}"
    );
}

/// Writes every snapshot. Run by hand, never in a normal run, and alone:
/// `cargo test --test corpus update_snapshots -- --ignored`; never with `--include-ignored`, where
/// it would write the snapshots while `snapshots_match_byte_for_byte` reads them. A bare
/// `cargo test -- --ignored` runs it too: always name the test file and the test.
#[test]
#[ignore = "rewrites tests/corpus/expected/; run alone by hand after a deliberate change: \
            cargo test --test corpus update_snapshots -- --ignored; never with --include-ignored"]
fn update_snapshots() {
    for (lang, file, set_name, output_name, output) in all_snapshots() {
        let path = snapshot_path(lang, file, set_name, output_name);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(&path, render_snapshot(lang, file, set_name, output)).unwrap();
    }
}

#[test]
fn snapshots_of_unicode_keep_their_glue_and_typographic_characters() {
    // A guard against a snapshot that went through a normalizing editor or git: the Unicode
    // snapshot of a typeset file holds U+00A0 and the typographic characters themselves.
    for lang in LANGS {
        let prose =
            fs::read_to_string(snapshot_path(lang, "prose.txt", "prose", "unicode")).unwrap();
        assert!(
            prose.contains('\u{a0}'),
            "{lang}: no U+00A0 in the Unicode snapshot of prose.txt"
        );
        let ui = fs::read_to_string(snapshot_path(lang, "ui.tmpl", "prose", "unicode")).unwrap();
        assert!(
            ui.contains('\u{a0}'),
            "{lang}: no U+00A0 in the Unicode snapshot of ui.tmpl"
        );
    }
    let ru = fs::read_to_string(snapshot_path("ru-RU", "prose.txt", "prose", "unicode")).unwrap();
    assert!(ru.contains('«') && ru.contains('—'));
    let en = fs::read_to_string(snapshot_path("en-US", "prose.txt", "prose", "unicode")).unwrap();
    assert!(en.contains('“') && en.contains('’'));
}

#[test]
fn idempotence_on_the_corpus() {
    let mut failures: Vec<String> = Vec::new();
    for lang in LANGS {
        for (set_name, _) in SETS {
            let t = typographer(lang, set_name);
            for file in ["prose.txt", "tech.txt"] {
                for line in text_lines(lang, file) {
                    let once = t.typeset(&line, Output::Unicode).into_owned();
                    let twice = t.typeset(&once, Output::Unicode);
                    if twice != once {
                        failures.push(format!(
                            "{lang}/{file} {set_name}\n  once  {}\n  twice {}",
                            show(&once),
                            show(&twice)
                        ));
                    }
                }
            }
            for message in messages(lang) {
                let once = t.typeset_template(message.pieces(), Output::Unicode);
                let twice = t.typeset_template(template_pieces(&once), Output::Unicode);
                if split_template(&twice) != split_template(&once) {
                    failures.push(format!(
                        "{lang}/ui.tmpl {set_name} {}\n  once  {}\n  twice {}",
                        message.id,
                        show(&render_message(&t, &message, Output::Unicode)),
                        show(&format!("{:?}", twice.texts()))
                    ));
                }
            }
        }
    }
    assert!(
        failures.is_empty(),
        "{} lines are not idempotent:\n{}",
        failures.len(),
        failures.join("\n")
    );
}

#[test]
fn every_output_kind_decodes_to_the_unicode_result_on_the_corpus() {
    // Property 12 on the corpus: `decode(typeset(x, out)) == typeset(x, Unicode)`.
    let mut failures: Vec<String> = Vec::new();
    for lang in LANGS {
        for (set_name, _) in SETS {
            let t = typographer(lang, set_name);
            for file in ["prose.txt", "tech.txt"] {
                for line in text_lines(lang, file) {
                    let unicode = t.typeset(&line, Output::Unicode).into_owned();
                    for (output_name, output) in snapshot_outputs() {
                        let encoded = t.typeset(&line, output);
                        match decode(output, &encoded) {
                            Ok(decoded) if decoded == unicode => {}
                            other => failures.push(format!(
                                "{lang}/{file} {set_name} {output_name}: {line:?} -> {other:?}"
                            )),
                        }
                    }
                }
            }
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

#[test]
fn the_corpus_exercises_every_rule() {
    // The corpus is the project's own and meant to reach every rule of version 1: each rule of
    // `Prose` but `dash.double-hyphen` changes at least one line of some corpus file in some language (report records).
    use std::collections::BTreeSet;
    use typolish::Rule;
    let mut seen: BTreeSet<&'static str> = BTreeSet::new();
    for lang in LANGS {
        let t = typographer(lang, "prose");
        for file in ["prose.txt", "tech.txt"] {
            for line in text_lines(lang, file) {
                for change in t.report(&line, Output::Unicode).changes() {
                    seen.insert(change.rule().id());
                }
            }
        }
        for message in messages(lang) {
            for change in t
                .report_template(message.pieces(), Output::Unicode)
                .changes()
            {
                seen.insert(change.rule().id());
            }
        }
    }
    // `dash.double-hyphen` reads `--`, which is not in the canonical plain form of the corpus
    // (design section 12.1); its non-canonical input is in the table of known losses.
    let missing: Vec<&'static str> = Rule::ALL
        .iter()
        .map(|r| r.id())
        .filter(|id| *id != "dash.double-hyphen" && !seen.contains(id))
        .collect();
    assert!(
        missing.is_empty(),
        "rules that no corpus line exercises: {missing:?}"
    );
}
