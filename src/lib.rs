//! `svg-dom-graph` — dynamically re-routable connectors between SVG boxes, built with [`svg_dom`].
//!
//! The eventual goal is arbitrary graphs of boxes.
//! A graph may be cyclic or acyclic, directed or undirected.
//! As boxes move, their connectors stay correctly routed.
//!
//! This first demo keeps that scope minimal: a static directed tree of three boxes.
//! It demonstrates the box/connector rendering and the boundary-routing geometry that later features will extend.
//!
//! See [`geometry::boundary_point`] for the routing math and [`graph::build_demo_tree`] for the demo scene itself.

pub mod geometry;
pub mod graph;

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
    graph::build_demo_tree(&svg)
}
