//! The pure parts of drawing a data node: grid arithmetic and the node's accessible name. Neither touches the DOM, so
//! both run under a plain `cargo test`.

use super::*;
use crate::{
    scene::{DataFormat, NodeValues},
    test_support::check,
};

fn check_eq<T: PartialEq + std::fmt::Debug>(got: T, expected: T) -> Result<(), String> {
    check(got == expected, &format!("expected {expected:?}, got {got:?}"))
}

/// A 20 x 10 cell in a 2 x 3 grid, with `column_group` columns per group.
fn grid(column_group: usize) -> GridLayoutMetrics {
    GridLayoutMetrics::new(Size::new(20.0, 10.0), OUTER_PADDING, (2, 3), column_group, false, CELL_GAP, 0.0)
}

#[test]
fn a_single_value_is_exactly_one_cell() -> Result<(), String> {
    let layout = GridLayoutMetrics::new(Size::new(20.0, 10.0), OUTER_PADDING, (1, 1), 0, true, CELL_GAP, 0.0);
    check_eq(layout.content_size(), Size::new(20.0, 10.0))
}

#[test]
fn the_lines_of_a_hex_dump_touch_so_it_adds_only_the_padding_all_round() -> Result<(), String> {
    // 3 lines of 20 x 10 with no gap between them, and the padding of 10 on every side.
    let dump = GridLayoutMetrics::new(Size::new(20.0, 10.0), OUTER_PADDING, (3, 1), 0, false, 0.0, 0.0);
    check_eq(dump.content_size(), Size::new(40.0, 50.0))
}

#[test]
fn a_grid_adds_the_gaps_between_cells_and_the_padding_all_round() -> Result<(), String> {
    // Width: 3 cells of 20, 2 gaps of 6, 2 paddings of 10. Height: 2 cells of 10, 1 gap of 6, 2 paddings of 10.
    check_eq(grid(0).content_size(), Size::new(92.0, 46.0))
}

#[test]
fn a_grids_name_gives_its_type_shape_and_value_count() -> Result<(), String> {
    let content = DataNodeContent::new(NodeValues::U8(vec![1, 2, 3, 4]), DataFormat::Hexadecimal);
    check_eq(
        describe(None, &content, ""),
        "u8 data grid, 2 rows by 2 columns, 4 values".to_owned(),
    )?;
    check_eq(
        describe(Some("A"), &content, ""),
        "A: u8 data grid, 2 rows by 2 columns, 4 values".to_owned(),
    )
}

#[test]
fn a_single_values_name_gives_its_type_and_text() -> Result<(), String> {
    let content = DataNodeContent::new(NodeValues::U8(vec![0x0A]), DataFormat::Hexadecimal);
    check_eq(describe(None, &content, "0A"), "u8 = 0A".to_owned())?;
    check_eq(describe(Some("B"), &content, "0A"), "B: u8 = 0A".to_owned())
}

#[test]
fn extra_left_padding_widens_the_grid_by_exactly_that_much() -> Result<(), String> {
    let wide = GridLayoutMetrics::new(Size::new(20.0, 10.0), OUTER_PADDING + 4.0, (2, 3), 0, false, CELL_GAP, 0.0);
    check_eq(wide.content_size().width, grid(0).content_size().width + 4.0)?;
    check_eq(
        wide.grid(Point::origin(), 6).cell_rect(0).map(|r| r.origin.x),
        Some(OUTER_PADDING + 4.0),
    )
}

#[test]
fn the_layouts_grid_places_a_cell_after_the_padding_and_gaps_the_size_counts() -> Result<(), String> {
    let layout = grid(0);
    let cells = layout.grid(Point::origin(), 6);
    // The last cell's far edge plus the padding is exactly the content box.
    let last = cells.cell_rect(5).ok_or("no last cell")?;
    check_eq(last.origin.x + last.size.width + OUTER_PADDING, layout.content_size().width)?;
    check_eq(last.origin.y + last.size.height + OUTER_PADDING, layout.content_size().height)
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
//   cell_name_into
// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -

fn name(labelling: Option<crate::scene::LabellingStyle>, i: usize, cols: usize, text: &str) -> String {
    let mut out = String::from("stale");
    cell_name_into(labelling, i, cols, text, &mut out);
    out
}

#[test]
fn an_unlabelled_cell_is_named_by_its_row_and_column() -> Result<(), String> {
    check_eq(name(None, 5, 4, "6"), "row 1, column 1: 6".to_owned())
}

#[test]
fn an_alphabetic_cell_name_starts_with_its_letter() -> Result<(), String> {
    check_eq(
        name(Some(crate::scene::LabellingStyle::Alphabetic), 0, 1, "FF"),
        "element a, row 0, column 0: FF".to_owned(),
    )
}

#[test]
fn a_numeric_cell_name_starts_with_its_index() -> Result<(), String> {
    check_eq(
        name(Some(crate::scene::LabellingStyle::Numeric), 7, 4, "x"),
        "element 7, row 1, column 3: x".to_owned(),
    )
}

#[test]
fn a_text_cell_name_introduces_its_label_as_a_row_label() -> Result<(), String> {
    let style = crate::scene::LabellingStyle::Text(&["0-3", "4-7"]);
    check_eq(name(Some(style), 5, 4, "x"), "row label 4-7, row 1, column 1: x".to_owned())
}

#[test]
fn a_text_cell_name_has_no_label_phrase_for_a_row_with_no_text() -> Result<(), String> {
    let style = crate::scene::LabellingStyle::Text(&["0-3", ""]);
    check_eq(name(Some(style), 5, 4, "x"), "row 1, column 1: x".to_owned())?;
    check_eq(name(Some(style), 9, 4, "x"), "row 2, column 1: x".to_owned())
}

#[test]
fn a_cell_name_reuses_the_buffer_it_is_given() -> Result<(), String> {
    let mut out = String::with_capacity(64);
    let before = out.as_ptr();
    for i in 0..20 {
        cell_name_into(Some(crate::scene::LabellingStyle::Alphabetic), i, 4, "00 11 22 33", &mut out);
    }
    check_eq(out.as_ptr(), before)
}

#[test]
fn a_toolbar_row_adds_its_height_under_the_name_and_a_width_the_bar_fits() -> Result<(), String> {
    let content = Size::new(40.0, 30.0);
    let without = named_frame_size(10.0, content, 0.0);
    let with = named_frame_size(10.0, content, TOOLBAR_ROW_HEIGHT);
    check_eq(with.height, without.height + TOOLBAR_ROW_HEIGHT)?;
    check_eq(without.width, 60.0)?;
    check_eq(with.width, TOOLBAR_MIN_WIDTH)
}

#[test]
fn only_a_named_node_holds_a_toolbar_row() -> Result<(), String> {
    let content = DataNodeContent::new(NodeValues::U8(vec![1]), DataFormat::Decimal).with_toolbar();
    check_eq(toolbar_row(Some("name"), &content), TOOLBAR_ROW_HEIGHT)?;
    check_eq(toolbar_row(None, &content), 0.0)?;
    let plain = DataNodeContent::new(NodeValues::U8(vec![1]), DataFormat::Decimal);
    check_eq(toolbar_row(Some("name"), &plain), 0.0)
}

#[test]
fn a_hex_dump_is_described_by_its_bytes_and_data_lines_and_not_by_its_cells() -> Result<(), String> {
    use crate::scene::HexDumpFormat;
    let dump = |len: usize, format| DataNodeContent::new(NodeValues::U8(vec![0; len]), DataFormat::HexDump(format));
    // The line of the total length is a cell, but it is not a data line.
    let format = HexDumpFormat::new().with_offset(true).with_total_length(true);
    check_eq(
        describe(None, &dump(40, format), ""),
        "hex dump, 40 bytes across 3 data lines".to_owned(),
    )?;
    check_eq(
        describe(Some("D"), &dump(1, format), ""),
        "D: hex dump, 1 byte across 1 data line".to_owned(),
    )
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
//   Which pictures a replacement writes
// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -

#[test]
fn a_replacement_writes_the_pictures_of_the_cells_that_changed() {
    assert_eq!(pictures_to_write(4, &[1, 3], &[false; 4], false), vec![1, 3]);
    assert_eq!(pictures_to_write(4, &[], &[false; 4], false), Vec::<usize>::new());
}

#[test]
fn a_stale_picture_is_written_even_when_nothing_changed() {
    // The values are the ones the node holds, so no cell changed, and the picture that failed to be written is still due.
    assert_eq!(pictures_to_write(3, &[], &[false, true, false], false), vec![1]);
    assert_eq!(pictures_to_write(3, &[], &[true, true, true], false), vec![0, 1, 2]);
}

#[test]
fn a_changed_and_stale_picture_is_written_once_and_the_list_is_in_order() {
    assert_eq!(
        pictures_to_write(5, &[3, 1], &[false, true, false, true, true], false),
        vec![1, 3, 4]
    );
    assert_eq!(pictures_to_write(3, &[2, 2], &[false; 3], false), vec![2]);
}

#[test]
fn a_new_scale_writes_every_picture() {
    assert_eq!(pictures_to_write(3, &[], &[false; 3], true), vec![0, 1, 2]);
    assert_eq!(pictures_to_write(0, &[], &[], true), Vec::<usize>::new());
}

#[test]
fn an_index_past_the_pictures_is_dropped_and_a_short_stale_list_is_not_an_error() {
    assert_eq!(pictures_to_write(2, &[1, 5], &[false, false], false), vec![1]);
    assert_eq!(pictures_to_write(3, &[], &[true], false), vec![0]);
    assert_eq!(pictures_to_write(2, &[], &[true, true, true, true], false), vec![0, 1]);
}
