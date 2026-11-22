//! Typesetting of plain text for a language.
//!
//! The crate turns text typed the easy way (straight quotes, hyphens, three full stops, ordinary
//! spaces) into typeset text: quotes by nesting level, apostrophes, dashes, ellipses, no-break
//! spaces, and rules for words, signs and numbers. Its design is in `docs/design/` of the
//! repository.

#![forbid(unsafe_code)]

mod error;
mod rule;

pub use error::Error;
pub use rule::{Rule, RuleSet};
