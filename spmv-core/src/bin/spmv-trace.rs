//! Prints the outcome of scenarios as JSON lines, for the differential run to compare with the oracle's.
//!
//!   spmv-trace panel FROM TO     the panel seeds FROM up to TO, with the default budget
//!   spmv-trace budget            the default budget, so that the oracle uses the same one for the panel
//!   spmv-trace cases PATH        every case of a JSON file, `{"name": {"spec": {...}, "budget": {...}}}`; PATH `-` is standard input
//!
//! It runs natively and under WASI, so that the 32-bit build can be compared with the oracle as well.

use spmv_core::budget::Budget;
use spmv_core::spec::Spec;
use spmv_core::trace::outcome;
use std::io::Read;

fn run() -> Result<(), String> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match args.first().map(String::as_str) {
        Some("panel") => {
            let (from, to) = (parse(args.get(1))?, parse(args.get(2))?);
            for seed in from..to {
                let mut v = outcome(&Spec::panel(seed), &Budget::default());
                v["name"] = format!("panel/{seed}").into();
                println!("{v}");
            }
            Ok(())
        },
        Some("budget") => {
            println!("{}", Budget::default().to_json());
            Ok(())
        },
        Some("cases") => {
            let path = args.get(1).ok_or("PATH is missing")?;
            let mut text = String::new();
            if path == "-" {
                std::io::stdin()
                    .read_to_string(&mut text)
                    .map_err(|e| format!("standard input: {e}"))?;
            } else {
                text = std::fs::read_to_string(path).map_err(|e| format!("{path}: {e}"))?;
            }
            let all: serde_json::Value = serde_json::from_str(&text).map_err(|e| format!("{path}: {e}"))?;
            for (name, case) in all.as_object().ok_or("the cases are not an object")? {
                let spec = Spec::from_json(&case["spec"]);
                let budget = Budget::from_json(case.get("budget").unwrap_or(&serde_json::Value::Null));
                let mut v = match (spec, budget) {
                    (Ok(s), Ok(b)) => outcome(&s, &b),
                    (Err(e), _) | (_, Err(e)) => spmv_core::trace::failure_json(&e),
                };
                v["name"] = name.clone().into();
                println!("{v}");
            }
            Ok(())
        },
        _ => Err("usage: spmv-trace (panel FROM TO | budget | cases PATH)".into()),
    }
}

fn parse(arg: Option<&String>) -> Result<u64, String> {
    arg.ok_or("a number is missing")?
        .parse()
        .map_err(|e| format!("not a number: {e}"))
}

fn main() {
    if let Err(message) = run() {
        eprintln!("spmv-trace: {message}");
        std::process::exit(2);
    }
}
