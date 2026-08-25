//! Renders a graph onto the DOM, and keeps each node's and edge's rendered SVG handles alongside it.
//!
//! The topology model (crate-private while this crate's API is still taking shape) owns the topology and is the
//! single source of truth for it.
//! This module pairs each of its ids with a rendered handle, and keeps both in sync as nodes move.
//!
//! This crate has no opinion about which HTML page hosts a [`Scene`], or what graph a caller builds with one.
//! See the sibling `demo-app` crate for a small worked example.

pub(crate) mod drag;

use crate::{
    error::Error,
    geometry::{apply_matrix, boundary_point},
    model::{edge::EdgeId, graph::Graph, node::NodeId},
};
use std::{
    cell::RefCell,
    collections::HashMap,
    rc::Rc,
    sync::atomic::{AtomicUsize, Ordering},
};
use svg_dom::{
    DominantBaseline, MarkerUnits, SvgMarker, SvgNode, SvgRoot, TextAnchor,
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
/// Converts `client` (viewport CSS pixels, such as `PointerEvent::client_x`/`client_y`) into user-space
/// coordinates, via `inverse_ctm`.
fn client_to_user_space(client: Point, inverse_ctm: Matrix2D) -> Point {
    apply_matrix(inverse_ctm, client)
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The centre point of a box's rectangle.
fn box_centre(rect: Rect) -> Point {
    Point::new(rect.origin.x + rect.size.width / 2.0, rect.origin.y + rect.size.height / 2.0)
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Assigns each `Scene` a distinct number, so its arrow marker gets an id no other `Scene` — and, so long as a
/// caller's own document doesn't deliberately collide with this crate's naming, no unrelated content either — is
/// likely to claim.
static NEXT_SCENE_ID: AtomicUsize = AtomicUsize::new(0);

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Defines a small filled-triangle arrowhead marker in `<defs>` and returns its handle.
///
/// `ref_x`/`ref_y` place the marker's anchor point (the tip of the triangle) at the very end of the line it attaches
/// to.
/// `orient("auto")` then rotates the marker to follow that line's own direction.
///
/// `marker_id` must be unique within `svg`'s document.
/// A hardcoded id such as `"arrow"` would collide the moment a second `Scene` shares the same `<svg>`, or the
/// caller's own document already defines an element with that id.
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
/// Draws a box's rectangle and its centred label, grouped under one `<g>`, and returns their handles.
fn draw_box(svg: &SvgRoot, rect: Rect, label: &str) -> Result<BoxHandles, Error> {
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
/// `move_node` (called internally by [`Scene::make_draggable`]) is the one place that keeps a moved node's rectangle,
/// its rendered box/label position, and its incident connectors all in sync.
///
/// Owns the `SvgRoot` it renders into.
/// `Scene::new(svg)` binds them for the `SceneInner`'s whole lifetime, so every node and edge in one `Scene` is
/// guaranteed to live in the same `<svg>` document — there is no `svg` parameter on [`Scene::add_node`] or
/// [`Scene::add_edge`] through which a caller could pass a different root by mistake.
struct SceneInner {
    svg: SvgRoot,
    graph: Graph,
    node_handles: HashMap<NodeId, BoxHandles>,
    edge_handles: HashMap<EdgeId, SvgNode>,
    arrow: SvgMarker,
}

impl SceneInner {
    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    /// The current rectangle of node `id`.
    ///
    /// # Errors
    ///
    /// Returns [`Error::UnknownNode`] if `id` does not name a node in this scene's graph — for example, a `NodeId`
    /// from a different `Scene`.
    fn node_rect(&self, id: NodeId) -> Result<Rect, Error> {
        self.graph.node(id).map(|node| node.rect).ok_or(Error::UnknownNode(id))
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    /// Moves node `id` to `new_origin`: updates the graph, the rendered box/label, and every incident connector.
    ///
    /// `scratch` is a caller-owned buffer, reused across calls to avoid a fresh allocation on every move.
    /// See [`SvgNode::set_attr_display`].
    ///
    /// # Errors
    ///
    /// Returns [`Error::UnknownNode`] if `id` does not name a node in this scene.
    fn move_node(&mut self, id: NodeId, new_origin: Point, scratch: &mut String) -> Result<(), Error> {
        let size = self.node_rect(id)?.size;
        self.graph.set_node_rect(id, Rect { origin: new_origin, size });

        let handles = self.node_handles.get(&id).ok_or(Error::UnknownNode(id))?;
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
    ///
    /// # Errors
    ///
    /// Returns [`Error::UnknownEdge`] if `id` does not name an edge in this scene, or [`Error::UnknownNode`] if
    /// either of its endpoints no longer does.
    fn redraw_edge(&self, id: EdgeId, scratch: &mut String) -> Result<(), Error> {
        let edge = self.graph.edge(id).ok_or(Error::UnknownEdge(id))?;
        let from_rect = self.node_rect(edge.from)?;
        let to_rect = self.node_rect(edge.to)?;

        let start = boundary_point(from_rect, box_centre(to_rect));
        let end = boundary_point(to_rect, box_centre(from_rect));

        let connector = self.edge_handles.get(&id).ok_or(Error::UnknownEdge(id))?;
        connector.set_attr_display(scratch, "x1", start.x)?;
        connector.set_attr_display(scratch, "y1", start.y)?;
        connector.set_attr_display(scratch, "x2", end.x)?;
        connector.set_attr_display(scratch, "y2", end.y)?;

        Ok(())
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// A cheap, cloneable handle to a rendered graph.
///
/// Internally an `Rc<RefCell<SceneInner>>` — this crate owns that sharing strategy, not the caller.
/// A `Scene` can be cloned freely (every clone refers to the same underlying graph and DOM state) and its methods take
/// `&self`, not `&mut self`, so a caller never has to wrap it in `Rc<RefCell<_>>` themselves just to call
/// [`make_draggable`](Self::make_draggable) or to share it with more than one closure.
#[derive(Clone)]
pub struct Scene {
    inner: Rc<RefCell<SceneInner>>,
}

impl Scene {
    /// Creates an empty scene, ready to hold nodes and edges within `svg`.
    ///
    /// Also defines the arrow marker every edge's connector uses, since every `Scene` needs exactly one, shared
    /// across all its edges.
    pub fn new(svg: SvgRoot) -> Result<Self, Error> {
        let marker_id = format!("svg-dom-graph-arrow-{}", NEXT_SCENE_ID.fetch_add(1, Ordering::Relaxed));
        let arrow = define_arrow_marker(&svg, &marker_id)?;
        Ok(Self {
            inner: Rc::new(RefCell::new(SceneInner {
                svg,
                graph: Graph::new(),
                node_handles: HashMap::new(),
                edge_handles: HashMap::new(),
                arrow,
            })),
        })
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    /// Adds a node to the graph, draws its box and label, and returns its id.
    pub fn add_node(&self, top_left: Point, size: Size, label: impl Into<String>) -> Result<NodeId, Error> {
        let label = label.into();
        let rect = Rect { origin: top_left, size };
        let mut inner = self.inner.borrow_mut();
        let handles = draw_box(&inner.svg, rect, &label)?;
        let id = inner.graph.add_node(rect, label);
        inner.node_handles.insert(id, handles);
        Ok(id)
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    /// Adds a directed edge to the graph, draws its arrow-tipped connector, and returns its id.
    ///
    /// # Errors
    ///
    /// Returns [`Error::UnknownNode`] if `from` or `to` does not name a node in this scene — for example, a
    /// `NodeId` from a different `Scene`.
    /// Checked before the self-loop check below, so a foreign id is always reported as unknown, even if `from` and
    /// `to` are the same foreign id.
    /// Returns [`Error::SelfLoopUnsupported`] if `from` and `to` are the same node in this scene — not yet
    /// supported, see that variant's own doc comment for why.
    pub fn add_edge(&self, from: NodeId, to: NodeId) -> Result<EdgeId, Error> {
        let mut inner = self.inner.borrow_mut();
        let from_rect = inner.node_rect(from)?;
        let to_rect = inner.node_rect(to)?;

        if from == to {
            return Err(Error::SelfLoopUnsupported(from));
        }

        let start = boundary_point(from_rect, box_centre(to_rect));
        let end = boundary_point(to_rect, box_centre(from_rect));

        let connector = inner.svg.line(start, end)?;
        connector.set_stroke("#555")?;
        connector.set_stroke_width(1.5)?;
        connector.set_marker_end_ref(&inner.arrow)?;

        let id = inner.graph.add_edge(from, to);
        inner.edge_handles.insert(id, connector);
        Ok(id)
    }
}
