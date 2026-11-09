use super::*;
use crate::manifest::ViewPolicy;
use crate::model::Model;
use crate::outcome::performance_runs_checkpoint;

fn manifest(workload: &str, nodes: u32, edges: u32, warmup: u32, iterations: u32, every: Option<u32>) -> Manifest {
    let every = every.map_or(String::new(), |n| format!("checkpoint_every = {n}"));
    let modes = if every.is_empty() {
        r#"["performance"]"#
    } else {
        r#"["correctness", "performance"]"#
    };
    let text = format!(
        r#"
        schema_version = 1
        workload = "{workload}"
        seed = 5
        prng = "splitmix64"
        [graph]
        nodes = {nodes}
        edges = {edges}
        maximum_degree = 8
        [execution]
        modes = {modes}
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
        {every}
        [checkpoint]
        view = "unchanged"
        [limits]
        wall_ms = 1000
        "#
    );
    Manifest::from_toml(&text).expect("the test manifest is valid")
}

/// Every workload, with checkpoint intervals that divide the run, that do not, and that are absent, and with and
/// without a warm-up. The last two checkpoints must vouch for the end of the measured work however the run is cut up.
fn manifests() -> Vec<Manifest> {
    let mut all = Vec::new();
    for workload in ["scene_construction", "connector_rerouting", "scene_lifecycle"] {
        for (warmup, iterations, every) in [
            (0, 6, Some(3)),
            (2, 9, Some(3)),
            (2, 10, Some(3)),
            (3, 7, Some(5)),
            (2, 8, None),
            (0, 1, None),
        ] {
            all.push(manifest(workload, 12, 20, warmup, iterations, every));
        }
    }
    all
}

fn stream(manifest: &Manifest) -> Result<Vec<Op>, String> {
    let mut ops = Vec::new();
    generate(manifest, &mut |op: &Op| {
        ops.push(op.clone());
        Ok(())
    })
    .map_err(|e| e.to_string())?;
    Ok(ops)
}

/// The positions of the checkpoints that a performance run executes.
fn executed(ops: &[Op]) -> Vec<usize> {
    let total = ops.iter().filter(|op| matches!(op, Op::Checkpoint { .. })).count() as u64;
    ops.iter()
        .enumerate()
        .filter(|(_, op)| matches!(op, Op::Checkpoint { index } if performance_runs_checkpoint(*index, total)))
        .map(|(position, _)| position)
        .collect()
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The requirement itself: nothing that changes the scene may come after the last checkpoint that a performance run
/// executes, and that checkpoint must be the last operation, so that the state it vouches for is the end state.
#[test]
fn a_performance_run_executes_a_checkpoint_after_every_mutation() -> Result<(), String> {
    for manifest in manifests() {
        let ops = stream(&manifest)?;
        let run = executed(&ops);
        let last = *run.last().ok_or("no checkpoint is executed")?;
        let name = format!("{:?}, {} iterations", manifest.workload, manifest.total_iterations());
        if last != ops.len() - 1 {
            return Err(format!("{name}: the last executed checkpoint is not the last operation"));
        }
        if run.len() != 2 && ops.iter().filter(|op| matches!(op, Op::Checkpoint { .. })).count() >= 2 {
            return Err(format!("{name}: {} checkpoints are executed, not 2", run.len()));
        }
        let last_measured = ops.iter().rposition(|op| op.is_mutation()).ok_or("no mutation in the stream")?;
        if last_measured > last {
            return Err(format!("{name}: a mutation follows the last executed checkpoint"));
        }
    }
    Ok(())
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// For the cycle workloads the two executed checkpoints are a completed scene and the empty page, and for rerouting
/// they are the built scene after the whole movement sequence and an earlier state of the same scene.
#[test]
fn the_executed_checkpoints_are_the_completed_scene_and_the_final_state() -> Result<(), String> {
    for manifest in manifests() {
        let ops = stream(&manifest)?;
        let run = executed(&ops);
        let (penultimate, last) = (run[run.len() - 2], run[run.len() - 1]);
        let (nodes, edges) = (manifest.graph.nodes as usize, manifest.graph.edges.unwrap_or(0) as usize);
        let state_at = |position: usize| -> Result<(usize, usize), String> {
            let mut model = Model::new();
            for op in &ops[..position] {
                model.apply(op)?;
            }
            let checkpoint = model.checkpoint(ViewPolicy::Unchanged)?;
            Ok((checkpoint.topology.nodes.len(), checkpoint.topology.edges.len()))
        };
        let name = format!("{:?}, {} iterations", manifest.workload, manifest.total_iterations());
        match manifest.workload {
            crate::manifest::Workload::ConnectorRerouting => {
                if state_at(penultimate)? != (nodes, edges) || state_at(last)? != (nodes, edges) {
                    return Err(format!("{name}: a rerouting checkpoint does not see the whole graph"));
                }
                // Every move comes before the last checkpoint, which therefore sees the position that the last move
                // gave.
                let moves_after_penultimate = ops[penultimate..last]
                    .iter()
                    .filter(|op| matches!(op, Op::MoveNode { .. }))
                    .count();
                if manifest.measurement.checkpoint_every.is_none() && moves_after_penultimate == 0 {
                    return Err(format!(
                        "{name}: no move lies between the two checkpoints, so the last was not checked after the moves"
                    ));
                }
            },
            _ => {
                if state_at(penultimate)? != (nodes, edges) {
                    return Err(format!(
                        "{name}: the completed scene is not complete, {:?}",
                        state_at(penultimate)?
                    ));
                }
                if state_at(last)? != (0, 0) {
                    return Err(format!("{name}: the final state is not empty, {:?}", state_at(last)?));
                }
            },
        }
    }
    Ok(())
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The mutation that is located is the last one before the checkpoint, and nothing that changes the scene lies between.
#[test]
fn the_corruption_is_the_last_mutation_before_its_checkpoint() -> Result<(), String> {
    for manifest in manifests() {
        let ops = stream(&manifest)?;
        let run = executed(&ops);
        for (target, checkpoint) in [(Target::Penultimate, run[run.len() - 2]), (Target::Final, run[run.len() - 1])] {
            let found = locate(&manifest, target).map_err(|e| e.to_string())?;
            let ordinal = found.ordinal as usize;
            if !ops[ordinal].is_mutation() || ops[ordinal] != found.operation {
                return Err(format!("{target:?}: the operation at {ordinal} is {:?}", ops[ordinal]));
            }
            if ops[ordinal + 1..checkpoint].iter().any(Op::is_mutation) {
                return Err(format!(
                    "{target:?}: a mutation comes between operation {ordinal} and its checkpoint"
                ));
            }
        }
    }
    Ok(())
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[test]
fn the_operation_before_each_checkpoint_is_the_one_the_workload_ends_with() -> Result<(), String> {
    let kind = |workload, target| -> Result<&'static str, String> {
        let found = locate(&manifest(workload, 12, 20, 1, 6, Some(3)), target).map_err(|e| e.to_string())?;
        Ok(match found.operation {
            Op::MoveNode { .. } => "move",
            Op::AddEdge { .. } => "edge",
            Op::RemoveHost => "remove",
            other => return Err(format!("{other:?}")),
        })
    };
    for (workload, penultimate, last) in [
        ("scene_construction", "edge", "remove"),
        ("scene_lifecycle", "edge", "remove"),
        ("connector_rerouting", "move", "move"),
    ] {
        let found = (kind(workload, Target::Penultimate)?, kind(workload, Target::Final)?);
        if found != (penultimate, last) {
            return Err(format!("{workload}: {found:?}"));
        }
    }
    Ok(())
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[test]
fn the_shortest_run_has_both_checkpoints_and_a_target_name_is_exact() -> Result<(), String> {
    locate(&manifest("scene_construction", 3, 2, 0, 1, None), Target::Penultimate).map_err(|e| e.to_string())?;
    if Target::parse("final") != Some(Target::Final) || Target::parse("last").is_some() {
        return Err("Target::parse".to_string());
    }
    Ok(())
}
