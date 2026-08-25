use super::{Scene, client_to_user_space};
use crate::{error::Error, geometry::invert_matrix, model::node::NodeId};
use std::{cell::Cell, rc::Rc};
use svg_dom::root::utils::{Matrix2D, Point};

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Style applied while a box is idle: a grab cursor, no touch scrolling/panning, and no native text selection.
///
/// `user-select: none` alone does not reliably suppress a click-drag text selection in every engine — Safari in
/// particular has still started one with only the CSS property set — so `make_draggable` also calls
/// `prevent_default()` on `pointerdown`/`pointermove`. The two are kept together: CSS blocks selection from a mouse
/// drag that starts outside this element and passes over it without ever firing this element's own `pointerdown`,
/// while `prevent_default()` blocks it for the drag this element's own listeners actually see.
const GRAB_STYLE: &str = "cursor: grab; touch-action: none; user-select: none; -webkit-user-select: none;";
/// Style applied while a box is actively being dragged — same as [`GRAB_STYLE`], but with a grabbing cursor.
const GRABBING_STYLE: &str = "cursor: grabbing; touch-action: none; user-select: none; -webkit-user-select: none;";

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// What [`Scene::make_draggable_with`] does when a drop leaves the dragged node overlapping another one.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum CollisionPolicy {
    /// Leaves the dropped node exactly where the pointer released it, even if that overlaps another node.
    ///
    /// Choose this when overlapping nodes are a legitimate outcome for the caller's own graph — this crate does
    /// not otherwise have an opinion on whether nodes may overlap.
    Allow,
    /// Pushes the dropped node back along the line from its pre-drag position, clear of whatever it overlaps.
    ///
    /// This is a best-effort, single-pass correction, not a guarantee that the node ends up clear of every other
    /// node. It resolves against only the nearest node the drop overlaps — the corrected position can still
    /// overlap a different node than the one it was pushed clear of. [`Scene::add_node`] also does not itself
    /// reject an overlapping starting position, so a hard "nodes never overlap" invariant is not achievable by
    /// this policy in general, only reduced.
    PushClear {
        /// Extra clearance kept between the dropped node and whatever it overlapped, in this scene's user-space
        /// units, so the two end up with a visible gap rather than touching edges.
        padding: f64,
    },
}

/// Configures the pointer-drag behaviour [`Scene::make_draggable_with`] wires up for one node.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DragOptions {
    /// What happens when a drop leaves the dragged node overlapping another one — see [`CollisionPolicy`].
    pub collision: CollisionPolicy,
}

impl Default for DragOptions {
    /// [`CollisionPolicy::PushClear`] with 6 user-space units of padding: [`Scene::make_draggable`]'s behaviour.
    fn default() -> Self {
        Self {
            collision: CollisionPolicy::PushClear { padding: 6.0 },
        }
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The pointer position and box origin recorded when a drag starts.
///
/// The delta between the pointer's current position and `pointer` describes how far to move `box_origin`. Both are in
/// the dragged box's own user-space coordinates, not viewport CSS pixels — see `inverse_ctm`.
#[derive(Clone, Copy)]
struct DragStart {
    /// The pointer that started this drag.
    ///
    /// A pointer's own `pointerdown` grants it exclusive capture (see `set_pointer_capture` below), but a `pointermove`
    /// `pointerup` or `pointercancel` for a *different*, unrelated pointer can still reach this same listener. For
    /// example a second finger touching the same element mid-drag.
    ///
    /// Checking this field against each such event's own id avoids the case in which a different pointer attempte to
    /// drive or end another pointer's drag event.
    pointer_id: i32,
    pointer: Point,
    box_origin: Point,
    /// The dragged group's screen CTM, inverted once at pointerdown and reused for the duration of this drag event.
    ///
    /// `SvgNode::screen_ctm()` may force a synchronous layout, so this is captured once per drag rather than on every
    /// pointermove. Caching it here (rather than recomputing it per drag event) assumes that neither the group's own
    /// transform nor any ancestor transform up to the viewport, changes mid-drag.
    ///
    /// This is true for this crate's current rendering, since nothing sets a transform on a box's group after it has
    /// been drawn.
    inverse_ctm: Matrix2D,
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
impl Scene {
    /// Wires up pointer dragging for node `id`, with [`DragOptions::default`]'s collision behaviour: a drop that
    /// overlaps another node is pushed back clear of it, along the line to where the drag started, plus 6
    /// user-space units of padding.
    ///
    /// See [`make_draggable_with`](Self::make_draggable_with) to allow overlapping nodes, or to use different
    /// padding.
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
    /// `PointerEvent::client_x`/`client_y` are viewport CSS pixels, not this scene's user-space coordinates — the
    /// two only coincide when the `<svg>` has no CSS scaling and its `viewBox` matches its pixel size exactly.
    /// This converts through the dragged group's own screen CTM (see `invert_matrix`/`apply_matrix` in
    /// `geometry`), so dragging stays correct under scaling, a resized `viewBox`, or CSS transforms.
    ///
    /// # Errors
    ///
    /// Returns [`Error::UnknownNode`] if `id` does not name a node in this scene — for example, a `NodeId` from a
    /// different `Scene`.
    ///
    /// Returns [`Error::AlreadyDraggable`] if `id` is already draggable — calling this (or [`Scene::make_draggable`])
    /// a second time for the same node does not replace the first installation, so this is rejected outright rather
    /// than silently doubling up its listeners and drag-state. Reusing `id` after such an error is safe: the first
    /// installation is untouched.
    pub fn make_draggable_with(&self, id: NodeId, options: DragOptions) -> Result<(), Error> {
        let group = {
            let mut inner = self.inner.borrow_mut();
            let handles = inner.node_handles.get_mut(&id).ok_or(Error::UnknownNode(id))?;
            if handles.draggable {
                return Err(Error::AlreadyDraggable(id));
            }
            handles.draggable = true;
            handles.group.clone()
        };
        group.set_attr("style", GRAB_STYLE)?;

        let drag_start: Rc<Cell<Option<DragStart>>> = Rc::new(Cell::new(None));

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
        // listener on every node in that scene).  This would happend even after every external `Scene` handle has been
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
                let Some(box_origin) = inner.borrow().node_rect(id).ok().map(|rect| rect.origin) else {
                    return;
                };
                drag_start.set(Some(DragStart {
                    pointer_id: evt.pointer_id(),
                    pointer,
                    box_origin,
                    inverse_ctm,
                }));
            })?;
        }

        // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
        // Weak clone used for the same reason as the pointerdown handler above.
        {
            let inner_weak = Rc::downgrade(&self.inner);
            let drag_start = drag_start.clone();
            // Reused across every pointermove call in this drag — and across drags, since the closure's
            // environment persists between invocations — rather than allocating a fresh String each time. See
            // `SvgNode::set_attr_display`'s own doc comment for why this pattern exists.
            let mut scratch = String::new();

            group.on_pointermove(move |evt| {
                let Some(inner) = inner_weak.upgrade() else { return };
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

                let _ = inner.borrow_mut().move_node(id, new_origin, &mut scratch);
            })?;
        }

        // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
        // Weak clone used for the same reason as the pointerdown handler above.
        {
            let group_weak = group.downgrade();
            let inner_weak = Rc::downgrade(&self.inner);
            let drag_start = drag_start.clone();
            let collision = options.collision;
            // Reused for the corrective `move_node` call this handler makes when a drop overlaps another node —
            // same reasoning as the pointermove handler's own `scratch` above.
            let mut scratch = String::new();

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

                // `CollisionPolicy::Allow` leaves the drop exactly where the pointer released it — nothing more to
                // do. `PushClear` pushes this node back to a clear position, along the line to where it started
                // this drag, if the drop overlaps another node.
                let CollisionPolicy::PushClear { padding } = collision else { return };
                let Some(inner) = inner_weak.upgrade() else { return };
                let corrected = inner.borrow().resolve_overlap(id, start.box_origin, padding);
                if let Some(corrected_origin) = corrected {
                    let _ = inner.borrow_mut().move_node(id, corrected_origin, &mut scratch);
                }
            })?;
        }

        // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
        // The browser can abort a pointer sequence without ever firing pointerup — for example a touch drag interrupted
        // by a system gesture. Without this handler, drag_start would stay set, so a later stray pointermove (including
        // one for an unrelated pointer_id) would move the box using a stale drag.
        {
            let group_weak = group.downgrade();
            let drag_start = drag_start.clone();

            group.on_pointercancel(move |evt| {
                let Some(group) = group_weak.upgrade() else { return };
                // Same pointer_id check as pointerup, and for the same reason.
                if !drag_start.get().is_some_and(|start| start.pointer_id == evt.pointer_id()) {
                    return;
                }
                let _ = group.as_element().release_pointer_capture(evt.pointer_id());
                let _ = group.set_attr("style", GRAB_STYLE);
                drag_start.set(None);
            })?;
        }

        Ok(())
    }
}
