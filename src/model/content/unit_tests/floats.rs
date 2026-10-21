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

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
//   The shortest digits
// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -

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

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
//   Decimal places
// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -

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

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
//   Significant digits
// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -

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

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
//   Scientific notation
// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -

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

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
//   Special values
// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -

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

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
//   IEEE-754 bits
// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -

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

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
//   The widest cell
// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -

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

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
//   Validity
// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -

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

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
//   Replacing values
// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -

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

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
//   Naming and colour
// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -

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

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
//   Equality
// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -

#[test]
fn node_values_and_content_are_eq() {
    fn is_eq<T: Eq>() {}
    is_eq::<NodeValues>();
    is_eq::<DataNodeContent>();
}

#[test]
fn equality_is_by_bits_so_a_negative_zero_is_not_a_zero() -> Result<(), String> {
    check_eq(NodeValues::F64(vec![0.0]) == NodeValues::F64(vec![-0.0]), false)?;
    check_eq(NodeValues::F32(vec![0.0]) == NodeValues::F32(vec![-0.0]), false)?;
    check_eq(NodeValues::F64(vec![0.0]) == NodeValues::F64(vec![0.0]), true)
}

#[test]
fn equality_is_by_bits_so_a_nan_equals_itself() -> Result<(), String> {
    let nan = NodeValues::F64(vec![f64::NAN, 1.0]);
    check_eq(nan == nan.clone(), true)?;
    check_eq(NodeValues::F32(vec![f32::NAN]) == NodeValues::F32(vec![f32::NAN]), true)
}

#[test]
fn equality_needs_the_same_type_and_the_same_count() -> Result<(), String> {
    check_eq(NodeValues::F32(vec![1.0]) == NodeValues::F64(vec![1.0]), false)?;
    check_eq(NodeValues::F64(vec![1.0]) == NodeValues::F64(vec![1.0, 1.0]), false)?;
    check_eq(NodeValues::I8(vec![1]) == NodeValues::U8(vec![1]), false)?;
    check_eq(NodeValues::I8(vec![-1, 2]) == NodeValues::I8(vec![-1, 2]), true)
}

#[test]
fn content_equality_follows_its_values_and_so_agrees_with_change_detection() -> Result<(), String> {
    let zero = as_f64(vec![0.0], FloatFormat::shortest());
    let negative_zero = as_f64(vec![-0.0], FloatFormat::shortest());
    check_eq(zero == negative_zero, false)?;
    check_eq(zero.differs_from(&NodeValues::F64(vec![-0.0])), true)?;
    let nan = as_f64(vec![f64::NAN], FloatFormat::shortest());
    check_eq(nan == nan.clone(), true)?;
    check_eq(nan.differs_from(&NodeValues::F64(vec![f64::NAN])), false)
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
//   The edges of the range
// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -

/// The decimal exponent of `value`, as Rust writes it in scientific notation: `-324` for the smallest `f64`.
fn exponent_of(value: f64) -> i32 {
    format!("{value:e}")
        .split_once('e')
        .and_then(|(_, e)| e.parse().ok())
        .unwrap_or(0)
}

#[test]
fn the_smallest_and_largest_values_read_back_as_themselves_in_the_shortest_form() -> Result<(), String> {
    let format = FloatFormat::shortest().scientific();
    for value in [f64::MIN_POSITIVE, f64::from_bits(1), f64::MAX, -f64::MAX, -f64::from_bits(1)] {
        let text = one(value, format);
        check_eq(text.parse::<f64>().ok(), Some(value)).map_err(|e| format!("{text}: {e}"))?;
    }
    check_eq(one(f64::MAX, format), "1.7976931348623157e308".to_owned())?;
    check_eq(one(f64::from_bits(1), format), "5e-324".to_owned())?;
    let single =
        |value: f32| cells(&DataNodeContent::new(NodeValues::F32(vec![value]), DataFormat::Float(format))).remove(0);
    check_eq(single(f32::MAX), "3.4028235e38".to_owned())?;
    check_eq(single(f32::from_bits(1)), "1e-45".to_owned())?;
    check_eq(single(f32::MIN_POSITIVE).parse::<f32>().ok(), Some(f32::MIN_POSITIVE))
}

#[test]
fn the_normal_and_subnormal_boundary_is_not_a_boundary_for_formatting() -> Result<(), String> {
    // The largest subnormal and the smallest normal differ by one bit, and are written as neighbouring numbers.
    let largest_subnormal = f64::from_bits(f64::MIN_POSITIVE.to_bits() - 1);
    for value in [largest_subnormal, f64::MIN_POSITIVE] {
        let text = one(value, FloatFormat::shortest().scientific());
        check_eq(text.parse::<f64>().ok(), Some(value)).map_err(|e| format!("{text}: {e}"))?;
        check_eq(exponent_of(value), -308)?;
    }
    Ok(())
}

#[test]
fn plain_notation_expands_the_largest_and_smallest_values_in_full() -> Result<(), String> {
    // `f64::MAX` is a 309-digit number, so it takes 309 characters with 3 significant digits, and 312 with 2 decimal
    // places.
    let digits = one(f64::MAX, FloatFormat::significant_digits(3));
    check_eq(digits.len(), 309)?;
    check_eq(digits.starts_with("180"), true)?;
    check_eq(digits[3..].bytes().all(|b| b == b'0'), true)?;
    let places = one(f64::MAX, FloatFormat::decimal_places(2));
    check_eq(places.len(), 312)?;
    check_eq(places.starts_with("17976931348623157"), true)?;
    check_eq(places.ends_with(".00"), true)?;
    // The smallest subnormal has 323 leading zeros after its point.
    let tiny = one(f64::from_bits(1), FloatFormat::significant_digits(3));
    check_eq(tiny.len(), 2 + 323 + 3)?;
    check_eq(tiny.starts_with("0.000"), true)?;
    check_eq(tiny.ends_with("494"), true)?;
    check_eq(one(f64::from_bits(1), FloatFormat::decimal_places(2)), "0.00".to_owned())
}

#[test]
fn the_widest_cell_of_the_extremes_follows_their_exponents() -> Result<(), String> {
    let format = FloatFormat::significant_digits(3);
    // The tiniest value has the most leading zeros, and `f64::MAX` the most trailing ones. The tiniest is wider.
    let widest_text = widest(vec![f64::MAX, f64::from_bits(1), 1.0], format);
    check_eq(widest_text.len(), 2 + 323 + 3)?;
    check_eq(widest(vec![f64::MAX, 1.0], format).len(), 309)?;
    check_eq(widest(vec![-f64::MAX, f64::MAX], FloatFormat::decimal_places(0)).len(), 310)
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
//   The most digits
// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -

#[test]
fn forty_significant_digits_fit_and_read_back_exactly() -> Result<(), String> {
    let format = FloatFormat::significant_digits(MAX_FLOAT_DIGITS);
    for value in [std::f64::consts::PI, 1.0 / 3.0, 1e-7, 123_456.789, f64::MAX, -2.5] {
        let text = one(value, format);
        check_eq(text.bytes().filter(u8::is_ascii_digit).count() >= 40, true).map_err(|e| format!("{text}: {e}"))?;
        check_eq(text.parse::<f64>().ok(), Some(value)).map_err(|e| format!("{text}: {e}"))?;
    }
    // Pi is `3.141592653589793115997963468544185161590576171875` exactly, so its 40 digits end by rounding up.
    check_eq(
        one(std::f64::consts::PI, format),
        "3.141592653589793115997963468544185161591".to_owned(),
    )?;
    check_eq(one(5.0, format), format!("5.{}", "0".repeat(39)))
}

#[test]
fn forty_decimal_places_fit_and_read_back_exactly() -> Result<(), String> {
    let format = FloatFormat::decimal_places(MAX_FLOAT_DIGITS);
    check_eq(one(0.5, format), format!("0.5{}", "0".repeat(39)))?;
    let pi = one(std::f64::consts::PI, format);
    check_eq(pi, "3.1415926535897931159979634685441851615906".to_owned())?;
    check_eq(one(-1.0, format), format!("-1.{}", "0".repeat(40)))?;
    check_eq(one(1e-7, format).parse::<f64>().ok(), Some(1e-7))
}

#[test]
fn forty_scientific_digits_fit() -> Result<(), String> {
    let digits = one(
        std::f64::consts::PI,
        FloatFormat::significant_digits(MAX_FLOAT_DIGITS).scientific(),
    );
    check_eq(digits, "3.141592653589793115997963468544185161591e0".to_owned())?;
    let places = one(-1234.5, FloatFormat::decimal_places(MAX_FLOAT_DIGITS).scientific());
    check_eq(places, format!("-1.2345{}e3", "0".repeat(36)))
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
//   Rounding at a power of ten
// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -

#[test]
fn rounding_up_to_a_power_of_ten_lengthens_the_integer_part_and_the_exponent() -> Result<(), String> {
    check_eq(one(9.96, FloatFormat::decimal_places(1)), "10.0".to_owned())?;
    check_eq(one(99.96, FloatFormat::decimal_places(1)), "100.0".to_owned())?;
    check_eq(one(999.5, FloatFormat::decimal_places(0)).len(), 4)?;
    check_eq(one(9.96, FloatFormat::decimal_places(1).scientific()), "1.0e1".to_owned())?;
    check_eq(one(9.9999, FloatFormat::significant_digits(2).scientific()), "1.0e1".to_owned())?;
    check_eq(
        one(9.9999e99, FloatFormat::significant_digits(2).scientific()),
        "1.0e100".to_owned(),
    )?;
    check_eq(
        one(9.9999e-101, FloatFormat::significant_digits(2).scientific()),
        "1.0e-100".to_owned(),
    )?;
    // Rounding down at the other end keeps the shorter form.
    check_eq(
        one(1.00001e100, FloatFormat::significant_digits(2).scientific()),
        "1.0e100".to_owned(),
    )?;
    check_eq(one(0.0999, FloatFormat::significant_digits(1)), "0.1".to_owned())
}

/// Values just either side of a power of ten, and of the point where rounding carries into it, for every power in a
/// range wide enough to cross the lengths of one, two and three exponent digits, and both signs.
fn values_around_powers_of_ten() -> Vec<f64> {
    let mut values = vec![0.0, -0.0];
    for power in [
        -101, -100, -99, -12, -11, -10, -9, -4, -3, -2, -1, 0, 1, 2, 3, 4, 9, 10, 11, 12, 99, 100, 101,
    ] {
        let ten = 10_f64.powi(power);
        for factor in [
            0.5, 0.95, 0.995, 0.9995, 0.99999999, 1.0, 1.0000001, 1.5, 9.5, 9.95, 9.9995, 9.99999999,
        ] {
            values.push(ten * factor);
            values.push(-ten * factor);
        }
    }
    values.extend([
        f64::MIN_POSITIVE,
        f64::from_bits(1),
        f64::MAX,
        -f64::MAX,
        f64::NAN,
        f64::INFINITY,
        f64::NEG_INFINITY,
    ]);
    values
}

fn every_format() -> Vec<FloatFormat> {
    let mut formats = vec![FloatFormat::shortest(), FloatFormat::shortest().scientific()];
    for n in [0, 1, 2, 5, MAX_FLOAT_DIGITS] {
        formats.push(FloatFormat::decimal_places(n));
        formats.push(FloatFormat::decimal_places(n).scientific());
    }
    for n in [1, 2, 3, 6, MAX_FLOAT_DIGITS] {
        formats.push(FloatFormat::significant_digits(n));
        formats.push(FloatFormat::significant_digits(n).scientific());
    }
    formats
}

#[test]
fn the_widest_cell_of_every_value_around_every_power_of_ten_is_never_beaten() -> Result<(), String> {
    // One node holding the whole range: if a value in the middle were ever wider than the extremes the shortcut looks
    // at, it would beat the cell here. Checked against formatting every value, for every format.
    let values = values_around_powers_of_ten();
    for format in every_format() {
        let longest = values.iter().map(|&v| one(v, format).len()).max().unwrap_or(0);
        check_eq(widest(values.clone(), format).len(), longest).map_err(|e| format!("{format:?}: {e}"))?;
    }
    Ok(())
}

#[test]
fn sets_of_values_around_powers_of_ten_never_beat_their_widest_cell() -> Result<(), String> {
    // Many small sets, drawn deterministically, so that a value which is not an extreme of the whole range is the middle
    // of a set it could beat.
    let pool = values_around_powers_of_ten();
    let mut state: u64 = 0x9E37_79B9_7F4A_7C15;
    let mut next = move || {
        state ^= state << 13;
        state ^= state >> 7;
        state ^= state << 17;
        usize::try_from(state % pool.len() as u64).unwrap_or(0)
    };
    let pool = values_around_powers_of_ten();
    for format in every_format() {
        for _ in 0..400 {
            let size = 2 + next() % 4;
            let values: Vec<f64> = (0..size).map(|_| pool[next()]).collect();
            let longest = values.iter().map(|&v| one(v, format).len()).max().unwrap_or(0);
            check_eq(widest(values.clone(), format).len(), longest)
                .map_err(|e| format!("{values:?} under {format:?}: {e}"))?;
        }
    }
    Ok(())
}

#[test]
fn the_same_holds_for_f32_around_its_own_limits() -> Result<(), String> {
    let values: Vec<f32> = vec![
        f32::MIN_POSITIVE,
        f32::from_bits(1),
        f32::MAX,
        -f32::MAX,
        9.5e-5,
        -9.9999e30,
        1.0,
        0.0,
        -0.0,
        f32::NAN,
        f32::NEG_INFINITY,
    ];
    for format in every_format() {
        let content = DataNodeContent::new(NodeValues::F32(values.clone()), DataFormat::Float(format));
        let longest = cells(&content).iter().map(String::len).max().unwrap_or(0);
        let mut out = String::new();
        content.widest_cell_string(&mut out);
        check_eq(out.len(), longest).map_err(|e| format!("{format:?}: {e}"))?;
    }
    Ok(())
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
//   NaN payloads and signs
// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -

#[test]
fn two_nans_with_different_bits_are_different_values_though_both_read_nan() -> Result<(), String> {
    let quiet = f64::from_bits(0x7FF8_0000_0000_0000);
    let payload = f64::from_bits(0x7FF8_0000_0000_0001);
    check_eq(quiet.is_nan() && payload.is_nan(), true)?;
    check_eq(NodeValues::F64(vec![quiet]) == NodeValues::F64(vec![payload]), false)?;
    check_eq(NodeValues::F64(vec![payload]) == NodeValues::F64(vec![payload]), true)?;
    check_eq(one(payload, FloatFormat::decimal_places(2)), "NaN".to_owned())?;
    let hex = |v: f64| cells(&DataNodeContent::new(NodeValues::F64(vec![v]), DataFormat::Hexadecimal)).remove(0);
    check_eq(hex(quiet), "7F F8 00 00 00 00 00 00".to_owned())?;
    check_eq(hex(payload), "7F F8 00 00 00 00 00 01".to_owned())
}

#[test]
fn replacing_one_nan_with_another_of_different_bits_is_a_change() -> Result<(), String> {
    let quiet = f64::from_bits(0x7FF8_0000_0000_0000);
    let payload = f64::from_bits(0x7FF8_0000_0000_0001);
    let mut content = DataNodeContent::new(NodeValues::F64(vec![quiet]), DataFormat::Hexadecimal);
    check_eq(content.differs_from(&NodeValues::F64(vec![payload])), true)?;
    let mut written = Vec::new();
    content.try_replace_values(NodeValues::F64(vec![payload]), &mut String::new(), |_, text| {
        written.push(text.to_owned());
        Ok::<(), String>(())
    })?;
    check_eq(written, vec!["7F F8 00 00 00 00 00 01".to_owned()])?;
    // Under a format that reads both as `NaN`, the replacement is still a change, though its text is the same.
    let mut shown = DataNodeContent::new(NodeValues::F64(vec![quiet]), DataFormat::Decimal);
    check_eq(shown.differs_from(&NodeValues::F64(vec![payload])), true)?;
    let mut calls = 0;
    shown.try_replace_values(NodeValues::F64(vec![payload]), &mut String::new(), |_, text| {
        check_eq(text, "NaN").map_err(|e| e.to_string())?;
        calls += 1;
        Ok::<(), String>(())
    })?;
    check_eq(calls, 1)
}

#[test]
fn a_negative_nan_has_no_sign_in_its_text_but_keeps_it_in_its_bits() -> Result<(), String> {
    let negative = f64::from_bits(0xFFF8_0000_0000_0000);
    check_eq(negative.is_nan() && negative.is_sign_negative(), true)?;
    for format in [
        FloatFormat::shortest(),
        FloatFormat::decimal_places(2),
        FloatFormat::significant_digits(3).scientific(),
    ] {
        check_eq(one(negative, format), "NaN".to_owned())?;
    }
    let hex = DataNodeContent::new(NodeValues::F64(vec![negative]), DataFormat::Hexadecimal);
    check_eq(cells(&hex), vec!["FF F8 00 00 00 00 00 00".to_owned()])?;
    let single = DataNodeContent::new(NodeValues::F32(vec![f32::from_bits(0xFFC0_0000)]), DataFormat::Hexadecimal);
    check_eq(cells(&single), vec!["FF C0 00 00".to_owned()])?;
    check_eq(
        NodeValues::F64(vec![negative]) == NodeValues::F64(vec![f64::NAN]),
        negative.to_bits() == f64::NAN.to_bits(),
    )
}

#[test]
fn a_negative_nan_is_no_wider_than_a_positive_one() -> Result<(), String> {
    let negative = f64::from_bits(0xFFF8_0000_0000_0000);
    check_eq(widest(vec![negative, 0.5], FloatFormat::decimal_places(0)), "NaN".to_owned())
}
