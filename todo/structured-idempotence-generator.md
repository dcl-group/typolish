# Idempotence and P2 properties over structured input

## Problem

The properties of idempotence and of the round trip (design sections 12.2 and 13.5) draw their input
from generators that join characters and short fragments without structure, with the built-in
languages and the default dash style only. Classes that need a dash next to an abbreviation, a unit
or a particle, nested quotes, the closed style or a declared language were found only by a separate
grammar generator (the audit of 2026-11-24): the merged and split fragments of the closed style, the
added units and signs that two rules read, the ellipsis forms of a declared language. Those classes
are now fixed and their examples are seeds of `idempotence_of_the_seeds`, but the properties would
not find the next one.

## Goal

A strategy `grammar()` in `tests/common/strategies.rs` that builds sentences of words of the
language, abbreviations (`т. е.`, `и т. д.`, `St.`, `p.m.`), units after numbers, particles and
short words, labels (`№5`, `§V`), dashes in every form (a spaced `-`, a spaced `--`, an attached
`-`, a spaced `–`, an attached `—`), wrappers (`(` `)`), ellipses and sentence ends, and nested
quotes of every glyph, up to a depth of four. A `Config` strategy that varies the dash style
(`Spaced`, `SpacedGlued`, `Closed`), the single quote mode and the language: `RU_RU`, `EN_US` and
two languages declared with `LanguageBuilder` (primary pair `‘’` with nested `“”`; `«»` with nested
`‹›` and the closed style). The idempotence property and P2 run over both. Design section 12.2 then
says "for both languages and the declared languages of the test setup".

## Acceptance

Both properties pass over the grammar in the default number of cases and in a long `#[ignore]` run
of at least 100 000 cases per configuration. The properties join the gates only after the known
idempotence defect of mis-nested quotes is fixed (`todo/quotes-misnested-idempotence.md`); until
then that class would make the gate unstable.
