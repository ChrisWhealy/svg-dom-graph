//! `NodeValues::Bits`: validation, the digits of each format, the width of a cell, and replacing values.

use super::super::*;
use super::support::{cells, check_eq};
use crate::error::Error;
use crate::test_support::check;

fn bits(width: u8, values: Vec<u8>, format: DataFormat) -> Result<DataNodeContent, Error> {
    Ok(DataNodeContent::new(NodeValues::Bits(BitValues::new(width, values)?), format))
}

fn texts(width: u8, values: Vec<u8>, format: DataFormat) -> Result<Vec<String>, String> {
    Ok(cells(&bits(width, values, format).map_err(|e| e.to_string())?))
}

fn strings(values: &[&str]) -> Vec<String> {
    values.iter().map(ToString::to_string).collect()
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[test]
fn a_width_of_zero_or_eight_or_more_is_rejected() -> Result<(), String> {
    for width in [0, 8, 9, 255] {
        check(
            matches!(BitValues::new(width, vec![0]), Err(Error::InvalidBitValues { index: None, .. })),
            &format!("width {width} was accepted"),
        )?;
    }
    check(
        BitValues::new(1, vec![]).is_ok() && BitValues::new(7, vec![]).is_ok(),
        "the limits are 1 and 7",
    )
}

#[test]
fn a_value_too_wide_for_its_width_is_rejected_with_its_index() -> Result<(), String> {
    check(
        matches!(
            BitValues::new(6, vec![0, 63, 64, 65]),
            Err(Error::InvalidBitValues { width: 6, index: Some(2) })
        ),
        "64 does not fit in six bits",
    )?;
    check(BitValues::new(6, vec![0, 63]).is_ok(), "63 fits in six bits")
}

#[test]
fn binary_writes_exactly_the_width_in_digits_with_no_grouping() -> Result<(), String> {
    check_eq(
        texts(6, vec![0, 19, 63], DataFormat::Binary)?,
        strings(&["000000", "010011", "111111"]),
    )?;
    check_eq(texts(1, vec![0, 1], DataFormat::Binary)?, strings(&["0", "1"]))?;
    check_eq(texts(7, vec![127, 5], DataFormat::Binary)?, strings(&["1111111", "0000101"]))
}

#[test]
fn hexadecimal_writes_one_digit_for_each_four_bits_or_part_of_four() -> Result<(), String> {
    check_eq(texts(4, vec![0, 10, 15], DataFormat::Hexadecimal)?, strings(&["0", "A", "F"]))?;
    check_eq(texts(6, vec![5, 63], DataFormat::Hexadecimal)?, strings(&["05", "3F"]))?;
    check_eq(texts(7, vec![127], DataFormat::Hexadecimal)?, strings(&["7F"]))
}

#[test]
fn decimal_writes_the_plain_number() -> Result<(), String> {
    check_eq(texts(6, vec![0, 19, 63], DataFormat::Decimal)?, strings(&["0", "19", "63"]))
}

#[test]
fn the_widest_cell_is_the_largest_in_decimal_and_any_in_the_other_formats() -> Result<(), String> {
    let widest = |content: &DataNodeContent| {
        let mut out = String::new();
        content.widest_cell_string(&mut out);
        out
    };
    let decimal = bits(6, vec![3, 63, 7], DataFormat::Decimal).map_err(|e| e.to_string())?;
    check_eq(widest(&decimal), "63".to_owned())?;
    let binary = bits(6, vec![3, 63, 7], DataFormat::Binary).map_err(|e| e.to_string())?;
    check_eq(widest(&binary).len(), 6)
}

#[test]
fn a_node_says_its_width_and_equality_follows_width_and_values() -> Result<(), String> {
    let six = bits(6, vec![1], DataFormat::Binary).map_err(|e| e.to_string())?;
    check_eq(six.type_name(), "6-bit")?;
    check(
        NodeValues::Bits(BitValues::new(6, vec![1]).map_err(|e| e.to_string())?)
            != NodeValues::Bits(BitValues::new(5, vec![1]).map_err(|e| e.to_string())?),
        "the same value at two widths is not the same",
    )?;
    check(
        NodeValues::Bits(BitValues::new(6, vec![1]).map_err(|e| e.to_string())?) != NodeValues::U8(vec![1]),
        "six-bit values are not u8 values",
    )
}

#[test]
fn only_the_values_that_changed_are_formatted() -> Result<(), String> {
    let old = NodeValues::Bits(BitValues::new(6, vec![1, 2, 3]).map_err(|e| e.to_string())?);
    let new = NodeValues::Bits(BitValues::new(6, vec![1, 9, 3]).map_err(|e| e.to_string())?);
    let mut seen = Vec::new();
    let mut scratch = String::new();
    new.try_for_each_changed_cell_string(&old, DataFormat::Binary, ByteOrder::BigEndian, &mut scratch, |i, text| {
        seen.push((i, text.to_owned()));
        Ok::<(), ()>(())
    })
    .map_err(|()| "unexpected error".to_owned())?;
    check_eq(seen, vec![(1, "001001".to_owned())])
}

#[test]
fn values_of_another_width_cannot_replace_them() -> Result<(), String> {
    let mut content = bits(6, vec![1, 2], DataFormat::Binary).map_err(|e| e.to_string())?;
    let five = NodeValues::Bits(BitValues::new(5, vec![1, 2]).map_err(|e| e.to_string())?);
    let six = NodeValues::Bits(BitValues::new(6, vec![1, 9]).map_err(|e| e.to_string())?);
    let mut scratch = String::new();
    check_eq(
        content.try_replace_values(five, &mut scratch, |_, _| Ok::<(), ()>(())),
        Ok(false),
    )?;
    check_eq(content.try_replace_values(six, &mut scratch, |_, _| Ok::<(), ()>(())), Ok(true))
}
