//! The Layer B runner of the performance harness.
//!
//! It drives a real `svg-dom-graph` scene in a browser, through the library's public API only, times what it does, observes
//! what it drew, and counts what it allocated. The pieces that need a browser are built for wasm only. The counting
//! allocator's arithmetic is plain Rust and is tested natively.

pub mod alloc;
pub mod memory;
pub mod parse;

#[cfg(target_arch = "wasm32")]
pub mod observe;

#[cfg(target_arch = "wasm32")]
pub mod executor;

#[cfg(target_arch = "wasm32")]
pub mod allocator_check;

#[cfg(target_arch = "wasm32")]
mod exports {
    use perf_harness::manifest::{Manifest, Mode};
    use perf_harness::outcome::RunOutcome;
    use wasm_bindgen::prelude::*;

    /// Reports a panic in the page's console, where a bare trap would give no message.
    #[wasm_bindgen(start)]
    fn start() {
        std::panic::set_hook(Box::new(|info| web_sys::console::error_1(&info.to_string().into())));
    }

    fn failed(mode: Mode, message: String) -> String {
        let mut outcome = RunOutcome::new("B", mode);
        outcome.status = "error";
        outcome.error = Some(message);
        serde_json::to_string(&outcome).unwrap_or_else(|_| "{\"status\":\"error\"}".to_string())
    }

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

    /// Runs a workload in the correctness mode with a deliberate fault in what is observed, to prove that the comparison
    /// notices it. `fault` is one of the names in `executor::Fault`.
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

    /// Runs the counting allocator's checks and returns the result as JSON.
    #[wasm_bindgen]
    pub fn verify_allocator() -> String {
        crate::allocator_check::verify()
    }
}
