//! Grid arithmetic: `grid_shape` under `GridLayout::Automatic` and the fixed layouts, `GridLayout::is_valid`, and
//! `best_power_of_two_rows`.

use super::super::*;
use super::support::check_eq;

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
//   grid_shape
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
    // 2 is a power of two, strictly between 1 and 6, dividing it evenly. It gives a squarer grid (2x3, diff 1) than the
    // sqrt-based fallback would. 3x2 is the same shape transposed. So this is really about which one
    // best_power_of_two_rows picks as "rows". See that function's own doc comment on ties.
    check_eq(grid_shape(6, GridLayout::Automatic), (2, 3))
}

#[test]
fn grid_shape_of_seven_values_is_three_rows_of_three_columns() -> Result<(), String> {
    // 7 is prime: no power of two other than the trivial 1 divides it, so this falls back to ceil(sqrt(n)).
    check_eq(grid_shape(7, GridLayout::Automatic), (3, 3))
}

#[test]
fn grid_shape_of_eight_values_prefers_two_rows_of_four_over_a_square_with_a_gap() -> Result<(), String> {
    // The revised feature request's own example: 2 rows of 4 (an exact fit). It is not the 3x3 square the plain
    // "closest to square" rule would have picked, which would leave one slot blank.
    check_eq(grid_shape(8, GridLayout::Automatic), (2, 4))
}

#[test]
fn grid_shape_of_twenty_five_values_is_a_five_by_five_square() -> Result<(), String> {
    // 25 is odd (5^2): no power of two divides it, so this falls back to ceil(sqrt(n)), which happens to be an exact
    // square anyway.
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
//   grid_shape — GridLayout::Columns/Rows/MaxColumns
// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -

#[test]
fn columns_fixes_the_column_count_regardless_of_squareness() -> Result<(), String> {
    // 8 values, forced into 3 columns. Automatic would pick 2x4 (see the test above), but Columns overrides that
    // entirely. The result is 3 rows of 3 columns, with the last row only 2/3 full.
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
    // Automatic renders 20 values as 4 rows of 5 (see grid_shape_of_twenty_values_prefers_four_rows_of_five above) —
    // MaxColumns(3) caps that at 3 columns regardless, giving 7 rows of 3 (21 slots, one blank).
    check_eq(grid_shape(20, GridLayout::MaxColumns(3)), (7, 3))
}

#[test]
fn max_columns_above_the_value_count_behaves_like_a_single_row() -> Result<(), String> {
    check_eq(grid_shape(3, GridLayout::MaxColumns(10)), (1, 3))
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
//   GridLayout::is_valid
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
//   best_power_of_two_rows
// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -

#[test]
fn best_power_of_two_rows_breaks_a_tie_toward_fewer_wider_rows() -> Result<(), String> {
    // 8's two equally square candidates are rows=2 (2x4, diff 2) and rows=4 (4x2, diff 2) — the smaller, wider one
    // wins.
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
