//! What a runner reports after one repetition of one workload in one mode.
//!
//! It is the part of a result document that only the runner can know: the metrics, the raw observations, how the
//! checkpoints came out and which operations were really executed. The orchestrator wraps it with the environment and
//! the manifest to make the document that `perf/schemas/result.schema.json` describes. Both runners, native and wasm,
//! produce this one shape.

use crate::manifest::Mode;
use serde::Serialize;
use serde_json::Value;
use std::collections::BTreeMap;

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct FailedCheckpoint {
    pub section: String,
    pub detail: String,
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// A deterministic check that a run makes of itself, and what it found.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Gate {
    pub name: String,
    /// `pass`, `fail` or `inconclusive`. Inconclusive is never a pass.
    pub status: &'static str,
    pub detail: String,
}

/// Which digest of the workload report a run executed, and whether it matched.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ExecutedStream {
    /// `"full"` for a run that executed every operation, and `"mutations"` for one that skipped the checkpoints.
    pub scope: &'static str,
    pub operations: u64,
    pub sha256: String,
    pub expected_sha256: String,
    pub matches: bool,
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[derive(Debug, Clone, Serialize)]
pub struct RunOutcome {
    /// `ok`, `invalid_incorrect`, `refused`, `timed_out` or `error`, as in the result schema.
    pub status: &'static str,
    pub error: Option<String>,
    pub mode: Mode,
    /// `"A"` for the native runner and `"B"` for the wasm one.
    pub layer: &'static str,
    pub metrics: Vec<Value>,
    /// Every observation behind a timing metric, by metric name. Nothing is dropped.
    pub samples: BTreeMap<String, Vec<f64>>,
    pub checkpoints_passed: u64,
    pub checkpoints_failed: Vec<FailedCheckpoint>,
    pub stream: Option<ExecutedStream>,
    pub gates: Vec<Gate>,
    pub timer_resolution_us: f64,
    /// Findings worth keeping that are not failures, such as a provisional rule that held.
    pub notes: Vec<String>,
}

impl RunOutcome {
    pub fn new(layer: &'static str, mode: Mode) -> Self {
        Self {
            status: "ok",
            error: None,
            mode,
            layer,
            metrics: Vec::new(),
            samples: BTreeMap::new(),
            checkpoints_passed: 0,
            checkpoints_failed: Vec::new(),
            stream: None,
            gates: Vec::new(),
            timer_resolution_us: 0.0,
            notes: Vec::new(),
        }
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    /// Marks the run as not valid if a checkpoint failed or the executed stream is not the intended one.
    ///
    /// A figure from such a run must never be used, however good it looks. An error is left alone, since it is already
    /// worse.
    pub fn settle(&mut self) {
        if self.status != "ok" {
            return;
        }
        if !self.checkpoints_failed.is_empty() || self.stream.as_ref().is_some_and(|stream| !stream.matches) {
            self.status = "invalid_incorrect";
        }
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Whether a checkpoint with this index is executed in a performance run.
///
/// A performance run keeps its timed region free of checks, so it skips the checkpoints along the way. It still has to
/// prove that what it timed was right, so it executes the last two outside the timing: the last, and the one before it.
/// For the cycle workloads those are the scene after the final build and the empty page after the final removal. For
/// rerouting, they are the state a short while before the end, and the end.
pub fn performance_runs_checkpoint(index: u64, total: u64) -> bool {
    index + 2 >= total
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[cfg(test)]
mod unit_tests;
