//! `Scene::set_secondary_selection`: marking the cells a step derives from its own current selection — independent of
//! `Scene::set_selection`, drawn with both a teal fill and a dashed outline, and described in the node's own
//! `aria-label`.

use crate::common::{check, make_svg};
use svg_dom::root::utils::{Point, Size};
use svg_dom_graph::{
    Error,
    scene::{DataFormat, DataNodeContent, GridLayout, NodeValues, Scene, Selection},
};
use wasm_bindgen::JsCast;
use wasm_bindgen_test::wasm_bindgen_test;

const SECONDARY_FILL: &str = "#7fd8be";
const FOCUS_FILL: &str = "#ff6b4a";
const BAND_FILL: &str = "#ffe066";
const DEFAULT_FILL: &str = "#fdebd3";

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// A 5-column, 2-row `u8` grid in a fresh scene, plus the inner cell `<rect>`s of its own one `<g>`.
fn grid(id: &str) -> Result<(Scene, svg_dom_graph::NodeId, Vec<web_sys::Element>, web_sys::Element), String> {
    let svg = make_svg(id, Size::new(400.0, 260.0), Size::new(400.0, 260.0));
    let scene = Scene::new(svg).map_err(|e| e.to_string())?;
    let node = scene
        .add_data_node(
            Point::new(10.0, 10.0),
            DataNodeContent::new(NodeValues::U8((1..=10).collect()), DataFormat::Decimal)
                .with_layout(GridLayout::Columns(5)),
        )
        .map_err(|e| e.to_string())?;
    let group = crate::common::nth_group(id, 0)?;
    let nodes = group.query_selector_all("rect").map_err(|e| format!("{e:?}"))?;
    let mut rects = Vec::new();
    // Index `0` is the outer box, which is not a cell.
    for i in 1..nodes.length() {
        rects.push(
            nodes
                .get(i)
                .ok_or("query_selector_all reported a longer length than it returned")?
                .dyn_into::<web_sys::Element>()
                .map_err(|_| "rect is not an Element".to_owned())?,
        );
    }
    Ok((scene, node, rects, group))
}

fn attr(cell: &web_sys::Element, name: &str) -> String {
    cell.get_attribute(name).unwrap_or_default()
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// A secondary cell gets its own fill, a dashed outline, and a thicker stroke than the default; clearing restores every
/// one of them.
#[wasm_bindgen_test]
fn secondary_cells_get_their_own_fill_and_a_dashed_outline_and_clearing_restores_them() -> Result<(), String> {
    let (scene, node, cells, _) = grid("secondary-basic")?;
    scene.set_secondary_selection(node, &[1, 2]).map_err(|e| e.to_string())?;

    for (i, cell) in cells.iter().enumerate() {
        let secondary = i == 1 || i == 2;
        check(
            (attr(cell, "fill") == SECONDARY_FILL) == secondary,
            &format!("cell {i}: fill {:?}", attr(cell, "fill")),
        )?;
        check(
            (attr(cell, "stroke-dasharray") == "5 3") == secondary,
            &format!("cell {i}: dash {:?}", attr(cell, "stroke-dasharray")),
        )?;
    }
    check(
        attr(&cells[1], "stroke-width") == "2",
        "secondary cells have a stroke width of 2",
    )?;

    scene.set_secondary_selection(node, &[]).map_err(|e| e.to_string())?;
    for (i, cell) in cells.iter().enumerate() {
        check(attr(cell, "fill") == DEFAULT_FILL, &format!("cell {i} fill after clearing"))?;
        check(
            attr(cell, "stroke-dasharray") != "5 3",
            &format!("cell {i} still dashed after clearing"),
        )?;
        check(
            attr(cell, "stroke-width") == "1",
            &format!("cell {i} stroke width after clearing"),
        )?;
    }
    Ok(())
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// `set_selection` neither clears nor is cleared by the secondary cells, and the toolbar-visible primary `Selection` is
/// untouched by secondary updates. Precedence: focus, then secondary, then band.
#[wasm_bindgen_test]
fn secondary_selection_is_independent_of_the_primary_one_and_ranks_between_focus_and_band() -> Result<(), String> {
    let (scene, node, cells, _) = grid("secondary-precedence")?;
    scene.set_secondary_selection(node, &[0, 3]).map_err(|e| e.to_string())?;
    // Row 0 bands cells 0..5, and focuses cell 0.
    scene
        .set_selection(node, Selection::Row { row: 0, col: Some(0) })
        .map_err(|e| e.to_string())?;

    check(attr(&cells[0], "fill") == FOCUS_FILL, "a focused cell beats secondary")?;
    check(attr(&cells[3], "fill") == SECONDARY_FILL, "a secondary cell beats the band")?;
    check(attr(&cells[1], "fill") == BAND_FILL, "an unrelated banded cell stays banded")?;
    check(attr(&cells[6], "fill") == DEFAULT_FILL, "an unrelated cell stays default")?;

    // Moving the focus off cell 0 reveals its own secondary style underneath.
    scene
        .set_selection(node, Selection::Row { row: 1, col: Some(0) })
        .map_err(|e| e.to_string())?;
    check(
        attr(&cells[0], "fill") == SECONDARY_FILL,
        "cell 0 reverts to secondary, not default",
    )?;
    check(
        attr(&cells[3], "fill") == SECONDARY_FILL,
        "cell 3 is still secondary after the row moved",
    )?;
    check(attr(&cells[5], "fill") == FOCUS_FILL, "the new focus shows")?;

    // Secondary updates leave the primary highlight alone.
    scene.set_secondary_selection(node, &[7]).map_err(|e| e.to_string())?;
    check(attr(&cells[5], "fill") == FOCUS_FILL, "focus unchanged by a secondary update")?;
    check(attr(&cells[0], "fill") == DEFAULT_FILL, "cell 0 leaves the secondary set")?;
    check(attr(&cells[7], "fill") == SECONDARY_FILL, "cell 7 joins it")?;
    Ok(())
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Replacement, duplicates, ordering and an identical set are all handled.
#[wasm_bindgen_test]
fn secondary_selection_replaces_dedups_and_ignores_order() -> Result<(), String> {
    let (scene, node, cells, group) = grid("secondary-replace")?;
    scene.set_secondary_selection(node, &[4, 2, 4]).map_err(|e| e.to_string())?;
    let first_label = attr(&group, "aria-label");
    check(first_label.ends_with(", also highlighted: cells 2, 4"), &first_label)?;

    scene.set_secondary_selection(node, &[2, 4]).map_err(|e| e.to_string())?;
    check(attr(&group, "aria-label") == first_label, "an identical set changes nothing")?;

    scene.set_secondary_selection(node, &[9]).map_err(|e| e.to_string())?;
    check(attr(&cells[2], "fill") == DEFAULT_FILL, "cell 2 left the set")?;
    check(attr(&cells[9], "fill") == SECONDARY_FILL, "cell 9 joined it")?;
    check(
        attr(&group, "aria-label").ends_with(", also highlighted: cell 9"),
        &attr(&group, "aria-label"),
    )?;

    scene.set_secondary_selection(node, &[]).map_err(|e| e.to_string())?;
    check(!attr(&group, "aria-label").contains("also highlighted"), "cleared label")?;
    check(
        group
            .query_selector(":scope > title")
            .map_err(|e| format!("{e:?}"))?
            .and_then(|t| t.text_content())
            .unwrap_or_default()
            == attr(&group, "aria-label"),
        "the tooltip tracks aria-label",
    )?;
    Ok(())
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// A secondary cell is also described after a primary selection, and survives an unrelated primary change.
#[wasm_bindgen_test]
fn secondary_label_follows_the_primary_selection_description() -> Result<(), String> {
    let (scene, node, _, group) = grid("secondary-label")?;
    scene
        .set_selection(node, Selection::Row { row: 0, col: Some(1) })
        .map_err(|e| e.to_string())?;
    scene.set_secondary_selection(node, &[6]).map_err(|e| e.to_string())?;
    let label = attr(&group, "aria-label");
    check(label.contains("row 0 selected, column 1 focused"), &label)?;
    check(label.ends_with(", also highlighted: cell 6"), &label)?;

    scene.set_selection(node, Selection::Cell(3)).map_err(|e| e.to_string())?;
    let label = attr(&group, "aria-label");
    check(label.contains("cell 3 selected"), &label)?;
    check(label.ends_with(", also highlighted: cell 6"), &label)
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Out-of-range indices and a plain label node are rejected, and a rejected call changes nothing.
#[wasm_bindgen_test]
fn secondary_selection_rejects_out_of_range_indices_and_label_nodes() -> Result<(), String> {
    let (scene, node, cells, _) = grid("secondary-validation")?;
    scene.set_secondary_selection(node, &[1]).map_err(|e| e.to_string())?;
    check(
        matches!(
            scene.set_secondary_selection(node, &[2, 10]),
            Err(Error::InvalidSelection(_, Selection::Cell(10)))
        ),
        "index 10 is out of range for 10 cells",
    )?;
    check(
        attr(&cells[1], "fill") == SECONDARY_FILL,
        "the rejected call left cell 1 secondary",
    )?;
    check(attr(&cells[2], "fill") == DEFAULT_FILL, "and did not apply cell 2")?;

    let label = scene
        .add_node(Point::new(200.0, 10.0), Size::new(60.0, 30.0), "plain")
        .map_err(|e| e.to_string())?;
    check(
        matches!(scene.set_secondary_selection(label, &[0]), Err(Error::InvalidSelection(..))),
        "a plain label node has no cells",
    )
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// A single-value node's lone cell is its own outer box; it can be marked secondary too.
#[wasm_bindgen_test]
fn secondary_selection_works_on_a_single_value_node() -> Result<(), String> {
    let svg = make_svg("secondary-single", Size::new(400.0, 260.0), Size::new(400.0, 260.0));
    let scene = Scene::new(svg).map_err(|e| e.to_string())?;
    let node = scene
        .add_data_node(
            Point::new(10.0, 10.0),
            DataNodeContent::new(NodeValues::U8(vec![1]), DataFormat::Decimal),
        )
        .map_err(|e| e.to_string())?;
    let group = crate::common::nth_group("secondary-single", 0)?;
    let rect = group.query_selector("rect").map_err(|e| format!("{e:?}"))?.ok_or("no rect")?;

    scene.set_secondary_selection(node, &[0]).map_err(|e| e.to_string())?;
    check(attr(&rect, "fill") == SECONDARY_FILL, &attr(&rect, "fill"))?;
    check(attr(&rect, "stroke-dasharray") == "5 3", "dashed")?;
    scene.set_secondary_selection(node, &[]).map_err(|e| e.to_string())?;
    check(attr(&rect, "fill") == DEFAULT_FILL, &attr(&rect, "fill"))
}
