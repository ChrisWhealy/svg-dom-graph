//! `Scene::node_rect`: a node's own current rendered rectangle, in the `<svg>`'s own user space — the same value
//! this crate itself already tracks internally, exposed so a caller can lay out a node relative to another one
//! whose own rendered size (a data node's, in particular) is not knowable ahead of drawing it.

use crate::common::{attr_f64, check, check_close, dispatch_pointer_event, make_svg, nth_group};
use svg_dom::root::utils::{Point, Size};
use svg_dom_graph::{
    Error,
    scene::{DataFormat, DataNodeContent, NodeValues, Scene},
};
use wasm_bindgen_test::wasm_bindgen_test;

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// A plain node's own `node_rect` is exactly the `top_left`/`size` its own constructor was given — nothing this
/// crate itself computes for it.
#[wasm_bindgen_test]
fn node_rect_of_a_plain_node_is_exactly_what_it_was_constructed_with() -> Result<(), String> {
    let svg = make_svg("node-rect-plain", Size::new(300.0, 200.0), Size::new(300.0, 200.0));
    let scene = Scene::new(svg).map_err(|e| e.to_string())?;
    let node = scene
        .add_node(Point::new(30.0, 40.0), Size::new(90.0, 70.0), "A")
        .map_err(|e| e.to_string())?;

    let rect = scene.node_rect(node).map_err(|e| e.to_string())?;
    check(
        rect.origin == Point::new(30.0, 40.0),
        &format!("unexpected origin: {:?}", rect.origin),
    )?;
    check(rect.size == Size::new(90.0, 70.0), &format!("unexpected size: {:?}", rect.size))
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// A data node's own `node_rect` matches the box this crate itself computed to fit its own content — exactly the
/// `width`/`height` its rendered outer `<rect>` was actually given, not a guess or an estimate. This is the whole
/// point of `node_rect`: a data node's own real size is not knowable ahead of drawing it.
#[wasm_bindgen_test]
fn node_rect_of_a_data_node_matches_its_own_rendered_outer_box() -> Result<(), String> {
    let svg = make_svg("node-rect-data", Size::new(400.0, 260.0), Size::new(400.0, 260.0));
    let scene = Scene::new(svg).map_err(|e| e.to_string())?;
    let content = DataNodeContent::new(NodeValues::U64(vec![0xF0E1D2C3B4A59687]), DataFormat::Hexadecimal);
    let node = scene
        .add_data_node(Point::new(10.0, 20.0), content)
        .map_err(|e| e.to_string())?;

    let rect = scene.node_rect(node).map_err(|e| e.to_string())?;
    check(
        rect.origin == Point::new(10.0, 20.0),
        &format!("unexpected origin: {:?}", rect.origin),
    )?;

    let group = nth_group("node-rect-data", 0)?;
    let outer_rect = group
        .query_selector("rect")
        .map_err(|e| format!("{e:?}"))?
        .ok_or("no outer <rect> found")?;
    check_close(rect.size.width, attr_f64(&outer_rect, "width")?)?;
    check_close(rect.size.height, attr_f64(&outer_rect, "height")?)
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// `node_rect` rejects a `NodeId` that does not belong to this `Scene` — for example, one from a different `Scene`.
#[wasm_bindgen_test]
fn node_rect_rejects_a_node_id_from_a_different_scene() -> Result<(), String> {
    let svg_a = make_svg("node-rect-unknown-a", Size::new(200.0, 200.0), Size::new(200.0, 200.0));
    let scene_a = Scene::new(svg_a).map_err(|e| e.to_string())?;
    let svg_b = make_svg("node-rect-unknown-b", Size::new(200.0, 200.0), Size::new(200.0, 200.0));
    let scene_b = Scene::new(svg_b).map_err(|e| e.to_string())?;
    let foreign = scene_b
        .add_node(Point::new(0.0, 0.0), Size::new(40.0, 20.0), "B")
        .map_err(|e| e.to_string())?;

    check(
        matches!(scene_a.node_rect(foreign), Err(Error::UnknownNode(id)) if id == foreign),
        "node_rect on a foreign NodeId did not fail with UnknownNode",
    )
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// `node_rect` reflects a node's own current position after a drag moves it, not the one it was created with.
#[wasm_bindgen_test]
fn node_rect_reflects_a_nodes_own_position_after_a_drag_moves_it() -> Result<(), String> {
    let svg = make_svg("node-rect-dragged", Size::new(300.0, 200.0), Size::new(300.0, 200.0));
    let scene = Scene::new(svg).map_err(|e| e.to_string())?;
    let node = scene
        .add_node(Point::new(0.0, 0.0), Size::new(40.0, 20.0), "A")
        .map_err(|e| e.to_string())?;
    scene.make_draggable(node).map_err(|e| e.to_string())?;

    let group = nth_group("node-rect-dragged", 0)?;
    // 1:1 client-pixel-to-user-space here — same reasoning `drag_basics.rs`'s own tests already document.
    dispatch_pointer_event(&group, "pointerdown", 10, 10, 1)?;
    dispatch_pointer_event(&group, "pointermove", 60, 50, 1)?;
    dispatch_pointer_event(&group, "pointerup", 60, 50, 1)?;

    let rect = scene.node_rect(node).map_err(|e| e.to_string())?;
    check(
        rect.origin == Point::new(50.0, 40.0),
        &format!("expected the dragged origin (50, 40), got {:?}", rect.origin),
    )
}
