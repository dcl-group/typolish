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
            Input::Text(s) | Input::Protected(s) | Input::Markup(s) => {
                u32::try_from(s.len()).unwrap_or(u32::MAX)
            }
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
        Input::Markup(text) if escaped => Some(*text),
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
    use crate::{EscapeStyle, HtmlStyle, Language, Piece, RuleSet, SingleQuotes};
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

    /// The structural changes of I2 (an inserted or a removed space) as closed byte intervals of x
    /// and of f(x).
    fn structural(changes: &[(u32, u32, usize)]) -> (Vec<Interval>, Vec<Interval>) {
        let mut sorted = changes.to_vec();
        sorted.sort_unstable();
        let (mut x, mut y) = (Vec::new(), Vec::new());
        let mut shift = 0i64;
        for (s, e, len) in sorted {
            let ys = i64::from(s) + shift;
            if s == e || len == 0 {
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
        let (sx, sy) = structural(&tx.changes);
        assert_eq!(sx.len(), 2, "{:?}", tx.changes);
        assert_eq!(sy.len(), 2);
    }

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(1024))]

        /// I2 and I4 of design section 4.3 (design section 13.5, property 3), with the stage 2
        /// rules (no `nbsp.*`). I2: outside the fragments that an inserted or a removed space split
        /// or merged, the fragments of f(x) have the kinds, flags and extents of those of x. I4: no
        /// match on f(x) has an edit, and every match on x outside those fragments recurs on f(x)
        /// with the same rule (`dash.double-hyphen` as `dash.spaced-hyphen`) and `consume`, except
        /// a dash match whose operand a removed space merged into a `Joined` fragment (design
        /// section 8.4: a `Joined` core is not an operand); the quote phase has its own proof
        /// (design section 6.11) and is left out of the recurrence.
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
            let mut scratch = Scratch::new();
            let (tx, out) = trace(&t, &mut scratch, &text);
            let (ty, _) = trace(&t, &mut scratch, &out);
            let (sx, sy) = structural(&tx.changes);

            let before: Vec<_> = tx
                .frags
                .iter()
                .filter(|f| !touches(&sx, i64::from(f.3), i64::from(f.4)))
                .map(|f| (f.0, f.1, f.2, map(&tx.changes, f.3, false), map(&tx.changes, f.4, true)))
                .collect();
            let after: Vec<_> = ty
                .frags
                .iter()
                .filter(|f| !touches(&sy, i64::from(f.3), i64::from(f.4)))
                .map(|f| (f.0, f.1, f.2, i64::from(f.3), i64::from(f.4)))
                .collect();
            prop_assert_eq!(before, after, "I2: {:?} -> {:?}", text, out);

            for m in &ty.matches {
                prop_assert!(!m.4, "I4: {:?} has an edit on {:?} (from {:?})", m, out, text);
            }
            let quotes = [Rule::QuotesApostrophe, Rule::QuotesPairs, Rule::QuotesPrimes];
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
                .filter(|m| !beside_a_merge(&tx, &ty, &sy, m))
            {
                let rule = if m.0 == Rule::DashDoubleHyphen { Rule::DashSpacedHyphen } else { m.0 };
                let (s, e) = (map(&tx.changes, m.1, false), map(&tx.changes, m.2, true));
                prop_assert!(
                    ty.matches.iter().any(|n| (n.0 == m.0 || n.0 == rule)
                        && i64::from(n.1) == s
                        && i64::from(n.2) == e
                        && n.3 == m.3),
                    "I4: {:?} on {:?} does not recur on {:?}: {:?}", m, text, out, ty.matches
                );
            }
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
            let (sx, sy) = structural(&tx.changes);
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
        let (_, sy) = structural(&tx.changes);
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
