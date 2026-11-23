//! Load measurements of design section 13.8: `cargo bench --bench load` from `dev/`.
//!
//! Prints the corpus throughput per set and output kind, the latency of one interface string, the
//! time per byte by input size, and the pathological inputs of design section 3.12. Std only.

#![allow(
    clippy::print_stdout,
    clippy::print_stderr,
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic
)]

#[path = "../tests/common/load_inputs.rs"]
mod load_inputs;

use std::fs;
use std::hint::black_box;
use std::path::PathBuf;
use std::process::Command;
use std::time::{Duration, Instant};

use load_inputs::{PATHOLOGICAL, repeat_to};
use typolish::{
    EscapeStyle, HtmlStyle, Language, Output, Piece, RuleSet, Scratch, Slot, SlotKind, Typographer,
};

const LANGS: [&str; 2] = ["ru-RU", "en-US"];

fn corpus_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/corpus")
}

fn read(lang: &str, file: &str) -> String {
    let path = corpus_dir().join(lang).join(file);
    fs::read_to_string(&path).unwrap_or_else(|e| panic!("cannot read {}: {e}", path.display()))
}

fn outputs() -> Vec<(&'static str, Output)> {
    vec![
        ("unicode", Output::Unicode),
        ("html-named", Output::Html(HtmlStyle::NAMED)),
        (
            "html-chars-nowrap-class",
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

fn language(tag: &str) -> Language {
    Language::from_tag(tag).unwrap_or_else(|e| panic!("no language {tag}: {e}"))
}

fn median(times: &mut [Duration]) -> Duration {
    times.sort();
    times[times.len() / 2]
}

/// A message of `ui.tmpl`: texts and slot kinds, in order.
enum Part {
    Text(String),
    Slot(String, SlotKind),
}

fn parse_tmpl(source: &str) -> Vec<Vec<Part>> {
    let mut messages = Vec::new();
    for line in source.lines() {
        if line.trim().is_empty() || line.starts_with('#') {
            continue;
        }
        let Some((_, text)) = line.split_once(" = ") else {
            continue;
        };
        let text = text.replace('⍽', "\u{a0}").replace('⎵', "\u{202f}");
        let mut parts = Vec::new();
        let mut rest = text.as_str();
        while let Some(open) = rest.find('{') {
            let Some(close) = rest[open..].find('}').map(|c| c + open) else {
                break;
            };
            if open > 0 {
                parts.push(Part::Text(rest[..open].to_owned()));
            }
            let body = &rest[open + 1..close];
            let (name, kind) = match body.split_once(':') {
                Some((n, "number")) => (n, SlotKind::Number),
                Some((n, "word")) => (n, SlotKind::Word),
                Some((n, "date")) => (n, SlotKind::Date),
                Some((n, _)) => (n, SlotKind::Other),
                None => (body, SlotKind::Other),
            };
            parts.push(Part::Slot(name.to_owned(), kind));
            rest = &rest[close + 1..];
        }
        if !rest.is_empty() {
            parts.push(Part::Text(rest.to_owned()));
        }
        messages.push(parts);
    }
    messages
}

fn mb_per_s(bytes: usize, time: Duration) -> f64 {
    bytes as f64 / 1e6 / time.as_secs_f64()
}

fn corpus_throughput() {
    println!("== corpus throughput, MB/s of input (median of 15 passes over a file) ==");
    println!(
        "{:<7} {:<10} {:<10} {:<24} {:>8} {:>10}",
        "lang", "set", "file", "output", "bytes", "MB/s"
    );
    for lang in LANGS {
        let ui = parse_tmpl(&read(lang, "ui.tmpl"));
        let ui_bytes: usize = ui
            .iter()
            .flatten()
            .map(|p| match p {
                Part::Text(t) => t.len(),
                Part::Slot(..) => 0,
            })
            .sum();
        for (set_name, set) in [("prose", RuleSet::PROSE), ("interface", RuleSet::INTERFACE)] {
            let t = Typographer::new(language(lang), set);
            let mut scratch = Scratch::new();
            for file in ["prose.txt", "tech.txt", "ui.tmpl"] {
                let lines: Vec<String> = if file == "ui.tmpl" {
                    Vec::new()
                } else {
                    read(lang, file).lines().map(str::to_owned).collect()
                };
                let bytes = if file == "ui.tmpl" {
                    ui_bytes
                } else {
                    lines.iter().map(String::len).sum()
                };
                for (output_name, output) in outputs() {
                    let mut times = Vec::new();
                    for _ in 0..15 {
                        let start = Instant::now();
                        if file == "ui.tmpl" {
                            for message in &ui {
                                let pieces = message.iter().map(|p| match p {
                                    Part::Text(s) => Piece::Text(s.as_str()),
                                    Part::Slot(n, k) => Piece::Slot(n.as_str(), Slot::new(*k)),
                                });
                                black_box(t.typeset_template_with(&mut scratch, pieces, output));
                            }
                        } else {
                            for line in &lines {
                                black_box(t.typeset_with(&mut scratch, black_box(line), output));
                            }
                        }
                        times.push(start.elapsed());
                    }
                    println!(
                        "{lang:<7} {set_name:<10} {file:<10} {output_name:<24} {bytes:>8} {:>10.1}",
                        mb_per_s(bytes, median(&mut times))
                    );
                }
            }
        }
    }
}

fn latency() {
    println!("\n== one interface string, ns per call over 100000 calls ==");
    println!(
        "{:<7} {:<44} {:<10} {:>8} {:>8}",
        "lang", "string", "path", "median", "p99"
    );
    let cases = [
        ("ru-RU", "Удалено \"3\" файла - 12 шт. за 4 с. Сохранить?"),
        ("ru-RU", "Сохранить изменения для вашей учётной записи"),
        ("en-US", "Deleted \"3\" files - 12 pcs. in 4 s. Save it?"),
        ("en-US", "Save the changes for your own account page"),
        ("ru-RU", "Сохранить"),
        ("en-US", "Save"),
    ];
    for (lang, text) in cases {
        let t = Typographer::new(language(lang), RuleSet::INTERFACE);
        let mut scratch = Scratch::new();
        for path in ["typeset", "typeset_with"] {
            let mut times = Vec::with_capacity(100_000);
            for i in 0..110_000 {
                let start = Instant::now();
                if path == "typeset" {
                    black_box(t.typeset(black_box(text), Output::Unicode));
                } else {
                    black_box(t.typeset_with(&mut scratch, black_box(text), Output::Unicode));
                }
                let elapsed = start.elapsed();
                if i >= 10_000 {
                    times.push(elapsed);
                }
            }
            times.sort();
            let p99 = times[times.len() * 99 / 100];
            let med = times[times.len() / 2];
            let shown: String = text.chars().take(40).collect();
            println!(
                "{lang:<7} {shown:<44} {path:<10} {:>8} {:>8}",
                med.as_nanos(),
                p99.as_nanos()
            );
        }
    }
}

fn joined_corpus(size: usize) -> String {
    let mut text = String::new();
    'fill: loop {
        for lang in LANGS {
            for file in ["prose.txt", "tech.txt"] {
                for line in read(lang, file).lines() {
                    text.push_str(line);
                    text.push('\n');
                    if text.len() >= size {
                        break 'fill;
                    }
                }
            }
        }
    }
    text
}

fn scaling() {
    println!(
        "\n== time per byte by size, ns/B (median of 5; 1 run at 16 MiB), ru-RU prose, Unicode =="
    );
    println!(
        "{:<22} {:>10} {:>10} {:>10} {:>10}",
        "input", "1 KiB", "64 KiB", "1 MiB", "16 MiB"
    );
    let t = Typographer::new(Language::RU_RU, RuleSet::PROSE);
    let mut scratch = Scratch::new();
    let mut row = |name: &str, make: &dyn Fn(usize) -> String| {
        let mut cells = Vec::new();
        for size in [1 << 10, 64 << 10, 1 << 20, 16 << 20] {
            let text = make(size);
            let runs = if size >= 16 << 20 { 1 } else { 5 };
            let mut times: Vec<Duration> = (0..runs)
                .map(|_| {
                    let start = Instant::now();
                    black_box(t.typeset_with(&mut scratch, black_box(&text), Output::Unicode));
                    start.elapsed()
                })
                .collect();
            cells.push(median(&mut times).as_secs_f64() * 1e9 / text.len() as f64);
        }
        println!(
            "{name:<22} {:>10.2} {:>10.2} {:>10.2} {:>10.2}",
            cells[0], cells[1], cells[2], cells[3]
        );
    };
    row("joined corpus (lines)", &joined_corpus);
    // One paragraph: the corpus lines joined with spaces, no line break.
    row("one paragraph", &|size| {
        let mut text = joined_corpus(size).replace('\n', " ");
        text.truncate(text.trim_end().len());
        text
    });
}

fn pathological() {
    println!(
        "\n== pathological inputs, ns/B (median of 3; 1 run at 16 MiB), ru-RU prose and en-US interface =="
    );
    println!(
        "{:<34} {:<16} {:>9} {:>9} {:>9} {:>7}",
        "input", "config", "64 KiB", "1 MiB", "16 MiB", "ratio"
    );
    let configs = [
        ("ru-RU prose", Language::RU_RU, RuleSet::PROSE),
        ("en-US interface", Language::EN_US, RuleSet::INTERFACE),
    ];
    for (name, make) in PATHOLOGICAL {
        for (config, lang, set) in configs.iter().cloned() {
            let t = Typographer::new(lang, set);
            let mut scratch = Scratch::new();
            let mut cells = Vec::new();
            for size in [64usize << 10, 1 << 20, 16 << 20] {
                let text = make(size);
                let runs = if size >= 16 << 20 {
                    1
                } else if size >= 1 << 20 {
                    3
                } else {
                    9
                };
                let mut times: Vec<Duration> = (0..runs)
                    .map(|_| {
                        let start = Instant::now();
                        black_box(t.typeset_with(&mut scratch, black_box(&text), Output::Unicode));
                        start.elapsed()
                    })
                    .collect();
                let per = median(&mut times).as_secs_f64() * 1e9 / text.len() as f64;
                cells.push(per);
                // A quadratic input would not finish at 16 MiB: stop after 1 MiB.
                if size == 1 << 20 && per > cells[0] * 20.0 {
                    break;
                }
            }
            let large = cells
                .get(2)
                .map_or_else(|| "skipped".to_owned(), |c| format!("{c:.2}"));
            let ratio = cells
                .get(2)
                .map_or_else(|| "-".to_owned(), |c| format!("{:.2}", c / cells[0]));
            println!(
                "{name:<34} {config:<16} {:>9.2} {:>9.2} {large:>9} {ratio:>7}",
                cells[0], cells[1]
            );
        }
    }
}

/// `VmHWM` (peak resident set) and `VmRSS` of this process in KiB, from `/proc/self/status`.
fn memory_kib() -> (u64, u64) {
    let status = fs::read_to_string("/proc/self/status").unwrap_or_default();
    let field = |name: &str| {
        status
            .lines()
            .find_map(|l| l.strip_prefix(name))
            .and_then(|v| v.trim().trim_end_matches("kB").trim().parse().ok())
            .unwrap_or(0)
    };
    (field("VmHWM:"), field("VmRSS:"))
}

/// The child process of the memory measurement: one paragraph, one call, peak before and after.
fn peak_child(clusters: usize, output_name: &str) {
    let family = "\u{1F468}\u{200D}\u{1F469}\u{200D}\u{1F467}\u{200D}\u{1F466}";
    let text = family.repeat(clusters);
    let output = outputs()
        .into_iter()
        .find(|(n, _)| *n == output_name)
        .map_or(Output::Unicode, |(_, o)| o);
    let t = Typographer::new(Language::RU_RU, RuleSet::PROSE);
    let (peak_before, rss_before) = memory_kib();
    let mut scratch = Scratch::new();
    let result = t.typeset_with(&mut scratch, black_box(&text), output);
    black_box(&result);
    let (peak_after, rss_after) = memory_kib();
    println!(
        "{clusters} {} {peak_before} {rss_before} {peak_after} {rss_after}",
        text.len()
    );
}

fn memory() {
    println!(
        "\n== peak memory for one paragraph of ZWJ emoji clusters (child process per row), KiB =="
    );
    println!(
        "{:<10} {:>10} {:>12} {:>12} {:>14} {:>14}",
        "clusters", "bytes", "peak before", "peak after", "growth (KiB)", "growth/cluster B"
    );
    let Ok(exe) = std::env::current_exe() else {
        println!("no current_exe");
        return;
    };
    for clusters in [10_000usize, 100_000, 1_000_000] {
        let out = Command::new(&exe)
            .args(["--peak", &clusters.to_string(), "unicode"])
            .output();
        let Ok(out) = out else { continue };
        let line = String::from_utf8_lossy(&out.stdout);
        let nums: Vec<u64> = line
            .split_whitespace()
            .filter_map(|w| w.parse().ok())
            .collect();
        if let [c, bytes, pb, _rb, pa, _ra] = nums[..] {
            let growth = pa.saturating_sub(pb);
            println!(
                "{c:<10} {bytes:>10} {pb:>12} {pa:>12} {growth:>14} {:>14.0}",
                growth as f64 * 1024.0 / c as f64
            );
        }
    }
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.get(1).map(String::as_str) == Some("--peak") {
        let clusters = args.get(2).and_then(|a| a.parse().ok()).unwrap_or(100_000);
        peak_child(clusters, args.get(3).map_or("unicode", String::as_str));
        return;
    }
    let only = args.iter().skip(1).find(|a| !a.starts_with('-')).cloned();
    let wants = |name: &str| only.as_deref().is_none_or(|o| o == name);
    if wants("corpus") {
        corpus_throughput();
    }
    if wants("latency") {
        latency();
    }
    if wants("scaling") {
        scaling();
    }
    if wants("pathological") {
        pathological();
    }
    if wants("memory") {
        memory();
    }
    let _ = repeat_to;
}
