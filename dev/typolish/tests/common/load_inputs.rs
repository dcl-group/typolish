//! The pathological inputs of design section 3.12, generated at a requested size. Shared by
//! `benches/load.rs` and `tests/load.rs` through `#[path]`, so both measure the same strings.

/// A name and a generator.
pub type Pathological = (&'static str, fn(usize) -> String);

/// The name and the generator of every pathological input. A generator returns a string of about
/// `size` bytes (never fewer than `size`, at most a few dozen bytes more); the structure scales
/// with `size`, so a deeper nesting means a longer string.
pub const PATHOLOGICAL: &[Pathological] = &[
    ("run of \"", |n| "\"".repeat(n)),
    ("run of '", |n| "'".repeat(n)),
    ("run of `", |n| "`".repeat(n)),
    ("run of -", |n| "-".repeat(n)),
    ("run of .", |n| ".".repeat(n)),
    ("run of (", |n| "(".repeat(n)),
    ("unclosed backtick words", |n| repeat_to("`a ", n)),
    ("alternating Cyrillic and Latin", |n| repeat_to("аa", n)),
    ("ZWJ emoji clusters", |n| {
        repeat_to(
            "\u{1F468}\u{200D}\u{1F469}\u{200D}\u{1F467}\u{200D}\u{1F466}",
            n,
        )
    }),
    ("one line without spaces", |n| {
        repeat_to("ab.cd/ef_gh-ij", n)
    }),
    ("quote nesting", |n| {
        // `"x "x "x ... x" x" x"`: a quote opens after a space and closes after a letter; each
        // level takes six bytes.
        let depth = n / 6 + 1;
        let mut s = "\"x ".repeat(depth);
        s.push_str(&"x\" ".repeat(depth));
        s
    }),
    ("Format run before dashes", |n| {
        let mut s = "\u{200B}".repeat(n / 2 / 3 + 1);
        s.push_str(&repeat_to(" - a", n / 2));
        s
    }),
    ("Format pairs between dashes", |n| {
        repeat_to("a \u{2060}\u{200C}- \u{2060}\u{200C}b ", n)
    }),
    ("quotes inside a backtick wrapper", |n| {
        format!("`{}`", "\"".repeat(n))
    }),
    ("quotes inside a URL wrapper", |n| {
        format!("(http://a.b/c{})", "\"".repeat(n))
    }),
    // A typed `’` after a `‘` may close it as a sign-only pair (design section 6.3).
    ("run of ‘’", |n| repeat_to("‘’", n)),
];

/// `unit` repeated until the string is at least `size` bytes long.
pub fn repeat_to(unit: &str, size: usize) -> String {
    let times = size / unit.len().max(1) + 1;
    unit.repeat(times)
}
