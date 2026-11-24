# A sign-only pair of `‘’` before a number is not idempotent

## Problem

In a declared language whose primary pair is `‘’` (en-US data with
`QuotePairs::new(('‘', '’'), ('“', '”'))`, an en-GB), Prose, a sign-only pair inside a top-level
straight single pair before a number gives an output that a second pass changes again (design
section 6.11 claims a fixed point for the quote phase):

| Input      | First pass | Second pass |
| ---------- | ---------- | ----------- |
| `'a ""-3'` | `'a⍽‘’-3'` | `'a⍽‘’-3′`  |
| `'a "" 3'` | `'a⍽‘’ 3'` | `'a⍽‘’ 3′`  |

On the first pass the weak `'` after `3` pairs in pass 2 (design section 6.5) with the opening `'`
that pass 1 left unpaired, and the single pair stays straight (`SingleQuotes::NestedOnly`). On the
second pass the typed `‘’` stands between them, and the weak `'` is read as a prime. The typed `’`
is U+2019, which the quote phase reads as an apostrophe or a closer; the case does not occur in
en-US, whose primary closer is `”` (`'a ""-3'` gives `'a⍽“”-3'` on both passes), nor in Interface,
which has no `quotes.primes`. The mechanism is not analyzed further yet. Found by the grammar-based
generator of the 0.1.1 review; the same output was given before that work.

## Goal

The first pass is a fixed point on these inputs in the declared language, while the output of the
first pass on the corpus and on the tables of `tests/` stays the same.

## Acceptance

- The inputs above run in an idempotence test with the declared pairs.
- The limit is removed from design section 10.8 and from the README.
