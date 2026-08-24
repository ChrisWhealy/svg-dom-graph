//! Wasm entry point for `svg-dom-graph`'s demo.
//!
//! Attaches to `<svg id="diagram">` and builds a small demo graph.
//! This crate — not the library — owns every demo-specific decision: which element to attach to, and what graph to
//! build.

use std::{cell::RefCell, rc::Rc};
use svg_dom::{
    Error, SvgRoot,
    root::utils::{Point, Size},
};
use svg_dom_graph::scene::{Scene, make_draggable};
use wasm_bindgen::prelude::*;

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[wasm_bindgen(start)]
pub fn run() -> Result<(), JsValue> {
    build().map_err(|e| JsValue::from_str(&e.to_string()))
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
fn build() -> Result<(), Error> {
    // Attach to <svg id="diagram"> already present in index.html.
    let svg = SvgRoot::attach("diagram")?;
    build_demo_tree(&svg)
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Builds the demo scene: a root box with two children, connected by directed, arrow-tipped edges.
/// This is a minimal directed tree — the simplest case of the general graph `svg-dom-graph` targets.
///
/// The two child boxes are draggable.
/// Their connectors stay attached to the root and redraw as each child moves.
fn build_demo_tree(svg: &SvgRoot) -> Result<(), Error> {
    let scene = Rc::new(RefCell::new(Scene::new(svg)?));

    let box_size = Size::new(90.0, 50.0);
    let root = scene.borrow_mut().add_node(svg, Point::new(155.0, 20.0), box_size, "Root")?;
    let left = scene
        .borrow_mut()
        .add_node(svg, Point::new(25.0, 180.0), box_size, "Left child")?;
    let right = scene
        .borrow_mut()
        .add_node(svg, Point::new(285.0, 180.0), box_size, "Right child")?;

    scene.borrow_mut().add_edge(svg, root, left)?;
    scene.borrow_mut().add_edge(svg, root, right)?;

    make_draggable(&scene, left)?;
    make_draggable(&scene, right)?;

    Ok(())
}
