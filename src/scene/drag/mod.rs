pub(crate) mod collision_policy;
mod drag_options;
mod drag_start;
mod handlers;
mod install_guard;
mod observer;
mod pointer_coalescer;

use super::{Scene, client_to_user_space};
use crate::{error::Error, model::node::NodeId};
use collision_policy::CollisionPolicy;
pub use drag_options::DragOptions;
use drag_start::DragStart;
use install_guard::InstallGuard;
pub use observer::{DragObserverHandle, DragPhase};
use observer::{ObserverSlot, WeakObserver};
use pointer_coalescer::PointerCoalescer;
use std::{cell::Cell, rc::Rc};
use svg_dom::root::utils::Rect;

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// `user-select: none` alone does not reliably suppress a click-drag text selection in every engine: Safari in
/// particular has still started one with only the CSS property set. Consequently, `make_draggable` also calls
/// `prevent_default()` on `pointerdown`/`pointermove`. The two are kept together. CSS blocks selection from a mouse
/// drag that starts outside this element and passes over it without ever firing this element's own `pointerdown`.
/// `prevent_default()` blocks it for the drag this element's own listeners actually see.
const GRAB_STYLE: &str = "touch-action: none; user-select: none; -webkit-user-select: none;";
/// Style applied while a box is actively being dragged — same as [`GRAB_STYLE`], but with a grabbing cursor. A drag
/// already in progress benefits from that feedback in a way merely hovering over the node does not.
const GRABBING_STYLE: &str = "cursor: grabbing; touch-action: none; user-select: none; -webkit-user-select: none;";

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// One draggable node's own in-progress drag, shared by its four pointer handlers. `None` while no drag is active.
type DragState = Rc<Cell<Option<DragStart>>>;

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The pointer event types [`Scene::make_draggable_with`] registers a listener for, in registration order.
const DRAG_EVENT_TYPES: [&str; 4] = ["pointerdown", "pointermove", "pointerup", "pointercancel"];

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Returns [`Error::InvalidCollisionPadding`] if `options.collision` is [`CollisionPolicy::PushClear`] with a `padding`
/// that is not a finite value `>= 0.0`, or [`Error::InvalidDragBounds`] for bad `options.bounds`. Checked before anything
/// else, so a rejected call leaves the scene untouched.
fn validate_options(options: &DragOptions) -> Result<(), Error> {
    if let CollisionPolicy::PushClear { padding } = options.collision {
        if !(padding.is_finite() && padding >= 0.0) {
            return Err(Error::InvalidCollisionPadding(padding));
        }
    }
    validate_bounds(options.bounds)
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Returns [`Error::InvalidDragBounds`] if `bounds` is `Some` with a non-finite origin or size, or a negative width or
/// height. `None`, and every finite rect with a non-negative width and height, are valid — including a zero width or
/// height, which [`clamp_to_bounds`] already handles deterministically.
fn validate_bounds(bounds: Option<Rect>) -> Result<(), Error> {
    let Some(bounds) = bounds else { return Ok(()) };

    let finite = bounds.origin.x.is_finite()
        && bounds.origin.y.is_finite()
        && bounds.size.width.is_finite()
        && bounds.size.height.is_finite();

    if finite && bounds.size.width >= 0.0 && bounds.size.height >= 0.0 {
        Ok(())
    } else {
        Err(Error::InvalidDragBounds(bounds))
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
impl Scene {
    /// Wires up pointer dragging for node `id`, with [`DragOptions::default`]'s collision behaviour. A drop that
    /// overlaps another node is pushed back clear of it, along the line to where the drag started, plus 6 user-space
    /// units of padding.
    ///
    /// See [`make_draggable_with`](Self::make_draggable_with) to allow overlapping nodes, or to use different padding.
    /// See that method's own doc comment for this call's accessibility contract too.
    ///
    /// # Errors
    ///
    /// Returns [`Error::UnknownNode`] if `id` does not name a node in this scene — for example, a `NodeId` from a
    /// different `Scene`.
    ///
    /// Returns [`Error::AlreadyDraggable`] if `id` is already draggable. Calling this (or [`Scene::make_draggable`]) a
    /// second time for the same node does not replace the first installation. So it is rejected outright rather than
    /// silently doubling up its listeners and drag-state.
    pub fn make_draggable(&self, id: NodeId) -> Result<(), Error> {
        self.make_draggable_with(id, DragOptions::default())
    }

    /// Wires up pointer dragging for node `id`, with `options` controlling what happens when a drop leaves it
    /// overlapping another node — see [`CollisionPolicy`].
    ///
    /// Moves the node, and redraws its incident connectors, as the pointer moves.
    ///
    /// `PointerEvent::client_x`/`client_y` are viewport CSS pixels, not this scene's user-space coordinates — the two
    /// only coincide when the `<svg>` has no CSS scaling and its `viewBox` matches its pixel size exactly. This
    /// converts through the dragged group's own screen CTM (see `invert_matrix`/`apply_matrix` in `geometry`), so
    /// dragging stays correct under scaling, a resized `viewBox`, or CSS transforms.
    ///
    /// Pointer-based dragging is the only interaction mechanism offered here: not a complete WCAG-compliant way to
    /// reposition a node. The keyboard equivalent required by WCAG 2.1.1 has not been implemented, neither has the WCAG
    /// 2.5.single-pointer alternative.
    ///
    /// # Errors
    ///
    /// Returns [`Error::UnknownNode`] if `id` does not name a node in this scene — for example, a `NodeId` from a
    /// different `Scene`.
    ///
    /// Returns [`Error::AlreadyDraggable`] if `id` is already draggable. Calling this (or [`Scene::make_draggable`]) a
    /// second time for the same node does not replace the first installation. So it is rejected outright rather than
    /// silently doubling up its listeners and drag-state. Reusing `id` after such an error is safe: the first
    /// installation is untouched.
    ///
    /// Returns [`Error::InvalidCollisionPadding`] if `options.collision` is [`CollisionPolicy::PushClear`] with a
    /// `padding` that is not a finite value `>= 0.0`.
    ///
    /// Returns [`Error::InvalidDragBounds`] if `options.bounds` is `Some` with an origin or size that is not finite, or
    /// a negative width or height.
    ///
    /// Both are checked before anything else, so this scene's existing state is left untouched either way.
    ///
    /// If `set_attr` or any one of the four pointer-listener registrations this method makes fails partway through,
    /// `id` is left exactly as it was before the call. A failure is expected to be extremely rare, since it means the
    /// underlying `addEventListener` DOM call itself failed. `id` is not marked draggable, and none of this method's
    /// own listeners are left dangling on it. A failed call can safely be retried.
    pub fn make_draggable_with(&self, id: NodeId, options: DragOptions) -> Result<(), Error> {
        self.install_drag(id, options, None)
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    /// Wires up pointer dragging for node `id` exactly as [`make_draggable_with`](Self::make_draggable_with) does, and
    /// also tells `on_drag` what the drag does, as a [`DragPhase`]: that it started, that each move was drawn, and that it
    /// ended or was cancelled.
    ///
    /// This is for a host that keeps something else in step with a dragged node, such as a physics simulation. It
    /// follows the library's own drag, which already gets the pointer, the zoom and the bounds right, and adds no second
    /// set of pointer listeners.
    ///
    /// The drag itself is unchanged: the same listeners, the same pointer capture and the same coalescing of moves to
    /// one a frame.
    ///
    /// # Ownership
    ///
    /// The returned [`DragObserverHandle`] owns the callback, and the drag listeners hold it only weakly. **Keep the
    /// handle for as long as the events are wanted.** The observer stops the moment the handle is dropped, and a handle
    /// that is not bound to anything is dropped at once, which `#[must_use]` warns about.
    ///
    /// **Dropping the handle is final for that node.** It does not remove the drag listeners, and it does not let the node
    /// be made draggable again: a later call to this method or to [`make_draggable_with`](Self::make_draggable_with)
    /// returns [`Error::AlreadyDraggable`]. There is no way to attach a different observer to a node that is already
    /// draggable.
    ///
    /// # Timing and re-entrancy
    ///
    /// Events are delivered synchronously, from inside the pointer event handler or the animation frame that caused
    /// them, and never while the scene is borrowed, so `on_drag` may call any method of the scene, `node_rect` included.
    /// It must not assume it runs at a frame boundary, and it should do little: a host that has more to do queues the
    /// event and does it later.
    ///
    /// [`DragPhase::Applied`] is sent only for a move that succeeded. A move that failed to write sends nothing, and the
    /// error is dropped as it is for an unobserved drag. A host that wants the position the scene really has asks
    /// [`node_rect`](Self::node_rect), and [`DragPhase::End`] and [`DragPhase::Cancel`] carry it.
    ///
    /// The caller must keep the handle. A helper that only installs the observer, and returns without handing the handle
    /// back, ends the notifications as it returns:
    ///
    /// ```no_run
    /// use std::{cell::RefCell, rc::Rc};
    /// use svg_dom::root::utils::{Point, Size};
    /// use svg_dom_graph::scene::{DragObserverHandle, DragOptions, DragPhase, Scene};
    ///
    /// /// Returns the handle, and with it the log of where each drag ended. The caller keeps the handle for as long as
    /// /// the events are wanted, and the notifications stop when it is dropped.
    /// fn watch(
    ///     scene: &Scene,
    /// ) -> Result<(DragObserverHandle, Rc<RefCell<Vec<Point>>>), svg_dom_graph::Error> {
    ///     let node = scene.add_node(Point::new(10.0, 10.0), Size::new(60.0, 30.0), "A")?;
    ///     let log = Rc::new(RefCell::new(Vec::new()));
    ///     let sink = log.clone();
    ///     let handle = scene.make_draggable_with_observer(node, DragOptions::default(), move |_id, phase| {
    ///         if let DragPhase::End { origin } = phase {
    ///             sink.borrow_mut().push(origin);
    ///         }
    ///     })?;
    ///     Ok((handle, log))
    /// }
    /// ```
    ///
    /// # Errors
    ///
    /// The same as [`make_draggable_with`](Self::make_draggable_with): [`Error::UnknownNode`],
    /// [`Error::AlreadyDraggable`], [`Error::InvalidCollisionPadding`] and [`Error::InvalidDragBounds`], and a wrapped
    /// [`Error::Svg`] if a listener could not be registered. All are checked, or rolled back, before the call returns, so
    /// a failed call installs nothing, makes no node draggable and returns no handle.
    pub fn make_draggable_with_observer(
        &self,
        id: NodeId,
        options: DragOptions,
        on_drag: impl Fn(NodeId, DragPhase) + 'static,
    ) -> Result<DragObserverHandle, Error> {
        let slot = ObserverSlot::new(on_drag);
        // If installing fails, `slot` is dropped here, with the callback in it, and nothing was left behind.
        self.install_drag(id, options, Some(Rc::downgrade(&slot)))?;
        Ok(DragObserverHandle::new(slot))
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    /// The installation both public methods share: validate, register the four listeners transactionally, and mark the
    /// node draggable. `observer` is `None` for an unobserved drag.
    fn install_drag(&self, id: NodeId, options: DragOptions, observer: Option<WeakObserver>) -> Result<(), Error> {
        validate_options(&options)?;
        let group = self.undragged_group(id)?;
        let guard = InstallGuard::new(group.clone());

        let drag_start: DragState = Rc::new(Cell::new(None));
        // Coalesces this node's own pointermove positions to at most one applied `move_node` per animation frame — see
        // `PointerCoalescer`'s own doc comment. Created once here, alongside `drag_start`, and reused across every drag
        // this node goes through for as long as it stays draggable, not just the next one.
        let coalescer = PointerCoalescer::new(Rc::downgrade(&self.inner), id, observer)?;
        let inner_weak = Rc::downgrade(&self.inner);

        // Each handler holds only weak references — see `handlers`' own module doc comment for why.
        group.on_pointerdown(handlers::pointerdown(
            id,
            group.downgrade(),
            inner_weak.clone(),
            drag_start.clone(),
            coalescer.clone(),
        ))?;
        let confinement = handlers::Confinement {
            bounds: options.bounds,
            confine_to_view: options.confine_to_view,
        };
        group.on_pointermove(handlers::pointermove(
            confinement,
            inner_weak.clone(),
            drag_start.clone(),
            coalescer.clone(),
        ))?;
        group.on_pointerup(handlers::pointerup(
            id,
            options.collision,
            confinement,
            group.downgrade(),
            inner_weak.clone(),
            drag_start.clone(),
            coalescer.clone(),
        ))?;
        group.on_pointercancel(handlers::pointercancel(
            id,
            group.downgrade(),
            inner_weak,
            drag_start,
            coalescer,
        ))?;

        // Every listener has now registered successfully.
        //
        // Only now should the idle style be set. A listener-registration failure above must leave `group` with no style
        // change at all. That matches `InstallGuard`'s own promise to unwind back to exactly the state it found `group`
        // in.
        //
        // Since this function runs synchronously, no pointer event can interleave between this call and `disarm` below.
        group.set_attr("style", GRAB_STYLE)?;
        guard.disarm();
        self.inner
            .borrow_mut()
            .node_handle_mut(id)
            .ok_or(Error::UnknownNode(id))?
            .draggable = true;

        Ok(())
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    /// The `<g>` of node `id`, if it exists and is not yet draggable. Returns [`Error::UnknownNode`] for an unknown id and
    /// [`Error::AlreadyDraggable`] for one that is already draggable.
    fn undragged_group(&self, id: NodeId) -> Result<svg_dom::SvgNode, Error> {
        let inner = self.inner.borrow();
        let handles = inner.node_handle(id).ok_or(Error::UnknownNode(id))?;
        if handles.draggable {
            return Err(Error::AlreadyDraggable(id));
        }
        Ok(handles.group.clone())
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[cfg(test)]
mod unit_tests;
