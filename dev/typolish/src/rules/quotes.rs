//! `quotes.*` (design section 8.3): the trigger bytes so far.

use super::Triggers;

/// `quotes.apostrophe`: `'` between letters and after digits.
pub(crate) const APOSTROPHE: Triggers = Triggers::of(b"'");
/// `quotes.pairs`: straight quotes; typed quotes are context.
pub(crate) const PAIRS: Triggers = Triggers::of(b"\"'").union(Triggers::TYPOGRAPHIC);
/// `quotes.primes`: `'` and `"` after numbers.
pub(crate) const PRIMES: Triggers = Triggers::of(b"\"'").union(Triggers::DIGITS);
