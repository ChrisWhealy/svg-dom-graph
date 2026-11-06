//! What a run counts before the build, after it and after the teardown, and the figures made from the counts.

use super::Run;
use super::phase;
use crate::alloc;
use crate::memory::capacity_bytes;
use perf_harness::manifest::Mode;
use perf_harness::metrics::{self, Layer};
use wasm_bindgen::JsCast;

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// What was counted after one teardown, and which cycle it was.
///
/// The cycle and the phase are kept with the figures, so that the endurance summary draws the end of the warm-up from them and
/// not from the position of the sample in a list that has had cycles taken out of it.
#[derive(Debug, Clone, Copy)]
pub(super) struct SeriesEntry {
    pub(super) cycle: u64,
    pub(super) measured: bool,
    pub(super) resources: Resources,
}

/// What can be counted at one moment.
#[derive(Debug, Clone, Copy, Default)]
pub(super) struct Resources {
    pub(super) dom_elements: Option<f64>,
    pub(super) listeners: Option<f64>,
    pub(super) live_bytes: Option<f64>,
    pub(super) live_allocations: Option<f64>,
    pub(super) capacity: Option<f64>,
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
impl Run<'_> {
    pub(super) fn sample_resources(&self) -> Resources {
        alloc::set_tag(phase::SAMPLE);
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
    pub(super) fn emit_resource_metrics(&mut self) {
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
}
