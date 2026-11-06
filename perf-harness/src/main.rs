//! Command line front end: `perf-harness <command> <manifest.toml>`.
//!
//! * `validate` reads and checks a manifest.
//! * `generate` prints the workload report as JSON, or writes it to `--out`.
//! * `stream` prints the operation stream, one JSON operation per line.
//! * `canonical` prints the manifest in its canonical JSON form, exactly as it is hashed.
//! * `baseline-key` prints the name of the environment a result was measured in, which is where its baseline is kept.
//! * `compare` compares a candidate collection of runs with a baseline one and writes the comparison report.
//! * `native` runs the Layer A runner (`--mode correctness|performance`) and prints its outcome as JSON, or writes it to `--out`.

// Only the native runner reads `--mode`, so a build without it parses the option and has no use for it.
#![cfg_attr(not(feature = "layer-a"), allow(unused_assignments, unused_variables))]

use perf_harness::manifest::Manifest;
use perf_harness::ops::Op;
use perf_harness::report::build_report;
use perf_harness::workload::{WorkloadError, generate};
use std::io::Write;
use std::process::ExitCode;

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.is_empty() {
        return usage();
    }
    match run(&args) {
        Ok(()) => ExitCode::SUCCESS,
        Err(message) => {
            eprintln!("perf-harness: {message}");
            ExitCode::FAILURE
        },
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
fn usage() -> ExitCode {
    eprintln!(
        "usage: perf-harness (validate | stream | canonical | generate [--id NAME] [--out FILE] | gestures [--out FILE] | native [--mode MODE] [--out FILE]) <manifest.toml>"
    );
    ExitCode::from(2)
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Reads a JSON array of numbers from standard input and writes the metric that describes them, so that a runner that is not
/// written in Rust describes its samples with the same statistics as the ones that are.
///
/// `metric --name NAME --layer A|B|C --unit ms|us --mechanism TEXT [--batch N]`
fn metric(args: &[String]) -> Result<(), String> {
    use perf_harness::metrics::{Layer, mark_resolution_limited, timing};
    let (mut name, mut layer, mut unit, mut mechanism, mut batch, mut resolution) =
        (None, None, None, None, None, None);
    let mut args = args.iter();
    while let Some(arg) = args.next() {
        let mut value = || args.next().cloned().ok_or(format!("{arg} needs a value"));
        match arg.as_str() {
            "--name" => name = Some(value()?),
            "--layer" => layer = Some(value()?),
            "--unit" => unit = Some(value()?),
            "--mechanism" => mechanism = Some(value()?),
            "--batch" => batch = Some(value()?.parse::<u32>().map_err(|e| e.to_string())?),
            "--resolution-us" => resolution = Some(value()?.parse::<f64>().map_err(|e| e.to_string())?),
            other => return Err(format!("unknown option {other}")),
        }
    }
    let layer = match layer.as_deref() {
        Some("A") => Layer::A,
        Some("B") => Layer::B,
        Some("C") => Layer::C,
        _ => return Err("--layer must be A, B or C".to_string()),
    };
    let samples: Vec<f64> = serde_json::from_reader(std::io::stdin()).map_err(|e| format!("the samples: {e}"))?;
    let name = name.ok_or("--name is required")?;
    let mut value = timing(
        &name,
        layer,
        &unit.ok_or("--unit is required")?,
        &samples,
        batch,
        &mechanism.ok_or("--mechanism is required")?,
    );
    if let Some(resolution) = resolution {
        mark_resolution_limited(&mut value, resolution);
    }
    println!("{value}");
    Ok(())
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Compares a candidate collection of runs with a baseline one and writes the comparison report.
///
/// `compare --baseline DIR --candidate DIR [--candidate DIR ...] --promotion FILE --layer A|B|C --mode MODE --generated-at TIME
/// [--tolerance 0.1] [--confidence 0.95] [--resamples 4000] [--seed 7] [--min-repetitions 5] [--runs-to-block 10] --out FILE`
///
/// It reports and never judges: the exit status says whether the report was written, and a regression is in the report.
fn compare(args: &[String]) -> Result<(), String> {
    use perf_harness::regression::{
        Settings,
        report::{Context, build_report, load_collection},
    };
    use std::path::PathBuf;
    let (mut baseline, mut candidate) = (Vec::new(), Vec::new());
    let (mut promotion, mut layer, mut mode, mut generated_at, mut out) = (None, None, None, None, None);
    let mut runs_to_block = 10usize;
    let mut settings = Settings::default();
    let mut rest = args.iter();
    while let Some(arg) = rest.next() {
        let mut value = || rest.next().cloned().ok_or(format!("{arg} needs a value"));
        let number = |text: String| text.parse::<f64>().map_err(|e| format!("{arg}: {e}"));
        match arg.as_str() {
            "--baseline" => baseline.push(PathBuf::from(value()?)),
            "--candidate" => candidate.push(PathBuf::from(value()?)),
            "--promotion" => promotion = Some(value()?),
            "--layer" => layer = Some(value()?),
            "--mode" => mode = Some(value()?),
            "--generated-at" => generated_at = Some(value()?),
            "--out" => out = Some(value()?),
            "--tolerance" => settings.tolerance = number(value()?)?,
            "--confidence" => settings.confidence = number(value()?)?,
            "--resamples" => settings.resamples = number(value()?)? as u32,
            "--seed" => settings.seed = number(value()?)? as u64,
            "--min-repetitions" => settings.min_repetitions = number(value()?)? as usize,
            "--runs-to-block" => runs_to_block = number(value()?)? as usize,
            other => return Err(format!("compare has no option {other}")),
        }
    }
    if !(settings.tolerance > 0.0
        && settings.confidence > 0.0
        && settings.confidence < 1.0
        && settings.resamples >= 1000)
    {
        return Err(
            "a tolerance above 0, a confidence between 0 and 1 and at least 1000 resamples are needed".to_string(),
        );
    }
    let (layer, mode) = (layer.ok_or("--layer is required")?, mode.ok_or("--mode is required")?);
    let promotion_file = promotion.ok_or("--promotion is required")?;
    let promotion: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&promotion_file).map_err(|e| format!("{promotion_file}: {e}"))?)
            .map_err(|e| format!("{promotion_file}: {e}"))?;
    let (baseline, candidate) = (
        load_collection(&baseline, &layer, &mode)?,
        load_collection(&candidate, &layer, &mode)?,
    );
    let generated_at = generated_at.ok_or("--generated-at is required")?;
    let context = Context {
        generated_at: &generated_at,
        promotion: &promotion["promotion"],
        runs_to_block,
        layer: &layer,
    };
    let (report, _) = build_report(&baseline, &candidate, &settings, &context)?;
    let json = serde_json::to_string_pretty(&report).map_err(|e| e.to_string())?;
    let file = out.ok_or("--out is required")?;
    std::fs::write(&file, json + "\n").map_err(|e| format!("{file}: {e}"))
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
fn run(args: &[String]) -> Result<(), String> {
    let (command, rest) = args.split_first().ok_or("no command")?;
    if command == "metric" {
        return metric(rest);
    }
    if command == "compare" {
        return compare(rest);
    }
    if command == "baseline-key" {
        let file = rest.first().ok_or("baseline-key needs a result file")?;
        let run = perf_harness::regression::report::load_run(std::path::Path::new(file))?;
        println!("{}", run.key.slug());
        return Ok(());
    }
    let mut path = None;
    let mut id = None;
    let mut out = None;
    let mut mode = None;
    let mut rest = rest.iter();
    while let Some(arg) = rest.next() {
        match arg.as_str() {
            "--id" => id = Some(rest.next().ok_or("--id needs a name")?.clone()),
            "--out" => out = Some(rest.next().ok_or("--out needs a file")?.clone()),
            "--mode" => mode = Some(rest.next().ok_or("--mode needs a name")?.clone()),
            other => path = Some(other.to_string()),
        }
    }
    let path = path.ok_or("no manifest given")?;
    let text = std::fs::read_to_string(&path).map_err(|e| format!("{path}: {e}"))?;
    let manifest = Manifest::from_toml(&text).map_err(|e| e.to_string())?;

    match command.as_str() {
        "validate" => {
            println!("{path}: ok, manifest sha256 {}", manifest.canonical_sha256());
            Ok(())
        },
        "generate" => {
            let id = id.unwrap_or_else(|| {
                std::path::Path::new(&path)
                    .file_stem()
                    .map_or_else(|| "manifest".to_string(), |s| s.to_string_lossy().into())
            });
            let report = build_report(&manifest, &id).map_err(|e| e.to_string())?;
            let json = serde_json::to_string_pretty(&report).map_err(|e| e.to_string())?;
            match out {
                Some(file) => std::fs::write(&file, json + "\n").map_err(|e| format!("{file}: {e}")),
                None => {
                    println!("{json}");
                    Ok(())
                },
            }
        },
        #[cfg(feature = "layer-a")]
        "native" => {
            use perf_harness::manifest::Mode;
            let mode = match mode.as_deref().unwrap_or("performance") {
                "correctness" => Mode::Correctness,
                "performance" => Mode::Performance,
                other => return Err(format!("the native runner has no mode {other}")),
            };
            let outcome = perf_harness::layer_a::run(&manifest, mode).map_err(|e| e.to_string())?;
            let json = serde_json::to_string_pretty(&outcome).map_err(|e| e.to_string())?;
            match out {
                Some(file) => std::fs::write(&file, json + "\n").map_err(|e| format!("{file}: {e}")),
                None => {
                    println!("{json}");
                    Ok(())
                },
            }
        },
        "gestures" => {
            let plan = perf_harness::gestures::plan(&manifest).map_err(|e| e.to_string())?;
            let json = serde_json::to_string(&serde_json::json!({"sha256": plan.sha256(), "plan": plan}))
                .map_err(|e| e.to_string())?;
            match out {
                Some(file) => std::fs::write(&file, json + "\n").map_err(|e| format!("{file}: {e}")),
                None => {
                    println!("{json}");
                    Ok(())
                },
            }
        },
        "canonical" => {
            println!("{}", String::from_utf8(manifest.canonical_json()).map_err(|e| e.to_string())?);
            Ok(())
        },
        "stream" => {
            let stdout = std::io::stdout();
            let mut lock = std::io::BufWriter::new(stdout.lock());
            generate(&manifest, &mut |op: &Op| {
                serde_json::to_writer(&mut lock, op).map_err(|e| e.to_string())?;
                lock.write_all(b"\n").map_err(|e| e.to_string())
            })
            .map_err(|e: WorkloadError| e.to_string())?;
            lock.flush().map_err(|e| e.to_string())
        },
        _ => Err(format!("unknown command {command}")),
    }
}
