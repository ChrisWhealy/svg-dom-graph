use super::*;

fn manifest(workload: &str, nodes: u32, edges: u32, warmup: u32, iterations: u32) -> Manifest {
    let text = format!(
        r#"
        schema_version = 1
        workload = "{workload}"
        seed = 99
        prng = "splitmix64"
        [graph]
        nodes = {nodes}
        edges = {edges}
        maximum_degree = 8
        [execution]
        modes = ["correctness", "performance"]
        warmup_iterations = {warmup}
        measurement_iterations = {iterations}
        repetitions = 1
        [measurement]
        capture_dom_counts = true
        capture_listener_tally = true
        capture_heap = false
        capture_wasm_capacity = true
        capture_wasm_live_allocation = true
        capture_frame_timing = false
        checkpoint_every = 5
        [checkpoint]
        view = "unchanged"
        [limits]
        wall_ms = 60000
        "#
    );
    Manifest::from_toml(&text).expect("the test manifest is valid")
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The library's own `boundary_point` and the model's separate implementation of the rule have to agree at every
/// checkpoint of every workload. This is the check that the reference model's geometry is the library's geometry.
#[test]
fn the_librarys_geometry_agrees_with_the_model_at_every_checkpoint() -> Result<(), String> {
    for workload in ["scene_construction", "connector_rerouting", "scene_lifecycle"] {
        let outcome = run(&manifest(workload, 30, 60, 3, 20), Mode::Correctness).map_err(|e| e.to_string())?;
        if outcome.status != "ok" || !outcome.checkpoints_failed.is_empty() {
            return Err(format!("{workload}: {} {:?}", outcome.status, outcome.checkpoints_failed));
        }
        if outcome.checkpoints_passed == 0 {
            return Err(format!("{workload}: no checkpoint was checked"));
        }
    }
    Ok(())
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[test]
fn a_correctness_run_executes_the_whole_stream_and_a_performance_run_the_mutations() -> Result<(), String> {
    let m = manifest("connector_rerouting", 20, 40, 2, 12);
    let report = build_report(&m, "t").map_err(|e| e.to_string())?;
    let correctness = run(&m, Mode::Correctness).map_err(|e| e.to_string())?;
    let performance = run(&m, Mode::Performance).map_err(|e| e.to_string())?;
    let (c, p) = (correctness.stream.ok_or("no stream")?, performance.stream.ok_or("no stream")?);
    if c.scope != "full" || c.sha256 != report.stream.sha256 || c.operations != report.stream.operations {
        return Err(format!("correctness executed {c:?}"));
    }
    if p.scope != "mutations" || p.sha256 != report.stream.mutations_sha256 || !p.matches {
        return Err(format!("performance executed {p:?}"));
    }
    Ok(())
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[test]
fn a_performance_run_of_rerouting_times_the_moves_and_says_how() -> Result<(), String> {
    let outcome =
        run(&manifest("connector_rerouting", 40, 100, 2, 60), Mode::Performance).map_err(|e| e.to_string())?;
    let metric = outcome
        .metrics
        .iter()
        .find(|m| m["name"] == "rust.connector_endpoints")
        .ok_or("no metric")?;
    if metric["status"] != "measured" || metric["layer"] != "A" || metric["unit"] != "us" {
        return Err(format!("{metric}"));
    }
    let samples = outcome.samples.get("rust.connector_endpoints").ok_or("no samples kept")?;
    if samples.is_empty() || samples.iter().any(|s| s.is_nan() || *s <= 0.0) {
        return Err(format!("{} samples, some not positive", samples.len()));
    }
    if metric["samples"] != samples.len() || outcome.timer_resolution_us.is_nan() || outcome.timer_resolution_us <= 0.0
    {
        return Err(format!("{metric} with a timer of {} us", outcome.timer_resolution_us));
    }
    Ok(())
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[test]
fn the_other_workloads_and_the_correctness_mode_report_no_timing_and_say_why() -> Result<(), String> {
    for (workload, mode) in [
        ("scene_construction", Mode::Performance),
        ("scene_lifecycle", Mode::Performance),
        ("connector_rerouting", Mode::Correctness),
    ] {
        let outcome = run(&manifest(workload, 12, 20, 1, 4), mode).map_err(|e| e.to_string())?;
        let metric = outcome
            .metrics
            .iter()
            .find(|m| m["name"] == "rust.connector_endpoints")
            .ok_or("no metric")?;
        if metric["status"] != "unavailable" || metric.get("reason").is_none() || !outcome.samples.is_empty() {
            return Err(format!("{workload} {mode:?}: {metric}"));
        }
    }
    Ok(())
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[test]
fn the_native_timer_has_a_resolution() -> Result<(), String> {
    let resolution = measure_resolution();
    if resolution > 0.0 && resolution < 1000.0 {
        Ok(())
    } else {
        Err(format!("{resolution} us"))
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[test]
fn the_remaining_modes_are_refused() -> Result<(), String> {
    for mode in [Mode::Recovery, Mode::Endurance] {
        if run(&manifest("connector_rerouting", 12, 20, 1, 4), mode).is_ok() {
            return Err(format!("{mode:?} was accepted"));
        }
    }
    Ok(())
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// If the compiler computed the repeats once and reused the answer, the time would not depend on how many connectors
/// there are. A node with ten times the connectors has to take clearly longer.
#[test]
fn the_time_scales_with_the_connectors() -> Result<(), String> {
    let build = |edges: usize| {
        let mut state = State::default();
        for index in 0..=edges {
            state.names.push(format!("n{index}"));
            state.boxes.push((index as f64 * 100.0, (index % 7) as f64 * 50.0, 80.0, 36.0));
            state.incident.push(Vec::new());
        }
        for target in 1..=edges {
            state.edges.push((0, target));
            state.incident[0].push(target - 1);
            state.incident[target].push(target - 1);
        }
        state
    };
    let (few, many) = (build(2), build(40));
    // The best of several attempts, since a pause in the machine only ever makes one slower.
    let best = |state: &State| (0..7).map(|_| time_endpoints(state, 0, 20_000)).fold(f64::INFINITY, f64::min);
    let (small, large) = (best(&few), best(&many));
    if large < small * 4.0 {
        return Err(format!(
            "2 connectors took {small} us and 40 took {large} us: the work is not being repeated"
        ));
    }
    Ok(())
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// A performance run executes only the last two checkpoints, and they have to vouch for the end of the measured work.
/// So a mutation that is applied wrongly just before either of them, in any workload and either mode, must make the run
/// invalid. The stream's digest is untouched by it, since the operations are as generated, so it is the checkpoint that
/// notices.
#[test]
fn a_wrong_final_mutation_invalidates_the_run() -> Result<(), String> {
    for workload in ["scene_construction", "connector_rerouting", "scene_lifecycle"] {
        for mode in [Mode::Performance, Mode::Correctness] {
            let m = manifest(workload, 30, 60, 3, 20);
            let clean = run_corrupted(&m, mode, None).map_err(|e| e.to_string())?;
            if clean.status != "ok" {
                return Err(format!("{workload} {mode:?}: a clean run is {}", clean.status));
            }
            for target in [Target::Penultimate, Target::Final] {
                let outcome = run_corrupted(&m, mode, Some(target)).map_err(|e| e.to_string())?;
                let stream = outcome.stream.as_ref().ok_or("no stream")?;
                if outcome.status != "invalid_incorrect" || outcome.checkpoints_failed.is_empty() || !stream.matches {
                    return Err(format!(
                        "{workload} {mode:?} {target:?}: {}, {} checkpoints failed, stream matches {}",
                        outcome.status,
                        outcome.checkpoints_failed.len(),
                        stream.matches
                    ));
                }
            }
        }
    }
    Ok(())
}
