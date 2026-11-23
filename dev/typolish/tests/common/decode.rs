//! Decoders, strict parsers and a reference encoder for the output kinds (design sections 11.8
//! and 13.5).
//!
//! The decoders are the inverse of the encoders of design section 11 and read their input strictly:
//! an escape that the syntax does not define, a raw character that the kind must not write, a
//! reference that is not in the table of design section 11.2 and an element that is not a nowrap
//! element are errors, so the same functions serve the decoder properties (property 12) and the
//! safe alphabet property (property 17). The reference encoder is written from the tables of design
//! sections 11.2, 11.4 and 11.6 alone and does not group nowrap elements; it checks the fixed
//! tables of `tests/output.rs` against the Unicode result of the typographer.
//!
//! The module is declared by `tests/common/mod.rs`.

#![allow(dead_code)]

use typolish::{EscapeStyle, HtmlStyle, Output};

/// The typographic set of design section 11.2, every character the rules produce.
pub const TYPOGRAPHIC: &[char] = &[
    '\u{a0}', '\u{202f}', '\u{2009}', '\u{2011}', '–', '—', '…', '«', '»', '„', '“', '”', '‘', '’',
    '‚', '‹', '›', '−', '×', '′', '″', '©', '®', '™', '→', '←', '↔', '≥', '≤', '≠', '±',
];

/// The named references of design section 11.2 as `(name, character)`. U+202F and U+2011 have no
/// name.
pub const NAMES: &[(&str, char)] = &[
    ("nbsp", '\u{a0}'),
    ("thinsp", '\u{2009}'),
    ("ndash", '–'),
    ("mdash", '—'),
    ("hellip", '…'),
    ("laquo", '«'),
    ("raquo", '»'),
    ("bdquo", '„'),
    ("ldquo", '“'),
    ("rdquo", '”'),
    ("lsquo", '‘'),
    ("rsquo", '’'),
    ("sbquo", '‚'),
    ("lsaquo", '‹'),
    ("rsaquo", '›'),
    ("minus", '−'),
    ("times", '×'),
    ("prime", '′'),
    ("Prime", '″'),
    ("copy", '©'),
    ("reg", '®'),
    ("trade", '™'),
    ("rarr", '→'),
    ("larr", '←'),
    ("harr", '↔'),
    ("ge", '≥'),
    ("le", '≤'),
    ("ne", '≠'),
    ("plusmn", '±'),
];

/// The references that escape markup in HTML text (design section 11.4).
const MARKUP_NAMES: &[(&str, char)] = &[("amp", '&'), ("lt", '<'), ("gt", '>'), ("quot", '"')];

/// The name of a character of the typographic set, if HTML has one.
pub fn name_of(c: char) -> Option<&'static str> {
    NAMES.iter().find(|(_, ch)| *ch == c).map(|(n, _)| *n)
}

/// Every output kind of the stage 3 properties, nowrap styles included.
pub fn all_outputs() -> Vec<Output> {
    vec![
        Output::Unicode,
        Output::Html(HtmlStyle::NAMED),
        Output::Html(HtmlStyle::DECIMAL),
        Output::Html(HtmlStyle::HEX),
        Output::Html(HtmlStyle::CHARACTERS),
        Output::Html(HtmlStyle::NAMED.nowrap_class("nowrap")),
        Output::Html(HtmlStyle::HEX.nowrap_style()),
        Output::Html(HtmlStyle::CHARACTERS.nowrap_class("nw")),
        Output::Escaped(EscapeStyle::JS),
        Output::Escaped(EscapeStyle::JS_HEX),
        Output::Escaped(EscapeStyle::JS_CODE_POINT),
        Output::Escaped(EscapeStyle::JSON),
        Output::Escaped(EscapeStyle::RUST),
        Output::Escaped(EscapeStyle::CSS),
        Output::Escaped(EscapeStyle::JS.non_ascii()),
        Output::Escaped(EscapeStyle::JS_HEX.non_ascii()),
        Output::Escaped(EscapeStyle::JS_CODE_POINT.non_ascii()),
        Output::Escaped(EscapeStyle::JSON.non_ascii()),
        Output::Escaped(EscapeStyle::RUST.non_ascii()),
        Output::Escaped(EscapeStyle::CSS.non_ascii()),
    ]
}

/// The string syntaxes of `EscapeStyle`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Syntax {
    /// ` `.
    Js,
    /// `\xA0`.
    JsHex,
    /// `\u{A0}`.
    JsCodePoint,
    /// ` `.
    Json,
    /// `\u{a0}`.
    Rust,
    /// `\a0 `.
    Css,
}

/// The syntax of a style and whether its scope is every non-ASCII character.
pub fn syntax_of(style: EscapeStyle) -> (Syntax, bool) {
    for (syntax, base) in [
        (Syntax::Js, EscapeStyle::JS),
        (Syntax::JsHex, EscapeStyle::JS_HEX),
        (Syntax::JsCodePoint, EscapeStyle::JS_CODE_POINT),
        (Syntax::Json, EscapeStyle::JSON),
        (Syntax::Rust, EscapeStyle::RUST),
        (Syntax::Css, EscapeStyle::CSS),
    ] {
        if style == base {
            return (syntax, false);
        }
        if style == base.non_ascii() {
            return (syntax, true);
        }
    }
    panic!("an escape style that is none of the built-in ones: {style:?}")
}

/// The raw characters a syntax never writes (design section 13.5, property 17): `c` followed by
/// `next`.
pub fn raw_forbidden(syntax: Syntax, c: char, next: Option<char>) -> bool {
    let control = c <= '\u{1f}' || c == '\u{7f}';
    match syntax {
        Syntax::Js | Syntax::JsHex | Syntax::JsCodePoint => {
            matches!(
                c,
                '"' | '\'' | '`' | '<' | '>' | '&' | '\u{2028}' | '\u{2029}'
            ) || (c == '$' && matches!(next, Some('{') | None))
                || control
        }
        Syntax::Json => matches!(c, '"' | '<' | '>' | '&' | '\u{2028}' | '\u{2029}') || control,
        Syntax::Rust => c == '"' || control,
        Syntax::Css => matches!(c, '"' | '\'' | '<') || control,
    }
}

/// Decodes the body of a string literal, reading it as strictly as its syntax does: every `\`
/// must start an escape that the syntax defines (the CSS hex escape with its space), and no raw
/// character of `raw_forbidden` may appear.
pub fn decode_escaped(style: EscapeStyle, s: &str) -> Result<String, String> {
    let (syntax, non_ascii) = syntax_of(style);
    let chars: Vec<char> = s.chars().collect();
    let mut out = String::new();
    let mut i = 0;
    while i < chars.len() {
        let c = chars[i];
        if c != '\\' {
            if raw_forbidden(syntax, c, chars.get(i + 1).copied()) {
                return Err(format!("raw {c:?} at char {i} of {s:?}"));
            }
            if non_ascii && !c.is_ascii() {
                return Err(format!("raw non-ASCII {c:?} at char {i} of {s:?}"));
            }
            out.push(c);
            i += 1;
            continue;
        }
        let Some(&e) = chars.get(i + 1) else {
            return Err(format!("a backslash ends {s:?}"));
        };
        i += 2;
        match (syntax, e) {
            (_, '\\' | '"') => out.push(e),
            (Syntax::Css, '\'') | (Syntax::Rust, '\'') => out.push(e),
            (Syntax::Json, '/') => out.push('/'),
            (Syntax::Json, 'b') => out.push('\u{8}'),
            (Syntax::Json, 'f') => out.push('\u{c}'),
            (Syntax::Css, _) if e.is_ascii_hexdigit() => {
                let mut digits = String::from(e);
                while digits.len() < 6 && chars.get(i).is_some_and(char::is_ascii_hexdigit) {
                    digits.push(chars[i]);
                    i += 1;
                }
                // The escape always ends with its space, which is part of it.
                if chars.get(i) != Some(&' ') {
                    return Err(format!("a CSS hex escape without its space in {s:?}"));
                }
                i += 1;
                let cp = u32::from_str_radix(&digits, 16).map_err(|e| e.to_string())?;
                out.push(match cp {
                    0 => '\u{fffd}',
                    _ => char::from_u32(cp).unwrap_or('\u{fffd}'),
                });
            }
            (Syntax::Css, _) => return Err(format!("CSS escape `\\{e}` in {s:?}")),
            (
                Syntax::Js | Syntax::JsHex | Syntax::JsCodePoint | Syntax::Json | Syntax::Rust,
                'n',
            ) => out.push('\n'),
            (
                Syntax::Js | Syntax::JsHex | Syntax::JsCodePoint | Syntax::Json | Syntax::Rust,
                'r',
            ) => out.push('\r'),
            (
                Syntax::Js | Syntax::JsHex | Syntax::JsCodePoint | Syntax::Json | Syntax::Rust,
                't',
            ) => out.push('\t'),
            (Syntax::Rust, '0') => out.push('\0'),
            (Syntax::JsHex, 'x') | (Syntax::Rust, 'x') => {
                let hex: String = chars.iter().skip(i).take(2).collect();
                if hex.len() != 2 || !hex.chars().all(|c| c.is_ascii_hexdigit()) {
                    return Err(format!("bad \\x escape in {s:?}"));
                }
                i += 2;
                let cp = u32::from_str_radix(&hex, 16).map_err(|e| e.to_string())?;
                if syntax == Syntax::Rust && cp > 0x7f {
                    return Err(format!("Rust \\x escape above 7F in {s:?}"));
                }
                out.push(char::from_u32(cp).ok_or("bad \\x")?);
            }
            (Syntax::JsCodePoint | Syntax::Rust, 'u') => {
                if chars.get(i) != Some(&'{') {
                    return Err(format!("\\u without a brace in {s:?}"));
                }
                let close = chars[i..]
                    .iter()
                    .position(|c| *c == '}')
                    .ok_or_else(|| format!("unclosed \\u{{ in {s:?}"))?;
                let hex: String = chars[i + 1..i + close].iter().collect();
                if hex.is_empty() || hex.len() > 6 || !hex.chars().all(|c| c.is_ascii_hexdigit()) {
                    return Err(format!("bad \\u{{{hex}}} in {s:?}"));
                }
                let cp = u32::from_str_radix(&hex, 16).map_err(|e| e.to_string())?;
                out.push(char::from_u32(cp).ok_or_else(|| format!("not a scalar value: {hex}"))?);
                i += close + 1;
            }
            (Syntax::Js | Syntax::JsHex | Syntax::Json, 'u') => {
                let unit = |from: usize| -> Result<u32, String> {
                    let hex: String = chars.iter().skip(from).take(4).collect();
                    if hex.len() != 4 || !hex.chars().all(|c| c.is_ascii_hexdigit()) {
                        return Err(format!("bad \\u escape in {s:?}"));
                    }
                    u32::from_str_radix(&hex, 16).map_err(|e| e.to_string())
                };
                let first = unit(i)?;
                i += 4;
                if (0xd800..0xdc00).contains(&first) {
                    if chars.get(i) != Some(&'\\') || chars.get(i + 1) != Some(&'u') {
                        return Err(format!("a lone high surrogate in {s:?}"));
                    }
                    let second = unit(i + 2)?;
                    if !(0xdc00..0xe000).contains(&second) {
                        return Err(format!("a high surrogate without a low one in {s:?}"));
                    }
                    i += 6;
                    let cp = 0x10000 + ((first - 0xd800) << 10) + (second - 0xdc00);
                    out.push(char::from_u32(cp).ok_or("bad pair")?);
                } else if (0xdc00..0xe000).contains(&first) {
                    return Err(format!("a lone low surrogate in {s:?}"));
                } else {
                    out.push(char::from_u32(first).ok_or("bad unit")?);
                }
            }
            _ => {
                return Err(format!(
                    "escape `\\{e}` is not defined by {syntax:?} in {s:?}"
                ));
            }
        }
    }
    Ok(out)
}

fn reference(name: &str) -> Result<char, String> {
    if let Some(hex) = name.strip_prefix("#x").or_else(|| name.strip_prefix("#X")) {
        let cp = u32::from_str_radix(hex, 16).map_err(|e| format!("&{name}; {e}"))?;
        return char::from_u32(cp).ok_or_else(|| format!("&{name}; is not a scalar value"));
    }
    if let Some(dec) = name.strip_prefix('#') {
        let cp: u32 = dec.parse().map_err(|e| format!("&{name}; {e}"))?;
        return char::from_u32(cp).ok_or_else(|| format!("&{name}; is not a scalar value"));
    }
    NAMES
        .iter()
        .chain(MARKUP_NAMES)
        .find(|(n, _)| *n == name)
        .map(|(_, c)| *c)
        .ok_or_else(|| format!("unknown reference &{name};"))
}

/// Decodes HTML text, reading it as strictly as the encoder writes it: no raw `<`, `>`, `"` or `'`
/// outside the nowrap elements, no `&` that does not start a reference of the table of design
/// section 11.2 or a numeric one, and no element other than `<span class="…">` and
/// `<span style="white-space:nowrap">`, which do not nest. Inside an element an ordinary space is
/// U+00A0 glue (design section 11.5).
pub fn decode_html(s: &str) -> Result<String, String> {
    let mut out = String::new();
    let mut rest = s;
    let mut inside = false;
    while let Some(c) = rest.chars().next() {
        match c {
            '<' => {
                if let Some(after) = rest.strip_prefix("</span>") {
                    if !inside {
                        return Err(format!("a closing tag outside an element in {s:?}"));
                    }
                    inside = false;
                    rest = after;
                } else if let Some(after) = rest.strip_prefix("<span style=\"white-space:nowrap\">")
                {
                    if inside {
                        return Err(format!("nested elements in {s:?}"));
                    }
                    inside = true;
                    rest = after;
                } else if let Some(after) = rest.strip_prefix("<span class=\"") {
                    if inside {
                        return Err(format!("nested elements in {s:?}"));
                    }
                    let end = after
                        .find('"')
                        .ok_or_else(|| format!("an unclosed class attribute in {s:?}"))?;
                    // The class is an attribute value: escaped, with no raw markup character.
                    decode_html(&after[..end])?;
                    let Some(tail) = after[end..].strip_prefix("\">") else {
                        return Err(format!("a malformed span tag in {s:?}"));
                    };
                    inside = true;
                    rest = tail;
                } else {
                    return Err(format!("a raw `<` in {s:?}"));
                }
            }
            '>' | '"' | '\'' => return Err(format!("a raw {c:?} in {s:?}")),
            '&' => {
                let end = rest
                    .find(';')
                    .ok_or_else(|| format!("an `&` that starts no reference in {s:?}"))?;
                let name = &rest[1..end];
                if name.is_empty() || !name.chars().all(|c| c.is_ascii_alphanumeric() || c == '#') {
                    return Err(format!("an `&` that starts no reference in {s:?}"));
                }
                out.push(reference(name)?);
                rest = &rest[end + 1..];
            }
            ' ' if inside => {
                out.push('\u{a0}');
                rest = &rest[1..];
            }
            _ => {
                out.push(c);
                rest = &rest[c.len_utf8()..];
            }
        }
    }
    if inside {
        return Err(format!("an unclosed element in {s:?}"));
    }
    Ok(out)
}

/// Decodes the output of a kind; `Unicode` is the identity.
pub fn decode(output: Output, s: &str) -> Result<String, String> {
    match output {
        Output::Unicode => Ok(s.to_owned()),
        Output::Html(_) => decode_html(s),
        Output::Escaped(style) => decode_escaped(style, s),
        _ => Err("an output kind of a newer version".to_owned()),
    }
}

/// The checks of property 17 beyond decoding: the string decodes strictly, with `Html` and a style
/// other than the `CHARACTERS` ones none of the typographic set is raw, and with `non_ascii()` no
/// character from U+0080 is raw (`decode_escaped` checks that).
pub fn check_safe_alphabet(output: Output, s: &str) -> Result<(), String> {
    match output {
        Output::Html(style) => {
            decode_html(s)?;
            if !writes_characters(style)
                && let Some(c) = s.chars().find(|c| TYPOGRAPHIC.contains(c))
            {
                return Err(format!("raw typographic {c:?} in {s:?}"));
            }
            Ok(())
        }
        Output::Escaped(style) => decode_escaped(style, s).map(|_| ()),
        _ => Ok(()),
    }
}

/// Whether a style writes the typographic set as characters. The nowrap class is private to the
/// style, so the classes of `all_outputs` are the ones recognized.
pub fn writes_characters(style: HtmlStyle) -> bool {
    style == HtmlStyle::CHARACTERS
        || style == HtmlStyle::CHARACTERS.nowrap_style()
        || style == HtmlStyle::CHARACTERS.nowrap_class("nowrap")
        || style == HtmlStyle::CHARACTERS.nowrap_class("nw")
}

fn utf16(c: char) -> Vec<u16> {
    let mut buf = [0u16; 2];
    c.encode_utf16(&mut buf).to_vec()
}

/// The reference encoder of design sections 11.4 and 11.6: the Unicode result written in the kind,
/// for every kind except the nowrap styles of HTML.
pub fn encode_reference(output: Output, unicode: &str) -> String {
    match output {
        Output::Unicode => unicode.to_owned(),
        Output::Html(style) => encode_html(style, unicode),
        Output::Escaped(style) => encode_escaped(style, unicode),
        _ => panic!("an output kind of a newer version"),
    }
}

fn encode_html(style: HtmlStyle, text: &str) -> String {
    assert!(
        style == HtmlStyle::NAMED
            || style == HtmlStyle::DECIMAL
            || style == HtmlStyle::HEX
            || style == HtmlStyle::CHARACTERS,
        "the reference encoder does not write nowrap elements"
    );
    let mut out = String::new();
    for c in text.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&#39;"),
            c if TYPOGRAPHIC.contains(&c) && style != HtmlStyle::CHARACTERS => {
                if style == HtmlStyle::NAMED {
                    match name_of(c) {
                        Some(name) => out.push_str(&format!("&{name};")),
                        None => out.push_str(&format!("&#x{:X};", u32::from(c))),
                    }
                } else if style == HtmlStyle::DECIMAL {
                    out.push_str(&format!("&#{};", u32::from(c)));
                } else {
                    out.push_str(&format!("&#x{:X};", u32::from(c)));
                }
            }
            c => out.push(c),
        }
    }
    out
}

fn encode_escaped(style: EscapeStyle, text: &str) -> String {
    let (syntax, non_ascii) = syntax_of(style);
    let mut out = String::new();
    let mut chars = text.chars().peekable();
    while let Some(c) = chars.next() {
        let next = chars.peek().copied();
        let cp = u32::from(c);
        let upper = |n: u32| format!("{n:04X}");
        match c {
            '\\' => out.push_str("\\\\"),
            '"' => out.push_str("\\\""),
            '\'' | '`' => match syntax {
                Syntax::Js => out.push_str(&format!("\\u{}", upper(cp))),
                Syntax::JsHex => out.push_str(&format!("\\x{cp:02X}")),
                Syntax::JsCodePoint => out.push_str(&format!("\\u{{{cp:X}}}")),
                Syntax::Css if c == '\'' => out.push_str("\\'"),
                _ => out.push(c),
            },
            '<' | '>' | '&' => match syntax {
                Syntax::Js => out.push_str(&format!("\\u{}", upper(cp))),
                Syntax::JsHex => out.push_str(&format!("\\x{cp:02X}")),
                Syntax::JsCodePoint => out.push_str(&format!("\\u{{{cp:X}}}")),
                Syntax::Json => out.push_str(&format!("\\u{cp:04x}")),
                Syntax::Css if c == '<' => out.push_str(&format!("\\{cp:x} ")),
                _ => out.push(c),
            },
            '$' if matches!(next, Some('{') | None) => match syntax {
                Syntax::Js => out.push_str("\\u0024"),
                Syntax::JsHex => out.push_str("\\x24"),
                Syntax::JsCodePoint => out.push_str("\\u{24}"),
                _ => out.push(c),
            },
            '\n' | '\r' | '\t' if syntax != Syntax::Css => out.push_str(match c {
                '\n' => "\\n",
                '\r' => "\\r",
                _ => "\\t",
            }),
            '\0' => match syntax {
                Syntax::Js | Syntax::Json => out.push_str("\\u0000"),
                Syntax::JsHex => out.push_str("\\x00"),
                Syntax::JsCodePoint => out.push_str("\\u{0}"),
                Syntax::Rust => out.push_str("\\0"),
                Syntax::Css => out.push_str("\\fffd "),
            },
            c if c <= '\u{1f}' || c == '\u{7f}' => match syntax {
                Syntax::Js => out.push_str(&format!("\\u{}", upper(cp))),
                Syntax::JsHex => out.push_str(&format!("\\x{cp:02X}")),
                Syntax::JsCodePoint => out.push_str(&format!("\\u{{{cp:X}}}")),
                Syntax::Json => out.push_str(&format!("\\u{cp:04x}")),
                Syntax::Rust => out.push_str(&format!("\\u{{{cp:x}}}")),
                Syntax::Css => out.push_str(&format!("\\{cp:x} ")),
            },
            '\u{2028}' | '\u{2029}'
                if matches!(
                    syntax,
                    Syntax::Js | Syntax::JsHex | Syntax::JsCodePoint | Syntax::Json
                ) =>
            {
                match syntax {
                    Syntax::JsCodePoint => out.push_str(&format!("\\u{{{cp:X}}}")),
                    Syntax::Json => out.push_str(&format!("\\u{cp:04x}")),
                    _ => out.push_str(&format!("\\u{cp:04X}")),
                }
            }
            c if TYPOGRAPHIC.contains(&c) || (non_ascii && !c.is_ascii()) => match syntax {
                Syntax::Js => {
                    for unit in utf16(c) {
                        out.push_str(&format!("\\u{unit:04X}"));
                    }
                }
                Syntax::JsHex => {
                    if cp <= 0xff {
                        out.push_str(&format!("\\x{cp:02X}"));
                    } else {
                        for unit in utf16(c) {
                            out.push_str(&format!("\\u{unit:04X}"));
                        }
                    }
                }
                Syntax::JsCodePoint => out.push_str(&format!("\\u{{{cp:X}}}")),
                Syntax::Json => {
                    for unit in utf16(c) {
                        out.push_str(&format!("\\u{unit:04x}"));
                    }
                }
                Syntax::Rust => out.push_str(&format!("\\u{{{cp:x}}}")),
                Syntax::Css => out.push_str(&format!("\\{cp:x} ")),
            },
            c => out.push(c),
        }
    }
    out
}
