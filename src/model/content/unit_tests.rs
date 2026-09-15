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
    check_eq(grid_shape(1), (1, 1))
}

#[test]
fn grid_shape_of_two_values_is_two_rows_of_one_column() -> Result<(), String> {
    // The example from the feature request: two values stack vertically, not side by side.
    check_eq(grid_shape(2), (2, 1))
}

#[test]
fn grid_shape_of_three_values_prefers_a_square_over_a_single_column() -> Result<(), String> {
    check_eq(grid_shape(3), (2, 2))
}

#[test]
fn grid_shape_of_four_values_is_an_exact_square() -> Result<(), String> {
    check_eq(grid_shape(4), (2, 2))
}

#[test]
fn grid_shape_of_five_values_is_three_rows_of_two_columns() -> Result<(), String> {
    check_eq(grid_shape(5), (3, 2))
}

#[test]
fn grid_shape_of_six_values_prefers_two_rows_of_three_over_three_rows_of_two() -> Result<(), String> {
    // 2 is a power of two, strictly between 1 and 6, dividing it evenly, and gives a squarer grid (2x3, diff 1)
    // than the sqrt-based fallback would (3x2 is the same shape transposed, so this is really about which one
    // best_power_of_two_rows picks as "rows" — see that function's own doc comment on ties).
    check_eq(grid_shape(6), (2, 3))
}

#[test]
fn grid_shape_of_seven_values_is_three_rows_of_three_columns() -> Result<(), String> {
    // 7 is prime: no power of two other than the trivial 1 divides it, so this falls back to ceil(sqrt(n)).
    check_eq(grid_shape(7), (3, 3))
}

#[test]
fn grid_shape_of_eight_values_prefers_two_rows_of_four_over_a_square_with_a_gap() -> Result<(), String> {
    // The revised feature request's own example: 2 rows of 4 (an exact fit), not the 3x3 square the plain
    // "closest to square" rule would have picked (which would leave one slot blank).
    check_eq(grid_shape(8), (2, 4))
}

#[test]
fn grid_shape_of_twenty_five_values_is_a_five_by_five_square() -> Result<(), String> {
    // 25 is odd (5^2): no power of two divides it, so this falls back to ceil(sqrt(n)), which happens to be an
    // exact square anyway.
    check_eq(grid_shape(25), (5, 5))
}

#[test]
fn grid_shape_of_twenty_values_prefers_four_rows_of_five() -> Result<(), String> {
    // The revised feature request's own second example: of 20's power-of-two divisors (2, 4), rows = 4 gives the
    // squarer grid (4x5, diff 1) over rows = 2 (2x10, diff 8).
    check_eq(grid_shape(20), (4, 5))
}

#[test]
fn grid_shape_of_zero_values_is_empty() -> Result<(), String> {
    check_eq(grid_shape(0), (0, 0))
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
