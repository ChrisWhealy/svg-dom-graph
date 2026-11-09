use super::*;
use crate::manifest::{Interaction, Manifest};
use crate::workload::{MARGIN, PITCH_X};

fn manifest(
    nodes: u32,
    edges: u32,
    scales: &str,
    gestures: &str,
    steps: u32,
    warmup: u32,
    iterations: u32,
) -> Manifest {
    let text = format!(
        r#"
        schema_version = 1
        workload = "interaction"
        seed = 11
        prng = "splitmix64"
        [graph]
        nodes = {nodes}
        edges = {edges}
        maximum_degree = 6
        [execution]
        modes = ["correctness", "performance"]
        warmup_iterations = {warmup}
        measurement_iterations = {iterations}
        repetitions = 1
        [measurement]
        capture_dom_counts = true
        capture_listener_tally = true
        capture_heap = false
        capture_wasm_capacity = false
        capture_wasm_live_allocation = false
        capture_frame_timing = true
        checkpoint_every = 1
        [checkpoint]
        view = "unchanged"
        [interaction]
        scales_percent = {scales}
        gestures = {gestures}
        steps = {steps}
        step_pixels = 6
        [limits]
        wall_ms = 1000
        "#
    );
    Manifest::from_toml(&text).expect("the test manifest is valid")
}

fn small() -> Manifest {
    manifest(24, 30, "[50, 100, 200]", r#"["drag", "pan", "wheel_zoom"]"#, 8, 2, 6)
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[test]
fn there_is_a_case_for_every_scale_and_gesture_with_the_counts_asked_for() -> Result<(), String> {
    let plan = plan(&small()).map_err(|e| e.to_string())?;
    if plan.cases.len() != 9 {
        return Err(format!("{} cases", plan.cases.len()));
    }
    for case in &plan.cases {
        if case.warmup.len() != 2 || case.measured.len() != 6 {
            return Err(format!(
                "{:?} at {}: {} and {}",
                case.gesture,
                case.scale_percent,
                case.warmup.len(),
                case.measured.len()
            ));
        }
    }
    let order: Vec<(u32, GestureKind)> = plan.cases.iter().map(|c| (c.scale_percent, c.gesture)).collect();
    if order[0] != (50, GestureKind::Drag) || order[1] != (50, GestureKind::Pan) || order[3] != (100, GestureKind::Drag)
    {
        return Err(format!("{order:?}"));
    }
    Ok(())
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[test]
fn the_same_manifest_gives_the_same_plan_and_another_seed_does_not() -> Result<(), String> {
    let a = plan(&small()).map_err(|e| e.to_string())?;
    let b = plan(&small()).map_err(|e| e.to_string())?;
    if a != b || a.sha256() != b.sha256() {
        return Err("the same manifest gave two plans".to_string());
    }
    let mut other = small();
    other.seed += 1;
    if plan(&other).map_err(|e| e.to_string())?.sha256() == a.sha256() {
        return Err("another seed gave the same plan".to_string());
    }
    Ok(())
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// One case does not depend on the cases before it, so a manifest with more scales does not change the cases it shares.
#[test]
fn a_case_does_not_depend_on_the_cases_before_it() -> Result<(), String> {
    let few = plan(&manifest(24, 30, "[100]", r#"["pan"]"#, 8, 2, 6)).map_err(|e| e.to_string())?;
    let many = plan(&manifest(24, 30, "[100]", r#"["drag", "pan"]"#, 8, 2, 6)).map_err(|e| e.to_string())?;
    // The pan is stream 0 in the first and stream 1 in the second, so the two differ, but each is itself reproducible.
    if few.cases[0] == many.cases[1] {
        return Err("a case that moved down a stream gave the same draws".to_string());
    }
    let again = plan(&manifest(24, 30, "[100]", r#"["drag", "pan"]"#, 8, 2, 6)).map_err(|e| e.to_string())?;
    if again != many {
        return Err("a plan was not reproducible".to_string());
    }
    Ok(())
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[test]
fn every_drag_starts_on_a_node_that_is_in_view_and_ends_in_view() -> Result<(), String> {
    let manifest = small();
    let plan = plan(&manifest).map_err(|e| e.to_string())?;
    let graph = graph_plan(&manifest).map_err(|e| e.to_string())?;
    for case in plan.cases.iter().filter(|c| c.gesture == GestureKind::Drag) {
        let scale = f64::from(case.scale_percent) / 100.0;
        for item in case.warmup.iter().chain(&case.measured) {
            let Item::Drag {
                node,
                from,
                dx,
                dy,
                steps,
                start,
                expected,
            } = item
            else {
                return Err("a drag case holds another gesture".to_string());
            };
            let planned = graph
                .nodes
                .iter()
                .find(|n| n.key == *node)
                .ok_or("a node that is not in the graph")?;
            if [planned.x, planned.y] != *start || *steps != 8 {
                return Err(format!("{node}: {start:?}"));
            }
            let [left, top, right, bottom] = screen_box(planned.x, planned.y, scale);
            let centre = [(left + right) / 2.0, (top + bottom) / 2.0];
            if (centre[0] - from[0] as f64).abs() > 0.5 || (centre[1] - from[1] as f64).abs() > 0.5 {
                return Err(format!("{node} is pressed at {from:?}, not on its centre {centre:?}"));
            }
            // The box after the move is wholly inside the host.
            let (ex, ey) = (expected[0] * scale, expected[1] * scale);
            if ex < 0.0
                || ey < 0.0
                || ex + NODE_W as f64 * scale > plan.host[0] as f64
                || ey + NODE_H as f64 * scale > plan.host[1] as f64
            {
                return Err(format!("{node} would leave the host at {expected:?} ({dx}, {dy})"));
            }
            if (expected[0] - (planned.x as f64 + *dx as f64 / scale)).abs() > 1e-9 {
                return Err(format!(
                    "{node}: the expected position is not the start plus the pointer movement over the scale"
                ));
            }
        }
    }
    Ok(())
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[test]
fn every_pan_starts_on_the_background_and_translates_by_the_movement() -> Result<(), String> {
    let manifest = small();
    let plan = plan(&manifest).map_err(|e| e.to_string())?;
    let graph = graph_plan(&manifest).map_err(|e| e.to_string())?;
    for case in plan.cases.iter().filter(|c| c.gesture == GestureKind::Pan) {
        let scale = f64::from(case.scale_percent) / 100.0;
        for item in case.warmup.iter().chain(&case.measured) {
            let Item::Pan { from, dx, dy, expected, .. } = item else {
                return Err("not a pan".to_string());
            };
            for node in &graph.nodes {
                let [l, t, r, b] = screen_box(node.x, node.y, scale);
                let (x, y) = (from[0] as f64, from[1] as f64);
                if x >= l && x <= r && y >= t && y <= b {
                    return Err(format!("the pan at {from:?} starts on {}", node.key));
                }
            }
            if *expected
                != (View {
                    scale,
                    tx: *dx as f64,
                    ty: *dy as f64,
                })
            {
                return Err(format!("{expected:?}"));
            }
        }
    }
    Ok(())
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[test]
fn a_wheel_zoom_keeps_the_pointer_where_it_is_and_stays_within_the_librarys_scales() -> Result<(), String> {
    let plan = plan(&small()).map_err(|e| e.to_string())?;
    let mut ins = 0;
    let mut outs = 0;
    for case in plan.cases.iter().filter(|c| c.gesture == GestureKind::WheelZoom) {
        for item in case.warmup.iter().chain(&case.measured) {
            let Item::WheelZoom { at, delta_y, expected } = item else {
                return Err("not a wheel".to_string());
            };
            let scale = f64::from(case.scale_percent) / 100.0;
            // The point of the scene under the pointer before is under it after.
            let (x, y) = (at[0] as f64, at[1] as f64);
            let (before, after) = (x / scale, (x - expected.tx) / expected.scale);
            let (before_y, after_y) = (y / scale, (y - expected.ty) / expected.scale);
            if (before - after).abs() > 1e-9 || (before_y - after_y).abs() > 1e-9 {
                return Err(format!("the pointer moved over the scene: {before} to {after}"));
            }
            if !(0.25..=4.0).contains(&expected.scale) {
                return Err(format!("the scale {} is outside the library's", expected.scale));
            }
            if (expected.scale / scale - ZOOM_STEP.powf(-(*delta_y as f64) / 100.0)).abs() > 1e-9 {
                return Err("the factor is not a step to the power of the notches".to_string());
            }
            if *delta_y < 0 { ins += 1 } else { outs += 1 }
        }
    }
    if ins == 0 || outs == 0 {
        return Err(format!(
            "{ins} zoom in and {outs} zoom out: a stream of one direction tests half of the rule"
        ));
    }
    Ok(())
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[test]
fn the_scales_a_manifest_may_ask_for_are_the_librarys_range() -> Result<(), String> {
    // The library keeps a view between a quarter and four times, and a step is a quarter more. Those are constants it
    // does not export, so a browser run is what holds the plan to them: a sample whose view did not end where the plan
    // said fails.
    let scales = Interaction::SCALE_RANGE_PERCENT;
    if *scales.start() != 25 || *scales.end() != 400 || ZOOM_STEP != 1.25 {
        return Err(format!("{scales:?} {ZOOM_STEP}"));
    }
    Ok(())
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[test]
fn a_drag_with_no_node_in_view_is_an_error_and_not_a_guess() -> Result<(), String> {
    // One node makes a host 160 pixels wide, and at 400% that node alone is 320 pixels wide, so none is wholly in view.
    let result = plan(&manifest(1, 0, "[400]", r#"["drag"]"#, 4, 1, 1));
    match result {
        Ok(_) => Err("a drag was planned with no node in view".to_string()),
        Err(error) if error.to_string().contains("wholly in view") => Ok(()),
        Err(other) => Err(other.to_string()),
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[test]
fn only_an_interaction_manifest_has_gestures() -> Result<(), String> {
    let text = include_str!("../../../perf/manifests/w2-connector-rerouting-small.toml");
    let other = Manifest::from_toml(text).map_err(|e| e.to_string())?;
    if plan(&other).is_ok() {
        return Err("a rerouting manifest was given gestures".to_string());
    }
    Ok(())
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[test]
fn the_interaction_stream_is_one_build_and_one_checkpoint() -> Result<(), String> {
    use crate::ops::Op;
    let manifest = small();
    let mut ops = Vec::new();
    crate::workload::generate(&manifest, &mut |op: &Op| {
        ops.push(op.clone());
        Ok(())
    })
    .map_err(|e| e.to_string())?;
    let kinds: Vec<&str> = ops
        .iter()
        .map(|op| match op {
            Op::CreateScene => "create",
            Op::AddNode { .. } => "node",
            Op::AddEdge { .. } => "edge",
            Op::Checkpoint { .. } => "checkpoint",
            _ => "other",
        })
        .collect();
    if kinds.first() != Some(&"create") || kinds.last() != Some(&"checkpoint") || kinds.contains(&"other") {
        return Err(format!("{kinds:?}"));
    }
    if kinds.iter().filter(|k| **k == "node").count() != 24 || kinds.iter().filter(|k| **k == "edge").count() != 30 {
        return Err("the build is not the graph".to_string());
    }
    let _ = (MARGIN, PITCH_X);
    Ok(())
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[test]
fn the_manifest_refuses_a_bad_interaction_table() -> Result<(), String> {
    let good = r#"["drag"]"#;
    for (scales, gestures, steps, why) in [
        ("[10]", good, 4, "a scale below the library's range"),
        ("[500]", good, 4, "a scale above it"),
        ("[100, 100]", good, 4, "a repeated scale"),
        ("[]", good, 4, "no scale"),
        ("[100]", r#"["drag", "drag"]"#, 4, "a repeated gesture"),
        ("[100]", "[]", 4, "no gesture"),
        ("[100]", good, 0, "no steps"),
    ] {
        let text = small_text()
            .replace("scales_percent = [50, 100, 200]", &format!("scales_percent = {scales}"))
            .replace(r#"gestures = ["drag", "pan", "wheel_zoom"]"#, &format!("gestures = {gestures}"))
            .replace("steps = 8", &format!("steps = {steps}"));
        if Manifest::from_toml(&text).is_ok() {
            return Err(format!("{why} was accepted"));
        }
    }
    Ok(())
}

fn small_text() -> String {
    include_str!("../../../perf/manifests/w4-interaction-small.toml").to_string()
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// A known answer. The plan of the reference manifest has this digest on every platform, the 32-bit wasm build
/// included, so a change to the generator, to how a gesture is drawn, or to the arithmetic that predicts its effect
/// shows here at once.
#[test]
fn the_plan_of_the_reference_manifest_is_fixed() -> Result<(), String> {
    let manifest = Manifest::from_toml(&small_text()).map_err(|e| e.to_string())?;
    let digest = plan(&manifest).map_err(|e| e.to_string())?.sha256();
    if digest != "b071705bd763d5560308fe209365687236b242b24862d4770a46b17f7719e09a" {
        return Err(format!("the plan of the reference manifest is now {digest}"));
    }
    Ok(())
}
