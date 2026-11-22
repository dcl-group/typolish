//! `range.*` (design section 8.5): the trigger bytes so far.

use super::Triggers;

/// `range.placeholders`: a hyphen between two slots.
pub(crate) const PLACEHOLDERS: Triggers = Triggers::of(b"-");
/// `range.digits`: a hyphen between two numbers.
pub(crate) const DIGITS: Triggers = Triggers::of(b"-").union(Triggers::DIGITS);
/// `range.context`: a hyphen in a range with a context word.
pub(crate) const CONTEXT: Triggers = Triggers::of(b"-").union(Triggers::DIGITS);
/// `range.time`: a hyphen between two times.
pub(crate) const TIME: Triggers = Triggers::of(b"-").union(Triggers::DIGITS);
