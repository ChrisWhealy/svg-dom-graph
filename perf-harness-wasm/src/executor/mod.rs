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

use crate::alloc;
use crate::memory::capacity_bytes;
use crate::observe::observe;
use perf_harness::compare::compare;
use perf_harness::manifest::{Manifest, Mode, ViewPolicy, Workload};
use perf_harness::metrics::{self, Layer};
use perf_harness::model::{Checkpoint, Model};
use perf_harness::ops::{Op, Phase, StreamHasher};
use perf_harness::outcome::{ExecutedStream, FailedCheckpoint, RunOutcome, performance_runs_checkpoint};
use perf_harness::stats::batch_size;
use perf_harness::workload::{field_for, generate};
use std::collections::BTreeMap;
use svg_dom::{
    SvgRoot,
    root::utils::{Point, Size},
};
use svg_dom_graph::{
    NodeId,
    scene::{ConnectorOptions, ConnectorType, Scene},
};
use wasm_bindgen::JsCast;

const SVG_NS: &str = "http://www.w3.org/2000/svg";
/// Numbers the host elements of every run in a page, so that a later run never makes an id that an earlier one used.
static SVG_SEQUENCE: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(0);
/// A checkpoint that goes wrong reports this many differences, since the rest repeat the same fault.
const MAX_REPORTED_PER_CHECKPOINT: usize = 20;
/// How many times `performance.now()` is called to find the smallest step it makes.
const RESOLUTION_PROBES: u32 = 200_000;

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// A deliberate fault in what was observed, to prove that the comparison notices it.
///
/// It is applied to the observed checkpoint and never to the scene, so it tests the check and not the library. A run with a
/// fault must come out `invalid_incorrect` and name the section that the fault damaged.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Fault {
    /// The first connector's start is a unit out.
    ConnectorOffset,
    /// The first node is three units lower than it is.
    NodeMoved,
    /// The first node, and everything about it, is gone.
    NodeRemoved,
    /// The first node's text is not what it was given.
    TextChanged,
    /// The first two nodes are painted in the other order.
    PaintOrderSwapped,
    /// One edge is missing from the relationships.
    EdgeDropped,
}

impl Fault {
    pub fn parse(name: &str) -> Option<Self> {
        Some(match name {
            "connector_offset" => Self::ConnectorOffset,
            "node_moved" => Self::NodeMoved,
            "node_removed" => Self::NodeRemoved,
            "text_changed" => Self::TextChanged,
            "paint_order_swapped" => Self::PaintOrderSwapped,
            "edge_dropped" => Self::EdgeDropped,
            _ => return None,
        })
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    fn apply(self, actual: &mut Checkpoint) {
        match self {
            Self::ConnectorOffset => {
                if let Some(connector) = actual.geometry.elements.iter_mut().find(|e| e.kind == "connector_path") {
                    connector.coordinates[0] += 1.0;
                }
            },
            Self::NodeMoved => {
                if let Some(node) = actual.geometry.elements.iter_mut().find(|e| e.kind == "node_box") {
                    node.coordinates[1] += 3.0;
                }
            },
            Self::NodeRemoved => {
                if let Some(key) = actual.topology.nodes.first().cloned() {
                    actual.topology.nodes.retain(|k| *k != key);
                    actual.content.remove(&key);
                    actual.paint_order.retain(|k| *k != key);
                    actual.geometry.elements.retain(|e| !(e.key == key && e.kind == "node_box"));
                }
            },
            Self::TextChanged => {
                if let Some(text) = actual.content.values_mut().next() {
                    text.push_str(" (changed)");
                }
            },
            Self::PaintOrderSwapped => {
                if actual.paint_order.len() >= 2 {
                    actual.paint_order.swap(0, 1);
                }
            },
            Self::EdgeDropped => {
                actual.topology.edges.pop();
            },
        }
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The digests of the stream that the workload report expects, produced by a different process.
pub struct Expected {
    pub sha256: String,
    pub mutations_sha256: String,
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Kind {
    Create,
    Nodes,
    Edges,
    Moves,
    Drop,
    Remove,
}

impl Kind {
    fn of(op: &Op) -> Option<Self> {
        match op {
            Op::CreateScene => Some(Self::Create),
            Op::AddNode { .. } => Some(Self::Nodes),
            Op::AddEdge { .. } => Some(Self::Edges),
            Op::MoveNode { .. } => Some(Self::Moves),
            Op::DropScene => Some(Self::Drop),
            Op::RemoveHost => Some(Self::Remove),
            Op::Phase { .. } | Op::Checkpoint { .. } => None,
        }
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// An operation with everything it needs already worked out, so that running it is only the library call.
///
/// Finding a node's id, building a key, copying a label and making the host element are the harness's own work. They are done
/// when the operation is prepared, before any clock is read, so that a timed interval holds nothing but calls into the library.
enum Prepared {
    Create {
        id: String,
    },
    AddNode {
        key: String,
        point: Point,
        size: Size,
        label: String,
    },
    AddEdge {
        key: String,
        from: NodeId,
        to: NodeId,
    },
    Move {
        id: NodeId,
        point: Point,
    },
    Drop,
    Remove,
}

/// What a run of waiting operations is: their kind, whether they are in the measured part, and whether they are timed.
type Waiting = (Kind, bool, bool);

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// What can be counted at one moment.
#[derive(Debug, Clone, Copy, Default)]
struct Resources {
    dom_elements: Option<f64>,
    listeners: Option<f64>,
    live_bytes: Option<f64>,
    live_allocations: Option<f64>,
    capacity: Option<f64>,
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
    move_batch: Option<u32>,
    warmup_move_us: f64,
    warmup_moves: u64,
    cycle_construction_us: f64,
    cycle_teardown_us: f64,
    samples: BTreeMap<&'static str, Vec<f64>>,

    baseline: Resources,
    after_build: Option<Resources>,
    after_teardown: Option<Resources>,
    teardown_series: Vec<Resources>,
    /// Whether the cycle in progress ran a checkpoint, whose own memory use must not be read as the library's.
    checkpoint_this_cycle: bool,
    outcome: RunOutcome,
    fault: Option<Fault>,
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
fn describe(error: impl std::fmt::Display) -> String {
    error.to_string()
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
        let total_cycles = if manifest.workload == Workload::ConnectorRerouting {
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
    fn now_ms(&self) -> f64 {
        self.performance.now()
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    /// The smallest step that `performance.now()` makes, and what one call costs, both in microseconds.
    fn measure_resolution(&self) -> (f64, f64) {
        let mut smallest = f64::INFINITY;
        let began = self.now_ms();
        let mut last = began;
        for _ in 0..RESOLUTION_PROBES {
            let now = self.now_ms();
            let step = (now - last) * 1000.0;
            if step > 0.0 && step < smallest {
                smallest = step;
            }
            last = now;
        }
        let per_call = (last - began) * 1000.0 / f64::from(RESOLUTION_PROBES);
        (if smallest.is_finite() { smallest } else { 0.0 }, per_call)
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    fn sample_resources(&self) -> Resources {
        let manifest = &self.manifest.measurement;
        // The allocator is read first, so that nothing done to count the rest is in its figure.
        let allocator = manifest.capture_wasm_live_allocation.then(alloc::stats);
        Resources {
            live_bytes: allocator.map(|s| s.live_bytes as f64),
            live_allocations: allocator.map(|s| s.live_allocations as f64),
            dom_elements: manifest
                .capture_dom_counts
                .then(|| self.document.query_selector_all("*").ok().map(|list| f64::from(list.length())))
                .flatten(),
            listeners: manifest.capture_listener_tally.then(|| self.listener_total()).flatten(),
            capacity: manifest.capture_wasm_capacity.then(capacity_bytes).flatten(),
        }
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    /// The effective listener registrations that the page's tracker counts, if the page has one.
    fn listener_total(&self) -> Option<f64> {
        let tracker = js_sys::Reflect::get(&self.window, &"__listeners".into())
            .ok()
            .filter(|value| !value.is_undefined())?;
        let snapshot = js_sys::Reflect::get(&tracker, &"snapshot".into())
            .ok()?
            .dyn_into::<js_sys::Function>()
            .ok()?;
        let result = snapshot.call0(&tracker).ok()?;
        js_sys::Reflect::get(&result, &"total".into()).ok()?.as_f64()
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    fn execute(&mut self) -> Result<(), String> {
        let manifest = self.manifest;
        let mut failure: Option<String> = None;
        let result = generate(manifest, &mut |op: &Op| match self.step(op) {
            Ok(()) => Ok(()),
            Err(message) => {
                failure = Some(message.clone());
                Err(message)
            },
        });
        // Whatever still waits to be run is run now, unless the run has already failed.
        let flushed = if failure.is_none() { self.flush() } else { Ok(()) };
        match (result, failure, flushed) {
            (_, Some(message), _) => Err(message),
            (Err(error), None, _) => Err(describe(error)),
            (Ok(()), None, flushed) => flushed,
        }
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    /// Takes in one operation of the stream.
    ///
    /// Everything here is the harness's own work: the reference model, the hash of the stream, the delay of the
    /// self-test, and the preparation of the operation. None of it is done while a clock is running. An operation is
    /// only queued here, and the library calls of a queue are made together by `flush`, which is the one place that
    /// reads a clock.
    fn step(&mut self, op: &Op) -> Result<(), String> {
        self.model.apply(op)?;
        self.hold_up();

        // The build is over once an operation that is not part of it arrives, and what waits has to run first.
        let is_build = matches!(op, Op::CreateScene | Op::AddNode { .. } | Op::AddEdge { .. });
        if self.building && !is_build {
            self.flush()?;
            self.building = false;
            self.on_build_complete();
        }

        match op {
            Op::Phase { phase } => {
                self.flush()?;
                self.hasher.push(op);
                if *phase == Phase::Measure {
                    self.finish_warmup();
                }
                self.measuring = *phase == Phase::Measure;
            },
            Op::Checkpoint { index } => {
                self.flush()?;
                if self.mode == Mode::Correctness || performance_runs_checkpoint(*index, self.total_checkpoints) {
                    self.hasher.push(op);
                    self.check(*index)?;
                }
            },
            _ => {
                self.hasher.push(op);
                let kind = Kind::of(op).ok_or("a marker reached the queue")?;
                // The measured part is timed. In the warm-up only the moves are, to find how many to time together.
                let recording = self.measuring;
                let timed = self.timing_on && (recording || kind == Kind::Moves);
                let waiting = (kind, recording, timed);
                // An operation of another kind runs what waits first, since it may need what that made, such as the nodes that
                // an edge joins. This comes before it is prepared, for that reason.
                if self.pending_meta != Some(waiting) {
                    self.flush()?;
                }
                let prepared = self.prepare(op)?;
                self.submit(prepared, waiting)?;
            },
        }
        Ok(())
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    /// A deliberate delay in the harness's own work, to prove that it never reaches a timed figure.
    fn hold_up(&self) {
        if self.harness_delay_us > 0.0 {
            let until = self.now_ms() + self.harness_delay_us / 1000.0;
            while self.now_ms() < until {}
        }
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    /// Works out everything an operation needs, so that running it is only the library call.
    fn prepare(&mut self, op: &Op) -> Result<Prepared, String> {
        Ok(match op {
            Op::CreateScene => {
                self.cycle += 1;
                self.checkpoint_this_cycle = false;
                self.building = true;
                let id = format!(
                    "perf-svg-{}",
                    SVG_SEQUENCE.fetch_add(1, std::sync::atomic::Ordering::Relaxed) + 1
                );
                // The host element is the page's, not the library's, so it is made here and the scene is made on it later.
                let (width, height) = field_for(self.manifest.graph.nodes);
                let svg = self
                    .document
                    .create_element_ns(Some(SVG_NS), "svg")
                    .map_err(|e| format!("{e:?}"))?;
                for (name, value) in [
                    ("id", id.clone()),
                    ("width", width.to_string()),
                    ("height", height.to_string()),
                    ("viewBox", format!("0 0 {width} {height}")),
                ] {
                    svg.set_attribute(name, &value).map_err(|e| format!("{e:?}"))?;
                }
                self.document
                    .body()
                    .ok_or("there is no body")?
                    .append_child(&svg)
                    .map_err(|e| format!("{e:?}"))?;
                self.svg = Some(svg);
                Prepared::Create { id }
            },
            Op::AddNode { key, x, y, w, h, label } => Prepared::AddNode {
                key: key.clone(),
                point: Point::new(*x as f64, *y as f64),
                size: Size::new(*w as f64, *h as f64),
                label: label.clone(),
            },
            Op::AddEdge { from, to } => Prepared::AddEdge {
                key: format!("{from}->{to}"),
                from: self.node(from)?,
                to: self.node(to)?,
            },
            Op::MoveNode { key, x, y } => Prepared::Move {
                id: self.node(key)?,
                point: Point::new(*x as f64, *y as f64),
            },
            Op::DropScene => Prepared::Drop,
            Op::RemoveHost => Prepared::Remove,
            Op::Phase { .. } | Op::Checkpoint { .. } => return Err("a marker reached prepare".to_string()),
        })
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    /// Queues a prepared operation, and runs the queue if it is full.
    ///
    /// A queue that is not timed holds one operation and runs at once. A timed batch of moves is as long as the warm-up
    /// said it should be. Every other timed queue is a whole run of its kind, up to the capacity of the vector.
    fn submit(&mut self, prepared: Prepared, waiting: Waiting) -> Result<(), String> {
        self.pending_meta = Some(waiting);
        self.pending.push(prepared);
        let (kind, recording, timed) = waiting;
        let wanted = if !timed {
            1
        } else if kind == Kind::Moves && recording {
            self.move_batch.unwrap_or(1) as usize
        } else {
            self.pending_capacity
        };
        if self.pending.len() >= wanted.min(self.pending_capacity) {
            self.flush()
        } else {
            Ok(())
        }
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
   /// Runs the queue.
    fn flush(&mut self) -> Result<(), String> {
        let Some((kind, recording, timed)) = self.pending_meta.take() else {
            return Ok(());
        };
        let count = self.pending.len();
        if count == 0 {
            return Ok(());
        }
        // The vector is taken out and put back, so that it keeps its capacity and no run allocates one.
        let mut items = std::mem::take(&mut self.pending);
        let elapsed = self.run_batch(kind, &mut items, timed);
        items.clear();
        self.pending = items;
        let elapsed_us = elapsed?;
        if timed {
            self.record(kind, recording, count as u32, elapsed_us);
        }
        if kind == Kind::Remove {
            self.on_teardown_complete();
        }
        Ok(())
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    /// Makes the library calls of one queue, one after the other, and gives the time they took.
    ///
    /// The clock is read once before the first call and once after the last. Nothing else is between them: not the
    /// model, not the hash, not the bookkeeping of what the calls returned, which is done after the second reading.
    fn run_batch(&mut self, kind: Kind, items: &mut [Prepared], timed: bool) -> Result<f64, String> {
        let started = if timed { self.now_ms() } else { 0.0 };
        match kind {
            Kind::Create => {
                let Some(Prepared::Create { id }) = items.first() else {
                    return Err("a create had no id".to_string());
                };
                let scene = Scene::new(SvgRoot::attach(id).map_err(describe)?).map_err(describe)?;
                self.scene = Some(scene);
            },
            Kind::Nodes => {
                let scene = self.scene.as_ref().ok_or("there is no scene")?;
                self.scratch_ids.clear();
                for item in items.iter_mut() {
                    if let Prepared::AddNode { point, size, label, .. } = item {
                        // The label is moved in and not copied, so that no copy is made while the clock runs.
                        self.scratch_ids
                            .push(scene.add_node(*point, *size, std::mem::take(label)).map_err(describe)?);
                    }
                }
            },
            Kind::Edges => {
                let scene = self.scene.as_ref().ok_or("there is no scene")?;
                // The library's default connector is an elbow, and the reference model predicts a straight one.
                let options = ConnectorOptions::default().with_connector_type(ConnectorType::Straight);
                for item in items.iter() {
                    if let Prepared::AddEdge { from, to, .. } = item {
                        scene.add_edge_with(*from, *to, options).map_err(describe)?;
                    }
                }
            },
            Kind::Moves => {
                let scene = self.scene.as_ref().ok_or("there is no scene")?;
                for item in items.iter() {
                    if let Prepared::Move { id, point } = item {
                        scene.move_node(*id, *point).map_err(describe)?;
                    }
                }
            },
            Kind::Drop => drop(self.scene.take()),
            Kind::Remove => {
                if let Some(svg) = self.svg.take() {
                    svg.remove();
                }
            },
        }
        let elapsed_us = if timed { (self.now_ms() - started) * 1000.0 } else { 0.0 };

        // What the calls returned is kept now, with the clock stopped.
        match kind {
            Kind::Nodes => {
                for (item, id) in items.iter_mut().zip(self.scratch_ids.iter()) {
                    if let Prepared::AddNode { key, .. } = item {
                        self.nodes.insert(std::mem::take(key), *id);
                    }
                }
            },
            Kind::Edges => {
                for item in items.iter_mut() {
                    if let Prepared::AddEdge { key, .. } = item {
                        self.edge_keys.push(std::mem::take(key));
                    }
                }
            },
            Kind::Drop => self.nodes.clear(),
            Kind::Remove => self.edge_keys.clear(),
            Kind::Create | Kind::Moves => {},
        }
        Ok(elapsed_us)
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    /// Keeps the time of a queue that was run.
    fn record(&mut self, kind: Kind, recording: bool, count: u32, microseconds: f64) {
        match (kind, recording) {
            (Kind::Moves, false) => {
                self.warmup_move_us += microseconds;
                self.warmup_moves += u64::from(count);
            },
            (Kind::Moves, true) => self
                .samples
                .entry("mutation.move_node")
                .or_default()
                .push(microseconds / f64::from(count)),
            (kind, _) => {
                let name = match kind {
                    Kind::Create => "construction.create_scene",
                    Kind::Nodes => "construction.nodes",
                    Kind::Edges => "construction.edges",
                    Kind::Drop => "teardown.drop_scene",
                    Kind::Remove => "teardown.remove_host",
                    Kind::Moves => unreachable!("moves are handled above"),
                };
                self.samples.entry(name).or_default().push(microseconds);
                if matches!(kind, Kind::Create | Kind::Nodes | Kind::Edges) {
                    self.cycle_construction_us += microseconds;
                } else {
                    self.cycle_teardown_us += microseconds;
                }
            },
        }
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    /// Works out from the warm-up how many moves to time together, so that an observation spans enough timer steps.
    fn finish_warmup(&mut self) {
        if self.timing_on && self.manifest.workload == Workload::ConnectorRerouting {
            if self.warmup_moves > 0 {
                let average = self.warmup_move_us / self.warmup_moves as f64;
                let wanted = batch_size(self.resolution_us, average);
                let batch = wanted.min(self.pending_capacity as u32);
                if batch < wanted {
                    self.outcome.notes.push(format!(
                        "a batch of {wanted} moves was wanted, and {batch} is the most that can wait, so each observation is shorter than 50 timer steps"
                    ));
                }
                self.move_batch = Some(batch);
            } else {
                self.outcome
                    .notes
                    .push("there was no warm-up, so every move was timed on its own".to_string());
            }
        }
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    fn on_build_complete(&mut self) {
        if self.measuring && self.timing_on && self.cycle_construction_us > 0.0 {
            self.samples
                .entry("construction.total")
                .or_default()
                .push(self.cycle_construction_us);
        }
        self.cycle_construction_us = 0.0;
        if self.mode == Mode::Endurance || self.cycle >= self.total_cycles {
            self.after_build = Some(self.sample_resources());
        }
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    fn on_teardown_complete(&mut self) {
        if self.measuring && self.timing_on && self.cycle_teardown_us > 0.0 {
            self.samples.entry("teardown.total").or_default().push(self.cycle_teardown_us);
        }
        self.cycle_teardown_us = 0.0;
        if self.mode == Mode::Endurance || self.cycle >= self.total_cycles {
            let sampled = self.sample_resources();
            self.after_teardown = Some(sampled);
            // A cycle that ran a checkpoint is left out of the series, since the checking itself uses memory.
            if self.mode == Mode::Endurance && !self.checkpoint_this_cycle {
                self.teardown_series.push(sampled);
            }
        }
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    fn node(&self, key: &str) -> Result<NodeId, String> {
        self.nodes.get(key).copied().ok_or_else(|| format!("the node {key} is unknown"))
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    /// Compares what is in the document with what the model predicted.
    fn check(&mut self, index: u64) -> Result<(), String> {
        self.checkpoint_this_cycle = true;
        let expected = self.model.checkpoint(self.view)?;
        let mut actual = match (&self.svg, &self.scene) {
            (Some(svg), Some(scene)) => observe(svg, scene, &self.edge_keys, self.view)?,
            _ => self.observe_nothing()?,
        };
        if let Some(fault) = self.fault {
            fault.apply(&mut actual);
        }
        let differences = compare(&expected, &actual);
        if differences.is_empty() {
            self.outcome.checkpoints_passed += 1;
        } else {
            let total = differences.len();
            for difference in differences.iter().take(MAX_REPORTED_PER_CHECKPOINT) {
                self.outcome.checkpoints_failed.push(FailedCheckpoint {
                    section: difference.section.to_string(),
                    detail: format!("checkpoint {index}: {}", difference.summary()),
                });
            }
            if total > MAX_REPORTED_PER_CHECKPOINT {
                self.outcome.notes.push(format!("checkpoint {index} differed in {total} places, and the first {MAX_REPORTED_PER_CHECKPOINT} are reported"));
            }
        }
        Ok(())
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    /// The checkpoint of a page with no scene on it, which is an empty one unless a host was left behind.
    fn observe_nothing(&self) -> Result<Checkpoint, String> {
        let leftover = self
            .document
            .query_selector("svg[id^='perf-svg-']")
            .map_err(|e| format!("{e:?}"))?;
        let mut empty = Model::new().checkpoint(self.view)?;
        if let Some(element) = leftover {
            empty
                .topology
                .nodes
                .push(format!("(a host svg, {}, is still in the document)", element.id()));
        }
        Ok(empty)
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
        self.scene = None;
        self.nodes.clear();
        if let Some(svg) = self.svg.take() {
            svg.remove();
        }
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    fn emit_timing_metrics(&mut self) {
        if !self.timing_on {
            self.outcome
                .metrics
                .push(metrics::unavailable("timing", Layer::B, "the correctness mode keeps no timing"));
            return;
        }
        let names = [
            "construction.create_scene",
            "construction.nodes",
            "construction.edges",
            "construction.total",
            "teardown.drop_scene",
            "teardown.remove_host",
            "teardown.total",
            "mutation.move_node",
        ];
        for name in names {
            let Some(samples) = self.samples.get(name).filter(|samples| !samples.is_empty()) else {
                continue;
            };
            let batch = if name == "mutation.move_node" { self.move_batch } else { None };
            let mut metric = metrics::timing(
                name,
                Layer::B,
                "us",
                samples,
                batch,
                "performance.now over synchronous Scene calls",
            );
            metrics::mark_resolution_limited(&mut metric, self.resolution_us);
            self.outcome.metrics.push(metric);
        }
        if self.samples.values().all(Vec::is_empty) {
            self.outcome.metrics.push(metrics::unavailable(
                "timing",
                Layer::B,
                "no operation was in the measured part of the run",
            ));
        }
        self.outcome.samples = self
            .samples
            .iter()
            .filter(|(_, values)| !values.is_empty())
            .map(|(name, values)| ((*name).to_string(), values.clone()))
            .collect();
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    fn emit_resource_metrics(&mut self) {
        let m = &self.manifest.measurement;
        let (baseline, built, torn) = (self.baseline, self.after_build, self.after_teardown);
        type Pick = fn(&Resources) -> Option<f64>;
        let figures: [(&str, bool, &str, &str, Pick); 4] = [
            (
                "dom_elements",
                m.capture_dom_counts,
                "count",
                "document.querySelectorAll('*').length",
                |r| r.dom_elements,
            ),
            (
                "listeners",
                m.capture_listener_tally,
                "count",
                "the effective-registration tracker",
                |r| r.listeners,
            ),
            (
                "live_bytes",
                m.capture_wasm_live_allocation,
                "bytes",
                "the counting allocator",
                |r| r.live_bytes,
            ),
            (
                "live_allocations",
                m.capture_wasm_live_allocation,
                "count",
                "the counting allocator",
                |r| r.live_allocations,
            ),
        ];
        for (name, wanted, unit, mechanism, pick) in figures {
            if !wanted {
                continue;
            }
            for (moment, resources) in [("baseline", Some(baseline)), ("after_build", built), ("after_teardown", torn)]
            {
                let metric_name = format!("resource.{name}.{moment}");
                match resources.and_then(|r| pick(&r)) {
                    Some(value) => {
                        self.outcome
                            .metrics
                            .push(metrics::count(&metric_name, Layer::B, unit, value, mechanism))
                    },
                    None if moment == "baseline" || resources.is_some() => self.outcome.metrics.push(
                        metrics::unavailable(&metric_name, Layer::B, &format!("{mechanism} gave no reading")),
                    ),
                    None => {},
                }
            }
            // The deterministic gate: once the scene is dropped and its host removed, the count is what it was before the build.
            if let (Some(before), Some(after)) = (pick(&baseline), torn.and_then(|r| pick(&r))) {
                self.outcome.metrics.push(metrics::count(
                    &format!("resource.{name}.delta_after_teardown"),
                    Layer::B,
                    unit,
                    after - before,
                    mechanism,
                ));
            }
        }
        if m.capture_wasm_capacity {
            match capacity_bytes() {
                Some(bytes) => self.outcome.metrics.push(metrics::capacity(
                    "wasm.linear_memory_high_water",
                    Layer::B,
                    bytes,
                    "WebAssembly.Memory.buffer.byteLength",
                )),
                None => self.outcome.metrics.push(metrics::unavailable(
                    "wasm.linear_memory_high_water",
                    Layer::B,
                    "no wasm memory could be read",
                )),
            }
        }
        if m.capture_wasm_live_allocation {
            self.outcome.metrics.push(metrics::count(
                "wasm.allocator_peak_live_bytes",
                Layer::B,
                "bytes",
                alloc::stats().peak_live_bytes as f64,
                "the counting allocator",
            ));
        }
        if self.mode == Mode::Endurance {
            self.emit_endurance_metrics();
        }
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    /// Whether capacity is reused, and whether live bytes keep growing, once the warm-up cycles are over.
    fn emit_endurance_metrics(&mut self) {
        let series = &self.teardown_series;
        let warmup = self.manifest.execution.warmup_iterations as usize;
        if series.len() < 2 {
            self.outcome.metrics.push(metrics::unavailable(
                "wasm.memory_grow_events_after_warmup",
                Layer::B,
                "the endurance series has fewer than two cycles",
            ));
            return;
        }
        let start = warmup.max(1).min(series.len() - 1);
        let capacity: Vec<f64> = series.iter().filter_map(|r| r.capacity).collect();
        if capacity.len() == series.len() {
            let events = (start..capacity.len()).filter(|i| capacity[*i] != capacity[*i - 1]).count();
            self.outcome.metrics.push(metrics::count(
                "wasm.memory_grow_events_after_warmup",
                Layer::B,
                "count",
                events as f64,
                "WebAssembly.Memory.buffer.byteLength sampled after each teardown",
            ));
        }
        let live: Vec<f64> = series.iter().filter_map(|r| r.live_bytes).collect();
        if live.len() == series.len() {
            let growth = live[live.len() - 1] - live[start - 1];
            self.outcome.metrics.push(metrics::trend(
                "wasm.live_bytes_growth_after_warmup",
                Layer::B,
                "bytes",
                growth,
                "the counting allocator, after each teardown",
            ));
        }
        for (name, pick) in [
            (
                "resource.listeners.after_teardown.per_cycle",
                (|r: &Resources| r.listeners) as fn(&Resources) -> Option<f64>,
            ),
            ("resource.live_bytes.after_teardown.per_cycle", |r| r.live_bytes),
            ("resource.capacity.after_teardown.per_cycle", |r| r.capacity),
            ("resource.dom_elements.after_teardown.per_cycle", |r| r.dom_elements),
        ] {
            let values: Vec<f64> = series.iter().filter_map(pick).collect();
            if !values.is_empty() {
                self.outcome.samples.insert(name.to_string(), values);
            }
        }
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Runs `manifest` in `mode`, and always gives an outcome: a failure is a status and a message, never a trap.
pub fn run(manifest: &Manifest, mode: Mode, expected: Option<&Expected>) -> RunOutcome {
    run_with_fault(manifest, mode, expected, None)
}

/// The same, with a deliberate fault in what is observed. See [`Fault`].
pub fn run_with_fault(
    manifest: &Manifest,
    mode: Mode,
    expected: Option<&Expected>,
    fault: Option<Fault>,
) -> RunOutcome {
    run_with_options(manifest, mode, expected, fault, 0.0)
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The same, with a deliberate delay of `harness_delay_us` microseconds in the harness's own work for every operation.
///
/// The delay is in the reference model's turn, which is not part of any timed interval, so no timing may show it. The
/// self-test runs a workload with and without it, and fails if a figure moves.
pub fn run_with_options(
    manifest: &Manifest,
    mode: Mode,
    expected: Option<&Expected>,
    fault: Option<Fault>,
    harness_delay_us: f64,
) -> RunOutcome {
    if !matches!(mode, Mode::Correctness | Mode::Performance | Mode::Endurance) {
        let mut outcome = RunOutcome::new("B", mode);
        outcome.status = "error";
        outcome.error = Some(format!("the {mode:?} mode is not implemented in this runner yet"));
        return outcome;
    }
    let mut run = match Run::new(manifest, mode) {
        Ok(run) => run,
        Err(message) => {
            let mut outcome = RunOutcome::new("B", mode);
            outcome.status = "error";
            outcome.error = Some(message);
            return outcome;
        },
    };
    run.fault = fault;
    run.harness_delay_us = harness_delay_us;
    if let Err(message) = run.execute() {
        run.outcome.status = "error";
        run.outcome.error = Some(message);
    }
    run.finish(expected)
}
