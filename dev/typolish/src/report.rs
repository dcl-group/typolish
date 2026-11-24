//! `report`: the result of a call together with its edits, protected spans and diagnostics
//! (design sections 2.9, 12.4 and 12.5).

use core::fmt;
use core::ops::Range;
use std::borrow::Cow;

use crate::Rule;

/// The result of [`Typographer::report`](crate::Typographer::report) or
/// [`Typographer::report_template`](crate::Typographer::report_template): what `typeset` or
/// `typeset_template` returns, plus every edit, every protected span and every diagnostic.
///
/// ```
/// use typolish::{Language, Output, ProtectedKind, RuleSet, TechKind, Typographer};
///
/// static RU: Typographer = Typographer::new(Language::RU_RU, RuleSet::INTERFACE);
///
/// let r = RU.report("Сервис слушает host:8080", Output::Unicode);
/// assert_eq!(r.result(), "Сервис слушает host:8080");
/// let span = &r.protected()[0];
/// assert_eq!(span.kind(), ProtectedKind::Technical(TechKind::Colon));
/// assert_eq!(&"Сервис слушает host:8080"[span.range()], "host:8080");
/// ```
#[derive(Clone, Debug)]
pub struct Report<'a, T = Cow<'a, str>> {
    result: T,
    changes: Vec<Change>,
    protected: Vec<ProtectedSpan>,
    diagnostics: Vec<Diagnostic>,
    /// The text of every input piece, by piece index; a slot has an empty text.
    inputs: Vec<&'a str>,
}

impl<'a, T> Report<'a, T> {
    /// A report from its parts.
    pub(crate) fn new(result: T, records: Records, inputs: Vec<&'a str>) -> Report<'a, T> {
        Report {
            result,
            changes: records.changes,
            protected: records.protected,
            diagnostics: records.diagnostics,
            inputs,
        }
    }

    /// The result, the same as `typeset` or `typeset_template` returns.
    ///
    /// ```
    /// use typolish::{Language, Output, RuleSet, Typographer};
    ///
    /// let t = Typographer::new(Language::EN_US, RuleSet::INTERFACE);
    /// assert_eq!(t.report("plain", Output::Unicode).result(), "plain");
    /// ```
    #[must_use]
    pub fn result(&self) -> &T {
        &self.result
    }

    /// The result, consuming the report.
    ///
    /// ```
    /// use typolish::{Language, Output, RuleSet, Typographer};
    ///
    /// let t = Typographer::new(Language::EN_US, RuleSet::INTERFACE);
    /// let text = t.report("plain", Output::Unicode).into_result();
    /// assert_eq!(text, "plain");
    /// ```
    #[must_use]
    pub fn into_result(self) -> T {
        self.result
    }

    /// The edits, ordered by position, then by rule order; an edit across a boundary of `Text`
    /// pieces gives one record per piece (design section 2.9).
    ///
    /// ```
    /// use typolish::{Language, Output, RuleSet, Typographer};
    ///
    /// let t = Typographer::new(Language::EN_US, RuleSet::EMPTY);
    /// assert!(t.report("No rule runs.", Output::Unicode).changes().is_empty());
    /// ```
    #[must_use]
    pub fn changes(&self) -> &[Change] {
        &self.changes
    }

    /// The spans that no rule may touch, with the reason (design section 12.5), ordered by piece
    /// and position; a span over several pieces gives one record per piece (design section 3.11).
    ///
    /// ```
    /// use typolish::{Language, Output, ProtectedKind, RuleSet, Typographer};
    ///
    /// let t = Typographer::new(Language::EN_US, RuleSet::INTERFACE);
    /// let r = t.report("Run `ls -la` now", Output::Unicode);
    /// assert_eq!(r.protected()[0].kind(), ProtectedKind::Delimited);
    /// assert_eq!(r.protected()[0].range(), 4..12);
    /// ```
    #[must_use]
    pub fn protected(&self) -> &[ProtectedSpan] {
        &self.protected
    }

    /// The diagnostics, ordered by piece, byte range, then kind (design section 12.4). A
    /// diagnostic over several pieces is reported at its first piece, with the range clipped to
    /// that piece (design section 3.11).
    ///
    /// ```
    /// use typolish::{DiagnosticKind, Language, Output, RuleSet, Severity, Typographer};
    ///
    /// let t = Typographer::new(Language::RU_RU, RuleSet::INTERFACE);
    /// let r = t.report("Мoсква", Output::Unicode); // a Latin `o`
    /// assert_eq!(r.diagnostics()[0].kind(), &DiagnosticKind::MixedScript);
    /// assert_eq!(r.diagnostics()[0].severity(), Severity::Warning);
    /// ```
    #[must_use]
    pub fn diagnostics(&self) -> &[Diagnostic] {
        &self.diagnostics
    }

    /// The input text that an edit replaced: the slice of its own piece.
    ///
    /// ```
    /// use typolish::{Language, Output, RuleSet, Typographer};
    ///
    /// let t = Typographer::new(Language::EN_US, RuleSet::PROSE);
    /// let r = t.report("She said \"hi\" - and left", Output::Unicode);
    /// assert_eq!(r.original(&r.changes()[0]), "\"");
    /// assert_eq!(r.original(&r.changes()[2]), "-");
    /// ```
    #[must_use]
    pub fn original(&self, change: &Change) -> &'a str {
        self.inputs
            .get(change.piece)
            .and_then(|text| text.get(change.range.clone()))
            .unwrap_or("")
    }
}

impl<T> fmt::Display for Report<'_, T> {
    /// A table for reports of differences (design section 12.5): one line per record of
    /// `changes()` (rule, `piece:range`, the input text, `->`, the replacement), then one line per
    /// record of `protected()` (`protected`, the kind, `piece:range`, the protected text). Each
    /// column is as wide as its longest entry plus two spaces, separately for the two parts. A
    /// text that is a single invisible character (a space other than U+0020, a control or a
    /// format character) is written as `U+XXXX`, any other text in quotes, with `"`, `\` and
    /// invisible characters escaped. The text is not part of the API contract (design section
    /// 2.13).
    ///
    /// ```
    /// use typolish::{Language, Output, RuleSet, Typographer};
    ///
    /// static RU: Typographer = Typographer::new(Language::RU_RU, RuleSet::INTERFACE);
    ///
    /// let r = RU.report("Цена - 5 руб.", Output::Unicode);
    /// assert_eq!(r.to_string(), "\
    /// nbsp.dash           0:8..9    \" \" -> U+00A0
    /// dash.spaced-hyphen  0:9..10   \"-\" -> \"—\"
    /// nbsp.units          0:12..13  \" \" -> U+00A0
    /// ");
    /// ```
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let place =
            |piece: usize, range: &Range<usize>| format!("{piece}:{}..{}", range.start, range.end);
        let rule_width = self
            .changes
            .iter()
            .map(|c| c.rule.id().len())
            .max()
            .unwrap_or(0)
            .saturating_add(2);
        let place_width = self
            .changes
            .iter()
            .map(|c| place(c.piece, &c.range).len())
            .max()
            .unwrap_or(0)
            .saturating_add(2);
        for change in &self.changes {
            writeln!(
                f,
                "{:<rule_width$}{:<place_width$}{} -> {}",
                change.rule.id(),
                place(change.piece, &change.range),
                Shown(self.original(change)),
                Shown(&change.replacement),
            )?;
        }
        let kind_width = self
            .protected
            .iter()
            .map(|s| format!("{:?}", s.kind).len())
            .max()
            .unwrap_or(0)
            .saturating_add(2);
        let place_width = self
            .protected
            .iter()
            .map(|s| place(s.piece, &s.range).len())
            .max()
            .unwrap_or(0)
            .saturating_add(2);
        for span in &self.protected {
            let text = self
                .inputs
                .get(span.piece)
                .and_then(|text| text.get(span.range.clone()))
                .unwrap_or("");
            let line = format!(
                "protected  {:<kind_width$}{:<place_width$}{}",
                format!("{:?}", span.kind),
                place(span.piece, &span.range),
                Raw(text),
            );
            writeln!(f, "{}", line.trim_end_matches(' '))?;
        }
        Ok(())
    }
}

/// Whether a character is invisible in a table or an error message: a space other than U+0020,
/// a control, a character of general category `Cf` or a reserved default-ignorable code point
/// (design sections 2.11, 7.6 and 12.5; a wider set than the format characters of the lexer,
/// design section 5.4).
pub(crate) fn invisible(c: char) -> bool {
    (c.is_whitespace() && c != ' ')
        || c.is_control()
        || crate::lang::data::is_format_category(c)
        || crate::lang::data::is_reserved_default_ignorable(c)
}

/// A text of the change table: `U+XXXX` for a single invisible character, otherwise quoted.
struct Shown<'s>(&'s str);

impl fmt::Display for Shown<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut chars = self.0.chars();
        if let (Some(c), None) = (chars.next(), chars.next())
            && invisible(c)
        {
            return write!(f, "U+{:04X}", u32::from(c));
        }
        f.write_str("\"")?;
        for c in self.0.chars() {
            match c {
                '"' => f.write_str("\\\"")?,
                '\\' => f.write_str("\\\\")?,
                c if invisible(c) => write!(f, "\\u{{{:x}}}", u32::from(c))?,
                c => write!(f, "{c}")?,
            }
        }
        f.write_str("\"")
    }
}

/// A protected text in the table or a caller's string in an error message: as it is, with
/// invisible characters escaped, so that a record or a message stays on one line.
pub(crate) struct Raw<'s>(pub(crate) &'s str);

impl fmt::Display for Raw<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for c in self.0.chars() {
            if invisible(c) {
                write!(f, "\\u{{{:x}}}", u32::from(c))?;
            } else {
                write!(f, "{c}")?;
            }
        }
        Ok(())
    }
}

/// The records of one call, collected by the journal (design section 3.11).
#[derive(Debug, Default)]
pub(crate) struct Records {
    pub(crate) changes: Vec<Change>,
    pub(crate) protected: Vec<ProtectedSpan>,
    pub(crate) diagnostics: Vec<Diagnostic>,
}

/// One atomic edit in terms of the input and the Unicode result.
///
/// ```
/// use typolish::{Language, Output, RuleSet, Typographer};
///
/// let t = Typographer::new(Language::EN_US, RuleSet::PROSE);
/// let r = t.report("She said \"hi\" - and left", Output::Unicode);
/// let dash = &r.changes()[2];
/// assert_eq!(dash.rule().id(), "dash.spaced-hyphen");
/// assert_eq!((dash.piece(), dash.range()), (0, 14..15));
/// assert_eq!(dash.replacement(), "—");
/// ```
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Change {
    rule: Rule,
    piece: usize,
    range: Range<usize>,
    replacement: String,
}

impl Change {
    /// A record of one piece.
    pub(crate) fn new(
        rule: Rule,
        piece: usize,
        range: Range<usize>,
        replacement: String,
    ) -> Change {
        Change {
            rule,
            piece,
            range,
            replacement,
        }
    }

    /// The order of design section 2.9: piece, byte start, byte end, then rule order.
    pub(crate) fn sort_key(&self) -> (usize, usize, usize, u8) {
        (
            self.piece,
            self.range.start,
            self.range.end,
            self.rule as u8,
        )
    }

    /// The rule that made the edit.
    ///
    /// ```
    /// # use typolish::{Language, Output, RuleSet, Typographer};
    /// # let t = Typographer::new(Language::EN_US, RuleSet::PROSE);
    /// let r = t.report("She said \"hi\" - and left", Output::Unicode);
    /// assert_eq!(r.changes()[0].rule().id(), "quotes.pairs");
    /// ```
    #[must_use]
    pub fn rule(&self) -> Rule {
        self.rule
    }

    /// The index of the input piece (0 for string input).
    ///
    /// ```
    /// # use typolish::{Language, Output, RuleSet, Typographer};
    /// # let t = Typographer::new(Language::EN_US, RuleSet::PROSE);
    /// let r = t.report("She said \"hi\" - and left", Output::Unicode);
    /// assert_eq!(r.changes()[0].piece(), 0);
    /// ```
    #[must_use]
    pub fn piece(&self) -> usize {
        self.piece
    }

    /// The byte range in that piece; empty for an insertion.
    ///
    /// ```
    /// # use typolish::{Language, Output, RuleSet, Typographer};
    /// # let t = Typographer::new(Language::EN_US, RuleSet::PROSE);
    /// let r = t.report("She said \"hi\" - and left", Output::Unicode);
    /// assert_eq!(r.changes()[0].range(), 9..10);
    /// ```
    #[must_use]
    pub fn range(&self) -> Range<usize> {
        self.range.clone()
    }

    /// The replacement in Unicode, before output encoding; empty for a deletion.
    ///
    /// ```
    /// # use typolish::{Language, Output, RuleSet, Typographer};
    /// # let t = Typographer::new(Language::EN_US, RuleSet::PROSE);
    /// let r = t.report("She said \"hi\" - and left", Output::Unicode);
    /// assert_eq!(r.changes()[0].replacement(), "“");
    /// ```
    #[must_use]
    pub fn replacement(&self) -> &str {
        &self.replacement
    }
}

/// A span that no rule may touch, with the reason (design section 12.5).
///
/// ```
/// use typolish::{Language, Output, ProtectedKind, RuleSet, TechKind, Typographer};
///
/// let t = Typographer::new(Language::EN_US, RuleSet::INTERFACE);
/// let r = t.report("Use --force", Output::Unicode);
/// let span = &r.protected()[0];
/// assert_eq!((span.piece(), span.range()), (0, 4..11));
/// assert_eq!(span.kind(), ProtectedKind::Technical(TechKind::Flag));
/// ```
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ProtectedSpan {
    piece: usize,
    range: Range<usize>,
    kind: ProtectedKind,
}

impl ProtectedSpan {
    /// A record of one piece.
    pub(crate) fn new(piece: usize, range: Range<usize>, kind: ProtectedKind) -> ProtectedSpan {
        ProtectedSpan { piece, range, kind }
    }

    /// The index of the input piece (0 for string input).
    ///
    /// ```
    /// # use typolish::{Language, Output, RuleSet, Typographer};
    /// # let t = Typographer::new(Language::EN_US, RuleSet::INTERFACE);
    /// assert_eq!(t.report("a `b`", Output::Unicode).protected()[0].piece(), 0);
    /// ```
    #[must_use]
    pub fn piece(&self) -> usize {
        self.piece
    }

    /// The byte range in that piece. A slot has no text, so its record has an empty range at 0.
    ///
    /// ```
    /// # use typolish::{Language, Output, RuleSet, Typographer};
    /// # let t = Typographer::new(Language::EN_US, RuleSet::INTERFACE);
    /// assert_eq!(t.report("a `b`", Output::Unicode).protected()[0].range(), 2..5);
    /// ```
    #[must_use]
    pub fn range(&self) -> Range<usize> {
        self.range.clone()
    }

    /// Why the span is protected.
    ///
    /// ```
    /// # use typolish::{Language, Output, ProtectedKind, RuleSet, Typographer};
    /// # let t = Typographer::new(Language::EN_US, RuleSet::INTERFACE);
    /// let r = t.report("a `b`", Output::Unicode);
    /// assert_eq!(r.protected()[0].kind(), ProtectedKind::Delimited);
    /// ```
    #[must_use]
    pub fn kind(&self) -> ProtectedKind {
        self.kind
    }
}

/// Why a span is protected (design section 12.5).
///
/// ```
/// use typolish::{Language, Output, Piece, ProtectedKind, RuleSet, Slot, Typographer};
///
/// let t = Typographer::new(Language::EN_US, RuleSet::INTERFACE);
/// let r = t.report_template(
///     [Piece::Text("Run "), Piece::Protected("make"), Piece::Slot("n", Slot::NUMBER)],
///     Output::Unicode,
/// );
/// let kinds: Vec<_> = r.protected().iter().map(|s| s.kind()).collect();
/// assert_eq!(kinds, [ProtectedKind::ProtectedPiece, ProtectedKind::Slot]);
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum ProtectedKind {
    /// Between protection delimiters: backticks, a fenced block or an own pair, the delimiters
    /// included (design section 10.1).
    ///
    /// ```
    /// # use typolish::{Language, Output, ProtectedKind, RuleSet, Typographer};
    /// # let t = Typographer::new(Language::EN_US, RuleSet::INTERFACE);
    /// let r = t.report("Run `make` now", Output::Unicode);
    /// assert_eq!(r.protected()[0].kind(), ProtectedKind::Delimited);
    /// assert_eq!(r.protected()[0].range(), 4..10);
    /// ```
    Delimited,
    /// A `Piece::Protected`.
    ///
    /// ```
    /// # use typolish::{Language, Output, Piece, ProtectedKind, RuleSet, Typographer};
    /// # let t = Typographer::new(Language::EN_US, RuleSet::INTERFACE);
    /// let r = t.report_template([Piece::<&str>::Text("Run "), Piece::Protected("make")], Output::Unicode);
    /// assert_eq!(r.protected()[0].kind(), ProtectedKind::ProtectedPiece);
    /// ```
    ProtectedPiece,
    /// A slot.
    ///
    /// ```
    /// # use typolish::{Language, Output, Piece, ProtectedKind, RuleSet, Slot, Typographer};
    /// # let t = Typographer::new(Language::EN_US, RuleSet::INTERFACE);
    /// let r = t.report_template([Piece::Text("Got "), Piece::Slot("n", Slot::NUMBER)], Output::Unicode);
    /// assert_eq!(r.protected()[0].kind(), ProtectedKind::Slot);
    /// ```
    Slot,
    /// A fragment that the classifier froze (design section 3.6).
    ///
    /// ```
    /// # use typolish::{Language, Output, ProtectedKind, RuleSet, TechKind, Typographer};
    /// # let t = Typographer::new(Language::EN_US, RuleSet::INTERFACE);
    /// let r = t.report("https://example.com", Output::Unicode);
    /// assert_eq!(r.protected()[0].kind(), ProtectedKind::Technical(TechKind::Url));
    /// ```
    Technical(TechKind),
}

/// The kind of a frozen fragment (design section 3.6). `CompoundHyphens`: only the hyphens of a
/// compound word; the word itself is not technical and is an operand like any word (design
/// section 8.2).
///
/// ```
/// use typolish::{Language, Output, ProtectedKind, RuleSet, TechKind, Typographer};
///
/// let t = Typographer::new(Language::EN_US, RuleSet::INTERFACE);
/// let r = t.report("tls-server", Output::Unicode);
/// assert_eq!(r.protected()[0].kind(), ProtectedKind::Technical(TechKind::CompoundHyphens));
/// assert_eq!(r.protected()[0].range(), 3..4);
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum TechKind {
    /// Check 1: `https://example.com`, `www.example.com`.
    ///
    /// ```
    /// # use typolish::{Language, Output, ProtectedKind, RuleSet, TechKind, Typographer};
    /// # let t = Typographer::new(Language::EN_US, RuleSet::INTERFACE);
    /// let r = t.report("https://example.com", Output::Unicode);
    /// assert_eq!(r.protected()[0].kind(), ProtectedKind::Technical(TechKind::Url));
    /// assert_eq!(r.protected()[0].range(), 0..19);
    /// ```
    Url,
    /// Check 2: `user@example.com`, `@user`.
    ///
    /// ```
    /// # use typolish::{Language, Output, ProtectedKind, RuleSet, TechKind, Typographer};
    /// # let t = Typographer::new(Language::EN_US, RuleSet::INTERFACE);
    /// let r = t.report("user@example.com", Output::Unicode);
    /// assert_eq!(r.protected()[0].kind(), ProtectedKind::Technical(TechKind::Email));
    /// assert_eq!(r.protected()[0].range(), 0..16);
    /// ```
    Email,
    /// Check 3: `-v`, `--force`.
    ///
    /// ```
    /// # use typolish::{Language, Output, ProtectedKind, RuleSet, TechKind, Typographer};
    /// # let t = Typographer::new(Language::EN_US, RuleSet::INTERFACE);
    /// let r = t.report("--force", Output::Unicode);
    /// assert_eq!(r.protected()[0].kind(), ProtectedKind::Technical(TechKind::Flag));
    /// assert_eq!(r.protected()[0].range(), 0..7);
    /// ```
    Flag,
    /// Check 4: `/etc/hosts`, `C:\Users`.
    ///
    /// ```
    /// # use typolish::{Language, Output, ProtectedKind, RuleSet, TechKind, Typographer};
    /// # let t = Typographer::new(Language::EN_US, RuleSet::INTERFACE);
    /// let r = t.report("/etc/hosts", Output::Unicode);
    /// assert_eq!(r.protected()[0].kind(), ProtectedKind::Technical(TechKind::Path));
    /// assert_eq!(r.protected()[0].range(), 0..10);
    /// ```
    Path,
    /// Check 5: the two times of `10:00-12:00`, without the separator.
    ///
    /// ```
    /// # use typolish::{Language, Output, ProtectedKind, RuleSet, TechKind, Typographer};
    /// # let t = Typographer::new(Language::EN_US, RuleSet::INTERFACE);
    /// let r = t.report("10:00-12:00", Output::Unicode);
    /// assert_eq!(r.protected()[0].kind(), ProtectedKind::Technical(TechKind::TimeRange));
    /// assert_eq!(r.protected()[0].range(), 0..5);
    /// ```
    TimeRange,
    /// Check 6: `host:8080`, `::1`.
    ///
    /// ```
    /// # use typolish::{Language, Output, ProtectedKind, RuleSet, TechKind, Typographer};
    /// # let t = Typographer::new(Language::EN_US, RuleSet::INTERFACE);
    /// let r = t.report("host:8080", Output::Unicode);
    /// assert_eq!(r.protected()[0].kind(), ProtectedKind::Technical(TechKind::Colon));
    /// assert_eq!(r.protected()[0].range(), 0..9);
    /// ```
    Colon,
    /// Check 8: `a>>b`, `a=b`.
    ///
    /// ```
    /// # use typolish::{Language, Output, ProtectedKind, RuleSet, TechKind, Typographer};
    /// # let t = Typographer::new(Language::EN_US, RuleSet::INTERFACE);
    /// let r = t.report("a>>b", Output::Unicode);
    /// assert_eq!(r.protected()[0].kind(), ProtectedKind::Technical(TechKind::Operator));
    /// assert_eq!(r.protected()[0].range(), 0..4);
    /// ```
    Operator,
    /// Check 8 for `_` (`kv_lang`) and check 20 (`x86`).
    ///
    /// ```
    /// # use typolish::{Language, Output, ProtectedKind, RuleSet, TechKind, Typographer};
    /// # let t = Typographer::new(Language::EN_US, RuleSet::INTERFACE);
    /// let r = t.report("kv_lang", Output::Unicode);
    /// assert_eq!(r.protected()[0].kind(), ProtectedKind::Technical(TechKind::Identifier));
    /// assert_eq!(r.protected()[0].range(), 0..7);
    /// ```
    Identifier,
    /// Check 8: `<word>`.
    ///
    /// ```
    /// # use typolish::{Language, Output, ProtectedKind, RuleSet, TechKind, Typographer};
    /// # let t = Typographer::new(Language::EN_US, RuleSet::INTERFACE);
    /// let r = t.report("<word>", Output::Unicode);
    /// assert_eq!(r.protected()[0].kind(), ProtectedKind::Technical(TechKind::Angle));
    /// assert_eq!(r.protected()[0].range(), 0..6);
    /// ```
    Angle,
    /// Check 9.
    ///
    /// ```
    /// # use typolish::{Language, Output, ProtectedKind, RuleSet, TechKind, Typographer};
    /// # let t = Typographer::new(Language::EN_US, RuleSet::INTERFACE);
    /// let r = t.report("550e8400-e29b-41d4-a716-446655440000", Output::Unicode);
    /// assert_eq!(r.protected()[0].kind(), ProtectedKind::Technical(TechKind::Uuid));
    /// assert_eq!(r.protected()[0].range(), 0..36);
    /// ```
    Uuid,
    /// Checks 10 and 11: `0x1F`, `deadbeef0123`.
    ///
    /// ```
    /// # use typolish::{Language, Output, ProtectedKind, RuleSet, TechKind, Typographer};
    /// # let t = Typographer::new(Language::EN_US, RuleSet::INTERFACE);
    /// let r = t.report("0x1F", Output::Unicode);
    /// assert_eq!(r.protected()[0].kind(), ProtectedKind::Technical(TechKind::Hex));
    /// assert_eq!(r.protected()[0].range(), 0..4);
    /// ```
    Hex,
    /// Check 12: `2026-11-21`.
    ///
    /// ```
    /// # use typolish::{Language, Output, ProtectedKind, RuleSet, TechKind, Typographer};
    /// # let t = Typographer::new(Language::EN_US, RuleSet::INTERFACE);
    /// let r = t.report("2026-11-21", Output::Unicode);
    /// assert_eq!(r.protected()[0].kind(), ProtectedKind::Technical(TechKind::IsoDate));
    /// assert_eq!(r.protected()[0].range(), 0..10);
    /// ```
    IsoDate,
    /// Check 13: `0.2.27`, `10.0.0.1`, `21.11.2026`.
    ///
    /// ```
    /// # use typolish::{Language, Output, ProtectedKind, RuleSet, TechKind, Typographer};
    /// # let t = Typographer::new(Language::EN_US, RuleSet::INTERFACE);
    /// let r = t.report("0.2.27", Output::Unicode);
    /// assert_eq!(r.protected()[0].kind(), ProtectedKind::Technical(TechKind::Version));
    /// assert_eq!(r.protected()[0].range(), 0..6);
    /// ```
    Version,
    /// Check 18: `example.com`, `file.txt`.
    ///
    /// ```
    /// # use typolish::{Language, Output, ProtectedKind, RuleSet, TechKind, Typographer};
    /// # let t = Typographer::new(Language::EN_US, RuleSet::INTERFACE);
    /// let r = t.report("file.txt", Output::Unicode);
    /// assert_eq!(r.protected()[0].kind(), ProtectedKind::Technical(TechKind::Dotted));
    /// assert_eq!(r.protected()[0].range(), 0..8);
    /// ```
    Dotted,
    /// Check 19: `a,b,c`.
    ///
    /// ```
    /// # use typolish::{Language, Output, ProtectedKind, RuleSet, TechKind, Typographer};
    /// # let t = Typographer::new(Language::EN_US, RuleSet::INTERFACE);
    /// let r = t.report("a,b,c", Output::Unicode);
    /// assert_eq!(r.protected()[0].kind(), ProtectedKind::Technical(TechKind::List));
    /// assert_eq!(r.protected()[0].range(), 0..5);
    /// ```
    List,
    /// Check 21: the hyphens of `tls-server`.
    ///
    /// ```
    /// # use typolish::{Language, Output, ProtectedKind, RuleSet, TechKind, Typographer};
    /// # let t = Typographer::new(Language::EN_US, RuleSet::INTERFACE);
    /// let r = t.report("tls-server", Output::Unicode);
    /// assert_eq!(r.protected()[0].kind(), ProtectedKind::Technical(TechKind::CompoundHyphens));
    /// assert_eq!(r.protected()[0].range(), 3..4);
    /// ```
    CompoundHyphens,
    /// Check 24: anything the classifier does not recognize as a word or a number.
    ///
    /// ```
    /// # use typolish::{Language, Output, ProtectedKind, RuleSet, TechKind, Typographer};
    /// # let t = Typographer::new(Language::EN_US, RuleSet::INTERFACE);
    /// let r = t.report("×", Output::Unicode);
    /// assert_eq!(r.protected()[0].kind(), ProtectedKind::Technical(TechKind::Unclassified));
    /// assert_eq!(r.protected()[0].range(), 0..2);
    /// ```
    Unclassified,
}

/// A message about the input (design section 12.4).
///
/// ```
/// use typolish::{DiagnosticKind, Language, Output, RuleSet, Severity, Typographer};
///
/// let t = Typographer::new(Language::EN_US, RuleSet::INTERFACE);
/// let r = t.report("don`t", Output::Unicode);
/// let d = &r.diagnostics()[0];
/// assert_eq!(d.kind(), &DiagnosticKind::UnclosedDelimiter);
/// assert_eq!((d.severity(), d.rule(), d.piece(), d.range()), (Severity::Warning, None, 0, 3..4));
/// ```
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Diagnostic {
    kind: DiagnosticKind,
    severity: Severity,
    rule: Option<Rule>,
    piece: usize,
    range: Range<usize>,
}

impl Diagnostic {
    /// A diagnostic of one piece.
    pub(crate) fn new(
        kind: DiagnosticKind,
        severity: Severity,
        rule: Option<Rule>,
        piece: usize,
        range: Range<usize>,
    ) -> Diagnostic {
        Diagnostic {
            kind,
            severity,
            rule,
            piece,
            range,
        }
    }

    /// The kind.
    ///
    /// ```
    /// # use typolish::{DiagnosticKind, Language, Output, RuleSet, Typographer};
    /// # let t = Typographer::new(Language::EN_US, RuleSet::INTERFACE);
    /// let r = t.report("don`t", Output::Unicode);
    /// assert_eq!(r.diagnostics()[0].kind(), &DiagnosticKind::UnclosedDelimiter);
    /// ```
    #[must_use]
    pub fn kind(&self) -> &DiagnosticKind {
        &self.kind
    }

    /// The severity.
    ///
    /// ```
    /// # use typolish::{Language, Output, RuleSet, Severity, Typographer};
    /// # let t = Typographer::new(Language::EN_US, RuleSet::INTERFACE);
    /// let r = t.report("don`t", Output::Unicode);
    /// assert_eq!(r.diagnostics()[0].severity(), Severity::Warning);
    /// ```
    #[must_use]
    pub fn severity(&self) -> Severity {
        self.severity
    }

    /// The rule that raised it; `None` for protection, the classifier and the renderer.
    ///
    /// ```
    /// # use typolish::{Language, Output, RuleSet, Typographer};
    /// # let t = Typographer::new(Language::EN_US, RuleSet::INTERFACE);
    /// let r = t.report("don`t", Output::Unicode);
    /// assert_eq!(r.diagnostics()[0].rule(), None);
    /// ```
    #[must_use]
    pub fn rule(&self) -> Option<Rule> {
        self.rule
    }

    /// The index of the input piece (0 for string input).
    ///
    /// ```
    /// # use typolish::{Language, Output, RuleSet, Typographer};
    /// # let t = Typographer::new(Language::EN_US, RuleSet::INTERFACE);
    /// let r = t.report("don`t", Output::Unicode);
    /// assert_eq!(r.diagnostics()[0].piece(), 0);
    /// ```
    #[must_use]
    pub fn piece(&self) -> usize {
        self.piece
    }

    /// The byte range in that piece.
    ///
    /// ```
    /// # use typolish::{Language, Output, RuleSet, Typographer};
    /// # let t = Typographer::new(Language::EN_US, RuleSet::INTERFACE);
    /// let r = t.report("don`t", Output::Unicode);
    /// assert_eq!(r.diagnostics()[0].range(), 3..4);
    /// ```
    #[must_use]
    pub fn range(&self) -> Range<usize> {
        self.range.clone()
    }

    /// The order of design section 12.4: piece, byte start, byte end, then kind.
    pub(crate) fn sort_key(&self) -> (usize, usize, usize, u8) {
        (
            self.piece,
            self.range.start,
            self.range.end,
            self.kind.order(),
        )
    }
}

impl fmt::Display for Diagnostic {
    /// `warning protection 0:3..4: protection delimiter is not closed; treated as text`; the
    /// text is not part of the API contract (design section 2.13).
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let severity = match self.severity {
            Severity::Info => "info",
            Severity::Warning => "warning",
        };
        let source = match (self.rule, &self.kind) {
            (Some(rule), _) => rule.id(),
            (None, DiagnosticKind::UnclosedDelimiter) => "protection",
            (None, DiagnosticKind::NowrapFallback) => "renderer",
            (None, DiagnosticKind::TypographicChar { .. }) => "lint",
            (None, _) => "classifier",
        };
        write!(
            f,
            "{severity} {source} {}:{}..{}: ",
            self.piece, self.range.start, self.range.end
        )?;
        match (&self.kind, self.severity) {
            (DiagnosticKind::UnmatchedQuote, Severity::Warning) => {
                f.write_str("straight quote has no pair; left as is")
            }
            (DiagnosticKind::UnmatchedQuote, Severity::Info) => {
                f.write_str("quote has no partner; left neutral")
            }
            (DiagnosticKind::CrossingQuotes, _) => {
                f.write_str("quote pairs would cross; left as is")
            }
            (DiagnosticKind::AmbiguousQuote, _) => {
                f.write_str("quote has no clear role; left as is")
            }
            (DiagnosticKind::UnclosedDelimiter, _) => {
                f.write_str("protection delimiter is not closed; treated as text")
            }
            (DiagnosticKind::AmbiguousHyphen, _) => {
                f.write_str("hyphen is not turned into a dash here; left as is")
            }
            (DiagnosticKind::MixedScript, _) => f.write_str("word mixes letters of two scripts"),
            (DiagnosticKind::SlotGlue, _) => f.write_str("slot is glued to its neighbor"),
            (DiagnosticKind::NowrapFallback, _) => {
                f.write_str("nowrap group split at markup; glue written as an entity")
            }
            (DiagnosticKind::TypographicChar { found }, _) => write!(
                f,
                "typographic character U+{:04X} in a plain source",
                u32::from(*found)
            ),
        }
    }
}

/// The level of a diagnostic, ordered: `Info < Warning`. A level added later takes its place in
/// that order (design section 2.13).
///
/// ```
/// use typolish::Severity;
///
/// assert!(Severity::Info < Severity::Warning);
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[non_exhaustive]
pub enum Severity {
    /// For information; the output is as intended.
    ///
    /// ```
    /// # use typolish::{Language, Output, RuleSet, Severity, Typographer};
    /// # let t = Typographer::new(Language::EN_US, RuleSet::INTERFACE);
    /// let r = t.report("a - b", Output::Unicode);
    /// assert_eq!(r.diagnostics()[0].severity(), Severity::Info);
    /// ```
    Info,
    /// Something in the input could not be resolved and was left as it is.
    ///
    /// ```
    /// # use typolish::{Language, Output, RuleSet, Severity, Typographer};
    /// # let t = Typographer::new(Language::EN_US, RuleSet::INTERFACE);
    /// let r = t.report("He said \"hi", Output::Unicode);
    /// assert_eq!(r.diagnostics()[0].severity(), Severity::Warning);
    /// ```
    Warning,
}

/// What a diagnostic is about (design section 12.4).
///
/// ```
/// use typolish::{DiagnosticKind, Language, Output, Piece, RuleSet, Slot, Typographer};
///
/// let t = Typographer::new(Language::RU_RU, RuleSet::INTERFACE);
/// let r = t.report_template([Piece::Slot("n", Slot::NUMBER), Piece::Text("МБ")], Output::Unicode);
/// assert_eq!(r.diagnostics()[0].kind(), &DiagnosticKind::SlotGlue);
/// ```
#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum DiagnosticKind {
    /// A quote without a pair (`quotes.pairs`).
    ///
    /// ```
    /// # use typolish::{DiagnosticKind, Language, Output, RuleSet, Typographer};
    /// # let t = Typographer::new(Language::EN_US, RuleSet::INTERFACE);
    /// let r = t.report("He said \"hi", Output::Unicode);
    /// assert!(r.diagnostics().iter().any(|d| d.kind() == &DiagnosticKind::UnmatchedQuote));
    /// ```
    UnmatchedQuote,
    /// Quote pairs would cross each other or a typed pair (`quotes.pairs`).
    ///
    /// ```
    /// # use typolish::{DiagnosticKind, Language, Output, RuleSet, Typographer};
    /// # let t = Typographer::new(Language::RU_RU, RuleSet::INTERFACE);
    /// let r = t.report("«a \"b» c\"", Output::Unicode);
    /// assert!(r.diagnostics().iter().any(|d| d.kind() == &DiagnosticKind::CrossingQuotes));
    /// ```
    CrossingQuotes,
    /// A straight quote without a role (`quotes.pairs`).
    ///
    /// ```
    /// # use typolish::{DiagnosticKind, Language, Output, RuleSet, Typographer};
    /// # let t = Typographer::new(Language::EN_US, RuleSet::INTERFACE);
    /// let r = t.report("He said - \" a cat\"", Output::Unicode);
    /// assert!(r.diagnostics().iter().any(|d| d.kind() == &DiagnosticKind::AmbiguousQuote));
    /// ```
    AmbiguousQuote,
    /// An opening protection delimiter without its closing one; it was treated as text.
    ///
    /// ```
    /// # use typolish::{DiagnosticKind, Language, Output, RuleSet, Typographer};
    /// # let t = Typographer::new(Language::EN_US, RuleSet::INTERFACE);
    /// let r = t.report("don`t", Output::Unicode);
    /// assert!(r.diagnostics().iter().any(|d| d.kind() == &DiagnosticKind::UnclosedDelimiter));
    /// ```
    UnclosedDelimiter,
    /// A hyphen that the dash rules refuse (`dash.spaced-hyphen`, `dash.double-hyphen`).
    ///
    /// ```
    /// # use typolish::{DiagnosticKind, Language, Output, RuleSet, Typographer};
    /// # let t = Typographer::new(Language::EN_US, RuleSet::INTERFACE);
    /// let r = t.report("a - b", Output::Unicode);
    /// assert!(r.diagnostics().iter().any(|d| d.kind() == &DiagnosticKind::AmbiguousHyphen));
    /// ```
    AmbiguousHyphen,
    /// A word with letters of two scripts (`Мoсква` with a Latin `o`).
    ///
    /// ```
    /// # use typolish::{DiagnosticKind, Language, Output, RuleSet, Typographer};
    /// # let t = Typographer::new(Language::EN_US, RuleSet::INTERFACE);
    /// let r = t.report("Мoсква", Output::Unicode);
    /// assert!(r.diagnostics().iter().any(|d| d.kind() == &DiagnosticKind::MixedScript));
    /// ```
    MixedScript,
    /// A `Number` slot glued to letters (warning) or two slots glued together (info).
    ///
    /// ```
    /// # use typolish::{DiagnosticKind, Language, Output, Piece, RuleSet, Slot, Typographer};
    /// # let t = Typographer::new(Language::RU_RU, RuleSet::INTERFACE);
    /// let r = t.report_template([Piece::Slot("n", Slot::NUMBER), Piece::Text("МБ")], Output::Unicode);
    /// assert!(r.diagnostics().iter().any(|d| d.kind() == &DiagnosticKind::SlotGlue));
    /// ```
    SlotGlue,
    /// A nowrap group split at a `Markup` piece; the glue at the split written as an entity.
    ///
    /// ```
    /// # use typolish::{DiagnosticKind, HtmlStyle, Language, Output, Piece, RuleSet, Typographer};
    /// # let t = Typographer::new(Language::RU_RU, RuleSet::INTERFACE);
    /// let pieces = [Piece::<&str>::Text("в "), Piece::Markup("<b>"), Piece::Text("доме")];
    /// let html = Output::Html(HtmlStyle::NAMED.nowrap_class("nw"));
    /// let r = t.report_template(pieces, html);
    /// assert_eq!(r.diagnostics()[0].kind(), &DiagnosticKind::NowrapFallback);
    /// ```
    NowrapFallback,
    /// A character that `to_plain` would change (`lint` only).
    ///
    /// ```
    /// # use typolish::{DiagnosticKind, Language, RuleSet, Typographer};
    /// # let t = Typographer::new(Language::RU_RU, RuleSet::INTERFACE);
    /// let d = t.lint("Файл «a» не найден");
    /// assert!(matches!(d[0].kind(), DiagnosticKind::TypographicChar { found: '«', .. }));
    /// ```
    #[non_exhaustive]
    TypographicChar {
        /// The character found.
        found: char,
    },
}

impl DiagnosticKind {
    /// The position in the declaration order, which orders diagnostics at one range.
    fn order(&self) -> u8 {
        match self {
            DiagnosticKind::UnmatchedQuote => 0,
            DiagnosticKind::CrossingQuotes => 1,
            DiagnosticKind::AmbiguousQuote => 2,
            DiagnosticKind::UnclosedDelimiter => 3,
            DiagnosticKind::AmbiguousHyphen => 4,
            DiagnosticKind::MixedScript => 5,
            DiagnosticKind::SlotGlue => 6,
            DiagnosticKind::NowrapFallback => 7,
            DiagnosticKind::TypographicChar { .. } => 8,
        }
    }
}
