//! Node configuration and the `Scene` methods that add or reconfigure a node.
//!
//! Each node kind's own drawing and construction lives in its own child module. [`plain`] draws a plain label box,
//! [`data`] a [`DataNodeContent`] grid, and [`operator`] a unary/binary/arithmetic operator node.
//!
//! This file keeps only what more than one of them shares: [`EdgeAnchors`] validation, shared style constants, and
//! [`Scene::set_edge_anchors`], which applies to any node kind.

mod construction_guard;
mod data;
mod edge_anchors;
mod node_options;
mod operator;
mod plain;
mod render_guard;

use super::Scene;
use crate::{error::Error, model::node::NodeId};
pub use edge_anchors::EdgeAnchors;
pub use node_options::NodeOptions;

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
// Shared node-drawing style constants — each used by more than one of this module's own child modules, so none of
// them owns a single one exclusively. A constant only [`plain`], [`data`], or [`operator`] itself needs instead
// lives in that child module.
// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -

/// The label's default font size, in user-space units, before a node's own label is ever shrunk to fit its box.
const LABEL_FONT_SIZE: f64 = 14.0;

/// Font size for a data node's cell text, in user-space units. Deliberately smaller than [`LABEL_FONT_SIZE`]: a
/// byte-group value (e.g. `"F0 E1 D2 C3 B4 A5 96 87"`) is far longer than a typical plain label, so a slightly smaller
/// size keeps a modest grid from demanding an oversized box by default.
const GRID_FONT_SIZE: f64 = 13.0;

/// A generic monospace font stack. Digits render at a uniform width under a monospace font — a proportional font would
/// render `"1"` narrower than `"8"`, throwing off a byte-group's own internal alignment. Several names are offered
/// since not every browser/OS ships the same monospace font; `monospace` itself is the universally supported fallback.
const GRID_FONT_FAMILY: &str = "ui-monospace, SFMono-Regular, Menlo, Consolas, monospace";

/// The height of one value's own cell, in user-space units — a fixed multiple of [`GRID_FONT_SIZE`], not measured.
///
/// Unlike a cell's width — which depends entirely on how many characters its own value holds, and so is measured via
/// [`SvgNode::bounding_box`](svg_dom::SvgNode::bounding_box) — a monospace font's own line height at one fixed size
/// is predictable enough that measuring it separately for every node would only add overhead, not accuracy.
const CELL_HEIGHT: f64 = GRID_FONT_SIZE * 1.4;

/// The gap kept clear, on every side, between one value's own text and that value's own cell edges.
const CELL_PADDING: f64 = 6.0;

/// The gap kept clear, on every side, between a node's own inset value cell(s) and its outer box edges — so a
/// connector anchored anywhere on the outer box's own perimeter never coincides with a cell's own border. See
/// [`operator`]'s own module doc comment for why this matters for an operator node specifically.
const OUTER_PADDING: f64 = 10.0;

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
impl Scene {
    /// Updates node `id`'s [`EdgeAnchors`] configuration, and redraws every incident connector immediately with the
    /// new value.
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
    /// Dragging a node already carries this same property for its own incident redraws, so this is not a new,
    /// weaker guarantee.
    pub fn set_edge_anchors(&self, id: NodeId, edge_anchors: Option<EdgeAnchors>) -> Result<(), Error> {
        validate_edge_anchors(edge_anchors)?;

        let mut inner = self.inner.borrow_mut();
        let handles = inner.node_handle_mut(id).ok_or(Error::UnknownNode(id))?;
        if handles.edge_anchors == edge_anchors {
            return Ok(());
        }
        handles.edge_anchors = edge_anchors;
        let own_input_edges = handles.binary_operator_input_edges;

        // Taken out for the call so `redraw_edge`/`redraw_binary_operator_inputs` can freely borrow the rest of
        // `inner` on every iteration, then put back — see `SceneInner::scratch`'s own doc comment for why this,
        // rather than a fresh `String` per call.
        let mut scratch = std::mem::take(&mut inner.scratch);

        // `id` is itself a binary operator node, so `edge_anchors` is *its own* fixing-point configuration: it
        // feeds `binary_operator_anchors` for both input edges at once, and both are redrawn together, once, here
        // — the same reasoning `SceneInner::move_node` follows for `redraw_binary_operator_inputs`. Changing an
        // ordinary node's, or an operand's own, `edge_anchors` never needs this: it only ever affects that one
        // node's own from-side anchor, never the operator-side split.
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
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[cfg(test)]
mod unit_tests;
