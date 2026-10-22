//! Grid arithmetic: `grid_shape` under `GridLayout::Automatic` and the fixed layouts, `GridLayout::is_valid`, and
//! `best_power_of_two_rows`.

use super::super::*;
use super::support::check_eq;

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
//   grid_shape
// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -

/// `GridLayout::Automatic` for every count whose shape shows a distinct case of the rule. A failure names the count.
#[test]
fn automatic_layout_picks_the_expected_shape_for_each_count() -> Result<(), String> {
    for (count, expected, why) in [
        (0, (0, 0), "nothing to lay out"),
        (1, (1, 1), "a single cell"),
        // The example from the feature request: two values stack vertically, not side by side.
        (2, (2, 1), "two values stack vertically"),
        (3, (2, 2), "a square is preferred over a single column"),
        (4, (2, 2), "an exact square"),
        (5, (3, 2), "three rows of two columns"),
        // 2 is a power of two, strictly between 1 and 6, dividing it evenly. It gives a squarer grid (2x3, diff 1)
        // than the sqrt-based fallback would. 3x2 is the same shape transposed, so this is really about which one
        // best_power_of_two_rows picks as "rows". See that function's own doc comment on ties.
        (6, (2, 3), "two rows of three over three rows of two"),
        // 7 is prime: no power of two other than the trivial 1 divides it, so this falls back to ceil(sqrt(n)).
        (7, (3, 3), "a prime falls back to ceil(sqrt(n))"),
        // The revised feature request's own example: 2 rows of 4 (an exact fit). It is not the 3x3 square the plain
        // "closest to square" rule would have picked, which would leave one slot blank.
        (8, (2, 4), "two rows of four over a square with a gap"),
        // The revised feature request's own second example: of 20's power-of-two divisors (2, 4), rows = 4 gives the
        // squarer grid (4x5, diff 1) over rows = 2 (2x10, diff 8).
        (20, (4, 5), "the squarer of the power-of-two row counts"),
        // 25 is odd (5^2): no power of two divides it, so this falls back to ceil(sqrt(n)), which happens to be an
        // exact square anyway.
        (25, (5, 5), "an exact square from the fallback"),
    ] {
        check_eq(grid_shape(count, GridLayout::Automatic), expected)
            .map_err(|e| format!("{count} values ({why}): {e}"))?;
    }
    Ok(())
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
//   grid_shape — GridLayout::Columns/Rows/MaxColumns
// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -

#[test]
fn columns_fixes_the_column_count_regardless_of_squareness() -> Result<(), String> {
    // 8 values, forced into 3 columns. Automatic would pick 2x4 (see the table above), but Columns overrides that
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
    // Automatic renders 20 values as 4 rows of 5 (see the table of Automatic shapes above) —
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
