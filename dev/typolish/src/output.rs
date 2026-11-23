//! Output kinds (design sections 2.8 and 11) and their encoders.
//!
//! Rules work in Unicode and never see the output kind; the encoder writes the result in the
//! requested kind while rendering, straight into the output, with no intermediate Unicode string
//! (design section 11.1). The renderer (`engine::render`) feeds it the characters of the result
//! with where each comes from; the nowrap groups of section 11.5 are found there, and this module
//! writes their tags.

use core::fmt;

use crate::rules::Triggers;

/// How the result is written. Rules never depend on it (design section 11).
///
/// ```
/// use typolish::{EscapeStyle, HtmlStyle, Language, Output, RuleSet, Typographer};
///
/// static RU: Typographer = Typographer::new(Language::RU_RU, RuleSet::INTERFACE);
/// let src = "Цена - 5 руб.";
///
/// assert_eq!(RU.typeset(src, Output::Unicode), "Цена\u{a0}— 5\u{a0}руб.");
/// assert_eq!(
///     RU.typeset(src, Output::Html(HtmlStyle::NAMED)),
///     "Цена&nbsp;&mdash; 5&nbsp;руб."
/// );
/// assert_eq!(
///     RU.typeset(src, Output::Html(HtmlStyle::HEX)),
///     "Цена&#xA0;&#x2014; 5&#xA0;руб."
/// );
/// assert_eq!(
///     RU.typeset(src, Output::Html(HtmlStyle::NAMED.nowrap_class("nowrap"))),
///     "<span class=\"nowrap\">Цена &mdash;</span> <span class=\"nowrap\">5 руб.</span>",
/// );
/// assert_eq!(
///     RU.typeset(src, Output::Escaped(EscapeStyle::JS)),
///     "Цена\\u00A0\\u2014 5\\u00A0руб."
/// );
/// assert_eq!(
///     RU.typeset(src, Output::Escaped(EscapeStyle::JS_HEX)),
///     "Цена\\xA0\\u2014 5\\xA0руб."
/// );
/// assert_eq!(
///     RU.typeset(src, Output::Escaped(EscapeStyle::RUST)),
///     "Цена\\u{a0}\\u{2014} 5\\u{a0}руб."
/// );
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum Output {
    /// Characters as they are (UTF-8). Nothing is escaped.
    Unicode,
    /// HTML text: `& < > " '` escaped, typographic characters as entities, no-break groups as
    /// entities or as nowrap elements (design sections 11.4 and 11.5). Safe as element content and
    /// as an attribute value in either kind of quotes.
    Html(HtmlStyle),
    /// The body of a string literal of a given syntax, ready to paste between its quotes (design
    /// section 11.6).
    Escaped(EscapeStyle),
}

/// How characters of the typographic set are written in HTML (design section 11.4).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) enum Entities {
    Named,
    Decimal,
    Hex,
    Characters,
}

/// How glue is written in HTML (design section 11.5).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) enum Nowrap {
    Entity,
    Class(&'static str),
    Style,
}

/// Parameters of [`Output::Html`]: how the typographic set is written and how no-break groups
/// are rendered.
///
/// ```
/// use typolish::{HtmlStyle, Language, Output, RuleSet, Typographer};
///
/// const OUT: Output = Output::Html(HtmlStyle::HEX.nowrap_style());
/// static RU: Typographer = Typographer::new(Language::RU_RU, RuleSet::INTERFACE);
///
/// assert_eq!(RU.typeset("и т. д.", OUT), "<span style=\"white-space:nowrap\">и т. д.</span>");
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct HtmlStyle {
    pub(crate) entities: Entities,
    pub(crate) nowrap: Nowrap,
}

impl HtmlStyle {
    /// Named references (`&nbsp;`, `&mdash;`), hexadecimal ones where HTML has no name.
    ///
    /// ```
    /// use typolish::{HtmlStyle, Language, Output, RuleSet, Typographer};
    ///
    /// static RU: Typographer = Typographer::new(Language::RU_RU, RuleSet::INTERFACE);
    ///
    /// assert_eq!(RU.typeset("да - нет", Output::Html(HtmlStyle::NAMED)), "да&nbsp;&mdash; нет");
    /// ```
    pub const NAMED: HtmlStyle = HtmlStyle {
        entities: Entities::Named,
        nowrap: Nowrap::Entity,
    };
    /// Decimal references (`&#160;`, `&#8212;`).
    ///
    /// ```
    /// use typolish::{HtmlStyle, Language, Output, RuleSet, Typographer};
    ///
    /// static RU: Typographer = Typographer::new(Language::RU_RU, RuleSet::INTERFACE);
    ///
    /// assert_eq!(RU.typeset("да - нет", Output::Html(HtmlStyle::DECIMAL)), "да&#160;&#8212; нет");
    /// ```
    pub const DECIMAL: HtmlStyle = HtmlStyle {
        entities: Entities::Decimal,
        nowrap: Nowrap::Entity,
    };
    /// Hexadecimal references (`&#xA0;`, `&#x2014;`).
    ///
    /// ```
    /// use typolish::{HtmlStyle, Language, Output, RuleSet, Typographer};
    ///
    /// static RU: Typographer = Typographer::new(Language::RU_RU, RuleSet::INTERFACE);
    ///
    /// assert_eq!(RU.typeset("да - нет", Output::Html(HtmlStyle::HEX)), "да&#xA0;&#x2014; нет");
    /// ```
    pub const HEX: HtmlStyle = HtmlStyle {
        entities: Entities::Hex,
        nowrap: Nowrap::Entity,
    };
    /// Typographic characters as characters; only markup is escaped.
    ///
    /// ```
    /// use typolish::{HtmlStyle, Language, Output, RuleSet, Typographer};
    ///
    /// static RU: Typographer = Typographer::new(Language::RU_RU, RuleSet::INTERFACE);
    ///
    /// assert_eq!(
    ///     RU.typeset("да - <нет>", Output::Html(HtmlStyle::CHARACTERS)),
    ///     "да\u{a0}— &lt;нет&gt;"
    /// );
    /// ```
    pub const CHARACTERS: HtmlStyle = HtmlStyle {
        entities: Entities::Characters,
        nowrap: Nowrap::Entity,
    };

    /// Wraps each no-break group in `<span class="{class}">`; U+00A0 glue becomes an ordinary
    /// space inside (design section 11.5). The class is escaped as an attribute value.
    ///
    /// ```
    /// use typolish::{HtmlStyle, Language, Output, RuleSet, Typographer};
    ///
    /// static RU: Typographer = Typographer::new(Language::RU_RU, RuleSet::INTERFACE);
    ///
    /// let out = Output::Html(HtmlStyle::NAMED.nowrap_class("nowrap"));
    /// assert_eq!(RU.typeset("да - нет", out), "<span class=\"nowrap\">да &mdash;</span> нет");
    /// ```
    #[must_use]
    pub const fn nowrap_class(self, class: &'static str) -> HtmlStyle {
        HtmlStyle {
            entities: self.entities,
            nowrap: Nowrap::Class(class),
        }
    }

    /// The same as [`HtmlStyle::nowrap_class`] with `<span style="white-space:nowrap">`.
    ///
    /// ```
    /// use typolish::{HtmlStyle, Language, Output, RuleSet, Typographer};
    ///
    /// static RU: Typographer = Typographer::new(Language::RU_RU, RuleSet::INTERFACE);
    ///
    /// let out = Output::Html(HtmlStyle::NAMED.nowrap_style());
    /// assert_eq!(
    ///     RU.typeset("да - нет", out),
    ///     "<span style=\"white-space:nowrap\">да &mdash;</span> нет"
    /// );
    /// ```
    #[must_use]
    pub const fn nowrap_style(self) -> HtmlStyle {
        HtmlStyle {
            entities: self.entities,
            nowrap: Nowrap::Style,
        }
    }
}

/// The string syntax of [`Output::Escaped`] (design section 11.6).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) enum Syntax {
    Js,
    JsHex,
    JsCodePoint,
    Json,
    Rust,
    Css,
}

/// Which characters besides the container's own become escapes.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) enum Scope {
    Typographic,
    NonAscii,
}

/// Parameters of [`Output::Escaped`]: the syntax of the string literal and the scope of escaping.
///
/// The container is always escaped completely: `\`, the quotes of the syntax and control
/// characters, whatever the scope. With the default scope the typographic set of design section
/// 11.2 is escaped too; [`EscapeStyle::non_ascii`] escapes every non-ASCII character.
///
/// ```
/// use typolish::{EscapeStyle, Language, Output, RuleSet, Typographer};
///
/// static RU: Typographer = Typographer::new(Language::RU_RU, RuleSet::INTERFACE);
///
/// assert_eq!(RU.typeset("Путь `C:\\temp` - \"да\"", Output::Escaped(EscapeStyle::RUST)),
///            "Путь `C:\\\\temp`\\u{a0}\\u{2014} \\u{ab}да\\u{bb}");
/// assert_eq!(RU.typeset("Да", Output::Escaped(EscapeStyle::JSON.non_ascii())), "\\u0414\\u0430");
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct EscapeStyle {
    pub(crate) syntax: Syntax,
    pub(crate) scope: Scope,
}

impl EscapeStyle {
    /// JavaScript: `\u00A0`; above the BMP a surrogate pair. Safe in every kind of JavaScript
    /// string literal and inside an HTML `<script>` element.
    ///
    /// ```
    /// use typolish::{EscapeStyle, Language, Output, RuleSet, Typographer};
    ///
    /// static RU: Typographer = Typographer::new(Language::RU_RU, RuleSet::INTERFACE);
    ///
    /// assert_eq!(
    ///     RU.typeset("да - нет", Output::Escaped(EscapeStyle::JS)),
    ///     "да\\u00A0\\u2014 нет"
    /// );
    /// ```
    pub const JS: EscapeStyle = EscapeStyle::of(Syntax::Js);
    /// JavaScript: `\xA0` up to U+00FF, `\u2014` above, surrogate pairs above the BMP.
    ///
    /// ```
    /// use typolish::{EscapeStyle, Language, Output, RuleSet, Typographer};
    ///
    /// static RU: Typographer = Typographer::new(Language::RU_RU, RuleSet::INTERFACE);
    ///
    /// assert_eq!(
    ///     RU.typeset("да - нет", Output::Escaped(EscapeStyle::JS_HEX)),
    ///     "да\\xA0\\u2014 нет"
    /// );
    /// ```
    pub const JS_HEX: EscapeStyle = EscapeStyle::of(Syntax::JsHex);
    /// JavaScript (ES2015): `\u{A0}`, no surrogate pairs.
    ///
    /// ```
    /// use typolish::{EscapeStyle, Language, Output, RuleSet, Typographer};
    ///
    /// static RU: Typographer = Typographer::new(Language::RU_RU, RuleSet::INTERFACE);
    ///
    /// let out = Output::Escaped(EscapeStyle::JS_CODE_POINT);
    /// assert_eq!(RU.typeset("да - нет", out), "да\\u{A0}\\u{2014} нет");
    /// ```
    pub const JS_CODE_POINT: EscapeStyle = EscapeStyle::of(Syntax::JsCodePoint);
    /// JSON: `\u00a0`; `<`, `>`, `&`, U+2028 and U+2029 escaped too.
    ///
    /// ```
    /// use typolish::{EscapeStyle, Language, Output, RuleSet, Typographer};
    ///
    /// static RU: Typographer = Typographer::new(Language::RU_RU, RuleSet::INTERFACE);
    ///
    /// let out = Output::Escaped(EscapeStyle::JSON);
    /// assert_eq!(
    ///     RU.typeset("да - \"<нет>\"", out),
    ///     "да\\u00a0\\u2014 \\u00ab\\u003cнет\\u003e\\u00bb"
    /// );
    /// ```
    pub const JSON: EscapeStyle = EscapeStyle::of(Syntax::Json);
    /// Rust: `\u{a0}`, the body of a normal (non-raw) string literal.
    ///
    /// ```
    /// use typolish::{EscapeStyle, Language, Output, RuleSet, Typographer};
    ///
    /// static RU: Typographer = Typographer::new(Language::RU_RU, RuleSet::INTERFACE);
    ///
    /// assert_eq!(
    ///     RU.typeset("да - нет", Output::Escaped(EscapeStyle::RUST)),
    ///     "да\\u{a0}\\u{2014} нет"
    /// );
    /// ```
    pub const RUST: EscapeStyle = EscapeStyle::of(Syntax::Rust);
    /// CSS: `\a0` followed by a space, the body of a CSS string.
    ///
    /// ```
    /// use typolish::{EscapeStyle, Language, Output, RuleSet, Typographer};
    ///
    /// static RU: Typographer = Typographer::new(Language::RU_RU, RuleSet::INTERFACE);
    ///
    /// assert_eq!(RU.typeset("да - нет", Output::Escaped(EscapeStyle::CSS)), "да\\a0 \\2014  нет");
    /// ```
    pub const CSS: EscapeStyle = EscapeStyle::of(Syntax::Css);

    const fn of(syntax: Syntax) -> EscapeStyle {
        EscapeStyle {
            syntax,
            scope: Scope::Typographic,
        }
    }

    /// Escapes every non-ASCII character, not only the typographic set.
    ///
    /// ```
    /// use typolish::{EscapeStyle, Language, Output, RuleSet, Typographer};
    ///
    /// static RU: Typographer = Typographer::new(Language::RU_RU, RuleSet::INTERFACE);
    ///
    /// let out = Output::Escaped(EscapeStyle::RUST.non_ascii());
    /// assert_eq!(
    ///     RU.typeset("да - нет", out),
    ///     "\\u{434}\\u{430}\\u{a0}\\u{2014} \\u{43d}\\u{435}\\u{442}"
    /// );
    /// ```
    #[must_use]
    pub const fn non_ascii(self) -> EscapeStyle {
        EscapeStyle {
            syntax: self.syntax,
            scope: Scope::NonAscii,
        }
    }
}

/// The typographic set of design section 11.2 with the name of each character in HTML, if it has
/// one; checked against the WHATWG list of named character references (`tests/output.rs`).
pub(crate) const TYPOGRAPHIC: [(char, Option<&str>); 31] = [
    ('\u{a0}', Some("nbsp")),
    ('\u{202f}', None),
    ('\u{2009}', Some("thinsp")),
    ('\u{2011}', None),
    ('\u{2013}', Some("ndash")),
    ('\u{2014}', Some("mdash")),
    ('\u{2026}', Some("hellip")),
    ('\u{ab}', Some("laquo")),
    ('\u{bb}', Some("raquo")),
    ('\u{201e}', Some("bdquo")),
    ('\u{201c}', Some("ldquo")),
    ('\u{201d}', Some("rdquo")),
    ('\u{2018}', Some("lsquo")),
    ('\u{2019}', Some("rsquo")),
    ('\u{201a}', Some("sbquo")),
    ('\u{2039}', Some("lsaquo")),
    ('\u{203a}', Some("rsaquo")),
    ('\u{2212}', Some("minus")),
    ('\u{d7}', Some("times")),
    ('\u{2032}', Some("prime")),
    ('\u{2033}', Some("Prime")),
    ('\u{a9}', Some("copy")),
    ('\u{ae}', Some("reg")),
    ('\u{2122}', Some("trade")),
    ('\u{2192}', Some("rarr")),
    ('\u{2190}', Some("larr")),
    ('\u{2194}', Some("harr")),
    ('\u{2265}', Some("ge")),
    ('\u{2264}', Some("le")),
    ('\u{2260}', Some("ne")),
    ('\u{b1}', Some("plusmn")),
];

/// The entry of a character of the typographic set: `Some(name)` where HTML has a name.
fn typographic(c: char) -> Option<Option<&'static str>> {
    if c < '\u{a0}' {
        return None;
    }
    TYPOGRAPHIC
        .iter()
        .find(|(t, _)| *t == c)
        .map(|(_, name)| *name)
}

/// The lead bytes of the UTF-8 forms of the typographic set, derived from its table (0xC2, 0xC3
/// and 0xE2), so the prefilter and the encoder cannot drift apart (design section 3.2).
const SET_BYTES: Triggers = {
    let mut set = Triggers::NONE;
    let mut rest: &[(char, Option<&str>)] = &TYPOGRAPHIC;
    while let [(c, _), tail @ ..] = rest {
        set = set.with_lead(*c);
        rest = tail;
    }
    set
};

/// The control characters of the table of section 11.6: U+0000 to U+001F and U+007F.
const CONTROLS: Triggers = byte_range(0, 0x1f).with(0x7f);

const fn byte_range(first: u8, last: u8) -> Triggers {
    let mut set = Triggers::NONE;
    let mut b = first;
    loop {
        set = set.with(b);
        if b == last {
            return set;
        }
        b += 1;
    }
}

/// The bytes that an output kind encodes (design sections 3.2 and 11.6): a text without any of
/// them comes out of the encoder unchanged. The set may hold more bytes than are encoded (U+0085
/// shares its lead byte with U+00A0); the renderer then compares the result with the input.
pub(crate) const fn encoded_bytes(output: Output) -> Triggers {
    const MARKUP: Triggers = Triggers::of(b"&<>\"'");
    match output {
        Output::Unicode => Triggers::NONE,
        Output::Html(style) => match (style.entities, style.nowrap) {
            (Entities::Characters, Nowrap::Entity) => MARKUP,
            // Inside a nowrap element U+00A0 becomes a space, and U+202F and U+2011 are joints.
            (Entities::Characters, _) => MARKUP.union(Triggers::of(&[0xc2, 0xe2])),
            _ => MARKUP.union(SET_BYTES),
        },
        Output::Escaped(style) => {
            let base = Triggers::of(b"\\\"").union(CONTROLS).union(SET_BYTES);
            let syntax = match style.syntax {
                Syntax::Js | Syntax::JsHex | Syntax::JsCodePoint => Triggers::of(b"'`<>&$"),
                Syntax::Json => Triggers::of(b"<>&"),
                Syntax::Rust => Triggers::NONE,
                Syntax::Css => Triggers::of(b"'<"),
            };
            let scope = match style.scope {
                Scope::Typographic => Triggers::NONE,
                Scope::NonAscii => byte_range(0x80, 0xff),
            };
            base.union(syntax).union(scope)
        }
    }
}

/// The nowrap rendering of an output kind, if it has one.
pub(crate) const fn nowrap_of(output: Output) -> Option<Nowrap> {
    match output {
        Output::Html(HtmlStyle {
            nowrap: nowrap @ (Nowrap::Class(_) | Nowrap::Style),
            ..
        }) => Some(nowrap),
        _ => None,
    }
}

/// The end of an encoded write, asked for by the sink (a comparison that found a difference) or
/// caused by it (a formatter error).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Stop;

/// The result of a write into a sink.
pub(crate) type Res = Result<(), Stop>;

/// Where encoded text goes: a string, a formatter, a counter or a comparison.
pub(crate) trait Out {
    fn put(&mut self, s: &str) -> Res;

    fn put_char(&mut self, c: char) -> Res {
        let mut buf = [0u8; 4];
        self.put(c.encode_utf8(&mut buf))
    }
}

impl Out for String {
    fn put(&mut self, s: &str) -> Res {
        self.push_str(s);
        Ok(())
    }

    fn put_char(&mut self, c: char) -> Res {
        self.push(c);
        Ok(())
    }
}

/// A formatter as a sink.
pub(crate) struct Fmt<'a, 'b>(pub(crate) &'a mut fmt::Formatter<'b>);

impl Out for Fmt<'_, '_> {
    fn put(&mut self, s: &str) -> Res {
        self.0.write_str(s).map_err(|_| Stop)
    }
}

/// Counts the bytes of the output, so that a string can be allocated once at its length.
#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct Count(pub(crate) usize);

impl Out for Count {
    fn put(&mut self, s: &str) -> Res {
        self.0 = self.0.saturating_add(s.len());
        Ok(())
    }
}

/// Compares the output with the concatenation of `parts` and stops at the first difference.
pub(crate) struct Same<'a, I> {
    parts: I,
    current: &'a [u8],
}

impl<'a, I: Iterator<Item = &'a str>> Same<'a, I> {
    pub(crate) fn new(parts: I) -> Same<'a, I> {
        Same {
            parts,
            current: &[],
        }
    }

    /// Whether the whole of `parts` was matched.
    pub(crate) fn finished(mut self) -> bool {
        self.current.is_empty() && self.parts.all(str::is_empty)
    }
}

impl<'a, I: Iterator<Item = &'a str>> Out for Same<'a, I> {
    fn put(&mut self, s: &str) -> Res {
        let mut rest = s.as_bytes();
        while !rest.is_empty() {
            while self.current.is_empty() {
                self.current = self.parts.next().ok_or(Stop)?.as_bytes();
            }
            let n = rest.len().min(self.current.len());
            let (Some(a), Some(b)) = (rest.get(..n), self.current.get(..n)) else {
                return Err(Stop);
            };
            if a != b {
                return Err(Stop);
            }
            rest = rest.get(n..).unwrap_or_default();
            self.current = self.current.get(n..).unwrap_or_default();
        }
        Ok(())
    }
}

/// The closing tag of a nowrap element.
pub(crate) const CLOSE: &str = "</span>";

/// Writes the opening tag of a nowrap element, the class escaped as an attribute value.
pub(crate) fn open<O: Out>(o: &mut O, nowrap: Nowrap) -> Res {
    match nowrap {
        Nowrap::Entity => Ok(()),
        Nowrap::Style => o.put("<span style=\"white-space:nowrap\">"),
        Nowrap::Class(class) => {
            o.put("<span class=\"")?;
            for c in class.chars() {
                match markup_reference(c) {
                    Some(r) => o.put(r)?,
                    None => o.put_char(c)?,
                }
            }
            o.put("\">")
        }
    }
}

/// The reference of a character that HTML text must escape (design section 11.4).
const fn markup_reference(c: char) -> Option<&'static str> {
    match c {
        '&' => Some("&amp;"),
        '<' => Some("&lt;"),
        '>' => Some("&gt;"),
        '"' => Some("&quot;"),
        '\'' => Some("&#39;"),
        _ => None,
    }
}

/// The encoder of one output kind: one character at a time, with the one character of lookahead
/// that `$` before `{` needs in JavaScript.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Encoder {
    output: Output,
    /// A `$` held back until the next character shows whether it starts `${`; at the end of a
    /// text it is escaped.
    dollar: bool,
}

impl Encoder {
    pub(crate) const fn new(output: Output) -> Encoder {
        Encoder {
            output,
            dollar: false,
        }
    }

    /// Writes one character of text or of a protected span. `in_group` is set inside a nowrap
    /// element, where U+00A0 glue is an ordinary space (design section 11.5).
    pub(crate) fn char<O: Out>(&mut self, o: &mut O, c: char, in_group: bool) -> Res {
        match self.output {
            Output::Unicode => o.put_char(c),
            Output::Html(style) => html_char(o, style.entities, c, in_group),
            Output::Escaped(style) => {
                if matches!(
                    style.syntax,
                    Syntax::Js | Syntax::JsHex | Syntax::JsCodePoint
                ) {
                    if self.dollar {
                        self.dollar = false;
                        if c == '{' {
                            code(o, style.syntax, u32::from('$'))?;
                        } else {
                            o.put_char('$')?;
                        }
                    }
                    if c == '$' {
                        self.dollar = true;
                        return Ok(());
                    }
                }
                escaped_char(o, style, c)
            }
        }
    }

    /// Writes a `Markup` piece: as it is in Unicode and HTML output, escaped like text in escaped
    /// output, where it is part of the string (design section 11.1).
    pub(crate) fn markup<O: Out>(&mut self, o: &mut O, s: &str) -> Res {
        match self.output {
            Output::Escaped(_) => s.chars().try_for_each(|c| self.char(o, c, false)),
            Output::Unicode | Output::Html(_) => o.put(s),
        }
    }

    /// Writes a held-back `$`: at the end of a text and before a slot the next character is
    /// unknown, so in JavaScript it is escaped (design section 11.6).
    pub(crate) fn flush<O: Out>(&mut self, o: &mut O) -> Res {
        if self.dollar {
            self.dollar = false;
            match self.output {
                Output::Escaped(style)
                    if matches!(
                        style.syntax,
                        Syntax::Js | Syntax::JsHex | Syntax::JsCodePoint
                    ) =>
                {
                    code(o, style.syntax, u32::from('$'))?;
                }
                _ => o.put_char('$')?,
            }
        }
        Ok(())
    }
}

fn html_char<O: Out>(o: &mut O, entities: Entities, c: char, in_group: bool) -> Res {
    if let Some(r) = markup_reference(c) {
        return o.put(r);
    }
    if c == '\u{a0}' && in_group {
        return o.put_char(' ');
    }
    match typographic(c) {
        None => o.put_char(c),
        Some(name) => match (entities, name) {
            (Entities::Characters, _) => o.put_char(c),
            (Entities::Named, Some(name)) => {
                o.put("&")?;
                o.put(name)?;
                o.put(";")
            }
            (Entities::Named | Entities::Hex, _) => {
                o.put("&#x")?;
                digits(o, u32::from(c), 16, 1, true)?;
                o.put(";")
            }
            (Entities::Decimal, _) => {
                o.put("&#")?;
                digits(o, u32::from(c), 10, 1, true)?;
                o.put(";")
            }
        },
    }
}

/// One character in an escaped string body (the table of design section 11.6).
fn escaped_char<O: Out>(o: &mut O, style: EscapeStyle, c: char) -> Res {
    let syntax = style.syntax;
    let js = matches!(syntax, Syntax::Js | Syntax::JsHex | Syntax::JsCodePoint);
    let cp = u32::from(c);
    match c {
        '\\' => o.put("\\\\"),
        '"' => o.put("\\\""),
        '\'' if syntax == Syntax::Css => o.put("\\'"),
        '\'' | '`' if js => code(o, syntax, cp),
        '<' | '>' | '&' if js || syntax == Syntax::Json => code(o, syntax, cp),
        '<' if syntax == Syntax::Css => code(o, syntax, cp),
        '\n' | '\r' | '\t' if syntax != Syntax::Css => o.put(match c {
            '\n' => "\\n",
            '\r' => "\\r",
            _ => "\\t",
        }),
        // CSS cannot carry U+0000: a reader takes both a raw one and `\0 ` as U+FFFD.
        '\0' if syntax == Syntax::Css => o.put("\\fffd "),
        '\0' if syntax == Syntax::Rust => o.put("\\0"),
        '\0'..='\u{1f}' | '\u{7f}' => code(o, syntax, cp),
        '\u{2028}' | '\u{2029}' if js || syntax == Syntax::Json => code(o, syntax, cp),
        _ if (style.scope == Scope::NonAscii && !c.is_ascii()) || typographic(c).is_some() => {
            code(o, syntax, cp)
        }
        _ => o.put_char(c),
    }
}

/// The escape of a code point: `\u00A0`, `\xA0`, `\u{A0}`, `\u00a0`, `\u{a0}` or `\a0 `.
fn code<O: Out>(o: &mut O, syntax: Syntax, cp: u32) -> Res {
    match syntax {
        Syntax::Js => utf16(o, cp, true),
        Syntax::JsHex if cp <= 0xff => {
            o.put("\\x")?;
            digits(o, cp, 16, 2, true)
        }
        Syntax::JsHex => utf16(o, cp, true),
        Syntax::JsCodePoint => {
            o.put("\\u{")?;
            digits(o, cp, 16, 1, true)?;
            o.put("}")
        }
        Syntax::Json => utf16(o, cp, false),
        Syntax::Rust => {
            o.put("\\u{")?;
            digits(o, cp, 16, 1, false)?;
            o.put("}")
        }
        Syntax::Css => {
            o.put("\\")?;
            digits(o, cp, 16, 1, false)?;
            o.put(" ")
        }
    }
}

/// `\uXXXX` per UTF-16 unit: one, or a surrogate pair above the BMP.
fn utf16<O: Out>(o: &mut O, cp: u32, upper: bool) -> Res {
    let mut units = [0u16; 2];
    let c = char::from_u32(cp).unwrap_or('\u{fffd}');
    for unit in c.encode_utf16(&mut units) {
        o.put("\\u")?;
        digits(o, u32::from(*unit), 16, 4, upper)?;
    }
    Ok(())
}

/// Writes `value` in base 10 or 16 with at least `width` digits, without allocation.
fn digits<O: Out>(o: &mut O, value: u32, base: u32, width: usize, upper: bool) -> Res {
    let table: &[u8; 16] = if upper {
        b"0123456789ABCDEF"
    } else {
        b"0123456789abcdef"
    };
    let mut buf = [b'0'; 10];
    let mut start = buf.len();
    let mut v = value;
    while start > 0 && (v > 0 || buf.len() - start < width.max(1)) {
        start -= 1;
        if let (Some(slot), Some(digit)) = (buf.get_mut(start), table.get((v % base) as usize)) {
            *slot = *digit;
        }
        v /= base;
    }
    match buf.get(start..).map(core::str::from_utf8) {
        Some(Ok(s)) => o.put(s),
        _ => Err(Stop),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lang::data::QUOTE_GLYPHS;

    #[test]
    fn modifiers_keep_the_other_field() {
        let html = HtmlStyle::HEX.nowrap_class("n");
        assert_eq!(html.entities, Entities::Hex);
        assert_eq!(html.nowrap, Nowrap::Class("n"));
        assert_eq!(HtmlStyle::DECIMAL.nowrap_style().nowrap, Nowrap::Style);
        let esc = EscapeStyle::CSS.non_ascii();
        assert_eq!(esc.syntax, Syntax::Css);
        assert_eq!(esc.scope, Scope::NonAscii);
        assert_eq!(EscapeStyle::JSON.scope, Scope::Typographic);
    }

    #[test]
    fn the_quotes_row_holds_every_glyph_a_language_may_use() {
        // Design sections 11.2 and 13.9: a quote pair set by a caller is encoded like a built-in
        // one, and every `Quote` character of the lexer but `"` and `'` is in the set.
        for c in QUOTE_GLYPHS {
            assert!(typographic(c).is_some(), "{c:?}");
        }
        for c in ['«', '»', '„', '“', '”', '‘', '’', '‚', '‹', '›', '′', '″']
        {
            assert!(crate::engine::lex::is_quote_char(c));
            assert!(typographic(c).is_some(), "{c:?}");
        }
    }

    #[test]
    fn every_byte_of_the_set_is_encoded() {
        assert_eq!(SET_BYTES, Triggers::of(&[0xc2, 0xc3, 0xe2]));
        // The prefilter must not skip a text whose characters the encoder changes.
        for (c, _) in TYPOGRAPHIC {
            let mut buf = [0u8; 4];
            let lead = c.encode_utf8(&mut buf).as_bytes()[0];
            assert!(SET_BYTES.contains(lead), "{c:?}");
        }
    }

    #[test]
    fn digits_are_written_with_their_width() {
        let mut s = String::new();
        digits(&mut s, 0, 16, 1, true).unwrap();
        digits(&mut s, 0xa0, 16, 4, true).unwrap();
        digits(&mut s, 0x1f600, 16, 1, false).unwrap();
        digits(&mut s, u32::MAX, 10, 1, true).unwrap();
        assert_eq!(s, "000A01f6004294967295");
    }

    #[test]
    fn same_compares_across_parts() {
        let mut same = Same::new(["ab", "", "c"].into_iter());
        assert!(same.put("a").is_ok() && same.put("bc").is_ok());
        assert!(same.finished());
        let mut same = Same::new(["ab"].into_iter());
        assert!(same.put("ax").is_err());
        let mut same = Same::new(["ab"].into_iter());
        assert!(same.put("a").is_ok());
        assert!(!same.finished());
    }
}
