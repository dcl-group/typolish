//! The pipeline (design section 3.1) and its entry points: the prefilter, ingest and protection,
//! windows, the lexer, the classifier, the seven phases and rendering, with the records of
//! `report`.

pub(crate) mod classify;
pub(crate) mod ingest;
pub(crate) mod lex;
pub(crate) mod phase;
mod plain;
pub(crate) mod record;
mod render;
pub(crate) mod scratch;
mod window;

use std::borrow::Cow;

use crate::report::Records;
use crate::rule::Phase;
use crate::rules::Triggers;
use crate::rules::quotes;
use crate::{DiagnosticKind, Output, Rule, Severity, Slot, Typographer};
use phase::Cx;
use record::{Journal, Loc, NoRecord, Recorder, Sink};
use scratch::Scratch;

/// The phases in their order (design section 3.8).
pub(crate) const PHASES: [Phase; 7] = [
    Phase::Symbols,
    Phase::Ellipsis,
    Phase::Dash,
    Phase::Range,
    Phase::Number,
    Phase::Quotes,
    Phase::Nbsp,
];

/// One input piece without the caller's payload: the call-local vector of piece texts (design
/// section 2.12).
#[derive(Clone, Copy, Debug)]
pub(crate) enum Input<'t> {
    Text(&'t str),
    Slot(Slot),
    Protected(&'t str),
    Markup(&'t str),
    /// A `Piece::BlockMarkup`: written as markup, and a window boundary (design section 3.3).
    Block(&'t str),
}

/// Runs the pipeline over `input` and reports whether any window changed; the edits of the changed
/// windows are left in `scratch.edits` and the table of `Text` pieces in `scratch.pieces`.
fn run<R: Recorder>(
    typographer: &Typographer,
    scratch: &mut Scratch,
    input: &[Input<'_>],
    rec: &mut R,
) -> bool {
    let mut text = core::mem::take(&mut scratch.text);
    let changed = run_on(typographer, scratch, input, &mut text, rec);
    text.clear();
    scratch.text = text;
    changed
}

fn run_on<R: Recorder>(
    typographer: &Typographer,
    scratch: &mut Scratch,
    input: &[Input<'_>],
    buf: &mut String,
    rec: &mut R,
) -> bool {
    let Scratch {
        atoms,
        next_atoms,
        pieces,
        lens,
        delim,
        clusters,
        tokens,
        next,
        classify,
        removed,
        edits,
        marks,
        quotes: quote_work,
        ..
    } = scratch;
    edits.clear();
    let Some(text) = ingest::layout(input, buf, pieces, atoms) else {
        // Positions do not fit `u32`: the input is returned unchanged (design section 3.4).
        return false;
    };
    lens.clear();
    if R::ACTIVE {
        lens.extend(input.iter().map(|item| match item {
            Input::Text(s) | Input::Protected(s) | Input::Markup(s) | Input::Block(s) => s.len(),
            Input::Slot(_) => 0,
        }));
    }
    let backticks = typographer.protects_backticks();
    if backticks {
        ingest::cut_fences(text, atoms, next_atoms, delim);
        core::mem::swap(atoms, next_atoms);
    }
    window::split_windows(text, atoms, next_atoms);
    core::mem::swap(atoms, next_atoms);
    ingest::cut_delimiters(
        text,
        atoms,
        backticks,
        typographer.protection_pairs(),
        next_atoms,
        delim,
    );
    core::mem::swap(atoms, next_atoms);

    let compiled = typographer.compiled();
    let active = compiled.active;
    let report_pairs = R::ACTIVE && active.contains(Rule::QuotesPairs);
    let data = typographer.language().data();
    let pairs = typographer.quote_pairs();
    let single_quotes = typographer.single_quotes();
    let dash = typographer.dash_data();

    let mut sink = Sink { rec, pieces, lens };
    for &(start, end) in delim.unclosed() {
        sink.diagnostic(
            DiagnosticKind::UnclosedDelimiter,
            Severity::Warning,
            Loc::Flat(start, end),
        );
    }
    for window in atoms.split(|atom| matches!(atom, ingest::Atom::Boundary { .. })) {
        lex::lex_window(text, window, clusters, tokens);
        classify::classify(typographer, text, tokens, classify, &mut sink);
        if R::ACTIVE {
            classify::record_protected(tokens, &classify.frags, &mut sink);
        }
        if R::TRACE {
            for f in &classify.frags {
                let start = tokens.get(f.start as usize).map_or(0, |t| t.src.start);
                let end = f
                    .end
                    .checked_sub(1)
                    .and_then(|e| tokens.get(e as usize))
                    .map_or(start, |t| t.src.end);
                let frozen = tokens
                    .get(f.start as usize..f.end as usize)
                    .is_some_and(|ts| ts.iter().any(lex::Tok::is_frozen));
                sink.rec.fragment(f.kind, f.label, frozen, start..end);
            }
        }
        // The phases (design section 3.8): each one pass over a snapshot of the tokens, with the
        // classification of the pass. A phase none of whose rules can match at a kind of token
        // of the window is skipped; edits keep the kinds of tokens and insert only spaces.
        removed.clear();
        let mut changed = false;
        let mut kinds = tokens.iter().fold(0, |set, t| set | t.kind.bit());
        for (phase, rules) in PHASES.iter().zip(&compiled.phases) {
            if rules.anchors() & kinds == 0 {
                continue;
            }
            let frags = &mut classify.frags;
            let mut cx = Cx {
                t: typographer,
                data,
                text,
                toks: tokens,
                frags,
                quotes: &[],
                pairs,
                single_quotes,
                dash,
                overlay: &[],
                hint: core::cell::Cell::new(0),
                sides: core::cell::Cell::new(None),
            };
            if *phase == Phase::Quotes {
                quotes::analyze(&cx, quote_work, marks, &mut sink, report_pairs);
                cx.quotes = marks;
            }
            let outcome = phase::run_phase(&cx, rules, next, removed, &mut sink);
            if outcome.written {
                core::mem::swap(tokens, next);
            }
            if outcome.moved {
                phase::remap(tokens, frags);
                kinds |= lex::Kind::Space.bit();
            }
            changed |= outcome.changed;
        }
        if changed {
            render::collect(tokens, removed, edits);
        }
    }
    !edits.is_empty()
}

/// The bytes that the output kind encodes (design sections 3.2 and 11.6).
fn encoded(output: Output) -> Triggers {
    crate::output::encoded_bytes(output)
}

/// The prefilter (design section 3.2): whether any byte of the texts can make a rule or the
/// output kind change something. `texts` are read by the rules and written by the encoder;
/// `encoded_only` are written by the encoder and read by no rule (protected and markup pieces).
fn wanted<'a>(
    typographer: &Typographer,
    output: Output,
    texts: impl Iterator<Item = &'a str>,
    encoded_only: impl Iterator<Item = &'a str>,
) -> bool {
    scan(
        typographer.compiled().triggers,
        encoded(output),
        texts,
        encoded_only,
    )
}

/// The byte scan of the prefilter: `texts` against the rule triggers and the encoded set,
/// `encoded_only` against the encoded set alone.
fn scan<'a>(
    triggers: Triggers,
    encoded: Triggers,
    mut texts: impl Iterator<Item = &'a str>,
    mut encoded_only: impl Iterator<Item = &'a str>,
) -> bool {
    let set = triggers.union(encoded);
    texts.any(|text| set.any_in(text.as_bytes()))
        || (encoded != Triggers::NONE && encoded_only.any(|text| encoded.any_in(text.as_bytes())))
}

/// `Typographer::typeset_with`.
pub(crate) fn typeset<'a>(
    typographer: &Typographer,
    scratch: &mut Scratch,
    text: &'a str,
    output: Output,
) -> Cow<'a, str> {
    if !wanted(
        typographer,
        output,
        core::iter::once(text),
        core::iter::empty(),
    ) {
        return Cow::Borrowed(text);
    }
    typeset_unfiltered(typographer, scratch, text, output)
}

/// `Typographer::typeset_into`: appends the result to `buf` with no allocation of its own once
/// the scratch memory is warm (design section 2.12).
pub(crate) fn typeset_into(
    typographer: &Typographer,
    scratch: &mut Scratch,
    text: &str,
    output: Output,
    buf: &mut String,
) {
    if !wanted(
        typographer,
        output,
        core::iter::once(text),
        core::iter::empty(),
    ) {
        buf.push_str(text);
        return;
    }
    let changed = run(typographer, scratch, &[Input::Text(text)], &mut NoRecord);
    let data = typographer.language().data();
    if output == Output::Unicode {
        if changed {
            render::render_text_into(text, &scratch.edits, &data, buf);
        } else {
            buf.push_str(text);
        }
        return;
    }
    let input = [Input::Text(text)];
    let view = view(scratch, &input, &data);
    render::render_into_encoded(&view, output, buf);
}

/// What the encoded renderer reads after a run.
fn view<'v, 't>(
    scratch: &'v Scratch,
    input: &'v [Input<'t>],
    data: &'v crate::lang::data::Table<'v>,
) -> render::View<'v, 't> {
    render::View {
        input,
        pieces: &scratch.pieces,
        atoms: &scratch.atoms,
        edits: &scratch.edits,
        data,
    }
}

/// The result for string input after `run`, in the output kind.
fn text_result<'a>(
    typographer: &Typographer,
    scratch: &Scratch,
    text: &'a str,
    changed: bool,
    output: Output,
) -> Cow<'a, str> {
    let data = typographer.language().data();
    if output == Output::Unicode {
        if !changed {
            return Cow::Borrowed(text);
        }
        return render::render_text(text, &scratch.edits, &data);
    }
    let input = [Input::Text(text)];
    let view = view(scratch, &input, &data);
    render::render_text_encoded(text, &view, output)
}

/// The whole pipeline without the prefilter (design section 13.5, property 8).
fn typeset_unfiltered<'a>(
    typographer: &Typographer,
    scratch: &mut Scratch,
    text: &'a str,
    output: Output,
) -> Cow<'a, str> {
    let changed = run(typographer, scratch, &[Input::Text(text)], &mut NoRecord);
    text_result(typographer, scratch, text, changed, output)
}

/// `Typographer::typeset_template_with`: returns the texts in the n + 1 form and whether any of
/// them changed.
///
/// `slot_count` is the number of `Input::Slot` items, so that the vector of texts is allocated
/// once; a text merged from several non-empty pieces is one string of their total length (design
/// section 2.12).
pub(crate) fn typeset_template<'t>(
    typographer: &Typographer,
    scratch: &mut Scratch,
    input: &[Input<'t>],
    slot_count: usize,
    output: Output,
) -> (Vec<Cow<'t, str>>, bool) {
    let texts = input.iter().filter_map(|item| match item {
        Input::Text(text) => Some(*text),
        _ => None,
    });
    // Markup is written as is in Unicode and HTML output and encoded in escaped output (design
    // sections 3.2, 11).
    let escaped = matches!(output, Output::Escaped(_));
    let encoded_only = input.iter().filter_map(move |item| match item {
        Input::Protected(text) => Some(*text),
        Input::Markup(text) | Input::Block(text) if escaped => Some(*text),
        _ => None,
    });
    if !wanted(typographer, output, texts, encoded_only) {
        // No byte that a rule reads or the output kind encodes: every text is its merge. No run
        // happened, so nothing of the scratch is read.
        let data = typographer.language().data();
        return render::render_template(input, &[], &[], slot_count, &data);
    }
    let changed = run(typographer, scratch, input, &mut NoRecord);
    template_texts(
        typographer,
        scratch,
        input,
        slot_count,
        changed,
        output,
        &mut NoRecord,
    )
}

/// The texts of a template in the n + 1 form after `run`, in the output kind, and whether any of
/// them changed.
fn template_texts<'t, R: Recorder>(
    typographer: &Typographer,
    scratch: &Scratch,
    input: &[Input<'t>],
    slot_count: usize,
    changed: bool,
    output: Output,
    rec: &mut R,
) -> (Vec<Cow<'t, str>>, bool) {
    let data = typographer.language().data();
    if output == Output::Unicode {
        let edits = if changed { &scratch.edits[..] } else { &[] };
        return render::render_template(input, &scratch.pieces, edits, slot_count, &data);
    }
    let view = view(scratch, input, &data);
    render::render_template_encoded(&view, output, slot_count, rec)
}

/// `Typographer::report`: the whole pipeline with the journal, without the prefilter (design
/// section 3.2).
pub(crate) fn report<'a>(
    typographer: &Typographer,
    scratch: &mut Scratch,
    text: &'a str,
    output: Output,
) -> (Cow<'a, str>, Records) {
    let mut journal = Journal::default();
    let changed = run(typographer, scratch, &[Input::Text(text)], &mut journal);
    let result = text_result(typographer, scratch, text, changed, output);
    (result, journal.finish())
}

/// `Typographer::report_template`.
pub(crate) fn report_template<'t>(
    typographer: &Typographer,
    scratch: &mut Scratch,
    input: &[Input<'t>],
    slot_count: usize,
    output: Output,
) -> (Vec<Cow<'t, str>>, bool, Records) {
    let mut journal = Journal::default();
    let changed = run(typographer, scratch, input, &mut journal);
    let (texts, changed) = template_texts(
        typographer,
        scratch,
        input,
        slot_count,
        changed,
        output,
        &mut journal,
    );
    (texts, changed, journal.finish())
}

/// `Typographer::lint` and `Typographer::lint_template` (design section 12.3): the whole pipeline
/// with the `Diagnostics` recorder and no rendering (so no `NowrapFallback`), plus a
/// `TypographicChar` for every character that `to_plain` changes, in the order of design section
/// 12.4.
pub(crate) fn lint(
    typographer: &Typographer,
    scratch: &mut Scratch,
    input: &[Input<'_>],
) -> Vec<crate::Diagnostic> {
    let mut diagnostics = record::Diagnostics::default();
    let _ = run(typographer, scratch, input, &mut diagnostics);
    for (piece, range, found) in plain::changed_chars(typographer, scratch, input) {
        diagnostics.diagnostic(crate::Diagnostic::new(
            DiagnosticKind::TypographicChar { found },
            Severity::Info,
            None,
            piece,
            range,
        ));
    }
    diagnostics.finish()
}

/// `Typographer::to_plain` (design section 12.1).
pub(crate) fn to_plain<'a>(
    typographer: &Typographer,
    scratch: &mut Scratch,
    text: &'a str,
) -> Cow<'a, str> {
    plain::to_plain(typographer, scratch, text)
}

/// `Typographer::to_plain_template`: the texts in the n + 1 form and whether any changed.
pub(crate) fn to_plain_template<'t>(
    typographer: &Typographer,
    scratch: &mut Scratch,
    input: &[Input<'t>],
    slot_count: usize,
) -> (Vec<Cow<'t, str>>, bool) {
    plain::to_plain_template(typographer, scratch, input, slot_count)
}

// The generators of design section 13.5; the file allows `dead_code` itself.
#[cfg(test)]
#[path = "../../tests/common/strategies.rs"]
mod strategies;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{DashStyle, EscapeStyle, HtmlStyle, Language, Piece, RuleSet, SingleQuotes};
    use proptest::prelude::*;

    const LANGUAGES: [Language; 3] = [Language::RU_RU, Language::EN_US, Language::COMMON];

    /// Every output kind: `Unicode`, each HTML style plain and with each nowrap form, each escaped
    /// syntax with the typographic set and with `non_ascii()` (design section 13.5, property 8).
    fn outputs() -> Vec<Output> {
        let html = [
            HtmlStyle::NAMED,
            HtmlStyle::DECIMAL,
            HtmlStyle::HEX,
            HtmlStyle::CHARACTERS,
        ];
        let escaped = [
            EscapeStyle::JS,
            EscapeStyle::JS_HEX,
            EscapeStyle::JS_CODE_POINT,
            EscapeStyle::JSON,
            EscapeStyle::RUST,
            EscapeStyle::CSS,
        ];
        let mut out = vec![Output::Unicode];
        for style in html {
            out.push(Output::Html(style));
            out.push(Output::Html(style.nowrap_class("nowrap")));
            out.push(Output::Html(style.nowrap_style()));
        }
        for style in escaped {
            out.push(Output::Escaped(style));
            out.push(Output::Escaped(style.non_ascii()));
        }
        out
    }

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(512))]

        /// Property 8 of design section 13.5: the prefilter changes no result.
        #[test]
        fn the_prefilter_changes_no_result(
            text in strategies::any_text(), lang in 0usize..3, set in strategies::rule_set(),
        ) {
            let t = Typographer::new(LANGUAGES[lang].clone(), set);
            for output in outputs() {
                let mut scratch = Scratch::new();
                let filtered = typeset(&t, &mut scratch, &text, output);
                let unfiltered = typeset_unfiltered(&t, &mut scratch, &text, output);
                prop_assert_eq!(&filtered, &unfiltered);
                // `typeset_into` writes the same result after what `buf` holds.
                let mut buf = String::from("> ");
                typeset_into(&t, &mut scratch, &text, output, &mut buf);
                prop_assert_eq!(buf.strip_prefix("> "), Some(filtered.as_ref()));
                prop_assert_eq!(
                    matches!(filtered, Cow::Borrowed(_)),
                    matches!(unfiltered, Cow::Borrowed(_))
                );
            }
        }

        #[test]
        fn the_prefilter_changes_no_template(
            parts in strategies::pieces(), lang in 0usize..3, set in strategies::rule_set(),
        ) {
            let t = Typographer::new(LANGUAGES[lang].clone(), set);
            let pieces = strategies::borrow(&parts);
            for output in outputs() {
                let filtered = t.typeset_template(pieces.iter().copied(), output);
                let input: Vec<Input<'_>> = pieces
                    .iter()
                    .map(|p| match *p {
                        Piece::Text(s) => Input::Text(s),
                        Piece::Slot(_, slot) => Input::Slot(slot),
                        Piece::Protected(s) => Input::Protected(s),
                        Piece::Markup(s) => Input::Markup(s),
                        Piece::BlockMarkup(s) => Input::Block(s),
                    })
                    .collect();
                let slots = input.iter().filter(|i| matches!(i, Input::Slot(_))).count();
                let mut scratch = Scratch::new();
                let changed = run(&t, &mut scratch, &input, &mut NoRecord);
                let (texts, changed) =
                    template_texts(&t, &scratch, &input, slots, changed, output, &mut NoRecord);
                prop_assert_eq!(filtered.texts(), texts.as_slice());
                prop_assert_eq!(filtered.changed(), changed);
            }
        }
    }

    /// What a run records for the invariants I2 and I4 (design sections 4.3 and 13.5, property 3):
    /// the fragments of the classification, the matches of the phases and the changes.
    #[derive(Debug, Default)]
    struct Trace {
        /// Kind, `LABEL`, `FROZEN`, byte range.
        frags: Vec<(classify::FragKind, bool, bool, u32, u32)>,
        /// Rule, byte range from the anchor to the last consumed token, `consume`, edits.
        matches: Vec<Matched>,
        /// Byte range and length of the replacement.
        changes: Vec<(u32, u32, usize)>,
        /// The replacement of each change.
        replacements: Vec<String>,
        /// Whether each change inserts or removes a space (`trace` fills it).
        spaced: Vec<bool>,
    }

    impl Recorder for Trace {
        const ACTIVE: bool = true;
        const TRACE: bool = true;

        fn change(&mut self, change: crate::report::Change) {
            let range = change.range();
            self.changes.push((
                u32::try_from(range.start).unwrap_or(u32::MAX),
                u32::try_from(range.end).unwrap_or(u32::MAX),
                change.replacement().len(),
            ));
            self.replacements.push(change.replacement().to_owned());
        }

        fn protected(&mut self, _span: crate::report::ProtectedSpan) {}

        fn diagnostic(&mut self, _diagnostic: crate::report::Diagnostic) {}

        fn fragment(
            &mut self,
            kind: classify::FragKind,
            label: bool,
            frozen: bool,
            range: core::ops::Range<u32>,
        ) {
            self.frags
                .push((kind, label, frozen, range.start, range.end));
        }

        fn matched(&mut self, rule: Rule, range: core::ops::Range<u32>, consume: u8, edits: bool) {
            self.matches
                .push((rule, range.start, range.end, consume, edits));
        }
    }

    /// A recorded match: rule, byte range from the anchor to the last consumed token, `consume`,
    /// edits.
    type Matched = (Rule, u32, u32, u8, bool);

    /// Where a position of x lies in f(x): `end` maps the exclusive end of a range, so an
    /// insertion there is after it.
    fn map(changes: &[(u32, u32, usize)], p: u32, end: bool) -> i64 {
        let shift: i64 = changes
            .iter()
            .filter(|&&(s, e, _)| e < p || (e == p && (!end || s < e)))
            .map(|&(s, e, len)| len as i64 - i64::from(e - s))
            .sum();
        i64::from(p) + shift
    }

    /// A closed byte interval.
    type Interval = (i64, i64);

    /// The structural changes of I2 (an inserted or a removed space, also where a replacement
    /// drops or adds one, as the percent setting does) as closed byte intervals of x and of f(x).
    fn structural(tr: &Trace) -> (Vec<Interval>, Vec<Interval>) {
        let mut sorted: Vec<_> = tr
            .changes
            .iter()
            .zip(&tr.spaced)
            .map(|(&(s, e, len), &spaced)| (s, e, len, spaced))
            .collect();
        sorted.sort_unstable();
        let (mut x, mut y) = (Vec::new(), Vec::new());
        let mut shift = 0i64;
        for (s, e, len, spaced) in sorted {
            let ys = i64::from(s) + shift;
            if s == e || len == 0 || spaced {
                x.push((i64::from(s), i64::from(e)));
                y.push((ys, ys + len as i64));
            }
            shift += len as i64 - i64::from(e - s);
        }
        (x, y)
    }

    fn touches(intervals: &[Interval], start: i64, end: i64) -> bool {
        intervals.iter().any(|&(s, e)| start <= e && s <= end)
    }

    /// Whether a match on x lies inside a fragment that an inserted or a removed space split or
    /// merged (`structural`), or is the match of a short word or a particle whose trigger word is
    /// such a fragment: its tokens or its trigger word are in the merged fragment, so it may not
    /// recur (design section 4.3, I4: `5–10 %` → `5–10%`, where `range.digits` no longer reads the
    /// `–`; `nbsp.short-words` on `the` in `Moscow—the⍽capital`).
    fn inside_a_merge(tx: &Trace, sx: &[Interval], m: &Matched) -> bool {
        let merged = |f: &&(classify::FragKind, bool, bool, u32, u32)| {
            touches(sx, i64::from(f.3), i64::from(f.4))
        };
        let inside = tx
            .frags
            .iter()
            .filter(merged)
            .any(|f| f.3 <= m.1 && m.2 <= f.4);
        // The trigger word of a short word glues the space after it, a particle the space
        // before it (design section 8.7).
        let before = || tx.frags.iter().filter(|f| f.4 <= m.1).max_by_key(|f| f.4);
        let after = || tx.frags.iter().filter(|f| f.3 >= m.2).min_by_key(|f| f.3);
        let trigger = match m.0 {
            Rule::NbspShortWords | Rule::NbspShortLength => before().is_some_and(|f| merged(&f)),
            Rule::NbspParticles => after().is_some_and(|f| merged(&f)),
            _ => false,
        };
        inside || trigger
    }

    /// Whether a dash match on x has an operand that a removed space merged into a `Joined`
    /// fragment of f(x). A `Joined` core is not an operand (design section 8.4), so such a match
    /// does not recur: its edit stays (design section 4.3, I3 and I4), and a refusal stays refused.
    fn beside_a_merge(tx: &Trace, ty: &Trace, sy: &[Interval], m: &Matched) -> bool {
        if !matches!(m.0, Rule::DashSpacedHyphen | Rule::DashDoubleHyphen) {
            return false;
        }
        // The operands: the last fragment before the anchor and the first one after the match.
        let left = tx
            .frags
            .iter()
            .filter(|f| f.4 <= m.1)
            .max_by_key(|f| f.4)
            .map(|f| map(&tx.changes, f.4, true));
        let right = tx
            .frags
            .iter()
            .filter(|f| f.3 >= m.2)
            .min_by_key(|f| f.3)
            .map(|f| map(&tx.changes, f.3, false));
        [left, right].into_iter().flatten().any(|p| {
            ty.frags.iter().any(|g| {
                let (s, e) = (i64::from(g.3), i64::from(g.4));
                g.0 == classify::FragKind::Joined && s <= p && p <= e && touches(sy, s, e)
            })
        })
    }

    fn trace(t: &Typographer, scratch: &mut Scratch, text: &str) -> (Trace, String) {
        let mut tr = Trace::default();
        let changed = run(t, scratch, &[Input::Text(text)], &mut tr);
        let spaces = |s: &str| {
            s.chars()
                .filter(|c| matches!(c, ' ' | '\u{a0}' | '\u{202f}'))
                .count()
        };
        tr.spaced = tr
            .changes
            .iter()
            .zip(&tr.replacements)
            .map(|(&(s, e, _), r)| {
                spaces(text.get(s as usize..e as usize).unwrap_or("")) != spaces(r)
            })
            .collect();
        let out = if changed {
            render::render_text(text, &scratch.edits, &t.language().data()).into_owned()
        } else {
            text.to_owned()
        };
        (tr, out)
    }

    #[test]
    fn the_trace_sees_fragments_matches_and_changes() {
        // The I2 and I4 property reads what it compares: on `Москва - столица...` the dash and the
        // ellipsis match with edits on the first pass and with empty edits on the second.
        let t = Typographer::new(Language::RU_RU, RuleSet::PROSE);
        let mut scratch = Scratch::new();
        let (tx, out) = trace(&t, &mut scratch, "Москва - столица...");
        let (ty, _) = trace(&t, &mut scratch, &out);
        assert!(tx.frags.len() >= 3, "{:?}", tx.frags);
        assert_eq!(tx.frags.len(), ty.frags.len());
        for rule in [Rule::DashSpacedHyphen, Rule::EllipsisDots] {
            assert!(tx.matches.iter().any(|m| m.0 == rule && m.4), "{rule:?}");
            assert!(ty.matches.iter().any(|m| m.0 == rule && !m.4), "{rule:?}");
        }
        assert!(!tx.changes.is_empty());
        // A removed space is structural: `Moscow - the` merges into one `Joined` fragment.
        let t = Typographer::new(Language::EN_US, RuleSet::PROSE);
        let (tx, _) = trace(&t, &mut scratch, "Moscow - the capital");
        let (sx, sy) = structural(&tx);
        assert_eq!(sx.len(), 2, "{:?}", tx.changes);
        assert_eq!(sy.len(), 2);
    }

    /// I2 and I4 of design section 4.3 (design section 13.5, property 3) on `text` with `t`,
    /// whose rule set is `set`. I2: outside the fragments that an inserted or a removed space split
    /// or merged, the fragments of f(x) have the kinds, flags and extents of those of x. I4: no
    /// match on f(x) has an edit, and every match on x outside those fragments recurs on f(x) with
    /// the same rule (`dash.double-hyphen` as `dash.spaced-hyphen`) and `consume`, except a dash
    /// match whose operand a removed space merged into a `Joined` fragment (design section 8.4: a
    /// `Joined` core is not an operand); the quote phase has its own proof (design section 6.11)
    /// and is left out of the recurrence.
    /// Whether a straight `'` before the first letter or digit of the fragment `f` of x became
    /// the opening quote of a pair, `‘` or `“` in f(x). The classification reads a straight `'`
    /// before a word as a letter of the word (an elision, `'cause`, `'tis`) and a typed opener as
    /// no part of its core, so the kind of such a fragment may differ between the passes
    /// (`“'cause''”` is `Dictionary`, `“‘cause’'”` a `Word`; `'St.'` a `Word`, `“St.”`
    /// `Dictionary`); its extent, its flags and every other fragment do not (design section 4.3,
    /// I2).
    fn straight_opens_a_pair(
        changes: &[(u32, u32, usize)],
        text: &str,
        out: &str,
        f: &(classify::FragKind, bool, bool, u32, u32),
    ) -> bool {
        let Some(frag) = text.get(f.3 as usize..f.4 as usize) else {
            return false;
        };
        frag.char_indices()
            .take_while(|(_, c)| !c.is_alphanumeric())
            .filter(|&(_, c)| c == '\'')
            .any(|(i, _)| {
                let at = map(changes, f.3 + i as u32, false);
                usize::try_from(at)
                    .ok()
                    .and_then(|at| out.get(at..))
                    .is_some_and(|rest| rest.starts_with(['‘', '“']))
            })
    }

    /// Whether a dash match on x has a straight `'` as its right operand that the quote phase
    /// wrote as `’`. The dash rules read such a quote by its local role (design section 6.4): an
    /// opener after the space, so the right side holds; the quote phase made it an apostrophe or a
    /// closer, and on f(x) the `’` after a space has no role and no core, so the match does not
    /// recur. The dash is written on x, so f(x) is a fixed point (design section 4.3, I4).
    fn before_an_opener_written_as_apostrophe(
        changes: &[(u32, u32, usize)],
        text: &str,
        out: &str,
        m: &Matched,
    ) -> bool {
        // The right operand: the first character after the spaces after the match, which ends
        // after the dash where it is refused and after the space where it is not.
        let Some(rest) = text.get(m.2 as usize..) else {
            return false;
        };
        let gap = rest.len() - rest.trim_start_matches(' ').len();
        let at = map(changes, m.2 + gap as u32, false);
        matches!(m.0, Rule::DashSpacedHyphen | Rule::DashDoubleHyphen)
            && rest[gap..].starts_with('\'')
            && usize::try_from(at)
                .ok()
                .and_then(|at| out.get(at..))
                .is_some_and(|rest| rest.starts_with('’'))
    }

    fn i2_and_i4(t: &Typographer, set: RuleSet, text: &str) -> Result<(), TestCaseError> {
        let mut scratch = Scratch::new();
        let (tx, out) = trace(t, &mut scratch, text);
        let (ty, _) = trace(t, &mut scratch, &out);
        let (sx, sy) = structural(&tx);

        let before: Vec<_> = tx
            .frags
            .iter()
            .filter(|f| !touches(&sx, i64::from(f.3), i64::from(f.4)))
            .map(|f| {
                let start = map(&tx.changes, f.3, false);
                let kind = (!straight_opens_a_pair(&tx.changes, text, &out, f)).then_some(f.0);
                (kind, f.1, f.2, start, map(&tx.changes, f.4, true))
            })
            .collect();
        // The fragments of f(x) whose kind `straight_opens_a_pair` leaves out.
        let opened: Vec<i64> = before
            .iter()
            .filter(|f| f.0.is_none())
            .map(|f| f.3)
            .collect();
        let after: Vec<_> = ty
            .frags
            .iter()
            .filter(|f| !touches(&sy, i64::from(f.3), i64::from(f.4)))
            .map(|f| {
                let start = i64::from(f.3);
                let kind = (!opened.contains(&start)).then_some(f.0);
                (kind, f.1, f.2, start, i64::from(f.4))
            })
            .collect();
        prop_assert_eq!(before, after, "I2: {:?} -> {:?}", text, out);

        for m in &ty.matches {
            prop_assert!(
                !m.4,
                "I4: {:?} has an edit on {:?} (from {:?})",
                m,
                out,
                text
            );
        }
        let quotes = [
            Rule::QuotesApostrophe,
            Rule::QuotesPairs,
            Rule::QuotesPrimes,
        ];
        // The dash that `dash.double-hyphen` writes is matched by `dash.spaced-hyphen`, never by
        // itself (design section 8.4): with `dash.spaced-hyphen` off, no rule matches it on
        // f(x).
        let spaced = set.contains(Rule::DashSpacedHyphen);
        for m in tx
            .matches
            .iter()
            .filter(|m| !quotes.contains(&m.0))
            .filter(|m| spaced || m.0 != Rule::DashDoubleHyphen)
            .filter(|m| !touches(&sx, i64::from(m.1), i64::from(m.2)))
            .filter(|m| !inside_a_merge(&tx, &sx, m))
            .filter(|m| !beside_a_merge(&tx, &ty, &sy, m))
            .filter(|m| !before_an_opener_written_as_apostrophe(&tx.changes, text, &out, m))
        {
            let rule = if m.0 == Rule::DashDoubleHyphen {
                Rule::DashSpacedHyphen
            } else {
                m.0
            };
            let (s, e) = (map(&tx.changes, m.1, false), map(&tx.changes, m.2, true));
            prop_assert!(
                ty.matches.iter().any(|n| (n.0 == m.0 || n.0 == rule)
                    && i64::from(n.1) == s
                    && i64::from(n.2) == e
                    && n.3 == m.3),
                "I4: {:?} on {:?} does not recur on {:?}: {:?}; changes {:?}",
                m,
                text,
                out,
                ty.matches,
                tx.changes
            );
        }
        Ok(())
    }

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(1024))]

        /// `i2_and_i4` with the stage 2 rules (no `nbsp.*`) over `any_text`.
        #[test]
        fn i2_and_i4_hold_on_the_classification_and_the_matches(
            text in strategies::any_text(), lang in 0usize..3, set in strategies::rule_set(),
            mode in 0usize..3,
        ) {
            let set: RuleSet = set.iter().filter(|r| !r.id().starts_with("nbsp.")).collect();
            let mode = [SingleQuotes::NestedOnly, SingleQuotes::Everywhere, SingleQuotes::Never][mode];
            let t = Typographer::builder(LANGUAGES[lang].clone(), set)
                .single_quotes(mode)
                .build()
                .unwrap();
            i2_and_i4(&t, set, &text)?;
        }
    }

    /// The typographer of a configuration of the grammar with its rule set (`all` false: without
    /// the `nbsp.*` rules), and that set.
    fn grammar_typographer(
        config: &strategies::GrammarConfig,
        all: bool,
    ) -> (Typographer, RuleSet) {
        let set: RuleSet = config
            .set
            .iter()
            .filter(|r| all || !r.id().starts_with("nbsp."))
            .collect();
        let builder = Typographer::builder(config.language(), set)
            .single_quotes(config.single)
            .percent(config.percent);
        let t = match config.dash {
            Some(style) => builder.dash_style(style),
            None => builder,
        }
        .build()
        .unwrap();
        (t, set)
    }

    #[test]
    fn i2_and_i4_hold_on_the_rows_of_the_grammar() {
        // Sentences of the grammar that broke `i2_and_i4` (design section 4.3): an elision or a
        // word after a straight `'` that pairs, a dash match next to a straight `'` before a
        // number that becomes `’`, and one next to a straight `'` that keeps its text. Every `SingleQuotes` mode, Prose, Interface and all rules.
        use strategies::GrammarConfig;
        for (lang, dash, text) in [
            (3, None, "! s: S -- 'tis'"),
            (
                1,
                Some(DashStyle::Closed),
                "'don't S a file rock'n'roll 'cause'",
            ),
            (1, None, "“'cause''”"),
            (1, None, "'St.’"),
            (
                1,
                Some(DashStyle::SpacedGlued),
                "B “” S an U.S. — '10-20?!’",
            ),
            (
                3,
                None,
                "?-( - rock'n'roll) 1941-1945 km file — '27 % host:8080’",
            ),
            // A straight `'` after a dash whose opening role the placed quotes would drop: the
            // window stays straight, so the dash match recurs (design section 6.6).
            (0, None, "- ''.-'"),
            (1, None, "word - ''.-'"),
            (3, Some(DashStyle::Spaced), "word — '...-'' \" - \"'"),
        ] {
            for single in strategies::GRAMMAR_SINGLE_QUOTES {
                for set in [RuleSet::PROSE, RuleSet::INTERFACE, RuleSet::ALL] {
                    let config = GrammarConfig {
                        lang,
                        set,
                        dash,
                        single,
                        percent: crate::PercentSpacing::AsWritten,
                    };
                    let (t, set) = grammar_typographer(&config, false);
                    if let Err(e) = i2_and_i4(&t, set, text) {
                        panic!("{} {text:?}: {e}", config.describe());
                    }
                }
            }
        }
    }

    /// How many fragments and matches of x each exemption of `i2_and_i4` leaves out:
    /// `straight_opens_a_pair`, `before_an_opener_written_as_apostrophe` and `inside_a_merge`.
    fn exempted(t: &Typographer, text: &str) -> (usize, usize, usize) {
        let mut scratch = Scratch::new();
        let (tx, out) = trace(t, &mut scratch, text);
        let (sx, _) = structural(&tx);
        let opened = tx
            .frags
            .iter()
            .filter(|f| straight_opens_a_pair(&tx.changes, text, &out, f))
            .count();
        let apostrophe = tx
            .matches
            .iter()
            .filter(|m| before_an_opener_written_as_apostrophe(&tx.changes, text, &out, m))
            .count();
        let merged = tx
            .matches
            .iter()
            .filter(|m| inside_a_merge(&tx, &sx, m))
            .count();
        (opened, apostrophe, merged)
    }

    #[test]
    fn the_exemptions_of_i2_and_i4_name_only_their_cases() {
        // Each exemption of `i2_and_i4` (design section 4.3) leaves out the fragments and the
        // matches it names, and nothing on the rows next to them.
        use crate::PercentSpacing::{AsWritten, Attached};
        use SingleQuotes::{Everywhere, NestedOnly, Never};
        let en = Language::EN_US;
        let ru = Language::RU_RU;
        let no_nbsp: RuleSet = RuleSet::PROSE
            .iter()
            .filter(|r| !r.id().starts_with("nbsp."))
            .collect();
        for (language, set, single, percent, text, expected) in [
            // A straight `'` before a word that the quote phase made an opener.
            (&en, no_nbsp, Everywhere, AsWritten, "x 'tis' y", (1, 0, 0)),
            (&en, no_nbsp, NestedOnly, AsWritten, "“'cause''”", (1, 0, 0)),
            (&en, no_nbsp, Everywhere, AsWritten, "'St.'", (1, 0, 0)),
            // ... and not where it became `’`, stayed straight, stands inside a word, or is `"`.
            (&en, no_nbsp, Never, AsWritten, "x 'tis' y", (0, 0, 0)),
            (&en, no_nbsp, NestedOnly, AsWritten, "x 'tis y", (0, 0, 0)),
            (
                &en,
                no_nbsp,
                NestedOnly,
                AsWritten,
                "don't rock'n'roll",
                (0, 0, 0),
            ),
            (
                &en,
                no_nbsp,
                NestedOnly,
                AsWritten,
                "x \"tis\" y",
                (0, 0, 0),
            ),
            // A dash before a straight `'` that the quote phase wrote as `’`.
            (&en, no_nbsp, Never, AsWritten, "S -- 'tis'", (0, 1, 0)),
            (&en, no_nbsp, Never, AsWritten, "I - '10 x 5'", (0, 1, 0)),
            // ... and not before a `'` that became an opener, before a word or `"`, or after `'`.
            (&en, no_nbsp, Everywhere, AsWritten, "S -- 'tis'", (1, 0, 0)),
            (&en, no_nbsp, Never, AsWritten, "S -- tis", (0, 0, 0)),
            (&en, no_nbsp, Never, AsWritten, "S -- \"tis\"", (0, 0, 0)),
            (&en, no_nbsp, Never, AsWritten, "'tis' -- S", (0, 0, 0)),
            // A match inside a merged fragment, or a no-break match triggered by one.
            (
                &ru,
                RuleSet::PROSE,
                NestedOnly,
                Attached,
                "5–10 % - норма",
                (0, 0, 1),
            ),
            (
                &en,
                RuleSet::PROSE,
                NestedOnly,
                AsWritten,
                "Moscow - the capital",
                (0, 0, 1),
            ),
            // ... and not where nothing merges.
            (
                &ru,
                RuleSet::PROSE,
                NestedOnly,
                AsWritten,
                "5–10 % - норма",
                (0, 0, 0),
            ),
            (
                &en,
                RuleSet::PROSE,
                NestedOnly,
                AsWritten,
                "Moscow - 5 capital",
                (0, 0, 0),
            ),
        ] {
            let t = Typographer::builder(language.clone(), set)
                .single_quotes(single)
                .percent(percent)
                .build()
                .unwrap();
            assert_eq!(
                exempted(&t, text),
                expected,
                "{language} {single:?} {text:?}"
            );
        }
    }

    #[test]
    fn i2_and_i4_hold_on_the_splits_and_merges_with_every_rule() {
        // The splits and merges of design section 4.3, I2, with the `nbsp.*` rules on, which the
        // properties over `any_text` and the grammar leave out: Prose, Interface and all rules,
        // every percent setting.
        for (lang, text) in [
            (0, "т.е. 5"),
            (0, "Школа №5 г. Москвы"),
            (0, "§5 ч. 2"),
            (0, "¶IV"),
            (0, "и т.д..."),
            (0, "Слово - т.е."),
            (0, "5 - т.е."),
            (0, "т.е. - 5"),
            (0, "100%"),
            (0, "100 %"),
            (0, "5–10%"),
            (0, "5–10 % - норма"),
            (0, "Рост - 5–10%"),
            (0, "1,5-2,5 т.е. так"),
            (0, "5 кг"),
            (1, "Moscow - the capital"),
            (1, "100 %"),
            (1, "No. 5"),
        ] {
            for set in [RuleSet::PROSE, RuleSet::INTERFACE, RuleSet::ALL] {
                for percent in [
                    crate::PercentSpacing::AsWritten,
                    crate::PercentSpacing::Language,
                    crate::PercentSpacing::Attached,
                    crate::PercentSpacing::Spaced,
                ] {
                    for dash in [None, Some(DashStyle::Closed)] {
                        let config = strategies::GrammarConfig {
                            lang,
                            set,
                            dash,
                            single: SingleQuotes::NestedOnly,
                            percent,
                        };
                        let (t, set) = grammar_typographer(&config, true);
                        if let Err(e) = i2_and_i4(&t, set, text) {
                            panic!("{} {text:?}: {e}", config.describe());
                        }
                    }
                }
            }
        }
    }

    #[test]
    #[ignore = "long run of design section 13.5, property 3 over the grammar"]
    fn i2_and_i4_hold_over_the_grammar_long() {
        // 20 000 sentences for each configuration of `grammar_long_configurations`.
        strategies::run_grammar_long(20_000, 0x7970_6f6c_6973_6822, |config, sentence| {
            let (t, set) = grammar_typographer(config, false);
            i2_and_i4(&t, set, &sentence.text)?;
            i2_and_i4(&t, set, &sentence.plain)
        });
    }

    #[test]
    fn i2_and_i4_hold_over_the_grammar() {
        // `i2_and_i4` over the structured grammar of design section 13.5 in its five languages,
        // on the sentence with its typed glyphs and on the plain one, with fixed seeds.
        for lang in 0..strategies::GRAMMAR_LANGUAGES {
            strategies::run_grammar(
                lang,
                512,
                0x7970_6f6c_6973_6821 + lang as u64,
                |config, sentence| {
                    let (t, set) = grammar_typographer(config, false);
                    i2_and_i4(&t, set, &sentence.text)?;
                    i2_and_i4(&t, set, &sentence.plain)
                },
            );
        }
    }

    #[test]
    fn a_dash_beside_a_merged_fragment_does_not_recur() {
        // I4 of design section 4.3: the closed style merges `details - a` into the `Joined`
        // fragment `details—a`, which is not an operand (design section 8.4), so the next dash is
        // matched on x and by no rule on f(x). Its edit stays and a refusal stays refused; the
        // output is a fixed point. The first input is the one the property once shrank to.
        let set: RuleSet = RuleSet::ALL
            .iter()
            .filter(|r| !r.id().starts_with("nbsp."))
            .collect();
        let t = Typographer::new(Language::EN_US, set);
        let mut scratch = Scratch::new();
        for (text, expected, edited) in [
            (
                " - т.е.Follow the news on the site, and for details - a - т.е.sk?.. the organizers.",
                " - т.е.Follow the news on the site, and for details—a — т.е.sk?.. the organizers.",
                true,
            ),
            ("details - a - т.е.sk", "details—a — т.е.sk", true),
            ("details - a - b.c", "details—a — b.c", true),
            ("word - word - 5", "word—word — 5", true),
            ("word - a - b", "word—a - b", false),
            ("5 - x - Moscow", "5 - x—Moscow", false),
        ] {
            let (tx, out) = trace(&t, &mut scratch, text);
            assert_eq!(out, expected, "{text:?}");
            let (ty, again) = trace(&t, &mut scratch, &out);
            assert_eq!(again, out, "{text:?}");
            assert!(ty.matches.iter().all(|m| !m.4), "{out:?}: {:?}", ty.matches);
            let (sx, sy) = structural(&tx);
            let beside: Vec<_> = tx
                .matches
                .iter()
                .filter(|m| !touches(&sx, i64::from(m.1), i64::from(m.2)))
                .filter(|m| beside_a_merge(&tx, &ty, &sy, m))
                .collect();
            assert_eq!(beside.len(), 1, "{text:?}: {:?}", tx.matches);
            assert!(beside.iter().all(|m| m.4 == edited), "{text:?}: {beside:?}");
            let dashes = |tr: &Trace| {
                tr.matches
                    .iter()
                    .filter(|m| matches!(m.0, Rule::DashSpacedHyphen | Rule::DashDoubleHyphen))
                    .count()
            };
            assert_eq!(dashes(&tx), 2, "{text:?}");
            assert_eq!(dashes(&ty), 0, "{out:?}");
        }
        // Without the merge the same dash recurs with an empty edit.
        let (tx, out) = trace(&t, &mut scratch, "x - a - т.е.sk");
        let (ty, _) = trace(&t, &mut scratch, &out);
        let (_, sy) = structural(&tx);
        assert!(tx.matches.iter().all(|m| !beside_a_merge(&tx, &ty, &sy, m)));
        assert!(
            ty.matches
                .iter()
                .any(|m| m.0 == Rule::DashSpacedHyphen && !m.4),
            "{out:?}"
        );
    }

    #[test]
    fn dashes_next_to_a_prime_and_a_minus_recur_on_the_second_pass() {
        // I4 of design section 4.3: the prime that `quotes.primes` writes reads as the closing
        // quote it was typed as, and `−` as the sign `-` of a number (design section 8.4); both
        // inputs once failed the property above.
        let set: RuleSet = RuleSet::ALL
            .iter()
            .filter(|r| !r.id().starts_with("nbsp."))
            .collect();
        let t = Typographer::new(Language::RU_RU, set);
        let mut scratch = Scratch::new();
        for text in ["5'10\" - т.е.", "-1,5 -- т.е.\u{7f}"] {
            let (tx, out) = trace(&t, &mut scratch, text);
            let (ty, _) = trace(&t, &mut scratch, &out);
            let dashes = |tr: &Trace| {
                tr.matches
                    .iter()
                    .filter(|m| matches!(m.0, Rule::DashSpacedHyphen | Rule::DashDoubleHyphen))
                    .count()
            };
            assert_eq!(dashes(&tx), 1, "{text:?}");
            assert_eq!(dashes(&ty), 1, "{text:?} -> {out:?}");
            assert!(ty.matches.iter().all(|m| !m.4), "{out:?}");
        }
    }

    /// Design section 3.11: the three recorders see the same run; `Diagnostics` keeps the
    /// diagnostics of `Journal`, and `NoRecord` gives the result of `Journal`.
    #[test]
    fn the_recorders_agree() {
        let inputs = [
            "Он сказал \"привет.",
            "«a \"b» c\"",
            "Слово \" в тексте и 10 - 20 штук, Мoсква, don`t",
            "He said - \"a cat\" at 5'10\".",
            "- Привет!\n- Пока... 1941-45 гг.",
        ];
        for language in LANGUAGES {
            for set in [RuleSet::PROSE, RuleSet::INTERFACE] {
                let t = Typographer::new(language.clone(), set);
                for text in inputs {
                    let mut scratch = Scratch::new();
                    let mut diagnostics = record::Diagnostics::default();
                    let _ = run(&t, &mut scratch, &[Input::Text(text)], &mut diagnostics);
                    let (result, records) = report(&t, &mut scratch, text, Output::Unicode);
                    assert_eq!(diagnostics.finish(), records.diagnostics, "{text}");
                    let plain = typeset_unfiltered(&t, &mut scratch, text, Output::Unicode);
                    assert_eq!(plain, result, "{text}");
                }
            }
        }
    }

    #[test]
    fn protected_and_markup_pieces_are_scanned_against_the_encoded_set_only() {
        // Design section 3.2: the encoder writes protected pieces (and markup in escaped output),
        // so their bytes are matched against the encoded set; the rules never read them.
        let triggers = Triggers::of(b"-");
        let encoded = Triggers::of(b"\"");
        let none = core::iter::empty::<&str>;
        assert!(!scan(triggers, encoded, ["a"].into_iter(), none()));
        assert!(scan(
            triggers,
            encoded,
            ["a"].into_iter(),
            ["\""].into_iter()
        ));
        assert!(!scan(
            triggers,
            encoded,
            ["a"].into_iter(),
            ["-"].into_iter()
        ));
        assert!(scan(triggers, encoded, ["a-"].into_iter(), none()));
        assert!(scan(triggers, encoded, ["\""].into_iter(), none()));
        assert!(!scan(
            triggers,
            Triggers::NONE,
            ["a"].into_iter(),
            ["\""].into_iter()
        ));
    }

    #[test]
    fn the_trigger_set_is_the_union_of_the_active_rules() {
        // Design section 13.9.
        for language in LANGUAGES {
            for set in [
                RuleSet::EMPTY,
                RuleSet::INTERFACE,
                RuleSet::PROSE,
                RuleSet::ALL,
            ] {
                let t = Typographer::new(language.clone(), set);
                let expected = t.active_rules().iter().fold(Triggers::NONE, |acc, rule| {
                    acc.union(crate::rules::triggers(rule, &language.data(), false))
                });
                assert_eq!(t.compiled().triggers, expected, "{language} {set}");
            }
        }
        let t = Typographer::new(Language::RU_RU, RuleSet::EMPTY);
        assert_eq!(t.compiled().triggers, Triggers::NONE);
        let t = Typographer::new(Language::RU_RU, RuleSet::INTERFACE);
        let set = t.compiled().triggers;
        for byte in *b"\"-. " {
            assert!(set.contains(byte), "{}", byte as char);
        }
        // `№` through `nbsp.signs`.
        assert!(set.contains(0xe2));
        // Cyrillic letters never match.
        assert!(!set.contains(0xd0) && !set.contains(0xd1));
    }
}
