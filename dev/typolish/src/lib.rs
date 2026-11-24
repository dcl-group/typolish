//! Typesetting of plain text for a language.
//!
//! The crate turns text typed the easy way (straight quotes, hyphens, three full stops, ordinary
//! spaces) into typeset text: quotes by nesting level, apostrophes, dashes, ellipses, no-break
//! spaces, and rules for words, signs and numbers. A [`Typographer`] is built once with a
//! [`Language`] and a [`RuleSet`] and then only applied; the [`Output`] kind is given with every
//! call. Its design is in
//! [`docs/design/`](https://github.com/dcl-group/typolish/tree/main/docs/design) of the
//! repository; "design section N.M" in this documentation refers to it.
//!
//! The three inputs are a whole string ([`Typographer::typeset`]), a message template of
//! [`Piece`]s whose slots keep their place ([`Typographer::typeset_template`]), and a number
//! ([`Typographer::format_number`], [`Typographer::format_range`]). [`Typographer::report`] lists
//! every edit, protected span and diagnostic, [`Typographer::lint`] checks a plain source, and
//! [`Typographer::to_plain`] maps typeset text back to the plain form. Every [`Output`] kind is
//! encoded: Unicode, HTML with entities or nowrap elements, and escaped string bodies for
//! JavaScript, JSON, Rust and CSS. Commands and code are protected with backticks, delimiters of
//! the caller's own or [`Piece::Protected`]; the README lists the known limits.
//!
//! The crate does not normalize Unicode. A word in NFD does not match an entry of a language
//! list: `дюйм.` written with `и` followed by U+0306 COMBINING BREVE in place of `й` is not the
//! unit of the ru-RU list, so `5 дюйм.` in that form gets no no-break space before the unit.
//! Rules that read no list, such as `nbsp.short-length`, still apply. Callers that need the
//! lists to match normalize the text to NFC before typesetting.
//!
//! ```
//! use typolish::{Language, Output, RuleSet, Typographer};
//!
//! static RU_UI: Typographer = Typographer::new(Language::RU_RU, RuleSet::INTERFACE);
//!
//! assert_eq!(RU_UI.typeset("Файл не найден", Output::Unicode), "Файл не\u{a0}найден");
//! assert_eq!(
//!     RU_UI.typeset("Он сказал \"привет\" - и ушёл...", Output::Unicode),
//!     "Он сказал «привет»\u{a0}— и\u{a0}ушёл…"
//! );
//! assert_eq!(RU_UI.format_number(-12345, Output::Unicode).to_string(), "\u{2212}12\u{a0}345");
//! ```

#![forbid(unsafe_code)]

mod engine;
mod error;
mod lang;
mod language;
mod number;
mod output;
mod piece;
mod report;
mod rule;
mod rules;
mod typographer;

// The generators of `tests/common/strategies.rs` name the crate `typolish` (design section 13.5,
// property 8).
#[cfg(test)]
extern crate self as typolish;

// The examples of the README run as doctests (design section 13.10).
#[cfg(doctest)]
#[doc = include_str!("../README.md")]
struct ReadmeDoctests;

pub use engine::scratch::Scratch;
pub use error::Error;
pub use lang::data::{
    Abbreviation, Bind, DashData, DashStyle, EllipsisForms, LanguageData, MonthPosition,
    NumberData, QuotePairs, Script,
};
pub use language::{Language, LanguageBuilder};
pub use number::{Number, NumberDisplay, RangeDisplay};
pub use output::{EscapeStyle, HtmlStyle, Output};
pub use piece::{Piece, Slot, SlotKind, Template, TemplatePart};
pub use report::{
    Change, Diagnostic, DiagnosticKind, ProtectedKind, ProtectedSpan, Report, Severity, TechKind,
};
pub use rule::{Rule, RuleSet};
pub use typographer::{NarrowGlue, PercentSpacing, SingleQuotes, Typographer, TypographerBuilder};

/// Compile-time checks of the commitments of design section 2.13.
const _: () = {
    const fn send_sync<T: Send + Sync + 'static>() {}
    send_sync::<Typographer>();
    send_sync::<Scratch>();
    send_sync::<Language>();
    send_sync::<LanguageBuilder>();
    send_sync::<RuleSet>();
};
