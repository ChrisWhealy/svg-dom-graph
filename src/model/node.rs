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
/// One node's data: its position and its label.
pub struct Node {
    pub rect: Rect,
    // Not read anywhere yet: nothing re-queries a node's label after creation, only its rect (for redraw-on-move).
    // Kept as node data regardless, since a label is part of a node's identity, not just a one-shot render
    // parameter — a future feature such as re-rendering or editing labels would need it stored here.
    #[allow(dead_code)]
    pub label: String,
}
