//! The condensation: one node for each component that has been emitted, and an arrow wherever the original graph has an
//! edge from one to another. Pure.
//!
//! It is built only from what the snapshot has emitted, and so it grows by one node at each closure and never has a
//! node for a component that is yet to be found. A component is only emitted after every component that it leads to, so
//! each new node has arrows to nodes that are already there, and none of them ever moves.

use super::solver::{Graph, Snapshot};
use std::fmt;

/// The size that a node of the condensation is given for placing, and the room left round it.
pub(crate) const BOX: (f64, f64) = (96.0, 56.0);
#[cfg(test)]
pub(crate) const MARGIN: f64 = 4.0;

/// Where the centre of each node goes, by its column and then by its place within the column. A column's nodes are
/// placed in the order that they were emitted in, so a place is never taken from a node that is already on the page.
pub(crate) const SLOTS: [&[(f64, f64)]; 6] = [
    &[(100.0, 148.0)],
    &[(258.0, 103.0)],
    &[(416.0, 92.0), (416.0, 181.0)],
    &[(574.0, 47.0), (574.0, 136.0), (574.0, 225.0)],
    &[(732.0, 92.0), (732.0, 181.0)],
    &[(890.0, 47.0)],
];

/// One component that has been emitted.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Node {
    /// Its number in the order of emission, counted from 1: the C-ID that the page shows.
    pub(crate) id: usize,
    /// Its vertices, in the order that they were popped.
    pub(crate) members: Vec<usize>,
}

impl Node {
    pub(crate) fn name(&self) -> String {
        format!("C{}", self.id)
    }
}

/// The nodes and arrows that a snapshot has made known.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct Condensation {
    pub(crate) nodes: Vec<Node>,
    /// Pairs of C-IDs `(from, to)`, sorted, with no pair twice and no pair from a component to itself.
    pub(crate) arcs: Vec<(usize, usize)>,
}

/// What a component's node is placed by.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct Placement {
    pub(crate) id: usize,
    pub(crate) column: usize,
    pub(crate) row: usize,
    pub(crate) centre: (f64, f64),
}

/// Why a condensation could not be placed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum PlaceError {
    /// A node has an arrow to a component that has not been emitted, which Tarjan's algorithm cannot do.
    ForwardArc { from: usize, to: usize },
    /// A node falls outside the table of places.
    NoSlot { column: usize, row: usize },
}

impl fmt::Display for PlaceError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ForwardArc { from, to } => write!(f, "C{from} has an arrow to C{to}, which is not emitted yet"),
            Self::NoSlot { column, row } => write!(f, "there is no place for column {column}, row {row}"),
        }
    }
}

impl std::error::Error for PlaceError {}

impl Condensation {
    /// What `snapshot` has emitted, for the graph that it is a snapshot of.
    pub(crate) fn of(graph: &Graph, snapshot: &Snapshot) -> Self {
        let nodes = snapshot
            .components
            .iter()
            .enumerate()
            .map(|(i, members)| Node {
                id: i + 1,
                members: members.clone(),
            })
            .collect();
        let mut arcs: Vec<(usize, usize)> = graph
            .edges()
            .into_iter()
            .filter_map(|(u, v)| match (snapshot.membership[u], snapshot.membership[v]) {
                (Some(a), Some(b)) if a != b => Some((a + 1, b + 1)),
                _ => None,
            })
            .collect();
        arcs.sort_unstable();
        arcs.dedup();
        Self { nodes, arcs }
    }

    /// Whether the arrows have a cycle, which Kahn's algorithm finds by removing every node that nothing leads to.
    #[cfg(test)]
    pub(crate) fn has_cycle(&self) -> bool {
        let mut incoming: Vec<usize> = self
            .nodes
            .iter()
            .map(|n| self.arcs.iter().filter(|&&(_, to)| to == n.id).count())
            .collect();
        let mut removed = 0;
        let mut ready: Vec<usize> = (0..self.nodes.len()).filter(|&i| incoming[i] == 0).collect();
        while let Some(i) = ready.pop() {
            removed += 1;
            let id = self.nodes[i].id;
            for &(from, to) in &self.arcs {
                if from == id {
                    let at = self.nodes.iter().position(|n| n.id == to);
                    if let Some(at) = at {
                        incoming[at] -= 1;
                        if incoming[at] == 0 {
                            ready.push(at);
                        }
                    }
                }
            }
        }
        removed != self.nodes.len()
    }

    /// Where each node goes: its column is the longest path from it to a node with no arrows out, and its place in the
    /// column is the number of nodes that came to the column before it.
    ///
    /// # Errors
    ///
    /// Returns an error for an arrow to a node that is not there, or a node that the table has no place for.
    pub(crate) fn placements(&self) -> Result<Vec<Placement>, PlaceError> {
        let mut out: Vec<Placement> = Vec::with_capacity(self.nodes.len());
        for node in &self.nodes {
            let mut column = 0;
            for &(from, to) in self.arcs.iter().filter(|&&(from, _)| from == node.id) {
                let target = out.iter().find(|p| p.id == to).ok_or(PlaceError::ForwardArc { from, to })?;
                column = column.max(target.column + 1);
            }
            let row = out.iter().filter(|p| p.column == column).count();
            let centre = *SLOTS
                .get(column)
                .and_then(|slots| slots.get(row))
                .ok_or(PlaceError::NoSlot { column, row })?;
            out.push(Placement {
                id: node.id,
                column,
                row,
                centre,
            });
        }
        Ok(out)
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[cfg(test)]
mod unit_tests;
