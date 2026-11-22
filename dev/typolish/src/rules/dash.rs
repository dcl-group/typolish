//! `dash.*` (design section 8.4): the trigger bytes so far.

use super::Triggers;

/// `dash.dialogue`: a hyphen at a line start.
pub(crate) const DIALOGUE: Triggers = Triggers::of(b"-");
/// `dash.double-hyphen`: `--` and `---`.
pub(crate) const DOUBLE_HYPHEN: Triggers = Triggers::of(b"-");
/// `dash.spaced-hyphen`: a lone hyphen; typed dashes are matched with an empty edit.
pub(crate) const SPACED_HYPHEN: Triggers = Triggers::of(b"-").union(Triggers::TYPOGRAPHIC);
