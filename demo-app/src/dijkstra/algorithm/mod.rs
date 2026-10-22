//! Dijkstra's shortest-path algorithm in plain Rust, recorded one step at a time so that the demo can show every one.
//! Nothing here touches the DOM or the library.
//!
//! The algorithm keeps a tentative distance for every vertex, and a queue of the vertices it has reached. It takes the
//! nearest vertex from the queue and settles it, since no later route can be shorter. Then it looks at each edge that
//! leaves that vertex, and keeps a route that is strictly shorter than the one it knew. A vertex that was queued with an
//! older, longer distance stays in the queue. When that entry is taken out, it is ignored.
//!
//! Distances are whole numbers, and `None` means that a vertex has not been reached. Nothing stands in for infinity, so
//! a real distance can have any value. A distance that would overflow is an error, not a wrap.
//!
//! A tie keeps the first route found: a candidate that is only equal to the distance a vertex has does not replace it.
//! The queue is ordered by distance, then by vertex number, so the same graph always gives the same steps.

use std::{cmp::Reverse, collections::BinaryHeap, fmt};

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// One directed edge, as the caller gave it: from a vertex number, to a vertex number, with a weight that may be
/// negative, so that the check for it has something to refuse.
pub(super) type RawEdge = (usize, usize, i64);

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// One directed edge of a [`Graph`], with a weight that is not negative.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct WeightedEdge {
    pub(super) from: usize,
    pub(super) to: usize,
    pub(super) weight: u64,
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Why a graph, or a run on it, was refused.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) enum Error {
    /// An edge has a weight below zero. Dijkstra's algorithm settles a vertex for good, which is wrong if a later edge
    /// can make a route shorter.
    NegativeWeight { edge: usize, weight: i64 },
    /// An edge starts and ends at the same vertex. The library cannot draw one, so the demo does not accept it.
    SelfLoop { edge: usize, vertex: usize },
    /// An edge, the source or the destination names a vertex that the graph does not have.
    UnknownVertex { vertex: usize },
    /// Adding an edge's weight to a distance went past what a `u64` holds.
    DistanceOverflow { edge: usize },
    /// The graph has no vertex to start from.
    Empty,
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NegativeWeight { edge, weight } => {
                write!(f, "Edge {edge} has the weight {weight}. Weights must not be negative.")
            },
            Self::SelfLoop { edge, vertex } => {
                write!(
                    f,
                    "Edge {edge} starts and ends at vertex {vertex}. A self-loop cannot be drawn."
                )
            },
            Self::UnknownVertex { vertex } => write!(f, "There is no vertex {vertex}."),
            Self::DistanceOverflow { edge } => write!(f, "A distance through edge {edge} is too large to hold."),
            Self::Empty => write!(f, "The graph has no vertices."),
        }
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// A directed graph with a name for each vertex and a weight on each edge. Edges keep the order they were given in,
/// and that order decides which of several equal choices the algorithm makes first.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct Graph {
    names: Vec<String>,
    edges: Vec<WeightedEdge>,
}

impl Graph {
    /// A graph of the vertices `names` and the edges `edges`. Edges are checked in order, and the first that is not
    /// allowed is the one reported.
    ///
    /// # Errors
    ///
    /// Returns [`Error::Empty`] for no vertices, [`Error::UnknownVertex`] for an edge that names a vertex that is not
    /// there, [`Error::SelfLoop`] for an edge from a vertex to itself, and [`Error::NegativeWeight`] for a weight below
    /// zero.
    pub(super) fn new(names: &[&str], edges: &[RawEdge]) -> Result<Self, Error> {
        if names.is_empty() {
            return Err(Error::Empty);
        }
        let mut checked = Vec::with_capacity(edges.len());
        for (edge, &(from, to, weight)) in edges.iter().enumerate() {
            for vertex in [from, to] {
                if vertex >= names.len() {
                    return Err(Error::UnknownVertex { vertex });
                }
            }
            if from == to {
                return Err(Error::SelfLoop { edge, vertex: from });
            }
            let weight = u64::try_from(weight).map_err(|_| Error::NegativeWeight { edge, weight })?;
            checked.push(WeightedEdge { from, to, weight });
        }
        Ok(Self {
            names: names.iter().map(|&name| name.to_owned()).collect(),
            edges: checked,
        })
    }

    pub(super) fn names(&self) -> &[String] {
        &self.names
    }

    pub(super) fn edges(&self) -> &[WeightedEdge] {
        &self.edges
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// One entry of the queue as it is shown: the distance it was queued with and the vertex it is for. It is `stale` if the
/// vertex has been settled since, or has a shorter distance now.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct QueueEntry {
    pub(super) distance: u64,
    pub(super) vertex: usize,
    pub(super) stale: bool,
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// What a step did to the edge it examined.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Relaxation {
    /// The route through the edge is strictly shorter, so the distance and the predecessor of its end were replaced.
    Improved,
    /// The route is no shorter than the one the end vertex has, or the end vertex is settled already.
    Kept,
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The state of the run after one step: everything the demo shows.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(super) struct Snapshot {
    /// The tentative distance of each vertex, or `None` if it has not been reached.
    pub(super) distance: Vec<Option<u64>>,
    /// The vertex each vertex was reached from, or `None` for the source and for a vertex not reached.
    pub(super) predecessor: Vec<Option<usize>>,
    /// The edge each vertex was reached by. This is how one of several parallel edges is told apart.
    pub(super) via: Vec<Option<usize>>,
    /// Whether each vertex is settled: its distance is final.
    pub(super) settled: Vec<bool>,
    /// What is in the queue, nearest first. Entries that are out of date stay in it, marked, until they are taken out.
    pub(super) queue: Vec<QueueEntry>,
    /// The vertex being worked on: the one just settled, whose edges are being looked at.
    pub(super) active: Option<usize>,
    /// The edge being looked at, and what came of it.
    pub(super) examined: Option<(usize, Relaxation)>,
    /// The edges of the route to the destination, from the source, once the run has found it.
    pub(super) path: Vec<usize>,
    /// What the step did, in a sentence or two of plain text.
    pub(super) note: String,
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The working state of a run, from which each [`Snapshot`] is taken.
struct Run<'a> {
    graph: &'a Graph,
    distance: Vec<Option<u64>>,
    predecessor: Vec<Option<usize>>,
    via: Vec<Option<usize>>,
    settled: Vec<bool>,
    heap: BinaryHeap<Reverse<(u64, usize)>>,
    steps: Vec<Snapshot>,
}

impl Run<'_> {
    /// Records a step on the state as it is now. The caller adds what the step is about.
    fn record(&mut self, note: String) -> &mut Snapshot {
        let mut queue: Vec<QueueEntry> = self
            .heap
            .iter()
            .map(|&Reverse((distance, vertex))| QueueEntry {
                distance,
                vertex,
                stale: self.settled[vertex] || self.distance[vertex] != Some(distance),
            })
            .collect();
        queue.sort_unstable_by_key(|entry| (entry.distance, entry.vertex));
        self.steps.push(Snapshot {
            distance: self.distance.clone(),
            predecessor: self.predecessor.clone(),
            via: self.via.clone(),
            settled: self.settled.clone(),
            queue,
            note,
            ..Snapshot::default()
        });
        self.steps.last_mut().unwrap_or_else(|| unreachable!("a step was just pushed"))
    }

    fn name(&self, vertex: usize) -> &str {
        &self.graph.names[vertex]
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The edges of the route from the source to `destination`, in order, read back from the edge each vertex was reached
/// by. Empty if the destination is the source, or has not been reached.
fn path_to(graph: &Graph, via: &[Option<usize>], destination: usize) -> Vec<usize> {
    let mut path = Vec::new();
    let mut at = destination;
    while let Some(edge) = via[at] {
        path.push(edge);
        at = graph.edges[edge].from;
    }
    path.reverse();
    path
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Every step of finding the shortest routes from `source`. With a `destination`, the run stops as soon as that vertex
/// is settled, and the last step shows its route. Without one, the run goes on until the queue is empty.
///
/// The first step is the start, with the source at distance 0. Then each step either takes an entry out of the queue,
/// settling a vertex or ignoring a stale entry, or looks at one edge that leaves the vertex just settled. The last step
/// says how the run ended.
///
/// # Errors
///
/// Returns [`Error::UnknownVertex`] if `source` or `destination` is not a vertex of `graph`, and
/// [`Error::DistanceOverflow`] if adding a weight to a distance would overflow.
pub(super) fn steps(graph: &Graph, source: usize, destination: Option<usize>) -> Result<Vec<Snapshot>, Error> {
    let count = graph.names.len();
    for vertex in std::iter::once(source).chain(destination) {
        if vertex >= count {
            return Err(Error::UnknownVertex { vertex });
        }
    }
    let mut run = Run {
        graph,
        distance: vec![None; count],
        predecessor: vec![None; count],
        via: vec![None; count],
        settled: vec![false; count],
        heap: BinaryHeap::new(),
        steps: Vec::new(),
    };
    run.distance[source] = Some(0);
    run.heap.push(Reverse((0, source)));
    let note = format!(
        "Start at {}, with distance 0. Every other vertex has not been reached, which is shown as ∞.",
        run.name(source)
    );
    run.record(note);

    while let Some(Reverse((distance, vertex))) = run.heap.pop() {
        if run.settled[vertex] || run.distance[vertex] != Some(distance) {
            let note = format!(
                "Take {distance} for {} out of the queue. It is out of date, so it is ignored.",
                run.name(vertex)
            );
            run.record(note);
            continue;
        }
        run.settled[vertex] = true;
        let note = format!(
            "Take {distance} for {} out of the queue. It is the nearest, so {} is settled at {distance}.",
            run.name(vertex),
            run.name(vertex)
        );
        run.record(note).active = Some(vertex);
        if destination == Some(vertex) {
            break;
        }
        for (index, edge) in graph.edges.iter().enumerate().filter(|(_, edge)| edge.from == vertex) {
            let candidate = distance
                .checked_add(edge.weight)
                .ok_or(Error::DistanceOverflow { edge: index })?;
            let (from, to) = (run.name(edge.from).to_owned(), run.name(edge.to).to_owned());
            let current = run.distance[edge.to];
            let (relaxation, note) = if run.settled[edge.to] {
                (
                    Relaxation::Kept,
                    format!(
                        "Edge {from} to {to}, weight {}: {to} is settled already, so it stays.",
                        edge.weight
                    ),
                )
            } else if current.is_none_or(|current| candidate < current) {
                run.distance[edge.to] = Some(candidate);
                run.predecessor[edge.to] = Some(edge.from);
                run.via[edge.to] = Some(index);
                run.heap.push(Reverse((candidate, edge.to)));
                let was = current.map_or("∞".to_owned(), |current| current.to_string());
                (
                    Relaxation::Improved,
                    format!(
                        "Edge {from} to {to}, weight {}: {distance} + {} = {candidate}, which is less than {was}. \
                         {to} is now reached from {from}.",
                        edge.weight, edge.weight
                    ),
                )
            } else {
                (
                    Relaxation::Kept,
                    format!(
                        "Edge {from} to {to}, weight {}: {distance} + {} = {candidate}, which is not less than {}. \
                         {to} keeps its route.",
                        edge.weight,
                        edge.weight,
                        current.map_or("∞".to_owned(), |current| current.to_string())
                    ),
                )
            };
            let step = run.record(note);
            step.active = Some(vertex);
            step.examined = Some((index, relaxation));
        }
    }

    let (path, note) = match destination {
        Some(destination) if run.settled[destination] => {
            let path = path_to(graph, &run.via, destination);
            let note = if path.is_empty() {
                format!(
                    "{} is the source, so the route to it has no edges and a length of 0.",
                    run.name(destination)
                )
            } else {
                let route: Vec<&str> = std::iter::once(run.name(source))
                    .chain(path.iter().map(|&edge| run.name(graph.edges[edge].to)))
                    .collect();
                format!(
                    "The shortest route to {} is {}, with a length of {}.",
                    run.name(destination),
                    route.join(" to "),
                    run.distance[destination].unwrap_or_default()
                )
            };
            (path, note)
        },
        Some(destination) => (
            Vec::new(),
            format!("No route from {} to {}.", run.name(source), run.name(destination)),
        ),
        None => (
            Vec::new(),
            "The queue is empty, so every vertex that can be reached is settled.".to_owned(),
        ),
    };
    run.record(note).path = path;
    Ok(run.steps)
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[cfg(test)]
mod unit_tests;
