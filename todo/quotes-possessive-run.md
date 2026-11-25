# A possessive placed inside a typed pair moves the closer of the pairs around it

## Problem

A trailing possessive placed inside a typed pair that `’` closes is read by a second pass as the
closer of that pair, which closes early, and the old closer then closes the pair around it if `’`
closes that one too (the run of design section 6.6). Design section 6.11 shows that only a straight
`'` opener that the old closers popped could pair differently. Choice C54 (0.1.2) added a second
reason to keep a possessive straight: the closer of its pair follows `s` or `S` and an unpaired
straight `'` opener or a quote of a pair stands between them (for `to_plain`). That reason reads the
closer of the pair, which the run moves on the second pass, so a possessive kept straight by it on
the first pass is placed on the second. English, Prose and Interface:

| Mode                       | Input          | First pass     | Second pass    |
| -------------------------- | -------------- | -------------- | -------------- |
| every mode                 | `‘S' ‘s'’ s’`  | `‘S' ‘s’’ s’`  | `‘S’ ‘s’’ s’`  |
| `NestedOnly`, `Everywhere` | `‘"s'" s' 's’` | `‘‘s’’ s' 's’` | `‘‘s’’ s’ 's’` |

In the first row the inner `s'` is placed, `‘S'` keeps its `'` because the outer closer follows `s`
and the inner pair stands between; on the second pass the outer pair closes at the `’` after the
inner one, which follows no `s`, and `S'` is placed. In the second row the second `s'` keeps its `'`
because the outer closer follows `s` of `'s` and the opener `'` stands between; on the second pass
the outer pair closes before it, and it is placed. 0.1.1 gave the second pass at once on both rows.
A declared language whose primary pair is `‘’` (en-GB) fails on the first row in Prose and Interface
as well. Found by the structured grammar of design section 13.5; it found the class only with a
typed `‘` in the input. Listed in design section 10.8, the README and `limits()` of
`tests/documentation.rs`.

## Analysis so far

The possessives are decided from the end of the window, so the possessive before a placed one (the
first row) sees the final mark of the later one, and could read the closer that a second pass would
see: the old closer of the run inside its pair. The possessive after a placed one (the second row)
is decided before it; there the earlier possessive could stay straight when a possessive kept
straight by C54 stands between its old closer and the closer of its run. Neither was tried.

## Goal

The first pass is a fixed point on these inputs, P2 holds where it held, and the output on the
corpus and on the tables of `tests/` stays the same.

## Acceptance

- The inputs above are in `SEEDS` of `tests/properties.rs`.
- The limit is removed from design section 10.8, the README and `limits()` of
  `tests/documentation.rs`, and the filter of the typed `‘` from `grammar_known_limit` once
  `todo/quotes-neutral-opener-repaired.md` is closed as well.
