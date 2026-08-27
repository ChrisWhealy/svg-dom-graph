//! Graph and scene construction correctness: self-loop rejection, cross-scene id isolation, and node geometry
//! validation.

use crate::common::{check, connector_count, make_svg, marker_ids, nth_group};
use svg_dom::{
    SvgRoot,
    root::utils::{Point, Size},
};
use svg_dom_graph::{Error, scene::Scene};
use wasm_bindgen_test::wasm_bindgen_test;

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// `add_edge(node, node)` must be rejected, not silently accepted as a zero-length connector.
///
/// Both endpoints of a self-edge share one rectangle, so both ends would anchor at the same point — a real `<path>`
/// element with no visible extent, not a meaningful connector. Checks both that the call returns `Err` and that it
/// drew nothing, since a caller could otherwise still end up with invisible connector elements accumulating in the
/// DOM.
#[wasm_bindgen_test]
fn add_edge_rejects_a_self_loop() -> Result<(), String> {
    let svg = make_svg("self-loop", Size::new(400.0, 260.0), Size::new(400.0, 260.0));

    let scene = Scene::new(svg).map_err(|e| e.to_string())?;
    let a = scene
        .add_node(Point::new(0.0, 0.0), Size::new(90.0, 50.0), "A")
        .map_err(|e| e.to_string())?;

    let result = scene.add_edge(a, a);
    check(result.is_err(), "add_edge(node, node) unexpectedly succeeded")?;
    let message = result.unwrap_err().to_string();
    check(
        message.contains("not yet supported"),
        &format!("unexpected error message for a self-loop: {message:?}"),
    )?;

    check(
        connector_count("self-loop")? == 0,
        "add_edge(node, node) drew a connector despite returning Err",
    )
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// A `NodeId` from a different `Scene` must be rejected with an error, not silently treated as one of this
/// `Scene`'s own nodes.
///
/// Both `Scene`s' graphs number their nodes from zero, so `foreign`'s `NodeId` and this scene's own first `NodeId`
/// share the same internal sequence position — exactly the case a numbering scheme not scoped to its owning
/// `Scene` would confuse.
#[wasm_bindgen_test]
fn a_node_id_from_a_different_scene_is_rejected_not_silently_mismatched() -> Result<(), String> {
    let foreign_svg = make_svg("foreign-scene", Size::new(400.0, 260.0), Size::new(400.0, 260.0));
    let foreign_scene = Scene::new(foreign_svg).map_err(|e| e.to_string())?;
    let foreign = foreign_scene
        .add_node(Point::new(0.0, 0.0), Size::new(90.0, 50.0), "Foreign")
        .map_err(|e| e.to_string())?;

    let svg = make_svg("local-scene", Size::new(400.0, 260.0), Size::new(400.0, 260.0));
    let scene = Scene::new(svg).map_err(|e| e.to_string())?;
    let local = scene
        .add_node(Point::new(0.0, 0.0), Size::new(90.0, 50.0), "Local")
        .map_err(|e| e.to_string())?;

    check(
        scene.add_edge(local, foreign).is_err(),
        "add_edge silently accepted a NodeId from a different Scene",
    )?;

    check(
        scene.make_draggable(foreign).is_err(),
        "make_draggable silently accepted a NodeId from a different Scene",
    )
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// `add_edge(foreign, foreign)` — the same foreign id used for both endpoints — must be reported as an unknown
/// node, not a self-loop.
///
/// `foreign` names no node at all in `scene`, so `UnknownNode` is the correct diagnosis even though `from == to`.
/// Checking membership before the self-loop comparison is what makes this so: reversing that order would report
/// `SelfLoopUnsupported` instead, which is misleading — the id does not belong to this scene at all, let alone
/// name a node connected to itself.
#[wasm_bindgen_test]
fn add_edge_with_the_same_foreign_id_twice_is_reported_as_unknown_not_a_self_loop() -> Result<(), String> {
    let foreign_svg = make_svg("foreign-self-pair", Size::new(400.0, 260.0), Size::new(400.0, 260.0));
    let foreign_scene = Scene::new(foreign_svg).map_err(|e| e.to_string())?;
    let foreign = foreign_scene
        .add_node(Point::new(0.0, 0.0), Size::new(90.0, 50.0), "Foreign")
        .map_err(|e| e.to_string())?;

    let svg = make_svg("local-only", Size::new(400.0, 260.0), Size::new(400.0, 260.0));
    let scene = Scene::new(svg).map_err(|e| e.to_string())?;

    let result = scene.add_edge(foreign, foreign);
    check(result.is_err(), "add_edge(foreign, foreign) unexpectedly succeeded")?;
    let message = result.unwrap_err().to_string();
    check(
        message.contains("does not belong to this Scene"),
        &format!("expected an UnknownNode-style message, got {message:?}"),
    )
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Two `Scene`s sharing one `<svg>` must not collide on their arrow marker's id.
///
/// A hardcoded id such as `"arrow"` would make the second `Scene::new` either fail outright or silently produce a
/// second `<marker id="arrow">`, leaving both `Scene`s' connectors pointing at whichever one the browser resolves
/// `url(#arrow)` to.
#[wasm_bindgen_test]
fn scenes_sharing_one_svg_get_distinct_arrow_marker_ids() -> Result<(), String> {
    let svg1 = make_svg("shared-svg", Size::new(400.0, 260.0), Size::new(400.0, 260.0));
    // A second, independent `SvgRoot` handle attached to the same live `<svg id="shared-svg">` — exactly the
    // "two Scenes exist in one SVG" scenario the marker id needs to survive.
    let svg2 = SvgRoot::attach("shared-svg").map_err(|e| e.to_string())?;

    let _scene1 = Scene::new(svg1).map_err(|e| e.to_string())?;
    let _scene2 = Scene::new(svg2).map_err(|e| e.to_string())?;

    let ids = marker_ids("shared-svg")?;
    check(
        ids.len() == 2,
        &format!("expected exactly 2 <marker> elements, found {}", ids.len()),
    )?;
    check(
        ids[0] != ids[1],
        &format!("both Scenes' arrow markers share the same id: {ids:?}"),
    )
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// `add_node` rejects non-finite or non-positive geometry before drawing anything or touching the graph's model —
/// a rejected call leaves the scene with no rendered `<g>` at all, and a later, valid `add_node` call still lands
/// as the scene's first (and only) node, not a second one after some partially-added ghost.
#[wasm_bindgen_test]
fn add_node_rejects_invalid_geometry_before_touching_the_scene() -> Result<(), String> {
    let svg = make_svg("add-node-geometry", Size::new(400.0, 260.0), Size::new(400.0, 260.0));
    let scene = Scene::new(svg).map_err(|e| e.to_string())?;
    let box_size = Size::new(80.0, 40.0);
    let origin = Point::new(20.0, 20.0);

    let invalid_sizes = [
        Size::new(-80.0, 40.0),             // negative width
        Size::new(80.0, -40.0),             // negative height
        Size::new(0.0, 40.0),               // zero width
        Size::new(80.0, 0.0),               // zero height
        Size::new(f64::NAN, 40.0),          // NaN width
        Size::new(80.0, f64::NAN),          // NaN height
        Size::new(f64::INFINITY, 40.0),     // +inf width
        Size::new(f64::NEG_INFINITY, 40.0), // -inf width
    ];
    for size in invalid_sizes {
        let result = scene.add_node(origin, size, "invalid");
        check(
            matches!(result, Err(Error::InvalidNodeGeometry(_))),
            &format!("size {size:?} should have been rejected as Err(Error::InvalidNodeGeometry(_)), got {result:?}"),
        )?;
    }

    let invalid_origins = [
        Point::new(f64::NAN, 20.0),
        Point::new(20.0, f64::NAN),
        Point::new(f64::INFINITY, 20.0),
        Point::new(20.0, f64::NEG_INFINITY),
    ];
    for invalid_origin in invalid_origins {
        let result = scene.add_node(invalid_origin, box_size, "invalid");
        check(
            matches!(result, Err(Error::InvalidNodeGeometry(_))),
            &format!(
                "origin {invalid_origin:?} should have been rejected as Err(Error::InvalidNodeGeometry(_)), got {result:?}"
            ),
        )?;
    }

    // None of the rejected calls above touched the scene — no <g> has been rendered yet.
    check(
        nth_group("add-node-geometry", 0).is_err(),
        "a rejected add_node call left a <g> rendered in the scene",
    )?;

    // A valid call afterwards still lands as the scene's first node.
    scene.add_node(origin, box_size, "valid").map_err(|e| e.to_string())?;
    nth_group("add-node-geometry", 0)?;
    check(
        nth_group("add-node-geometry", 1).is_err(),
        "expected exactly one <g> after the rejected calls and one valid add_node call",
    )
}
