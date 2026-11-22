//! `nbsp.*` (design section 8.7): the trigger bytes so far. Every no-break rule glues a space,
//! so the space is a trigger byte of each.

use super::Triggers;

const SPACE: Triggers = Triggers::of(b" ");

/// `nbsp.abbreviations`: the space inside `т.е.` is inserted.
pub(crate) const ABBREVIATIONS: Triggers = SPACE.union(Triggers::of(b"."));
/// `nbsp.initials`.
pub(crate) const INITIALS: Triggers = SPACE;
/// `nbsp.dates`.
pub(crate) const DATES: Triggers = SPACE;
/// `nbsp.units`.
pub(crate) const UNITS: Triggers = SPACE;
/// `nbsp.units` when `PercentSpacing` is not `AsWritten`: `%` and the lead byte of `‰`.
pub(crate) const PERCENT: Triggers = Triggers::of(b"%").with_lead('‰');
/// `nbsp.signs`; the lead bytes of the language's signs are added (`№5` gets a space).
pub(crate) const SIGNS: Triggers = SPACE;
/// `nbsp.phone`.
pub(crate) const PHONE: Triggers = SPACE;
/// `nbsp.short-words`.
pub(crate) const SHORT_WORDS: Triggers = SPACE;
/// `nbsp.short-length`.
pub(crate) const SHORT_LENGTH: Triggers = SPACE;
/// `nbsp.particles`.
pub(crate) const PARTICLES: Triggers = SPACE;
/// `nbsp.dash`: the space before a dash; typed dashes are context.
pub(crate) const DASH: Triggers = SPACE.union(Triggers::TYPOGRAPHIC);
