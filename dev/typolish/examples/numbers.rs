//! `format_number` and `format_range` for ru-RU and en-US (design section 2.7): grouping, the
//! decimal separator and the minus sign of each language, decimal strings, rounding, the symbols of
//! `NaN` and infinity, ranges, and the same values in HTML with and without nowrap elements.
//!
//! Run with `cargo run --example numbers`.

#![allow(
    clippy::print_stdout,
    clippy::print_stderr,
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic
)]

use core::fmt::Write;

use typolish::{HtmlStyle, Language, Number, Output, RuleSet, Typographer};

static RU: Typographer = Typographer::new(Language::RU_RU, RuleSet::PROSE);
static EN: Typographer = Typographer::new(Language::EN_US, RuleSet::PROSE);

/// U+00A0 and U+202F shown as visible markers, so the output can be read in a terminal.
fn visible(s: &str) -> String {
    s.replace('\u{a0}', "⍽").replace('\u{202f}', "⎵")
}

fn main() -> Result<(), typolish::Error> {
    let values: [(&str, Number<'static>); 9] = [
        ("123", Number::from(123)),
        ("1234", Number::from(1234)),
        ("-1234567.5", Number::from(-1234567.5)),
        ("0.1f32", Number::from(0.1f32)),
        ("u128::MAX", Number::from(u128::MAX)),
        ("fixed(2.675, 2)", Number::fixed(2.675, 2)),
        ("fixed(-0.001, 1)", Number::fixed(-0.001, 1)),
        ("NaN", Number::from(f64::NAN)),
        ("-inf", Number::from(f64::NEG_INFINITY)),
    ];
    println!("{:<18} {:<34} en-US", "value", "ru-RU");
    for (name, n) in values {
        let ru = RU.format_number(n, Output::Unicode).to_string();
        let en = EN.format_number(n, Output::Unicode).to_string();
        println!("{name:<18} {:<34} {en}", visible(&ru));
    }

    // Digits as the caller wrote them: leading zeros and precision are kept, `+` is not written.
    for text in ["007", "+1.50", "-0.00", "0012345"] {
        let n = Number::decimal(text)?;
        println!(
            "decimal {text:<10} ru {:<14} en {}",
            visible(&RU.format_number(n, Output::Unicode).to_string()),
            EN.format_number(n, Output::Unicode)
        );
    }
    assert!(Number::decimal("1e3").is_err());

    // Ranges: the CLDR pattern for every pair; negative and equal ends are kept.
    for (a, b) in [(8000, 8100), (-5, -1), (8000, 8000)] {
        println!(
            "range {a}..{b}: ru {}  en {}",
            visible(&RU.format_range(a, b, Output::Unicode).to_string()),
            EN.format_range(a, b, Output::Unicode)
        );
    }

    // The same values for an HTML page: entities, or one nowrap element around a number with glue.
    let named = Output::Html(HtmlStyle::NAMED);
    let nowrap = Output::Html(HtmlStyle::NAMED.nowrap_class("nowrap"));
    println!("html  {}", RU.format_number(-12345, named));
    println!("html  {}", RU.format_number(-12345, nowrap));
    println!("html  {}", RU.format_number(123, nowrap));
    println!("html  {}", RU.format_range(8000, 8100, nowrap));

    // No allocation: the display writes straight into a buffer with room.
    let mut buf = String::with_capacity(64);
    write!(buf, "{} МБ", RU.format_number(1536, Output::Unicode)).expect("a String accepts writes");
    println!("buffer {}", visible(&buf));
    Ok(())
}
