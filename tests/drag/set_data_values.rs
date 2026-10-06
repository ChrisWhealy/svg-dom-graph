//! `Scene::set_data_values`: replacing the values a multi-value data node shows, in place — the cells keep their size,
//! shape, selection and colours, and each cell's own text and accessible name follow the new values.

use crate::common::{check, make_svg, nth_group};
use svg_dom::root::utils::{Point, Size};
use svg_dom_graph::{
    Error,
    scene::{DataFormat, DataNodeContent, GridLayout, NodeValues, Scene, Selection},
};
use wasm_bindgen::JsCast;
use wasm_bindgen_test::wasm_bindgen_test;

fn cell_texts(group: &web_sys::Element) -> Result<Vec<String>, String> {
    let nodes = group.query_selector_all("text").map_err(|e| format!("{e:?}"))?;
    let mut out = Vec::new();
    for i in 0..nodes.length() {
        let el = nodes
            .get(i)
            .ok_or("query_selector_all reported a longer length than it returned")?
            .dyn_into::<web_sys::Element>()
            .map_err(|_| "text is not an Element".to_owned())?;
        out.push(el.text_content().unwrap_or_default());
    }
    Ok(out)
}

fn grid(id: &str) -> Result<(Scene, svg_dom_graph::NodeId, web_sys::Element), String> {
    let svg = make_svg(id, Size::new(400.0, 260.0), Size::new(400.0, 260.0));
    let scene = Scene::new(svg).map_err(|e| e.to_string())?;
    let node = scene
        .add_data_node(
            Point::new(10.0, 10.0),
            DataNodeContent::new(NodeValues::U8(vec![1, 2, 3, 4, 5, 6]), DataFormat::Decimal)
                .with_layout(GridLayout::Columns(3)),
        )
        .map_err(|e| e.to_string())?;
    Ok((scene, node, nth_group(id, 0)?))
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Each cell's own text and accessible name follow the new values, and the selection survives.
#[wasm_bindgen_test]
fn new_values_rewrite_each_cells_text_and_accessible_name_and_keep_the_selection() -> Result<(), String> {
    let (scene, node, group) = grid("set-values-basic")?;
    scene
        .set_selection(node, Selection::Row { row: 1, col: Some(2) })
        .map_err(|e| e.to_string())?;
    let rect_before = scene.node_rect(node).map_err(|e| e.to_string())?;

    scene
        .set_data_values(node, NodeValues::U8(vec![9, 8, 7, 6, 5, 4]))
        .map_err(|e| e.to_string())?;

    check(
        cell_texts(&group)? == ["9", "8", "7", "6", "5", "4"],
        &format!("{:?}", cell_texts(&group)?),
    )?;
    let named = group
        .query_selector_all("text[aria-label]")
        .map_err(|e| format!("{e:?}"))?
        .get(5)
        .and_then(|n| n.dyn_into::<web_sys::Element>().ok())
        .and_then(|e| e.get_attribute("aria-label"))
        .unwrap_or_default();
    check(named == "row 1, column 2: 4", &named)?;
    check(
        scene.node_rect(node).map_err(|e| e.to_string())? == rect_before,
        "the node's own rect changed",
    )?;
    check(
        group
            .get_attribute("aria-label")
            .unwrap_or_default()
            .contains("row 1 selected, column 2 focused"),
        "the selection description was lost",
    )
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// A different width or count is rejected, and a rejected call changes nothing.
#[wasm_bindgen_test]
fn incompatible_values_are_rejected_and_change_nothing() -> Result<(), String> {
    let (scene, node, group) = grid("set-values-reject")?;
    for bad in [
        NodeValues::U16(vec![1, 2, 3, 4, 5, 6]),
        NodeValues::U8(vec![1, 2, 3]),
        NodeValues::U8(vec![1, 2, 3, 4, 5, 6, 7]),
    ] {
        check(
            matches!(scene.set_data_values(node, bad), Err(Error::IncompatibleNodeValues(_))),
            "an incompatible set of values was accepted",
        )?;
    }
    check(
        cell_texts(&group)? == ["1", "2", "3", "4", "5", "6"],
        "a rejected call changed the cells",
    )
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// A single-value node, an operator result and a plain label node are rejected.
#[wasm_bindgen_test]
fn single_value_and_label_nodes_are_rejected() -> Result<(), String> {
    let svg = make_svg("set-values-single", Size::new(400.0, 260.0), Size::new(400.0, 260.0));
    let scene = Scene::new(svg).map_err(|e| e.to_string())?;
    let single = scene
        .add_data_node(
            Point::new(10.0, 10.0),
            DataNodeContent::new(NodeValues::U8(vec![1]), DataFormat::Decimal),
        )
        .map_err(|e| e.to_string())?;
    let label = scene
        .add_node(Point::new(200.0, 10.0), Size::new(60.0, 30.0), "plain")
        .map_err(|e| e.to_string())?;
    check(
        matches!(
            scene.set_data_values(single, NodeValues::U8(vec![2])),
            Err(Error::IncompatibleNodeValues(_))
        ),
        "a single-value node was accepted",
    )?;
    check(
        matches!(
            scene.set_data_values(label, NodeValues::U8(vec![2])),
            Err(Error::IncompatibleNodeValues(_))
        ),
        "a plain label node was accepted",
    )
}
