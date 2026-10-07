//! `Scene::cell_rect`: the reported rectangle matches the rendered cell, rejects a bad index, and follows the node.

use super::support::rect_children;
use crate::common::{attr_f64, check, check_close, group_translate, make_svg, nth_group};
use svg_dom::root::utils::{Point, Size};
use svg_dom_graph::scene::{DataFormat, DataNodeContent, NodeValues, Scene};
use wasm_bindgen_test::wasm_bindgen_test;

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// `Scene::cell_rect` reports each cell's real rectangle in scene coordinates. It matches the rendered `<rect>`, and
/// follows the node when it moves.
#[wasm_bindgen_test]
fn cell_rect_matches_the_rendered_cell_and_follows_the_node() -> Result<(), String> {
    let svg = make_svg("data-node-cell-rect", Size::new(600.0, 300.0), Size::new(600.0, 300.0));
    let scene = Scene::new(svg).map_err(|e| e.to_string())?;
    let id = scene
        .add_data_node(
            Point::new(30.0, 40.0),
            DataNodeContent::new(NodeValues::U8(vec![1, 2, 3, 4]), DataFormat::Hexadecimal),
        )
        .map_err(|e| e.to_string())?;

    let group = nth_group("data-node-cell-rect", 0)?;
    let rects = rect_children(&group)?;
    let (tx, ty) = group_translate(&group)?;
    for i in 0..4 {
        let cell = scene.cell_rect(id, i).map_err(|e| e.to_string())?;
        check_close(cell.origin.x, tx + attr_f64(&rects[i + 1], "x")?)?;
        check_close(cell.origin.y, ty + attr_f64(&rects[i + 1], "y")?)?;
        check_close(cell.size.width, attr_f64(&rects[i + 1], "width")?)?;
    }
    check(scene.cell_rect(id, 4).is_err(), "an out-of-range index is rejected")?;

    let before = scene.cell_rect(id, 0).map_err(|e| e.to_string())?;
    scene.move_node(id, Point::new(130.0, 90.0)).map_err(|e| e.to_string())?;
    let after = scene.cell_rect(id, 0).map_err(|e| e.to_string())?;
    check_close(after.origin.x - before.origin.x, 100.0)?;
    check_close(after.origin.y - before.origin.y, 50.0)
}
