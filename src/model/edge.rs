use super::node::NodeId;

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// One directed edge between two nodes.
pub struct Edge {
    pub from: NodeId,
    pub to: NodeId,
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Identifies one edge in a graph.
///
/// See [`NodeId`] for why this is a distinct type rather than a bare `usize`, and why it carries its owning graph's
/// id as well as a sequence number.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct EdgeId {
    pub(crate) graph: usize,
    pub(crate) index: usize,
}
