use crate::{
    model::{content::Selection, edge::EdgeId, node::NodeId},
    scene::node::EdgeAnchors,
};
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
    /// `Some((left, right))` for a binary operator node — the ids of the two edges
    /// `Scene::add_binary_operator_node_with` auto-wired from `binary_operator_inputs.0`/`.1`, in the same order.
    /// `None` for every other node, exactly matching `binary_operator_inputs`.
    ///
    /// `SceneInner::redraw_binary_operator_inputs` reads this to redraw both edges together in one pass, rather
    /// than searching either operand's own incident edges for the one that also points at this operator.
    pub(crate) binary_operator_input_edges: Option<(EdgeId, EdgeId)>,
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
    /// Every entry in `cell_rects`' own stroke width, as drawn — `"1.5"` for a single-value node's own outer box,
    /// `"1"` for a multi-value grid's inner cells or an operator's own result row. Unused (`""`) for a plain
    /// label node, which has no `cell_rects` to begin with.
    ///
    /// Already formatted, rather than a plain `f64`: every stroke width this crate ever draws is one of a small
    /// fixed set (this default, or [`Scene::set_selection`](crate::scene::Scene::set_selection)'s own band/focus
    /// widths), so there is no reason to format one from scratch on a hot path — see `svg-dom`'s own
    /// `SvgNode::set_stroke_width` doc comment for why that convenience setter allocates a `String` on every call.
    ///
    /// `Scene::set_selection` restores this on every cell it does not band or focus. That way a selection's own
    /// thicker stroke never lingers once a cell is deselected — see that method's own doc comment for why it uses
    /// one.
    pub(crate) cell_stroke_width: &'static str,
    /// The current [`Selection`] `Scene::set_selection` last recoloured this node's own cells to, defaulting to
    /// [`Selection::None`] at creation.
    ///
    /// `Scene::set_selection` compares its own new `Selection` against this before touching anything: an identical
    /// selection is an immediate no-op, and even a genuinely different one only rewrites whichever cells actually
    /// changed category (focused/banded/default), not all `N` of them unconditionally.
    pub(crate) selection: Selection,
    /// This node's own live `aria-label` text, reused in place rather than rebuilt from scratch on every
    /// [`Scene::set_selection`](crate::scene::Scene::set_selection) call.
    ///
    /// Starts off as the node's base description alone; for instance `"u8 data grid, 7 values"`, with no selection
    /// appended. Empty for a plain label node, whose own visible text already serves as its accessible name.
    ///
    /// `Scene::set_selection` truncates this back to [`base_label_len`](Self::base_label_len), then appends the new
    /// selection's own [`Selection::describe_into`](crate::scene::Selection::describe_into) onto what remains —
    /// after the first call grows its capacity, a later selection change needs no further allocation.
    pub(crate) aria_label: String,
    /// `aria_label`'s own length at creation, before any selection was ever appended — the point
    /// `Scene::set_selection` truncates back to before appending a new selection's own description.
    pub(crate) base_label_len: usize,
}
