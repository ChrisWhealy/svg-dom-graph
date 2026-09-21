//! `Scene::set_selection`: highlighting a cell, row, or column of a [`DataNodeContent`] node — the mechanism behind
//! stepping through an array's values, e.g. via "previous"/"next" controls, and marking where processing currently
//! is.

use crate::common::{attr_f64, check, make_svg};
use svg_dom::root::utils::{Point, Size};
use svg_dom_graph::{
    Error,
    scene::{DataFormat, DataNodeContent, GridLayout, NodeValues, Scene, Selection},
};
use wasm_bindgen::JsCast;
use wasm_bindgen_test::wasm_bindgen_test;

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
fn rect_children(group: &web_sys::Element) -> Result<Vec<web_sys::Element>, String> {
    let nodes = group.query_selector_all("rect").map_err(|e| format!("{e:?}"))?;
    let mut out = Vec::with_capacity(nodes.length() as usize);
    for i in 0..nodes.length() {
        let el = nodes
            .get(i)
            .ok_or("query_selector_all reported a length longer than it could actually return")?
            .dyn_into::<web_sys::Element>()
            .map_err(|_| "rect is not an Element".to_owned())?;
        out.push(el);
    }
    Ok(out)
}

fn fill_of(element: &web_sys::Element) -> Option<String> {
    element.get_attribute("fill")
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// A single-value node has no separate inner cell — `Scene::set_selection`'s own `Selection::Cell(0)` recolours the
/// node's own outer rect directly, and `Selection::None` restores its default type colour.
#[wasm_bindgen_test]
fn cell_selection_on_a_single_value_node_recolours_its_own_outer_rect() -> Result<(), String> {
    let svg = make_svg("selection-single-value", Size::new(400.0, 260.0), Size::new(400.0, 260.0));
    let scene = Scene::new(svg).map_err(|e| e.to_string())?;
    let node = scene
        .add_data_node(
            Point::new(10.0, 10.0),
            DataNodeContent::new(NodeValues::U8(vec![1]), DataFormat::Decimal),
        )
        .map_err(|e| e.to_string())?;

    let group = crate::common::nth_group("selection-single-value", 0)?;
    let rects = rect_children(&group)?;
    check(rects.len() == 1, &format!("expected 1 rect, found {}", rects.len()))?;
    check(
        fill_of(&rects[0]).as_deref() == Some("#fdebd3"),
        &format!("{:?}", fill_of(&rects[0])),
    )?;

    scene.set_selection(node, Selection::Cell(0)).map_err(|e| e.to_string())?;
    check(
        fill_of(&rects[0]).as_deref() == Some("#ff6b4a"),
        &format!("{:?}", fill_of(&rects[0])),
    )?;

    scene.set_selection(node, Selection::None).map_err(|e| e.to_string())?;
    check(
        fill_of(&rects[0]).as_deref() == Some("#fdebd3"),
        &format!("{:?}", fill_of(&rects[0])),
    )
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// A single-row (one-dimensional) grid's own `Selection::Cell(i)` recolours exactly cell `i`, leaving every other
/// cell — and the outer box, which is not itself a "cell" once there is more than one value — untouched.
#[wasm_bindgen_test]
fn cell_selection_on_a_one_dimensional_grid_recolours_exactly_one_cell() -> Result<(), String> {
    let svg = make_svg("selection-1d", Size::new(400.0, 260.0), Size::new(400.0, 260.0));
    let scene = Scene::new(svg).map_err(|e| e.to_string())?;
    let node = scene
        .add_data_node(
            Point::new(10.0, 10.0),
            DataNodeContent::new(NodeValues::U8(vec![1, 2, 3, 4]), DataFormat::Decimal)
                .with_layout(GridLayout::Rows(1)),
        )
        .map_err(|e| e.to_string())?;

    let group = crate::common::nth_group("selection-1d", 0)?;
    let rects = rect_children(&group)?;
    check(
        rects.len() == 5,
        &format!("expected 1 outer + 4 cell rects, found {}", rects.len()),
    )?;

    scene.set_selection(node, Selection::Cell(2)).map_err(|e| e.to_string())?;

    check(
        fill_of(&rects[0]).as_deref() == Some("#eef4ff"),
        "outer box should stay its own plain colour",
    )?;
    for (i, cell) in rects[1..].iter().enumerate() {
        let expected = if i == 2 { "#ff6b4a" } else { "#fdebd3" };
        check(
            fill_of(cell).as_deref() == Some(expected),
            &format!("cell {i}: expected {expected:?}, got {:?}", fill_of(cell)),
        )?;
    }
    Ok(())
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// On a two-dimensional grid, `Selection::Row` bands every cell in that row, and its own optional `col` gets the
/// stronger focus colour instead — the two-tier highlight a data-flow walk over a matrix needs.
#[wasm_bindgen_test]
fn row_selection_bands_the_row_and_focuses_the_named_cell() -> Result<(), String> {
    let svg = make_svg("selection-row", Size::new(400.0, 260.0), Size::new(400.0, 260.0));
    let scene = Scene::new(svg).map_err(|e| e.to_string())?;
    let node = scene
        .add_data_node(
            Point::new(10.0, 10.0),
            DataNodeContent::new(NodeValues::U8(vec![1, 2, 3, 4, 5, 6]), DataFormat::Decimal)
                .with_layout(GridLayout::Rows(2)),
        )
        .map_err(|e| e.to_string())?;

    let group = crate::common::nth_group("selection-row", 0)?;
    let rects = rect_children(&group)?;
    let cells = &rects[1..]; // index 0 is the outer box; cells are flat, row-major: [row0: 0,1,2][row1: 3,4,5]

    scene
        .set_selection(node, Selection::Row { row: 1, col: Some(2) })
        .map_err(|e| e.to_string())?;

    let expected = ["#fdebd3", "#fdebd3", "#fdebd3", "#ffe066", "#ffe066", "#ff6b4a"];
    for (i, cell) in cells.iter().enumerate() {
        check(
            fill_of(cell).as_deref() == Some(expected[i]),
            &format!("cell {i}: expected {:?}, got {:?}", expected[i], fill_of(cell)),
        )?;
    }

    // `Selection::None` resets every cell back to its own default type colour, not just the ones just touched.
    scene.set_selection(node, Selection::None).map_err(|e| e.to_string())?;
    for (i, cell) in cells.iter().enumerate() {
        check(
            fill_of(cell).as_deref() == Some("#fdebd3"),
            &format!("cell {i}: expected the default colour restored, got {:?}", fill_of(cell)),
        )?;
    }
    Ok(())
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// `Selection::Column` bands every cell in that column, and its own optional `row` gets the focus colour — the
/// column-major counterpart to `Row`.
#[wasm_bindgen_test]
fn column_selection_bands_the_column_and_focuses_the_named_cell() -> Result<(), String> {
    let svg = make_svg("selection-column", Size::new(400.0, 260.0), Size::new(400.0, 260.0));
    let scene = Scene::new(svg).map_err(|e| e.to_string())?;
    let node = scene
        .add_data_node(
            Point::new(10.0, 10.0),
            DataNodeContent::new(NodeValues::U8(vec![1, 2, 3, 4, 5, 6]), DataFormat::Decimal)
                .with_layout(GridLayout::Rows(2)),
        )
        .map_err(|e| e.to_string())?;

    let group = crate::common::nth_group("selection-column", 0)?;
    let rects = rect_children(&group)?;
    let cells = &rects[1..];

    scene
        .set_selection(node, Selection::Column { col: 2, row: Some(1) })
        .map_err(|e| e.to_string())?;

    // Column 2 is flat indices 2 and 5; row 1 within it (flat index 5) gets the focus colour.
    let expected = ["#fdebd3", "#fdebd3", "#ffe066", "#fdebd3", "#fdebd3", "#ff6b4a"];
    for (i, cell) in cells.iter().enumerate() {
        check(
            fill_of(cell).as_deref() == Some(expected[i]),
            &format!("cell {i}: expected {:?}, got {:?}", expected[i], fill_of(cell)),
        )?;
    }
    Ok(())
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// `set_selection` rejects an id this scene never issued — the same isolation every other `NodeId`-taking method
/// already guarantees.
#[wasm_bindgen_test]
fn set_selection_rejects_a_node_id_from_a_different_scene() -> Result<(), String> {
    let foreign_svg = make_svg("selection-foreign", Size::new(400.0, 260.0), Size::new(400.0, 260.0));
    let foreign_scene = Scene::new(foreign_svg).map_err(|e| e.to_string())?;
    let foreign = foreign_scene
        .add_data_node(
            Point::new(0.0, 0.0),
            DataNodeContent::new(NodeValues::U8(vec![1]), DataFormat::Decimal),
        )
        .map_err(|e| e.to_string())?;

    let svg = make_svg("selection-local", Size::new(400.0, 260.0), Size::new(400.0, 260.0));
    let scene = Scene::new(svg).map_err(|e| e.to_string())?;

    let outcome = scene.set_selection(foreign, Selection::Cell(0));
    check(
        matches!(outcome, Err(Error::UnknownNode(id)) if id == foreign),
        &format!("expected Err(Error::UnknownNode(foreign)), got {outcome:?}"),
    )
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// A plain label node has no cells to select at all.
#[wasm_bindgen_test]
fn set_selection_rejects_a_plain_label_node() -> Result<(), String> {
    let svg = make_svg("selection-label", Size::new(400.0, 260.0), Size::new(400.0, 260.0));
    let scene = Scene::new(svg).map_err(|e| e.to_string())?;
    let label = scene
        .add_node(Point::new(0.0, 0.0), Size::new(90.0, 50.0), "Not data")
        .map_err(|e| e.to_string())?;

    let outcome = scene.set_selection(label, Selection::Cell(0));
    check(
        matches!(outcome, Err(Error::InvalidSelection(id, Selection::Cell(0))) if id == label),
        &format!("expected Err(Error::InvalidSelection(label, Cell(0))), got {outcome:?}"),
    )
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// An out-of-range `Cell`, and an out-of-range row/column, are each rejected before touching any rendered cell.
#[wasm_bindgen_test]
fn set_selection_rejects_out_of_range_indices() -> Result<(), String> {
    let svg = make_svg("selection-out-of-range", Size::new(400.0, 260.0), Size::new(400.0, 260.0));
    let scene = Scene::new(svg).map_err(|e| e.to_string())?;
    let node = scene
        .add_data_node(
            Point::new(0.0, 0.0),
            DataNodeContent::new(NodeValues::U8(vec![1, 2, 3, 4, 5, 6]), DataFormat::Decimal)
                .with_layout(GridLayout::Rows(2)),
        )
        .map_err(|e| e.to_string())?;

    check(
        scene.set_selection(node, Selection::Cell(6)).is_err(),
        "expected Cell(6) to be rejected for a 6-value node",
    )?;
    check(
        scene.set_selection(node, Selection::Row { row: 2, col: None }).is_err(),
        "expected row 2 to be rejected for a 2-row grid",
    )?;
    check(
        scene.set_selection(node, Selection::Column { col: 0, row: Some(2) }).is_err(),
        "expected an out-of-range row within a valid column to be rejected",
    )
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// A band and a focus are each also distinguishable by stroke width alone, not just by fill colour — the same
/// "not colour alone" reasoning `NodeValues::type_color`'s own `<title>`/`aria-label` pairing already follows.
#[wasm_bindgen_test]
fn selection_gives_band_and_focus_cells_a_thicker_stroke_than_the_default() -> Result<(), String> {
    let svg = make_svg("selection-stroke-width", Size::new(400.0, 260.0), Size::new(400.0, 260.0));
    let scene = Scene::new(svg).map_err(|e| e.to_string())?;
    let node = scene
        .add_data_node(
            Point::new(10.0, 10.0),
            DataNodeContent::new(NodeValues::U8(vec![1, 2, 3, 4, 5, 6]), DataFormat::Decimal)
                .with_layout(GridLayout::Rows(2)),
        )
        .map_err(|e| e.to_string())?;

    let group = crate::common::nth_group("selection-stroke-width", 0)?;
    let rects = rect_children(&group)?;
    let cells = &rects[1..]; // flat, row-major: [row0: 0,1,2][row1: 3,4,5]

    let default_width = attr_f64(&cells[0], "stroke-width")?;

    scene
        .set_selection(node, Selection::Row { row: 1, col: Some(2) })
        .map_err(|e| e.to_string())?;

    let untouched_width = attr_f64(&cells[0], "stroke-width")?;
    let band_width = attr_f64(&cells[3], "stroke-width")?;
    let focus_width = attr_f64(&cells[5], "stroke-width")?;

    check(
        (untouched_width - default_width).abs() < f64::EPSILON,
        &format!("expected an unselected cell to keep its own default stroke width, got {untouched_width}"),
    )?;
    check(
        band_width > default_width,
        &format!("expected a banded cell's stroke to thicken beyond the default {default_width}, got {band_width}"),
    )?;
    check(
        focus_width > band_width,
        &format!("expected the focused cell's stroke ({focus_width}) to thicken beyond the band's own ({band_width})"),
    )?;

    // `Selection::None` restores every cell's own default stroke width, not just its fill.
    scene.set_selection(node, Selection::None).map_err(|e| e.to_string())?;
    for (i, cell) in cells.iter().enumerate() {
        let width = attr_f64(cell, "stroke-width")?;
        check(
            (width - default_width).abs() < f64::EPSILON,
            &format!("cell {i}: expected the default stroke width restored, got {width}"),
        )?;
    }
    Ok(())
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The current selection is also exposed as text, via the node's own `aria-label`, not only through colour and
/// stroke width.
#[wasm_bindgen_test]
fn set_selection_updates_the_nodes_own_aria_label() -> Result<(), String> {
    let svg = make_svg("selection-aria-label", Size::new(400.0, 260.0), Size::new(400.0, 260.0));
    let scene = Scene::new(svg).map_err(|e| e.to_string())?;
    let node = scene
        .add_data_node(
            Point::new(10.0, 10.0),
            DataNodeContent::new(NodeValues::U8(vec![1, 2, 3, 4, 5, 6]), DataFormat::Decimal)
                .with_layout(GridLayout::Rows(2)),
        )
        .map_err(|e| e.to_string())?;

    let group = crate::common::nth_group("selection-aria-label", 0)?;
    let base_label = group
        .get_attribute("aria-label")
        .ok_or("expected an aria-label to already be set at creation")?;

    scene
        .set_selection(node, Selection::Row { row: 1, col: Some(2) })
        .map_err(|e| e.to_string())?;
    let selected_label = group.get_attribute("aria-label").unwrap_or_default();
    check(
        selected_label == format!("{base_label}, row 1 selected, column 2 focused"),
        &format!("got aria-label {selected_label:?}"),
    )?;

    // `Selection::None` restores exactly the original label, with no leftover selection text.
    scene.set_selection(node, Selection::None).map_err(|e| e.to_string())?;
    let cleared_label = group.get_attribute("aria-label").unwrap_or_default();
    check(
        cleared_label == base_label,
        &format!("expected the original aria-label {base_label:?} restored, got {cleared_label:?}"),
    )
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// `set_selection`'s own no-op fast path: an identical selection must touch no cell at all, not merely leave every
/// cell's own rendered value unchanged.
///
/// Proved here by planting a sentinel `fill` directly on a cell via the raw DOM, bypassing `Scene` entirely, right
/// after selecting it once. A call that actually rewrote the cell would overwrite the sentinel back to the focus
/// colour — its surviving an identical second call is the only way to observe, from here, that the call did
/// nothing at all.
#[wasm_bindgen_test]
fn set_selection_with_an_identical_selection_touches_no_cell() -> Result<(), String> {
    let svg = make_svg("selection-no-op", Size::new(400.0, 260.0), Size::new(400.0, 260.0));
    let scene = Scene::new(svg).map_err(|e| e.to_string())?;
    let node = scene
        .add_data_node(
            Point::new(10.0, 10.0),
            DataNodeContent::new(NodeValues::U8(vec![1, 2, 3]), DataFormat::Decimal),
        )
        .map_err(|e| e.to_string())?;

    let group = crate::common::nth_group("selection-no-op", 0)?;
    let rects = rect_children(&group)?;
    let cells = &rects[1..];

    scene.set_selection(node, Selection::Cell(1)).map_err(|e| e.to_string())?;

    const SENTINEL: &str = "sentinel-fill";
    cells[1].set_attribute("fill", SENTINEL).map_err(|e| format!("{e:?}"))?;

    // Same selection again: a true no-op must leave the sentinel in place.
    scene.set_selection(node, Selection::Cell(1)).map_err(|e| e.to_string())?;

    check(
        fill_of(&cells[1]).as_deref() == Some(SENTINEL),
        "an identical selection rewrote a cell it should have left untouched",
    )
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// `set_selection` only rewrites the cells whose own colour/stroke category (focused, banded, default) actually
/// changes between the old selection and the new one — not every cell, even for a genuinely different selection.
///
/// Proved the same way as the no-op case above: a sentinel `fill`, planted directly via the raw DOM, survives a
/// selection change that moves the focus elsewhere but leaves this cell's own category at "default" throughout,
/// then is gone once a further change actually does bring this cell into the band — confirming the sentinel
/// technique itself is sensitive enough to catch a real rewrite, not just an accident of timing.
#[wasm_bindgen_test]
fn set_selection_only_rewrites_cells_whose_own_category_changed() -> Result<(), String> {
    let svg = make_svg("selection-partial-rewrite", Size::new(400.0, 260.0), Size::new(400.0, 260.0));
    let scene = Scene::new(svg).map_err(|e| e.to_string())?;
    let node = scene
        .add_data_node(
            Point::new(10.0, 10.0),
            DataNodeContent::new(NodeValues::U8(vec![1, 2, 3, 4, 5, 6]), DataFormat::Decimal)
                .with_layout(GridLayout::Rows(2)),
        )
        .map_err(|e| e.to_string())?;

    let group = crate::common::nth_group("selection-partial-rewrite", 0)?;
    let rects = rect_children(&group)?;
    let cells = &rects[1..]; // flat, row-major: [row0: 0,1,2][row1: 3,4,5]

    // Cell 0 sits in row 0; neither selection below ever bands or focuses it.
    scene
        .set_selection(node, Selection::Row { row: 1, col: None })
        .map_err(|e| e.to_string())?;

    const SENTINEL: &str = "sentinel-fill";
    cells[0].set_attribute("fill", SENTINEL).map_err(|e| format!("{e:?}"))?;

    // Moves the focused cell within the same banded row; cell 0's own category — plain default — never changes.
    scene
        .set_selection(node, Selection::Row { row: 1, col: Some(2) })
        .map_err(|e| e.to_string())?;
    check(
        fill_of(&cells[0]).as_deref() == Some(SENTINEL),
        "a selection change that never touched cell 0's own category rewrote it anyway",
    )?;

    // Now genuinely bands cell 0: its category does change, so this call must overwrite the sentinel.
    scene
        .set_selection(node, Selection::Row { row: 0, col: None })
        .map_err(|e| e.to_string())?;
    check(
        fill_of(&cells[0]).as_deref() != Some(SENTINEL),
        "cell 0 was newly banded but its sentinel fill was left in place",
    )
}
