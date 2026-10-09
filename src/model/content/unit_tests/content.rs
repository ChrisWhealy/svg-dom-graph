//! `DataNodeContent`'s own basic properties: the default and overridden layout, the value count, whether it holds a
//! single value, and its type colour and name.

use super::super::*;
use super::support::check_eq;

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
//   is_single_value / type_colour
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
fn every_width_gets_a_distinct_type_colour() -> Result<(), String> {
    let colours = [
        DataNodeContent::new(NodeValues::U8(vec![1]), DataFormat::Decimal).type_colour(),
        DataNodeContent::new(NodeValues::U16(vec![1]), DataFormat::Decimal).type_colour(),
        DataNodeContent::new(NodeValues::U32(vec![1]), DataFormat::Decimal).type_colour(),
        DataNodeContent::new(NodeValues::U64(vec![1]), DataFormat::Decimal).type_colour(),
    ];
    let mut unique = colours.to_vec();
    unique.sort_unstable();
    unique.dedup();
    check_eq(unique.len(), colours.len())
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
