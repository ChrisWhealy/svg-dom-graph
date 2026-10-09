//! Selections against a content's grid: `resolve_selection` (including incomplete grids), `ResolvedBand::contains`,
//! `Selection::describe_into`, and `natural_selection`/`flat_index`.

use super::super::*;
use super::support::check_eq;

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
//   resolve_selection
// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -

#[test]
fn resolve_selection_of_none_bands_and_focuses_nothing() -> Result<(), String> {
    let content = DataNodeContent::new(NodeValues::U8(vec![1, 2, 3, 4]), DataFormat::Decimal);
    check_eq(content.resolve_selection(Selection::None), Some((ResolvedBand::None, None)))
}

#[test]
fn resolve_selection_of_an_in_range_cell_focuses_it_alone() -> Result<(), String> {
    let content = DataNodeContent::new(NodeValues::U8(vec![1, 2, 3, 4]), DataFormat::Decimal);
    check_eq(
        content.resolve_selection(Selection::Cell(2)),
        Some((ResolvedBand::None, Some(2))),
    )
}

#[test]
fn resolve_selection_of_an_out_of_range_cell_is_none() -> Result<(), String> {
    let content = DataNodeContent::new(NodeValues::U8(vec![1, 2, 3, 4]), DataFormat::Decimal);
    check_eq(content.resolve_selection(Selection::Cell(4)), None)
}

#[test]
fn resolve_selection_of_a_row_bands_every_cell_in_that_row() -> Result<(), String> {
    // Forced to exactly 2 rows of 3 columns, so the flat-index math is unambiguous: row 1 is indices 3, 4, 5.
    let content = DataNodeContent::new(NodeValues::U8(vec![1, 2, 3, 4, 5, 6]), DataFormat::Decimal)
        .with_layout(GridLayout::Rows(2));
    check_eq(
        content.resolve_selection(Selection::Row { row: 1, col: None }),
        Some((ResolvedBand::Row { row: 1, cols: 3 }, None)),
    )
}

#[test]
fn resolve_selection_of_a_row_with_a_cell_also_focuses_that_one_cell() -> Result<(), String> {
    let content = DataNodeContent::new(NodeValues::U8(vec![1, 2, 3, 4, 5, 6]), DataFormat::Decimal)
        .with_layout(GridLayout::Rows(2));
    check_eq(
        content.resolve_selection(Selection::Row { row: 1, col: Some(2) }),
        Some((ResolvedBand::Row { row: 1, cols: 3 }, Some(5))),
    )
}

#[test]
fn resolve_selection_of_an_out_of_range_row_is_none() -> Result<(), String> {
    let content = DataNodeContent::new(NodeValues::U8(vec![1, 2, 3, 4, 5, 6]), DataFormat::Decimal)
        .with_layout(GridLayout::Rows(2));
    check_eq(content.resolve_selection(Selection::Row { row: 2, col: None }), None)
}

#[test]
fn resolve_selection_of_a_row_with_an_out_of_range_cell_is_none() -> Result<(), String> {
    let content = DataNodeContent::new(NodeValues::U8(vec![1, 2, 3, 4, 5, 6]), DataFormat::Decimal)
        .with_layout(GridLayout::Rows(2));
    check_eq(content.resolve_selection(Selection::Row { row: 0, col: Some(3) }), None)
}

#[test]
fn resolve_selection_of_a_column_bands_every_cell_in_that_column() -> Result<(), String> {
    // Same 2×3 shape: column 2 is indices 2, 5.
    let content = DataNodeContent::new(NodeValues::U8(vec![1, 2, 3, 4, 5, 6]), DataFormat::Decimal)
        .with_layout(GridLayout::Rows(2));
    check_eq(
        content.resolve_selection(Selection::Column { col: 2, row: None }),
        Some((ResolvedBand::Column { col: 2, cols: 3 }, None)),
    )
}

#[test]
fn resolve_selection_of_a_column_with_a_row_also_focuses_that_one_cell() -> Result<(), String> {
    let content = DataNodeContent::new(NodeValues::U8(vec![1, 2, 3, 4, 5, 6]), DataFormat::Decimal)
        .with_layout(GridLayout::Rows(2));
    check_eq(
        content.resolve_selection(Selection::Column { col: 2, row: Some(1) }),
        Some((ResolvedBand::Column { col: 2, cols: 3 }, Some(5))),
    )
}

#[test]
fn resolve_selection_of_an_out_of_range_column_is_none() -> Result<(), String> {
    let content = DataNodeContent::new(NodeValues::U8(vec![1, 2, 3, 4, 5, 6]), DataFormat::Decimal)
        .with_layout(GridLayout::Rows(2));
    check_eq(content.resolve_selection(Selection::Column { col: 3, row: None }), None)
}

#[test]
fn resolve_selection_of_a_column_with_an_out_of_range_row_is_none() -> Result<(), String> {
    let content = DataNodeContent::new(NodeValues::U8(vec![1, 2, 3, 4, 5, 6]), DataFormat::Decimal)
        .with_layout(GridLayout::Rows(2));
    check_eq(content.resolve_selection(Selection::Column { col: 0, row: Some(2) }), None)
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
//   resolve_selection — incomplete grids (a row/column index within shape, but the flat cell it names is blank)
// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -

/// Seven values under `Automatic` render as a 3×3 grid with the last two positions blank:
///
/// ```text
/// 0 1 2
/// 3 4 5
/// 6 - -
/// ```
fn seven_values_as_a_three_by_three_grid() -> DataNodeContent {
    DataNodeContent::new(NodeValues::U8(vec![1, 2, 3, 4, 5, 6, 7]), DataFormat::Decimal)
}

#[test]
fn resolve_selection_of_a_row_focusing_a_real_cell_in_an_incomplete_grid_succeeds() -> Result<(), String> {
    let content = seven_values_as_a_three_by_three_grid();
    // Row 2, column 0 is flat index 6 — the grid's own real last value.
    check_eq(
        content.resolve_selection(Selection::Row { row: 2, col: Some(0) }),
        Some((ResolvedBand::Row { row: 2, cols: 3 }, Some(6))),
    )
}

#[test]
fn resolve_selection_of_a_row_focusing_a_blank_cell_in_an_incomplete_grid_is_none() -> Result<(), String> {
    let content = seven_values_as_a_three_by_three_grid();
    // Row 2, column 1 is flat index 7 — within the 3×3 shape, but past the content's own 7 real values.
    check_eq(content.resolve_selection(Selection::Row { row: 2, col: Some(1) }), None)
}

#[test]
fn resolve_selection_of_a_column_focusing_a_real_cell_in_an_incomplete_grid_succeeds() -> Result<(), String> {
    let content = seven_values_as_a_three_by_three_grid();
    // Column 2, row 1 is flat index 5 — a real value.
    check_eq(
        content.resolve_selection(Selection::Column { col: 2, row: Some(1) }),
        Some((ResolvedBand::Column { col: 2, cols: 3 }, Some(5))),
    )
}

#[test]
fn resolve_selection_of_a_column_focusing_a_blank_cell_in_an_incomplete_grid_is_none() -> Result<(), String> {
    let content = seven_values_as_a_three_by_three_grid();
    // Column 2, row 2 is flat index 8 — within the 3×3 shape, but blank.
    check_eq(content.resolve_selection(Selection::Column { col: 2, row: Some(2) }), None)
}

#[test]
fn resolve_selection_of_a_row_with_no_focus_still_succeeds_on_an_incomplete_grid() -> Result<(), String> {
    let content = seven_values_as_a_three_by_three_grid();
    // Row 2 is nominally indices 6, 7, 8, of which only 6 is real. `resolve_selection` itself still succeeds. See the
    // `ResolvedBand::contains` tests below for how the blank indices are kept out of the recoloured set.
    check_eq(
        content.resolve_selection(Selection::Row { row: 2, col: None }),
        Some((ResolvedBand::Row { row: 2, cols: 3 }, None)),
    )
}

#[test]
fn resolve_selection_of_an_entirely_blank_row_under_an_over_specified_layout_still_resolves() -> Result<(), String> {
    // `GridLayout::Rows(10)` with only 3 values legitimately creates a (10, 1) shape — rows 3 through 9 have no cell in
    // them at all, not merely a short last row.
    let content =
        DataNodeContent::new(NodeValues::U8(vec![1, 2, 3]), DataFormat::Decimal).with_layout(GridLayout::Rows(10));
    check_eq(
        content.resolve_selection(Selection::Row { row: 5, col: None }),
        Some((ResolvedBand::Row { row: 5, cols: 1 }, None)),
    )
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
//   ResolvedBand::contains
// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -

#[test]
fn resolved_band_none_contains_nothing() -> Result<(), String> {
    check_eq(ResolvedBand::None.contains(0), false)?;
    check_eq(ResolvedBand::None.contains(41), false)
}

#[test]
fn resolved_band_row_contains_exactly_that_rows_own_flat_indices() -> Result<(), String> {
    // A 2×3 grid's own row 1 is flat indices 3, 4, 5.
    let band = ResolvedBand::Row { row: 1, cols: 3 };
    check_eq(band.contains(3), true)?;
    check_eq(band.contains(4), true)?;
    check_eq(band.contains(5), true)?;
    check_eq(band.contains(2), false)?;
    check_eq(band.contains(6), false)
}

#[test]
fn resolved_band_column_contains_exactly_that_columns_own_flat_indices() -> Result<(), String> {
    // A 3-column grid's own column 2 is flat indices 2, 5, 8, ...
    let band = ResolvedBand::Column { col: 2, cols: 3 };
    check_eq(band.contains(2), true)?;
    check_eq(band.contains(5), true)?;
    check_eq(band.contains(0), false)?;
    check_eq(band.contains(4), false)
}

#[test]
fn resolved_band_contains_is_a_pure_shape_arithmetic_with_no_notion_of_a_blank_cell() -> Result<(), String> {
    // `contains` alone cannot tell a nominal row/column position from a real one — `Scene::set_selection` never queries
    // it with anything but a real flat index, so it never needs to.
    check_eq(ResolvedBand::Row { row: 2, cols: 3 }.contains(8), true)
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
//   Selection::describe_into
// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -

/// `Selection::describe_into`'s own output, as an owned `String` — this file's own tests only ever check the finished
/// text, never the buffer-reuse behaviour `describe_into` exists for.
fn describe(selection: Selection) -> String {
    let mut out = String::new();
    selection.describe_into(&mut out);
    out
}

#[test]
fn describe_of_none_is_empty() -> Result<(), String> {
    check_eq(describe(Selection::None), String::new())
}

#[test]
fn describe_of_a_cell_names_its_own_index() -> Result<(), String> {
    check_eq(describe(Selection::Cell(3)), ", cell 3 selected".to_owned())
}

#[test]
fn describe_of_a_row_with_no_cell_names_only_the_row() -> Result<(), String> {
    check_eq(describe(Selection::Row { row: 2, col: None }), ", row 2 selected".to_owned())
}

#[test]
fn describe_of_a_row_with_a_cell_names_both() -> Result<(), String> {
    check_eq(
        describe(Selection::Row { row: 2, col: Some(1) }),
        ", row 2 selected, column 1 focused".to_owned(),
    )
}

#[test]
fn describe_of_a_column_with_no_cell_names_only_the_column() -> Result<(), String> {
    check_eq(
        describe(Selection::Column { col: 2, row: None }),
        ", column 2 selected".to_owned(),
    )
}

#[test]
fn describe_of_a_column_with_a_cell_names_both() -> Result<(), String> {
    check_eq(
        describe(Selection::Column { col: 2, row: Some(1) }),
        ", column 2 selected, row 1 focused".to_owned(),
    )
}

#[test]
fn describe_into_appends_rather_than_replacing_existing_content() -> Result<(), String> {
    // `Scene::set_selection` relies on this. It truncates its reused buffer back to the node's own base label, then
    // calls `describe_into` to append onto whatever remains. It never replaces the whole buffer.
    let mut out = String::from("base label");
    Selection::Cell(3).describe_into(&mut out);
    check_eq(out, "base label, cell 3 selected".to_owned())
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
//   natural_selection / flat_index
// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -

#[test]
fn natural_selection_of_a_one_dimensional_content_is_always_a_cell() -> Result<(), String> {
    // Forced to a single row: a one-dimensional shape, regardless of what `Automatic` would otherwise pick for 4 values
    // (a 2x2 square).
    let content =
        DataNodeContent::new(NodeValues::U8(vec![1, 2, 3, 4]), DataFormat::Decimal).with_layout(GridLayout::Rows(1));
    check_eq(content.natural_selection(0), Some(Selection::Cell(0)))?;
    check_eq(content.natural_selection(3), Some(Selection::Cell(3)))
}

#[test]
fn natural_selection_out_of_range_is_none() -> Result<(), String> {
    let content = DataNodeContent::new(NodeValues::U8(vec![1, 2, 3, 4]), DataFormat::Decimal);
    check_eq(content.natural_selection(4), None)
}

#[test]
fn natural_selection_of_an_empty_content_is_always_none() -> Result<(), String> {
    let content = DataNodeContent::new(NodeValues::U8(vec![]), DataFormat::Decimal);
    check_eq(content.natural_selection(0), None)
}

#[test]
fn natural_selection_of_a_two_dimensional_content_is_a_row_with_a_focused_column() -> Result<(), String> {
    // 6 values forced into 2 rows of 3: a genuinely two-dimensional shape.
    let content = DataNodeContent::new(NodeValues::U8(vec![1, 2, 3, 4, 5, 6]), DataFormat::Decimal)
        .with_layout(GridLayout::Rows(2));
    check_eq(content.natural_selection(4), Some(Selection::Row { row: 1, col: Some(1) }))
}

#[test]
fn flat_index_is_the_inverse_of_natural_selection_over_a_one_dimensional_content() -> Result<(), String> {
    let content =
        DataNodeContent::new(NodeValues::U8(vec![1, 2, 3, 4]), DataFormat::Decimal).with_layout(GridLayout::Rows(1));
    for i in 0..4 {
        let selection = content.natural_selection(i).expect("in range");
        check_eq(content.flat_index(&selection), Some(i))?;
    }
    Ok(())
}

#[test]
fn flat_index_is_the_inverse_of_natural_selection_over_a_two_dimensional_content() -> Result<(), String> {
    let content = DataNodeContent::new(NodeValues::U8(vec![1, 2, 3, 4, 5, 6]), DataFormat::Decimal)
        .with_layout(GridLayout::Rows(2));
    for i in 0..6 {
        let selection = content.natural_selection(i).expect("in range");
        check_eq(content.flat_index(&selection), Some(i))?;
    }
    Ok(())
}

#[test]
fn flat_index_of_a_column_is_none() -> Result<(), String> {
    // `natural_selection` never produces `Column` — this content's own shape would never have produced it.
    let content = DataNodeContent::new(NodeValues::U8(vec![1, 2, 3, 4, 5, 6]), DataFormat::Decimal)
        .with_layout(GridLayout::Rows(2));
    check_eq(content.flat_index(&Selection::Column { col: 1, row: None }), None)
}

#[test]
fn flat_index_of_a_row_with_no_focused_column_is_none() -> Result<(), String> {
    let content = DataNodeContent::new(NodeValues::U8(vec![1, 2, 3, 4, 5, 6]), DataFormat::Decimal)
        .with_layout(GridLayout::Rows(2));
    check_eq(content.flat_index(&Selection::Row { row: 1, col: None }), None)
}

#[test]
fn flat_index_of_a_cell_on_a_two_dimensional_shape_is_none() -> Result<(), String> {
    // A two-dimensional shape's own `natural_selection` never produces `Cell` — only `Row { .., col: Some(_) }`.
    let content = DataNodeContent::new(NodeValues::U8(vec![1, 2, 3, 4, 5, 6]), DataFormat::Decimal)
        .with_layout(GridLayout::Rows(2));
    check_eq(content.flat_index(&Selection::Cell(4)), None)
}

#[test]
fn flat_index_of_none_is_none() -> Result<(), String> {
    let content = DataNodeContent::new(NodeValues::U8(vec![1, 2, 3, 4]), DataFormat::Decimal);
    check_eq(content.flat_index(&Selection::None), None)
}

#[test]
fn flat_index_of_an_out_of_range_cell_is_none() -> Result<(), String> {
    // One-dimensional, so `Cell` is otherwise the right shape — only the out-of-range index itself is at fault.
    let content =
        DataNodeContent::new(NodeValues::U8(vec![1, 2, 3, 4]), DataFormat::Decimal).with_layout(GridLayout::Rows(1));
    check_eq(content.flat_index(&Selection::Cell(4)), None)
}
