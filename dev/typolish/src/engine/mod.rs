//! The pipeline (design section 3.1) and its entry points: the prefilter, ingest and protection,
//! windows, the lexer, the classifier, the seven phases and rendering, with the records of
//! `report`.

pub(crate) mod classify;
pub(crate) mod ingest;
pub(crate) mod lex;
pub(crate) mod phase;
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
use phase::{Cx, PhaseRules};
use record::{Journal, Loc, NoRecord, Recorder, Sink};
use scratch::Scratch;

/// The phases in their order (design section 3.8).
const PHASES: [Phase; 7] = [
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

    let active = typographer.compiled().active;
    let phases = PHASES.map(|phase| (phase, PhaseRules::of(phase, active)));
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
        // The phases (design section 3.8): each one pass over a snapshot of the tokens, with the
        // classification of the pass.
        removed.clear();
        let mut changed = false;
        for (phase, rules) in &phases {
            if rules.is_empty() {
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
            };
            if *phase == Phase::Quotes {
                quotes::analyze(&cx, quote_work, marks, &mut sink, report_pairs);
                cx.quotes = marks;
            }
            let outcome = phase::run_phase(&cx, rules.as_slice(), next, removed, &mut sink);
            core::mem::swap(tokens, next);
            if outcome.moved {
                phase::remap(tokens, frags);
            }
            changed |= outcome.changed;
        }
        if changed {
            render::collect(tokens, removed, edits);
        }
    }
    !edits.is_empty()
}

/// The bytes that the output kind encodes (design section 3.2). Only `Output::Unicode` is
/// rendered so far, and `Html` and `Escaped` are written as Unicode until stage 3, so no output
/// kind encodes a byte yet.
fn encoded(output: Output) -> Triggers {
    match output {
        Output::Unicode | Output::Html(_) | Output::Escaped(_) => Triggers::NONE,
    }
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
    let changed = wanted(
        typographer,
        output,
        core::iter::once(text),
        core::iter::empty(),
    ) && run(typographer, scratch, &[Input::Text(text)], &mut NoRecord);
    if changed {
        // `Html` and `Escaped` are written as Unicode until the encoders of stage 3.
        render::render_text_into(text, &scratch.edits, typographer.language().data(), buf);
    } else {
        buf.push_str(text);
    }
}

/// The whole pipeline without the prefilter (design section 13.5, property 8).
fn typeset_unfiltered<'a>(
    typographer: &Typographer,
    scratch: &mut Scratch,
    text: &'a str,
    _output: Output,
) -> Cow<'a, str> {
    if !run(typographer, scratch, &[Input::Text(text)], &mut NoRecord) {
        return Cow::Borrowed(text);
    }
    // `Html` and `Escaped` are written as Unicode until the encoders of stage 3.
    render::render_text(text, &scratch.edits, typographer.language().data())
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
    let changed = wanted(typographer, output, texts, encoded_only)
        && run(typographer, scratch, input, &mut NoRecord);
    template_texts(typographer, scratch, input, slot_count, changed)
}

/// The texts of a template in the n + 1 form after `run`, and whether any of them changed.
fn template_texts<'t>(
    typographer: &Typographer,
    scratch: &Scratch,
    input: &[Input<'t>],
    slot_count: usize,
    changed: bool,
) -> (Vec<Cow<'t, str>>, bool) {
    let edits = if changed { &scratch.edits[..] } else { &[] };
    render::render_template(
        input,
        &scratch.pieces,
        edits,
        slot_count,
        typographer.language().data(),
    )
}

/// `Typographer::report`: the whole pipeline with the journal, without the prefilter (design
/// section 3.2).
pub(crate) fn report<'a>(
    typographer: &Typographer,
    scratch: &mut Scratch,
    text: &'a str,
    _output: Output,
) -> (Cow<'a, str>, Records) {
    let mut journal = Journal::default();
    let result = if run(typographer, scratch, &[Input::Text(text)], &mut journal) {
        render::render_text(text, &scratch.edits, typographer.language().data())
    } else {
        Cow::Borrowed(text)
    };
    (result, journal.finish())
}

/// `Typographer::report_template`.
pub(crate) fn report_template<'t>(
    typographer: &Typographer,
    scratch: &mut Scratch,
    input: &[Input<'t>],
    slot_count: usize,
    _output: Output,
) -> (Vec<Cow<'t, str>>, bool, Records) {
    let mut journal = Journal::default();
    let changed = run(typographer, scratch, input, &mut journal);
    let (texts, changed) = template_texts(typographer, scratch, input, slot_count, changed);
    (texts, changed, journal.finish())
}

// The generators of design section 13.5; the file allows `dead_code` itself.
#[cfg(test)]
#[path = "../../tests/common/strategies.rs"]
mod strategies;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{EscapeStyle, HtmlStyle, Language, Piece, RuleSet};
    use proptest::prelude::*;

    const LANGUAGES: [Language; 3] = [Language::RU_RU, Language::EN_US, Language::COMMON];

    fn outputs() -> [Output; 5] {
        [
            Output::Unicode,
            Output::Html(HtmlStyle::NAMED),
            Output::Html(HtmlStyle::CHARACTERS),
            Output::Escaped(EscapeStyle::JS),
            Output::Escaped(EscapeStyle::RUST),
        ]
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
                let (texts, changed) = template_texts(&t, &scratch, &input, slots, changed);
                prop_assert_eq!(filtered.texts(), texts.as_slice());
                prop_assert_eq!(filtered.changed(), changed);
            }
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
                    acc.union(crate::rules::triggers(rule, language.data(), false))
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
