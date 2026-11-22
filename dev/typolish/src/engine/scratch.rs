//! Working memory: `Scratch` and the thread-local one (design section 2.12).

use core::cell::RefCell;
use core::fmt;
use core::mem::size_of;

use super::classify::ClassifyScratch;
use super::ingest::{Atom, DelimScratch, TextPiece};
use super::lex::Tok;

/// When the buffers of the thread-local scratch have grown past this many bytes, it is replaced
/// by a new one after the call.
const THREAD_LOCAL_LIMIT: usize = 64 * 1024;

/// Reusable working memory for callers that manage it themselves.
///
/// [`Typographer::typeset`](crate::Typographer::typeset) uses a thread-local scratch;
/// [`Typographer::typeset_with`](crate::Typographer::typeset_with) takes this one, for example one
/// per worker of an executor that moves tasks between threads. It holds only `'static` data and is
/// reused for inputs of any lifetime; it is never shrunk.
///
/// ```
/// use typolish::{Language, Output, RuleSet, Scratch, Typographer};
///
/// static EN: Typographer = Typographer::new(Language::EN_US, RuleSet::PROSE);
///
/// let mut scratch = Scratch::new();
/// for line in ["one", "two"] {
///     let out = EN.typeset_with(&mut scratch, line, Output::Unicode);
///     assert_eq!(out, line);
/// }
/// ```
#[derive(Default)]
pub struct Scratch {
    /// The call's text when it is copied from several `Text` pieces.
    pub(crate) text: String,
    pub(crate) pieces: Vec<TextPiece>,
    pub(crate) lens: Vec<u32>,
    pub(crate) atoms: Vec<Atom>,
    pub(crate) next_atoms: Vec<Atom>,
    pub(crate) delim: DelimScratch,
    pub(crate) clusters: Vec<(u32, u32)>,
    pub(crate) tokens: Vec<Tok>,
    pub(crate) next: Vec<Tok>,
    pub(crate) classify: ClassifyScratch,
    pub(crate) quote_stack: Vec<u32>,
}

impl Scratch {
    /// Empty working memory; allocates nothing.
    #[must_use]
    pub const fn new() -> Scratch {
        Scratch {
            text: String::new(),
            pieces: Vec::new(),
            lens: Vec::new(),
            atoms: Vec::new(),
            next_atoms: Vec::new(),
            delim: DelimScratch::new(),
            clusters: Vec::new(),
            tokens: Vec::new(),
            next: Vec::new(),
            classify: ClassifyScratch::new(),
            quote_stack: Vec::new(),
        }
    }

    /// The capacity of the buffers in bytes.
    fn heap_bytes(&self) -> usize {
        self.text.capacity()
            + self.pieces.capacity() * size_of::<TextPiece>()
            + self.lens.capacity() * size_of::<u32>()
            + (self.atoms.capacity() + self.next_atoms.capacity()) * size_of::<Atom>()
            + self.delim.heap_bytes()
            + self.clusters.capacity() * size_of::<(u32, u32)>()
            + (self.tokens.capacity() + self.next.capacity()) * size_of::<Tok>()
            + self.classify.heap_bytes()
            + self.quote_stack.capacity() * size_of::<u32>()
    }
}

impl fmt::Debug for Scratch {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Scratch").finish_non_exhaustive()
    }
}

thread_local! {
    static SCRATCH: RefCell<Scratch> = const { RefCell::new(Scratch::new()) };
}

/// Runs `f` with the thread-local scratch. If it is borrowed already (an iterator of pieces that
/// calls the typographer) or the thread is being torn down, `f` gets a local scratch instead,
/// without panicking. A scratch that grew past 64 KiB is replaced after the call.
pub(crate) fn with_thread_scratch<R>(mut f: impl FnMut(&mut Scratch) -> R) -> R {
    let mut result = None;
    // An error means the thread-local is gone; the fallback below handles it.
    let _ = SCRATCH.try_with(|cell| {
        if let Ok(mut scratch) = cell.try_borrow_mut() {
            result = Some(f(&mut scratch));
            if scratch.heap_bytes() > THREAD_LOCAL_LIMIT {
                *scratch = Scratch::new();
            }
        }
    });
    match result {
        Some(result) => result,
        None => f(&mut Scratch::new()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nested_use_falls_back_to_a_local_scratch() {
        let depth = with_thread_scratch(|_| with_thread_scratch(|_| 2));
        assert_eq!(depth, 2);
    }

    #[test]
    fn a_large_scratch_is_replaced() {
        with_thread_scratch(|s| s.quote_stack.reserve(THREAD_LOCAL_LIMIT));
        let after = with_thread_scratch(|s| s.heap_bytes());
        assert_eq!(after, 0);
    }
}
