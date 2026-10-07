//! Node configuration and the `Scene` methods that add or reconfigure a node.
//!
//! Each node kind's own drawing and construction lives in its own child module. [`plain`] draws a plain label box,
//! [`data`] a [`DataNodeContent`] grid, and [`operator`] a unary/binary/arithmetic operator node.
//!
//! This file keeps only what more than one of them shares: [`EdgeAnchors`] validation, shared style constants, and
//! [`Scene::set_edge_anchors`]/[`Scene::set_focus`], each of which applies to any node kind.

mod construction_guard;
mod container;
mod data;
mod edge_anchors;
mod node_options;
mod operator;
mod plain;
mod render_guard;

use super::Scene;
use crate::{
    colours::{BOX_STROKE, FOCUS_RING},
    error::Error,
    model::node::NodeId,
    scene::DataNodeContent,
};
pub use edge_anchors::EdgeAnchors;
pub use node_options::NodeOptions;
use svg_dom::root::utils::{Point, Rect};

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
/// Returns [`Error::EmptyNodeContent`] if `content` holds no values, or [`Error::InvalidGridLayout`] if its own grid
/// layout wraps `0`. Shared by every path that draws a `DataNodeContent` grid: a data node's own construction, a
/// `measure_data_node`/`measure_named_data_node` call, and an operator node's own `result` (via
/// `validate_operator_result`). So content validity can never drift between drawing a real node and merely measuring
/// what one would look like.
fn validate_data_content(content: &DataNodeContent) -> Result<(), Error> {
    if content.len() == 0 {
        return Err(Error::EmptyNodeContent);
    }
    if !content.layout().is_valid() {
        return Err(Error::InvalidGridLayout(content.layout()));
    }
    Ok(())
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
//   Shared node-drawing style constants — each used by more than one of this module's own child modules, so none of
//   them owns a single one exclusively. A constant only [`plain`], [`data`], or [`operator`] itself needs instead lives
//   in that child module.
// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -

/// The label's default font size, in user-space units, before a node's own label is ever shrunk to fit its box.
const LABEL_FONT_SIZE: f64 = 14.0;

/// Font size for a data node's cell text, in user-space units. Deliberately smaller than [`LABEL_FONT_SIZE`]. A
/// byte-group value (e.g. `"F0 E1 D2 C3 B4 A5 96 87"`) is far longer than a typical plain label. So a slightly smaller
/// size keeps a modest grid from demanding an oversized box by default.
const GRID_FONT_SIZE: f64 = 13.0;

/// A generic monospace font stack. Digits render at a uniform width under a monospace font — a proportional font would
/// render `"1"` narrower than `"8"`, throwing off a byte-group's own internal alignment. Several names are offered
/// since not every browser/OS ships the same monospace font; `monospace` itself is the universally supported fallback.
const GRID_FONT_FAMILY: &str = "ui-monospace, SFMono-Regular, Menlo, Consolas, monospace";

/// The height of one value's own cell, in user-space units — a fixed multiple of [`GRID_FONT_SIZE`], not measured.
///
/// Unlike a cell's width, this is predictable. A cell's width depends entirely on how many characters its own value
/// holds, so it is measured via [`SvgNode::bounding_box`](svg_dom::SvgNode::bounding_box). A monospace font's own line
/// height at one fixed size is predictable enough that measuring it separately for every node would only add overhead,
/// not accuracy.
const CELL_HEIGHT: f64 = GRID_FONT_SIZE * 1.4;

/// The gap kept clear, on every side, between one value's own text and that value's own cell edges.
const CELL_PADDING: f64 = 6.0;

/// The gap kept clear, on every side, between a node's own inset value cell(s) and its outer box edges. So a connector
/// anchored anywhere on the outer box's own perimeter never coincides with a cell's own border. See [`operator`]'s own
/// module doc comment for why this matters for an operator node specifically.
const OUTER_PADDING: f64 = 10.0;

/// The height of an outer labelled box's own label row, in user-space units — a fixed multiple of [`LABEL_FONT_SIZE`],
/// the same [`CELL_HEIGHT`] approach applied at [`LABEL_FONT_SIZE`] rather than [`GRID_FONT_SIZE`]. Shared by
/// [`operator`]'s own operator-name row and [`data`]'s own optional variable-name row.
const LABEL_ROW_HEIGHT: f64 = LABEL_FONT_SIZE * 1.4 + 2.0 * CELL_PADDING;

/// Every node kind's own outer box stroke width at rest — matches `plain`/`data`/`operator`'s own `draw_*` functions,
/// each of which sets this directly rather than reading it from here. Kept here too only so [`Scene::set_focus`] has
/// the exact value to restore, rather than a second, independently chosen one.
const OUTER_BOX_STROKE_WIDTH: f64 = 1.5;

/// [`Scene::set_focus`]'s own outer box stroke width once focused — thicker than [`OUTER_BOX_STROKE_WIDTH`], the same
/// relationship [`super::selection_toolbar`]'s own focus ring already has to its own resting stroke.
const FOCUS_RING_STROKE_WIDTH: f64 = 3.0;

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
impl Scene {
    /// Updates node `id`'s [`EdgeAnchors`] configuration, and redraws every incident connector immediately with the new
    /// value.
    ///
    /// This is the only way to change a node's anchor configuration after [`Scene::add_node`] or
    /// [`Scene::add_node_with`] first draws it — for example, from a live slider control.
    ///
    /// An `edge_anchors` identical to `id`'s own current configuration is an immediate no-op: no incident edge is
    /// redrawn. [`EdgeAnchors`] is `Copy` and `Eq`, so this comparison is free next to the DOM writes it can skip.
    ///
    /// # Errors
    ///
    /// Returns [`Error::InvalidEdgeAnchors`] if `edge_anchors` is `Some(EdgeAnchors(0))`. Checked before touching the
    /// scene, so a rejected call leaves the node exactly as it was.
    ///
    /// Returns [`Error::UnknownNode`] if `id` does not name a node in this scene.
    ///
    /// Also returns an error — [`Error::UnknownEdge`] or a wrapped [`Error::Svg`] — if redrawing an incident connector
    /// fails partway through. A failure here can leave some incident connectors already redrawn and others not.
    /// Dragging a node already carries this same property for its own incident redraws, so this is not a new, weaker
    /// guarantee.
    pub fn set_edge_anchors(&self, id: NodeId, edge_anchors: Option<EdgeAnchors>) -> Result<(), Error> {
        validate_edge_anchors(edge_anchors)?;

        let mut inner = self.inner.borrow_mut();
        let handles = inner.node_handle_mut(id).ok_or(Error::UnknownNode(id))?;
        if handles.edge_anchors == edge_anchors {
            return Ok(());
        }
        handles.edge_anchors = edge_anchors;
        let own_input_edges = handles.binary_operator_input_edges;

        // Taken out for the call so `redraw_edge`/`redraw_binary_operator_inputs` can freely borrow the rest of `inner`
        // on every iteration. It is then put back. See `SceneInner::scratch`'s own doc comment for why this, rather
        // than a fresh `String` per call.
        let mut scratch = std::mem::take(&mut inner.scratch);

        // `id` is itself a binary operator node, so `edge_anchors` is *its own* fixing-point configuration. It feeds
        // `binary_operator_anchors` for both input edges at once, and both are redrawn together, once, here.
        // `SceneInner::move_node` follows the same reasoning for `redraw_binary_operator_inputs`. Changing an ordinary
        // node's, or an operand's own, `edge_anchors` never needs this: it only ever affects that one node's own
        // from-side anchor, never the operator-side split.
        if own_input_edges.is_some() {
            if let Err(e) = inner.redraw_binary_operator_inputs(id, &mut scratch) {
                inner.scratch = scratch;
                return Err(e);
            }
        }

        for edge_id in inner.graph.incident_edges(id) {
            if own_input_edges.is_some_and(|(a, b)| *edge_id == a || *edge_id == b) {
                continue; // already redrawn together, above
            }
            if let Err(e) = inner.redraw_edge(*edge_id, &mut scratch) {
                inner.scratch = scratch;
                return Err(e);
            }
        }
        inner.scratch = scratch;
        Ok(())
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    /// The current rendered rectangle of node `id` — its own top-left origin and size, in the `<svg>`'s own user space,
    /// as this crate itself last drew it.
    ///
    /// For a plain or container node this is exactly the `top_left`/`size` its own constructor was given. For a data or
    /// operator node it is the box this crate itself computed to fit its own content instead. A caller has no way to
    /// know that size ahead of drawing, because `draw_content_box` measures each cell's own real rendered text width
    /// (see its own doc comment). So this is the only way to learn it afterward. Useful for laying out a node relative
    /// to another one already drawn. For example, it can position a second node so its own centre lines up with a first
    /// node's, when the first node's own rendered width was not known in advance.
    ///
    /// Reflects `id`'s own position as most recently written: after a drag moves it, this returns the moved rectangle,
    /// not the one it was created with.
    ///
    /// # Errors
    ///
    /// Returns [`Error::UnknownNode`] if `id` does not name a node in this scene — for example, a `NodeId` from a
    /// different `Scene`.
    pub fn node_rect(&self, id: NodeId) -> Result<Rect, Error> {
        self.inner.borrow().node_rect(id)
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    /// Moves node `id` to `top_left`, keeping its own current size, and redraws every incident connector to match. The
    /// same functionality is used here as [`Scene::make_draggable`](crate::scene::Scene::make_draggable) already
    /// performs for every dragged frame. The differences however are these:
    /// * the logic is driven programmatically instead of by a pointer gesture
    /// * none of [`DragOptions`](crate::scene::DragOptions)'s bounds-clamping or collision handling is performed
    ///
    /// The node is always moved to exactly the position given.
    ///
    /// This function pairs with [`Scene::node_rect`] where you first add a node at some placeholder position, then read
    /// back its rendered size. Then you can compute where it actually belongs — for instance, centred under some other
    /// node whose width is based on dynamic content and cannot therefore be known in advance.
    ///
    /// Any edge already wired to `id` at its own placeholder position (e.g. an operator node's own auto-wired inputs)
    /// is redrawn against the new position too. A connector's own side is re-resolved from the node's current rect on
    /// every redraw. It is not cached from whichever position was current when the edge was first drawn. This uses the
    /// same ray-cast calculation as a live drag does on every frame.
    ///
    /// `id`'s own current position already equal to `top_left` is a no-op: nothing is redrawn.
    ///
    /// # Errors
    ///
    /// Returns [`Error::UnknownNode`] if `id` does not name a node in this scene.
    ///
    /// Returns [`Error::InvalidNodeGeometry`] if `top_left`'s coordinates are not finite. Also returned, despite `id`
    /// naming a node whose size this call never changes, if that node's own current size is somehow not finite or not
    /// strictly positive. This is the same defensive check every other entry point accepting node geometry in this
    /// crate already applies. A node that was ever successfully added can't actually be in that state, but this does
    /// not rely on that remaining true forever. Checked before touching the scene, so a rejected call leaves the node
    /// exactly as it was.
    ///
    /// Also returns a wrapped [`Error::Svg`] if redrawing an incident connector fails partway through. This is the same
    /// "can leave some incident connectors already redrawn and others not" property [`Scene::set_edge_anchors`]'s own
    /// doc comment already describes, for the same reason.
    pub fn move_node(&self, id: NodeId, top_left: Point) -> Result<(), Error> {
        let size = self.inner.borrow().node_rect(id)?.size;
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

        let mut inner = self.inner.borrow_mut();
        // Taken out for the call so `redraw_edge`/`redraw_binary_operator_inputs` can freely borrow the rest of `inner`
        // on every iteration. It is then put back. See `SceneInner::scratch`'s own doc comment for why this, rather
        // than a fresh `String` per call.
        let mut scratch = std::mem::take(&mut inner.scratch);
        let result = inner.move_node(id, top_left, &mut scratch);
        inner.scratch = scratch;
        result
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    /// Rings or un-rings node `id`'s own outer box as a whole, with the same focus-ring stroke this crate's own toolbar
    /// buttons already use. Unlike [`Scene::set_selection`](crate::scene::Scene::set_selection), this works on any node
    /// kind: a plain node, a container node, or a data node. It never looks at a node's own cells, only the one outer
    /// box every kind draws.
    ///
    /// A host driving its own multi-node walk uses this to mark whichever nodes the current stage puts in focus. An
    /// example is stepping through a diagram's own stages rather than one node's own values. It also clears the mark
    /// from whichever nodes it moves away from. There is no single-node "current stage" this crate tracks on a host's
    /// behalf. Unlike [`Scene::show_selection_toolbar`](crate::scene::Scene::show_selection_toolbar)'s own managed
    /// `Selection`, calling this twice with `focused: true` for two different nodes focuses both at once.
    ///
    /// `focused: false` always restores the plain default stroke, not whatever this node's own stroke was before its
    /// last `focused: true` call. For a single-value data node, that default is also what
    /// [`Scene::set_selection`](crate::scene::Scene::set_selection) itself restores a deselected cell to. The two
    /// features agree on an unfocused/unselected node's own resting look. Neither coordinates with the other's own
    /// writes to the same `<rect>`. A host that calls both on the same node is responsible for not fighting itself over
    /// which one writes last.
    ///
    /// # Errors
    ///
    /// Returns [`Error::UnknownNode`] if `id` does not name a node in this scene.
    pub fn set_focus(&self, id: NodeId, focused: bool) -> Result<(), Error> {
        let inner = self.inner.borrow();
        let handles = inner.node_handle(id).ok_or(Error::UnknownNode(id))?;
        if focused {
            handles.outer_rect.set_stroke(FOCUS_RING)?;
            handles.outer_rect.set_stroke_width(FOCUS_RING_STROKE_WIDTH)?;
        } else {
            handles.outer_rect.set_stroke(BOX_STROKE)?;
            handles.outer_rect.set_stroke_width(OUTER_BOX_STROKE_WIDTH)?;
        }
        Ok(())
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[cfg(test)]
mod unit_tests;
