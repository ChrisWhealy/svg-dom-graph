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
