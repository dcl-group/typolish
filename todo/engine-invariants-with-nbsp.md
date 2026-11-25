# I4 of the engine with the `nbsp.*` rules

## Problem

`i2_and_i4` in `src/engine/mod.rs` checks I2 and I4 of design section 4.3 over `any_text` and over
the structured grammar of design section 13.5 without the `nbsp.*` rules, and with every rule only
on the splits and merges of I2 (`i2_and_i4_hold_on_the_splits_and_merges_with_every_rule`). Run over
the grammar with every rule (`grammar_typographer(config, true)` in the long run), it finds one
class that section 4.3 now names as an exception and that the unit test does not exempt.

A dash before a label that `nbsp.signs` splits: on f(x) the right operand of the dash is the sign
alone (a `Sign` fragment), which is no operand, so no dash rule matches. ru-RU and declared
languages, Prose and `ALL`: `word – §5` → `word – §⍽5`, `дом – №V` → `дом – №⍽V`, `5 – №V`,
`"" – §5`, and the dialogue dash in `- №V` → `—⍽№⍽V` and `- §V`. The dash on x is typed or written,
so no edit is lost, and the properties over the grammar (which run every rule) find no second pass
that changes such a sentence.

Section 4.3 (I4, the reading of a label by its neighbors, C42) says a rule that reads a neighboring
fragment reads a label as its sign on the right; the dash rules read it so for their refusals
(`plain_side`) but not for the right operand itself.

## Goal

Either the dash rules (`between` and `dash.dialogue` in `src/rules/dash.rs`) read a sign that
`nbsp.signs` split from its number as the label it was, so the match recurs with an empty edit, and
the exception leaves section 4.3; or the exception stays, and `i2_and_i4` exempts exactly this class
with negative rows in `the_exemptions_of_i2_and_i4_name_only_their_cases`.

## Acceptance

- The long run of `i2_and_i4` over the grammar runs with every rule and passes.
- The rows above run in `i2_and_i4_hold_on_the_rows_of_the_grammar` with every rule.
