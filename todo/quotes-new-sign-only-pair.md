# A new `‘` before signs and a typed `’` is a sign-only pair only on the second pass

## Problem

Choice C55 (0.1.2) lets a typed `’` close the `‘` of its pair as the second quote of a sign-only
pair (design section 6.3), so that the `‘’` a first pass writes for `""` reads as a pair again. A
straight quote that the first pass writes as `‘` (a pair of its level whose glyph is `‘`) before
signs, or nothing, and a typed `’` is no sign-only pair on the first pass, where the straight quote
pairs with another one, but it is on the second, where both are typed. The second pass then pairs
the quotes in another way, and a rule that reads the role of the `’` decides differently (design
section 6.11 claims a fixed point):

| Language and set                    | Input            | First pass       | Second pass      |
| ----------------------------------- | ---------------- | ---------------- | ---------------- |
| en-US, Prose/Interface, not `Never` | `"a '-’ - x' c"` | `“a⍽‘-’ - x’ c”` | `“a⍽‘-’ — x’ c”` |
| en-GB, Prose                        | `"-’ - x"`       | `‘-’ - x’`       | `‘-’ — x’`       |
| en-GB, Prose, `Everywhere`          | `‘'—’ l' s'`     | `‘‘—’ l’ s'`     | `‘‘—’ l’ s’`     |

In the first two rows the `’` closes the sign-only pair on the second pass and `dash.spaced-hyphen`
reads it as a closing quote before the dash; in the third the outer `‘` pairs on the second pass and
the trailing possessive after it is placed. 0.1.1 gave the first pass on both passes: the defect
came with C55 (commit `fce23b9`). Found by the structured grammar of design section 13.5; listed in
design section 10.8, the README and `limits()` of `tests/documentation.rs`. The properties over the
grammar skip every input in `en-US` and `en-GB` with a straight quote, then signs or nothing, then
`’` (`grammar_known_limit` in `tests/common/strategies.rs`).

## Analysis so far

Two directions were seen and neither was tried: the first pass reads a straight quote, signs and a
typed `’` as the sign-only pair the second pass sees (then the straight quote is no opener of
another pair, and the window may not converge where it converged before), or C55 applies only where
the `‘` cannot be the opener that a first pass wrote for a pair of straight quotes, which the second
pass cannot tell from a typed one.

## Goal

The first pass is a fixed point on these inputs, the rows of C55 (`'a ""-3'` in en-GB) stay fixed
points, and the output on the corpus and on the tables of `tests/` stays the same.

## Acceptance

- The inputs above are in `SEEDS` of `tests/properties.rs` and in the idempotence test with the
  declared pairs.
- The limit is removed from design section 10.8, the README and `limits()` of
  `tests/documentation.rs`, and its filter from `grammar_known_limit`.
