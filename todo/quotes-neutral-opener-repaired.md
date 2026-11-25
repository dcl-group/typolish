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

The structured grammar of design section 13.5 found two more causes of the neutral `‘`. In English
(en-US and en-GB) a straight `"` opener whose closer is a weak quote after a digit (design section
6.5) stands above the `‘` when its typed `’` comes; pass 1 ignores weak quotes, so the `’` finds the
`"` entry on top and the `‘` stays neutral. Older than 0.1.2. Prose and Interface:

| Language and mode | Input                     | First pass                | Second pass               |
| ----------------- | ------------------------- | ------------------------- | ------------------------- |
| en-US, every mode | `He said ‘the dogs' "5"’` | `He said ‘the⍽dogs' “5”’` | `He said ‘the⍽dogs’ “5”’` |
| en-US, every mode | `‘s' "0"’`                | `‘s' “0”’`                | `‘s’ “0”’`                |
| en-US, every mode | `‘"0" S'’`                | `‘“0” S'’`                | `‘“0” S’’`                |
| en-GB, every mode | `‘"8"’ s'`                | `‘‘8’’ s'`                | `‘‘8’’ s’`                |

The same neutral `‘`, paired on the second pass, also encloses a straight `'` pair that the first
pass cancelled as a top-level pair (`SingleQuotes::NestedOnly`, design section 6.8), which then
pairs; section 6.11 states that a re-pairing cannot enclose such a pair. Older than 0.1.2. English,
Prose and Interface, `NestedOnly`: `x ‘a 'b' "2"’ y` gives `x ‘a⍽'b' “2”’ y`, then
`x ‘a⍽‘b’ “2”’ y`; en-GB: `‘ '' "2"’` gives `‘ '' ‘2’’`, then `‘ “” ‘2’’`.

The properties over the grammar skip a failure in `en-US` and `en-GB` whose first pass holds a `‘`
before a straight `'` and whose second pass changes straight `'` only (`OPENER_REPAIRED` of
`grammar_known_failure` in `tests/common/strategies.rs`); that filter goes with this task and with
`todo/quotes-possessive-run.md`.

## Goal

The first pass is a fixed point on these inputs, while the output of the first pass on the corpus
and on the tables of `tests/` stays the same.

## Acceptance

- The inputs above run in an idempotence test with the declared pairs, and the English rows in
  `SEEDS` of `tests/properties.rs`.
- The limit is removed from design section 10.8, the README and `limits()` of
  `tests/documentation.rs`, and the filter `OPENER_REPAIRED` from `grammar_known_failure` (with its
  rows in `the_grammar_filters_name_their_limits`) once `todo/quotes-possessive-run.md` is closed as
  well.
