//! The allocation table of design section 2.12, counted by a global allocator (design section
//! 13.8).
//!
//! Only the current thread is counted, so tests running in parallel do not disturb each other.
//! Stage 3 (design section 15.4) adds the rows of the encoders and of `format_number`: zero
//! allocations for numbers written into a buffer with room, for an unchanged string in every output
//! kind after warm-up, for `typeset_into` in every kind; one for a changed string and one more per
//! changed text of a template; and the first short call after an input larger than 64 KiB.

// The counting allocator needs `unsafe impl GlobalAlloc`; this is the only `unsafe` code of the
// package (design section 14.5).
#![allow(unsafe_code)]
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    clippy::print_stdout,
    clippy::print_stderr,
    clippy::dbg_macro
)]

use std::alloc::{GlobalAlloc, Layout, System};
use std::borrow::Cow;
use std::cell::Cell;

use typolish::{
    EscapeStyle, HtmlStyle, Language, Number, Output, Piece, RuleSet, Scratch, Slot, Typographer,
};

/// Delegates to `System` and counts allocations and reallocations of the current thread while
/// counting is on.
struct Counting;

thread_local! {
    static COUNTING: Cell<bool> = const { Cell::new(false) };
    static ALLOCS: Cell<usize> = const { Cell::new(0) };
    static REALLOCS: Cell<usize> = const { Cell::new(0) };
}

fn bump(counter: &'static std::thread::LocalKey<Cell<usize>>) {
    // `try_with`: a thread being torn down still allocates and must not panic here.
    let on = COUNTING.try_with(Cell::get).unwrap_or(false);
    if on {
        let _ = counter.try_with(|c| c.set(c.get() + 1));
    }
}

// SAFETY: every method delegates to `System` with the arguments it received.
unsafe impl GlobalAlloc for Counting {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        bump(&ALLOCS);
        // SAFETY: the caller upholds the contract of `GlobalAlloc::alloc`.
        unsafe { System.alloc(layout) }
    }

    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
        bump(&ALLOCS);
        // SAFETY: the caller upholds the contract of `GlobalAlloc::alloc_zeroed`.
        unsafe { System.alloc_zeroed(layout) }
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        // SAFETY: the caller upholds the contract of `GlobalAlloc::dealloc`.
        unsafe { System.dealloc(ptr, layout) }
    }

    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        bump(&REALLOCS);
        // SAFETY: the caller upholds the contract of `GlobalAlloc::realloc`.
        unsafe { System.realloc(ptr, layout, new_size) }
    }
}

#[global_allocator]
static GLOBAL: Counting = Counting;

/// Allocations and reallocations made on this thread by `f`, and its result.
fn count<R>(f: impl FnOnce() -> R) -> (usize, usize, R) {
    ALLOCS.with(|c| c.set(0));
    REALLOCS.with(|c| c.set(0));
    COUNTING.with(|c| c.set(true));
    let result = f();
    COUNTING.with(|c| c.set(false));
    (ALLOCS.with(Cell::get), REALLOCS.with(Cell::get), result)
}

static RU_UI: Typographer = Typographer::new(Language::RU_RU, RuleSet::INTERFACE);

#[test]
fn the_counter_counts() {
    let (allocs, reallocs, v) = count(|| {
        let mut v = vec![1u8];
        v.extend_from_slice(&[2, 3, 4, 5]);
        v
    });
    assert_eq!((allocs, reallocs), (1, 1));
    assert_eq!(v, [1, 2, 3, 4, 5]);
}

#[test]
fn typographer_new_does_not_allocate() {
    let (allocs, reallocs, t) = count(|| Typographer::new(Language::EN_US, RuleSet::PROSE));
    assert_eq!((allocs, reallocs), (0, 0));
    drop(t);
}

#[test]
fn an_unchanged_string_does_not_allocate_after_warm_up() {
    let _ = RU_UI.typeset("Сохранить", Output::Unicode);
    let (allocs, reallocs, out) = count(|| RU_UI.typeset("Сохранить", Output::Unicode));
    assert_eq!((allocs, reallocs), (0, 0));
    assert!(matches!(out, Cow::Borrowed(_)));

    let mut scratch = Scratch::new();
    let _ = RU_UI.typeset_with(&mut scratch, "Сохранить", Output::Unicode);
    let (allocs, reallocs, out) =
        count(|| RU_UI.typeset_with(&mut scratch, "Сохранить", Output::Unicode));
    assert_eq!((allocs, reallocs), (0, 0));
    assert!(matches!(out, Cow::Borrowed(_)));
}

#[test]
fn an_unchanged_string_through_the_whole_pipeline_does_not_allocate_after_warm_up() {
    // Design section 13.8: a string with trigger bytes (`.`, `-`, digits) goes through ingest,
    // windows, the lexer and the classifier, with protection and technical fragments, and no rule
    // of Interface changes it.
    const INPUT: &str = "Адрес host:8080, версия v1.2.3, код `a-b`\n\nПуть /usr/bin, id 0x1F";
    let _ = RU_UI.typeset(INPUT, Output::Unicode);
    let (allocs, reallocs, out) = count(|| RU_UI.typeset(INPUT, Output::Unicode));
    assert_eq!((allocs, reallocs), (0, 0));
    assert!(matches!(out, Cow::Borrowed(_)));

    let mut scratch = Scratch::new();
    let _ = RU_UI.typeset_with(&mut scratch, INPUT, Output::Unicode);
    let (allocs, reallocs, out) =
        count(|| RU_UI.typeset_with(&mut scratch, INPUT, Output::Unicode));
    assert_eq!((allocs, reallocs), (0, 0));
    assert!(matches!(out, Cow::Borrowed(_)));
}

#[test]
fn typeset_into_a_buffer_with_room_does_not_allocate() {
    // Design section 2.12: `typeset_into` makes no allocation when `buf` has room, for an unchanged
    // and for a changed string; `typeset` makes one for a changed string (design section 13.8).
    const CHANGED: &str = "Он сказал \"привет\" и ушёл...";
    const UNCHANGED: &str = "Сохранить";
    let prose = Typographer::new(Language::RU_RU, RuleSet::PROSE);
    // Interface glues only after the listed `и`; Prose also glues the two-letter `Он`
    // (`nbsp.short-length` is Prose only, design section 8.7).
    for (t, exact) in [
        (&RU_UI, "Он сказал «привет» и\u{a0}ушёл…"),
        (&prose, "Он\u{a0}сказал «привет» и\u{a0}ушёл…"),
    ] {
        let expected = t.typeset(CHANGED, Output::Unicode).into_owned();
        assert_eq!(expected, exact);
        let mut buf = String::with_capacity(1024);
        t.typeset_into(CHANGED, Output::Unicode, &mut buf);
        t.typeset_into(UNCHANGED, Output::Unicode, &mut buf);
        for (text, out) in [(CHANGED, expected.as_str()), (UNCHANGED, UNCHANGED)] {
            buf.clear();
            buf.push_str("> ");
            let (allocs, reallocs, ()) = count(|| t.typeset_into(text, Output::Unicode, &mut buf));
            assert_eq!((allocs, reallocs), (0, 0), "{text}");
            assert_eq!(buf, format!("> {out}"));
        }
        let _ = t.typeset(CHANGED, Output::Unicode);
        let (allocs, reallocs, out) = count(|| t.typeset(CHANGED, Output::Unicode));
        assert_eq!((allocs, reallocs), (1, 0));
        assert!(matches!(out, Cow::Owned(_)));
    }
}

#[test]
fn a_changed_string_is_one_allocation_however_much_it_grows() {
    // Design section 2.12: the Unicode output string is sized exactly, so a text that the rules
    // grow to twice its length or more (`"a"` → `“a”`, three bytes to seven) is still one
    // allocation.
    let t = Typographer::new(Language::EN_US, RuleSet::PROSE);
    // Short enough that the scratch stays under the 64 KiB it keeps between calls.
    let long = "\"a\" ".repeat(100);
    // 240 edits in one window: more than a stable sort sorts without a heap buffer, so the edits
    // are sorted in place (design section 15.3, C39); still under 64 KiB of scratch.
    let many = "\"a\" ".repeat(120);
    for text in ["\"a\"", long.as_str(), many.as_str()] {
        let first = t.typeset(text, Output::Unicode);
        assert!(first.len() >= 2 * text.len(), "{text:?} -> {first:?}");
        drop(first);
        let (allocs, reallocs, out) = count(|| t.typeset(text, Output::Unicode));
        assert!(matches!(out, Cow::Owned(_)));
        assert_eq!((allocs, reallocs), (1, 0), "{text:?}");
        let mut scratch = Scratch::new();
        let _ = t.typeset_with(&mut scratch, text, Output::Unicode);
        let (allocs, reallocs, _) = count(|| t.typeset_with(&mut scratch, text, Output::Unicode));
        assert_eq!((allocs, reallocs), (1, 0), "with {text:?}");
    }
}

/// A template of `n` repetitions of a text and a slot.
fn template(n: u8) -> Vec<Piece<'static, u8>> {
    (0..n)
        .flat_map(|i| [Piece::Text("слово "), Piece::Slot(i, Slot::WORD)])
        .collect()
}

#[test]
fn a_template_allocates_three_vectors_whatever_its_length() {
    for n in [1, 3, 5, 9, 17, 100] {
        // The warm-up grows the thread-local scratch to this input; the call counted below then
        // allocates only what design section 2.12 lists.
        let _ = RU_UI.typeset_template(template(n), Output::Unicode);
        let pieces = template(n);
        let (allocs, reallocs, t) = count(|| RU_UI.typeset_template(pieces, Output::Unicode));
        assert_eq!((allocs, reallocs), (3, 0), "n = {n}");
        assert_eq!(t.slots().len(), usize::from(n));
        assert!(!t.changed());
    }
}

#[test]
fn a_merged_text_is_one_allocation() {
    // Two texts merged from several non-empty pieces: one string each (design section 2.12).
    let pieces = vec![
        Piece::Text("a"),
        Piece::Text("b"),
        Piece::Slot(0u8, Slot::WORD),
        Piece::Text("c"),
        Piece::Markup("<b>"),
        Piece::Text(""),
        Piece::Protected("`x`"),
    ];
    let _ = RU_UI.typeset_template(pieces.clone(), Output::Unicode);
    let (allocs, reallocs, t) = count(|| RU_UI.typeset_template(pieces, Output::Unicode));
    assert_eq!((allocs, reallocs), (3 + 2, 0));
    assert_eq!(t.texts(), ["ab", "c<b>`x`"]);
    assert!(!t.changed());
}

#[test]
fn a_template_iterator_that_calls_the_typographer_adds_nothing_else() {
    let _ = RU_UI.typeset_template(template(1), Output::Unicode);
    let _ = RU_UI.typeset("слово", Output::Unicode);
    let pieces = [Piece::Text("слово"), Piece::Slot(0u8, Slot::NUMBER)];
    let (allocs, reallocs, t) = count(|| {
        RU_UI.typeset_template(
            pieces.into_iter().inspect(|piece| {
                if let Piece::Text(text) = piece {
                    let _ = RU_UI.typeset(text, Output::Unicode);
                }
            }),
            Output::Unicode,
        )
    });
    assert_eq!((allocs, reallocs), (3, 0));
    assert_eq!(t.texts(), ["слово", ""]);
}

/// A row of the template growth table: the pieces, the texts and the number of changed texts.
type Grown = (Vec<Piece<'static, u8>>, &'static [&'static str], usize);

#[test]
fn a_changed_template_text_is_one_allocation_however_much_it_grows() {
    // Design section 2.12: at most 3 plus one per changed text. A glued space takes two bytes for
    // one, so a short glued text must not outgrow its string (`{n} дн.`, design section 2.6).
    let rows: [Grown; 3] = [
        (
            vec![Piece::Slot(0, Slot::NUMBER), Piece::Text(" дн.")],
            &["", "\u{a0}дн."],
            1,
        ),
        (
            vec![
                Piece::Text("Скачано "),
                Piece::Slot(0, Slot::NUMBER),
                Piece::Text(" МБ за "),
                Piece::Slot(1, Slot::NUMBER),
                Piece::Text(" с."),
            ],
            &["Скачано ", "\u{a0}МБ за\u{a0}", "\u{a0}с."],
            2,
        ),
        (
            // Ten quotes in a short text: every one grows by a byte.
            vec![
                Piece::Text("\"a\" \"b\" \"c\" \"d\" \"e\" "),
                Piece::Slot(0, Slot::WORD),
            ],
            &["«a» «b» «c» «d» «e» ", ""],
            1,
        ),
    ];
    for (pieces, texts, changed) in rows {
        let _ = RU_UI.typeset_template(pieces.clone(), Output::Unicode);
        let (allocs, reallocs, t) = count(|| RU_UI.typeset_template(pieces, Output::Unicode));
        assert_eq!(t.texts(), texts);
        assert_eq!((allocs, reallocs), (3 + changed, 0), "{texts:?}");
    }
}

/// Every output kind of design section 2.8, the nowrap styles and both scopes included.
fn outputs() -> Vec<Output> {
    vec![
        Output::Unicode,
        Output::Html(HtmlStyle::NAMED),
        Output::Html(HtmlStyle::DECIMAL),
        Output::Html(HtmlStyle::HEX),
        Output::Html(HtmlStyle::CHARACTERS),
        Output::Html(HtmlStyle::NAMED.nowrap_class("nowrap")),
        Output::Html(HtmlStyle::HEX.nowrap_style()),
        Output::Escaped(EscapeStyle::JS),
        Output::Escaped(EscapeStyle::JS_HEX),
        Output::Escaped(EscapeStyle::JS_CODE_POINT),
        Output::Escaped(EscapeStyle::JSON),
        Output::Escaped(EscapeStyle::RUST),
        Output::Escaped(EscapeStyle::CSS),
        Output::Escaped(EscapeStyle::JS.non_ascii()),
        Output::Escaped(EscapeStyle::CSS.non_ascii()),
    ]
}

/// Values of every variant of `Number`, the widest of each and the symbols included.
fn values() -> Vec<Number<'static>> {
    vec![
        Number::from(0),
        Number::from(-12345),
        Number::from(i128::MIN),
        Number::from(u128::MAX),
        Number::from(1234567.5),
        Number::from(-12345.678f32),
        Number::from(f64::MAX),
        Number::from(f64::MIN_POSITIVE),
        Number::from(5e-324),
        Number::from(f32::MAX),
        Number::from(0.1f32),
        Number::from(f64::NAN),
        Number::from(f64::NEG_INFINITY),
        Number::from(-0.0),
        Number::fixed(1234.5678, 2),
        Number::fixed(-0.001, 1),
        Number::fixed(1e300, 3),
    ]
}

#[test]
fn numbers_written_into_a_buffer_with_room_do_not_allocate() {
    use std::fmt::Write;
    let ru = Typographer::new(Language::RU_RU, RuleSet::PROSE);
    let en = Typographer::new(Language::EN_US, RuleSet::INTERFACE);
    let common = Typographer::new(Language::COMMON, RuleSet::EMPTY);
    let narrow = Typographer::builder(Language::RU_RU, RuleSet::PROSE)
        .group_separator('\u{202f}')
        .min_grouping_digits(0)
        .build()
        .unwrap();
    // The longest `f64` in `{}` is 327 bytes; every group separator of a 309-digit number is a
    // six-byte escape in the widest syntax, and the buffer has room for all of it.
    let mut buf = String::with_capacity(16 * 1024);
    for t in [&ru, &en, &common, &narrow] {
        for output in outputs() {
            for n in values() {
                buf.clear();
                let (allocs, reallocs, ()) = count(|| {
                    write!(buf, "{}", t.format_number(n, output)).unwrap();
                });
                assert_eq!((allocs, reallocs), (0, 0), "{n:?} {output:?}");
                assert!(!buf.is_empty());
            }
            buf.clear();
            let (allocs, reallocs, ()) = count(|| {
                write!(buf, "{}", t.format_range(-12345, 1234567.5, output)).unwrap();
                write!(buf, "{}", t.format_range(f64::MAX, f64::MIN, output)).unwrap();
            });
            assert_eq!((allocs, reallocs), (0, 0), "range {output:?}");
        }
    }
}

#[test]
fn a_decimal_string_and_a_float_cost_nothing() {
    use std::fmt::Write;
    let t = Typographer::new(Language::RU_RU, RuleSet::PROSE);
    let mut buf = String::with_capacity(4096);
    let (allocs, reallocs, ()) = count(|| {
        let n = Number::decimal("-0012345678.5000").unwrap();
        write!(buf, "{}", t.format_number(n, Output::Unicode)).unwrap();
        let n = Number::decimal("+7").unwrap();
        write!(
            buf,
            "{}",
            t.format_number(n, Output::Escaped(EscapeStyle::RUST))
        )
        .unwrap();
        write!(
            buf,
            "{}",
            t.format_number(Number::fixed(2.5, 3), Output::Unicode)
        )
        .unwrap();
    });
    assert_eq!((allocs, reallocs), (0, 0));
    assert!(buf.contains("0\u{a0}012"), "{buf}");
    // Only a rejected string allocates, for the text of its error.
    let (allocs, _, err) = count(|| Number::decimal("1e3"));
    assert!(err.is_err());
    assert!(allocs >= 1);
}

#[test]
fn an_unchanged_string_does_not_allocate_in_any_output_kind() {
    // Design section 2.12: with `Html` and `Escaped` only if nothing needs encoding. No rule
    // changes the texts, and none of their bytes is encoded by any kind below.
    const SHORT: &str = "Сохранить";
    const PIPELINE: &str = "Адрес host:8080, версия v1.2.3, id 0x1F, путь /usr/bin";
    let t = Typographer::new(Language::RU_RU, RuleSet::INTERFACE);
    let mut scratch = Scratch::new();
    for output in outputs() {
        for input in [SHORT, PIPELINE] {
            let _ = t.typeset(input, output);
            let (allocs, reallocs, out) = count(|| t.typeset(input, output));
            let non_ascii = matches!(output, Output::Escaped(s) if s == EscapeStyle::JS.non_ascii() || s == EscapeStyle::CSS.non_ascii());
            if non_ascii {
                // The Cyrillic is encoded by `non_ascii()`: one allocation, never borrowed.
                assert!(matches!(out, Cow::Owned(_)), "{input} {output:?}");
                assert_eq!((allocs, reallocs), (1, 0), "{input} {output:?}");
            } else {
                assert_eq!((allocs, reallocs), (0, 0), "{input} {output:?}");
                assert!(matches!(out, Cow::Borrowed(_)), "{input} {output:?}");
            }
            let _ = t.typeset_with(&mut scratch, input, output);
            let (allocs, reallocs, _) = count(|| t.typeset_with(&mut scratch, input, output));
            let expected = if non_ascii { (1, 0) } else { (0, 0) };
            assert_eq!((allocs, reallocs), expected, "with {input} {output:?}");
        }
    }
}

#[test]
fn typeset_into_a_buffer_with_room_does_not_allocate_in_any_output_kind() {
    const INPUT: &str = "Он сказал \"привет\" - и ушёл... \\ <b> & `x` \n5 МБ";
    let ui = Typographer::new(Language::RU_RU, RuleSet::INTERFACE);
    let prose = Typographer::new(Language::RU_RU, RuleSet::PROSE);
    for t in [&ui, &prose] {
        for output in outputs() {
            let expected = t.typeset(INPUT, output).into_owned();
            let mut buf = String::with_capacity(4096);
            t.typeset_into(INPUT, output, &mut buf);
            buf.clear();
            buf.push_str("> ");
            let (allocs, reallocs, ()) = count(|| t.typeset_into(INPUT, output, &mut buf));
            assert_eq!((allocs, reallocs), (0, 0), "{output:?}");
            assert_eq!(buf, format!("> {expected}"), "{output:?}");
        }
    }
}

#[test]
fn a_text_that_needs_encoding_is_one_allocation() {
    // Design section 2.12: one allocation, the output string, counted first, however much the
    // encoding grows the text: one `<` in a long text, a short text that grows almost fivefold, a
    // thousand apostrophes and Cyrillic escaped by `non_ascii()`. No rule runs, so only the
    // encoding changes the text.
    const LONG: &str = "Сохранить изменения в файле перед закрытием окна <выйти> из программы и вернуться к списку";
    let apostrophes = "'".repeat(1000);
    let cyrillic = "Сохранить изменения в файле перед закрытием окна ".repeat(4);
    let texts = [LONG, "\"<>&'", apostrophes.as_str(), cyrillic.as_str()];
    let t = Typographer::new(Language::RU_RU, RuleSet::EMPTY);
    let mut grew_past_double = false;
    for output in outputs() {
        for text in texts {
            let first = t.typeset(text, output);
            if matches!(first, Cow::Borrowed(_)) {
                continue;
            }
            grew_past_double |= first.len() > 2 * text.len();
            drop(first);
            let (allocs, reallocs, out) = count(|| t.typeset(text, output));
            assert!(matches!(out, Cow::Owned(_)), "{output:?}");
            let head: String = text.chars().take(5).collect();
            assert_eq!((allocs, reallocs), (1, 0), "{output:?} {head:?}");
        }
    }
    assert!(grew_past_double, "some text grows past twice its length");
    // The same through `typeset_with`.
    let mut scratch = Scratch::new();
    let _ = t.typeset_with(&mut scratch, LONG, Output::Html(HtmlStyle::NAMED));
    let (allocs, reallocs, _) =
        count(|| t.typeset_with(&mut scratch, LONG, Output::Html(HtmlStyle::NAMED)));
    assert_eq!((allocs, reallocs), (1, 0));
}

#[test]
fn a_template_text_that_needs_encoding_is_one_allocation_however_much_it_grows() {
    // Design section 2.12: at most 3 plus one per text that is changed; the encoding is a change.
    let t = Typographer::new(Language::RU_RU, RuleSet::EMPTY);
    let wide = [
        Output::Html(HtmlStyle::NAMED),
        Output::Html(HtmlStyle::HEX),
        Output::Escaped(EscapeStyle::JS),
        Output::Escaped(EscapeStyle::JSON),
        Output::Escaped(EscapeStyle::CSS),
        Output::Escaped(EscapeStyle::JS.non_ascii()),
    ];
    for output in wide {
        let pieces = || {
            vec![
                Piece::Text("<<<<<<<<"),
                Piece::Slot(0u8, Slot::WORD),
                Piece::Text("&&&&"),
                Piece::Slot(1, Slot::NUMBER),
                Piece::Text("слово"),
            ]
        };
        let _ = t.typeset_template(pieces(), output);
        let input = pieces();
        let (allocs, reallocs, out) = count(|| t.typeset_template(input, output));
        assert!(out.changed(), "{output:?}");
        // The first text is encoded by every kind and the second by all but CSS, which leaves `&`
        // alone; `слово` is changed only with `non_ascii()`.
        let changed = out
            .texts()
            .iter()
            .filter(|text| matches!(text, Cow::Owned(_)))
            .count();
        assert!((1..=3).contains(&changed), "{output:?}: {changed}");
        assert_eq!((allocs, reallocs), (3 + changed, 0), "{output:?}");
    }
    // A text that needs no encoding adds nothing.
    for output in outputs() {
        let pieces = || vec![Piece::Text("слово "), Piece::Slot(0u8, Slot::WORD)];
        let non_ascii = matches!(output, Output::Escaped(s) if s == EscapeStyle::JS.non_ascii() || s == EscapeStyle::CSS.non_ascii());
        let _ = t.typeset_template(pieces(), output);
        let input = pieces();
        let (allocs, reallocs, out) = count(|| t.typeset_template(input, output));
        assert_eq!(out.changed(), non_ascii, "{output:?}");
        assert_eq!(
            (allocs, reallocs),
            (3 + usize::from(non_ascii), 0),
            "{output:?}"
        );
    }
}

#[test]
fn after_an_input_larger_than_64_kib_the_first_short_call_allocates_and_the_second_does_not() {
    // Design section 2.12: when the capacity of the thread-local scratch grew past 64 KiB, it is
    // replaced by `Scratch::new()` after the call, so the next call allocates as a first call does;
    // a caller's `Scratch` is never shrunk.
    const SHORT: &str = "Адрес host:8080, версия v1.2.3, id 0x1F";
    let big = "слово - слово... \"да\" 5 МБ. ".repeat(4000);
    assert!(big.len() > 64 * 1024);
    let t = Typographer::new(Language::RU_RU, RuleSet::PROSE);

    let _ = t.typeset(SHORT, Output::Unicode);
    let (allocs, _, _) = count(|| t.typeset(SHORT, Output::Unicode));
    assert_eq!(allocs, 0, "warm");
    let out = t.typeset(&big, Output::Unicode);
    assert!(matches!(out, Cow::Owned(_)));
    drop(out);
    let (first, _, out) = count(|| t.typeset(SHORT, Output::Unicode));
    assert!(matches!(out, Cow::Borrowed(_)));
    assert!(
        first > 0,
        "the first short call after a large input allocates"
    );
    let (second, reallocs, _) = count(|| t.typeset(SHORT, Output::Unicode));
    assert_eq!((second, reallocs), (0, 0), "the second does not");

    // The same for a changed short string: one allocation for the output once the scratch is warm.
    let _ = t.typeset(&big, Output::Html(HtmlStyle::NAMED));
    let _ = t.typeset("Он сказал \"да\"", Output::Unicode);
    let (allocs, _, out) = count(|| t.typeset("Он сказал \"да\"", Output::Unicode));
    assert!(matches!(out, Cow::Owned(_)));
    assert_eq!(allocs, 1);

    // A caller's scratch keeps its capacity: warmed by both inputs, it needs nothing more for the
    // short one after the large one.
    let mut scratch = Scratch::new();
    let _ = t.typeset_with(&mut scratch, SHORT, Output::Unicode);
    let out = t.typeset_with(&mut scratch, &big, Output::Unicode);
    drop(out);
    let (allocs, reallocs, out) = count(|| t.typeset_with(&mut scratch, SHORT, Output::Unicode));
    assert_eq!((allocs, reallocs), (0, 0));
    assert!(matches!(out, Cow::Borrowed(_)));
}
