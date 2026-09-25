//! Coalesces a burst of view changes into one DOM write per animation frame.
//!
//! A trackpad pinch or a fast pan can deliver more wheel or pointer events than the browser paints frames. Each one
//! updates [`SceneInner::view`] at once — that is cheap arithmetic, and consecutive events must compose against the
//! latest view — but writing the content layer's `transform` more than once per frame is wasted work. So the write is
//! deferred to a single `requestAnimationFrame` callback, the same way [`PointerCoalescer`](
//! crate::scene::drag::pointer_coalescer::PointerCoalescer) defers a dragged node's move.

use crate::{error::Error, scene::SceneInner};
use std::{
    cell::{Cell, RefCell},
    rc::{Rc, Weak},
};
use wasm_bindgen::{JsCast, closure::Closure};

type FrameClosure = Closure<dyn FnMut(f64)>;

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
struct FlusherState {
    /// Weak, so a pending frame never keeps a dropped scene alive.
    inner: Weak<RefCell<SceneInner>>,
    /// The pending `requestAnimationFrame` request, if one is scheduled.
    raf_id: Cell<Option<i32>>,
}

impl FlusherState {
    fn flush(&self) {
        if let Some(inner) = self.inner.upgrade() {
            // A frame callback has nowhere to report an error to. A failed write leaves the view marked as waiting.
            let _ = inner.borrow_mut().flush_view();
        }
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Schedules at most one pending [`SceneInner::flush_view`] per animation frame. Cheap to clone.
#[derive(Clone)]
pub(super) struct ViewFlusher {
    window: web_sys::Window,
    state: Rc<FlusherState>,
    callback: Rc<FrameClosure>,
}

impl ViewFlusher {
    /// # Errors
    ///
    /// Returns [`Error::Svg`] if there is no `window` to schedule frames on.
    pub(super) fn new(inner: Weak<RefCell<SceneInner>>) -> Result<Self, Error> {
        let window = web_sys::window().ok_or_else(|| svg_dom::Error::Dom("no window".into()))?;
        let state = Rc::new(FlusherState { inner, raf_id: Cell::new(None) });

        // The callback holds only a weak reference back to the state that owns its request id.
        let weak_state = Rc::downgrade(&state);
        let callback: FrameClosure = Closure::new(move |_ts: f64| {
            let Some(state) = weak_state.upgrade() else { return };
            // This request has now fired, so clear it first: nothing here should mistake it for one still cancellable.
            state.raf_id.set(None);
            state.flush();
        });

        Ok(Self {
            window,
            state,
            callback: Rc::new(callback),
        })
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    /// Asks for the current view to be written on the next animation frame. Does nothing if a frame is already asked
    /// for, since that frame will write the latest view whenever it runs.
    pub(super) fn schedule(&self) {
        if self.state.raf_id.get().is_some() {
            return;
        }
        // If scheduling fails the view stays marked as waiting, and the next change tries again.
        if let Ok(id) = self.window.request_animation_frame((*self.callback).as_ref().unchecked_ref()) {
            self.state.raf_id.set(Some(id));
        }
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    /// Writes the current view now and cancels any frame still pending, so a gesture that has just ended leaves the
    /// DOM exactly at its final position rather than one frame behind.
    pub(super) fn flush_now(&self) {
        if let Some(id) = self.state.raf_id.take() {
            let _ = self.window.cancel_animation_frame(id);
        }
        self.state.flush();
    }
}
