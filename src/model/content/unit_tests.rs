use super::*;

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
fn check_eq<T: PartialEq + std::fmt::Debug>(got: T, expected: T) -> Result<(), String> {
    if got == expected {
        Ok(())
    } else {
        Err(format!("expected {expected:?}, got {got:?}"))
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
// grid_shape
// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -

#[test]
fn grid_shape_of_one_value_is_a_single_cell() -> Result<(), String> {
    check_eq(grid_shape(1, GridLayout::Automatic), (1, 1))
}

#[test]
fn grid_shape_of_two_values_is_two_rows_of_one_column() -> Result<(), String> {
    // The example from the feature request: two values stack vertically, not side by side.
    check_eq(grid_shape(2, GridLayout::Automatic), (2, 1))
}

#[test]
fn grid_shape_of_three_values_prefers_a_square_over_a_single_column() -> Result<(), String> {
    check_eq(grid_shape(3, GridLayout::Automatic), (2, 2))
}

#[test]
fn grid_shape_of_four_values_is_an_exact_square() -> Result<(), String> {
    check_eq(grid_shape(4, GridLayout::Automatic), (2, 2))
}

#[test]
fn grid_shape_of_five_values_is_three_rows_of_two_columns() -> Result<(), String> {
    check_eq(grid_shape(5, GridLayout::Automatic), (3, 2))
}

#[test]
fn grid_shape_of_six_values_prefers_two_rows_of_three_over_three_rows_of_two() -> Result<(), String> {
    // 2 is a power of two, strictly between 1 and 6, dividing it evenly, and gives a squarer grid (2x3, diff 1)
    // than the sqrt-based fallback would (3x2 is the same shape transposed, so this is really about which one
    // best_power_of_two_rows picks as "rows" — see that function's own doc comment on ties).
    check_eq(grid_shape(6, GridLayout::Automatic), (2, 3))
}

#[test]
fn grid_shape_of_seven_values_is_three_rows_of_three_columns() -> Result<(), String> {
    // 7 is prime: no power of two other than the trivial 1 divides it, so this falls back to ceil(sqrt(n)).
    check_eq(grid_shape(7, GridLayout::Automatic), (3, 3))
}

#[test]
fn grid_shape_of_eight_values_prefers_two_rows_of_four_over_a_square_with_a_gap() -> Result<(), String> {
    // The revised feature request's own example: 2 rows of 4 (an exact fit), not the 3x3 square the plain
    // "closest to square" rule would have picked (which would leave one slot blank).
    check_eq(grid_shape(8, GridLayout::Automatic), (2, 4))
}

#[test]
fn grid_shape_of_twenty_five_values_is_a_five_by_five_square() -> Result<(), String> {
    // 25 is odd (5^2): no power of two divides it, so this falls back to ceil(sqrt(n)), which happens to be an
    // exact square anyway.
    check_eq(grid_shape(25, GridLayout::Automatic), (5, 5))
}

#[test]
fn grid_shape_of_twenty_values_prefers_four_rows_of_five() -> Result<(), String> {
    // The revised feature request's own second example: of 20's power-of-two divisors (2, 4), rows = 4 gives the
    // squarer grid (4x5, diff 1) over rows = 2 (2x10, diff 8).
    check_eq(grid_shape(20, GridLayout::Automatic), (4, 5))
}

#[test]
fn grid_shape_of_zero_values_is_empty() -> Result<(), String> {
    check_eq(grid_shape(0, GridLayout::Automatic), (0, 0))
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
// grid_shape — GridLayout::Columns/Rows/MaxColumns
// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -

#[test]
fn columns_fixes_the_column_count_regardless_of_squareness() -> Result<(), String> {
    // 8 values, forced into 3 columns: Automatic would pick 2x4 (see the test above), but Columns overrides that
    // entirely — 3 rows of 3 columns, with the last row only 2/3 full.
    check_eq(grid_shape(8, GridLayout::Columns(3)), (3, 3))
}

#[test]
fn columns_wider_than_the_value_count_still_yields_one_row() -> Result<(), String> {
    check_eq(grid_shape(3, GridLayout::Columns(10)), (1, 10))
}

#[test]
fn rows_fixes_the_row_count_regardless_of_squareness() -> Result<(), String> {
    check_eq(grid_shape(8, GridLayout::Rows(3)), (3, 3))
}

#[test]
fn max_columns_caps_the_column_count_below_automatics_own_choice() -> Result<(), String> {
    // Automatic renders 20 values as 4 rows of 5 (see grid_shape_of_twenty_values_prefers_four_rows_of_five above)
    // — MaxColumns(3) caps that at 3 columns regardless, giving 7 rows of 3 (21 slots, one blank).
    check_eq(grid_shape(20, GridLayout::MaxColumns(3)), (7, 3))
}

#[test]
fn max_columns_above_the_value_count_behaves_like_a_single_row() -> Result<(), String> {
    check_eq(grid_shape(3, GridLayout::MaxColumns(10)), (1, 3))
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
// GridLayout::is_valid
// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -

#[test]
fn automatic_is_always_valid() -> Result<(), String> {
    check_eq(GridLayout::Automatic.is_valid(), true)
}

#[test]
fn a_positive_columns_rows_or_max_columns_is_valid() -> Result<(), String> {
    check_eq(GridLayout::Columns(1).is_valid(), true)?;
    check_eq(GridLayout::Rows(1).is_valid(), true)?;
    check_eq(GridLayout::MaxColumns(1).is_valid(), true)
}

#[test]
fn a_zero_columns_rows_or_max_columns_is_invalid() -> Result<(), String> {
    check_eq(GridLayout::Columns(0).is_valid(), false)?;
    check_eq(GridLayout::Rows(0).is_valid(), false)?;
    check_eq(GridLayout::MaxColumns(0).is_valid(), false)
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
// best_power_of_two_rows
// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -

#[test]
fn best_power_of_two_rows_breaks_a_tie_toward_fewer_wider_rows() -> Result<(), String> {
    // 8's two equally square candidates are rows=2 (2x4, diff 2) and rows=4 (4x2, diff 2) — the smaller, wider
    // one wins.
    check_eq(best_power_of_two_rows(8), Some(2))
}

#[test]
fn best_power_of_two_rows_picks_the_squarest_candidate_when_not_tied() -> Result<(), String> {
    check_eq(best_power_of_two_rows(20), Some(4))
}

#[test]
fn best_power_of_two_rows_is_none_for_a_prime() -> Result<(), String> {
    check_eq(best_power_of_two_rows(5), None)
}

#[test]
fn best_power_of_two_rows_is_none_when_the_only_divisor_is_the_trivial_one() -> Result<(), String> {
    // 2's only power-of-two divisors are 1 and 2 itself — nothing strictly between them.
    check_eq(best_power_of_two_rows(2), None)
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
// Per-value formatting
// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -

#[test]
fn hexadecimal_u64_matches_the_feature_requests_own_example() -> Result<(), String> {
    let content = DataNodeContent::new(NodeValues::U64(vec![0xF0E1D2C3B4A59687]), DataFormat::Hexadecimal);
    check_eq(content.cells(), vec!["F0 E1 D2 C3 B4 A5 96 87".to_owned()])
}

#[test]
fn hexadecimal_u8_is_a_single_byte_group() -> Result<(), String> {
    let content = DataNodeContent::new(NodeValues::U8(vec![0xAB]), DataFormat::Hexadecimal);
    check_eq(content.cells(), vec!["AB".to_owned()])
}

#[test]
fn hexadecimal_u16_is_two_byte_groups_big_endian() -> Result<(), String> {
    let content = DataNodeContent::new(NodeValues::U16(vec![0xABCD]), DataFormat::Hexadecimal);
    check_eq(content.cells(), vec!["AB CD".to_owned()])
}

#[test]
fn hexadecimal_u32_is_four_byte_groups_big_endian() -> Result<(), String> {
    let content = DataNodeContent::new(NodeValues::U32(vec![0xAABBCCDD]), DataFormat::Hexadecimal);
    check_eq(content.cells(), vec!["AA BB CC DD".to_owned()])
}

#[test]
fn binary_u8_splits_into_upper_and_lower_nybbles() -> Result<(), String> {
    let content = DataNodeContent::new(NodeValues::U8(vec![0xF0]), DataFormat::Binary);
    check_eq(content.cells(), vec!["1111 0000".to_owned()])
}

#[test]
fn binary_u16_splits_every_bytes_own_nybbles_big_endian() -> Result<(), String> {
    let content = DataNodeContent::new(NodeValues::U16(vec![0xF00F]), DataFormat::Binary);
    check_eq(content.cells(), vec!["1111 0000 0000 1111".to_owned()])
}

#[test]
fn decimal_does_not_split_into_bytes() -> Result<(), String> {
    let content = DataNodeContent::new(NodeValues::U32(vec![1_234_567]), DataFormat::Decimal);
    check_eq(content.cells(), vec!["1234567".to_owned()])
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
// ByteOrder
// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -

#[test]
fn new_defaults_to_big_endian_byte_order() -> Result<(), String> {
    let content = DataNodeContent::new(NodeValues::U32(vec![0xAABBCCDD]), DataFormat::Hexadecimal);
    check_eq(content.cells(), vec!["AA BB CC DD".to_owned()])
}

#[test]
fn with_byte_order_little_endian_reverses_the_byte_groups() -> Result<(), String> {
    let content = DataNodeContent::new(NodeValues::U32(vec![0xAABBCCDD]), DataFormat::Hexadecimal)
        .with_byte_order(ByteOrder::LittleEndian);
    check_eq(content.cells(), vec!["DD CC BB AA".to_owned()])
}

#[test]
fn little_endian_also_reverses_binary_byte_groups() -> Result<(), String> {
    let content = DataNodeContent::new(NodeValues::U16(vec![0xF00F]), DataFormat::Binary)
        .with_byte_order(ByteOrder::LittleEndian);
    check_eq(content.cells(), vec!["0000 1111 1111 0000".to_owned()])
}

#[test]
fn byte_order_has_no_visible_effect_on_a_single_byte_value() -> Result<(), String> {
    let big_endian = DataNodeContent::new(NodeValues::U8(vec![0xAB]), DataFormat::Hexadecimal);
    let little_endian = DataNodeContent::new(NodeValues::U8(vec![0xAB]), DataFormat::Hexadecimal)
        .with_byte_order(ByteOrder::LittleEndian);
    check_eq(big_endian.cells(), little_endian.cells())
}

#[test]
fn byte_order_has_no_visible_effect_under_decimal_format() -> Result<(), String> {
    let big_endian = DataNodeContent::new(NodeValues::U32(vec![1_234_567]), DataFormat::Decimal);
    let little_endian = DataNodeContent::new(NodeValues::U32(vec![1_234_567]), DataFormat::Decimal)
        .with_byte_order(ByteOrder::LittleEndian);
    check_eq(big_endian.cells(), little_endian.cells())
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
// DataNodeContent::cells — one string per value, not yet arranged into rows
// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -

#[test]
fn a_single_value_produces_one_cell() -> Result<(), String> {
    let content = DataNodeContent::new(NodeValues::U64(vec![0x1122334455667788]), DataFormat::Hexadecimal);
    check_eq(content.cells().len(), 1)
}

#[test]
fn two_values_produce_two_cells_in_order() -> Result<(), String> {
    let content = DataNodeContent::new(
        NodeValues::U64(vec![0x1111111111111111, 0x2222222222222222]),
        DataFormat::Hexadecimal,
    );
    check_eq(
        content.cells(),
        vec!["11 11 11 11 11 11 11 11".to_owned(), "22 22 22 22 22 22 22 22".to_owned()],
    )
}

#[test]
fn twenty_five_values_produce_twenty_five_cells_arranged_as_a_five_by_five_grid() -> Result<(), String> {
    let values = (0..25u64).collect();
    let content = DataNodeContent::new(NodeValues::U64(values), DataFormat::Decimal);
    check_eq(content.cells().len(), 25)?;
    check_eq(content.shape(), (5, 5))
}

#[test]
fn cells_are_returned_even_when_the_last_grid_row_is_only_partially_filled() -> Result<(), String> {
    // 7 values -> a 3x3 grid (see grid_shape_of_seven_values above), with 2 blank slots in the last row —
    // cells() itself has no concept of blanks, it is draw_content_box's job to stop after the 7th.
    let values = (0..7u8).collect();
    let content = DataNodeContent::new(NodeValues::U8(values), DataFormat::Decimal);
    check_eq(content.cells().len(), 7)?;
    check_eq(content.shape(), (3, 3))
}

#[test]
fn new_defaults_to_automatic_layout() -> Result<(), String> {
    let content = DataNodeContent::new(NodeValues::U8(vec![1, 2, 3, 4, 5, 6, 7, 8]), DataFormat::Decimal);
    check_eq(content.layout(), GridLayout::Automatic)?;
    // Automatic's own 2x4 choice for 8 values — see grid_shape_of_eight_values_prefers_two_rows_of_four above.
    check_eq(content.shape(), (2, 4))
}

#[test]
fn with_layout_overrides_automatics_own_shape() -> Result<(), String> {
    let content = DataNodeContent::new(NodeValues::U8(vec![1, 2, 3, 4, 5, 6, 7, 8]), DataFormat::Decimal)
        .with_layout(GridLayout::MaxColumns(3));
    check_eq(content.layout(), GridLayout::MaxColumns(3))?;
    check_eq(content.shape(), (3, 3))
}

#[test]
fn len_reports_the_value_count_regardless_of_width() -> Result<(), String> {
    let content = DataNodeContent::new(NodeValues::U16(vec![1, 2, 3]), DataFormat::Decimal);
    check_eq(content.len(), 3)
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
// is_single_value / type_color
// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -

#[test]
fn a_single_value_is_reported_as_such() -> Result<(), String> {
    let content = DataNodeContent::new(NodeValues::U8(vec![1]), DataFormat::Decimal);
    check_eq(content.is_single_value(), true)
}

#[test]
fn two_values_are_not_reported_as_a_single_value() -> Result<(), String> {
    let content = DataNodeContent::new(NodeValues::U8(vec![1, 2]), DataFormat::Decimal);
    check_eq(content.is_single_value(), false)
}

#[test]
fn every_width_gets_a_distinct_type_color() -> Result<(), String> {
    let colors = [
        DataNodeContent::new(NodeValues::U8(vec![1]), DataFormat::Decimal).type_color(),
        DataNodeContent::new(NodeValues::U16(vec![1]), DataFormat::Decimal).type_color(),
        DataNodeContent::new(NodeValues::U32(vec![1]), DataFormat::Decimal).type_color(),
        DataNodeContent::new(NodeValues::U64(vec![1]), DataFormat::Decimal).type_color(),
    ];
    let mut unique = colors.to_vec();
    unique.sort_unstable();
    unique.dedup();
    check_eq(unique.len(), colors.len())
}

#[test]
fn type_name_matches_each_widths_own_rust_type() -> Result<(), String> {
    check_eq(
        DataNodeContent::new(NodeValues::U8(vec![1]), DataFormat::Decimal).type_name(),
        "u8",
    )?;
    check_eq(
        DataNodeContent::new(NodeValues::U16(vec![1]), DataFormat::Decimal).type_name(),
        "u16",
    )?;
    check_eq(
        DataNodeContent::new(NodeValues::U32(vec![1]), DataFormat::Decimal).type_name(),
        "u32",
    )?;
    check_eq(
        DataNodeContent::new(NodeValues::U64(vec![1]), DataFormat::Decimal).type_name(),
        "u64",
    )
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
// resolve_selection
// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -

#[test]
fn resolve_selection_of_none_bands_and_focuses_nothing() -> Result<(), String> {
    let content = DataNodeContent::new(NodeValues::U8(vec![1, 2, 3, 4]), DataFormat::Decimal);
    check_eq(content.resolve_selection(Selection::None), Some((Vec::new(), None)))
}

#[test]
fn resolve_selection_of_an_in_range_cell_focuses_it_alone() -> Result<(), String> {
    let content = DataNodeContent::new(NodeValues::U8(vec![1, 2, 3, 4]), DataFormat::Decimal);
    check_eq(content.resolve_selection(Selection::Cell(2)), Some((Vec::new(), Some(2))))
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
        Some((vec![3, 4, 5], None)),
    )
}

#[test]
fn resolve_selection_of_a_row_with_a_cell_also_focuses_that_one_cell() -> Result<(), String> {
    let content = DataNodeContent::new(NodeValues::U8(vec![1, 2, 3, 4, 5, 6]), DataFormat::Decimal)
        .with_layout(GridLayout::Rows(2));
    check_eq(
        content.resolve_selection(Selection::Row { row: 1, col: Some(2) }),
        Some((vec![3, 4, 5], Some(5))),
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
        Some((vec![2, 5], None)),
    )
}

#[test]
fn resolve_selection_of_a_column_with_a_row_also_focuses_that_one_cell() -> Result<(), String> {
    let content = DataNodeContent::new(NodeValues::U8(vec![1, 2, 3, 4, 5, 6]), DataFormat::Decimal)
        .with_layout(GridLayout::Rows(2));
    check_eq(
        content.resolve_selection(Selection::Column { col: 2, row: Some(1) }),
        Some((vec![2, 5], Some(5))),
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
// resolve_selection — incomplete grids (a row/column index within shape, but the flat cell it names is blank)
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
        Some((vec![6], Some(6))),
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
        Some((vec![2, 5], Some(5))),
    )
}

#[test]
fn resolve_selection_of_a_column_focusing_a_blank_cell_in_an_incomplete_grid_is_none() -> Result<(), String> {
    let content = seven_values_as_a_three_by_three_grid();
    // Column 2, row 2 is flat index 8 — within the 3×3 shape, but blank.
    check_eq(content.resolve_selection(Selection::Column { col: 2, row: Some(2) }), None)
}

#[test]
fn resolve_selection_of_a_row_with_no_focus_bands_only_its_real_cells() -> Result<(), String> {
    let content = seven_values_as_a_three_by_three_grid();
    // Row 2 is nominally indices 6, 7, 8 — only 6 is real, so the band excludes the other two.
    check_eq(
        content.resolve_selection(Selection::Row { row: 2, col: None }),
        Some((vec![6], None)),
    )
}

#[test]
fn resolve_selection_of_an_entirely_blank_row_under_an_over_specified_layout_bands_nothing() -> Result<(), String> {
    // `GridLayout::Rows(10)` with only 3 values legitimately creates a (10, 1) shape — rows 3 through 9 have no
    // cell in them at all, not merely a short last row.
    let content =
        DataNodeContent::new(NodeValues::U8(vec![1, 2, 3]), DataFormat::Decimal).with_layout(GridLayout::Rows(10));
    check_eq(
        content.resolve_selection(Selection::Row { row: 5, col: None }),
        Some((Vec::new(), None)),
    )
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
// UnaryOperator::label / BinaryOperator::label
// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -

#[test]
fn unary_operator_label_names_not_with_no_operand() -> Result<(), String> {
    check_eq(UnaryOperator::Not.label(), "NOT".to_owned())
}

#[test]
fn unary_operator_label_includes_the_shift_or_rotate_amount() -> Result<(), String> {
    check_eq(UnaryOperator::ShiftLeft(3).label(), "SHL 3".to_owned())?;
    check_eq(UnaryOperator::ShiftRight(5).label(), "SHR 5".to_owned())?;
    check_eq(UnaryOperator::RotateLeft(1).label(), "ROTL 1".to_owned())?;
    check_eq(UnaryOperator::RotateRight(1).label(), "ROTR 1".to_owned())
}

#[test]
fn binary_operator_label_names_each_variant() -> Result<(), String> {
    check_eq(BinaryOperator::And.label(), "AND")?;
    check_eq(BinaryOperator::Or.label(), "OR")?;
    check_eq(BinaryOperator::Xor.label(), "XOR")
}
