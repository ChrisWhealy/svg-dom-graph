//! `Scene::set_focus`: a whole-box focus ring, independent of `Scene::set_selection`'s own per-cell highlighting —
//! works on a plain node, an operator node, and a named data node alike, since none of them need a cell to ring.

use crate::common::{check, make_svg, nth_group};
use svg_dom::root::utils::{Point, Size};
use svg_dom_graph::{
    Error,
    scene::{BinaryOperator, DataFormat, DataNodeContent, NodeValues, Scene},
};
use wasm_bindgen_test::wasm_bindgen_test;

/// `(stroke, stroke-width)` of `container_id`'s own `n`th node's outer `<rect>` — the first `rect` in document
/// order under that node's own `<g>`, which is always the outer box itself (see `BoxHandles::outer_rect`'s own
/// doc comment for why that holds for every node kind this crate draws).
fn outer_stroke(container_id: &str, n: u32) -> Result<(String, String), String> {
    let group = nth_group(container_id, n)?;
    let rect = group
        .query_selector("rect")
        .map_err(|e| format!("{e:?}"))?
        .ok_or("no <rect> found under this node's own group")?;
    let stroke = rect.get_attribute("stroke").ok_or("no stroke attribute")?;
    let width = rect.get_attribute("stroke-width").ok_or("no stroke-width attribute")?;
    Ok((stroke, width))
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Focusing a plain node rings its own outer box, and un-focusing restores the plain default — proving this
/// works where `Scene::set_selection` flatly cannot: a plain node has no cell to select at all.
#[wasm_bindgen_test]
fn focusing_a_plain_node_rings_its_own_outer_box() -> Result<(), String> {
    let svg = make_svg("focus-plain", Size::new(400.0, 200.0), Size::new(400.0, 200.0));
    let scene = Scene::new(svg).map_err(|e| e.to_string())?;
    let node = scene
        .add_node(Point::new(20.0, 20.0), Size::new(120.0, 60.0), "A box")
        .map_err(|e| e.to_string())?;

    let (before_stroke, before_width) = outer_stroke("focus-plain", 0)?;

    scene.set_focus(node, true).map_err(|e| e.to_string())?;
    let (focused_stroke, focused_width) = outer_stroke("focus-plain", 0)?;
    check(
        focused_stroke != before_stroke && focused_width != before_width,
        &format!(
            "focusing did not change the outer box's own stroke: still {focused_stroke:?}/{focused_width:?}, \
             same as before ({before_stroke:?}/{before_width:?})"
        ),
    )?;

    scene.set_focus(node, false).map_err(|e| e.to_string())?;
    let (after_stroke, after_width) = outer_stroke("focus-plain", 0)?;
    check(
        after_stroke == before_stroke && after_width == before_width,
        &format!(
            "un-focusing did not restore the original stroke: got {after_stroke:?}/{after_width:?}, expected \
             {before_stroke:?}/{before_width:?}"
        ),
    )
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Focusing an operator node rings its own outer box — the label/value-cell wrapper, not the value cell itself,
/// which `cell_rects` already owns for `Scene::set_selection`'s own unrelated purposes.
#[wasm_bindgen_test]
fn focusing_an_operator_node_rings_its_own_outer_box() -> Result<(), String> {
    let svg = make_svg("focus-operator", Size::new(400.0, 300.0), Size::new(400.0, 300.0));
    let scene = Scene::new(svg).map_err(|e| e.to_string())?;
    let hex = |v: u64| DataNodeContent::new(NodeValues::U64(vec![v]), DataFormat::Hexadecimal);
    let a = scene.add_data_node(Point::new(20.0, 20.0), hex(1)).map_err(|e| e.to_string())?;
    let b = scene
        .add_data_node(Point::new(20.0, 120.0), hex(2))
        .map_err(|e| e.to_string())?;
    let xor = scene
        .add_binary_operator_node(Point::new(200.0, 70.0), BinaryOperator::Xor, (a, b), hex(3))
        .map_err(|e| e.to_string())?;

    let (before_stroke, _) = outer_stroke("focus-operator", 2)?;
    scene.set_focus(xor, true).map_err(|e| e.to_string())?;
    let (focused_stroke, focused_width) = outer_stroke("focus-operator", 2)?;
    check(
        focused_stroke != before_stroke,
        &format!("focusing the operator node did not change its own outer box's stroke from {before_stroke:?}"),
    )?;
    check(
        focused_width == "3",
        &format!("expected a 3-unit focus ring, got stroke-width {focused_width:?}"),
    )
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Focusing a named data node rings the named outer wrapper — not the inner content rect `Scene::set_selection`
/// itself recolours for a single-value node's own focus/band highlighting.
#[wasm_bindgen_test]
fn focusing_a_named_data_node_rings_the_named_wrapper_not_the_content_cell() -> Result<(), String> {
    let svg = make_svg("focus-named", Size::new(400.0, 200.0), Size::new(400.0, 200.0));
    let scene = Scene::new(svg).map_err(|e| e.to_string())?;
    let node = scene
        .add_named_data_node(
            Point::new(20.0, 20.0),
            "Capacity",
            DataNodeContent::new(NodeValues::U8((1..=6).collect()), DataFormat::Decimal),
        )
        .map_err(|e| e.to_string())?;

    let (before_stroke, before_width) = outer_stroke("focus-named", 0)?;
    scene.set_focus(node, true).map_err(|e| e.to_string())?;
    let (focused_stroke, focused_width) = outer_stroke("focus-named", 0)?;
    check(
        focused_stroke != before_stroke && focused_width != before_width,
        "focusing a named data node did not change its own outer wrapper's stroke",
    )
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Any number of nodes can be focused at once — unlike `Scene::show_selection_toolbar`'s own managed `Selection`,
/// there is no single "current" focus this crate tracks, so focusing a second node never un-focuses a first.
#[wasm_bindgen_test]
fn more_than_one_node_can_be_focused_at_once() -> Result<(), String> {
    let svg = make_svg("focus-multi", Size::new(400.0, 300.0), Size::new(400.0, 300.0));
    let scene = Scene::new(svg).map_err(|e| e.to_string())?;
    let a = scene
        .add_node(Point::new(20.0, 20.0), Size::new(100.0, 60.0), "A")
        .map_err(|e| e.to_string())?;
    let b = scene
        .add_node(Point::new(20.0, 120.0), Size::new(100.0, 60.0), "B")
        .map_err(|e| e.to_string())?;

    scene.set_focus(a, true).map_err(|e| e.to_string())?;
    scene.set_focus(b, true).map_err(|e| e.to_string())?;

    let (a_stroke, _) = outer_stroke("focus-multi", 0)?;
    let (b_stroke, _) = outer_stroke("focus-multi", 1)?;
    check(
        a_stroke == b_stroke,
        &format!("expected both focused nodes to share one ring colour, got {a_stroke:?}/{b_stroke:?}"),
    )
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// An unknown `NodeId` is rejected, not silently ignored.
#[wasm_bindgen_test]
fn set_focus_rejects_an_unknown_node() -> Result<(), String> {
    let svg_a = make_svg("focus-unknown-a", Size::new(200.0, 200.0), Size::new(200.0, 200.0));
    let scene_a = Scene::new(svg_a).map_err(|e| e.to_string())?;
    let svg_b = make_svg("focus-unknown-b", Size::new(200.0, 200.0), Size::new(200.0, 200.0));
    let scene_b = Scene::new(svg_b).map_err(|e| e.to_string())?;
    let foreign = scene_b
        .add_node(Point::new(20.0, 20.0), Size::new(80.0, 40.0), "Foreign")
        .map_err(|e| e.to_string())?;

    check(
        matches!(scene_a.set_focus(foreign, true), Err(Error::UnknownNode(_))),
        "set_focus did not reject a NodeId from a different Scene with Err(Error::UnknownNode(_))",
    )
}
