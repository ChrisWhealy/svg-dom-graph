//! Reads a checkpoint from what was drawn.
//!
//! The runner applies operations through the library's API, and this reads the result back out of the document and from the
//! scene, so that a comparison with the reference model is a comparison with what exists and not with what the runner meant.
//!
//! What it reads, and where:
//!
//! * Node boxes: the `translate(x, y)` of each node's group, and the `width` and `height` of its `rect`.
//! * A node's key: its `<text>`. The workloads label each node with its key, so the text is the key.
//! * Edges: the "Output to" clauses of each node's accessible name, which is how the library says which node feeds which.
//! * Connector ends: the first and last point of each connector's `<path d>`. A connector carries no name of its own, so it
//!   is given the key of the edge made at the same position in the order of creation, which is also its order in the document.
//! * Paint order: the order of the node groups in the document.
//! * The view: the scene's own `view()`.

use crate::parse::{numbers_in, outputs_in};
use perf_harness::manifest::ViewPolicy;
use perf_harness::model::{
    Accessibility, Checkpoint, DomIdentity, EdgeOut, Geometry, GeometryElement, Interaction, Topology, View,
};
use std::collections::BTreeMap;
use svg_dom_graph::scene::Scene;
use wasm_bindgen::JsCast;

/// The tolerance a checkpoint carries for geometry, in scene units.
const GEOMETRY_TOLERANCE: f64 = 0.01;

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[derive(Debug, Default)]
struct NodeSeen {
    key: String,
    x: f64,
    y: f64,
    w: f64,
    h: f64,
    outputs: Vec<String>,
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Reads the checkpoint of the scene drawn in `svg`.
///
/// `edge_keys` are the keys of the edges in the order they were made, which is how the connectors in the document are named.
///
/// # Errors
///
/// Returns a message if the document does not have the shape the library draws, such as a node with no box.
pub fn observe(
    svg: &web_sys::Element,
    scene: &Scene,
    edge_keys: &[String],
    view: ViewPolicy,
) -> Result<Checkpoint, String> {
    let content = svg
        .query_selector("g.svg-dom-graph-content")
        .map_err(|e| format!("{e:?}"))?
        .ok_or("the scene has no content layer")?;

    let mut nodes: Vec<NodeSeen> = Vec::new();
    let mut paths: Vec<String> = Vec::new();
    let children = content.children();
    for index in 0..children.length() {
        let child = children.item(index).ok_or("a child vanished")?;
        match child.local_name().as_str() {
            "g" => {
                let (Some(rect), Some(text)) = (
                    child.query_selector("rect").map_err(|e| format!("{e:?}"))?,
                    child.query_selector("text").map_err(|e| format!("{e:?}"))?,
                ) else {
                    continue;
                };
                let translate = numbers_in(&child.get_attribute("transform").ok_or("a node has no transform")?);
                let size = |name: &str| {
                    rect.get_attribute(name)
                        .and_then(|value| value.parse::<f64>().ok())
                        .ok_or(format!("a node's rect has no {name}"))
                };
                let &[x, y, ..] = translate.as_slice() else {
                    return Err("a node's transform has no x and y".to_string());
                };
                nodes.push(NodeSeen {
                    key: text.text_content().unwrap_or_default(),
                    x,
                    y,
                    w: size("width")?,
                    h: size("height")?,
                    outputs: child
                        .get_attribute("aria-label")
                        .map(|name| outputs_in(&name))
                        .unwrap_or_default(),
                });
            },
            "path" => paths.push(child.get_attribute("d").ok_or("a connector has no path")?),
            _ => {},
        }
    }

    let mut elements: Vec<GeometryElement> = nodes
        .iter()
        .map(|node| GeometryElement {
            key: node.key.clone(),
            kind: "node_box",
            coordinates: vec![node.x, node.y, node.w, node.h],
        })
        .collect();
    for (position, path) in paths.iter().enumerate() {
        let numbers = numbers_in(path);
        let key = edge_keys
            .get(position)
            .cloned()
            .unwrap_or_else(|| format!("connector {position}"));
        // An empty path is a connector that is not drawn, which the library does when its two ends are the same point.
        let coordinates = if numbers.is_empty() {
            Vec::new()
        } else {
            let &[x1, y1, ..] = numbers.as_slice() else {
                return Err(format!("the connector {key} has no start: its path is {path:?}"));
            };
            let &[.., x2, y2] = numbers.as_slice() else {
                return Err(format!("the connector {key} has no end: its path is {path:?}"));
            };
            vec![x1, y1, x2, y2]
        };
        elements.push(GeometryElement {
            key,
            kind: "connector_path",
            coordinates,
        });
    }

    let mut sorted: Vec<&NodeSeen> = nodes.iter().collect();
    sorted.sort_by(|a, b| a.key.cmp(&b.key));
    let edges: Vec<EdgeOut> = nodes
        .iter()
        .flat_map(|node| {
            node.outputs.iter().map(|target| EdgeOut {
                from: node.key.clone(),
                to: target.clone(),
            })
        })
        .collect();

    let mut id_counts: BTreeMap<String, usize> = BTreeMap::new();
    let with_ids = svg.query_selector_all("[id]").map_err(|e| format!("{e:?}"))?;
    for index in 0..with_ids.length() {
        if let Some(element) = with_ids.item(index).and_then(|node| node.dyn_into::<web_sys::Element>().ok()) {
            *id_counts.entry(element.id()).or_insert(0) += 1;
        }
    }

    let focused = web_sys::window()
        .and_then(|window| window.document())
        .and_then(|document| document.active_element())
        .filter(|active| svg.contains(Some(active)))
        .and_then(|active| {
            active.query_selector("text").ok().flatten().or_else(|| {
                active
                    .closest("g")
                    .ok()
                    .flatten()
                    .and_then(|group| group.query_selector("text").ok().flatten())
            })
        })
        .and_then(|text| text.text_content());

    let transform = scene.view();
    Ok(Checkpoint {
        schema_version: 1,
        topology: Topology {
            nodes: sorted.iter().map(|node| node.key.clone()).collect(),
            edges,
            dangling: Vec::new(),
            duplicate_keys: nodes
                .iter()
                .filter(|node| nodes.iter().filter(|other| other.key == node.key).count() > 1)
                .map(|node| node.key.clone())
                .collect(),
        },
        dom_identity: DomIdentity {
            ids: id_counts.keys().cloned().collect(),
            duplicates: id_counts
                .iter()
                .filter(|(_, count)| **count > 1)
                .map(|(id, _)| id.clone())
                .collect(),
        },
        geometry: Geometry {
            tolerance: GEOMETRY_TOLERANCE,
            elements,
        },
        content: nodes.iter().map(|node| (node.key.clone(), node.key.clone())).collect(),
        interaction: Interaction {
            selection: Vec::new(),
            focused,
            view: match view {
                ViewPolicy::Ignored => None,
                _ => Some(View {
                    x: transform.tx,
                    y: transform.ty,
                    scale: transform.scale,
                }),
            },
        },
        accessibility: Accessibility { elements: Vec::new() },
        paint_order: nodes.iter().map(|node| node.key.clone()).collect(),
    })
}
