//! Prints the trace of scenarios as JSON, one per line, for the differential run.
//!
//!   dess-trace panel FROM TO        the panel seeds FROM to TO - 1
//!   dess-trace panel2 FROM TO       the disturbed panel: the same seeds with a script added
//!   dess-trace manifest             the frozen revision `mixed_outage/7` for the manifest seed 20261106
//!   dess-trace fixtures PATH        every fixture of the file, as `{"name": ..., "trace": ...}`. PATH `-` reads standard input
//!
//! It runs natively and under WASI. The harness's WASI runner preopens no directory, so the differential run gives the fixtures on
//! standard input.

use service_sim_engine::trace::{Scenario, trace};
use service_sim_engine::{Error, Limits};
use std::process::ExitCode;

fn run() -> Result<(), String> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let num = |s: Option<&String>, what: &str| -> Result<u64, String> {
        s.ok_or(format!("{what} is missing"))?
            .parse::<u64>()
            .map_err(|e| format!("{what}: {e}"))
    };
    let show = |e: Error| e.to_string();
    match args.first().map(String::as_str) {
        Some("panel") => {
            let (from, to) = (num(args.get(1), "FROM")?, num(args.get(2), "TO")?);
            for seed in from..to {
                let scenario = Scenario::panel(seed).map_err(show)?;
                println!("{}", trace(&scenario, Limits::default()).map_err(show)?);
            }
            Ok(())
        },
        Some("panel2") => {
            let (from, to) = (num(args.get(1), "FROM")?, num(args.get(2), "TO")?);
            for seed in from..to {
                let scenario = Scenario::panel2(seed).map_err(show)?;
                println!("{}", trace(&scenario, Limits::default()).map_err(show)?);
            }
            Ok(())
        },
        Some("manifest") => {
            let scenario = Scenario::mixed_outage_7(20_261_106).map_err(show)?;
            println!("{}", trace(&scenario, Limits::default()).map_err(show)?);
            Ok(())
        },
        Some("fixtures") => {
            let path = args.get(1).ok_or("PATH is missing")?;
            let mut text = String::new();
            if path == "-" {
                std::io::Read::read_to_string(&mut std::io::stdin(), &mut text)
                    .map_err(|e| format!("standard input: {e}"))?;
            } else {
                text = std::fs::read_to_string(path).map_err(|e| format!("{path}: {e}"))?;
            }
            let all: serde_json::Value = serde_json::from_str(&text).map_err(|e| format!("{path}: {e}"))?;
            for (name, fixture) in all.as_object().ok_or("the fixtures are not an object")? {
                let scenario = Scenario::fixture(name, fixture).map_err(show)?;
                let t = trace(&scenario, Limits::default()).map_err(show)?;
                println!("{}", serde_json::json!({ "name": name, "trace": t }));
            }
            Ok(())
        },
        _ => Err("usage: dess-trace (panel FROM TO | panel2 FROM TO | manifest | fixtures PATH)".into()),
    }
}

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(message) => {
            eprintln!("dess-trace: {message}");
            ExitCode::FAILURE
        },
    }
}
