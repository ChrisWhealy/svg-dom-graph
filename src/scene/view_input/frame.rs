//! Coalesces a burst of view changes into one DOM write per animation frame.
//!
//! A trackpad pinch or a fast pan can deliver more wheel or pointer events than the browser paints frames. Each one
//! updates [`SceneInner::view`] at once — that is cheap arithmetic, and consecutive events must compose against the
//! latest view — but writing the content layer's `transform` more than once per frame is wasted work. So the write is
//! deferred to a single animation frame, the same way [`PointerCoalescer`](
//! crate::scene::drag::pointer_coalescer::PointerCoalescer) defers a dragged node's move.
//!
//! The frame is a [`FrameRequest`], which cancels it when the last clone of the flusher goes. So the pending frame can
//! never outlive the callback behind it, however the input handling is torn down.

use crate::{
    error::Error,
    scene::{SceneInner, frame_request::FrameRequest},
};
use std::{
    cell::RefCell,
    rc::{Rc, Weak},
};

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Writes the current view to the DOM, if it has changed since it was last written.
fn flush(inner: &Weak<RefCell<SceneInner>>) {
    if let Some(inner) = inner.upgrade() {
        // A frame callback has nowhere to report an error to. A failed write leaves the view marked as waiting.
        let _ = inner.borrow_mut().flush_view();
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Schedules at most one pending [`SceneInner::flush_view`] per animation frame. Cheap to clone.
#[derive(Clone)]
pub(super) struct ViewFlusher {
    /// Weak, so a pending frame never keeps a dropped scene alive.
    inner: Weak<RefCell<SceneInner>>,
    frame: Rc<FrameRequest>,
}

impl ViewFlusher {
    /// # Errors
    ///
    /// Returns [`Error::Svg`] if there is no `window` to schedule frames on.
    pub(super) fn new(inner: Weak<RefCell<SceneInner>>) -> Result<Self, Error> {
        let on_frame = inner.clone();
        let frame = FrameRequest::new(move || flush(&on_frame))?;
        Ok(Self { inner, frame })
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    /// Asks for the current view to be written on the next animation frame. Does nothing if a frame is already asked
    /// for, since that frame will write the latest view whenever it runs.
    pub(super) fn schedule(&self) {
        self.frame.request();
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    /// Writes the current view now and cancels any frame still pending, so a gesture that has just ended leaves the
    /// DOM exactly at its final position rather than one frame behind.
    pub(super) fn flush_now(&self) {
        self.frame.cancel();
        flush(&self.inner);
    }
}
