# typolish design: Output kinds

Part of the [typolish design](./README.md). Section numbers are those of the whole design.

## 11. Output kinds

### 11.1 Principle

The output kind is an argument of every call (section 2.8). It is an encoding of the result, not
HTML parsing: the core still works on plain text, so `Output::Html` lives in the core while the HTML
module (stage 6) stays out. Rules always work in Unicode and never see the output kind; the encoder
of `src/output.rs` runs while rendering and writes straight into the output string, with no
intermediate Unicode string.

The encoder treats a character the same way whether a rule produced it or it was in the input, so
the output is uniform. It applies to `Text` and `Protected` alike (protection means "not typeset",
not "not encoded") and never touches slots. `Markup` pieces are written as is in Unicode and HTML
output and escaped like text in escaped output, where they are part of the string.

Slot values are not encoded and not checked: the crate never sees them. The caller encodes a value
for the string it is inserted into. With `fill` (section 2.6) that string is still encoded output,
so the value is encoded for the output kind of the template (HTML-escaped for `Output::Html`,
escaped for the string syntax for `Output::Escaped`), and a value from `format_number` or
`format_range` called with that output kind is already encoded. In generated code (section 2.14) an
`Output::Escaped` text becomes a plain Unicode string when the program is compiled or loaded, so
run-time values are inserted as plain Unicode with no escaping, and `format_number` and
`format_range` are called with `Output::Unicode`; for an `Output::Html` template the run-time string
is HTML, so values are HTML-escaped and numbers are formatted with the same `Output::Html`. The
safety statements of sections 11.4 and 11.6 and the decoding guarantee of section 11.8 are about the
texts the crate writes. A value with spaces inserted into a nowrap element (section 11.5) cannot
break inside the element.

### 11.2 Typographic set

The characters the rules produce, and the same characters in the input. The quotes row holds every
character that section 7.6 allows as a quote glyph (the `Quote` characters of section 3.5 other than
`"`, `'`, `′` and `″`), so quote pairs set by the caller (`TypographerBuilder::quote_pairs`,
`LanguageBuilder`) are covered too; a conformance check (section 13.9) fails when such a glyph is
missing from the row:

| Character             | Code points                                                                    | `HtmlStyle::NAMED`                                                                                    | `EscapeStyle::JS` |
| --------------------- | ------------------------------------------------------------------------------ | ----------------------------------------------------------------------------------------------------- | ----------------- |
| no-break space        | U+00A0                                                                         | `&nbsp;`                                                                                              | `\u00A0`          |
| narrow no-break space | U+202F                                                                         | `&#x202F;` (no name)                                                                                  | `\u202F`          |
| thin space            | U+2009                                                                         | `&thinsp;`                                                                                            | `\u2009`          |
| non-breaking hyphen   | U+2011                                                                         | `&#x2011;` (no name)                                                                                  | `\u2011`          |
| en and em dash        | U+2013, U+2014                                                                 | `&ndash;` `&mdash;`                                                                                   | `\u2013` `\u2014` |
| ellipsis              | U+2026                                                                         | `&hellip;`                                                                                            | `\u2026`          |
| quotes                | U+00AB, U+00BB, U+201E, U+201C, U+201D, U+2018, U+2019, U+201A, U+2039, U+203A | `&laquo;` `&raquo;` `&bdquo;` `&ldquo;` `&rdquo;` `&lsquo;` `&rsquo;` `&sbquo;` `&lsaquo;` `&rsaquo;` | `\u00AB` ...      |
| minus, times          | U+2212, U+00D7                                                                 | `&minus;` `&times;`                                                                                   | `\u2212` `\u00D7` |
| primes                | U+2032, U+2033                                                                 | `&prime;` `&Prime;`                                                                                   | `\u2032` `\u2033` |
| legal marks           | U+00A9, U+00AE, U+2122                                                         | `&copy;` `&reg;` `&trade;`                                                                            | `\u00A9` ...      |
| arrows                | U+2192, U+2190, U+2194                                                         | `&rarr;` `&larr;` `&harr;`                                                                            | `\u2192` ...      |
| math                  | U+2265, U+2264, U+2260, U+00B1                                                 | `&ge;` `&le;` `&ne;` `&plusmn;`                                                                       | `\u2265` ...      |

Every name and every absence of a name in the table was checked against the WHATWG list of named
character references on 2026-11-21 (section 14.8); the table is a `const` in `src/output.rs`, and a
test compares it with `tests/fixtures/whatwg-entities.txt`, an extract of that list (section 13.2).
`№`, `§`, `¶` and `°` are not in the set: no rule produces them, and they are written as characters.

### 11.3 Unicode

`Output::Unicode`: characters as they are; glue as U+00A0 or U+202F. Nothing is escaped.

### 11.4 HTML

- `&` → `&amp;`, `<` → `&lt;`, `>` → `&gt;`, `"` → `&quot;`, `'` → `&#39;` everywhere in text and
  protected spans. The output is safe both as element content and as an attribute value in either
  kind of quotes. Without this, `<word>` would become a tag and a literal `&nbsp;` in the text could
  not be told from the entity; escaping after the crate would ruin its own entities.
- Characters of the typographic set: `HtmlStyle::NAMED` writes named references where HTML has them
  and hexadecimal references otherwise; `DECIMAL` writes `&#160;`, `&#8212;`; `HEX` writes `&#xA0;`,
  `&#x2014;`; `CHARACTERS` writes the characters themselves (only markup is escaped). Cyrillic and
  other text stays UTF-8 in every style: encoding all non-ASCII would bloat HTML for nothing.
- Glue: as the entity of its character, or as nowrap groups (section 11.5).

### 11.5 nowrap groups

`HtmlStyle::nowrap_class(c)` and `HtmlStyle::nowrap_style()` render no-break groups as elements
instead of entities. A **group** is a maximal run of non-space fragments joined by glue marks; its
**joints** are those glue marks and every U+2011 inside its fragments. Protected spans never belong
to a group. A group with at least one joint is written as `<span class="c">…</span>` or as
`<span style="white-space:nowrap">…</span>`. Inside it, U+00A0 glue becomes an ordinary space;
U+202F glue, U+2011 and the other characters of the set are written as references of the chosen
style (`&#x202F;`, `&#x2011;`, `&mdash;`; the characters themselves with `CHARACTERS`). So the
narrow space keeps its width, a hyphen the caller typed as U+2011 comes back as U+2011, and an
ordinary space inside the element can only be U+00A0 glue, which keeps decoding exact (section
11.8). The class string is escaped as an attribute value. Groups do not nest.

- Example: `и⍽т.⍽д.` → `<span class="nowrap">и т. д.</span>`; a telephone number `+7 495 123-45-67`
  glued by `nbsp.phone` (Prose) → `<span class="nowrap">+7 495 123-45-67</span>`; the same number
  typed by the caller with U+00A0 and U+2011 →
  `<span class="nowrap">+7 495 123&#x2011;45&#x2011;67</span>`.
- Numbers: `format_number` and `format_range` in nowrap mode write one element around the whole
  result when it contains glue (a group separator U+00A0 or U+202F), and no element otherwise: ru
  `-12345` → `<span class="nowrap">&minus;12 345</span>`, ru `123` → `123`, en `1234` → `1,234`.
- Templates: the n + 1 form (n slots and n + 1 texts, `texts()[i]` before `slots()[i]`, section 2.6)
  always has room for the tags. A group that starts with slot i opens at the end of `texts()[i]`; a
  group that ends with slot j closes at the start of `texts()[j + 1]`; a slot inside a group stays
  inside the element (`в {city}` → texts `["<span class=\"nowrap\">в ", "</span>"]`).
- Splits: a group that crosses a `Markup` piece is split there, because a span cannot be opened
  across foreign markup; the glue at the split is written as an entity, and `report` adds
  `NowrapFallback` (info). A group also ends at a protected span, without a diagnostic: the span's
  content may be long and has spaces of its own, which an element would make unbreakable and a
  decoder could not tell from glue (the reason no glue sticks to an `Other` slot, section 2.6). The
  glue between a group and a protected span is written as an entity (`в&nbsp;` before
  `` `--config` ``), and so is glue inside a protected span.

### 11.6 Escaped string bodies

`Output::Escaped(EscapeStyle)` writes the body of a string literal of the chosen syntax: the caller
pastes it between quotes with no escaping of its own. The container is always escaped completely,
because the crate leaves straight quotes it could not pair, and protected spans may contain
backslashes; escaping after the crate would ruin its own escapes.

| What                                                    | `JS`                                                    | `JS_HEX`                                                           | `JSON`                       | `RUST`                |
| ------------------------------------------------------- | ------------------------------------------------------- | ------------------------------------------------------------------ | ---------------------------- | --------------------- |
| `\`                                                     | `\\`                                                    | `\\`                                                               | `\\`                         | `\\`                  |
| `"`                                                     | `\"`                                                    | `\"`                                                               | `\"`                         | `\"`                  |
| `'`, backtick                                           | `\u0027`, `\u0060`                                      | `\x27`, `\x60`                                                     | as is                        | as is                 |
| `<`, `>`, `&`                                           | `\u003C`, `\u003E`, `\u0026`                            | `\x3C`, `\x3E`, `\x26`                                             | `\u003c`, `\u003e`, `\u0026` | as is                 |
| `$` before `{`                                          | `\u0024`                                                | `\x24`                                                             | as is                        | as is                 |
| `\n`, `\r`, `\t`                                        | `\n`, `\r`, `\t`                                        | `\n`, `\r`, `\t`                                                   | `\n`, `\r`, `\t`             | `\n`, `\r`, `\t`      |
| other controls (U+0000 to U+001F, U+007F)               | `\u0000`                                                | `\x00`                                                             | `\u0000`                     | `\0`, `\u{1}`         |
| U+2028, U+2029                                          | `\u2028`, `\u2029`                                      | `\u2028`, `\u2029`                                                 | `\u2028`, `\u2029`           | as is                 |
| typographic set (or every non-ASCII with `non_ascii()`) | `\u00A0`; above the BMP a surrogate pair `\uD83D\uDE00` | `\xA0` up to U+00FF, `\u2014` above, surrogate pairs above the BMP | `\u00a0`, surrogate pairs    | `\u{a0}`, `\u{1f600}` |

- `JS` and `JS_HEX` use only escapes valid in every JavaScript string form, so the result is safe in
  single-quoted, double-quoted and template literals and inside an HTML `<script>` element. `JS_HEX`
  is the `\xHH` form (`\x00`), with the same complete container escaping.
- `JSON` uses only escapes that JSON defines and also escapes U+2028, U+2029, `<`, `>` and `&`, so
  the result is safe to embed in a script element too.
- `RUST` is for code generated at build time (section 2.14): the body of a normal (non-raw) string
  literal.
- `JS_CODE_POINT` escapes the same characters as `JS` in the ES2015 form, with uppercase hex digits
  and no leading zeros (`\u{A0}`, `\u{27}` for `'`, `\u{0}` for U+0000, `\u{2028}`, `\u{1F600}`
  above the BMP, no surrogate pairs), for codebases that target modern engines only.
- `CSS` writes the body of a CSS string (`content: "…"`): `\` → `\\`, `"` → `\"`, `'` → `\'`, the
  controls U+0001 to U+001F and U+007F (among them tab and LF, CR and FF, the newlines of CSS) and
  `<` → a hexadecimal escape followed by a space (`\a `, `\c `, `\3c `); U+0000, which CSS cannot
  carry (it reads both a raw U+0000 and `\0 ` as U+FFFD), → `\fffd `; U+0085, U+2028 and U+2029 are
  not newlines in CSS and are written as they are (escaped only with `non_ascii()`); the typographic
  set → the same form (`\a0 `, `\2014 `). The trailing space ends the escape and is part of it, as
  CSS defines.
- With the default scope only the typographic set and the characters above are escaped; Cyrillic and
  other text, and the C1 controls U+0080 to U+009F (U+0085 among them), stay UTF-8. `non_ascii()`
  escapes every non-ASCII character for pipelines that require ASCII.
- The prefilter (section 3.2) takes the bytes that a style encodes from this same table, so text
  that needs escaping is never returned borrowed: `a</script>b` in `JS`, `AT&T` in `JSON` and `Да`
  with `JS.non_ascii()` always come out escaped.

Rejected: an escape mode that leaves `\` and quotes of the input alone (the result could break or
change the meaning of the literal, and a second escaping by the caller would ruin `\u00A0`).

### 11.7 Numbers

`format_number` and `format_range` use the same encoders: the group separator U+00A0 becomes
`&nbsp;` in HTML, `\u00A0` in JS, `\u{a0}` in Rust; the minus becomes `&minus;`, `\u2212`,
`\u{2212}`; nowrap mode wraps the whole result in one element when it contains glue (section 11.5).

### 11.8 Idempotence and decoding

Idempotence holds for `Output::Unicode`. For the other kinds the guarantee is
`decode(typeset(x, out)) == typeset(x, Unicode)` and
`typeset(decode(typeset(x, out)), out) == typeset(x, out)`, with `decode` the inverse of the encoder
(section 4.6). The one exception is `Output::Escaped(EscapeStyle::CSS)` on text with U+0000: CSS has
no way to write it, so the encoder writes `\fffd ` (section 11.6), which a CSS reader and the CSS
test decoder (a strict parser of CSS) read as U+FFFD. The encoder writes a raw U+FFFD as it is (as
`\fffd ` with `non_ascii()`), so no decoder can tell the two apart. For `CSS` both equations
therefore run on `x` with every U+0000 replaced by U+FFFD; the encoding of U+0000 itself is checked
by the fixed row `a\u{0}b` → `a\fffd b` of section 13.2. Both are property tests with decoders
written as test helpers (section 13.5), for every kind including the nowrap styles: the decoder
turns an element back into its content, reads an ordinary space inside it as U+00A0 and every
reference as its character. Encoded output fed back as input is plain text with literal ampersands
and backslashes, and is encoded again; this is documented, not a defect.
