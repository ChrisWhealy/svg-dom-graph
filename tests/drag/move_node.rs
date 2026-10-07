//! `Scene::move_node`: repositioning a node already added to a scene — the programmatic counterpart to dragging it, for
//! the case `Scene::node_rect`'s own doc comment describes: laying a node out relative to another whose own rendered
//! size was not knowable ahead of drawing it.
//!
//! These observe the real rendered DOM, queried directly, not through any crate-internal state — the same reasoning
//! [`drag_basics`](super::drag_basics)'s own module doc comment gives.

use crate::common::{
    check, check_close, group_translate, last_point_of_path, make_svg, nth_group, path_d, the_connector,
};
use svg_dom::root::utils::{Point, Rect, Size};
use svg_dom_graph::{
    Error,
    scene::{BinaryOperator, DataFormat, DataNodeContent, GridLayout, NodeValues, Scene},
};
use wasm_bindgen_test::wasm_bindgen_test;

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// `move_node` moves a node's own `<g>` via its `transform`, exactly as a drag does, and reroutes its connector's far
/// end to match — the same assertions
/// [`dragging_a_node_moves_its_rect_label_and_reroutes_its_edge`](super::drag_basics::dragging_a_node_moves_its_rect_label_and_reroutes_its_edge)
/// makes for a live drag, called here programmatically instead.
#[wasm_bindgen_test]
fn move_node_moves_a_plain_nodes_own_rect_and_reroutes_its_edge() -> Result<(), String> {
    let svg = make_svg("move-node-1to1", Size::new(400.0, 260.0), Size::new(400.0, 260.0));

    let a_rect = Rect {
        origin: Point::new(0.0, 0.0),
        size: Size::new(90.0, 50.0),
    };
    let b_rect_before = Rect {
        origin: Point::new(200.0, 150.0),
        size: Size::new(90.0, 50.0),
    };

    let scene = Scene::new(svg).map_err(|e| e.to_string())?;
    let a = scene.add_node(a_rect.origin, a_rect.size, "A").map_err(|e| e.to_string())?;
    let b = scene
        .add_node(b_rect_before.origin, b_rect_before.size, "B")
        .map_err(|e| e.to_string())?;
    scene.add_edge(a, b).map_err(|e| e.to_string())?;

    let group_b = nth_group("move-node-1to1", 1)?; // B was added second.
    let connector = the_connector("move-node-1to1")?;

    let b_rect_after = Rect {
        origin: Point::new(b_rect_before.origin.x + 50.0, b_rect_before.origin.y + 30.0),
        size: b_rect_before.size,
    };
    scene.move_node(b, b_rect_after.origin).map_err(|e| e.to_string())?;

    let (group_x, group_y) = group_translate(&group_b)?;
    check_close(group_x, b_rect_after.origin.x)?;
    check_close(group_y, b_rect_after.origin.y)?;

    let rect = scene.node_rect(b).map_err(|e| e.to_string())?;
    check(
        rect.origin == b_rect_after.origin,
        &format!("unexpected origin: {:?}", rect.origin),
    )?;
    check(rect.size == b_rect_after.size, "move_node must not change a node's own size")?;

    // The connector's B-end rerouted to sit at the midpoint of one of B's new rect's four sides — the same
    // anti-stale-side check `drag_basics`'s own equivalent test makes.
    let (end_x, end_y) = last_point_of_path(&path_d(&connector)?)?;
    let b = b_rect_after;
    let side_midpoints = [
        (b.origin.x, b.origin.y + b.size.height / 2.0),
        (b.origin.x + b.size.width, b.origin.y + b.size.height / 2.0),
        (b.origin.x + b.size.width / 2.0, b.origin.y),
        (b.origin.x + b.size.width / 2.0, b.origin.y + b.size.height),
    ];
    check(
        side_midpoints
            .iter()
            .any(|&(mx, my)| (end_x - mx).abs() < 0.01 && (end_y - my).abs() < 0.01),
        &format!("connector end ({end_x}, {end_y}) is not the midpoint of any side of B's new rect {b:?}"),
    )
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// `move_node` rejects a `NodeId` that does not belong to this `Scene` — the same contract `node_rect` already has, and
/// for the same reason: a foreign id's own size can't be read to validate against either.
#[wasm_bindgen_test]
fn move_node_rejects_a_node_id_from_a_different_scene() -> Result<(), String> {
    let svg_a = make_svg("move-node-unknown-a", Size::new(200.0, 200.0), Size::new(200.0, 200.0));
    let scene_a = Scene::new(svg_a).map_err(|e| e.to_string())?;
    let svg_b = make_svg("move-node-unknown-b", Size::new(200.0, 200.0), Size::new(200.0, 200.0));
    let scene_b = Scene::new(svg_b).map_err(|e| e.to_string())?;
    let foreign = scene_b
        .add_node(Point::new(0.0, 0.0), Size::new(40.0, 20.0), "B")
        .map_err(|e| e.to_string())?;

    check(
        matches!(scene_a.move_node(foreign, Point::new(10.0, 10.0)), Err(Error::UnknownNode(id)) if id == foreign),
        "move_node on a foreign NodeId did not fail with UnknownNode",
    )
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// `move_node` rejects a non-finite `top_left`, leaving the node exactly where it was.
#[wasm_bindgen_test]
fn move_node_rejects_a_non_finite_top_left() -> Result<(), String> {
    let svg = make_svg("move-node-non-finite", Size::new(200.0, 200.0), Size::new(200.0, 200.0));
    let scene = Scene::new(svg).map_err(|e| e.to_string())?;
    let node = scene
        .add_node(Point::new(10.0, 10.0), Size::new(40.0, 20.0), "A")
        .map_err(|e| e.to_string())?;

    for invalid in [
        Point::new(f64::NAN, 10.0),
        Point::new(10.0, f64::NAN),
        Point::new(f64::INFINITY, 10.0),
    ] {
        let result = scene.move_node(node, invalid);
        check(
            matches!(result, Err(Error::InvalidNodeGeometry(_))),
            &format!(
                "top_left {invalid:?} should have been rejected as Err(Error::InvalidNodeGeometry(_)), got {result:?}"
            ),
        )?;
    }

    let rect = scene.node_rect(node).map_err(|e| e.to_string())?;
    check(
        rect.origin == Point::new(10.0, 10.0),
        "a rejected move must leave the node exactly where it was",
    )
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The motivating case: an operator node's own real width is not known until after it is drawn, so it can't be centred
/// under a wider data node at construction time. Added at a placeholder position, measured via `node_rect`, then moved
/// to the computed centre — with its own already-auto-wired input edges rerouted to the new position, not left pointing
/// at the placeholder one.
#[wasm_bindgen_test]
fn move_node_centers_an_operator_node_under_a_wider_data_node_using_its_own_measured_size() -> Result<(), String> {
    let svg = make_svg("move-node-centre", Size::new(600.0, 400.0), Size::new(600.0, 400.0));
    let scene = Scene::new(svg).map_err(|e| e.to_string())?;

    let array = scene
        .add_data_node(
            Point::new(20.0, 20.0),
            DataNodeContent::new(NodeValues::U64(vec![1, 2, 3, 4, 5]), DataFormat::Hexadecimal)
                .with_layout(GridLayout::Rows(1)),
        )
        .map_err(|e| e.to_string())?;
    let array_rect = scene.node_rect(array).map_err(|e| e.to_string())?;

    let op0 = scene
        .add_data_node(
            Point::new(20.0, 150.0),
            DataNodeContent::new(NodeValues::U64(vec![10]), DataFormat::Hexadecimal),
        )
        .map_err(|e| e.to_string())?;
    let op1 = scene
        .add_data_node(
            Point::new(120.0, 150.0),
            DataNodeContent::new(NodeValues::U64(vec![20]), DataFormat::Hexadecimal),
        )
        .map_err(|e| e.to_string())?;

    // Placeholder position: wherever, since the operator's own real width is not known until it is drawn.
    let result_content = DataNodeContent::new(NodeValues::U64(vec![30]), DataFormat::Hexadecimal);
    let op = scene
        .add_binary_operator_node(Point::new(0.0, 250.0), BinaryOperator::Xor, (op0, op1), result_content)
        .map_err(|e| e.to_string())?;
    let op_rect_at_placeholder = scene.node_rect(op).map_err(|e| e.to_string())?;

    let target_x = array_rect.origin.x + (array_rect.size.width - op_rect_at_placeholder.size.width) / 2.0;
    let target = Point::new(target_x, 250.0);
    scene.move_node(op, target).map_err(|e| e.to_string())?;

    let op_rect_after = scene.node_rect(op).map_err(|e| e.to_string())?;
    check(
        op_rect_after.origin == target,
        &format!("unexpected origin after move: {:?}", op_rect_after.origin),
    )?;
    check(
        op_rect_after.size == op_rect_at_placeholder.size,
        "move_node must not change the operator node's own measured size",
    )?;

    let centre_x = op_rect_after.origin.x + op_rect_after.size.width / 2.0;
    check_close(centre_x, array_rect.origin.x + array_rect.size.width / 2.0)?;

    // Both of the operator's own auto-wired input edges, drawn against the placeholder position, rerouted to the new
    // one — neither still ends at a point that would only make sense for the old, discarded position. Exactly two
    // connectors exist (one per operand), both ending at `op`'s own new rect.
    let count = crate::common::connector_count("move-node-centre")?;
    check(
        count == 2,
        &format!("expected exactly 2 connectors (one per operand), found {count}"),
    )?;
    for i in 0..count {
        let connector = crate::common::nth_connector("move-node-centre", i)?;
        let (end_x, end_y) = last_point_of_path(&path_d(&connector)?)?;
        let on_a_side = (end_x - op_rect_after.origin.x).abs() < 0.01
            || (end_x - (op_rect_after.origin.x + op_rect_after.size.width)).abs() < 0.01
            || (end_y - op_rect_after.origin.y).abs() < 0.01
            || (end_y - (op_rect_after.origin.y + op_rect_after.size.height)).abs() < 0.01;
        check(
            on_a_side,
            &format!(
                "connector {i}'s own end ({end_x}, {end_y}) does not sit on any side of the moved operator's new rect {op_rect_after:?}"
            ),
        )?;
    }
    Ok(())
}
