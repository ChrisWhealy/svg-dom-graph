use crate::{error::Error, model::node::NodeId, scene::SceneInner};
use std::{
    cell::{Cell, RefCell},
    rc::{Rc, Weak},
};
use svg_dom::root::utils::Point;
use wasm_bindgen::{JsCast, closure::Closure};

/// The `requestAnimationFrame` callback [`PointerCoalescer::push`] schedules. This mirrors `svg-dom`'s own internal
/// `FrameClosure` type alias, for the same reason: it keeps the field/local types below readable.
type FrameClosure = Closure<dyn FnMut(f64)>;

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The state shared by a [`PointerCoalescer`] and its own `requestAnimationFrame` callback.
///
/// Split out from [`PointerCoalescer`] itself so the callback can hold a [`Weak`] reference to it — see
/// [`PointerCoalescer`]'s own doc comment ("Ownership") for why.
struct CoalescerState {
    inner: Weak<RefCell<SceneInner>>,
    id: NodeId,
    /// The most recently pushed position not yet applied. `None` once applied (or never pushed).
    pending: Cell<Option<Point>>,
    /// The pending `requestAnimationFrame` id, if a frame is currently scheduled.
    raf_id: Cell<Option<i32>>,
    /// Reused across every applied position, in and across frames — the same reasoning every other `move_node`
    /// caller in this crate reuses one scratch buffer for, rather than allocating a fresh `String` per call.
    scratch: RefCell<String>,
}

impl CoalescerState {
    /// Applies `origin` immediately, synchronously, using this state's own reused scratch buffer. Shared by
    /// [`PointerCoalescer::apply_now`] and the `requestAnimationFrame` callback itself.
    fn apply_now(&self, origin: Point) {
        if let Some(inner) = self.inner.upgrade() {
            let _ = inner.borrow_mut().move_node(self.id, origin, &mut self.scratch.borrow_mut());
        }
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Coalesces a dragged node's own pointer positions into, at most, one applied `move_node` per animation frame.
///
/// A mouse or pen's own hardware polling rate can approach 1000 Hz. If events were to be delivered to `pointermove`
/// event handler at this rate, it would overwhelm the browser's event queue and cause substantial lag. Since the
/// browser ever paints at a far slower rate, it is necessary to coalesce pointer events down to the animation frame
/// rate. This is done via [`push`](Self::push), which records only the latest position and applies it at most once per
/// animation frame, via `requestAnimationFrame`.
///
/// # Ownership
///
/// Holds only a [`Weak`] reference to the scene, exactly like the drag listener closures elsewhere in this module.
/// Keeping a strong reference would keep the whole scene alive for as long as this coalescer itself is, which is
/// fine unless the scene is dropped while a `requestAnimationFrame` request is still pending — in which case the
/// callback runs, finds nothing left to upgrade to, and simply does nothing.
///
/// The `requestAnimationFrame` callback is built once, in [`new`](Self::new), rather than freshly per
/// [`push`](Self::push) call. It captures a [`Weak`] reference to [`CoalescerState`], not a strong one, so it forms
/// no ownership cycle with the [`Rc`] `state` this coalescer (and every one of its clones) holds. [`push`](Self::push)
/// re-arms the same, already-built callback with a fresh `requestAnimationFrame` call each time a new frame needs
/// scheduling, rather than building a new closure and its capture environment at up to display refresh rate for the
/// whole duration of a drag.
///
/// Every clone of a `PointerCoalescer` shares the same underlying [`CoalescerState`] and callback. Cloning is how one
/// coalescer, created once per [`Scene::make_draggable_with`](super::Scene::make_draggable_with) call, is shared
/// across that node's own pointermove, pointerup, and pointercancel listener closures.
#[derive(Clone)]
pub(super) struct PointerCoalescer {
    window: web_sys::Window,
    state: Rc<CoalescerState>,
    /// The persistent `requestAnimationFrame` callback — see this type's own doc comment ("Ownership").
    callback: Rc<FrameClosure>,
}

impl PointerCoalescer {
    /// Creates a coalescer that applies coalesced positions to node `id`, against `inner`.
    ///
    /// # Errors
    ///
    /// Returns [`Error::Svg`] (wrapping [`svg_dom::Error::Dom`]) if no `window` is available. Expected never to
    /// happen in this crate's only supported environment, a real browser tab, but checked rather than assumed —
    /// see `svg-dom`'s own `AnimationLoop::start` for the same check.
    pub(super) fn new(inner: Weak<RefCell<SceneInner>>, id: NodeId) -> Result<Self, Error> {
        let window = web_sys::window().ok_or_else(|| svg_dom::Error::Dom("no window".into()))?;
        let state = Rc::new(CoalescerState {
            inner,
            id,
            pending: Cell::new(None),
            raf_id: Cell::new(None),
            scratch: RefCell::new(String::new()),
        });

        let weak_state = Rc::downgrade(&state);
        let callback: FrameClosure = Closure::new(move |_ts: f64| {
            let Some(state) = weak_state.upgrade() else { return };
            // This request has now fired, so it is no longer pending — clear it first, so nothing here could
            // mistake it for a request that could still be cancelled.
            state.raf_id.set(None);
            if let Some(origin) = state.pending.take() {
                state.apply_now(origin);
            }
        });

        Ok(Self {
            window,
            state,
            callback: Rc::new(callback),
        })
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    /// Records `origin` as the position to apply next and, unless a frame is already scheduled, schedules one.
    ///
    /// A `pointermove` that arrives while a frame is still pending only overwrites the pending position — it
    /// neither blocks nor schedules a second frame. So a whole burst of same-frame events collapses to exactly one
    /// `move_node` call, always using whichever position was most recent once that one frame actually runs.
    pub(super) fn push(&self, origin: Point) {
        self.state.pending.set(Some(origin));
        if self.state.raf_id.get().is_some() {
            // Already scheduled — the pending frame will pick up this newer position when it runs.
            return;
        }

        let Ok(raf_handle) = self.window.request_animation_frame((*self.callback).as_ref().unchecked_ref()) else {
            // Scheduling failed. `pending` is left set, so the next `push` tries again.
            return;
        };
        self.state.raf_id.set(Some(raf_handle));
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    /// Applies `origin` immediately, synchronously, using this coalescer's own reused scratch buffer.
    ///
    /// For a caller that already has an authoritative position of its own to apply right now — `pointerup`'s own
    /// collision-resolution correction is the one caller in this crate — rather than allocating a second buffer to
    /// live for the drag's own whole lifetime beside this one.
    ///
    /// Does not touch any pending coalesced position or scheduled frame — call [`cancel`](Self::cancel) or
    /// [`flush`](Self::flush) first if a stale one could otherwise still fire afterward and overwrite this.
    pub(super) fn apply_now(&self, origin: Point) {
        self.state.apply_now(origin);
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    /// Applies a pending position immediately, synchronously, cancelling any still-scheduled frame — for
    /// `pointerup`, so neither the node's own final rendered position nor a collision-resolution rect read
    /// straight afterward is ever one frame stale.
    ///
    /// Does nothing if nothing is pending, the common case: most drags end on a frame that has already run.
    pub(super) fn flush(&self) {
        self.cancel_scheduled();
        if let Some(origin) = self.state.pending.take() {
            self.apply_now(origin);
        }
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    /// Discards a pending position and cancels any still-scheduled frame, without applying it — for
    /// `pointercancel`, where the drag itself is being abandoned, not completed.
    ///
    /// Unlike [`flush`](Self::flush), a position pushed since the last applied frame is lost here, not applied.
    /// `pointercancel` fires when something *other than* the user's own deliberate release interrupted the
    /// gesture, so the position it fires at is not one this crate treats as the user's intended drop point.
    pub(super) fn cancel(&self) {
        self.cancel_scheduled();
        self.state.pending.set(None);
    }

    /// Cancels a still-pending `requestAnimationFrame` request, if there is one. The callback itself stays alive —
    /// it is reused, not rebuilt, the next time [`push`](Self::push) schedules a frame.
    fn cancel_scheduled(&self) {
        if let Some(id) = self.state.raf_id.take() {
            let _ = self.window.cancel_animation_frame(id);
        }
    }
}
