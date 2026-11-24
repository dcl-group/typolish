# `to_plain` finds no reading that brings some quote windows back

## Problem

P2 (design section 12.2) fails on plain input whose typeset text is a fixed point: `to_plain` reads
the glyphs of a pair as `"`, or as `'` where that does not pair again (design section 12.1, choices
C32 and C45), and in these windows no reading it tries brings the quotes back.

| Language and set         | Input               | Typeset             | `to_plain`          | Typeset again       |
| ------------------------ | ------------------- | ------------------- | ------------------- | ------------------- |
| `ru-RU`, Prose/Interface | `"a '(" x' "y'" b"` | `«a „(" x“ „y'“ b»` | `"a "(" x" "y'" b"` | `"a "(" x" "y'" b"` |
| `en-US`, Prose/Interface | `"'S '5' 3'"`       | `“‘S “5” 3’”`       | `""S "5" 3""`       | `""S "5" 3""`       |
| `en-US`, Prose/Interface | `"5 ''3' 5' s"`     | `“5 ‘“3” 5’ s”`     | `"5 ""3" 5" s"`     | `"5 ""3" 5" s"`     |
| `en-US`, `Everywhere`    | `'(" s' "s'"`       | `“(" s” “s’”`       | `"(" s" "s’"`       | `"(" s" "s’"`       |

Two causes are seen so far. A straight `"` without a role inside a pair (design section 6.3) forms a
sign-only pair with the quote of the pair once that quote reads as `"` (`"("`). A weak quote after a
digit (design section 6.5) that closes a pair reads differently once the pair reads as `"`: `3"`
waits for pass 2 where `3'` closed the `'` pair.

The defect is older than 0.1.2: 0.1.1 fails on the same inputs. Found by a structured generator of
nested straight quotes, words ending in `s`, short numbers and stray signs; on 240,000 such plain
strings in `en-US`, `ru-RU` and a declared language whose primary pair is `‘’`, with Prose,
Interface and `ALL` and every `SingleQuotes` mode, P2 failed for 175 inputs (all with a digit next
to a quote); on 120,000 strings without digits and with signs next to quotes, for 77 inputs, most of
them only with `SingleQuotes::Everywhere`. It is listed in design section 10.8, the README and
`limits()` of `tests/documentation.rs`, and
`windows_whose_plain_form_does_not_typeset_back_are_listed` in `tests/roundtrip.rs` pins the first
two rows.

## Analysis so far

Adding the tries "nested and deep glyphs as `'`" and "nested, primary and deep glyphs as `'`" to
`settle_quotes` removed only about one failure in six on the generator above. The readings are
chosen per tier for a whole window; the rows above need a reading per pair (`'` for one pair, `"`
for its neighbor), or a reading that keeps a role-less `"` from pairing.

## Goal

P2 holds on these inputs, with `to_plain` linear in the length of the text, and the canonical form
(P1) and the corpus snapshots unchanged.

## Acceptance

- The rows above run as P2 tests in `tests/roundtrip.rs`.
- The limit is removed from design section 10.8, the README, `limits()` of `tests/documentation.rs`
  and the exception of P2 in design section 12.2.
