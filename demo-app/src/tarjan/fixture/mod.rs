//! The one large graph that the demo shows: 64 vertices and 114 directed edges, generated from a formula. Pure.
//!
//! The graph is made of ten regions, B0 to B9. Each region is a ring of vertices with some chords, and the regions are
//! joined by 17 bridges that follow a directed acyclic graph. The region names are for the tests and the layout only.
//! They never appear in the page as components, since which vertices form a component is for the solver to find.
//!
//! Nothing here depends on a random number, a clock or the order of a hash map, so the same graph and the same order of
//! edges come out every time. The tests pin them with a hash.

use super::solver::Graph;

/// How many vertices each region has, which add up to 64.
pub(crate) const SIZES: [usize; 10] = [11, 10, 9, 8, 7, 6, 5, 4, 3, 1];

/// How many regions, and how many vertices and edges the graph has.
pub(crate) const REGIONS: usize = SIZES.len();
pub(crate) const VERTICES: usize = 64;
pub(crate) const EDGES: usize = 114;

/// The bridges: a region and the region that it has an edge to, which are the arcs of the inter-region DAG.
pub(crate) const DAG_PAIRS: [(usize, usize); 17] = [
    (0, 1),
    (0, 2),
    (0, 3),
    (1, 3),
    (1, 4),
    (1, 5),
    (2, 4),
    (2, 5),
    (3, 6),
    (3, 7),
    (4, 6),
    (4, 7),
    (5, 7),
    (6, 8),
    (6, 9),
    (7, 8),
    (8, 9),
];

/// The column that each region is in when the regions are laid out from upstream to downstream: the longest path to it
/// from a region that nothing leads to.
#[cfg(test)]
pub(crate) const LAYERS: [usize; 10] = [0, 1, 1, 2, 2, 2, 3, 3, 4, 5];

/// Why the graph could not be made.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum FixtureError {
    /// Two rules made the same edge.
    Duplicate(usize, usize),
}

/// The first vertex of each region.
pub(crate) fn starts() -> [usize; REGIONS] {
    let mut start = [0; REGIONS];
    for g in 1..REGIONS {
        start[g] = start[g - 1] + SIZES[g - 1];
    }
    start
}

/// The region that vertex `v` is in.
#[cfg(test)]
pub(crate) fn region_of(v: usize) -> usize {
    let start = starts();
    (0..REGIONS).rev().find(|&g| start[g] <= v).unwrap_or(0)
}

/// The vertices of region `g`, in ID order.
#[cfg(test)]
pub(crate) fn members_of(g: usize) -> std::ops::Range<usize> {
    let start = starts()[g];
    start..start + SIZES[g]
}

/// The edges of the graph, sorted by `(from, to)`, which is also the order of their IDs.
///
/// # Errors
///
/// Returns the edge that two rules both made.
pub(crate) fn edges() -> Result<Vec<(usize, usize)>, FixtureError> {
    let start = starts();
    let mut out: Vec<(usize, usize)> = Vec::with_capacity(EDGES);
    let mut add = |from: usize, to: usize| -> Result<(), FixtureError> {
        if out.contains(&(from, to)) {
            return Err(FixtureError::Duplicate(from, to));
        }
        out.push((from, to));
        Ok(())
    };
    for (g, &s) in SIZES.iter().enumerate() {
        if s >= 2 {
            for i in 0..s {
                add(start[g] + i, start[g] + (i + 1) % s)?;
            }
        }
    }
    for (g, &s) in SIZES.iter().enumerate() {
        if s >= 3 {
            for i in (0..s).step_by(2) {
                add(start[g] + i, start[g] + (i + 2) % s)?;
            }
        }
    }
    for &(a, b) in &DAG_PAIRS {
        add(start[a] + (a + b) % SIZES[a], start[b] + (2 * a + b) % SIZES[b])?;
    }
    out.sort_unstable();
    Ok(out)
}

/// The graph, with every adjacency list sorted by target.
///
/// # Panics
///
/// Never: the rules above make no edge twice, which a test checks.
pub(crate) fn graph() -> Graph {
    let edges = edges().expect("the fixed rules make no edge twice");
    Graph::from_edges(VERTICES, &edges).expect("the fixed graph is within the budget")
}

/// The name that a person reads for vertex `v`: `v00` to `v63`.
pub(crate) fn name(v: usize) -> String {
    format!("v{v:02}")
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[cfg(test)]
mod unit_tests;
