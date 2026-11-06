//! The scene that Layer C drives with real input.
//!
//! Workload W4 builds one scene and checks it, and a browser is then driven through it. This builds that scene on a page, with
//! every node draggable and the pan and the wheel zoom switched on, and gives the runner what it needs to find where to put the
//! pointer and to read what a gesture did. Nothing here is timed, since the time that Layer C measures is of the browser's, from
//! the input to the frame, and it is read in the page and from the trace.
//!
//! The scene is kept in a stage of this module, the way the fixture of the browser tests keeps its own, so that a page can build
//! it, drive it and take it down again without the runner holding a handle.

use crate::observe::observe;
use perf_harness::compare::compare;
use perf_harness::manifest::{Manifest, ViewPolicy, Workload};
use perf_harness::model::Model;
use perf_harness::ops::{Op, StreamHasher};
use perf_harness::workload::{field_for, generate};
use std::cell::RefCell;
use std::collections::BTreeMap;
use svg_dom::{
    SvgRoot,
    root::utils::{Point, Size},
};
use svg_dom_graph::{
    NodeId,
    scene::{CollisionPolicy, ConnectorOptions, ConnectorType, DragOptions, InputMode, Scene, ViewTransform},
};

const SVG_NS: &str = "http://www.w3.org/2000/svg";
const HOST_ID: &str = "perf-interactive-svg";

thread_local! {
    static STAGE: RefCell<Option<Stage>> = const { RefCell::new(None) };
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
struct Stage {
    scene: Option<Scene>,
    svg: web_sys::Element,
    nodes: BTreeMap<String, NodeId>,
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
fn error(message: impl std::fmt::Display) -> String {
    serde_json::json!({ "error": message.to_string() }).to_string()
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Builds the scene of an interaction manifest at `scale_percent`, checks it against the model, and says what it made.
///
/// The host is as many pixels wide as its view box, so a pixel of the pointer is a unit of the scene at a scale of 100%. Every
/// node is draggable with collisions allowed, so that where a drag ends is the pointer's doing and not a correction's.
pub fn build(manifest_json: &str, scale_percent: u32) -> String {
    teardown();
    let manifest = match serde_json::from_str(manifest_json)
        .map_err(|e| e.to_string())
        .and_then(|v| Manifest::from_json(v).map_err(|e| e.to_string()))
    {
        Ok(manifest) => manifest,
        Err(message) => return error(message),
    };
    if manifest.workload != Workload::Interaction {
        return error("only the interaction workload is driven with input");
    }
    match build_stage(&manifest, scale_percent) {
        Ok(summary) => summary.to_string(),
        Err(message) => {
            teardown();
            error(message)
        },
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
fn build_stage(manifest: &Manifest, scale_percent: u32) -> Result<serde_json::Value, String> {
    let document = web_sys::window().and_then(|w| w.document()).ok_or("there is no document")?;
    let (width, height) = field_for(manifest.graph.nodes);
    let svg = document.create_element_ns(Some(SVG_NS), "svg").map_err(|e| format!("{e:?}"))?;
    for (name, value) in [
        ("id", HOST_ID.to_string()),
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

    let mut model = Model::new();
    let mut scene: Option<Scene> = None;
    let mut nodes: BTreeMap<String, NodeId> = BTreeMap::new();
    let mut edge_keys: Vec<String> = Vec::new();
    let mut verdict = serde_json::json!(null);
    let mut hasher = StreamHasher::new();
    let straight = ConnectorOptions::default().with_connector_type(ConnectorType::Straight);
    let draggable = DragOptions::default().with_collision(CollisionPolicy::Allow);

    generate(manifest, &mut |op: &Op| {
        model.apply(op)?;
        hasher.push(op);
        match op {
            Op::CreateScene => {
                scene =
                    Some(Scene::new(SvgRoot::attach(HOST_ID).map_err(|e| e.to_string())?).map_err(|e| e.to_string())?);
            },
            Op::AddNode { key, x, y, w, h, label } => {
                let scene = scene.as_ref().ok_or("there is no scene")?;
                let id = scene
                    .add_node(Point::new(*x as f64, *y as f64), Size::new(*w as f64, *h as f64), label.clone())
                    .map_err(|e| e.to_string())?;
                scene.make_draggable_with(id, draggable).map_err(|e| e.to_string())?;
                nodes.insert(key.clone(), id);
            },
            Op::AddEdge { from, to } => {
                let scene = scene.as_ref().ok_or("there is no scene")?;
                let (a, b) = (
                    *nodes.get(from).ok_or("an unknown node")?,
                    *nodes.get(to).ok_or("an unknown node")?,
                );
                scene.add_edge_with(a, b, straight).map_err(|e| e.to_string())?;
                edge_keys.push(format!("{from}->{to}"));
            },
            Op::Checkpoint { .. } => {
                let scene = scene.as_ref().ok_or("there is no scene")?;
                let expected = model.checkpoint(ViewPolicy::Unchanged)?;
                let actual = observe(&svg, scene, &edge_keys, ViewPolicy::Unchanged)?;
                let differences = compare(&expected, &actual);
                verdict = serde_json::json!({
                    "passed": differences.is_empty(),
                    "differences": differences.iter().take(5).map(|d| d.summary()).collect::<Vec<_>>(),
                });
            },
            _ => return Err("the interaction workload builds and checks, and does nothing else".to_string()),
        }
        Ok(())
    })
    .map_err(|e| e.to_string())?;

    let scene = scene.ok_or("the stream made no scene")?;
    // Both gestures are switched on with no toolbar, so that no control is on the page for the pointer to land on.
    scene.set_pan_mode(InputMode::On).map_err(|e| e.to_string())?;
    scene.set_wheel_zoom_mode(InputMode::On).map_err(|e| e.to_string())?;
    scene
        .set_view(ViewTransform {
            scale: f64::from(scale_percent) / 100.0,
            tx: 0.0,
            ty: 0.0,
        })
        .map_err(|e| e.to_string())?;

    let operations = hasher.operations();
    let (sha256, mutations_sha256) = hasher.finish();
    let summary = serde_json::json!({
        "svg_id": HOST_ID,
        "operations": operations,
        "sha256": sha256,
        "mutations_sha256": mutations_sha256,
        "host": [width, height],
        "nodes": nodes.keys().collect::<Vec<_>>(),
        "checkpoint": verdict,
    });
    STAGE.with_borrow_mut(|stage| *stage = Some(Stage { scene: Some(scene), svg, nodes }));
    Ok(summary)
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
fn with_scene<T>(read: impl FnOnce(&Scene, &Stage) -> Result<T, String>) -> Result<T, String> {
    STAGE.with_borrow(|stage| {
        let stage = stage.as_ref().ok_or("there is no scene on the stage")?;
        let scene = stage.scene.as_ref().ok_or("the scene was dropped")?;
        read(scene, stage)
    })
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The scene's own view, as `{scale, tx, ty}`.
pub fn view() -> String {
    with_scene(|scene, _| {
        let view = scene.view();
        Ok(serde_json::json!({ "scale": view.scale, "tx": view.tx, "ty": view.ty }))
    })
    .map_or_else(error, |value| value.to_string())
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Where the scene says the node `key` is, as `{x, y}`.
pub fn node(key: &str) -> String {
    with_scene(|scene, stage| {
        let id = *stage.nodes.get(key).ok_or_else(|| format!("the node {key} is unknown"))?;
        let rect = scene.node_rect(id).map_err(|e| e.to_string())?;
        Ok(serde_json::json!({ "x": rect.origin.x, "y": rect.origin.y }))
    })
    .map_or_else(error, |value| value.to_string())
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Puts the view back at `scale_percent` with no translation, and each node named in `positions_json`, an object of
/// `key: [x, y]`, back where it was. The runner does this between samples, so that every sample starts from the same scene.
pub fn reset(scale_percent: u32, positions_json: &str) -> String {
    let positions: BTreeMap<String, [f64; 2]> = match serde_json::from_str(positions_json) {
        Ok(positions) => positions,
        Err(e) => return error(e),
    };
    with_scene(|scene, stage| {
        scene
            .set_view(ViewTransform {
                scale: f64::from(scale_percent) / 100.0,
                tx: 0.0,
                ty: 0.0,
            })
            .map_err(|e| e.to_string())?;
        for (key, [x, y]) in positions {
            let id = *stage.nodes.get(&key).ok_or_else(|| format!("the node {key} is unknown"))?;
            scene.move_node(id, Point::new(x, y)).map_err(|e| e.to_string())?;
        }
        Ok(serde_json::json!("ok"))
    })
    .map_or_else(error, |value| value.to_string())
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Drops the scene and removes its host, which is what takes down its listeners, and says `ok`. It is safe to call with
/// nothing on the stage.
pub fn teardown() -> String {
    STAGE.with_borrow_mut(|stage| {
        if let Some(mut taken) = stage.take() {
            taken.scene = None;
            taken.svg.remove();
        }
    });
    "ok".to_string()
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Whether the host element is in the document, for a test that the teardown removed it.
pub fn host_present() -> bool {
    web_sys::window()
        .and_then(|w| w.document())
        .is_some_and(|d| d.get_element_by_id(HOST_ID).is_some())
}
