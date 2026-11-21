# typolish design: Unicode and mixed languages

Part of the [typolish design](./README.md). Section numbers are those of the whole design.

## 5. Unicode and mixed languages

### 5.1 Grapheme clusters

The lexer walks extended grapheme clusters (UAX #29) through `unicode-segmentation` (section 14.1).
Without clusters a combining accent would split a word: `char::is_alphabetic` is false for U+0301,
so `за́мок` would become two words and the short-word lists would misfire.

Rules work on whole clusters:

- A replacement replaces a whole cluster. A cluster of several characters led by a non-letter is a
  `Symbol`: a straight quote with a combining mark attached (`"` + U+0301) is not a quote candidate,
  and a keycap (`1️⃣`, `#️⃣`) is not a number or a sign. The ASCII fast path of section 3.5 hands the
  ASCII bytes on both sides of a non-ASCII character to the segmenter, so this holds right before
  and right after ASCII text too.
- An insertion (`т.е.` → `т.⍽е.`, `№5` → `№⍽5`) is made only between clusters.
- Glue is set only on a `Space` token, which is a whole cluster.
- Emoji sequences (ZWJ, skin-tone modifiers, flags, keycaps, tag sequences) are single `Symbol`
  tokens; for neighbouring quotes they behave as words: `"🙂"` → `«🙂»`.

Every edit position is therefore a cluster boundary of the input. A test runs every line of the
Unicode `GraphemeBreakTest.txt` for the Unicode version of the dependency (17.0.0) through both rule
sets, alone and inside sentences, and checks with the report that no edit starts or ends inside a
cluster and that every cluster of the input appears whole in the output (section 13.6).

Rejected: a hand-made table of `Extend` characters (it would miss `SpacingMark`, `Prepend`, regional
indicators and Hangul syllable sequences, and it would need manual updates with every Unicode
version); grapheme support behind a feature (Cargo unifies features across the dependency graph, so
a foreign crate enabling it would silently change everybody's output).

### 5.2 Letters, digits and scripts

- **Letters** are characters with `char::is_alphabetic()`, together with the combining marks of
  their cluster.
- **Script** of a letter comes from fixed ranges in code: Cyrillic U+0400–U+052F, U+1C80–U+1C8F,
  U+2DE0–U+2DFF, U+A640–U+A69F; Latin ASCII letters, U+00C0–U+024F, U+1E00–U+1EFF, U+2C60–U+2C7F,
  U+A720–U+A7FF; Greek U+0370–U+03FF, U+1F00–U+1FFF; everything else `Other`. A word whose letters
  come from more than one script is `Mixed` (`Мoсква` with a Latin `o`). The ranges are a constant
  table, so the script of a letter does not depend on the Rust version. The `MixedScript` diagnostic
  is not raised for a core that matched a dictionary entry (check 0 of section 3.6), so the
  language's own units pass silently: `μs` (Greek U+03BC, Latin `s`) and `µs` (U+00B5, which falls
  outside the Latin ranges).
- **Digits** for number rules are ASCII `0` to `9` only. Digits of other systems are word
  characters. Digits break a word: `x86` is a Latin word and a number in one fragment (check 20 of
  section 3.6 freezes it).

### 5.3 Spaces, line breaks, paragraphs

- U+0020 SPACE is the only character that a rule turns into a no-break space, and only when it
  stands alone between two fragments.
- U+00A0 and U+202F in the input are spaces that are already glued (section 3.9); rules use them as
  context and never change them. U+2009 THIN SPACE and other horizontal whitespace (tab, U+2000 to
  U+200A except as noted) are spaces that separate fragments and are never glued or removed.
- Line breaks: `\n`, `\r\n`, `\r`, U+0085, U+2028. U+2029 is a paragraph break. A paragraph boundary
  is two or more line breaks with only horizontal whitespace between them (section 3.3). Line breaks
  are never glued.
- A space cluster with a combining mark attached (U+0020 + U+0301) is a frozen `Space`: it separates
  fragments, is never glued or removed, and the mark stays. A no-break rule that would have glued it
  simply does not, without a diagnostic.

### 5.4 Format characters and Fluent isolation marks

Format characters are U+00AD SOFT HYPHEN, U+200B ZERO WIDTH SPACE, U+200C to U+200F (ZWNJ, ZWJ
outside an emoji cluster, LRM, RLM), U+202A to U+202E (embeddings and overrides), U+2060 to U+2064
(word joiner and invisible operators), U+2066 to U+2069 (isolates: LRI, RLI, FSI, PDI) and U+FEFF.
They form `Format` tokens that are transparent for context: rules skip them when they look for
neighbours, never insert or remove them, and never put them inside an edit. Inside a fragment the
classifier and the list lookups skip them too (section 3.6), so a word with soft hyphens from a
hyphenation preprocessor is still a word: `21 ноя\u{AD}бря` gets its glue, and `Мо\u{AD}скве` is a
`Word`, not `Unclassified`. UAX #29 makes U+00AD a cluster of its own, so the characters stay
exactly where they were.

This matters for strings that `fluent-bundle` formats at run time: by default it wraps every
placeable in U+2068 FIRST STRONG ISOLATE and U+2069 POP DIRECTIONAL ISOLATE. With transparency,
`Файл "⁨report⁩"` gets quotes around the isolated word, `в ⁨Москве⁩` gets glue after `в`, and
`⁨5⁩ МБ` gets glue before the unit (the isolate around `5` is skipped, the number is seen). A test
table typesets such strings and checks the same results as without the isolates, with the isolates
unchanged (section 13.6). U+2060 WORD JOINER in the input is kept and is not treated as glue (a
caller who wants glue writes U+00A0).

### 5.5 No normalization

The crate does not normalize Unicode (NFC needs large tables). A word in NFD (`и` + U+0306 instead
of `й`) does not match a language list, so it gets no no-break space; nothing breaks. The rustdoc of
the crate states this; callers that need matching normalize before typesetting.

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

| Situation                                                    | Behavior                                                                                                              |
| ------------------------------------------------------------ | --------------------------------------------------------------------------------------------------------------------- |
| quotes around foreign words (`"hello"` in ru)                | quotes of the language of the call: `«hello»`; quotes follow the surrounding text                                     |
| dash style, ellipsis forms, number separators                | those of the language of the call, for the whole text (`What?...` in ru text → `What?..`)                             |
| short words, particles, abbreviations, units, months         | compared with the lists of the language; a word of another script never matches                                       |
| Latin `a`, `I`, `to`, `ls` in Russian text                   | no glue after them: not in the ru lists, and length rules count only Cyrillic words (`short_words_any_script(false)`) |
| Cyrillic `и`, `в` in English text                            | no glue: not in the en lists                                                                                          |
| Russian preposition before a Latin word                      | glue is set: the trigger is the Russian word (`в⍽db`, `для⍽id`)                                                       |
| apostrophe `don't` in Russian text                           | `don’t`: an apostrophe between letters is U+2019 in every language (Unicode chapter 6)                                |
| a word of two scripts (`Мoсква`, `Cервер`)                   | never changed; `MixedScript` diagnostic (warning)                                                                     |
| `-` between two words of another script (`cat - file` in ru) | not a dash, nor next to a flag or between technical fragments; `AmbiguousHyphen` (section 8.4)                        |
| a quotation in another language                              | the caller typesets it with another typographer and passes it as `Piece::Protected`                                   |

Letters are never changed: no transliteration, no "fixing" of look-alike letters (section 10.5).
