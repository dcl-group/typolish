# typolish design: Quotes

Part of the [typolish design](./README.md). Section numbers are those of the whole design.

## 6. Quotes

The quote phase serves three rules: `quotes.apostrophe`, `quotes.pairs` and `quotes.primes`. It runs
over a whole window in linear time, in the sub-steps below. Classification of quotes does not depend
on which of the three rules are enabled; a disabled rule only suppresses its own edits. So with
`quotes.apostrophe` disabled, `don't` still does not take part in pairing.

### 6.1 Candidates

Candidates are tokens of kind `Quote` outside frozen cores: straight `"` and `'`, and the typed
glyphs of the two quote pairs of the language. Typed glyphs that belong to neither pair of the
language (`”` in Russian text, `«` in English text) are neutral: they never change and take no part
in pairing or levels. `′` and `″` are resolved primes and are never candidates.

A straight quote in the wrapper of a frozen fragment is a candidate only if the core of that
fragment contains no straight quote. So `"host:8080"` → `«host:8080»` (the address stays intact),
while in `key="value"` both quotes belong to the technical notation and are left alone.

### 6.2 Step 1: certain apostrophes and primes

Local context removes from pairing what is certainly not a quote. The decision is the same for the
plain (`'`) and the typed (`’`) form, so a second pass removes the same tokens (invariant I4).

| Context of `'` or `’`                                                                               | Result     | Rule                |
| --------------------------------------------------------------------------------------------------- | ---------- | ------------------- |
| a letter on both sides, any script (`don't`, `O'Brien`, `rock'n'roll`, `Д'Артаньян`)                | `’` U+2019 | `quotes.apostrophe` |
| a `Word` or `Number` slot on the left, a letter on the right (`{name}'s`)                           | `’`        | `quotes.apostrophe` |
| a digit on the left, a letter on the right (`90's`)                                                 | `’`        | `quotes.apostrophe` |
| word start and an elision of the language's list (`'tis`, `'em`, `'n'`, `'cause`)                   | `’`        | `quotes.apostrophe` |
| word start, two digits, then `s` or a boundary, if the language has the decade flag (`'90s`, `'26`) | `’`        | `quotes.apostrophe` |
| a digit on both sides (`5'10`)                                                                      | `′` U+2032 | `quotes.primes`     |

The apostrophe is U+2019 for every language, as the Unicode Standard (chapter 6) recommends for
typeset text. A straight `"` has no step 1: after a digit it is decided in section 6.5.

### 6.3 Roles

Adjacent quotes form a **series** (plain and typed mixed). The role of a straight quote is decided
for the whole series from the classes of its neighbours, the left one L and the right one R, with
`Format` and `Markup` tokens skipped. Classes: `Space` (also the start and end of the window and a
line break); `OpenP` (`( [ {`, an operator token, `/`); `CloseP` (`) ] } . , ; : ! ?` and `Dots`);
`DashC` (a `Dash` in any form); `Word` (a word, a number, a slot, a protected span, a frozen core, a
legal mark, a `Symbol`).

- opening: R ∉ {Space, CloseP} and L ∈ {Space, OpenP, DashC};
- closing: L ∉ {Space, OpenP, DashC} and R ∈ {Space, CloseP, DashC};
- otherwise no role: spaces on both sides (`Слово " в тексте`) or letters on both sides (`a"b`).

A dash may stand left of an opening quote (`—"Да"`) and right of a closing one (`"no"—and`, closed
style), hence its own class. In a series after a letter and before a space every quote is closing;
after a space and before a letter every quote is opening; inside a series, closing goes right to
left over the stack.

A straight quote without a role is not a candidate, never changes, and gives `AmbiguousQuote`
(info). It does not block the window: it can pair with nothing and creates no ambiguity for the
others.

Typed glyphs have the role of their glyph in the language's pairs: in Russian `«` and `„` open, `»`
and `“` close; in English `“` and `‘` open, `”` closes. U+2019 `’` closes a `‘` only if one is open;
otherwise it is an apostrophe and never blocks anything (it is both the apostrophe and the English
closing single quote).

### 6.4 Families and stack matching

Every candidate with a role has a **family**: `D` for straight `"`, `S` for straight `'`, and for a
typed glyph its pair (primary or nested). One pass over the window with a stack of (family, index):

- an opening quote is pushed;
- a closing quote c of family F looks for the nearest stack entry of family F:
  - on top: (entry, c) is a pair, the entry is popped;
  - covered only by `S` entries: those straight `'` are unpaired and popped, then the pair is made;
  - covered by a `D` entry or a typed entry: the pairs would cross; the window does not converge,
    `CrossingQuotes` (warning) at the covering entry;
  - absent: a straight `"` makes the window non-converging with `UnmatchedQuote` (warning); a
    straight `'` is unpaired; `’` is an apostrophe (section 6.3); any other typed glyph is neutral,
    with `UnmatchedQuote` (info);
- at the end of the window, remaining `D` entries make it non-converging (`UnmatchedQuote`,
  warning); remaining `S` entries are unpaired; remaining typed openers are neutral
  (`UnmatchedQuote`, info).

Every quote is pushed and popped at most once. The nearest entry of a family is found through a
per-family pointer to its topmost entry, so the pass is linear even with many quotes; there is no
look-ahead.

### 6.5 Quotes after a number

A closing quote that directly follows a digit or a `Number` slot (no space between) may be a closing
quote or an inch or second mark. Such a quote is **weak**; every other closing quote is **strong**.

1. **Pass 1** matches with the stack of section 6.4 and ignores weak quotes.
2. **Pass 2** walks the window again with the pairs of pass 1 fixed. A weak quote of family F pairs
   with the top of the stack if that entry is an opener of family F that pass 1 left unpaired, and
   the pair nests properly with the pairs of pass 1 by construction. Otherwise the weak quote is a
   prime.
3. A weak quote resolved as a prime is not a candidate any more. It becomes `″` (from `"`) or `′`
   (from `'`) when `quotes.primes` is enabled and stays straight when it is not; either way it does
   not make the window non-converging.

The decision is made for each quote on its own, so mixed cases resolve: `Экран 27" и "ручка"` pairs
`"ручка"` in pass 1 and turns `27"` into a prime; `Он сказал "диагональ 27".` pairs the weak quote
with the opener that pass 1 left unpaired; `"Экран 27" дюймов"` pairs the outer quotes in pass 1 and
makes `27"` a prime; `"Диагональ 27" и экран 24"` pairs the first weak quote and makes the second a
prime. The ambiguity "a quotation inside quotes ends with an inch value" (`"Монитор 24" и ручка"`)
resolves as an inch: the outer pair is strong.

### 6.6 All or nothing per window

After pass 2 the window **converges** when no pairs cross and every straight `"` with a role is
paired or a prime. Then:

- every pair of straight quotes gets the glyphs of its level (section 6.7);
- weak quotes resolved as primes get `″` or `′` (if `quotes.primes` is enabled);
- an unpaired straight `'` directly after the letter `s` and before a non-letter becomes `’` if the
  language has the trailing-possessive flag (`dogs'`, `the Joneses'`).

If the window does **not** converge, no straight quote of the window changes; the edits of step 1
(section 6.2) are kept. The caller gets a diagnostic at the position of the first cause.

Why not "set what paired and leave the rest": with an unpaired `"` in a paragraph, it is unknown
which of the other quotes are pairs. `"a" b "c` reads as `«a» b "c` or as `"a «b» c`; the task says
not to guess. Partial placement is also not idempotent: there are inputs where a second pass pairs
what the first left alone (`"x 'y" z'` with single quotes converted everywhere, `"x 'y «z" w'`). The
cost of this choice: one typo leaves its whole paragraph, or a whole template, with straight quotes,
with an exact diagnostic. This is open point 8.

### 6.7 Levels

The level of a pair is the number of matched pairs, typed or new, that enclose it; neutral glyphs
and unpaired quotes do not count. A second linear pass with a depth counter computes it. An even
level gets the primary pair of the language, an odd level the nested pair: Russian `«…„…«…»…“…»`,
English `“…‘…“…”…’…”`. Typed pairs count, so `«Он сказал "да"»` → `«Он сказал „да“»`.

Which straight character was used does not choose the glyph; the level does. The language sets the
glyphs and the source only marks "a quote here": in English `"a 'b' c"` → `“a ‘b’ c”`, in Russian
`"a 'b' c"` → `«a „b“ c»`. Keeping the "family" of the source character (`'x'` → `‘x’`) was
rejected: Russian has no single-quote pair to map `'` to.

### 6.8 Single quotes at the top level

`SingleQuotes` (setting of `quotes.pairs`):

- `NestedOnly` (default): a pair of straight `'` at level 0 is cancelled; both quotes stay straight,
  without a diagnostic. So `She said 'hi'` and `echo 'hi'` do not change, while `"He said 'hi'"` →
  `“He said ‘hi’”`. A straight `'` in plain text is more often an apostrophe or code than a quote.
- `Everywhere`: pairs of `'` get glyphs by level like pairs of `"` (`'hi'` → `“hi”` in English).
- `Never`: straight `'` never pairs; only step 1 and the trailing possessive apply.

Open point 9.

### 6.9 Slots and protected spans

A slot of any kind and a protected span are of class `Word` for roles. `"{name}"`, ``"`ls`"`` and
`"{a}" и "{b}"` are quoted like words; the slot itself never changes (it is frozen), and the glyphs
go into the neighbouring texts. A `Number` slot before a closing quote makes it weak: `"{n}"` pairs
in pass 2 → `«{n}»`; `{n}"` alone becomes a prime `{n}″` (with `quotes.primes`).

### 6.10 Scope

The scope of pairing is the window, a paragraph (section 3.3). For pieces it is the same over the
text of the pieces, so a message without blank lines is one scope, and a quote opened in one text
closes in another.

Quotes spanning paragraphs (the book convention of an opening quote at the start of every paragraph
of a long quotation) do not converge in each paragraph and stay straight, with `UnmatchedQuote`.
This is a known limit of version 1 (section 1.5).

### 6.11 Idempotence of the phase

Roles on a second pass are those of the first: a role comes from the neighbours of a series, typed
glyphs are in the series like straight ones, and neighbours do not change (I1). Step 1 removes the
same tokens (I4). The first pass either converged or did not.

(a) **Converged.** Every straight `"` became a glyph of a pair or a prime; the new pairs nest
properly, and the family of each new glyph is the primary or the nested pair according to its level.
On the second pass the stack meets the new glyphs in the same positions and pairs them the same way,
because the pairs are properly nested and each opener and closer of a new pair share a family. Typed
glyphs that were neutral on the first pass can pair with a new glyph on the second, but they never
change, and such a re-pairing cannot enclose a straight `'` pair that was at level 0: that pair lay
outside every pair of the first pass, and a re-pairing with a glyph inside a first-pass pair would
have been a crossing, which contradicts convergence. So cancelled top-level `'` pairs stay at level
0, unpaired `'` stay unpaired, primes are not candidates, and nothing changes.

(b) **Not converged.** No straight quote changed. The quotes with roles and their families are the
same on the second pass, so the window again does not converge, and nothing changes.

In both cases the phase is the identity on its own output. The property tests of section 13.5 check
it on generated strings with all three `SingleQuotes` modes, and a fixed-point oracle compares one
pass with passes repeated until nothing changes.

### 6.12 Examples

| Language   | Input                                | Output                               | Note                                |
| ---------- | ------------------------------------ | ------------------------------------ | ----------------------------------- |
| ru         | `Он сказал "привет" и ушёл.`         | `Он сказал «привет» и ушёл.`         |                                     |
| ru         | `"Он сказал "привет""`               | `«Он сказал „привет“»`               | levels                              |
| en         | `He said, "I read "War and Peace"".` | `He said, “I read ‘War and Peace’”.` |                                     |
| en         | `"He said 'hi'"`                     | `“He said ‘hi’”`                     | nested `'`                          |
| en         | `She said 'hi' to him.`              | unchanged                            | `NestedOnly`                        |
| ru         | `Он сказал "привет.`                 | unchanged, `UnmatchedQuote`          | not guessed                         |
| ru         | `"Один "два три"`                    | unchanged, `UnmatchedQuote`          | the paragraph keeps straight quotes |
| ru         | `Слово " в тексте.`                  | unchanged, `AmbiguousQuote`          | no role                             |
| ru         | `«a "b» c"`                          | unchanged, `CrossingQuotes`          |                                     |
| ru, pieces | `Файл "` `{Word}` `" не найден`      | `Файл «` `{Word}` `» не найден`      | across pieces                       |
| en         | `I don't know. It's the '90s.`       | `I don’t know. It’s the ’90s.`       | step 1                              |
| en         | `Rock 'n' roll, the dogs' bowls`     | `Rock ’n’ roll, the dogs’ bowls`     | elision, trailing possessive        |
| ru         | `Это rock'n'roll и don't.`           | `Это rock’n’roll и don’t.`           | any script                          |
| ru         | `Экран 27" и "ручка"` (Prose)        | `Экран 27″ и «ручка»`                | weak quote is a prime               |
| ru         | `Экран 27" и "ручка"` (Interface)    | `Экран 27" и «ручка»`                | `quotes.primes` is not in Interface |
| ru         | `"Экран 27" дюймов"` (Prose)         | `«Экран 27″ дюймов»`                 |                                     |
| en         | `He is 5'10" tall.` (Prose)          | `He is 5′10″ tall.`                  |                                     |
| ru         | `Угол 30° 15' 20" и точка.` (Prose)  | `Угол 30° 15′ 20″ и точка.`          |                                     |
| ru         | `Он сказал «привет „мир“»`           | unchanged (`Cow::Borrowed`)          | already typeset                     |
| ru         | `Задайте key="value" в файле.`       | unchanged                            | quotes belong to a frozen fragment  |

### 6.13 Rejected

| Alternative                                                                 | Why not                                                                                         |
| --------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------- |
| Deciding each quote by the previous character, without pairs                | `"foo "` gets an opening quote at the end; unpaired quotes cannot be found.                     |
| A flat counter of double quotes and look-ahead for single ones              | Quadratic on many `'`; a closing quote is pulled from another sentence; no levels.              |
| Delimiter flags as in CommonMark, one level per quote kind                  | An unpaired `"` becomes `“` (a guess); an inch after a number is confused with a closing quote. |
| Partial placement (set the pairs, leave the unpaired)                       | Guesses when ambiguous, and is not idempotent (section 6.6).                                    |
| Scope: the whole string                                                     | One error blocks a whole long text.                                                             |
| Scope: a sentence                                                           | Quotations span sentences; sentence boundaries are unreliable (`т. е.`, `e.g.`).                |
| Two global attempts for quotes after numbers (all closing, then all primes) | Fails on mixed cases such as `"Диагональ 27" и экран 24"`, where no global attempt converges.   |
