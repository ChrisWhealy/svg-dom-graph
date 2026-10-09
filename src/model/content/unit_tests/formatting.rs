//! Per-value formatting: hexadecimal, binary and decimal digits, `ByteOrder`, and the streaming `for_each_cell_string`
//! and `widest_cell_string` that draw a node's cells.

use super::super::*;
use super::support::{cells, check_eq};

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
//   Per-value formatting
// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -

#[test]
fn hexadecimal_u64_matches_the_feature_requests_own_example() -> Result<(), String> {
    let content = DataNodeContent::new(NodeValues::U64(vec![0xF0E1D2C3B4A59687]), DataFormat::Hexadecimal);
    check_eq(cells(&content), vec!["F0 E1 D2 C3 B4 A5 96 87".to_owned()])
}

#[test]
fn hexadecimal_u8_is_a_single_byte_group() -> Result<(), String> {
    let content = DataNodeContent::new(NodeValues::U8(vec![0xAB]), DataFormat::Hexadecimal);
    check_eq(cells(&content), vec!["AB".to_owned()])
}

#[test]
fn hexadecimal_u16_is_two_byte_groups_big_endian() -> Result<(), String> {
    let content = DataNodeContent::new(NodeValues::U16(vec![0xABCD]), DataFormat::Hexadecimal);
    check_eq(cells(&content), vec!["AB CD".to_owned()])
}

#[test]
fn hexadecimal_u32_is_four_byte_groups_big_endian() -> Result<(), String> {
    let content = DataNodeContent::new(NodeValues::U32(vec![0xAABBCCDD]), DataFormat::Hexadecimal);
    check_eq(cells(&content), vec!["AA BB CC DD".to_owned()])
}

#[test]
fn binary_u8_splits_into_upper_and_lower_nybbles() -> Result<(), String> {
    let content = DataNodeContent::new(NodeValues::U8(vec![0xF0]), DataFormat::Binary);
    check_eq(cells(&content), vec!["1111 0000".to_owned()])
}

#[test]
fn binary_u16_splits_every_bytes_own_nybbles_big_endian() -> Result<(), String> {
    let content = DataNodeContent::new(NodeValues::U16(vec![0xF00F]), DataFormat::Binary);
    check_eq(cells(&content), vec!["1111 0000 0000 1111".to_owned()])
}

#[test]
fn decimal_does_not_split_into_bytes() -> Result<(), String> {
    let content = DataNodeContent::new(NodeValues::U32(vec![1_234_567]), DataFormat::Decimal);
    check_eq(cells(&content), vec!["1234567".to_owned()])
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
//   ByteOrder
// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -

#[test]
fn new_defaults_to_big_endian_byte_order() -> Result<(), String> {
    let content = DataNodeContent::new(NodeValues::U32(vec![0xAABBCCDD]), DataFormat::Hexadecimal);
    check_eq(cells(&content), vec!["AA BB CC DD".to_owned()])
}

#[test]
fn with_byte_order_little_endian_reverses_the_byte_groups() -> Result<(), String> {
    let content = DataNodeContent::new(NodeValues::U32(vec![0xAABBCCDD]), DataFormat::Hexadecimal)
        .with_byte_order(ByteOrder::LittleEndian);
    check_eq(cells(&content), vec!["DD CC BB AA".to_owned()])
}

#[test]
fn little_endian_also_reverses_binary_byte_groups() -> Result<(), String> {
    let content = DataNodeContent::new(NodeValues::U16(vec![0xF00F]), DataFormat::Binary)
        .with_byte_order(ByteOrder::LittleEndian);
    check_eq(cells(&content), vec!["0000 1111 1111 0000".to_owned()])
}

#[test]
fn byte_order_has_no_visible_effect_on_a_single_byte_value() -> Result<(), String> {
    let big_endian = DataNodeContent::new(NodeValues::U8(vec![0xAB]), DataFormat::Hexadecimal);
    let little_endian = DataNodeContent::new(NodeValues::U8(vec![0xAB]), DataFormat::Hexadecimal)
        .with_byte_order(ByteOrder::LittleEndian);
    check_eq(cells(&big_endian), cells(&little_endian))
}

#[test]
fn byte_order_has_no_visible_effect_under_decimal_format() -> Result<(), String> {
    let big_endian = DataNodeContent::new(NodeValues::U32(vec![1_234_567]), DataFormat::Decimal);
    let little_endian = DataNodeContent::new(NodeValues::U32(vec![1_234_567]), DataFormat::Decimal)
        .with_byte_order(ByteOrder::LittleEndian);
    check_eq(cells(&big_endian), cells(&little_endian))
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
//   DataNodeContent::for_each_cell_string — one string per value, not yet arranged into rows
// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -

#[test]
fn a_single_value_produces_one_cell() -> Result<(), String> {
    let content = DataNodeContent::new(NodeValues::U64(vec![0x1122334455667788]), DataFormat::Hexadecimal);
    check_eq(cells(&content).len(), 1)
}

#[test]
fn two_values_produce_two_cells_in_order() -> Result<(), String> {
    let content = DataNodeContent::new(
        NodeValues::U64(vec![0x1111111111111111, 0x2222222222222222]),
        DataFormat::Hexadecimal,
    );
    check_eq(
        cells(&content),
        vec!["11 11 11 11 11 11 11 11".to_owned(), "22 22 22 22 22 22 22 22".to_owned()],
    )
}

#[test]
fn twenty_five_values_produce_twenty_five_cells_arranged_as_a_five_by_five_grid() -> Result<(), String> {
    let values = (0..25u64).collect();
    let content = DataNodeContent::new(NodeValues::U64(values), DataFormat::Decimal);
    check_eq(cells(&content).len(), 25)?;
    check_eq(content.shape(), (5, 5))
}

#[test]
fn cells_are_returned_even_when_the_last_grid_row_is_only_partially_filled() -> Result<(), String> {
    // 7 values -> a 3x3 grid (see grid_shape_of_seven_values above), with 2 blank slots in the last row.
    // for_each_cell_string itself has no concept of blanks. It is draw_content_box's job to stop after the 7th.
    let values = (0..7u8).collect();
    let content = DataNodeContent::new(NodeValues::U8(values), DataFormat::Decimal);
    check_eq(cells(&content).len(), 7)?;
    check_eq(content.shape(), (3, 3))
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
//   DataNodeContent::widest_cell_string — the one value guaranteed to render the widest cell, found and formatted
//   without formatting every value first
// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -

#[test]
fn widest_cell_string_picks_the_largest_value_under_decimal() -> Result<(), String> {
    // Digit count grows with magnitude for an unsigned decimal value. So the numerically largest value, not whichever
    // happens to come first, is always the one with the most digits to render.
    let content = DataNodeContent::new(NodeValues::U32(vec![1, 4_294_967_295, 42]), DataFormat::Decimal);
    let mut out = String::new();
    content.widest_cell_string(&mut out);
    check_eq(out, "4294967295".to_owned())
}

#[test]
fn widest_cell_string_uses_any_value_under_hexadecimal_since_every_cell_shares_one_width() -> Result<(), String> {
    // Every value under one integer width renders the same number of hexadecimal characters regardless of its own
    // magnitude. So there is no widest value to search for. The first is exactly as representative as any other.
    let content = DataNodeContent::new(NodeValues::U16(vec![0x0001, 0xFFFF]), DataFormat::Hexadecimal);
    let mut out = String::new();
    content.widest_cell_string(&mut out);
    check_eq(out, "00 01".to_owned())
}

#[test]
fn widest_cell_string_uses_any_value_under_binary_since_every_cell_shares_one_width() -> Result<(), String> {
    let content = DataNodeContent::new(NodeValues::U8(vec![0x00, 0xFF]), DataFormat::Binary);
    let mut out = String::new();
    content.widest_cell_string(&mut out);
    check_eq(out, "0000 0000".to_owned())
}

#[test]
fn widest_cell_string_leaves_out_empty_for_content_with_no_values() -> Result<(), String> {
    let content = DataNodeContent::new(NodeValues::U8(Vec::new()), DataFormat::Decimal);
    let mut out = String::from("stale");
    content.widest_cell_string(&mut out);
    check_eq(out, String::new())
}
