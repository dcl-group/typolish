//! The pipeline (design section 3.1) and its entry points.
//!
//! So far the pipeline has ingest only: windows, the lexer, the classifier and the phases arrive
//! with the rules of stage 2 (design section 15.4). No rule is implemented yet, so every window is
//! unchanged and every entry point returns its input.

mod ingest;
mod lex;
pub(crate) mod scratch;

use std::borrow::Cow;

use crate::{Output, RuleSet, Slot, Typographer};
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

/// The rules the engine implements so far; an active rule outside this set does nothing.
const IMPLEMENTED: RuleSet = RuleSet::EMPTY;

/// Runs the pipeline over `input` and reports whether any window changed.
fn run(typographer: &Typographer, scratch: &mut Scratch, input: &[Input<'_>]) -> bool {
    if !ingest::ingest(input, &mut scratch.atoms) {
        // Positions do not fit `u32`: the input is returned unchanged (design section 3.4).
        return false;
    }
    // The phases of stage 2 run the implemented active rules over each window here. No rule is
    // implemented yet (`IMPLEMENTED` is empty), so no window changes.
    let _implemented = typographer.active_rules().intersection(IMPLEMENTED);
    false
}

/// Whether the output kind encodes any character of `text` (design section 3.2). Only
/// `Output::Unicode` is rendered so far; the HTML and escaping encoders of stage 3 extend this.
fn encodes(output: Output, _text: &str) -> bool {
    match output {
        Output::Unicode | Output::Html(_) | Output::Escaped(_) => false,
    }
}

/// `Typographer::typeset_with`.
pub(crate) fn typeset<'a>(
    typographer: &Typographer,
    scratch: &mut Scratch,
    text: &'a str,
    output: Output,
) -> Cow<'a, str> {
    let _changed = run(typographer, scratch, &[Input::Text(text)]) || encodes(output, text);
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
    let changed = run(typographer, scratch, input);
    let mut texts = Vec::with_capacity(slot_count.saturating_add(1));
    for group in input.split(|item| matches!(item, Input::Slot(_))) {
        texts.push(merge(group));
    }
    let encoded = texts.iter().any(|text| encodes(output, text));
    (texts, changed || encoded)
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
