//! `panel-tree` / `#diagram`: a minimal directed tree with straight connectors. Shows ordinary dragging and
//! connector reroute.

use crate::util::{stringify, view_box_rect};
use std::cell::RefCell;
use svg_dom::{
    SvgRoot,
    root::utils::{Point, Size},
};
use svg_dom_graph::scene::{ConnectorOptions, ConnectorType, DragOptions, Scene};

/// This module's own full source, embedded at compile time — see `crate::source_frame`'s own doc comment for why.
pub(crate) const SOURCE: &str = include_str!("tree.rs");

thread_local! {
    // `Scene` is a cheap handle around an `Rc`-shared state, and its own listener closures deliberately hold only
    // `Weak` references back to it. A strong self-reference there would leak the whole scene forever. That means
    // nothing keeps a `Scene` alive once the function that built it returns: a `Scene` created, used, and simply let go
    // out of scope (the natural shape of a `#[wasm_bindgen(start)]` function), drops there and then — long before the
    // user ever gets a chance to click anything.  Thus it silently kills every listener with no panic and no console
    // output.
    //
    // `SCENE` keeps `build_demo_tree`'s only `Scene` handle alive for the page's whole lifetime.
    static SCENE: RefCell<Option<Scene>> = const { RefCell::new(None) };
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Builds the demo scene: a root box with two children, connected by directed, straight, arrow-tipped edges. This is a
/// minimal directed tree — the simplest case of the general graph `svg-dom-graph` targets.
///
/// Uses [`ConnectorType::Straight`] deliberately, so this first, simplest demo also shows the crate's original
/// connector style — [`crate::elbow::build_elbow_demo`] is where the elbow style, added later, gets its own
/// demonstration.
///
/// The two child boxes are draggable. Their connectors stay attached to the root and redraw as each child moves.
pub(crate) fn build_demo_tree() -> Result<(), String> {
    let svg = SvgRoot::attach("diagram").map_err(stringify)?;
    let bounds = view_box_rect(&svg)?;
    let scene = Scene::new(svg).map_err(stringify)?;

    let box_size = Size::new(90.0, 50.0);
    let root = scene.add_node(Point::new(155.0, 20.0), box_size, "Root").map_err(stringify)?;
    let left = scene
        .add_node(Point::new(25.0, 180.0), box_size, "Left child")
        .map_err(stringify)?;
    let right = scene
        .add_node(Point::new(285.0, 180.0), box_size, "Right child")
        .map_err(stringify)?;

    let straight = ConnectorOptions::default().with_connector_type(ConnectorType::Straight);
    scene.add_edge_with(root, left, straight).map_err(stringify)?;
    scene.add_edge_with(root, right, straight).map_err(stringify)?;

    // Bounded to the diagram's own viewBox: without this, a child dragged past the visible edge and dropped there
    // renders clipped, and can never be clicked to pick up again — see `DragOptions::bounds`'s own doc comment.
    let drag_options = DragOptions::default().with_bounds(Some(bounds));
    scene.make_draggable_with(left, drag_options).map_err(stringify)?;
    scene.make_draggable_with(right, drag_options).map_err(stringify)?;

    // Keeps this Scene's only strong handle alive for the page's lifetime — see SCENE's own doc comment above.
    SCENE.with_borrow_mut(|slot| *slot = Some(scene));

    Ok(())
}
