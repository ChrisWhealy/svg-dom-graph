use crate::{error::Error, model::node::NodeId, scene::SceneInner};
use std::{
    cell::{Cell, RefCell},
    rc::{Rc, Weak},
};
use svg_dom::root::utils::Point;
use wasm_bindgen::{JsCast, closure::Closure};

/// The `requestAnimationFrame` callback [`PointerCoalescer::push`] schedules.  This mirrors `svg-dom`'s own internal
/// `FrameClosure` type alias, for the same reason: it keeps the field/local types below readable.
type FrameClosure = Closure<dyn FnMut(f64)>;

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
/// Keeping a strong reference would keep the whole scene alive for as long as a `requestAnimationFrame` request stays
/// pending, which is fine unless the drag event is abandoned mid-gesture (i.e. the `pointerup`/`pointercancel` events
/// are never fired), in which case the scheduled `Closure` will never run, and the slot it occupies will never be
/// cleared.
///
/// [`push`](Self::push)'s own scheduled [`Closure`] holds an `Rc` clone of the slot it is stored in, so it can drop
/// itself once it has run.  This is the same self-referencing-but-not-leaking pattern used internally by
/// `AnimationLoop` in `svg-dom`, and is safe for the same reason documented there: `Closure::once` never reschedules
/// itself, so this is a one-shot slot the closure clears on its own way out, not a growing cycle.
/// [`cancel_scheduled`] clears it explicitly for the case the closure never gets to run at all.
///
/// Every clone of a `PointerCoalescer` shares the same underlying pending position, scheduled-frame id, and closure
/// slot. Cloning is how one coalescer, created once per [`Scene::make_draggable_with`](super::Scene::make_draggable_with)
/// call, is shared across that node's own pointermove, pointerup, and pointercancel listener closures.
#[derive(Clone)]
pub(super) struct PointerCoalescer {
    window: web_sys::Window,
    inner: Weak<RefCell<SceneInner>>,
    id: NodeId,
    /// The most recently pushed position not yet applied. `None` once applied (or never pushed).
    pending: Rc<Cell<Option<Point>>>,
    /// The pending `requestAnimationFrame` id, if a frame is currently scheduled.
    raf_id: Rc<Cell<Option<i32>>>,
    /// Owns the currently-scheduled closure, if any — see this type's own doc comment for why it is structured
    /// this way.
    closure_slot: Rc<RefCell<Option<FrameClosure>>>,
    /// Reused across every applied position, in and across frames — the same reasoning every other `move_node`
    /// caller in this crate reuses one scratch buffer for, rather than allocating a fresh `String` per call.
    scratch: Rc<RefCell<String>>,
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
        Ok(Self {
            window,
            inner,
            id,
            pending: Rc::new(Cell::new(None)),
            raf_id: Rc::new(Cell::new(None)),
            closure_slot: Rc::new(RefCell::new(None)),
            scratch: Rc::new(RefCell::new(String::new())),
        })
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    /// Records `origin` as the position to apply next and, unless a frame is already scheduled, schedules one.
    ///
    /// A `pointermove` that arrives while a frame is still pending only overwrites the pending position — it
    /// neither blocks nor schedules a second frame. So a whole burst of same-frame events collapses to exactly one
    /// `move_node` call, always using whichever position was most recent once that one frame actually runs.
    pub(super) fn push(&self, origin: Point) {
        self.pending.set(Some(origin));
        if self.raf_id.get().is_some() {
            // Already scheduled — the pending frame will pick up this newer position when it runs.
            return;
        }

        let inner = self.inner.clone();
        let id = self.id;
        let pending = self.pending.clone();
        let raf_id = self.raf_id.clone();
        let closure_slot = self.closure_slot.clone();
        let scratch = self.scratch.clone();

        let closure: FrameClosure = Closure::once(move |_ts: f64| {
            // This request has now fired, so it is no longer pending — clear it first, so nothing here could
            // mistake it for a request that could still be cancelled.
            raf_id.set(None);
            if let Some(origin) = pending.take() {
                if let Some(inner) = inner.upgrade() {
                    let _ = inner.borrow_mut().move_node(id, origin, &mut scratch.borrow_mut());
                }
            }
            // Releases the closure now that it has run exactly once — safe from inside its own invocation, since
            // `wasm_bindgen::Closure` keeps the executing call's own data alive until that call returns. See this
            // type's own doc comment, and `svg-dom`'s `AnimationLoop::stop` doc comment for the same reasoning.
            *closure_slot.borrow_mut() = None;
        });

        let Ok(raf_handle) = self.window.request_animation_frame(closure.as_ref().unchecked_ref()) else {
            // Scheduling failed. `pending` is left set, so the next `push` tries again.
            return;
        };
        self.raf_id.set(Some(raf_handle));
        *self.closure_slot.borrow_mut() = Some(closure);
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
        if let Some(inner) = self.inner.upgrade() {
            let _ = inner.borrow_mut().move_node(self.id, origin, &mut self.scratch.borrow_mut());
        }
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    /// Applies a pending position immediately, synchronously, cancelling any still-scheduled frame — for
    /// `pointerup`, so neither the node's own final rendered position nor a collision-resolution rect read
    /// straight afterward is ever one frame stale.
    ///
    /// Does nothing if nothing is pending, the common case: most drags end on a frame that has already run.
    pub(super) fn flush(&self) {
        self.cancel_scheduled();
        if let Some(origin) = self.pending.take() {
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
        self.pending.set(None);
    }

    /// Cancels a still-pending `requestAnimationFrame` request, if there is one, and releases its closure.
    fn cancel_scheduled(&self) {
        if let Some(id) = self.raf_id.take() {
            let _ = self.window.cancel_animation_frame(id);
        }
        *self.closure_slot.borrow_mut() = None;
    }
}
