# typolish design: Public API

Part of the [typolish design](./README.md). Section numbers are those of the whole design.

## 2. Public API

Signatures below are sketches without bodies and without rustdoc. Every public item gets rustdoc
with an example in stage 2 to 4 (`missing_docs = "deny"`). None of the examples has been compiled as
a whole; a sketch of the load-bearing parts (a `const fn` constructor used in a `static`, `const`
language data built by `const fn` setters in another crate, a thread-local buffer with `const`
initialization, the `Send + Sync` check) was compiled with `rustc 1.100.0` (section 14.8).

### 2.1 Surface

| Item                                                                                         | Kind                                                                                      | Purpose                                                                             |
| -------------------------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------- | ----------------------------------------------------------------------------------- |
| `Typographer`                                                                                | struct, private fields, `Clone + Debug + Send + Sync`                                     | immutable typographer (one write-once cache, section 2.4); every application method |
| `TypographerBuilder`                                                                         | struct, `#[must_use]`                                                                     | settings, `build()`                                                                 |
| `Language`                                                                                   | struct, private fields, `Clone + Debug + Eq + Hash`                                       | a language: built-in constants or caller data                                       |
| `LanguageData`                                                                               | struct, traits of `Language`, `const fn` setters                                          | the data table of a language, built at compile time (section 7)                     |
| `LanguageBuilder`                                                                            | struct, `#[must_use]`                                                                     | a language built at run time from owned lists (section 7.1)                         |
| `Rule`                                                                                       | `#[non_exhaustive]` enum, `Copy`                                                          | a rule with a stable string id                                                      |
| `RuleSet`                                                                                    | struct, `Copy`, `const` methods, `FromStr`                                                | a set of rules; `PROSE`, `INTERFACE`, `EMPTY`, `ALL`                                |
| `Piece<'t, S>`                                                                               | `#[non_exhaustive]` enum                                                                  | one input piece of a template; `S` is the caller's slot payload                     |
| `Slot`, `SlotKind`                                                                           | struct (private field), `#[non_exhaustive]` enum                                          | an opaque placeholder and its kind                                                  |
| `Template<'t, S>`, `TemplatePart`                                                            | struct, `#[non_exhaustive]` enum                                                          | typeset template: n slots and n + 1 texts                                           |
| `Number<'n>`                                                                                 | struct, `Copy`                                                                            | a number to format: integers, floats, decimal strings                               |
| `NumberDisplay`, `RangeDisplay`                                                              | struct, `Display`                                                                         | lazy formatting without allocation                                                  |
| `Output`                                                                                     | `#[non_exhaustive]` enum, `Copy`                                                          | output kind, given with every call                                                  |
| `HtmlStyle`, `EscapeStyle`                                                                   | struct, `Copy`, `const` constructors                                                      | parameters of the HTML and escaped outputs                                          |
| `Report<'a, T>`, `Change`                                                                    | struct                                                                                    | result plus edits, protected spans and diagnostics                                  |
| `ProtectedSpan`, `ProtectedKind`, `TechKind`                                                 | struct, `#[non_exhaustive]` enums                                                         | what was recognized as protected and why                                            |
| `Diagnostic`, `DiagnosticKind`, `Severity`                                                   | struct, `#[non_exhaustive]` enums                                                         | messages: unpaired quote, ambiguous hyphen, mixed script, ...                       |
| `QuotePairs`, `SingleQuotes`, `DashStyle`, `PercentSpacing`, `NarrowGlue`                    | `Copy + Debug + Eq + Hash` value types                                                    | values of settings                                                                  |
| `Script`, `DashData`, `EllipsisForms`, `Abbreviation`, `Bind`, `MonthPosition`, `NumberData` | as above, with `const` constructors; `NumberData: Default` (equal to `NumberData::new()`) | parts of `LanguageData` (section 7.1)                                               |
| `Scratch`                                                                                    | struct, `Default`, opaque `Debug`, `const fn new`, `Send + Sync`                          | reusable working memory for callers that manage it themselves                       |
| `Error`                                                                                      | `#[non_exhaustive]` enum, `thiserror`                                                     | configuration and parsing errors                                                    |

Everything else (lexer, classifier, phases, encoders) is `pub(crate)`.

### 2.2 Language

```rust
/// A language: typographic data plus its tag.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Language { repr: Repr } // Static(&'static LanguageData) | Shared(Arc<owned data>);
                                   // `Repr` implements PartialEq, Eq and Hash by hand: equality
                                   // compares the data table whichever variant holds it, the hash
                                   // uses the tag only (consistent, because the tag is part of
                                   // the data)

impl Language {
    pub const RU_RU: Language;
    pub const EN_US: Language;
    /// No language data: only rules that need none are active (values: the table of section
    /// 7.4).
    pub const COMMON: Language;

    /// Resolves a BCP 47 tag: ASCII case-insensitive, `_` read as `-`. An exact tag matches;
    /// a bare language subtag matches its registered default region (`ru` -> `ru-RU`,
    /// `en` -> `en-US`). `und` resolves to `COMMON`, so `from_tag(l.tag())` gives `l` back for
    /// every built-in language and for `COMMON`. Anything else is `Error::UnknownLanguage`.
    pub fn from_tag(tag: &str) -> Result<Language, Error>;
    /// A language from caller data built at compile time, validated (section 7.6).
    pub fn from_data(data: &'static LanguageData) -> Result<Language, Error>;
    /// The same without validation, for a `const` or `static` typographer. Invalid data never
    /// panics, but rules may misbehave; validate the table once in a test with `from_data`.
    pub const fn from_static(data: &'static LanguageData) -> Language;
    /// Canonical tag, for example `"ru-RU"`; `"und"` for `COMMON`.
    pub fn tag(&self) -> &str;
    /// Built-in languages in tag order, `COMMON` excluded.
    pub fn builtin() -> impl Iterator<Item = Language>;
    /// Whether the language has the data a rule needs (section 7.4). A rule without data is inert.
    pub fn supports(&self, rule: Rule) -> bool;
}
/// The same as `from_tag`.
impl core::str::FromStr for Language { type Err = Error; }
impl core::fmt::Display for Language { /* the tag */ }
```

The caller chooses between an error and the common rules for an unknown language with one line;
there is no policy enum:

```rust
use typolish::{Error, Language};

// An explicit error.
assert!(matches!(Language::from_tag("de-DE"), Err(Error::UnknownLanguage { .. })));
// Common rules only.
let lang = Language::from_tag("de-DE").unwrap_or(Language::COMMON);
assert_eq!(lang.tag(), "und");
// A bare subtag resolves to the default region; a different region is not substituted.
assert_eq!(Language::from_tag("RU")?.tag(), "ru-RU");
assert!(Language::from_tag("en-GB").is_err());
# Ok::<(), typolish::Error>(())
```

A caller can declare its own language as a `const` table in its own crate and use it without any
rule code (section 7.7):

```rust
use typolish::{DashStyle, Language, LanguageData, RuleSet, Typographer};

const EN_GB: LanguageData = LanguageData::EN_US.with_tag("en-GB").with_dash_style(DashStyle::Spaced);
static EN_GB_DATA: LanguageData = EN_GB;

// Validated at run time.
let lang = Language::from_data(&EN_GB_DATA)?;
// A true static; a test of the caller checks `Language::from_data(&EN_GB_DATA).is_ok()` once.
static EN_GB_PROSE: Typographer = Typographer::new(Language::from_static(&EN_GB_DATA), RuleSet::PROSE);
# Ok::<(), typolish::Error>(())
```

A language read from a caller's configuration at run time is built with `LanguageBuilder`, which
owns its lists and normalizes them (section 7.1):

```rust
use typolish::{Language, LanguageBuilder};

let lang: Language = LanguageBuilder::from_language(&Language::RU_RU, "ru-KZ")
    .short_words(["а", "в", "и", "к", "о", "с", "у"])
    .build()?;
# Ok::<(), typolish::Error>(())
```

Choices:

- `Result` plus the `COMMON` constant meets "an explicit error or common rules only, as the caller
  chooses" without a policy type in the API.
- A region is never substituted: `en-GB` is unknown until its data exists. Substituting `en-US`
  would change the output silently on the day `en-GB` data ships. A bare `ru` or `en` is unambiguous
  (the CLDR likely-subtags rule). New built-in languages arrive as section 15.1 states: only in a
  release that raises the breaking number.
- `Language` is `Clone` but not `Copy`: inside it is either a `&'static LanguageData` or an `Arc` of
  owned data (`LanguageBuilder`). Built-in languages and caller tables wrapped by the `const fn`
  `from_static` are usable in `const` items and in `Typographer::new`. `from_data` cannot be
  `const`: validation needs `char::is_alphabetic` and case mapping, which are not `const fn` in Rust
  1.100 (checked, section 14.8), and its error owns the tag. A validated caller language in a
  `static` therefore needs `LazyLock`, or `from_static` plus a test that calls `from_data`. Equality
  compares the whole data table, so two languages with the same tag and different data are not
  equal; the hash uses the tag only, which is consistent with that equality.
- `LanguageData` has private fields and `const fn` setters (section 7.1); it is `Clone`, not `Copy`,
  so that its representation stays free. New fields can be added without a breaking release: a
  setter is added and existing tables keep the value of the table they started from. Language data
  is public in version 1 (settled in section 15.5, item 14).

Rejected: a string tag in the builder with the error deferred to `build()` (a typo surfaces late);
`enum Language { RuRu, EnUs }` (a language would be code, and adding one would change a public
enum).

### 2.3 Rules and rule sets

```rust
/// A typographic rule. The string id is stable across versions.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[non_exhaustive]
pub enum Rule {
    // declared in the execution order of section 4.1
    SymbolsCopyright, SymbolsArrows, SymbolsMath,
    EllipsisDots, EllipsisInner,
    DashDialogue, DashDoubleHyphen, DashSpacedHyphen,
    RangePlaceholders, RangeDigits, RangeContext, RangeTime,
    NumberMinus, NumberTimes,
    QuotesApostrophe, QuotesPairs, QuotesPrimes,
    NbspAbbreviations, NbspInitials, NbspDates, NbspUnits, NbspSigns, NbspPhone,
    NbspShortWords, NbspShortLength, NbspParticles, NbspDash,
}

impl Rule {
    /// Every rule, in execution order (section 4.1).
    pub const ALL: &'static [Rule];
    /// Stable id, for example `"nbsp.short-words"`.
    pub const fn id(self) -> &'static str;
    /// Looks a rule up by id; also accepts the retired ids of renamed rules.
    pub fn from_id(id: &str) -> Option<Rule>;
    /// One English line for logs and reports.
    pub const fn summary(self) -> &'static str;
}
impl core::str::FromStr for Rule { type Err = Error; } // Error::UnknownRule
impl core::fmt::Display for Rule { /* the id */ }

/// A set of rules. Execution order never depends on how the set was built.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
pub struct RuleSet { bits: u128 } // private; the representation may grow

impl RuleSet {
    pub const EMPTY: RuleSet;
    /// Every rule of the running version: a rule added in a later release joins it (section 15.1).
    pub const ALL: RuleSet;
    pub const PROSE: RuleSet;
    pub const INTERFACE: RuleSet;
    pub const fn with(self, rule: Rule) -> RuleSet;
    pub const fn without(self, rule: Rule) -> RuleSet;
    pub const fn contains(self, rule: Rule) -> bool;
    pub const fn union(self, other: RuleSet) -> RuleSet;
    pub const fn difference(self, other: RuleSet) -> RuleSet;
    pub const fn len(self) -> usize;
    pub const fn is_empty(self) -> bool;
    /// Rules of the set in execution order.
    pub fn iter(self) -> impl Iterator<Item = Rule>;
}
impl FromIterator<Rule> for RuleSet {}
impl Extend<Rule> for RuleSet {}
/// `"interface,+number.times,-nbsp.initials"`; see below.
impl core::str::FromStr for RuleSet { type Err = Error; }
/// Ids in execution order, comma-separated; `empty` for the empty set. `FromStr` reads it back.
impl core::fmt::Display for RuleSet {}
```

The rule list with ids, behavior, languages, settings and set membership is section 8.1.

Ids:

- Form `group.name`: lowercase ASCII, words of the name joined by `-`. The language is not part of
  the id: one rule serves every language, and language data shapes its behavior. A rule without the
  data it needs is inert for that language and is absent from `Typographer::active_rules()`.
- An id never changes meaning. A renamed rule keeps its old id as an alias in `from_id`. A removed
  rule stays as a `#[deprecated]` variant that does nothing until the next breaking release; while
  the variant exists, `FromStr` of `Rule` and of `RuleSet` accepts its id. That breaking release
  deletes the variant; its id is then reserved forever, and `FromStr` of `Rule` and of `RuleSet`
  answers with `Error::RemovedRule { id, release }` ("rule id `{id}` was removed in {release}"), a
  variant added to the `#[non_exhaustive]` `Error` by the first release that deletes a deprecated
  variant; version 1 has no removed rules. A deprecated variant is not in `Rule::ALL`, in the order
  table of section 4.1 or in any built-in set (`ALL`, `PROSE`, `INTERFACE`, base names of
  `FromStr`), and never in `active_rules()`. `with` may still add it; `contains`, `iter` (after the
  other rules) and `Display` show it, so the round trip of section 13.9 holds.
- The string id serves configuration files of callers, reports and test tables. The enum variant
  gives compile-time checking in code.

`RuleSet::from_str` grammar: comma-separated items, spaces around commas allowed. The first item may
be a base set name (`prose`, `interface`, `empty`, `all`); without one the base is `empty`. Every
other item, and the first one when it is not a base name, is an id, optionally prefixed by `+` (add)
or `-` (remove). Spaces around every item, the first and the last included, are ignored; a string of
spaces is one empty item. No space may follow a sign (`+ a` is the unknown id ` a`); a sign with
nothing after it (`+`, `-`) is an empty item. The same id given twice with the same sign is not an
error. The result is `(base ∪ added) \ removed`, independent of item order. An unknown id is
`Error::UnknownRule`. `Error::InvalidRuleSet` covers the rest, with these reasons: an id both added
and removed ("rule both added and removed"), a base name in a later position, with or without a sign
("base set name after the first item"), a base name with a sign in the first position ("sign before
a base set name"), and an empty item or an empty string ("empty item"). `Display` prints plain ids,
and the base name `empty` for the empty set, so `s.to_string().parse() == Ok(s)` for every set,
`EMPTY` included (a test, section 13.9). The base `all` and `RuleSet::ALL` take in rules added in
later releases (section 15.1). "Spaces" in this grammar are whitespace as `str::trim` reads it.
Items are read from left to right, and the first item in error decides the error; the check for an
id both added and removed runs after every item has been read.

Why an enum with a bit set: `RuleSet` is 16 bytes, `Copy` and usable in `const` items
(`RuleSet::INTERFACE.with(Rule::NumberTimes)`); membership is one bit test. Bit positions are an
internal detail: execution order comes from the order table (section 4.1), and a test checks that
the table and `Rule::ALL` contain every rule exactly once.

Rejected: string ids as the only key (typos found at run time only); `struct RuleId(&'static str)`
(callers could make ids that do not exist); user rules through a trait (section 1.5).

### 2.4 Typographer and builder

```rust
/// An immutable typographer: language, rules and settings fixed at construction.
#[derive(Clone, Debug)]
pub struct Typographer { /* private */ }

impl Typographer {
    /// Language defaults, backtick protection on. Usable in a `static`.
    pub const fn new(language: Language, rules: RuleSet) -> Typographer;
    /// Starts a builder; `rules` is the base set.
    pub fn builder(language: Language, rules: RuleSet) -> TypographerBuilder;
    pub fn language(&self) -> &Language;
    /// The rules as configured.
    pub fn rules(&self) -> RuleSet;
    /// The configured rules that the language supports.
    pub fn active_rules(&self) -> RuleSet;
    // Application methods: sections 2.5 to 2.10.
}

#[derive(Clone, Debug)]
#[must_use]
pub struct TypographerBuilder { /* private */ }

impl TypographerBuilder {
    pub fn enable(self, rule: Rule) -> Self;
    pub fn disable(self, rule: Rule) -> Self;
    // quotes.pairs
    pub fn quote_pairs(self, pairs: QuotePairs) -> Self;         // QuotePairs::new(('«', '»'), ('„', '“')), section 7.1
    pub fn single_quotes(self, mode: SingleQuotes) -> Self;      // NestedOnly | Everywhere | Never
    // dash.spaced-hyphen, dash.double-hyphen, nbsp.dash
    pub fn dash_style(self, style: DashStyle) -> Self;           // Closed | Spaced | SpacedGlued
    // nbsp.units
    pub fn percent(self, spacing: PercentSpacing) -> Self;       // AsWritten | Language | Attached | Spaced
    // nbsp.units, range.context
    pub fn add_units<I>(self, units: I) -> Self where I: IntoIterator, I::Item: AsRef<str>;
    pub fn remove_units<I>(self, units: I) -> Self where I: IntoIterator, I::Item: AsRef<str>;
    // nbsp.short-words
    pub fn add_short_words<I>(self, words: I) -> Self where I: IntoIterator, I::Item: AsRef<str>;
    pub fn remove_short_words<I>(self, words: I) -> Self where I: IntoIterator, I::Item: AsRef<str>;
    // nbsp.short-length
    pub fn short_word_length(self, max_chars: u8) -> Self;       // 0 makes nbsp.short-length inert
    // nbsp.short-words, nbsp.short-length
    pub fn short_words_any_script(self, yes: bool) -> Self;
    // nbsp.particles
    pub fn add_particles<I>(self, words: I) -> Self where I: IntoIterator, I::Item: AsRef<str>;
    pub fn remove_particles<I>(self, words: I) -> Self where I: IntoIterator, I::Item: AsRef<str>;
    // nbsp.abbreviations, nbsp.initials, nbsp.signs: where U+202F is used instead of U+00A0
    pub fn narrow_glue(self, places: NarrowGlue) -> Self;        // NarrowGlue::NONE by default
    // format_number, format_range (grouping); number.minus (minus)
    pub fn min_grouping_digits(self, digits: u8) -> Self;        // 0..=4: group from 3 + digits digits; 0 never groups
    pub fn group_separator(self, separator: char) -> Self;
    pub fn minus_sign(self, minus: char) -> Self;
    // protection (section 10.1)
    pub fn protect_backticks(self, on: bool) -> Self;
    pub fn protect_between(self, open: impl Into<String>, close: impl Into<String>) -> Self;
    /// Validates everything (section 2.11) and builds the typographer.
    pub fn build(self) -> Result<Typographer, Error>;
}

/// Places where glue is written as U+202F instead of U+00A0 (section 15.5, item 10). A set:
/// the order of `union` calls does not matter.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct NarrowGlue { /* private bit set */ }
impl NarrowGlue {
    pub const NONE: NarrowGlue;          // the default: U+00A0 everywhere
    pub const ABBREVIATIONS: NarrowGlue; // nbsp.abbreviations: the spaces between the parts of an entry (section 8.7)
    pub const INITIALS: NarrowGlue;      // nbsp.initials: every space it glues
    pub const SIGNS: NarrowGlue;         // nbsp.signs: the space after `№`, `§`, `¶`
    pub const ALL: NarrowGlue;
    pub const fn union(self, other: NarrowGlue) -> NarrowGlue;
    pub const fn contains(self, other: NarrowGlue) -> bool;
}
```

Glue typed by the author (a U+00A0 or U+202F in the input) is never rewritten, whatever
`narrow_glue` says (section 4.5).

```rust
use std::sync::LazyLock;
use typolish::{DashStyle, Language, Rule, RuleSet, Typographer};

// Defaults: a true static with no initialization cost.
static RU_UI: Typographer = Typographer::new(Language::RU_RU, RuleSet::INTERFACE);

// Custom settings: built once.
static EN_DOCS: LazyLock<Typographer> = LazyLock::new(|| {
    Typographer::builder(Language::EN_US, RuleSet::INTERFACE)
        .enable(Rule::NumberTimes)
        .disable(Rule::NbspShortWords)
        .dash_style(DashStyle::Spaced)
        .add_units(["req/s", "vCPU"])
        .protect_between("{{", "}}")
        .build()
        .expect("settings are valid")
});

// Rules from a caller's configuration file.
let rules: RuleSet = "interface,+symbols.arrows,-nbsp.initials".parse()?;
let ui = Typographer::builder(Language::RU_RU, rules).build()?;
# Ok::<(), typolish::Error>(())
```

Choices and reasons:

- **The rule set is mandatory** in `new` and `builder`. There is no default set: Prose and Interface
  treat technical text differently, and a silent default would be wrong for half of the callers.
- **`const fn new`.** A typographer with default settings holds only `&'static` data, empty `Vec`s
  (`Vec::new()` is `const`) and `None` overrides, so it fits a `static` without `LazyLock`. This is
  a commitment (section 2.13): the representation of default settings stays computable at compile
  time. It holds because every derived table (sorted lists) lives in the language data, and the
  builder stores only additions and removals.
- **The builder is order-independent.** The active set is `(base ∪ enabled) \ disabled`. A rule that
  is both enabled and disabled is `Error::RuleConflict`, not "the last call wins", so behavior does
  not depend on the order of lines in someone else's code. A word both added and removed in the same
  list is `Error::InvalidSetting`. Setting methods only record values; `build()` checks them in a
  fixed order that does not depend on the order of calls and returns the first error: (1) rule
  conflicts, the rule first in `Rule::ALL` reported; (2) protection delimiters; (3) `QuotePairs`;
  (4) word and unit lists, in the order units, short words, particles; (5) group separator, minus
  sign, `min_grouping_digits`. Within a check, the offending value that is smallest in code point
  order is reported.
- **Flat setting methods instead of an options struct.** A new setting is a new method, which breaks
  nothing; the rustdoc of each method names the rules it affects. Settings do not depend on rule
  sets: a set only switches rules on and off, and default values come from the language data. Prose
  and Interface therefore differ only in their rule lists (section 9).
- **Immutability.** No interior mutability, except one write-once, thread-safe cache: a `OnceLock`
  of the active rules and the trigger set, filled by `build()` or by the first call of a typographer
  made by the `const fn` `new` (section 3.2). The cache is not observable through the API: its value
  depends only on language, rules and settings, and `Debug` leaves it out (`Debug` is written by
  hand, so `#[derive(Debug)]` above stands for that impl). The working buffer lives outside the
  object, in a thread-local or in a caller's `Scratch` (section 2.12), so `&Typographer` can be
  shared freely. A compile-time check asserts `Send + Sync`.

Rejected: `Typographer::default()` (language and rules cannot be guessed); an `Arc` inside (the
caller chooses `static`, `Arc` or `LazyLock`); a mutable `set_rule()` (breaks immutability).

### 2.5 Input 1: a whole string

```rust
impl Typographer {
    /// Typesets plain text. Returns the input borrowed when the output equals it byte for byte.
    #[must_use]
    pub fn typeset<'a>(&self, text: &'a str, output: Output) -> Cow<'a, str>;
    /// The same with working memory supplied by the caller.
    #[must_use]
    pub fn typeset_with<'a>(&self, scratch: &mut Scratch, text: &'a str, output: Output) -> Cow<'a, str>;
    /// Appends the typeset text to `buf`; no allocation when `buf` has room.
    pub fn typeset_into(&self, text: &str, output: Output, buf: &mut String);
}
```

Argument order: working memory first (`scratch`, as context of the call), then the input and the
output kind, then the destination (`buf`, as in `write!`). `typeset_template_with` follows the same
order.

```rust
use std::borrow::Cow;
use typolish::{HtmlStyle, Language, Output, RuleSet, Typographer};

static RU_UI: Typographer = Typographer::new(Language::RU_RU, RuleSet::INTERFACE);

let s = RU_UI.typeset("Файл \"report.pdf\" не найден - проверьте путь в `--config`.", Output::Unicode);
assert_eq!(s, "Файл «report.pdf» не\u{a0}найден\u{a0}— проверьте путь в\u{a0}`--config`.");

// Technical text is neither changed nor copied.
assert!(matches!(RU_UI.typeset("host:8080", Output::Unicode), Cow::Borrowed("host:8080")));

// Already typeset text comes back borrowed (idempotence).
assert!(matches!(RU_UI.typeset(&s, Output::Unicode), Cow::Borrowed(_)));

// The same text as HTML.
assert_eq!(RU_UI.typeset("Цена - 5 руб.", Output::Html(HtmlStyle::NAMED)), "Цена&nbsp;&mdash; 5&nbsp;руб.");
```

Protection in this mode uses delimiters (section 10.1): backticks are on by default, and
`protect_between` adds pairs. `Cow::Borrowed` is returned exactly when the output equals the input
byte for byte; with `Output::Html` or `Output::Escaped` a string containing any character that the
output kind encodes is `Owned` even when no rule fired, because encoding changes bytes too: for HTML
`& < > " '` and, except with `HtmlStyle::CHARACTERS`, the typographic set (U+00A0 among it); for
`Escaped` the characters of section 11.6 (`\`, the quotes of the syntax, controls, the typographic
set and the rest of its table). Section 3.2 lists these bytes.

### 2.6 Input 2: pieces of a template

```rust
/// One input piece of a message template. `S` is the caller's payload for a slot
/// (a variable name, an AST node index); the crate never reads it.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum Piece<'t, S> {
    /// Text to typeset.
    Text(&'t str),
    /// An opaque placeholder whose value is unknown; rules see only its kind.
    Slot(S, Slot),
    /// Visible text that is never typeset (a command, a path, code). Acts as one word for its
    /// neighbours; encoded for the output kind.
    Protected(&'t str),
    /// Invisible markup (for example a tag supplied by an HTML splitter). Transparent for rules;
    /// written as is in Unicode and HTML output, escaped in escaped output.
    Markup(&'t str),
}

/// A placeholder. The field is private so that hints can be added later.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Slot { kind: SlotKind }
impl Slot {
    pub const NUMBER: Slot;
    pub const WORD: Slot;
    pub const DATE: Slot;
    pub const OTHER: Slot;
    pub const fn new(kind: SlotKind) -> Slot;
    pub const fn kind(self) -> SlotKind;
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum SlotKind {
    /// A number formatted by `format_number` of the same typographer (count, size, port).
    Number,
    /// A word or a name.
    Word,
    /// A date or a time, in words or digits.
    Date,
    /// Anything else: a path, a URL, code, a string of unknown length.
    Other,
}

/// A typeset template: n slots and exactly n + 1 texts; `texts()[i]` stands before `slots()[i]`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Template<'t, S> { /* texts: Vec<Cow<'t, str>>, slots: Vec<(S, Slot)>, changed: bool */ }

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum TemplatePart<'a, S> { Text(&'a str), Slot(&'a S, Slot) }

impl<'t, S> Template<'t, S> {
    pub fn texts(&self) -> &[Cow<'t, str>];
    pub fn slots(&self) -> &[(S, Slot)];
    pub fn changed(&self) -> bool;
    /// Parts in order; empty texts are skipped.
    pub fn iter(&self) -> impl Iterator<Item = TemplatePart<'_, S>>;
    pub fn into_parts(self) -> (Vec<Cow<'t, str>>, Vec<(S, Slot)>);
    /// Joins texts with slot values (tests, run-time use without code generation). Values are
    /// inserted as they are: with `Output::Html` or `Output::Escaped` the closure must return values
    /// already encoded for that output (section 11.1).
    pub fn fill<'v>(&self, value: impl FnMut(&S) -> Cow<'v, str>) -> String;
}

impl Typographer {
    #[must_use]
    pub fn typeset_template<'t, S>(&self, pieces: impl IntoIterator<Item = Piece<'t, S>>, output: Output)
        -> Template<'t, S>;
    #[must_use]
    pub fn typeset_template_with<'t, S>(&self, scratch: &mut Scratch,
        pieces: impl IntoIterator<Item = Piece<'t, S>>, output: Output) -> Template<'t, S>;
}
```

Example: the Fluent message `Файл "{ $name }" весит { NUMBER($size) } МБ - удалить?` at build time:

```rust
use typolish::{Language, Output, Piece, RuleSet, Slot, Typographer};

static RU_UI: Typographer = Typographer::new(Language::RU_RU, RuleSet::INTERFACE);

let t = RU_UI.typeset_template(
    [
        Piece::Text("Файл \""),
        Piece::Slot("name", Slot::WORD),
        Piece::Text("\" весит "),
        Piece::Slot("size", Slot::NUMBER),
        Piece::Text(" МБ - удалить?"),
    ],
    Output::Unicode,
);
assert_eq!(t.texts(), ["Файл «", "» весит ", "\u{a0}МБ\u{a0}— удалить?"]);
assert_eq!(t.slots().iter().map(|(s, _)| *s).collect::<Vec<_>>(), ["name", "size"]);

// A range between two number slots gets an en dash in both rule sets.
let t = RU_UI.typeset_template(
    [Piece::Slot("a", Slot::NUMBER), Piece::Text("-"), Piece::Slot("b", Slot::NUMBER), Piece::Text(" дн.")],
    Output::Unicode,
);
assert_eq!(t.texts(), ["", "\u{2013}", "\u{a0}дн."]);
```

Output contract:

1. Slots come out in input order, each exactly once, with the caller's payload;
   `texts().len() == slots().len() + 1` always (an empty input gives one empty, borrowed text and no
   slots).
2. Adjacent `Text`, `Protected` and `Markup` pieces merge into one output text. A text that maps to
   exactly one unchanged input piece is `Cow::Borrowed` with the input lifetime `'t`.
3. Rules see the whole message as one stream: a slot does not break context (`{n} дн.` is glued
   although the number and the unit are in different pieces), and a quote opened in one text closes
   in another (`"{name}"`). Edits land in the texts next to slots; a slot is never part of an edit.
4. Tokens may span adjacent `Text` pieces, so a template of `Text` pieces gives the same as their
   concatenation; a slot, a `Protected` or a `Markup` piece ends a token (section 3.4).
5. The n + 1 form always has room for markup around a slot: a nowrap group that starts with slot i
   opens at the end of `texts()[i]`, one that ends with slot j closes at the start of
   `texts()[j + 1]` (section 11.5).
6. `changed()` is true when any text differs from the input text it came from: a rule edited it or
   the output kind encoded it. Merging adjacent pieces into one text is not a change.

How a slot takes part in rules (the slot participation table). A slot is one opaque, frozen token;
for its neighbours it behaves as its kind says:

| Context                                                                               | `Number`                                                        | `Word`   | `Date`              | `Other`  |
| ------------------------------------------------------------------------------------- | --------------------------------------------------------------- | -------- | ------------------- | -------- |
| quotes around it (`"{x}"`)                                                            | word                                                            | word     | word                | word     |
| apostrophe after it (`{x}'s`)                                                         | yes                                                             | yes      | no                  | no       |
| quote after it as a prime (`{n}"`, Prose)                                             | yes, if the quote is unpaired                                   | no       | no                  | no       |
| `range.placeholders` (`{a}-{b}`)                                                      | with another `Number`                                           | no       | with another `Date` | no       |
| `nbsp.units` on its left (`{n} МБ`)                                                   | yes                                                             | no       | no                  | no       |
| `nbsp.dates` (`{n} мая`, `{d} г.`)                                                    | day or year                                                     | no       | year                | no       |
| a `Next` abbreviation after it does not bind forward (`{n} ч. Потом`, `{d} г. Затем`) | yes                                                             | no       | yes                 | no       |
| `number.times` (`{w}x{h}`, Prose)                                                     | yes                                                             | no       | no                  | no       |
| glue before it (`в {x}`, `№ {x}`)                                                     | yes                                                             | yes      | yes                 | no       |
| glue after it (`{x} же`)                                                              | yes                                                             | yes      | yes                 | no       |
| operand of `dash.spaced-hyphen` (`{x} - слово`)                                       | yes; next to a number, a range or a single-letter word: no dash | yes      | yes                 | yes      |
| `SlotGlue` diagnostic                                                                 | `{n}МБ`, `{a}{b}`                                               | `{a}{b}` | `{a}{b}`            | `{a}{b}` |

No glue sticks to an `Other` slot: its value may be long, and a nowrap group around a long path
breaks the layout. A `Number` slot after `№`, `§` or `¶` (`№ {n}`, `№{n}`) is a label number
(section 8.2): `nbsp.signs` glues it to the sign, and the rules that read a quantity or a year
(`nbsp.units`, `nbsp.dates`) leave it alone. Two `Number` slots or two `Date` slots joined by `-`
form a `Range` core (check 17 of section 3.6), which is how `range.placeholders` reaches them; a
`Number` slot and a literal number form one too, but no rule edits its dash.

Choices:

- **Pieces are an enum with a caller payload**, not a trait and not a string with `{name}` markers:
  a generator already holds a parsed template, `&'t str` pieces are built from its AST without
  copies, and the payload removes the need for parallel arrays. Parsing placeholder syntax is not a
  typographer's job.
- **Output in the canonical n + 1 form**, not one output part per input part: a generator needs
  exactly `texts[0], slots[0], texts[1], ...`, and the form always has room for nowrap markup.
  Rejected: a one-to-one output with extra parts inserted for markup (a rare branch every caller
  must handle); a string with markers (markers can collide with text).
- **`Protected` and `Markup` exist from the start**, although HTML is not in version 1. The HTML
  module of stage 6 becomes a splitter: tags become `Markup`, the content of `code`, `pre` and `kbd`
  becomes `Protected`, text stays `Text`. The core does not change for it.

### 2.7 Input 3: numbers

```rust
/// A number to format: integers, `f32`/`f64`, or an ASCII decimal string.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Number<'n> { /* i128 | u128 | f32 | f64 | decimal digits borrowed from 'n */ }

impl<'n> Number<'n> {
    /// ASCII decimal: an optional sign `+` or `-`, digits, optional `.` and digits. A `+` is
    /// accepted and not written. Digits are kept as given: leading zeros ("007" stays `007`)
    /// and the caller's precision ("1.50" keeps two fraction digits). A zero value is written
    /// without a sign ("-0.00" is `0,00` in ru). `.5`, `5.`, `1e3`, `--5`, `+-5`, a sign
    /// without digits, spaces and the empty string are `Error::InvalidNumber` (section 2.11).
    pub fn decimal(text: &'n str) -> Result<Number<'n>, Error>;
    /// A float rounded to `fraction_digits` (round half to even on the exact binary value, as
    /// `format!("{:.N}")` in core).
    pub fn fixed(value: f64, fraction_digits: u8) -> Number<'static>;
}
// From<i8 | i16 | i32 | i64 | i128 | isize | u8 | u16 | u32 | u64 | u128 | usize | f32 | f64>

impl Typographer {
    /// Formats a number by the language: decimal separator, grouping from the configured digit
    /// count, minus sign. Lazy: implements `Display`, allocates nothing.
    pub fn format_number<'n>(&self, number: impl Into<Number<'n>>, output: Output) -> NumberDisplay<'_, 'n>;
    /// Two numbers with the language's range pattern (`{0}–{1}`): the explicit way to get a
    /// dash between two values.
    pub fn format_range<'n>(&self, from: impl Into<Number<'n>>, to: impl Into<Number<'n>>, output: Output)
        -> RangeDisplay<'_, 'n>;
}
```

```rust
use typolish::{HtmlStyle, Language, Number, Output, RuleSet, Typographer};

static RU: Typographer = Typographer::new(Language::RU_RU, RuleSet::PROSE);
static EN: Typographer = Typographer::new(Language::EN_US, RuleSet::PROSE);

assert_eq!(RU.format_number(-1234567.5, Output::Unicode).to_string(), "\u{2212}1\u{a0}234\u{a0}567,5");
assert_eq!(RU.format_number(1234, Output::Unicode).to_string(), "1\u{a0}234"); // ru: from 4 digits
assert_eq!(RU.format_number(12345, Output::Unicode).to_string(), "12\u{a0}345");
assert_eq!(EN.format_number(1234, Output::Unicode).to_string(), "1,234");   // en: from 4 digits
assert_eq!(RU.format_number(Number::decimal("1.50")?, Output::Unicode).to_string(), "1,50");
assert_eq!(RU.format_number(Number::decimal("+5")?, Output::Unicode).to_string(), "5");
assert_eq!(RU.format_number(Number::decimal("007")?, Output::Unicode).to_string(), "007");
assert_eq!(RU.format_number(Number::decimal("-0.00")?, Output::Unicode).to_string(), "0,00");
assert_eq!(RU.format_number(Number::fixed(-0.001, 1), Output::Unicode).to_string(), "0,0");
assert_eq!(RU.format_number(0.1f32, Output::Unicode).to_string(), "0,1");   // f32 is not widened
assert_eq!(RU.format_number(f64::NAN, Output::Unicode).to_string(), "не\u{a0}число");
assert_eq!(RU.format_range(8000, 8100, Output::Unicode).to_string(), "8\u{a0}000–8\u{a0}100");
assert_eq!(RU.format_number(-12345, Output::Html(HtmlStyle::NAMED)).to_string(), "&minus;12&nbsp;345");
assert_eq!(
    RU.format_number(-12345, Output::Html(HtmlStyle::NAMED.nowrap_class("nowrap"))).to_string(),
    "<span class=\"nowrap\">&minus;12 345</span>",
);
// No glue, no element (section 11.5).
assert_eq!(RU.format_number(123, Output::Html(HtmlStyle::NAMED.nowrap_class("nowrap"))).to_string(), "123");

// No allocation: straight into a response buffer.
use core::fmt::Write;
let mut buf = String::with_capacity(64);
write!(buf, "{}", RU.format_number(42u32, Output::Unicode)).unwrap();
# Ok::<(), typolish::Error>(())
```

Formatting rules:

- Integers go through `unsigned_abs()`, so `i128::MIN` does not overflow. `f64` uses the shortest
  representation of core (`{}`), which is deterministic on every platform and has no exponent; `f32`
  keeps a variant of its own and uses the shortest representation of `f32`, so `0.1f32` is `0,1`,
  not the `0,10000000149011612` of a widened `f64`; `Number::fixed` uses `{:.N}`. `NaN` and
  infinities are written with the CLDR symbols of the language (`не⍽число`, `NaN`, `∞`); there is no
  error. `NaN` is written without a sign; negative infinity is the minus sign followed by the
  infinity symbol (`−∞`). Negative zero is written as `0`.
- `Number::decimal` takes an optional leading `+` or `-`; a `+` is accepted and not written. Its
  digits are written as given, leading zeros and every fraction digit included (`007` → `007`,
  `1.50` → ru `1,50`); nothing is rounded or trimmed.
- A zero value is written without a sign, for every variant: `-0.0` → `0`,
  `Number::decimal("-0.00")` → `0,00`, and `Number::fixed(-0.001, 1)`, whose rounding gives `-0.0`,
  → `0,0` (ru).
- Grouping: with `min_grouping_digits` from 1 to 4 (the CLDR meaning of the parameter), the integer
  part is split by three digits when it has at least `3 + min_grouping_digits` digits; 0 is not a
  CLDR value and turns grouping off for numbers of every length (`COMMON` uses it, section 7.4).
  Defaults: ru and en 1 (from 4 digits), as in CLDR 49 (settled in section 15.5, item 4). The
  fraction is not grouped. For `Number::decimal` the integer part is grouped as written, leading
  zeros included (ru `0012345` → `0⍽012⍽345`).
- Minus: U+2212 for both languages by default, as the task requires; a deviation from CLDR 49, where
  the minus sign is U+002D (section 7.5). Setting `minus_sign`.
- Range: the language's CLDR range pattern, `{0}–{1}` (U+2013, no spaces) for both languages, for
  every pair of values. A language whose pattern has spaces (`range_spaced`, none of the built-in
  ones) gets U+00A0 on both sides of its dash, so the range stays one unbreakable unit. Negative
  ends are written as they are (`−5–−1`), and equal ends are not collapsed (ru `8⍽000–8⍽000`): the
  pattern applies to every pair, and the caller decides what is a range. Rejected: the CLDR
  `approximately` pattern for equal ends (`≈8⍽000`), which changes the meaning of a range. A group
  separator that is a space is glue (only U+00A0 and U+202F are allowed, section 2.11), so `{n}⍽МБ`
  and `12⍽345` never break apart; a separator that is not a space (`,` in English) needs no glue.
- No allocation: for `f32` and `f64` the `Display` makes two passes (a counting `fmt::Write` to
  learn the length of the integer part, then the real write), because grouping is counted from the
  right. The longest `f64` in `{}` is 327 bytes with its sign (`-5e-324` written out), so a stack
  buffer would fit too; the counting pass avoids a fixed bound that would have to be re-checked if
  the representation of core changed.

A template and its value stay consistent because one `Typographer` (one `LanguageData`, one set of
number settings) typesets the template at build time and formats the numbers at run time; the range
dash of `{a}-{b}` and of `format_range` comes from the same data field, and both use the same output
kind.

Why `Display` and not `String`: the number is built anew anyway, so `Cow` gains nothing, while
`Display` writes into an existing buffer and `to_string()` remains for callers that want a string.
Rejected: `TryFrom<f64>` with an error on `NaN` (CLDR has a symbol for it); percent, currency and
unit formatting of values (the job of i18n libraries).

### 2.8 Output kinds

```rust
/// How the result is written. Rules never depend on it (section 11).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum Output {
    /// Characters as they are (UTF-8). Nothing is escaped.
    Unicode,
    /// HTML text: `& < > " '` escaped, typographic characters as entities, no-break groups as
    /// entities or as nowrap elements.
    Html(HtmlStyle),
    /// The body of a string literal of a given syntax, ready to paste between its quotes.
    Escaped(EscapeStyle),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct HtmlStyle { /* entities: Named | Decimal | Hex | Characters; nbsp: Entity | NowrapClass | NowrapStyle */ }
impl HtmlStyle {
    pub const NAMED: HtmlStyle;      // &nbsp; &mdash;, numeric where HTML has no name
    pub const DECIMAL: HtmlStyle;    // &#160; &#8212;
    pub const HEX: HtmlStyle;        // &#xA0; &#x2014;
    pub const CHARACTERS: HtmlStyle; // typographic characters as characters; only markup escaped
    /// Wraps each no-break group in `<span class="{class}">`; U+00A0 glue as ordinary spaces
    /// inside (section 11.5).
    pub const fn nowrap_class(self, class: &'static str) -> HtmlStyle;
    /// The same with `<span style="white-space:nowrap">`.
    pub const fn nowrap_style(self) -> HtmlStyle;
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct EscapeStyle { /* syntax: Js | JsHex | JsCodePoint | Json | Rust | Css; scope: Typographic | NonAscii */ }
impl EscapeStyle {
    pub const JS: EscapeStyle;            // \u00A0
    pub const JS_HEX: EscapeStyle;        // \xA0 up to U+00FF, \u2014 above
    pub const JS_CODE_POINT: EscapeStyle; // \u{A0} (ES2015)
    pub const JSON: EscapeStyle;          // \u00a0
    pub const RUST: EscapeStyle;          // \u{a0}
    pub const CSS: EscapeStyle;           // \a0 followed by a space
    /// Escape every non-ASCII character, not only the typographic set.
    pub const fn non_ascii(self) -> EscapeStyle;
}
```

`Output::Escaped` always escapes the container completely: `\`, the quote characters of the syntax
and control characters are escaped whatever the scope, because unpaired straight quotes stay in the
text and protected spans may contain backslashes. The scope (`non_ascii()`) only chooses which other
characters become escapes (section 11.6).

```rust
use typolish::{EscapeStyle, HtmlStyle, Language, Output, RuleSet, Typographer};
static RU: Typographer = Typographer::new(Language::RU_RU, RuleSet::INTERFACE);
let src = "Цена - 5 руб.";

assert_eq!(RU.typeset(src, Output::Unicode), "Цена\u{a0}— 5\u{a0}руб.");
assert_eq!(RU.typeset(src, Output::Html(HtmlStyle::NAMED)), "Цена&nbsp;&mdash; 5&nbsp;руб.");
assert_eq!(RU.typeset(src, Output::Html(HtmlStyle::HEX)), "Цена&#xA0;&#x2014; 5&#xA0;руб.");
assert_eq!(
    RU.typeset(src, Output::Html(HtmlStyle::NAMED.nowrap_class("nowrap"))),
    "<span class=\"nowrap\">Цена &mdash;</span> <span class=\"nowrap\">5 руб.</span>",
);
assert_eq!(RU.typeset(src, Output::Escaped(EscapeStyle::JS)), "Цена\\u00A0\\u2014 5\\u00A0руб.");
assert_eq!(RU.typeset(src, Output::Escaped(EscapeStyle::JS_HEX)), "Цена\\xA0\\u2014 5\\xA0руб.");
assert_eq!(RU.typeset(src, Output::Escaped(EscapeStyle::RUST)), "Цена\\u{a0}\\u{2014} 5\\u{a0}руб.");
```

Choices:

- **The output kind is given with every call**, as the task requires. There is no default in the
  builder, so one typographer serves an HTML page and a JSON response alike.
- **Output is a separate encoding pass over the Unicode result.** Rules work in Unicode and never
  see the output kind; explanation, reverse transformation and tests are the same for every output.
- **Every output is safe where it is meant to go**: HTML text and attribute values, the inside of a
  JS, JSON or Rust string literal. There is no "entities without escaping markup" mode: it would
  produce invalid or dangerous HTML. Markup is passed as `Piece::Markup`.
- `class` is `&'static str` so that `HtmlStyle` stays `Copy` and usable in `const`; it is escaped as
  an attribute value when written.

Rejected: the output kind as a type parameter (`typeset::<Html>`; HTML and escaping have parameters,
and a `Copy` enum is easier to pass from configuration); `Output<'a>` with a run-time class string
(a lifetime on a frequently passed type for a rare case); separate `typeset_html` and `typeset_js`
methods (every new output would multiply methods by inputs).

### 2.9 Report, protected spans, lint

```rust
impl Typographer {
    /// The same result as `typeset`, plus every edit, every protected span and every diagnostic.
    pub fn report<'a>(&self, text: &'a str, output: Output) -> Report<'a>;
    pub fn report_template<'t, S>(&self, pieces: impl IntoIterator<Item = Piece<'t, S>>, output: Output)
        -> Report<'t, Template<'t, S>>;
    /// Source check: diagnostics only, no output is built (section 12.3), in the same order.
    pub fn lint(&self, text: &str) -> Vec<Diagnostic>;
    pub fn lint_template<'t, S>(&self, pieces: impl IntoIterator<Item = Piece<'t, S>>) -> Vec<Diagnostic>;
}

#[derive(Clone, Debug)]
pub struct Report<'a, T = Cow<'a, str>> { /* result, changes, protected, diagnostics, inputs */ }
impl<'a, T> Report<'a, T> {
    pub fn result(&self) -> &T;
    pub fn into_result(self) -> T;
    /// Edits ordered by position, then by rule order.
    pub fn changes(&self) -> &[Change];
    /// Spans that no rule may touch, with the reason (section 12.5); a span over several
    /// pieces gives one record per piece (section 3.11).
    pub fn protected(&self) -> &[ProtectedSpan];
    /// Ordered by piece, byte range, then kind (section 12.4). A diagnostic over several pieces
    /// is reported at its first piece, with the range clipped to that piece (section 3.11).
    pub fn diagnostics(&self) -> &[Diagnostic];
    /// The input text an edit replaced.
    pub fn original(&self, change: &Change) -> &'a str;
}
impl<T> core::fmt::Display for Report<'_, T> { /* one line per record of changes(), then per record of protected() */ }

/// One atomic edit in terms of the input and the Unicode result.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Change { /* rule, piece, range, replacement */ }
impl Change {
    pub fn rule(&self) -> Rule;
    /// Index of the input piece (0 for string input).
    pub fn piece(&self) -> usize;
    /// Byte range in that piece; empty for an insertion.
    pub fn range(&self) -> core::ops::Range<usize>;
    /// The replacement in Unicode, before output encoding; empty for a deletion.
    pub fn replacement(&self) -> &str;
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ProtectedSpan { /* piece, range, kind */ }
impl ProtectedSpan {
    /// Index of the input piece (0 for string input).
    pub fn piece(&self) -> usize;
    /// Byte range in that piece.
    pub fn range(&self) -> core::ops::Range<usize>;
    /// Why the span is protected.
    pub fn kind(&self) -> ProtectedKind;
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum ProtectedKind { Delimited, ProtectedPiece, Slot, Technical(TechKind) }
/// `CompoundHyphens`: only the hyphens of a compound word; the word itself is not technical
/// and is an operand like any word (section 8.2).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum TechKind {
    Url, Email, Flag, Path, TimeRange, Colon, Operator, Identifier, Angle, Uuid, Hex, IsoDate,
    Version, Dotted, List, CompoundHyphens, Unclassified,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Diagnostic { /* kind, severity, rule, piece, range */ }
impl Diagnostic {
    pub fn kind(&self) -> &DiagnosticKind;
    pub fn severity(&self) -> Severity;          // Info | Warning
    pub fn rule(&self) -> Option<Rule>;          // the rule that raised it, if any
    pub fn piece(&self) -> usize;
    pub fn range(&self) -> core::ops::Range<usize>;
}
impl core::fmt::Display for Diagnostic { /* `warning quotes.pairs 0:10..11: message` (section 12.4) */ }
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[non_exhaustive]
pub enum Severity { Info, Warning }
#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum DiagnosticKind {
    UnmatchedQuote, CrossingQuotes, AmbiguousQuote, UnclosedDelimiter, AmbiguousHyphen,
    MixedScript, SlotGlue, NowrapFallback,
    #[non_exhaustive] TypographicChar { found: char },
}
```

```rust
use typolish::{DiagnosticKind, Language, Output, RuleSet, Typographer};
static RU: Typographer = Typographer::new(Language::RU_RU, RuleSet::INTERFACE);

let r = RU.report("Цена - 5 руб.", Output::Unicode);
assert_eq!(r.result(), "Цена\u{a0}— 5\u{a0}руб.");
assert_eq!(r.to_string(), "\
nbsp.dash           0:8..9    \" \" -> U+00A0
dash.spaced-hyphen  0:9..10   \"-\" -> \"—\"
nbsp.units          0:12..13  \" \" -> U+00A0
");

let r = RU.report("Он сказал \"привет.", Output::Unicode);
assert_eq!(r.result(), "Он сказал \"привет.");          // not guessed
assert_eq!(r.diagnostics()[0].kind(), &DiagnosticKind::UnmatchedQuote);
assert_eq!(r.diagnostics()[0].range(), 18..19);
```

Choices:

- **Edits are atomic and relative to the input.** Each record replaces one contiguous byte range of
  one piece. An edit across a boundary of adjacent `Text` pieces (`Text("Ждём.")`, `Text("..")`)
  becomes several records in input order: the first holds the replacement, the others delete the
  rest; `original()` of each returns the slice of its own piece. A report of differences against the
  reference implementation is printed from them without a second run.
- **`report` and `typeset` are one engine**; `report` turns on a recorder that the plain path
  compiles out (section 3.11). A property test checks `report(x).result() == typeset(x)` and that
  applying `changes()` to the input yields the result.
- **Diagnostics come only from `report` and `lint`.** `typeset` has nowhere to put them on the hot
  path, and the result does not depend on them. A generator calls `report_template` and turns
  warnings into build warnings.

Rejected: `typeset() -> Typeset { text, warnings }` (every caller would unpack it); edits as
before/after strings per rule (ambiguous when rules overlap); a callback observer (a type parameter
in every signature).

### 2.10 Reverse transformation

```rust
impl Typographer {
    /// Maps typeset Unicode text back to the canonical plain form, undoing exactly the enabled
    /// rules (section 12.1). The spans of `protected()` (section 12.5) and markup are left alone.
    /// The input is Unicode text (`Output::Unicode`); HTML entities and escapes are not decoded and
    /// are read as ordinary characters (section 12.1).
    pub fn to_plain<'a>(&self, text: &'a str) -> Cow<'a, str>;
    pub fn to_plain_template<'t, S>(&self, pieces: impl IntoIterator<Item = Piece<'t, S>>) -> Template<'t, S>;
}
```

```rust
# use typolish::{Language, RuleSet, Typographer};
# static RU: Typographer = Typographer::new(Language::RU_RU, RuleSet::INTERFACE);
assert_eq!(
    RU.to_plain("Файл «report.pdf» не\u{a0}найден\u{a0}— проверьте"),
    "Файл \"report.pdf\" не найден - проверьте",
);
```

It is a method, not a free function, because it needs the protection settings, the quote pairs,
single-quote mode and dash style of the typographer (with the builder's overrides), and the set of
enabled rules.

### 2.11 Errors

```rust
#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
#[non_exhaustive]
pub enum Error {
    #[error("unknown language tag `{tag}`")]
    #[non_exhaustive] UnknownLanguage { tag: String },
    #[error("invalid language data for `{tag}`: {reason}")]
    #[non_exhaustive] InvalidLanguageData { tag: String, reason: &'static str },
    #[error("unknown rule id `{id}`")]
    #[non_exhaustive] UnknownRule { id: String },
    #[error("invalid rule set `{input}`: {reason}")]
    #[non_exhaustive] InvalidRuleSet { input: String, reason: &'static str },
    #[error("rule `{id}` is both enabled and disabled")]
    #[non_exhaustive] RuleConflict { id: &'static str },
    #[error("invalid number `{input}`: {reason}")]
    #[non_exhaustive] InvalidNumber { input: String, reason: &'static str },
    #[error("invalid setting `{setting}`: {reason}")]
    #[non_exhaustive] InvalidSetting { setting: &'static str, reason: String },
}
```

| Method                                          | Error                            | When                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                |
| ----------------------------------------------- | -------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `Language::from_tag`                            | `UnknownLanguage`                | the tag is malformed or has no data                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                 |
| `Language::from_data`, `LanguageBuilder::build` | `InvalidLanguageData`            | a check of section 7.6 fails                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                        |
| `Rule::from_str`                                | `UnknownRule`                    | no such id                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                          |
| `RuleSet::from_str`                             | `UnknownRule`, `InvalidRuleSet`  | `UnknownRule`: unknown id; `InvalidRuleSet`: an id both added and removed, a base name after the first item, a signed base name first, an empty item or string                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                      |
| `Number::decimal`                               | `InvalidNumber`                  | not an ASCII decimal of section 2.7: the empty string; a sign without digits (`-`, `+`); more than one sign, or a sign that is not first or not `+` or `-` (`--5`, `+-5`, `5-`, `−5`, `±5`); no digit before or after the `.` (`.5`, `5.`, `-.5`); more than one `.`; an exponent (`1e3`); a `,`, a space or any other character                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                    |
| `TypographerBuilder::build`                     | `RuleConflict`, `InvalidSetting` | (1) `RuleConflict`: a rule both enabled and disabled. `InvalidSetting`, checked in this order: (2) protection delimiters: an empty delimiter, one containing whitespace, or one containing a letter or digit, a delimiter equal to or a prefix of a delimiter of another pair, a delimiter that is a proper prefix of the other one of its pair (equal delimiters of one pair are allowed); (3) `QuotePairs`: `"` or `'` or a space or a letter (output would become input again, breaking idempotence), or four glyphs that are not pairwise distinct or not quote characters (section 7.6); (4) lists, in the order units, short words, particles: a word that is empty, contains whitespace, or is both added and removed; a unit that starts with an ASCII digit (0-9); (5) numbers: a group separator that is a digit, a letter, or whitespace other than U+00A0 and U+202F; a minus that is a digit, a letter, whitespace, `"` or `'` (it is written into text by `number.minus`; whitespace would erase the sign, a quote would break idempotence); `min_grouping_digits` above 4 (0 turns grouping off). The first failing check is reported (section 2.4). |

Choices:

- **One error enum for the crate.** Errors arise only in configuration and parsing; one type suits
  `?`. Variants are structs and `#[non_exhaustive]`, so fields can be added.
- **Application never fails and never panics.** `typeset*`, `format_*`, `report*`, `to_plain*` and
  `lint*` return no `Result`. What cannot be resolved stays as it is and, where useful, produces a
  diagnostic. Library code has no `unwrap`, `expect`, `panic!` or panicking indexing (clippy lints,
  section 14.5). If an internal invariant is violated anyway (a bug), the engine returns the input
  unchanged for that window rather than panicking; a test with arbitrary strings, including lone
  combining marks, emoji sequences and control characters, guards this.
- `RemovedRule` is not in version 1: it arrives with the first release that deletes a deprecated
  rule variant (section 2.3).
- Fields are `&'static str` when the value comes from a closed set (a setting name, a fixed reason)
  and `String` when the text is the caller's or built per call (a tag, an offending word).
- The `setting` of `InvalidSetting` names the setting by its builder method, with the list methods
  under the name of their list: `protect_between`, `quote_pairs`, `units`, `short_words`,
  `particles`, `group_separator`, `minus_sign`, `min_grouping_digits`. Its `reason` quotes the
  offending value. "A letter or a digit" in this table and in section 7.6 is a character for which
  `char::is_alphanumeric` holds. A unit's leading digit, by contrast, is an ASCII digit (0-9),
  matching `Kind::Number` (section 3.5): a unit that starts with another digit, such as `٣px`, is
  accepted.

### 2.12 Allocation, Cow and scratch memory

| Call                            | Allocations                                                                                                                                                                                                                                                                                                         |
| ------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `typeset`, output equals input  | 0 after a previous call on the thread that did not grow the scratch past 64 KiB (`Cow::Borrowed`; with `Html`/`Escaped` only if nothing needs encoding)                                                                                                                                                             |
| `typeset`, output differs       | 1: the output string, capacity `len + len / 8`; a second only if it grows past that                                                                                                                                                                                                                                 |
| `typeset_with`                  | as `typeset`, with the caller's buffer instead of the thread-local one                                                                                                                                                                                                                                              |
| `typeset_into`                  | 0 when `buf` has room                                                                                                                                                                                                                                                                                               |
| `typeset_template`              | at most 3 (the list of piece texts, the texts and slots vectors, each sized once from the lower bound of the iterator's `size_hint`) plus one per text that is changed or merged from more than one non-empty piece; an iterator whose lower bound is below its length makes the vectors reallocate while they grow |
| `format_number`, `format_range` | 0 (`Display`)                                                                                                                                                                                                                                                                                                       |
| `report*`, `lint*`              | vectors of edits, spans and diagnostics (the debugging path)                                                                                                                                                                                                                                                        |
| building                        | only in `build()`; none in `Typographer::new`                                                                                                                                                                                                                                                                       |

`report*`, `lint*` and `to_plain*` use the thread-local scratch only: they are the debugging and
tooling path and make no allocation promise, so version 1 has no `_with` variants of them (adding
one later is not breaking). `typeset_template` has no `_into` form, because its result owns its
vectors.

The working buffers (the call's text of a template, atoms and tokens of the current window, two
token vectors for phases, the quote stack) live in a `Scratch`. Atoms and tokens hold no references
into the input: a token stores a byte range of the call's text (the input itself, or the
concatenation of the `Text` pieces of a template) and, when a rule replaced its text, a replacement
id (sections 3.4, 3.5). So `Scratch` holds only `'static` data, is `Send + Sync`, and is reused for
inputs of any lifetime without `unsafe` (the crate forbids it). `Scratch::new()` is a `const fn`,
and `Scratch` implements `Default`. `typeset` uses one held in a `thread_local!` (`RefCell<Scratch>`
with `const` initialization) and reused, so after the first call on a thread the hot path does not
allocate. If the capacity of its buffers grew past 64 KiB in total on a large input, the
thread-local scratch is replaced by `Scratch::new()` after the call, so the next call on the thread
allocates again as a first call does. A caller-held `Scratch` of `typeset_with` is never shrunk. The
pieces of a template are the only caller code that runs during a call: the iterator is consumed into
the call-local vector of piece texts before the thread-local `Scratch` is borrowed, so an iterator
that calls the typographer meets a free scratch. Should `try_borrow_mut` fail anyway, the call works
with a local buffer, without panicking. `typeset_with` takes a caller-held `Scratch` (one per
worker), for callers that do not want thread-local state, for example executors that move tasks
between threads or a future `no_std` build. The texts of template pieces are the only per-call data
with a lifetime; they go into a call-local vector, the third allocation of `typeset_template`. In
`typeset*` a prefilter returns `Cow::Borrowed` without touching the buffers when the input contains
no byte that any active rule reacts to and no byte that the output kind encodes (section 3.2);
`report*` and `lint*` always run the pipeline. A test with a counting global allocator checks this
table (section 13.8).

Rejected: a new token vector on every call (one to three allocations per string, unchanged ones
included); streaming without a buffer (quote pairing defers decisions to the end of a paragraph).

### 2.13 SemVer commitments

- Every public enum is `#[non_exhaustive]`; so are variants with fields. Structs with invariants
  have private fields and `const` constructors.
- Commitments that cannot be lifted without a breaking release: `Typographer::new`,
  `Language::from_static` and `Scratch::new` are `const`; `Typographer: Send + Sync + Clone`;
  `Scratch: Send + Sync + Default + Debug` (its `Debug` shows no contents); `NumberData: Default`,
  equal to `NumberData::new()` (Clippy's `new_without_default` asks for it); `Output`, `RuleSet`,
  `Rule`, `Slot`, `HtmlStyle`, `EscapeStyle` are `Copy`; `Severity` is `Copy` and ordered,
  `Info < Warning`, and a level added later takes its place in that order; `Piece<'t, S>` is `Copy`
  when `S` is; `Language` and `LanguageData` are not `Copy` (deliberately); `LanguageData` setters
  are `const fn`; `Language`, `LanguageData` and the value types of section 2.1 (the rows from
  `QuotePairs` to `NumberData`) implement `Debug`, `PartialEq`, `Eq` and `Hash`, and the value types
  also `Copy`, so a field added to them later must have these traits too (language data holds
  strings, characters, numbers and flags, which do); rule ids are permanent. `Number` holds floats
  and is `PartialEq` only.
- Not part of the contract: the order of `Rule::ALL` (it is the execution order, and a new rule may
  go into the middle); the representation of `RuleSet`; the contents of language lists; the text of
  `summary()` and of the `Display` of `Error`, `Diagnostic` and `Report`. The `Display` of `Rule`,
  `RuleSet` and `Language` is part of the contract: it prints the id, the id list and the tag, and
  `FromStr` reads it back. For `Language` the round trip covers the built-in languages and `COMMON`
  (section 2.2): the tag of a caller's language (`from_data`, `from_static`, `LanguageBuilder`) is
  read against the registry too, so `FromStr` answers it with `Error::UnknownLanguage`, or, when
  `from_tag` resolves the tag (a built-in tag, its bare language subtag such as `ru`, or `und`),
  with that built-in language or `COMMON`, which is not equal to the caller's language when their
  data differ.
- The policy for changes of output, for new built-in languages and for the MSRV is section 15.1.
- `cargo-semver-checks` is not installed on the development machine. For 0.1.0 there is no previous
  release; the public surface is fixed by the doctests. From the second release on, the preparation
  of each release compares the public surface with the previous release (`cargo semver-checks` where
  installed, otherwise a review of `cargo doc` output).

### 2.14 Build-time use with Fluent

The crate has no Fluent dependency; the generator maps Fluent syntax to pieces itself:

| Fluent                            | Piece                                                                                         |
| --------------------------------- | --------------------------------------------------------------------------------------------- |
| `TextElement`                     | `Piece::Text`                                                                                 |
| `{ "…" }` (StringLiteral)         | `Piece::Text` with the unescaped value (`{ "\u00A0" }` gives a glue that is already in place) |
| `{ 42 }` (NumberLiteral)          | `Piece::Text("42")`: a literal takes part in rules                                            |
| `{ NUMBER($n) }`                  | `Piece::Slot(_, Slot::NUMBER)`                                                                |
| `{ DATETIME($d) }`                | `Piece::Slot(_, Slot::DATE)`                                                                  |
| `{ -brand }`, `{ other-message }` | `Piece::Slot(_, Slot::WORD)`                                                                  |
| `{ $var }`                        | the kind from the generator's schema; `Slot::OTHER` when unknown                              |
| `{ $n -> [one] … *[other] … }`    | one `Slot(_, Slot::WORD)` in the outer template; every variant is typeset as its own template |
| attribute `.title`                | its own template                                                                              |

Protection delimiters in text elements are matched across placeables:
``Run `git checkout -- { $branch }` to reset.`` keeps the whole command, with its slot, protected
(section 10.1).

Select variants are typeset separately. A quote opened outside a select and closed inside a variant
produces `UnmatchedQuote` in both places, and the author rewrites the message. Rejected: typesetting
every combination of variants with the outer text (exponential, and the result would have to be cut
back apart); passing "boundary context" into a variant (API weight for a rare case).

A generator (in `build.rs` or a separate tool) turns warnings into `cargo::warning` lines and writes
texts as Rust literals:

```rust
use typolish::{EscapeStyle, Language, Output, Piece, RuleSet, Severity, TemplatePart, Typographer};

pub fn typographer(tag: &str) -> Result<Typographer, typolish::Error> {
    Typographer::builder(Language::from_tag(tag)?, RuleSet::INTERFACE)
        .add_short_words(["через"])
        .add_units(["ед."])
        .build()
}

fn emit(id: &str, pieces: Vec<Piece<'_, usize>>, t: &Typographer) -> String {
    let r = t.report_template(pieces, Output::Escaped(EscapeStyle::RUST));
    for d in r.diagnostics() {
        if d.severity() >= Severity::Warning {
            println!("cargo::warning={id}: {d}");
        }
    }
    let mut code = String::from("|args: &[&str]| { let mut s = String::new(); ");
    for part in r.result().iter() {
        match part {
            TemplatePart::Text(t) => code.push_str(&format!("s.push_str(\"{t}\"); ")),
            TemplatePart::Slot(i, _) => code.push_str(&format!("s.push_str(args[{i}]); ")),
            // build-time code: failing loudly beats losing text
            _ => panic!("typolish: a TemplatePart variant this generator does not know; update it"),
        }
    }
    code.push_str("s }");
    code
}
```

`EscapeStyle::RUST` makes each text the body of a Rust string literal, so the generator pastes it
between quotes without escaping of its own (the container is escaped completely, section 11.6). At
run time the application formats `Number` arguments with `format_number` of the same typographer
(the same `typographer(tag)` function in a shared crate) and `Output::Unicode`, so template and
value fit together: the compiler has already turned the literal body into Unicode, and a value
escaped again would show its escapes (`1\u{a0}000` instead of `1⍽000`). Values are inserted as they
are (section 11.1). A generator of a JS bundle writes the texts of an `Output::Escaped` template
with `EscapeStyle::JSON` or `EscapeStyle::JS` the same way, and its run-time values are plain
Unicode too. With `Output::Html` the texts are HTML, not bodies of string literals, and no
`EscapeStyle` applies to them: they may hold `\` (from a protected span such as `C:\temp`), raw line
breaks and, with a nowrap style, the `"` of `<span class="nowrap">`. Such a generator writes each
text as a complete string literal of the target language itself. In Rust code that is
`TemplatePart::Text(t) => code.push_str(&format!("s.push_str({t:?}); "))` (the `Debug` form of `str`
is a valid Rust string literal); in a JS or JSON bundle it is the JSON string of the text, written
by a JSON encoder, with `<`, U+2028 and U+2029 written as `\u003c`, `\u2028` and `\u2029` as
`EscapeStyle::JSON` does (section 11.6), so that every JavaScript engine reads it back unchanged,
also inside a `<script>` element (the `Debug` form is not JSON: it writes U+00A0 as `\u{a0}`). Its
run-time code escapes `Word`, `Date` and `Other` values itself and formats numbers with the same
`Output::Html`; the crate encodes only the texts (section 11.1).

### 2.15 Per-request use

```rust
use std::borrow::Cow;
use typolish::{HtmlStyle, Language, Output, RuleSet, Typographer};

static RU: Typographer = Typographer::new(Language::RU_RU, RuleSet::PROSE);
static EN: Typographer = Typographer::new(Language::EN_US, RuleSet::PROSE);

fn typographer(tag: &str) -> &'static Typographer {
    if tag.starts_with("ru") { &RU } else { &EN }
}

fn render_comment<'a>(tag: &str, body: &'a str) -> Cow<'a, str> {
    typographer(tag).typeset(body, Output::Html(HtmlStyle::NAMED))
}
```

The typographers are true statics; each call costs one linear pass and allocates only when the text
changes or needs encoding.
