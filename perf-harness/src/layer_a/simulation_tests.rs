use super::*;

const MANIFEST: &str = include_str!("../../../perf/manifests/w6-service-simulation-smoke.toml");

/// The trace as the engine itself works it out. It stands in for the independent engine's trace, which the orchestrator
/// supplies: the runner's own mechanics are what is tested here, and whether the two engines agree is the
/// orchestrator's and the S1 differential run's.
fn own_trace(seed: u64) -> Expected {
    let mut engine = engine_for(seed, None).expect("an engine");
    let mut checkpoints = vec![engine.checkpoint(0, 0)];
    while let Some(record) = engine.step().expect("a step") {
        checkpoints.push(record.checkpoint);
    }
    let counters = engine.counters();
    Expected {
        checkpoints,
        pops: counters.pops,
        stale: counters.stale,
        slices: counters.slices,
        warmup_starts_at: 257,
    }
}

fn manifest() -> Manifest {
    Manifest::from_toml(MANIFEST).expect("a manifest")
}

#[test]
fn a_run_that_agrees_with_the_trace_is_valid_in_both_modes() {
    let m = manifest();
    let expected = own_trace(m.seed);
    let correctness = run(&m, Mode::Correctness, &expected).expect("a run");
    assert_eq!(correctness.status, "ok");
    assert_eq!(
        correctness.checkpoints_passed,
        expected.slices + 1,
        "a checkpoint at the start and for each slice"
    );
    assert!(correctness.stream.as_ref().expect("a stream").matches);
    let performance = run(&m, Mode::Performance, &expected).expect("a run");
    assert_eq!(performance.status, "ok");
    let stream = performance.stream.as_ref().expect("a stream");
    assert_eq!((stream.scope, stream.operations, stream.matches), ("mutations", 536, true));
    assert_eq!(
        performance.samples["simulation.engine.four_slices"].len(),
        60,
        "sixty measured observations"
    );
    assert_eq!(performance.cases.len(), 60, "and a record for each");
}

#[test]
fn a_wrong_checkpoint_makes_the_run_invalid_and_names_the_slice() {
    let m = manifest();
    for mode in [Mode::Correctness, Mode::Performance] {
        let mut expected = own_trace(m.seed);
        expected.checkpoints[300] = "0".repeat(64);
        let outcome = run(&m, mode, &expected).expect("a run");
        assert_eq!(outcome.status, "invalid_incorrect", "{mode:?}");
        assert!(
            outcome.checkpoints_failed.iter().any(|f| f.detail.starts_with("slice 300:")),
            "{mode:?}: {:?}",
            outcome.checkpoints_failed
        );
        assert!(
            !outcome.stream.as_ref().expect("a stream").matches,
            "{mode:?}: and the digest says so"
        );
    }
}

#[test]
fn a_wrong_count_is_a_failed_checkpoint() {
    let m = manifest();
    let mut expected = own_trace(m.seed);
    expected.stale += 1;
    let outcome = run(&m, Mode::Correctness, &expected).expect("a run");
    assert_eq!(outcome.status, "invalid_incorrect");
    assert!(outcome.checkpoints_failed.iter().any(|f| f.detail.starts_with("stale pops:")));
}

#[test]
fn a_trace_too_short_for_the_windows_is_refused_before_any_timing() {
    let m = manifest();
    let mut expected = own_trace(m.seed);
    expected.slices = 535;
    let refusal = run(&m, Mode::Performance, &expected).expect_err("a refusal");
    assert!(refusal.contains("refused before any timing"), "{refusal}");
}

#[test]
fn every_batch_records_four_consecutive_slices_and_its_popped_events() {
    let m = manifest();
    let expected = own_trace(m.seed);
    let outcome = run(&m, Mode::Performance, &expected).expect("a run");
    let mut next = 297_u64;
    for case in &outcome.cases {
        let slices: Vec<u64> = case["slices"]
            .as_array()
            .expect("slices")
            .iter()
            .map(|s| s.as_u64().expect("a slice"))
            .collect();
        assert_eq!(
            slices,
            (next..next + 4).collect::<Vec<_>>(),
            "the measured window is consumed once, in order"
        );
        assert!(
            case["pops"].as_u64().expect("pops") >= 4,
            "a nonempty slice pops at least one event"
        );
        assert!(case["stale_pops"].as_u64().expect("stale") <= case["pops"].as_u64().expect("pops"));
        assert_eq!(case["end_checkpoint"], json!(expected.checkpoints[(next + 3) as usize]));
        next += 4;
    }
    assert_eq!(next, 537, "and it ends at slice 536");
}

#[test]
fn the_expected_trace_is_read_from_the_orchestrators_file() {
    let good = json!({"checkpoints": ["a", "b"], "counters": {"pops": 3, "stale": 1, "slices": 1}, "windows": {"warmup": [257, 296]}});
    let expected = Expected::from_json(&good).expect("a trace");
    assert_eq!(
        (expected.checkpoints.len(), expected.pops, expected.stale, expected.slices),
        (2, 3, 1, 1)
    );
    assert!(Expected::from_json(&json!({"checkpoints": ["a"], "counters": {"pops": 1}})).is_err());
    assert!(Expected::from_json(&json!({"counters": {"pops": 1, "stale": 0, "slices": 0}})).is_err());
    assert!(
        Expected::from_json(&json!({"checkpoints": ["a"], "counters": {"pops": 1, "stale": 0, "slices": 0}})).is_err(),
        "a trace that does not say where the warm-up window is, is not one that a window can be held to"
    );
}
