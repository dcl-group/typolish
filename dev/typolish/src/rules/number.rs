//! `number.*` (design section 8.8): the trigger bytes so far.

use super::Triggers;

/// `number.minus`: a hyphen before a number.
pub(crate) const MINUS: Triggers = Triggers::of(b"-").union(Triggers::DIGITS);
/// `number.times`: `x` between numbers (the Cyrillic letters are found through the digits).
pub(crate) const TIMES: Triggers = Triggers::of(b"xX").union(Triggers::DIGITS);
