//! Builds the metric entries of a result document, in the shape `perf/schemas/result.schema.json` asks for.
//!
//! A measured metric carries what its kind needs and nothing else, and one that could not be measured carries a reason
//! and no figures. The result schema holds every runner to that, so the builders are the one place it is written down.

use crate::stats::{Description, MIN_STEPS_PER_OBSERVATION, describe};
use serde_json::{Value, json};

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Where a metric was measured: native Rust, the wasm module driving a scene, or the browser.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Layer {
    A,
    B,
    C,
}

impl Layer {
    fn as_str(self) -> &'static str {
        match self {
            Self::A => "A",
            Self::B => "B",
            Self::C => "C",
        }
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
fn description_fields(description: &Description) -> (Value, Value) {
    let mut summary =
        json!({"min": description.summary.min, "p50": description.summary.p50, "max": description.summary.max});
    if let Some(p95) = description.summary.p95 {
        summary["p95"] = json!(p95);
    }
    if let Some(p99) = description.summary.p99 {
        summary["p99"] = json!(p99);
    }
    let within = json!({
        "mean": description.within.mean,
        "stddev": description.within.stddev,
        "iqr": description.within.iqr,
        "tukey_low": description.within.tukey_low,
        "tukey_high": description.within.tukey_high,
    });
    (summary, within)
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// A timing metric. `unit` is `"ms"` or `"us"`.
///
/// With `batch` above one, each observation is the time of that many operations together, divided by their number, and
/// the metric is a batch throughput and not a latency. With no observations it is reported as unavailable.
pub fn timing(name: &str, layer: Layer, unit: &str, samples: &[f64], batch: Option<u32>, mechanism: &str) -> Value {
    let Some(description) = describe(samples) else {
        return unavailable(name, layer, "no observation was made");
    };
    let (summary, within) = description_fields(&description);
    let mut metric = json!({
        "name": name,
        "layer": layer.as_str(),
        "status": "measured",
        "unit": unit,
        "kind": "latency",
        "samples": description.samples,
        "dropped": 0,
        "outliers_flagged": description.outliers_flagged,
        "summary": summary,
        "within_run": within,
        "mechanism": mechanism,
    });
    if let Some(size) = batch.filter(|size| *size > 1) {
        metric["kind"] = json!("batch_throughput");
        metric["batch_size"] = json!(size);
    }
    metric
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Flags a timing whose typical value is under 50 timer steps and that was not batched.
///
/// Such a figure is quantised: with a 5 us timer, an operation of 3 us reads as 0 or as 5. It is kept, since a larger
/// sample of them still says something, but it must not be compared finely, and the flag says so. Only an unbatched
/// latency in microseconds can carry it, since a batch is already made long enough.
pub fn mark_resolution_limited(metric: &mut Value, resolution_us: f64) {
    let (Some(p50), Some("latency"), Some("us")) = (
        metric["summary"]["p50"].as_f64(),
        metric["kind"].as_str(),
        metric["unit"].as_str(),
    ) else {
        return;
    };
    if resolution_us > 0.0 && p50 < MIN_STEPS_PER_OBSERVATION * resolution_us {
        metric["resolution_limited"] = json!(true);
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// A count or a byte total read at one moment.
pub fn count(name: &str, layer: Layer, unit: &str, value: f64, mechanism: &str) -> Value {
    json!({"name": name, "layer": layer.as_str(), "status": "measured", "unit": unit, "kind": "count", "value": value, "mechanism": mechanism})
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// A change over a run, such as how far a figure has grown since the warm-up.
pub fn trend(name: &str, layer: Layer, unit: &str, value: f64, mechanism: &str) -> Value {
    json!({"name": name, "layer": layer.as_str(), "status": "measured", "unit": unit, "kind": "trend", "value": value, "mechanism": mechanism})
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The high-water mark of a resource that is not handed back, such as wasm linear memory.
pub fn capacity(name: &str, layer: Layer, bytes: f64, mechanism: &str) -> Value {
    json!({"name": name, "layer": layer.as_str(), "status": "measured", "unit": "bytes", "kind": "capacity", "value": bytes, "mechanism": mechanism})
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// A metric that has no figure: the mechanism is not there, or it failed.
pub fn unavailable(name: &str, layer: Layer, reason: &str) -> Value {
    json!({"name": name, "layer": layer.as_str(), "status": "unavailable", "reason": reason})
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// A metric whose mechanism exists and failed.
pub fn measurement_error(name: &str, layer: Layer, reason: &str) -> Value {
    json!({"name": name, "layer": layer.as_str(), "status": "measurement_error", "reason": reason})
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[cfg(test)]
mod unit_tests;
