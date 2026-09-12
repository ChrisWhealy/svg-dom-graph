//! `EdgeAnchors`/`NodeOptions` behaviour: rejecting zero fixing points, snapping each incident edge to its own
//! candidate, and live redraws via `Scene::set_edge_anchors`.

use crate::common::{
    check, check_close, connector_count, first_point_of_path, make_svg, nth_connector, nth_group, path_d, the_connector,
};
use svg_dom::root::utils::{Point, Size};
use svg_dom_graph::{
    Error,
    scene::{EdgeAnchors, NodeOptions, Scene},
};
use wasm_bindgen_test::wasm_bindgen_test;

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// `Scene::add_node_with` rejects `Some(EdgeAnchors(0))` before drawing anything or touching the graph's model — a
/// rejected call leaves the scene with no rendered `<g>` at all, and a later, valid call still lands as the
/// scene's first (and only) node.
#[wasm_bindgen_test]
fn add_node_with_rejects_edge_anchors_zero_before_touching_the_scene() -> Result<(), String> {
    let svg = make_svg(
        "edge-anchors-add-node-invalid",
        Size::new(400.0, 260.0),
        Size::new(400.0, 260.0),
    );
    let scene = Scene::new(svg).map_err(|e| e.to_string())?;
    let options = NodeOptions::default().with_edge_anchors(Some(EdgeAnchors(0)));

    let result = scene.add_node_with(Point::new(20.0, 20.0), Size::new(80.0, 40.0), "invalid", options);
    check(
        matches!(result, Err(Error::InvalidEdgeAnchors(0))),
        &format!("EdgeAnchors(0) should have been rejected as Err(Error::InvalidEdgeAnchors(0)), got {result:?}"),
    )?;
    check(
        nth_group("edge-anchors-add-node-invalid", 0).is_err(),
        "a rejected add_node_with call left a <g> rendered in the scene",
    )?;

    scene
        .add_node(Point::new(20.0, 20.0), Size::new(80.0, 40.0), "valid")
        .map_err(|e| e.to_string())?;
    nth_group("edge-anchors-add-node-invalid", 0)?;
    check(
        nth_group("edge-anchors-add-node-invalid", 1).is_err(),
        "expected exactly one <g> after the rejected call and one valid add_node call",
    )
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// `Scene::set_edge_anchors` rejects `Some(EdgeAnchors(0))` before touching the node, so a rejected call leaves
/// every incident connector rendered exactly as it was.
#[wasm_bindgen_test]
fn set_edge_anchors_rejects_edge_anchors_zero_and_leaves_the_connector_unchanged() -> Result<(), String> {
    let svg = make_svg("edge-anchors-set-invalid", Size::new(400.0, 260.0), Size::new(400.0, 260.0));
    let scene = Scene::new(svg).map_err(|e| e.to_string())?;
    let a = scene
        .add_node(Point::new(0.0, 0.0), Size::new(80.0, 40.0), "A")
        .map_err(|e| e.to_string())?;
    let b = scene
        .add_node(Point::new(140.0, 90.0), Size::new(80.0, 40.0), "B")
        .map_err(|e| e.to_string())?;
    scene.add_edge(a, b).map_err(|e| e.to_string())?;

    let connector = the_connector("edge-anchors-set-invalid")?;
    let before = path_d(&connector)?;

    let result = scene.set_edge_anchors(a, Some(EdgeAnchors(0)));
    check(
        matches!(result, Err(Error::InvalidEdgeAnchors(0))),
        &format!("EdgeAnchors(0) should have been rejected as Err(Error::InvalidEdgeAnchors(0)), got {result:?}"),
    )?;

    let after = path_d(&connector)?;
    check(
        after == before,
        &format!("a rejected set_edge_anchors call changed the rendered path: {before:?} -> {after:?}"),
    )
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// `Scene::set_edge_anchors` rejects a `NodeId` that does not name a node in this scene.
#[wasm_bindgen_test]
fn set_edge_anchors_rejects_an_unknown_node() -> Result<(), String> {
    let foreign_svg = make_svg("edge-anchors-foreign", Size::new(400.0, 260.0), Size::new(400.0, 260.0));
    let foreign_scene = Scene::new(foreign_svg).map_err(|e| e.to_string())?;
    let foreign = foreign_scene
        .add_node(Point::new(0.0, 0.0), Size::new(80.0, 40.0), "Foreign")
        .map_err(|e| e.to_string())?;

    let svg = make_svg("edge-anchors-local", Size::new(400.0, 260.0), Size::new(400.0, 260.0));
    let scene = Scene::new(svg).map_err(|e| e.to_string())?;

    check(
        matches!(
            scene.set_edge_anchors(foreign, Some(EdgeAnchors(2))),
            Err(Error::UnknownNode(_))
        ),
        "set_edge_anchors silently accepted a NodeId from a different Scene",
    )
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// For an elbow connector specifically, `Some(EdgeAnchors(1))` renders in a way that is similar, but not identical, to
/// `None`.  If EdgeConnectors is `Noen`, then the straight connector always binds to the centre of the node.  This in
/// turn means that the location of the connector along the node's edge varies based on the straight line between the
/// centres of the two nodes.
///
/// For `Some(EdgeAnchors(1))` however, a straight connector will always anchors to the centre of the node's edge. This
/// is visually slightly different from the EdgeConnectors `None` case. See the "Fixing Points" demo to see the rendered
/// difference.
#[wasm_bindgen_test]
fn edge_anchors_one_matches_default_elbow_midpoint() -> Result<(), String> {
    let default_svg = make_svg("edge-anchors-one-default", Size::new(300.0, 300.0), Size::new(300.0, 300.0));
    let default_scene = Scene::new(default_svg).map_err(|e| e.to_string())?;
    let a1 = default_scene
        .add_node(Point::new(0.0, 0.0), Size::new(40.0, 20.0), "A")
        .map_err(|e| e.to_string())?;
    let b1 = default_scene
        .add_node(Point::new(40.0, 100.0), Size::new(40.0, 20.0), "B")
        .map_err(|e| e.to_string())?;
    default_scene.add_edge(a1, b1).map_err(|e| e.to_string())?;
    let default_d = path_d(&the_connector("edge-anchors-one-default")?)?;

    let configured_svg = make_svg("edge-anchors-one-configured", Size::new(300.0, 300.0), Size::new(300.0, 300.0));
    let configured_scene = Scene::new(configured_svg).map_err(|e| e.to_string())?;
    let one_anchor = NodeOptions::default().with_edge_anchors(Some(EdgeAnchors(1)));
    let a2 = configured_scene
        .add_node_with(Point::new(0.0, 0.0), Size::new(40.0, 20.0), "A", one_anchor)
        .map_err(|e| e.to_string())?;
    let b2 = configured_scene
        .add_node_with(Point::new(40.0, 100.0), Size::new(40.0, 20.0), "B", one_anchor)
        .map_err(|e| e.to_string())?;
    configured_scene.add_edge(a2, b2).map_err(|e| e.to_string())?;
    let configured_d = path_d(&the_connector("edge-anchors-one-configured")?)?;

    check(
        configured_d == default_d,
        &format!("expected EdgeAnchors(1) to match the default midpoint anchor {default_d:?}, got {configured_d:?}"),
    )
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// `Scene::set_edge_anchors` snaps every incident connector to its own nearest candidate, and redraws each one
/// live. This proves sibling edges sharing one node can land on distinct fixing points, and that a
/// reconfiguration reaches every incident edge, not just the first.
///
/// # Expected anchors, worked by hand
///
/// `C` is `(100, 0)`, size `(80, 40)` — centre `(140, 20)`, half-extents `(40, 20)`.
/// `P` is `(0, 120)`, size `(80, 40)` — centre `(40, 140)`.
/// `Q` is `(200, 120)`, size `(80, 40)` — centre `(240, 140)`.
///
/// Before any `EdgeAnchors` are configured, both edges leave `C` through its south side, at that side's own
/// midpoint: `(140, 40)`, for `C -> P` and `C -> Q` alike. They share this one point because the default rule
/// ignores exactly where each ray happens to cross.
///
/// From `C` toward `P`: `dx = -100`, `dy = 120`. The ray crosses `C`'s south side at `x = 123.33`. With
/// `EdgeAnchors(3)`, `C`'s south side offers three candidates at `x = 120, 140, 160`. `123.33` snaps to `120`.
///
/// From `C` toward `Q`: `dx = 100`, `dy = 120` — the same magnitudes, mirrored. The ray crosses at `x = 156.67`,
/// which snaps to `160`.
#[wasm_bindgen_test]
fn set_edge_anchors_snaps_each_incident_edge_independently_and_redraws_live() -> Result<(), String> {
    let svg = make_svg("edge-anchors-live", Size::new(400.0, 300.0), Size::new(400.0, 300.0));
    let scene = Scene::new(svg).map_err(|e| e.to_string())?;

    let c = scene
        .add_node(Point::new(100.0, 0.0), Size::new(80.0, 40.0), "C")
        .map_err(|e| e.to_string())?;
    let p = scene
        .add_node(Point::new(0.0, 120.0), Size::new(80.0, 40.0), "P")
        .map_err(|e| e.to_string())?;
    let q = scene
        .add_node(Point::new(200.0, 120.0), Size::new(80.0, 40.0), "Q")
        .map_err(|e| e.to_string())?;
    scene.add_edge(c, p).map_err(|e| e.to_string())?; // connector 0
    scene.add_edge(c, q).map_err(|e| e.to_string())?; // connector 1

    let connector_cp = nth_connector("edge-anchors-live", 0)?;
    let connector_cq = nth_connector("edge-anchors-live", 1)?;

    let (before_cp_x, before_cp_y) = first_point_of_path(&path_d(&connector_cp)?)?;
    let (before_cq_x, before_cq_y) = first_point_of_path(&path_d(&connector_cq)?)?;
    check_close(before_cp_x, 140.0)?;
    check_close(before_cp_y, 40.0)?;
    check_close(before_cq_x, 140.0)?;
    check_close(before_cq_y, 40.0)?;

    scene.set_edge_anchors(c, Some(EdgeAnchors(3))).map_err(|e| e.to_string())?;

    let (after_cp_x, after_cp_y) = first_point_of_path(&path_d(&connector_cp)?)?;
    let (after_cq_x, after_cq_y) = first_point_of_path(&path_d(&connector_cq)?)?;
    check_close(after_cp_x, 120.0)?;
    check_close(after_cp_y, 40.0)?;
    check_close(after_cq_x, 160.0)?;
    check_close(after_cq_y, 40.0)?;

    check(
        connector_count("edge-anchors-live")? == 2,
        "set_edge_anchors changed how many connectors are rendered",
    )
}
