//! `ConnectorType` behaviour: corner-radius validation, live updates via `set_connector_type`, clamping to
//! available room, and switching between `Straight` and `Elbow`.

use crate::common::{check, connector_count, dispatch_pointer_event, make_svg, nth_group, path_d, the_connector};
use svg_dom::root::utils::{Point, Size};
use svg_dom_graph::{
    Error,
    scene::{ConnectorOptions, ConnectorType, Scene},
};
use wasm_bindgen_test::wasm_bindgen_test;

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// `Scene::set_connector_type` rejects a non-finite or negative `ConnectorType::Elbow` corner radius, before
/// touching the connector, so a rejected call never corrupts the previously-rendered path.
#[wasm_bindgen_test]
fn set_connector_type_rejects_a_non_finite_or_negative_radius() -> Result<(), String> {
    let svg = make_svg("corner-radius-invalid", Size::new(400.0, 260.0), Size::new(400.0, 260.0));
    let scene = Scene::new(svg).map_err(|e| e.to_string())?;
    let a = scene
        .add_node(Point::new(0.0, 0.0), Size::new(80.0, 40.0), "A")
        .map_err(|e| e.to_string())?;
    let b = scene
        .add_node(Point::new(140.0, 90.0), Size::new(80.0, 40.0), "B")
        .map_err(|e| e.to_string())?;
    let edge = scene.add_edge(a, b).map_err(|e| e.to_string())?;

    let connector = the_connector("corner-radius-invalid")?;
    let before = path_d(&connector)?;

    for corner_radius in [-1.0, f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
        let result = scene.set_connector_type(edge, ConnectorType::Elbow { corner_radius });
        check(
            matches!(result, Err(Error::InvalidCornerRadius(_))),
            &format!(
                "radius {corner_radius} should have been rejected as Err(Error::InvalidCornerRadius(_)), got {result:?}"
            ),
        )?;
    }

    let after = path_d(&connector)?;
    check(
        after == before,
        &format!("a rejected set_connector_type call changed the rendered path: {before:?} -> {after:?}"),
    )
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// `Scene::set_connector_type` rejects an `EdgeId` that does not name an edge in this scene.
#[wasm_bindgen_test]
fn set_connector_type_rejects_an_unknown_edge() -> Result<(), String> {
    let foreign_svg = make_svg("corner-radius-foreign", Size::new(400.0, 260.0), Size::new(400.0, 260.0));
    let foreign_scene = Scene::new(foreign_svg).map_err(|e| e.to_string())?;
    let foreign_a = foreign_scene
        .add_node(Point::new(0.0, 0.0), Size::new(80.0, 40.0), "A")
        .map_err(|e| e.to_string())?;
    let foreign_b = foreign_scene
        .add_node(Point::new(140.0, 90.0), Size::new(80.0, 40.0), "B")
        .map_err(|e| e.to_string())?;
    let foreign_edge = foreign_scene.add_edge(foreign_a, foreign_b).map_err(|e| e.to_string())?;

    let svg = make_svg("corner-radius-local", Size::new(400.0, 260.0), Size::new(400.0, 260.0));
    let scene = Scene::new(svg).map_err(|e| e.to_string())?;

    check(
        matches!(
            scene.set_connector_type(foreign_edge, ConnectorType::Elbow { corner_radius: 10.0 }),
            Err(Error::UnknownEdge(_))
        ),
        "set_connector_type silently accepted an EdgeId from a different Scene",
    )
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// A corner radius too large for the current geometry renders clamped to the available room, with no error. Once a
/// drag gives the connector more room, the very same stored radius renders at its full, unclamped value — proving
/// the clamp recomputes from the current geometry on every redraw, rather than permanently shrinking the setting.
///
/// # Expected paths, worked by hand
///
/// `A` is fixed at `(0, 0)`, size `(80, 40)` — centre `(40, 20)`, half-extents `(40, 20)`.
/// `B` starts at `(140, 90)`, same size — centre `(180, 110)`.
///
/// From `A` toward `B`: `dx = 140`, `dy = 90`. `20 / 90 ≈ 0.222` is smaller than `40 / 140 ≈ 0.286`, so `A` anchors
/// on its south side: `(40, 40)`. The same comparison, from `B` toward `A`, anchors `B` on its north side:
/// `(180, 90)`.
///
/// Both anchors leave vertically and do not share an x coordinate, so the route jogs across their midpoint:
/// `mid_y = (40 + 90) / 2 = 65`. Each of the two vertical segments is only `25` units long, so a requested radius of
/// `30.0` shrinks to `12.5` at both corners.
///
/// Dragging `B` down by `100` moves it to `(140, 190)` — centre `(180, 210)`. `A`'s own anchor is unchanged, since
/// neither `A` nor the direction toward it moved. `B`'s new anchor is `(180, 190)`. The new `mid_y = (40 + 190) / 2
/// = 115`, so both vertical segments are now `75` units long — comfortably past `2 * 30.0` — so the full requested
/// `30.0` now applies at both corners.
#[wasm_bindgen_test]
fn set_connector_type_clamps_to_available_room_and_restores_when_room_returns() -> Result<(), String> {
    let svg = make_svg("corner-radius-clamp", Size::new(400.0, 400.0), Size::new(400.0, 400.0));
    let scene = Scene::new(svg).map_err(|e| e.to_string())?;

    let a = scene
        .add_node(Point::new(0.0, 0.0), Size::new(80.0, 40.0), "A")
        .map_err(|e| e.to_string())?;
    let b = scene
        .add_node(Point::new(140.0, 90.0), Size::new(80.0, 40.0), "B")
        .map_err(|e| e.to_string())?;
    scene.make_draggable(b).map_err(|e| e.to_string())?;

    let edge = scene.add_edge(a, b).map_err(|e| e.to_string())?;
    scene
        .set_connector_type(edge, ConnectorType::Elbow { corner_radius: 30.0 })
        .map_err(|e| e.to_string())?;

    let connector = the_connector("corner-radius-clamp")?;
    let clamped = path_d(&connector)?;
    let expected_clamped = "M 40 40 L 40 52.5 A 12.5 12.5 0 0 0 52.5 65 L 167.5 65 A 12.5 12.5 0 0 1 180 77.5 L 180 90";
    check(
        clamped == expected_clamped,
        &format!("expected the clamped path to be {expected_clamped:?}, got {clamped:?}"),
    )?;

    // Drag B down by 100 units, giving both corners far more than 2 * 30.0 units of headroom.
    let group_b = nth_group("corner-radius-clamp", 1)?; // B was added second.
    dispatch_pointer_event(&group_b, "pointerdown", 180, 110, 1)?;
    dispatch_pointer_event(&group_b, "pointermove", 180, 210, 1)?;
    dispatch_pointer_event(&group_b, "pointerup", 180, 210, 1)?;

    let restored = path_d(&connector)?;
    let expected_restored = "M 40 40 L 40 85 A 30 30 0 0 0 70 115 L 150 115 A 30 30 0 0 1 180 145 L 180 190";
    check(
        restored == expected_restored,
        &format!("expected the restored path to be {expected_restored:?}, got {restored:?}"),
    )
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// `Scene::add_edge_with` rejects a non-finite or negative `ConnectorType::Elbow` corner radius, before drawing
/// anything or touching the graph.
///
/// `Scene::set_connector_type`'s own equivalent test (`set_connector_type_rejects_a_non_finite_or_negative_radius`)
/// already proves this validation rule against an existing edge. `add_edge_with` is the other public entry point that
/// accepts a `ConnectorType`. It shares the same `validate_connector_type` call internally, but that internal sharing
/// is not itself part of this crate's public contract. A direct test here guarantees `add_edge_with`'s own transactional
/// behaviour — a rejected call draws no connector at all — independently of how its validation happens to be implemented.
#[wasm_bindgen_test]
fn add_edge_with_rejects_a_non_finite_or_negative_radius() -> Result<(), String> {
    let svg = make_svg("add-edge-with-invalid-radius", Size::new(400.0, 260.0), Size::new(400.0, 260.0));
    let scene = Scene::new(svg).map_err(|e| e.to_string())?;
    let a = scene
        .add_node(Point::new(0.0, 0.0), Size::new(80.0, 40.0), "A")
        .map_err(|e| e.to_string())?;
    let b = scene
        .add_node(Point::new(140.0, 90.0), Size::new(80.0, 40.0), "B")
        .map_err(|e| e.to_string())?;

    for corner_radius in [-1.0, f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
        let options = ConnectorOptions::default().with_connector_type(ConnectorType::Elbow { corner_radius });
        let result = scene.add_edge_with(a, b, options);
        check(
            matches!(result, Err(Error::InvalidCornerRadius(_))),
            &format!(
                "radius {corner_radius} should have been rejected as Err(Error::InvalidCornerRadius(_)), got {result:?}"
            ),
        )?;
    }

    check(
        connector_count("add-edge-with-invalid-radius")? == 0,
        "add_edge_with drew a connector despite every call returning Err",
    )
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// `ConnectorType::Straight` draws the crate's original, pre-elbow connector style: a straight line between the
/// two boxes' own ray/boundary crossings, not an elbowed route.
///
/// `A` is `(0, 0)`, size `(40, 20)` — centre `(20, 10)`. `B` is `(40, 100)`, same size — centre `(60, 110)`. This is
/// the same pair `straight_vertices_between_diagonal_boxes_lands_on_each_rays_own_crossing` uses in
/// `src/geometry/unit_tests.rs`, so the expected endpoints, `(24, 20)` and `(56, 100)`, come from that test.
#[wasm_bindgen_test]
fn add_edge_with_straight_connector_type_draws_the_original_boundary_to_boundary_line() -> Result<(), String> {
    let svg = make_svg("connector-type-straight", Size::new(300.0, 300.0), Size::new(300.0, 300.0));
    let scene = Scene::new(svg).map_err(|e| e.to_string())?;
    let a = scene
        .add_node(Point::new(0.0, 0.0), Size::new(40.0, 20.0), "A")
        .map_err(|e| e.to_string())?;
    let b = scene
        .add_node(Point::new(40.0, 100.0), Size::new(40.0, 20.0), "B")
        .map_err(|e| e.to_string())?;

    scene
        .add_edge_with(a, b, ConnectorOptions::default().with_connector_type(ConnectorType::Straight))
        .map_err(|e| e.to_string())?;

    let connector = the_connector("connector-type-straight")?;
    let d = path_d(&connector)?;
    check(
        d == "M 24 20 L 56 100",
        &format!("expected a straight line from (24, 20) to (56, 100), got {d:?}"),
    )
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// `Scene::set_connector_type` can switch a live connector between `Straight` and `Elbow`, and back, redrawing it
/// correctly each time — not just adjusting an elbow's own radius.
///
/// Same `A`/`B` pair as `add_edge_with_straight_connector_type_draws_the_original_boundary_to_boundary_line`. The
/// sharp elbow route between them, `(20, 20) -> (20, 60) -> (60, 60) -> (60, 100)`, is worked out the same way
/// `elbow_vertices_between_stacked_boxes_is_one_straight_vertical_segment`'s neighbouring tests in
/// `src/geometry/unit_tests.rs` work out theirs.
#[wasm_bindgen_test]
fn set_connector_type_toggles_a_connector_between_straight_and_elbow() -> Result<(), String> {
    let svg = make_svg("connector-type-toggle", Size::new(300.0, 300.0), Size::new(300.0, 300.0));
    let scene = Scene::new(svg).map_err(|e| e.to_string())?;
    let a = scene
        .add_node(Point::new(0.0, 0.0), Size::new(40.0, 20.0), "A")
        .map_err(|e| e.to_string())?;
    let b = scene
        .add_node(Point::new(40.0, 100.0), Size::new(40.0, 20.0), "B")
        .map_err(|e| e.to_string())?;
    let edge = scene.add_edge(a, b).map_err(|e| e.to_string())?; // default: sharp elbow

    let connector = the_connector("connector-type-toggle")?;
    let elbow_d = path_d(&connector)?;
    let expected_elbow = "M 20 20 L 20 60 L 60 60 L 60 100";
    check(
        elbow_d == expected_elbow,
        &format!("expected the sharp elbow route {expected_elbow:?}, got {elbow_d:?}"),
    )?;

    scene
        .set_connector_type(edge, ConnectorType::Straight)
        .map_err(|e| e.to_string())?;
    let straight_d = path_d(&connector)?;
    check(
        straight_d == "M 24 20 L 56 100",
        &format!("expected the straight route after toggling, got {straight_d:?}"),
    )?;

    scene
        .set_connector_type(edge, ConnectorType::Elbow { corner_radius: 0.0 })
        .map_err(|e| e.to_string())?;
    let elbow_again = path_d(&connector)?;
    check(
        elbow_again == expected_elbow,
        &format!("expected toggling back to Elbow to restore {expected_elbow:?}, got {elbow_again:?}"),
    )
}
