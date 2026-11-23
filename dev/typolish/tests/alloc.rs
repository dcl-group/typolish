//! The allocation table of design section 2.12, counted by a global allocator (design section
//! 13.8).
//!
//! Only the current thread is counted, so tests running in parallel do not disturb each other.
//! The rows that need the rules, the encoders and `format_number` join with them (design section
//! 15.4, stage 3).

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

use typolish::{Language, Output, Piece, RuleSet, Scratch, Slot, Typographer};

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
    // Section 13.8: a string with trigger bytes (`.`, `-`, digits) goes through ingest, windows,
    // the lexer and the classifier, with protection and technical fragments, and no rule of
    // Interface changes it.
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
    // Section 2.12: `typeset_into` makes no allocation when `buf` has room, for an unchanged and
    // for a changed string; `typeset` makes one for a changed string (section 13.8).
    const CHANGED: &str = "Он сказал \"привет\" и ушёл...";
    const UNCHANGED: &str = "Сохранить";
    let prose = Typographer::new(Language::RU_RU, RuleSet::PROSE);
    for t in [&RU_UI, &prose] {
        let expected = t.typeset(CHANGED, Output::Unicode).into_owned();
        // Prose also glues the two-letter `Он` (`nbsp.short-length`, section 8.7).
        assert!(expected.contains("сказал «привет» и") && expected.ends_with("ушёл…"));
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
        // allocates only what section 2.12 lists.
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
    // Section 2.12: at most 3 plus one per changed text. A glued space takes two bytes for one,
    // so a short glued text must not outgrow its string (`{n} дн.`, section 2.6).
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
