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
in pairing or levels. `′` and `″` are resolved primes and are never candidates. In a language whose
pairs do not contain `’`, U+2019 is always an apostrophe: it never changes, never pairs and gets no
diagnostic.

A straight quote in the wrapper of a frozen fragment is a candidate only if the core of that
fragment contains no straight quote. So `"host:8080"` → `«host:8080»` (the address stays intact),
while in `key="value"` both quotes belong to the technical notation and are left alone. When a
frozen core ends in `=` and the start wrapper of its fragment holds no straight quote, the end
wrapper holds no candidate either: `key=""` stays as written, like `key="value"`. This holds only
for a frozen core that ends in `=`. After a core that is not frozen, a wrapper such as `:""`, `,""`
or `.""` is sentence punctuation followed by two closing quotes, as at the end of
`"Он сказал "привет.""`; with no opener, `Значение:"" и "да"` does not converge (`UnmatchedQuote`,
section 6.6). The same happens after a frozen core that does not end in `=` and holds no straight
quote: both quotes of `/etc/""` are candidates by the rule above and both close (L is of class
`Word`, section 6.3), so `Путь /etc/"" и "да".` does not converge either. `Значение: ""` with a
space gives `Значение: «»`. If the start wrapper holds a straight quote, the end wrapper is a
candidate as usual: `"SGVsbG8="` → `«SGVsbG8=»`.

### 6.2 Step 1: certain apostrophes and primes

Local context removes from pairing what is certainly not a quote. The decision is the same for the
plain (`'`) and the typed form (`’` for an apostrophe, `′` for a prime), so a second pass removes
the same tokens (invariant I4): `’` stays an apostrophe, and `′` is never a candidate (section 6.1).

| Context of `'` or `’`                                                                                                                                                                                | Result     | Rule                |
| ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ---------- | ------------------- |
| a letter on both sides, any script (`don't`, `O'Brien`, `rock'n'roll`, `Д'Артаньян`)                                                                                                                 | `’` U+2019 | `quotes.apostrophe` |
| a `Word` or `Number` slot on the left, a letter on the right (`{name}'s`)                                                                                                                            | `’`        | `quotes.apostrophe` |
| a digit on the left, a letter on the right (`90's`)                                                                                                                                                  | `’`        | `quotes.apostrophe` |
| word start and an elision of the language's list (`'tis`, `'em`, `'cause`)                                                                                                                           | `’`        | `quotes.apostrophe` |
| word start, two digits, then `s` and then a character that is neither a letter nor a digit, or the end of the window, if the language has the decade flag (`decade_apostrophe`, section 7.1; `'90s`) | `’`        | `quotes.apostrophe` |
| a number of one to three digits on the left and one or two digits ending the core on the right (`5'10`; a straight `'` only: `5’10` is frozen, section 3.6)                                          | `′` U+2032 | `quotes.primes`     |

The apostrophe is U+2019 for every language, as the Unicode Standard (chapter 6) recommends for
typeset text; it is a constant of the crate, not a field of `LanguageData` (section 7.1). A straight
`"` has no step 1: after a digit it is decided in section 6.5.

Step 1 holds only contexts in which a quote cannot be meant. Two look alike but can open a quoted
value, so they wait for pairing: a decade without `s` (`'26`) becomes an apostrophe only if it stays
unpaired (section 6.6), so `Press '42' to confirm` and `'12:30'` keep a pair of quotes; and `'n'` is
not in the elision list, because `Press 'y' to confirm, 'n' to cancel` quotes a letter. An elision
closed at once by a straight `'` (`'round'`, followed by a `'` that no letter or digit follows)
waits for pairing, like `'n'`: `Choose the 'round' button` keeps its straight pair, and
`"Say 'twas' or 'tis'"` gives nested pairs. `rock'n'roll` gets apostrophes between letters;
`rock 'n' roll` with spaces is a top-level single pair and stays straight under `NestedOnly`
(section 10.8).

### 6.3 Roles

Adjacent quotes form a **series** (plain and typed mixed). A series is a maximal run of adjacent
`Quote` tokens of any form: straight, typed, neutral, apostrophes and primes (`Format` and `Markup`
tokens between them are skipped). L and R are the nearest tokens before and after the whole series;
they are never quotes, and every quote of the series has the same L and R. The role of a straight
quote is decided for the whole series from the classes of its neighbors, the left one L and the
right one R, with `Format` and `Markup` tokens skipped. Classes: `Space` (also the start and end of
the window and a line break); `OpenP` (`( [ {`, an operator token, `/`); `CloseP`
(`) ] } . , ; : ! ?` and `Dots`); `DashC` (a `Dash` in any form); `Word` (a word, a number, a slot,
a protected span, a frozen core, a legal mark, a `Symbol`, and every other sign: `№ § % # @ & * $`).
A neighbor inside a frozen core is of class `Word`, whatever its own token: in `"/etc/"` the quote
after `/etc/` has L = `Word`, not `OpenP`. Wrapper tokens and tokens of cores that are not frozen
have the class of their token. Whether a closing quote is weak (section 6.5) still depends on the
character before it, with `Format` and `Markup` tokens skipped: `27\u{AD}"` and
`\u{2068}27\u{2069}"` are weak like `27"`, while the quote after `/etc/` is not weak.

- opening: L ∈ {Space, OpenP, DashC}, and either R ∉ {Space, CloseP}, or R ∈ CloseP and the first
  token after the run of `CloseP` tokens, within the same fragment, is of class `Word` (`".env"`,
  `"...и тогда"`);
- closing: L ∉ {Space, OpenP, DashC} and R ∈ {Space, CloseP, DashC};
- sign-only pair: two straight quotes of the same character in one fragment with nothing but signs
  between them (punctuation, dots, dashes, operators and the other signs listed under `Word`; no
  word, number, slot, protected span or `Symbol`), the first with L ∈ {Space, OpenP, DashC} and the
  second with R ∈ {Space, CloseP, DashC}: the first opens and the second closes, whatever the two
  rules above say (`"?"`, `"..."`, `"-"`, `"—"`, `"->"`). The two quotes may be adjacent: nothing
  between them also counts, so `""` is a sign-only pair; `Format` and `Markup` tokens between them
  count as nothing (section 5.4), so `"\u{2068}\u{2069}"` is a sign-only pair too. This rule
  overrides the role of the whole series. A series of three or more quotes, of any form, has no
  sign-only pair inside it (`"""`, `''''`, and also `”""`: there a first pass has set one quote of
  `"""` as the closer of a pair that `"--` opened, and counting only straight quotes would let a
  second pass pair the two that stayed, against section 6.11);
- otherwise no role: spaces on both sides (`Слово " в тексте`) or letters on both sides (`a"b`).

The look-through and the sign-only pair never turn a closing quote into an opening one: a quote with
L ∈ {Space, OpenP, DashC} cannot close. Without them, an opening quote before `.`, `?`, `!` or `...`
or around a lone sign would get no role, its partner would stay unpaired, and the whole window would
keep straight quotes.

A dash may stand left of an opening quote (`—"Да"`) and right of a closing one (`"no"—and`, closed
style), hence its own class. So in a series after a letter and before a space every straight quote
is closing, and after a space and before a letter every straight quote is opening. The quotes of a
series are matched left to right like all others: at the end of `"He said 'hi'"` the `'` meets the
top of the stack and closes first, then the `"`.

A straight quote without a role is not a candidate, never changes, and gives `AmbiguousQuote`
(info). It does not block the window: it can pair with nothing and creates no ambiguity for the
others.

Typed glyphs have the role of their glyph in the language's pairs: in Russian `«` and `„` open, `»`
and `“` close; in English `“` and `‘` open, `”` closes. U+2019 `’` closes a `‘` only if one is open
(on the stack of section 6.4 and covered by nothing but straight `'` entries, so that it never
causes a crossing) and its neighbors allow a closing role (the closing test above: L ∉ {Space,
OpenP, DashC} and R ∈ {Space, CloseP, DashC}); otherwise it is an apostrophe and never blocks
anything (it is both the apostrophe and the English closing single quote). So `’26` and `’tis` at a
word start are always apostrophes.

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
- at the end of the window (after pass 2 of section 6.5), remaining `D` entries make it
  non-converging (`UnmatchedQuote`, warning); remaining `S` entries are unpaired; remaining typed
  openers are neutral (`UnmatchedQuote`, info). Verdicts and diagnostics for entries left on the
  stack are given only then.

Every quote is pushed and popped at most once. The nearest entry of a family is found through a
per-family pointer to its topmost entry, so the pass is linear even with many quotes; there is no
look-ahead.

### 6.5 Quotes after a number

A closing straight quote (`"` or `'`) that directly follows a digit or a `Number` slot (no space
between; `Format` and `Markup` tokens are skipped, section 6.3) may be a closing quote or an inch or
second mark. Such a quote is **weak**; every other closing quote is **strong**. A typed closing
glyph is always strong: after a digit it pairs in pass 1 like any typed quote (`“Enter 5”`), and
without an opener it is neutral with `UnmatchedQuote` (info), section 6.4.

1. **Pass 1** matches with the stack of section 6.4 and ignores weak quotes.
2. **Pass 2** runs only if pass 1 found no crossing. It walks the window again with the stack of
   section 6.4 and the pairs of pass 1 fixed: an opener is pushed; the closing quote of a pass-1
   pair pops its opener and every entry above it (straight `'` that pass 1 left unpaired; they stay
   unpaired); a weak quote of family F pairs with the top entry and pops it if that entry is an
   opener of family F that pass 1 left unpaired. Otherwise the weak quote is a prime. Since the
   entries above a pass-1 pair are popped when the pair closes, every new pair lies inside or
   outside each pass-1 pair: the pairs nest properly.
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

After pass 2 the window **converges** when no pairs cross, every straight `"` with a role is paired
or a prime, and no typed closer that stayed neutral (section 6.4) lies inside a new pair whose
opening glyph, by its level (section 6.7), is of the closer's family. Such a closer gives
`CrossingQuotes` (warning): on a second pass it would pair with that glyph and narrow the new pair,
and what the first pass decided for the quotes between the closer and the end of the pair would no
longer hold. In `x '"»b 5' c"` with `Everywhere` (Interface) a first pass would give `x '«»b 5' c»`
and a second `x ««»b 5» c»`, so the window stays as it is; for the same reason `"a » b"` stays.
Then, when the window converges:

- every pair of straight quotes gets the glyphs of its level (section 6.7);
- weak quotes resolved as primes get `″` or `′` (if `quotes.primes` is enabled);
- a straight `'` that is in no pair after pass 2 (the quotes of a pair cancelled by `SingleQuotes`,
  section 6.8, are in a pair and stay straight) directly after the letter `s` and before a
  non-letter or the end of the window becomes `’` if the language has the trailing-possessive flag
  (`trailing_possessive`, section 7.1; `dogs'`, `the Joneses'`), unless a typed opener whose closer
  is `’` (English `‘`) that stayed neutral (left on the stack at the end of the window, section 6.4)
  stands before it anywhere in the window; then it stays straight (a second pass would let that `’`
  close the opener);
- a straight `'` that is in no pair after pass 2 (the quotes of a pair cancelled by `SingleQuotes`,
  section 6.8, are in a pair and stay straight) at a word start before exactly two digits and then a
  character that is neither a letter nor a digit, or the end of the window, becomes `’` if the
  language has the decade flag (`the class of '26`).

If the window does **not** converge, no straight quote of the window changes; the edits of step 1
(section 6.2) are kept. The caller gets a diagnostic at the position of the first cause.

Why not "set what paired and leave the rest": with an unpaired `"` in a paragraph, it is unknown
which of the other quotes are pairs. `"a" b "c` reads as `«a» b "c` or as `"a «b» c`; the design
reports and does not guess (section 1.4, principle 5). Partial placement is also not idempotent:
there are inputs where a second pass pairs what the first left alone (`"x 'y" z'` with single quotes
converted everywhere, `"x 'y «z" w'`). The cost of this choice: one typo leaves its whole paragraph,
or a whole template, with straight quotes, with an exact diagnostic. Settled in section 15.5,
item 8.

### 6.7 Levels

The level of a pair is the number of matched pairs, typed or new, that enclose it; neutral glyphs,
unpaired quotes and pairs cancelled by the `SingleQuotes` setting (section 6.8) do not count. A
second linear pass with a depth counter computes it. An even level gets the primary pair of the
language, an odd level the nested pair: Russian `«…„…«…»…“…»`, English `“…‘…“…”…’…”`. Typed pairs
count, so `«Он сказал "да"»` → `«Он сказал „да“»`.

Which straight character was used does not choose the glyph; the level does. The language sets the
glyphs and the source only marks "a quote here": in English `"a 'b' c"` → `“a ‘b’ c”`, in Russian
`"a 'b' c"` → `«a „b“ c»`. Keeping the "family" of the source character (`'x'` → `‘x’`) was
rejected: Russian has no single-quote pair to map `'` to.

### 6.8 Single quotes at the top level

`SingleQuotes` (setting of `quotes.pairs`):

- `NestedOnly` (default): a pair of straight `'` that no pair of straight `"` and no typed pair
  encloses is cancelled before levels are computed; both quotes stay straight, without a diagnostic,
  and the pair does not count for the levels of the pairs inside it. So `She said 'hi'` and
  `echo 'hi'` do not change, `"He said 'hi'"` → `“He said ‘hi’”`, and `She said 'he told "no"'` →
  `She said 'he told “no”'`: nested glyphs never appear where the outer quotes stayed straight. A
  straight `'` in plain text is more often an apostrophe or code than a quote.
- `Everywhere`: pairs of `'` get glyphs by level like pairs of `"` (`'hi'` → `“hi”` in English).
- `Never`: straight `'` never pairs; only step 1, the trailing possessive and the decade apostrophe
  of section 6.6 and the primes of section 6.5 apply. Every `'` is unpaired (the caller said that
  `'` is never a quote), so in `'42'` the first `'` is a decade apostrophe and the last one, a weak
  quote that pairs with nothing, is a prime: `’42′` with `quotes.primes` enabled (Prose), `’42'`
  without it (Interface).

Settled in section 15.5, item 9.

### 6.9 Slots and protected spans

A slot of any kind and a protected span are of class `Word` for roles. `"{name}"`, ``"`ls`"`` and
`"{a}" и "{b}"` are quoted like words; the slot itself never changes (it is frozen), and the glyphs
go into the neighboring texts. A `Number` slot before a closing straight quote makes it weak:
`"{n}"` pairs in pass 2 → `«{n}»`; `{n}"` alone becomes a prime `{n}″` (with `quotes.primes`).

### 6.10 Scope

The scope of pairing is the window, a paragraph (section 3.3). For pieces it is the same over the
text of the pieces (an atom between line breaks is content, section 3.3), so a message without blank
lines is one scope, and a quote opened in one text closes in another.

Quotes spanning paragraphs (the book convention of an opening quote at the start of every paragraph
of a long quotation) do not converge in each paragraph and stay straight, with `UnmatchedQuote`.
This is a known limit of version 1 (section 1.5).

### 6.11 Idempotence of the phase

Roles on a second pass are those of the first: a role comes from the classes of the neighbors of a
series, every `Quote` token is in its series (section 6.3), and the class of a neighbor does not
change, because an edit never changes the kind of a token (I1). Step 1 removes the same tokens (I4).
The first pass either converged or did not.

(a) **Converged.** Every straight `"` became a glyph of a pair or a prime; the new pairs nest
properly (section 6.5, item 2), and the family of each new glyph is the primary or the nested pair
according to its level. On the second pass the stack meets the new glyphs in the same positions and
pairs them the same way, because the pairs are properly nested and each opener and closer of a new
pair share a family. A typed closer that was neutral on the first pass found no entry of its family
on the stack (section 6.4; one covered by another entry would have been a crossing), so on the
second pass the only entries of its family are new openers around it, and section 6.6 does not let
the window converge when one of them shares its family: it pairs with no new glyph. A typed opener
that was neutral on the first pass can pair with a new closer on the second, but it never changes,
and such a re-pairing cannot enclose a straight `'` pair that was cancelled: that pair lay outside
every pair of straight `"` and every typed pair of the first pass, and a re-pairing with a glyph
inside a first-pass pair would have been a crossing, which contradicts convergence. A
trailing-possessive `’` could close a neutral `‘` on the second pass; section 6.6 therefore does not
produce it after a neutral `‘`. So cancelled `'` pairs stay cancelled and count for no level,
unpaired `'` stay unpaired, primes are not candidates, and nothing changes.

(b) **Not converged.** No straight quote changed. The quotes with roles and their families are the
same on the second pass, so the window again does not converge, and nothing changes.

In both cases the phase is the identity on its own output. The property tests of section 13.5 check
it on generated strings with all three `SingleQuotes` modes, and a fixed-point oracle compares one
pass with passes repeated until nothing changes.

### 6.12 Examples

Outputs show the edits of the three quote rules only; in the full sets the no-break rules add glue
as well (`и⍽«ручка»`, `to⍽confirm`). "(Prose)" and "(Interface)" mark whether `quotes.primes` is
enabled; "(`Never`)" marks `SingleQuotes::Never`. A row without a mark holds in both sets, with
`SingleQuotes::NestedOnly`.

| Language   | Input                                           | Output                               | Note                                                          |
| ---------- | ----------------------------------------------- | ------------------------------------ | ------------------------------------------------------------- |
| ru         | `Он сказал "привет" и ушёл.`                    | `Он сказал «привет» и ушёл.`         |                                                               |
| ru         | `"Он сказал "привет""`                          | `«Он сказал „привет“»`               | levels                                                        |
| en         | `He said, "I read "War and Peace"".`            | `He said, “I read ‘War and Peace’”.` |                                                               |
| en         | `"He said 'hi'"`                                | `“He said ‘hi’”`                     | nested `'`                                                    |
| en         | `She said 'hi' to him.`                         | unchanged                            | `NestedOnly`                                                  |
| ru         | `Он сказал "привет.`                            | unchanged, `UnmatchedQuote`          | not guessed                                                   |
| ru         | `"Один "два три"`                               | unchanged, `UnmatchedQuote`          | the paragraph keeps straight quotes                           |
| ru         | `Слово " в тексте.`                             | unchanged, `AmbiguousQuote`          | no role                                                       |
| ru         | `«a "b» c"`                                     | unchanged, `CrossingQuotes`          |                                                               |
| en         | `He said ‘x’(see note)`                         | unchanged, `UnmatchedQuote` (info)   | `’` before `(` is an apostrophe                               |
| ru, pieces | `Файл "` `{Word}` `" не найден`                 | `Файл «` `{Word}` `» не найден`      | across pieces                                                 |
| en         | `I don't know. It's the '90s.`                  | `I don’t know. It’s the ’90s.`       | step 1                                                        |
| en         | `Rock 'n' roll, the dogs' bowls`                | `Rock 'n' roll, the dogs’ bowls`     | top-level single pair; possessive                             |
| en         | `She loves 'the dogs'`                          | unchanged                            | cancelled pair, not a possessive                              |
| en         | `She loves 'the dogs'` (`Never`)                | `She loves 'the dogs’`               | `Never`: every `'` is unpaired                                |
| ru         | `Это rock'n'roll и don't.`                      | `Это rock’n’roll и don’t.`           | any script                                                    |
| en         | `Press '42' to confirm`                         | unchanged                            | a pair, not a decade                                          |
| en         | `Press 'y' or 'n' now`                          | unchanged                            | `'n'` is not an elision                                       |
| en         | `Choose the 'round' button`                     | unchanged                            | an elision closed by `'` waits for pairing                    |
| en         | `"Say 'twas' or 'tis'"`                         | `“Say ‘twas’ or ‘tis’”`              | the same, nested                                              |
| en         | `'Tis 'round' here`                             | `’Tis 'round' here`                  |                                                               |
| en         | `Set the time to '12:30'`                       | unchanged                            | a pair, not a decade                                          |
| en         | `"Enter '42' now"`                              | `“Enter ‘42’ now”`                   | the weak quote pairs                                          |
| en         | `"a 'b c" 5'` (Prose)                           | `“a 'b c” 5′`                        | an opener popped by a fixed pair is not taken by a weak quote |
| en         | `"a 'b c" 5'` (Interface)                       | `“a 'b c” 5'`                        |                                                               |
| en         | `The class of '26`                              | `The class of ’26`                   | unpaired: decade                                              |
| en         | `She said 'he told "no"'`                       | `She said 'he told “no”'`            | cancelled pair: no level                                      |
| ru         | `Он ответил 'мы сказали "нет"'`                 | `Он ответил 'мы сказали «нет»'`      | the same                                                      |
| ru         | `Файл ".env" не найден`                         | `Файл «.env» не найден`              | look-through past `.`                                         |
| ru         | `Папка "/etc/" пуста`                           | `Папка «/etc/» пуста`                |                                                               |
| en         | `Run "x->" now`                                 | `Run “x->” now`                      |                                                               |
| ru         | `Адрес "::1"`                                   | `Адрес «::1»`                        |                                                               |
| ru         | `Нажмите "?" для справки`                       | `Нажмите «?» для справки`            | sign-only pair                                                |
| ru         | `Он писал: "...и всё".`                         | `Он писал: «...и всё».`              | look-through past `...`                                       |
| ru         | `Введите "-" или "—"`                           | `Введите «-» или «—»`                | sign-only pairs                                               |
| ru         | `Он сказал "" и ушёл.`                          | `Он сказал «» и ушёл.`               | empty sign-only pair                                          |
| ru         | `Пустая строка "".`                             | `Пустая строка «».`                  | empty sign-only pair before `.`                               |
| ru         | `Он сказал """ и ушёл.`                         | unchanged, `AmbiguousQuote` ×3       | no pair in a series of three                                  |
| ru         | `Экран 27" и "ручка"` (Prose)                   | `Экран 27″ и «ручка»`                | weak quote is a prime                                         |
| ru         | `Экран 27" и "ручка"` (Interface)               | `Экран 27" и «ручка»`                | `quotes.primes` is not in Interface                           |
| ru         | `"Экран 27" дюймов"` (Prose)                    | `«Экран 27″ дюймов»`                 |                                                               |
| en         | `“Say "hi" at 5”`                               | `“Say ‘hi’ at 5”`                    | a typed closer after a digit is strong                        |
| ru         | `Цена 27»`                                      | unchanged, `UnmatchedQuote` (info)   |                                                               |
| en         | `He is 5'10" tall.` (Prose)                     | `He is 5′10″ tall.`                  |                                                               |
| en         | `He is 5’10 tall.`                              | unchanged                            | `’` between digits: frozen                                    |
| ru         | `Угол 30° 15' 20" и точка.` (Prose)             | `Угол 30° 15′ 20″ и точка.`          |                                                               |
| ru         | `Он сказал «привет „мир“»`                      | unchanged (`Cow::Borrowed`)          | already typeset                                               |
| ru         | `Задайте key="value" в файле.`                  | unchanged                            | quotes belong to a frozen fragment                            |
| ru         | `Он сказал:"привет" и ушёл.`                    | unchanged                            | frozen fragment (section 10.8)                                |
| ru         | `Задайте key="" и нажмите "OK".`                | `Задайте key="" и нажмите «OK».`     | assignment: frozen                                            |
| ru         | `Поле Значение:"" и "да".`                      | unchanged, `UnmatchedQuote`          | two closing quotes after `:`, no opener                       |
| ru         | `Путь /etc/"" и "да".`                          | unchanged, `UnmatchedQuote`          | two closing quotes after a frozen core without `=`            |
| en         | `"She said "no.""`                              | `“She said ‘no.’”`                   | two closers after `.`                                         |
| ru         | `Токен "SGVsbG8gd29ybGQ=" и "имя".`             | `Токен «SGVsbG8gd29ybGQ=» и «имя».`  | a quoted frozen core ending in `=`                            |
| en         | `‘Note: 'beta' builds of the developers' tools` | unchanged, `UnmatchedQuote` (info)   | no possessive after a neutral `‘`                             |
| ru         | `a""b`                                          | unchanged, no diagnostic             | frozen core (section 3.6)                                     |

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
