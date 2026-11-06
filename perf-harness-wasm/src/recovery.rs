//! The scenes that the recovery runner breaks on purpose.
//!
//! The runner is a page script that makes a DOM call fail at a chosen place (`perf/page/fault-injector.js`) and looks at what
//! the library left behind. This is the Rust side of that: it builds a scene of an interaction or recovery manifest on a
//! numbered slot, so that a scene and its reference can stand side by side, performs one operation at a time, and reads a
//! scene back into a checkpoint that can be kept under a name and compared with another.
//!
//! Two things are kept apart. What the scene was asked to do is applied to the reference model as an *intention*, once. What
//! was done to the scene is a separate call that can fail and be tried again, so a retry never changes the reference.
//! Nothing here is timed. The times of a recovery are taken around these calls by the page script.

use crate::observe::observe;
use perf_harness::compare::compare;
use perf_harness::manifest::{Manifest, ViewPolicy, Workload};
use perf_harness::model::{Checkpoint, Model};
use perf_harness::ops::{Op, StreamHasher};
use perf_harness::workload::{field_for, generate};
use std::cell::RefCell;
use std::collections::BTreeMap;
use svg_dom::{
    SvgRoot,
    root::utils::{Point, Size},
};
use svg_dom_graph::{
    EdgeId, NodeId,
    scene::{CollisionPolicy, ConnectorOptions, ConnectorType, DragOptions, Scene},
};

const SVG_NS: &str = "http://www.w3.org/2000/svg";

thread_local! {
    static STAGES: RefCell<BTreeMap<u32, Stage>> = const { RefCell::new(BTreeMap::new()) };
    static SAVED: RefCell<BTreeMap<String, Checkpoint>> = const { RefCell::new(BTreeMap::new()) };
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
struct Stage {
    svg: web_sys::Element,
    scene: Option<Scene>,
    nodes: BTreeMap<String, NodeId>,
    edges: Vec<EdgeId>,
    /// The keys of the edges whose connectors the scene drew, in the order it drew them, which is how observation names them.
    edge_keys: Vec<String>,
    model: Model,
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The id of the `<svg>` that holds the scene on `slot`.
pub fn host_id(slot: u32) -> String {
    format!("perf-recovery-{slot}")
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
fn error(message: impl std::fmt::Display) -> String {
    serde_json::json!({ "ok": false, "error": message.to_string() }).to_string()
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
fn ok(value: serde_json::Value) -> String {
    let mut value = value;
    if let Some(object) = value.as_object_mut() {
        object.insert("ok".to_string(), serde_json::json!(true));
    }
    value.to_string()
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
fn with_stage<T>(slot: u32, work: impl FnOnce(&mut Stage) -> Result<T, String>) -> Result<T, String> {
    STAGES.with_borrow_mut(|stages| {
        work(
            stages
                .get_mut(&slot)
                .ok_or_else(|| format!("nothing is built on slot {slot}"))?,
        )
    })
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Builds the scene of `manifest_json` on `slot`, drawn as the stream says, and gives its nodes and edges.
///
/// With `labelled`, every second edge is given a label, since a label is what the label-update fault point needs to break, and
/// the reference model does not model labels.
pub fn build(manifest_json: &str, slot: u32, labelled: bool) -> String {
    teardown(slot);
    let manifest = match serde_json::from_str(manifest_json)
        .map_err(|e| e.to_string())
        .and_then(|v| Manifest::from_json(v).map_err(|e| e.to_string()))
    {
        Ok(manifest) => manifest,
        Err(message) => return error(message),
    };
    if !matches!(manifest.workload, Workload::Recovery | Workload::Interaction) {
        return error("only the recovery workload is broken on purpose");
    }
    match build_stage(&manifest, slot, labelled) {
        Ok(summary) => ok(summary),
        Err(message) => {
            teardown(slot);
            error(message)
        },
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
fn build_stage(manifest: &Manifest, slot: u32, labelled: bool) -> Result<serde_json::Value, String> {
    let document = web_sys::window().and_then(|w| w.document()).ok_or("there is no document")?;
    let (width, height) = field_for(manifest.graph.nodes);
    let id = host_id(slot);
    let svg = document.create_element_ns(Some(SVG_NS), "svg").map_err(|e| format!("{e:?}"))?;
    for (name, value) in [
        ("id", id.clone()),
        ("width", width.to_string()),
        ("height", height.to_string()),
        ("viewBox", format!("0 0 {width} {height}")),
        ("style", "display: block; margin: 0".to_string()),
    ] {
        svg.set_attribute(name, &value).map_err(|e| format!("{e:?}"))?;
    }
    document
        .body()
        .ok_or("there is no body")?
        .append_child(&svg)
        .map_err(|e| format!("{e:?}"))?;

    let straight = ConnectorOptions::default().with_connector_type(ConnectorType::Straight);
    let mut model = Model::new();
    let mut scene: Option<Scene> = None;
    let mut nodes = BTreeMap::new();
    let mut edges = Vec::new();
    let mut edge_keys = Vec::new();
    let mut pairs: Vec<(String, String)> = Vec::new();
    let mut hasher = StreamHasher::new();

    generate(manifest, &mut |op: &Op| {
        model.apply(op)?;
        hasher.push(op);
        match op {
            Op::CreateScene => {
                scene = Some(Scene::new(SvgRoot::attach(&id).map_err(|e| e.to_string())?).map_err(|e| e.to_string())?);
            },
            Op::AddNode { key, x, y, w, h, label } => {
                let scene = scene.as_ref().ok_or("there is no scene")?;
                let node = scene
                    .add_node(Point::new(*x as f64, *y as f64), Size::new(*w as f64, *h as f64), label.clone())
                    .map_err(|e| e.to_string())?;
                nodes.insert(key.clone(), node);
            },
            Op::AddEdge { from, to } => {
                let scene = scene.as_ref().ok_or("there is no scene")?;
                let (a, b) = (
                    *nodes.get(from).ok_or("an unknown node")?,
                    *nodes.get(to).ok_or("an unknown node")?,
                );
                edges.push(scene.add_edge_with(a, b, straight).map_err(|e| e.to_string())?);
                edge_keys.push(format!("{from}->{to}"));
                pairs.push((from.clone(), to.clone()));
            },
            Op::Checkpoint { .. } => {},
            _ => return Err("the recovery workload builds one scene and does nothing else".to_string()),
        }
        Ok(())
    })
    .map_err(|e| e.to_string())?;

    let scene = scene.ok_or("the stream made no scene")?;
    if labelled {
        for (index, edge) in edges.iter().enumerate().filter(|(index, _)| index % 2 == 0) {
            scene
                .set_edge_label(*edge, Some(&format!("w{index}")))
                .map_err(|e| e.to_string())?;
        }
    }
    let operations = hasher.operations();
    let (sha256, mutations_sha256) = hasher.finish();
    let summary = serde_json::json!({
        "svg_id": id,
        "operations": operations,
        "sha256": sha256,
        "mutations_sha256": mutations_sha256,
        "nodes": nodes.keys().collect::<Vec<_>>(),
        "edges": pairs,
    });
    STAGES.with_borrow_mut(|stages| {
        stages.insert(
            slot,
            Stage {
                svg,
                scene: Some(scene),
                nodes,
                edges,
                edge_keys,
                model,
            },
        );
    });
    Ok(summary)
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
fn text(op: &serde_json::Value, name: &str) -> Result<String, String> {
    op[name]
        .as_str()
        .map(str::to_owned)
        .ok_or_else(|| format!("the operation has no {name}"))
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
fn number(op: &serde_json::Value, name: &str) -> Result<i64, String> {
    op[name]
        .as_i64()
        .ok_or_else(|| format!("the operation has no whole number {name}"))
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// What the scene was asked to do, as the reference model has it. Called once for each thing asked, whatever becomes of it.
pub fn intend(slot: u32, op_json: &str) -> String {
    let result = (|| {
        let op: serde_json::Value = serde_json::from_str(op_json).map_err(|e| e.to_string())?;
        let op = match op["op"].as_str() {
            Some("add_node") => Op::AddNode {
                key: text(&op, "key")?,
                x: number(&op, "x")?,
                y: number(&op, "y")?,
                w: number(&op, "w")?,
                h: number(&op, "h")?,
                label: text(&op, "key")?,
            },
            Some("add_edge") => Op::AddEdge {
                from: text(&op, "from")?,
                to: text(&op, "to")?,
            },
            Some("move_node") => Op::MoveNode {
                key: text(&op, "key")?,
                x: number(&op, "x")?,
                y: number(&op, "y")?,
            },
            // Making a node draggable, or labelling an edge, is not in the model.
            Some("make_draggable" | "set_edge_label") => return Ok(()),
            other => return Err(format!("there is no operation {other:?}")),
        };
        with_stage(slot, |stage| stage.model.apply(&op))
    })();
    result.map_or_else(error, |()| ok(serde_json::json!({})))
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Does one thing to the scene, which can fail, and says whether it did. A failure is a result and not a trap, and is not
/// recorded anywhere but in what is returned: the stage keeps only what the scene confirmed.
pub fn act(slot: u32, op_json: &str) -> String {
    let result = (|| {
        let op: serde_json::Value = serde_json::from_str(op_json).map_err(|e| e.to_string())?;
        let straight = ConnectorOptions::default().with_connector_type(ConnectorType::Straight);
        let kind = op["op"].as_str().ok_or("the operation has no name")?.to_owned();
        with_stage(slot, |stage| {
            let scene = stage.scene.clone().ok_or("the scene was dropped")?;
            let known = |stage: &Stage, key: &str| {
                stage
                    .nodes
                    .get(key)
                    .copied()
                    .ok_or_else(|| format!("the node {key} is unknown"))
            };
            match kind.as_str() {
                "add_node" => {
                    let key = text(&op, "key")?;
                    let node = scene.add_node(
                        Point::new(number(&op, "x")? as f64, number(&op, "y")? as f64),
                        Size::new(number(&op, "w")? as f64, number(&op, "h")? as f64),
                        key.clone(),
                    );
                    stage.nodes.insert(key, node.map_err(|e| e.to_string())?);
                },
                "add_edge" => {
                    let (from, to) = (text(&op, "from")?, text(&op, "to")?);
                    let edge = scene.add_edge_with(known(stage, &from)?, known(stage, &to)?, straight);
                    stage.edges.push(edge.map_err(|e| e.to_string())?);
                    stage.edge_keys.push(format!("{from}->{to}"));
                },
                "move_node" => scene
                    .move_node(
                        known(stage, &text(&op, "key")?)?,
                        Point::new(number(&op, "x")? as f64, number(&op, "y")? as f64),
                    )
                    .map_err(|e| e.to_string())?,
                "make_draggable" => scene
                    .make_draggable_with(
                        known(stage, &text(&op, "key")?)?,
                        DragOptions::default().with_collision(CollisionPolicy::Allow),
                    )
                    .map_err(|e| e.to_string())?,
                "set_edge_label" => {
                    let edge = *stage.edges.get(number(&op, "edge")? as usize).ok_or("the edge is unknown")?;
                    scene
                        .set_edge_label(edge, Some(&text(&op, "text")?))
                        .map_err(|e| e.to_string())?;
                },
                other => return Err(format!("there is no operation {other}")),
            }
            Ok(())
        })
    })();
    result.map_or_else(error, |()| ok(serde_json::json!({})))
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Reads the scene on `slot` back from the document and keeps it as `name`.
pub fn snapshot(slot: u32, name: &str) -> String {
    let result = with_stage(slot, |stage| {
        let scene = stage.scene.as_ref().ok_or("the scene was dropped")?;
        observe(&stage.svg, scene, &stage.edge_keys, ViewPolicy::Unchanged)
    });
    match result {
        Ok(checkpoint) => {
            SAVED.with_borrow_mut(|saved| saved.insert(name.to_owned(), checkpoint));
            ok(serde_json::json!({}))
        },
        Err(message) => error(message),
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Keeps what the reference model predicts for the scene on `slot` as `name`.
pub fn predicted(slot: u32, name: &str) -> String {
    match with_stage(slot, |stage| stage.model.checkpoint(ViewPolicy::Unchanged)) {
        Ok(checkpoint) => {
            SAVED.with_borrow_mut(|saved| saved.insert(name.to_owned(), checkpoint));
            ok(serde_json::json!({}))
        },
        Err(message) => error(message),
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The differences between two kept checkpoints, `expected` first, as a list of the sections that differ with the first few
/// accounts of how.
pub fn differences(expected: &str, actual: &str) -> String {
    SAVED.with_borrow(|saved| match (saved.get(expected), saved.get(actual)) {
        (Some(a), Some(b)) => {
            let found = compare(a, b);
            let mut sections: Vec<&str> = found.iter().map(|d| d.section).collect();
            sections.dedup();
            ok(serde_json::json!({
                "sections": sections,
                "count": found.len(),
                "first": found.iter().take(3).map(perf_harness::compare::Difference::summary).collect::<Vec<_>>(),
            }))
        },
        _ => error(format!("there is no checkpoint kept as {expected} or {actual}")),
    })
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Where the scene on `slot` says node `key` is.
pub fn node_rect(slot: u32, key: &str) -> String {
    let result = with_stage(slot, |stage| {
        let scene = stage.scene.as_ref().ok_or("the scene was dropped")?;
        let id = *stage.nodes.get(key).ok_or_else(|| format!("the node {key} is unknown"))?;
        scene.node_rect(id).map_err(|e| e.to_string())
    });
    result.map_or_else(error, |rect| ok(serde_json::json!({ "x": rect.origin.x, "y": rect.origin.y })))
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// How many nodes and edges the stage has confirmed, which is what the scene's own handles number.
pub fn counts(slot: u32) -> String {
    with_stage(slot, |stage| {
        Ok(serde_json::json!({ "nodes": stage.nodes.len(), "edges": stage.edges.len() }))
    })
    .map_or_else(error, ok)
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Drops the scene on `slot` and removes its host. Safe to call with nothing built.
pub fn teardown(slot: u32) -> String {
    STAGES.with_borrow_mut(|stages| {
        if let Some(mut stage) = stages.remove(&slot) {
            stage.scene = None;
            stage.svg.remove();
        }
    });
    ok(serde_json::json!({}))
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Drops the scene on `slot` and leaves its host where it is, which is what dropping the last handle does by itself.
pub fn drop_scene(slot: u32) -> String {
    STAGES.with_borrow_mut(|stages| {
        if let Some(stage) = stages.get_mut(&slot) {
            stage.scene = None;
        }
    });
    ok(serde_json::json!({}))
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Drops every kept checkpoint.
pub fn forget() {
    SAVED.with_borrow_mut(BTreeMap::clear);
}
