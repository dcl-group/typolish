//! Linearity of the pathological inputs (design sections 3.12 and 13.8): `load_is_linear` is
//! `#[ignore]`, because it needs a release build and time:
//! `cargo test --release -p typolish --test load -- --ignored` from `dev/`.
//! `to_plain_is_linear_on_prose` (also `#[ignore]`) does the same for `to_plain` on ordinary prose,
//! `to_plain_is_linear_on_many_paragraphs_with_broken_quotes` on many short paragraphs whose
//! quotes do not come back as double quotes, and `to_plain_is_linear_on_one_long_paragraph_of_quotes`
//! on long runs of quote characters in one paragraph (design section 12.1). `generators_keep_their_size`
//! checks the sizes of the generated inputs in every run.
//!
//! The time per byte for 16 MiB is at most twice that for 64 KiB, separately for each input. There
//! are no absolute thresholds: the machine is shared.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    clippy::print_stdout,
    clippy::print_stderr,
    clippy::dbg_macro
)]

#[path = "common/load_inputs.rs"]
mod load_inputs;

use std::hint::black_box;
use std::time::{Duration, Instant};

use load_inputs::PATHOLOGICAL;
use typolish::{Language, Output, RuleSet, Scratch, Typographer};

const SMALL: usize = 64 * 1024;
const MIDDLE: usize = 1024 * 1024;
const LARGE: usize = 16 * 1024 * 1024;

/// The median of `runs` timings of one call, in seconds per byte.
fn per_byte(t: &Typographer, scratch: &mut Scratch, text: &str, runs: usize) -> f64 {
    let mut times: Vec<Duration> = (0..runs)
        .map(|_| {
            let start = Instant::now();
            black_box(t.typeset_with(scratch, black_box(text), Output::Unicode));
            start.elapsed()
        })
        .collect();
    times.sort();
    times[times.len() / 2].as_secs_f64() / text.len() as f64
}

#[test]
fn generators_keep_their_size() {
    // The contract of `PATHOLOGICAL`: never fewer than `size` bytes, at most a few dozen more.
    for (name, make) in PATHOLOGICAL {
        for size in [1, 7, 100, 4096, 65536] {
            let len = make(size).len();
            assert!(
                size <= len && len <= size + 64,
                "{name}: {len} bytes for {size}"
            );
        }
    }
}

#[test]
#[ignore = "needs a release build and time; run with cargo test --release --test load -- --ignored"]
fn load_is_linear() {
    let configs = [
        ("ru-RU prose", Language::RU_RU, RuleSet::PROSE),
        ("en-US interface", Language::EN_US, RuleSet::INTERFACE),
    ];
    let mut failures = Vec::new();
    for (config, language, rules) in configs {
        let t = Typographer::new(language, rules);
        let mut scratch = Scratch::new();
        for (name, make) in PATHOLOGICAL {
            let small_text = make(SMALL);
            let small = per_byte(&t, &mut scratch, &small_text, 15);
            // A cheap guard first: a quadratic input would not finish at 16 MiB.
            let middle_text = make(MIDDLE);
            let middle = per_byte(&t, &mut scratch, &middle_text, 1);
            if middle > small * 8.0 {
                println!(
                    "{config} / {name}: 64 KiB {:.1} ns/B, 1 MiB {:.1} ns/B",
                    small * 1e9,
                    middle * 1e9
                );
                failures.push(format!("{config} / {name}: nonlinear already at 1 MiB"));
                continue;
            }
            drop(middle_text);
            let large_text = make(LARGE);
            let large = per_byte(&t, &mut scratch, &large_text, 1);
            let ratio = large / small;
            println!(
                "{config} / {name}: 64 KiB {:.1} ns/B, 1 MiB {:.1} ns/B, 16 MiB {:.1} ns/B, ratio {ratio:.2}",
                small * 1e9,
                middle * 1e9,
                large * 1e9
            );
            if ratio > 2.0 {
                failures.push(format!("{config} / {name}: ratio {ratio:.2} > 2"));
            }
        }
    }
    assert!(failures.is_empty(), "nonlinear inputs: {failures:#?}");
}

/// The median of `runs` timings of `to_plain` on one text, in seconds per byte.
fn plain_per_byte(t: &Typographer, text: &str, runs: usize) -> f64 {
    let mut times: Vec<Duration> = (0..runs)
        .map(|_| {
            let start = Instant::now();
            black_box(t.to_plain(black_box(text)));
            start.elapsed()
        })
        .collect();
    times.sort();
    times[times.len() / 2].as_secs_f64() / text.len() as f64
}

/// `to_plain` on ordinary prose whose readings the forward check rejects (an English dash with
/// spaces, which the closed style would not write) is linear in one call, whether the text is one
/// line or many paragraphs (design section 12.1); `lint` runs the same check. The time per byte
/// for 512 KiB is at most twice that for 64 KiB.
#[test]
#[ignore = "needs a release build and time; run with cargo test --release --test load -- --ignored"]
fn to_plain_is_linear_on_prose() {
    let t = Typographer::new(Language::EN_US, RuleSet::PROSE);
    let sentences = [
        "The answer — as always — is no. ",
        "This is an ordinary sentence of English prose that has a spaced dash — like this one — \
         and goes on for a while before it ends.\n\n",
    ];
    let mut failures = Vec::new();
    for sentence in sentences {
        let make = |size: usize| sentence.repeat(size / sentence.len() + 1);
        let small = plain_per_byte(&t, &make(SMALL), 3);
        let large = plain_per_byte(&t, &make(8 * SMALL), 1);
        let ratio = large / small;
        println!(
            "{sentence:?}: 64 KiB {:.1} ns/B, 512 KiB {:.1} ns/B, ratio {ratio:.2}",
            small * 1e9,
            large * 1e9
        );
        if ratio > 2.0 {
            failures.push(format!("{sentence:?}: ratio {ratio:.2} > 2"));
        }
    }
    assert!(failures.is_empty(), "nonlinear inputs: {failures:#?}");
}

/// `to_plain` on many short paragraphs whose quote readings do not come back as double quotes is
/// linear: the bookkeeping of the windows that stay broken is linear in their number (design
/// section 12.1). `«x „“»` stays broken in every tier; `«„«6»“»` is settled by the last one. The
/// time per byte for 512 KiB is at most twice that for 64 KiB.
#[test]
#[ignore = "needs a release build and time; run with cargo test --release --test load -- --ignored"]
fn to_plain_is_linear_on_many_paragraphs_with_broken_quotes() {
    let t = Typographer::new(Language::RU_RU, RuleSet::PROSE);
    let mut failures = Vec::new();
    for paragraph in ["«x „“»\n\n", "«„«6»“»\n\n"] {
        let make = |size: usize| paragraph.repeat(size / paragraph.len() + 1);
        let small = plain_per_byte(&t, &make(SMALL), 3);
        let large = plain_per_byte(&t, &make(8 * SMALL), 1);
        let ratio = large / small;
        println!(
            "{paragraph:?}: 64 KiB {:.1} ns/B, 512 KiB {:.1} ns/B, ratio {ratio:.2}",
            small * 1e9,
            large * 1e9
        );
        if ratio > 2.0 {
            failures.push(format!("{paragraph:?}: ratio {ratio:.2} > 2"));
        }
    }
    assert!(failures.is_empty(), "nonlinear inputs: {failures:#?}");
}

/// `to_plain` on one long paragraph of quote characters is linear: the neighbors of a series of
/// quotes are found once for the series, the open pairs are kept with the places of each kind, and
/// the later closers of design section 12.1 are found in one pass. `’` repeated and `«„` repeated,
/// then `x`, then `“»` repeated, in Russian Prose (the measurement of design section 12.1); `«`
/// repeated around `x`, deep in one kind of pair; and in English nested pairs whose `’` after `s`
/// wait for a later closer. The time per byte for 512 KiB is at most twice that for 64 KiB.
#[test]
#[ignore = "needs a release build and time; run with cargo test --release --test load -- --ignored"]
fn to_plain_is_linear_on_one_long_paragraph_of_quotes() {
    let ru = Typographer::new(Language::RU_RU, RuleSet::PROSE);
    let en = Typographer::new(Language::EN_US, RuleSet::PROSE);
    // A text of about `size` bytes: `open` repeated, then `middle`, then `close` repeated.
    let make = |open: &str, middle: &str, close: &str, size: usize| {
        let n = size / (open.len() + close.len()) + 1;
        format!("{}{middle}{}", open.repeat(n), close.repeat(n))
    };
    let inputs: [(&str, &Typographer, &str, &str, &str); 4] = [
        ("ru `’`", &ru, "’", "", ""),
        ("ru `«„` x `“»`", &ru, "«„", "x", "“»"),
        ("ru `«` x `»`", &ru, "«", "x", "»"),
        ("en `“‘a dogs’ ` b `’”`", &en, "“‘a dogs’ ", "b", "’”"),
    ];
    let mut failures = Vec::new();
    for (name, t, open, middle, close) in inputs {
        let small = plain_per_byte(t, &make(open, middle, close, SMALL), 3);
        let large = plain_per_byte(t, &make(open, middle, close, 8 * SMALL), 1);
        let ratio = large / small;
        println!(
            "{name}: 64 KiB {:.1} ns/B, 512 KiB {:.1} ns/B, ratio {ratio:.2}",
            small * 1e9,
            large * 1e9
        );
        if ratio > 2.0 {
            failures.push(format!("{name}: ratio {ratio:.2} > 2"));
        }
    }
    assert!(failures.is_empty(), "nonlinear inputs: {failures:#?}");
}
