//! The pipeline (design section 3.1) and its entry points.
//!
//! So far the pipeline runs the prefilter, ingest and protection, windows, the lexer and the
//! classifier, and records protected spans and the diagnostics of protection and the classifier.
//! The phases and the rules come next in stage 2 (design section 15.4); until then no window
//! changes, and every entry point returns its input.

pub(crate) mod classify;
mod ingest;
mod lex;
pub(crate) mod phase;
pub(crate) mod record;
pub(crate) mod scratch;
mod window;

use std::borrow::Cow;

use crate::report::Records;
use crate::rules::Triggers;
use crate::{DiagnosticKind, Output, Severity, Slot, Typographer};
use record::{Journal, Loc, NoRecord, Recorder, Sink};
use scratch::Scratch;

/// One input piece without the caller's payload: the call-local vector of piece texts (design
/// section 2.12).
#[derive(Clone, Copy, Debug)]
pub(crate) enum Input<'t> {
    Text(&'t str),
    Slot(Slot),
    Protected(&'t str),
    Markup(&'t str),
}

/// Runs the pipeline over `input` and reports whether any window changed.
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
        classify,
        ..
    } = scratch;
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
        // The phases run the active rules over the window here (stage 2, design section 3.8);
        // no rule is implemented yet, so no window changes.
    }
    false
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

/// The whole pipeline without the prefilter (design section 13.5, property 8).
fn typeset_unfiltered<'a>(
    typographer: &Typographer,
    scratch: &mut Scratch,
    text: &'a str,
    _output: Output,
) -> Cow<'a, str> {
    let _changed = run(typographer, scratch, &[Input::Text(text)], &mut NoRecord);
    // Rendering of changed windows arrives with the phases (stage 2) and the encoders (stage 3);
    // until then nothing changes, and the input comes back borrowed.
    Cow::Borrowed(text)
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
    template_texts(input, slot_count, changed)
}

/// The texts of a template in the n + 1 form.
fn template_texts<'t>(
    input: &[Input<'t>],
    slot_count: usize,
    changed: bool,
) -> (Vec<Cow<'t, str>>, bool) {
    let mut texts = Vec::with_capacity(slot_count.saturating_add(1));
    for group in input.split(|item| matches!(item, Input::Slot(_))) {
        texts.push(merge(group));
    }
    (texts, changed)
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
    let _changed = run(typographer, scratch, &[Input::Text(text)], &mut journal);
    (Cow::Borrowed(text), journal.finish())
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
    let (texts, changed) = template_texts(input, slot_count, changed);
    (texts, changed, journal.finish())
}

/// The text of the pieces between two slots: borrowed while it maps to at most one non-empty
/// piece, otherwise one string allocated at its final length (design section 2.6, output
/// contract item 2).
fn merge<'t>(group: &[Input<'t>]) -> Cow<'t, str> {
    let mut parts = group.iter().filter_map(|item| match *item {
        Input::Text(text) | Input::Protected(text) | Input::Markup(text) if !text.is_empty() => {
            Some(text)
        }
        Input::Text(_) | Input::Protected(_) | Input::Markup(_) | Input::Slot(_) => None,
    });
    let Some(first) = parts.next() else {
        return Cow::Borrowed("");
    };
    if parts.clone().next().is_none() {
        return Cow::Borrowed(first);
    }
    let len = parts
        .clone()
        .fold(first.len(), |len, text| len.saturating_add(text.len()));
    let mut all = String::with_capacity(len);
    all.push_str(first);
    parts.for_each(|text| all.push_str(text));
    Cow::Owned(all)
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
                let (texts, changed) = template_texts(&input, slots, changed);
                prop_assert_eq!(filtered.texts(), texts.as_slice());
                prop_assert_eq!(filtered.changed(), changed);
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
