# typolish design: Unicode and mixed languages

Part of the [typolish design](./README.md). Section numbers are those of the whole design.

## 5. Unicode and mixed languages

### 5.1 Grapheme clusters

The lexer walks extended grapheme clusters (UAX #29) through `unicode-segmentation` (section 14.1).
Without clusters a combining accent would split a word: `char::is_alphabetic` is false for U+0301,
so `за́мок` would become two words and the short-word lists would misfire.

Rules work on whole clusters:

- A replacement replaces one or more whole clusters. A cluster of several characters led by a
  non-letter is a `Symbol` (except `\r\n`, a cluster of ZWNJ and ZWJ only, section 5.4, and a space
  with a combining mark (Mn, Mc or Me), ZWNJ or ZWJ attached, section 5.3): a straight quote with a
  combining mark attached (`"` + U+0301) is not a quote candidate, and a keycap (`1️⃣`, `#️⃣`) is not
  a number or a sign. The fast path of section 3.5 hands the simple characters on both sides of any
  other character to the segmenter, so this holds right before and right after ASCII, Latin, Greek
  and Cyrillic text too.
- An insertion (`т.е.` → `т.⍽е.`, `№5` → `№⍽5`) is made only between clusters.
- Glue is set only on a `Space` token, which is a whole cluster.
- Emoji sequences (ZWJ, skin-tone modifiers, flags, keycaps, tag sequences) are single `Symbol`
  tokens; for neighboring quotes they behave as words: `"🙂"` → `«🙂»`.

Every edit position is therefore a cluster boundary of the input. A test runs every line of the
Unicode `GraphemeBreakTest.txt` for the Unicode version of the dependency (17.0.0) through both rule
sets, alone and inside sentences, and checks with the report that no edit starts or ends inside a
cluster and that every cluster of the input that no replacement or removal covers appears byte for
byte in the output (section 13.6).

Rejected: a hand-made table of `Extend` characters (it would miss `SpacingMark`, `Prepend`, regional
indicators and Hangul syllable sequences, and it would need manual updates with every Unicode
version); grapheme support behind a feature (Cargo unifies features across the dependency graph, so
a foreign crate enabling it would silently change everybody's output).

### 5.2 Letters, digits and scripts

- **Letters** are characters with `char::is_alphabetic()`, together with the combining marks of
  their cluster. ZWNJ and ZWJ inside a word are neither letters nor breaks, and do not count for its
  script.
- **Script** of a letter comes from fixed ranges in code: Cyrillic U+0400–U+052F, U+1C80–U+1C8F,
  U+2DE0–U+2DFF, U+A640–U+A69F; Latin ASCII letters, U+00C0–U+024F, U+1E00–U+1EFF, U+2C60–U+2C7F,
  U+A720–U+A7FF; Greek U+0370–U+03FF, U+1F00–U+1FFF; everything else `Other`. A word whose letters
  come from more than one script is `Mixed` (`Мoсква` with a Latin `o`). The ranges are a constant
  table, so the script of a letter does not depend on the Rust version. The `MixedScript` diagnostic
  is not raised for a core that matched a dictionary entry (check 0 of section 3.6), so the
  language's own units pass silently: `μs` (Greek U+03BC, Latin `s`) and `µs` (U+00B5, which falls
  outside the Latin ranges).
- **Digits** for number rules are ASCII `0` to `9` only. Digits of other systems are word
  characters: a cluster led by a non-ASCII character with `char::is_numeric` (Arabic-Indic and
  fullwidth digits, and also superscript digits, fractions and circled numbers) is a letter cluster
  for the lexer, and it adds no script to its word. Digits break a word: `x86` is a Latin word and a
  number in one fragment (check 20 of section 3.6 freezes it).

### 5.3 Spaces, line breaks, paragraphs

- U+0020 SPACE is the only character that a rule turns into a no-break space, and only when it
  stands alone between two fragments.
- U+00A0 and U+202F in the input are spaces that are already glued (section 3.9); rules use them as
  context and never change them. U+2009 THIN SPACE and every other character with
  `char::is_whitespace` that is not a line break or U+2029 (tab, U+000B, U+000C, U+2000 to U+200A,
  U+205F, U+3000 and so on) is horizontal whitespace: a space that separates fragments and is never
  glued or removed.
- Line breaks: `\n`, `\r\n`, `\r`, U+0085, U+2028. U+2029 is a paragraph separator, neither a line
  break nor a token: it always ends a window. A window boundary (section 3.3) is U+2029 or two or
  more line breaks with only horizontal whitespace between them. Line breaks are never glued.
- A space cluster with a combining mark (Mn, Mc or Me), ZWNJ or ZWJ attached (U+0020 + U+0301,
  U+0020 + U+0903, U+0020 + U+200D) is a frozen `Space`: it separates fragments, is never glued or
  removed, and the mark stays. A no-break rule that would have glued it simply does not, without a
  diagnostic.

### 5.4 Format characters and Fluent isolation marks

Format characters are U+00AD SOFT HYPHEN, U+200B ZERO WIDTH SPACE, U+200C ZWNJ, U+200D ZWJ, U+200E
LRM, U+200F RLM, U+202A to U+202E (embeddings and overrides), U+2060 to U+2064 (word joiner and
invisible operators), U+2066 to U+2069 (isolates: LRI, RLI, FSI, PDI) and U+FEFF. UAX #29 treats
them in two ways. All of them except ZWNJ and ZWJ are `Control` characters and always form a cluster
of their own; such a cluster is a `Format` token. ZWNJ (`Extend`) and ZWJ attach to the character
before them (rule GB9): they stay inside its cluster and are part of the text of its token, so
`сло\u{200D}во` is one `Word` and `می\u{200C}شود` is one `Word`. They form a cluster of their own
only at the start of a text and after a character of the UAX #29 classes `Control`, `CR` or `LF`
(rules GB4 and GB5): a line break, a tab or another C0 or C1 control, or a format character of this
list other than ZWNJ and ZWJ (`a\t\u{200D}b`, `a\u{AD}\u{200D}b`). A ZWNJ or ZWJ right after such a
lone ZWNJ or ZWJ joins its cluster (`\n\u{200C}\u{200D}` gives one cluster of two characters). A
cluster of ZWNJ and ZWJ only is a `Format` token. `Format` tokens are transparent for context: rules
skip them when they look for neighbors, never insert or remove them, and never put them inside an
edit. Inside a fragment the classifier and the list lookups skip `Format` tokens, and ZWNJ and ZWJ
inside the text of a token, too (section 3.6), so a word with soft hyphens from a hyphenation
preprocessor is still a word: `21 ноя\u{AD}бря` gets its glue, and `Мо\u{AD}скве` is a `Word`, not
`Unclassified`. UAX #29 makes U+00AD a cluster of its own, so the characters stay exactly where they
were. A ZWNJ or ZWJ inside a cluster moves with its cluster, and no edit falls between it and the
character before it. Any other `Extend` character, such as a combining mark, at the start of a text
or after a `Control`, `CR` or `LF` character also forms a cluster of its own; that cluster is a
`Symbol` (section 3.5), so it is frozen with the fragment it belongs to: in `a:b\t\u{301}c:d` the
tab separates, and the colon core is `\u{301}c:d`; a mark alone between two line breaks is an
`Unclassified` fragment.

This matters for strings that `fluent-bundle` formats at run time: by default it wraps every
placeable in U+2068 FIRST STRONG ISOLATE and U+2069 POP DIRECTIONAL ISOLATE. With transparency,
`Файл "⁨report⁩"` gets quotes around the isolated word, `в ⁨Москве⁩` gets glue after `в`, and
`⁨5⁩ МБ` gets glue before the unit (the isolate around `5` is skipped, the number is seen). A test
table typesets such strings and checks the same results as without the isolates, with the isolates
unchanged (section 13.2, `tests/fluent_isolation.rs`). U+2060 WORD JOINER in the input is kept and
is not treated as glue (a caller who wants glue writes U+00A0).

### 5.5 No normalization

The crate does not normalize Unicode (NFC needs large tables). A word in NFD (`и` + U+0306 instead
of `й`) does not match an entry of a language list: `5 дюйм.` with the unit in NFD gets no glue from
`nbsp.units`, and a word added with `add_short_words` gets none from `nbsp.short-words`. Rules that
read no list still apply: `й слово` gets glue from `nbsp.short-length` in `PROSE` in either form.
Nothing breaks. The rustdoc of the crate states this; callers that need matching normalize before
typesetting.

### 5.6 Unicode versions

Grapheme boundaries come from `unicode-segmentation` 1.13.3 (Unicode 17.0.0); `is_alphabetic` and
`is_whitespace` come from std, which is Unicode 18.0.0 in Rust 1.100 (both checked, section 14.8);
scripts come from the crate's own ranges. The result is identical on all platforms for the same
versions. A newer dependency or compiler can change the classification of characters assigned after
these versions only. A conformance test pins the Unicode version of the dependency, so that an
update also updates the `GraphemeBreakTest.txt` fixture.

### 5.7 Mixed languages

One call is one language, the language of the surrounding text. The language is never detected from
the text: detection is nondeterministic at boundaries and wrong on loan words and names.

| Situation                                                    | Behavior                                                                                                                               |
| ------------------------------------------------------------ | -------------------------------------------------------------------------------------------------------------------------------------- |
| quotes around foreign words (`"hello"` in ru)                | quotes of the language of the call: `«hello»`; quotes follow the surrounding text                                                      |
| dash style, ellipsis forms, number separators                | those of the language of the call, for the whole text (`What?...` in ru text → `What?..`)                                              |
| short words, particles, abbreviations, units, months         | compared with the lists of the language; a word of another script never matches                                                        |
| Latin `a`, `I`, `to`, `ls` in Russian text                   | no glue after them: not in the ru lists, and length rules count only Cyrillic words (`short_words_any_script(false)`)                  |
| Cyrillic `и`, `в` in English text                            | no glue: not in the en lists                                                                                                           |
| Russian preposition before a Latin word                      | glue is set: the trigger is the Russian word (`в⍽db`, `для⍽id`)                                                                        |
| apostrophe `don't` in Russian text                           | `don’t`: an apostrophe between letters is U+2019 in every language (Unicode chapter 6)                                                 |
| a word of two scripts (`Мoсква`, `Cервер`)                   | never changed; `MixedScript` diagnostic (warning)                                                                                      |
| `-` between two words of another script (`cat - file` in ru) | the `-` stays: words outside the language's script or technical fragments on both sides are a refusal; `AmbiguousHyphen` (section 8.4) |
| a quotation in another language                              | the caller typesets it with another typographer and passes it as `Piece::Protected`                                                    |

Letters are never changed: no transliteration, no "fixing" of look-alike letters (section 10.5).
