//! The graph's topology: nodes, edges, and incidence.
//!
//! Kept free of any DOM/wasm dependency, so it stays testable with a plain `cargo test`.
//! This is the single source of truth for what the graph contains.
//! [`crate::scene`] renders it, and keeps a parallel map of DOM handles keyed by the same [`NodeId`]/[`EdgeId`]s this
//! module hands out.

use super::{NEXT_GRAPH_ID, edge::*, node::*};
use std::{collections::HashMap, sync::atomic::Ordering};
use svg_dom::root::utils::Rect;

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The graph's topology.
///
/// Holds every node and edge, plus each node's incident edges.
/// A caller can then find what connects to a node without scanning every edge in the graph.
///
/// Carries no rendering state of its own.
/// `crate::scene::Scene` pairs each id this graph hands out with its rendered SVG handles.
pub struct Graph {
    pub id: usize,
    pub nodes: HashMap<NodeId, Node>,
    pub edges: HashMap<EdgeId, Edge>,
    pub incident: HashMap<NodeId, Vec<EdgeId>>,
    pub next_node_id: usize,
    pub next_edge_id: usize,
}

impl Graph {
    pub fn new() -> Self {
        Self {
            id: NEXT_GRAPH_ID.fetch_add(1, Ordering::Relaxed),
            nodes: HashMap::new(),
            edges: HashMap::new(),
            incident: HashMap::new(),
            next_node_id: 0,
            next_edge_id: 0,
        }
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    /// Adds a node and returns its id.
    pub fn add_node(&mut self, rect: Rect, label: impl Into<String>) -> NodeId {
        let id = NodeId {
            graph: self.id,
            index: self.next_node_id,
        };
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
        let id = EdgeId {
            graph: self.id,
            index: self.next_edge_id,
        };
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
