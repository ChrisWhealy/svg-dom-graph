//! The Layer B runner of the performance harness.
//!
//! It drives a real `svg-dom-graph` scene in a browser, through the library's public API only, times what it does, observes
//! what it drew, and counts what it allocated. The pieces that need a browser are built for wasm only. The counting
//! allocator's arithmetic is plain Rust and is tested natively.

pub mod alloc;
pub mod memory;
pub mod parse;

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[cfg(target_arch = "wasm32")]
pub mod observe;

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[cfg(target_arch = "wasm32")]
pub mod executor;

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[cfg(target_arch = "wasm32")]
pub mod allocator_check;

#[cfg(target_arch = "wasm32")]
pub mod interactive;

#[cfg(target_arch = "wasm32")]
pub mod persistent;

#[cfg(target_arch = "wasm32")]
pub mod recovery;

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[cfg(target_arch = "wasm32")]
mod exports {
    use perf_harness::manifest::{Manifest, Mode};
    use perf_harness::outcome::RunOutcome;
    use wasm_bindgen::prelude::*;

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    /// Reports a panic in the page's console, where a bare trap would give no message.
    #[wasm_bindgen(start)]
    fn start() {
        std::panic::set_hook(Box::new(|info| web_sys::console::error_1(&info.to_string().into())));
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    fn failed(mode: Mode, message: String) -> String {
        let mut outcome = RunOutcome::new("B", mode);
        outcome.status = "error";
        outcome.error = Some(message);
        serde_json::to_string(&outcome).unwrap_or_else(|_| "{\"status\":\"error\"}".to_string())
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    /// Runs one repetition of a workload and returns the outcome as JSON. It never throws: a failure is a status.
    ///
    /// `manifest_json` is the canonical manifest of a workload report. `expected_json` is `{"sha256", "mutations_sha256"}`
    /// from that report, made by another process, or empty to compare the stream with itself.
    #[wasm_bindgen]
    pub fn run_workload(manifest_json: &str, mode: &str, expected_json: &str) -> String {
        let mode = match mode {
            "correctness" => Mode::Correctness,
            "performance" => Mode::Performance,
            "endurance" => Mode::Endurance,
            "recovery" => Mode::Recovery,
            other => return failed(Mode::Performance, format!("there is no mode {other}")),
        };
        let manifest = match serde_json::from_str(manifest_json)
            .map_err(|e| e.to_string())
            .and_then(|v| Manifest::from_json(v).map_err(|e| e.to_string()))
        {
            Ok(manifest) => manifest,
            Err(message) => return failed(mode, message),
        };
        let expected = serde_json::from_str::<serde_json::Value>(expected_json).ok().and_then(|v| {
            Some(crate::executor::Expected {
                sha256: v["sha256"].as_str()?.to_string(),
                mutations_sha256: v["mutations_sha256"].as_str()?.to_string(),
            })
        });
        let outcome = crate::executor::run(&manifest, mode, expected.as_ref());
        serde_json::to_string(&outcome).unwrap_or_else(|e| failed(mode, e.to_string()))
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    /// Runs one repetition of a workload as a candidate that has been changed on purpose, and returns the outcome as JSON.
    ///
    /// `slowdown_us` is spent inside the timed interval after every library call of a node, an edge or a move, as a library
    /// that had got slower would. `leak` is `none`, `bytes`, `closure` or `library_watch`, made after every teardown, so that a
    /// run with `library_watch` ends with listeners that the scene's teardown did not release. A run with neither is the same
    /// as `run_workload`. `selftest-regression.mjs` uses it to prove that the comparison and the deterministic gates each
    /// notice what they exist to notice.
    #[wasm_bindgen]
    pub fn run_candidate(manifest_json: &str, mode: &str, expected_json: &str, slowdown_us: f64, leak: &str) -> String {
        let mode = match mode {
            "correctness" => Mode::Correctness,
            "performance" => Mode::Performance,
            other => return failed(Mode::Performance, format!("a candidate has no mode {other}")),
        };
        let manifest = match serde_json::from_str(manifest_json)
            .map_err(|e| e.to_string())
            .and_then(|v| Manifest::from_json(v).map_err(|e| e.to_string()))
        {
            Ok(manifest) => manifest,
            Err(message) => return failed(mode, message),
        };
        let expected = serde_json::from_str::<serde_json::Value>(expected_json).ok().and_then(|v| {
            Some(crate::executor::Expected {
                sha256: v["sha256"].as_str()?.to_string(),
                mutations_sha256: v["mutations_sha256"].as_str()?.to_string(),
            })
        });
        let leak = match leak {
            "none" => None,
            other => match crate::executor::Leak::parse(other, 64) {
                Some(leak) => Some(leak),
                None => return failed(mode, format!("there is no leak {other}")),
            },
        };
        let outcome = crate::executor::run_with_options(
            &manifest,
            mode,
            expected.as_ref(),
            crate::executor::Options {
                slowdown_us,
                leak,
                ..Default::default()
            },
        );
        serde_json::to_string(&outcome).unwrap_or_else(|e| failed(mode, e.to_string()))
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    /// Runs a workload in the correctness mode with a deliberate fault in what is observed, to prove that the
    /// comparison notices it. `fault` is one of the names in `executor::Fault`.
    #[wasm_bindgen]
    pub fn selftest_checker(manifest_json: &str, fault: &str) -> String {
        let mode = Mode::Correctness;
        let manifest = match serde_json::from_str(manifest_json)
            .map_err(|e| e.to_string())
            .and_then(|v| Manifest::from_json(v).map_err(|e| e.to_string()))
        {
            Ok(manifest) => manifest,
            Err(message) => return failed(mode, message),
        };
        let fault = match fault {
            "none" => None,
            other => match crate::executor::Fault::parse(other) {
                Some(fault) => Some(fault),
                None => return failed(mode, format!("there is no fault {other}")),
            },
        };
        let outcome = crate::executor::run_with_fault(&manifest, mode, None, fault);
        serde_json::to_string(&outcome).unwrap_or_else(|e| failed(mode, e.to_string()))
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    /// Runs a workload with its last mutation before one of the two executed checkpoints applied wrongly, and returns the
    /// outcome as JSON. `target` is `final`, `penultimate` or `none`. A run that does it has to come out invalid, in the
    /// performance mode as in the correctness mode, which `selftest-final-state.mjs` checks.
    #[wasm_bindgen]
    pub fn selftest_corruption(manifest_json: &str, mode: &str, target: &str) -> String {
        let mode = match mode {
            "correctness" => Mode::Correctness,
            "performance" => Mode::Performance,
            other => return failed(Mode::Performance, format!("the corruption self-test has no mode {other}")),
        };
        let manifest = match serde_json::from_str(manifest_json)
            .map_err(|e| e.to_string())
            .and_then(|v| Manifest::from_json(v).map_err(|e| e.to_string()))
        {
            Ok(manifest) => manifest,
            Err(message) => return failed(mode, message),
        };
        let corrupt = match target {
            "none" => None,
            other => match perf_harness::corruption::Target::parse(other) {
                Some(target) => Some(target),
                None => return failed(mode, format!("there is no corruption target {other}")),
            },
        };
        let outcome = crate::executor::run_with_options(
            &manifest,
            mode,
            None,
            crate::executor::Options { corrupt, ..Default::default() },
        );
        serde_json::to_string(&outcome).unwrap_or_else(|e| failed(mode, e.to_string()))
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    /// Runs a workload with a deliberate delay of `delay_us` microseconds in the harness's own work for every
    /// operation, and returns the outcome as JSON. No timing may show the delay, which the timing self-test checks.
    #[wasm_bindgen]
    pub fn selftest_timing(manifest_json: &str, mode: &str, delay_us: f64) -> String {
        let mode = match mode {
            "performance" => Mode::Performance,
            "endurance" => Mode::Endurance,
            other => return failed(Mode::Performance, format!("the timing self-test has no mode {other}")),
        };
        let manifest = match serde_json::from_str(manifest_json)
            .map_err(|e| e.to_string())
            .and_then(|v| Manifest::from_json(v).map_err(|e| e.to_string()))
        {
            Ok(manifest) => manifest,
            Err(message) => return failed(mode, message),
        };
        let outcome = crate::executor::run_with_options(
            &manifest,
            mode,
            None,
            crate::executor::Options {
                harness_delay_us: delay_us,
                ..Default::default()
            },
        );
        serde_json::to_string(&outcome).unwrap_or_else(|e| failed(mode, e.to_string()))
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    /// Runs a manifest as an endurance run with a deliberate leak in every cycle, and returns the outcome as JSON. `kind` is
    /// `none`, `bytes`, `closure` or `library_watch`, and `bytes` is how many a cycle leaks. The endurance gate has to fail
    /// for every leak and pass for none, which `selftest-endurance.mjs` checks.
    #[wasm_bindgen]
    pub fn selftest_endurance(manifest_json: &str, kind: &str, bytes: u32) -> String {
        let mode = Mode::Endurance;
        let manifest = match serde_json::from_str(manifest_json)
            .map_err(|e| e.to_string())
            .and_then(|v| Manifest::from_json(v).map_err(|e| e.to_string()))
        {
            Ok(manifest) => manifest,
            Err(message) => return failed(mode, message),
        };
        let leak = match kind {
            "none" => None,
            other => match crate::executor::Leak::parse(other, bytes as usize) {
                Some(leak) => Some(leak),
                None => return failed(mode, format!("there is no leak {other}")),
            },
        };
        let outcome = crate::executor::run_with_options(
            &manifest,
            mode,
            None,
            crate::executor::Options { leak, ..Default::default() },
        );
        serde_json::to_string(&outcome).unwrap_or_else(|e| failed(mode, e.to_string()))
    }

    /// Runs a manifest as an endurance run with the allocator tracking every block, and returns what is still live after the
    /// teardown of each of the first `cycles` cycles, by the phase of the harness that allocated it, as JSON.
    #[wasm_bindgen]
    pub fn attribute_persistent_run(manifest_json: &str, cycles: u32) -> String {
        match serde_json::from_str(manifest_json)
            .map_err(|e| e.to_string())
            .and_then(|v| Manifest::from_json(v).map_err(|e| e.to_string()))
        {
            Ok(manifest) => crate::executor::attribute(&manifest, u64::from(cycles)),
            Err(message) => serde_json::json!({"error": message}).to_string(),
        }
    }

    /// Runs `cycles` of one stage of the lifecycle and returns what the allocator counted after each, as JSON. See `persistent`.
    #[wasm_bindgen]
    pub fn attribute_persistent(stage: &str, cycles: u32, before_baseline: &str, nodes: u32) -> String {
        crate::persistent::run(stage, cycles as usize, before_baseline, nodes as usize)
    }

    /// Builds the scene of an interaction manifest at a scale of the view, checks it against the reference model, and says what
    /// it made, as JSON. See `interactive`.
    #[wasm_bindgen]
    pub fn interactive_build(manifest_json: &str, scale_percent: u32) -> String {
        crate::interactive::build(manifest_json, scale_percent)
    }

    /// The view of the scene on the stage, as `{scale, tx, ty}`.
    #[wasm_bindgen]
    pub fn interactive_view() -> String {
        crate::interactive::view()
    }

    /// Where the scene says a node is, as `{x, y}`.
    #[wasm_bindgen]
    pub fn interactive_node(key: &str) -> String {
        crate::interactive::node(key)
    }

    /// Puts the view and the named nodes back, between samples.
    #[wasm_bindgen]
    pub fn interactive_reset(scale_percent: u32, positions_json: &str) -> String {
        crate::interactive::reset(scale_percent, positions_json)
    }

    /// Drops the scene on the stage and removes its host.
    #[wasm_bindgen]
    pub fn interactive_teardown() -> String {
        crate::interactive::teardown()
    }

    /// Drops the scene on the stage and leaves its host in the document.
    #[wasm_bindgen]
    pub fn interactive_drop_scene() -> String {
        crate::interactive::drop_scene()
    }

    /// Whether the host of the stage's scene is in the document.
    #[wasm_bindgen]
    pub fn interactive_host_present() -> bool {
        crate::interactive::host_present()
    }

    /// Builds the scene of a recovery manifest on a slot, with labels on some edges if asked, and says what it made, as JSON.
    #[wasm_bindgen]
    pub fn recovery_build(manifest_json: &str, slot: u32, labelled: bool) -> String {
        crate::recovery::build(manifest_json, slot, labelled)
    }

    /// What the scene on a slot was asked to do, as the reference model has it. See `recovery`.
    #[wasm_bindgen]
    pub fn recovery_intend(slot: u32, op_json: &str) -> String {
        crate::recovery::intend(slot, op_json)
    }

    /// Does one thing to the scene on a slot, which can fail, and says whether it did.
    #[wasm_bindgen]
    pub fn recovery_act(slot: u32, op_json: &str) -> String {
        crate::recovery::act(slot, op_json)
    }

    /// Reads the scene on a slot back from the document, and keeps it under a name.
    #[wasm_bindgen]
    pub fn recovery_snapshot(slot: u32, name: &str) -> String {
        crate::recovery::snapshot(slot, name)
    }

    /// Keeps what the reference model predicts for the scene on a slot, under a name.
    #[wasm_bindgen]
    pub fn recovery_predicted(slot: u32, name: &str) -> String {
        crate::recovery::predicted(slot, name)
    }

    /// The differences between two kept checkpoints, the expected one first.
    #[wasm_bindgen]
    pub fn recovery_differences(expected: &str, actual: &str) -> String {
        crate::recovery::differences(expected, actual)
    }

    /// Where the scene on a slot says a node is.
    #[wasm_bindgen]
    pub fn recovery_node_rect(slot: u32, key: &str) -> String {
        crate::recovery::node_rect(slot, key)
    }

    /// How many nodes and edges the scene on a slot has confirmed.
    #[wasm_bindgen]
    pub fn recovery_counts(slot: u32) -> String {
        crate::recovery::counts(slot)
    }

    /// Drops the scene on a slot and removes its host.
    #[wasm_bindgen]
    pub fn recovery_teardown(slot: u32) -> String {
        crate::recovery::teardown(slot)
    }

    /// Drops the scene on a slot and leaves its host in the document.
    #[wasm_bindgen]
    pub fn recovery_drop_scene(slot: u32) -> String {
        crate::recovery::drop_scene(slot)
    }

    /// Forgets every kept checkpoint.
    #[wasm_bindgen]
    pub fn recovery_forget() {
        crate::recovery::forget();
    }

    /// Starts following every block that is allocated from now on. A diagnostic, for finding what a leak is made of.
    #[wasm_bindgen]
    pub fn tracking_start() {
        crate::alloc::start_tracking();
    }

    /// Stops following new blocks, and says which of the blocks followed are still live, with the start of what each holds, as JSON.
    #[wasm_bindgen]
    pub fn tracking_stop_and_dump(preview: u32) -> String {
        crate::alloc::stop_tracking();
        let blocks = crate::alloc::tracked_live();
        let rows: Vec<serde_json::Value> = blocks
            .iter()
            .map(|block| {
                let length = block.size.min(preview as usize);
                // The block is live, so its first bytes are readable. They are looked at as text and as numbers.
                let bytes = unsafe { std::slice::from_raw_parts(block.address as *const u8, length) };
                serde_json::json!({
                    "size": block.size,
                    "text": String::from_utf8_lossy(bytes).replace(|c: char| c.is_control(), "."),
                    "words": bytes.chunks(4).map(|c| c.iter().rev().fold(0u32, |a, b| (a << 8) | u32::from(*b))).collect::<Vec<_>>(),
                })
            })
            .collect();
        serde_json::json!({ "blocks": rows, "untracked": crate::alloc::untracked(), "growths": crate::alloc::growths() }).to_string()
    }

    /// Holds `count` JavaScript objects at once and then lets them all go, so that wasm-bindgen's table of externrefs, and the list
    /// of its free slots, are as large as they will need to be. A run does this before it reads its baseline, as it sizes its own
    /// vectors, so that the slab doubling is not mistaken for growth that the library caused.
    #[wasm_bindgen]
    pub fn reserve_handles(count: u32) {
        let held: Vec<wasm_bindgen::JsValue> = (0..count).map(|_| js_sys::Object::new().into()).collect();
        std::hint::black_box(&held);
    }

    /// Allocates `bytes` bytes and never frees them. For the self-test of the endurance gate alone.
    #[wasm_bindgen]
    pub fn leak_bytes(bytes: u32) {
        let leaked: &'static mut [u8] = Box::leak(vec![1u8; bytes as usize].into_boxed_slice());
        std::hint::black_box(leaked);
    }

    /// What the counting allocator and the module's memory say now, as JSON: live bytes, live blocks and linear-memory capacity.
    #[wasm_bindgen]
    pub fn resource_reading() -> String {
        let stats = crate::alloc::stats();
        serde_json::json!({
            "live_bytes": stats.live_bytes,
            "live_allocations": stats.live_allocations,
            "capacity": crate::memory::capacity_bytes(),
        })
        .to_string()
    }

    /// The endurance verdict of a series of readings taken after each teardown, as JSON. The points are
    /// `{cycle, measured, live_bytes, capacity}`, and the allowance is the most that live bytes may have grown by.
    #[wasm_bindgen]
    pub fn endurance_verdict(points_json: &str, allowance_bytes: f64) -> String {
        use perf_harness::endurance::{SeriesPoint, Verdict, judge, summarize};
        #[derive(serde::Deserialize)]
        struct Point {
            cycle: u64,
            measured: bool,
            live_bytes: Option<f64>,
            capacity: Option<f64>,
        }
        let points: Vec<SeriesPoint> = match serde_json::from_str::<Vec<Point>>(points_json) {
            Ok(points) => points
                .into_iter()
                .map(|p| SeriesPoint {
                    cycle: p.cycle,
                    measured: p.measured,
                    live_bytes: p.live_bytes,
                    capacity: p.capacity,
                })
                .collect(),
            Err(e) => return serde_json::json!({ "error": e.to_string() }).to_string(),
        };
        let summary = summarize(&points);
        let (status, detail) = match judge(&summary, allowance_bytes) {
            Verdict::Pass(detail) => ("pass", detail),
            Verdict::Fail(detail) => ("fail", detail),
            Verdict::Inconclusive(detail) => ("inconclusive", detail),
        };
        serde_json::json!({
            "status": status,
            "detail": detail,
            "measured_points": summary.measured_points,
            "first_measured_cycle": summary.first_measured_cycle,
            "last_measured_cycle": summary.last_measured_cycle,
            "reference_cycle": summary.reference_cycle,
            "net_live_change": summary.net_live_change,
            "live_slope_per_cycle": summary.live_slope_per_cycle,
            "grow_events": summary.grow_events,
        })
        .to_string()
    }

    /// Describes samples as a latency metric, with the same statistics as the other layers, as JSON. `layer` is `A`, `B` or `C`.
    #[wasm_bindgen]
    pub fn describe_metric(name: &str, layer: &str, unit: &str, samples_json: &str, mechanism: &str) -> String {
        use perf_harness::metrics::{Layer, timing, unavailable};
        let layer = match layer {
            "A" => Layer::A,
            "C" => Layer::C,
            _ => Layer::B,
        };
        let samples: Vec<f64> = serde_json::from_str(samples_json).unwrap_or_default();
        if samples.is_empty() {
            return unavailable(name, layer, "no observation was made").to_string();
        }
        timing(name, layer, unit, &samples, None, mechanism).to_string()
    }

    /// Runs the counting allocator's checks and returns the result as JSON.
    #[wasm_bindgen]
    pub fn verify_allocator() -> String {
        crate::allocator_check::verify()
    }
}
