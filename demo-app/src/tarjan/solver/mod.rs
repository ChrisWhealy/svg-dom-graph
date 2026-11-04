//! Tarjan's algorithm, as a list of events and a snapshot after each. Pure: nothing here touches the DOM.
//!
//! The whole trace is worked out before anything is drawn. A snapshot is a full copy of the state after one event, so
//! going back is choosing an earlier snapshot, and never undoing a step. The search keeps its own stack of calls, so
//! that it can stop after any event and carry on, and so that the stack of the algorithm is a different thing from it.

use std::fmt;

/// The most vertices, edges and snapshots that a graph may have. The demo's own graph is within these, and so are the
/// small graphs that the tests use.
pub(crate) const MAX_VERTICES: usize = 64;
pub(crate) const MAX_EDGES: usize = 128;
pub(crate) const MAX_SNAPSHOTS: usize = 512;

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Why a graph could not be solved.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum SolveError {
    TooManyVertices(usize),
    TooManyEdges(usize),
    /// An edge names a vertex that the graph does not have.
    UnknownVertex(usize),
    /// The same edge is given twice.
    DuplicateEdge(usize, usize),
    /// The trace would have more snapshots than the budget allows.
    TooManySnapshots,
}

impl fmt::Display for SolveError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::TooManyVertices(n) => write!(f, "{n} vertices is more than {MAX_VERTICES}"),
            Self::TooManyEdges(n) => write!(f, "{n} edges is more than {MAX_EDGES}"),
            Self::UnknownVertex(v) => write!(f, "an edge names the vertex {v}, which there is not"),
            Self::DuplicateEdge(a, b) => write!(f, "the edge {a} to {b} is given twice"),
            Self::TooManySnapshots => write!(f, "the trace would have more than {MAX_SNAPSHOTS} snapshots"),
        }
    }
}

impl std::error::Error for SolveError {}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// A directed graph, with each vertex's targets in ascending order.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Graph {
    adjacency: Vec<Vec<usize>>,
}

impl Graph {
    /// A graph of `n` vertices with these edges. A self-loop is an edge, and an edge twice is refused.
    ///
    /// # Errors
    ///
    /// Returns which limit the graph passes, or which edge is wrong.
    pub(crate) fn from_edges(n: usize, edges: &[(usize, usize)]) -> Result<Self, SolveError> {
        if n > MAX_VERTICES {
            return Err(SolveError::TooManyVertices(n));
        }
        if edges.len() > MAX_EDGES {
            return Err(SolveError::TooManyEdges(edges.len()));
        }
        let mut adjacency = vec![Vec::new(); n];
        for &(a, b) in edges {
            let unknown = [a, b].into_iter().find(|&v| v >= n);
            if let Some(v) = unknown {
                return Err(SolveError::UnknownVertex(v));
            }
            if adjacency[a].contains(&b) {
                return Err(SolveError::DuplicateEdge(a, b));
            }
            adjacency[a].push(b);
        }
        for list in &mut adjacency {
            list.sort_unstable();
        }
        Ok(Self { adjacency })
    }

    pub(crate) fn len(&self) -> usize {
        self.adjacency.len()
    }

    pub(crate) fn targets(&self, v: usize) -> &[usize] {
        &self.adjacency[v]
    }

    /// Every edge, by source and then by target.
    pub(crate) fn edges(&self) -> Vec<(usize, usize)> {
        self.adjacency
            .iter()
            .enumerate()
            .flat_map(|(v, list)| list.iter().map(move |&w| (v, w)))
            .collect()
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// What an inspected edge leads to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Kind {
    /// A vertex that has not been discovered, so the search will go down this edge.
    Tree,
    /// A vertex that has been discovered and is still on the stack of the algorithm.
    Stack,
    /// A vertex whose component has been emitted, which changes nothing.
    Closed,
}

/// What happened, with what it changed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Event {
    /// `v` was discovered, given its index and lowlink, and pushed.
    Enter { v: usize },
    /// The edge `v` to `w` was looked at. A stack edge may have lowered `v`'s lowlink, from `before` to `after`.
    Inspect {
        v: usize,
        w: usize,
        kind: Kind,
        before: usize,
        after: usize,
    },
    /// The search came back from `child` to `parent`, and `parent`'s lowlink went from `before` to `after`.
    Return {
        parent: usize,
        child: usize,
        before: usize,
        after: usize,
    },
    /// The vertices of one component were popped together, `root` last. `members` is in pop order.
    Closed { root: usize, members: Vec<usize> },
    /// The search is over.
    Complete,
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The whole state of the algorithm after one event.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Snapshot {
    /// The event that made this snapshot, which the first snapshot lacks.
    pub(crate) event: Option<Event>,
    pub(crate) index: Vec<Option<usize>>,
    pub(crate) lowlink: Vec<Option<usize>>,
    pub(crate) on_stack: Vec<bool>,
    /// The vertices that are discovered and not yet in a component, in the order they were pushed.
    pub(crate) scc_stack: Vec<usize>,
    /// The chain of vertices whose calls have not returned, which is not the same as the stack above.
    pub(crate) chain: Vec<usize>,
    /// The components that have been emitted, in pop order, each in pop order.
    pub(crate) components: Vec<Vec<usize>>,
    /// The component of each vertex, as an index into `components`, once it has been emitted.
    pub(crate) membership: Vec<Option<usize>>,
}

impl Snapshot {
    fn blank(n: usize) -> Self {
        Self {
            event: None,
            index: vec![None; n],
            lowlink: vec![None; n],
            on_stack: vec![false; n],
            scc_stack: Vec::new(),
            chain: Vec::new(),
            components: Vec::new(),
            membership: vec![None; n],
        }
    }

    /// The vertex that the page draws attention to: the event's own vertex, the root of the component that closed, or
    /// for the end the root of the last one. The first snapshot, before any event, draws it at vertex 0. A graph with
    /// no vertex has none.
    pub(crate) fn attention(&self) -> Option<usize> {
        match &self.event {
            None => (!self.index.is_empty()).then_some(0),
            Some(Event::Enter { v }) | Some(Event::Inspect { v, .. }) => Some(*v),
            Some(Event::Return { parent, .. }) => Some(*parent),
            Some(Event::Closed { root, .. }) => Some(*root),
            Some(Event::Complete) => self.components.last().and_then(|c| c.last().copied()),
        }
    }

    /// Checks the rules that hold at every snapshot.
    ///
    /// # Errors
    ///
    /// Returns a sentence for the first rule that is broken.
    #[cfg(test)]
    pub(crate) fn check(&self) -> Result<(), String> {
        let n = self.index.len();
        for v in 0..n {
            let on_stack = self.on_stack[v];
            let times = self.scc_stack.iter().filter(|&&s| s == v).count();
            if times != usize::from(on_stack) {
                return Err(format!("vertex {v} is on the stack {times} times and its flag is {on_stack}"));
            }
            match (self.index[v], self.lowlink[v]) {
                (None, None) => {
                    if on_stack || self.membership[v].is_some() {
                        return Err(format!("vertex {v} is not discovered but is on the stack or in a component"));
                    }
                },
                (Some(i), Some(l)) => {
                    if l > i {
                        return Err(format!("vertex {v} has lowlink {l} above its index {i}"));
                    }
                    if on_stack == self.membership[v].is_some() {
                        return Err(format!("vertex {v} is both or neither of on the stack and in a component"));
                    }
                },
                _ => return Err(format!("vertex {v} has an index without a lowlink, or the other way")),
            }
        }
        let mut seen = vec![false; n];
        for (c, component) in self.components.iter().enumerate() {
            for &v in component {
                if std::mem::replace(&mut seen[v], true) {
                    return Err(format!("vertex {v} is in two components"));
                }
                if self.membership[v] != Some(c) {
                    return Err(format!("vertex {v} says it is in another component than {c}"));
                }
            }
        }
        Ok(())
    }
}

/// One call that has not returned: the vertex, and which of its targets comes next.
struct Frame {
    v: usize,
    next: usize,
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Every snapshot of a run, in order. Snapshot 0 is before anything has happened, and event k makes snapshot k.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Trace {
    snapshots: Vec<Snapshot>,
}

impl Trace {
    /// Runs the algorithm on `graph`, starting a search at each vertex that is still undiscovered, in ID order.
    ///
    /// # Errors
    ///
    /// Returns [`SolveError::TooManySnapshots`] if the run would pass the budget.
    pub(crate) fn build(graph: &Graph) -> Result<Self, SolveError> {
        let n = graph.len();
        let mut state = Snapshot::blank(n);
        let mut snapshots = vec![state.clone()];
        let mut next_index = 0;
        let mut frames: Vec<Frame> = Vec::new();
        let push = |state: &Snapshot, snapshots: &mut Vec<Snapshot>| -> Result<(), SolveError> {
            if snapshots.len() >= MAX_SNAPSHOTS {
                return Err(SolveError::TooManySnapshots);
            }
            snapshots.push(state.clone());
            Ok(())
        };
        for root in 0..n {
            if state.index[root].is_some() {
                continue;
            }
            Self::enter(&mut state, &mut frames, &mut next_index, root);
            state.event = Some(Event::Enter { v: root });
            push(&state, &mut snapshots)?;
            while let Some(frame) = frames.last_mut() {
                let v = frame.v;
                if let Some(&w) = graph.targets(v).get(frame.next) {
                    frame.next += 1;
                    let kind = match (state.index[w], state.on_stack[w]) {
                        (None, _) => Kind::Tree,
                        (Some(_), true) => Kind::Stack,
                        (Some(_), false) => Kind::Closed,
                    };
                    let before = state.lowlink[v].unwrap_or(0);
                    if kind == Kind::Stack {
                        state.lowlink[v] = Some(before.min(state.index[w].unwrap_or(before)));
                    }
                    let after = state.lowlink[v].unwrap_or(0);
                    state.event = Some(Event::Inspect { v, w, kind, before, after });
                    push(&state, &mut snapshots)?;
                    if kind == Kind::Tree {
                        Self::enter(&mut state, &mut frames, &mut next_index, w);
                        state.event = Some(Event::Enter { v: w });
                        push(&state, &mut snapshots)?;
                    }
                    continue;
                }
                // Every target has been looked at, so this call is over.
                if state.lowlink[v] == state.index[v] {
                    let mut members = Vec::new();
                    while let Some(x) = state.scc_stack.pop() {
                        state.on_stack[x] = false;
                        members.push(x);
                        if x == v {
                            break;
                        }
                    }
                    let id = state.components.len();
                    for &x in &members {
                        state.membership[x] = Some(id);
                    }
                    state.components.push(members.clone());
                    state.event = Some(Event::Closed { root: v, members });
                    push(&state, &mut snapshots)?;
                }
                frames.pop();
                state.chain.pop();
                if let Some(parent) = frames.last().map(|f| f.v) {
                    let before = state.lowlink[parent].unwrap_or(0);
                    let child_low = state.lowlink[v].unwrap_or(before);
                    state.lowlink[parent] = Some(before.min(child_low));
                    state.event = Some(Event::Return {
                        parent,
                        child: v,
                        before,
                        after: before.min(child_low),
                    });
                    push(&state, &mut snapshots)?;
                }
            }
        }
        state.event = Some(Event::Complete);
        push(&state, &mut snapshots)?;
        Ok(Self { snapshots })
    }

    /// Discovers `v`: gives it the next index as both its index and its lowlink, and pushes it and its call.
    fn enter(state: &mut Snapshot, frames: &mut Vec<Frame>, next_index: &mut usize, v: usize) {
        state.index[v] = Some(*next_index);
        state.lowlink[v] = Some(*next_index);
        *next_index += 1;
        state.scc_stack.push(v);
        state.on_stack[v] = true;
        state.chain.push(v);
        frames.push(Frame { v, next: 0 });
    }

    /// The index of the last snapshot, which is the number of events.
    pub(crate) fn last(&self) -> usize {
        self.snapshots.len() - 1
    }

    pub(crate) fn snapshot(&self, index: usize) -> Option<&Snapshot> {
        self.snapshots.get(index)
    }

    #[cfg(test)]
    pub(crate) fn snapshots(&self) -> &[Snapshot] {
        &self.snapshots
    }

    /// The indices of the snapshots that close a component, in order.
    pub(crate) fn closures(&self) -> Vec<usize> {
        self.snapshots
            .iter()
            .enumerate()
            .filter(|(_, s)| matches!(s.event, Some(Event::Closed { .. })))
            .map(|(i, _)| i)
            .collect()
    }

    /// The closest closure strictly before `index`, if there is one.
    pub(crate) fn previous_closure(&self, index: usize) -> Option<usize> {
        self.closures().into_iter().rev().find(|&c| c < index)
    }

    /// The closest closure strictly after `index`, if there is one.
    pub(crate) fn next_closure(&self, index: usize) -> Option<usize> {
        self.closures().into_iter().find(|&c| c > index)
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[cfg(test)]
mod oracle;
#[cfg(test)]
mod unit_tests;
