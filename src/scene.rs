//! Renders a graph onto the DOM, and keeps each node's and edge's rendered SVG handles alongside it.
//!
//! The topology model (crate-private while this crate's API is still taking shape) owns the topology and is the
//! single source of truth for it.
//! This module pairs each of its ids with a rendered handle, and keeps both in sync as nodes move.
//!
//! This crate has no opinion about which HTML page hosts a [`Scene`], or what graph a caller builds with one.
//! See the sibling `demo-app` crate for a small worked example.

use crate::{
    geometry::{apply_matrix, boundary_point, invert_matrix},
    model::{EdgeId, Graph, NodeId},
};
use std::{
    cell::{Cell, RefCell},
    collections::HashMap,
    rc::Rc,
};
use svg_dom::{
    DominantBaseline, Error, MarkerUnits, SvgMarker, SvgNode, SvgRoot, TextAnchor,
    root::utils::{Matrix2D, Point, Rect, Size},
};

/// The rendered elements that make up one box, kept so a drag handler can reposition them.
struct BoxHandles {
    /// The `<g>` wrapping `rect_el` and `label_el`.
    /// Event listeners attach here, so a click on either child starts a drag.
    group: SvgNode,
    rect_el: SvgNode,
    label_el: SvgNode,
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The centre point of a box's rectangle.
fn box_centre(rect: Rect) -> Point {
    Point::new(rect.origin.x + rect.size.width / 2.0, rect.origin.y + rect.size.height / 2.0)
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Defines a small filled-triangle arrowhead marker in `<defs>` and returns its handle.
///
/// `ref_x`/`ref_y` place the marker's anchor point (the tip of the triangle) at the very end of the line it attaches
/// to.
/// `orient("auto")` then rotates the marker to follow that line's own direction.
fn define_arrow_marker(svg: &SvgRoot) -> Result<SvgMarker, Error> {
    let defs = svg.defs()?;
    let marker = defs.marker("arrow")?;

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
/// Draws a box's rectangle and its centred label, grouped under one `<g>`, and returns their handles.
fn draw_box(svg: &SvgRoot, rect: Rect, label: &'static str) -> Result<BoxHandles, Error> {
    let group = svg.group()?;

    let rect_el = svg.rect(rect.origin, rect.size)?;
    rect_el.set_fill("#eef4ff")?;
    rect_el.set_stroke("#2a5db0")?;
    rect_el.set_stroke_width(1.5)?;

    let label_el = svg.text(box_centre(rect), label)?;
    label_el.set_text_anchor(TextAnchor::Middle)?;
    label_el.set_dominant_baseline(DominantBaseline::Middle)?;
    label_el.set_font_size(14.0)?;
    label_el.set_fill("#1b1b1b")?;

    group.append(&rect_el)?;
    group.append(&label_el)?;

    Ok(BoxHandles { group, rect_el, label_el })
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// A rendered `Graph`, paired with each node's and edge's own SVG handles.
///
/// `Graph` owns the topology.
/// This owns everything DOM-specific, keyed by the same ids `Graph` hands out.
/// `move_node` (called internally by [`make_draggable`]) is the one place that keeps a moved node's rectangle, its
/// rendered box/label position, and its incident connectors all in sync.
pub struct Scene {
    graph: Graph,
    node_handles: HashMap<NodeId, BoxHandles>,
    edge_handles: HashMap<EdgeId, SvgNode>,
    arrow: SvgMarker,
}

impl Scene {
    /// Creates an empty scene, ready to hold nodes and edges within `svg`.
    ///
    /// Also defines the arrow marker every edge's connector uses, since every `Scene` needs exactly one, shared
    /// across all its edges.
    pub fn new(svg: &SvgRoot) -> Result<Self, Error> {
        let arrow = define_arrow_marker(svg)?;
        Ok(Self {
            graph: Graph::new(),
            node_handles: HashMap::new(),
            edge_handles: HashMap::new(),
            arrow,
        })
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    /// Adds a node to the graph, draws its box and label, and returns its id.
    pub fn add_node(
        &mut self,
        svg: &SvgRoot,
        top_left: Point,
        size: Size,
        label: &'static str,
    ) -> Result<NodeId, Error> {
        let rect = Rect { origin: top_left, size };
        let handles = draw_box(svg, rect, label)?;
        let id = self.graph.add_node(rect, label);
        self.node_handles.insert(id, handles);
        Ok(id)
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    /// Adds a directed edge to the graph, draws its arrow-tipped connector, and returns its id.
    pub fn add_edge(&mut self, svg: &SvgRoot, from: NodeId, to: NodeId) -> Result<EdgeId, Error> {
        let from_rect = self.node_rect(from);
        let to_rect = self.node_rect(to);

        let start = boundary_point(from_rect, box_centre(to_rect));
        let end = boundary_point(to_rect, box_centre(from_rect));

        let connector = svg.line(start, end)?;
        connector.set_stroke("#555")?;
        connector.set_stroke_width(1.5)?;
        connector.set_marker_end_ref(&self.arrow)?;

        let id = self.graph.add_edge(from, to);
        self.edge_handles.insert(id, connector);
        Ok(id)
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    /// The current rectangle of node `id`.
    ///
    /// # Panics
    ///
    /// Panics if `id` does not name a node in this scene's graph.
    /// Every `NodeId` this module uses comes from this same `Scene`, so that would be an internal-consistency bug,
    /// not a condition callers need to guard against.
    fn node_rect(&self, id: NodeId) -> Rect {
        self.graph
            .node(id)
            .expect("NodeId used within its own Scene is always valid")
            .rect
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    /// Moves node `id` to `new_origin`: updates the graph, the rendered box/label, and every incident connector.
    ///
    /// `scratch` is a caller-owned buffer, reused across calls to avoid a fresh allocation on every move.
    /// See [`SvgNode::set_attr_display`].
    fn move_node(&mut self, id: NodeId, new_origin: Point, scratch: &mut String) -> Result<(), Error> {
        let size = self.node_rect(id).size;
        self.graph.set_node_rect(id, Rect { origin: new_origin, size });

        let handles = self
            .node_handles
            .get(&id)
            .expect("NodeId used within its own Scene is always valid");
        handles.rect_el.set_attr_display(scratch, "x", new_origin.x)?;
        handles.rect_el.set_attr_display(scratch, "y", new_origin.y)?;

        let centre = box_centre(Rect { origin: new_origin, size });
        handles.label_el.set_attr_display(scratch, "x", centre.x)?;
        handles.label_el.set_attr_display(scratch, "y", centre.y)?;

        let incident = self.graph.incident_edges(id).to_vec();
        for edge_id in incident {
            self.redraw_edge(edge_id, scratch)?;
        }

        Ok(())
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    /// Recomputes both endpoints of edge `id` from its current node positions, and rewrites its line coordinates.
    fn redraw_edge(&self, id: EdgeId, scratch: &mut String) -> Result<(), Error> {
        let edge = self.graph.edge(id).expect("EdgeId used within its own Scene is always valid");
        let from_rect = self.node_rect(edge.from);
        let to_rect = self.node_rect(edge.to);

        let start = boundary_point(from_rect, box_centre(to_rect));
        let end = boundary_point(to_rect, box_centre(from_rect));

        let connector = self
            .edge_handles
            .get(&id)
            .expect("EdgeId used within its own Scene is always valid");
        connector.set_attr_display(scratch, "x1", start.x)?;
        connector.set_attr_display(scratch, "y1", start.y)?;
        connector.set_attr_display(scratch, "x2", end.x)?;
        connector.set_attr_display(scratch, "y2", end.y)?;

        Ok(())
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The pointer position and box origin recorded when a drag starts.
///
/// A delta between the pointer's current position and `pointer` gives how far to move `box_origin`.
/// Both are in the dragged box's own user-space coordinates, not viewport CSS pixels — see `inverse_ctm`.
#[derive(Clone, Copy)]
struct DragStart {
    pointer: Point,
    box_origin: Point,
    /// The dragged group's screen CTM, inverted once at pointerdown and reused for the rest of this drag.
    ///
    /// `SvgNode::screen_ctm()` may force a synchronous layout, so this is captured once per drag rather than on
    /// every pointermove.
    /// Caching it here (rather than recomputing per drag) assumes the group's own transform, and any ancestor
    /// transform up to the viewport, does not change mid-drag — true for this crate's current rendering, since
    /// nothing sets a transform on a box's group after it is drawn.
    inverse_ctm: Matrix2D,
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Converts `client` (viewport CSS pixels, such as `PointerEvent::client_x`/`client_y`) into user-space
/// coordinates, via `inverse_ctm`.
fn client_to_user_space(client: Point, inverse_ctm: Matrix2D) -> Point {
    apply_matrix(inverse_ctm, client)
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Wires up pointer dragging for node `id`.
///
/// Moves it, and redraws its incident connectors, via `scene` as the pointer moves.
///
/// `PointerEvent::client_x`/`client_y` are viewport CSS pixels, not `scene`'s user-space coordinates — the two only
/// coincide when the `<svg>` has no CSS scaling and its `viewBox` matches its pixel size exactly.
/// This converts through the dragged group's own screen CTM (see [`invert_matrix`]/[`apply_matrix`]), so dragging
/// stays correct under scaling, a resized `viewBox`, or CSS transforms.
pub fn make_draggable(scene: &Rc<RefCell<Scene>>, id: NodeId) -> Result<(), Error> {
    let group = scene
        .borrow()
        .node_handles
        .get(&id)
        .expect("NodeId used within its own Scene is always valid")
        .group
        .clone();
    group.set_attr("style", "cursor: grab; touch-action: none;")?;

    let drag_start: Rc<Cell<Option<DragStart>>> = Rc::new(Cell::new(None));

    {
        // A weak handle, not a strong clone: `group` is the node this listener is registered on, so a strong
        // capture here would create a cycle (SvgNodeInner -> listener store -> closure -> SvgNode -> the same
        // SvgNodeInner) that leaks the node and defeats its automatic listener cleanup. See `WeakSvgNode`'s doc
        // comment.
        let group_weak = group.downgrade();
        let scene = scene.clone();
        let drag_start = drag_start.clone();
        group.on_pointerdown(move |evt| {
            let Some(group) = group_weak.upgrade() else { return };
            // Can't route the drag without a way to convert client pixels into this group's own coordinates.
            let Some(inverse_ctm) = group.screen_ctm().and_then(invert_matrix) else {
                return;
            };
            let client = Point::new(evt.client_x() as f64, evt.client_y() as f64);
            let pointer = client_to_user_space(client, inverse_ctm);

            let _ = group.as_element().set_pointer_capture(evt.pointer_id());
            let _ = group.set_attr("style", "cursor: grabbing; touch-action: none;");
            let box_origin = scene.borrow().node_rect(id).origin;
            drag_start.set(Some(DragStart {
                pointer,
                box_origin,
                inverse_ctm,
            }));
        })?;
    }

    {
        let scene = scene.clone();
        let drag_start = drag_start.clone();
        // Reused across every pointermove call in this drag — and across drags, since the closure's environment
        // persists between invocations — rather than allocating a fresh String each time. See
        // `SvgNode::set_attr_display`'s own doc comment for why this pattern exists.
        let mut scratch = String::new();
        group.on_pointermove(move |evt| {
            let Some(start) = drag_start.get() else { return };
            let client = Point::new(evt.client_x() as f64, evt.client_y() as f64);
            let pointer_now = client_to_user_space(client, start.inverse_ctm);

            let new_origin = Point::new(
                start.box_origin.x + (pointer_now.x - start.pointer.x),
                start.box_origin.y + (pointer_now.y - start.pointer.y),
            );

            let _ = scene.borrow_mut().move_node(id, new_origin, &mut scratch);
        })?;
    }

    {
        // Weak for the same reason as the pointerdown handler above.
        let group_weak = group.downgrade();
        let drag_start = drag_start.clone();
        group.on_pointerup(move |evt| {
            let Some(group) = group_weak.upgrade() else { return };
            let _ = group.as_element().release_pointer_capture(evt.pointer_id());
            let _ = group.set_attr("style", "cursor: grab; touch-action: none;");
            drag_start.set(None);
        })?;
    }

    {
        // The browser can abort a pointer sequence without ever firing pointerup — for example a touch drag
        // interrupted by a system gesture. Without this handler, drag_start would stay set, so a later stray
        // pointermove (including one for an unrelated pointer_id) would move the box using a stale drag.
        let group_weak = group.downgrade();
        let drag_start = drag_start.clone();
        group.on_pointercancel(move |evt| {
            let Some(group) = group_weak.upgrade() else { return };
            let _ = group.as_element().release_pointer_capture(evt.pointer_id());
            let _ = group.set_attr("style", "cursor: grab; touch-action: none;");
            drag_start.set(None);
        })?;
    }

    Ok(())
}
