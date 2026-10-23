//! Watching a drag from outside the library, without a second set of pointer listeners.
//!
//! [`Scene::make_draggable_with_observer`](crate::scene::Scene::make_draggable_with_observer) installs the ordinary drag
//! and also reports what the drag does: it started, a move was drawn, it ended, or it was cancelled. A host that keeps
//! something else in step with a dragged node, such as a physics simulation, can then follow the library's own drag
//! instead of listening for pointer events itself and getting the pointer and the zoom wrong.
//!
//! # Who owns the callback
//!
//! The callback is owned by the [`DragObserverHandle`] that the call returns, and by nothing else. The drag listeners
//! and the pointer coalescer hold only a weak reference to the slot the callback sits in, and upgrade it for each
//! notification. So the callback can capture whatever it likes, including a handle to the scene's owner, without a cycle
//! through the scene, and dropping the handle ends the notifications at once.

use crate::model::node::NodeId;
use std::{
    cell::RefCell,
    fmt,
    rc::{Rc, Weak},
};
use svg_dom::root::utils::Point;

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// One step of a drag, as [`Scene::make_draggable_with_observer`](crate::scene::Scene::make_draggable_with_observer)
/// reports it.
///
/// Every `origin` is the top-left corner of the node's rectangle in the scene's own user space: the same unit that
/// [`Scene::move_node`](crate::scene::Scene::move_node) takes and [`Scene::node_rect`](crate::scene::Scene::node_rect)
/// gives. A host that wants the node's centre adds half of `node_rect(id).size`, and never assumes the size.
///
/// # Order
///
/// A drag is a [`Start`](Self::Start), any number of [`Applied`](Self::Applied), and then exactly one of
/// [`End`](Self::End) or [`Cancel`](Self::Cancel). Events for one node arrive in that order. Events for different nodes
/// can interleave, since two pointers can drag two nodes at once.
///
/// # Re-entrancy
///
/// The scene is not borrowed while the observer runs. It can call [`node_rect`](crate::scene::Scene::node_rect) or any
/// other method of the scene.
///
/// This enum is `#[non_exhaustive]`: match it with a wildcard arm.
#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
pub enum DragPhase {
    /// A drag began. `origin` is where the node is, read after the drag was established.
    Start { origin: Point },
    /// A move was written to the page without an error. `origin` is where the node now is.
    ///
    /// It is sent only when the move succeeded. A move that returned an error sends nothing. A successful move to the
    /// origin the node already has is still reported, since it can still retry a label that failed to draw earlier, so a
    /// duplicate is possible and harmless.
    Applied { origin: Point },
    /// The pointer was released. `origin` is where the node ended up, after the last pending move and after any push
    /// clear of an overlapping node, read from the scene.
    End { origin: Point },
    /// The drag was cancelled, for example by the system. `origin` is where the node is, which is its last committed
    /// position. A position the pointer had reached but that was never drawn is discarded.
    Cancel { origin: Point },
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The shared slot the callback sits in. Private to the library: the handle owns it, and the drag handlers hold it weakly.
pub(super) struct ObserverSlot {
    callback: RefCell<Option<Callback>>,
}

/// The callback, shared so that a notification can run it without holding the slot's borrow.
type Callback = Rc<dyn Fn(NodeId, DragPhase)>;

impl ObserverSlot {
    pub(super) fn new(callback: impl Fn(NodeId, DragPhase) + 'static) -> Rc<Self> {
        Rc::new(Self {
            callback: RefCell::new(Some(Rc::new(callback))),
        })
    }
}

/// A reference to an observer's slot that does not keep it alive.
pub(super) type WeakObserver = Weak<ObserverSlot>;

/// Tells the observer, if there is one and it is still there, that `phase` happened to node `id`.
///
/// The callback is cloned out of the slot before it runs, so the slot is not borrowed while it does. A callback that
/// drops its own handle, or asks for anything else of the slot, cannot panic, and it finishes. Nothing after it is
/// delivered.
pub(super) fn notify(observer: Option<&WeakObserver>, id: NodeId, phase: DragPhase) {
    let Some(slot) = observer.and_then(Weak::upgrade) else {
        return;
    };
    let callback = slot.callback.borrow().clone();
    if let Some(callback) = callback {
        callback(id, phase);
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The owner of a drag observer, returned by
/// [`Scene::make_draggable_with_observer`](crate::scene::Scene::make_draggable_with_observer).
///
/// The observer is called for as long as this handle exists. **Dropping it ends observation of that node for good.** It
/// does not remove the node's drag listeners, and it does not make the node eligible for a second call to
/// [`make_draggable`](crate::scene::Scene::make_draggable) or
/// [`make_draggable_with_observer`](crate::scene::Scene::make_draggable_with_observer), which still return
/// [`Error::AlreadyDraggable`](crate::error::Error::AlreadyDraggable). There is no way to attach a new observer to a node
/// that is already draggable.
///
/// A notification that is already running when the handle is dropped finishes, and none is delivered after it.
///
/// The handle is deliberately not `Clone`, and `#[must_use]`: a handle that is not kept is dropped at once, and the
/// observer would never be called.
#[must_use = "dropping this handle ends observation of the drag at once; keep it for as long as the events are needed"]
pub struct DragObserverHandle {
    slot: Rc<ObserverSlot>,
}

impl DragObserverHandle {
    pub(super) fn new(slot: Rc<ObserverSlot>) -> Self {
        Self { slot }
    }

    /// Whether the observer can still be called. It always can while this handle exists.
    pub fn is_observing(&self) -> bool {
        self.slot.callback.borrow().is_some()
    }
}

impl Drop for DragObserverHandle {
    fn drop(&mut self) {
        // Taken out, not just cleared, so a callback that is running keeps its own `Rc` and finishes.
        self.slot.callback.borrow_mut().take();
    }
}

impl fmt::Debug for DragObserverHandle {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("DragObserverHandle")
            .field("observing", &self.is_observing())
            .finish()
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::Cell;

    fn id() -> NodeId {
        NodeId { graph: 1, index: 0 }
    }

    fn origin() -> Point {
        Point::new(1.0, 2.0)
    }

    #[test]
    fn a_notification_reaches_a_callback_that_is_owned_by_a_live_handle() {
        let seen = Rc::new(Cell::new(0));
        let counter = seen.clone();
        let slot = ObserverSlot::new(move |_, _| counter.set(counter.get() + 1));
        let weak = Rc::downgrade(&slot);
        let handle = DragObserverHandle::new(slot);
        notify(Some(&weak), id(), DragPhase::Start { origin: origin() });
        notify(Some(&weak), id(), DragPhase::Applied { origin: origin() });
        assert_eq!(seen.get(), 2);
        assert!(handle.is_observing());
    }

    #[test]
    fn dropping_the_handle_ends_notifications_and_releases_what_the_callback_captured() {
        let seen = Rc::new(Cell::new(0));
        let counter = seen.clone();
        let slot = ObserverSlot::new(move |_, _| counter.set(counter.get() + 1));
        let weak = Rc::downgrade(&slot);
        let handle = DragObserverHandle::new(slot);
        assert_eq!(Rc::strong_count(&seen), 2);
        drop(handle);
        notify(Some(&weak), id(), DragPhase::End { origin: origin() });
        assert_eq!(seen.get(), 0);
        // The callback, and the counter it captured, are gone with the handle.
        assert_eq!(Rc::strong_count(&seen), 1);
        assert!(weak.upgrade().is_none());
    }

    #[test]
    fn no_observer_and_a_slot_that_has_gone_are_silent_no_ops() {
        notify(None, id(), DragPhase::Cancel { origin: origin() });
        let gone: WeakObserver = Weak::new();
        notify(Some(&gone), id(), DragPhase::Cancel { origin: origin() });
    }

    #[test]
    fn a_callback_that_drops_its_own_handle_while_running_finishes_and_gets_nothing_after() {
        let handle: Rc<RefCell<Option<DragObserverHandle>>> = Rc::new(RefCell::new(None));
        let calls = Rc::new(Cell::new(0));
        let (holder, counter) = (handle.clone(), calls.clone());
        let slot = ObserverSlot::new(move |_, _| {
            counter.set(counter.get() + 1);
            // Dropping the handle from inside the callback must not panic on the slot's borrow.
            holder.borrow_mut().take();
        });
        let weak = Rc::downgrade(&slot);
        *handle.borrow_mut() = Some(DragObserverHandle::new(slot));
        notify(Some(&weak), id(), DragPhase::Start { origin: origin() });
        notify(Some(&weak), id(), DragPhase::Applied { origin: origin() });
        assert_eq!(calls.get(), 1);
    }

    #[test]
    fn a_notification_sent_from_inside_a_notification_is_delivered_too() {
        let slot_cell: Rc<RefCell<Option<Weak<ObserverSlot>>>> = Rc::new(RefCell::new(None));
        let seen = Rc::new(Cell::new(0));
        let (cell, count) = (slot_cell.clone(), seen.clone());
        let slot = ObserverSlot::new(move |_, _| {
            count.set(count.get() + 1);
            if count.get() == 1 {
                let weak = cell.borrow().clone();
                notify(
                    weak.as_ref(),
                    NodeId { graph: 1, index: 0 },
                    DragPhase::Applied { origin: Point::new(0.0, 0.0) },
                );
            }
        });
        *slot_cell.borrow_mut() = Some(Rc::downgrade(&slot));
        let _handle = DragObserverHandle::new(slot);
        let weak = slot_cell.borrow().clone();
        notify(weak.as_ref(), id(), DragPhase::Start { origin: origin() });
        assert_eq!(seen.get(), 2);
    }

    #[test]
    fn the_debug_form_says_whether_it_is_observing() {
        let slot = ObserverSlot::new(|_, _| {});
        let handle = DragObserverHandle::new(slot);
        assert_eq!(format!("{handle:?}"), "DragObserverHandle { observing: true }");
    }
}
