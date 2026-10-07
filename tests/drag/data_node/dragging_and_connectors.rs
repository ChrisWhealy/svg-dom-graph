//! Behaviour shared with every other node: dragging moves the box and every cell, and connectors route to and from a data
//! node, including with custom `EdgeAnchors`.

use super::support::{rect_children, text_children};
use crate::common::{
    attr_f64, check, check_close, dispatch_pointer_event, group_translate, make_svg, nth_group, the_connector,
};
use svg_dom::root::utils::{Point, Size};
use svg_dom_graph::scene::{DataFormat, DataNodeContent, EdgeAnchors, NodeOptions, NodeValues, Scene};
use wasm_bindgen_test::wasm_bindgen_test;

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Dragging a data node moves the whole box — every value's own text and inner cell rect included. This works by moving
/// just the node's own `<g>` `transform`, not by rewriting every cell's own coordinates.
///
/// Every cell is drawn once, at creation, in local coordinates relative to `(0, 0)`. See `draw_content_box`'s own doc
/// comment. So a data node with hundreds of cells moves exactly as cheaply as one with a handful. A pointer move only
/// ever rewrites the group's one `transform`. This checks both halves of that: the group's translate changes by the
/// drag delta, and every cell's own local `x`/`y` stays exactly as it was.
#[wasm_bindgen_test]
fn dragging_a_data_node_moves_the_outer_box_and_every_cell() -> Result<(), String> {
    let svg = make_svg("data-node-drag", Size::new(400.0, 260.0), Size::new(400.0, 260.0));
    let scene = Scene::new(svg).map_err(|e| e.to_string())?;
    // 4 values -> a 2x2 grid (see grid_shape), so four inner cells exist to prove all four moved.
    let content = DataNodeContent::new(NodeValues::U8(vec![1, 2, 3, 4]), DataFormat::Decimal);
    let node = scene
        .add_data_node(Point::new(20.0, 20.0), content)
        .map_err(|e| e.to_string())?;
    scene.make_draggable(node).map_err(|e| e.to_string())?;

    let group = nth_group("data-node-drag", 0)?;
    let rects = rect_children(&group)?;
    let texts = text_children(&group)?;
    check(
        rects.len() == 5,
        &format!("expected 1 outer + 4 inner cell rects, found {}", rects.len()),
    )?;
    check(
        texts.len() == 4,
        &format!("expected 4 texts for a 2x2 grid, found {}", texts.len()),
    )?;

    let group_xy_before = group_translate(&group)?;
    let rect_positions_before: Vec<(f64, f64)> = rects
        .iter()
        .map(|r| Ok::<_, String>((attr_f64(r, "x")?, attr_f64(r, "y")?)))
        .collect::<Result<_, _>>()?;
    let text_positions_before: Vec<(f64, f64)> = texts
        .iter()
        .map(|t| Ok::<_, String>((attr_f64(t, "x")?, attr_f64(t, "y")?)))
        .collect::<Result<_, _>>()?;

    dispatch_pointer_event(&group, "pointerdown", 100, 100, 1)?;
    dispatch_pointer_event(&group, "pointermove", 140, 125, 1)?;
    dispatch_pointer_event(&group, "pointerup", 140, 125, 1)?;

    // The group's own translate moved by exactly the drag delta.
    let (group_x_after, group_y_after) = group_translate(&group)?;
    check_close(group_x_after, group_xy_before.0 + 40.0)?;
    check_close(group_y_after, group_xy_before.1 + 25.0)?;

    // Every cell's own local coordinates are untouched — the move never rewrote a single one of them.
    for (i, rect) in rects.iter().enumerate() {
        let (before_x, before_y) = rect_positions_before[i];
        check_close(attr_f64(rect, "x")?, before_x)?;
        check_close(attr_f64(rect, "y")?, before_y)?;
    }
    for (i, text) in texts.iter().enumerate() {
        let (before_x, before_y) = text_positions_before[i];
        check_close(attr_f64(text, "x")?, before_x)?;
        check_close(attr_f64(text, "y")?, before_y)?;
    }
    Ok(())
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// A connector routes to/from a data node exactly as it would an ordinary label node. It uses the same
/// `boundary_point`/elbow routing logic, since it only ever looks at a node's `Rect`, never its content.
#[wasm_bindgen_test]
fn a_connector_routes_to_a_data_node_like_any_other_node() -> Result<(), String> {
    let svg = make_svg("data-node-connector", Size::new(400.0, 300.0), Size::new(400.0, 300.0));
    let scene = Scene::new(svg).map_err(|e| e.to_string())?;

    let a = scene
        .add_node(Point::new(0.0, 0.0), Size::new(60.0, 30.0), "A")
        .map_err(|e| e.to_string())?;
    let content = DataNodeContent::new(NodeValues::U64(vec![0x1122334455667788]), DataFormat::Hexadecimal);
    let b = scene
        .add_data_node(Point::new(200.0, 150.0), content)
        .map_err(|e| e.to_string())?;
    scene.add_edge(a, b).map_err(|e| e.to_string())?;

    check(
        crate::common::connector_count("data-node-connector")? == 1,
        "expected exactly one connector between a label node and a data node",
    )?;

    let group_b = nth_group("data-node-connector", 1)?; // B was added second.
    let rect_b = &rect_children(&group_b)?[0]; // the outer box — B holds a single value, so it is the only rect.
    // The rect itself is drawn at local (0, 0); B's world-space box origin lives on the group's own transform.
    let (bx, by) = group_translate(&group_b)?;
    let bw = attr_f64(rect_b, "width")?;
    let bh = attr_f64(rect_b, "height")?;

    let d = crate::common::path_d(&the_connector("data-node-connector")?)?;
    let (end_x, end_y) = crate::common::last_point_of_path(&d)?;
    let side_midpoints = [
        (bx, by + bh / 2.0),
        (bx + bw, by + bh / 2.0),
        (bx + bw / 2.0, by),
        (bx + bw / 2.0, by + bh),
    ];
    check(
        side_midpoints
            .iter()
            .any(|&(mx, my)| (end_x - mx).abs() < 0.01 && (end_y - my).abs() < 0.01),
        &format!("connector end ({end_x}, {end_y}) is not the midpoint of any side of the data node's rect"),
    )
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// `EdgeAnchors` configures a data node's own connector fixing points exactly as it would an ordinary node's. Snapping
/// only ever looks at the node's `Rect`, never its content.
///
/// # Expected anchor, worked by hand
///
/// `B` (the data node) holds a single value, so its own box is measured after creation, not assumed. Unconfigured,
/// `edge_anchor` always returns the exact midpoint of whichever side is chosen, regardless of the other endpoint's
/// exact position — see `a_connector_routes_to_a_data_node_like_any_other_node` above, and `edge_anchor`'s own doc
/// comment.
///
/// `A` is placed far enough above and to the left of `B` that `snapped_anchor` still picks `B`'s west side. But the
/// unsnapped ray crosses deep inside its topmost quarter. With `EdgeAnchors(3)`, that side is divided into 4 equal
/// segments. So the connector snaps to the first of the 3 candidates: `B`'s own `(bx, by + bh / 4)`. A plain,
/// unconfigured node would have used the midpoint `(bx, by + bh / 2)` instead.
#[wasm_bindgen_test]
fn a_data_node_with_custom_edge_anchors_snaps_like_any_other_node() -> Result<(), String> {
    let svg = make_svg("data-node-edge-anchors", Size::new(500.0, 300.0), Size::new(500.0, 300.0));
    let scene = Scene::new(svg).map_err(|e| e.to_string())?;

    let content = DataNodeContent::new(NodeValues::U64(vec![0x1122334455667788]), DataFormat::Hexadecimal);
    let options = NodeOptions::default().with_edge_anchors(Some(EdgeAnchors(3)));
    let b = scene
        .add_data_node_with(Point::new(250.0, 150.0), content, options)
        .map_err(|e| e.to_string())?;

    let group_b = nth_group("data-node-edge-anchors", 0)?; // B was added first.
    let rect_b = &rect_children(&group_b)?[0]; // B holds a single value, so it is the only rect.
    let (bx, by) = group_translate(&group_b)?;
    let bw = attr_f64(rect_b, "width")?;
    let bh = attr_f64(rect_b, "height")?;
    let (half_w, half_h) = (bw / 2.0, bh / 2.0);
    let b_centre = Point::new(bx + half_w, by + half_h);

    // `snapped_anchor`'s own crossing formula is `centre.y + dy * (half_w / dx.abs())`. Choosing `dy = -0.9 * (half_h /
    // half_w) * dx.abs()` makes the `dx` term cancel out algebraically. That leaves the crossing point fixed at `0.9 *
    // half_h` above B's own centre, deep inside the topmost quarter of its west side. This holds for any `dx` at all,
    // as long as `dx` stays large enough to keep the west side selected.
    let dx: f64 = -300.0;
    let dy = -0.9 * (half_h / half_w) * dx.abs();
    let a_centre = Point::new(b_centre.x + dx, b_centre.y + dy);
    let a_size = Size::new(60.0, 30.0);
    let a_origin = Point::new(a_centre.x - a_size.width / 2.0, a_centre.y - a_size.height / 2.0);
    let a = scene.add_node(a_origin, a_size, "A").map_err(|e| e.to_string())?;
    scene.add_edge(a, b).map_err(|e| e.to_string())?;

    let d = crate::common::path_d(&the_connector("data-node-edge-anchors")?)?;
    let (end_x, end_y) = crate::common::last_point_of_path(&d)?;
    check_close(end_x, bx)?;
    check_close(end_y, by + bh * 0.25)
}
