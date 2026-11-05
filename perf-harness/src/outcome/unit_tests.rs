use super::*;

fn failed() -> FailedCheckpoint {
    FailedCheckpoint {
        section: "topology".into(),
        detail: "a node is missing".into(),
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[test]
fn a_failed_checkpoint_or_a_different_stream_makes_a_run_invalid() -> Result<(), String> {
    let mut clean = RunOutcome::new("B", Mode::Performance);
    clean.settle();
    if clean.status != "ok" {
        return Err(format!("a clean run was {}", clean.status));
    }
    let mut wrong_checkpoint = RunOutcome::new("B", Mode::Performance);
    wrong_checkpoint.checkpoints_failed.push(failed());
    wrong_checkpoint.settle();
    let mut wrong_stream = RunOutcome::new("B", Mode::Performance);
    wrong_stream.stream = Some(ExecutedStream {
        scope: "mutations",
        operations: 3,
        sha256: "a".into(),
        expected_sha256: "b".into(),
        matches: false,
    });
    wrong_stream.settle();
    if wrong_checkpoint.status != "invalid_incorrect" || wrong_stream.status != "invalid_incorrect" {
        return Err(format!("{} and {}", wrong_checkpoint.status, wrong_stream.status));
    }
    Ok(())
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[test]
fn an_error_stays_an_error() -> Result<(), String> {
    let mut run = RunOutcome::new("A", Mode::Correctness);
    run.status = "error";
    run.checkpoints_failed.push(failed());
    run.settle();
    if run.status == "error" { Ok(()) } else { Err(run.status.to_string()) }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[test]
fn a_performance_run_executes_only_the_last_two_checkpoints() -> Result<(), String> {
    let executed: Vec<u64> = (0..6).filter(|index| performance_runs_checkpoint(*index, 6)).collect();
    if executed != [4, 5] {
        return Err(format!("{executed:?}"));
    }
    // With one or two checkpoints in all, every one is executed.
    if !performance_runs_checkpoint(0, 1) || !performance_runs_checkpoint(0, 2) || !performance_runs_checkpoint(1, 2) {
        return Err("a short stream skipped a checkpoint".to_string());
    }
    Ok(())
}
