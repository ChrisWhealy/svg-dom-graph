//! The life of a run: taking in each operation of the stream, and what is done when a build or a teardown is over.

use super::phase;
use super::queue::Kind;
use super::resources::SeriesEntry;
use super::{Run, describe};
use crate::alloc;
use perf_harness::manifest::Mode;
use perf_harness::ops::{Op, Phase};
use perf_harness::outcome::performance_runs_checkpoint;
use perf_harness::workload::generate;

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
impl Run<'_> {
    pub(super) fn execute(&mut self) -> Result<(), String> {
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
        alloc::set_tag(phase::COLLECT);
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
                alloc::set_tag(phase::PREPARE);
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
    pub(super) fn on_teardown_complete(&mut self) {
        if self.measuring && self.timing_on && self.cycle_teardown_us > 0.0 {
            self.samples.entry("teardown.total").or_default().push(self.cycle_teardown_us);
        }
        self.cycle_teardown_us = 0.0;
        if self.track_cycles >= self.cycle {
            // The blocks that were allocated since the baseline and are still live, listed with the tracker off, since
            // listing them allocates. Tracking begins again straight away.
            alloc::stop_tracking();
            let now = alloc::stats();
            let over_baseline = now.live_bytes.saturating_sub(self.baseline.live_bytes.unwrap_or(0.0) as usize);
            let blocks_over = now
                .live_allocations
                .saturating_sub(self.baseline.live_allocations.unwrap_or(0.0) as usize);
            self.tracking_log
                .push((self.cycle, over_baseline, blocks_over, alloc::tracked_live()));
            alloc::start_tracking();
        }
        self.inject_leak();
        if self.mode == Mode::Endurance || self.cycle >= self.total_cycles {
            let sampled = self.sample_resources();
            self.after_teardown = Some(sampled);
            // A cycle that ran a checkpoint is left out of the series, since the checking itself uses memory.
            if self.mode == Mode::Endurance && !self.checkpoint_this_cycle {
                self.teardown_series.push(SeriesEntry {
                    cycle: self.cycle,
                    measured: self.measuring,
                    resources: sampled,
                });
            }
        }
    }
}
