use super::*;

fn node(key: &str, x: i64, y: i64) -> Op {
    Op::AddNode {
        key: key.into(),
        x,
        y,
        w: 80,
        h: 36,
        label: key.to_uppercase(),
    }
}

fn edge(from: &str, to: &str) -> Op {
    Op::AddEdge {
        from: from.into(),
        to: to.into(),
    }
}

fn live(ops: &[Op]) -> Result<Model, String> {
    let mut model = Model::new();
    model.apply(&Op::CreateScene)?;
    for op in ops {
        model.apply(op)?;
    }
    Ok(model)
}

fn element<'a>(checkpoint: &'a Checkpoint, key: &str) -> Result<&'a GeometryElement, String> {
    checkpoint
        .geometry
        .elements
        .iter()
        .find(|e| e.key == key)
        .ok_or_else(|| format!("no element {key}"))
}

fn near(got: &[f64], want: &[f64]) -> Result<(), String> {
    if got.len() == want.len() && got.iter().zip(want).all(|(g, w)| (g - w).abs() < 1e-9) {
        Ok(())
    } else {
        Err(format!("expected {want:?}, got {got:?}"))
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[test]
fn a_straight_connector_runs_between_the_boundaries_along_the_line_of_centres() -> Result<(), String> {
    // Side by side at one height: the ends are on the facing sides, level with the centres.
    let level = live(&[node("a", 0, 0), node("b", 200, 0), edge("a", "b")])?.checkpoint(ViewPolicy::Unchanged)?;
    near(&element(&level, "a->b")?.coordinates, &[80.0, 18.0, 200.0, 18.0])?;
    // Corner to corner, 200 across and 100 down: the ray meets the bottom and top first, at 0.18 of the distance.
    let slanted = live(&[node("a", 0, 0), node("b", 200, 100), edge("a", "b")])?.checkpoint(ViewPolicy::Unchanged)?;
    near(&element(&slanted, "a->b")?.coordinates, &[76.0, 36.0, 204.0, 100.0])?;
    // Stacked: the ends are on the facing top and bottom.
    let stacked = live(&[node("a", 0, 0), node("b", 0, 100), edge("a", "b")])?.checkpoint(ViewPolicy::Unchanged)?;
    near(&element(&stacked, "a->b")?.coordinates, &[40.0, 36.0, 40.0, 100.0])
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[test]
fn boxes_whose_centres_coincide_give_the_centre_as_both_ends() -> Result<(), String> {
    let checkpoint =
        live(&[node("a", 10, 10), node("b", 10, 10), edge("a", "b")])?.checkpoint(ViewPolicy::Unchanged)?;
    near(&element(&checkpoint, "a->b")?.coordinates, &[50.0, 28.0, 50.0, 28.0])
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[test]
fn a_move_changes_the_geometry_and_nothing_else() -> Result<(), String> {
    let mut model = live(&[node("a", 0, 0), node("b", 200, 0), edge("a", "b")])?;
    let before = model.checkpoint(ViewPolicy::Unchanged)?;
    model.apply(&Op::MoveNode {
        key: "b".into(),
        x: 200,
        y: 100,
    })?;
    let after = model.checkpoint(ViewPolicy::Unchanged)?;
    near(&element(&after, "b")?.coordinates, &[200.0, 100.0, 80.0, 36.0])?;
    if element(&after, "a->b")?.coordinates == element(&before, "a->b")?.coordinates {
        return Err("the connector did not follow the node".to_string());
    }
    if before.model_sha256() == after.model_sha256() {
        return Err("the whole-checkpoint digest ignored a move".to_string());
    }
    let (b, a) = (before.section_hashes(), after.section_hashes());
    if b != a {
        return Err("a move changed an exactly-compared section".to_string());
    }
    Ok(())
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[test]
fn the_topology_is_sorted_by_key_and_edges_keep_their_order() -> Result<(), String> {
    let checkpoint = live(&[
        node("n10", 0, 0),
        node("n2", 100, 0),
        node("n1", 200, 0),
        edge("n2", "n1"),
        edge("n10", "n2"),
    ])?
    .checkpoint(ViewPolicy::Unchanged)?;
    if checkpoint.topology.nodes != ["n1", "n10", "n2"] {
        return Err(format!("nodes {:?}", checkpoint.topology.nodes));
    }
    let edges: Vec<String> = checkpoint
        .topology
        .edges
        .iter()
        .map(|e| format!("{}->{}", e.from, e.to))
        .collect();
    if edges != ["n2->n1", "n10->n2"] {
        return Err(format!("edges {edges:?}"));
    }
    if checkpoint.paint_order != ["n10", "n2", "n1"] {
        return Err(format!("paint order {:?}", checkpoint.paint_order));
    }
    if checkpoint.content.get("n2").map(String::as_str) != Some("N2") {
        return Err("content did not carry the label".to_string());
    }
    Ok(())
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[test]
fn the_view_follows_the_policy() -> Result<(), String> {
    let model = live(&[node("a", 0, 0)])?;
    let shown = model.checkpoint(ViewPolicy::Unchanged)?.interaction.view;
    let ignored = model.checkpoint(ViewPolicy::Ignored)?.interaction.view;
    if shown != Some(View { x: 0.0, y: 0.0, scale: 1.0 }) || ignored.is_some() {
        return Err(format!("unchanged gave {shown:?} and ignored gave {ignored:?}"));
    }
    Ok(())
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[test]
fn dropping_and_removing_leaves_nothing_and_the_scene_can_start_again() -> Result<(), String> {
    let mut model = live(&[node("a", 0, 0), node("b", 200, 0), edge("a", "b")])?;
    model.apply(&Op::DropScene)?;
    if model.checkpoint(ViewPolicy::Unchanged).is_ok() {
        return Err("a checkpoint was allowed between the drop and the removal".to_string());
    }
    model.apply(&Op::RemoveHost)?;
    let empty = model.checkpoint(ViewPolicy::Unchanged)?;
    if !empty.topology.nodes.is_empty() || !empty.geometry.elements.is_empty() || model.node_count() != 0 {
        return Err("something was left after the host was removed".to_string());
    }
    model.apply(&Op::CreateScene)?;
    model.apply(&node("a", 0, 0))
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[test]
fn a_stream_that_makes_no_sense_is_refused() -> Result<(), String> {
    let cases: Vec<(&str, Vec<Op>)> = vec![
        ("a node with no scene", vec![node("a", 0, 0)]),
        ("a scene while one is there", vec![Op::CreateScene, Op::CreateScene]),
        ("a dropped scene that was never made", vec![Op::DropScene]),
        ("removing a host before the drop", vec![Op::CreateScene, Op::RemoveHost]),
        ("a repeated node key", vec![Op::CreateScene, node("a", 0, 0), node("a", 5, 5)]),
        ("an edge to nowhere", vec![Op::CreateScene, node("a", 0, 0), edge("a", "z")]),
        ("a self-loop", vec![Op::CreateScene, node("a", 0, 0), edge("a", "a")]),
        (
            "a repeated edge",
            vec![
                Op::CreateScene,
                node("a", 0, 0),
                node("b", 100, 0),
                edge("a", "b"),
                edge("a", "b"),
            ],
        ),
        (
            "a move of nothing",
            vec![Op::CreateScene, Op::MoveNode { key: "a".into(), x: 0, y: 0 }],
        ),
        (
            "a node with no area",
            vec![
                Op::CreateScene,
                Op::AddNode {
                    key: "a".into(),
                    x: 0,
                    y: 0,
                    w: 0,
                    h: 36,
                    label: "a".into(),
                },
            ],
        ),
        ("work after the drop", vec![Op::CreateScene, Op::DropScene, node("a", 0, 0)]),
    ];
    for (label, ops) in cases {
        let mut model = Model::new();
        if ops.iter().all(|op| model.apply(op).is_ok()) {
            return Err(format!("{label} was accepted"));
        }
    }
    Ok(())
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// What the model produces has to be a document that the checkpoint schema accepts, since runners compare against it.
#[cfg(feature = "schema")]
#[test]
fn a_checkpoint_satisfies_the_checkpoint_schema() -> Result<(), String> {
    let schema: serde_json::Value = serde_json::from_str(include_str!("../../../perf/schemas/checkpoint.schema.json"))
        .map_err(|e| e.to_string())?;
    let validator = jsonschema::validator_for(&schema).map_err(|e| e.to_string())?;
    let model = live(&[node("a", 0, 0), node("b", 200, 100), edge("a", "b")])?;
    for (label, checkpoint) in [
        ("a built scene", model.checkpoint(ViewPolicy::Unchanged)?),
        ("a built scene with the view ignored", model.checkpoint(ViewPolicy::Ignored)?),
        ("an empty scene", Model::new().checkpoint(ViewPolicy::Unchanged)?),
    ] {
        let document = serde_json::to_value(&checkpoint).map_err(|e| e.to_string())?;
        let violations: Vec<String> = validator
            .iter_errors(&document)
            .map(|e| format!("{e} at {}", e.instance_path()))
            .collect();
        if !violations.is_empty() {
            return Err(format!("{label}: {}", violations.join("; ")));
        }
    }
    Ok(())
}
