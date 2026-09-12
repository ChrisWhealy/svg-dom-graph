//! Node configuration and the `Scene` methods that add or reconfigure a node.

use super::{BoxHandles, Scene, box_centre};
use crate::{error::Error, model::node::NodeId};
use svg_dom::{
    DominantBaseline, SvgRoot, TextAnchor,
    root::utils::{Point, Rect, Size},
};

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// `EdgeAnchors` defines the number of evenly spaced connector fixing points available on each of a node's four sides.
/// This can be configured instead of the default single anchor point every connector uses by default.
///
/// The wrapped value must be `>= 1`.
///
/// `Scene::add_node_with`/`Scene::set_edge_anchors` reject `0` with [`Error::InvalidEdgeAnchors`] — a side with no
/// candidate point cannot anchor a connector, so `0` has no meaning here.
///
/// Consequently, you must use `None` rather than `Some(EdgeAnchors(0))` to keep a connector's own default anchor rule.
///
/// # How a connector picks one of the `n` candidates
///
/// A connector still picks *which side* to leave from exactly as it always does: by the ray from this node's own centre
/// toward the other endpoint's centre, and whichever side that ray crosses first. `EdgeAnchors` only changes *where on
/// that side* the connector lands.
///
/// That side is divided into `n + 1` equal segments, giving `n` internal division points — the two corners bounding the
/// side are never candidates. The connector then snaps to whichever of those `n` points sits closest to where the
/// unsnapped ray would have crossed.
///
/// If the two nodes' centres exactly coincide, that ray has no direction to pick a side from. This is the same
/// pre-existing degenerate case ordinary, unconfigured routing already has to handle, and `EdgeAnchors` resolves
/// it the same way: falling back to this node's own centre and `Side::East`, rather than an actual fixing point.
///
/// # `EdgeAnchors(1)` does not always match `None`
///
/// With `n = 1`, `n + 1 = 2` and this makes the segment's internal division point identical to the side's midpoint.
/// For an elbow connector this is no change at all — the default (`None`) elbow rule already always anchors at the same
/// midpoint, so `Some(EdgeAnchors(1))` and `None` render identically.
///
/// For a straight connector however, this does *not* generally hold: its default (`None`) is the ray's own exact
/// boundary crossing, which would only land on the midpoint by coincidence. `Some(EdgeAnchors(1))` forces a straight
/// connector onto the midpoint regardless, so the two will often render at visibly different points.
///
/// Every connector touching this node makes this choice independently, from its own other endpoint's position alone.
/// Fixing points are not reserved or assigned: nothing stops two, or all, of a node's incident connectors from landing
/// on the same point — there is no occupancy tracking or one-connector-per-point allocation.
///
/// `EdgeAnchors(5)` means "five candidate positions per side," not "capacity for five edges."
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EdgeAnchors(pub u8);

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Configures how [`Scene::add_node_with`] anchors connectors to a node.
///
/// Build one either with [`NodeOptions::default`] or with [`with_edge_anchors`](Self::with_edge_anchors).
/// A struct literal does not compile outside this crate.
///
/// ***A note on `Copy`***
///
/// Deriving `Copy` is a deliberate compatibility commitment, not an oversight. Removing `Copy` later is a
/// breaking change, so every field this type gains must itself stay `Copy`. See the same note on
/// [`DragOptions`](crate::scene::DragOptions), which shares the same commitment.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[non_exhaustive]
pub struct NodeOptions {
    /// How many evenly spaced connector fixing points this node's sides offer — see [`EdgeAnchors`].
    ///
    /// `None` (the default) keeps each connector's own default anchor rule.
    pub edge_anchors: Option<EdgeAnchors>,
}

impl NodeOptions {
    /// Returns `self` with `edge_anchors` set to `edge_anchors`.
    ///
    /// ```
    /// use svg_dom_graph::scene::{EdgeAnchors, NodeOptions};
    /// let options = NodeOptions::default().with_edge_anchors(Some(EdgeAnchors(3)));
    /// assert_eq!(options.edge_anchors, Some(EdgeAnchors(3)));
    /// ```
    #[must_use]
    pub fn with_edge_anchors(mut self, edge_anchors: Option<EdgeAnchors>) -> Self {
        self.edge_anchors = edge_anchors;
        self
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Returns [`Error::InvalidEdgeAnchors`] if `edge_anchors` is `Some(EdgeAnchors(0))`. `None` and every
/// `Some(EdgeAnchors(1..))` are valid.
fn validate_edge_anchors(edge_anchors: Option<EdgeAnchors>) -> Result<(), Error> {
    match edge_anchors {
        Some(EdgeAnchors(0)) => Err(Error::InvalidEdgeAnchors(0)),
        _ => Ok(()),
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Draws a box's rectangle and its centred label, grouped under one `<g>`, and returns their handles.
fn draw_box(svg: &SvgRoot, rect: Rect, label: &str, edge_anchors: Option<EdgeAnchors>) -> Result<BoxHandles, Error> {
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

    Ok(BoxHandles {
        group,
        rect_el,
        label_el,
        draggable: false,
        edge_anchors,
    })
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
impl Scene {
    /// Adds a node to the graph, draws its box and label, and returns its id.
    ///
    /// Equivalent to [`add_node_with`](Self::add_node_with) with [`NodeOptions::default`].
    ///
    /// # Errors
    ///
    /// Returns [`Error::InvalidNodeGeometry`] if `top_left`'s coordinates or `size`'s dimensions are not finite,
    /// or if `size`'s width or height is not strictly positive — see that variant's own doc comment for why.
    pub fn add_node(&self, top_left: Point, size: Size, label: impl Into<String>) -> Result<NodeId, Error> {
        self.add_node_with(top_left, size, label, NodeOptions::default())
    }

    /// Adds a node to the graph, draws its box and label, and returns its id. `options` controls how many
    /// connector fixing points this node's sides offer — see [`EdgeAnchors`].
    ///
    /// # Errors
    ///
    /// Returns [`Error::InvalidEdgeAnchors`] if `options.edge_anchors` is `Some(EdgeAnchors(0))`. Checked before
    /// drawing anything or touching the graph's model, so a rejected call leaves the scene exactly as it was.
    ///
    /// Returns [`Error::InvalidNodeGeometry`] if `top_left`'s coordinates or `size`'s dimensions are not finite,
    /// or if `size`'s width or height is not strictly positive — see that variant's own doc comment for why.
    /// Checked before drawing anything or touching the graph's model, so a rejected call leaves the scene exactly
    /// as it was.
    pub fn add_node_with(
        &self,
        top_left: Point,
        size: Size,
        label: impl Into<String>,
        options: NodeOptions,
    ) -> Result<NodeId, Error> {
        validate_edge_anchors(options.edge_anchors)?;

        let rect = Rect { origin: top_left, size };
        if !top_left.x.is_finite()
            || !top_left.y.is_finite()
            || !size.width.is_finite()
            || !size.height.is_finite()
            || size.width <= 0.0
            || size.height <= 0.0
        {
            return Err(Error::InvalidNodeGeometry(rect));
        }

        let label = label.into();
        let mut inner = self.inner.borrow_mut();
        let handles = draw_box(&inner.svg, rect, &label, options.edge_anchors)?;
        let id = inner.graph.add_node(rect, label);
        inner.node_handles.insert(id, handles);
        Ok(id)
    }

    /// Updates node `id`'s [`EdgeAnchors`] configuration, and redraws every incident connector immediately with
    /// the new value.
    ///
    /// This is the only way to change a node's anchor configuration after [`Scene::add_node`] or
    /// [`Scene::add_node_with`] first draws it — for example, from a live slider control.
    ///
    /// # Errors
    ///
    /// Returns [`Error::InvalidEdgeAnchors`] if `edge_anchors` is `Some(EdgeAnchors(0))`. Checked before touching
    /// the scene, so a rejected call leaves the node exactly as it was.
    ///
    /// Returns [`Error::UnknownNode`] if `id` does not name a node in this scene.
    ///
    /// Also returns an error — [`Error::UnknownEdge`] or a wrapped [`Error::Svg`] — if redrawing an incident
    /// connector fails partway through. A failure here can leave some incident connectors already redrawn and
    /// others not. Dragging a node already carries this same property for its own incident redraws, so this is
    /// not a new, weaker guarantee.
    pub fn set_edge_anchors(&self, id: NodeId, edge_anchors: Option<EdgeAnchors>) -> Result<(), Error> {
        validate_edge_anchors(edge_anchors)?;

        let mut inner = self.inner.borrow_mut();
        inner.node_handles.get_mut(&id).ok_or(Error::UnknownNode(id))?.edge_anchors = edge_anchors;

        let mut scratch = String::new();
        let incident: Vec<_> = inner.graph.incident_edges(id).to_vec();
        for edge_id in incident {
            inner.redraw_edge(edge_id, &mut scratch)?;
        }
        Ok(())
    }
}
