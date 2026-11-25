//! Numbers (design section 2.7): [`Number`], [`Typographer::format_number`] and
//! [`Typographer::format_range`], written lazily through `Display` without allocation (design
//! section 2.12) and encoded for the output kind (design section 11.7).

use core::fmt::{self, Write as _};

use crate::lang::data::NumberData;
use crate::output::{self, Encoder, Out, Stop};
use crate::{Error, Output, Typographer};

/// A number to format: an integer, an `f32` or `f64`, or an ASCII decimal string.
///
/// Every integer type and both float types convert with `From`; [`Number::decimal`] takes digits
/// as written and [`Number::fixed`] rounds a float to a number of fraction digits. A float
/// converted with `From` is written with its shortest digits in full, never with an exponent, so
/// `1e30` has thirty-one digits and `0.1 + 0.2` is `0.30000000000000004` in English; use `fixed` or
/// `decimal` to control the digits. Integers compare by value whatever their type
/// (`Number::from(1u8) == Number::from(1i32)`); `1`, `1.0` and `Number::decimal("1")` are three
/// different numbers. Floats compare as floats, so a `NaN` is not equal to itself.
///
/// ```
/// use typolish::{Language, Number, Output, RuleSet, Typographer};
///
/// static RU: Typographer = Typographer::new(Language::RU_RU, RuleSet::PROSE);
///
/// let n = Number::decimal("-1234.50")?;
/// assert_eq!(RU.format_number(n, Output::Unicode).to_string(), "\u{2212}1\u{a0}234,50");
/// assert_eq!(RU.format_number(Number::from(7u8), Output::Unicode).to_string(), "7");
/// # Ok::<(), typolish::Error>(())
/// ```
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Number<'n> {
    repr: Repr<'n>,
}

/// The variants of a number.
#[derive(Clone, Copy, Debug, PartialEq)]
enum Repr<'n> {
    Signed(i128),
    Unsigned(u128),
    F32(f32),
    F64(f64),
    /// A float rounded to a number of fraction digits.
    Fixed(f64, u8),
    /// ASCII digits as the caller wrote them.
    Decimal {
        negative: bool,
        int: &'n str,
        frac: Option<&'n str>,
    },
}

impl Repr<'_> {
    /// A non-negative signed integer becomes the unsigned one, so that equal integers of different
    /// types compare equal (design section 2.13).
    fn normalized(self) -> Self {
        match self {
            Repr::Signed(v) => u128::try_from(v).map_or(self, Repr::Unsigned),
            other => other,
        }
    }
}

impl<'n> Number<'n> {
    /// An ASCII decimal: an optional sign `+` or `-`, digits, an optional `.` and digits.
    ///
    /// A `+` is accepted and not written. Digits are kept as given: leading zeros (`007` stays
    /// `007`) and the caller's precision (`1.50` keeps two fraction digits). A zero value is written
    /// without a sign (`-0.00` is `0,00` in Russian). The empty string, a sign without digits, more
    /// than one sign or a sign that is not first, a point without a digit on each side, more than
    /// one point, an exponent and any other character are [`Error::InvalidNumber`] (design section
    /// 2.11).
    ///
    /// ```
    /// use typolish::{Error, Language, Number, Output, RuleSet, Typographer};
    ///
    /// static EN: Typographer = Typographer::new(Language::EN_US, RuleSet::PROSE);
    ///
    /// let n = Number::decimal("+0012345.50")?;
    /// assert_eq!(EN.format_number(n, Output::Unicode).to_string(), "0,012,345.50");
    /// assert!(matches!(Number::decimal("1e3"), Err(Error::InvalidNumber { .. })));
    /// assert!(Number::decimal(".5").is_err());
    /// # Ok::<(), typolish::Error>(())
    /// ```
    pub fn decimal(text: &'n str) -> Result<Number<'n>, Error> {
        let invalid = |reason: &'static str| Error::InvalidNumber {
            input: text.to_owned(),
            reason,
        };
        if text.is_empty() {
            return Err(invalid("the string is empty"));
        }
        let (negative, body) = match (text.strip_prefix('-'), text.strip_prefix('+')) {
            (Some(rest), _) => (true, rest),
            (None, Some(rest)) => (false, rest),
            (None, None) => (false, text),
        };
        if body.is_empty() {
            return Err(invalid("a sign without digits"));
        }
        for c in body.chars() {
            match c {
                '0'..='9' | '.' => {}
                '+' | '-' => {
                    return Err(invalid("more than one sign, or a sign that is not first"));
                }
                'e' | 'E' => return Err(invalid("an exponent")),
                _ => {
                    return Err(invalid(
                        "a character that is not an ASCII digit, a sign or a point",
                    ));
                }
            }
        }
        let (int, frac) = match body.split_once('.') {
            Some((int, frac)) => (int, Some(frac)),
            None => (body, None),
        };
        if frac.is_some_and(|f| f.contains('.')) {
            return Err(invalid("more than one point"));
        }
        if int.is_empty() || frac == Some("") {
            return Err(invalid("no digit before or after the point"));
        }
        Ok(Number {
            repr: Repr::Decimal {
                negative,
                int,
                frac,
            },
        })
    }

    /// A float rounded to `fraction_digits` digits after the point: round half to even on the
    /// exact binary value, as `format!("{:.N}")` of core does.
    ///
    /// ```
    /// use typolish::{Language, Number, Output, RuleSet, Typographer};
    ///
    /// static RU: Typographer = Typographer::new(Language::RU_RU, RuleSet::PROSE);
    ///
    /// let price = RU.format_number(Number::fixed(1234.5678, 2), Output::Unicode);
    /// assert_eq!(price.to_string(), "1\u{a0}234,57");
    /// // A value that rounds to zero has no sign.
    /// assert_eq!(RU.format_number(Number::fixed(-0.001, 1), Output::Unicode).to_string(), "0,0");
    /// ```
    #[must_use]
    pub const fn fixed(value: f64, fraction_digits: u8) -> Number<'static> {
        Number {
            repr: Repr::Fixed(value, fraction_digits),
        }
    }
}

macro_rules! from_int {
    ($variant:ident, $wide:ty: $($t:ty),*) => {$(
        impl From<$t> for Number<'static> {
            /// The integer as it is.
            fn from(value: $t) -> Number<'static> {
                Number { repr: Repr::$variant(<$wide>::from(value)).normalized() }
            }
        }
    )*};
}

from_int!(Signed, i128: i8, i16, i32, i64, i128);
from_int!(Unsigned, u128: u8, u16, u32, u64, u128);

impl From<isize> for Number<'static> {
    /// The integer as it is.
    fn from(value: isize) -> Number<'static> {
        // Widening: `isize` is never wider than `i128`, so the cast is exact.
        Number {
            repr: Repr::Signed(value as i128).normalized(),
        }
    }
}

impl From<usize> for Number<'static> {
    /// The integer as it is.
    fn from(value: usize) -> Number<'static> {
        // Widening: `usize` is never wider than `u128`, so the cast is exact.
        Number {
            repr: Repr::Unsigned(value as u128),
        }
    }
}

impl From<f32> for Number<'static> {
    /// The float with the shortest representation of `f32`: `0.1f32` is `0.1`, not widened.
    fn from(value: f32) -> Number<'static> {
        Number {
            repr: Repr::F32(value),
        }
    }
}

impl From<f64> for Number<'static> {
    /// The float with the shortest digits that read back as the same `f64`, written in full and
    /// never with an exponent: `1e30` is thirty-one grouped digits and `1e-30` is `0.000…001`, and
    /// the sum `0.1 + 0.2` is `0.30000000000000004` in English. For a fixed number of fraction
    /// digits use [`Number::fixed`]; for digits as you have them, [`Number::decimal`].
    fn from(value: f64) -> Number<'static> {
        Number {
            repr: Repr::F64(value),
        }
    }
}

/// A number formatted by a typographer: [`Typographer::format_number`]. Lazy: it writes the number
/// when it is displayed and allocates nothing.
///
/// ```
/// use core::fmt::Write;
/// use typolish::{Language, Output, RuleSet, Typographer};
///
/// static EN: Typographer = Typographer::new(Language::EN_US, RuleSet::PROSE);
///
/// let mut buf = String::with_capacity(32);
/// write!(buf, "{} items", EN.format_number(12345, Output::Unicode))?;
/// assert_eq!(buf, "12,345 items");
/// # Ok::<(), core::fmt::Error>(())
/// ```
///
/// The formatter's width, fill, alignment and precision are ignored; format into a `String` and pad
/// that if needed.
///
/// ```
/// use typolish::{Language, Output, RuleSet, Typographer};
///
/// static RU: Typographer = Typographer::new(Language::RU_RU, RuleSet::PROSE);
///
/// let n = RU.format_number(12345, Output::Unicode);
/// assert_eq!(format!("[{n:>12}]"), "[12\u{a0}345]");
/// assert_eq!(format!("[{n:.1}]"), "[12\u{a0}345]");
/// assert_eq!(format!("[{:>12}]", n.to_string()), "[      12\u{a0}345]");
/// ```
#[must_use = "formats only when displayed"]
#[derive(Clone, Copy, Debug)]
pub struct NumberDisplay<'t, 'n> {
    typographer: &'t Typographer,
    number: Number<'n>,
    output: Output,
}

/// Two numbers with the range pattern of the language: [`Typographer::format_range`]. Lazy, like
/// [`NumberDisplay`], and like it, it ignores the formatter's width, fill, alignment and precision.
///
/// ```
/// use typolish::{Language, Output, RuleSet, Typographer};
///
/// static RU: Typographer = Typographer::new(Language::RU_RU, RuleSet::PROSE);
///
/// assert_eq!(RU.format_range(-5, -1, Output::Unicode).to_string(), "\u{2212}5–\u{2212}1");
/// assert_eq!(format!("[{:>9}]", RU.format_range(1, 2, Output::Unicode)), "[1–2]");
/// ```
#[must_use = "formats only when displayed"]
#[derive(Clone, Copy, Debug)]
pub struct RangeDisplay<'t, 'n> {
    typographer: &'t Typographer,
    from: Number<'n>,
    to: Number<'n>,
    output: Output,
}

impl Typographer {
    /// Formats a number by the language: decimal separator, grouping from the configured digit
    /// count, minus sign (design section 2.7). Lazy: the result implements `Display` and allocates
    /// nothing.
    ///
    /// `NaN` and the infinities are written with the symbols of the language; a zero value has no
    /// sign. The output kind encodes the result as it encodes text (design section 11.7), and the
    /// nowrap styles of HTML write one element around a result that contains glue.
    ///
    /// ```
    /// use typolish::{HtmlStyle, Language, Output, RuleSet, Typographer};
    ///
    /// static RU: Typographer = Typographer::new(Language::RU_RU, RuleSet::PROSE);
    ///
    /// assert_eq!(RU.format_number(-1234567.5, Output::Unicode).to_string(), "\u{2212}1\u{a0}234\u{a0}567,5");
    /// assert_eq!(RU.format_number(f64::NAN, Output::Unicode).to_string(), "не\u{a0}число");
    /// assert_eq!(RU.format_number(-12345, Output::Html(HtmlStyle::NAMED)).to_string(), "&minus;12&nbsp;345");
    /// assert_eq!(
    ///     RU.format_number(-12345, Output::Html(HtmlStyle::NAMED.nowrap_class("nowrap"))).to_string(),
    ///     "<span class=\"nowrap\">&minus;12 345</span>",
    /// );
    /// ```
    pub fn format_number<'n>(
        &self,
        number: impl Into<Number<'n>>,
        output: Output,
    ) -> NumberDisplay<'_, 'n> {
        NumberDisplay {
            typographer: self,
            number: number.into(),
            output,
        }
    }

    /// Formats two numbers with the language's range pattern (`{0}–{1}`): the explicit way to get
    /// a dash between two values. Negative ends are written as they are and equal ends are not
    /// collapsed: the caller decides what is a range (design section 2.7).
    ///
    /// ```
    /// use typolish::{Language, Output, RuleSet, Typographer};
    ///
    /// static RU: Typographer = Typographer::new(Language::RU_RU, RuleSet::PROSE);
    ///
    /// assert_eq!(RU.format_range(8000, 8100, Output::Unicode).to_string(), "8\u{a0}000–8\u{a0}100");
    /// ```
    pub fn format_range<'n>(
        &self,
        from: impl Into<Number<'n>>,
        to: impl Into<Number<'n>>,
        output: Output,
    ) -> RangeDisplay<'_, 'n> {
        RangeDisplay {
            typographer: self,
            from: from.into(),
            to: to.into(),
            output,
        }
    }
}

impl fmt::Display for NumberDisplay<'_, '_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let data = self.typographer.number_data();
        write_encoded(self.output, f, &|emit| {
            write_number(self.number, &data, emit)
        })
    }
}

impl fmt::Display for RangeDisplay<'_, '_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let data = self.typographer.number_data();
        write_encoded(self.output, f, &|emit| {
            write_number(self.from, &data, emit)?;
            if data.range_spaced {
                emit('\u{a0}')?;
            }
            emit(data.range_dash)?;
            if data.range_spaced {
                emit('\u{a0}')?;
            }
            write_number(self.to, &data, emit)
        })
    }
}

/// A sink of the characters of a number in Unicode.
type Emit<'e> = dyn FnMut(char) -> fmt::Result + 'e;

/// Writes the Unicode characters of a number, made by `produce`, in the output kind. A nowrap
/// style of HTML writes one element around the whole result when it contains glue (U+00A0 or
/// U+202F), with U+00A0 as an ordinary space inside (design section 11.5).
fn write_encoded(
    output: Output,
    f: &mut fmt::Formatter<'_>,
    produce: &dyn Fn(&mut Emit<'_>) -> fmt::Result,
) -> fmt::Result {
    if output == Output::Unicode {
        return produce(&mut |c| f.write_char(c));
    }
    let nowrap = output::nowrap_of(output);
    let mut glue = false;
    if nowrap.is_some() {
        // Stops at the first glue character; the error only ends the scan.
        let _ = produce(&mut |c| {
            if matches!(c, '\u{a0}' | '\u{202f}') {
                glue = true;
                Err(fmt::Error)
            } else {
                Ok(())
            }
        });
    }
    let mut out = output::Fmt(f);
    let mut encoder = Encoder::new(output);
    let wrap = match nowrap {
        Some(nowrap) if glue => Some(nowrap),
        _ => None,
    };
    if let Some(nowrap) = wrap {
        output::open(&mut out, nowrap).map_err(|Stop| fmt::Error)?;
    }
    produce(&mut |c| {
        encoder
            .char(&mut out, c, wrap.is_some())
            .map_err(|Stop| fmt::Error)
    })?;
    encoder.flush(&mut out).map_err(|Stop| fmt::Error)?;
    if wrap.is_some() {
        out.put(output::CLOSE).map_err(|Stop| fmt::Error)?;
    }
    Ok(())
}

/// Writes one number in Unicode: the sign, the integer part grouped from the right, the decimal
/// separator and the fraction; `NaN` and the infinities as the symbols of the language.
fn write_number(number: Number<'_>, data: &NumberData, emit: &mut Emit<'_>) -> fmt::Result {
    let (special, negative) = match number.repr {
        Repr::Signed(v) => (None, v < 0),
        Repr::Unsigned(_) => (None, false),
        Repr::F32(v) => (special(v.is_nan(), v.is_infinite()), v.is_sign_negative()),
        Repr::F64(v) | Repr::Fixed(v, _) => {
            (special(v.is_nan(), v.is_infinite()), v.is_sign_negative())
        }
        Repr::Decimal { negative, .. } => (None, negative),
    };
    match special {
        Some(Special::NotANumber) => return data.nan.chars().try_for_each(emit),
        Some(Special::Infinity) => {
            if negative {
                emit(data.minus)?;
            }
            return data.infinity.chars().try_for_each(emit);
        }
        None => {}
    }
    // The first pass learns the length of the integer part and whether the value is zero; the
    // second writes it (design section 2.7: grouping is counted from the right).
    let mut scan = Scan::default();
    digits(number, &mut scan)?;
    if negative && scan.nonzero {
        emit(data.minus)?;
    }
    let min = usize::from(data.min_grouping_digits);
    let mut group = Group {
        emit,
        total: scan.int,
        seen: 0,
        fraction: false,
        grouped: min > 0 && scan.int >= 3 + min,
        separator: data.group,
        decimal: data.decimal,
    };
    digits(number, &mut group)
}

/// `NaN` or an infinity.
#[derive(Clone, Copy)]
enum Special {
    NotANumber,
    Infinity,
}

const fn special(nan: bool, infinite: bool) -> Option<Special> {
    if nan {
        Some(Special::NotANumber)
    } else if infinite {
        Some(Special::Infinity)
    } else {
        None
    }
}

/// The ASCII digits of a finite number without its sign, with `.` before the fraction.
fn digits(number: Number<'_>, w: &mut impl fmt::Write) -> fmt::Result {
    match number.repr {
        Repr::Signed(v) => write!(w, "{}", v.unsigned_abs()),
        Repr::Unsigned(v) => write!(w, "{v}"),
        Repr::F32(v) => write!(w, "{}", v.abs()),
        Repr::F64(v) => write!(w, "{}", v.abs()),
        Repr::Fixed(v, n) => write!(w, "{:.*}", usize::from(n), v.abs()),
        Repr::Decimal { int, frac, .. } => {
            w.write_str(int)?;
            if let Some(frac) = frac {
                w.write_char('.')?;
                w.write_str(frac)?;
            }
            Ok(())
        }
    }
}

/// The first pass: the number of integer digits and whether any digit is not zero.
#[derive(Default)]
struct Scan {
    int: usize,
    fraction: bool,
    nonzero: bool,
}

impl fmt::Write for Scan {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        for c in s.chars() {
            match c {
                '.' => self.fraction = true,
                '0'..='9' => {
                    self.int += usize::from(!self.fraction);
                    self.nonzero |= c != '0';
                }
                _ => {}
            }
        }
        Ok(())
    }
}

/// The second pass: the digits with the group separator and the decimal separator.
struct Group<'g, 'e> {
    emit: &'g mut Emit<'e>,
    total: usize,
    seen: usize,
    fraction: bool,
    grouped: bool,
    separator: char,
    decimal: char,
}

impl fmt::Write for Group<'_, '_> {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        for c in s.chars() {
            if c == '.' {
                self.fraction = true;
                (self.emit)(self.decimal)?;
                continue;
            }
            if !self.fraction {
                if self.grouped && self.seen > 0 && (self.total - self.seen).is_multiple_of(3) {
                    (self.emit)(self.separator)?;
                }
                self.seen += 1;
            }
            (self.emit)(c)?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Language, RuleSet};

    #[test]
    fn the_scan_counts_integer_digits() {
        let mut scan = Scan::default();
        digits(Number::from(-12345.5), &mut scan).unwrap();
        assert_eq!((scan.int, scan.nonzero), (5, true));
        let mut scan = Scan::default();
        digits(Number::fixed(-0.001, 1), &mut scan).unwrap();
        assert_eq!((scan.int, scan.nonzero), (1, false));
    }

    #[test]
    fn a_spaced_range_pattern_glues_both_sides_of_its_dash() {
        // Design section 2.7: no built-in language has spaces in its range pattern.
        let language = crate::LanguageBuilder::from_language(&Language::COMMON, "xx")
            .numbers(NumberData::new().with_range_spaced(true))
            .build()
            .unwrap();
        let t = Typographer::new(language, RuleSet::EMPTY);
        assert_eq!(
            t.format_range(1, 2, Output::Unicode).to_string(),
            "1\u{a0}\u{2013}\u{a0}2"
        );
        let nowrap = Output::Html(crate::HtmlStyle::NAMED.nowrap_style());
        assert_eq!(
            t.format_range(1, 2, nowrap).to_string(),
            "<span style=\"white-space:nowrap\">1 &ndash; 2</span>"
        );
    }

    #[test]
    fn isize_and_usize_convert() {
        let t = Typographer::new(Language::EN_US, RuleSet::EMPTY);
        assert_eq!(
            t.format_number(-12345isize, Output::Unicode).to_string(),
            "\u{2212}12,345"
        );
        assert_eq!(t.format_number(5usize, Output::Unicode).to_string(), "5");
    }
}
