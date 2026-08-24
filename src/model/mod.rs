//! The graph's topology: nodes, edges, and incidence.
//!
//! Kept free of any DOM/wasm dependency, so it stays testable with a plain `cargo test`.
//! This is the single source of truth for what the graph contains.
//! [`crate::scene`] renders it, and keeps a parallel map of DOM handles keyed by the same [`NodeId`]/[`EdgeId`]s this
//! module hands out.

use std::collections::HashMap;
use svg_dom::root::utils::Rect;

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Identifies one node in a graph.
///
/// Opaque on purpose: only the crate-internal topology model can produce one.
/// So a `NodeId` can never be confused with a plain `usize`, or with an [`EdgeId`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct NodeId(usize);

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Identifies one edge in a graph.
///
/// See [`NodeId`] for why this is a distinct type rather than a bare `usize`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct EdgeId(usize);

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

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// One directed edge between two nodes.
pub struct Edge {
    pub from: NodeId,
    pub to: NodeId,
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The graph's topology.
///
/// Holds every node and edge, plus each node's incident edges.
/// A caller can then find what connects to a node without scanning every edge in the graph.
///
/// Carries no rendering state of its own.
/// `crate::scene::Scene` pairs each id this graph hands out with its rendered SVG handles.
#[derive(Default)]
pub struct Graph {
    nodes: HashMap<NodeId, Node>,
    edges: HashMap<EdgeId, Edge>,
    incident: HashMap<NodeId, Vec<EdgeId>>,
    next_node_id: usize,
    next_edge_id: usize,
}

impl Graph {
    pub fn new() -> Self {
        Self::default()
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    /// Adds a node and returns its id.
    pub fn add_node(&mut self, rect: Rect, label: impl Into<String>) -> NodeId {
        let id = NodeId(self.next_node_id);
        self.next_node_id += 1;
        self.nodes.insert(id, Node { rect, label: label.into() });
        self.incident.insert(id, Vec::new());
        id
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    /// Adds a directed edge from `from` to `to` and returns its id.
    ///
    /// Registers the edge as incident to both endpoints.
    /// So [`incident_edges`](Self::incident_edges) finds it from either side, regardless of direction.
    pub fn add_edge(&mut self, from: NodeId, to: NodeId) -> EdgeId {
        let id = EdgeId(self.next_edge_id);
        self.next_edge_id += 1;
        self.edges.insert(id, Edge { from, to });
        self.incident.entry(from).or_default().push(id);
        self.incident.entry(to).or_default().push(id);
        id
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    /// Returns `id`'s node data.
    ///
    /// Returns `None` if `id` does not name a node in this graph.
    pub fn node(&self, id: NodeId) -> Option<&Node> {
        self.nodes.get(&id)
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    /// Overwrites `id`'s stored rectangle.
    ///
    /// Does nothing if `id` does not name a node in this graph.
    pub fn set_node_rect(&mut self, id: NodeId, rect: Rect) {
        if let Some(node) = self.nodes.get_mut(&id) {
            node.rect = rect;
        }
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    /// Returns `id`'s edge data.
    ///
    /// Returns `None` if `id` does not name an edge in this graph.
    pub fn edge(&self, id: EdgeId) -> Option<&Edge> {
        self.edges.get(&id)
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    /// Returns every edge id incident to `id` — edges where `id` is either endpoint.
    ///
    /// Returns an empty slice for a node with no edges, or for an unknown `id`.
    pub fn incident_edges(&self, id: NodeId) -> &[EdgeId] {
        self.incident.get(&id).map(Vec::as_slice).unwrap_or(&[])
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[cfg(test)]
mod unit_tests;
