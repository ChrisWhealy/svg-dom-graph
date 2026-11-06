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

    /// Whether the host of the stage's scene is in the document.
    #[wasm_bindgen]
    pub fn interactive_host_present() -> bool {
        crate::interactive::host_present()
    }

    /// Runs the counting allocator's checks and returns the result as JSON.
    #[wasm_bindgen]
    pub fn verify_allocator() -> String {
        crate::allocator_check::verify()
    }
}
