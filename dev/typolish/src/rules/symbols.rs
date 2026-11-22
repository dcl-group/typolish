//! `symbols.*` (design section 8.9): the trigger bytes so far.

use super::Triggers;

/// `symbols.copyright`: `(c)`, `(r)`, `(tm)`.
pub(crate) const COPYRIGHT: Triggers = Triggers::of(b"(");
/// `symbols.arrows`: `->`, `<-`, `<->`.
pub(crate) const ARROWS: Triggers = Triggers::of(b"-<>");
/// `symbols.math`: `>=`, `<=`, `!=`, `+-`.
pub(crate) const MATH: Triggers = Triggers::of(b"<>!+");
