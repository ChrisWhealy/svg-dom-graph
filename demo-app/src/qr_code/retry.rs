//! Work that has to be done again, until it works.
//!
//! Bringing the nodes of the scene into line with new text writes to several of them one after another, and the browser
//! can refuse any write. Nothing can undo the writes that were made. What can be done is to remember that the work is
//! not finished, and to do all of it again at the next chance. Writing a value that a node already holds changes
//! nothing, so doing the work again is safe, and it repairs whatever the first try left out.

use std::cell::Cell;

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Whether some work is still to be done, and a way to do it only while it is.
#[derive(Debug, Default)]
pub(super) struct Retry {
    pending: Cell<bool>,
}

impl Retry {
    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    /// Records that the work has to be done, and is not yet.
    pub(super) fn require(&self) {
        self.pending.set(true);
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    /// Whether the work is still to be done.
    #[cfg(test)]
    pub(super) fn is_pending(&self) -> bool {
        self.pending.get()
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    /// Does `work` if it is still to be done, and says it is done only if `work` succeeds. If nothing is pending,
    /// `work` is not called, and so costs nothing.
    ///
    /// # Errors
    ///
    /// Returns the error of `work`. The work stays pending, so that the next call tries again.
    pub(super) fn run(&self, work: impl FnOnce() -> Result<(), String>) -> Result<(), String> {
        if !self.pending.get() {
            return Ok(());
        }
        work()?;
        self.pending.set(false);
        Ok(())
    }
}
