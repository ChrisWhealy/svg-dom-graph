//! A pending animation frame that cannot outlive the callback behind it.
//!
//! `requestAnimationFrame` hands the browser a JavaScript function, and here that function is a `wasm_bindgen::Closure`.
//! A `Closure` invalidates its JavaScript function the moment it is dropped. If a frame is still pending at that moment,
//! the browser goes on to call the function anyway, and it throws "closure invoked recursively or after being dropped".
//!
//! That is easy to arrange. Whatever owns the closure is dropped whenever the listeners holding it are — when a toolbar is
//! hidden, a mode is changed, or a `Scene` goes — and a wheel event or a pointer move may have scheduled a frame only
//! moments before.
//!
//! So the closure and the pending request live together here, and dropping the request cancels the frame first. Once it
//! is gone, the browser has nothing left to call.

use crate::error::Error;
use std::{
    cell::{Cell, OnceCell},
    rc::{Rc, Weak},
};
use wasm_bindgen::{JsCast, closure::Closure};

/// One callback, and at most one animation frame requested for it at a time.
///
/// Dropping it cancels any frame still pending. Hold it in an `Rc` and clone that to share it. The callback runs when the
/// browser reaches the frame, and never once the last `Rc` has gone.
pub(crate) struct FrameRequest {
    window: web_sys::Window,
    /// The id of the frame requested and not yet run, if any.
    pending: Cell<Option<i32>>,
    /// Set straight after construction, since the closure needs a weak reference back to this to clear `pending`.
    callback: OnceCell<Closure<dyn FnMut(f64)>>,
}

impl FrameRequest {
    /// A request that runs `on_frame` each time a requested frame arrives.
    ///
    /// # Errors
    ///
    /// Returns [`Error::Svg`] if there is no `window` to request frames from.
    pub(crate) fn new(mut on_frame: impl FnMut() + 'static) -> Result<Rc<Self>, Error> {
        let window = web_sys::window().ok_or_else(|| svg_dom::Error::Dom("no window".into()))?;
        let request = Rc::new(Self {
            window,
            pending: Cell::new(None),
            callback: OnceCell::new(),
        });

        // Weak, since the request owns the closure and a strong reference back would keep both alive forever.
        let weak: Weak<Self> = Rc::downgrade(&request);
        let closure = Closure::new(move |_timestamp: f64| {
            // This frame has now arrived, so it is no longer pending. Cleared first, so nothing `on_frame` does could
            // mistake it for one that could still be cancelled.
            if let Some(request) = weak.upgrade() {
                request.pending.set(None);
            }
            on_frame();
        });
        let _ = request.callback.set(closure);
        Ok(request)
    }

    /// Asks for `on_frame` to run at the next animation frame. Does nothing if a frame is already requested, since that
    /// one will run it.
    ///
    /// If the browser refuses the request, nothing is pending, so the next call tries again.
    pub(crate) fn request(&self) {
        if self.pending.get().is_some() {
            return;
        }
        let Some(callback) = self.callback.get() else { return };
        if let Ok(id) = self.window.request_animation_frame(callback.as_ref().unchecked_ref()) {
            self.pending.set(Some(id));
        }
    }

    /// Cancels the requested frame, if there is one, so `on_frame` does not run for it.
    pub(crate) fn cancel(&self) {
        if let Some(id) = self.pending.take() {
            self.window.cancel_animation_frame(id).ok();
        }
    }
}

impl Drop for FrameRequest {
    /// Cancels the pending frame *before* the closure goes, so the browser never has a freed function left to call.
    fn drop(&mut self) {
        self.cancel();
    }
}
