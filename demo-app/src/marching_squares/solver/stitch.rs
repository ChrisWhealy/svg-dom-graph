//! Stitching segments into contours. It works on any set of segments, which is all of a field or the cells so far.
//!
//! A chain that ends at a crossing with one segment is open, and a chain that comes back to where it began is closed.
//! In the cells so far, an end on the frontier of the cells that have been visited is just such a crossing, and is a
//! provisional open end. Nothing is joined across a cell that has not been visited.

use super::{Point2, Segment, SegmentId, VertexKey};
use std::collections::{BTreeMap, BTreeSet};

/// One contour: the keys and the points of its vertices, in order. A closed contour does not repeat its first vertex.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Contour {
    pub(crate) keys: Vec<VertexKey>,
    pub(crate) vertices: Vec<Point2>,
    pub(crate) closed: bool,
}

impl Contour {
    /// The number of segments in it.
    pub(crate) fn segment_count(&self) -> usize {
        if self.closed { self.keys.len() } else { self.keys.len().saturating_sub(1) }
    }

    /// The points to draw: a closed contour repeats its first point at the end.
    pub(crate) fn drawn_points(&self) -> Vec<Point2> {
        let mut points = self.vertices.clone();
        if self.closed {
            if let Some(first) = points.first().copied() {
                points.push(first);
            }
        }
        points
    }
}

/// How many contours, of each kind, and how many exact ties.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(crate) struct Totals {
    pub(crate) open: usize,
    pub(crate) closed: usize,
    pub(crate) ties: usize,
}

/// Joins `segments` into contours: the open chains first, each from its lowest end, and then the cycles, each from its
/// lowest key. Every segment is used once. A chain is walked from a crossing along the one segment that has not been
/// used, and a crossing is left by its lowest unused segment, so the result does not depend on the order of `segments`.
pub(crate) fn stitch(segments: &[Segment]) -> Vec<Contour> {
    let mut at: BTreeMap<VertexKey, Vec<usize>> = BTreeMap::new();
    let mut points: BTreeMap<VertexKey, Point2> = BTreeMap::new();
    for (i, s) in segments.iter().enumerate() {
        for end in [s.a, s.b] {
            at.entry(end.key).or_default().push(i);
            points.insert(end.key, end.point);
        }
    }
    for list in at.values_mut() {
        list.sort_by_key(|&i| segments[i].id);
    }
    let mut used: BTreeSet<SegmentId> = BTreeSet::new();
    let mut contours = Vec::new();
    // The open chains start at the lowest crossing with one segment, and their other end is then used.
    let ends: Vec<VertexKey> = at.iter().filter(|(_, v)| v.len() == 1).map(|(k, _)| *k).collect();
    for start in ends {
        if let Some(only) = at[&start].first() {
            if used.contains(&segments[*only].id) {
                continue;
            }
            contours.push(walk(segments, &at, &points, &mut used, start, false));
        }
    }
    // What is left is cycles, and any chain through a crossing with more than two segments, which a valid field has
    // none.
    let keys: Vec<VertexKey> = at.keys().copied().collect();
    for start in keys {
        if at[&start].iter().any(|&i| !used.contains(&segments[i].id)) {
            contours.push(walk(segments, &at, &points, &mut used, start, true));
        }
    }
    contours
}

fn other_end(s: &Segment, from: VertexKey) -> VertexKey {
    if s.a.key == from { s.b.key } else { s.a.key }
}

/// Walks from `start` along unused segments. `cycle` says to canonicalise it as a cycle if it comes back to `start`.
fn walk(
    segments: &[Segment],
    at: &BTreeMap<VertexKey, Vec<usize>>,
    points: &BTreeMap<VertexKey, Point2>,
    used: &mut BTreeSet<SegmentId>,
    start: VertexKey,
    cycle: bool,
) -> Contour {
    let mut keys = vec![start];
    let mut here = start;
    let mut closed = false;
    loop {
        let next = at[&here].iter().map(|&i| &segments[i]).find(|s| !used.contains(&s.id));
        let Some(s) = next else { break };
        used.insert(s.id);
        let there = other_end(s, here);
        if cycle && there == start {
            closed = true;
            break;
        }
        keys.push(there);
        here = there;
    }
    if closed {
        keys = canonical_cycle(keys);
    }
    let vertices = keys.iter().map(|k| points[k]).collect();
    Contour { keys, vertices, closed }
}

/// Rotates a cycle to start at its lowest key, and chooses the direction whose sequence is the lower.
fn canonical_cycle(keys: Vec<VertexKey>) -> Vec<VertexKey> {
    let n = keys.len();
    let lowest = (0..n).min_by_key(|&i| keys[i]).unwrap_or(0);
    let forward: Vec<VertexKey> = (0..n).map(|i| keys[(lowest + i) % n]).collect();
    let backward: Vec<VertexKey> = (0..n).map(|i| keys[(lowest + n - i) % n]).collect();
    if backward < forward { backward } else { forward }
}

/// Counts the contours of each kind, and the exact ties among the segments.
pub(crate) fn totals(contours: &[Contour], segments: &[Segment]) -> Totals {
    let tie_cells: BTreeSet<u32> = segments
        .iter()
        .filter(|s| matches!(s.a.key, VertexKey::SaddleV { .. }) || matches!(s.b.key, VertexKey::SaddleV { .. }))
        .map(|s| s.id.cell)
        .collect();
    Totals {
        open: contours.iter().filter(|c| !c.closed).count(),
        closed: contours.iter().filter(|c| c.closed).count(),
        ties: tie_cells.len(),
    }
}
