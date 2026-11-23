//! Linearity of the pathological inputs (design sections 3.12 and 13.8): `load_is_linear` is
//! `#[ignore]`, because it needs a release build and time:
//! `cargo test --release -p typolish --test load -- --ignored` from `dev/`.
//! `generators_keep_their_size` checks the sizes of the generated inputs in every run.
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
#[ignore = "needs a release build and time; run with --release -- --ignored"]
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
