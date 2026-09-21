mod node_content;
mod node_id;

pub(crate) use node_content::NodeContent;
pub use node_id::NodeId;

use super::edge::EdgeId;
use svg_dom::root::utils::Rect;

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// One node's data: its position, its [`NodeContent`], and every edge incident to it.
pub(crate) struct Node {
    pub rect: Rect,
    // Read back in production code, not just tests: `scene::node::operand_content` resolves it in order to type-check
    // an operator's own operands, and `Scene::set_selection` resolves it to validate/interpret a selection against the
    // node's own actual value count and grid shape. Content is part of a node's identity, not just a one-shot render
    // parameter — see `NodeContent`'s own doc comment for why it is retained at all.
    pub content: NodeContent,
    /// Every edge id incident to this node: E.G. a connector endpoint, regardless of direction.
    ///
    /// These are kept beside the node itself rather than in a separate node-keyed map, so a node and its own incidence
    /// data can never drift out of sync with each other.
    pub incident: Vec<EdgeId>,
}
