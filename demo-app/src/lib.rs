//! Wasm entry point for `svg-dom-graph`'s demo.
//!
//! Attaches to `<svg id="diagram">` and builds a small demo graph.
//! This crate — not the library — owns every demo-specific decision: which element to attach to, and what graph to
//! build.

use std::cell::RefCell;
use svg_dom::{
    SvgRoot,
    root::utils::{Point, Size},
};
use svg_dom_graph::{Error, scene::Scene};
use wasm_bindgen::prelude::*;

thread_local! {
    // `Scene` is a cheap handle around an `Rc`-shared state, and its own listener closures deliberately hold only
    // `Weak` references back to it. A strong self-reference there would leak the whole scene forever. That means
    // nothing keeps a `Scene` alive once the function that built it returns: a `Scene` created, used, and simply let go
    // out of scope (the natural shape of a `#[wasm_bindgen(start)]` function) drops there and then — long before the
    // user ever gets a chance to click anything.  Thus it silently kills every listener with no panic and no console
    // output.
    //
    // `SCENE` keeps this demo's only `Scene` handle alive for the page's whole lifetime.
    static SCENE: RefCell<Option<Scene>> = const { RefCell::new(None) };
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[wasm_bindgen(start)]
pub fn run() -> Result<(), JsValue> {
    build().map_err(|e| JsValue::from_str(&e.to_string()))
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
fn build() -> Result<(), Error> {
    // Attach to <svg id="diagram"> already present in index.html.
    let svg = SvgRoot::attach("diagram")?;
    build_demo_tree(svg)
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Builds the demo scene: a root box with two children, connected by directed, arrow-tipped edges. This is a minimal
/// directed tree — the simplest case of the general graph `svg-dom-graph` targets.
///
/// The two child boxes are draggable. Their connectors stay attached to the root and redraw as each child moves.
fn build_demo_tree(svg: SvgRoot) -> Result<(), Error> {
    let scene = Scene::new(svg)?;

    let box_size = Size::new(90.0, 50.0);
    let root = scene.add_node(Point::new(155.0, 20.0), box_size, "Root")?;
    let left = scene.add_node(Point::new(25.0, 180.0), box_size, "Left child")?;
    let right = scene.add_node(Point::new(285.0, 180.0), box_size, "Right child")?;

    scene.add_edge(root, left)?;
    scene.add_edge(root, right)?;

    scene.make_draggable(left)?;
    scene.make_draggable(right)?;

    // Keeps this Scene's only strong handle alive for the page's lifetime — see SCENE's own doc comment above.
    SCENE.with_borrow_mut(|slot| *slot = Some(scene));

    Ok(())
}
