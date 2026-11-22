//! Typesetting of plain text for a language.
//!
//! The crate turns text typed the easy way (straight quotes, hyphens, three full stops, ordinary
//! spaces) into typeset text: quotes by nesting level, apostrophes, dashes, ellipses, no-break
//! spaces, and rules for words, signs and numbers. A [`Typographer`] is built once with a
//! [`Language`] and a [`RuleSet`] and then only applied; the [`Output`] kind is given with every
//! call. Its design is in `docs/design/` of the repository.
//!
//! The crate is in development: the engine recognizes protected and technical fragments and
//! applies the rules of quotes, dashes, ranges and ellipses; [`Typographer::report`] lists every edit,
//! protected span and diagnostic. The other rules (no-break spaces, numbers, symbols) change
//! nothing yet.
//!
//! ```
//! use typolish::{Language, Output, RuleSet, Typographer};
//!
//! static RU_UI: Typographer = Typographer::new(Language::RU_RU, RuleSet::INTERFACE);
//!
//! assert_eq!(RU_UI.typeset("Файл не найден", Output::Unicode), "Файл не найден");
//! assert_eq!(
//!     RU_UI.typeset("Он сказал \"привет\" - и ушёл...", Output::Unicode),
//!     "Он сказал «привет» — и ушёл…"
//! );
//! ```

#![forbid(unsafe_code)]

mod engine;
mod error;
mod lang;
mod language;
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

pub use engine::scratch::Scratch;
pub use error::Error;
pub use lang::data::{
    Abbreviation, Bind, DashData, DashStyle, EllipsisForms, LanguageData, MonthPosition,
    NumberData, QuotePairs, Script,
};
pub use language::Language;
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
    send_sync::<RuleSet>();
};
