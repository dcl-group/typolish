//! The pipeline (design section 3.1) and its entry points.
//!
//! So far the pipeline has the prefilter and ingest only: windows, the lexer, the classifier and
//! the phases arrive with the rules of stage 2 (design section 15.4). No rule is implemented yet,
//! so every window is unchanged and every entry point returns its input.

mod ingest;
mod lex;
pub(crate) mod scratch;

use std::borrow::Cow;

use crate::rules::Triggers;
use crate::{Output, Slot, Typographer};
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
fn run(_typographer: &Typographer, scratch: &mut Scratch, input: &[Input<'_>]) -> bool {
    if !ingest::ingest(input, &mut scratch.atoms) {
        // Positions do not fit `u32`: the input is returned unchanged (design section 3.4).
        return false;
    }
    // The phases of stage 2 run the active rules over each window here, reading the active set
    // and the trigger set that the typographer computes once when it is built (design section
    // 3.2), not per call. No rule is implemented yet, so no window changes.
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
/// output kind change something.
fn wanted<'a>(
    typographer: &Typographer,
    output: Output,
    texts: impl Iterator<Item = &'a str> + Clone,
) -> bool {
    let set = typographer.compiled().triggers.union(encoded(output));
    texts.clone().any(|text| set.any_in(text.as_bytes()))
}

/// `Typographer::typeset_with`.
pub(crate) fn typeset<'a>(
    typographer: &Typographer,
    scratch: &mut Scratch,
    text: &'a str,
    output: Output,
) -> Cow<'a, str> {
    if !wanted(typographer, output, core::iter::once(text)) {
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
    let _changed = run(typographer, scratch, &[Input::Text(text)]);
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
    let changed = wanted(typographer, output, texts) && run(typographer, scratch, input);
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
                let changed = run(&t, &mut scratch, &input);
                let (texts, changed) = template_texts(&input, slots, changed);
                prop_assert_eq!(filtered.texts(), texts.as_slice());
                prop_assert_eq!(filtered.changed(), changed);
            }
        }
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
