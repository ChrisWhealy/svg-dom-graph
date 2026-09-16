use super::content::DataNodeContent;
use svg_dom::root::utils::Rect;

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Identifies one node in a graph.
///
/// This is purposefully opaque: only the crate-internal topology model can produce a `NodeId`.
/// So a `NodeId` can never be confused with a plain `usize`, or with an `EdgeId`.
///
/// Carries the id of the `Graph` that created it, not just a per-graph sequence number.
/// So a `NodeId` from one `Graph` can never collide with one from another, even when both graphs assigned the same
/// sequence number — `Graph::node` simply will not find it there.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct NodeId {
    pub(crate) graph: usize,
    pub(crate) index: usize,
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// A node's semantic content holding either the plain text label of an ordinary node or the typed numeric values of a
/// data node.
///
/// The graph model retains this regardless of how a node was rendered. So a reader need not parse the generated
/// SVG to recover the node's actual content.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NodeContent {
    Label(String),
    Data(DataNodeContent),
}

impl From<String> for NodeContent {
    fn from(label: String) -> Self {
        Self::Label(label)
    }
}

impl From<&str> for NodeContent {
    fn from(label: &str) -> Self {
        Self::Label(label.to_string())
    }
}

impl From<DataNodeContent> for NodeContent {
    fn from(data: DataNodeContent) -> Self {
        Self::Data(data)
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// One node's data: its position and its [`NodeContent`].
pub struct Node {
    pub rect: Rect,
    // Not read anywhere yet outside tests: nothing re-queries a node's content after creation, only its rect (for
    // redraw-on-move). Kept as node data regardless, since content is part of a node's identity, not just a
    // one-shot render parameter — see `NodeContent`'s own doc comment for the features that will need it.
    #[allow(dead_code)]
    pub content: NodeContent,
}
