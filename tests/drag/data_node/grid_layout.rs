//! Grid shape: how many rows and columns the cells fall into, an incomplete last row, `GridLayout` overrides and column
//! groups.

use super::support::{rect_children, text_children};
use crate::common::{attr_f64, check, check_close, make_svg, nth_group};
use svg_dom::root::utils::{Point, Size};
use svg_dom_graph::scene::{DataFormat, DataNodeContent, GridLayout, NodeValues, Scene};
use wasm_bindgen_test::wasm_bindgen_test;

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Five values under [`GridLayout::Automatic`] render a `3 x 2` grid, with the last row only half full. See
/// `grid_shape_of_five_values_is_three_rows_of_two_columns` in `model::content::unit_tests`. That is a non-complete
/// final row, rather than the exact multiple of columns every other rendering test here happens to use.
#[wasm_bindgen_test]
fn add_data_node_with_five_values_leaves_the_last_row_incomplete() -> Result<(), String> {
    let svg = make_svg("data-node-five-values", Size::new(400.0, 260.0), Size::new(400.0, 260.0));
    let scene = Scene::new(svg).map_err(|e| e.to_string())?;
    let content = DataNodeContent::new(NodeValues::U8(vec![1, 2, 3, 4, 5]), DataFormat::Decimal);
    scene
        .add_data_node(Point::new(10.0, 10.0), content)
        .map_err(|e| e.to_string())?;

    let group = nth_group("data-node-five-values", 0)?;
    let texts = text_children(&group)?;
    check(texts.len() == 5, &format!("expected 5 texts, found {}", texts.len()))?;

    let rects = rect_children(&group)?;
    check(
        rects.len() == 6,
        &format!("expected 1 outer + 5 inner cell rects, found {}", rects.len()),
    )?;

    let mut xs: Vec<i64> = rects[1..]
        .iter()
        .map(|r| attr_f64(r, "x").map(|x| x.round() as i64))
        .collect::<Result<_, _>>()?;
    let mut ys: Vec<i64> = rects[1..]
        .iter()
        .map(|r| attr_f64(r, "y").map(|y| y.round() as i64))
        .collect::<Result<_, _>>()?;
    xs.sort_unstable();
    xs.dedup();
    ys.sort_unstable();
    ys.dedup();

    check(
        xs.len() == 2,
        &format!("expected 2 distinct column positions for a 3x2 grid, found {}", xs.len()),
    )?;
    check(
        ys.len() == 3,
        &format!("expected 3 distinct row positions for a 3x2 grid, found {}", ys.len()),
    )
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// `DataNodeContent::with_layout(GridLayout::MaxColumns(n))` overrides `GridLayout::Automatic`'s own cell-count-only
/// choice. 8 values render as 2 rows of 4 under `Automatic` — see `grid_shape_of_eight_values_prefers_two_rows_of_four`
/// in `model::content::unit_tests`. `MaxColumns(2)` caps that at 2 columns, giving 4 rows of 2 instead. This is checked
/// by counting each inner cell's own distinct `x`/`y` — the row/column count, not any specific pixel value.
#[wasm_bindgen_test]
fn with_layout_max_columns_overrides_automatics_own_shape() -> Result<(), String> {
    let svg = make_svg("data-node-max-columns", Size::new(400.0, 400.0), Size::new(400.0, 400.0));
    let scene = Scene::new(svg).map_err(|e| e.to_string())?;
    let content = DataNodeContent::new(NodeValues::U8(vec![1, 2, 3, 4, 5, 6, 7, 8]), DataFormat::Decimal)
        .with_layout(GridLayout::MaxColumns(2));
    scene
        .add_data_node(Point::new(10.0, 10.0), content)
        .map_err(|e| e.to_string())?;

    let group = nth_group("data-node-max-columns", 0)?;
    let rects = rect_children(&group)?;
    check(
        rects.len() == 9,
        &format!("expected 1 outer + 8 inner cell rects, found {}", rects.len()),
    )?;

    // rects[0] is the outer box; the 8 inner cells follow.
    let mut xs: Vec<i64> = rects[1..]
        .iter()
        .map(|r| attr_f64(r, "x").map(|x| x.round() as i64))
        .collect::<Result<_, _>>()?;
    let mut ys: Vec<i64> = rects[1..]
        .iter()
        .map(|r| attr_f64(r, "y").map(|y| y.round() as i64))
        .collect::<Result<_, _>>()?;
    xs.sort_unstable();
    xs.dedup();
    ys.sort_unstable();
    ys.dedup();

    check(
        xs.len() == 2,
        &format!("expected 2 distinct column positions under MaxColumns(2), found {}", xs.len()),
    )?;
    check(
        ys.len() == 4,
        &format!("expected 4 distinct row positions under MaxColumns(2), found {}", ys.len()),
    )
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// `with_column_groups(8)` on a 16-column grid widens only the gap after the eighth column. It leaves every row and the
/// cell order alone. It makes the node exactly one group gap wider than the same grid without groups.
#[wasm_bindgen_test]
fn column_groups_widen_only_the_gap_between_groups() -> Result<(), String> {
    let svg = make_svg("data-node-column-groups", Size::new(600.0, 300.0), Size::new(600.0, 300.0));
    let scene = Scene::new(svg).map_err(|e| e.to_string())?;
    let values: Vec<u8> = (0..40).collect();
    let plain = scene
        .add_data_node(
            Point::new(10.0, 10.0),
            DataNodeContent::new(NodeValues::U8(values.clone()), DataFormat::Ascii)
                .with_layout(GridLayout::Columns(16)),
        )
        .map_err(|e| e.to_string())?;
    let grouped = scene
        .add_data_node(
            Point::new(10.0, 120.0),
            DataNodeContent::new(NodeValues::U8(values), DataFormat::Ascii)
                .with_layout(GridLayout::Columns(16))
                .with_column_groups(8),
        )
        .map_err(|e| e.to_string())?;

    let rects = rect_children(&nth_group("data-node-column-groups", 1)?)?;
    // Index 0 is the node's own outer box; every rect after it is one value's cell, in row-major order.
    let cells: Vec<_> = rects.iter().skip(1).collect();
    let cell_x = |i: usize| attr_f64(cells[i], "x");
    let step = cell_x(1)? - cell_x(0)?;
    for col in 1..16 {
        let gap = cell_x(col)? - cell_x(col - 1)?;
        let expected = if col == 8 { step + 8.0 } else { step };
        check_close(gap, expected).map_err(|e| format!("the gap before column {col}: {e}"))?;
    }
    check(
        cell_x(16)? == cell_x(0)?,
        "the second row starts back at the first column's own x",
    )?;

    let plain_width = scene.node_rect(plain).map_err(|e| e.to_string())?.size.width;
    let grouped_width = scene.node_rect(grouped).map_err(|e| e.to_string())?.size.width;
    check_close(grouped_width - plain_width, 8.0)
}
