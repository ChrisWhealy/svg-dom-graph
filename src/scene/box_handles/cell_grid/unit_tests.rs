use super::*;
use crate::test_support::check;

fn check_eq<T: PartialEq + std::fmt::Debug>(got: T, expected: T) -> Result<(), String> {
    check(got == expected, &format!("expected {expected:?}, got {got:?}"))
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// A 20 x 10 cell in a grid of 3 columns and 6 cells, padded 10 all round, with 6 between cells.
fn grid(column_group: usize) -> CellGrid {
    CellGrid {
        content_origin: Point::new(5.0, 7.0),
        cell: Size::new(20.0, 10.0),
        left_pad: 10.0,
        top_pad: 10.0,
        gap: 6.0,
        group_gap: 8.0,
        cols: 3,
        column_group,
        len: 6,
        single_value: false,
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[test]
fn the_first_cell_sits_one_padding_in_from_the_content_origin() -> Result<(), String> {
    check_eq(
        grid(0).cell_rect(0),
        Some(Rect {
            origin: Point::new(15.0, 17.0),
            size: Size::new(20.0, 10.0),
        }),
    )
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[test]
fn a_cells_origin_follows_its_row_and_column() -> Result<(), String> {
    // Index 4 is row 1, column 1: one cell and one gap right, one cell and one gap down.
    check_eq(
        grid(0).cell_rect(4).map(|r| r.origin),
        Some(Point::new(15.0 + 26.0, 17.0 + 16.0)),
    )
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[test]
fn a_cell_past_the_last_has_no_rectangle() -> Result<(), String> {
    check_eq(grid(0).cell_rect(6), None)
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[test]
fn a_column_group_adds_a_wider_gap_only_after_each_complete_group() -> Result<(), String> {
    let grid = grid(2);
    // Column 1 ends the first group of two, so column 2 starts after the wider gap. Column 1 itself does not.
    check_eq(grid.cell_rect(1).map(|r| r.origin.x), Some(15.0 + 26.0))?;
    check_eq(grid.cell_rect(2).map(|r| r.origin.x), Some(15.0 + 52.0 + 8.0))
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[test]
fn group_gaps_never_count_one_after_the_last_group() -> Result<(), String> {
    check_eq(group_gaps(2, 4), 1)?;
    check_eq(group_gaps(2, 2), 0)?;
    check_eq(group_gaps(0, 9), 0)
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[test]
fn a_single_value_is_the_whole_content_box() -> Result<(), String> {
    let single = CellGrid {
        single_value: true,
        len: 1,
        cols: 1,
        ..grid(0)
    };
    check_eq(
        single.cell_rect(0),
        Some(Rect {
            origin: Point::new(5.0, 7.0),
            size: Size::new(20.0, 10.0),
        }),
    )?;
    check_eq(single.cell_rect(1), None)
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[test]
fn a_grid_stays_small_however_many_cells_it_holds() -> Result<(), String> {
    let many = CellGrid { len: 1600, ..grid(0) };
    check(
        std::mem::size_of_val(&many) < 128,
        "a CellGrid should be a few words, not one rectangle per cell",
    )
}
