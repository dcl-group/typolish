//! Ingest: input pieces become a stream of atoms (design section 3.4).
//!
//! Atoms, like tokens, hold positions and no references, so the buffers that keep them are reused
//! across calls for inputs of any lifetime (design section 2.12).

use core::ops::Range;

use super::Input;
use crate::Slot;

/// One atom of the input: a byte range of an input piece, or a slot.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum Atom {
    /// Text to typeset: a byte range in the input piece.
    Text { piece: u32, range: Range<u32> },
    /// An opaque placeholder.
    Slot { piece: u32, slot: Slot },
    /// Protected text: a protected piece, or a span cut out by delimiters (`delimited`).
    Protected {
        piece: u32,
        range: Range<u32>,
        delimited: bool,
    },
    /// Invisible markup.
    Markup { piece: u32, range: Range<u32> },
}

/// Turns the input into atoms, one per non-empty piece. Returns `false`, leaving `atoms` empty,
/// when a piece index or a piece length does not fit the `u32` positions of atoms and tokens; the
/// caller then returns the input unchanged (design section 3.4).
///
/// The cutting of fenced blocks and of the other protection delimiters (design section 10.1)
/// arrives with the lexer.
pub(crate) fn ingest(input: &[Input<'_>], atoms: &mut Vec<Atom>) -> bool {
    atoms.clear();
    for (index, item) in input.iter().enumerate() {
        let Ok(piece) = u32::try_from(index) else {
            atoms.clear();
            return false;
        };
        let atom = match *item {
            Input::Slot(slot) => Some(Atom::Slot { piece, slot }),
            Input::Text(text) | Input::Protected(text) | Input::Markup(text) => {
                let Ok(end) = u32::try_from(text.len()) else {
                    atoms.clear();
                    return false;
                };
                let range = 0..end;
                match *item {
                    _ if end == 0 => None,
                    Input::Text(_) => Some(Atom::Text { piece, range }),
                    Input::Protected(_) => Some(Atom::Protected {
                        piece,
                        range,
                        delimited: false,
                    }),
                    _ => Some(Atom::Markup { piece, range }),
                }
            }
        };
        atoms.extend(atom);
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn one_atom_per_non_empty_piece() {
        let input = [
            Input::Text("ab"),
            Input::Text(""),
            Input::Slot(Slot::NUMBER),
            Input::Protected("`x`"),
            Input::Markup("<b>"),
        ];
        let mut atoms = Vec::new();
        assert!(ingest(&input, &mut atoms));
        assert_eq!(
            atoms,
            [
                Atom::Text {
                    piece: 0,
                    range: 0..2
                },
                Atom::Slot {
                    piece: 2,
                    slot: Slot::NUMBER
                },
                Atom::Protected {
                    piece: 3,
                    range: 0..3,
                    delimited: false
                },
                Atom::Markup {
                    piece: 4,
                    range: 0..3
                },
            ]
        );
    }
}
