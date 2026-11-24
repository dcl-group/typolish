# typolish

`typolish` is a Rust crate that typesets plain text for a language. Text typed the easy way, with
straight quotes, hyphens, three full stops and ordinary spaces, becomes typeset text: quotes by
nesting level, apostrophes, dashes, ellipses, no-break spaces, and rules for words, signs and
numbers. The built-in languages are `ru-RU` and `en-US`; other languages are added as data, within
the limits listed under Languages.

- Three inputs: a whole string, a message template made of text pieces and opaque placeholders whose
  kind is known and whose value is not, and a number to format.
- Output as Unicode, as HTML text (entities or nowrap elements), or as the escaped body of a string
  literal for JavaScript, JSON, Rust or CSS; the output kind is given with every call.
- 27 rules with stable ids, two ready sets (`PROSE` and `INTERFACE`), and sets of your own.
- Protection of commands, paths and code: backticks, delimiters of your own, protected pieces, and
  technical fragments the crate recognizes by itself.
- Deterministic, and idempotent except for the cases of straight quotes listed under Known limits;
  an unchanged string is returned borrowed and, after the first call on a thread, without allocation
  for an input up to about 50 KiB.
- Explanation of every edit, a check of plain sources, and the reverse transformation.
- No `unsafe`, no regular expressions, no panics, and nothing read from the environment.

## Status

Version 0.1.1 is prepared and not published yet. The design is in
[`docs/design/`](https://github.com/dcl-group/typolish/tree/main/docs/design). The minimum supported
Rust version is 1.100.

## Usage

A `Typographer` is built once with a language and a rule set and then only applied. It is immutable,
`Send + Sync`, and can live in a `static`:

```rust
use std::borrow::Cow;
use typolish::{Language, Output, RuleSet, Typographer};

static RU_UI: Typographer = Typographer::new(Language::RU_RU, RuleSet::INTERFACE);
static EN: Typographer = Typographer::new(Language::EN_US, RuleSet::PROSE);

assert_eq!(
    RU_UI.typeset("Он сказал \"привет\" - и ушёл...", Output::Unicode),
    "Он сказал «привет»\u{a0}— и\u{a0}ушёл…"
);
assert_eq!(
    EN.typeset("\"Don't\" - she said...", Output::Unicode),
    "“Don’t” — she said…"
);
// Nothing to change: the input comes back borrowed.
assert!(matches!(RU_UI.typeset("Готово", Output::Unicode), Cow::Borrowed(_)));
```

Use `RuleSet::PROSE` for running text written for reading, and `RuleSet::INTERFACE` for UI strings,
error messages and technical text, where a command or a path may stand as its own fragment.

Rules and settings are chosen when the typographer is built; a rule set can also be parsed from a
configuration string:

```rust
use typolish::{Language, Output, PercentSpacing, Rule, RuleSet, Typographer};

fn main() -> Result<(), typolish::Error> {
    let rules: RuleSet = "interface,+symbols.copyright".parse()?;
    let t = Typographer::builder(Language::RU_RU, rules)
        .disable(Rule::NbspShortWords)
        .percent(PercentSpacing::Spaced)
        .add_units(["ядер"])
        .build()?;
    assert_eq!(
        t.typeset("(c) 2026, скидка 5% на 8 ядер", Output::Unicode),
        "© 2026, скидка 5\u{a0}% на 8\u{a0}ядер"
    );
    Ok(())
}
```

### Message templates

A template is a sequence of text pieces and slots. The crate knows the kind of each slot (number,
word, date, other) but never its value; the rules treat a slot as an opaque element of its kind, and
the result keeps every slot in its place:

```rust
use typolish::{Language, Output, Piece, RuleSet, Slot, Typographer};

static RU_UI: Typographer = Typographer::new(Language::RU_RU, RuleSet::INTERFACE);

let t = RU_UI.typeset_template(
    [
        Piece::Text("Удалено "),
        Piece::Slot("n", Slot::NUMBER),
        Piece::Text(" файлов из папки \""),
        Piece::Slot("dir", Slot::WORD),
        Piece::Text("\""),
    ],
    Output::Unicode,
);
assert_eq!(t.texts(), ["Удалено ", " файлов из\u{a0}папки «", "»"]);
assert_eq!(t.slots(), [("n", Slot::NUMBER), ("dir", Slot::WORD)]);
```

How each slot kind takes part in the rules is the slot participation table of
[design section 2.6](https://github.com/dcl-group/typolish/blob/main/docs/design/02-public-api.md).

A build-time generator over messages already mapped from Fluent syntax to pieces (design section
2.14) is in
[`examples/fluent_generator.rs`](https://github.com/dcl-group/typolish/blob/main/dev/typolish/examples/fluent_generator.rs).

### Numbers

`format_number` and `format_range` write a number by the language: group separator, decimal
separator, minus sign and range dash. They return a display that allocates nothing:

```rust
use typolish::{HtmlStyle, Language, Number, Output, RuleSet, Typographer};

static RU: Typographer = Typographer::new(Language::RU_RU, RuleSet::INTERFACE);
static EN: Typographer = Typographer::new(Language::EN_US, RuleSet::INTERFACE);

assert_eq!(RU.format_number(1234567.5, Output::Unicode).to_string(), "1\u{a0}234\u{a0}567,5");
assert_eq!(EN.format_number(1234567.5, Output::Unicode).to_string(), "1,234,567.5");
assert_eq!(EN.format_number(Number::fixed(-0.125, 2), Output::Unicode).to_string(), "\u{2212}0.12");
assert_eq!(RU.format_range(10, 20, Output::Unicode).to_string(), "10–20");
assert_eq!(
    RU.format_number(-1234.5, Output::Html(HtmlStyle::NAMED)).to_string(),
    "&minus;1&nbsp;234,5"
);
```

### Output kinds

```rust
use typolish::{EscapeStyle, HtmlStyle, Language, Output, RuleSet, Typographer};

static RU_UI: Typographer = Typographer::new(Language::RU_RU, RuleSet::INTERFACE);
const TEXT: &str = "Он сказал \"привет\" - и ушёл...";

assert_eq!(
    RU_UI.typeset(TEXT, Output::Html(HtmlStyle::NAMED)),
    "Он сказал &laquo;привет&raquo;&nbsp;&mdash; и&nbsp;ушёл&hellip;"
);
// Where a no-break space cannot be written as a character: `white-space: nowrap`.
assert_eq!(
    RU_UI.typeset(TEXT, Output::Html(HtmlStyle::NAMED.nowrap_class("nowrap"))),
    "Он сказал <span class=\"nowrap\">&laquo;привет&raquo; &mdash;</span> \
     <span class=\"nowrap\">и ушёл&hellip;</span>"
);
assert_eq!(
    RU_UI.typeset(TEXT, Output::Escaped(EscapeStyle::JS)),
    "Он сказал \\u00ABпривет\\u00BB\\u00A0\\u2014 и\\u00A0ушёл\\u2026"
);
```

### Explanation, source check, reverse transformation

`report` returns the result together with every edit, the protected spans and the diagnostics;
`lint` checks a source that must stay plain; `to_plain` maps typeset text back to the plain form:

```rust
use typolish::{Language, Output, Rule, RuleSet, Typographer};

static RU_UI: Typographer = Typographer::new(Language::RU_RU, RuleSet::INTERFACE);

let report = RU_UI.report("Он сказал \"привет\" - и ушёл...", Output::Unicode);
let rules: Vec<Rule> = report.changes().iter().map(|c| c.rule()).collect();
assert_eq!(rules[0], Rule::QuotesPairs);
// One line per edit (rule, piece:range, before -> after), then one per protected span.
println!("{report}");

// An unpaired quote is left as it is and reported, never guessed.
let report = RU_UI.report("Ошибка: \"x", Output::Unicode);
assert_eq!(report.result(), "Ошибка: \"x");
assert_eq!(report.diagnostics()[0].rule(), Some(Rule::QuotesPairs));

assert_eq!(RU_UI.lint("Он сказал «привет»").len(), 2);
assert_eq!(
    RU_UI.to_plain("Он сказал «привет»\u{a0}— и\u{a0}ушёл…"),
    "Он сказал \"привет\" - и ушёл..."
);
```

## Protecting commands and code

The rules work on text, and a command typed in running text looks like text: in `cat - file` the `-`
reads as a dash. Mark such fragments, and the crate leaves them as they are:

- **Backticks**, on by default: `` `cat - file` `` and fenced blocks of three backticks are never
  typeset. `protect_backticks(false)` turns this off.
- **Delimiters of your own**: `protect_between("{{", "}}")` protects everything between the pair.
- **Protected pieces**: in a template, `Piece::Protected("cat - file")` is visible text that is
  never typeset; it acts as one word for its neighbors.

```rust
use typolish::{Language, Output, Piece, RuleSet, Slot, Typographer};

static EN: Typographer = Typographer::new(Language::EN_US, RuleSet::INTERFACE);

fn main() -> Result<(), typolish::Error> {
    assert_eq!(EN.typeset("Run cat - file", Output::Unicode), "Run cat—file");
    assert_eq!(EN.typeset("Run `cat - file`", Output::Unicode), "Run `cat - file`");

    let ru = Typographer::builder(Language::RU_RU, RuleSet::INTERFACE)
        .protect_between("{{", "}}")
        .build()?;
    assert_eq!(
        ru.typeset("Выполните {{git commit -m \"fix\"}} - и всё", Output::Unicode),
        "Выполните {{git commit -m \"fix\"}}\u{a0}— и\u{a0}всё"
    );

    let t = EN.typeset_template(
        [
            Piece::Text("Run "),
            Piece::Protected("cat - file"),
            Piece::Text(" in "),
            Piece::Slot("dir", Slot::OTHER),
        ],
        Output::Unicode,
    );
    assert_eq!(t.texts(), ["Run cat - file in ", ""]);
    Ok(())
}
```

Without marks, the crate still recognizes many technical fragments by their shape and leaves them
alone: URLs, e-mail addresses, flags (`-v`, `--force`), paths, `host:port`, operators, UUIDs, hex
numbers, ISO dates, versions, file and host names, identifiers such as `x86` or `kv_lang`.
`Report::protected` lists what was frozen and why.

## Languages

`Language::RU_RU` and `Language::EN_US` are built in; `Language::COMMON` applies only the rules that
need no language data. `Language::from_tag` returns an error for a language without data, and the
caller chooses whether to fall back to `COMMON`. A caller that falls back tries the full tag, then
the bare language subtag (`Language::from_tag("en")` gives `en-US`), and only then
`Language::COMMON`, which has no quotes and no dashes. A new language is a table of data (quote
pairs, short words, particles, abbreviations, units, months, number symbols), declared as a `const`
`LanguageData` or built at run time with `LanguageBuilder`:

```rust
use typolish::{Language, LanguageBuilder, Output, QuotePairs, RuleSet, Typographer};

fn main() -> Result<(), typolish::Error> {
    let de = LanguageBuilder::from_language(&Language::EN_US, "de-DE")
        .quotes(Some(QuotePairs::new(('„', '“'), ('‚', '‘'))))
        .short_words(["und", "in", "zu"])
        .build()?;
    let t = Typographer::new(de, RuleSet::INTERFACE);
    assert_eq!(t.typeset("\"Ja\" und \"Nein\"", Output::Unicode), "„Ja“ und\u{a0}„Nein“");
    Ok(())
}
```

The four quote glyphs of a language are distinct and taken from « » „ “ ” ‘ ’ ‚ ‹ › (design section
7.6). A language whose opening and closing quote is the same character (Swedish ”…”) or whose quotes
are outside this list (CJK 「…」) cannot be declared yet: `LanguageBuilder::build` returns
`Error::InvalidLanguageData`.

## Known limits

The rules read the shape of the text, not its meaning. These cases are known and deliberate:

- A lone `-` between two words of the language is a dash in both sets. In English text `cat - file`
  becomes `cat—file`, and `tls-server - agent-ca` becomes `tls-server — agent-ca`; in Russian text
  both stay (foreign words on both sides). Next to a flag a `-` is never a dash, even where one was
  meant (`флаг -v - он включает вывод`). Commands with spaces must be protected with backticks or
  passed as `Piece::Protected` (see above).
- With the closed style of English the dash keeps the author's spaces next to anything but a plain
  word (`Hello, — he said`, `Chapter — 5`, `See https://example.com — the guide`); only two plain
  words are joined (`word—word`).
- A capital letter with a full stop at a sentence end before a capitalized word gets a no-break
  space, as an initial before a surname would: `вариант А.⍽После`, `plan B.⍽Then`,
  `Neither did I.⍽Then`, `220⍽В.⍽Затем`. The glue changes no visible character; it only forbids a
  line break there.
- `rock 'n' roll` with spaces is a top-level single pair and stays straight by default;
  `rock'n'roll` gets apostrophes.
- In `PROSE`, `dash.double-hyphen` turns `git checkout -- file` into a dash in English text,
  `number.minus` turns `tail -5` into `tail −5`, `range.digits` turns a telephone fragment
  `123-4567` into a range, and `ellipsis.inner` turns `...args` into `…args`. `PROSE` assumes
  commands are protected; `INTERFACE` has none of these rules.
- A mention such as `@user` is frozen whole, like an e-mail address: rules never edit inside it.
- Swiss grouping `1'000'000` and digit separators `0b1'0000` are frozen and keep `'`.
- A number grouped with commas is read as a number only with one group and no decimal part
  (`1,234 kg` → `1,234⍽kg`): `1,234,567 kg` and `1,000.5 kg` are frozen, so `number.minus` and
  `nbsp.units` do not apply to them, also where `format_number` wrote them. Pass such a value as a
  `Slot::NUMBER`.
- Frozen fragments are left alone even where a rule would be welcome: `50%-ный`, `C#`, `AT&T`.
- A quote glued to a word by punctuation (`сказал:"привет"`, `сказал,"привет"`) belongs to a frozen
  fragment and stays straight; with the space (`сказал: "привет"`) the quotes are placed. An empty
  value glued the same way (`key:""`) leaves the paragraph's quotes straight with an
  `UnmatchedQuote` warning; `key=""` stays as written.
- The unit lists are a selection: `5 ГВт`, `5 м³` and `5 p.m.` are not glued, while `5⍽МВт`, `5⍽м²`
  and `5⍽kg` are. Add the units you need with `add_units`.
- Straight quotes nested out of order around a trailing possessive are not idempotent in English:
  `""s' 'x."'"` gives `“‘s’ 'x.’'”`, and a second pass pairs the remaining `'` (`“‘s’ ‘x.’’”`). For
  the same reason `to_plain` does not give back what typesets to the same text when an unpaired `'s`
  follows a nested trailing possessive: `"a "dogs' 's" b"` gives `“a⍽‘dogs’ 's’ b”`, and its plain
  form `"a "dogs" 's' b"` gives `“a⍽‘dogs’ ‘s’ b”`. This is a defect, not a choice.
- In a declared language whose primary pair is `‘’` (en-GB), `PROSE` is not idempotent on a
  sign-only pair inside straight single quotes before a number: `'a ""-3'` gives `'a⍽‘’-3'`, and a
  second pass gives `'a⍽‘’-3′`. This is a defect, not a choice.
- A quoted value with a space glued to `=` is cut by the space: `title="My` is frozen, the quote
  after `App` closes nothing, and `Set title="My App" for the "main" window.` keeps every quote of
  the paragraph straight with an `UnmatchedQuote` warning. Protect such a notation with backticks.
- `Piece::Markup` is inline markup and transparent for rules: a block tag passed as `Markup` ends no
  paragraph, so `"да`, `</p><p>`, `Нет"` pair across the tag, and a dialogue dash right after it is
  not placed. Pass each block in a call of its own, or end the `Text` piece before a block tag with
  `\n\n`.
- A quote glued to a dash (`он —"Да"`, `"no"—and`) belongs to a frozen `Unclassified` core and stays
  straight, without a diagnostic; with a space (`он — "Да"`) the quotes are placed.
- A number with a unit attached without a space (`-15°C`) is not a number core: it is frozen whole
  and keeps its hyphen, while `-15 °C` gets the minus and the glue in `PROSE` (`−15⍽°C`).
- A time range with spaces (`10:00 - 12:00`) is not a dash: both sides are technical fragments; the
  hyphen stays and `AmbiguousHyphen` is reported. `10:00-12:00` gets the en dash in `PROSE`.
- A chain of short words is glued whole, with no limit on its length (`Он⍽и⍽я⍽и⍽ты⍽и⍽мы`, Russian
  `PROSE`): a text made of words of one or two letters has no place to break a line.
- After a number a capital `В` or `А` is read as a unit (volt, ampere) and glued, and the word after
  it in turn: `Часть 3⍽В⍽лесу`. The glue changes no visible character.
- A percentage range with a hyphen and the sign attached (`10-20%`) is a frozen fragment and keeps
  its hyphen; with a space (`10-20 %`) it gets the en dash in `PROSE` (`10–20⍽%`).

Here `⍽` stands for a no-break space. The full list, with the reasons, is in section 10.8 of the
design.

## License

Licensed under either of [MIT](./LICENSE-MIT) or [Apache-2.0](./LICENSE-APACHE) at your option.
