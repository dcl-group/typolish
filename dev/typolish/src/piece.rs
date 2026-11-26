//! Input pieces of a template and the typeset template (design section 2.6).

use std::borrow::Cow;

/// One input piece of a message template. `S` is the caller's payload for a slot (a variable
/// name, an AST node index); the crate never reads it.
///
/// ```
/// use typolish::{Piece, Slot};
///
/// let pieces = [Piece::Text("Файл \""), Piece::Slot("name", Slot::WORD), Piece::Text("\"")];
/// assert_eq!(pieces.len(), 3);
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum Piece<'t, S> {
    /// Text to typeset. Adjacent `Text`, `Protected`, `Markup` and `BlockMarkup` pieces come out as
    /// one entry of [`Template::texts`]; to tell which input piece an edit belongs to, use the
    /// journal of [`Typographer::report_template`](crate::Typographer::report_template), made with
    /// [`Output::Unicode`](crate::Output::Unicode).
    ///
    /// ```
    /// use typolish::{Language, Output, Piece, RuleSet, Slot, Typographer};
    ///
    /// static RU: Typographer = Typographer::new(Language::RU_RU, RuleSet::INTERFACE);
    ///
    /// let t = RU.typeset_template([Piece::<()>::Text("Файл не найден")], Output::Unicode);
    /// assert_eq!(t.texts(), ["Файл не\u{a0}найден"]);
    /// ```
    Text(&'t str),
    /// An opaque placeholder whose value is unknown; rules see only its kind.
    ///
    /// ```
    /// use typolish::{Language, Output, Piece, RuleSet, Slot, Typographer};
    ///
    /// static RU: Typographer = Typographer::new(Language::RU_RU, RuleSet::INTERFACE);
    ///
    /// let t = RU.typeset_template(
    ///     [Piece::Text("Файл \""), Piece::Slot(7, Slot::WORD), Piece::Text("\"")],
    ///     Output::Unicode,
    /// );
    /// assert_eq!(t.texts(), ["Файл «", "»"]);
    /// assert_eq!(t.slots(), [(7, Slot::WORD)]);
    /// ```
    Slot(S, Slot),
    /// Visible text that is never typeset (a command, a path, code). Acts as one word for its
    /// neighbors; encoded for the output kind.
    ///
    /// ```
    /// use typolish::{Language, Output, Piece, RuleSet, Slot, Typographer};
    ///
    /// static RU: Typographer = Typographer::new(Language::RU_RU, RuleSet::INTERFACE);
    ///
    /// let t = RU.typeset_template(
    ///     [
    ///         Piece::<()>::Text("Запустите "),
    ///         Piece::Protected("--no-cache -x"),
    ///         Piece::Text(" - и ждите"),
    ///     ],
    ///     Output::Unicode,
    /// );
    /// assert_eq!(t.texts(), ["Запустите --no-cache -x\u{a0}— и\u{a0}ждите"]);
    /// ```
    Protected(&'t str),
    /// Invisible inline markup (for example `<b>` or `</em>` supplied by an HTML splitter).
    /// Transparent for rules, so it ends no paragraph: a block boundary such as `</p><p>` is a
    /// [`Piece::BlockMarkup`]. Written as is in Unicode and HTML output, escaped in escaped
    /// output.
    ///
    /// ```
    /// use typolish::{Language, Output, Piece, RuleSet, Slot, Typographer};
    ///
    /// static RU: Typographer = Typographer::new(Language::RU_RU, RuleSet::INTERFACE);
    ///
    /// let t = RU.typeset_template(
    ///     [
    ///         Piece::<()>::Text("Файл "),
    ///         Piece::Markup("<b>"),
    ///         Piece::Text("не найден"),
    ///         Piece::Markup("</b>"),
    ///     ],
    ///     Output::Unicode,
    /// );
    /// assert_eq!(t.texts(), ["Файл <b>не\u{a0}найден</b>"]);
    /// ```
    Markup(&'t str),
    /// Invisible markup at a block boundary (for example `</p><p>` or `</li><li>` supplied by an
    /// HTML splitter). Written like [`Piece::Markup`], as is in Unicode and HTML output and
    /// escaped in escaped output, but it also ends the paragraph, as a blank line does (design
    /// section 3.3): no quote pair, glue or dash operand reaches across it, and the text after it
    /// starts a line, so `dash.dialogue` places a dialogue dash there.
    ///
    /// ```
    /// use typolish::{HtmlStyle, Language, Output, Piece, RuleSet, Typographer};
    ///
    /// static RU: Typographer = Typographer::new(Language::RU_RU, RuleSet::PROSE);
    ///
    /// let t = RU.typeset_template(
    ///     [
    ///         Piece::<()>::Text("Он сказал: \"Да.\""),
    ///         Piece::BlockMarkup("</p><p>"),
    ///         Piece::Text("- Нет, - ответила она."),
    ///     ],
    ///     Output::Html(HtmlStyle::NAMED),
    /// );
    /// assert_eq!(
    ///     t.texts(),
    ///     ["Он&nbsp;сказал: &laquo;Да.&raquo;</p><p>&mdash;&nbsp;Нет,&nbsp;&mdash; ответила она."]
    /// );
    /// ```
    BlockMarkup(&'t str),
}

/// An opaque placeholder. The field is private so that hints can be added later.
///
/// ```
/// use typolish::{Slot, SlotKind};
///
/// assert_eq!(Slot::NUMBER.kind(), SlotKind::Number);
/// assert_eq!(Slot::new(SlotKind::Date), Slot::DATE);
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Slot {
    kind: SlotKind,
}

impl Slot {
    /// A number formatted by `format_number` of the same typographer. What it enables is
    /// [`SlotKind::Number`].
    ///
    /// ```
    /// use typolish::{Language, Output, Piece, RuleSet, Slot, Typographer};
    ///
    /// static RU: Typographer = Typographer::new(Language::RU_RU, RuleSet::INTERFACE);
    ///
    /// let t = RU.typeset_template(
    ///     [Piece::Slot("n", Slot::NUMBER), Piece::Text(" дн.")],
    ///     Output::Unicode,
    /// );
    /// assert_eq!(t.texts(), ["", "\u{a0}дн."]);
    /// ```
    pub const NUMBER: Slot = Slot::new(SlotKind::Number);
    /// A word or a name. What it enables is [`SlotKind::Word`].
    ///
    /// ```
    /// use typolish::{Language, Output, Piece, RuleSet, Slot, Typographer};
    ///
    /// static RU: Typographer = Typographer::new(Language::RU_RU, RuleSet::INTERFACE);
    ///
    /// let t = RU.typeset_template(
    ///     [Piece::Text("Файл \""), Piece::Slot("name", Slot::WORD), Piece::Text("\" удалён")],
    ///     Output::Unicode,
    /// );
    /// assert_eq!(t.texts(), ["Файл «", "» удалён"]);
    /// ```
    pub const WORD: Slot = Slot::new(SlotKind::Word);
    /// A date or a time. What it enables is [`SlotKind::Date`].
    ///
    /// ```
    /// use typolish::{Language, Output, Piece, RuleSet, Slot, Typographer};
    ///
    /// static RU: Typographer = Typographer::new(Language::RU_RU, RuleSet::INTERFACE);
    ///
    /// let t = RU.typeset_template(
    ///     [Piece::Slot("from", Slot::DATE), Piece::Text("-"), Piece::Slot("to", Slot::DATE)],
    ///     Output::Unicode,
    /// );
    /// assert_eq!(t.texts(), ["", "–", ""]);
    /// ```
    pub const DATE: Slot = Slot::new(SlotKind::Date);
    /// Anything else: a path, a URL, code, a string of unknown length. What it enables is
    /// [`SlotKind::Other`]; it is the slot to pass when the kind is unknown.
    ///
    /// ```
    /// use typolish::{Language, Output, Piece, RuleSet, Slot, Typographer};
    ///
    /// static RU: Typographer = Typographer::new(Language::RU_RU, RuleSet::INTERFACE);
    ///
    /// // No glue sticks to a slot of unknown length.
    /// let t = RU.typeset_template(
    ///     [Piece::Text("в "), Piece::Slot("path", Slot::OTHER)],
    ///     Output::Unicode,
    /// );
    /// assert_eq!(t.texts(), ["в ", ""]);
    /// ```
    pub const OTHER: Slot = Slot::new(SlotKind::Other);

    /// A slot of the given kind.
    ///
    /// ```
    /// use typolish::{Slot, SlotKind};
    ///
    /// assert_eq!(Slot::new(SlotKind::Number), Slot::NUMBER);
    /// ```
    #[must_use]
    pub const fn new(kind: SlotKind) -> Slot {
        Slot { kind }
    }

    /// The kind of the slot.
    ///
    /// ```
    /// use typolish::{Slot, SlotKind};
    ///
    /// assert_eq!(Slot::WORD.kind(), SlotKind::Word);
    /// ```
    #[must_use]
    pub const fn kind(self) -> SlotKind {
        self.kind
    }
}

/// The kind of a slot: how it takes part in the rules around it.
///
/// A slot is one opaque, frozen token; the rules never edit it, and for its neighbors it behaves
/// as its kind says. The table is the slot participation table of design section 2.6 (a rule
/// named in it applies only where the rule set holds it):
///
/// | Context                                                                               | `Number`                                                        | `Word`   | `Date`              | `Other`               |
/// | ------------------------------------------------------------------------------------- | --------------------------------------------------------------- | -------- | ------------------- | --------------------- |
/// | quotes around it (`"{x}"`)                                                            | word                                                            | word     | word                | word                  |
/// | apostrophe after it (`{x}'s`)                                                         | yes                                                             | yes      | no                  | no                    |
/// | quote after it as a prime (`{n}"`, Prose)                                             | yes, if the quote is unpaired                                   | no       | no                  | no                    |
/// | `range.placeholders` (`{a}-{b}`)                                                      | with another `Number`                                           | no       | with another `Date` | no                    |
/// | `nbsp.units` on its left (`{n} МБ`)                                                   | yes                                                             | no       | no                  | no                    |
/// | `nbsp.dates` (`{n} мая`, `{d} г.`)                                                    | day or year                                                     | no       | year                | no                    |
/// | a `Next` abbreviation after it does not bind forward (`{n} ч. Потом`, `{d} г. Затем`) | yes                                                             | no       | yes                 | no                    |
/// | `number.times` (`{w}x{h}`, Prose)                                                     | yes                                                             | no       | no                  | no                    |
/// | glue before it (`в {x}`, `№ {x}`)                                                     | yes                                                             | yes      | yes                 | no                    |
/// | glue after it (`{x} же`)                                                              | yes                                                             | yes      | yes                 | only `nbsp.dash`      |
/// | operand of `dash.spaced-hyphen` (`{x} - слово`)                                       | yes; next to a number, a range or a single-letter word: no dash | yes      | yes                 | yes                   |
/// | `nbsp.dash` after it (`{x} - слово`, `{x}, - сказал`)                                 | yes                                                             | yes      | yes                 | yes                   |
/// | the word after `dash.dialogue` (`- {x} пришёл`, Prose)                                | no                                                              | yes      | no                  | yes; the space stays  |
/// | a word for `ellipsis.inner` (`{a}...{b}`, Prose)                                      | no                                                              | yes      | no                  | yes                   |
/// | `SlotGlue` diagnostic                                                                 | `{n}МБ`, `МБ{n}`, `{a}{b}`                                      | `{a}{b}` | `{a}{b}`            | `{a}{b}`              |
/// | any rule across a `BlockMarkup` piece (`{x}</p><p> МБ`, `{a}</p><p>{b}`)              | no                                                              | no       | no                  | no                    |
///
/// No glue sticks to an `Other` slot, nor to the fragment that holds it: `и "{o}"` and `в ({o})`
/// keep their spaces. The one exception is the space before a dash: in a language whose dash
/// style is [`DashStyle::SpacedGlued`](crate::DashStyle::SpacedGlued) (ru-RU) a dash never starts
/// a line after a slot of any kind, so `nbsp.dash` binds it to the slot as it binds it to a word
/// (ru `{o} - слово` → `{o}⍽— слово`, `"{o}" - слово` → `«{o}»⍽— слово`), and with a nowrap style
/// that glue is written as an entity, so no nowrap group takes the value. en-US, whose dash is
/// closed, and `Language::COMMON` keep the author's space before a dash after a slot of every kind
/// (en `{x} - word` → `{x} — word`; `Language::COMMON` keeps `{x} — word` as it is). A `Number`
/// slot after `№`, `§` or `¶` is a label number: `nbsp.signs` glues it to the sign, and
/// `nbsp.units` and `nbsp.dates` leave it alone.
///
/// # When the kind is unknown
///
/// Pass [`SlotKind::Other`] ([`Slot::OTHER`]) for a placeholder whose kind the caller does not
/// know. It assumes the least about the value: no glue sticks to it but the one before a dash
/// after it, so a long value is never held in one unbreakable group with its neighbors, and no
/// rule reads it as a number, a date or a year, so no unit, day, year, range, dimension or prime
/// is glued or edited on a guess. The price is the glue a short value would have had (`в {x}`,
/// `{x} МБ`). `Word` would glue a value of any length; `Number` and `Date` would read the text
/// around a value that is no number or date as units, ranges and dates.
///
/// `Other` is not invisible to the rules: like every kind it is a word for the quotes around it,
/// and like `Word` it is an operand of a spaced hyphen and, in Prose, the word of `dash.dialogue`
/// and `ellipsis.inner`. So `{a} - {b}` gets a dash glued to the slot before it with two `Other`
/// slots and keeps the hyphen with two `Number` slots. Where that matters, declare the kind.
///
/// ```
/// use typolish::{Language, Output, Piece, RuleSet, Slot, Typographer};
///
/// static RU: Typographer = Typographer::new(Language::RU_RU, RuleSet::INTERFACE);
///
/// // The texts of `pieces` with every slot of the kind `slot`.
/// fn texts(pieces: &[Piece<'static, ()>], slot: Slot) -> Vec<String> {
///     let pieces = pieces.iter().map(|p| match p {
///         Piece::Slot(..) => Piece::Slot((), slot),
///         other => *other,
///     });
///     let t = RU.typeset_template(pieces, Output::Unicode);
///     t.texts().iter().map(|text| text.to_string()).collect()
/// }
///
/// let unit = [Piece::Slot((), Slot::OTHER), Piece::Text(" МБ")];
/// assert_eq!(texts(&unit, Slot::NUMBER), ["", "\u{a0}МБ"]);
/// assert_eq!(texts(&unit, Slot::OTHER), ["", " МБ"]);
///
/// let before = [Piece::Text("в "), Piece::Slot((), Slot::OTHER)];
/// assert_eq!(texts(&before, Slot::WORD), ["в\u{a0}", ""]);
/// assert_eq!(texts(&before, Slot::OTHER), ["в ", ""]);
///
/// let hyphen = [Piece::Slot((), Slot::OTHER), Piece::Text(" - "), Piece::Slot((), Slot::OTHER)];
/// assert_eq!(texts(&hyphen, Slot::NUMBER), ["", " - ", ""]);
/// assert_eq!(texts(&hyphen, Slot::OTHER), ["", "\u{a0}— ", ""]);
///
/// // In Russian a dash never starts a line after a slot, whatever its kind.
/// let dash = [Piece::Slot((), Slot::OTHER), Piece::Text(" - слово")];
/// assert_eq!(texts(&dash, Slot::WORD), ["", "\u{a0}— слово"]);
/// assert_eq!(texts(&dash, Slot::OTHER), ["", "\u{a0}— слово"]);
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum SlotKind {
    /// A number formatted by `format_number` of the same typographer (count, size, port).
    ///
    /// Enables what a number enables: glue before a unit, a month or a year word after it, a range
    /// dash between two `Number` slots, the times sign and, in Prose, a prime after it; also glue
    /// before and after it and an apostrophe after it. A spaced hyphen between it and a number, a
    /// range or a single-letter word stays a hyphen, and it is not a word for `dash.dialogue` and
    /// `ellipsis.inner`. See the table of [`SlotKind`].
    ///
    /// ```
    /// use typolish::{Language, Output, Piece, RuleSet, Slot, SlotKind, Typographer};
    ///
    /// static RU: Typographer = Typographer::new(Language::RU_RU, RuleSet::INTERFACE);
    ///
    /// let t = RU.typeset_template(
    ///     [Piece::Slot("x", Slot::new(SlotKind::Number)), Piece::Text(" дн.")],
    ///     Output::Unicode,
    /// );
    /// assert_eq!(t.slots(), [("x", Slot::new(SlotKind::Number))]);
    /// assert_eq!(t.texts(), ["", "\u{a0}дн."]);
    /// ```
    Number,
    /// A word or a name.
    ///
    /// Enables what a word enables: glue before and after it, an apostrophe after it, a dash
    /// operand, and the word of `dash.dialogue` and `ellipsis.inner`. No rule reads it as a number
    /// or a date. See the table of [`SlotKind`].
    ///
    /// ```
    /// use typolish::{Language, Output, Piece, RuleSet, Slot, SlotKind, Typographer};
    ///
    /// static RU: Typographer = Typographer::new(Language::RU_RU, RuleSet::INTERFACE);
    ///
    /// let t = RU.typeset_template(
    ///     [
    ///         Piece::Text("Файл \""),
    ///         Piece::Slot("x", Slot::new(SlotKind::Word)),
    ///         Piece::Text("\" удалён"),
    ///     ],
    ///     Output::Unicode,
    /// );
    /// assert_eq!(t.slots(), [("x", Slot::new(SlotKind::Word))]);
    /// assert_eq!(t.texts(), ["Файл «", "» удалён"]);
    /// ```
    Word,
    /// A date or a time, in words or digits.
    ///
    /// Enables glue before and after it, glue before a year word after it, a range dash between
    /// two `Date` slots, and a dash operand. No apostrophe after it, and not a word for
    /// `dash.dialogue` and `ellipsis.inner`. See the table of [`SlotKind`].
    ///
    /// ```
    /// use typolish::{Language, Output, Piece, RuleSet, Slot, SlotKind, Typographer};
    ///
    /// static RU: Typographer = Typographer::new(Language::RU_RU, RuleSet::INTERFACE);
    ///
    /// let t = RU.typeset_template(
    ///     [
    ///         Piece::Slot("x", Slot::new(SlotKind::Date)),
    ///         Piece::Text("-"),
    ///         Piece::Slot("x", Slot::new(SlotKind::Date)),
    ///     ],
    ///     Output::Unicode,
    /// );
    /// assert_eq!(t.slots(), [("x", Slot::DATE), ("x", Slot::DATE)]);
    /// assert_eq!(t.texts(), ["", "–", ""]);
    /// ```
    Date,
    /// Anything else: a path, a URL, code, a string of unknown length.
    ///
    /// Enables the least: no glue before or after it except the space before a dash after it, no
    /// apostrophe, and no rule reads it as a number or a date; it is still a word for quotes, a
    /// dash operand, and the word of `dash.dialogue` and `ellipsis.inner`. The kind to pass when
    /// the kind is unknown (see [`SlotKind`]).
    ///
    /// ```
    /// use typolish::{Language, Output, Piece, RuleSet, Slot, SlotKind, Typographer};
    ///
    /// static RU: Typographer = Typographer::new(Language::RU_RU, RuleSet::INTERFACE);
    ///
    /// let t = RU.typeset_template(
    ///     [Piece::Text("в "), Piece::Slot("x", Slot::new(SlotKind::Other))],
    ///     Output::Unicode,
    /// );
    /// assert_eq!(t.slots(), [("x", Slot::new(SlotKind::Other))]);
    /// assert_eq!(t.texts(), ["в ", ""]);
    /// ```
    Other,
}

/// A typeset template: n slots and exactly n + 1 texts; `texts()[i]` stands before `slots()[i]`.
///
/// ```
/// use typolish::{Language, Output, Piece, RuleSet, Slot, Typographer};
///
/// static RU: Typographer = Typographer::new(Language::RU_RU, RuleSet::INTERFACE);
///
/// let t = RU.typeset_template(
///     [Piece::Text("Привет, "), Piece::Slot("name", Slot::WORD), Piece::Text("!")],
///     Output::Unicode,
/// );
/// assert_eq!(t.texts(), ["Привет, ", "!"]);
/// assert_eq!(t.fill(|name| (*name).into()), "Привет, name!");
/// ```
///
/// A code generator walks the result in order with [`Template::iter`] and writes texts and slots
/// back interleaved; empty texts are skipped. Here each part becomes a line of Rust code (the
/// `Debug` form of a `str` is a valid Rust string literal):
///
/// ```
/// use typolish::{Language, Output, Piece, RuleSet, Slot, TemplatePart, Typographer};
///
/// static RU: Typographer = Typographer::new(Language::RU_RU, RuleSet::INTERFACE);
///
/// let t = RU.typeset_template(
///     [
///         Piece::Slot("count", Slot::NUMBER),
///         Piece::Text(" файлов в папке \""),
///         Piece::Slot("dir", Slot::WORD),
///         Piece::Text("\""),
///     ],
///     Output::Unicode,
/// );
/// let mut code = String::new();
/// for part in t.iter() {
///     match part {
///         TemplatePart::Text(text) => code.push_str(&format!("s.push_str({text:?});\n")),
///         TemplatePart::Slot(name, _) => code.push_str(&format!("s.push_str({name});\n")),
///         // `TemplatePart` is non-exhaustive: refuse a part this generator does not know
///         // rather than lose text.
///         _ => return Err("a template part this generator does not know"),
///     }
/// }
/// assert_eq!(
///     code,
///     "s.push_str(count);\n\
///      s.push_str(\" файлов в\\u{a0}папке «\");\n\
///      s.push_str(dir);\n\
///      s.push_str(\"»\");\n"
/// );
/// # Ok::<(), &str>(())
/// ```
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Template<'t, S> {
    texts: Vec<Cow<'t, str>>,
    slots: Vec<(S, Slot)>,
    changed: bool,
}

/// One part of a typeset template, in order.
///
/// A part borrows the payload of its slot, so it is `Copy` whatever the payload type (design
/// section 2.13).
///
/// ```
/// use typolish::{Language, Output, Piece, RuleSet, Slot, TemplatePart, Typographer};
///
/// static EN: Typographer = Typographer::new(Language::EN_US, RuleSet::INTERFACE);
///
/// let t = EN.typeset_template(
///     [Piece::Slot(0usize, Slot::NUMBER), Piece::Text(" files")],
///     Output::Unicode,
/// );
/// let parts: Vec<_> = t.iter().collect();
/// assert_eq!(
///     parts,
///     [
///         TemplatePart::Slot(&0, Slot::NUMBER),
///         TemplatePart::Text(" files")
///     ]
/// );
/// ```
#[derive(Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum TemplatePart<'a, S> {
    /// A typeset text.
    ///
    /// ```
    /// use typolish::{Language, Output, Piece, RuleSet, Slot, TemplatePart, Typographer};
    ///
    /// static EN: Typographer = Typographer::new(Language::EN_US, RuleSet::INTERFACE);
    ///
    /// let pieces = [Piece::Slot(1u8, Slot::NUMBER), Piece::Text(" files")];
    /// let t = EN.typeset_template(pieces, Output::Unicode);
    /// let mut parts = t.iter();
    /// assert_eq!(parts.next(), Some(TemplatePart::Slot(&1, Slot::NUMBER)));
    /// assert_eq!(parts.next(), Some(TemplatePart::Text(" files")));
    /// ```
    Text(&'a str),
    /// A slot with the caller's payload.
    ///
    /// ```
    /// use typolish::{Language, Output, Piece, RuleSet, Slot, TemplatePart, Typographer};
    ///
    /// static EN: Typographer = Typographer::new(Language::EN_US, RuleSet::INTERFACE);
    ///
    /// let pieces = [Piece::Slot(1u8, Slot::NUMBER), Piece::Text(" files")];
    /// let t = EN.typeset_template(pieces, Output::Unicode);
    /// let mut parts = t.iter();
    /// assert_eq!(parts.next(), Some(TemplatePart::Slot(&1, Slot::NUMBER)));
    /// ```
    Slot(&'a S, Slot),
}

// By hand: a derive would require `S: Copy`, which a borrowed payload does not need.
impl<S> Clone for TemplatePart<'_, S> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<S> Copy for TemplatePart<'_, S> {}

impl<'t, S> Template<'t, S> {
    /// A template from its parts; `texts.len() == slots.len() + 1`.
    pub(crate) fn from_parts(
        texts: Vec<Cow<'t, str>>,
        slots: Vec<(S, Slot)>,
        changed: bool,
    ) -> Template<'t, S> {
        Template {
            texts,
            slots,
            changed,
        }
    }

    /// The n + 1 texts; `texts()[i]` stands before `slots()[i]`. Adjacent text pieces of the input
    /// are one entry here; [`Report::changes`](crate::Report::changes) is the supported way to
    /// distribute the result over them again: the records rebuild the
    /// [`Output::Unicode`](crate::Output::Unicode) result, and each rebuilt part is encoded on its
    /// own for another output kind, without the nowrap groups that span pieces.
    ///
    /// ```
    /// use typolish::{Language, Output, Piece, RuleSet, Slot, Typographer};
    ///
    /// static RU: Typographer = Typographer::new(Language::RU_RU, RuleSet::INTERFACE);
    ///
    /// let t = RU.typeset_template(
    ///     [Piece::Slot("n", Slot::NUMBER), Piece::Text(" из "), Piece::Slot("m", Slot::NUMBER)],
    ///     Output::Unicode,
    /// );
    /// assert_eq!(t.texts(), ["", " из\u{a0}", ""]);
    /// ```
    #[must_use]
    pub fn texts(&self) -> &[Cow<'t, str>] {
        &self.texts
    }

    /// The n slots in input order, each with the caller's payload.
    ///
    /// ```
    /// use typolish::{Language, Output, Piece, RuleSet, Slot, Typographer};
    ///
    /// static RU: Typographer = Typographer::new(Language::RU_RU, RuleSet::INTERFACE);
    ///
    /// let t = RU.typeset_template(
    ///     [Piece::Slot("n", Slot::NUMBER), Piece::Text(" из "), Piece::Slot("m", Slot::NUMBER)],
    ///     Output::Unicode,
    /// );
    /// assert_eq!(t.slots(), [("n", Slot::NUMBER), ("m", Slot::NUMBER)]);
    /// ```
    #[must_use]
    pub fn slots(&self) -> &[(S, Slot)] {
        &self.slots
    }

    /// Whether any text differs from the input text it came from (a rule edited it or the output
    /// kind encoded it). Merging adjacent input pieces into one text is not a change.
    ///
    /// ```
    /// use typolish::{Language, Output, Piece, RuleSet, Slot, Typographer};
    ///
    /// static RU: Typographer = Typographer::new(Language::RU_RU, RuleSet::INTERFACE);
    ///
    /// let plain = RU.typeset_template(
    ///     [Piece::Text("Привет, "), Piece::Slot(0, Slot::WORD)],
    ///     Output::Unicode,
    /// );
    /// assert!(!plain.changed());
    /// let edited = RU.typeset_template(
    ///     [Piece::Slot(0, Slot::WORD), Piece::Text(" не найден")],
    ///     Output::Unicode,
    /// );
    /// assert!(edited.changed());
    /// ```
    #[must_use]
    pub fn changed(&self) -> bool {
        self.changed
    }

    /// The parts in order; empty texts are skipped.
    ///
    /// ```
    /// use typolish::{Language, Output, Piece, RuleSet, Slot, TemplatePart, Typographer};
    ///
    /// static RU: Typographer = Typographer::new(Language::RU_RU, RuleSet::INTERFACE);
    ///
    /// let t = RU.typeset_template(
    ///     [Piece::Slot("n", Slot::NUMBER), Piece::Text(" МБ")],
    ///     Output::Unicode,
    /// );
    /// let parts: Vec<_> = t.iter().collect();
    /// // The empty text before the slot is skipped.
    /// assert_eq!(
    ///     parts,
    ///     [
    ///         TemplatePart::Slot(&"n", Slot::NUMBER),
    ///         TemplatePart::Text("\u{a0}МБ")
    ///     ]
    /// );
    /// ```
    pub fn iter(&self) -> impl Iterator<Item = TemplatePart<'_, S>> {
        let mut slots = self.slots.iter();
        self.texts.iter().flat_map(move |text| {
            let text = (!text.is_empty()).then_some(TemplatePart::Text(text.as_ref()));
            let slot = slots
                .next()
                .map(|(payload, slot)| TemplatePart::Slot(payload, *slot));
            text.into_iter().chain(slot)
        })
    }

    /// The texts and the slots.
    ///
    /// ```
    /// use typolish::{Language, Output, Piece, RuleSet, Slot, Typographer};
    ///
    /// static RU: Typographer = Typographer::new(Language::RU_RU, RuleSet::INTERFACE);
    ///
    /// let t = RU.typeset_template(
    ///     [Piece::Slot("n", Slot::NUMBER), Piece::Text(" МБ")],
    ///     Output::Unicode,
    /// );
    /// let (texts, slots) = t.into_parts();
    /// assert_eq!(texts, ["", "\u{a0}МБ"]);
    /// assert_eq!(slots, [("n", Slot::NUMBER)]);
    /// ```
    #[must_use]
    pub fn into_parts(self) -> (Vec<Cow<'t, str>>, Vec<(S, Slot)>) {
        (self.texts, self.slots)
    }

    /// Joins the texts with slot values, for tests and run-time use without code generation.
    ///
    /// Values are inserted as they are: with `Output::Html` or `Output::Escaped` the closure must
    /// return values already encoded for that output (design section 11.1). In the JavaScript
    /// styles a text never ends in a raw `$`; a value encoded by other means must neither end in a
    /// raw `$` nor start with a raw `{`, so that no `${` forms across a slot boundary.
    ///
    /// ```
    /// use typolish::{Language, Output, Piece, RuleSet, Slot, Typographer};
    ///
    /// static RU: Typographer = Typographer::new(Language::RU_RU, RuleSet::INTERFACE);
    ///
    /// let t = RU.typeset_template(
    ///     [Piece::Slot("n", Slot::NUMBER), Piece::Text(" МБ")],
    ///     Output::Unicode,
    /// );
    /// let size = RU.format_number(1536, Output::Unicode).to_string();
    /// assert_eq!(t.fill(|_| size.as_str().into()), "1\u{a0}536\u{a0}МБ");
    /// ```
    #[must_use]
    pub fn fill<'v>(&self, mut value: impl FnMut(&S) -> Cow<'v, str>) -> String {
        let mut out = String::new();
        for part in self.iter() {
            match part {
                TemplatePart::Text(text) => out.push_str(text),
                TemplatePart::Slot(payload, _) => out.push_str(&value(payload)),
            }
        }
        out
    }
}
