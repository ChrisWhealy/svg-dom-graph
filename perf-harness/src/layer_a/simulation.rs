//! The Layer A runner of workload W6: time the pure simulation engine, natively.
//!
//! An observation is four consecutive nonempty timestamp slices, so the engine is stepped four times between two reads
//! of the clock. The trace is fixed before anything is timed. Slices 1 to 256 are priming and are not timed. The next
//! 40 are the warm-up and the 240 after them the measured window, which are 10 and 60 batches of four. A second series
//! times the replay that a seek makes: a fresh engine stepped to the cursor that a batch ended at.
//!
//! Nothing here touches a DOM, a controller or a surface, and none of it is a frame-time figure. The independent
//! engine's hash of the state at every slice is given to the runner, and every checkpoint that it executes is compared
//! with it. A batch's popped-event count, its four slice indices and its end-of-batch checkpoint are recorded, outside
//! the timed interval. A pop is a popped heap event, and a stale one counts.

use crate::layer_a::measure_resolution;
use crate::manifest::{Manifest, Mode, Simulation};
use crate::metrics::{Layer, count, mark_resolution_limited, timing};
use crate::outcome::{ExecutedStream, FailedCheckpoint, Gate, RunOutcome};
use serde_json::{Value, json};
use service_sim_engine::trace::Scenario;
use service_sim_engine::{Engine, Limits, Outcome};
use sha2::{Digest, Sha256};
use std::hint::black_box;
use std::time::Instant;

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// What the independent engine says the trace is: its hash of the state at the start and after each slice, and the
/// counters at the end. The runner is given it, and is not given the way to work it out.
#[derive(Debug, Clone)]
pub struct Expected {
    /// The hash at slice 0, and then at each slice.
    pub checkpoints: Vec<String>,
    pub pops: u64,
    pub stale: u64,
    pub slices: u64,
    /// The first slice of the warm-up window, which the independent engine's trace says. The runner's own priming count
    /// is held to it.
    pub warmup_starts_at: u64,
}

impl Expected {
    /// Reads the file that the orchestrator writes: `{ "checkpoints": [...], "counters": { "pops", "stale", "slices" }
    /// }`.
    ///
    /// # Errors
    ///
    /// Returns an error if a field is missing or of the wrong kind.
    pub fn from_json(value: &Value) -> Result<Self, String> {
        let checkpoints = value["checkpoints"]
            .as_array()
            .ok_or("the expected trace has no checkpoints")?
            .iter()
            .map(|c| c.as_str().map(str::to_owned).ok_or("a checkpoint is not text"))
            .collect::<Result<Vec<_>, _>>()?;
        let counter = |name: &str| {
            value["counters"][name]
                .as_u64()
                .ok_or(format!("the expected trace has no counter {name}"))
        };
        Ok(Self {
            checkpoints,
            pops: counter("pops")?,
            stale: counter("stale")?,
            slices: counter("slices")?,
            warmup_starts_at: value["windows"]["warmup"][0]
                .as_u64()
                .ok_or("the expected trace has no warm-up window")?,
        })
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
fn engine_for(seed: u64, arrivals: Option<u64>) -> Result<Engine, String> {
    let mut s = Scenario::mixed_outage_7(seed).map_err(|e| e.message())?;
    if let (Some(n), Some(params)) = (arrivals, s.params.as_mut()) {
        params.arrivals = n;
    }
    Engine::new(&s.config_id, s.seed, s.config, &s.script, s.params.as_ref(), Limits::default())
        .map_err(|e| e.message())
}

fn chain_sha256(checkpoints: &[String]) -> String {
    let digest = Sha256::digest(checkpoints.join("\n").as_bytes());
    digest.iter().map(|byte| format!("{byte:02x}")).collect()
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Runs the simulator workload natively, in its correctness or performance mode.
///
/// Correctness steps the whole trace and compares every slice's checkpoint with the independent engine's, and the
/// counters at the end. Performance steps the priming slices, then times the warm-up and measured batches, and compares
/// the checkpoint at the end of each batch, outside the timing.
///
/// # Errors
///
/// Returns an error if the engine cannot be built or stepped, or the mode is not one that this layer has.
pub fn run(manifest: &Manifest, mode: Mode, expected: &Expected) -> Result<RunOutcome, String> {
    let simulation = manifest.simulation.as_ref().ok_or("the manifest has no [simulation] table")?;
    let mut outcome = RunOutcome::new("A", mode);
    outcome.timer_resolution_us = measure_resolution();
    match mode {
        Mode::Correctness => correctness(manifest, &mut outcome, expected)?,
        Mode::Performance => performance(manifest, simulation, &mut outcome, expected)?,
        other => return Err(format!("the Layer A runner of the simulator has no {other:?} mode")),
    }
    outcome.settle();
    Ok(outcome)
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
fn fail(outcome: &mut RunOutcome, detail: String) {
    outcome.checkpoints_failed.push(FailedCheckpoint {
        section: "content".to_owned(),
        detail,
    });
}

fn correctness(manifest: &Manifest, outcome: &mut RunOutcome, expected: &Expected) -> Result<(), String> {
    let arrivals = manifest.simulation.as_ref().and_then(|s| s.arrivals);
    let mut engine = engine_for(manifest.seed, arrivals)?;
    let mut chain = vec![engine.checkpoint(0, 0)];
    check_slice(outcome, expected, 0, &chain[0]);
    while let Some(record) = engine.step().map_err(|e| e.message())? {
        let index = record.index;
        check_slice(outcome, expected, index, &record.checkpoint);
        chain.push(record.checkpoint);
    }
    let counters = engine.counters();
    let mut agree = |name: &str, got: u64, want: u64| {
        if got != want {
            fail(
                outcome,
                format!("{name}: the engine counted {got} and the independent engine {want}"),
            );
        }
    };
    agree("pops", counters.pops, expected.pops);
    agree("stale pops", counters.stale, expected.stale);
    agree("slices", counters.slices, expected.slices);
    let sha = chain_sha256(&chain);
    let wanted = chain_sha256(&expected.checkpoints);
    outcome.stream = Some(ExecutedStream {
        scope: "full",
        operations: counters.slices,
        matches: sha == wanted,
        sha256: sha,
        expected_sha256: wanted,
    });
    outcome.metrics.push(count(
        "simulation.actual_pops",
        Layer::A,
        "count",
        counters.pops as f64,
        "the engine's own count of popped heap events, stale ones included",
    ));
    Ok(())
}

fn check_slice(outcome: &mut RunOutcome, expected: &Expected, index: u64, got: &str) {
    match expected.checkpoints.get(index as usize) {
        Some(want) if want == got => outcome.checkpoints_passed += 1,
        Some(want) => fail(
            outcome,
            format!("slice {index}: the engine's hash {got} is not the independent engine's {want}"),
        ),
        None => fail(outcome, format!("slice {index}: the independent engine has no such slice")),
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
fn performance(
    manifest: &Manifest,
    simulation: &Simulation,
    outcome: &mut RunOutcome,
    expected: &Expected,
) -> Result<(), String> {
    let [priming, warmup_slices, _] = Simulation::WINDOW_SLICES;
    let batch = simulation.batch_slices;
    let warmup = manifest.execution.warmup_iterations;
    let measured = manifest.execution.measurement_iterations;
    if expected.slices < u64::from(Simulation::MIN_SLICES) {
        return Err(format!(
            "the trace has {} slices and the windows need {}: it should have been refused before any timing",
            expected.slices,
            Simulation::MIN_SLICES
        ));
    }
    let mut engine = engine_for(manifest.seed, simulation.arrivals)?;
    // The priming slices are not timed and are not a steady state. The last of them is still checked, since everything
    // after it rests on it.
    let mut executed = vec![engine.checkpoint(0, 0)];
    check_slice(outcome, expected, 0, &executed[0]);
    for index in 1..=u64::from(priming) {
        let record = engine.step().map_err(|e| e.message())?.ok_or("the trace ended while priming")?;
        check_slice(outcome, expected, index, &record.checkpoint);
        executed.push(record.checkpoint);
    }

    let mut batches = Vec::new();
    let mut replays = Vec::new();
    let mut records = Vec::with_capacity(batch as usize);
    let mut pops_total = 0_u64;
    let mut cursor = u64::from(priming);
    for number in 0..(warmup + measured) {
        records.clear();
        let timed = number >= warmup;
        // The timed interval is four steps of the engine and nothing else: the records are looked at afterwards.
        let start = Instant::now();
        for _ in 0..batch {
            records.push(black_box(engine.step()));
        }
        let elapsed_us = start.elapsed().as_secs_f64() * 1e6;
        let mut indices = Vec::with_capacity(batch as usize);
        let (mut pops, mut stale) = (0_u64, 0_u64);
        let mut end = String::new();
        for record in records.drain(..) {
            let record = record.map_err(|e| e.message())?.ok_or("the trace ended inside a batch")?;
            indices.push(record.index);
            pops += record.pops.len() as u64;
            stale += record.pops.iter().filter(|pop| pop.outcome == Outcome::Stale).count() as u64;
            check_slice(outcome, expected, record.index, &record.checkpoint);
            executed.push(record.checkpoint.clone());
            end = record.checkpoint;
        }
        cursor += u64::from(batch);
        pops_total += pops;
        if timed {
            batches.push(elapsed_us);
            outcome.cases.push(json!({
                "observation": number - warmup,
                "slices": indices,
                "pops": pops,
                "stale_pops": stale,
                "end_checkpoint": end,
            }));
            // A seek to this cursor replays the run from its start, so this is what a seek here costs the engine.
            let start = Instant::now();
            let mut replay = engine_for(manifest.seed, simulation.arrivals)?;
            for _ in 0..cursor {
                black_box(replay.step().map_err(|e| e.message())?);
            }
            replays.push(start.elapsed().as_secs_f64() * 1e6);
        }
    }
    let _ = warmup_slices;
    let resolution = outcome.timer_resolution_us;
    let mut batch_metric = timing(
        "simulation.engine.four_slices",
        Layer::A,
        "us",
        &batches,
        None,
        "the pure engine stepped through four consecutive nonempty slices, each step working out the slice and its checkpoint, and nothing else",
    );
    mark_resolution_limited(&mut batch_metric, resolution);
    let mut replay_metric = timing(
        "simulation.engine.replay_to_cursor",
        Layer::A,
        "us",
        &replays,
        None,
        "a fresh engine stepped from the start to the cursor at which a measured batch ended, which is what a seek there costs the engine",
    );
    mark_resolution_limited(&mut replay_metric, resolution);
    outcome.metrics.push(batch_metric);
    outcome.metrics.push(replay_metric);
    outcome.metrics.push(count(
        "simulation.actual_pops_in_batches",
        Layer::A,
        "count",
        pops_total as f64,
        "popped heap events, stale ones included, over every warm-up and measured batch",
    ));
    outcome.samples.insert("simulation.engine.four_slices".to_owned(), batches);
    outcome.samples.insert("simulation.engine.replay_to_cursor".to_owned(), replays);
    // What was executed is the trace up to the end of the measured window, and the digest is held to the independent
    // engine's for the same prefix. It is the mutations' scope: a performance run does not execute the whole trace.
    let wanted = chain_sha256(&expected.checkpoints[..=cursor as usize]);
    let sha = chain_sha256(&executed);
    outcome.stream = Some(ExecutedStream {
        scope: "mutations",
        operations: cursor,
        matches: sha == wanted,
        sha256: sha,
        expected_sha256: wanted,
    });
    // The windows are fixed. The measured one begins right after the warm-up one, and a first measured slice that is
    // anywhere else means that a window was wrapped, reused or shifted.
    let starts_at = expected.warmup_starts_at + u64::from(warmup) * u64::from(batch);
    let ends_at = starts_at + u64::from(measured) * u64::from(batch) - 1;
    let first = outcome.cases.first().and_then(|case| case["slices"][0].as_u64());
    let fixed = first == Some(starts_at) && cursor == ends_at && ends_at <= u64::from(Simulation::MIN_SLICES);
    outcome.gates.push(Gate {
        name: "simulation.windows_are_not_wrapped".to_owned(),
        status: if fixed { "pass" } else { "fail" },
        detail: format!(
            "{warmup} warm-up and {measured} measured batches of {batch} slices: the measured ones began at slice {} and ended at {cursor}, and the fixed windows have them begin at {starts_at} and end at {ends_at}, inside the {} slices",
            first.map_or_else(|| "none".to_owned(), |slice| slice.to_string()),
            Simulation::MIN_SLICES
        ),
    });
    Ok(())
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[cfg(test)]
#[path = "simulation_tests.rs"]
mod tests;
