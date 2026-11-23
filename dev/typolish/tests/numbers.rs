//! `format_number` and `format_range` (design sections 2.7, 2.11, 7.2 to 7.4, 11.5, 11.7 and
//! 13.2): integers, floats, decimal strings, grouping and its settings, the symbols of the
//! language, ranges, every output kind and the nowrap styles.
//!
//! The expectations are written with the markers of `common::vis` (`⍽` is U+00A0, `⎵` is U+202F).
//! Values of the widest types and of `f32::MAX` are built from the digits that core's `{}` writes,
//! since the design says `f32` and `f64` use the shortest representation of core (design section
//! 2.7).

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

use common::decode::{all_outputs, decode};
use common::{show, vis};
use typolish::{
    Error, EscapeStyle, HtmlStyle, Language, LanguageBuilder, Number, NumberData, Output, Rule,
    RuleSet, Typographer,
};

static RU: Typographer = Typographer::new(Language::RU_RU, RuleSet::PROSE);
static EN: Typographer = Typographer::new(Language::EN_US, RuleSet::PROSE);
static COMMON: Typographer = Typographer::new(Language::COMMON, RuleSet::PROSE);

const UNICODE: Output = Output::Unicode;

fn num<'n>(t: &Typographer, n: impl Into<Number<'n>>) -> String {
    t.format_number(n, UNICODE).to_string()
}

fn dec(t: &Typographer, text: &str) -> String {
    t.format_number(Number::decimal(text).unwrap(), UNICODE)
        .to_string()
}

#[track_caller]
fn same(what: &str, got: String, expected: &str) {
    let expected = vis(expected);
    assert!(
        got == expected,
        "{what}\nexpected {}\ngot      {}",
        show(&expected),
        show(&got)
    );
}

/// The number as core's `{}` writes it, grouped from `threshold` integer digits with `sep`, the
/// decimal point replaced by `decimal` and a leading `-` by U+2212.
fn from_digits(text: &str, sep: &str, decimal: &str, threshold: usize) -> String {
    let (sign, rest) = match text.strip_prefix('-') {
        Some(rest) => ("\u{2212}", rest),
        None => ("", text),
    };
    let (int, frac) = match rest.split_once('.') {
        Some((i, f)) => (i, Some(f)),
        None => (rest, None),
    };
    let mut grouped = String::new();
    for (i, c) in int.chars().enumerate() {
        if int.len() >= threshold && i > 0 && (int.len() - i) % 3 == 0 {
            grouped.push_str(sep);
        }
        grouped.push(c);
    }
    let mut out = format!("{sign}{grouped}");
    if let Some(f) = frac {
        out.push_str(decimal);
        out.push_str(f);
    }
    out
}

// Integers: the group separator and the minus sign of the language; ru and en group from four
// digits (`min_grouping_digits = 1`, design section 15.5, item 4).
const INTEGERS: &[(i64, &str, &str)] = &[
    (0, "0", "0"),
    (5, "5", "5"),
    (42, "42", "42"),
    (99, "99", "99"),
    (999, "999", "999"),
    (1000, "1⍽000", "1,000"),
    (1234, "1⍽234", "1,234"),
    (12345, "12⍽345", "12,345"),
    (123456, "123⍽456", "123,456"),
    (1234567, "1⍽234⍽567", "1,234,567"),
    (1234567890, "1⍽234⍽567⍽890", "1,234,567,890"),
    (-1, "−1", "−1"),
    (-999, "−999", "−999"),
    (-1000, "−1⍽000", "−1,000"),
    (-12345, "−12⍽345", "−12,345"),
    (
        i64::MIN,
        "−9⍽223⍽372⍽036⍽854⍽775⍽808",
        "−9,223,372,036,854,775,808",
    ),
    (
        i64::MAX,
        "9⍽223⍽372⍽036⍽854⍽775⍽807",
        "9,223,372,036,854,775,807",
    ),
];

#[test]
fn integers() {
    for (n, ru, en) in INTEGERS {
        same(&format!("ru {n}"), num(&RU, *n), ru);
        same(&format!("en {n}"), num(&EN, *n), en);
    }
}

#[test]
fn every_integer_type_converts() {
    same("i8", num(&RU, i8::MIN), "−128");
    same("u8", num(&RU, u8::MAX), "255");
    same("i16", num(&RU, i16::MIN), "−32⍽768");
    same("u16", num(&RU, u16::MAX), "65⍽535");
    same("i32", num(&RU, i32::MIN), "−2⍽147⍽483⍽648");
    same("u32", num(&RU, u32::MAX), "4⍽294⍽967⍽295");
    same("i64", num(&RU, i64::MIN), "−9⍽223⍽372⍽036⍽854⍽775⍽808");
    same("u64", num(&RU, u64::MAX), "18⍽446⍽744⍽073⍽709⍽551⍽615");
    same("isize", num(&RU, -12345isize), "−12⍽345");
    same("usize", num(&RU, 12345usize), "12⍽345");
    #[cfg(target_pointer_width = "64")]
    {
        same(
            "isize::MIN",
            num(&RU, isize::MIN),
            "−9⍽223⍽372⍽036⍽854⍽775⍽808",
        );
        same(
            "usize::MAX",
            num(&RU, usize::MAX),
            "18⍽446⍽744⍽073⍽709⍽551⍽615",
        );
    }
    same("i128", num(&EN, 1234i128), "1,234");
    same("u128", num(&EN, 1234u128), "1,234");
    same("f32", num(&EN, 1234.5f32), "1,234.5");
    same("f64", num(&EN, 1234.5f64), "1,234.5");
}

#[test]
fn the_widest_integers_do_not_overflow() {
    // `i128::MIN` has no positive counterpart: the integers go through `unsigned_abs()`.
    same(
        "i128::MIN ru",
        num(&RU, i128::MIN),
        "−170⍽141⍽183⍽460⍽469⍽231⍽731⍽687⍽303⍽715⍽884⍽105⍽728",
    );
    same(
        "i128::MIN en",
        num(&EN, i128::MIN),
        "−170,141,183,460,469,231,731,687,303,715,884,105,728",
    );
    same(
        "i128::MAX ru",
        num(&RU, i128::MAX),
        "170⍽141⍽183⍽460⍽469⍽231⍽731⍽687⍽303⍽715⍽884⍽105⍽727",
    );
    same(
        "u128::MAX ru",
        num(&RU, u128::MAX),
        "340⍽282⍽366⍽920⍽938⍽463⍽463⍽374⍽607⍽431⍽768⍽211⍽455",
    );
    same(
        "u128::MAX en",
        num(&EN, u128::MAX),
        "340,282,366,920,938,463,463,374,607,431,768,211,455",
    );
}

#[test]
fn floats() {
    same("ru 1234567.5", num(&RU, -1234567.5), "−1⍽234⍽567,5");
    same("en 1234567.5", num(&EN, -1234567.5), "−1,234,567.5");
    same("ru 0.5", num(&RU, 0.5), "0,5");
    same("en 0.5", num(&EN, 0.5), "0.5");
    same("ru 3.0", num(&RU, 3.0), "3");
    same("ru 0.1", num(&RU, 0.1), "0,1");
    same(
        "ru 0.30000000000000004",
        num(&RU, 0.1 + 0.2),
        "0,30000000000000004",
    );
    same("ru 12345.678", num(&RU, 12345.678), "12⍽345,678");
    // The fraction is not grouped.
    same("ru fraction", num(&RU, 0.123456789), "0,123456789");
    same("en fraction", num(&EN, 1234.123456), "1,234.123456");
    // `{}` of core writes no exponent, whatever the magnitude.
    same("ru 1e21", num(&RU, 1e21), "1⍽000⍽000⍽000⍽000⍽000⍽000⍽000");
    same("ru 1e-7", num(&RU, 1e-7), "0,0000001");
    let tiny = from_digits(&format!("{}", 5e-324), "\u{a0}", ",", 4);
    assert_eq!(num(&RU, 5e-324), tiny);
    assert!(tiny.starts_with("0,0000") && tiny.ends_with('5'));
    let big = from_digits(&format!("{}", f64::MAX), "\u{a0}", ",", 4);
    assert_eq!(num(&RU, f64::MAX), big);
    assert!(big.starts_with("179⍽769⍽313⍽486⍽231⍽57".replace('⍽', "\u{a0}").as_str()));
    let negative = from_digits(&format!("{}", f64::MIN), ",", ".", 4);
    assert_eq!(num(&EN, f64::MIN), negative);
    assert!(negative.starts_with("\u{2212}179,769,313,486,231,57"));
}

#[test]
fn f32_keeps_its_own_shortest_representation() {
    same("ru 0.1f32", num(&RU, 0.1f32), "0,1");
    same("ru 1.1f32", num(&RU, 1.1f32), "1,1");
    same("en 1.1f32", num(&EN, 1.1f32), "1.1");
    same("ru 16777216f32", num(&RU, 16777216f32), "16⍽777⍽216");
    same("ru -0.3f32", num(&RU, -0.3f32), "−0,3");
    // A widened `f64` would write `0,10000000149011612`.
    assert_ne!(num(&RU, 0.1f32), num(&RU, f64::from(0.1f32)));
    let max = from_digits(&format!("{}", f32::MAX), "\u{a0}", ",", 4);
    assert_eq!(num(&RU, f32::MAX), max);
    assert!(max.starts_with("340\u{a0}282\u{a0}35"), "{}", show(&max));
    let min = from_digits(&format!("{}", f32::MIN), ",", ".", 4);
    assert_eq!(num(&EN, f32::MIN), min);
    assert!(min.starts_with("\u{2212}340,282,35"));
    same("f32 nan", num(&RU, f32::NAN), "не⍽число");
    same("f32 inf", num(&RU, f32::INFINITY), "∞");
    same("f32 -inf", num(&RU, f32::NEG_INFINITY), "−∞");
    same("f32 -0.0", num(&RU, -0.0f32), "0");
}

#[test]
fn not_a_number_and_infinities() {
    same("ru NaN", num(&RU, f64::NAN), "не⍽число");
    same("en NaN", num(&EN, f64::NAN), "NaN");
    same("COMMON NaN", num(&COMMON, f64::NAN), "NaN");
    same("ru inf", num(&RU, f64::INFINITY), "∞");
    same("en inf", num(&EN, f64::INFINITY), "∞");
    // Negative infinity is the minus sign followed by the symbol of infinity.
    same("ru -inf", num(&RU, f64::NEG_INFINITY), "−∞");
    same("en -inf", num(&EN, f64::NEG_INFINITY), "−∞");
    // NaN is written without a sign, whatever its sign bit.
    let negative_nan = f64::from_bits(f64::NAN.to_bits() | (1 << 63));
    assert!(negative_nan.is_nan() && negative_nan.is_sign_negative());
    same("ru -NaN", num(&RU, negative_nan), "не⍽число");
    same("en -NaN", num(&EN, negative_nan), "NaN");
    same(
        "fixed NaN",
        num(&RU, Number::fixed(f64::NAN, 2)),
        "не⍽число",
    );
    same("fixed inf", num(&RU, Number::fixed(f64::INFINITY, 2)), "∞");
    same(
        "fixed -inf",
        num(&EN, Number::fixed(f64::NEG_INFINITY, 2)),
        "−∞",
    );
}

#[test]
fn negative_zero_and_zero_values_have_no_sign() {
    same("ru -0.0", num(&RU, -0.0), "0");
    same("en -0.0", num(&EN, -0.0), "0");
    same("ru 0.0", num(&RU, 0.0), "0");
    same("ru -0", num(&RU, 0i32), "0");
    same("ru decimal -0.00", dec(&RU, "-0.00"), "0,00");
    same("en decimal -0.00", dec(&EN, "-0.00"), "0.00");
    same("ru decimal -0", dec(&RU, "-0"), "0");
    same("ru decimal +0.0", dec(&RU, "+0.0"), "0,0");
    same("ru decimal -000", dec(&RU, "-000"), "000");
    same("ru fixed -0.001", num(&RU, Number::fixed(-0.001, 1)), "0,0");
    same("en fixed -0.001", num(&EN, Number::fixed(-0.001, 1)), "0.0");
    same("ru fixed -0.5", num(&RU, Number::fixed(-0.5, 0)), "0");
    same("ru fixed -1e-9", num(&RU, Number::fixed(-1e-9, 3)), "0,000");
    // A negative value that rounds to a nonzero number keeps its sign.
    same("ru fixed -0.06", num(&RU, Number::fixed(-0.06, 1)), "−0,1");
}

// Number::decimal keeps the digits as given: leading zeros, the caller's precision, a `+` that is
// not written (design sections 2.7 and 15.2, item 17). The integer part is grouped as written.
const DECIMALS: &[(&str, &str, &str)] = &[
    ("5", "5", "5"),
    ("+5", "5", "5"),
    ("-5", "−5", "−5"),
    ("007", "007", "007"),
    ("0012345", "0⍽012⍽345", "0,012,345"),
    ("-0012345", "−0⍽012⍽345", "−0,012,345"),
    ("1.50", "1,50", "1.50"),
    ("-1.50", "−1,50", "−1.50"),
    ("+1.50", "1,50", "1.50"),
    ("0.10", "0,10", "0.10"),
    ("1.0", "1,0", "1.0"),
    ("1234", "1⍽234", "1,234"),
    ("999", "999", "999"),
    ("1234567.891", "1⍽234⍽567,891", "1,234,567.891"),
    ("-1234567.8910", "−1⍽234⍽567,8910", "−1,234,567.8910"),
    // The fraction is not grouped and not trimmed.
    ("0.123456789", "0,123456789", "0.123456789"),
    ("1234.56789000", "1⍽234,56789000", "1,234.56789000"),
    // Digits beyond every integer and float type stay as given.
    (
        "123456789012345678901234567890123456789012345678901234567890",
        "123⍽456⍽789⍽012⍽345⍽678⍽901⍽234⍽567⍽890⍽123⍽456⍽789⍽012⍽345⍽678⍽901⍽234⍽567⍽890",
        "123,456,789,012,345,678,901,234,567,890,123,456,789,012,345,678,901,234,567,890",
    ),
    (
        "0.000000000000000000000000000001",
        "0,000000000000000000000000000001",
        "0.000000000000000000000000000001",
    ),
];

#[test]
fn decimal_strings() {
    for (text, ru, en) in DECIMALS {
        same(&format!("ru {text}"), dec(&RU, text), ru);
        same(&format!("en {text}"), dec(&EN, text), en);
    }
}

#[test]
fn decimal_rejects_what_section_2_11_lists() {
    for text in [
        // the empty string
        "",
        // a sign without digits
        "-",
        "+",
        // more than one sign, a sign that is not first or not `+` or `-`
        "--5",
        "+-5",
        "-+5",
        "++5",
        "5-",
        "5+",
        "\u{2212}5",
        "±5",
        "\u{2013}5",
        // no digit before or after the point
        ".5",
        "5.",
        "-.5",
        "+.5",
        "-5.",
        ".",
        "-.",
        // more than one point
        "1.2.3",
        "1..2",
        // an exponent
        "1e3",
        "1E3",
        "1e-3",
        "1.5e3",
        // a comma, a space or any other character
        "1,5",
        "1,000",
        " 5",
        "5 ",
        "1 000",
        "1\u{a0}000",
        "\t5",
        "5\n",
        "5a",
        "a5",
        "NaN",
        "inf",
        "-inf",
        "Infinity",
        "0x10",
        "5_000",
        "٣",
        "５",
        "1.5f",
        "١٢",
    ] {
        match Number::decimal(text) {
            Err(Error::InvalidNumber { input, reason, .. }) => {
                assert_eq!(input, text, "the error quotes the input");
                assert!(!reason.is_empty(), "{text:?}");
            }
            Err(other) => panic!("{text:?}: {other:?}"),
            Ok(n) => panic!("{text:?} was accepted: {n:?}"),
        }
    }
}

#[test]
fn the_invalid_number_error_names_the_input() {
    let err = Number::decimal("1e3").unwrap_err();
    // The text of the message is not a contract (design section 2.13); it names the input.
    assert!(err.to_string().contains("1e3"), "{err}");
}

#[test]
fn decimal_borrows_its_digits_from_the_caller() {
    let owned = String::from("-1234.50");
    let n = Number::decimal(&owned).unwrap();
    same("borrowed decimal", num(&RU, n), "−1⍽234,50");
    // `Number` is `Copy`: the same value formats twice.
    same("copied decimal", num(&EN, n), "−1,234.50");
}

// Number::fixed rounds half to even on the exact binary value, as `format!("{:.N}")` does.
#[test]
fn fixed_rounds_as_core_does() {
    let rows: &[(f64, u8, &str, &str)] = &[
        (0.5, 0, "0", "0"),
        (1.5, 0, "2", "2"),
        (2.5, 0, "2", "2"),
        (3.5, 0, "4", "4"),
        (0.125, 2, "0,12", "0.12"),
        (0.375, 2, "0,38", "0.38"),
        // Not ties in binary: 2.675 is a little below, 1.005 a little below.
        (2.675, 2, "2,67", "2.67"),
        (1.005, 2, "1,00", "1.00"),
        (1234.5678, 2, "1⍽234,57", "1,234.57"),
        (5.0, 3, "5,000", "5.000"),
        (-1.5, 0, "−2", "−2"),
        (-1234567.891, 1, "−1⍽234⍽567,9", "−1,234,567.9"),
        (0.0, 0, "0", "0"),
        (0.0, 4, "0,0000", "0.0000"),
        (999.5, 0, "1⍽000", "1,000"),
        (999.4, 0, "999", "999"),
        (1e15, 0, "1⍽000⍽000⍽000⍽000⍽000", "1,000,000,000,000,000"),
        (0.1, 20, "0,10000000000000000555", "0.10000000000000000555"),
    ];
    for (value, digits, ru, en) in rows {
        same(
            &format!("ru fixed({value}, {digits})"),
            num(&RU, Number::fixed(*value, *digits)),
            ru,
        );
        same(
            &format!("en fixed({value}, {digits})"),
            num(&EN, Number::fixed(*value, *digits)),
            en,
        );
        // The digits are those of `format!("{:.N}")`.
        let core = format!("{:.*}", usize::from(*digits), value);
        let expected = from_digits(&core, "\u{a0}", ",", 4);
        let zero = core
            .trim_start_matches('-')
            .chars()
            .all(|c| matches!(c, '0' | '.'));
        let expected = if zero {
            expected.replace('\u{2212}', "")
        } else {
            expected
        };
        assert_eq!(
            num(&RU, Number::fixed(*value, *digits)),
            expected,
            "{value} {digits}"
        );
    }
}

// The grouping threshold: ru and en group from four digits (`min_grouping_digits = 1`).
#[test]
fn the_default_threshold_is_four_digits() {
    same("ru 999", num(&RU, 999), "999");
    same("ru 1000", num(&RU, 1000), "1⍽000");
    same("en 999", num(&EN, 999), "999");
    same("en 1000", num(&EN, 1000), "1,000");
    same("ru -999", num(&RU, -999), "−999");
    same("ru -1000", num(&RU, -1000), "−1⍽000");
    // The sign is not a digit: -123 has three.
    same("ru decimal 999.99", dec(&RU, "999.99"), "999,99");
    same("ru decimal 1000.99", dec(&RU, "1000.99"), "1⍽000,99");
}

fn grouping(language: Language, digits: u8) -> Typographer {
    Typographer::builder(language, RuleSet::PROSE)
        .min_grouping_digits(digits)
        .build()
        .unwrap()
}

#[test]
fn min_grouping_digits_moves_the_threshold() {
    // 0 turns grouping off for numbers of every length.
    let none = grouping(Language::RU_RU, 0);
    same("0: 1234567890", num(&none, 1234567890u64), "1234567890");
    same("0: 1000", num(&none, 1000), "1000");
    same("0: decimal", dec(&none, "1234567.89"), "1234567,89");
    same("0: -1234567", num(&none, -1234567), "−1234567");
    same(
        "0: u128::MAX",
        num(&none, u128::MAX),
        &u128::MAX.to_string(),
    );
    // 1..=4: group from 3 + n digits (CLDR 49 meaning).
    for (digits, ungrouped, grouped, expected) in [
        (1u8, 999i64, 1000i64, "1⍽000"),
        (2, 9999, 10000, "10⍽000"),
        (3, 99999, 100000, "100⍽000"),
        (4, 999999, 1000000, "1⍽000⍽000"),
    ] {
        let t = grouping(Language::RU_RU, digits);
        assert_eq!(num(&t, ungrouped), ungrouped.to_string(), "{digits}");
        same(&format!("{digits}"), num(&t, grouped), expected);
    }
    let four = grouping(Language::RU_RU, 4);
    same("4: 123456", num(&four, 123456), "123456");
    same("4: 1234567", num(&four, 1234567), "1⍽234⍽567");
    same("4: 12345678", num(&four, 12345678), "12⍽345⍽678");
    same("4: decimal 123456.7", dec(&four, "123456.7"), "123456,7");
    same(
        "4: decimal 1234567.7",
        dec(&four, "1234567.7"),
        "1⍽234⍽567,7",
    );
    let two = grouping(Language::EN_US, 2);
    same("en 2: 1234", num(&two, 1234), "1234");
    same("en 2: 12345", num(&two, 12345), "12,345");
}

#[test]
fn common_does_not_group() {
    // `COMMON` has `min_grouping_digits = 0`, decimal `.` and the minus U+2212 (design section
    // 7.4).
    same("COMMON 1234567", num(&COMMON, 1234567), "1234567");
    same("COMMON -1234567.5", num(&COMMON, -1234567.5), "−1234567.5");
    same("COMMON decimal", dec(&COMMON, "1234567.50"), "1234567.50");
    same("COMMON 12", num(&COMMON, 12), "12");
    // Asking for grouping on `COMMON` uses its group separator, U+00A0.
    let grouped = grouping(Language::COMMON, 1);
    same("COMMON grouped", num(&grouped, 1234567), "1⍽234⍽567");
}

#[test]
fn the_rule_set_does_not_change_numbers() {
    // Settings do not depend on rule sets (design section 2.4): a set only switches rules on and
    // off.
    for set in [
        RuleSet::EMPTY,
        RuleSet::INTERFACE,
        RuleSet::PROSE,
        RuleSet::ALL,
    ] {
        let t = Typographer::new(Language::RU_RU, set);
        same(&format!("{set}"), num(&t, -1234567.5), "−1⍽234⍽567,5");
        // Not even `number.minus` being off changes the minus of a value.
        let t = Typographer::builder(Language::RU_RU, set)
            .disable(Rule::NumberMinus)
            .build()
            .unwrap();
        same("without number.minus", num(&t, -5), "−5");
    }
}

#[test]
fn ranges() {
    let rows: &[(i64, i64, &str, &str)] = &[
        (8000, 8100, "8⍽000–8⍽100", "8,000–8,100"),
        (1, 5, "1–5", "1–5"),
        (1941, 1945, "1⍽941–1⍽945", "1,941–1,945"),
        (999, 1000, "999–1⍽000", "999–1,000"),
        // Negative ends are written as they are.
        (-5, -1, "−5–−1", "−5–−1"),
        (-1000, 1000, "−1⍽000–1⍽000", "−1,000–1,000"),
        // Equal ends are not collapsed.
        (8000, 8000, "8⍽000–8⍽000", "8,000–8,000"),
        (5, 5, "5–5", "5–5"),
        // Descending ends are not reordered: the caller decides what is a range.
        (10, 5, "10–5", "10–5"),
    ];
    for (from, to, ru, en) in rows {
        same(
            &format!("ru {from}..{to}"),
            RU.format_range(*from, *to, UNICODE).to_string(),
            ru,
        );
        same(
            &format!("en {from}..{to}"),
            EN.format_range(*from, *to, UNICODE).to_string(),
            en,
        );
    }
    // The dash is U+2013, without spaces.
    assert!(
        RU.format_range(1, 2, UNICODE)
            .to_string()
            .contains('\u{2013}')
    );
}

#[test]
fn ranges_take_any_two_kinds_of_values() {
    same(
        "int and float",
        RU.format_range(1, 2.5, UNICODE).to_string(),
        "1–2,5",
    );
    same(
        "decimals",
        RU.format_range(
            Number::decimal("1.50").unwrap(),
            Number::decimal("-0.00").unwrap(),
            UNICODE,
        )
        .to_string(),
        "1,50–0,00",
    );
    same(
        "fixed",
        EN.format_range(Number::fixed(0.125, 2), Number::fixed(1234.5, 1), UNICODE)
            .to_string(),
        "0.12–1,234.5",
    );
    same(
        "nan and infinity",
        RU.format_range(f64::NAN, f64::INFINITY, UNICODE)
            .to_string(),
        "не⍽число–∞",
    );
    same(
        "f32",
        RU.format_range(0.1f32, 1.1f32, UNICODE).to_string(),
        "0,1–1,1",
    );
    same(
        "wide",
        EN.format_range(i128::MIN, u128::MAX, UNICODE).to_string(),
        "−170,141,183,460,469,231,731,687,303,715,884,105,728–340,282,366,920,938,463,463,374,607,431,768,211,455",
    );
}

#[test]
fn the_template_dash_and_the_range_dash_come_from_one_field() {
    use typolish::{Piece, Slot};
    for (language, html_dash) in [
        (Language::RU_RU, "&ndash;"),
        (Language::EN_US, "&ndash;"),
        (Language::COMMON, "&ndash;"),
    ] {
        let t = Typographer::new(language, RuleSet::PROSE);
        for (output, dash) in [
            (UNICODE, "\u{2013}"),
            (Output::Html(HtmlStyle::NAMED), html_dash),
        ] {
            let out = t.typeset_template(
                [
                    Piece::Slot("a", Slot::NUMBER),
                    Piece::Text("-"),
                    Piece::Slot("b", Slot::NUMBER),
                ],
                output,
            );
            assert_eq!(out.texts(), ["", dash, ""]);
            let range = t.format_range(1, 2, output).to_string();
            assert_eq!(range, format!("1{dash}2"));
        }
    }
    // A language with another dash in its number data: both follow it. The dash is one of the
    // two that design section 7.6 allows.
    let language = LanguageBuilder::from_language(&Language::COMMON, "xx")
        .numbers(NumberData::new().with_range_dash('\u{2014}'))
        .build()
        .unwrap();
    let t = Typographer::new(language, RuleSet::PROSE);
    let out = t.typeset_template(
        [
            Piece::Slot("a", Slot::NUMBER),
            Piece::Text("-"),
            Piece::Slot("b", Slot::NUMBER),
        ],
        UNICODE,
    );
    assert_eq!(out.texts(), ["", "\u{2014}", ""]);
    assert_eq!(t.format_range(1, 2, UNICODE).to_string(), "1\u{2014}2");
}

#[test]
fn a_language_built_from_data_formats_by_its_data() {
    let numbers = NumberData::new()
        .with_decimal(',')
        .with_group('.')
        .with_min_grouping_digits(2)
        .with_minus('-')
        .with_nan("keine Zahl")
        .with_infinity("unendlich");
    let language = LanguageBuilder::from_language(&Language::COMMON, "de-DE")
        .numbers(numbers)
        .build()
        .unwrap();
    let t = Typographer::new(language, RuleSet::PROSE);
    same("de 1234", num(&t, 1234), "1234");
    same("de 12345.5", num(&t, 12345.5), "12.345,5");
    same("de -1234567", num(&t, -1234567), "-1.234.567");
    same("de NaN", num(&t, f64::NAN), "keine Zahl");
    same("de inf", num(&t, f64::INFINITY), "unendlich");
    same("de -inf", num(&t, f64::NEG_INFINITY), "-unendlich");
    same(
        "de range",
        t.format_range(10000, 20000, UNICODE).to_string(),
        "10.000–20.000",
    );
    // A group separator that is not a space needs no glue: no element in nowrap mode.
    let nowrap = Output::Html(HtmlStyle::NAMED.nowrap_class("nowrap"));
    assert_eq!(t.format_number(12345, nowrap).to_string(), "12.345");
}

#[test]
fn builder_settings_change_the_symbols() {
    let t = Typographer::builder(Language::RU_RU, RuleSet::PROSE)
        .group_separator('.')
        .minus_sign('-')
        .build()
        .unwrap();
    same("ru . and -", num(&t, -1234567.5), "-1.234.567,5");
    let t = Typographer::builder(Language::EN_US, RuleSet::PROSE)
        .group_separator('\u{202f}')
        .build()
        .unwrap();
    same("en narrow", num(&t, 1234567.5), "1⎵234⎵567.5");
    let t = Typographer::builder(Language::EN_US, RuleSet::PROSE)
        .minus_sign('\u{2013}')
        .build()
        .unwrap();
    same("en en dash minus", num(&t, -5), "–5");
}

fn invalid(builder: typolish::TypographerBuilder, expected: &str) {
    match builder.build() {
        Err(Error::InvalidSetting {
            setting, reason, ..
        }) => {
            assert_eq!(setting, expected);
            assert!(!reason.is_empty());
        }
        Err(other) => panic!("{other:?}"),
        Ok(_) => panic!("{expected} was accepted"),
    }
}

fn base() -> typolish::TypographerBuilder {
    Typographer::builder(Language::RU_RU, RuleSet::PROSE)
}

#[test]
fn the_builder_rejects_separators_and_signs_that_break_the_text() {
    invalid(base().group_separator(' '), "group_separator");
    invalid(base().group_separator('\u{2009}'), "group_separator");
    invalid(base().group_separator('\t'), "group_separator");
    invalid(base().group_separator('5'), "group_separator");
    invalid(base().group_separator('a'), "group_separator");
    // A dash would make `12-345` the range `12–345` on a second pass (design section 7.6).
    invalid(base().group_separator('-'), "group_separator");
    invalid(base().minus_sign(' '), "minus_sign");
    invalid(base().minus_sign('\u{a0}'), "minus_sign");
    invalid(base().minus_sign('"'), "minus_sign");
    invalid(base().minus_sign('\''), "minus_sign");
    invalid(base().minus_sign('5'), "minus_sign");
    invalid(base().minus_sign('x'), "minus_sign");
    invalid(base().minus_sign('~'), "minus_sign");
    invalid(base().minus_sign('('), "minus_sign");
    invalid(base().min_grouping_digits(5), "min_grouping_digits");
    invalid(base().min_grouping_digits(255), "min_grouping_digits");
}

#[test]
fn the_builder_accepts_the_glue_characters_and_the_whole_scale() {
    // `,` is the decimal separator of ru-RU and no group separator there (design sections 2.11
    // and 7.6, `decimal != group`); en-US takes it.
    // A quote character is followed by three digits and is not read as a prime (`’` of de-CH).
    for separator in ['\u{a0}', '\u{202f}', '.', '_', '\u{2019}'] {
        assert!(
            base().group_separator(separator).build().is_ok(),
            "{separator:?}"
        );
    }
    assert!(
        Typographer::builder(Language::EN_US, RuleSet::PROSE)
            .group_separator(',')
            .build()
            .is_ok()
    );
    for digits in 0..=4 {
        assert!(
            base().min_grouping_digits(digits).build().is_ok(),
            "{digits}"
        );
    }
    for minus in ['-', '\u{2212}', '\u{2013}', '\u{2012}'] {
        assert!(base().minus_sign(minus).build().is_ok(), "{minus:?}");
    }
}

#[test]
fn the_narrow_separator_is_glue_in_nowrap_mode() {
    let t = Typographer::builder(Language::RU_RU, RuleSet::PROSE)
        .group_separator('\u{202f}')
        .build()
        .unwrap();
    let named = Output::Html(HtmlStyle::NAMED);
    assert_eq!(t.format_number(12345, named).to_string(), "12&#x202F;345");
    let class = Output::Html(HtmlStyle::NAMED.nowrap_class("nowrap"));
    assert_eq!(
        t.format_number(12345, class).to_string(),
        "<span class=\"nowrap\">12&#x202F;345</span>"
    );
    let characters = Output::Html(HtmlStyle::CHARACTERS.nowrap_class("nowrap"));
    assert_eq!(
        t.format_number(12345, characters).to_string(),
        "<span class=\"nowrap\">12\u{202f}345</span>"
    );
    // Four digits are the first to be grouped; three have no glue and no element.
    assert_eq!(t.format_number(123, class).to_string(), "123");
    assert_eq!(
        t.format_range(8000, 8100, class).to_string(),
        "<span class=\"nowrap\">8&#x202F;000&ndash;8&#x202F;100</span>"
    );
    // The same value in the other kinds.
    assert_eq!(
        t.format_number(12345, Output::Escaped(EscapeStyle::JS))
            .to_string(),
        "12\\u202F345"
    );
    assert_eq!(
        t.format_number(12345, Output::Escaped(EscapeStyle::RUST))
            .to_string(),
        "12\\u{202f}345"
    );
    assert_eq!(
        t.format_number(12345, Output::Escaped(EscapeStyle::CSS))
            .to_string(),
        "12\\202f 345"
    );
}

// Every output kind (design sections 11.2, 11.4, 11.6 and 11.7): the group separator U+00A0
// becomes `&nbsp;` in HTML, ` ` in JS, `\u{a0}` in Rust; the minus becomes `&minus;`,
// `−`, `\u{2212}`.
#[test]
fn every_output_kind_writes_a_number() {
    let n = -12345;
    let rows: &[(Output, &str)] = &[
        (UNICODE, "−12⍽345"),
        (Output::Html(HtmlStyle::NAMED), "&minus;12&nbsp;345"),
        (Output::Html(HtmlStyle::DECIMAL), "&#8722;12&#160;345"),
        (Output::Html(HtmlStyle::HEX), "&#x2212;12&#xA0;345"),
        (Output::Html(HtmlStyle::CHARACTERS), "−12⍽345"),
        (Output::Escaped(EscapeStyle::JS), "\\u221212\\u00A0345"),
        (Output::Escaped(EscapeStyle::JS_HEX), "\\u221212\\xA0345"),
        (
            Output::Escaped(EscapeStyle::JS_CODE_POINT),
            "\\u{2212}12\\u{A0}345",
        ),
        (Output::Escaped(EscapeStyle::JSON), "\\u221212\\u00a0345"),
        (Output::Escaped(EscapeStyle::RUST), "\\u{2212}12\\u{a0}345"),
        (Output::Escaped(EscapeStyle::CSS), "\\2212 12\\a0 345"),
        (
            Output::Escaped(EscapeStyle::JS.non_ascii()),
            "\\u221212\\u00A0345",
        ),
        (
            Output::Escaped(EscapeStyle::CSS.non_ascii()),
            "\\2212 12\\a0 345",
        ),
    ];
    for (output, expected) in rows {
        same(
            &format!("{output:?}"),
            RU.format_number(n, *output).to_string(),
            expected,
        );
    }
    // English: a comma and a point are not in the set and stay as they are in every kind.
    let rows: &[(Output, &str)] = &[
        (Output::Html(HtmlStyle::NAMED), "&minus;1,234.5"),
        (Output::Escaped(EscapeStyle::JS), "\\u22121,234.5"),
        (Output::Escaped(EscapeStyle::JSON), "\\u22121,234.5"),
        (Output::Escaped(EscapeStyle::RUST), "\\u{2212}1,234.5"),
        (Output::Escaped(EscapeStyle::CSS), "\\2212 1,234.5"),
    ];
    for (output, expected) in rows {
        same(
            &format!("en {output:?}"),
            EN.format_number(-1234.5, *output).to_string(),
            expected,
        );
    }
}

#[test]
fn every_output_kind_writes_a_range_and_the_symbols() {
    same(
        "named range",
        RU.format_range(8000, 8100, Output::Html(HtmlStyle::NAMED))
            .to_string(),
        "8&nbsp;000&ndash;8&nbsp;100",
    );
    same(
        "named negative range",
        EN.format_range(-5, -1, Output::Html(HtmlStyle::NAMED))
            .to_string(),
        "&minus;5&ndash;&minus;1",
    );
    same(
        "JS range",
        RU.format_range(8000, 8100, Output::Escaped(EscapeStyle::JS))
            .to_string(),
        "8\\u00A0000\\u20138\\u00A0100",
    );
    same(
        "Rust range",
        RU.format_range(8000, 8100, Output::Escaped(EscapeStyle::RUST))
            .to_string(),
        "8\\u{a0}000\\u{2013}8\\u{a0}100",
    );
    same(
        "CSS range",
        RU.format_range(8000, 8100, Output::Escaped(EscapeStyle::CSS))
            .to_string(),
        "8\\a0 000\\2013 8\\a0 100",
    );
    same(
        "named NaN",
        RU.format_number(f64::NAN, Output::Html(HtmlStyle::NAMED))
            .to_string(),
        "не&nbsp;число",
    );
    same(
        "JS NaN",
        RU.format_number(f64::NAN, Output::Escaped(EscapeStyle::JS))
            .to_string(),
        "не\\u00A0число",
    );
    same(
        "named -infinity",
        EN.format_number(f64::NEG_INFINITY, Output::Html(HtmlStyle::NAMED))
            .to_string(),
        "&minus;∞",
    );
    // `non_ascii()` escapes the infinity sign and the Cyrillic of the symbols too.
    same(
        "JS non_ascii infinity",
        EN.format_number(f64::INFINITY, Output::Escaped(EscapeStyle::JS.non_ascii()))
            .to_string(),
        "\\u221E",
    );
    same(
        "JS non_ascii NaN",
        RU.format_number(f64::NAN, Output::Escaped(EscapeStyle::JS.non_ascii()))
            .to_string(),
        "\\u043D\\u0435\\u00A0\\u0447\\u0438\\u0441\\u043B\\u043E",
    );
    same(
        "default scope keeps the infinity sign",
        EN.format_number(f64::INFINITY, Output::Escaped(EscapeStyle::JS))
            .to_string(),
        "∞",
    );
}

#[test]
fn nowrap_writes_one_element_around_a_number_with_glue() {
    let class = Output::Html(HtmlStyle::NAMED.nowrap_class("nowrap"));
    let style = Output::Html(HtmlStyle::NAMED.nowrap_style());
    // Design sections 2.7 and 11.5.
    same(
        "ru -12345",
        RU.format_number(-12345, class).to_string(),
        "<span class=\"nowrap\">&minus;12 345</span>",
    );
    same(
        "ru -12345 style",
        RU.format_number(-12345, style).to_string(),
        "<span style=\"white-space:nowrap\">&minus;12 345</span>",
    );
    // No glue, no element: `123` has no group separator.
    same("ru 123", RU.format_number(123, class).to_string(), "123");
    same(
        "ru -123",
        RU.format_number(-123, class).to_string(),
        "&minus;123",
    );
    same(
        "ru 123 style",
        RU.format_number(123, style).to_string(),
        "123",
    );
    same(
        "en 1234",
        EN.format_number(1234, class).to_string(),
        "1,234",
    );
    same(
        "en -1234.5",
        EN.format_number(-1234.5, class).to_string(),
        "&minus;1,234.5",
    );
    same(
        "ru 1234",
        RU.format_number(1234, class).to_string(),
        "<span class=\"nowrap\">1 234</span>",
    );
    same(
        "ru 1234.5",
        RU.format_number(1234.5, class).to_string(),
        "<span class=\"nowrap\">1 234,5</span>",
    );
    same(
        "ru range",
        RU.format_range(8000, 8100, class).to_string(),
        "<span class=\"nowrap\">8 000&ndash;8 100</span>",
    );
    // One element around the whole range, even when only one side has glue.
    same(
        "ru range, one side",
        RU.format_range(5, 8100, class).to_string(),
        "<span class=\"nowrap\">5&ndash;8 100</span>",
    );
    same(
        "ru short range",
        RU.format_range(1, 2, class).to_string(),
        "1&ndash;2",
    );
    same(
        "en range",
        EN.format_range(8000, 8100, class).to_string(),
        "8,000&ndash;8,100",
    );
    // The other entity styles keep their references outside the glue, and the ordinary space
    // inside the element.
    same(
        "hex",
        RU.format_number(-12345, Output::Html(HtmlStyle::HEX.nowrap_class("n")))
            .to_string(),
        "<span class=\"n\">&#x2212;12 345</span>",
    );
    same(
        "decimal",
        RU.format_number(-12345, Output::Html(HtmlStyle::DECIMAL.nowrap_style()))
            .to_string(),
        "<span style=\"white-space:nowrap\">&#8722;12 345</span>",
    );
    same(
        "characters",
        RU.format_number(
            -12345,
            Output::Html(HtmlStyle::CHARACTERS.nowrap_class("n")),
        )
        .to_string(),
        "<span class=\"n\">−12 345</span>",
    );
    // The class is escaped as an attribute value.
    same(
        "class escaping",
        RU.format_number(
            12345,
            Output::Html(HtmlStyle::NAMED.nowrap_class("a\"b<c&d'e")),
        )
        .to_string(),
        "<span class=\"a&quot;b&lt;c&amp;d&#39;e\">12 345</span>",
    );
}

fn samples() -> Vec<Number<'static>> {
    vec![
        Number::from(0),
        Number::from(-12345),
        Number::from(1234567),
        Number::from(i128::MIN),
        Number::from(u128::MAX),
        Number::from(1234567.5),
        Number::from(-0.001),
        Number::from(0.1f32),
        Number::from(f64::NAN),
        Number::from(f64::INFINITY),
        Number::from(f64::NEG_INFINITY),
        Number::from(-0.0),
        Number::decimal("-1234.50").unwrap(),
        Number::decimal("0012345").unwrap(),
        Number::fixed(1234.5678, 2),
    ]
}

#[test]
fn decoding_every_kind_gives_the_unicode_number() {
    // Design section 11.8: decode(out) equals the Unicode result, nowrap styles included (an
    // ordinary space inside an element is U+00A0).
    let narrow = Typographer::builder(Language::RU_RU, RuleSet::PROSE)
        .group_separator('\u{202f}')
        .build()
        .unwrap();
    for t in [&RU, &EN, &COMMON, &narrow] {
        for n in samples() {
            let unicode = t.format_number(n, UNICODE).to_string();
            for output in all_outputs() {
                let out = t.format_number(n, output).to_string();
                assert_eq!(
                    decode(output, &out).as_deref(),
                    Ok(unicode.as_str()),
                    "{n:?} {output:?}: {out}"
                );
            }
        }
        for (a, b) in [(1, 2), (8000, 8100), (-5, -1), (8000, 8000)] {
            let unicode = t.format_range(a, b, UNICODE).to_string();
            for output in all_outputs() {
                let out = t.format_range(a, b, output).to_string();
                assert_eq!(
                    decode(output, &out).as_deref(),
                    Ok(unicode.as_str()),
                    "{a}..{b} {output:?}: {out}"
                );
            }
        }
    }
}

#[test]
fn a_formatted_number_is_a_lazy_display() {
    use std::fmt::Write;
    let shown = RU.format_number(1234, UNICODE);
    // The display can be written twice: it holds no state.
    let mut buf = String::new();
    write!(buf, "{shown}|{shown}").unwrap();
    assert_eq!(buf, vis("1⍽234|1⍽234"));
    assert_eq!(format!("[{}]", RU.format_range(1, 2, UNICODE)), "[1–2]");
    // `to_string` gives the same as `write!`.
    assert_eq!(shown.to_string(), vis("1⍽234"));
}

#[test]
fn number_is_a_plain_value() {
    let a = Number::from(5);
    // `Number` is `Copy`: `a` is used after the copy.
    let b = a;
    assert_eq!(b, Number::from(5));
    assert_eq!(
        Number::decimal("1.5").unwrap(),
        Number::decimal("1.5").unwrap()
    );
    assert_ne!(Number::from(1.0), Number::from(2.0));
    // Floats compare as floats.
    assert_ne!(Number::from(f64::NAN), Number::from(f64::NAN));
    assert!(!format!("{a:?}").is_empty());
}

#[test]
fn a_number_with_any_valid_decimal_separator_is_stable_under_typeset() {
    // Design section 7.6: the decimal separator keeps the number's meaning, so typeset leaves a
    // formatted number and range as they are. A quote character would not do: the quote rules
    // made the second separator of `1'5–2'5` a prime.
    for decimal in [',', '.', '\u{66b}', '\u{b7}', '\u{2396}', '!'] {
        let language = LanguageBuilder::from_language(&Language::RU_RU, "xx-XX")
            .numbers(NumberData::new().with_decimal(decimal).with_group('\u{a0}'))
            .build()
            .unwrap();
        let t = Typographer::new(language, RuleSet::PROSE);
        for text in [
            t.format_number(1234.5f64, Output::Unicode).to_string(),
            t.format_range(1.5f64, 2.5f64, Output::Unicode).to_string(),
        ] {
            assert_eq!(t.typeset(&text, Output::Unicode), text, "{decimal:?}");
        }
    }
    for decimal in ['\'', '\u{ff0d}', '\u{206a}'] {
        assert!(
            LanguageBuilder::from_language(&Language::RU_RU, "xx-XX")
                .numbers(NumberData::new().with_decimal(decimal))
                .build()
                .is_err(),
            "{decimal:?}"
        );
    }
}
