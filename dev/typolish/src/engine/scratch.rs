//! Working memory: `Scratch` and the thread-local one (design section 2.12).

use core::cell::RefCell;
use core::fmt;
use core::mem::size_of;

use super::classify::ClassifyScratch;
use super::ingest::{Atom, DelimScratch, TextPiece};
use super::lex::Tok;
use super::render::REdit;
use crate::rules::quotes::{Mark, Work};

/// When the buffers of the thread-local scratch have grown past this many bytes, it is replaced
/// by a new one after the call. 4 MiB keeps the buffers of a paragraph up to about 50 KiB (design
/// section 2.12) at the cost of that much memory kept per thread.
const THREAD_LOCAL_LIMIT: usize = 4 * 1024 * 1024;

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
    /// The source ranges of the spaces that the phases of a window removed.
    pub(crate) removed: Vec<(u32, u32)>,
    /// The edits of the changed windows of a call (design section 3.10).
    pub(crate) edits: Vec<REdit>,
    /// The decisions of the quote phase, one per token.
    pub(crate) marks: Vec<Mark>,
    pub(crate) quotes: Work,
}

impl Scratch {
    /// Empty working memory; allocates nothing.
    ///
    /// ```
    /// use typolish::{Language, Output, RuleSet, Scratch, Typographer};
    ///
    /// static RU: Typographer = Typographer::new(Language::RU_RU, RuleSet::INTERFACE);
    ///
    /// let mut scratch = Scratch::new();
    /// let out = RU.typeset_with(&mut scratch, "Файл не найден", Output::Unicode);
    /// assert_eq!(out, "Файл не\u{a0}найден");
    /// ```
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
            removed: Vec::new(),
            edits: Vec::new(),
            marks: Vec::new(),
            quotes: Work::new(),
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
            + self.removed.capacity() * size_of::<(u32, u32)>()
            + self.edits.capacity() * size_of::<REdit>()
            + self.marks.capacity() * size_of::<Mark>()
            + self.quotes.heap_bytes()
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
/// without panicking. A scratch that grew past 4 MiB is replaced after the call.
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
        with_thread_scratch(|s| s.removed.reserve(THREAD_LOCAL_LIMIT));
        let after = with_thread_scratch(|s| s.heap_bytes());
        assert_eq!(after, 0);
    }
}
