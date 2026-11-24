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
    /// Text to typeset.
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
    /// separate call or a `\n\n` at the end of the `Text` piece before it (design section 10.8).
    /// Written as is in Unicode and HTML output, escaped in escaped output.
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
    /// A number formatted by `format_number` of the same typographer.
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
    /// A word or a name.
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
    /// A date or a time.
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
    /// Anything else: a path, a URL, code, a string of unknown length.
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

/// The kind of a slot: how it takes part in the rules around it (the slot participation table of
/// design section 2.6).
///
/// ```
/// use typolish::{Slot, SlotKind};
///
/// assert_eq!(Slot::OTHER.kind(), SlotKind::Other);
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum SlotKind {
    /// A number formatted by `format_number` of the same typographer (count, size, port).
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

    /// The n + 1 texts; `texts()[i]` stands before `slots()[i]`.
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
