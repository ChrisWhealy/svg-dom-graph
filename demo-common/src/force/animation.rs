//! A cancellable animation frame that hands its callback the browser's own timestamp.
//!
//! The library's own frame request is private to it, and its callback takes no timestamp, which physics needs. This one
//! has the same ownership rules. `requestAnimationFrame` is given a `wasm_bindgen::Closure`, and a `Closure` invalidates
//! its JavaScript function when it is dropped, so a frame still pending at that moment would make the browser call a
//! function that is gone. So the closure and the pending request live together, and dropping the request cancels the
//! frame first.

use std::{
    cell::{Cell, OnceCell},
    rc::{Rc, Weak},
};
use wasm_bindgen::{JsCast, closure::Closure};

/// One callback, and at most one animation frame requested for it at a time.
///
/// Hold it in an `Rc`. Dropping the last one cancels the pending frame, and the callback never runs after that.
pub struct AnimationLoop {
    window: web_sys::Window,
    pending: Cell<Option<i32>>,
    callback: OnceCell<Closure<dyn FnMut(f64)>>,
}

impl AnimationLoop {
    /// A loop that calls `on_frame` with the browser's timestamp, in milliseconds, each time a requested frame arrives.
    ///
    /// # Errors
    ///
    /// Returns `Err` if there is no `window`.
    pub fn new(mut on_frame: impl FnMut(f64) + 'static) -> Result<Rc<Self>, String> {
        let window = web_sys::window().ok_or_else(|| "no window to request animation frames from".to_owned())?;
        let this = Rc::new(Self {
            window,
            pending: Cell::new(None),
            callback: OnceCell::new(),
        });
        // Weak, since the loop owns the closure and a strong reference back would keep both alive for ever.
        let weak: Weak<Self> = Rc::downgrade(&this);
        let closure = Closure::new(move |timestamp: f64| {
            // This frame has arrived, so it is no longer pending. Cleared before `on_frame`, so nothing it does mistakes
            // it for one that could still be cancelled, and `on_frame` may request the next.
            if let Some(this) = weak.upgrade() {
                this.pending.set(None);
            }
            on_frame(timestamp);
        });
        let _ = this.callback.set(closure);
        Ok(this)
    }

    /// Asks for a frame, unless one is already pending. If the browser refuses, nothing is pending, so the next call
    /// tries again.
    pub fn request(&self) {
        if self.pending.get().is_some() {
            return;
        }
        let Some(callback) = self.callback.get() else {
            return;
        };
        if let Ok(id) = self.window.request_animation_frame(callback.as_ref().unchecked_ref()) {
            self.pending.set(Some(id));
        }
    }

    /// Cancels the pending frame, if there is one.
    pub fn cancel(&self) {
        if let Some(id) = self.pending.take() {
            self.window.cancel_animation_frame(id).ok();
        }
    }

    /// Whether a frame is pending. This is the number the panel publishes as `data-physics-pending`.
    pub fn is_pending(&self) -> bool {
        self.pending.get().is_some()
    }
}

impl Drop for AnimationLoop {
    /// Cancels the pending frame before the closure goes, so the browser never has a freed function left to call.
    fn drop(&mut self) {
        self.cancel();
    }
}
