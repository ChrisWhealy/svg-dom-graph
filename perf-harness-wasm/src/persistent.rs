//! Finds out where the bytes that stay allocated after a teardown come from.
//!
//! After a scene is dropped and its host removed, a few thousand bytes and a few dozen blocks are still allocated, and they are
//! there from the first cycle on and never grow. This runs the lifecycle in stages, each a little more of it than the one
//! before, in a page of its own, so that the cost of each stage's first use shows apart from the others:
//!
//! * `nothing`: the loop and its recording, with no DOM and no library. This is the harness's own cost.
//! * `host`: the host element is made and removed. This is the page and wasm-bindgen's cost.
//! * `scene`: the scene is made on the host and dropped. This adds the library's.
//! * `nodes`: the scene has 12 nodes.
//! * `edges`: the scene has 12 nodes and 24 straight connectors.
//!
//! Each stage is a fresh instance of the module, because what it measures is a first use, and a second stage in the same
//! instance would find the first one's work already done. The difference between one stage and the one before is what that
//! stage's first use leaves behind. A stage that is repeated 30 times and leaves the same bytes after the 30th as after the
//! first has one-time costs only. One that leaves more has a leak, which is what the endurance check is for.

use crate::alloc::stats;
use serde_json::json;
use std::hint::black_box;
use svg_dom::{
    SvgRoot,
    root::utils::{Point, Size},
};
use svg_dom_graph::{
    NodeId,
    scene::{ConnectorOptions, ConnectorType, Scene},
};

/// The nodes of a scene when the caller does not say.
const DEFAULT_NODES: usize = 12;
const SVG_NS: &str = "http://www.w3.org/2000/svg";

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum Stage {
    Nothing,
    Host,
    Scene,
    Nodes,
    Edges,
}

impl Stage {
    fn parse(name: &str) -> Option<Self> {
        Some(match name {
            "nothing" => Self::Nothing,
            "host" => Self::Host,
            "scene" => Self::Scene,
            "nodes" => Self::Nodes,
            "edges" => Self::Edges,
            _ => return None,
        })
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The connectors of the stage `edges`: each node to the next, and each to the one three on.
fn pairs(nodes: usize) -> Vec<(usize, usize)> {
    if nodes < 4 {
        return Vec::new();
    }
    (0..nodes).flat_map(|i| [(i, (i + 1) % nodes), (i, (i + 3) % nodes)]).collect()
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// What a run does before it reads its baseline, which a stage can repeat to see whether that is where a block comes from.
///
/// * `clock`: read `performance.now()` a great many times, as the harness does to find the smallest step it makes.
/// * `tracker`: call the page's listener tracker once, as a count of listeners does.
/// * `both`: the two.
fn prelude(name: &str, window: &web_sys::Window) {
    use wasm_bindgen::JsCast;
    if matches!(name, "clock" | "both") {
        if let Some(performance) = window.performance() {
            let mut last = 0.0;
            for _ in 0..200_000 {
                last = performance.now();
            }
            black_box(last);
        }
    }
    if matches!(name, "tracker" | "both") {
        let tracker = js_sys::Reflect::get(window, &"__listeners".into())
            .ok()
            .filter(|value| !value.is_undefined());
        if let Some(tracker) = tracker {
            if let Some(snapshot) = js_sys::Reflect::get(&tracker, &"snapshot".into())
                .ok()
                .and_then(|f| f.dyn_into::<js_sys::Function>().ok())
            {
                black_box(snapshot.call0(&tracker).ok());
            }
        }
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Runs `cycles` of one stage and gives what the allocator counted after each, as JSON. `before_baseline` is one of the
/// preludes above, or `none`, and is done before the baseline is read.
pub fn run(stage: &str, cycles: usize, before_baseline: &str, nodes: usize) -> String {
    let nodes = if nodes == 0 { DEFAULT_NODES } else { nodes };
    let Some(stage) = Stage::parse(stage) else {
        return json!({"error": format!("there is no stage {stage}")}).to_string();
    };
    let Some(window) = web_sys::window() else {
        return json!({"error": "there is no window"}).to_string();
    };
    let Some(document) = window.document() else {
        return json!({"error": "there is no document"}).to_string();
    };
    let Some(body) = document.body() else {
        return json!({"error": "there is no body"}).to_string();
    };

    // Everything the loop records is made at full size first, so that its own growth cannot be mistaken for what it measures.
    let mut after_each: Vec<(usize, usize)> = Vec::with_capacity(cycles + 1);
    let mut ids: Vec<NodeId> = Vec::with_capacity(nodes);
    let connections = pairs(nodes);
    let options = ConnectorOptions::default().with_connector_type(ConnectorType::Straight);
    prelude(before_baseline, &window);
    let baseline = stats();

    for cycle in 0..cycles {
        let mut host = None;
        if stage >= Stage::Host {
            let id = format!("perf-persistent-{cycle}");
            let Ok(svg) = document.create_element_ns(Some(SVG_NS), "svg") else {
                return json!({"error": "no host could be made"}).to_string();
            };
            let _ = svg.set_attribute("id", &id);
            let _ = svg.set_attribute("width", "800");
            let _ = svg.set_attribute("height", "600");
            let _ = body.append_child(&svg);
            host = Some((svg, id));
        }
        if stage >= Stage::Scene {
            let (_, id) = host.as_ref().expect("the scene stage has a host");
            let scene = match SvgRoot::attach(id)
                .map_err(|e| e.to_string())
                .and_then(|root| Scene::new(root).map_err(|e| e.to_string()))
            {
                Ok(scene) => scene,
                Err(message) => return json!({"error": message}).to_string(),
            };
            ids.clear();
            if stage >= Stage::Nodes {
                for index in 0..nodes {
                    let corner = Point::new(20.0 + (index % 4) as f64 * 140.0, 20.0 + (index / 4) as f64 * 90.0);
                    match scene.add_node(corner, Size::new(80.0, 36.0), format!("n{index}")) {
                        Ok(node) => ids.push(node),
                        Err(error) => return json!({"error": error.to_string()}).to_string(),
                    }
                }
            }
            if stage >= Stage::Edges {
                for &(from, to) in &connections {
                    if let Err(error) = scene.add_edge_with(ids[from], ids[to], options) {
                        return json!({"error": error.to_string()}).to_string();
                    }
                }
            }
            black_box(&scene);
            drop(scene);
        }
        if let Some((svg, _)) = host.take() {
            svg.remove();
        }
        let now = stats();
        after_each.push((now.live_bytes, now.live_allocations));
    }

    let first = after_each.first().copied().unwrap_or((0, 0));
    let last = after_each.last().copied().unwrap_or((0, 0));
    let signed = |a: usize, b: usize| a as i64 - b as i64;
    json!({
        "cycles": cycles,
        "baseline": {"live_bytes": baseline.live_bytes, "live_allocations": baseline.live_allocations},
        // What the first cycle left behind: a first use, whatever it was.
        "after_first_cycle": {
            "bytes_over_baseline": signed(first.0, baseline.live_bytes),
            "blocks_over_baseline": signed(first.1, baseline.live_allocations),
        },
        // What every cycle after the first added. A cost of first use adds nothing here, and a leak adds something each time.
        "growth_after_first_cycle": {
            "bytes": signed(last.0, first.0),
            "blocks": signed(last.1, first.1),
        },
        "live_bytes_by_cycle": after_each.iter().map(|c| c.0).collect::<Vec<_>>(),
        "live_blocks_by_cycle": after_each.iter().map(|c| c.1).collect::<Vec<_>>(),
    })
    .to_string()
}
