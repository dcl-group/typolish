//! Freezing and edit refusal (design section 3.7): a rule gets no mutable access to tokens; it
//! returns a proposal, and the engine applies it only if no affected token is `FROZEN` and no
//! insertion falls inside a frozen fragment. The check sits here, in one place, so protection does
//! not depend on the care of each rule. The phases that call it (design section 3.8) arrive with
//! the rules of stage 2.

#![cfg_attr(
    not(test),
    expect(dead_code, reason = "the phases of stage 2 apply proposals")
)]

use super::lex::{Glue, Repl, Tok};

/// The most edits one proposal holds; `nbsp.phone` glues up to five spaces.
pub(crate) const MAX_EDITS: usize = 6;

/// A proposal of a rule at an anchor token.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Match {
    /// Tokens from the anchor that are closed to the other rules of the phase.
    pub(crate) consume: u8,
    /// At most `MAX_EDITS` edits inside `anchor..=anchor + consume`.
    pub(crate) edits: Edits,
}

/// One edit, by token offset from the anchor.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Edit {
    /// New text of a token; the kind stays.
    Replace { at: u8, repl: Repl },
    /// Set the glue mark of a `Space` token.
    Glue { at: u8, glue: Glue },
    /// Insert a `Space` token carrying glue before the token at this offset.
    Insert { before: u8, glue: Glue },
    /// Remove a `Space` token.
    Remove { at: u8 },
}

/// The edits of a proposal, without allocation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Edits {
    len: u8,
    items: [Edit; MAX_EDITS],
}

impl Edits {
    /// No edit: a pattern already in its target form (design section 3.8, item 3).
    pub(crate) const NONE: Edits = Edits {
        len: 0,
        items: [Edit::Remove { at: 0 }; MAX_EDITS],
    };

    /// The edits with one more; a seventh edit is dropped, which a rule never proposes.
    pub(crate) fn with(mut self, edit: Edit) -> Edits {
        if let Some(slot) = self.items.get_mut(self.len as usize) {
            *slot = edit;
            self.len += 1;
        }
        self
    }

    /// The edits in order.
    pub(crate) fn iter(&self) -> impl Iterator<Item = &Edit> {
        self.items.iter().take(self.len as usize)
    }
}

/// Whether the engine applies a proposal at `anchor` (design section 3.7): every affected token
/// exists, lies within the taken range and is not `FROZEN`, and no insertion stands next to a
/// frozen token of the call's text (the inside of a frozen fragment, the hyphen of a compound
/// word or a frozen space). An insertion next to a slot or protected text, which are frozen but
/// opaque, is allowed: it falls outside them.
pub(crate) fn edits_allowed(tokens: &[Tok], anchor: usize, m: &Match) -> bool {
    let last = anchor + m.consume as usize;
    let editable = |at: u8| {
        let i = anchor + at as usize;
        i <= last && tokens.get(i).is_some_and(|t| !t.is_frozen())
    };
    let open_side = |i: Option<usize>| match i.and_then(|i| tokens.get(i)) {
        Some(t) => !t.is_frozen() || t.kind.is_opaque(),
        None => true,
    };
    if tokens.get(anchor).is_none_or(Tok::is_frozen) {
        return false;
    }
    m.edits.iter().all(|edit| match *edit {
        Edit::Replace { at, .. } | Edit::Glue { at, .. } | Edit::Remove { at } => editable(at),
        Edit::Insert { before, .. } => {
            let right = anchor + before as usize;
            right <= last + 1
                && open_side((anchor + before as usize).checked_sub(1))
                && open_side(Some(right))
        }
    })
}

#[cfg(test)]
mod tests {
    use super::super::ingest::Atom;
    use super::super::lex::{Flags, Kind, lex_window};
    use super::*;

    fn tokens(text: &str, frozen: &[usize]) -> Vec<Tok> {
        let atoms = [Atom::Text {
            start: 0,
            end: u32::try_from(text.len()).unwrap(),
        }];
        let mut buf = Vec::new();
        let mut out = Vec::new();
        lex_window(text, &atoms, &mut buf, &mut out);
        for i in frozen {
            out[*i].flags = out[*i].flags.with(Flags::FROZEN);
        }
        out
    }

    fn one(edit: Edit, consume: u8) -> Match {
        Match {
            consume,
            edits: Edits::NONE.with(edit),
        }
    }

    #[test]
    fn edits_of_frozen_tokens_are_refused() {
        // `в 10.0.0.0/8`: the space may be glued, the fragment may not change.
        let t = tokens("в 10.0.0.0/8", &[2, 3, 4, 5, 6, 7, 8, 9, 10]);
        assert!(edits_allowed(
            &t,
            0,
            &one(
                Edit::Glue {
                    at: 1,
                    glue: Glue::NoBreak
                },
                1
            )
        ));
        assert!(!edits_allowed(
            &t,
            0,
            &one(
                Edit::Replace {
                    at: 2,
                    repl: Repl::Char('x')
                },
                2
            )
        ));
        assert!(!edits_allowed(
            &t,
            2,
            &Match {
                consume: 0,
                edits: Edits::NONE
            }
        ));
        // An edit outside the taken range is refused.
        assert!(!edits_allowed(
            &t,
            0,
            &one(
                Edit::Glue {
                    at: 1,
                    glue: Glue::NoBreak
                },
                0
            )
        ));
        // A whole proposal is dropped when one of its edits is refused.
        let both = Match {
            consume: 2,
            edits: Edits::NONE
                .with(Edit::Glue {
                    at: 1,
                    glue: Glue::NoBreak,
                })
                .with(Edit::Remove { at: 2 }),
        };
        assert!(!edits_allowed(&t, 0, &both));
    }

    #[test]
    fn insertions_inside_a_frozen_fragment_are_refused() {
        let t = tokens("a.b", &[0, 1, 2]);
        assert!(!edits_allowed(
            &t,
            0,
            &one(
                Edit::Insert {
                    before: 1,
                    glue: Glue::NoBreak
                },
                1
            )
        ));
        // `т.е` unfrozen: the insertion is allowed.
        let t = tokens("т.е", &[]);
        assert!(edits_allowed(
            &t,
            0,
            &one(
                Edit::Insert {
                    before: 2,
                    glue: Glue::NoBreak
                },
                2
            )
        ));
        // Next to an opaque token the insertion falls outside it.
        let mut t = tokens("№", &[]);
        let mut slot = t[0];
        slot.kind = Kind::Slot(crate::SlotKind::Number);
        slot.flags = Flags::FROZEN;
        t.push(slot);
        assert!(edits_allowed(
            &t,
            0,
            &one(
                Edit::Insert {
                    before: 1,
                    glue: Glue::NoBreak
                },
                1
            )
        ));
        assert_eq!(Edits::NONE.with(Edit::Remove { at: 0 }).iter().count(), 1);
    }
}
