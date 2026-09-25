pub(super) use super::node::EdgeAnchors;
pub(crate) use super::{box_handles::BoxHandles, connector::ConnectorHandle};
pub(super) use super::{
    toolbar::Toolbar,
    view_input::{InputMode, ViewInput},
};

use super::*;
use crate::{
    error::Error,
    geometry::{
        binary_operator_anchors, elbow_path_into, nearest_clear_centre, port_marker_position, rects_overlap,
        side::Side, view::ViewTransform,
    },
    model::{edge::EdgeId, graph::Graph, node::NodeId},
};
use svg_dom::{
    SvgMarker, SvgNode, SvgRoot,
    root::utils::{Point, Rect},
};

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
pub(super) struct SceneInner {
    pub svg: SvgRoot,
    /// The `<g>` layer every node, connector, and port marker lives in, so a single `transform` on it zooms and pans
    /// the whole graph. Anything that must not scale with the graph, such as the button bar, is a sibling of this
    /// layer under `svg`, never a child of it.
    pub content: SvgNode,
    /// The current zoom and pan applied to [`content`](Self::content).
    pub view: ViewTransform,
    /// Whether [`view`](Self::view) has changed since it was last written to the content layer's `transform`.
    ///
    /// High-frequency gestures (wheel zoom, panning) update `view` at once but leave the DOM write to one animation
    /// frame — see [`toolbar`](super::toolbar)'s own `frame` module.
    pub view_dirty: bool,
    /// The button bar, if one is currently shown. See [`toolbar`](super::toolbar).
    pub toolbar: Option<Toolbar>,
    /// When dragging the background pans the content. See [`InputMode`].
    pub pan_mode: InputMode,
    /// When Ctrl or Cmd plus the wheel zooms the content. See [`InputMode`].
    pub wheel_zoom_mode: InputMode,
    /// The surface behind the content layer that both gestures work through, while either is active.
    pub view_input: Option<ViewInput>,
    pub graph: Graph,
    pub node_handles: Vec<BoxHandles>,
    pub edge_handles: Vec<ConnectorHandle>,
    pub arrow: SvgMarker,
    /// A single reused buffer, shared by every one-shot construction/redraw call that needs to format a path `d` or
    /// element attribute — [`Scene::add_edge_with`], [`Scene::set_connector_type`], [`Scene::set_edge_anchors`],
    /// and node construction (`draw_box`/`draw_content_box`/`draw_operator_box`, called from
    /// [`Scene::add_node_with`], [`Scene::add_data_node_with`], and the operator constructors) — rather than each
    /// allocating its own fresh `String`.
    ///
    /// A caller driving a redraw through a live slider fires one call per input event, so a fresh allocation per
    /// call would mean one per event; a caller building many nodes in a loop would likewise mean one per node.
    /// Taken out via [`std::mem::take`] for the duration of a call (its callers can then freely borrow the rest of
    /// `SceneInner` without conflicting with it) and put back once the call is done, so its capacity — not its
    /// content — is what persists between calls.
    ///
    /// The pointer-move/pointer-up drag handlers in [`drag`](super::drag) keep their own separate, closure-captured
    /// buffer instead, reused for the lifetime of one drag rather than the whole scene — already the right shape
    /// for a handler that fires far more often, for as long as a single gesture lasts.
    pub scratch: String,
}

impl SceneInner {
    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    /// Moves `element` — freshly created by one of `svg`'s own factory methods, which always append to the `<svg>`
    /// root — into the [`content`](Self::content) layer.
    ///
    /// Removes `element` again if the move fails, so a failed call never leaves a stray element behind at the root.
    /// Every caller creates its element and then attaches it here, before touching the graph model. So a failure is
    /// reported with the model still unchanged.
    ///
    /// # Errors
    ///
    /// Returns whatever error the underlying DOM append reports.
    pub(super) fn attach(&self, element: &SvgNode) -> Result<(), Error> {
        Ok(self.content.append(element).inspect_err(|_| element.remove())?)
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    /// The current rectangle of node `id`.
    ///
    /// # Errors
    ///
    /// Returns [`Error::UnknownNode`] if `id` does not name a node in this scene's graph — for example, a `NodeId` from
    /// a different `Scene`.
    pub(super) fn node_rect(&self, id: NodeId) -> Result<Rect, Error> {
        self.graph.node(id).map(|node| node.rect).ok_or(Error::UnknownNode(id))
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    /// Node `id`'s own rendered box handles, or `None` if `id` does not name a node in this scene.
    pub(super) fn node_handle(&self, id: NodeId) -> Option<&BoxHandles> {
        if id.graph != self.graph.id {
            return None;
        }
        self.node_handles.get(id.index)
    }

    /// The mutable counterpart to [`node_handle`](Self::node_handle).
    pub(super) fn node_handle_mut(&mut self, id: NodeId) -> Option<&mut BoxHandles> {
        if id.graph != self.graph.id {
            return None;
        }
        self.node_handles.get_mut(id.index)
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    /// Appends `clause` to node `id`'s own `aria-label`/`<title>`, as a new sentence.
    ///
    /// `Scene::add_edge_with` calls this once per endpoint of every edge it adds. So a connector's own `<path>` is
    /// never the only place that conveys which node feeds which. See `BoxHandles::ref_name`'s own doc comment for
    /// the accessibility reasoning behind naming nodes at all.
    ///
    /// A plain label node starts with an empty `aria_label` — its own visible text already serves as its accessible
    /// name, so nothing ever set one. Seeds it with `ref_name` first in that case, so this call adds to that name
    /// instead of silently replacing it once `aria-label` is set.
    ///
    /// Advances `base_label_len` past whatever this call appends. So a later `Scene::set_selection` on the same
    /// node truncates back to the base description plus every relationship clause appended so far, never past one.
    ///
    /// # Errors
    ///
    /// Returns [`Error::UnknownNode`] if `id` does not name a node in this scene.
    pub(super) fn append_relationship(&mut self, id: NodeId, clause: &str) -> Result<(), Error> {
        let handles = self.node_handle_mut(id).ok_or(Error::UnknownNode(id))?;
        if handles.aria_label.is_empty() {
            handles.aria_label.push_str(&handles.ref_name);
        }
        // Only a node's own base description, or its first clause, can lack a trailing period here. A clause
        // already appended always leaves one, so a second clause never doubles it.
        if !handles.aria_label.ends_with('.') {
            handles.aria_label.push('.');
        }
        handles.aria_label.push(' ');
        handles.aria_label.push_str(clause);
        handles.aria_label.push('.');
        handles.base_label_len = handles.aria_label.len();
        handles.group.set_attr("aria-label", &handles.aria_label)?;
        // Keeps the browser's own mouse-hover tooltip reading exactly the same text as `aria-label` — see
        // `draw_content_box`'s own doc comment on why `<title>` is set to that same text at construction.
        handles.group.set_title(&handles.aria_label)?;
        Ok(())
    }

    /// Stores `handles` as node `id`'s own box handles. Called once, right after `id` is first added to `graph` —
    /// always in lockstep with it, so `id.index` is always exactly `self.node_handles.len()` here, the same
    /// always-an-append reasoning [`Graph::add_node`](crate::model::graph::Graph::add_node)'s own comment gives.
    pub(super) fn insert_node_handle(&mut self, id: NodeId, handles: BoxHandles) {
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
    pub(super) fn remove_node_handle(&mut self, id: NodeId) -> Option<BoxHandles> {
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
    pub(super) fn edge_handle(&self, id: EdgeId) -> Option<&ConnectorHandle> {
        if id.graph != self.graph.id {
            return None;
        }
        self.edge_handles.get(id.index)
    }

    /// The mutable counterpart to [`edge_handle`](Self::edge_handle).
    pub(super) fn edge_handle_mut(&mut self, id: EdgeId) -> Option<&mut ConnectorHandle> {
        if id.graph != self.graph.id {
            return None;
        }
        self.edge_handles.get_mut(id.index)
    }

    /// Stores `handle` as edge `id`'s own connector handle. The same always-an-append call pattern as
    /// [`insert_node_handle`](Self::insert_node_handle), one call right after `id` is first added to `graph`.
    pub(super) fn insert_edge_handle(&mut self, id: EdgeId, handle: ConnectorHandle) {
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
    pub(super) fn remove_edge_handle(&mut self, id: EdgeId) -> Option<ConnectorHandle> {
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
    pub(super) fn node_edge_anchors(&self, id: NodeId) -> Result<Option<EdgeAnchors>, Error> {
        self.node_handle(id)
            .map(|handles| handles.edge_anchors)
            .ok_or(Error::UnknownNode(id))
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    /// The `to`-side routing override for the edge from `from` to `to`, if `to` is a binary operator node and
    /// `from` is one of its own two known inputs — see [`connector::route`]'s own `to_override` parameter.
    ///
    /// `None` for every other edge: an unknown `from`/`to`, a `to` that is not a two-input operator node, or a
    /// `from` that is not one of its two inputs (an edge a caller wired up by hand, bypassing both
    /// [`Scene::add_binary_operator_node_with`] and [`Scene::add_arithmetic_operator_node_with`]). Each of those
    /// falls back to `route`'s own existing default, exactly as before this existed.
    pub(super) fn binary_operator_to_override(
        &self,
        from: NodeId,
        to: NodeId,
    ) -> Option<connector::BinaryOperatorRoute> {
        let (input_a, input_b) = self.node_handle(to)?.binary_operator_inputs?;
        // Both two-input operator constructors reject `input_a == input_b`, so this is an unambiguous, stable identity
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
    /// If `edge_id` runs from `mover` into a two-input operator node with `mover` registered as one of its own two
    /// inputs, that operator's own id. `None` otherwise: an ordinary edge, or one a caller wired by hand, bypassing
    /// both [`Scene::add_binary_operator_node_with`] and [`Scene::add_arithmetic_operator_node_with`].
    ///
    /// `move_node` redraws every edge already incident to the node that moved. For an edge this identifies, it
    /// calls [`redraw_binary_operator_inputs`](Self::redraw_binary_operator_inputs) instead of
    /// [`redraw_edge`](Self::redraw_edge) — the sibling operand's own edge into the same operator is not incident
    /// to `mover`, so nothing else would ever redraw it, and it would keep showing wherever it last computed its
    /// own anchor, stale, until something else happened to move it too.
    pub(super) fn binary_operator_input_target(&self, mover: NodeId, edge_id: EdgeId) -> Option<NodeId> {
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
    /// Every child of the node's own `<g>` — its outer rect, and its label or grid cells — was drawn once, at
    /// creation, in local coordinates relative to `(0, 0)`. See
    /// [`node::plain::draw_box`]/[`node::data::draw_content_box`]. So moving the node only ever means rewriting the
    /// group's own `transform`. It never touches any child's own coordinates. This stays exactly as cheap for a
    /// data node with hundreds of value cells as for a plain label.
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
    pub(super) fn move_node(&mut self, id: NodeId, new_origin: Point, scratch: &mut String) -> Result<(), Error> {
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
    pub(super) fn write_edge_path(
        &self,
        id: EdgeId,
        vertices: &[Point],
        radius: f64,
        scratch: &mut String,
    ) -> Result<(), Error> {
        elbow_path_into(vertices, radius, scratch);
        let handle = self.edge_handle(id).ok_or(Error::UnknownEdge(id))?;
        handle.path.set_attr("d", scratch)?;
        Ok(())
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    /// Moves edge `id`'s own "L"/"R" port marker — if it has one — to [`port_marker_position`]'s own point for
    /// `anchor`/`side`. Does nothing for an edge with no marker: an ordinary edge, or a commutative operator's own
    /// input edge — see `node::operator::draw_port_marker`'s own doc comment for which edges get one.
    ///
    /// Repositions the existing element rather than recreating it, the same as [`write_edge_path`](Self::write_edge_path)
    /// does for the connector's own `<path>` — both are direct SVG-root children in absolute coordinates, rewritten
    /// on every redraw rather than relying on any node's own `transform`.
    ///
    /// `scratch` is a caller-owned buffer, reused across calls to avoid allocating a fresh `String` on every move
    /// event.
    ///
    /// # Errors
    ///
    /// Returns [`Error::UnknownEdge`] if `id` does not name an edge in this scene.
    pub(super) fn reposition_port_marker(
        &self,
        id: EdgeId,
        anchor: Point,
        side: Side,
        scratch: &mut String,
    ) -> Result<(), Error> {
        let Some(marker) = self.edge_handle(id).ok_or(Error::UnknownEdge(id))?.port_marker.as_ref() else {
            return Ok(());
        };
        let point = port_marker_position(anchor, side);
        marker.set_attr_display(scratch, "x", point.x)?;
        marker.set_attr_display(scratch, "y", point.y)?;
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
    pub(super) fn redraw_edge(&self, id: EdgeId, scratch: &mut String) -> Result<(), Error> {
        let edge = self.graph.edge(id).ok_or(Error::UnknownEdge(id))?;
        let from_rect = self.node_rect(edge.from)?;
        let from_anchors = self.node_edge_anchors(edge.from)?;
        let to_rect = self.node_rect(edge.to)?;
        let to_anchors = self.node_edge_anchors(edge.to)?;
        let to_override = self.binary_operator_to_override(edge.from, edge.to);
        if let Some(connector::BinaryOperatorRoute { anchor, side, .. }) = &to_override {
            self.reposition_port_marker(id, *anchor, *side, scratch)?;
        }
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
    pub(super) fn redraw_binary_operator_inputs(&self, operator: NodeId, scratch: &mut String) -> Result<(), Error> {
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
        self.reposition_port_marker(edge_a, anchor_a, side_a, scratch)?;

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
        self.write_edge_path(edge_b, &vertices_b, radius_b, scratch)?;
        self.reposition_port_marker(edge_b, anchor_b, side_b, scratch)
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
    pub(super) fn redraw_edge_with_type(
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
    pub(super) fn resolve_overlap(&self, id: NodeId, pre_drag_origin: Point, padding: f64) -> Option<Point> {
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
