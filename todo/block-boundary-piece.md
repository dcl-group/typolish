# A piece for a block boundary in a message template

## Problem

`Piece::Markup` is inline markup and transparent for rules (design sections 2.6 and 3.4), so a block
tag passed as `Markup` ends no paragraph. With
`[Text("Он сказал \"да"), Markup("</p><p>"), Text("Нет\" - ответила она.")]` the quotes pair across
the tag, and a dialogue dash right after it is not placed (design section 10.8,
`a_block_tag_passed_as_markup_ends_no_paragraph` in `tests/documentation.rs`). Today a splitter
makes one call per block, or ends the `Text` piece before a block tag with `\n\n`, which puts two
line breaks into the output.

## Goal

A variant of `Piece` for a block boundary: invisible markup written as is, like `Markup`, that also
ends the paragraph (the window of design section 3.3) and starts a line for `dash.dialogue`. `Piece`
is `#[non_exhaustive]`, so the variant can be added in a minor release (design section 2.13). The
choice is recorded next to C9 in design section 15.3.

## Acceptance

The four cases of `a_block_tag_passed_as_markup_ends_no_paragraph`, with the new variant in place of
`Markup` plus `\n\n`, give the paragraph-split results without the line breaks; the variant is in
the slot participation table of design section 2.6; the 10.8 item and the README limit are updated
in the same change.
