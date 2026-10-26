//! `NodeValues::Complex32`/`Complex64` and `DataFormat::ComplexNumber`: each `ComplexNumberFormat` written and fitted,
//! the matrix drawn as two rows between brackets and rebuilt in place, what a screen reader is told, replacements, the
//! operators that accept a complex number, and what is refused.

use super::support::{check_fits, elements_matching, rect_children, text_children, texts_of};
use crate::common::{check, make_svg, nth_group};
use svg_dom::root::utils::{Point, Size};
use svg_dom_graph::{
    Error, NodeId,
    complex::{Complex, Complex32},
    scene::{
        AngleUnit, ArithmeticOperator, BinaryOperator, ComplexNumberFormat, ComplexStyle, DataFormat, DataNodeContent,
        FloatFormat, GridLayout, NodeValues, Scene, UnaryOperator,
    },
};
use wasm_bindgen_test::wasm_bindgen_test;

/// `3 - 4i`.
const Z: Complex = Complex::new(3.0, -4.0);

fn complex_format(format: ComplexNumberFormat) -> DataFormat {
    DataFormat::ComplexNumber(format, ComplexStyle::default())
}

fn complex_node(id: &str, values: Vec<Complex>, format: ComplexNumberFormat) -> Result<(Scene, NodeId), String> {
    let svg = make_svg(id, Size::new(900.0, 500.0), Size::new(900.0, 500.0));
    let scene = Scene::new(svg).map_err(|e| e.to_string())?;
    let node = scene
        .add_data_node(
            Point::new(10.0, 10.0),
            DataNodeContent::new(NodeValues::Complex64(values), complex_format(format))
                .with_layout(GridLayout::Columns(1)),
        )
        .map_err(|e| e.to_string())?;
    Ok((scene, node))
}

/// The rows of each `<tspan>` of the first `<text>` of node `id`.
fn rows_of(id: &str) -> Result<Vec<String>, String> {
    let group = nth_group(id, 0)?;
    let text = text_children(&group)?.into_iter().next().ok_or("no <text>")?;
    Ok(elements_matching(&text, "tspan")?
        .iter()
        .map(|span| span.text_content().unwrap_or_default())
        .collect())
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Every form that is one line writes the same number its own way, and its cell is wide enough.
#[wasm_bindgen_test]
fn every_one_line_form_is_written_and_fits() -> Result<(), String> {
    let cases = [
        ("cx-rect", ComplexNumberFormat::Rectangular, "3.000 - 4.000i"),
        ("cx-polar", ComplexNumberFormat::Polar, "5.000 \u{2220} -53.130\u{b0}"),
        (
            "cx-trig",
            ComplexNumberFormat::Trigonometric,
            "5.000 (cos -53.130\u{b0} + i sin -53.130\u{b0})",
        ),
        // The exponent is a raised `<tspan>`, so the text is the base and then the exponent, with no `^(` between.
        ("cx-exp", ComplexNumberFormat::Exponential, "5.000 ei-0.927"),
        ("cx-log", ComplexNumberFormat::LogPolar, "(1.609, -0.927)"),
    ];
    for (id, format, expected) in cases {
        let _scene = complex_node(id, vec![Z], format)?;
        let texts = texts_of(id)?;
        check(texts == [expected], &format!("{id}: {texts:?}"))?;
        check_fits(id)?;
    }
    Ok(())
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// A grid of several numbers has a cell as wide as the widest, and every text fits it.
#[wasm_bindgen_test]
fn a_grid_of_numbers_sizes_every_cell_for_the_widest() -> Result<(), String> {
    let values = vec![Complex::new(1.0, 1.0), Complex::new(-123.5, -4.0), Complex::new(0.5, 0.25)];
    let _scene = complex_node("cx-grid", values, ComplexNumberFormat::Rectangular)?;
    check_fits("cx-grid")?;
    let widths: Vec<f64> = rect_children(&nth_group("cx-grid", 0)?)?
        .iter()
        .skip(1)
        .map(|rect| rect.get_bounding_client_rect().width())
        .collect();
    check(
        widths.windows(2).all(|pair| (pair[0] - pair[1]).abs() < 0.01),
        &format!("cell widths {widths:?}"),
    )
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// A matrix is drawn as two rows, one `<tspan>` each, with a pair of brackets. Its cell is taller than any other and
/// holds both rows.
#[wasm_bindgen_test]
fn a_matrix_is_two_rows_between_brackets_in_a_taller_cell() -> Result<(), String> {
    let (scene, node) = complex_node("cx-matrix", vec![Z, Complex::I], ComplexNumberFormat::Matrix)?;
    let _plain = complex_node("cx-plain", vec![Z, Complex::I], ComplexNumberFormat::Rectangular)?;
    check(
        rows_of("cx-matrix")? == [" 3.000 4.000", "-4.000 3.000"],
        &format!("the rows were {:?}", rows_of("cx-matrix")?),
    )?;
    check_fits("cx-matrix")?;

    let group = nth_group("cx-matrix", 0)?;
    let brackets = elements_matching(&group, "path[aria-hidden='true']")?;
    check(
        brackets.len() == 2,
        &format!("one pair of brackets for each number, found {}", brackets.len()),
    )?;

    let height = |id: &str| -> Result<f64, String> {
        Ok(rect_children(&nth_group(id, 0)?)?[1].get_bounding_client_rect().height())
    };
    let (matrix, plain) = (height("cx-matrix")?, height("cx-plain")?);
    check(
        matrix > plain * 1.4,
        &format!("a matrix cell is {matrix} high and a plain cell {plain}"),
    )?;

    // Both rows lie inside the cell.
    let cell = rect_children(&group)?[1].get_bounding_client_rect();
    let text = text_children(&group)?[0].get_bounding_client_rect();
    check(
        text.top() >= cell.top() && text.bottom() <= cell.bottom(),
        &format!(
            "the rows ({}..{}) should lie inside the cell ({}..{})",
            text.top(),
            text.bottom(),
            cell.top(),
            cell.bottom()
        ),
    )?;
    let _ = (scene, node);
    Ok(())
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Replacing a matrix's values rebuilds its two rows in place. The rows do not pile up, and the node keeps its size.
#[wasm_bindgen_test]
fn a_matrix_is_rewritten_in_place() -> Result<(), String> {
    let (scene, node) = complex_node("cx-matrix-set", vec![Z], ComplexNumberFormat::Matrix)?;
    let before = scene.node_rect(node).map_err(|e| e.to_string())?;

    for value in [Complex::new(1.0, 2.0), Complex::new(-3.0, 4.0), Z] {
        scene
            .set_data_values(node, NodeValues::Complex64(vec![value]))
            .map_err(|e| e.to_string())?;
        let rows = rows_of("cx-matrix-set")?;
        check(rows.len() == 2, &format!("{} rows after setting {value}", rows.len()))?;
        check_fits("cx-matrix-set")?;
    }
    check(
        rows_of("cx-matrix-set")? == [" 3.000 4.000", "-4.000 3.000"],
        "the original numbers are back",
    )?;
    check(
        scene.node_rect(node).map_err(|e| e.to_string())? == before,
        "rewriting the same width of numbers should not resize the node",
    )
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// A matrix is spoken as the one-line text, brackets and all, and not as two disconnected rows.
#[wasm_bindgen_test]
fn a_matrix_is_named_for_assistive_technology_as_one_line() -> Result<(), String> {
    let (_scene, _node) = complex_node("cx-matrix-name", vec![Z, Complex::ONE], ComplexNumberFormat::Matrix)?;
    let group = nth_group("cx-matrix-name", 0)?;
    let name = text_children(&group)?[0].get_attribute("aria-label");
    check(
        name.as_deref() == Some("row 0, column 0: [ 3.000 4.000; -4.000 3.000]"),
        &format!("aria-label {name:?}"),
    )
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// A single number names its type and value, and a replacement updates both.
#[wasm_bindgen_test]
fn a_single_number_names_its_type_and_value() -> Result<(), String> {
    let (scene, node) = complex_node("cx-single", vec![Z], ComplexNumberFormat::Rectangular)?;
    let group = nth_group("cx-single", 0)?;
    check(
        group.get_attribute("aria-label").as_deref() == Some("complex64 = 3.000 - 4.000i"),
        &format!("aria-label {:?}", group.get_attribute("aria-label")),
    )?;
    scene
        .set_data_values(node, NodeValues::Complex64(vec![Complex::new(0.0, 1.0)]))
        .map_err(|e| e.to_string())?;
    check(
        group.get_attribute("aria-label").as_deref() == Some("complex64 = 0.000 + 1.000i"),
        &format!("aria-label {:?}", group.get_attribute("aria-label")),
    )?;
    check_fits("cx-single")
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The angle unit and the digits of the style change what is written.
#[wasm_bindgen_test]
fn the_style_changes_the_angle_unit_and_the_digits() -> Result<(), String> {
    let svg = make_svg("cx-style", Size::new(600.0, 200.0), Size::new(600.0, 200.0));
    let scene = Scene::new(svg).map_err(|e| e.to_string())?;
    let style = ComplexStyle::default()
        .with_angle(AngleUnit::Radians)
        .with_digits(FloatFormat::decimal_places(1));
    scene
        .add_data_node(
            Point::new(10.0, 10.0),
            DataNodeContent::new(
                NodeValues::Complex64(vec![Z]),
                DataFormat::ComplexNumber(ComplexNumberFormat::Polar, style),
            ),
        )
        .map_err(|e| e.to_string())?;
    let texts = texts_of("cx-style")?;
    check(texts == ["5.0 \u{2220} -0.9"], &format!("{texts:?}"))
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Single precision numbers are drawn the same way, and a node cannot be changed from one precision to the other.
#[wasm_bindgen_test]
fn single_precision_is_drawn_and_cannot_be_swapped_for_double() -> Result<(), String> {
    let svg = make_svg("cx-single-precision", Size::new(600.0, 200.0), Size::new(600.0, 200.0));
    let scene = Scene::new(svg).map_err(|e| e.to_string())?;
    let node = scene
        .add_data_node(
            Point::new(10.0, 10.0),
            DataNodeContent::new(
                NodeValues::Complex32(vec![Complex32::new(3.0, -4.0)]),
                complex_format(ComplexNumberFormat::Rectangular),
            ),
        )
        .map_err(|e| e.to_string())?;
    check(texts_of("cx-single-precision")? == ["3.000 - 4.000i"], "complex32 is written")?;
    check(
        nth_group("cx-single-precision", 0)?.get_attribute("aria-label").as_deref()
            == Some("complex32 = 3.000 - 4.000i"),
        "the type is named",
    )?;
    let swapped = scene.set_data_values(node, NodeValues::Complex64(vec![Z]));
    check(
        matches!(swapped, Err(Error::IncompatibleNodeValues(_))),
        &format!("expected IncompatibleNodeValues, got {swapped:?}"),
    )
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Addition, subtraction, multiplication and division accept complex numbers. Everything else names the operator and
/// refuses them, and draws nothing. A matrix cannot be an operator's result.
#[wasm_bindgen_test]
fn only_the_four_arithmetic_operators_accept_complex_numbers() -> Result<(), String> {
    let svg = make_svg("cx-operators", Size::new(900.0, 500.0), Size::new(900.0, 500.0));
    let scene = Scene::new(svg).map_err(|e| e.to_string())?;
    let number = |y: f64, z: Complex| {
        scene
            .add_data_node(
                Point::new(10.0, y),
                DataNodeContent::new(NodeValues::Complex64(vec![z]), complex_format(ComplexNumberFormat::Rectangular)),
            )
            .map_err(|e| e.to_string())
    };
    let (a, b) = (number(10.0, Z)?, number(110.0, Complex::I)?);
    let result = |format| {
        DataNodeContent::new(
            NodeValues::Complex64(vec![Complex::new(4.0, 3.0)]),
            DataFormat::ComplexNumber(format, ComplexStyle::default()),
        )
    };
    let drawn = |i: u32| nth_group("cx-operators", i).is_ok();

    scene
        .add_arithmetic_operator_node(
            Point::new(300.0, 60.0),
            ArithmeticOperator::Multiply,
            (a, b),
            result(ComplexNumberFormat::Polar),
        )
        .map_err(|e| e.to_string())?;
    check(drawn(2) && !drawn(3), "the product should have been drawn")?;

    let modulus = scene.add_arithmetic_operator_node(
        Point::new(300.0, 200.0),
        ArithmeticOperator::Modulus,
        (a, b),
        result(ComplexNumberFormat::Rectangular),
    );
    check(
        matches!(&modulus, Err(Error::OperatorOnComplex { operator }) if operator == "MOD"),
        &format!("expected OperatorOnComplex for MOD, got {modulus:?}"),
    )?;
    let and = scene.add_binary_operator_node(
        Point::new(300.0, 200.0),
        BinaryOperator::And,
        (a, b),
        result(ComplexNumberFormat::Rectangular),
    );
    check(
        matches!(&and, Err(Error::OperatorOnComplex { operator }) if operator == "AND"),
        &format!("expected OperatorOnComplex for AND, got {and:?}"),
    )?;
    let not = scene.add_unary_operator_node(
        Point::new(300.0, 200.0),
        UnaryOperator::Not,
        a,
        result(ComplexNumberFormat::Rectangular),
    );
    check(
        matches!(&not, Err(Error::OperatorOnComplex { .. })),
        &format!("expected OperatorOnComplex for NOT, got {not:?}"),
    )?;
    let matrix = scene.add_arithmetic_operator_node(
        Point::new(300.0, 300.0),
        ArithmeticOperator::Add,
        (a, b),
        result(ComplexNumberFormat::Matrix),
    );
    check(
        matches!(matrix, Err(Error::InvalidComplexFormat)),
        &format!("a matrix result should be refused, got {matrix:?}"),
    )?;
    check(!drawn(3), "a rejected operator should draw nothing")
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// A complex format asks for complex numbers and sensible digits, and a float format does not take complex numbers.
/// Anything else is rejected before anything is drawn.
#[wasm_bindgen_test]
fn what_is_refused_is_refused_before_anything_is_drawn() -> Result<(), String> {
    let svg = make_svg("cx-invalid", Size::new(600.0, 200.0), Size::new(600.0, 200.0));
    let scene = Scene::new(svg).map_err(|e| e.to_string())?;
    let at = Point::new(10.0, 10.0);
    let polar = complex_format(ComplexNumberFormat::Polar);

    for values in [
        NodeValues::F64(vec![1.0]),
        NodeValues::U8(vec![1]),
        NodeValues::Bool(vec![true]),
    ] {
        let outcome = scene.add_data_node(at, DataNodeContent::new(values, polar));
        check(
            matches!(outcome, Err(Error::InvalidComplexFormat)),
            &format!("expected InvalidComplexFormat, got {outcome:?}"),
        )?;
    }
    let too_precise = ComplexStyle::default().with_digits(FloatFormat::decimal_places(200));
    let outcome = scene.add_data_node(
        at,
        DataNodeContent::new(
            NodeValues::Complex64(vec![Z]),
            DataFormat::ComplexNumber(ComplexNumberFormat::Polar, too_precise),
        ),
    );
    check(
        matches!(outcome, Err(Error::InvalidComplexFormat)),
        &format!("too many digits: expected InvalidComplexFormat, got {outcome:?}"),
    )?;
    let float_format = scene.add_data_node(
        at,
        DataNodeContent::new(NodeValues::Complex64(vec![Z]), DataFormat::Float(FloatFormat::shortest())),
    );
    check(
        matches!(float_format, Err(Error::InvalidFloatFormat)),
        &format!("expected InvalidFloatFormat, got {float_format:?}"),
    )?;
    check(nth_group("cx-invalid", 0).is_err(), "a rejected call should draw nothing")
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The formats that are not complex ones still draw complex numbers, as their IEEE-754 bits, or as the shortest
/// rectangular form for decimal.
#[wasm_bindgen_test]
fn the_other_formats_show_the_bits_or_the_shortest_rectangular_form() -> Result<(), String> {
    for (id, format, expected) in [
        ("cx-dec", DataFormat::Decimal, "1 + 2.5i"),
        (
            "cx-hex",
            DataFormat::Hexadecimal,
            "3F F0 00 00 00 00 00 00 40 04 00 00 00 00 00 00",
        ),
    ] {
        let svg = make_svg(id, Size::new(800.0, 200.0), Size::new(800.0, 200.0));
        let scene = Scene::new(svg).map_err(|e| e.to_string())?;
        scene
            .add_data_node(
                Point::new(10.0, 10.0),
                DataNodeContent::new(NodeValues::Complex64(vec![Complex::new(1.0, 2.5)]), format),
            )
            .map_err(|e| e.to_string())?;
        let texts = texts_of(id)?;
        check(texts == [expected], &format!("{id}: {texts:?}"))?;
        check_fits(id)?;
    }
    Ok(())
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The pictures of a node: each hidden `<g>`, and the `d` of each `<path>` in it, in order.
fn pictures_of(id: &str) -> Result<Vec<(web_sys::Element, Vec<String>)>, String> {
    let group = nth_group(id, 0)?;
    let mut out = Vec::new();
    for picture in elements_matching(&group, "g[aria-hidden='true']")? {
        let paths = elements_matching(&picture, "path")?
            .iter()
            .map(|path| path.get_attribute("d").unwrap_or_default())
            .collect();
        out.push((picture, paths));
    }
    Ok(out)
}

/// The numbers in path data, in order.
fn numbers(path: &str) -> Vec<f64> {
    path.split_whitespace().filter_map(|word| word.parse().ok()).collect()
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// A polar cell and a rectangular cell each begin with a small picture of the number, and the text starts clear of it. No
/// other form has one.
#[wasm_bindgen_test]
fn polar_and_rectangular_cells_have_a_picture_and_the_text_clears_it() -> Result<(), String> {
    for (id, format, pictures) in [
        ("cx-pic-polar", ComplexNumberFormat::Polar, 2),
        ("cx-pic-rect", ComplexNumberFormat::Rectangular, 2),
        ("cx-pic-trig", ComplexNumberFormat::Trigonometric, 0),
        ("cx-pic-exp", ComplexNumberFormat::Exponential, 0),
        ("cx-pic-log", ComplexNumberFormat::LogPolar, 0),
        ("cx-pic-matrix", ComplexNumberFormat::Matrix, 0),
    ] {
        let _scene = complex_node(id, vec![Z, Complex::new(-1.0, 2.0)], format)?;
        let found = pictures_of(id)?;
        check(
            found.len() == pictures,
            &format!("{id}: {} pictures, expected {pictures}", found.len()),
        )?;
        check_fits(id)?;

        let texts = text_children(&nth_group(id, 0)?)?;
        for ((picture, paths), text) in found.iter().zip(&texts) {
            check(
                paths.len() == 3,
                &format!("{id}: a picture is the axes and two legs, found {}", paths.len()),
            )?;
            check(!paths[1].is_empty(), &format!("{id}: the first leg should be drawn"))?;
            let (picture, text) = (picture.get_bounding_client_rect(), text.get_bounding_client_rect());
            check(
                text.left() >= picture.right() - 0.5,
                &format!(
                    "{id}: the text starts at {} but the picture ends at {}",
                    text.left(),
                    picture.right()
                ),
            )?;
        }
    }
    Ok(())
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// A polar picture is one arrow, and a rectangular picture is two legs. The legs are along the real axis and then up or
/// down, and the arrow points where the number does.
#[wasm_bindgen_test]
fn a_polar_picture_is_the_hypotenuse_and_a_rectangular_one_the_two_legs() -> Result<(), String> {
    let _a = complex_node("cx-shape-polar", vec![Complex::new(3.0, 4.0)], ComplexNumberFormat::Polar)?;
    let _b = complex_node("cx-shape-rect", vec![Complex::new(3.0, 4.0)], ComplexNumberFormat::Rectangular)?;

    let polar = &pictures_of("cx-shape-polar")?[0].1;
    check(polar[2].is_empty(), "a polar picture has no second path")?;
    let arrow = numbers(&polar[1]);
    // From the middle of the 22 unit box, up and to the right.
    check(
        arrow[0] == 11.0 && arrow[2] > 11.0 && arrow[3] < 11.0,
        &format!("the arrow is {:?}", polar[1]),
    )?;

    let rect = &pictures_of("cx-shape-rect")?[0].1;
    check(
        rect[1].contains('H') && !rect[1].contains('V'),
        &format!("the adjacent leg is horizontal: {}", rect[1]),
    )?;
    check(
        rect[2].contains('V') && !rect[2].contains('H'),
        &format!("the opposite leg is vertical: {}", rect[2]),
    )?;
    let (across, up) = (numbers(&rect[1]), numbers(&rect[2]));
    check(
        across[2] == up[0] && up[1] == 11.0 && up[2] < 11.0,
        &format!("the legs meet at the number: {rect:?}"),
    )
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// A picture follows its number when the number is replaced. And as every picture of a node is drawn to the scale of the
/// largest modulus, a number that is not replaced is redrawn when the largest changes.
#[wasm_bindgen_test]
fn a_picture_follows_its_number_and_the_scale_of_the_node() -> Result<(), String> {
    let (scene, node) = complex_node(
        "cx-pic-follow",
        vec![Complex::new(1.0, 0.0), Complex::new(0.0, 1.0)],
        ComplexNumberFormat::Polar,
    )?;
    let before = pictures_of("cx-pic-follow")?;

    // Only the second number changes, and it stays the largest, so the first picture is untouched.
    scene
        .set_data_values(
            node,
            NodeValues::Complex64(vec![Complex::new(1.0, 0.0), Complex::new(0.0, -1.0)]),
        )
        .map_err(|e| e.to_string())?;
    let after = pictures_of("cx-pic-follow")?;
    check(
        after[0].1 == before[0].1,
        "an unchanged number keeps its picture when the scale is the same",
    )?;
    check(after[1].1 != before[1].1, "a changed number is redrawn")?;
    let tip = numbers(&after[1].1[1]);
    check(tip[3] > 11.0, &format!("-i points down the page: {:?}", after[1].1[1]))?;

    // The second becomes twice as large, so the first is drawn half as long, though it did not change. (A quarter as long
    // would be shorter than the shortest arrow that is drawn, which is longer than that.)
    scene
        .set_data_values(
            node,
            NodeValues::Complex64(vec![Complex::new(1.0, 0.0), Complex::new(0.0, -2.0)]),
        )
        .map_err(|e| e.to_string())?;
    let rescaled = pictures_of("cx-pic-follow")?;
    check(rescaled[0].1 != after[0].1, "the first picture follows the new scale")?;
    let (was, now) = (numbers(&after[0].1[1])[2] - 11.0, numbers(&rescaled[0].1[1])[2] - 11.0);
    check(
        (was - 9.0).abs() < 0.01 && (now - 4.5).abs() < 0.01,
        &format!("the arrow went from {was} to {now}"),
    )?;
    check_fits("cx-pic-follow")
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// A number that cannot be drawn, such as `NaN`, has a picture that draws nothing, and is drawn again when it is replaced.
#[wasm_bindgen_test]
fn a_number_that_cannot_be_drawn_has_a_picture_that_draws_nothing() -> Result<(), String> {
    let (scene, node) = complex_node(
        "cx-pic-nan",
        vec![Complex::new(f64::NAN, 1.0), Complex::new(1.0, 1.0)],
        ComplexNumberFormat::Polar,
    )?;
    let first = pictures_of("cx-pic-nan")?;
    check(first[0].1[1].is_empty(), "NaN has no arrow")?;
    check(!first[1].1[1].is_empty(), "the other number is still drawn")?;

    scene
        .set_data_values(
            node,
            NodeValues::Complex64(vec![Complex::new(1.0, 1.0), Complex::new(1.0, 1.0)]),
        )
        .map_err(|e| e.to_string())?;
    check(!pictures_of("cx-pic-nan")?[0].1[1].is_empty(), "the replaced number is drawn")
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// An exponential cell is the base and then a smaller, raised `<tspan>` holding the exponent: an `i` and the angle. It is
/// rebuilt in place when the number is replaced, and it is still named as the one line.
#[wasm_bindgen_test]
fn the_exponent_is_a_smaller_raised_span_that_starts_with_i() -> Result<(), String> {
    let (scene, node) = complex_node("cx-exponent", vec![Z, Complex::I], ComplexNumberFormat::Exponential)?;
    let spans = |index: usize| -> Result<Vec<web_sys::Element>, String> {
        let text = text_children(&nth_group("cx-exponent", 0)?)?.remove(index);
        elements_matching(&text, "tspan")
    };

    let first = spans(0)?;
    check(first.len() == 2, &format!("{} spans", first.len()))?;
    check(
        first[0].text_content().as_deref() == Some("5.000 e"),
        "the base is the modulus and the e",
    )?;
    check(
        first[1].text_content().as_deref() == Some("i-0.927"),
        "the exponent is an i and the angle",
    )?;
    let dy: f64 = first[1]
        .get_attribute("dy")
        .and_then(|v| v.parse().ok())
        .ok_or("the exponent has no dy")?;
    let size: f64 = first[1]
        .get_attribute("font-size")
        .and_then(|v| v.parse().ok())
        .ok_or("no font-size")?;
    check(dy < 0.0, &format!("the exponent is raised, dy {dy}"))?;
    check(size < 13.0, &format!("the exponent is smaller than the 13 unit base, {size}"))?;
    check(
        first[1].get_bounding_client_rect().bottom() <= first[0].get_bounding_client_rect().bottom() + 0.5,
        "it is not lower than the base",
    )?;
    check_fits("cx-exponent")?;

    scene
        .set_data_values(node, NodeValues::Complex64(vec![Complex::new(1.0, 1.0), Complex::I]))
        .map_err(|e| e.to_string())?;
    let again = spans(0)?;
    check(again.len() == 2, "the exponent does not pile up")?;
    check(
        again[1].text_content().as_deref() == Some("i0.785"),
        "the exponent follows the number",
    )?;
    let name = text_children(&nth_group("cx-exponent", 0)?)?[0].get_attribute("aria-label");
    check(
        name.as_deref() == Some("row 0, column 0: 1.414 e^(i0.785)"),
        &format!("aria-label {name:?}"),
    )
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
fn styled_node(id: &str, format: ComplexNumberFormat, style: ComplexStyle) -> Result<(Scene, NodeId), String> {
    let svg = make_svg(id, Size::new(900.0, 500.0), Size::new(900.0, 500.0));
    let scene = Scene::new(svg).map_err(|e| e.to_string())?;
    let node = scene
        .add_data_node(
            Point::new(10.0, 10.0),
            DataNodeContent::new(
                NodeValues::Complex64(vec![Z, Complex::new(-1.0, 2.0)]),
                DataFormat::ComplexNumber(format, style),
            )
            .with_layout(GridLayout::Columns(1)),
        )
        .map_err(|e| e.to_string())?;
    Ok((scene, node))
}

/// The width of the first picture of node `id`, the height of its first cell, and whether its text clears the picture.
fn picture_and_cell(id: &str) -> Result<(f64, f64, bool), String> {
    let group = nth_group(id, 0)?;
    let picture = pictures_of(id)?.remove(0).0.get_bounding_client_rect();
    let cell = rect_children(&group)?[1].get_bounding_client_rect();
    let text = text_children(&group)?[0].get_bounding_client_rect();
    Ok((picture.width(), cell.height(), text.left() >= picture.right() - 0.5))
}

/// The scale of the first picture of node `id`, as its transform says: how many times its 22 unit design is enlarged.
fn picture_scale(id: &str) -> Result<f64, String> {
    let transform = pictures_of(id)?.remove(0).0.get_attribute("transform").ok_or("no transform")?;
    transform
        .split("scale(")
        .nth(1)
        .and_then(|rest| rest.trim_end_matches(')').parse().ok())
        .ok_or_else(|| format!("no scale in {transform}"))
}

/// The scale of the picture is a multiple of the height of a line of text. A larger one draws a larger picture, makes the
/// cell taller to hold it, and still leaves the text clear. A smaller one leaves the cell the height it was.
#[wasm_bindgen_test]
fn the_icon_scale_sets_the_size_of_the_picture_and_the_cell_grows_to_hold_it() -> Result<(), String> {
    let at = |scale: f64| ComplexStyle::default().with_icon_scale(scale);
    let _a = styled_node("cx-scale-default", ComplexNumberFormat::Polar, ComplexStyle::default())?;
    let _b = styled_node("cx-scale-large", ComplexNumberFormat::Polar, at(3.0))?;
    let _c = styled_node("cx-scale-small", ComplexNumberFormat::Polar, at(0.5))?;
    let _d = styled_node("cx-scale-rect", ComplexNumberFormat::Rectangular, at(3.0))?;

    let (default_width, default_height, default_clear) = picture_and_cell("cx-scale-default")?;
    let (large_width, large_height, large_clear) = picture_and_cell("cx-scale-large")?;
    let (small_width, small_height, small_clear) = picture_and_cell("cx-scale-small")?;
    let (rect_width, _, rect_clear) = picture_and_cell("cx-scale-rect")?;

    // The box is 3 lines of text, against the default 1.2, so it is scaled 2.5 times as much. That is read from the
    // transform the picture was given and not from its bounding box, because a browser may or may not count the width of
    // the lines in a box, and the two answers differ by a few units.
    let (default_scale, large_scale, small_scale, rect_scale) = (
        picture_scale("cx-scale-default")?,
        picture_scale("cx-scale-large")?,
        picture_scale("cx-scale-small")?,
        picture_scale("cx-scale-rect")?,
    );
    check(
        (large_scale / default_scale - 2.5).abs() < 0.01,
        &format!(
            "3.0 against 1.2 should be scaled 2.5 times as much, was {}",
            large_scale / default_scale
        ),
    )?;
    check(
        small_scale < default_scale,
        &format!("0.5 is smaller than the default: {small_scale} against {default_scale}"),
    )?;
    check(
        (rect_scale - large_scale).abs() < 0.001,
        "a rectangular picture is the same size as a polar one",
    )?;
    // Whatever a browser counts, a larger picture is a wider box.
    check(
        large_width > default_width && default_width > small_width && rect_width > default_width,
        &format!("widths {small_width}, {default_width}, {large_width} should grow with the scale"),
    )?;
    check(
        large_height > default_height + 20.0,
        &format!("a large picture makes a taller cell: {large_height} against {default_height}"),
    )?;
    check(
        (small_height - default_height).abs() < 0.01,
        "a small picture leaves the cell as tall as it was",
    )?;
    check(
        default_clear && large_clear && small_clear && rect_clear,
        "the text starts clear of every picture",
    )?;
    for id in ["cx-scale-default", "cx-scale-large", "cx-scale-small", "cx-scale-rect"] {
        check_fits(id)?;
    }

    // The lines are as thin on a large picture as on a small one: the width that is written, times the scale the picture
    // is drawn at, is a little over one unit.
    for id in ["cx-scale-default", "cx-scale-large", "cx-scale-small"] {
        let (picture, _) = pictures_of(id)?.remove(0);
        let transform = picture.get_attribute("transform").ok_or("no transform")?;
        let scale: f64 = transform
            .split("scale(")
            .nth(1)
            .and_then(|rest| rest.trim_end_matches(')').parse().ok())
            .ok_or_else(|| format!("no scale in {transform}"))?;
        let paths = elements_matching(&picture, "path")?;
        for (path, expected) in paths.iter().zip([0.6, 1.1, 1.1]) {
            let width: f64 = path
                .get_attribute("stroke-width")
                .and_then(|w| w.parse().ok())
                .ok_or("no stroke-width")?;
            check(
                (width * scale - expected).abs() < 0.01,
                &format!(
                    "{id}: a line written {width} at scale {scale} is {} thick, expected {expected}",
                    width * scale
                ),
            )?;
        }
    }
    Ok(())
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// A scale outside a quarter to eight lines of text, and one that is not a number, is rejected before anything is drawn.
#[wasm_bindgen_test]
fn an_icon_scale_out_of_range_is_rejected_before_anything_is_drawn() -> Result<(), String> {
    let svg = make_svg("cx-scale-invalid", Size::new(400.0, 200.0), Size::new(400.0, 200.0));
    let scene = Scene::new(svg).map_err(|e| e.to_string())?;
    for scale in [0.0, 0.1, 8.5, -1.0, f64::NAN, f64::INFINITY] {
        let outcome = scene.add_data_node(
            Point::new(10.0, 10.0),
            DataNodeContent::new(
                NodeValues::Complex64(vec![Z]),
                DataFormat::ComplexNumber(ComplexNumberFormat::Polar, ComplexStyle::default().with_icon_scale(scale)),
            ),
        );
        check(
            matches!(outcome, Err(Error::InvalidComplexFormat)),
            &format!("scale {scale}: expected InvalidComplexFormat, got {outcome:?}"),
        )?;
    }
    check(nth_group("cx-scale-invalid", 0).is_err(), "a rejected call should draw nothing")
}
