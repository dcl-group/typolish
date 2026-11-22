//! `ellipsis.*` (design section 8.6): the trigger bytes so far.

use super::Triggers;

/// `ellipsis.dots`: runs of full stops; typed quotes and dashes are context.
pub(crate) const DOTS: Triggers = Triggers::of(b".").union(Triggers::TYPOGRAPHIC);
/// `ellipsis.inner`: runs of full stops.
pub(crate) const INNER: Triggers = Triggers::of(b".");
