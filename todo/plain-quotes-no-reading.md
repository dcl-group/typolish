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

The structured grammar of design section 13.5 found a third form, also older than 0.1.2: a reading
that brings the quotes back but not a rule that reads them. In a declared language whose primary
pair is `‘’` (en-GB), Prose and Interface, `SingleQuotes::Never`:

| Input            | Typeset          | `to_plain`       | Typeset again    |
| ---------------- | ---------------- | ---------------- | ---------------- |
| `"dogs' - 'tis"` | `‘dogs’ - ’tis’` | `"dogs" - 'tis'` | `‘dogs’ — ’tis’` |

`to_plain` reads the first `’` after `s` as the closer, since the closer follows `s` too (design
section 12.1); the quotes come back, and the dash between them is placed. P2 over the grammar skips
a failure where `to_plain` read a `"` of the input as `'` or the other way round
(`grammar_known_p2_failure` in `tests/common/strategies.rs`, named `NO_READING`); that filter goes
with this task. The plain writing of the grammar finds the class with labels and kept apostrophes
too: en-GB, Prose, `""dogs'" word '1920x1080'"` gives `‘“dogs’” word “1920×1080”’`, whose plain form
`""dogs’" word "1920x1080""` gives `""dogs’" word "1920×1080""`.

A misreading of the quotes can also change what `to_plain` writes for a glued space: in English with
`SingleQuotes::Everywhere`, Prose, `'-" - §5 '" "" dogs' word "v1.2.3"'"'` gives
`“-" - §⍽5 '" ‘’ dogs” word “v1.2.3″'”'`, whose plain form `"-" - § 5 '" "" dogs" word "v1.2.3″'"'`
writes `§⍽5` as `§ 5`; and `'(" - §5 "... - "")'` gives `“(" - §⍽5 "… — ‘’)”`, plain form
`"(" - § 5 "... - "")"`. With the quotes of the input back, each plain form typesets to the output,
so the filter names them (found by the long run of P2 over the grammar with other seeds).

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
  and the exception of P2 in design section 12.2, and the filter `NO_READING` from
  `grammar_known_p2_failure` in `tests/common/strategies.rs` (with its rows in
  `the_p2_filters_name_their_limits` of `tests/roundtrip.rs`).
