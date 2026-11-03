//! Edmonds-Karp maximum flow as a list of events, and a standalone snapshot of the whole state after each. Pure:
//! nothing here touches the DOM.
//!
//! Each search is a breadth-first search of the residual network. From each vertex it takes, the search looks at the
//! transitions that still have room, in the order of their original edge ID. The first time it finds the sink, it
//! stops, and the path it found is augmented all at once. A search that cannot reach the sink ends the run, and the
//! vertices it did reach are one side of a minimum cut.

use std::collections::VecDeque;

/// The most vertices and edges that a graph can have, and the most snapshots that a trace can hold.
pub(crate) const MAX_VERTICES: usize = 12;
pub(crate) const MAX_EDGES: usize = 48;
pub(crate) const SNAPSHOT_BUDGET: usize = 20_000;

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Which end of an edge a vertex ID was found at.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum End {
    From,
    To,
}

/// Why a graph or a trace was refused. Every case has the values that name it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum SolverError {
    TooFewVertices {
        count: usize,
        min: usize,
    },
    TooManyVertices {
        count: usize,
        max: usize,
    },
    TooManyEdges {
        count: usize,
        max: usize,
    },
    DuplicateVertexId {
        vertex_id: u32,
    },
    DuplicateEdgeId {
        edge_id: u32,
    },
    UnknownSource {
        source_id: u32,
    },
    UnknownSink {
        sink_id: u32,
    },
    SameSourceAndSink {
        vertex_id: u32,
    },
    UnknownEndpoint {
        edge_id: u32,
        vertex_id: u32,
        end: End,
    },
    SelfLoop {
        edge_id: u32,
        vertex_id: u32,
    },
    /// The trace needs more snapshots than the budget allows. `attempted` is the number that would have been held.
    EventBudgetExceeded {
        limit: usize,
        attempted: usize,
    },
}

impl std::fmt::Display for SolverError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::TooFewVertices { count, min } => write!(f, "{count} vertices is fewer than the {min} needed"),
            Self::TooManyVertices { count, max } => write!(f, "{count} vertices is more than the {max} allowed"),
            Self::TooManyEdges { count, max } => write!(f, "{count} edges is more than the {max} allowed"),
            Self::DuplicateVertexId { vertex_id } => write!(f, "vertex {vertex_id} is declared twice"),
            Self::DuplicateEdgeId { edge_id } => write!(f, "edge {edge_id} is declared twice"),
            Self::UnknownSource { source_id } => write!(f, "the source {source_id} is not a vertex"),
            Self::UnknownSink { sink_id } => write!(f, "the sink {sink_id} is not a vertex"),
            Self::SameSourceAndSink { vertex_id } => write!(f, "vertex {vertex_id} is both the source and the sink"),
            Self::UnknownEndpoint { edge_id, vertex_id, end } => {
                write!(
                    f,
                    "edge {edge_id} {} unknown vertex {vertex_id}",
                    if *end == End::From { "leaves" } else { "enters" }
                )
            },
            Self::SelfLoop { edge_id, vertex_id } => write!(f, "edge {edge_id} is a loop on vertex {vertex_id}"),
            Self::EventBudgetExceeded { limit, attempted } => {
                write!(f, "the trace needs {attempted} snapshots, and {limit} are allowed")
            },
        }
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// One original edge as it is declared. Its place in the list is its declaration index.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct EdgeSpec {
    pub(crate) id: u32,
    pub(crate) from: u32,
    pub(crate) to: u32,
    pub(crate) capacity: u32,
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// A directed multigraph, as it is given. Nothing about it is checked until a trace is made.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Graph {
    pub(crate) vertices: Vec<u32>,
    pub(crate) source: u32,
    pub(crate) sink: u32,
    pub(crate) edges: Vec<EdgeSpec>,
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Which way along an original edge a residual transition goes. A reverse one belongs to the same edge, and subtracts
/// from its flow.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Direction {
    Forward,
    Reverse,
}

impl Direction {
    /// `F` or `R`, as the text writes it.
    pub(crate) fn letter(self) -> char {
        match self {
            Self::Forward => 'F',
            Self::Reverse => 'R',
        }
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// One step along a path: the original edge that is used, which way, and the vertices it joins in that direction.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Hop {
    pub(crate) edge: usize,
    pub(crate) direction: Direction,
    pub(crate) from: usize,
    pub(crate) to: usize,
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// How a vertex was first reached: from which vertex, by which edge, and which way along it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Pred {
    pub(crate) from: usize,
    pub(crate) edge: usize,
    pub(crate) direction: Direction,
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The transition that was last looked at, and whether the vertex it leads to had been reached already.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Examined {
    pub(crate) hop: Hop,
    pub(crate) already_discovered: bool,
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The state of the search that is going on. Vertices are counted by their place in the list of vertices.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub(crate) struct Bfs {
    /// Which search this is, counted from 1, or 0 before the first.
    pub(crate) search: usize,
    pub(crate) queue: Vec<usize>,
    /// The vertices reached so far, in the order they were reached.
    pub(crate) discovered: Vec<usize>,
    pub(crate) predecessor: Vec<Option<Pred>>,
    pub(crate) examined: Option<Examined>,
    /// The vertex that has just been taken off the queue.
    pub(crate) current: Option<usize>,
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// What an augmentation changed, with everything needed to show it without the snapshot before.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Augmentation {
    pub(crate) path: Vec<Hop>,
    pub(crate) delta: u64,
    pub(crate) flows_before: Vec<u64>,
    pub(crate) flows_after: Vec<u64>,
    pub(crate) total_before: u64,
    pub(crate) total_after: u64,
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The cut that ends the run: the vertices the last search reached, and the original edges that go out of them.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Cut {
    pub(crate) reachable: Vec<usize>,
    pub(crate) crossing: Vec<usize>,
    pub(crate) capacity: u64,
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// One step of the run. Vertices are counted by their place in the list, and edges by their declaration index.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Event {
    Initial,
    SearchStarted,
    VertexDequeued { vertex: usize },
    ArcExamined { hop: Hop, already_discovered: bool },
    VertexDiscovered { vertex: usize, edge: usize, direction: Direction },
    PathFound,
    BottleneckDetermined { bottleneck: u64 },
    FlowAugmented,
    SearchExhausted,
    MinCutShown,
}

impl Event {
    /// The name of the kind of event, which is also the name of the phase.
    pub(crate) fn name(&self) -> &'static str {
        match self {
            Self::Initial => "Initial",
            Self::SearchStarted => "SearchStarted",
            Self::VertexDequeued { .. } => "VertexDequeued",
            Self::ArcExamined { .. } => "ArcExamined",
            Self::VertexDiscovered { .. } => "VertexDiscovered",
            Self::PathFound => "PathFound",
            Self::BottleneckDetermined { .. } => "BottleneckDetermined",
            Self::FlowAugmented => "FlowAugmented",
            Self::SearchExhausted => "SearchExhausted",
            Self::MinCutShown => "MinCutShown",
        }
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The whole state after one event. It is complete: a page can be drawn from it without any other snapshot.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Snapshot {
    pub(crate) index: usize,
    pub(crate) event: Event,
    pub(crate) flows: Vec<u64>,
    pub(crate) total_flow: u64,
    pub(crate) bfs: Bfs,
    pub(crate) path: Vec<Hop>,
    pub(crate) bottleneck: Option<u64>,
    pub(crate) augmentation: Option<Augmentation>,
    pub(crate) final_cut: Option<Cut>,
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// One edge of a validated graph, with its endpoints as places in the list of vertices.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct TraceEdge {
    pub(crate) id: u32,
    pub(crate) from: usize,
    pub(crate) to: usize,
    pub(crate) capacity: u32,
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The whole run of one graph: the graph, and a snapshot after each event, from the empty flow to the cut.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Trace {
    vertex_ids: Vec<u32>,
    source: usize,
    sink: usize,
    edges: Vec<TraceEdge>,
    snapshots: Vec<Snapshot>,
    augmentations: Vec<usize>,
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Checks `graph`, in the order that the errors are defined, and returns what the trace is built from.
///
/// # Errors
///
/// Returns the first error: the vertices (how many, then duplicates), the source and the sink, the edges (how many,
/// then duplicates), every endpoint, and last the loops. So a loop on an unknown vertex is an unknown endpoint.
fn validate(graph: &Graph) -> Result<(), SolverError> {
    let count = graph.vertices.len();
    if count < 2 {
        return Err(SolverError::TooFewVertices { count, min: 2 });
    }
    if count > MAX_VERTICES {
        return Err(SolverError::TooManyVertices { count, max: MAX_VERTICES });
    }
    for (at, id) in graph.vertices.iter().enumerate() {
        if graph.vertices[..at].contains(id) {
            return Err(SolverError::DuplicateVertexId { vertex_id: *id });
        }
    }
    if !graph.vertices.contains(&graph.source) {
        return Err(SolverError::UnknownSource { source_id: graph.source });
    }
    if !graph.vertices.contains(&graph.sink) {
        return Err(SolverError::UnknownSink { sink_id: graph.sink });
    }
    if graph.source == graph.sink {
        return Err(SolverError::SameSourceAndSink { vertex_id: graph.source });
    }
    if graph.edges.len() > MAX_EDGES {
        return Err(SolverError::TooManyEdges {
            count: graph.edges.len(),
            max: MAX_EDGES,
        });
    }
    for (at, edge) in graph.edges.iter().enumerate() {
        if graph.edges[..at].iter().any(|earlier| earlier.id == edge.id) {
            return Err(SolverError::DuplicateEdgeId { edge_id: edge.id });
        }
    }
    for edge in &graph.edges {
        for (end, vertex_id) in [(End::From, edge.from), (End::To, edge.to)] {
            if !graph.vertices.contains(&vertex_id) {
                return Err(SolverError::UnknownEndpoint {
                    edge_id: edge.id,
                    vertex_id,
                    end,
                });
            }
        }
    }
    if let Some(edge) = graph.edges.iter().find(|edge| edge.from == edge.to) {
        return Err(SolverError::SelfLoop {
            edge_id: edge.id,
            vertex_id: edge.from,
        });
    }
    Ok(())
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// What a test can change about how a trace is built. A shipped build has no such switch.
#[derive(Debug, Clone, Copy)]
#[cfg_attr(not(test), derive(Default))]
struct Options {
    /// Whether the search may use a reverse transition. Turning this off is how a test shows that its inputs can tell a
    /// solver that has no reverse arcs from one that has.
    #[cfg(test)]
    allow_reverse: bool,
}

#[cfg(test)]
impl Default for Options {
    fn default() -> Self {
        Self { allow_reverse: true }
    }
}

impl Options {
    /// Whether the search may use a reverse transition, which in a shipped build it always may.
    fn reverse_allowed(self) -> bool {
        #[cfg(test)]
        {
            self.allow_reverse
        }
        #[cfg(not(test))]
        {
            true
        }
    }
}

impl Trace {
    /// The trace of `graph`, within [`SNAPSHOT_BUDGET`] snapshots.
    ///
    /// # Errors
    ///
    /// Returns `Err` if the graph is not valid, or the run needs more snapshots than the budget.
    pub(crate) fn build(graph: &Graph) -> Result<Self, SolverError> {
        Self::build_within(graph, SNAPSHOT_BUDGET)
    }

    /// [`Trace::build`] with a budget of `limit` snapshots, which a test uses to reach the limit on a small graph.
    ///
    /// # Errors
    ///
    /// Returns `Err` if the graph is not valid, or the run needs more than `limit` snapshots. Nothing is returned of a
    /// run that was cut short.
    pub(crate) fn build_within(graph: &Graph, limit: usize) -> Result<Self, SolverError> {
        Self::build_with(graph, limit, Options::default())
    }

    /// The trace of `graph` when the search may not use a reverse transition. It is the negative control of the tests.
    #[cfg(test)]
    pub(crate) fn build_without_reverse(graph: &Graph) -> Result<Self, SolverError> {
        Self::build_with(graph, SNAPSHOT_BUDGET, Options { allow_reverse: false })
    }

    fn build_with(graph: &Graph, limit: usize, options: Options) -> Result<Self, SolverError> {
        validate(graph)?;
        let index_of = |id: u32| graph.vertices.iter().position(|&v| v == id).unwrap_or(0);
        let mut trace = Self {
            vertex_ids: graph.vertices.clone(),
            source: index_of(graph.source),
            sink: index_of(graph.sink),
            edges: graph
                .edges
                .iter()
                .map(|edge| TraceEdge {
                    id: edge.id,
                    from: index_of(edge.from),
                    to: index_of(edge.to),
                    capacity: edge.capacity,
                })
                .collect(),
            snapshots: Vec::new(),
            augmentations: Vec::new(),
        };
        Run::new(&mut trace, limit, options).run()?;
        Ok(trace)
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    /// How many snapshots there are, which is one more than there are events.
    #[cfg(test)]
    pub(crate) fn len(&self) -> usize {
        self.snapshots.len()
    }

    /// The index of the last snapshot, which the page calls M.
    pub(crate) fn last(&self) -> usize {
        self.snapshots.len().saturating_sub(1)
    }

    pub(crate) fn snapshot(&self, index: usize) -> Option<&Snapshot> {
        self.snapshots.get(index)
    }

    pub(crate) fn snapshots(&self) -> &[Snapshot] {
        &self.snapshots
    }

    pub(crate) fn vertex_count(&self) -> usize {
        self.vertex_ids.len()
    }

    pub(crate) fn source(&self) -> usize {
        self.source
    }

    pub(crate) fn sink(&self) -> usize {
        self.sink
    }

    pub(crate) fn edges(&self) -> &[TraceEdge] {
        &self.edges
    }

    /// The maximum flow, which is the total flow of the last snapshot.
    #[cfg(test)]
    pub(crate) fn max_flow(&self) -> u64 {
        self.snapshots.last().map_or(0, |last| last.total_flow)
    }

    /// The snapshots whose event is a flow augmentation, in order.
    #[cfg(test)]
    pub(crate) fn augmentation_indices(&self) -> &[usize] {
        &self.augmentations
    }

    /// The last augmentation that is strictly before `index`.
    pub(crate) fn previous_augmentation(&self, index: usize) -> Option<usize> {
        let before = self.augmentations.partition_point(|&at| at < index);
        before.checked_sub(1).map(|at| self.augmentations[at])
    }

    /// The first augmentation that is strictly after `index`.
    pub(crate) fn next_augmentation(&self, index: usize) -> Option<usize> {
        let up_to = self.augmentations.partition_point(|&at| at <= index);
        self.augmentations.get(up_to).copied()
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// A run in progress: the trace that is being filled, and everything that changes as it goes.
struct Run<'a> {
    trace: &'a mut Trace,
    limit: usize,
    options: Options,
    flows: Vec<u64>,
    total: u64,
    bfs: Bfs,
    path: Vec<Hop>,
    bottleneck: Option<u64>,
    augmentation: Option<Augmentation>,
    cut: Option<Cut>,
}

impl<'a> Run<'a> {
    fn new(trace: &'a mut Trace, limit: usize, options: Options) -> Self {
        let (edges, vertices) = (trace.edges.len(), trace.vertex_ids.len());
        Self {
            trace,
            limit,
            options,
            flows: vec![0; edges],
            total: 0,
            bfs: Bfs {
                predecessor: vec![None; vertices],
                ..Bfs::default()
            },
            path: Vec::new(),
            bottleneck: None,
            augmentation: None,
            cut: None,
        }
    }

    /// Adds the snapshot that `event` makes. The snapshot that would pass the budget is refused, and nothing is added.
    fn emit(&mut self, event: Event) -> Result<(), SolverError> {
        let index = self.trace.snapshots.len();
        if index >= self.limit {
            return Err(SolverError::EventBudgetExceeded {
                limit: self.limit,
                attempted: index + 1,
            });
        }
        if event == Event::FlowAugmented {
            self.trace.augmentations.push(index);
        }
        self.trace.snapshots.push(Snapshot {
            index,
            event,
            flows: self.flows.clone(),
            total_flow: self.total,
            bfs: self.bfs.clone(),
            path: self.path.clone(),
            bottleneck: self.bottleneck,
            augmentation: self.augmentation.clone(),
            final_cut: self.cut.clone(),
        });
        Ok(())
    }

    /// The transitions that leave `vertex` and have room, by original edge ID. A reverse one is the other end of an
    /// edge, and has room only as far as the edge has flow. An edge is never a loop, so it leaves a vertex one way at
    /// most.
    fn candidates(&self, vertex: usize) -> Vec<Hop> {
        let mut found: Vec<(u32, Hop)> = Vec::new();
        for (at, edge) in self.trace.edges.iter().enumerate() {
            let flow = self.flows[at];
            if edge.from == vertex && u64::from(edge.capacity) > flow {
                found.push((
                    edge.id,
                    Hop {
                        edge: at,
                        direction: Direction::Forward,
                        from: edge.from,
                        to: edge.to,
                    },
                ));
            }
            if self.options.reverse_allowed() && edge.to == vertex && flow > 0 {
                found.push((
                    edge.id,
                    Hop {
                        edge: at,
                        direction: Direction::Reverse,
                        from: edge.to,
                        to: edge.from,
                    },
                ));
            }
        }
        found.sort_by_key(|&(id, _)| id);
        found.into_iter().map(|(_, hop)| hop).collect()
    }

    /// How much room a hop has: what is left of the edge going forward, and what is on it going back.
    fn room(&self, hop: &Hop) -> u64 {
        let flow = self.flows[hop.edge];
        match hop.direction {
            Direction::Forward => u64::from(self.trace.edges[hop.edge].capacity) - flow,
            Direction::Reverse => flow,
        }
    }

    fn run(&mut self) -> Result<(), SolverError> {
        self.emit(Event::Initial)?;
        loop {
            if self.search()? {
                self.augment()?;
            } else {
                return Ok(());
            }
        }
    }

    /// Runs one search. It is `true` if the search reached the sink, with the path in `self.path`.
    fn search(&mut self) -> Result<bool, SolverError> {
        let (source, sink, count) = (self.trace.source, self.trace.sink, self.trace.vertex_ids.len());
        self.path.clear();
        self.bottleneck = None;
        self.augmentation = None;
        self.bfs = Bfs {
            search: self.bfs.search + 1,
            queue: vec![source],
            discovered: vec![source],
            predecessor: vec![None; count],
            examined: None,
            current: None,
        };
        self.emit(Event::SearchStarted)?;
        let mut queue: VecDeque<usize> = VecDeque::from([source]);
        while let Some(vertex) = queue.pop_front() {
            self.bfs.queue.remove(0);
            self.bfs.current = Some(vertex);
            self.bfs.examined = None;
            self.emit(Event::VertexDequeued { vertex })?;
            for hop in self.candidates(vertex) {
                let seen = self.bfs.discovered.contains(&hop.to);
                self.bfs.examined = Some(Examined { hop, already_discovered: seen });
                self.emit(Event::ArcExamined { hop, already_discovered: seen })?;
                if seen {
                    continue;
                }
                self.bfs.discovered.push(hop.to);
                self.bfs.predecessor[hop.to] = Some(Pred {
                    from: vertex,
                    edge: hop.edge,
                    direction: hop.direction,
                });
                self.bfs.queue.push(hop.to);
                queue.push_back(hop.to);
                self.emit(Event::VertexDiscovered {
                    vertex: hop.to,
                    edge: hop.edge,
                    direction: hop.direction,
                })?;
                if hop.to == sink {
                    return Ok(true);
                }
            }
        }
        self.bfs.current = None;
        self.bfs.examined = None;
        self.emit(Event::SearchExhausted)?;
        let reachable = {
            let mut found = self.bfs.discovered.clone();
            found.sort_unstable();
            found
        };
        let crossing: Vec<usize> = (0..self.trace.edges.len())
            .filter(|&at| {
                let edge = &self.trace.edges[at];
                reachable.contains(&edge.from) && !reachable.contains(&edge.to)
            })
            .collect();
        let capacity = crossing.iter().map(|&at| u64::from(self.trace.edges[at].capacity)).sum();
        self.cut = Some(Cut { reachable, crossing, capacity });
        self.emit(Event::MinCutShown)?;
        Ok(false)
    }

    /// Follows the predecessors back from the sink, finds the bottleneck, and changes every flow on the path at once.
    fn augment(&mut self) -> Result<(), SolverError> {
        let (source, sink) = (self.trace.source, self.trace.sink);
        let mut path = Vec::new();
        let mut at = sink;
        while at != source {
            let Some(pred) = self.bfs.predecessor[at] else {
                break;
            };
            path.push(Hop {
                edge: pred.edge,
                direction: pred.direction,
                from: pred.from,
                to: at,
            });
            at = pred.from;
        }
        path.reverse();
        self.path = path.clone();
        self.bfs.current = None;
        self.bfs.examined = None;
        self.emit(Event::PathFound)?;
        let bottleneck = path.iter().map(|hop| self.room(hop)).min().unwrap_or(0);
        self.bottleneck = Some(bottleneck);
        self.emit(Event::BottleneckDetermined { bottleneck })?;
        let (flows_before, total_before) = (self.flows.clone(), self.total);
        for hop in &path {
            match hop.direction {
                Direction::Forward => self.flows[hop.edge] += bottleneck,
                Direction::Reverse => self.flows[hop.edge] -= bottleneck,
            }
        }
        self.total += bottleneck;
        self.augmentation = Some(Augmentation {
            path,
            delta: bottleneck,
            flows_before,
            flows_after: self.flows.clone(),
            total_before,
            total_after: self.total,
        });
        self.emit(Event::FlowAugmented)
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[cfg(test)]
mod oracle;

#[cfg(test)]
mod unit_tests;
