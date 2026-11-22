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
pub(crate) fn typeset_template<'t>(
    typographer: &Typographer,
    scratch: &mut Scratch,
    input: &[Input<'t>],
    output: Output,
) -> (Vec<Cow<'t, str>>, bool) {
    let changed = run(typographer, scratch, input);
    let mut texts = Vec::new();
    let mut current = Text::Empty;
    for item in input {
        match *item {
            Input::Slot(_) => texts.push(core::mem::take(&mut current).finish()),
            Input::Text(text) | Input::Protected(text) | Input::Markup(text) => {
                current.push(text);
            }
        }
    }
    texts.push(current.finish());
    let encoded = texts.iter().any(|text| encodes(output, text));
    (texts, changed || encoded)
}

/// An output text being merged from adjacent pieces: borrowed while it maps to exactly one
/// non-empty input piece (design section 2.6, output contract item 2).
#[derive(Default)]
enum Text<'t> {
    #[default]
    Empty,
    One(&'t str),
    Many(String),
}

impl<'t> Text<'t> {
    fn push(&mut self, text: &'t str) {
        if text.is_empty() {
            return;
        }
        *self = match core::mem::take(self) {
            Text::Empty => Text::One(text),
            Text::One(first) => Text::Many([first, text].concat()),
            Text::Many(mut all) => {
                all.push_str(text);
                Text::Many(all)
            }
        };
    }

    fn finish(self) -> Cow<'t, str> {
        match self {
            Text::Empty => Cow::Borrowed(""),
            Text::One(text) => Cow::Borrowed(text),
            Text::Many(all) => Cow::Owned(all),
        }
    }
}
