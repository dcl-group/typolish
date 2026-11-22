//! Output kinds (design sections 2.8 and 11).
//!
//! Rules work in Unicode and never see the output kind; the encoder writes the result in the
//! requested kind while rendering. The encoders of `Output::Html` and `Output::Escaped` arrive in
//! stage 3 (design section 15.4); until then every kind is written as Unicode.

/// How the result is written. Rules never depend on it (design section 11).
///
/// ```
/// use typolish::{EscapeStyle, HtmlStyle, Output};
///
/// let kinds = [
///     Output::Unicode,
///     Output::Html(HtmlStyle::NAMED.nowrap_class("nowrap")),
///     Output::Escaped(EscapeStyle::JS.non_ascii()),
/// ];
/// assert_eq!(kinds.len(), 3);
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum Output {
    /// Characters as they are (UTF-8). Nothing is escaped.
    Unicode,
    /// HTML text: `& < > " '` escaped, typographic characters as entities, no-break groups as
    /// entities or as nowrap elements.
    ///
    /// Not encoded yet: until the encoders of stage 3 this kind is written as Unicode; do not use
    /// it for untrusted text.
    Html(HtmlStyle),
    /// The body of a string literal of a given syntax, ready to paste between its quotes.
    ///
    /// Not encoded yet: until the encoders of stage 3 this kind is written as Unicode; do not use
    /// it for untrusted text.
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

/// Parameters of [`Output::Html`].
///
/// ```
/// use typolish::{HtmlStyle, Output};
///
/// const OUT: Output = Output::Html(HtmlStyle::HEX.nowrap_style());
/// # let _ = OUT;
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct HtmlStyle {
    pub(crate) entities: Entities,
    pub(crate) nowrap: Nowrap,
}

impl HtmlStyle {
    /// Named references (`&nbsp;`, `&mdash;`), numeric where HTML has no name.
    pub const NAMED: HtmlStyle = HtmlStyle {
        entities: Entities::Named,
        nowrap: Nowrap::Entity,
    };
    /// Decimal references (`&#160;`, `&#8212;`).
    pub const DECIMAL: HtmlStyle = HtmlStyle {
        entities: Entities::Decimal,
        nowrap: Nowrap::Entity,
    };
    /// Hexadecimal references (`&#xA0;`, `&#x2014;`).
    pub const HEX: HtmlStyle = HtmlStyle {
        entities: Entities::Hex,
        nowrap: Nowrap::Entity,
    };
    /// Typographic characters as characters; only markup is escaped.
    pub const CHARACTERS: HtmlStyle = HtmlStyle {
        entities: Entities::Characters,
        nowrap: Nowrap::Entity,
    };

    /// Wraps each no-break group in `<span class="{class}">`; U+00A0 glue becomes an ordinary
    /// space inside (design section 11.5).
    #[must_use]
    pub const fn nowrap_class(self, class: &'static str) -> HtmlStyle {
        HtmlStyle {
            entities: self.entities,
            nowrap: Nowrap::Class(class),
        }
    }

    /// The same as [`HtmlStyle::nowrap_class`] with `<span style="white-space:nowrap">`.
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
/// characters, whatever the scope.
///
/// ```
/// use typolish::{EscapeStyle, Output};
///
/// const OUT: Output = Output::Escaped(EscapeStyle::RUST);
/// # let _ = OUT;
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct EscapeStyle {
    pub(crate) syntax: Syntax,
    pub(crate) scope: Scope,
}

impl EscapeStyle {
    /// JavaScript: `\u00A0`.
    pub const JS: EscapeStyle = EscapeStyle::of(Syntax::Js);
    /// JavaScript: `\xA0` up to U+00FF, `\u2014` above.
    pub const JS_HEX: EscapeStyle = EscapeStyle::of(Syntax::JsHex);
    /// JavaScript (ES2015): `\u{A0}`.
    pub const JS_CODE_POINT: EscapeStyle = EscapeStyle::of(Syntax::JsCodePoint);
    /// JSON: `\u00A0`.
    pub const JSON: EscapeStyle = EscapeStyle::of(Syntax::Json);
    /// Rust: `\u{a0}`.
    pub const RUST: EscapeStyle = EscapeStyle::of(Syntax::Rust);
    /// CSS: `\a0` followed by a space.
    pub const CSS: EscapeStyle = EscapeStyle::of(Syntax::Css);

    const fn of(syntax: Syntax) -> EscapeStyle {
        EscapeStyle {
            syntax,
            scope: Scope::Typographic,
        }
    }

    /// Escapes every non-ASCII character, not only the typographic set.
    #[must_use]
    pub const fn non_ascii(self) -> EscapeStyle {
        EscapeStyle {
            syntax: self.syntax,
            scope: Scope::NonAscii,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
}
