# Sign-only pairs that share a quote are not idempotent

## Problem

Two sign-only pairs of design section 6.3 can share a quote: in `''-''` the second `'` is the closer
of the pair `''` and the opener of the pair `'-'`, and the third `'` is the closer of `'-'` and the
opener of `''`. The roles of the series are set pair by pair and a later pair overrides an earlier
one, so the quotes get the roles open, open, open, close, and only the last two pair. On a second
pass the glyphs of that pair are no straight quotes any more, the chain is cut, and the first two
quotes pair (design section 6.11 claims a fixed point for the quote phase):

| Language and set             | Input         | First pass    | Second pass   |
| ---------------------------- | ------------- | ------------- | ------------- |
| `ru-RU`, Prose and Interface | `"x ''-'' y"` | `«x ''-„“ y»` | `«x „“-„“ y»` |
| `en-US`, `Everywhere`, both  | `''-''`       | `''-“”`       | `“”-“”`       |
| `en-US`, `Everywhere`, both  | `''-')'«`     | `''-“)”«`     | `“”-“)”«`     |

P2 (design section 12.2) fails on the same class in the default mode, in English as well:

| Language and set         | Input         | Typeset       | `to_plain`    | Typeset again |
| ------------------------ | ------------- | ------------- | ------------- | ------------- |
| `en-US`, Prose/Interface | `"x ''-'' y"` | `“x ''-‘’ y”` | `"x ''-"" y"` | `“x ‘’-‘’ y”` |
| `en-US`, `Everywhere`    | `""-''`       | `“”-“”`       | `""-""`       | `""-""`       |

Found while re-checking P2 after the fix of the mis-nested possessive (0.1.2), with a random
generator over quotes, signs and short words in `en-US`, `ru-RU` and a declared language whose
primary pair is `‘’`, all three `SingleQuotes` modes. The case is listed in design section 10.8 and
in the README.

The structured grammar of design section 13.5 finds the class in every language it runs, also with a
dash or a parenthesis as the sign: Russian `''—''` (`''—«»`, then `«»—«»`), English and en-GB
`'('')'` with `SingleQuotes::Everywhere` (`'('“)”`, then `“(”“)”`). The properties over the grammar
skip a failure whose input holds such a chain and whose second pass changes straight quotes only,
and a P2 failure on such an input where `to_plain` read a `"` as `'` or the other way round
(`SIGN_ONLY_CHAIN` of `grammar_known_failure` and `grammar_known_p2_failure` in
`tests/common/strategies.rs`); that filter goes with this task.

## Analysis so far

Reading the chain greedily from the left (a quote that closes a sign-only pair opens none) makes the
first pass a fixed point on the rows above, but `local_role` (design section 8.2), which other rules
call for one quote at a time, would then have to walk the chain back to its start to know the
parity, which is quadratic on a long chain such as `"-"-"-"-…`. A local rule that drops every link
whose opener also closes a link, or whose closer also opens one, needs no walk; whether it is a
fixed point when a quote of a dropped link pairs by its own role was not proven.

## Goal

The first pass is a fixed point on these inputs and P2 holds, while the output of the first pass on
the corpus and on the tables of `tests/` stays the same and `local_role` stays linear.

## Acceptance

- The inputs above are in `SEEDS` of `tests/properties.rs` and the P2 rows run in a test.
- The limit is removed from design section 10.8, from the README (both the known limit and the
  exception in the feature list, if no other remains), from `limits()` of `tests/documentation.rs`
  and from the exception of P2 in design section 12.2, and its filter `SIGN_ONLY_CHAIN` from
  `grammar_known_failure` and `grammar_known_p2_failure` (with its rows in the filter tests).
