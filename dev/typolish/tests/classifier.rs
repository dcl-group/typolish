//! The fragment classifier of design section 3.6 as a black box: tables of inputs and the
//! protected spans that `report().protected()` lists for them (design section 12.5), and the
//! `MixedScript` diagnostic of the classifier (design sections 5.2, 5.7).
//!
//! A frozen fragment is listed with the kind of its check (`Technical(TechKind)`); an unfrozen one
//! (`Dictionary`, `Abbreviation`, `Number`, `Range`, `Dimensions`, `Joined`, `Word`, `Sign`,
//! `Times`, `OperatorRun`, `YearMonth`, `Punctuation`) is not listed. So a row says: these and no
//! other spans are frozen, in this order, and the text of each span is this. The `FragKind` of an
//! unfrozen fragment and the `LABEL` flag are not visible through the public API; they are
//! checked by the unit tests of `src/engine/classify.rs` and, through the rules that read them, by
//! the tables of stage 3 (design sections 13.2 and 15.4).
//!
//! Protection does not depend on the rule set (design section 10.2), so every row runs under the
//! empty set, Interface, Prose and `ALL`, and for the languages the row names.

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
#[path = "common/probe.rs"]
mod probe;

use common::show;
use probe::{SETS, all_languages, diagnostics_of, span_texts, tech};
use typolish::{DiagnosticKind, Language, ProtectedKind, RuleSet, Severity, TechKind, Typographer};

type Spans = &'static [(ProtectedKind, &'static str)];

const URL: ProtectedKind = tech(TechKind::Url);
const EMAIL: ProtectedKind = tech(TechKind::Email);
const FLAG: ProtectedKind = tech(TechKind::Flag);
const PATH: ProtectedKind = tech(TechKind::Path);
const TIME: ProtectedKind = tech(TechKind::TimeRange);
const COLON: ProtectedKind = tech(TechKind::Colon);
const OPERATOR: ProtectedKind = tech(TechKind::Operator);
const IDENT: ProtectedKind = tech(TechKind::Identifier);
const ANGLE: ProtectedKind = tech(TechKind::Angle);
const UUID: ProtectedKind = tech(TechKind::Uuid);
const HEX: ProtectedKind = tech(TechKind::Hex);
const ISODATE: ProtectedKind = tech(TechKind::IsoDate);
const VERSION: ProtectedKind = tech(TechKind::Version);
const DOTTED: ProtectedKind = tech(TechKind::Dotted);
const LIST: ProtectedKind = tech(TechKind::List);
const HYPHEN: ProtectedKind = tech(TechKind::CompoundHyphens);
const UNCL: ProtectedKind = tech(TechKind::Unclassified);

/// Which built-in languages a row holds for.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Langs {
    /// Russian, English and `COMMON`.
    Any,
    /// Russian only.
    Ru,
    /// English only.
    En,
    /// `COMMON` only.
    Common,
}

struct Row {
    langs: Langs,
    input: &'static str,
    spans: Spans,
}

const fn any(input: &'static str, spans: Spans) -> Row {
    Row {
        langs: Langs::Any,
        input,
        spans,
    }
}

const fn ru(input: &'static str, spans: Spans) -> Row {
    Row {
        langs: Langs::Ru,
        input,
        spans,
    }
}

const fn en(input: &'static str, spans: Spans) -> Row {
    Row {
        langs: Langs::En,
        input,
        spans,
    }
}

const fn common(input: &'static str, spans: Spans) -> Row {
    Row {
        langs: Langs::Common,
        input,
        spans,
    }
}

fn languages_of(langs: Langs) -> Vec<Language> {
    match langs {
        Langs::Any => all_languages(),
        Langs::Ru => vec![Language::RU_RU],
        Langs::En => vec![Language::EN_US],
        Langs::Common => vec![Language::COMMON],
    }
}

#[track_caller]
fn run_rows(group: &str, rows: &[Row]) {
    assert!(!rows.is_empty(), "{group}");
    for row in rows {
        for language in languages_of(row.langs) {
            for set in SETS {
                let t = Typographer::new(language.clone(), set);
                let got = span_texts(&t, row.input);
                assert_eq!(
                    got,
                    row.spans,
                    "{group}: {language} {set}: {}",
                    show(row.input)
                );
            }
        }
    }
}

// Check 0: the dictionaries come before everything (design section 3.6, "Order matters").
const DICTIONARY: &[Row] = &[
    // Units with `/` would be paths; the dictionary of the language of the call decides.
    ru("км/ч", &[]),
    ru("МБ/с", &[]),
    ru("м/с", &[]),
    ru("об/мин", &[]),
    en("km/h", &[]),
    en("MB/s", &[]),
    en("m/s", &[]),
    ru("MB/s", &[(PATH, "MB/s")]),
    ru("km/h", &[(PATH, "km/h")]),
    en("км/ч", &[(PATH, "км/ч")]),
    en("МБ/с", &[(PATH, "МБ/с")]),
    common("км/ч", &[(PATH, "км/ч")]),
    common("km/h", &[(PATH, "km/h")]),
    // Units are compared with case.
    en("mb/s", &[(PATH, "mb/s")]),
    en("Km/h", &[(PATH, "Km/h")]),
    // Units that are not letters and digits.
    ru("°C", &[]),
    ru("\u{b0}\u{421}", &[]),
    en("°C", &[]),
    en("°F", &[]),
    common("°C", &[]),
    ru("°", &[]),
    en("°", &[]),
    ru("$", &[]),
    ru("€", &[]),
    ru("₽", &[]),
    ru("м²", &[]),
    ru("км2", &[]),
    en("₽", &[(UNCL, "₽")]),
    common("°", &[(UNCL, "°")]),
    // The Greek mu and the micro sign of the English units (design section 5.2).
    en("\u{3bc}s", &[]),
    en("\u{b5}s", &[]),
    // Atom exceptions and abbreviations: an entry that ends in `.` matches a core with a stop.
    en("e.g.", &[]),
    en("i.e.", &[]),
    en("etc.", &[]),
    en("vs.", &[]),
    en("cf.", &[]),
    en("a.m.", &[]),
    en("p.m.", &[]),
    en("e.g...", &[]),
    en("e.g…", &[]),
    en("etc...", &[]),
    en("etc…", &[]),
    en("e.g", &[(DOTTED, "e.g")]),
    en("i.e", &[(DOTTED, "i.e")]),
    ru("т.е.", &[]),
    ru("т.е...", &[]),
    // `Format` tokens between the core and its stop are skipped (design section 3.6).
    ru("т.е\u{200b}...", &[]),
    ru("т.е\u{ad}...", &[]),
    ru("т.е\u{2068}\u{2069}...", &[]),
    ru("т.е\u{ad}.", &[]),
    ru("т.е\u{AD}. да", &[]),
    ru("т.е\u{2069}. да", &[]),
    en("e.g\u{ad}...", &[]),
    en("e.g\u{AD}. foo", &[]),
    // A `Format` token after the core with no stop after it is no stop: the core stays protected.
    ru("т.е\u{200b}", &[(DOTTED, "т.е")]),
    ru("т.е\u{ad}", &[(DOTTED, "т.е")]),
    ru("т.е\u{200b} да", &[(DOTTED, "т.е")]),
    ru("т.е…", &[]),
    ru("т.д.", &[]),
    ru("т.п.", &[]),
    ru("т.к.", &[]),
    ru("т.н.", &[]),
    ru("т.ч.", &[]),
    ru("в т.ч.", &[]),
    ru("и т.д.", &[]),
    ru("и т.д...", &[]),
    ru("и т.п.", &[]),
    ru("до н.э.", &[]),
    ru("т.е", &[(DOTTED, "т.е")]),
    ru("т.д", &[(DOTTED, "т.д")]),
    // An entry without a `.` still matches a core that has a stop: the stop only adds the entries
    // that end in `.`.
    ru("Скорость 60 км/ч.", &[]),
    ru("Скорость 60 км/ч...", &[]),
    ru("Скорость 60 МБ/с.", &[]),
    ru("Скорость 60 МБ/с…", &[]),
    ru("Пришёл г-н.", &[]),
    en("Speed 60 km/h.", &[]),
    en("Speed 60 MB/s…", &[]),
    en("Latency 5 \u{b5}s.", &[]),
    en("km/h.", &[]),
    common("км/ч.", &[(PATH, "км/ч")]),
];

#[test]
fn check_0_dictionaries() {
    run_rows("check 0", DICTIONARY);
}

#[test]
fn check_0_user_units_come_first() {
    let plain = Typographer::new(Language::EN_US, RuleSet::INTERFACE);
    assert_eq!(
        span_texts(&plain, "req/s"),
        [(PATH, "req/s")],
        "not a unit of en"
    );
    let added = Typographer::builder(Language::EN_US, RuleSet::INTERFACE)
        .add_units(["req/s"])
        .build()
        .unwrap();
    assert_eq!(span_texts(&added, "req/s"), []);
    assert_eq!(span_texts(&added, "Run 20 req/s now."), []);
    let removed = Typographer::builder(Language::RU_RU, RuleSet::INTERFACE)
        .remove_units(["МБ/с"])
        .build()
        .unwrap();
    assert_eq!(span_texts(&removed, "МБ/с"), [(PATH, "МБ/с")]);
    let removed_deg = Typographer::builder(Language::EN_US, RuleSet::INTERFACE)
        .remove_units(["°C"])
        .build()
        .unwrap();
    assert_eq!(span_texts(&removed_deg, "°C"), [(UNCL, "°C")]);
    // An added unit matches before a full stop too.
    assert_eq!(span_texts(&added, "Run 20 req/s."), []);
}

#[test]
fn check_0_has_no_length_limit() {
    // Design section 3.6 gives check 0 no bound: a long unit added by the caller is a unit.
    let long = "килобайтвсекундунаядро/час";
    assert!(long.len() > 48);
    let plain = Typographer::new(Language::RU_RU, RuleSet::INTERFACE);
    assert_eq!(
        span_texts(&plain, "5 килобайтвсекундунаядро/час"),
        [(PATH, long)]
    );
    let added = Typographer::builder(Language::RU_RU, RuleSet::INTERFACE)
        .add_units([long])
        .build()
        .unwrap();
    for text in [
        "5 килобайтвсекундунаядро/час",
        "Скорость 5 килобайтвсекундунаядро/час.",
    ] {
        assert_eq!(span_texts(&added, text), [], "{text}");
    }
}

const URLS_AND_ADDRESSES: &[Row] = &[
    // Check 1.
    any(
        "https://example.com/a?b=c",
        &[(URL, "https://example.com/a?b=c")],
    ),
    any("http://example.com", &[(URL, "http://example.com")]),
    any("ftp://h", &[(URL, "ftp://h")]),
    any("file:///etc/hosts", &[(URL, "file:///etc/hosts")]),
    any("www.example.com", &[(URL, "www.example.com")]),
    any("www.example.com/x", &[(URL, "www.example.com/x")]),
    any("https://example.com/a.", &[(URL, "https://example.com/a")]),
    any("(https://example.com)", &[(URL, "https://example.com")]),
    any("\"https://example.com\",", &[(URL, "https://example.com")]),
    any("[https://example.com]", &[(URL, "[https://example.com]")]),
    // Check 2.
    any("user@example.com", &[(EMAIL, "user@example.com")]),
    any(
        "git@example.com:org/repo.git",
        &[(EMAIL, "git@example.com:org/repo.git")],
    ),
    any(
        "mailto:user@example.com",
        &[(EMAIL, "mailto:user@example.com")],
    ),
    any("a@b", &[(EMAIL, "a@b")]),
    any("@user", &[(EMAIL, "@user")]),
    any("@here", &[(EMAIL, "@here")]),
    any("@1", &[(EMAIL, "@1")]),
    any("user@", &[(UNCL, "user@")]),
    any("@", &[(UNCL, "@")]),
    // The check before a URL's host: `://` wins over `@`.
    any(
        "https://user@example.com/",
        &[(URL, "https://user@example.com/")],
    ),
];

#[test]
fn checks_1_and_2_urls_and_addresses() {
    run_rows("checks 1, 2", URLS_AND_ADDRESSES);
}

const FLAGS_AND_PATHS: &[Row] = &[
    // Check 3.
    any("-v", &[(FLAG, "-v")]),
    any("-x", &[(FLAG, "-x")]),
    any("-la", &[(FLAG, "-la")]),
    any("--force", &[(FLAG, "--force")]),
    any("--long-opt=value", &[(FLAG, "--long-opt=value")]),
    any("--no-preserve-root", &[(FLAG, "--no-preserve-root")]),
    any("--1", &[(FLAG, "--1")]),
    any("ls -la", &[(FLAG, "-la")]),
    any(
        "rm -rf --no-preserve-root /",
        &[(FLAG, "-rf"), (FLAG, "--no-preserve-root"), (PATH, "/")],
    ),
    any(
        "git commit -m msg --amend",
        &[(FLAG, "-m"), (FLAG, "--amend")],
    ),
    any("cargo build --release", &[(FLAG, "--release")]),
    any("Use - --force", &[(FLAG, "--force")]),
    any("diff -u - file.txt", &[(FLAG, "-u"), (DOTTED, "file.txt")]),
    any("tar -cf - -C dir", &[(FLAG, "-cf"), (FLAG, "-C")]),
    any("npm test -- --watch", &[(FLAG, "--watch")]),
    // A dash alone, a run of dashes and a signed number are not flags.
    any("-", &[]),
    any("--", &[]),
    any("---", &[]),
    any("-5", &[]),
    any("-1", &[]),
    // Check 4.
    any("/etc/hosts", &[(PATH, "/etc/hosts")]),
    any("C:\\Users\\x", &[(PATH, "C:\\Users\\x")]),
    any("~/.config/app", &[(PATH, "~/.config/app")]),
    any("a/b", &[(PATH, "a/b")]),
    any("a\\b", &[(PATH, "a\\b")]),
    any("/", &[(PATH, "/")]),
    any("1/2", &[(PATH, "1/2")]),
    any("1/2/2026", &[(PATH, "1/2/2026")]),
    any("10.0.0.0/8", &[(PATH, "10.0.0.0/8")]),
    any("a+b/c==", &[(PATH, "a+b/c==")]),
    any(
        "SHA256:47DEQpj8HBSa+/TImW+5JCeuQeRkm5NMpJWZG3hSuFU",
        &[(PATH, "SHA256:47DEQpj8HBSa+/TImW+5JCeuQeRkm5NMpJWZG3hSuFU")],
    ),
    ru("и/или", &[(PATH, "и/или")]),
    // The run of dots at the start is a wrapper: the core is `/x`.
    any("../x", &[(PATH, "/x")]),
];

#[test]
fn checks_3_and_4_flags_and_paths() {
    run_rows("checks 3, 4", FLAGS_AND_PATHS);
}

const TIMES_AND_COLONS: &[Row] = &[
    // Check 5: only the times are frozen, the separator stays editable.
    any("10:00-12:00", &[(TIME, "10:00"), (TIME, "12:00")]),
    any("10:00–12:00", &[(TIME, "10:00"), (TIME, "12:00")]),
    any(
        "09:30:15-10:45:59",
        &[(TIME, "09:30:15"), (TIME, "10:45:59")],
    ),
    any("23:59-00:00", &[(TIME, "23:59"), (TIME, "00:00")]),
    any(
        "Приём 10:00-12:00 ежедневно",
        &[(TIME, "10:00"), (TIME, "12:00")],
    ),
    // Hours 0 to 23, minutes 00 to 59: otherwise a colon fragment.
    any("24:00-25:00", &[(COLON, "24:00-25:00")]),
    any("10:60-11:00", &[(COLON, "10:60-11:00")]),
    // With spaces the times are two fragments.
    any("10:00 - 12:00", &[(COLON, "10:00"), (COLON, "12:00")]),
    // Check 6.
    any("host:8080", &[(COLON, "host:8080")]),
    any("a:b", &[(COLON, "a:b")]),
    any("::1", &[(COLON, "::1")]),
    any("fe80::1%eth0", &[(COLON, "fe80::1%eth0")]),
    any("[::1]:8080", &[(COLON, "[::1]:8080")]),
    any(
        "43:51:43:a1:b5:fc:8b:b7",
        &[(COLON, "43:51:43:a1:b5:fc:8b:b7")],
    ),
    any("12:30:45", &[(COLON, "12:30:45")]),
    any("10:00", &[(COLON, "10:00")]),
    any("192.168.1.1:443", &[(COLON, "192.168.1.1:443")]),
    any("2026-11-21T10:20:30Z", &[(COLON, "2026-11-21T10:20:30Z")]),
    any("key:value", &[(COLON, "key:value")]),
    // A colon at the start or at the end is punctuation.
    any("Value:", &[]),
    any("Значение: x", &[]),
    any(":", &[]),
    any("a:", &[]),
    any("host:8080,", &[(COLON, "host:8080")]),
    any("(host:8080)", &[(COLON, "host:8080")]),
    any("host:8080.", &[(COLON, "host:8080")]),
];

#[test]
fn checks_5_and_6_times_and_colons() {
    run_rows("checks 5, 6", TIMES_AND_COLONS);
}

const OPERATORS: &[Row] = &[
    // Check 7: only operator characters, never frozen (the symbol and dash rules decide).
    any("->", &[]),
    any("=>", &[]),
    any(">=", &[]),
    any("<=", &[]),
    any("<->", &[]),
    any(">>", &[]),
    any("--", &[]),
    any("-->", &[]),
    any("<-->", &[]),
    any(">>=", &[]),
    any("==", &[]),
    any("!=", &[]),
    any("+-", &[]),
    any("||", &[]),
    any("&&", &[]),
    any("**", &[]),
    any("=", &[]),
    any("+", &[]),
    any("<", &[]),
    any(">", &[]),
    any("*", &[]),
    any("|", &[]),
    any("%", &[]),
    any("x -> y", &[]),
    any("x == y", &[]),
    any("a <-> b", &[]),
    any("x -> y; z => w", &[]),
    // Check 8: an operator character inside, or a bracket of an angle form at an end.
    any("a>>b", &[(OPERATOR, "a>>b")]),
    any("a=b", &[(OPERATOR, "a=b")]),
    any("a=>b", &[(OPERATOR, "a=>b")]),
    any("a->b", &[(OPERATOR, "a->b")]),
    any("a|b", &[(OPERATOR, "a|b")]),
    any("a&b", &[(OPERATOR, "a&b")]),
    any("a^b", &[(OPERATOR, "a^b")]),
    any("a~b", &[(OPERATOR, "a~b")]),
    any("a*b", &[(OPERATOR, "a*b")]),
    any("a#b", &[(OPERATOR, "a#b")]),
    any("a$b", &[(OPERATOR, "a$b")]),
    any("a%b", &[(OPERATOR, "a%b")]),
    any("a+b", &[(OPERATOR, "a+b")]),
    any("a<b", &[(OPERATOR, "a<b")]),
    any("a>b", &[(OPERATOR, "a>b")]),
    any("2+2", &[(OPERATOR, "2+2")]),
    any("x==y", &[(OPERATOR, "x==y")]),
    any("AT&T", &[(OPERATOR, "AT&T")]),
    any("50%-ный", &[(OPERATOR, "50%-ный")]),
    any("vec<int>", &[(OPERATOR, "vec<int>")]),
    any("map<k,v>", &[(OPERATOR, "map<k,v>")]),
    any("1.2.3+build", &[(OPERATOR, "1.2.3+build")]),
    any(
        "1.2.3-beta.2+build.5",
        &[(OPERATOR, "1.2.3-beta.2+build.5")],
    ),
    any("key=\"value\"", &[(OPERATOR, "key=\"value")]),
    any("x=1;", &[(OPERATOR, "x=1")]),
    any("<word>", &[(ANGLE, "<word>")]),
    any("<b>", &[(ANGLE, "<b>")]),
    any("<T>", &[(ANGLE, "<T>")]),
    any("a>", &[(ANGLE, "a>")]),
    any("<a", &[(ANGLE, "<a")]),
    any("a <b> c", &[(ANGLE, "<b>")]),
    any("if a<b and c>d", &[(OPERATOR, "a<b"), (OPERATOR, "c>d")]),
    any("kv_lang", &[(IDENT, "kv_lang")]),
    any("my_var-name", &[(IDENT, "my_var-name")]),
    any("CamelCase-Mixed_id", &[(IDENT, "CamelCase-Mixed_id")]),
    any("x86_64", &[(IDENT, "x86_64")]),
    // The operator character at an end is not "inside": a number, a word or the last check.
    any("50%", &[]),
    any("C#", &[(UNCL, "C#")]),
    any("$HOME", &[(UNCL, "$HOME")]),
    any("#tag", &[(UNCL, "#tag")]),
];

#[test]
fn checks_7_and_8_operators() {
    run_rows("checks 7, 8", OPERATORS);
}

const HEX_DATES_VERSIONS: &[Row] = &[
    // Check 9.
    any(
        "550e8400-e29b-41d4-a716-446655440000",
        &[(UUID, "550e8400-e29b-41d4-a716-446655440000")],
    ),
    any(
        "550E8400-E29B-41D4-A716-446655440000",
        &[(UUID, "550E8400-E29B-41D4-A716-446655440000")],
    ),
    // One digit short: letters or digits joined by hyphens.
    any(
        "550e8400-e29b-41d4-a716-44665544000",
        &[(HYPHEN, "-"), (HYPHEN, "-"), (HYPHEN, "-"), (HYPHEN, "-")],
    ),
    // Check 10 before 16 and 20.
    any("0x1F", &[(HEX, "0x1F")]),
    any("0XDEAD", &[(HEX, "0XDEAD")]),
    any("0x10", &[(HEX, "0x10")]),
    any("0xff", &[(HEX, "0xff")]),
    any("0xZZ", &[(IDENT, "0xZZ")]),
    // Check 11: at least 8 characters, all hex, with a digit and a letter a to f.
    any("deadbeef0123", &[(HEX, "deadbeef0123")]),
    any("abcdef01", &[(HEX, "abcdef01")]),
    any("0123456789abcdef", &[(HEX, "0123456789abcdef")]),
    any(
        "d41d8cd98f00b204e9800998ecf8427e",
        &[(HEX, "d41d8cd98f00b204e9800998ecf8427e")],
    ),
    any("deadbeef", &[]),
    any("12345678", &[]),
    any("1234abc", &[(IDENT, "1234abc")]),
    any("1234567g", &[(IDENT, "1234567g")]),
    // Check 12.
    any("2026-11-21", &[(ISODATE, "2026-11-21")]),
    any("2026-11", &[]),
    any("2026–11", &[]),
    any("2026-13", &[]),
    any("1941-45", &[]),
    // Check 13 before 18.
    any("0.2.27", &[(VERSION, "0.2.27")]),
    any("1.2.3", &[(VERSION, "1.2.3")]),
    any("v1.2.3", &[(VERSION, "v1.2.3")]),
    any("V1.2.3", &[(VERSION, "V1.2.3")]),
    any("v1.2.3-rc.1", &[(VERSION, "v1.2.3-rc.1")]),
    any("1.2.3-rc1", &[(VERSION, "1.2.3-rc1")]),
    any("10.0.0.1", &[(VERSION, "10.0.0.1")]),
    any("255.255.255.255", &[(VERSION, "255.255.255.255")]),
    any("21.11.2026", &[(VERSION, "21.11.2026")]),
    any("21.11.26", &[(VERSION, "21.11.26")]),
    any("2026.11.21", &[(VERSION, "2026.11.21")]),
    any("1.2", &[]),
    any("v1.2", &[(DOTTED, "v1.2")]),
];

#[test]
fn checks_9_to_13_hex_dates_versions() {
    run_rows("checks 9 to 13", HEX_DATES_VERSIONS);
}

const NUMBERS_AND_SHAPES: &[Row] = &[
    // Check 14: groups of one or two letters with a stop; without the stop a dotted name.
    any("Ph.D.", &[]),
    any("А.С.", &[]),
    any("U.S.A.", &[]),
    any("т.е.", &[]),
    any("U.S.A", &[(DOTTED, "U.S.A")]),
    any("abc.de.", &[(DOTTED, "abc.de")]),
    any("example.com.", &[(DOTTED, "example.com")]),
    // Check 15: a number with a sign, a decimal part and a percent sign.
    any("5", &[]),
    any("-5", &[]),
    any("+5", &[]),
    any("+-5", &[]),
    any("\u{2212}5", &[]),
    any("±5", &[]),
    any("5%", &[]),
    any("5‰", &[]),
    any("100‰", &[]),
    any("1,5", &[]),
    any("1.5", &[]),
    any("3.14", &[]),
    any("-1,5%", &[]),
    any("1,000", &[]),
    any("007", &[]),
    any("12.", &[]),
    // Check 16: dimensions, the first factor without a leading zero.
    any("1920x1080", &[]),
    any("1920X1080", &[]),
    any("1920×1080", &[]),
    any("1920х1080", &[]),
    any("1920Х1080", &[]),
    any("010x5", &[(IDENT, "010x5")]),
    any("1920x1080x32", &[(IDENT, "1920x1080x32")]),
    any("5x", &[(IDENT, "5x")]),
    // The lone letter between number fragments is `Times`, never frozen, in every form.
    any("10 x 5", &[]),
    any("10 x\u{200d} 5", &[]),
    any("10 X 5", &[]),
    any("10 х 5", &[]),
    any("10 Х 5", &[]),
    any("10 × 5", &[]),
    any("10 x 5 x 3", &[]),
    // A lone `×` elsewhere falls through to check 24.
    any("a × b", &[(UNCL, "×")]),
    any("×", &[(UNCL, "×")]),
    any("10 ×", &[(UNCL, "×")]),
    // Check 17: ranges, never frozen.
    any("5-10", &[]),
    any("5–10", &[]),
    any("8000-8100", &[]),
    any("10-10", &[]),
    any("2010-13", &[]),
    any("IV-X", &[]),
    any("XIX-XX", &[]),
    any("XX-XIX", &[]),
    any("MCMXCIV-MMXXVI", &[]),
    any("V-5", &[]),
    ru("1,5-2,5", &[]),
    en("1.5-2.5", &[]),
    common("1.5-2.5", &[]),
    // The separator of the language decides, so the other form is a dotted name or a list.
    ru("1.5-2.5", &[(DOTTED, "1.5-2.5")]),
    en("1,5-2,5", &[(LIST, "1,5-2,5")]),
    // Not Roman numerals: compound words with frozen hyphens (design section 3.6, "Roman
    // numerals").
    any("IIII-V", &[(HYPHEN, "-")]),
    any("IC-V", &[(HYPHEN, "-")]),
    any("VV-X", &[(HYPHEN, "-")]),
    any("XM-V", &[(HYPHEN, "-")]),
    any("xix-xx", &[(HYPHEN, "-")]),
    // Cyrillic `Х` in the first and the last numeral.
    any("\u{425}IX-XX", &[(HYPHEN, "-")]),
    any("\u{425}I\u{425}-\u{425}\u{425}", &[(HYPHEN, "-")]),
    any("5-10-15", &[(HYPHEN, "-"), (HYPHEN, "-")]),
];

#[test]
fn checks_14_to_17_numbers_and_ranges() {
    run_rows("checks 14 to 17", NUMBERS_AND_SHAPES);
}

const NAMES_LISTS_WORDS: &[Row] = &[
    // Check 18: a single dot between letters or digits.
    any("db.example.internal", &[(DOTTED, "db.example.internal")]),
    any("db-01.example.com", &[(DOTTED, "db-01.example.com")]),
    any("example.com", &[(DOTTED, "example.com")]),
    any("file.txt", &[(DOTTED, "file.txt")]),
    any("README.md", &[(DOTTED, "README.md")]),
    any("a.b", &[(DOTTED, "a.b")]),
    any("nats-server-2.10", &[(DOTTED, "nats-server-2.10")]),
    any("localhost", &[]),
    // Check 19.
    any("a,b,c", &[(LIST, "a,b,c")]),
    any("x,y;z", &[(LIST, "x,y;z")]),
    any("1,2,3", &[(LIST, "1,2,3")]),
    any("red,green,blue", &[(LIST, "red,green,blue")]),
    any("a;b", &[(LIST, "a;b")]),
    any("a,b", &[(LIST, "a,b")]),
    any("a, b", &[]),
    any("a,", &[]),
    // Check 20.
    any("x86", &[(IDENT, "x86")]),
    any("h264", &[(IDENT, "h264")]),
    any("2FA", &[(IDENT, "2FA")]),
    any("5MB", &[(IDENT, "5MB")]),
    any("abc123", &[(IDENT, "abc123")]),
    any("2x", &[(IDENT, "2x")]),
    any("v2", &[(IDENT, "v2")]),
    any("90s", &[(IDENT, "90s")]),
    // A wrapper quote is cut off; the core `90s` has no quote.
    any("'90s", &[(IDENT, "90s")]),
    // Cyrillic letters make it a word (check 23), not an identifier.
    any("5МБ", &[]),
    any("5 МБ", &[]),
    // Check 21: compound words; only the hyphens are frozen.
    any("кто-то", &[(HYPHEN, "-")]),
    any("tls-server", &[(HYPHEN, "-")]),
    any("agent-ca", &[(HYPHEN, "-")]),
    any("pre-commit", &[(HYPHEN, "-")]),
    any("db-01", &[(HYPHEN, "-")]),
    any("25-й", &[(HYPHEN, "-")]),
    any("123-45-67", &[(HYPHEN, "-"), (HYPHEN, "-")]),
    any("mother-in-law", &[(HYPHEN, "-"), (HYPHEN, "-")]),
    any("a-b-c", &[(HYPHEN, "-"), (HYPHEN, "-")]),
    any("5-x", &[(HYPHEN, "-")]),
    any("IT-отдел", &[(HYPHEN, "-")]),
    // Check 22: parts joined by one dash or ellipsis, never frozen.
    any("word—word", &[]),
    any("слово–слово", &[]),
    any("да…нет", &[]),
    any("да...нет", &[]),
    any("-5...5", &[]),
    any("5—10", &[]),
    // Check 23: words with a quote or a sign, never frozen.
    any("don't", &[]),
    any("don’t", &[]),
    any("90's", &[]),
    any("5'10", &[]),
    any("5′10", &[]),
    any("5'10\"", &[]),
    any("rock'n'roll", &[]),
    any("№5", &[]),
    any("§12", &[]),
    any("¶IV", &[]),
    any("Brand(tm)", &[]),
    any("f(c)", &[]),
    any("S(r)", &[]),
    // Swiss grouping and other quote shapes that no word form allows.
    any("1'000'000", &[(UNCL, "1'000'000")]),
    any("5'100", &[(UNCL, "5'100")]),
    any("1234'10", &[(UNCL, "1234'10")]),
    // Marks stay in their cluster; a mark does not make a word unclassified.
    any("за\u{301}мок", &[]),
    any("\u{438}\u{306}", &[]),
];

#[test]
fn checks_18_to_23_names_lists_words() {
    run_rows("checks 18 to 23", NAMES_LISTS_WORDS);
}

const SPECIAL_CASES: &[Row] = &[
    // A sign of the language, single or doubled.
    any("№", &[]),
    any("§", &[]),
    any("¶", &[]),
    any("§§", &[]),
    any("№ 5", &[]),
    any("Документ № 5", &[]),
    any("§§ 5-7", &[]),
    any("¶ IV", &[]),
    any("№\u{a0}5", &[]),
    // A legal mark alone, or with a number after it.
    any("(c)", &[]),
    any("(C)", &[]),
    any("(\u{441})", &[]),
    any("(r)", &[]),
    any("(R)", &[]),
    any("(tm)", &[]),
    any("(TM)", &[]),
    any("©", &[]),
    any("®", &[]),
    any("™", &[]),
    any("(c)2026", &[]),
    any("(c) 2026", &[]),
    any("© 2026", &[]),
    any("2026 (c)", &[]),
    // A legal mark after a number or a range falls through to check 24.
    any("2026(c)", &[(UNCL, "2026(c)")]),
    any("1001(c)", &[(UNCL, "1001(c)")]),
    any("2020-2026(c)", &[(UNCL, "2020-2026(c)")]),
    any("(c)2020-2026", &[(UNCL, "(c)2020-2026")]),
    // Check 24: the white list ends with "frozen".
    any("SGVsbG8gd29ybGQ=", &[(UNCL, "SGVsbG8gd29ybGQ=")]),
    any("%d", &[(UNCL, "%d")]),
    any("{x}", &[(UNCL, "{x}")]),
    any("[x]", &[(UNCL, "[x]")]),
    any("🙂", &[(UNCL, "🙂")]),
    // A wrapper is cut off, the brackets of a function call are not.
    any("(x)", &[]),
    any("f(x)", &[(UNCL, "f(x")]),
];

#[test]
fn special_cases_and_the_last_check() {
    run_rows("special cases", SPECIAL_CASES);
}

const WRAPPER: &[Row] = &[
    // The wrapper is cut off by tokens; it stays editable, so it is outside the span.
    any("\"host:8080\"", &[(COLON, "host:8080")]),
    any("«host:8080»", &[(COLON, "host:8080")]),
    any("„host:8080“", &[(COLON, "host:8080")]),
    any("'a:b'", &[(COLON, "a:b")]),
    any("(a:b).", &[(COLON, "a:b")]),
    any("...host:8080", &[(COLON, "host:8080")]),
    any("host:8080...", &[(COLON, "host:8080")]),
    any("host:8080…", &[(COLON, "host:8080")]),
    any("host:8080?!", &[(COLON, "host:8080")]),
    any("Адрес \"::1\"", &[(COLON, "::1")]),
    any("Папка \"/etc/\" пуста", &[(PATH, "/etc/")]),
    any("Run \"x->\" now", &[(ANGLE, "x->")]),
    // Brackets are not wrapper.
    any("[a:b]", &[(COLON, "[a:b]")]),
    any("<a:b>", &[(COLON, "<a:b>")]),
    // Nothing is left of the core: a punctuation fragment, nothing frozen, nothing listed.
    any("...", &[]),
    any("…", &[]),
    any("\"", &[]),
    any("«»", &[]),
    any("()", &[]),
    any("?!", &[]),
    any("?..", &[]),
    any("!", &[]),
    any(",", &[]),
    any("\"?\"", &[]),
    any("\"\u{2068}\u{2069}\"", &[]),
    any("\"\"", &[]),
    any("\"\"\"", &[]),
    any("\".\"", &[]),
    any("(\"", &[]),
    any("\")", &[]),
    // A dash fragment keeps its core in its wrapper too.
    any("\"-\"", &[]),
    any("(--)", &[]),
];

#[test]
fn wrapper_and_punctuation_fragments() {
    run_rows("wrapper", WRAPPER);
}

const DIAGNOSTIC_FREE_SENTENCES: &[Row] = &[
    // Frozen cores inside sentences: the words around them are not frozen.
    any(
        "Сервис слушает host:8080 - проверьте",
        &[(COLON, "host:8080")],
    ),
    any(
        "See https://example.com/docs - the guide",
        &[(URL, "https://example.com/docs")],
    ),
    any(
        "Запустите ls -la, затем cat /etc/hosts.",
        &[(FLAG, "-la"), (PATH, "/etc/hosts")],
    ),
    any(
        "Токен \"SGVsbG8gd29ybGQ=\" и \"имя\".",
        &[(UNCL, "SGVsbG8gd29ybGQ=")],
    ),
    any(
        "Версия 0.2.27 от 2026-11-21 на 10.0.0.1",
        &[
            (VERSION, "0.2.27"),
            (ISODATE, "2026-11-21"),
            (VERSION, "10.0.0.1"),
        ],
    ),
    any("Школа №5 г. Москвы", &[]),
    any("и т.д... и так далее", &[]),
    any("в 2010-11 гг.", &[]),
    any("кто-то же", &[(HYPHEN, "-")]),
];

#[test]
fn frozen_cores_inside_sentences() {
    run_rows("sentences", DIAGNOSTIC_FREE_SENTENCES);
}

#[test]
fn the_span_of_a_frozen_fragment_is_its_core_with_exact_ranges() {
    // Design section 12.5: `Он - "врач", см. host:8080` has `Technical(Colon)` at `0:25..34`.
    let t = Typographer::new(Language::RU_RU, RuleSet::PROSE);
    let input = "Он - \"врач\", см. host:8080";
    let spans = probe::spans_of(&t, input);
    assert_eq!(spans, [(COLON, 0, 25..34)], "{}", show(input));
    assert_eq!(&input[25..34], "host:8080");
    // The same for a core with a wrapper.
    let input = "\"host:8080\".";
    let spans = probe::spans_of(&t, input);
    assert_eq!(spans, [(COLON, 0, 1..10)]);
    // A time range: the two times, not the separator.
    let input = "в 10:00-12:00";
    let spans = probe::spans_of(&t, input);
    let at = input.find("10:00").unwrap();
    assert_eq!(spans, [(TIME, 0, at..at + 5), (TIME, 0, at + 6..at + 11)]);
    // The hyphens of a compound word, one record each.
    let input = "mother-in-law";
    let spans = probe::spans_of(&t, input);
    assert_eq!(spans, [(HYPHEN, 0, 6..7), (HYPHEN, 0, 9..10)]);
    // Multi-byte text before the core.
    let input = "Ёж: кто-то";
    let spans = probe::spans_of(&t, input);
    let at = input.find('-').unwrap();
    assert_eq!(spans, [(HYPHEN, 0, at..at + 1)]);
}

#[test]
fn the_same_fragment_gets_the_same_kind_wherever_it_stands() {
    // Frames of design section 10.5 and the start and the end of the text.
    let cases: &[(&str, ProtectedKind)] = &[
        ("host:8080", COLON),
        ("--force", FLAG),
        ("/etc/hosts", PATH),
        ("user@example.com", EMAIL),
        ("0x1F", HEX),
        ("a,b,c", LIST),
        ("db.example.internal", DOTTED),
        ("kv_lang", IDENT),
        ("0.2.27", VERSION),
        ("2026-11-21", ISODATE),
        ("https://example.com/a?b=c", URL),
    ];
    let frames = [
        "X",
        "Запустите X сейчас.",
        "Значение: X, затем далее.",
        "Run X now.",
        "Value: X, then more.",
        "\"X\"",
        "(X)",
        "X\nX",
        "X\r\n\r\nX",
        "X X",
        "a X",
        "X b",
    ];
    for (text, kind) in cases {
        for frame in frames {
            let input = frame.replace('X', text);
            let n = frame.matches('X').count();
            for language in all_languages() {
                for set in SETS {
                    let t = Typographer::new(language.clone(), set);
                    let got = span_texts(&t, &input);
                    let kept: Vec<_> = got.iter().filter(|(k, _)| k == kind).collect();
                    assert_eq!(
                        kept.len(),
                        n,
                        "{language} {set}: {} in {}",
                        show(text),
                        show(&input)
                    );
                    assert!(
                        kept.iter().all(|(_, s)| s == text),
                        "{language} {set}: {}",
                        show(&input)
                    );
                }
            }
        }
    }
}

// MixedScript: raised by the classifier, no rule (design sections 5.2, 5.7, 12.4).

const MIXED: &[(&str, &[&str])] = &[
    // A Latin letter among Cyrillic ones.
    (
        "\u{41c}o\u{441}\u{43a}\u{432}\u{430}",
        &["\u{41c}o\u{441}\u{43a}\u{432}\u{430}"],
    ),
    (
        "C\u{435}\u{440}\u{432}\u{435}\u{440}",
        &["C\u{435}\u{440}\u{432}\u{435}\u{440}"],
    ),
    ("x\u{43b}\u{435}\u{431}", &["x\u{43b}\u{435}\u{431}"]),
    (
        "\u{41c}o\u{441}\u{43a}\u{432}\u{430} и Moscow",
        &["\u{41c}o\u{441}\u{43a}\u{432}\u{430}"],
    ),
    // Roman numerals with a Cyrillic `Х` are mixed, `ХХ` alone is Cyrillic.
    ("\u{425}IX", &["\u{425}IX"]),
    ("\u{425}I\u{425}", &["\u{425}I\u{425}"]),
    ("\u{425}\u{425}", &[]),
    ("XIX", &[]),
    // Greek and Latin.
    ("αβc", &["αβc"]),
    // Whole words in one script, digits, ZWJ and soft hyphens do not count.
    ("Москва", &[]),
    ("Moscow", &[]),
    ("x86", &[]),
    ("Москва2", &[]),
    ("сло\u{200d}во", &[]),
    ("Мо\u{ad}скве", &[]),
    ("Привет, world", &[]),
    // Letters of other scripts never make a word mixed (design section 5.2): a Latin word with a
    // schwa, a ligature or `º`, and Latin with CJK or with an Arabic-Indic digit after a soft
    // hyphen.
    ("Azərbaycan", &[]),
    ("\u{fb01}nd", &[]),
    ("N\u{ba} 5", &[]),
    ("日本語abc", &[]),
    ("abc\u{ad}\u{663}", &[]),
    // Two of Cyrillic, Latin and Greek, also across a soft hyphen.
    ("\u{41c}o\u{ad}c", &["\u{41c}o\u{ad}c"]),
    ("a\u{ad}\u{41c}b", &["a\u{ad}\u{41c}b"]),
    ("\u{3a9}mega", &["\u{3a9}mega"]),
    ("abc\u{41c}", &["abc\u{41c}"]),
];

#[test]
fn mixed_script_words_raise_a_warning() {
    for (input, expected) in MIXED {
        for language in all_languages() {
            for set in SETS {
                let t = Typographer::new(language.clone(), set);
                let diagnostics = diagnostics_of(&t, input);
                let got: Vec<&str> = diagnostics
                    .iter()
                    .filter(|d| d.0 == DiagnosticKind::MixedScript)
                    .map(|d| &input[d.4.clone()])
                    .collect();
                assert_eq!(got, expected.to_vec(), "{language} {set}: {}", show(input));
                for d in diagnostics
                    .iter()
                    .filter(|d| d.0 == DiagnosticKind::MixedScript)
                {
                    assert_eq!(d.1, Severity::Warning);
                    assert_eq!(d.2, None, "classifier diagnostics have no rule");
                    assert_eq!(d.3, 0);
                }
                // A mixed word is a word: never frozen.
                let spans = span_texts(&t, input);
                for (_, s) in spans {
                    assert!(
                        !expected.contains(&s),
                        "{language} {set}: mixed word frozen in {}",
                        show(input)
                    );
                }
            }
        }
    }
}

#[test]
fn a_dictionary_entry_raises_no_mixed_script() {
    // `μs` (Greek mu) and `µs` (micro sign) are units of the English table (design section 5.2).
    let t = Typographer::new(Language::EN_US, RuleSet::INTERFACE);
    for text in [
        "\u{3bc}s",
        "\u{b5}s",
        "5 \u{3bc}s",
        "5 \u{b5}s",
        "Latency 5 \u{b5}s.",
        "Latency 5 \u{3bc}s…",
    ] {
        let got = diagnostics_of(&t, text);
        assert!(
            got.iter().all(|d| d.0 != DiagnosticKind::MixedScript),
            "{}",
            show(text)
        );
    }
}

#[test]
fn mixed_script_is_reported_without_a_trigger_byte() {
    // Design section 3.2: `report` always runs the whole pipeline, and `typeset` skips it.
    let t = Typographer::new(Language::RU_RU, RuleSet::INTERFACE);
    let input = "\u{41c}o\u{441}\u{43a}\u{432}\u{430}";
    let report = t.report(input, typolish::Output::Unicode);
    assert_eq!(report.diagnostics().len(), 1);
    assert!(matches!(report.result(), std::borrow::Cow::Borrowed(_)));
}
