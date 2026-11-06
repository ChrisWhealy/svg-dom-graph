//! The endurance figures: whether capacity is reused, and whether live memory keeps growing.

use super::Run;
use super::resources::Resources;
use perf_harness::endurance::{SeriesPoint, Verdict, judge, summarize};
use perf_harness::metrics::{self, Layer};
use perf_harness::outcome::Gate;

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
impl Run<'_> {
    /// Whether capacity is reused, and whether live bytes keep growing, once the warm-up cycles are over.
    pub(super) fn emit_endurance_metrics(&mut self) {
        let points: Vec<SeriesPoint> = self
            .teardown_series
            .iter()
            .map(|entry| SeriesPoint {
                cycle: entry.cycle,
                measured: entry.measured,
                live_bytes: entry.resources.live_bytes,
                capacity: entry.resources.capacity,
            })
            .collect();
        let summary = summarize(&points);
        let measured_cycles = u64::from(self.manifest.execution.measurement_iterations);
        // Cycles that ran a checkpoint are not in the series, and what is in it is said, so that a figure is never read as
        // covering more than it does.
        let coverage = match (summary.first_measured_cycle, summary.last_measured_cycle) {
            (Some(first), Some(last)) => format!(
                "cycles {first} to {last}, {} of the {measured_cycles} measured cycles, the rest having run a checkpoint",
                summary.measured_points
            ),
            _ => "no measured cycle was sampled".to_string(),
        };
        let reference = summary
            .reference_cycle
            .map_or("none".to_string(), |cycle| format!("cycle {cycle}"));

        // The gate: live memory does not keep growing, to within what the manifest allows. A run with too little to judge by is
        // inconclusive, which is not a pass.
        let allowance = self.manifest.limits.live_growth_allowance_bytes.unwrap_or(0) as f64;
        let (status, detail) = match judge(&summary, allowance) {
            Verdict::Pass(detail) => ("pass", detail),
            Verdict::Fail(detail) => ("fail", detail),
            Verdict::Inconclusive(detail) => ("inconclusive", detail),
        };
        self.outcome.gates.push(Gate {
            name: "endurance.live_bytes_do_not_grow".to_string(),
            status,
            detail,
        });

        // What the first cycle left behind, which is a cost of first use and is not part of the interval above. It is the live
        // bytes after the first teardown less the baseline, and it is only known if the first cycle is in the series.
        match (self.teardown_series.first(), self.baseline.live_bytes) {
            (Some(first), Some(baseline)) if first.cycle == 1 && first.resources.live_bytes.is_some() => {
                self.outcome.metrics.push(metrics::count(
                    "wasm.live_bytes_one_time_after_first_cycle",
                    Layer::B,
                    "bytes",
                    first.resources.live_bytes.unwrap_or(0.0) - baseline,
                    "the counting allocator after the first teardown, less the baseline taken before any scene. A cost of first use, kept out of the endurance interval",
                ));
            },
            _ => self.outcome.metrics.push(metrics::unavailable(
                "wasm.live_bytes_one_time_after_first_cycle",
                Layer::B,
                "the first cycle is not in the series, or a live-bytes reading was missing",
            )),
        }

        self.outcome.metrics.push(metrics::count(
            "lifecycle.measured_cycles_sampled",
            Layer::B,
            "count",
            summary.measured_points as f64,
            &format!("the endurance series, {coverage}"),
        ));
        match summary.grow_events {
            Some(events) => self.outcome.metrics.push(metrics::count(
                "wasm.memory_grow_events_after_warmup",
                Layer::B,
                "count",
                events as f64,
                &format!("WebAssembly.Memory.buffer.byteLength after each teardown, from {reference} on, {coverage}"),
            )),
            None => self.outcome.metrics.push(metrics::unavailable(
                "wasm.memory_grow_events_after_warmup",
                Layer::B,
                "fewer than two cycles were sampled, or a capacity reading was missing",
            )),
        }
        // The net change and the slope are different figures. The first is how far live memory moved, and one stray reading
        // can move it. The second is how fast it moves each cycle, through every measured point.
        match summary.net_live_change {
            Some(change) => self.outcome.metrics.push(metrics::trend(
                "wasm.live_bytes_net_change_after_warmup",
                Layer::B,
                "bytes",
                change,
                &format!("the counting allocator, the last measured cycle less {reference}, {coverage}"),
            )),
            None => self.outcome.metrics.push(metrics::unavailable(
                "wasm.live_bytes_net_change_after_warmup",
                Layer::B,
                "no measured cycle differs from the reference, or a live-bytes reading was missing",
            )),
        }
        match summary.live_slope_per_cycle {
            Some(slope) => self.outcome.metrics.push(metrics::trend(
                "wasm.live_bytes_slope_per_cycle",
                Layer::B,
                "bytes",
                slope,
                &format!(
                    "the least-squares slope of the counting allocator's live bytes against the cycle, {coverage}"
                ),
            )),
            None => self.outcome.metrics.push(metrics::unavailable(
                "wasm.live_bytes_slope_per_cycle",
                Layer::B,
                "fewer than 3 measured cycles were sampled, which is not a trend, or a live-bytes reading was missing",
            )),
        }

        // The raw series, with the cycle of every point beside it, so that nobody has to guess which cycles are in it.
        let series = &self.teardown_series;
        self.outcome.samples.insert(
            "resource.after_teardown.cycle".to_string(),
            series.iter().map(|entry| entry.cycle as f64).collect(),
        );
        self.outcome.samples.insert(
            "resource.after_teardown.measured".to_string(),
            series.iter().map(|entry| f64::from(u8::from(entry.measured))).collect(),
        );
        for (name, pick) in [
            (
                "resource.listeners.after_teardown.per_cycle",
                (|r: &Resources| r.listeners) as fn(&Resources) -> Option<f64>,
            ),
            ("resource.live_bytes.after_teardown.per_cycle", |r| r.live_bytes),
            ("resource.capacity.after_teardown.per_cycle", |r| r.capacity),
            ("resource.dom_elements.after_teardown.per_cycle", |r| r.dom_elements),
        ] {
            // Kept only if every point has a value, so that it lines up with the cycles.
            let values: Vec<f64> = series.iter().filter_map(|entry| pick(&entry.resources)).collect();
            if values.len() == series.len() && !values.is_empty() {
                self.outcome.samples.insert(name.to_string(), values);
            }
        }
    }
}
