//! `svg-dom-graph` — dynamically re-routable connectors between SVG boxes, built with [`svg_dom`].
//!
//! The eventual goal is arbitrary graphs of boxes.
//! A graph may be cyclic or acyclic, directed or undirected.
//! As boxes move, their connectors stay correctly routed.
//!
//! [`model::Graph`] holds the topology: nodes, edges, and incidence, with no DOM dependency of its own.
//! [`scene`] renders a `Graph` onto the DOM, and keeps each node's and edge's rendered SVG handles in sync as nodes
//! move.
//!
//! This first demo keeps scope minimal: a directed tree of three boxes, with the two child boxes draggable.
//!
//! See [`geometry::boundary_point`] for the routing math and [`scene::build_demo_tree`] for the demo scene itself.

pub mod geometry;
pub mod model;
pub mod scene;

use svg_dom::SvgRoot;
use wasm_bindgen::prelude::*;

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[wasm_bindgen(start)]
pub fn run() -> Result<(), JsValue> {
    build().map_err(|e| JsValue::from_str(&e.to_string()))
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
fn build() -> Result<(), svg_dom::Error> {
    // Attach to <svg id="diagram"> already present in index.html.
    let svg = SvgRoot::attach("diagram")?;
    scene::build_demo_tree(&svg)
}
