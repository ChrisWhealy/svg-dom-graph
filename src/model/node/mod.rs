mod node_content;
mod node_id;

pub(crate) use node_content::NodeContent;
pub use node_id::NodeId;

use svg_dom::root::utils::Rect;

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// One node's data: its position and its [`NodeContent`].
pub(crate) struct Node {
    pub rect: Rect,
    // Not read anywhere yet outside tests: nothing re-queries a node's content after creation, only its rect (for
    // redraw-on-move). Kept as node data regardless, since content is part of a node's identity, not just a
    // one-shot render parameter — see `NodeContent`'s own doc comment for the features that will need it.
    #[allow(dead_code)]
    pub content: NodeContent,
}
