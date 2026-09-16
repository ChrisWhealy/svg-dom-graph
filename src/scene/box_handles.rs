use crate::{model::node::NodeId, scene::node::EdgeAnchors};
use svg_dom::SvgNode;

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The rendered elements that make up one box, kept so a drag handler can reposition them.
///
/// Every one of a box's own children — its outer rect, its label or grid cells — is drawn once, at creation, in local
/// coordinates relative to `(0, 0)`. `group`'s own `transform="translate(...)"` is the only thing that ever changes
/// afterward. [`SceneInner::move_node`] repositions a box by rewriting this one transform. This cost stays the same
/// regardless of how many children `group` holds. So a data node with hundreds of value cells moves exactly as cheaply
/// as a plain label. Moving a box needs no handle to any individual child beyond `group` — but recolouring one for
/// [`Scene::set_selection`](crate::scene::Scene::set_selection) does, hence `cell_rects` below.
pub(crate) struct BoxHandles {
    /// Event listeners attach here, so a click on any child starts a drag.
    pub(crate) group: SvgNode,
    /// Whether `Scene::make_draggable`/`Scene::make_draggable_with` has already been called for this node.
    ///
    /// `svg-dom`'s listener registration is append-only, so a second call would add a second, independent set of
    /// pointer listeners rather than replacing the first — see [`crate::Error::AlreadyDraggable`].
    pub(crate) draggable: bool,
    /// How many evenly spaced connector fixing points this node's own sides offer — see [`EdgeAnchors`].
    ///
    /// `redraw_edge` has no other way to learn a node's own anchor configuration once an incident edge needs a reroute.
    /// This value must live alongside the rendered handle, not just get used once at creation.
    pub(crate) edge_anchors: Option<EdgeAnchors>,
    /// `Some((left, right))` for a binary operator node — its own two operand ids, in the order
    /// `Scene::add_binary_operator_node_with` received them. `None` for every other node, unary operator nodes
    /// included, since only a binary node's two inputs can ever collide on the same side.
    ///
    /// `SceneInner::binary_operator_to_override` reads this on every redraw, so the two connectors split apart whenever
    /// they land on the same side, live — not just once, at creation.
    pub(crate) binary_operator_inputs: Option<(NodeId, NodeId)>,
    /// Every [`crate::scene::DataNodeContent`] cell's own `<rect>`, flat, in the same order
    /// [`crate::scene::DataNodeContent::cells`]/`shape` already use.
    ///
    /// A single-value node or an operator node's own single-value result draws no separate inner cell, so its own lone
    /// entry here is the outer box's own `rect` itself. Empty for a plain label node, which has no cell to select at
    /// all.
    ///
    /// `Scene::set_selection` is the only reader — nothing else needs to reach an individual cell again once it is
    /// drawn.
    pub(crate) cell_rects: Vec<SvgNode>,
}
