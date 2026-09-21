//! The graph's topology: nodes, edges, and incidence.
//!
//! Kept free of any DOM/wasm dependency, so it stays testable with a plain `cargo test`. This is the single source of
//! truth for what the graph contains. [`crate::scene`] renders it, and keeps a parallel map of DOM handles keyed by the
//! same [`NodeId`]/[`EdgeId`]s this module hands out.

use super::{NEXT_GRAPH_ID, edge::*, node::*};
use std::sync::atomic::Ordering;
use svg_dom::root::utils::Rect;

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The graph's topology.
///
/// Holds every node and edge; each [`Node`] carries its own incident edges alongside it — see that type's own doc
/// comment.
///
/// `nodes`/`edges` are stored densely rather than in a `HashMap`: `NodeId`/`EdgeId` already carry a monotonic index
/// within this graph, so `id.index` addresses a `Vec` slot directly, with no need to compute a hash.
///
/// `id.graph` is first checked everywhere, so an id belonging to a different `Graph` is rejected even when its own
/// index happens to coincide with a real slot here.
///
/// # Append-only, with one narrow exception
///
/// `add_node`/`add_edge` only ever append: a new id's own `index` is always exactly `nodes.len()`/`edges.len()`
/// before the push. [`remove_node`](Self::remove_node)/[`remove_edge`](Self::remove_edge) are the one exception,
/// and only ever unwind the single item just appended — see their own doc comments. So `nodes`/`edges` need no
/// `Option` tombstone layer: every live index is a live element, and a rolled-back index is simply reused by
/// whichever node/edge gets added next.
pub(crate) struct Graph {
    pub id: usize,
    pub nodes: Vec<Node>,
    pub edges: Vec<Edge>,
}

impl Graph {
    pub(crate) fn new() -> Self {
        Self {
            id: NEXT_GRAPH_ID.fetch_add(1, Ordering::Relaxed),
            nodes: Vec::new(),
            edges: Vec::new(),
        }
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    /// Adds a node and returns its id.
    pub(crate) fn add_node(&mut self, rect: Rect, content: impl Into<NodeContent>) -> NodeId {
        let id = NodeId {
            graph: self.id,
            index: self.nodes.len(),
        };
        self.nodes.push(Node {
            rect,
            content: content.into(),
            incident: Vec::new(),
        });
        id
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    /// Adds a directed edge from `from` to `to` and returns its id.
    ///
    /// Registers the edge as incident to both endpoints. So [`incident_edges`](Self::incident_edges) finds it from
    /// either side, regardless of direction.
    pub(crate) fn add_edge(&mut self, from: NodeId, to: NodeId) -> EdgeId {
        let id = EdgeId {
            graph: self.id,
            index: self.edges.len(),
        };
        self.edges.push(Edge { from, to });
        if let Some(node) = self.node_mut(from) {
            node.incident.push(id);
        }
        if let Some(node) = self.node_mut(to) {
            node.incident.push(id);
        }
        id
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    /// Returns `id`'s node data.
    ///
    /// Returns `None` if `id` does not name a node in this graph.
    pub(crate) fn node(&self, id: NodeId) -> Option<&Node> {
        if id.graph != self.id {
            return None;
        }
        self.nodes.get(id.index)
    }

    /// The mutable counterpart to [`node`](Self::node). Private: every external caller goes through a narrower,
    /// purpose-specific method instead — [`set_node_rect`](Self::set_node_rect), or `add_edge`'s own incidence
    /// bookkeeping above.
    fn node_mut(&mut self, id: NodeId) -> Option<&mut Node> {
        if id.graph != self.id {
            return None;
        }
        self.nodes.get_mut(id.index)
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    /// Overwrites `id`'s stored rectangle.
    ///
    /// Does nothing if `id` does not name a node in this graph.
    pub(crate) fn set_node_rect(&mut self, id: NodeId, rect: Rect) {
        if let Some(node) = self.node_mut(id) {
            node.rect = rect;
        }
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    /// Returns `id`'s edge data.
    ///
    /// Returns `None` if `id` does not name an edge in this graph.
    pub(crate) fn edge(&self, id: EdgeId) -> Option<&Edge> {
        if id.graph != self.id {
            return None;
        }
        self.edges.get(id.index)
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    /// Returns every edge id incident to `id` — edges where `id` is either endpoint.
    ///
    /// Returns an empty slice for a node with no edges, or for an unknown `id`.
    pub(crate) fn incident_edges(&self, id: NodeId) -> &[EdgeId] {
        self.node(id).map(|node| node.incident.as_slice()).unwrap_or(&[])
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    /// Removes edge `id`, and drops it from both of its own endpoints' incident lists.
    ///
    /// Does nothing if `id` does not name an edge in this graph, or if `edges` is empty.
    ///
    /// A narrow rollback primitive, not a general deletion API: this crate's only caller is
    /// `scene::node::OperatorConstructionGuard`, unwinding an edge it wired earlier in the same still-failing
    /// operator-creation call, always in reverse creation order. There is no public `Scene::remove_edge` — this
    /// graph never otherwise loses an edge once added.
    ///
    /// `id` is expected to always name the most recently added edge — the only one `pop()` can remove without
    /// shifting every other edge's own index. A debug build panics if it does not; a release build silently does
    /// nothing, the same as an unknown `id`, rather than removing the wrong edge or corrupting later indices.
    pub(crate) fn remove_edge(&mut self, id: EdgeId) {
        if id.graph != self.id {
            return;
        }
        let Some(last_index) = self.edges.len().checked_sub(1) else {
            return;
        };
        if id.index != last_index {
            debug_assert!(
                false,
                "Graph::remove_edge: {id:?} is not the most recently added edge (last is index {last_index})"
            );
            return;
        }
        let edge = self.edges.pop().expect("checked above: edges is non-empty");
        Self::pop_incidence(self.node_mut(edge.from), id);
        Self::pop_incidence(self.node_mut(edge.to), id);
    }

    /// Drops `id` from `node`'s own incident list. `id` is expected to be that list's own last entry — the same
    /// invariant, and for the same reason, as [`remove_edge`](Self::remove_edge)'s own doc comment: `add_edge`
    /// appended it there last, and no edge has touched this node since.
    fn pop_incidence(node: Option<&mut Node>, id: EdgeId) {
        let Some(node) = node else { return };
        if node.incident.last() == Some(&id) {
            node.incident.pop();
        } else {
            debug_assert!(false, "Graph::remove_edge: {id:?} is not the last edge incident to this node");
            node.incident.retain(|&e| e != id);
        }
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    /// Removes node `id` and its own incident-edge bookkeeping.
    ///
    /// Does nothing if `id` does not name a node in this graph, or if `nodes` is empty.
    ///
    /// The same narrow rollback purpose as [`remove_edge`](Self::remove_edge): only safe to call once every edge
    /// that could reference `id` has already been removed. Otherwise, those edges would keep pointing at a node
    /// that no longer exists. `OperatorConstructionGuard` always removes a node's own edges first, so this always
    /// holds for its one caller.
    ///
    /// `id` is expected to always name the most recently added node, for the same `pop()`-only reason
    /// [`remove_edge`](Self::remove_edge) documents. A debug build panics if it does not; a release build silently
    /// does nothing.
    pub(crate) fn remove_node(&mut self, id: NodeId) {
        if id.graph != self.id {
            return;
        }
        let Some(last_index) = self.nodes.len().checked_sub(1) else {
            return;
        };
        if id.index != last_index {
            debug_assert!(
                false,
                "Graph::remove_node: {id:?} is not the most recently added node (last is index {last_index})"
            );
            return;
        }
        self.nodes.pop();
    }
}
