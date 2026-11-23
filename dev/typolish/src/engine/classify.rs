//! The fragment classifier (design section 3.6): fragments, wrappers and cores, the kind of every
//! core, the `FROZEN` and `LABEL` flags, and the diagnostics of the classifier (`MixedScript`,
//! `SlotGlue`, design section 12.4).
//!
//! A fragment is a maximal run of tokens of one window without `Space` and `Newline` tokens. The
//! checks read the core as a list of units: its tokens without `Format` and `Markup` tokens, with
//! words, digit runs, hyphen runs and dot runs that only such tokens separate joined into one
//! unit, so `ноя\u{AD}бря` is the word `ноября` (design sections 3.6 and 5.4). Every check is a
//! function over the units; no regular expression is used.

use super::Recorder;
use super::lex::{Flags, Kind, Mark, NO_PIECE, Op, Tok, WordScript, is_format_char, is_joiner};
use super::record::{Loc, Sink};
use crate::lang::data::{Shape, Table, WordList, fold_char};
use crate::report::{ProtectedKind, TechKind};
use crate::{DiagnosticKind, Severity, SlotKind, Typographer};

/// The kind of a fragment (design section 3.6, the Kind column, plus `Times`, `Sign` and
/// `Punctuation`). Not the token `Kind` of section 3.5.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum FragKind {
    Dictionary,
    Url,
    Email,
    Flag,
    Path,
    TimeRange,
    Colon,
    OperatorRun,
    Operator,
    Identifier,
    Angle,
    Uuid,
    Hex,
    IsoDate,
    YearMonth,
    Version,
    Abbreviation,
    Number,
    Dimensions,
    Range,
    Dotted,
    List,
    Compound,
    Joined,
    Word,
    Unclassified,
    Times,
    Sign,
    Punctuation,
}

impl FragKind {
    /// The kind of `Report::protected()` for the frozen tokens of a core of this kind.
    pub(crate) const fn tech(self) -> Option<TechKind> {
        Some(match self {
            FragKind::Url => TechKind::Url,
            FragKind::Email => TechKind::Email,
            FragKind::Flag => TechKind::Flag,
            FragKind::Path => TechKind::Path,
            FragKind::TimeRange => TechKind::TimeRange,
            FragKind::Colon => TechKind::Colon,
            FragKind::Operator => TechKind::Operator,
            FragKind::Identifier => TechKind::Identifier,
            FragKind::Angle => TechKind::Angle,
            FragKind::Uuid => TechKind::Uuid,
            FragKind::Hex => TechKind::Hex,
            FragKind::IsoDate => TechKind::IsoDate,
            FragKind::Version => TechKind::Version,
            FragKind::Dotted => TechKind::Dotted,
            FragKind::List => TechKind::List,
            FragKind::Compound => TechKind::CompoundHyphens,
            FragKind::Unclassified => TechKind::Unclassified,
            _ => return None,
        })
    }

    /// Whether the table freezes the whole core.
    const fn freezes_whole_core(self) -> bool {
        !matches!(
            self.tech(),
            None | Some(TechKind::TimeRange | TechKind::CompoundHyphens)
        )
    }
}

/// One fragment of a window.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Frag {
    /// Token indices of the fragment, `start..end`.
    pub(crate) start: u32,
    pub(crate) end: u32,
    /// Token indices of the core, `core_start..core_end`; empty for `Punctuation`.
    pub(crate) core_start: u32,
    pub(crate) core_end: u32,
    pub(crate) kind: FragKind,
    /// The token after the core is a `Dots` token (design section 3.6, "stop").
    pub(crate) stop: bool,
    /// The core counts as a label number (design section 3.6, "Label numbers").
    pub(crate) label: bool,
    /// The fragment holds a slot of kind `Other` (section 8.2: no glue sticks to it). Slots are
    /// never edited, so this holds for every phase of the pass.
    pub(crate) other_slot: bool,
}

/// One unit of a core.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Unit {
    u: U,
    /// Byte range in the call's text.
    start: u32,
    end: u32,
    /// Token indices of the first and the last token of the unit.
    first: u32,
    last: u32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum U {
    Word(WordScript),
    Num,
    /// A `Number` slot.
    SlotNum,
    /// A `Date` slot.
    SlotDate,
    /// A `Word` or `Other` slot, or protected text: letters for the checks.
    Letters,
    /// A run of `-` of this length.
    Hy(u32),
    /// A run of `.` of this length.
    Dots(u32),
    /// `…`.
    Ell,
    /// A single-character dash: `‐ ‑ ‒ – — ― −`.
    Dash(char),
    Quote(char),
    Open(char),
    Close(char),
    Punct(char),
    Op(Op),
    Legal(Mark),
    Times,
    Sym,
}

/// Working memory of the classifier.
#[derive(Debug, Default)]
pub(crate) struct ClassifyScratch {
    pub(crate) frags: Vec<Frag>,
    units: Vec<Unit>,
    key: String,
    lowered: String,
}

impl ClassifyScratch {
    pub(crate) const fn new() -> ClassifyScratch {
        ClassifyScratch {
            frags: Vec::new(),
            units: Vec::new(),
            key: String::new(),
            lowered: String::new(),
        }
    }

    pub(crate) fn heap_bytes(&self) -> usize {
        self.frags.capacity() * core::mem::size_of::<Frag>()
            + self.units.capacity() * core::mem::size_of::<Unit>()
            + self.key.capacity()
            + self.lowered.capacity()
    }
}

/// The characters of a range of the call's text that the checks read: without format
/// characters, ZWNJ and ZWJ (design section 3.6).
fn visible(text: &str, start: u32, end: u32) -> impl Iterator<Item = char> + Clone + '_ {
    text.get(start as usize..end as usize)
        .unwrap_or("")
        .chars()
        .filter(|c| !is_joiner(*c) && !is_format_char(*c))
}

/// Classifies the fragments of one window: sets `FROZEN`, `LABEL` and the `Times` kind on
/// `tokens`, fills `cs.frags`, and reports the classifier's diagnostics.
pub(crate) fn classify<R: Recorder>(
    t: &Typographer,
    text: &str,
    tokens: &mut [Tok],
    cs: &mut ClassifyScratch,
    sink: &mut Sink<'_, R>,
) {
    let data = t.language().data();
    cs.frags.clear();
    let mut i = 0usize;
    while i < tokens.len() {
        let separates = |tok: &Tok| matches!(tok.kind, Kind::Space | Kind::Newline);
        if tokens.get(i).is_some_and(separates) {
            i += 1;
            continue;
        }
        let start = i;
        while tokens.get(i).is_some_and(|tok| !separates(tok)) {
            i += 1;
        }
        let (Ok(s), Ok(e)) = (u32::try_from(start), u32::try_from(i)) else {
            return;
        };
        let frag = classify_fragment(t, &data, text, tokens, s, e, cs, sink);
        cs.frags.push(frag);
    }
    lone_times(data.signs, text, tokens, &mut cs.frags);
    labels(text, tokens, &mut cs.frags);
    // Every token names its fragment, and the tokens of a core carry `CORE`: the phases find the
    // classification of the pass through them after an edit inserted or removed a space (design
    // section 3.6).
    for (index, frag) in cs.frags.iter().enumerate() {
        let Ok(index) = u32::try_from(index) else {
            break;
        };
        for (i, tok) in tokens
            .get_mut(frag.start as usize..frag.end as usize)
            .unwrap_or_default()
            .iter_mut()
            .enumerate()
        {
            tok.frag = index;
            let at = frag.start as usize + i;
            if (frag.core_start as usize..frag.core_end as usize).contains(&at) {
                tok.flags = tok.flags.with(Flags::CORE);
            }
        }
    }
    if R::ACTIVE {
        slot_glue(tokens, sink);
    }
}

/// Whether a token belongs to the start wrapper.
fn start_wrapper(tok: &Tok, text: &str) -> bool {
    match tok.kind {
        Kind::Open('(') | Kind::Dots => true,
        Kind::Quote => !matches!(first_char(text, tok), Some('′' | '″')),
        _ => false,
    }
}

/// Whether a token belongs to the end wrapper.
fn end_wrapper(tok: &Tok) -> bool {
    matches!(
        tok.kind,
        Kind::Close(')') | Kind::Quote | Kind::Dots | Kind::Punct(',' | ';' | ':' | '!' | '?')
    )
}

fn first_char(text: &str, tok: &Tok) -> Option<char> {
    text.get(tok.src.start as usize..tok.src.end as usize)?
        .chars()
        .next()
}

#[expect(clippy::too_many_arguments, reason = "the parts of one classification")]
fn classify_fragment<R: Recorder>(
    t: &Typographer,
    data: &Table<'_>,
    text: &str,
    tokens: &mut [Tok],
    start: u32,
    end: u32,
    cs: &mut ClassifyScratch,
    sink: &mut Sink<'_, R>,
) -> Frag {
    let tok = |i: u32| tokens.get(i as usize);
    // The wrapper, by tokens; `Format` and `Markup` tokens are transparent.
    let mut cs_i = start;
    while let Some(t) = tok(cs_i) {
        if cs_i >= end || !(t.kind.is_transparent() || start_wrapper(t, text)) {
            break;
        }
        cs_i += 1;
    }
    let mut ce_i = end;
    while ce_i > cs_i {
        match tok(ce_i - 1) {
            Some(t) if t.kind.is_transparent() || end_wrapper(t) => ce_i -= 1,
            _ => break,
        }
    }
    let mut frag = Frag {
        start,
        end,
        core_start: cs_i,
        core_end: ce_i,
        kind: FragKind::Punctuation,
        stop: false,
        label: false,
        other_slot: tokens
            .get(start as usize..end as usize)
            .unwrap_or_default()
            .iter()
            .any(|t| t.kind == Kind::Slot(SlotKind::Other)),
    };
    if cs_i >= ce_i {
        frag.core_start = cs_i;
        frag.core_end = cs_i;
        return frag;
    }
    // The stop after the core: `Format` and `Markup` tokens before it are skipped (design
    // section 3.6).
    let mut stop_i = ce_i;
    while stop_i < end && tok(stop_i).is_some_and(|t| t.kind.is_transparent()) {
        stop_i += 1;
    }
    frag.stop = stop_i < end && tok(stop_i).is_some_and(|t| t.kind == Kind::Dots);

    let core = tokens.get(cs_i as usize..ce_i as usize).unwrap_or_default();
    build_units(text, core, cs_i, &mut cs.units);
    let units = &cs.units;
    let decided = special_case(data, core, units).map(|kind| (kind, Freeze::Nothing));
    let (kind, freeze) = match decided {
        Some(decided) => decided,
        None if dictionary(t, data, text, tokens, &frag, cs) => {
            (FragKind::Dictionary, Freeze::Nothing)
        }
        None => checks(data, t.minus_sign(), text, &cs.units, frag.stop),
    };
    frag.kind = kind;
    // The core of a sign attached to a number or a Roman numeral is a label (`№5`).
    frag.label = kind == FragKind::Word && sign_with_numeral(data, text, &cs.units);

    // Diagnostics: a word of two scripts, unless the core is a dictionary entry (design section
    // 5.2).
    if R::ACTIVE && kind != FragKind::Dictionary {
        for unit in &cs.units {
            if unit.u == U::Word(WordScript::Mixed) {
                sink.diagnostic(
                    DiagnosticKind::MixedScript,
                    Severity::Warning,
                    Loc::Flat(unit.start, unit.end),
                );
            }
        }
    }

    // Freezing (design section 3.7) and the `Times` kind of check 16.
    let mut freeze_tokens = |first: u32, last: u32| {
        for i in first..=last {
            if let Some(t) = tokens.get_mut(i as usize) {
                t.flags = t.flags.with(Flags::FROZEN);
            }
        }
    };
    match freeze {
        Freeze::Nothing if kind.freezes_whole_core() => freeze_tokens(cs_i, ce_i - 1),
        Freeze::Nothing => {}
        Freeze::Units(a, b) => {
            for range in [a, b] {
                if let Some((first, last)) = range.and_then(|(x, y)| unit_tokens(&cs.units, x, y)) {
                    freeze_tokens(first, last);
                }
            }
        }
        Freeze::Hyphens => {
            for unit in &cs.units {
                if matches!(unit.u, U::Hy(1)) {
                    freeze_tokens(unit.first, unit.last);
                }
            }
        }
        Freeze::Times(index) => {
            if let Some(unit) = cs.units.get(index)
                && let Some(t) = tokens.get_mut(unit.first as usize)
            {
                t.kind = Kind::Times;
            }
        }
    }
    frag
}

/// What a check freezes besides the whole core.
#[derive(Clone, Copy, Debug)]
enum Freeze {
    /// Nothing more than the Frozen column says for the kind.
    Nothing,
    /// The tokens of these unit ranges (check 5: the two times).
    Units(Option<(usize, usize)>, Option<(usize, usize)>),
    /// The single hyphens (check 21).
    Hyphens,
    /// Nothing; the unit at this index becomes `Times` (check 16).
    Times(usize),
}

/// The token range of units `a..=b`.
fn unit_tokens(units: &[Unit], a: usize, b: usize) -> Option<(u32, u32)> {
    Some((units.get(a)?.first, units.get(b)?.last))
}

/// Builds the units of a core.
fn build_units(text: &str, core: &[Tok], offset: u32, out: &mut Vec<Unit>) {
    out.clear();
    for (i, tok) in core.iter().enumerate() {
        let Ok(i) = u32::try_from(i) else {
            return;
        };
        let index = offset + i;
        if tok.kind.is_transparent() {
            continue;
        }
        let count = |c: char| -> u32 {
            u32::try_from(
                visible(text, tok.src.start, tok.src.end)
                    .filter(|x| *x == c)
                    .count(),
            )
            .unwrap_or(u32::MAX)
        };
        let first = first_char(text, tok).unwrap_or(' ');
        let u = match tok.kind {
            Kind::Word(script) => U::Word(script),
            Kind::Number => U::Num,
            Kind::Dash if first == '-' => U::Hy(count('-')),
            Kind::Dash => U::Dash(first),
            Kind::Dots if first == '.' => U::Dots(count('.')),
            Kind::Dots => U::Ell,
            Kind::Quote => U::Quote(first),
            Kind::Open(c) => U::Open(c),
            Kind::Close(c) => U::Close(c),
            Kind::Punct(c) => U::Punct(c),
            Kind::Operator(op) => U::Op(op),
            Kind::LegalMark(mark) => U::Legal(mark),
            Kind::Times => U::Times,
            Kind::Slot(SlotKind::Number) => U::SlotNum,
            Kind::Slot(SlotKind::Date) => U::SlotDate,
            Kind::Slot(_) | Kind::Protected => U::Letters,
            Kind::Symbol | Kind::Space | Kind::Newline | Kind::Format | Kind::Markup => U::Sym,
        };
        // Units that only transparent tokens separate join (design section 5.4).
        if let Some(last) = out.last_mut()
            && last.last + 1 < index
            && core
                .get((last.last + 1 - offset) as usize..i as usize)
                .is_some_and(|between| between.iter().all(|t| t.kind.is_transparent()))
        {
            let joined = match (last.u, u) {
                (U::Word(a), U::Word(b)) => {
                    Some(U::Word(if a == b { a } else { WordScript::Mixed }))
                }
                (U::Num, U::Num) => Some(U::Num),
                (U::Hy(a), U::Hy(b)) => Some(U::Hy(a.saturating_add(b))),
                (U::Dots(a), U::Dots(b)) => Some(U::Dots(a.saturating_add(b))),
                _ => None,
            };
            if let Some(joined) = joined {
                last.u = joined;
                last.end = tok.src.end;
                last.last = index;
                continue;
            }
        }
        out.push(Unit {
            u,
            start: tok.src.start,
            end: tok.src.end,
            first: index,
            last: index,
        });
    }
}

/// The four special cases decided before the table (design section 3.6), the lone `x` aside,
/// which needs its neighbors (`lone_times`).
fn special_case(data: &Table<'_>, core: &[Tok], units: &[Unit]) -> Option<FragKind> {
    let mut substantive = core.iter().filter(|t| !t.kind.is_transparent());
    if let (Some(only), None) = (substantive.next(), substantive.next())
        && only.kind == Kind::Dash
    {
        return Some(FragKind::OperatorRun);
    }
    match units {
        [Unit { u: U::Punct(a), .. }] if data.signs.contains(a) => Some(FragKind::Sign),
        [Unit { u: U::Punct(a), .. }, Unit { u: U::Punct(b), .. }]
            if a == b && data.signs.contains(a) =>
        {
            Some(FragKind::Sign)
        }
        [Unit { u: U::Legal(_), .. }]
        | [
            Unit { u: U::Legal(_), .. },
            Unit {
                u: U::Num | U::SlotNum,
                ..
            },
        ] => Some(FragKind::Word),
        _ => None,
    }
}

/// Check 0: the core equals an entry of the language's dictionaries (design sections 3.6, 7.1).
///
/// The visible text of the core is looked up first as it is; only when that finds nothing and the
/// core has a stop is it looked up again with a full stop appended, for the entries that end in
/// `.` (`т.е.`, `нояб.`). The stop adds matches; it never hides an entry without one (`км/ч.`).
fn dictionary(
    t: &Typographer,
    data: &Table<'_>,
    text: &str,
    tokens: &[Tok],
    frag: &Frag,
    cs: &mut ClassifyScratch,
) -> bool {
    let core = tokens
        .get(frag.core_start as usize..frag.core_end as usize)
        .unwrap_or_default();
    let (Some(first), Some(last)) = (core.first(), core.last()) else {
        return false;
    };
    if core.iter().any(|t| t.kind.is_opaque()) {
        return false;
    }
    // An elision starts with an apostrophe, which the wrapper cuts off: the last token of the
    // wrapper that is not `Format` or `Markup` (design section 3.6).
    let apostrophe = tokens
        .get(frag.start as usize..frag.core_start as usize)
        .and_then(|wrapper| wrapper.iter().rev().find(|t| !t.kind.is_transparent()))
        .is_some_and(|t| t.kind == Kind::Quote && matches!(first_char(text, t), Some('\'' | '’')));
    // A key whose signature no entry has is no entry (design section 3.6); the key itself is
    // built only for the lookups.
    let chars = visible(text, first.src.start, last.src.end);
    let signatures = &t.compiled().signatures;
    // A key longer than every entry, with or without its stop or apostrophe, is none.
    let Some(shape) = Shape::of_at_most(chars.clone(), signatures.longest(WordList::Dictionary))
    else {
        return false;
    };
    let held = |stop: bool| {
        let key = if stop { shape.then('.') } else { shape };
        signatures.may_hold_shape(WordList::Dictionary, key)
            || (apostrophe && signatures.may_hold_shape(WordList::Dictionary, key.after('\'')))
    };
    let plain = held(false);
    let stopped = frag.stop && held(true);
    if !plain && !stopped {
        return false;
    }
    cs.key.clear();
    cs.key.extend(chars);
    if plain && entry(t, data, apostrophe, cs, shape) {
        return true;
    }
    stopped && {
        cs.key.push('.');
        entry(t, data, apostrophe, cs, shape.then('.'))
    }
}

/// Whether `cs.key`, of the shape `shape`, equals an entry of check 0, compared as design section
/// 7.1 says. A list is read only when the signature of the key is in its own signatures.
fn entry(
    t: &Typographer,
    data: &Table<'_>,
    apostrophe: bool,
    cs: &mut ClassifyScratch,
    shape: Shape,
) -> bool {
    let signatures = &t.compiled().signatures;
    let held = |list| signatures.may_hold_shape(list, shape);
    let key = cs.key.as_str();
    if held(WordList::Units) && t.has_unit(key) {
        return true;
    }
    let months = held(WordList::Months);
    let abbreviations = held(WordList::Abbreviations);
    // Entries compared with case, an entry that starts with a lowercase letter also matching the
    // text with that letter in uppercase, except a single letter with a full stop (section 7.1).
    cs.lowered.clear();
    let mut chars = key.chars();
    if let Some(c) = chars.next()
        && c.is_uppercase()
        && !(chars.clone().eq(['.']) && c.is_alphabetic())
    {
        cs.lowered.push(fold_char(c));
        cs.lowered.push_str(chars.as_str());
    }
    let cased = [key, cs.lowered.as_str()];
    let cased = cased.iter().filter(|k| !k.is_empty());

    for k in cased {
        if months && data.months.contains(k) {
            return true;
        }
        if abbreviations
            && data
                .abbreviations
                .iter()
                .any(|(parts, _)| parts_match(parts, k))
        {
            return true;
        }
    }
    let exceptions = held(WordList::Exceptions);
    if exceptions && (folded_in(data.atom_exceptions, key) || folded_in(data.elisions, key)) {
        return true;
    }
    let elided = signatures.may_hold_shape(WordList::Exceptions, shape.after('\''));
    apostrophe && elided && !data.elisions.is_empty() && {
        cs.lowered.clear();
        cs.lowered.push('\'');
        cs.lowered.push_str(&cs.key);
        folded_in(data.elisions, &cs.lowered)
    }
}

/// Whether `key` is one part of an abbreviation, or several consecutive parts that all end in a
/// full stop, written together (`т.е.` for `т.` `е.`).
fn parts_match(parts: crate::lang::data::List<'_>, key: &str) -> bool {
    for i in 0..parts.len() {
        let mut rest = key;
        for (n, part) in parts.iter().enumerate().skip(i) {
            if n > i && !(part.ends_with('.') && parts.get(n - 1).is_some_and(|p| p.ends_with('.')))
            {
                break;
            }
            match rest.strip_prefix(part) {
                Some("") => return true,
                Some(r) => rest = r,
                None => break,
            }
        }
    }
    false
}

/// Whether a lowercase word list holds `key`, compared without case (design section 7.1).
fn folded_in(list: crate::lang::data::List<'_>, key: &str) -> bool {
    list.binary_search_by(|entry| entry.chars().cmp(key.chars().map(fold_char)))
        .is_ok()
}

/// The checks 1 to 24 of design section 3.6, in order; `minus` is the minus sign of the settings,
/// which check 15 reads as a sign.
fn checks(data: &Table<'_>, minus: char, text: &str, u: &[Unit], stop: bool) -> (FragKind, Freeze) {
    // The most common core, one word unit, matches none of the checks 1 to 22 (each needs a
    // digit, a punctuation mark, a dash, a dot or a second unit) and is a word by check 23.
    if let [Unit { u: U::Word(_), .. }] = u {
        return (FragKind::Word, Freeze::Nothing);
    }
    table_checks(data, minus, text, u, stop)
}

/// The checks 1 to 24 without the shortcut for one word unit.
fn table_checks(
    data: &Table<'_>,
    minus: char,
    text: &str,
    u: &[Unit],
    stop: bool,
) -> (FragKind, Freeze) {
    let whole = |kind| (kind, Freeze::Nothing);
    if is_url(text, u) {
        return whole(FragKind::Url);
    }
    if is_email(u) {
        return whole(FragKind::Email);
    }
    if is_flag(text, u) {
        return whole(FragKind::Flag);
    }
    if u.iter().any(|x| matches!(x.u, U::Punct('/' | '\\'))) {
        return whole(FragKind::Path);
    }
    if let Some((a, b)) = time_range(text, u) {
        return (FragKind::TimeRange, Freeze::Units(Some(a), Some(b)));
    }
    if is_colon(u) {
        return whole(FragKind::Colon);
    }
    if is_operator_run(u) {
        return whole(FragKind::OperatorRun);
    }
    if let Some(kind) = operator(text, u) {
        return whole(kind);
    }
    if is_uuid(text, u) {
        return whole(FragKind::Uuid);
    }
    if is_hex(text, u) {
        return whole(FragKind::Hex);
    }
    if let Some(kind) = iso_date(text, u) {
        return whole(kind);
    }
    if is_version(text, u) {
        return whole(FragKind::Version);
    }
    if stop && is_abbreviation(text, u) {
        return whole(FragKind::Abbreviation);
    }
    if is_number(u, minus) {
        return whole(FragKind::Number);
    }
    if let Some(index) = dimensions(text, u) {
        return (FragKind::Dimensions, Freeze::Times(index));
    }
    if is_range(data, text, u) {
        return whole(FragKind::Range);
    }
    if is_dotted(u) {
        return whole(FragKind::Dotted);
    }
    if inside(u).any(|x| matches!(x.u, U::Punct(',' | ';'))) {
        return whole(FragKind::List);
    }
    if is_identifier(text, u) {
        return whole(FragKind::Identifier);
    }
    if is_compound(u) {
        return (FragKind::Compound, Freeze::Hyphens);
    }
    if is_joined(data, minus, text, u) {
        return whole(FragKind::Joined);
    }
    if is_word(data, text, u) {
        return whole(FragKind::Word);
    }
    whole(FragKind::Unclassified)
}

/// The units that are neither first nor last.
fn inside(u: &[Unit]) -> impl Iterator<Item = &Unit> {
    u.iter().skip(1).take(u.len().saturating_sub(2))
}

/// Letters or digits: words, numbers, slots and protected text.
fn is_ld(unit: &Unit) -> bool {
    matches!(
        unit.u,
        U::Word(_) | U::Num | U::SlotNum | U::SlotDate | U::Letters
    )
}

fn word_text<'a>(text: &'a str, unit: &Unit) -> impl Iterator<Item = char> + 'a {
    visible(text, unit.start, unit.end)
}

/// The visible text of a unit equals `s`.
fn text_is(text: &str, unit: &Unit, s: &str) -> bool {
    word_text(text, unit).eq(s.chars())
}

/// The number of digits of a `Num` unit.
fn digits(text: &str, unit: &Unit) -> Option<usize> {
    (unit.u == U::Num).then(|| word_text(text, unit).count())
}

/// The value of a `Num` unit of at most nine digits.
fn value(text: &str, unit: &Unit) -> Option<u32> {
    if unit.u != U::Num {
        return None;
    }
    let mut v: u32 = 0;
    let mut n = 0;
    for c in word_text(text, unit) {
        v = v.checked_mul(10)?.checked_add(c.to_digit(10)?)?;
        n += 1;
    }
    (n > 0 && n <= 9).then_some(v)
}

/// Check 1: contains `://`, or starts with `www.`.
fn is_url(text: &str, u: &[Unit]) -> bool {
    let scheme = u.windows(3).any(|w| {
        matches!(
            w,
            [
                Unit {
                    u: U::Punct(':'),
                    ..
                },
                Unit {
                    u: U::Punct('/'),
                    ..
                },
                Unit {
                    u: U::Punct('/'),
                    ..
                }
            ]
        )
    });
    scheme
        || matches!(u, [first, Unit { u: U::Dots(1), .. }, ..]
            if matches!(first.u, U::Word(_)) && text_is(text, first, "www"))
}

/// Check 2: `@` not first and not last; or `@` followed by a letter or digit at the start.
fn is_email(u: &[Unit]) -> bool {
    inside(u).any(|x| x.u == U::Punct('@'))
        || matches!(u, [Unit { u: U::Punct('@'), .. }, second, ..] if is_ld(second))
}

/// Check 3: `-` and an ASCII letter, or `--` and an ASCII letter or digit, at the start.
fn is_flag(text: &str, u: &[Unit]) -> bool {
    let ascii_letter = |unit: &Unit| {
        matches!(unit.u, U::Word(_))
            && word_text(text, unit)
                .next()
                .is_some_and(|c| c.is_ascii_alphabetic())
    };
    match u {
        [Unit { u: U::Hy(1), .. }, second, ..] => ascii_letter(second),
        [Unit { u: U::Hy(2), .. }, second, ..] => ascii_letter(second) || second.u == U::Num,
        _ => false,
    }
}

/// `HH:MM[:SS]` from unit `i`: the index after it.
fn time_at(text: &str, u: &[Unit], i: usize) -> Option<usize> {
    let hours = u.get(i)?;
    if !(1..=2).contains(&digits(text, hours)?) || value(text, hours)? > 23 {
        return None;
    }
    let two = |k: usize| -> Option<()> {
        let unit = u.get(k)?;
        (digits(text, unit)? == 2 && value(text, unit)? <= 59).then_some(())
    };
    if u.get(i + 1)?.u != U::Punct(':') {
        return None;
    }
    two(i + 2)?;
    if u.get(i + 3).is_some_and(|x| x.u == U::Punct(':')) {
        two(i + 4)?;
        return Some(i + 5);
    }
    Some(i + 3)
}

/// Check 5: two times joined by `-` or `–`; the unit ranges of the two times.
fn time_range(text: &str, u: &[Unit]) -> Option<((usize, usize), (usize, usize))> {
    let sep = time_at(text, u, 0)?;
    if !matches!(u.get(sep)?.u, U::Hy(1) | U::Dash('–')) {
        return None;
    }
    let end = time_at(text, u, sep + 1)?;
    (end == u.len()).then_some(((0, sep - 1), (sep + 1, end - 1)))
}

/// Check 6: `:` not first and not last, or the core starts with `::`.
fn is_colon(u: &[Unit]) -> bool {
    inside(u).any(|x| x.u == U::Punct(':'))
        || matches!(
            u,
            [
                Unit {
                    u: U::Punct(':'),
                    ..
                },
                Unit {
                    u: U::Punct(':'),
                    ..
                },
                ..
            ]
        )
}

/// Check 7: only operator characters. Operators count in both forms, so `→` keeps the kind of
/// `->` (class invariance, design section 3.5).
fn is_operator_run(u: &[Unit]) -> bool {
    u.iter().all(|x| {
        matches!(
            x.u,
            U::Hy(_)
                | U::Op(_)
                | U::Punct('<' | '>' | '=' | '!' | '+' | '~' | '|' | '&' | '*' | '^' | '%')
        )
    })
}

/// The characters of a unit for check 8: punctuation and operators as written, anything else
/// as one or more characters that no check reads.
fn check8_chars<'a>(text: &'a str, unit: &'a Unit) -> impl Iterator<Item = char> + 'a {
    let own = match unit.u {
        U::Punct(_) | U::Op(_) => Some(visible(text, unit.start, unit.end)),
        _ => None,
    };
    let other = match unit.u {
        U::Punct(_) | U::Op(_) => None,
        U::Word(_) | U::Num => Some(visible(text, unit.start, unit.end).map(|_| '\u{0}')),
        _ => None,
    };
    let single = match unit.u {
        U::Punct(_) | U::Op(_) | U::Word(_) | U::Num => None,
        _ => Some('\u{0}'),
    };
    own.into_iter()
        .flatten()
        .chain(other.into_iter().flatten())
        .chain(single)
}

/// Check 8: an operator character or a backtick not first and not last; or the core starts with
/// `<` or ends with `>`.
fn operator(text: &str, u: &[Unit]) -> Option<FragKind> {
    let total: usize = u.iter().map(|x| check8_chars(text, x).count()).sum();
    let mut index = 0usize;
    let mut operator = false;
    let mut underscore = false;
    let mut first = '\u{0}';
    let mut last = '\u{0}';
    for unit in u {
        for c in check8_chars(text, unit) {
            if index == 0 {
                first = c;
            }
            last = c;
            if index > 0 && index + 1 < total {
                match c {
                    '_' => underscore = true,
                    '=' | '<' | '>' | '|' | '&' | '^' | '~' | '*' | '#' | '$' | '%' | '+' | '`' => {
                        operator = true;
                    }
                    _ => {}
                }
            }
            index += 1;
        }
    }
    if operator {
        Some(FragKind::Operator)
    } else if underscore {
        Some(FragKind::Identifier)
    } else if first == '<' || last == '>' {
        Some(FragKind::Angle)
    } else {
        None
    }
}

/// The characters of a core made of ASCII words and numbers only.
fn alnum_chars<'a>(text: &'a str, u: &'a [Unit]) -> Option<impl Iterator<Item = char> + 'a> {
    u.iter()
        .all(|x| matches!(x.u, U::Word(_) | U::Num))
        .then(|| u.iter().flat_map(move |x| word_text(text, x)))
}

/// Check 9: UUID 8-4-4-4-12 of hex digits.
fn is_uuid(text: &str, u: &[Unit]) -> bool {
    const GROUPS: [usize; 5] = [8, 4, 4, 4, 12];
    let mut group = 0;
    let mut len = 0;
    for unit in u {
        match unit.u {
            U::Hy(1) => {
                if GROUPS.get(group) != Some(&len) {
                    return false;
                }
                group += 1;
                len = 0;
            }
            U::Word(_) | U::Num => {
                for c in word_text(text, unit) {
                    if !c.is_ascii_hexdigit() {
                        return false;
                    }
                    len += 1;
                }
            }
            _ => return false,
        }
    }
    group == 4 && len == 12
}

/// Checks 10 and 11: `0x` and hex digits; or at least 8 hex characters with a digit and a letter.
fn is_hex(text: &str, u: &[Unit]) -> bool {
    let Some(mut chars) = alnum_chars(text, u) else {
        return false;
    };
    let Some(first) = chars.next() else {
        return false;
    };
    let Some(second) = chars.next() else {
        return false;
    };
    if first == '0' && matches!(second, 'x' | 'X') {
        let mut rest = chars.peekable();
        if rest.peek().is_some() && rest.all(|c| c.is_ascii_hexdigit()) {
            return true;
        }
        return false;
    }
    let mut count = 2;
    let mut digit = first.is_ascii_digit() || second.is_ascii_digit();
    let mut letter = !first.is_ascii_digit() || !second.is_ascii_digit();
    if !first.is_ascii_hexdigit() || !second.is_ascii_hexdigit() {
        return false;
    }
    for c in chars {
        if !c.is_ascii_hexdigit() {
            return false;
        }
        count += 1;
        digit |= c.is_ascii_digit();
        letter |= !c.is_ascii_digit();
    }
    count >= 8 && digit && letter
}

/// Check 12: `YYYY-MM-DD`, or `YYYY-MM` / `YYYY–MM` with month 01 to 12.
fn iso_date(text: &str, u: &[Unit]) -> Option<FragKind> {
    let year = u.first()?;
    if digits(text, year)? != 4 {
        return None;
    }
    match u {
        [_, Unit { u: U::Hy(1), .. }, m, Unit { u: U::Hy(1), .. }, d]
            if digits(text, m) == Some(2) && digits(text, d) == Some(2) =>
        {
            Some(FragKind::IsoDate)
        }
        [_, sep, m] if matches!(sep.u, U::Hy(1) | U::Dash('–')) && digits(text, m) == Some(2) => {
            let month = value(text, m)?;
            (1..=12).contains(&month).then_some(FragKind::YearMonth)
        }
        _ => None,
    }
}

/// Check 13: `[vV]?` number (`.` number){2,} with an optional `-` or `+` tail.
fn is_version(text: &str, u: &[Unit]) -> bool {
    let mut i = 0;
    if let Some(first) = u.first()
        && matches!(first.u, U::Word(_))
        && (text_is(text, first, "v") || text_is(text, first, "V"))
    {
        i = 1;
    }
    if u.get(i).map(|x| x.u) != Some(U::Num) {
        return false;
    }
    i += 1;
    let mut dots = 0;
    while u.get(i).map(|x| x.u) == Some(U::Dots(1)) && u.get(i + 1).map(|x| x.u) == Some(U::Num) {
        dots += 1;
        i += 2;
    }
    if dots < 2 {
        return false;
    }
    match u.get(i).map(|x| x.u) {
        None => true,
        Some(U::Hy(1) | U::Punct('+')) => {
            let tail = u.get(i + 1..).unwrap_or_default();
            !tail.is_empty()
                && tail.iter().all(|x| {
                    matches!(
                        x.u,
                        U::Word(_) | U::Num | U::Dots(1) | U::Hy(1) | U::Punct('+')
                    )
                })
        }
        Some(_) => false,
    }
}

/// Check 14: two or more groups of one or two letters joined by `.` (the caller checks the stop).
fn is_abbreviation(text: &str, u: &[Unit]) -> bool {
    let group = |unit: &Unit| {
        matches!(unit.u, U::Word(_))
            && (1..=2).contains(&word_text(text, unit).filter(|c| c.is_alphabetic()).count())
    };
    u.len() >= 3
        && u.len() % 2 == 1
        && u.iter().enumerate().all(|(i, x)| {
            if i % 2 == 0 {
                group(x)
            } else {
                x.u == U::Dots(1)
            }
        })
}

/// Check 15: a number with an optional sign, decimal part and `%` or `‰`. The sign is `-`, `+`,
/// `±`, `−` or `minus`, the minus sign of the settings that `number.minus` writes, so that the
/// minus of the first pass changes nothing for the second (invariant I4 of design section 4.3).
fn is_number(u: &[Unit], minus: char) -> bool {
    number_end(u, minus) == Some(u.len())
}

/// The index after a number at the start of `u`.
fn number_end(u: &[Unit], minus: char) -> Option<usize> {
    let mut i = 0;
    let sign = match u.first()?.u {
        U::Hy(1) | U::Punct('+') | U::Op(Op::PlusMinus) | U::Dash('−') => true,
        U::Dash(c) => c == minus,
        _ => false,
    };
    if sign {
        i = 1;
    }
    if !matches!(u.get(i)?.u, U::Num | U::SlotNum) {
        return None;
    }
    i += 1;
    if matches!(u.get(i).map(|x| x.u), Some(U::Dots(1) | U::Punct(',')))
        && matches!(u.get(i + 1).map(|x| x.u), Some(U::Num | U::SlotNum))
    {
        i += 2;
    }
    if matches!(u.get(i).map(|x| x.u), Some(U::Punct('%' | '‰'))) {
        i += 1;
    }
    Some(i)
}

/// Check 16: `D+` `x` `D+`, the first factor without a leading zero; the index of the `x`.
fn dimensions(text: &str, u: &[Unit]) -> Option<usize> {
    let [a, x, b] = u else {
        return None;
    };
    let factor = |unit: &Unit| matches!(unit.u, U::Num | U::SlotNum);
    let letter = match x.u {
        U::Times => true,
        U::Word(_) => ["x", "X", "\u{445}", "\u{425}"]
            .iter()
            .any(|s| text_is(text, x, s)),
        _ => false,
    };
    let leading_zero =
        a.u == U::Num && word_text(text, a).next() == Some('0') && digits(text, a) > Some(1);
    (factor(a) && letter && factor(b) && !leading_zero).then_some(1)
}

/// Whether a word unit is a Roman numeral in the standard form, 1 to 3999 (design section 3.6).
fn is_roman(text: &str, unit: &Unit) -> bool {
    if !matches!(unit.u, U::Word(_)) {
        return false;
    }
    let mut chars = word_text(text, unit).peekable();
    // M{0,3}(CM|CD|D?C{0,3})(XC|XL|L?X{0,3})(IX|IV|V?I{0,3}), not empty.
    let mut any = false;
    let take = |chars: &mut core::iter::Peekable<_>, c: char, max: usize| -> usize {
        let mut n = 0;
        while n < max && chars.next_if_eq(&c).is_some() {
            n += 1;
        }
        n
    };
    let m = take(&mut chars, 'M', 3);
    any |= m > 0;
    for (one, five, ten) in [('C', 'D', 'M'), ('X', 'L', 'C'), ('I', 'V', 'X')] {
        if chars.next_if_eq(&one).is_some() {
            any = true;
            if chars.next_if_eq(&ten).is_some() || chars.next_if_eq(&five).is_some() {
                continue;
            }
            take(&mut chars, one, 2);
        } else if chars.next_if_eq(&five).is_some() {
            any = true;
            take(&mut chars, one, 3);
        }
    }
    any && chars.next().is_none()
}

/// One side of check 17 at `u`: an unsigned integer, a decimal with the language's separator, a
/// Roman numeral or a `Number` slot.
fn range_side(data: &Table<'_>, text: &str, u: &[Unit]) -> bool {
    let separator = |unit: &Unit| match data.numbers.decimal {
        '.' => unit.u == U::Dots(1),
        c => unit.u == U::Punct(c),
    };
    match u {
        [one] => matches!(one.u, U::Num | U::SlotNum) || is_roman(text, one),
        [a, sep, b] => a.u == U::Num && separator(sep) && b.u == U::Num,
        _ => false,
    }
}

/// Check 17: `N-N` or `N–N`, or two `Date` slots joined by `-` or `–`.
fn is_range(data: &Table<'_>, text: &str, u: &[Unit]) -> bool {
    let sep = |unit: &Unit| matches!(unit.u, U::Hy(1) | U::Dash('–'));
    if let [a, s, b] = u
        && a.u == U::SlotDate
        && b.u == U::SlotDate
        && sep(s)
    {
        return true;
    }
    [1, 3].iter().any(|&at| {
        u.get(at).is_some_and(sep)
            && range_side(data, text, u.get(..at).unwrap_or_default())
            && range_side(data, text, u.get(at + 1..).unwrap_or_default())
    })
}

/// Check 18: a single `.` between letters or digits.
fn is_dotted(u: &[Unit]) -> bool {
    u.windows(3)
        .any(|w| matches!(w, [a, Unit { u: U::Dots(1), .. }, b] if is_ld(a) && is_ld(b)))
}

/// Check 20: only ASCII letters and digits, both present.
fn is_identifier(text: &str, u: &[Unit]) -> bool {
    let mut letter = false;
    let mut digit = false;
    for unit in u {
        match unit.u {
            U::Num => digit = true,
            U::Word(_) if word_text(text, unit).all(|c| c.is_ascii_alphabetic()) => letter = true,
            _ => return false,
        }
    }
    letter && digit
}

/// Check 21: letters or digits joined by single hyphens.
fn is_compound(u: &[Unit]) -> bool {
    let mut parts = 0;
    let mut in_part = false;
    for unit in u {
        if is_ld(unit) {
            if !in_part {
                parts += 1;
            }
            in_part = true;
        } else if unit.u == U::Hy(1) && in_part {
            in_part = false;
        } else {
            return false;
        }
    }
    in_part && parts >= 2
}

/// Check 22: two or more parts joined by single `—`, `–` or `…`, or by `...`, each a word by
/// check 23 or a number by check 15.
fn is_joined(data: &Table<'_>, minus: char, text: &str, u: &[Unit]) -> bool {
    let joint = |unit: &Unit| matches!(unit.u, U::Dash('—' | '–') | U::Ell | U::Dots(3));
    if !u.iter().any(joint) {
        return false;
    }
    u.split(joint)
        .all(|part| !part.is_empty() && (is_number(part, minus) || is_word(data, text, part)))
}

/// Check 23: letters with their marks and digits, with the quotes, signs and legal marks it
/// allows.
fn is_word(data: &Table<'_>, text: &str, u: &[Unit]) -> bool {
    if sign_with_numeral(data, text, u) {
        return true;
    }
    // A word with a legal mark.
    let body = match u {
        [rest @ .., before, Unit { u: U::Legal(_), .. }] => {
            if !matches!(before.u, U::Word(_) | U::Letters) {
                return false;
            }
            u.get(..rest.len() + 1).unwrap_or_default()
        }
        _ => u,
    };
    if body.is_empty() {
        return false;
    }
    let letters = |unit: Option<&Unit>| {
        unit.is_some_and(|x| matches!(x.u, U::Word(_) | U::Letters | U::SlotDate))
    };
    let number = |unit: Option<&Unit>| unit.is_some_and(|x| matches!(x.u, U::Num | U::SlotNum));
    for (i, unit) in body.iter().enumerate() {
        match unit.u {
            U::Word(_) | U::Num | U::SlotNum | U::SlotDate | U::Letters => {}
            U::Quote(q) => {
                let before = i.checked_sub(1).and_then(|k| body.get(k));
                let after = body.get(i + 1);
                let apostrophe = matches!(q, '\'' | '’')
                    && letters(after)
                    && (letters(before) || number(before));
                let feet = matches!(q, '\'' | '′')
                    && i + 2 == body.len()
                    && before
                        .and_then(|b| digits(text, b))
                        .is_some_and(|n| (1..=3).contains(&n))
                    && after
                        .and_then(|a| digits(text, a))
                        .is_some_and(|n| (1..=2).contains(&n));
                if !(apostrophe || feet) {
                    return false;
                }
            }
            _ => return false,
        }
    }
    true
}

/// A sign of the language, single or doubled, with a number or a Roman numeral (`№5`, `¶IV`,
/// `№{n}`): a form of check 23 and a label.
fn sign_with_numeral(data: &Table<'_>, text: &str, u: &[Unit]) -> bool {
    let sign = |unit: &Unit| matches!(unit.u, U::Punct(c) if data.signs.contains(&c));
    let numeral = |unit: &Unit| matches!(unit.u, U::Num | U::SlotNum) || is_roman(text, unit);
    match u {
        [s, n] => sign(s) && numeral(n),
        [s, t, n] => sign(s) && s.u == t.u && numeral(n),
        _ => false,
    }
}

/// Whether a fragment ends in a number for `lone_times`: a `Number` core, or a sign of the
/// language with a number attached (`№10`), which `nbsp.signs` splits into the sign and a label
/// number (invariant I2 of design section 4.3: a label counts as a number for `number.times` in
/// both forms, section 8.2).
fn ends_in_number(signs: &[char], tokens: &[Tok], f: &Frag) -> bool {
    if f.kind == FragKind::Number {
        return true;
    }
    if f.kind != FragKind::Word {
        return false;
    }
    let mut core = tokens
        .get(f.core_start as usize..f.core_end as usize)
        .unwrap_or_default()
        .iter()
        .filter(|t| !t.kind.is_transparent())
        .map(|t| t.kind);
    let sign = |k: Option<Kind>| matches!(k, Some(Kind::Punct(c)) if signs.contains(&c));
    let number = |k: Option<Kind>| matches!(k, Some(Kind::Number | Kind::Slot(SlotKind::Number)));
    let (first, second, third, fourth) = (core.next(), core.next(), core.next(), core.next());
    sign(first)
        && ((number(second) && third.is_none())
            || (second == first && number(third) && fourth.is_none()))
}

/// Whether `f` is a lone `%` or `‰` after spaces that the fragment `owner` before the spaces
/// owns: a percentage in its spaced form (`10 % x 5`), which reads as the attached form `10% x 5`
/// for `lone_times` (invariant I2 of design section 4.3, section 8.2). The core of `f` is the
/// sign alone and nothing visible comes before it; the owner's core runs up to the spaces and
/// holds no percent sign of its own.
fn spaced_percent(tokens: &[Tok], owner: &Frag, f: &Frag) -> bool {
    let range = |a: u32, b: u32| tokens.get(a as usize..b as usize).unwrap_or_default();
    let visible = |a: u32, b: u32| range(a, b).iter().filter(|t| !t.kind.is_transparent());
    let percent = |t: &Tok| matches!(t.kind, Kind::Punct('%' | '‰'));
    let mut core = visible(f.core_start, f.core_end);
    let lone = matches!((core.next(), core.next()), (Some(t), None) if percent(t))
        && visible(f.start, f.core_start).next().is_none();
    let between = range(owner.end, f.start);
    lone && !between.is_empty()
        && between.iter().all(|t| t.kind == Kind::Space)
        && visible(owner.core_end, owner.end).next().is_none()
        && !visible(owner.core_start, owner.core_end).any(percent)
}

/// The special case of a lone `x`, `X`, `х`, `Х` or `×` fragment between two number fragments:
/// kind `Times`, never frozen (design section 3.6). On the left a percent sign after spaces is
/// read with its number (`spaced_percent`).
fn lone_times(signs: &[char], text: &str, tokens: &mut [Tok], frags: &mut [Frag]) {
    for i in 1..frags.len().saturating_sub(1) {
        let (Some(prev), Some(cur), Some(next)) =
            (frags.get(i - 1), frags.get(i), frags.get(i + 1))
        else {
            continue;
        };
        let left = match i.checked_sub(2).and_then(|k| frags.get(k)) {
            Some(owner) if spaced_percent(tokens, owner, prev) => owner,
            _ => prev,
        };
        if !ends_in_number(signs, tokens, left) || next.kind != FragKind::Number {
            continue;
        }
        // The one token of the fragment that is not `Format` or `Markup` (design section 3.6).
        let mut opaque = (cur.start as usize..cur.end as usize)
            .filter(|&j| tokens.get(j).is_some_and(|t| !t.kind.is_transparent()));
        let (Some(only), None) = (opaque.next(), opaque.next()) else {
            continue;
        };
        let Some(tok) = tokens.get_mut(only) else {
            continue;
        };
        let letter = match tok.kind {
            Kind::Times => true,
            Kind::Word(_) => matches!(
                text.get(tok.src.start as usize..tok.src.end as usize),
                Some("x" | "X" | "\u{445}" | "\u{425}")
            ),
            _ => false,
        };
        if letter {
            tok.kind = Kind::Times;
            tok.flags = Flags::default();
            if let Some(cur) = frags.get_mut(i) {
                cur.kind = FragKind::Times;
            }
        }
    }
}

/// The `LABEL` flag (design section 3.6, "Label numbers").
fn labels(text: &str, tokens: &mut [Tok], frags: &mut [Frag]) {
    // The nearest previous fragment that is not made of `Format` and `Markup` tokens only: those
    // are transparent (design section 5.4), so an empty isolated value between a sign and its
    // number (`№ `, U+2068, U+2069, ` 5`) does not hide the sign. Kept as the loop goes, so
    // marking labels stays linear.
    let mut prev: Option<Frag> = None;
    for i in 0..frags.len() {
        let Some(frag) = frags.get(i).copied() else {
            continue;
        };
        let transparent = tokens
            .get(frag.start as usize..frag.end as usize)
            .is_some_and(|ts| ts.iter().all(|t| t.kind.is_transparent()));
        if transparent {
            continue;
        }
        // Read only after a sign: most cores are not. `Format` and `Markup` tokens of the core
        // are skipped (design section 3.6): every other token is a word.
        let roman = || {
            let mut core = tokens
                .get(frag.core_start as usize..frag.core_end as usize)
                .unwrap_or_default()
                .iter()
                .filter(|t| !t.kind.is_transparent());
            let (Some(first), last) = (core.next(), core.clone().next_back()) else {
                return false;
            };
            let last = last.unwrap_or(first);
            core.all(|t| matches!(t.kind, Kind::Word(_)))
                && matches!(first.kind, Kind::Word(_))
                && is_roman(
                    text,
                    &Unit {
                        u: U::Word(WordScript::Latin),
                        start: first.src.start,
                        end: last.src.end,
                        first: 0,
                        last: 0,
                    },
                )
        };
        let after_sign = prev.replace(frag).is_some_and(|prev| {
            prev.kind == FragKind::Sign
                && tokens
                    .get(prev.end as usize..frag.start as usize)
                    .is_some_and(|between| {
                        between
                            .iter()
                            .all(|t| t.kind == Kind::Space || t.kind.is_transparent())
                    })
        });
        let label = match frag.kind {
            FragKind::Number | FragKind::Range => after_sign,
            // A Roman numeral that is also a unit (`V`, `L`) is a `Dictionary` core.
            FragKind::Word | FragKind::Dictionary => frag.label || (after_sign && roman()),
            _ => false,
        };
        if label {
            for t in tokens
                .get_mut(frag.core_start as usize..frag.core_end as usize)
                .unwrap_or_default()
            {
                t.flags = t.flags.with(Flags::LABEL);
            }
            if let Some(f) = frags.get_mut(i) {
                f.label = true;
            }
        }
    }
}

/// `SlotGlue` (design section 12.4): a `Number` slot glued to letters (warning), two slots glued
/// together (info). Transparent tokens between them do not separate them.
fn slot_glue<R: Recorder>(tokens: &[Tok], sink: &mut Sink<'_, R>) {
    let next = |i: usize| {
        tokens
            .get(i + 1..)
            .and_then(|rest| rest.iter().find(|t| !t.kind.is_transparent()))
    };
    let prev = |i: usize| {
        tokens
            .get(..i)
            .and_then(|rest| rest.iter().rev().find(|t| !t.kind.is_transparent()))
    };
    for (i, tok) in tokens.iter().enumerate() {
        let Kind::Slot(kind) = tok.kind else {
            continue;
        };
        let at_slot = Loc::Piece(tok.piece);
        if let Some(n) = next(i) {
            if matches!(n.kind, Kind::Slot(_)) {
                sink.diagnostic(DiagnosticKind::SlotGlue, Severity::Info, at_slot);
            } else if kind == SlotKind::Number && matches!(n.kind, Kind::Word(_)) {
                sink.diagnostic(DiagnosticKind::SlotGlue, Severity::Warning, at_slot);
            }
        }
        if kind == SlotKind::Number
            && let Some(p) = prev(i)
            && matches!(p.kind, Kind::Word(_))
        {
            sink.diagnostic(
                DiagnosticKind::SlotGlue,
                Severity::Warning,
                Loc::Flat(p.src.start, p.src.end),
            );
        }
    }
}

/// Records the protected spans of one window in token order (design section 12.5): slots,
/// protected pieces, delimited spans and the frozen tokens of technical cores, a run of frozen
/// tokens of one core being one span.
pub(crate) fn record_protected<R: Recorder>(
    tokens: &[Tok],
    frags: &[Frag],
    sink: &mut Sink<'_, R>,
) {
    for frag in frags {
        let tech = frag.kind.tech();
        let mut run: Option<(u32, u32)> = None;
        let flush = |run: &mut Option<(u32, u32)>, sink: &mut Sink<'_, R>| {
            if let (Some((start, end)), Some(tech)) = (run.take(), tech) {
                sink.protected(Loc::Flat(start, end), ProtectedKind::Technical(tech));
            }
        };
        for tok in tokens
            .get(frag.start as usize..frag.end as usize)
            .unwrap_or_default()
        {
            match tok.kind {
                Kind::Slot(_) => {
                    flush(&mut run, sink);
                    sink.protected(Loc::Piece(tok.piece), ProtectedKind::Slot);
                }
                Kind::Protected if tok.piece != NO_PIECE => {
                    flush(&mut run, sink);
                    sink.protected(Loc::Piece(tok.piece), ProtectedKind::ProtectedPiece);
                }
                Kind::Protected => {
                    flush(&mut run, sink);
                    sink.protected(
                        Loc::Flat(tok.src.start, tok.src.end),
                        ProtectedKind::Delimited,
                    );
                }
                _ if tok.is_frozen() && tech.is_some() => match run.as_mut() {
                    Some(run) => run.1 = tok.src.end,
                    None => run = Some((tok.src.start, tok.src.end)),
                },
                _ if tok.kind.is_transparent() && run.is_some() => {}
                _ => flush(&mut run, sink),
            }
        }
        flush(&mut run, sink);
    }
}

#[cfg(test)]
mod tests {
    use super::super::ingest::{Atom, TextPiece};
    use super::super::lex::lex_window;
    use super::super::record::NoRecord;
    use super::*;
    use crate::{Language, RuleSet};

    #[test]
    fn one_word_unit_is_a_word_by_the_whole_table() {
        // The shortcut of `checks` for one word unit gives what the checks 1 to 24 give.
        let words = [
            "www",
            "v",
            "V",
            "x",
            "X",
            "х",
            "Х",
            "deadbeef",
            "DEADBEEF",
            "abcdef",
            "IV",
            "XIX",
            "MCMXCIV",
            "I",
            "Москва",
            "сло\u{200d}во",
            "ноя\u{ad}бря",
            "e\u{301}",
            "Мoсква",
            "αβγ",
            "\u{663}\u{664}",
            "Ⅻ",
            "e",
            "a",
            "Ph",
        ];
        for language in [Language::RU_RU, Language::EN_US, Language::COMMON] {
            let data = language.data();
            for text in words {
                let len = u32::try_from(text.len()).unwrap();
                let atoms = [Atom::Text { start: 0, end: len }];
                let (mut clusters, mut tokens) = (Vec::new(), Vec::new());
                lex_window(text, &atoms, &mut clusters, &mut tokens);
                let mut units = Vec::new();
                build_units(text, &tokens, 0, &mut units);
                if !matches!(units[..], [Unit { u: U::Word(_), .. }]) {
                    continue;
                }
                for stop in [false, true] {
                    let (kind, freeze) = table_checks(&data, '-', text, &units, stop);
                    assert_eq!(kind, FragKind::Word, "{text}");
                    assert!(matches!(freeze, Freeze::Nothing), "{text}");
                }
            }
        }
    }

    /// The fragments of a one-window string: the text of the core (the whole fragment for
    /// `Punctuation`), its kind and its `LABEL` flag.
    fn frags(language: Language, text: &str) -> Vec<(String, FragKind, bool)> {
        let t = Typographer::new(language, RuleSet::INTERFACE);
        let len = u32::try_from(text.len()).unwrap();
        let atoms = [Atom::Text { start: 0, end: len }];
        let mut clusters = Vec::new();
        let mut tokens = Vec::new();
        lex_window(text, &atoms, &mut clusters, &mut tokens);
        let mut cs = ClassifyScratch::new();
        let pieces = [TextPiece {
            piece: 0,
            start: 0,
            end: len,
        }];
        let mut rec = NoRecord;
        let mut sink = Sink {
            rec: &mut rec,
            pieces: &pieces,
            lens: &[len],
        };
        classify(&t, text, &mut tokens, &mut cs, &mut sink);
        cs.frags
            .iter()
            .map(|f| {
                let (a, b) = if f.kind == FragKind::Punctuation {
                    (f.start, f.end)
                } else {
                    (f.core_start, f.core_end)
                };
                let start = tokens[a as usize].src.start as usize;
                let end = tokens[b as usize - 1].src.end as usize;
                for tok in &tokens[f.core_start as usize..f.core_end as usize] {
                    assert_eq!(tok.flags.contains(Flags::LABEL), f.label, "{text}");
                }
                (text[start..end].to_owned(), f.kind, f.label)
            })
            .collect()
    }

    /// The first fragment of a one-window string, as the classifier leaves it.
    fn first_frag(language: Language, text: &str) -> Frag {
        let t = Typographer::new(language, RuleSet::INTERFACE);
        let len = u32::try_from(text.len()).unwrap();
        let atoms = [Atom::Text { start: 0, end: len }];
        let mut clusters = Vec::new();
        let mut tokens = Vec::new();
        lex_window(text, &atoms, &mut clusters, &mut tokens);
        let mut cs = ClassifyScratch::new();
        let pieces = [TextPiece {
            piece: 0,
            start: 0,
            end: len,
        }];
        let mut rec = NoRecord;
        let mut sink = Sink {
            rec: &mut rec,
            pieces: &pieces,
            lens: &[len],
        };
        classify(&t, text, &mut tokens, &mut cs, &mut sink);
        cs.frags[0]
    }

    #[test]
    fn format_tokens_are_skipped_before_the_stop_and_in_a_label() {
        // Design section 3.6: `Format` tokens inside a fragment are skipped by every check.
        let f = first_frag(Language::RU_RU, "т.е\u{ad}...");
        assert!(f.stop);
        assert_eq!(f.kind, FragKind::Dictionary);
        let all = frags(Language::RU_RU, "§ I\u{AD}V");
        assert_eq!(
            all.last().map(|f| (f.0.as_str(), f.2)),
            Some(("I\u{AD}V", true))
        );
    }

    fn kind_of(language: Language, text: &str) -> FragKind {
        let all = frags(language, text);
        assert_eq!(all.len(), 1, "{text}: {all:?}");
        all[0].1
    }

    #[track_caller]
    fn check(language: Language, rows: &[(&str, FragKind)]) {
        for (text, kind) in rows {
            assert_eq!(kind_of(language.clone(), text), *kind, "{language}: {text}");
        }
    }

    #[test]
    fn classifier_tables_of_section_13_2() {
        use FragKind::*;
        let ru = Language::RU_RU;
        check(
            ru.clone(),
            &[
                ("Ph.D.", Abbreviation),
                ("А.С.", Abbreviation),
                ("№", Sign),
                ("§§", Sign),
                ("¶", Sign),
                ("¶IV", Word),
                ("№5", Word),
                ("(c)", Word),
                ("©", Word),
                ("(c)2026", Word),
                ("2026(c)", Unclassified),
                ("XIX-XX", Range),
                ("IIII-V", Compound),
                ("xix-xx", Compound),
            ],
        );
        assert_eq!(
            frags(ru.clone(), "Документ № 5"),
            [
                ("Документ".to_owned(), Word, false),
                ("№".to_owned(), Sign, false),
                ("5".to_owned(), Number, true),
            ]
        );
        assert!(frags(ru.clone(), "¶IV")[0].2);
        assert!(frags(ru.clone(), "№5")[0].2);
        assert_eq!(frags(ru.clone(), "¶ IV")[1], ("IV".to_owned(), Word, true));
        assert_eq!(frags(ru.clone(), "¶ VV")[1], ("VV".to_owned(), Word, false));
        assert_eq!(
            frags(ru.clone(), "§§ 5-7")[1],
            ("5-7".to_owned(), Range, true)
        );
        assert!(frags(ru.clone(), "№\u{a0}5")[1].2);
        // A line break between them: no label.
        assert!(!frags(ru.clone(), "№\n5")[1].2);
        // `г.` after a label is a dictionary entry, not a year.
        assert_eq!(
            frags(ru.clone(), "Школа №5 г. Москвы"),
            [
                ("Школа".to_owned(), Word, false),
                ("№5".to_owned(), Word, true),
                ("г".to_owned(), Dictionary, false),
                ("Москвы".to_owned(), Word, false),
            ]
        );
    }

    #[test]
    fn roman_numerals() {
        let roman = |text: &str| {
            is_roman(
                text,
                &Unit {
                    u: U::Word(WordScript::Latin),
                    start: 0,
                    end: u32::try_from(text.len()).unwrap(),
                    first: 0,
                    last: 0,
                },
            )
        };
        for yes in [
            "XIX",
            "IV",
            "MCMXCIV",
            "I",
            "V",
            "X",
            "MMMCMXCIX",
            "XL",
            "CD",
        ] {
            assert!(roman(yes), "{yes}");
        }
        for no in [
            "IIII",
            "IC",
            "VV",
            "XM",
            "xix",
            "Mix",
            "\u{425}I\u{425}",
            "",
            "MMMM",
            "IIV",
        ] {
            assert!(!roman(no), "{no}");
        }
    }

    #[test]
    fn kinds_that_are_not_frozen() {
        use FragKind::*;
        check(
            Language::RU_RU,
            &[
                ("км/ч", Dictionary),
                ("т.е.", Dictionary),
                ("т.е...", Dictionary),
                ("-5", Number),
                ("1,5%", Number),
                ("1920x1080", Dimensions),
                ("1920\u{445}1080", Dimensions),
                ("5-10", Range),
                ("2026-11", YearMonth),
                ("да...нет", Joined),
                ("слово–слово", Joined),
                ("кто-то", Compound),
                ("don't", Word),
                ("5'10", Word),
                ("->", OperatorRun),
                ("-", OperatorRun),
                ("—", OperatorRun),
                ("\"-\"", OperatorRun),
                ("=>", OperatorRun),
                ("...", Punctuation),
                ("«»", Punctuation),
                ("Мo\u{441}ква", Word),
            ],
        );
        check(
            Language::EN_US,
            &[
                ("e.g.", Dictionary),
                ("etc…", Dictionary),
                ("\u{3bc}s", Dictionary),
                ("1.5-2.5", Range),
            ],
        );
    }

    #[test]
    fn an_entry_without_a_full_stop_matches_a_core_that_has_a_stop() {
        // Design section 3.6, check 0: the stop only adds the entries that end in `.`.
        use FragKind::*;
        let last = |language: Language, text: &str| {
            let all = frags(language, text);
            let (core, kind, _) = all.last().cloned().unwrap();
            (core, kind)
        };
        for (text, core) in [
            ("60 км/ч.", "км/ч"),
            ("60 км/ч...", "км/ч"),
            ("60 МБ/с.", "МБ/с"),
            ("21 ноября.", "ноября"),
            ("Пришёл г-н.", "г-н"),
            ("т.е.", "т.е"),
        ] {
            assert_eq!(
                last(Language::RU_RU, text),
                (core.to_owned(), Dictionary),
                "{text}"
            );
        }
        for (text, core) in [
            ("60 km/h.", "km/h"),
            ("5 \u{b5}s.", "\u{b5}s"),
            ("5 \u{b5}s…", "\u{b5}s"),
            ("e.g.", "e.g"),
        ] {
            assert_eq!(
                last(Language::EN_US, text),
                (core.to_owned(), Dictionary),
                "{text}"
            );
        }
    }

    #[test]
    fn a_roman_numeral_that_is_a_unit_is_a_label_after_a_sign() {
        // Design section 3.6, "Label numbers": the core is a Roman numeral, whatever its kind.
        use FragKind::*;
        let en = Language::EN_US;
        assert_eq!(
            frags(en.clone(), "§ V")[1],
            ("V".to_owned(), Dictionary, true)
        );
        assert_eq!(
            frags(en.clone(), "¶ L")[1],
            ("L".to_owned(), Dictionary, true)
        );
        assert_eq!(frags(en.clone(), "§V")[0], ("§V".to_owned(), Word, true));
        assert_eq!(
            frags(en.clone(), "5 V")[1],
            ("V".to_owned(), Dictionary, false)
        );
    }

    #[test]
    fn a_lone_x_between_numbers_is_times() {
        let all = frags(Language::RU_RU, "10 x 5 \u{445} 3");
        let kinds: Vec<FragKind> = all.iter().map(|f| f.1).collect();
        use FragKind::*;
        assert_eq!(kinds, [Number, Times, Number, Times, Number]);
        let all = frags(Language::RU_RU, "a × b");
        assert_eq!(all[1].1, Unclassified);
    }

    #[test]
    fn the_stop_and_the_wrapper() {
        let t = Typographer::new(Language::EN_US, RuleSet::INTERFACE);
        let text = "(etc...) \"host:8080\",";
        let len = u32::try_from(text.len()).unwrap();
        let atoms = [Atom::Text { start: 0, end: len }];
        let mut clusters = Vec::new();
        let mut tokens = Vec::new();
        lex_window(text, &atoms, &mut clusters, &mut tokens);
        let mut cs = ClassifyScratch::new();
        let pieces = [TextPiece {
            piece: 0,
            start: 0,
            end: len,
        }];
        let mut rec = NoRecord;
        let mut sink = Sink {
            rec: &mut rec,
            pieces: &pieces,
            lens: &[len],
        };
        classify(&t, text, &mut tokens, &mut cs, &mut sink);
        let [first, second] = cs.frags[..] else {
            panic!("{:?}", cs.frags);
        };
        assert!(first.stop);
        assert_eq!(first.kind, FragKind::Dictionary);
        assert!(!second.stop);
        assert_eq!(second.kind, FragKind::Colon);
        // The wrapper stays editable, the core is frozen.
        for (i, tok) in tokens.iter().enumerate() {
            let inside = (second.core_start..second.core_end).contains(&(i as u32));
            assert_eq!(tok.is_frozen(), inside, "{i}");
        }
    }
}
