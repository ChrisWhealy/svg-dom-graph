use crate::{
    error::Error,
    model::node::NodeId,
    scene::{SceneInner, frame_request::FrameRequest},
};
use std::{
    cell::{Cell, RefCell},
    rc::{Rc, Weak},
};
use svg_dom::root::utils::Point;

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The state shared by a [`PointerCoalescer`] and its own animation-frame callback.
struct CoalescerState {
    inner: Weak<RefCell<SceneInner>>,
    id: NodeId,
    /// The most recently pushed position not yet applied. `None` once applied (or never pushed).
    pending: Cell<Option<Point>>,
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
/// Keeping a strong reference would keep the whole scene alive for as long as this coalescer itself is.
///
/// The animation-frame callback is a [`FrameRequest`], built once in [`new`](Self::new) rather than freshly per
/// [`push`](Self::push) call, and re-armed each time a new frame needs scheduling. That avoids building a new closure and
/// its capture environment at up to display refresh rate for the whole duration of a drag.
///
/// A [`FrameRequest`] cancels its pending frame when it is dropped. That matters when the last clone of this coalescer
/// goes — as it does when the scene is dropped, since the listeners that hold the clones go with it — while a frame is
/// still pending. Without the cancel, the browser would go on to call a callback that no longer exists, and it throws
/// "closure invoked recursively or after being dropped". So no callback is ever left for the browser to call, and a
/// position pushed but not yet applied at that moment is simply not applied.
///
/// Every clone of a `PointerCoalescer` shares the same underlying [`CoalescerState`] and frame request. Cloning is how one
/// coalescer, created once per [`Scene::make_draggable_with`](super::Scene::make_draggable_with) call, is shared
/// across that node's own pointermove, pointerup, and pointercancel listener closures.
#[derive(Clone)]
pub(super) struct PointerCoalescer {
    state: Rc<CoalescerState>,
    /// The persistent animation-frame callback — see this type's own doc comment ("Ownership").
    frame: Rc<FrameRequest>,
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
        let state = Rc::new(CoalescerState {
            inner,
            id,
            pending: Cell::new(None),
            scratch: RefCell::new(String::new()),
        });

        // The frame callback holds the state strongly. That is no cycle: the state does not hold the frame request.
        let on_frame = state.clone();
        let frame = FrameRequest::new(move || {
            if let Some(origin) = on_frame.pending.take() {
                on_frame.apply_now(origin);
            }
        })?;

        Ok(Self { state, frame })
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    /// Records `origin` as the position to apply next and, unless a frame is already scheduled, schedules one.
    ///
    /// A `pointermove` that arrives while a frame is still pending only overwrites the pending position — it
    /// neither blocks nor schedules a second frame. So a whole burst of same-frame events collapses to exactly one
    /// `move_node` call, always using whichever position was most recent once that one frame actually runs.
    pub(super) fn push(&self, origin: Point) {
        self.state.pending.set(Some(origin));
        // Does nothing if a frame is already scheduled: that frame will pick up this newer position when it runs. If the
        // browser refuses the request, `pending` is left set, so the next `push` tries again.
        self.frame.request();
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

    /// Cancels a still-pending animation frame, if there is one. The callback itself stays alive — it is reused, not
    /// rebuilt, the next time [`push`](Self::push) schedules a frame.
    fn cancel_scheduled(&self) {
        self.frame.cancel();
    }
}
