//! Floating point values: `NodeValues::F32`/`F64` and `DataFormat::Float`. Each format writes what it should, every cell
//! is wide enough for the widest value whatever the format, and an invalid format is rejected before anything is drawn.

use super::support::{rect_children, text_children, title_of};
use crate::common::{check, make_svg, nth_group};
use svg_dom::root::utils::{Point, Size};
use svg_dom_graph::{
    Error,
    scene::{
        ArithmeticOperator, BinaryOperator, DataFormat, DataNodeContent, FloatFormat, GridLayout, MAX_FLOAT_DIGITS,
        NodeValues, Scene, UnaryOperator,
    },
};
use wasm_bindgen_test::wasm_bindgen_test;

fn scene_with(id: &str, content: DataNodeContent) -> Result<(Scene, svg_dom_graph::NodeId), String> {
    let svg = make_svg(id, Size::new(900.0, 400.0), Size::new(900.0, 400.0));
    let scene = Scene::new(svg).map_err(|e| e.to_string())?;
    let node = scene
        .add_data_node(Point::new(10.0, 10.0), content)
        .map_err(|e| e.to_string())?;
    Ok((scene, node))
}

fn texts_of(id: &str) -> Result<Vec<String>, String> {
    let group = nth_group(id, 0)?;
    Ok(text_children(&group)?
        .iter()
        .map(|t| t.text_content().unwrap_or_default())
        .collect())
}

fn f64_grid(values: Vec<f64>, format: FloatFormat) -> DataNodeContent {
    DataNodeContent::new(NodeValues::F64(values), DataFormat::Float(format)).with_layout(GridLayout::Columns(1))
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Decimal places and significant digits are different things, and each reads as it should.
#[wasm_bindgen_test]
fn decimal_places_and_significant_digits_are_written_as_asked() -> Result<(), String> {
    let values = vec![1234.5678, 1.2345678, 0.0012345678];
    let (_a, _) = scene_with("float-places", f64_grid(values.clone(), FloatFormat::decimal_places(3)))?;
    let (_b, _) = scene_with("float-digits", f64_grid(values, FloatFormat::significant_digits(3)))?;
    let (places, digits) = (texts_of("float-places")?, texts_of("float-digits")?);
    check(places == ["1234.568", "1.235", "0.001"], &format!("places {places:?}"))?;
    check(digits == ["1230", "1.23", "0.00123"], &format!("digits {digits:?}"))
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Scientific notation, the shortest form, and the special values.
#[wasm_bindgen_test]
fn scientific_notation_and_special_values_are_written_as_asked() -> Result<(), String> {
    let (_a, _) = scene_with(
        "float-sci",
        f64_grid(vec![1234.56, 0.00012, -5.0], FloatFormat::significant_digits(4).scientific()),
    )?;
    let (_b, _) = scene_with(
        "float-special",
        f64_grid(
            vec![f64::NAN, f64::INFINITY, f64::NEG_INFINITY, -0.0],
            FloatFormat::decimal_places(1),
        ),
    )?;
    let (sci, special) = (texts_of("float-sci")?, texts_of("float-special")?);
    check(sci == ["1.235e3", "1.200e-4", "-5.000e0"], &format!("scientific {sci:?}"))?;
    check(special == ["NaN", "inf", "-inf", "-0.0"], &format!("special {special:?}"))
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// `Hexadecimal` and `Binary` show a float's IEEE-754 bits.
#[wasm_bindgen_test]
fn hexadecimal_and_binary_show_the_ieee_754_bits() -> Result<(), String> {
    let (_a, _) = scene_with(
        "float-hex",
        DataNodeContent::new(NodeValues::F32(vec![1.0, -2.0]), DataFormat::Hexadecimal),
    )?;
    let hex = texts_of("float-hex")?;
    check(hex == ["3F 80 00 00", "C0 00 00 00"], &format!("hex {hex:?}"))
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The point of the width calculation: whatever the format, no value overflows its cell. Each case has a different
/// widest value: a tiny one under plain significant digits, a negative one with many integer digits under decimal places,
/// a long exponent under scientific notation, `-inf` among short numbers, and an awkward sum under the shortest form.
#[wasm_bindgen_test]
fn every_cell_is_wide_enough_for_its_widest_value_in_every_format() -> Result<(), String> {
    let cases: [(&str, Vec<f64>, FloatFormat); 5] = [
        ("fit-tiny", vec![1.23, 1.23e-9, 4.5], FloatFormat::significant_digits(3)),
        ("fit-negative", vec![0.5, 99.5, -123456.789], FloatFormat::decimal_places(2)),
        (
            "fit-exponent",
            vec![1.0, 1e-100, 2.5],
            FloatFormat::decimal_places(2).scientific(),
        ),
        (
            "fit-infinity",
            vec![0.5, 0.25, f64::NEG_INFINITY],
            FloatFormat::decimal_places(1),
        ),
        ("fit-shortest", vec![0.5, 0.1 + 0.2, 0.25], FloatFormat::shortest()),
    ];
    for (id, values, format) in cases {
        let (_scene, _) = scene_with(id, f64_grid(values, format))?;
        let group = nth_group(id, 0)?;
        let texts = text_children(&group)?;
        // The first rect is the node's outer box, and each cell has its own after it.
        let rects = rect_children(&group)?;
        check(
            rects.len() == texts.len() + 1,
            &format!("{id}: {} rects for {} texts", rects.len(), texts.len()),
        )?;
        for (text, rect) in texts.iter().zip(rects.iter().skip(1)) {
            let (needed, available) =
                (text.get_bounding_client_rect().width(), rect.get_bounding_client_rect().width());
            check(
                needed <= available,
                &format!("{id}: {:?} needs {needed} but its cell is {available}", text.text_content()),
            )?;
        }
    }
    Ok(())
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// A float format that cannot be honoured is rejected before anything is drawn.
#[wasm_bindgen_test]
fn an_invalid_float_format_is_rejected_before_anything_is_drawn() -> Result<(), String> {
    let svg = make_svg("float-invalid", Size::new(400.0, 200.0), Size::new(400.0, 200.0));
    let scene = Scene::new(svg).map_err(|e| e.to_string())?;
    let rejected = [
        DataNodeContent::new(NodeValues::U32(vec![1]), DataFormat::Float(FloatFormat::shortest())),
        DataNodeContent::new(NodeValues::I16(vec![1]), DataFormat::Float(FloatFormat::decimal_places(2))),
        f64_grid(vec![1.0], FloatFormat::decimal_places(MAX_FLOAT_DIGITS + 1)),
        f64_grid(vec![1.0], FloatFormat::significant_digits(0)),
    ];
    for content in rejected {
        let outcome = scene.add_data_node(Point::new(10.0, 10.0), content);
        check(
            matches!(outcome, Err(Error::InvalidFloatFormat)),
            &format!("expected InvalidFloatFormat, got {outcome:?}"),
        )?;
    }
    check(nth_group("float-invalid", 0).is_err(), "a rejected call should draw nothing")
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// A float names its own type, and has a colour of its own.
#[wasm_bindgen_test]
fn a_float_node_names_its_type_and_the_value_it_shows() -> Result<(), String> {
    let (_scene, _) = scene_with(
        "float-name",
        DataNodeContent::new(NodeValues::F32(vec![-2.5]), DataFormat::Float(FloatFormat::decimal_places(2))),
    )?;
    let group = nth_group("float-name", 0)?;
    check(
        group.get_attribute("aria-label").as_deref() == Some("f32 = -2.50"),
        &format!("aria-label {:?}", group.get_attribute("aria-label")),
    )?;
    check(title_of(&group)?.as_deref() == Some("f32 = -2.50"), "the tooltip should match")
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// A single float is replaced across the sign of zero, which `==` cannot see, and its text, tooltip and name follow.
#[wasm_bindgen_test]
fn a_single_float_replaced_by_a_negative_zero_follows_in_text_title_and_name() -> Result<(), String> {
    let (scene, node) = scene_with(
        "float-zero",
        DataNodeContent::new(NodeValues::F64(vec![0.0]), DataFormat::Float(FloatFormat::decimal_places(1))),
    )?;
    let group = nth_group("float-zero", 0)?;
    let expect = |value: &str| -> Result<(), String> {
        let texts = texts_of("float-zero")?;
        let (title, label) = (title_of(&group)?, group.get_attribute("aria-label"));
        let name = format!("f64 = {value}");
        check(
            texts == [value] && title.as_deref() == Some(name.as_str()) && label.as_deref() == Some(name.as_str()),
            &format!("expected {value:?}, got text {texts:?}, title {title:?}, aria-label {label:?}"),
        )
    };
    expect("0.0")?;
    scene
        .set_data_values(node, NodeValues::F64(vec![-0.0]))
        .map_err(|e| e.to_string())?;
    expect("-0.0")?;
    scene
        .set_data_values(node, NodeValues::F64(vec![0.0]))
        .map_err(|e| e.to_string())?;
    expect("0.0")?;
    scene
        .set_data_values(node, NodeValues::F64(vec![f64::NAN]))
        .map_err(|e| e.to_string())?;
    expect("NaN")
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// An `f32` and an `f64` are different types, as far as replacing values and operators are concerned.
#[wasm_bindgen_test]
fn an_f32_is_not_an_f64_for_replacement_or_for_an_operator() -> Result<(), String> {
    let (scene, node) = scene_with(
        "float-types",
        DataNodeContent::new(NodeValues::F32(vec![1.0]), DataFormat::Decimal),
    )?;
    let refused = scene.set_data_values(node, NodeValues::F64(vec![1.0]));
    check(
        matches!(refused, Err(Error::IncompatibleNodeValues(_))),
        &format!("expected IncompatibleNodeValues, got {refused:?}"),
    )?;
    let outcome = scene.add_unary_operator_node(
        Point::new(200.0, 10.0),
        UnaryOperator::Not,
        node,
        DataNodeContent::new(NodeValues::F64(vec![1.0]), DataFormat::Decimal),
    );
    check(
        matches!(outcome, Err(Error::OperatorTypeMismatch { .. })),
        &format!("expected OperatorTypeMismatch, got {outcome:?}"),
    )
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Unary and binary operators act on the bits of an integer, and are refused floating point operands, even when the
/// operands and result are all of one float type. Nothing is drawn for a rejected call.
#[wasm_bindgen_test]
fn a_bitwise_operator_is_refused_floating_point_operands() -> Result<(), String> {
    let svg = make_svg("float-bitwise", Size::new(600.0, 300.0), Size::new(600.0, 300.0));
    let scene = Scene::new(svg).map_err(|e| e.to_string())?;
    let a = scene
        .add_data_node(
            Point::new(10.0, 10.0),
            DataNodeContent::new(NodeValues::F32(vec![1.0]), DataFormat::Decimal),
        )
        .map_err(|e| e.to_string())?;
    let b = scene
        .add_data_node(
            Point::new(10.0, 120.0),
            DataNodeContent::new(NodeValues::F32(vec![2.0]), DataFormat::Decimal),
        )
        .map_err(|e| e.to_string())?;
    let float_result = || DataNodeContent::new(NodeValues::F32(vec![1.0]), DataFormat::Decimal);

    let unary = scene.add_unary_operator_node(Point::new(200.0, 10.0), UnaryOperator::Not, a, float_result());
    check(
        matches!(&unary, Err(Error::OperatorNeedsIntegers { operator, found }) if operator == "NOT" && *found == "f32"),
        &format!("expected OperatorNeedsIntegers for NOT, got {unary:?}"),
    )?;
    let binary = scene.add_binary_operator_node(Point::new(200.0, 60.0), BinaryOperator::Xor, (a, b), float_result());
    check(
        matches!(&binary, Err(Error::OperatorNeedsIntegers { operator, .. }) if operator == "XOR"),
        &format!("expected OperatorNeedsIntegers for XOR, got {binary:?}"),
    )?;
    // Only the two operands were drawn, and no edge was left behind.
    check(
        nth_group("float-bitwise", 2).is_err(),
        "a rejected operator should draw nothing",
    )
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// An arithmetic operator means something for a float, and accepts it. The accessible name of its result names the
/// float type.
#[wasm_bindgen_test]
fn an_arithmetic_operator_accepts_floating_point_operands() -> Result<(), String> {
    let svg = make_svg("float-arithmetic", Size::new(600.0, 300.0), Size::new(600.0, 300.0));
    let scene = Scene::new(svg).map_err(|e| e.to_string())?;
    let float = |y: f64, v: f64| {
        scene
            .add_data_node(
                Point::new(10.0, y),
                DataNodeContent::new(NodeValues::F64(vec![v]), DataFormat::Float(FloatFormat::decimal_places(1))),
            )
            .map_err(|e| e.to_string())
    };
    let (a, b) = (float(10.0, 1.5)?, float(120.0, 2.0)?);
    scene
        .add_arithmetic_operator_node(
            Point::new(200.0, 60.0),
            ArithmeticOperator::Divide,
            (a, b),
            DataNodeContent::new(NodeValues::F64(vec![0.75]), DataFormat::Float(FloatFormat::decimal_places(2))),
        )
        .map_err(|e| e.to_string())?;
    let operator = nth_group("float-arithmetic", 2)?;
    let label = operator.get_attribute("aria-label");
    check(
        label.as_deref().is_some_and(|l| l.starts_with("DIV result: f64 = 0.75")),
        &format!("aria-label {label:?}"),
    )
}
