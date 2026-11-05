use super::*;
use crate::manifest::Manifest;

/// A manifest for tests, with the numbers that matter as arguments.
fn manifest(
    workload: &str,
    nodes: u32,
    edges: u32,
    degree: u32,
    warmup: u32,
    iterations: u32,
    every: Option<u32>,
) -> Manifest {
    // The schema asks for a checkpoint interval whenever the correctness mode is one of the modes.
    let modes = if every.is_some() {
        r#"["correctness", "performance"]"#
    } else {
        r#"["performance"]"#
    };
    let every = every.map_or(String::new(), |n| format!("checkpoint_every = {n}"));
    let text = format!(
        r#"
        schema_version = 1
        workload = "{workload}"
        seed = 7
        prng = "splitmix64"
        [graph]
        nodes = {nodes}
        edges = {edges}
        maximum_degree = {degree}
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

fn stream(manifest: &Manifest) -> Result<Vec<Op>, WorkloadError> {
    let mut ops = Vec::new();
    generate(manifest, &mut |op| {
        ops.push(op.clone());
        Ok(())
    })?;
    Ok(ops)
}

fn plan(seed: u64, nodes: u32, edges: u32, degree: Option<u32>) -> Result<GraphPlan, WorkloadError> {
    plan_graph(&mut SplitMix64::new(seed), nodes, edges, degree)
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[test]
fn nodes_sit_on_a_grid_that_does_not_make_them_overlap() -> Result<(), String> {
    let planned = plan(1, 10, 0, None).map_err(|e| e.to_string())?;
    // Ten nodes need four columns, since 3 x 3 is too few.
    let positions: Vec<(i64, i64)> = planned.nodes.iter().map(|n| (n.x, n.y)).collect();
    if positions[0] != (20, 20) || positions[3] != (20 + 3 * 140, 20) || positions[4] != (20, 20 + 90) {
        return Err(format!("unexpected grid: {positions:?}"));
    }
    if (planned.field_w, planned.field_h) != (20 + 4 * 140, 20 + 3 * 90) {
        return Err(format!("unexpected field {} x {}", planned.field_w, planned.field_h));
    }
    for (index, a) in positions.iter().enumerate() {
        for b in &positions[index + 1..] {
            if (a.0 - b.0).abs() < NODE_W && (a.1 - b.1).abs() < NODE_H {
                return Err(format!("{a:?} and {b:?} overlap"));
            }
        }
    }
    Ok(())
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[test]
fn edges_are_distinct_never_self_loops_and_within_the_degree_limit() -> Result<(), String> {
    let planned = plan(3, 40, 90, Some(6)).map_err(|e| e.to_string())?;
    if planned.edges.len() != 90 {
        return Err(format!("{} edges", planned.edges.len()));
    }
    let mut degree = vec![0u32; 40];
    let mut seen = std::collections::BTreeSet::new();
    for &(from, to) in &planned.edges {
        if from == to {
            return Err(format!("a self-loop on {from}"));
        }
        if !seen.insert((from, to)) {
            return Err(format!("{from}->{to} twice"));
        }
        degree[from] += 1;
        degree[to] += 1;
    }
    if degree.iter().any(|d| *d > 6) {
        return Err(format!("a degree over 6: {degree:?}"));
    }
    let top = *degree.iter().max().unwrap();
    let first_top = degree.iter().position(|d| *d == top).unwrap();
    if planned.hub != first_top {
        return Err(format!(
            "the hub is {} but the first node of degree {top} is {first_top}",
            planned.hub
        ));
    }
    Ok(())
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[test]
fn a_graph_that_cannot_be_placed_is_an_error_and_not_a_hang() -> Result<(), String> {
    // Five nodes have 20 ordered pairs, but a degree limit of 3 allows only 7 edges.
    match plan(1, 5, 20, Some(3)) {
        Err(WorkloadError::Plan(_)) => Ok(()),
        other => Err(format!("expected a planning error, got {other:?}")),
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[test]
fn the_same_manifest_gives_the_same_stream_and_another_seed_does_not() -> Result<(), String> {
    for workload in ["scene_construction", "connector_rerouting", "scene_lifecycle"] {
        let m = manifest(workload, 12, 20, 8, 2, 5, Some(2));
        if stream(&m).map_err(|e| e.to_string())? != stream(&m).map_err(|e| e.to_string())? {
            return Err(format!("{workload} gave two streams"));
        }
        let mut other = m.clone();
        other.seed += 1;
        if stream(&other).map_err(|e| e.to_string())? == stream(&m).map_err(|e| e.to_string())? {
            return Err(format!("{workload} ignored its seed"));
        }
    }
    Ok(())
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[test]
fn rerouting_builds_once_then_moves_inside_the_field() -> Result<(), String> {
    let m = manifest("connector_rerouting", 12, 20, 8, 3, 7, Some(4));
    let ops = stream(&m).map_err(|e| e.to_string())?;
    let kinds = |pick: fn(&Op) -> bool| ops.iter().filter(|op| pick(op)).count();
    if kinds(|op| matches!(op, Op::CreateScene)) != 1 || kinds(|op| matches!(op, Op::AddNode { .. })) != 12 {
        return Err("W2 must create one scene with 12 nodes".to_string());
    }
    if kinds(|op| matches!(op, Op::AddEdge { .. })) != 20 || kinds(|op| matches!(op, Op::MoveNode { .. })) != 10 {
        return Err("W2 must add 20 edges and make 3 + 7 moves".to_string());
    }
    if kinds(|op| matches!(op, Op::DropScene | Op::RemoveHost)) != 0 {
        return Err("W2 must leave its scene in place".to_string());
    }
    let planned = plan_for(&m, STREAM_GRAPH).map_err(|e| e.to_string())?;
    for op in &ops {
        if let Op::MoveNode { x, y, .. } = op {
            if *x < 0 || *y < 0 || *x + NODE_W > planned.field_w || *y + NODE_H > planned.field_h {
                return Err(format!(
                    "a move to {x},{y} leaves the field {} x {}",
                    planned.field_w, planned.field_h
                ));
            }
        }
    }
    Ok(())
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[test]
fn the_warm_up_and_measured_markers_sit_at_the_right_places() -> Result<(), String> {
    let m = manifest("connector_rerouting", 6, 8, 8, 3, 4, None);
    let ops = stream(&m).map_err(|e| e.to_string())?;
    let mut moves_seen = 0;
    let mut markers = Vec::new();
    for op in &ops {
        match op {
            Op::MoveNode { .. } => moves_seen += 1,
            Op::Phase { phase } => markers.push((*phase, moves_seen)),
            _ => {},
        }
    }
    if markers != vec![(Phase::Warmup, 0), (Phase::Measure, 3)] {
        return Err(format!("markers {markers:?}"));
    }
    let no_warmup = manifest("connector_rerouting", 6, 8, 8, 0, 4, None);
    let ops = stream(&no_warmup).map_err(|e| e.to_string())?;
    let phases: Vec<Phase> = ops
        .iter()
        .filter_map(|op| if let Op::Phase { phase } = op { Some(*phase) } else { None })
        .collect();
    if phases != vec![Phase::Measure] {
        return Err(format!("with no warm-up the markers were {phases:?}"));
    }
    Ok(())
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
fn checkpoint_positions(ops: &[Op]) -> Vec<(u64, usize)> {
    let mut moves = 0;
    let mut found = Vec::new();
    for op in ops {
        match op {
            Op::MoveNode { .. } => moves += 1,
            Op::Checkpoint { index } => found.push((*index, moves)),
            _ => {},
        }
    }
    found
}

#[test]
fn checkpoints_come_at_the_interval_and_at_the_end_without_a_repeat() -> Result<(), String> {
    // 10 moves, every 3: after 0 (the build), 3, 6, 9, and a last one after the 10th.
    let uneven = stream(&manifest("connector_rerouting", 6, 8, 8, 0, 10, Some(3))).map_err(|e| e.to_string())?;
    if checkpoint_positions(&uneven) != vec![(0, 0), (1, 3), (2, 6), (3, 9), (4, 10)] {
        return Err(format!("{:?}", checkpoint_positions(&uneven)));
    }
    // 10 moves, every 5: the one after the 10th move is the last, and is not repeated.
    let even = stream(&manifest("connector_rerouting", 6, 8, 8, 0, 10, Some(5))).map_err(|e| e.to_string())?;
    if checkpoint_positions(&even) != vec![(0, 0), (1, 5), (2, 10)] {
        return Err(format!("{:?}", checkpoint_positions(&even)));
    }
    // With no interval, only the build and the end.
    let none = stream(&manifest("connector_rerouting", 6, 8, 8, 0, 10, None)).map_err(|e| e.to_string())?;
    if checkpoint_positions(&none) != vec![(0, 0), (1, 10)] {
        return Err(format!("{:?}", checkpoint_positions(&none)));
    }
    Ok(())
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The graph a cycle builds, as the nodes and edges of its `AddNode` and `AddEdge` operations.
fn graphs(ops: &[Op]) -> Vec<Vec<Op>> {
    let mut all: Vec<Vec<Op>> = Vec::new();
    for op in ops {
        match op {
            Op::CreateScene => all.push(Vec::new()),
            Op::AddNode { .. } | Op::AddEdge { .. } => {
                all.last_mut().expect("a build follows a create").push(op.clone())
            },
            _ => {},
        }
    }
    all
}

#[test]
fn construction_repeats_one_graph_and_lifecycle_builds_a_new_one_each_cycle() -> Result<(), String> {
    let construction = stream(&manifest("scene_construction", 12, 20, 8, 1, 3, None)).map_err(|e| e.to_string())?;
    let built = graphs(&construction);
    if built.len() != 4 || built.windows(2).any(|pair| pair[0] != pair[1]) {
        return Err("W1 must build the same graph on each of its 4 cycles".to_string());
    }
    let lifecycle = stream(&manifest("scene_lifecycle", 12, 20, 8, 1, 3, None)).map_err(|e| e.to_string())?;
    let built = graphs(&lifecycle);
    if built.len() != 4 || built.windows(2).any(|pair| pair[0] == pair[1]) {
        return Err("W3 must build a different graph on each of its 4 cycles".to_string());
    }
    for ops in [&construction, &lifecycle] {
        let count = |pick: fn(&Op) -> bool| ops.iter().filter(|op| pick(op)).count();
        if count(|op| matches!(op, Op::DropScene)) != 4 || count(|op| matches!(op, Op::RemoveHost)) != 4 {
            return Err("each of 4 cycles must drop its scene and remove its host".to_string());
        }
        if !matches!(ops.last(), Some(Op::Checkpoint { .. })) {
            return Err("the stream must end on the checkpoint that shows nothing is left".to_string());
        }
    }
    Ok(())
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[test]
fn the_busiest_node_is_moved_more_often_than_chance() -> Result<(), String> {
    let m = manifest("connector_rerouting", 20, 40, 10, 0, 800, None);
    let planned = plan_for(&m, STREAM_GRAPH).map_err(|e| e.to_string())?;
    let hub_key = &planned.nodes[planned.hub].key;
    let ops = stream(&m).map_err(|e| e.to_string())?;
    let hub_moves = ops
        .iter()
        .filter(|op| matches!(op, Op::MoveNode { key, .. } if key == hub_key))
        .count();
    // One move in four is aimed at the hub, and one in twenty of the rest lands on it by chance: about 0.29 of 800.
    if !(160..=300).contains(&hub_moves) {
        return Err(format!("{hub_moves} of 800 moves were of the hub"));
    }
    Ok(())
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[test]
fn a_visitor_that_refuses_stops_the_stream() -> Result<(), String> {
    let m = manifest("connector_rerouting", 6, 8, 8, 0, 4, None);
    let mut seen = 0;
    let result = generate(&m, &mut |_| {
        seen += 1;
        if seen == 3 { Err("no".to_string()) } else { Ok(()) }
    });
    if result != Err(WorkloadError::Visit("no".to_string())) || seen != 3 {
        return Err(format!("expected a stop at 3, got {result:?} after {seen}"));
    }
    Ok(())
}
