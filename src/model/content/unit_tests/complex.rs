//! `NodeValues::Complex32`/`Complex64`: every `ComplexNumberFormat`, both angle units, the digits, special values, the
//! matrix text, the IEEE-754 bits, and the width of a cell.

use super::super::*;
use super::support::{cells, check_eq};
use crate::complex::{Complex, Complex32};
use crate::test_support::check;

fn style() -> ComplexStyle {
    ComplexStyle::default()
}

fn content(values: Vec<Complex>, format: ComplexNumberFormat, style: ComplexStyle) -> DataNodeContent {
    DataNodeContent::new(NodeValues::Complex64(values), DataFormat::ComplexNumber(format, style))
}

/// The text of one number under `format` and `style`.
fn one(value: Complex, format: ComplexNumberFormat, style: ComplexStyle) -> String {
    cells(&content(vec![value], format, style)).remove(0)
}

fn widest(content: &DataNodeContent) -> String {
    let mut out = String::new();
    content.widest_cell_string(&mut out);
    out
}

/// `3 - 4i`: a modulus of 5 and an angle of about -53.13 degrees, so every form has something to show.
const Z: Complex = Complex::new(3.0, -4.0);

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
//   The six forms
// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -

#[test]
fn every_form_writes_the_same_number_its_own_way() -> Result<(), String> {
    for (format, expected) in [
        (ComplexNumberFormat::Rectangular, "3.000 - 4.000i"),
        (ComplexNumberFormat::Polar, "5.000 \u{2220} -53.130\u{b0}"),
        (
            ComplexNumberFormat::Trigonometric,
            "5.000 (cos -53.130\u{b0} + i sin -53.130\u{b0})",
        ),
        (ComplexNumberFormat::Exponential, "5.000 e^(i-0.927)"),
        (ComplexNumberFormat::LogPolar, "(1.609, -0.927)"),
        (ComplexNumberFormat::Matrix, "[ 3.000 4.000; -4.000 3.000]"),
    ] {
        check_eq(one(Z, format, style()), expected.to_owned())?;
    }
    check_eq(ComplexNumberFormat::default(), ComplexNumberFormat::Rectangular)
}

#[test]
fn a_positive_imaginary_part_is_added_and_a_negative_one_subtracted() -> Result<(), String> {
    check_eq(
        one(Complex::new(-1.5, 2.0), ComplexNumberFormat::Rectangular, style()),
        "-1.500 + 2.000i".to_owned(),
    )?;
    check_eq(
        one(Complex::new(-1.5, -2.0), ComplexNumberFormat::Rectangular, style()),
        "-1.500 - 2.000i".to_owned(),
    )
}

#[test]
fn radians_replace_degrees_in_the_polar_and_trigonometric_forms_only() -> Result<(), String> {
    let radians = style().with_angle(AngleUnit::Radians);

    check_eq(one(Z, ComplexNumberFormat::Polar, radians), "5.000 \u{2220} -0.927".to_owned())?;
    check_eq(
        one(Z, ComplexNumberFormat::Trigonometric, radians),
        "5.000 (cos -0.927 + i sin -0.927)".to_owned(),
    )?;
    // An exponent and a logarithm's angle are radians whatever the style says.
    for format in [ComplexNumberFormat::Exponential, ComplexNumberFormat::LogPolar] {
        check_eq(one(Z, format, radians), one(Z, format, style()))?;
    }
    check_eq(AngleUnit::default(), AngleUnit::Degrees)
}

#[test]
fn the_digits_of_the_style_choose_how_every_number_is_written() -> Result<(), String> {
    let shortest = style().with_digits(FloatFormat::shortest());
    check_eq(
        one(Complex::new(0.5, 0.25), ComplexNumberFormat::Rectangular, shortest),
        "0.5 + 0.25i".to_owned(),
    )?;
    let one_place = style().with_digits(FloatFormat::decimal_places(1));
    check_eq(
        one(Z, ComplexNumberFormat::Polar, one_place),
        "5.0 \u{2220} -53.1\u{b0}".to_owned(),
    )?;
    let scientific = style().with_digits(FloatFormat::significant_digits(2).scientific());
    check_eq(
        one(Complex::new(1500.0, 0.0), ComplexNumberFormat::Rectangular, scientific),
        "1.5e3 + 0.0e0i".to_owned(),
    )
}

#[test]
fn the_default_style_is_degrees_and_three_decimal_places() -> Result<(), String> {
    check_eq(ComplexStyle::default().angle, AngleUnit::Degrees)?;
    check_eq(ComplexStyle::default().digits, FloatFormat::decimal_places(3))
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
//   Angles
// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -

#[test]
fn the_angle_runs_from_minus_pi_to_pi() -> Result<(), String> {
    let polar = |z: Complex| one(z, ComplexNumberFormat::Polar, style());

    check_eq(polar(Complex::new(1.0, 0.0)), "1.000 \u{2220} 0.000\u{b0}".to_owned())?;
    check_eq(polar(Complex::new(0.0, 1.0)), "1.000 \u{2220} 90.000\u{b0}".to_owned())?;
    check_eq(polar(Complex::new(-1.0, 0.0)), "1.000 \u{2220} 180.000\u{b0}".to_owned())?;
    check_eq(polar(Complex::new(0.0, -1.0)), "1.000 \u{2220} -90.000\u{b0}".to_owned())?;
    // A negative zero imaginary part is just below the negative real axis.
    check_eq(polar(Complex::new(-1.0, -0.0)), "1.000 \u{2220} -180.000\u{b0}".to_owned())
}

#[test]
fn zero_has_an_angle_of_zero_and_a_log_modulus_of_minus_infinity() -> Result<(), String> {
    check_eq(
        one(Complex::ZERO, ComplexNumberFormat::Polar, style()),
        "0.000 \u{2220} 0.000\u{b0}".to_owned(),
    )?;
    check_eq(
        one(Complex::ZERO, ComplexNumberFormat::LogPolar, style()),
        "(-inf, 0.000)".to_owned(),
    )
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
//   Special values
// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -

#[test]
fn nan_and_the_infinities_are_written_where_the_part_that_is_one_goes() -> Result<(), String> {
    check_eq(
        one(Complex::new(f64::NAN, 1.0), ComplexNumberFormat::Rectangular, style()),
        "NaN + 1.000i".to_owned(),
    )?;
    // A `NaN` carries a sign bit that means nothing, so it is never written as a subtraction.
    check_eq(
        one(Complex::new(1.0, -f64::NAN), ComplexNumberFormat::Rectangular, style()),
        "1.000 + NaNi".to_owned(),
    )?;
    check_eq(
        one(Complex::new(f64::INFINITY, 0.0), ComplexNumberFormat::Polar, style()),
        "inf \u{2220} 0.000\u{b0}".to_owned(),
    )?;
    check_eq(
        one(Complex::new(0.0, f64::NEG_INFINITY), ComplexNumberFormat::Rectangular, style()),
        "0.000 - infi".to_owned(),
    )
}

#[test]
fn a_negative_zero_imaginary_part_keeps_its_sign() -> Result<(), String> {
    check_eq(
        one(Complex::new(1.0, -0.0), ComplexNumberFormat::Rectangular, style()),
        "1.000 - 0.000i".to_owned(),
    )
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
//   The matrix
// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -

#[test]
fn a_matrix_is_two_rows_of_the_same_length_between_brackets() -> Result<(), String> {
    let text = one(Z, ComplexNumberFormat::Matrix, style());
    let (first, second) = matrix_rows(&text).ok_or("a matrix text splits into two rows")?;

    check_eq(first, " 3.000 4.000")?;
    check_eq(second, "-4.000 3.000")?;
    check_eq(first.len(), second.len())
}

#[test]
fn a_real_number_has_no_off_diagonal_and_never_a_negative_zero_that_was_not_there() -> Result<(), String> {
    check_eq(
        one(Complex::new(2.0, 0.0), ComplexNumberFormat::Matrix, style()),
        "[2.000 0.000; 0.000 2.000]".to_owned(),
    )
}

#[test]
fn a_matrix_multiplies_like_the_number_it_stands_for() -> Result<(), String> {
    // `i` is a quarter turn: the matrix [0 -1; 1 0].
    check_eq(
        one(Complex::I, ComplexNumberFormat::Matrix, style()),
        "[0.000 -1.000; 1.000  0.000]".to_owned(),
    )
}

#[test]
fn matrix_rows_refuses_text_that_is_not_a_matrix() -> Result<(), String> {
    check_eq(matrix_rows("3.000 - 4.000i"), None)?;
    check_eq(matrix_rows("[1 2 3]"), None)?;
    check_eq(matrix_rows("[1; 2]"), Some(("1", "2")))
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
//   Single precision, and the other formats
// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -

#[test]
fn single_precision_numbers_are_written_the_same_way() -> Result<(), String> {
    let single = |format| {
        let values = NodeValues::Complex32(vec![Complex32::new(3.0, -4.0)]);
        cells(&DataNodeContent::new(values, DataFormat::ComplexNumber(format, style()))).remove(0)
    };
    check_eq(single(ComplexNumberFormat::Rectangular), "3.000 - 4.000i".to_owned())?;
    check_eq(single(ComplexNumberFormat::Polar), "5.000 \u{2220} -53.130\u{b0}".to_owned())?;
    check_eq(single(ComplexNumberFormat::Matrix), "[ 3.000 4.000; -4.000 3.000]".to_owned())
}

#[test]
fn decimal_is_the_rectangular_form_with_the_shortest_digits() -> Result<(), String> {
    let decimal = DataNodeContent::new(NodeValues::Complex64(vec![Complex::new(1.0, 2.5)]), DataFormat::Decimal);
    check_eq(cells(&decimal), vec!["1 + 2.5i".to_owned()])
}

#[test]
fn hexadecimal_shows_the_bits_of_the_real_part_and_then_of_the_imaginary_part() -> Result<(), String> {
    let hex = |order| {
        let values = NodeValues::Complex64(vec![Complex::new(1.0, 2.0)]);
        cells(&DataNodeContent::new(values, DataFormat::Hexadecimal).with_byte_order(order)).remove(0)
    };
    check_eq(
        hex(ByteOrder::BigEndian),
        "3F F0 00 00 00 00 00 00 40 00 00 00 00 00 00 00".to_owned(),
    )?;
    // Each part is reversed. The real part still comes first, as it does in memory.
    check_eq(
        hex(ByteOrder::LittleEndian),
        "00 00 00 00 00 00 F0 3F 00 00 00 00 00 00 00 40".to_owned(),
    )?;

    let single = NodeValues::Complex32(vec![Complex32::new(1.0, 2.0)]);
    check_eq(
        cells(&DataNodeContent::new(single, DataFormat::Hexadecimal)),
        vec!["3F 80 00 00 40 00 00 00".to_owned()],
    )
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
//   Width
// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -

#[test]
fn the_widest_cell_is_the_number_with_the_most_characters() -> Result<(), String> {
    let values = vec![Complex::new(1.0, 1.0), Complex::new(-123.5, -4.0), Complex::new(2.0, 2.0)];
    let rectangular = content(values.clone(), ComplexNumberFormat::Rectangular, style());

    check_eq(widest(&rectangular), "-123.500 - 4.000i".to_owned())?;
    check_eq(widest(&content(Vec::new(), ComplexNumberFormat::Polar, style())), String::new())
}

#[test]
fn the_widest_cell_in_a_fixed_width_format_is_the_first() -> Result<(), String> {
    let values = NodeValues::Complex64(vec![Complex::new(1.0, 2.0), Complex::new(-1e9, 5.0)]);
    let hex = DataNodeContent::new(values, DataFormat::Hexadecimal);

    check_eq(widest(&hex), "3F F0 00 00 00 00 00 00 40 00 00 00 00 00 00 00".to_owned())
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
//   What is valid
// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -

#[test]
fn a_complex_format_is_valid_only_for_complex_values() -> Result<(), String> {
    let format = DataFormat::ComplexNumber(ComplexNumberFormat::Polar, style());
    check_eq(
        content(vec![Z], ComplexNumberFormat::Polar, style()).complex_format_is_valid(),
        true,
    )?;
    let single = NodeValues::Complex32(vec![Complex32::new(1.0, 0.0)]);
    check_eq(DataNodeContent::new(single, format).complex_format_is_valid(), true)?;
    for values in [
        NodeValues::F64(vec![1.0]),
        NodeValues::U8(vec![1]),
        NodeValues::Bool(vec![true]),
    ] {
        check_eq(DataNodeContent::new(values, format).complex_format_is_valid(), false)?;
    }
    check_eq(
        DataNodeContent::new(NodeValues::F64(vec![1.0]), DataFormat::Decimal).complex_format_is_valid(),
        true,
    )
}

#[test]
fn a_style_that_asks_for_too_many_digits_is_invalid() -> Result<(), String> {
    let too_many = style().with_digits(FloatFormat::decimal_places(MAX_FLOAT_DIGITS + 1));
    check_eq(
        content(vec![Z], ComplexNumberFormat::Polar, too_many).complex_format_is_valid(),
        false,
    )?;
    let no_digits = style().with_digits(FloatFormat::significant_digits(0));
    check_eq(
        content(vec![Z], ComplexNumberFormat::Polar, no_digits).complex_format_is_valid(),
        false,
    )?;
    let most = style().with_digits(FloatFormat::decimal_places(MAX_FLOAT_DIGITS));
    check_eq(
        content(vec![Z], ComplexNumberFormat::Polar, most).complex_format_is_valid(),
        true,
    )
}

#[test]
fn a_float_format_is_not_valid_for_complex_values() -> Result<(), String> {
    let complex = DataNodeContent::new(NodeValues::Complex64(vec![Z]), DataFormat::Float(FloatFormat::shortest()));
    check_eq(complex.float_format_is_valid(), false)
}

#[test]
fn only_a_matrix_is_two_rows_high_and_keeps_its_spaces() -> Result<(), String> {
    check(
        content(vec![Z], ComplexNumberFormat::Matrix, style()).is_matrix(),
        "a matrix is a matrix",
    )?;
    check(
        content(vec![Z], ComplexNumberFormat::Matrix, style()).preserves_spaces(),
        "a matrix keeps the spaces that line its columns up",
    )?;
    for format in [
        ComplexNumberFormat::Rectangular,
        ComplexNumberFormat::Polar,
        ComplexNumberFormat::Trigonometric,
        ComplexNumberFormat::Exponential,
        ComplexNumberFormat::LogPolar,
    ] {
        check(
            !content(vec![Z], format, style()).is_matrix(),
            "only the matrix form is a matrix",
        )?;
    }
    Ok(())
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
//   Equality, type and colour
// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -

#[test]
fn equality_is_by_bits_in_both_parts() -> Result<(), String> {
    let values = |z| NodeValues::Complex64(vec![z]);

    check_eq(values(Complex::new(0.0, 1.0)) == values(Complex::new(-0.0, 1.0)), false)?;
    check_eq(values(Complex::new(0.0, 0.0)) == values(Complex::new(0.0, -0.0)), false)?;
    check_eq(values(Complex::new(f64::NAN, 1.0)) == values(Complex::new(f64::NAN, 1.0)), true)?;
    check_eq(values(Z) == NodeValues::Complex32(vec![Complex32::new(3.0, -4.0)]), false)
}

#[test]
fn the_type_names_and_colours_say_what_the_values_are() -> Result<(), String> {
    let single = NodeValues::Complex32(vec![Complex32::ZERO]);
    let double = NodeValues::Complex64(vec![Complex::ZERO]);

    check_eq(single.type_name(), "complex32")?;
    check_eq(double.type_name(), "complex64")?;
    check_eq(single.type_colour() == double.type_colour(), false)?;
    check_eq(single.type_colour() == NodeValues::F32(vec![1.0]).type_colour(), false)
}

#[test]
fn a_replacement_must_be_the_same_kind() -> Result<(), String> {
    let double = NodeValues::Complex64(vec![Z]);
    check_eq(double.same_kind(&NodeValues::Complex64(vec![Complex::ONE, Complex::I])), true)?;
    check_eq(double.same_kind(&NodeValues::Complex32(vec![Complex32::ONE])), false)?;
    check_eq(double.same_kind(&NodeValues::F64(vec![1.0])), false)
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
//   The exponent
// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -

#[test]
fn the_exponent_is_an_i_and_then_the_angle() -> Result<(), String> {
    let exponential = |z| one(z, ComplexNumberFormat::Exponential, style());

    check_eq(exponential(Complex::new(3.0, 4.0)), "5.000 e^(i0.927)".to_owned())?;
    check_eq(exponential(Complex::new(1.0, 0.0)), "1.000 e^(i0.000)".to_owned())?;
    check_eq(exponential(Complex::new(-1.0, 0.0)), "1.000 e^(i3.142)".to_owned())
}

#[test]
fn exponent_parts_splits_the_base_from_the_raised_exponent() -> Result<(), String> {
    check_eq(exponent_parts("5.000 e^(i0.927)"), Some(("5.000 e", "i0.927")))?;
    check_eq(exponent_parts("1.5e3 e^(i-0.1e0)"), Some(("1.5e3 e", "i-0.1e0")))?;
    check_eq(exponent_parts("3.000 + 4.000i"), None)?;
    check_eq(exponent_parts("5.000 e^(i0.927"), None)
}

#[test]
fn only_the_matrix_and_the_exponential_forms_build_their_text_from_parts() -> Result<(), String> {
    let style_of = |format| content(vec![Z], format, style()).cell_text();

    check_eq(style_of(ComplexNumberFormat::Matrix), CellText::Matrix)?;
    check_eq(style_of(ComplexNumberFormat::Exponential), CellText::Exponent)?;
    for format in [
        ComplexNumberFormat::Rectangular,
        ComplexNumberFormat::Polar,
        ComplexNumberFormat::Trigonometric,
        ComplexNumberFormat::LogPolar,
    ] {
        check_eq(style_of(format), CellText::Plain)?;
    }
    // A text cell that happens to look like an exponent is not drawn as one.
    let text = DataNodeContent::new(NodeValues::Text(vec!["5 e^(i1)".to_owned()]), DataFormat::Text);
    check_eq(text.cell_text(), CellText::Plain)
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
//   The pictures
// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -

#[test]
fn only_polar_and_rectangular_cells_have_a_picture() -> Result<(), String> {
    let kind_of = |format| content(vec![Z], format, style()).icon_kind();

    check_eq(kind_of(ComplexNumberFormat::Polar), Some(IconKind::Polar))?;
    check_eq(kind_of(ComplexNumberFormat::Rectangular), Some(IconKind::Rectangular))?;
    for format in [
        ComplexNumberFormat::Trigonometric,
        ComplexNumberFormat::Exponential,
        ComplexNumberFormat::LogPolar,
        ComplexNumberFormat::Matrix,
    ] {
        check_eq(kind_of(format), None)?;
    }
    check_eq(
        DataNodeContent::new(NodeValues::Complex64(vec![Z]), DataFormat::Decimal).icon_kind(),
        None,
    )
}

#[test]
fn a_node_scales_every_picture_by_its_largest_modulus() -> Result<(), String> {
    let values = vec![Complex::new(3.0, 4.0), Complex::new(0.0, 1.0), Complex::new(f64::NAN, 0.0)];
    let node = content(values, ComplexNumberFormat::Polar, style());

    check_eq(node.max_modulus(), 5.0)?;
    check_eq(node.complex_at(1), Some((0.0, 1.0)))?;
    check_eq(node.complex_at(3), None)?;
    check_eq(content(Vec::new(), ComplexNumberFormat::Polar, style()).max_modulus(), 0.0)?;
    let single = DataNodeContent::new(
        NodeValues::Complex32(vec![Complex32::new(6.0, 8.0)]),
        DataFormat::ComplexNumber(ComplexNumberFormat::Polar, style()),
    );
    check_eq(single.max_modulus(), 10.0)
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
//   The scale of a picture
// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -

#[test]
fn a_picture_is_a_little_taller_than_a_line_of_text_unless_the_style_says_otherwise() -> Result<(), String> {
    check_eq(ComplexStyle::default().icon_scale(), ComplexStyle::DEFAULT_ICON_SCALE)?;
    check_eq(ComplexStyle::DEFAULT_ICON_SCALE, 1.2)?;
    check_eq(ComplexStyle::default().with_icon_scale(2.0).icon_scale(), 2.0)?;
    // Setting one scale leaves the angle and the digits as they were.
    let style = ComplexStyle::default().with_angle(AngleUnit::Radians).with_icon_scale(3.0);
    check_eq(style.angle, AngleUnit::Radians)?;
    check_eq(style.digits, ComplexStyle::default().digits)
}

#[test]
fn a_scale_is_kept_to_hundredths() -> Result<(), String> {
    check_eq(ComplexStyle::default().with_icon_scale(1.234).icon_scale(), 1.23)?;
    check_eq(ComplexStyle::default().with_icon_scale(1.236).icon_scale(), 1.24)?;
    check_eq(ComplexStyle::default().with_icon_scale(0.5).icon_scale(), 0.5)
}

#[test]
fn a_scale_is_valid_from_a_quarter_to_eight_lines_of_text() -> Result<(), String> {
    let valid = |scale: f64| {
        content(vec![Z], ComplexNumberFormat::Polar, style().with_icon_scale(scale)).complex_format_is_valid()
    };

    check_eq(valid(ComplexStyle::MIN_ICON_SCALE), true)?;
    check_eq(valid(ComplexStyle::MAX_ICON_SCALE), true)?;
    check_eq(valid(1.0), true)?;
    check_eq(valid(0.24), false)?;
    check_eq(valid(8.01), false)?;
    check_eq(valid(0.0), false)?;
    check_eq(valid(-1.0), false)
}

#[test]
fn a_scale_that_is_not_a_number_is_invalid_and_never_panics() -> Result<(), String> {
    for scale in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY, f64::MAX, f64::MIN] {
        let node = content(vec![Z], ComplexNumberFormat::Polar, style().with_icon_scale(scale));
        check_eq(node.complex_format_is_valid(), false)?;
    }
    Ok(())
}

#[test]
fn the_content_reports_the_scale_of_its_style_and_the_default_for_any_other_format() -> Result<(), String> {
    let scaled = content(vec![Z], ComplexNumberFormat::Rectangular, style().with_icon_scale(2.5));
    check_eq(scaled.icon_scale(), 2.5)?;
    check_eq(
        DataNodeContent::new(NodeValues::U8(vec![1]), DataFormat::Decimal).icon_scale(),
        1.2,
    )
}
