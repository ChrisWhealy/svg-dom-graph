//! The clock, and the timing figures that a run reports.

use super::Run;
use perf_harness::metrics::{self, Layer};

/// How many times `performance.now()` is called to find the smallest step it makes.
const RESOLUTION_PROBES: u32 = 200_000;

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
impl Run<'_> {
    pub(super) fn now_ms(&self) -> f64 {
        self.performance.now()
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    /// The smallest step that `performance.now()` makes, and what one call costs, both in microseconds.
    pub(super) fn measure_resolution(&self) -> (f64, f64) {
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
    pub(super) fn emit_timing_metrics(&mut self) {
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
}
