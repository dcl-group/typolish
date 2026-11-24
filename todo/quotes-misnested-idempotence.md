# Straight quotes nested out of order are not idempotent

## Problem

In English, straight quotes nested out of order around a trailing possessive give an output that a
second pass changes again (design section 6.11 claims a fixed point for the quote phase):

| Set                      | Input         | First pass    | Second pass   |
| ------------------------ | ------------- | ------------- | ------------- |
| `en-US`, Prose/Interface | `""s' 'x."'"` | `“‘s’ 'x.’'”` | `“‘s’ ‘x.’’”` |
| `en-US`, Prose           | `“"s' '."'”`  | `“‘s’ '.’'”`  | `“‘s’ ‘.’’”`  |

The same class breaks P2 in a declared language whose primary pair is `‘’` (en-GB, Interface):
`it) - 'cause' - 'cause -- "'cause) DOGS' - 'house' - it, - oz.) '90s" a` is typeset as
`it) — 'cause' - ’cause -- ‘’cause) DOGS’ — “house” — it, — oz.) ’90s’ a`, and typesetting its
`to_plain` gives `… DOGS’ — ‘house’ — …`. The trailing possessive after a capital `S` (design
section 6.6) reaches the class through `DOGS'` too. The mechanism inside the quote phase is not
analyzed yet; the case is listed in design section 10.8 and in the README.

## Goal

The first pass is a fixed point on these inputs and P2 holds for the en-GB case, while the output of
the first pass on the corpus and on the tables of `tests/` stays the same.

## Acceptance

- The inputs above are in `SEEDS` of `tests/properties.rs`, and the en-GB case is a P2 row.
- The limit is removed from design section 10.8, from the README (both the known limit and the
  exception in the feature list) and from `limits()` of `tests/documentation.rs`.
- The grammar-based idempotence generator (its own task) enters the gate only after this one.
