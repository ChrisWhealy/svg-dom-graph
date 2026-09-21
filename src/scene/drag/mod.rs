pub(crate) mod collision_policy;
mod drag_options;
mod drag_start;
mod install_guard;
mod pointer_coalescer;

use super::{Scene, client_to_user_space};
use crate::{
    error::Error,
    geometry::{clamp_to_bounds, invert_matrix},
    model::node::NodeId,
};
use collision_policy::CollisionPolicy;
pub use drag_options::DragOptions;
use drag_start::DragStart;
use install_guard::InstallGuard;
use pointer_coalescer::PointerCoalescer;
use std::{cell::Cell, rc::Rc};
use svg_dom::root::utils::{Point, Rect};

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// `user-select: none` alone does not reliably suppress a click-drag text selection in every engine: Safari in
/// particular has still started one with only the CSS property set. Consequently, `make_draggable` also calls
/// `prevent_default()` on `pointerdown`/`pointermove`. The two are kept together: CSS blocks selection from a mouse
/// drag that starts outside this element and passes over it without ever firing this element's own `pointerdown`, while
/// `prevent_default()` blocks it for the drag this element's own listeners actually see.
const GRAB_STYLE: &str = "touch-action: none; user-select: none; -webkit-user-select: none;";
/// Style applied while a box is actively being dragged — same as [`GRAB_STYLE`], but with a grabbing cursor. A drag
/// already in progress benefits from that feedback in a way merely hovering over the node does not.
const GRABBING_STYLE: &str = "cursor: grabbing; touch-action: none; user-select: none; -webkit-user-select: none;";

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The pointer event types [`Scene::make_draggable_with`] registers a listener for, in registration order.
const DRAG_EVENT_TYPES: [&str; 4] = ["pointerdown", "pointermove", "pointerup", "pointercancel"];

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
    /// Wires up pointer dragging for node `id`, with [`DragOptions::default`]'s collision behaviour: a drop that
    /// overlaps another node is pushed back clear of it, along the line to where the drag started, plus 6 user-space
    /// units of padding.
    ///
    /// See [`make_draggable_with`](Self::make_draggable_with) to allow overlapping nodes, or to use different padding.
    ///
    /// # Errors
    ///
    /// Returns [`Error::UnknownNode`] if `id` does not name a node in this scene — for example, a `NodeId` from a
    /// different `Scene`.
    ///
    /// Returns [`Error::AlreadyDraggable`] if `id` is already draggable — see
    /// [`make_draggable_with`](Self::make_draggable_with)'s own `# Errors` section for why.
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
    /// # Errors
    ///
    /// Returns [`Error::UnknownNode`] if `id` does not name a node in this scene — for example, a `NodeId` from a
    /// different `Scene`.
    ///
    /// Returns [`Error::AlreadyDraggable`] if `id` is already draggable — calling this (or [`Scene::make_draggable`]) a
    /// second time for the same node does not replace the first installation, so this is rejected outright rather than
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
    /// If `set_attr` or any one of the four pointer-listener registrations this method makes fails partway through —
    /// expected to be extremely rare, since it means the underlying `addEventListener` DOM call itself failed — `id` is
    /// left exactly as it was before the call: not marked draggable, and with none of this method's own listeners left
    /// dangling on it. A failed call can safely be retried.
    pub fn make_draggable_with(&self, id: NodeId, options: DragOptions) -> Result<(), Error> {
        if let CollisionPolicy::PushClear { padding } = options.collision {
            if !(padding.is_finite() && padding >= 0.0) {
                return Err(Error::InvalidCollisionPadding(padding));
            }
        }
        validate_bounds(options.bounds)?;

        let group = {
            let inner = self.inner.borrow();
            let handles = inner.node_handle(id).ok_or(Error::UnknownNode(id))?;
            if handles.draggable {
                return Err(Error::AlreadyDraggable(id));
            }
            handles.group.clone()
        };
        let guard = InstallGuard::new(group.clone());

        let drag_start: Rc<Cell<Option<DragStart>>> = Rc::new(Cell::new(None));
        // Coalesces this node's own pointermove positions to at most one applied `move_node` per animation frame
        // — see `PointerCoalescer`'s own doc comment. Created once here, alongside `drag_start`, and reused across
        // every drag this node goes through for as long as it stays draggable, not just the next one.
        let coalescer = PointerCoalescer::new(Rc::downgrade(&self.inner), id)?;

        // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
        // Both `group` and `inner` must be captured as weak clones to avoid creating an ownership cycle.
        //
        // `group` is the node on which this listener is registered: using a strong capture would create a cycle
        // (SvgNodeInner -> listener store -> closure -> SvgNode -> the same SvgNodeInner) that leaks the node and
        // defeats its automatic listener cleanup. See `WeakSvgNode`'s doc comment.
        //
        // `inner` needs the same treatment one level up: `SceneInner::node_handles` owns `group`, so a strong `inner`
        // clone in this closure would create the cycle back through `SceneInner` itself (`SceneInner -> group ->
        // listener store -> closure -> SceneInner`), leaking the whole scene (plus everything it renders along  every
        // listener on every node in that scene).  This would happen even after every external `Scene` handle has been
        // dropped.
        {
            let group_weak = group.downgrade();
            let inner_weak = Rc::downgrade(&self.inner);
            let drag_start = drag_start.clone();

            group.on_pointerdown(move |evt| {
                // Ignores a pointerdown while a drag is already active, otherwise a second pointer touching this
                // element mid-drag would silently steal it, overwriting the first pointer's `DragStart` before that
                // pointer's own pointerup/pointercancel ever fires.
                // Also ignores anything but the primary button — `button() == 0` is left mouse, touch, or ordinary pen
                // contact; 1 is middle mouse and 2 is right mouse, neither of which should start a drag.
                if drag_start.get().is_some() || evt.button() != 0 {
                    return;
                }
                // Stops the browser starting its own text-selection drag from this pointerdown — see `GRAB_STYLE`.
                evt.prevent_default();
                let Some(group) = group_weak.upgrade() else { return };
                let Some(inner) = inner_weak.upgrade() else { return };
                // Can't route the drag without a way to convert client pixels into this group's own coordinates.
                let Some(inverse_ctm) = group.screen_ctm().and_then(invert_matrix) else {
                    return;
                };
                let client = Point::new(evt.client_x() as f64, evt.client_y() as f64);
                let pointer = client_to_user_space(client, inverse_ctm);

                let _ = group.as_element().set_pointer_capture(evt.pointer_id());
                let _ = group.set_attr("style", GRABBING_STYLE);
                let Ok(rect) = inner.borrow().node_rect(id) else {
                    return;
                };
                drag_start.set(Some(DragStart {
                    pointer_id: evt.pointer_id(),
                    pointer,
                    box_origin: rect.origin,
                    box_size: rect.size,
                    inverse_ctm,
                }));
            })?;
        }

        // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
        {
            let drag_start = drag_start.clone();
            let bounds = options.bounds;
            let coalescer = coalescer.clone();

            group.on_pointermove(move |evt| {
                let Some(start) = drag_start.get() else { return };
                // Ignores a different pointer's move — for example a second finger touching this element mid-drag
                // — rather than letting it drive the drag this pointer's own pointerdown started.
                if evt.pointer_id() != start.pointer_id {
                    return;
                }
                // Same reason as the pointerdown handler's own call — see `GRAB_STYLE`.
                evt.prevent_default();
                let client = Point::new(evt.client_x() as f64, evt.client_y() as f64);
                let pointer_now = client_to_user_space(client, start.inverse_ctm);

                let new_origin = Point::new(
                    start.box_origin.x + (pointer_now.x - start.pointer.x),
                    start.box_origin.y + (pointer_now.y - start.pointer.y),
                );

                // Clamps before the move, not after: this keeps a bounded node from ever being rendered outside
                // `bounds`, even for one frame. See `DragOptions::bounds`'s own doc comment for why this matters —
                // a node dropped outside its `<svg>`'s visible area renders clipped, and can no longer be clicked
                // to pick up again.
                let new_origin = match bounds {
                    Some(bounds) => clamp_to_bounds(new_origin, start.box_size, bounds),
                    None => new_origin,
                };

                // Coalesced, not applied immediately: a pointer can deliver moves far faster than the browser
                // paints — see `PointerCoalescer`'s own doc comment.
                coalescer.push(new_origin);
            })?;
        }

        // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
        // Weak clone used for the same reason as the pointerdown handler above.
        {
            let group_weak = group.downgrade();
            let inner_weak = Rc::downgrade(&self.inner);
            let drag_start = drag_start.clone();
            let collision = options.collision;
            let bounds = options.bounds;
            let coalescer = coalescer.clone();

            group.on_pointerup(move |evt| {
                let Some(group) = group_weak.upgrade() else { return };
                // Ignores a different pointer's pointerup — for example a second finger lifting while this drag's
                // own pointer is still down — rather than ending a drag that pointer never started.
                let Some(start) = drag_start.get() else { return };
                if start.pointer_id != evt.pointer_id() {
                    return;
                }
                let _ = group.as_element().release_pointer_capture(evt.pointer_id());
                let _ = group.set_attr("style", GRAB_STYLE);
                drag_start.set(None);

                // Applies any position a still-pending coalesced frame has not applied yet, so neither the node's
                // own final rendered position nor the collision-resolution rect read below is ever one frame
                // stale.
                coalescer.flush();

                // `CollisionPolicy::Allow` leaves the drop exactly where the pointer released it — nothing more to
                // do. `PushClear` pushes this node back to a clear position, along the line to where it started
                // this drag, if the drop overlaps another node.
                let CollisionPolicy::PushClear { padding } = collision else { return };
                let Some(inner) = inner_weak.upgrade() else { return };
                let Some(corrected_origin) = inner.borrow().resolve_overlap(id, start.box_origin, padding) else {
                    return;
                };
                // The collision push can itself land outside `bounds`, near an edge — clamp its result too, not
                // just pointermove's, so this correction can never undo pointermove's own clamping.
                let corrected_origin = match bounds {
                    Some(bounds) => clamp_to_bounds(corrected_origin, start.box_size, bounds),
                    None => corrected_origin,
                };
                coalescer.apply_now(corrected_origin);
            })?;
        }

        // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
        // The browser can abort a pointer sequence without ever firing pointerup — for example a touch drag interrupted
        // by a system gesture. Without this handler, drag_start would stay set, so a later stray pointermove (including
        // one for an unrelated pointer_id) would move the box using a stale drag.
        {
            let group_weak = group.downgrade();
            let drag_start = drag_start.clone();
            let coalescer = coalescer.clone();

            group.on_pointercancel(move |evt| {
                let Some(group) = group_weak.upgrade() else { return };
                // Same pointer_id check as pointerup, and for the same reason.
                if !drag_start.get().is_some_and(|start| start.pointer_id == evt.pointer_id()) {
                    return;
                }
                let _ = group.as_element().release_pointer_capture(evt.pointer_id());
                let _ = group.set_attr("style", GRAB_STYLE);
                drag_start.set(None);
                // Discards any position pushed since the last applied frame, rather than applying it — the drag
                // was interrupted, not completed. See `PointerCoalescer::cancel`'s own doc comment.
                coalescer.cancel();
            })?;
        }

        // Every listener has now registered successfully.
        //
        // Only now should the idle style be set, since a listener-registration failure above must leave `group` with no
        // style change at all, matching `InstallGuard`'s own promise to unwind back to exactly the state it found
        // `group` in.
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
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[cfg(test)]
mod unit_tests;
