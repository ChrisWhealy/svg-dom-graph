//! Executes a workload against a real scene in a browser, and reports what it measured.
//!
//! The stream comes from `perf-harness`, which generates it from the manifest. Each operation is applied through the
//! library's public API. The reference model is driven alongside, and at a checkpoint what the document shows is compared with
//! what the model predicted.
//!
//! Timing is of synchronous API calls. `performance.now()` is read where one kind of operation gives way to another, and
//! never inside an operation, so a segment of the same kind is one observation (a build of nodes, say) or one batch of moves.
//! Checks, resource counts and the model all happen between segments, outside the timing. None of this is a frame time: the
//! browser has not painted when these calls return, and the browser layer measures that.

mod attribute;
mod checkpoint;
mod cycle;
mod endurance;
mod injection;
mod phase;
mod queue;
mod resources;
mod timing;

pub use attribute::attribute;
pub use injection::{Fault, Leak};

use crate::alloc;
use perf_harness::corruption::{self, Target};
use perf_harness::manifest::{Manifest, Mode, ViewPolicy, Workload};
use perf_harness::model::Model;
use perf_harness::ops::{Op, StreamHasher};
use perf_harness::outcome::{ExecutedStream, RunOutcome};
use perf_harness::workload::generate;
use queue::{Prepared, Waiting};
use resources::{Resources, SeriesEntry};
use std::collections::BTreeMap;
use svg_dom_graph::{NodeId, scene::Scene};

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The digests of the stream that the workload report expects, produced by a different process.
pub struct Expected {
    pub sha256: String,
    pub mutations_sha256: String,
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
struct Run<'a> {
    manifest: &'a Manifest,
    mode: Mode,
    view: ViewPolicy,
    window: web_sys::Window,
    document: web_sys::Document,
    performance: web_sys::Performance,
    resolution_us: f64,

    svg: Option<web_sys::Element>,
    scene: Option<Scene>,
    nodes: BTreeMap<String, NodeId>,
    edge_keys: Vec<String>,

    model: Model,
    hasher: StreamHasher,
    total_checkpoints: u64,

    measuring: bool,
    building: bool,
    cycle: u64,
    total_cycles: u64,

    timing_on: bool,
    /// The operations that wait to be run together, all of one kind, and what they are.
    pending: Vec<Prepared>,
    pending_meta: Option<Waiting>,
    /// The most that can wait. It is the size of the vector, which is made at full size so that it never grows during a run.
    pending_capacity: usize,
    /// The ids that a run of `add_node` calls gave back, kept until the calls are over.
    scratch_ids: Vec<NodeId>,
    /// A deliberate delay in the harness's own work, to prove that it is not in the timing. Zero outside the self-test.
    harness_delay_us: f64,
    /// How many of the first cycles to list the live blocks of after their teardown. Zero outside the attribution run.
    leak: Option<Leak>,
    track_cycles: u64,
    tracking_log: Vec<(u64, usize, usize, Vec<alloc::TrackedBlock>)>,
    move_batch: Option<u32>,
    warmup_move_us: f64,
    warmup_moves: u64,
    cycle_construction_us: f64,
    cycle_teardown_us: f64,
    samples: BTreeMap<&'static str, Vec<f64>>,

    baseline: Resources,
    after_build: Option<Resources>,
    after_teardown: Option<Resources>,
    teardown_series: Vec<SeriesEntry>,
    /// Whether the cycle in progress ran a checkpoint, whose own memory use must not be read as the library's.
    checkpoint_this_cycle: bool,
    outcome: RunOutcome,
    fault: Option<Fault>,
    /// The position among the stream's operations of the one to apply wrongly, and whether that has been done.
    corrupt_at: Option<u64>,
    ordinal: u64,
    corrupted: bool,
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
impl<'a> Run<'a> {
    fn new(manifest: &'a Manifest, mode: Mode) -> Result<Self, String> {
        let window = web_sys::window().ok_or("there is no window")?;
        let document = window.document().ok_or("there is no document")?;
        let performance = window.performance().ok_or("there is no performance object")?;
        let mut total_checkpoints = 0;
        generate(manifest, &mut |op: &Op| {
            if matches!(op, Op::Checkpoint { .. }) {
                total_checkpoints += 1;
            }
            Ok(())
        })
        .map_err(describe)?;
        // Rerouting and interaction build one scene and keep it, so they have one cycle whatever their iteration counts say.
        let total_cycles = if matches!(manifest.workload, Workload::ConnectorRerouting | Workload::Interaction) {
            1
        } else {
            manifest.total_iterations()
        };
        // Large enough for the biggest run of one kind in a build, and for a batch of moves, and never more than a
        // fixed ceiling.
        let pending_capacity =
            (manifest.graph.nodes.max(manifest.graph.edges.unwrap_or(0)) as usize + 1).clamp(1024, 8192);
        let mut run = Self {
            manifest,
            mode,
            view: manifest.checkpoint.as_ref().map_or(ViewPolicy::Unchanged, |policy| policy.view),
            window,
            document,
            performance,
            resolution_us: 0.0,
            svg: None,
            scene: None,
            nodes: BTreeMap::new(),
            edge_keys: Vec::with_capacity(manifest.graph.edges.unwrap_or(0) as usize),
            model: Model::new(),
            hasher: StreamHasher::new(),
            total_checkpoints,
            measuring: false,
            building: false,
            cycle: 0,
            total_cycles,
            timing_on: matches!(mode, Mode::Performance | Mode::Endurance),
            pending: Vec::with_capacity(pending_capacity),
            pending_meta: None,
            pending_capacity,
            scratch_ids: Vec::with_capacity(pending_capacity),
            harness_delay_us: 0.0,
            leak: None,
            track_cycles: 0,
            tracking_log: Vec::new(),
            move_batch: None,
            warmup_move_us: 0.0,
            warmup_moves: 0,
            cycle_construction_us: 0.0,
            cycle_teardown_us: 0.0,
            samples: Self::sample_vectors(manifest),
            baseline: Resources::default(),
            after_build: None,
            after_teardown: None,
            // Everything the run records is made at its full size before the first reading. A vector that grew while
            // the scene was being built and dropped would show in the live bytes, and would look like the library
            // keeping memory.
            teardown_series: Vec::with_capacity(total_cycles as usize),
            checkpoint_this_cycle: false,
            outcome: RunOutcome::new("B", mode),
            fault: None,
            corrupt_at: None,
            ordinal: 0,
            corrupted: false,
        };
        let (resolution_us, clock_read_us) = run.measure_resolution();
        run.resolution_us = resolution_us;
        run.outcome.timer_resolution_us = resolution_us;
        run.outcome.notes.push(format!(
            "a clock read costs about {clock_read_us:.2} us, and one is inside every timed interval"
        ));
        run.baseline = run.sample_resources();
        Ok(run)
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    /// One vector for every metric that the run can record, each as large as it can ever need to be.
    fn sample_vectors(manifest: &Manifest) -> BTreeMap<&'static str, Vec<f64>> {
        let most = manifest.execution.measurement_iterations as usize;
        [
            "construction.create_scene",
            "construction.nodes",
            "construction.edges",
            "construction.total",
            "teardown.drop_scene",
            "teardown.remove_host",
            "teardown.total",
            "mutation.move_node",
        ]
        .into_iter()
        .map(|name| (name, Vec::with_capacity(most)))
        .collect()
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    fn node(&self, key: &str) -> Result<NodeId, String> {
        self.nodes.get(key).copied().ok_or_else(|| format!("the node {key} is unknown"))
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    fn finish(mut self, expected: Option<&Expected>) -> RunOutcome {
        let operations = self.hasher.operations();
        let (full, mutations) = std::mem::take(&mut self.hasher).finish();
        let (scope, sha256, expected_sha256) = match (self.mode, expected) {
            (Mode::Correctness, Some(e)) => ("full", full, e.sha256.clone()),
            (_, Some(e)) => ("mutations", mutations, e.mutations_sha256.clone()),
            (Mode::Correctness, None) => ("full", full.clone(), full),
            (_, None) => ("mutations", mutations.clone(), mutations),
        };
        self.outcome.stream = Some(ExecutedStream {
            scope,
            operations,
            matches: sha256 == expected_sha256,
            sha256,
            expected_sha256,
        });

        self.emit_timing_metrics();
        self.emit_resource_metrics();
        // A workload such as W2 leaves its scene in the page, and the next run on that page must not find it there.
        self.clean_up();
        self.outcome.settle();
        self.outcome
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    /// Drops the scene and removes its host, if the workload left them behind. It is done after every figure is taken.
    fn clean_up(&mut self) {
        self.remove_leftover();
        self.scene = None;
        self.nodes.clear();
        if let Some(svg) = self.svg.take() {
            svg.remove();
        }
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
fn describe(error: impl std::fmt::Display) -> String {
    error.to_string()
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Runs `manifest` in `mode`, and always gives an outcome: a failure is a status and a message, never a trap.
pub fn run(manifest: &Manifest, mode: Mode, expected: Option<&Expected>) -> RunOutcome {
    run_with_options(manifest, mode, expected, Options::default())
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The same, with a deliberate fault in what is observed. See [`Fault`].
pub fn run_with_fault(
    manifest: &Manifest,
    mode: Mode,
    expected: Option<&Expected>,
    fault: Option<Fault>,
) -> RunOutcome {
    run_with_options(manifest, mode, expected, Options { fault, ..Options::default() })
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// What a self-test does to a run to prove that something notices. A run that is not a self-test has none of it.
#[derive(Debug, Clone, Copy, Default)]
pub struct Options {
    /// A fault in what is observed.
    pub fault: Option<Fault>,
    /// A delay in the harness's own work, in microseconds, for every operation.
    ///
    /// The delay is in the reference model's turn, which is not part of any timed interval, so no timing may show it. The
    /// self-test runs a workload with and without it, and fails if a figure moves.
    pub harness_delay_us: f64,
    /// A leak in every cycle.
    pub leak: Option<Leak>,
    /// The last mutation before one of the two executed checkpoints is applied wrongly. See [`perf_harness::corruption`].
    pub corrupt: Option<Target>,
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The same, with the self-test's `options`.
pub fn run_with_options(manifest: &Manifest, mode: Mode, expected: Option<&Expected>, options: Options) -> RunOutcome {
    let error = |message: String| {
        let mut outcome = RunOutcome::new("B", mode);
        outcome.status = "error";
        outcome.error = Some(message);
        outcome
    };
    if !matches!(mode, Mode::Correctness | Mode::Performance | Mode::Endurance) {
        return error(format!("the {mode:?} mode is not implemented in this runner yet"));
    }
    let corrupt_at = match options.corrupt.map(|target| corruption::locate(manifest, target)).transpose() {
        Ok(found) => found.map(|found| found.ordinal),
        Err(message) => return error(describe(message)),
    };
    let mut run = match Run::new(manifest, mode) {
        Ok(run) => run,
        Err(message) => return error(message),
    };
    run.fault = options.fault;
    run.harness_delay_us = options.harness_delay_us;
    run.leak = options.leak;
    run.corrupt_at = corrupt_at;
    let mut result = run.execute();
    // A corruption that was never applied would leave a run that passes, which proves nothing.
    if result.is_ok() && corrupt_at.is_some() && !run.corrupted {
        result = Err("the corruption was never applied".to_string());
    }
    if let Err(message) = result {
        run.outcome.status = "error";
        run.outcome.error = Some(message);
    }
    run.finish(expected)
}
