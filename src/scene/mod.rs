//! Renders a graph onto the DOM, and keeps each node's and edge's rendered SVG handles alongside it.
//!
//! The topology model (crate-private while this crate's API is still taking shape) owns the topology and is the single
//! source of truth for it. This module pairs each of its ids with a rendered handle, and keeps both in sync as
//! nodes move.
//!
//! This crate has no opinion about which HTML page hosts a [`Scene`], or what graph a caller builds with one. See the
//! sibling `demo-app` crate for a small worked example.

mod box_handles;
mod connector;
pub(crate) mod drag;
pub(crate) mod node;

pub use crate::model::content::{
    BinaryOperator, ByteOrder, DataFormat, DataNodeContent, GridLayout, NodeValues, Selection, UnaryOperator,
};
pub(crate) use box_handles::BoxHandles;
pub(crate) use connector::ConnectorHandle;
pub use connector::{ConnectorOptions, ConnectorType};
pub use drag::{DragOptions, collision_policy::CollisionPolicy};
pub use node::{EdgeAnchors, NodeOptions};

use crate::{
    error::Error,
    geometry::{apply_matrix, binary_operator_anchors, elbow_path_into, nearest_clear_centre, rects_overlap},
    model::{edge::EdgeId, graph::Graph, node::NodeId},
};
use std::{
    cell::RefCell,
    rc::Rc,
    sync::atomic::{AtomicUsize, Ordering},
};
use svg_dom::{
    MarkerUnits, SvgMarker, SvgRoot,
    root::utils::{Matrix2D, Point, Rect},
};

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Converts `client` (viewport CSS pixels, such as `PointerEvent::client_x`/`client_y`) into user-space coordinates,
/// via `inverse_ctm`.
fn client_to_user_space(client: Point, inverse_ctm: Matrix2D) -> Point {
    apply_matrix(inverse_ctm, client)
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The centre point of a box's rectangle.
fn box_centre(rect: Rect) -> Point {
    Point::new(rect.origin.x + rect.size.width / 2.0, rect.origin.y + rect.size.height / 2.0)
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Assigns each `Scene` a distinct number, so its arrow marker gets an id no other `Scene` — and, so long as a caller's
/// own document doesn't deliberately collide with this crate's naming, no unrelated content either — is likely
/// to claim.
static NEXT_SCENE_ID: AtomicUsize = AtomicUsize::new(0);

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The squared distance between `a` and `b`.
///
/// The distance squared, not the actual distance. Every caller only compares the square of the distance, so all
/// comparisons can still function but without the expensive square root operation.
fn distance_sq(a: Point, b: Point) -> f64 {
    let dx = a.x - b.x;
    let dy = a.y - b.y;
    dx * dx + dy * dy
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Defines a small filled-triangle arrowhead marker in `<defs>` and returns its handle.
///
/// `ref_x`/`ref_y` place the marker's anchor point (the tip of the triangle) at the very end of the line it attaches
/// to. `orient("auto")` then rotates the marker to follow that line's own direction.
///
/// `marker_id` must be unique within `svg`'s document. A hardcoded id such as `"arrow"` would collide the moment a
/// second `Scene` shares the same `<svg>`, or the caller's own document already defines an element with that id.
fn define_arrow_marker(svg: &SvgRoot, marker_id: &str) -> Result<SvgMarker, Error> {
    let defs = svg.defs()?;
    let marker = defs.marker(marker_id)?;

    marker.set_units(MarkerUnits::UserSpaceOnUse)?;
    marker.set_marker_width(10.0)?;
    marker.set_marker_height(7.0)?;
    marker.set_ref_x(9.0)?;
    marker.set_ref_y(3.5)?;
    marker.set_orient("auto")?;
    marker.polygon(&[Point::new(0.0, 0.0), Point::new(10.0, 3.5), Point::new(0.0, 7.0)])?;

    Ok(marker)
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// A rendered `Graph`, paired with each node's and edge's own SVG handles.
///
/// `Graph` owns the topology. This owns everything DOM-specific, keyed by the same ids `Graph` hands out. `move_node`
/// (called internally by [`Scene::make_draggable`]) is the one place that keeps a moved node's rectangle, its rendered
/// box/label position, and its incident connectors all in sync.
///
/// Owns the `SvgRoot` it renders into. `Scene::new(svg)` binds them for the `SceneInner`'s whole lifetime, so every
/// node and edge in one `Scene` is guaranteed to live in the same `<svg>` document — there is no `svg` parameter on
/// [`Scene::add_node`] or [`Scene::add_edge`] through which a caller could pass a different root by mistake.
///
/// `node_handles`/`edge_handles` are stored the same way [`Graph`] stores its own nodes/edges — densely, by
/// `id.index`, append-only, with `id.graph` checked first — rather than in a `HashMap`. See [`Graph`]'s own doc
/// comment for why, and its `remove_node`/`remove_edge` for what "append-only" allows removal to still do.
struct SceneInner {
    svg: SvgRoot,
    graph: Graph,
    node_handles: Vec<BoxHandles>,
    edge_handles: Vec<ConnectorHandle>,
    arrow: SvgMarker,
    /// A single reused `d`-attribute buffer, shared by every one-shot public mutator that redraws a path —
    /// [`Scene::set_connector_type`] and [`Scene::set_edge_anchors`] — rather than each allocating its own fresh
    /// `String` on every call.
    ///
    /// A caller driving either through a live slider fires one call per input event, so a fresh allocation per call
    /// would mean one per event. Taken out via [`std::mem::take`] for the duration of a call (its callers can then
    /// freely borrow the rest of `SceneInner` without conflicting with it) and put back once the redraw is done, so
    /// its capacity — not its content — is what persists between calls.
    ///
    /// The pointer-move/pointer-up drag handlers in [`drag`](super::drag) keep their own separate, closure-captured
    /// buffer instead, reused for the lifetime of one drag rather than the whole scene — already the right shape
    /// for a handler that fires far more often, for as long as a single gesture lasts.
    scratch: String,
}

impl SceneInner {
    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    /// The current rectangle of node `id`.
    ///
    /// # Errors
    ///
    /// Returns [`Error::UnknownNode`] if `id` does not name a node in this scene's graph — for example, a `NodeId` from
    /// a different `Scene`.
    fn node_rect(&self, id: NodeId) -> Result<Rect, Error> {
        self.graph.node(id).map(|node| node.rect).ok_or(Error::UnknownNode(id))
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    /// Node `id`'s own rendered box handles, or `None` if `id` does not name a node in this scene.
    fn node_handle(&self, id: NodeId) -> Option<&BoxHandles> {
        if id.graph != self.graph.id {
            return None;
        }
        self.node_handles.get(id.index)
    }

    /// The mutable counterpart to [`node_handle`](Self::node_handle).
    fn node_handle_mut(&mut self, id: NodeId) -> Option<&mut BoxHandles> {
        if id.graph != self.graph.id {
            return None;
        }
        self.node_handles.get_mut(id.index)
    }

    /// Stores `handles` as node `id`'s own box handles. Called once, right after `id` is first added to `graph` —
    /// always in lockstep with it, so `id.index` is always exactly `self.node_handles.len()` here, the same
    /// always-an-append reasoning [`Graph::add_node`](crate::model::graph::Graph::add_node)'s own comment gives.
    fn insert_node_handle(&mut self, id: NodeId, handles: BoxHandles) {
        debug_assert_eq!(
            id.graph, self.graph.id,
            "inserted a node handle for an id from a different scene"
        );
        debug_assert_eq!(
            id.index,
            self.node_handles.len(),
            "node_handles and graph.nodes fell out of step"
        );
        self.node_handles.push(handles);
    }

    /// Removes and returns node `id`'s own box handles, or `None` if `id` does not name a node in this scene, or if
    /// `id` does not name the most recently added one — see [`Graph::remove_node`](crate::model::graph::Graph::
    /// remove_node)'s own doc comment for why only the last is ever a valid target.
    fn remove_node_handle(&mut self, id: NodeId) -> Option<BoxHandles> {
        if id.graph != self.graph.id {
            return None;
        }
        let last_index = self.node_handles.len().checked_sub(1)?;
        if id.index != last_index {
            debug_assert!(
                false,
                "SceneInner::remove_node_handle: {id:?} is not the most recently added node handle (last is index \
                 {last_index})"
            );
            return None;
        }
        self.node_handles.pop()
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    /// Edge `id`'s own rendered connector handle, or `None` if `id` does not name an edge in this scene.
    fn edge_handle(&self, id: EdgeId) -> Option<&ConnectorHandle> {
        if id.graph != self.graph.id {
            return None;
        }
        self.edge_handles.get(id.index)
    }

    /// The mutable counterpart to [`edge_handle`](Self::edge_handle).
    fn edge_handle_mut(&mut self, id: EdgeId) -> Option<&mut ConnectorHandle> {
        if id.graph != self.graph.id {
            return None;
        }
        self.edge_handles.get_mut(id.index)
    }

    /// Stores `handle` as edge `id`'s own connector handle. The same always-an-append call pattern as
    /// [`insert_node_handle`](Self::insert_node_handle), one call right after `id` is first added to `graph`.
    fn insert_edge_handle(&mut self, id: EdgeId, handle: ConnectorHandle) {
        debug_assert_eq!(
            id.graph, self.graph.id,
            "inserted an edge handle for an id from a different scene"
        );
        debug_assert_eq!(
            id.index,
            self.edge_handles.len(),
            "edge_handles and graph.edges fell out of step"
        );
        self.edge_handles.push(handle);
    }

    /// Removes and returns edge `id`'s own connector handle, or `None` if `id` does not name an edge in this scene,
    /// or if `id` does not name the most recently added one — see [`Graph::remove_edge`](crate::model::graph::
    /// Graph::remove_edge)'s own doc comment for why only the last is ever a valid target.
    fn remove_edge_handle(&mut self, id: EdgeId) -> Option<ConnectorHandle> {
        if id.graph != self.graph.id {
            return None;
        }
        let last_index = self.edge_handles.len().checked_sub(1)?;
        if id.index != last_index {
            debug_assert!(
                false,
                "SceneInner::remove_edge_handle: {id:?} is not the most recently added edge handle (last is index \
                 {last_index})"
            );
            return None;
        }
        self.edge_handles.pop()
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    /// The current [`EdgeAnchors`] configuration of node `id`.
    ///
    /// # Errors
    ///
    /// Returns [`Error::UnknownNode`] if `id` does not name a node in this scene.
    fn node_edge_anchors(&self, id: NodeId) -> Result<Option<EdgeAnchors>, Error> {
        self.node_handle(id)
            .map(|handles| handles.edge_anchors)
            .ok_or(Error::UnknownNode(id))
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    /// The `to`-side routing override for the edge from `from` to `to`, if `to` is a binary operator node and
    /// `from` is one of its own two known inputs — see [`connector::route`]'s own `to_override` parameter.
    ///
    /// `None` for every other edge: an unknown `from`/`to`, a `to` that is not a binary operator node, or a `from`
    /// that is not one of its two inputs (an edge a caller wired up by hand, bypassing
    /// [`Scene::add_binary_operator_node_with`]). Each of those falls back to `route`'s own existing default, exactly
    /// as before this existed.
    fn binary_operator_to_override(&self, from: NodeId, to: NodeId) -> Option<connector::BinaryOperatorRoute> {
        let (input_a, input_b) = self.node_handle(to)?.binary_operator_inputs?;
        // `add_binary_operator_node_with` rejects `input_a == input_b`, so this is an unambiguous, stable identity
        // — not just "which `NodeId`", but "which of the two operand *slots* this edge is" — see
        // `binary_operator_anchors`'s own doc comment for why that stability matters.
        let from_is_a = if from == input_a {
            true
        } else if from == input_b {
            false
        } else {
            return None;
        };

        let to_rect = self.node_rect(to).ok()?;
        let to_fixing_points = self.node_edge_anchors(to).ok()?.map(|EdgeAnchors(n)| n);
        let a_centre = box_centre(self.node_rect(input_a).ok()?);
        let b_centre = box_centre(self.node_rect(input_b).ok()?);

        let [(anchor_a, side_a), (anchor_b, side_b)] =
            binary_operator_anchors(to_rect, a_centre, b_centre, to_fixing_points);
        // `Some` only on the same-side branch — see `BinaryOperatorRoute::sibling_end`'s own doc comment for why a
        // different-side pair must not carry the other operand's own, unrelated-side anchor here.
        let same_side = side_a == side_b;
        let (anchor, side, sibling_anchor) = if from_is_a {
            (anchor_a, side_a, anchor_b)
        } else {
            (anchor_b, side_b, anchor_a)
        };

        Some(connector::BinaryOperatorRoute {
            anchor,
            side,
            sibling_end: same_side.then_some(sibling_anchor),
        })
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    /// If `edge_id` runs from `mover` into a binary operator node with `mover` registered as one of its own two
    /// inputs, that operator's own id. `None` otherwise: an ordinary edge, or one a caller wired by hand,
    /// bypassing [`Scene::add_binary_operator_node_with`].
    ///
    /// `move_node` redraws every edge already incident to the node that moved. For an edge this identifies, it
    /// calls [`redraw_binary_operator_inputs`](Self::redraw_binary_operator_inputs) instead of
    /// [`redraw_edge`](Self::redraw_edge) — the sibling operand's own edge into the same operator is not incident
    /// to `mover`, so nothing else would ever redraw it, and it would keep showing wherever it last computed its
    /// own anchor, stale, until something else happened to move it too.
    fn binary_operator_input_target(&self, mover: NodeId, edge_id: EdgeId) -> Option<NodeId> {
        let edge = self.graph.edge(edge_id)?;
        if edge.from != mover {
            return None;
        }
        let (input_a, input_b) = self.node_handle(edge.to)?.binary_operator_inputs?;
        (mover == input_a || mover == input_b).then_some(edge.to)
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    /// Moves node `id` to `new_origin`: updates the graph, the rendered box, and every incident connector.
    ///
    /// Every child of the node's own `<g>` — its outer rect, and its label or grid cells — was drawn once, at creation,
    /// in local coordinates relative to `(0, 0)`. See [`node::draw_box`]/[`node::draw_content_box`]. So moving the node
    /// only ever means rewriting the group's own `transform`. It never touches any child's own coordinates. This stays
    /// exactly as cheap for a data node with hundreds of value cells as for a plain label.
    ///
    /// `scratch` is a caller-owned buffer, reused across calls to avoid a fresh allocation on every move. See
    /// [`SvgNode::set_transform_fmt`] — not [`SvgNode::set_translate`], whose fixed one-decimal-place precision would
    /// quantise the rendered position away from `new_origin`, by up to 0.05 user-space units.
    ///
    /// If `new_origin` exactly matches `id`'s current origin, then we can bail out early and ourselves from redrawing
    /// an unchanged incident-edge.
    ///
    /// A binary operator input edge — either because `id` itself is a binary operator node, redrawing its own two
    /// input edges, or because `id` is one of some other operator's own two operands — is routed through
    /// [`redraw_binary_operator_inputs`](Self::redraw_binary_operator_inputs) rather than
    /// [`redraw_edge`](Self::redraw_edge), so a dragged operand's own shared-side pair is only ever computed once
    /// per frame, not once per edge. See that method's own doc comment.
    ///
    /// # Errors
    ///
    /// Returns [`Error::UnknownNode`] if `id` does not name a node in this scene.
    fn move_node(&mut self, id: NodeId, new_origin: Point, scratch: &mut String) -> Result<(), Error> {
        let rect = self.node_rect(id)?;

        // Can we bail-out early?
        if rect.origin == new_origin {
            return Ok(());
        }

        self.graph.set_node_rect(
            id,
            Rect {
                origin: new_origin,
                size: rect.size,
            },
        );

        let handles = self.node_handle(id).ok_or(Error::UnknownNode(id))?;
        handles
            .group
            .set_transform_fmt(scratch, format_args!("translate({}, {})", new_origin.x, new_origin.y))?;
        let own_input_edges = handles.binary_operator_input_edges;

        // `id` is itself a binary operator node: its own two input edges are redrawn together, once, below —
        // rather than via two separate iterations in the loop that would each recompute their shared pair
        // geometry independently.
        if own_input_edges.is_some() {
            self.redraw_binary_operator_inputs(id, scratch)?;
        }

        for edge_id in self.graph.incident_edges(id) {
            if own_input_edges.is_some_and(|(a, b)| *edge_id == a || *edge_id == b) {
                continue; // already redrawn together, above
            }
            match self.binary_operator_input_target(id, *edge_id) {
                Some(operator) => self.redraw_binary_operator_inputs(operator, scratch)?,
                None => self.redraw_edge(*edge_id, scratch)?,
            }
        }

        Ok(())
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    /// Writes `vertices` as edge `id`'s own rendered path data, rounded by `radius` — the shared tail of
    /// [`redraw_edge`](Self::redraw_edge) and [`redraw_binary_operator_inputs`](Self::redraw_binary_operator_inputs),
    /// once each has its own route ready.
    ///
    /// `scratch` is a caller-owned buffer, reused across calls to avoid allocating a fresh `String` on every
    /// move event.
    ///
    /// # Errors
    ///
    /// Returns [`Error::UnknownEdge`] if `id` does not name an edge in this scene.
    fn write_edge_path(&self, id: EdgeId, vertices: &[Point], radius: f64, scratch: &mut String) -> Result<(), Error> {
        elbow_path_into(vertices, radius, scratch);
        let handle = self.edge_handle(id).ok_or(Error::UnknownEdge(id))?;
        handle.path.set_attr("d", scratch)?;
        Ok(())
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    /// Recomputes edge `id`'s route from its current node positions, and rewrites its path data.
    ///
    /// The ordinary-edge fallback: correct for a binary operator input edge too (`binary_operator_to_override`
    /// still recomputes the full pair to answer for this one edge), but [`move_node`](Self::move_node) prefers
    /// [`redraw_binary_operator_inputs`](Self::redraw_binary_operator_inputs) for those, so that pair is computed
    /// once for both edges rather than once per `redraw_edge` call.
    ///
    /// `scratch` is a caller-owned buffer, reused across calls to avoid allocating a fresh `String` on every
    /// move event.
    ///
    /// # Errors
    ///
    /// Returns [`Error::UnknownEdge`] if `id` does not name an edge in this scene, or [`Error::UnknownNode`] if either
    /// of its endpoints no longer does.
    fn redraw_edge(&self, id: EdgeId, scratch: &mut String) -> Result<(), Error> {
        let edge = self.graph.edge(id).ok_or(Error::UnknownEdge(id))?;
        let from_rect = self.node_rect(edge.from)?;
        let from_anchors = self.node_edge_anchors(edge.from)?;
        let to_rect = self.node_rect(edge.to)?;
        let to_anchors = self.node_edge_anchors(edge.to)?;
        let to_override = self.binary_operator_to_override(edge.from, edge.to);
        let connector_type = self.edge_handle(id).ok_or(Error::UnknownEdge(id))?.connector_type;
        let (vertices, radius) =
            connector::route(connector_type, from_rect, from_anchors, to_rect, to_anchors, to_override);
        self.write_edge_path(id, &vertices, radius, scratch)
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    /// Redraws both of binary operator node `operator`'s own two input edges together: computes the shared pair
    /// geometry — [`binary_operator_anchors`], and which side each input lands on — exactly once, then derives and
    /// writes each edge's own route from it.
    ///
    /// [`move_node`](Self::move_node) calls this instead of two separate [`redraw_edge`](Self::redraw_edge) calls
    /// whenever an edge it would otherwise redraw is one of a binary operator's own registered inputs. Two
    /// independent `redraw_edge` calls would each recompute this same pair from scratch, via
    /// [`binary_operator_to_override`](Self::binary_operator_to_override) — once for dragging either operand, and
    /// again for moving the operator itself, whose own two incident edges are exactly this pair.
    ///
    /// Does nothing if `operator` does not name a binary operator node in this scene with both its own input edges
    /// still wired — for example, a plain node, or (mid-construction only, never observable afterward) an operator
    /// whose own rollback is still in progress.
    ///
    /// # Errors
    ///
    /// Returns [`Error::UnknownNode`]/[`Error::UnknownEdge`] if `operator`'s own cached operand/edge ids no longer
    /// resolve. Not expected in practice — nothing in this crate's own public API can remove a node or edge once
    /// added.
    fn redraw_binary_operator_inputs(&self, operator: NodeId, scratch: &mut String) -> Result<(), Error> {
        let Some(handles) = self.node_handle(operator) else {
            return Ok(());
        };
        let (Some((input_a, input_b)), Some((edge_a, edge_b))) =
            (handles.binary_operator_inputs, handles.binary_operator_input_edges)
        else {
            return Ok(());
        };

        let to_rect = self.node_rect(operator)?;
        let to_edge_anchors = self.node_edge_anchors(operator)?;
        let to_fixing_points = to_edge_anchors.map(|EdgeAnchors(n)| n);
        let a_rect = self.node_rect(input_a)?;
        let b_rect = self.node_rect(input_b)?;
        let [(anchor_a, side_a), (anchor_b, side_b)] =
            binary_operator_anchors(to_rect, box_centre(a_rect), box_centre(b_rect), to_fixing_points);
        // `Some` only on the same-side branch — see `BinaryOperatorRoute::sibling_end`'s own doc comment.
        let same_side = side_a == side_b;

        let connector_type_a = self.edge_handle(edge_a).ok_or(Error::UnknownEdge(edge_a))?.connector_type;
        let (vertices_a, radius_a) = connector::route(
            connector_type_a,
            a_rect,
            self.node_edge_anchors(input_a)?,
            to_rect,
            to_edge_anchors,
            Some(connector::BinaryOperatorRoute {
                anchor: anchor_a,
                side: side_a,
                sibling_end: same_side.then_some(anchor_b),
            }),
        );
        self.write_edge_path(edge_a, &vertices_a, radius_a, scratch)?;

        let connector_type_b = self.edge_handle(edge_b).ok_or(Error::UnknownEdge(edge_b))?.connector_type;
        let (vertices_b, radius_b) = connector::route(
            connector_type_b,
            b_rect,
            self.node_edge_anchors(input_b)?,
            to_rect,
            to_edge_anchors,
            Some(connector::BinaryOperatorRoute {
                anchor: anchor_b,
                side: side_b,
                sibling_end: same_side.then_some(anchor_a),
            }),
        );
        self.write_edge_path(edge_b, &vertices_b, radius_b, scratch)
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    /// Redraws edge `id` with `connector_type`, and only then records it as the edge's new type.
    ///
    /// Writing the path before committing the type keeps `Scene::set_connector_type` transactional. If the DOM write
    /// fails partway through — [`SvgNode::set_attr`] can itself fail — the stored `connector_type` is left exactly as
    /// it was. It never claims a route the rendered path does not actually show.
    ///
    /// `scratch` is a caller-owned buffer, reused across calls to avoid allocating a fresh `String` on every
    /// move event.
    ///
    /// # Errors
    ///
    /// Returns [`Error::UnknownEdge`] if `id` does not name an edge in this scene, or [`Error::UnknownNode`] if either
    /// of its endpoints no longer does.
    fn redraw_edge_with_type(
        &mut self,
        id: EdgeId,
        connector_type: ConnectorType,
        scratch: &mut String,
    ) -> Result<(), Error> {
        let edge = self.graph.edge(id).ok_or(Error::UnknownEdge(id))?;
        let from_rect = self.node_rect(edge.from)?;
        let from_anchors = self.node_edge_anchors(edge.from)?;
        let to_rect = self.node_rect(edge.to)?;
        let to_anchors = self.node_edge_anchors(edge.to)?;
        let to_override = self.binary_operator_to_override(edge.from, edge.to);
        let (vertices, radius) =
            connector::route(connector_type, from_rect, from_anchors, to_rect, to_anchors, to_override);
        elbow_path_into(&vertices, radius, scratch);

        let handle = self.edge_handle_mut(id).ok_or(Error::UnknownEdge(id))?;
        handle.path.set_attr("d", scratch)?;
        handle.connector_type = connector_type;

        Ok(())
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    /// If node `id`'s current rect overlaps another node's, returns a corrected origin that resolves the overlap.
    ///
    /// Pushes `id`'s rect back along the straight line from `pre_drag_origin` — `id`'s own position before the drag
    /// that produced its current, overlapping position — through the overlapped node's centre, stopping just clear of
    /// that node's boundary, plus `padding` user-space units.
    ///
    /// When `id`'s rect overlaps more than one other node, it resolves against whichever overlapping node's centre is
    /// nearest to `id`'s own current centre. Ties are broken by `NodeId`'s index, so the choice stays deterministic
    /// rather than depending on `HashMap`'s unspecified iteration order.
    ///
    /// This does not attempt to resolve every simultaneous overlap in one pass: a resolved position could still overlap
    /// a different node than the one resolved against. See [`CollisionPolicy::PushClear`]'s own doc comment for why
    /// this is a best-effort correction, not a guarantee.
    ///
    /// If `pre_drag_origin`'s centre coincides exactly with the blocking node's own centre, there is no direction to
    /// retreat along, and [`nearest_clear_centre`] returns the blocker's own centre unchanged. This is handled
    /// explicitly by falling back to `pre_drag_origin` here, rather than converting that returned centre back to an
    /// origin via `dragged`'s size and relying on the two being numerically identical — which they always are in this
    /// case (`blocker_centre - dragged.size / 2 == pre_drag_origin` follows directly from `pre_drag_centre ==
    /// blocker_centre`), but only because of that algebraic identity, not because the conversion was written with this
    /// case in mind. Spelling it out here keeps that guarantee from depending on `nearest_clear_centre`'s internals
    /// never changing.
    ///
    /// Returns `None` if `id`'s current rect does not overlap any other node, or if `id` does not name a node in
    /// this scene.
    fn resolve_overlap(&self, id: NodeId, pre_drag_origin: Point, padding: f64) -> Option<Point> {
        let dragged = self.graph.node(id)?.rect;
        let dragged_centre = box_centre(dragged);

        let blocker = self
            .graph
            .nodes
            .iter()
            .enumerate()
            .map(|(index, node)| (NodeId { graph: self.graph.id, index }, node))
            .filter(|&(other_id, other)| other_id != id && rects_overlap(dragged, other.rect))
            .min_by(|&(id_a, a), &(id_b, b)| {
                // Ties (two blockers exactly equidistant from `dragged_centre`) break on `index`, so the choice
                // stays deterministic rather than depending on iteration order.
                distance_sq(dragged_centre, box_centre(a.rect))
                    .total_cmp(&distance_sq(dragged_centre, box_centre(b.rect)))
                    .then_with(|| id_a.index.cmp(&id_b.index))
            })
            .map(|(_, other)| other.rect)?;

        let pre_drag_centre = box_centre(Rect {
            origin: pre_drag_origin,
            size: dragged.size,
        });
        if pre_drag_centre == box_centre(blocker) {
            return Some(pre_drag_origin);
        }

        let new_centre = nearest_clear_centre(blocker, dragged.size, pre_drag_centre, padding);
        Some(Point::new(
            new_centre.x - dragged.size.width / 2.0,
            new_centre.y - dragged.size.height / 2.0,
        ))
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// A cheap, cloneable handle to a rendered graph.
///
/// Internally an `Rc<RefCell<SceneInner>>` — this crate owns that sharing strategy, not the caller. A `Scene` can be
/// cloned freely (every clone refers to the same underlying graph and DOM state) and its methods take `&self`, not
/// `&mut self`, so a caller never has to wrap it in `Rc<RefCell<_>>` themselves just to call
/// [`make_draggable`](Self::make_draggable) or to share it with more than one closure.
///
/// # Keep at least one handle alive for as long as the scene should stay interactive
///
/// [`make_draggable`](Self::make_draggable)'s own listener closures deliberately hold only `Weak` references back to
/// this scene's shared state, not strong ones — a strong self-reference there would leak the whole scene (and every
/// node, edge, and DOM element it owns) forever, since nothing would ever be able to drop the last strong handle.
///
/// The consequence: once every `Scene` handle a caller holds is dropped, the scene's shared state is freed immediately,
/// and every listener silently stops responding — no panic, nothing in the console. This is easy to trip over in
/// exactly the shape a `#[wasm_bindgen(start)]` entry point naturally takes:
///
/// ```rust,no_run
/// # use svg_dom::{SvgRoot, root::utils::{Point, Size}};
/// # use svg_dom_graph::{Error, scene::Scene};
/// fn build() -> Result<(), Error> {
///     let svg = SvgRoot::attach("diagram")?;
///     let scene = Scene::new(svg)?;
///     let node = scene.add_node(Point::new(0.0, 0.0), Size::new(90.0, 50.0), "Node")?;
///     scene.make_draggable(node)?;
///     Ok(())
///     // `scene` drops here, at the end of this function — which for a `#[wasm_bindgen(start)]` entry point
///     // happens at page load, long before the user ever gets a chance to click anything. Dragging silently
///     // does nothing.
/// }
/// ```
///
/// Keep a handle alive somewhere that outlives the function that built it — for example, in a `thread_local!` for the
/// page's whole lifetime, as `demo-app`'s own `SCENE` does.
#[derive(Clone)]
pub struct Scene {
    inner: Rc<RefCell<SceneInner>>,
}

impl Scene {
    /// Creates an empty scene, ready to hold nodes and edges within `svg`.
    ///
    /// Also defines the arrow marker every edge's connector uses, since every `Scene` needs exactly one, shared across
    /// all its edges.
    pub fn new(svg: SvgRoot) -> Result<Self, Error> {
        let marker_id = format!("svg-dom-graph-arrow-{}", NEXT_SCENE_ID.fetch_add(1, Ordering::Relaxed));
        let arrow = define_arrow_marker(&svg, &marker_id)?;
        Ok(Self {
            inner: Rc::new(RefCell::new(SceneInner {
                svg,
                graph: Graph::new(),
                node_handles: Vec::new(),
                edge_handles: Vec::new(),
                arrow,
                scratch: String::new(),
            })),
        })
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[cfg(test)]
mod unit_tests;
