# I2 and I4 of the engine over the structured grammar

## Problem

`i2_and_i4_hold_on_the_classification_and_the_matches` in `src/engine/mod.rs` checks I2 and I4 of
design section 4.3 on the fragments and the matches themselves, with the stage 2 rules, but draws
its input from `any_text`. Run over the structured grammar of design section 13.5
(`strategies::grammar_case`, the known limits skipped, the `nbsp.*` rules off), it finds two classes
that the public properties do not see: the output of both is a fixed point and P2 holds on them.

I2, en-US and en-GB with `SingleQuotes::Everywhere`: an elision of the list closed by a straight `'`
that pairs (design section 6.2) gives fragments that differ from those of the pair it becomes.
en-GB, Prose: `! s: S -- 'tis'` gives `! s: S — ‘tis’`; en-US, Prose, `DashStyle::Closed`:
`'don't S a file rock'n'roll 'cause'` gives `'don’t S a file rock’n’roll “cause”`.

I4, en-US and en-GB: a dash match next to a straight `'` before a number that becomes `’` (a decade
apostrophe or a quote) does not recur on the output: en-US, Prose, `DashStyle::SpacedGlued`,
`B “” S an U.S. — '10-20?!’` gives `B “” S an U.S. — ’10–20?!’`, and the match of
`dash.spaced-hyphen` on the typed `—` (no edit) is not there on the second pass; en-GB, Prose,
`?-( - rock'n'roll) 1941-1945 km file — '27 % host:8080’`. Both matches have no edit, so nothing
changes; a refusal that turns into a match on the second pass would break idempotence, and the
properties over the grammar found none.

## Goal

The two classes are fixed or shown to be outside I2 and I4 (and design section 4.3 says so), and the
unit test runs over `strategies::grammar_case` as well as over `any_text`.

## Acceptance

- The inputs above run in the unit test, which draws from the grammar in its five languages.
