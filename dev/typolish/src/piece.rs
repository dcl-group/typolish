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
    Number,
    /// A word or a name.
    Word,
    /// A date or a time, in words or digits.
    Date,
    /// Anything else: a path, a URL, code, a string of unknown length.
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
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum TemplatePart<'a, S> {
    /// A typeset text.
    Text(&'a str),
    /// A slot with the caller's payload.
    Slot(&'a S, Slot),
}

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
    /// return values already encoded for that output (design section 11.1).
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
