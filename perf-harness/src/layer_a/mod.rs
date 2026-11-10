//! The Layer A runner: time the library's own DOM-free geometry, natively.
//!
//! Layer A measures work that has no DOM in it, with native machine code. Its figures are never subtracted from a Layer
//! B figure, since the same algorithm costs differently as WebAssembly, and the difference of the two is not the cost
//! of the DOM. They are reported on their own.
//!
//! What it times is the end points of a straight connector, `svg_dom_graph::geometry::boundary_point`, for every
//! connector at the node that has just been moved. That is the library's routing work for one move of workload W2,
//! without writing it to anything. Each time the stream reaches a checkpoint it also computes every connector's end
//! points with the library and compares them with the reference model's, which works them out independently from the
//! same rule. So it checks the library's geometry as well as timing it.

pub mod simulation;

use crate::corruption::{self, DISPLACEMENT, Target};
use crate::manifest::{Manifest, Mode, ViewPolicy, Workload};
use crate::metrics::{Layer, mark_resolution_limited, timing, unavailable};
use crate::model::Model;
use crate::ops::{Op, Phase, StreamHasher};
use crate::outcome::{ExecutedStream, FailedCheckpoint, RunOutcome, performance_runs_checkpoint};
use crate::report::build_report;
use crate::stats::batch_size;
use crate::workload::{WorkloadError, generate};
use std::collections::BTreeMap;
use std::hint::black_box;
use std::time::Instant;
use svg_dom::root::utils::{Point, Rect, Size};
use svg_dom_graph::geometry::boundary_point;

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// How far the library's end points may be from the model's. Both are basic floating-point arithmetic on the same
/// numbers.
const TOLERANCE: f64 = 1e-9;
/// How many times the first measured move is repeated to find out how long one repeat takes.
const CALIBRATION_REPEATS: u32 = 400;
/// A failure list stops here, since a rule that is wrong is wrong everywhere.
const MAX_REPORTED: usize = 20;

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[derive(Debug, Default)]
struct State {
    names: Vec<String>,
    by_name: BTreeMap<String, usize>,
    boxes: Vec<(f64, f64, f64, f64)>,
    edges: Vec<(usize, usize)>,
    incident: Vec<Vec<usize>>,
}

impl State {
    fn rect(&self, node: usize) -> Rect {
        let (x, y, w, h) = self.boxes[node];
        Rect {
            origin: Point::new(x, y),
            size: Size::new(w, h),
        }
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    /// The library's end points of connector `edge`, as `[x1, y1, x2, y2]`.
    fn endpoints(&self, edge: usize) -> [f64; 4] {
        let (from, to) = self.edges[edge];
        let (a, b) = (self.rect(from), self.rect(to));
        let centre = |r: Rect| Point::new(r.origin.x + r.size.width / 2.0, r.origin.y + r.size.height / 2.0);
        let (start, end) = (boundary_point(a, centre(b)), boundary_point(b, centre(a)));
        [start.x, start.y, end.x, end.y]
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The smallest step that `Instant` makes, in microseconds.
pub fn measure_resolution() -> f64 {
    let mut smallest = f64::INFINITY;
    let mut last = Instant::now();
    for _ in 0..200_000 {
        let now = Instant::now();
        let step = now.duration_since(last).as_secs_f64() * 1e6;
        if step > 0.0 && step < smallest {
            smallest = step;
        }
        last = now;
    }
    if smallest.is_finite() { smallest } else { 0.0 }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Runs `manifest` in the correctness or performance mode.
///
/// Correctness runs every operation and cross-checks every checkpoint, and keeps no timing. Performance times the moves
/// of workload W2 and cross-checks only the last two checkpoints, outside the timing. The other workloads have no
/// DOM-free work to time, so they report that and are only cross-checked.
///
/// # Errors
///
/// Returns [`WorkloadError`] if the stream cannot be generated or the model refuses an operation.
pub fn run(manifest: &Manifest, mode: Mode) -> Result<RunOutcome, WorkloadError> {
    run_corrupted(manifest, mode, None)
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The same, with the last mutation before one of the two executed checkpoints applied wrongly to this runner's own
/// state.
///
/// The model is given the operation as the stream has it, so the checkpoint that follows must find the two apart and
/// the run must come out `invalid_incorrect`. See [`corruption`]. A corruption that finds nothing to corrupt is an
/// error, so that a test of it cannot pass by corrupting nothing.
///
/// # Errors
///
/// As for [`run`], and if the corruption was never applied.
pub fn run_corrupted(manifest: &Manifest, mode: Mode, corrupt: Option<Target>) -> Result<RunOutcome, WorkloadError> {
    if !matches!(mode, Mode::Correctness | Mode::Performance) {
        return Err(WorkloadError::Plan(format!("the Layer A runner has no {mode:?} mode")));
    }
    let report = build_report(manifest, "layer-a")?;
    let total_checkpoints = report.checkpoints.len() as u64;
    let view = manifest.checkpoint.as_ref().map_or(ViewPolicy::Unchanged, |policy| policy.view);
    let timing_wanted = mode == Mode::Performance && manifest.workload == Workload::ConnectorRerouting;

    let mut outcome = RunOutcome::new("A", mode);
    outcome.timer_resolution_us = measure_resolution();
    let resolution = outcome.timer_resolution_us;

    let mut state = State::default();
    let mut model = Model::new();
    let mut hasher = StreamHasher::new();
    let mut measuring = false;
    let mut batch: Option<u32> = None;
    let mut observations: Vec<f64> = Vec::new();
    let mut moves_without_connectors = 0u64;
    let corrupt_at = corrupt
        .map(|target| corruption::locate(manifest, target))
        .transpose()?
        .map(|c| c.ordinal);
    let (mut ordinal, mut corrupted) = (0u64, false);

    generate(manifest, &mut |op: &Op| {
        let corrupt_now = corrupt_at == Some(ordinal);
        ordinal += 1;
        corrupted |= corrupt_now;
        model.apply(op)?;
        let skipped = matches!(op, Op::Checkpoint { index } if mode == Mode::Performance && !performance_runs_checkpoint(*index, total_checkpoints));
        if !skipped {
            hasher.push(op);
        }
        match op {
            Op::CreateScene => state = State::default(),
            Op::AddNode { key, x, y, w, h, .. } => {
                state.by_name.insert(key.clone(), state.names.len());
                state.names.push(key.clone());
                let x = if corrupt_now { *x + DISPLACEMENT } else { *x };
                state.boxes.push((x as f64, *y as f64, *w as f64, *h as f64));
                state.incident.push(Vec::new());
            },
            Op::AddEdge { from, to } => {
                let (from, to) = (state.by_name[from], state.by_name[to]);
                // A corrupted edge is left out of this runner's state, and the model still has it.
                if !corrupt_now {
                    let edge = state.edges.len();
                    state.edges.push((from, to));
                    state.incident[from].push(edge);
                    state.incident[to].push(edge);
                }
            },
            Op::MoveNode { key, x, y } => {
                let node = state.by_name[key];
                let x = if corrupt_now { *x + DISPLACEMENT } else { *x };
                state.boxes[node].0 = x as f64;
                state.boxes[node].1 = *y as f64;
                if timing_wanted && measuring {
                    if state.incident[node].is_empty() {
                        moves_without_connectors += 1;
                    } else {
                        let size = *batch.get_or_insert_with(|| calibrate(&state, node, resolution));
                        observations.push(time_endpoints(&state, node, size));
                    }
                }
            },
            Op::Phase { phase } => measuring = *phase == Phase::Measure,
            Op::Checkpoint { index } => {
                if !skipped {
                    cross_check(&state, &model, view, *index, &mut outcome)?;
                }
            },
            // The host is gone with everything in it. Dropping the handle alone leaves the DOM, which this runner has
            // none of. A corrupted removal leaves this runner's scene where it was, as a removal that did not happen
            // would.
            Op::RemoveHost if !corrupt_now => state = State::default(),
            Op::RemoveHost => {},
            Op::DropScene => {},
        }
        if corrupt_now && matches!(op, Op::DropScene | Op::CreateScene) {
            return Err(format!("{op:?} cannot be corrupted"));
        }
        Ok(())
    })?;
    if corrupt.is_some() && !corrupted {
        return Err(WorkloadError::Plan("the corruption was never applied".to_string()));
    }

    if timing_wanted {
        let mut metric = timing(
            "rust.connector_endpoints",
            Layer::A,
            "us",
            &observations,
            batch,
            "std::time::Instant, over the library's boundary_point for each connector at the moved node",
        );
        mark_resolution_limited(&mut metric, resolution);
        outcome.metrics.push(metric);
        outcome.samples.insert("rust.connector_endpoints".to_string(), observations);
        if moves_without_connectors > 0 {
            outcome.notes.push(format!(
                "{moves_without_connectors} measured moves were of a node with no connector, and were not timed"
            ));
        }
    } else {
        let reason = if mode == Mode::Correctness {
            "the correctness mode keeps no timing"
        } else {
            "this workload has no DOM-free library work to time"
        };
        outcome.metrics.push(unavailable("rust.connector_endpoints", Layer::A, reason));
    }

    let operations = hasher.operations();
    let (full, mutations) = hasher.finish();
    let (scope, sha256, expected) = match mode {
        Mode::Performance => ("mutations", mutations, report.stream.mutations_sha256.clone()),
        _ => ("full", full, report.stream.sha256.clone()),
    };
    outcome.stream = Some(ExecutedStream {
        scope,
        operations,
        matches: sha256 == expected,
        sha256,
        expected_sha256: expected,
    });
    outcome.settle();
    Ok(outcome)
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Times `size` repeats of the library's end points for every connector at `node`, and gives the time of one repeat in
/// microseconds.
fn time_endpoints(state: &State, node: usize, size: u32) -> f64 {
    let incident = &state.incident[node];
    let mut checksum = 0.0;
    let started = Instant::now();
    for _ in 0..size {
        // Every repeat computes the same thing from the same state, so a compiler could compute it once and reuse the
        // answer for the rest. Passing the state and the edge through `black_box` on every repeat gives it no such
        // right, and the time then scales with the number of connectors, which `the_time_scales_with_the_connectors`
        // checks.
        let state = black_box(state);
        for &edge in incident {
            let points = black_box(state.endpoints(black_box(edge)));
            checksum += points[0] + points[3];
        }
    }
    let elapsed = started.elapsed();
    black_box(checksum);
    elapsed.as_secs_f64() * 1e6 / f64::from(size)
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// How many repeats to batch, from the time of one.
fn calibrate(state: &State, node: usize, resolution_us: f64) -> u32 {
    let single = time_endpoints(state, node, CALIBRATION_REPEATS);
    batch_size(resolution_us, single)
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Computes every connector's end points with the library, and compares them with the model's.
fn cross_check(
    state: &State,
    model: &Model,
    view: ViewPolicy,
    index: u64,
    outcome: &mut RunOutcome,
) -> Result<(), String> {
    let checkpoint = model.checkpoint(view)?;
    let connectors: Vec<_> = checkpoint
        .geometry
        .elements
        .iter()
        .filter(|element| element.kind == "connector_path")
        .collect();
    // A runner with a different number of connectors than the model has no end points to put beside the model's.
    if connectors.len() != state.edges.len() {
        outcome.checkpoints_failed.push(FailedCheckpoint {
            section: "topology".to_string(),
            detail: format!(
                "checkpoint {index}: the model has {} connectors and the runner has {}",
                connectors.len(),
                state.edges.len()
            ),
        });
        return Ok(());
    }
    let mut wrong = 0usize;
    for (edge, element) in connectors.into_iter().enumerate() {
        let got = state.endpoints(edge);
        let off = if element.coordinates.is_empty() {
            // The model says the two ends are one point, and the library draws no path for that.
            got[0] != got[2] || got[1] != got[3]
        } else {
            element.coordinates.len() != 4
                || element.coordinates.iter().zip(got).any(|(want, got)| {
                    let distance = (want - got).abs();
                    distance.is_nan() || distance > TOLERANCE
                })
        };
        if off {
            wrong += 1;
            if wrong <= MAX_REPORTED {
                outcome.checkpoints_failed.push(FailedCheckpoint {
                    section: "geometry".to_string(),
                    detail: format!(
                        "checkpoint {index}: the library's end points of {} are {got:?}, the model's are {:?}",
                        element.key, element.coordinates
                    ),
                });
            }
        }
    }
    if wrong == 0 {
        outcome.checkpoints_passed += 1;
    }
    Ok(())
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[cfg(test)]
mod unit_tests;
