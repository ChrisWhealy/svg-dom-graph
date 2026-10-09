//! A request that changes nothing is skipped as a no-op, except when an earlier call failed part way. Then repeating it
//! finishes the work: the label a selection call could not write, or the connectors a `set_edge_anchors` call could not
//! redraw. Each test makes a DOM write fail on purpose, checks the call reports it, then repeats the same request with
//! the write working again.

use crate::common::{FailingWrites, check, make_svg, nth_group, path_d, the_connector};
use svg_dom::root::utils::{Point, Size};
use svg_dom_graph::scene::{DataFormat, DataNodeContent, EdgeAnchors, GridLayout, NodeValues, Scene, Selection};
use wasm_bindgen_test::wasm_bindgen_test;

fn grid(id: &str) -> Result<(Scene, svg_dom_graph::NodeId, web_sys::Element), String> {
    let svg = make_svg(id, Size::new(400.0, 260.0), Size::new(400.0, 260.0));
    let scene = Scene::new(svg).map_err(|e| e.to_string())?;
    let node = scene
        .add_data_node(
            Point::new(10.0, 10.0),
            DataNodeContent::new(NodeValues::U8((1..=10).collect()), DataFormat::Decimal)
                .with_layout(GridLayout::Columns(5)),
        )
        .map_err(|e| e.to_string())?;
    Ok((scene, node, nth_group(id, 0)?))
}

fn label(group: &web_sys::Element) -> String {
    group.get_attribute("aria-label").unwrap_or_default()
}

#[wasm_bindgen_test]
fn repeating_a_selection_whose_label_failed_to_write_writes_the_label() -> Result<(), String> {
    let (scene, node, group) = grid("retry-selection")?;
    let before = label(&group);
    {
        let _failing = FailingWrites::start(&["aria-label"])?;
        let result = scene.set_selection(node, Selection::Cell(3));
        check(result.is_err(), "the injected label write failure should be reported")?;
    }
    check(
        label(&group) == before,
        "the label should not have changed while its write failed",
    )?;
    scene.set_selection(node, Selection::Cell(3)).map_err(|e| e.to_string())?;
    check(
        label(&group) != before && label(&group).contains('3'),
        &format!("repeating the selection should have written its label, got {:?}", label(&group)),
    )
}

#[wasm_bindgen_test]
fn repeating_secondary_cells_whose_label_failed_to_write_writes_the_label() -> Result<(), String> {
    let (scene, node, group) = grid("retry-secondary")?;
    {
        let _failing = FailingWrites::start(&["aria-label"])?;
        check(
            scene.set_secondary_selection(node, &[2, 4]).is_err(),
            "the injected failure should be reported",
        )?;
    }
    check(
        !label(&group).contains("also highlighted"),
        "the label should not mention them yet",
    )?;
    scene.set_secondary_selection(node, &[2, 4]).map_err(|e| e.to_string())?;
    check(label(&group).contains("also highlighted"), &format!("got {:?}", label(&group)))?;
    // The same call in another order is the same set, and is a no-op once the label is right.
    scene.set_secondary_selection(node, &[4, 2]).map_err(|e| e.to_string())?;
    check(
        label(&group).contains("also highlighted"),
        "a repeat should leave the label as it is",
    )
}

#[wasm_bindgen_test]
fn repeating_unreached_cells_whose_label_failed_to_write_writes_the_label() -> Result<(), String> {
    let (scene, node, group) = grid("retry-unreached")?;
    {
        let _failing = FailingWrites::start(&["aria-label"])?;
        check(
            scene.set_unreached_cells(node, &[5, 6, 7]).is_err(),
            "the injected failure should be reported",
        )?;
    }
    check(
        !label(&group).contains("not yet computed"),
        "the label should not mention them yet",
    )?;
    scene.set_unreached_cells(node, &[5, 6, 7]).map_err(|e| e.to_string())?;
    check(label(&group).contains("not yet computed"), &format!("got {:?}", label(&group)))
}

#[wasm_bindgen_test]
fn repeating_edge_anchors_whose_redraw_failed_redraws_the_connectors() -> Result<(), String> {
    let svg = make_svg("retry-anchors", Size::new(400.0, 260.0), Size::new(400.0, 260.0));
    let scene = Scene::new(svg).map_err(|e| e.to_string())?;
    let a = scene
        .add_node(Point::new(0.0, 0.0), Size::new(40.0, 20.0), "A")
        .map_err(|e| e.to_string())?;
    let b = scene
        .add_node(Point::new(60.0, 100.0), Size::new(40.0, 20.0), "B")
        .map_err(|e| e.to_string())?;
    scene.add_edge(a, b).map_err(|e| e.to_string())?;
    let connector = the_connector("retry-anchors")?;
    let before = path_d(&connector)?;
    {
        let _failing = FailingWrites::start(&["d"])?;
        let result = scene.set_edge_anchors(a, Some(EdgeAnchors(3)));
        check(result.is_err(), "the injected connector write failure should be reported")?;
    }
    check(
        path_d(&connector)? == before,
        "the connector should not have been redrawn while its write failed",
    )?;
    // The same anchors again must not be skipped as already applied.
    scene.set_edge_anchors(a, Some(EdgeAnchors(3))).map_err(|e| e.to_string())?;
    let after = path_d(&connector)?;
    check(
        after != before,
        &format!("repeating the anchors should have redrawn the connector, still {after:?}"),
    )
}
