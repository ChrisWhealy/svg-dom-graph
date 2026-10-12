//! `NodeValues::F32`/`F64`: every `FloatFormat`, special values, IEEE-754 bits, and above all the width of a cell, which
//! for a float depends on its sign, its integer digits, its leading zeros, its exponent, and whether it is `NaN` or
//! infinite.

use super::super::*;
use super::support::{cells, check_eq};

fn as_f64(values: Vec<f64>, format: FloatFormat) -> DataNodeContent {
    DataNodeContent::new(NodeValues::F64(values), DataFormat::Float(format))
}

/// The text of one `f64` under `format`.
fn one(value: f64, format: FloatFormat) -> String {
    cells(&as_f64(vec![value], format)).remove(0)
}

fn widest(values: Vec<f64>, format: FloatFormat) -> String {
    let mut out = String::new();
    as_f64(values, format).widest_cell_string(&mut out);
    out
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
//   The shortest digits
// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -

#[test]
fn decimal_is_the_shortest_digits_that_read_back_as_the_same_value() -> Result<(), String> {
    let content = DataNodeContent::new(NodeValues::F64(vec![0.1, 1.0, -2.5, 0.1 + 0.2]), DataFormat::Decimal);
    check_eq(
        cells(&content),
        ["0.1", "1", "-2.5", "0.30000000000000004"].map(String::from).to_vec(),
    )
}

#[test]
fn an_f32_and_an_f64_of_the_same_literal_differ_because_they_are_different_values() -> Result<(), String> {
    let single = DataNodeContent::new(NodeValues::F32(vec![0.1]), DataFormat::Decimal);
    check_eq(cells(&single), vec!["0.1".to_owned()])?;
    check_eq(
        one(f64::from(0.1_f32), FloatFormat::shortest()),
        "0.10000000149011612".to_owned(),
    )
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
//   Decimal places
// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -

#[test]
fn decimal_places_count_the_digits_after_the_point() -> Result<(), String> {
    let format = FloatFormat::decimal_places(2);
    check_eq(one(std::f64::consts::PI, format), "3.14".to_owned())?;
    check_eq(one(1234.5, format), "1234.50".to_owned())?;
    check_eq(one(0.0, format), "0.00".to_owned())?;
    check_eq(one(-0.001, format), "-0.00".to_owned())
}

#[test]
fn zero_decimal_places_show_no_point() -> Result<(), String> {
    let format = FloatFormat::decimal_places(0);
    check_eq(one(2.6, format), "3".to_owned())?;
    check_eq(one(-2.6, format), "-3".to_owned())
}

#[test]
fn a_negative_zero_keeps_its_sign() -> Result<(), String> {
    check_eq(one(-0.0, FloatFormat::decimal_places(1)), "-0.0".to_owned())?;
    check_eq(one(-0.0, FloatFormat::shortest()), "-0".to_owned())?;
    check_eq(one(0.0, FloatFormat::shortest()), "0".to_owned())
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
//   Significant digits
// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -

#[test]
fn significant_digits_count_from_the_first_nonzero_digit() -> Result<(), String> {
    let format = FloatFormat::significant_digits(3);
    check_eq(one(std::f64::consts::PI, format), "3.14".to_owned())?;
    check_eq(one(1234.5, format), "1230".to_owned())?;
    check_eq(one(-1234.5, format), "-1230".to_owned())?;
    check_eq(one(0.0012345, format), "0.00123".to_owned())
}

#[test]
fn a_value_always_shows_exactly_the_significant_digits_asked_for() -> Result<(), String> {
    let format = FloatFormat::significant_digits(3);
    check_eq(one(5.0, format), "5.00".to_owned())?;
    check_eq(one(0.0, format), "0.00".to_owned())?;
    check_eq(one(100.0, format), "100".to_owned())?;
    check_eq(one(0.5, format), "0.500".to_owned())
}

#[test]
fn rounding_up_to_a_new_power_of_ten_moves_the_point() -> Result<(), String> {
    check_eq(one(9.99, FloatFormat::significant_digits(2)), "10".to_owned())?;
    check_eq(one(99.95, FloatFormat::significant_digits(3)), "100".to_owned())?;
    check_eq(one(0.0999, FloatFormat::significant_digits(2)), "0.10".to_owned())
}

#[test]
fn significant_digits_of_a_huge_or_tiny_value_are_padded_with_zeros() -> Result<(), String> {
    let format = FloatFormat::significant_digits(3);
    check_eq(one(1e10, format), "10000000000".to_owned())?;
    check_eq(one(1e-10, format), "0.000000000100".to_owned())
}

#[test]
fn one_significant_digit_has_no_point() -> Result<(), String> {
    let format = FloatFormat::significant_digits(1);
    check_eq(one(7.0, format), "7".to_owned())?;
    check_eq(one(0.0, format), "0".to_owned())?;
    check_eq(one(0.04, format), "0.04".to_owned())
}

#[test]
fn significant_digits_and_decimal_places_are_different_things() -> Result<(), String> {
    // The same three digits asked for two ways, over three values of very different size.
    let places = FloatFormat::decimal_places(3);
    let digits = FloatFormat::significant_digits(3);
    let values = [1234.5678, 1.2345678, 0.0012345678];
    check_eq(values.map(|v| one(v, places)), ["1234.568", "1.235", "0.001"].map(String::from))?;
    check_eq(values.map(|v| one(v, digits)), ["1230", "1.23", "0.00123"].map(String::from))
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
//   Scientific notation
// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -

#[test]
fn scientific_shortest_writes_a_mantissa_and_an_exponent() -> Result<(), String> {
    let format = FloatFormat::shortest().scientific();
    check_eq(one(1234.5, format), "1.2345e3".to_owned())?;
    check_eq(one(0.00012, format), "1.2e-4".to_owned())?;
    check_eq(one(-5.0, format), "-5e0".to_owned())
}

#[test]
fn a_scientific_precision_counts_the_digits_of_the_mantissa() -> Result<(), String> {
    check_eq(one(1234.56, FloatFormat::decimal_places(3).scientific()), "1.235e3".to_owned())?;
    check_eq(
        one(1234.56, FloatFormat::significant_digits(4).scientific()),
        "1.235e3".to_owned(),
    )?;
    check_eq(one(1234.56, FloatFormat::significant_digits(1).scientific()), "1e3".to_owned())?;
    check_eq(one(0.0, FloatFormat::decimal_places(2).scientific()), "0.00e0".to_owned())
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
//   Special values
// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -

#[test]
fn nan_and_the_infinities_read_the_same_in_every_format() -> Result<(), String> {
    for format in [
        FloatFormat::shortest(),
        FloatFormat::decimal_places(3),
        FloatFormat::significant_digits(4),
        FloatFormat::shortest().scientific(),
        FloatFormat::decimal_places(2).scientific(),
    ] {
        check_eq(one(f64::NAN, format), "NaN".to_owned())?;
        check_eq(one(f64::INFINITY, format), "inf".to_owned())?;
        check_eq(one(f64::NEG_INFINITY, format), "-inf".to_owned())?;
    }
    Ok(())
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
//   IEEE-754 bits
// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -

#[test]
fn hexadecimal_shows_the_ieee_754_bits() -> Result<(), String> {
    let single = DataNodeContent::new(NodeValues::F32(vec![1.0, -2.0, -0.0]), DataFormat::Hexadecimal);
    check_eq(
        cells(&single),
        ["3F 80 00 00", "C0 00 00 00", "80 00 00 00"].map(String::from).to_vec(),
    )?;
    let double = DataNodeContent::new(NodeValues::F64(vec![1.0]), DataFormat::Hexadecimal);
    check_eq(cells(&double), vec!["3F F0 00 00 00 00 00 00".to_owned()])
}

#[test]
fn binary_shows_the_ieee_754_bits_and_honours_the_byte_order() -> Result<(), String> {
    let big = DataNodeContent::new(NodeValues::F32(vec![1.0]), DataFormat::Binary);
    check_eq(cells(&big), vec!["0011 1111 1000 0000 0000 0000 0000 0000".to_owned()])?;
    let little = DataNodeContent::new(NodeValues::F32(vec![-2.0]), DataFormat::Hexadecimal)
        .with_byte_order(ByteOrder::LittleEndian);
    check_eq(cells(&little), vec!["00 00 00 C0".to_owned()])
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
//   The widest cell
// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -

#[test]
fn the_widest_decimal_places_cell_counts_the_integer_digits_and_the_sign() -> Result<(), String> {
    let format = FloatFormat::decimal_places(1);
    check_eq(widest(vec![0.5, 99.5, 7.0], format), "99.5".to_owned())?;
    check_eq(widest(vec![0.5, 99.5, -1000.5], format), "-1000.5".to_owned())?;
    // `-12.5` and `123.5` are the same width, so either may be chosen. It is the width that matters.
    check_eq(widest(vec![-12.5, 123.5], format).len(), 5)
}

#[test]
fn a_special_value_can_be_wider_than_every_number() -> Result<(), String> {
    let format = FloatFormat::decimal_places(1);
    check_eq(widest(vec![0.5, f64::NEG_INFINITY], format), "-inf".to_owned())?;
    // `NaN` is no wider than `0.5`. Written with no decimal places, `0.5` is a single character, and `NaN` is wider.
    check_eq(widest(vec![0.5, f64::NAN], FloatFormat::decimal_places(0)), "NaN".to_owned())?;
    // But not when the numbers are already wider.
    check_eq(widest(vec![12345.5, f64::NEG_INFINITY], format), "12345.5".to_owned())
}

#[test]
fn a_negative_zero_is_wider_than_a_zero() -> Result<(), String> {
    check_eq(widest(vec![0.0, -0.0], FloatFormat::decimal_places(2)), "-0.00".to_owned())
}

#[test]
fn the_widest_significant_digits_cell_may_be_the_tiniest_value() -> Result<(), String> {
    let format = FloatFormat::significant_digits(3);
    // The leading zeros of a tiny value make it far wider than a large one: `1.23` is 4 characters, and
    // `0.00000000123` is 13.
    check_eq(widest(vec![1.23, 1.23e-9, 4.5], format), "0.00000000123".to_owned())?;
    check_eq(widest(vec![-1.23e-9, 1e3], format), "-0.00000000123".to_owned())
}

#[test]
fn the_widest_significant_digits_cell_may_be_the_largest_value() -> Result<(), String> {
    let format = FloatFormat::significant_digits(3);
    check_eq(widest(vec![1.23, 1.23e9, 4.5], format), "1230000000".to_owned())
}

#[test]
fn the_widest_scientific_cell_follows_the_digits_and_sign_of_the_exponent() -> Result<(), String> {
    let format = FloatFormat::decimal_places(2).scientific();
    check_eq(widest(vec![1.0, 1e-100], format), "1.00e-100".to_owned())?;
    check_eq(widest(vec![1.0, 1e100, 1e-5], format), "1.00e100".to_owned())?;
    check_eq(widest(vec![1.0, 2.0], format).len(), 6)
}

#[test]
fn the_widest_shortest_cell_is_found_by_measuring_every_value() -> Result<(), String> {
    // `0.5` is the largest, but `0.1 + 0.2` is by far the longest to write.
    let format = FloatFormat::shortest();
    check_eq(widest(vec![0.5, 0.1 + 0.2, 0.25], format), "0.30000000000000004".to_owned())
}

#[test]
fn hexadecimal_and_binary_cells_are_all_one_width() -> Result<(), String> {
    let mut out = String::new();
    DataNodeContent::new(NodeValues::F32(vec![1.0, -2.0]), DataFormat::Hexadecimal).widest_cell_string(&mut out);
    check_eq(out.len(), "3F 80 00 00".len())
}

#[test]
fn nothing_is_widest_of_no_values() -> Result<(), String> {
    check_eq(widest(Vec::new(), FloatFormat::decimal_places(2)), String::new())
}

#[test]
fn no_value_of_a_node_is_wider_than_its_widest_cell() -> Result<(), String> {
    // The point of all of the above, checked by brute force against the real formatting: for every format, and every
    // triple drawn from values of every size, sign and kind, the cell is as wide as the widest value.
    let samples = [
        0.0,
        -0.0,
        1.0,
        -1.0,
        0.5,
        -0.001,
        7.25,
        99.5,
        -1234.5678,
        1.23e-9,
        -4.56e-7,
        9.99e15,
        1e100,
        -1e-100,
        f64::NAN,
        f64::INFINITY,
        f64::NEG_INFINITY,
        0.1 + 0.2,
    ];
    let formats = [
        FloatFormat::shortest(),
        FloatFormat::decimal_places(0),
        FloatFormat::decimal_places(3),
        FloatFormat::significant_digits(1),
        FloatFormat::significant_digits(4),
        FloatFormat::shortest().scientific(),
        FloatFormat::decimal_places(2).scientific(),
        FloatFormat::significant_digits(5).scientific(),
    ];
    for format in formats {
        for a in samples {
            for b in samples {
                for c in samples {
                    let values = vec![a, b, c];
                    let longest = values.iter().map(|&v| one(v, format).len()).max().unwrap_or(0);
                    check_eq(widest(values.clone(), format).len(), longest)
                        .map_err(|e| format!("{values:?} under {format:?}: {e}"))?;
                }
            }
        }
    }
    Ok(())
}

#[test]
fn the_same_holds_for_an_f32() -> Result<(), String> {
    let values = vec![1.5_f32, -123456.0, 1.0e-6, f32::NAN];
    for format in [
        FloatFormat::decimal_places(2),
        FloatFormat::significant_digits(3),
        FloatFormat::shortest(),
    ] {
        let content = DataNodeContent::new(NodeValues::F32(values.clone()), DataFormat::Float(format));
        let longest = cells(&content).iter().map(String::len).max().unwrap_or(0);
        let mut out = String::new();
        content.widest_cell_string(&mut out);
        check_eq(out.len(), longest).map_err(|e| format!("{format:?}: {e}"))?;
    }
    Ok(())
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
//   Validity
// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -

#[test]
fn a_float_format_is_valid_for_floats_with_a_sensible_precision() -> Result<(), String> {
    for format in [
        FloatFormat::shortest(),
        FloatFormat::decimal_places(0),
        FloatFormat::decimal_places(MAX_FLOAT_DIGITS),
        FloatFormat::significant_digits(1),
        FloatFormat::significant_digits(MAX_FLOAT_DIGITS).scientific(),
    ] {
        check_eq(as_f64(vec![1.0], format).float_format_is_valid(), true)?;
    }
    Ok(())
}

#[test]
fn a_float_format_is_not_valid_for_an_excessive_or_empty_precision() -> Result<(), String> {
    check_eq(
        as_f64(vec![1.0], FloatFormat::decimal_places(MAX_FLOAT_DIGITS + 1)).float_format_is_valid(),
        false,
    )?;
    check_eq(
        as_f64(vec![1.0], FloatFormat::significant_digits(0)).float_format_is_valid(),
        false,
    )?;
    check_eq(
        as_f64(vec![1.0], FloatFormat::significant_digits(MAX_FLOAT_DIGITS + 1)).float_format_is_valid(),
        false,
    )
}

#[test]
fn a_float_format_is_not_valid_for_integers() -> Result<(), String> {
    let content = DataNodeContent::new(NodeValues::U32(vec![1]), DataFormat::Float(FloatFormat::shortest()));
    check_eq(content.float_format_is_valid(), false)?;
    let signed = DataNodeContent::new(NodeValues::I8(vec![1]), DataFormat::Float(FloatFormat::shortest()));
    check_eq(signed.float_format_is_valid(), false)
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
//   Replacing values
// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -

#[test]
fn replacing_a_zero_with_a_negative_zero_is_a_change() -> Result<(), String> {
    let mut content = as_f64(vec![0.0, 1.0], FloatFormat::decimal_places(1));
    check_eq(content.differs_from(&NodeValues::F64(vec![-0.0, 1.0])), true)?;
    let mut written = Vec::new();
    let mut scratch = String::new();
    let replaced = content.try_replace_values(NodeValues::F64(vec![-0.0, 1.0]), &mut scratch, |i, text| {
        written.push((i, text.to_owned()));
        Ok::<(), String>(())
    })?;
    check_eq(replaced, true)?;
    check_eq(written, vec![(0, "-0.0".to_owned())])
}

#[test]
fn replacing_a_nan_with_a_nan_is_not_a_change() -> Result<(), String> {
    let mut content = as_f64(vec![f64::NAN, 1.0], FloatFormat::shortest());
    check_eq(content.differs_from(&NodeValues::F64(vec![f64::NAN, 1.0])), false)?;
    let mut scratch = String::new();
    let mut calls = 0;
    content.try_replace_values(NodeValues::F64(vec![f64::NAN, 1.0]), &mut scratch, |_, _| {
        calls += 1;
        Ok::<(), String>(())
    })?;
    check_eq(calls, 0)
}

#[test]
fn replacing_an_f32_with_an_f64_is_refused() -> Result<(), String> {
    let mut content = DataNodeContent::new(NodeValues::F32(vec![1.0]), DataFormat::Decimal);
    let refused =
        content.try_replace_values(NodeValues::F64(vec![1.0]), &mut String::new(), |_, _| Ok::<(), String>(()))?;
    check_eq(refused, false)
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
//   Naming and colour
// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -

#[test]
fn floats_are_named_and_coloured_apart_from_every_integer() -> Result<(), String> {
    check_eq(NodeValues::F32(vec![0.0]).type_name(), "f32")?;
    check_eq(NodeValues::F64(vec![0.0]).type_name(), "f64")?;
    let every = [
        NodeValues::U8(vec![0]).type_colour(),
        NodeValues::U16(vec![0]).type_colour(),
        NodeValues::U32(vec![0]).type_colour(),
        NodeValues::U64(vec![0]).type_colour(),
        NodeValues::I8(vec![0]).type_colour(),
        NodeValues::I16(vec![0]).type_colour(),
        NodeValues::I32(vec![0]).type_colour(),
        NodeValues::I64(vec![0]).type_colour(),
        NodeValues::F32(vec![0.0]).type_colour(),
        NodeValues::F64(vec![0.0]).type_colour(),
    ];
    let mut unique = every.to_vec();
    unique.sort_unstable();
    unique.dedup();
    check_eq(unique.len(), every.len())
}
