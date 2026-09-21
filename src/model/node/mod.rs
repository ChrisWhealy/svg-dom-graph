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
    // Not read anywhere yet outside tests: nothing re-queries a node's content after creation, only its rect (for
    // redraw-on-move). Kept as node data regardless, since content is part of a node's identity, not just a
    // one-shot render parameter — see `NodeContent`'s own doc comment for the features that will need it.
    #[allow(dead_code)]
    pub content: NodeContent,
    /// Every edge id incident to this node: E.G. a connector endpoint, regardless of direction.
    ///
    /// These are kept beside the node itself rather than in a separate node-keyed map, so a node and its own incidence
    /// data can never drift out of sync with each other.
    pub incident: Vec<EdgeId>,
}
