//! The compile-time commitments of design section 2.13 and the auto traits of design section 2.4:
//! `Send + Sync`, `Copy`, `Clone`, `Debug`, `Eq`, `Hash` and `const` use.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    clippy::print_stdout,
    clippy::print_stderr,
    clippy::dbg_macro
)]

use std::fmt::Debug;
use std::hash::Hash;

use typolish::{
    Abbreviation, Bind, DashData, DashStyle, EllipsisForms, EscapeStyle, HtmlStyle, Language,
    LanguageData, MonthPosition, NarrowGlue, Number, NumberData, NumberDisplay, Output,
    PercentSpacing, Piece, QuotePairs, RangeDisplay, Rule, RuleSet, Scratch, Script, SingleQuotes,
    Slot, SlotKind, Template, TemplatePart, Typographer, TypographerBuilder,
};

fn send_sync<T: Send + Sync>() {}
fn copy<T: Copy>() {}
fn clone<T: Clone>() {}
fn value<T: Clone + Debug + PartialEq + Eq + Hash>() {}
fn kind<T: Clone + Copy + Debug + PartialEq + Eq>() {}

#[test]
fn send_and_sync() {
    send_sync::<Typographer>();
    send_sync::<TypographerBuilder>();
    send_sync::<Scratch>();
    send_sync::<Language>();
    send_sync::<LanguageData>();
    send_sync::<RuleSet>();
    send_sync::<Rule>();
    send_sync::<Output>();
    send_sync::<HtmlStyle>();
    send_sync::<EscapeStyle>();
    send_sync::<Slot>();
    send_sync::<Piece<'static, u32>>();
    send_sync::<Template<'static, u32>>();
    send_sync::<typolish::Error>();
    // A static typographer is shared by every thread.
    static T: Typographer = Typographer::new(Language::EN_US, RuleSet::PROSE);
    fn is_static<X: 'static>(_: &X) {}
    is_static(&T);
}

#[test]
fn copy_types() {
    copy::<Output>();
    copy::<RuleSet>();
    copy::<Rule>();
    copy::<Slot>();
    copy::<SlotKind>();
    copy::<HtmlStyle>();
    copy::<EscapeStyle>();
    copy::<Piece<'static, u8>>();
    copy::<TemplatePart<'static, u8>>();
    // A part borrows its payload, so it is `Copy` whatever the payload (design section 2.13).
    struct NotClone;
    copy::<TemplatePart<'static, String>>();
    copy::<TemplatePart<'static, NotClone>>();
    // The value types of design section 2.1.
    copy::<QuotePairs>();
    copy::<SingleQuotes>();
    copy::<DashStyle>();
    copy::<PercentSpacing>();
    copy::<NarrowGlue>();
    copy::<Script>();
    copy::<DashData>();
    copy::<EllipsisForms>();
    copy::<Abbreviation>();
    copy::<Bind>();
    copy::<MonthPosition>();
    copy::<NumberData>();
}

#[test]
fn report_kinds_are_copy_debug_and_eq() {
    // Design section 2.9 lists these derives and no `Hash` (design section 2.13).
    kind::<typolish::ProtectedKind>();
    kind::<typolish::TechKind>();
}

#[test]
fn clone_debug_eq_hash() {
    value::<Language>();
    value::<LanguageData>();
    value::<Rule>();
    value::<RuleSet>();
    value::<Output>();
    value::<HtmlStyle>();
    value::<EscapeStyle>();
    value::<Slot>();
    value::<SlotKind>();
    value::<Piece<'static, u8>>();
    value::<QuotePairs>();
    value::<SingleQuotes>();
    value::<DashStyle>();
    value::<PercentSpacing>();
    value::<NarrowGlue>();
    value::<Script>();
    value::<DashData>();
    value::<EllipsisForms>();
    value::<Abbreviation>();
    value::<Bind>();
    value::<MonthPosition>();
    value::<NumberData>();
    clone::<Typographer>();
    clone::<TypographerBuilder>();
    clone::<Template<'static, u8>>();
}

#[test]
fn const_use() {
    const _: Scratch = Scratch::new();
    const _: Language = Language::from_static(&LanguageData::COMMON);
    const _: RuleSet = RuleSet::INTERFACE.with(Rule::NumberTimes);
    const _: Slot = Slot::new(SlotKind::Number);
    const _: HtmlStyle = HtmlStyle::NAMED.nowrap_class("nowrap");
    const _: HtmlStyle = HtmlStyle::HEX.nowrap_style();
    const _: EscapeStyle = EscapeStyle::JS.non_ascii();
    const _: NarrowGlue = NarrowGlue::INITIALS.union(NarrowGlue::SIGNS);
    const _: QuotePairs = QuotePairs::new(('«', '»'), ('„', '“'));
    const _: DashData = DashData::new(DashStyle::Spaced, false);
    const _: EllipsisForms = EllipsisForms::new("?..", "!..", "?!..");
    const _: Abbreviation = Abbreviation::new(&["т.", "е."], Bind::Inner);
    const _: NumberData = NumberData::new().with_decimal(',');
    static _T: Typographer = Typographer::new(Language::RU_RU, RuleSet::PROSE);
}

#[test]
fn scratch_is_default_and_debug() {
    let a = Scratch::new();
    let b = Scratch::default();
    assert_eq!(format!("{a:?}"), format!("{b:?}"));
    // Opaque: the buffers are not shown (design sections 2.1 and 2.13).
    assert_eq!(format!("{a:?}"), "Scratch { .. }");
}

#[test]
fn number_data_default_is_new() {
    assert_eq!(NumberData::default(), NumberData::new());
}

#[test]
fn slots() {
    assert_eq!(Slot::NUMBER.kind(), SlotKind::Number);
    assert_eq!(Slot::WORD.kind(), SlotKind::Word);
    assert_eq!(Slot::DATE.kind(), SlotKind::Date);
    assert_eq!(Slot::OTHER.kind(), SlotKind::Other);
    for kind in [
        SlotKind::Number,
        SlotKind::Word,
        SlotKind::Date,
        SlotKind::Other,
    ] {
        assert_eq!(Slot::new(kind).kind(), kind);
    }
    assert_eq!(Slot::new(SlotKind::Number), Slot::NUMBER);
    assert_ne!(Slot::NUMBER, Slot::WORD);
}

#[test]
fn output_kinds_are_distinct_values() {
    let styles = [
        HtmlStyle::NAMED,
        HtmlStyle::DECIMAL,
        HtmlStyle::HEX,
        HtmlStyle::CHARACTERS,
    ];
    for (i, a) in styles.iter().enumerate() {
        for (j, b) in styles.iter().enumerate() {
            assert_eq!(a == b, i == j, "{i} {j}");
        }
        assert_ne!(*a, a.nowrap_style());
        assert_ne!(*a, a.nowrap_class("nowrap"));
        assert_ne!(a.nowrap_class("a"), a.nowrap_class("b"));
    }
    let escapes = [
        EscapeStyle::JS,
        EscapeStyle::JS_HEX,
        EscapeStyle::JS_CODE_POINT,
        EscapeStyle::JSON,
        EscapeStyle::RUST,
        EscapeStyle::CSS,
    ];
    for (i, a) in escapes.iter().enumerate() {
        for (j, b) in escapes.iter().enumerate() {
            assert_eq!(a == b, i == j, "{i} {j}");
        }
        assert_ne!(*a, a.non_ascii());
    }
    assert_ne!(Output::Unicode, Output::Html(HtmlStyle::NAMED));
    assert_ne!(Output::Unicode, Output::Escaped(EscapeStyle::JS));
    assert_ne!(Output::Html(HtmlStyle::NAMED), Output::Html(HtmlStyle::HEX));
    assert_ne!(
        Output::Escaped(EscapeStyle::JS),
        Output::Escaped(EscapeStyle::RUST)
    );
}

#[test]
fn pieces_are_copy_when_the_payload_is() {
    let piece: Piece<'_, u8> = Piece::Slot(3, Slot::NUMBER);
    let again = piece;
    assert_eq!(again, Piece::Slot(3, Slot::NUMBER));
    assert_ne!(Piece::<u8>::Text("a"), Piece::Protected("a"));
    assert_ne!(Piece::<u8>::Text("a"), Piece::Markup("a"));
    assert_ne!(Piece::<u8>::Slot(1, Slot::WORD), Piece::Slot(2, Slot::WORD));
}

#[test]
fn language_is_clone_but_not_copy() {
    // The inherent constant exists only for `Copy` types and shadows the trait's.
    trait NotCopy {
        const COPY: bool = false;
    }
    impl<T> NotCopy for T {}
    struct Probe<T>(std::marker::PhantomData<T>);
    #[allow(dead_code)]
    impl<T: Copy> Probe<T> {
        const COPY: bool = true;
    }
    const { assert!(!Probe::<Language>::COPY) };
    const { assert!(!Probe::<LanguageData>::COPY) };
    // The probe itself works.
    const { assert!(Probe::<Rule>::COPY) };
    const { assert!(Probe::<RuleSet>::COPY) };
}

// Design section 2.13: `Number` holds floats and is `PartialEq` only; it is `Copy`. The lazy
// displays of design section 2.7 implement `Display`.
#[test]
fn number_and_its_displays() {
    fn plain<T: Clone + Copy + Debug + PartialEq>() {}
    fn display<T: std::fmt::Display>() {}
    plain::<Number<'static>>();
    send_sync::<Number<'static>>();
    display::<NumberDisplay<'static, 'static>>();
    display::<RangeDisplay<'static, 'static>>();
    // Every type of design section 2.7 converts, and the formatting entry points take any of them.
    fn takes<'n>(_: impl Into<Number<'n>>) {}
    takes(1i8);
    takes(1i16);
    takes(1i32);
    takes(1i64);
    takes(1i128);
    takes(1isize);
    takes(1u8);
    takes(1u16);
    takes(1u32);
    takes(1u64);
    takes(1u128);
    takes(1usize);
    takes(1f32);
    takes(1f64);
    takes(Number::fixed(1.0, 1));
    takes(Number::decimal("1").unwrap());
    // `Number::fixed` owns its digits: `Number<'static>`.
    let _: Number<'static> = Number::fixed(1.0, 1);
}
