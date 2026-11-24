# A neutral `‘` that a second pass pairs lets a possessive through

## Problem

In a declared language whose primary pair is `‘’` (en-GB), a typed `‘` that stays neutral, followed
by a straight pair around a typed `’` and a trailing possessive, gives an output that a second pass
changes again (design section 6.11 claims a fixed point for the quote phase). Prose and Interface,
every `SingleQuotes` mode for the first two rows, `SingleQuotes::Never` for the third:

| Input          | First pass     | Second pass    |
| -------------- | -------------- | -------------- |
| `‘ "dogs'’" x` | `‘ ‘dogs'’’ x` | `‘ ‘dogs’’’ x` |
| `‘"s'»’".`     | `‘‘s'»’’.`     | `‘‘s’»’’.`     |
| `‘"'s'»’''". ` | `‘‘'s'»’''’. ` | `‘‘'s’»’''’. ` |

On the first pass the typed `’` inside the straight pair is covered by the `"` entry and becomes an
apostrophe, the typed `‘` stays neutral, and section 6.6 keeps `dogs'` straight because a neutral
`‘` stands before it. On the second pass the straight pair is `‘’`: the typed `’` closes the inner
`‘`, the new `’` closes the `‘` that was neutral, no neutral `‘` is left, and the possessive is
placed. P2 fails on the first row too: `to_plain` gives `" "dogs'"" x`, which does not converge.

The case is older than 0.1.2 (the output of 0.1.1 is the same). In en-US, whose nested pair is `‘’`,
the same input inside a straight `"` pair is a fixed point. Found by a random generator over typed
and straight quotes after the fixes of 0.1.2; listed in design section 10.8 and the README.

## Goal

The first pass is a fixed point on these inputs, while the output of the first pass on the corpus
and on the tables of `tests/` stays the same.

## Acceptance

- The inputs above run in an idempotence test with the declared pairs.
- The limit is removed from design section 10.8 and from the README.
