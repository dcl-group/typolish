//! Windows (design section 3.3): a window is a paragraph. Its boundary is two or more line breaks
//! with only horizontal whitespace between them, or U+2029 PARAGRAPH SEPARATOR. Atoms other than
//! text (fenced blocks, slots, protected pieces, markup) are content, so a boundary lies inside
//! one `Text` atom. The characters of a boundary belong to no window; horizontal whitespace before
//! the first or after the last line break of a boundary belongs to the window on that side.

use super::ingest::Atom;

/// A line break: `\n`, `\r`, U+0085, U+2028 (`\r\n` is one, design section 5.3). U+2029 is a
/// paragraph separator, not a line break.
pub(crate) const fn is_line_break(c: char) -> bool {
    matches!(c, '\n' | '\r' | '\u{85}' | '\u{2028}')
}

/// Horizontal whitespace: `char::is_whitespace`, except line breaks and U+2029 (design section
/// 3.5).
pub(crate) fn is_horizontal_space(c: char) -> bool {
    c.is_whitespace() && !is_line_break(c) && c != '\u{2029}'
}

/// Splits the `Text` atoms at window boundaries, which become `Boundary` atoms.
pub(crate) fn split_windows(text: &str, atoms: &[Atom], out: &mut Vec<Atom>) {
    out.clear();
    for atom in atoms {
        let Atom::Text { start, end } = *atom else {
            out.push(*atom);
            continue;
        };
        let mut cur = start;
        let mut scan = start as usize;
        let end_usize = end as usize;
        while let Some((at, next)) = find_boundary(text, scan, end_usize) {
            let (Ok(at32), Ok(next32)) = (u32::try_from(at), u32::try_from(next)) else {
                break;
            };
            if at32 > cur {
                out.push(Atom::Text {
                    start: cur,
                    end: at32,
                });
            }
            out.push(Atom::Boundary {
                start: at32,
                end: next32,
            });
            cur = next32;
            scan = next;
        }
        if cur < end {
            out.push(Atom::Text { start: cur, end });
        }
    }
}

/// The next boundary in `text[from..end]`: the range from its first to its last line break or
/// paragraph separator.
fn find_boundary(text: &str, from: usize, end: usize) -> Option<(usize, usize)> {
    let mut i = from;
    while i < end {
        let c = text.get(i..end)?.chars().next()?;
        if !(is_line_break(c) || c == '\u{2029}') {
            i += c.len_utf8();
            continue;
        }
        // A group of line breaks with horizontal whitespace between them.
        let start = i;
        let mut breaks = 0usize;
        let mut separator = false;
        let mut last = i;
        let mut j = i;
        while let Some(c) = text.get(j..end).and_then(|r| r.chars().next()) {
            if is_line_break(c) || c == '\u{2029}' {
                if c == '\u{2029}' {
                    separator = true;
                } else {
                    breaks += 1;
                }
                j += c.len_utf8();
                if c == '\r' && text.as_bytes().get(j) == Some(&b'\n') && j < end {
                    j += 1;
                }
                last = j;
            } else if is_horizontal_space(c) {
                j += c.len_utf8();
            } else {
                break;
            }
        }
        if separator || breaks >= 2 {
            return Some((start, last));
        }
        i = last;
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    fn boundaries(text: &str) -> Vec<&str> {
        let atoms = [Atom::Text {
            start: 0,
            end: u32::try_from(text.len()).unwrap(),
        }];
        let mut out = Vec::new();
        split_windows(text, &atoms, &mut out);
        out.iter()
            .filter_map(|a| match *a {
                Atom::Boundary { start, end } => Some(&text[start as usize..end as usize]),
                _ => None,
            })
            .collect()
    }

    #[test]
    fn boundaries_are_found() {
        assert_eq!(boundaries("a\n\nb"), ["\n\n"]);
        assert_eq!(boundaries("a \n \t \n b"), ["\n \t \n"]);
        assert_eq!(boundaries("a\r\n\r\nb"), ["\r\n\r\n"]);
        assert_eq!(boundaries("a\r\nb\nc"), Vec::<&str>::new());
        assert_eq!(boundaries("a \u{2029} b"), ["\u{2029}"]);
        assert_eq!(boundaries("a\n\u{2029}\nb"), ["\n\u{2029}\n"]);
        assert_eq!(boundaries("a\n.\nb"), Vec::<&str>::new());
        assert_eq!(boundaries("\n\n"), ["\n\n"]);
        assert_eq!(boundaries("a\nb\n\nc\n\n\nd"), ["\n\n", "\n\n\n"]);
    }

    #[test]
    fn content_atoms_break_a_boundary() {
        let text = "a\n\nb";
        let atoms = [
            Atom::Text { start: 0, end: 2 },
            Atom::Markup { piece: 1, at: 2 },
            Atom::Text { start: 2, end: 4 },
        ];
        let mut out = Vec::new();
        split_windows(text, &atoms, &mut out);
        assert_eq!(out, atoms);
    }
}
