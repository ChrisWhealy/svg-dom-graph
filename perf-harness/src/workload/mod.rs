//! Turns a manifest into the stream of operations a runner has to perform.
//!
//! The stream is produced by a visitor and never held in memory, because a long run of a large graph has millions of
//! operations. Everything here is integer arithmetic over a fixed generator, so the same manifest gives the same stream on
//! every platform.
//!
//! The three workloads share one graph planner and differ in what surrounds it:
//!
//! * W1, scene construction: the same graph is built, dropped and removed again and again.
//! * W2, connector rerouting: one scene is built, and then nodes are moved so that their connectors are routed again.
//! * W3, scene lifecycle: a different graph is built, dropped and removed on every cycle.
//! * W4, interaction: one scene is built and checked, and a browser is then driven through it with real input. The input is
//!   planned by [`crate::gestures`], and is not part of this stream.

use crate::manifest::{Manifest, Workload};
use crate::ops::{Op, Phase};
use crate::prng::SplitMix64;
use std::collections::HashSet;

/// The size of every planned node. Both are even, so a node's centre is a whole number whenever its corner is.
pub const NODE_W: i64 = 80;
pub const NODE_H: i64 = 36;
/// Where the grid of nodes starts, and the distance between neighbouring nodes.
pub const MARGIN: i64 = 20;
pub const PITCH_X: i64 = 140;
pub const PITCH_Y: i64 = 90;

const STREAM_GRAPH: u64 = 0;
const STREAM_MOVES: u64 = 1;
/// W3 draws its graph for cycle `c` from stream `STREAM_CYCLE_BASE + c`.
const STREAM_CYCLE_BASE: u64 = 0x1000;
/// One move in this many is aimed at the node with the highest degree, so that the busiest node is dragged often.
const HUB_ONE_IN: u64 = 4;

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WorkloadError {
    /// The graph could not be planned.
    Plan(String),
    /// The visitor refused an operation, so generation stopped.
    Visit(String),
}

impl std::fmt::Display for WorkloadError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Plan(message) => write!(f, "the graph could not be planned: {message}"),
            Self::Visit(message) => write!(f, "an operation was refused: {message}"),
        }
    }
}

impl std::error::Error for WorkloadError {}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlannedNode {
    pub key: String,
    pub x: i64,
    pub y: i64,
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GraphPlan {
    pub nodes: Vec<PlannedNode>,
    /// Directed edges as pairs of indexes into `nodes`, in the order they are added.
    pub edges: Vec<(usize, usize)>,
    /// The width and height of the area a node may be moved within.
    pub field_w: i64,
    pub field_h: i64,
    /// The node of highest degree, and the lowest index among equals.
    pub hub: usize,
}

impl GraphPlan {
    pub fn edge_key(&self, edge: (usize, usize)) -> String {
        format!("{}->{}", self.nodes[edge.0].key, self.nodes[edge.1].key)
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The columns and rows of the smallest square-ish grid that holds `count` nodes.
fn grid_for(count: usize) -> (usize, usize) {
    let mut columns = 1usize;
    while columns * columns < count {
        columns += 1;
    }
    (columns, count.div_ceil(columns))
}

/// The width and height of the area that `node_count` planned nodes sit in, and may be moved within.
pub fn field_for(node_count: u32) -> (i64, i64) {
    let (columns, rows) = grid_for(node_count as usize);
    (MARGIN + columns as i64 * PITCH_X, MARGIN + rows as i64 * PITCH_Y)
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Plans `node_count` nodes on a grid and `edge_count` distinct directed edges between them.
///
/// Edges are drawn at random. One that is a self-loop, a repeat, or would take either end over `max_degree` (in and out
/// together) is drawn again, up to a fixed number of times.
///
/// # Errors
///
/// Returns [`WorkloadError::Plan`] if the edges could not be placed within the attempts.
pub fn plan_graph(
    prng: &mut SplitMix64,
    node_count: u32,
    edge_count: u32,
    max_degree: Option<u32>,
) -> Result<GraphPlan, WorkloadError> {
    let count = node_count as usize;
    let (columns, rows) = grid_for(count);

    let nodes: Vec<PlannedNode> = (0..count)
        .map(|index| PlannedNode {
            key: format!("n{index}"),
            x: MARGIN + (index % columns) as i64 * PITCH_X,
            y: MARGIN + (index / columns) as i64 * PITCH_Y,
        })
        .collect();

    let limit = max_degree.unwrap_or(u32::MAX);
    let mut degree = vec![0u32; count];
    let mut seen: HashSet<(usize, usize)> = HashSet::new();
    let mut edges = Vec::with_capacity(edge_count as usize);
    let mut attempts = 0u64;
    let allowed = 100 * u64::from(edge_count) + 1000;
    while edges.len() < edge_count as usize {
        attempts += 1;
        if attempts > allowed {
            return Err(WorkloadError::Plan(format!(
                "could not place {edge_count} edges among {node_count} nodes within {allowed} attempts"
            )));
        }
        let from = prng.below(count as u64) as usize;
        let to = prng.below(count as u64) as usize;
        if from == to || degree[from] >= limit || degree[to] >= limit || !seen.insert((from, to)) {
            continue;
        }
        degree[from] += 1;
        degree[to] += 1;
        edges.push((from, to));
    }

    let hub = (0..count).fold(0, |best, index| if degree[index] > degree[best] { index } else { best });
    Ok(GraphPlan {
        nodes,
        edges,
        field_w: MARGIN + columns as i64 * PITCH_X,
        field_h: MARGIN + rows as i64 * PITCH_Y,
        hub,
    })
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
type Visit<'a> = &'a mut dyn FnMut(&Op) -> Result<(), String>;

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Hands out operations to the visitor, and numbers the checkpoints.
struct Emitter<'a> {
    visit: Visit<'a>,
    next_checkpoint: u64,
    last_was_checkpoint: bool,
}

impl Emitter<'_> {
    fn emit(&mut self, op: Op) -> Result<(), WorkloadError> {
        self.last_was_checkpoint = matches!(op, Op::Checkpoint { .. });
        (self.visit)(&op).map_err(WorkloadError::Visit)
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    fn checkpoint(&mut self) -> Result<(), WorkloadError> {
        let index = self.next_checkpoint;
        self.next_checkpoint += 1;
        self.emit(Op::Checkpoint { index })
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    fn build(&mut self, plan: &GraphPlan) -> Result<(), WorkloadError> {
        for node in &plan.nodes {
            self.emit(Op::AddNode {
                key: node.key.clone(),
                x: node.x,
                y: node.y,
                w: NODE_W,
                h: NODE_H,
                label: node.key.clone(),
            })?;
        }
        for &(from, to) in &plan.edges {
            self.emit(Op::AddEdge {
                from: plan.nodes[from].key.clone(),
                to: plan.nodes[to].key.clone(),
            })?;
        }
        Ok(())
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    /// The warm-up and measured phase markers for step `step` of a run with `warmup` warm-up steps.
    fn phase_marker(&mut self, step: u64, warmup: u64) -> Result<(), WorkloadError> {
        if step == 0 && warmup > 0 {
            self.emit(Op::Phase { phase: Phase::Warmup })?;
        }
        if step == warmup {
            self.emit(Op::Phase { phase: Phase::Measure })?;
        }
        Ok(())
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Generates the whole operation stream of `manifest`, handing each operation to `visit` in order.
///
/// A checkpoint comes at the manifest's `checkpoint_every` interval, if it has one, and always at the end.
///
/// # Errors
///
/// Returns [`WorkloadError::Plan`] if a graph cannot be planned, and [`WorkloadError::Visit`] with the visitor's message if
/// it refuses an operation.
pub fn generate(manifest: &Manifest, visit: Visit<'_>) -> Result<(), WorkloadError> {
    let mut emitter = Emitter {
        visit,
        next_checkpoint: 0,
        last_was_checkpoint: false,
    };
    match manifest.workload {
        Workload::SceneConstruction => cycles(manifest, &mut emitter, false),
        Workload::SceneLifecycle => cycles(manifest, &mut emitter, true),
        Workload::Interaction => interaction(manifest, &mut emitter),
        Workload::ConnectorRerouting => rerouting(manifest, &mut emitter),
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The graph that a scene of this manifest is built from, which is the one that the interaction workload plans gestures over.
///
/// # Errors
///
/// Returns [`WorkloadError::Plan`] if the graph cannot be planned.
pub fn graph_plan(manifest: &Manifest) -> Result<GraphPlan, WorkloadError> {
    plan_for(manifest, STREAM_GRAPH)
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
fn plan_for(manifest: &Manifest, stream: u64) -> Result<GraphPlan, WorkloadError> {
    plan_graph(
        &mut SplitMix64::derive(manifest.seed, stream),
        manifest.graph.nodes,
        manifest.graph.edges.unwrap_or(0),
        manifest.graph.maximum_degree,
    )
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// W1 and W3: build, then drop and remove, once for every warm-up and measured iteration.
///
/// A checkpoint follows the build of every `checkpoint_every`th cycle and of the last. One more follows the last removal,
/// to show that nothing is left.
fn cycles(manifest: &Manifest, emitter: &mut Emitter<'_>, new_graph_each_cycle: bool) -> Result<(), WorkloadError> {
    let warmup = u64::from(manifest.execution.warmup_iterations);
    let total = manifest.total_iterations();
    let every = manifest.measurement.checkpoint_every.map(u64::from);
    let fixed = if new_graph_each_cycle {
        None
    } else {
        Some(plan_for(manifest, STREAM_GRAPH)?)
    };

    for cycle in 0..total {
        emitter.phase_marker(cycle, warmup)?;
        // A graph that is new every cycle is planned before anything is emitted, as it always was, and is dropped as soon as it
        // has been built. Holding it to the end of the cycle would leave it allocated while the scene is torn down, and a runner
        // counting the bytes that stay allocated after a teardown would count the generator's own plan, as an early one did.
        {
            let planned;
            let plan = match &fixed {
                Some(plan) => plan,
                None => {
                    planned = plan_for(manifest, STREAM_CYCLE_BASE + cycle)?;
                    &planned
                },
            };
            emitter.emit(Op::CreateScene)?;
            emitter.build(plan)?;
        }
        if every.is_some_and(|every| (cycle + 1) % every == 0) || cycle + 1 == total {
            emitter.checkpoint()?;
        }
        emitter.emit(Op::DropScene)?;
        emitter.emit(Op::RemoveHost)?;
    }
    emitter.checkpoint()
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// W4: build the scene once and checkpoint it.
///
/// What the browser does to it afterwards is a plan of its own, so that a stream that is only the build is the same on every
/// layer, and a checkpoint proves that the scene it is about to be driven through is the one that was meant.
fn interaction(manifest: &Manifest, emitter: &mut Emitter<'_>) -> Result<(), WorkloadError> {
    let plan = plan_for(manifest, STREAM_GRAPH)?;
    emitter.emit(Op::CreateScene)?;
    emitter.build(&plan)?;
    emitter.checkpoint()
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// W2: build once and checkpoint, then make one move for every warm-up and measured iteration.
///
/// Each move draws its node and its place from the moves stream. One move in four is aimed at the node of highest degree.
/// The draws are made whether or not they are used, so the stream does not depend on which branch was taken.
fn rerouting(manifest: &Manifest, emitter: &mut Emitter<'_>) -> Result<(), WorkloadError> {
    let warmup = u64::from(manifest.execution.warmup_iterations);
    let total = manifest.total_iterations();
    let every = manifest.measurement.checkpoint_every.map(u64::from);
    let plan = plan_for(manifest, STREAM_GRAPH)?;
    let mut prng = SplitMix64::derive(manifest.seed, STREAM_MOVES);

    emitter.emit(Op::CreateScene)?;
    emitter.build(&plan)?;
    emitter.checkpoint()?;
    for step in 0..total {
        emitter.phase_marker(step, warmup)?;
        let aim_at_hub = prng.below(HUB_ONE_IN) == 0;
        let any = prng.below(plan.nodes.len() as u64) as usize;
        let x = prng.below((plan.field_w - NODE_W + 1) as u64) as i64;
        let y = prng.below((plan.field_h - NODE_H + 1) as u64) as i64;
        let node = if aim_at_hub { plan.hub } else { any };
        emitter.emit(Op::MoveNode {
            key: plan.nodes[node].key.clone(),
            x,
            y,
        })?;
        if every.is_some_and(|every| (step + 1) % every == 0) {
            emitter.checkpoint()?;
        }
    }
    if !emitter.last_was_checkpoint {
        emitter.checkpoint()?;
    }
    Ok(())
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[cfg(test)]
mod unit_tests;
