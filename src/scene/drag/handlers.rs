//! The four pointer handlers a draggable node registers, one function each: [`pointerdown`], [`pointermove`],
//! [`pointerup`] and [`pointercancel`].
//!
//! Each takes everything it needs by value and returns the closure to register. So [`Scene::make_draggable_with`] reads as
//! install, not as a hundred lines of behaviour, and a handler's own captures are listed in one place.
//!
//! None may capture `group` or the scene's shared state strongly. Both would make an ownership cycle. A strong `group`
//! makes `SvgNodeInner -> listener store -> closure -> SvgNode -> the same SvgNodeInner`, which leaks the node and
//! defeats its automatic listener cleanup (see `WeakSvgNode`'s own doc comment). A strong `inner` makes one a level up,
//! because `SceneInner::node_handles` owns `group`: `SceneInner -> group -> listener store -> closure -> SceneInner`.
//! That would leak the whole scene, plus everything it renders and every listener on every node in it, even after every
//! external `Scene` handle has been dropped. So each handler holds only weak references, and upgrades them when it runs.

use super::{
    DragState, GRAB_STYLE, GRABBING_STYLE, client_to_user_space, collision_policy::CollisionPolicy,
    drag_start::DragStart, pointer_coalescer::PointerCoalescer,
};
use crate::{
    geometry::{clamp_to_bounds, invert_matrix},
    model::node::NodeId,
    scene::scene_inner::SceneInner,
};
use std::{cell::RefCell, rc::Weak};
use svg_dom::{
    WeakSvgNode,
    root::utils::{Point, Rect},
};
use web_sys::PointerEvent;

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Starts a drag, if this is the primary button and no drag is already active.
pub(super) fn pointerdown(
    id: NodeId,
    group_weak: WeakSvgNode,
    inner_weak: Weak<RefCell<SceneInner>>,
    drag_start: DragState,
) -> impl FnMut(PointerEvent) + 'static {
    move |evt| {
        // Ignores a pointerdown while a drag is already active. Otherwise a second pointer touching this
        // element mid-drag would silently steal it. It would overwrite the first pointer's `DragStart` before
        // that pointer's own pointerup/pointercancel ever fires. Also ignores anything but the primary button.
        // `button() == 0` is left mouse, touch, or ordinary pen contact. 1 is middle mouse and 2 is right
        // mouse, neither of which should start a drag.
        if drag_start.get().is_some() || evt.button() != 0 {
            return;
        }
        // Stops the browser starting its own text-selection drag from this pointerdown — see `GRAB_STYLE`.
        evt.prevent_default();
        let Some(group) = group_weak.upgrade() else { return };
        let Some(inner) = inner_weak.upgrade() else { return };
        // A zoom or pan from the wheel, keyboard, or a button is written to the DOM one animation frame after
        // it is made. Settle it first, so the screen matrix read below shows the view the scene really has, and
        // not the one before it.
        let _ = inner.borrow_mut().flush_view();
        // Can't route the drag without a way to convert client pixels into this group's own coordinates.
        let Some(inverse_ctm) = group.screen_ctm().and_then(invert_matrix) else {
            return;
        };
        let client = Point::new(evt.client_x() as f64, evt.client_y() as f64);
        let pointer = client_to_user_space(client, inverse_ctm);

        let _ = group.as_element().set_pointer_capture(evt.pointer_id());
        let _ = group.set_attr("style", GRABBING_STYLE);
        let (rect, view) = {
            let inner = inner.borrow();
            let Ok(rect) = inner.node_rect(id) else { return };
            (rect, inner.view)
        };
        drag_start.set(Some(DragStart {
            pointer_id: evt.pointer_id(),
            pointer,
            box_origin: rect.origin,
            box_size: rect.size,
            inverse_ctm,
            view,
        }));
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Moves the node under the pointer, clamped to `bounds`, for the pointer that started the drag only.
pub(super) fn pointermove(
    bounds: Option<Rect>,
    inner_weak: Weak<RefCell<SceneInner>>,
    drag_start: DragState,
    coalescer: PointerCoalescer,
) -> impl FnMut(PointerEvent) + 'static {
    move |evt| {
        let Some(start) = drag_start.get() else { return };
        // Ignores a different pointer's move — for example a second finger touching this element mid-drag —
        // rather than letting it drive the drag this pointer's own pointerdown started.
        if evt.pointer_id() != start.pointer_id {
            return;
        }
        // Same reason as the pointerdown handler's own call — see `GRAB_STYLE`.
        evt.prevent_default();
        let client = Point::new(evt.client_x() as f64, evt.client_y() as f64);
        let pointer_now = client_to_user_space(client, start.inverse_ctm);

        // Where the pointer is, in content coordinates, as the view was when the drag began.
        let under_pointer = Point::new(start.box_origin.x + pointer_now.x, start.box_origin.y + pointer_now.y);
        // The view can have changed since — by the wheel, the keyboard, a button, or the application. The same
        // pointer position is then over a different point of content, so read it again under the view as it is
        // now. This is `under_pointer` itself, unchanged, when nothing has moved.
        let view_now = inner_weak.upgrade().map_or(start.view, |inner| inner.borrow().view);
        let under_pointer = start.view.reinterpret(under_pointer, view_now);

        // The node keeps hold of the same point of itself, `start.pointer` from its corner, under the pointer.
        let new_origin = Point::new(under_pointer.x - start.pointer.x, under_pointer.y - start.pointer.y);

        // Clamps before the move, not after: this keeps a bounded node from ever being rendered outside
        // `bounds`, even for one frame. See `DragOptions::bounds`'s own doc comment for why this matters — a
        // node dropped outside its `<svg>`'s visible area renders clipped, and can no longer be clicked to pick
        // up again.
        let new_origin = match bounds {
            Some(bounds) => clamp_to_bounds(new_origin, start.box_size, bounds),
            None => new_origin,
        };

        // Coalesced, not applied immediately: a pointer can deliver moves far faster than the browser paints —
        // see `PointerCoalescer`'s own doc comment.
        coalescer.push(new_origin);
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Ends the drag, applies any pending position, and resolves an overlap per `collision`.
pub(super) fn pointerup(
    id: NodeId,
    collision: CollisionPolicy,
    bounds: Option<Rect>,
    group_weak: WeakSvgNode,
    inner_weak: Weak<RefCell<SceneInner>>,
    drag_start: DragState,
    coalescer: PointerCoalescer,
) -> impl FnMut(PointerEvent) + 'static {
    move |evt| {
        let Some(group) = group_weak.upgrade() else { return };
        // Ignores a different pointer's pointerup — for example a second finger lifting while this drag's own
        // pointer is still down — rather than ending a drag that pointer never started.
        let Some(start) = drag_start.get() else { return };
        if start.pointer_id != evt.pointer_id() {
            return;
        }
        let _ = group.as_element().release_pointer_capture(evt.pointer_id());
        let _ = group.set_attr("style", GRAB_STYLE);
        drag_start.set(None);

        // Applies any position a still-pending coalesced frame has not applied yet, so neither the node's own
        // final rendered position nor the collision-resolution rect read below is ever one frame stale.
        coalescer.flush();

        // `CollisionPolicy::Allow` leaves the drop exactly where the pointer released it — nothing more to do.
        // `PushClear` pushes this node back to a clear position, along the line to where it started this drag,
        // if the drop overlaps another node.
        let CollisionPolicy::PushClear { padding } = collision else { return };
        let Some(inner) = inner_weak.upgrade() else { return };
        let Some(corrected_origin) = inner.borrow().resolve_overlap(id, start.box_origin, padding) else {
            return;
        };
        // The collision push can itself land outside `bounds`, near an edge — clamp its result too, not just
        // pointermove's, so this correction can never undo pointermove's own clamping.
        let corrected_origin = match bounds {
            Some(bounds) => clamp_to_bounds(corrected_origin, start.box_size, bounds),
            None => corrected_origin,
        };
        coalescer.apply_now(corrected_origin);
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Abandons the drag. The browser can abort a pointer sequence without ever firing pointerup, for example a touch drag
/// interrupted by a system gesture. Without this handler, `drag_start` would stay set, so a later stray pointermove
/// (including one for an unrelated pointer id) would move the box using a stale drag.
pub(super) fn pointercancel(
    group_weak: WeakSvgNode,
    drag_start: DragState,
    coalescer: PointerCoalescer,
) -> impl FnMut(PointerEvent) + 'static {
    move |evt| {
        let Some(group) = group_weak.upgrade() else { return };
        // Same pointer_id check as pointerup, and for the same reason.
        if !drag_start.get().is_some_and(|start| start.pointer_id == evt.pointer_id()) {
            return;
        }
        let _ = group.as_element().release_pointer_capture(evt.pointer_id());
        let _ = group.set_attr("style", GRAB_STYLE);
        drag_start.set(None);
        // Discards any position pushed since the last applied frame, rather than applying it — the drag was
        // interrupted, not completed. See `PointerCoalescer::cancel`'s own doc comment.
        coalescer.cancel();
    }
}
