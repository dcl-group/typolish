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
as well. Found by the structured grammar of design section 13.5. The input needs no typed `‘`: in
en-GB the first pass writes a straight `"` as `‘`, and `"s' y’ "" s"` gives `‘s' y’ “” s’`, then
`‘s’ y’ “” s’` in every `SingleQuotes` mode (0.1.1 gave `‘s’ y’ “” s’` at once). Listed in design
section 10.8, the README and `limits()` of `tests/documentation.rs`.

The closer moves for a second reason too: a typed `’` with a closing role inside a straight pair is
covered by the `"` entry on the first pass (design section 6.4) and becomes an apostrophe, so the
pair closes at its straight closer, which follows `s`, and C54 keeps the possessive straight; on the
second pass the pair is typed, the `’` closes it early, and the possessive is placed. The class is
in built-in en-US. Prose, Interface and `ALL`, every `SingleQuotes` mode unless the row says
otherwise:

| Language                             | Input                                               | First pass                                          | Second pass                                         |
| ------------------------------------ | --------------------------------------------------- | --------------------------------------------------- | --------------------------------------------------- |
| en-US                                | `‘S' "x’" s’`                                       | `‘S' ‘x’’ s’`                                       | `‘S’ ‘x’’ s’`                                       |
| en-US                                | `"He said "the boys’ dogs' "x" bowls" were empty."` | `“He said ‘the⍽boys’ dogs' “x” bowls’ were empty.”` | `“He said ‘the⍽boys’ dogs’ “x” bowls’ were empty.”` |
| en-GB, `NestedOnly` and `Everywhere` | `She said "the boys’ dogs' 'bowls" were empty.`     | `She said ‘the⍽boys’ dogs' 'bowls’ were empty.`     | `She said ‘the⍽boys’ dogs’ 'bowls’ were empty.`     |
| en-GB, Prose, `NestedOnly`           | `"a’ s' 's"`                                        | `‘a’ s' 's’`                                        | `‘a’ s’ 's’`                                        |

0.1.1 gave the second pass at once on every row (P2 fails on the en-US rows in 0.1.1 as well). Found
by review with a generator of nested typed and straight quotes: 22 distinct inputs out of 4,000, in
48 configurations, that 0.1.1 gave at once.

## Analysis so far

The possessives are decided from the end of the window, so the possessive before a placed one (the
first row) sees the final mark of the later one, and could read the closer that a second pass would
see: the old closer of the run inside its pair. The possessive after a placed one (the second row)
is decided before it; there the earlier possessive could stay straight when a possessive kept
straight by C54 stands between its old closer and the closer of its run. Neither was tried.

For the covered `’`, one rule was tried and not kept: place the possessive (as 0.1.1 did) when a `’`
that a second pass may read as a closer stands inside the run (a covered typed `’`, or a straight
`'` placed as an apostrophe). On the generator of the review it removed 99 failures of idempotence
and added 141 new ones and 1,335 new failures of P2, because the possessives placed on the first
pass are such `’` on the second and change the decision there, and because `to_plain` then reads the
placed possessive as the closer. With the typed `’` alone it added 1,023 failures of idempotence on
plain input. A rule that reads the same on both passes has to know, on the first pass, which typed
`’` a second pass pairs.

## Goal

The first pass is a fixed point on these inputs, P2 holds where it held, and the output on the
corpus and on the tables of `tests/` stays the same.

## Acceptance

- The inputs above are in `SEEDS` of `tests/properties.rs` (the en-GB rows in the idempotence test
  with a built language).
- The limit is removed from design section 10.8, the README and `limits()` of
  `tests/documentation.rs`, and the filter `OPENER_REPAIRED` from `grammar_known_failure` (with its
  rows in `the_grammar_filters_name_their_limits`) once `todo/quotes-neutral-opener-repaired.md` is
  closed as well.
