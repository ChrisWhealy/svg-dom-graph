//! Signed values: `NodeValues::I8`/`I16`/`I32`/`I64` show a sign only when the value is negative, name their own type,
//! and are a type of their own as far as replacing values and operator operands are concerned.

use super::support::{rect_children, text_children, title_of};
use crate::common::{check, make_svg, nth_group};
use svg_dom::root::utils::{Point, Size};
use svg_dom_graph::{
    Error,
    scene::{DataFormat, DataNodeContent, NodeValues, Scene, UnaryOperator},
};
use wasm_bindgen_test::wasm_bindgen_test;

fn scene_with(id: &str, content: DataNodeContent) -> Result<(Scene, svg_dom_graph::NodeId), String> {
    let svg = make_svg(id, Size::new(400.0, 260.0), Size::new(400.0, 260.0));
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

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// A negative value shows a minus sign, and a positive value or zero shows no sign at all, not even `+`.
#[wasm_bindgen_test]
fn a_sign_is_shown_only_for_a_negative_value() -> Result<(), String> {
    let (_scene, _) = scene_with(
        "signed-decimal",
        DataNodeContent::new(NodeValues::I16(vec![-12, 0, 345]), DataFormat::Decimal),
    )?;
    let texts = texts_of("signed-decimal")?;
    check(texts == ["-12", "0", "345"], &format!("got {texts:?}"))
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Hexadecimal and binary show the value's two's-complement representation, in the selected byte order.
#[wasm_bindgen_test]
fn hexadecimal_and_binary_show_the_twos_complement_of_a_negative_value() -> Result<(), String> {
    let (_a, _) = scene_with(
        "signed-hex",
        DataNodeContent::new(NodeValues::I32(vec![-2]), DataFormat::Hexadecimal),
    )?;
    let (_b, _) = scene_with(
        "signed-bin",
        DataNodeContent::new(NodeValues::I8(vec![-128]), DataFormat::Binary),
    )?;
    let (hex, bin) = (texts_of("signed-hex")?, texts_of("signed-bin")?);
    check(hex == ["FF FF FF FE"], &format!("hex {hex:?}"))?;
    check(bin == ["1000 0000"], &format!("bin {bin:?}"))
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// A signed width names its own type as text, and has a colour of its own that differs from its unsigned equivalent's.
#[wasm_bindgen_test]
fn a_signed_node_names_its_type_and_has_a_colour_of_its_own() -> Result<(), String> {
    let (_a, _) = scene_with(
        "signed-name",
        DataNodeContent::new(NodeValues::I64(vec![-5]), DataFormat::Decimal),
    )?;
    let (_b, _) = scene_with(
        "unsigned-name",
        DataNodeContent::new(NodeValues::U64(vec![5]), DataFormat::Decimal),
    )?;
    let signed = nth_group("signed-name", 0)?;
    let unsigned = nth_group("unsigned-name", 0)?;
    check(
        signed.get_attribute("aria-label").as_deref() == Some("i64 = -5"),
        &format!("aria-label {:?}", signed.get_attribute("aria-label")),
    )?;
    check(
        title_of(&signed)?.as_deref() == Some("i64 = -5"),
        "the tooltip should match the aria-label",
    )?;
    let fill = |g: &web_sys::Element| -> Result<Option<String>, String> {
        Ok(rect_children(g)?.first().and_then(|r| r.get_attribute("fill")))
    };
    check(fill(&signed)? != fill(&unsigned)?, "i64 and u64 should not share a colour")
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The cells of a signed node are all as wide as the widest value needs, and a negative value is one character wider
/// than its magnitude.
#[wasm_bindgen_test]
fn every_cell_is_wide_enough_for_the_minus_sign() -> Result<(), String> {
    let (_a, _) = scene_with(
        "signed-width-neg",
        DataNodeContent::new(NodeValues::I8(vec![-100, 5]), DataFormat::Decimal),
    )?;
    let (_b, _) = scene_with(
        "signed-width-pos",
        DataNodeContent::new(NodeValues::I8(vec![100, 5]), DataFormat::Decimal),
    )?;
    let width = |id: &str| -> Result<f64, String> {
        let group = nth_group(id, 0)?;
        Ok(rect_children(&group)?
            .first()
            .and_then(|r| r.get_attribute("width"))
            .and_then(|w| w.parse().ok())
            .ok_or("no outer rect width")?)
    };
    let (neg, pos) = (width("signed-width-neg")?, width("signed-width-pos")?);
    check(neg > pos, &format!("-100 needs more room than 100: {neg} against {pos}"))
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// A signed node's values can be replaced by values of the same signed width, but not by the unsigned equivalent.
#[wasm_bindgen_test]
fn replacing_values_needs_the_same_signed_width() -> Result<(), String> {
    let (scene, node) = scene_with(
        "signed-replace",
        DataNodeContent::new(NodeValues::I16(vec![1, 2]), DataFormat::Decimal),
    )?;
    scene
        .set_data_values(node, NodeValues::I16(vec![-1, 2]))
        .map_err(|e| e.to_string())?;
    let texts = texts_of("signed-replace")?;
    check(texts == ["-1", "2"], &format!("got {texts:?}"))?;
    let refused = scene.set_data_values(node, NodeValues::U16(vec![1, 2]));
    check(
        matches!(refused, Err(Error::IncompatibleNodeValues(_))),
        &format!("expected IncompatibleNodeValues, got {refused:?}"),
    )
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// An operator's operand and result must share one type, and a signed type and its unsigned equivalent are two types.
#[wasm_bindgen_test]
fn an_operator_rejects_an_operand_and_result_of_signed_and_unsigned_widths() -> Result<(), String> {
    let (scene, operand) = scene_with(
        "signed-operator",
        DataNodeContent::new(NodeValues::I8(vec![-1]), DataFormat::Decimal),
    )?;
    let outcome = scene.add_unary_operator_node(
        Point::new(200.0, 10.0),
        UnaryOperator::Not,
        operand,
        DataNodeContent::new(NodeValues::U8(vec![0]), DataFormat::Decimal),
    );
    check(
        matches!(outcome, Err(Error::OperatorTypeMismatch { .. })),
        &format!("expected OperatorTypeMismatch, got {outcome:?}"),
    )?;
    let matched = scene.add_unary_operator_node(
        Point::new(200.0, 10.0),
        UnaryOperator::Not,
        operand,
        DataNodeContent::new(NodeValues::I8(vec![0]), DataFormat::Decimal),
    );
    check(
        matched.is_ok(),
        &format!("a matching signed result should be accepted: {matched:?}"),
    )
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// A single signed value is replaced across zero, from positive to negative and back, with a new value that formats to
/// the same number of characters. The rendered text, the tooltip and the accessible name must each follow every
/// replacement, so the sign is written and then removed again.
#[wasm_bindgen_test]
fn a_single_value_replaced_across_zero_follows_its_sign_in_text_title_and_name() -> Result<(), String> {
    let (scene, node) = scene_with(
        "signed-across-zero",
        DataNodeContent::new(NodeValues::I16(vec![123]), DataFormat::Decimal),
    )?;
    let group = nth_group("signed-across-zero", 0)?;
    let state = || -> Result<(Vec<String>, Option<String>, Option<String>), String> {
        Ok((
            texts_of("signed-across-zero")?,
            title_of(&group)?,
            group.get_attribute("aria-label"),
        ))
    };
    let expect = |value: &str| -> Result<(), String> {
        let (texts, title, label) = state()?;
        let name = format!("i16 = {value}");
        check(
            texts == [value] && title.as_deref() == Some(name.as_str()) && label.as_deref() == Some(name.as_str()),
            &format!("expected {value:?}, got text {texts:?}, title {title:?}, aria-label {label:?}"),
        )
    };

    expect("123")?;
    // `-12` is as many characters as `123`.
    scene
        .set_data_values(node, NodeValues::I16(vec![-12]))
        .map_err(|e| e.to_string())?;
    expect("-12")?;
    scene
        .set_data_values(node, NodeValues::I16(vec![456]))
        .map_err(|e| e.to_string())?;
    expect("456")?;
    scene
        .set_data_values(node, NodeValues::I16(vec![-99]))
        .map_err(|e| e.to_string())?;
    expect("-99")?;
    scene
        .set_data_values(node, NodeValues::I16(vec![0]))
        .map_err(|e| e.to_string())?;
    expect("0")
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// `PlainText` is for `u8` bytes only. Signed bytes are rejected even when every value is positive and happens to be a
/// printable ASCII code, so this restriction is deliberate and not an accident of the values chosen.
#[wasm_bindgen_test]
fn plain_text_rejects_signed_input_even_for_printable_values() -> Result<(), String> {
    let svg = make_svg("signed-plain-text", Size::new(400.0, 200.0), Size::new(400.0, 200.0));
    let scene = Scene::new(svg).map_err(|e| e.to_string())?;
    let outcome = scene.add_data_node(
        Point::new(10.0, 10.0),
        DataNodeContent::new(NodeValues::I8(vec![65, 66]), DataFormat::PlainText),
    );
    check(
        matches!(outcome, Err(Error::InvalidPlainText)),
        &format!("expected InvalidPlainText, got {outcome:?}"),
    )?;
    let drawn = nth_group("signed-plain-text", 0).is_ok();
    check(!drawn, "a rejected call should draw nothing")?;

    // Replacing the bytes of a plain-text node with signed ones is refused too.
    let (scene, node) = scene_with(
        "signed-plain-text-replace",
        DataNodeContent::new(NodeValues::U8(b"AB".to_vec()), DataFormat::PlainText),
    )?;
    let refused = scene.set_data_values(node, NodeValues::I8(vec![65, 66]));
    check(
        matches!(refused, Err(Error::IncompatibleNodeValues(_))),
        &format!("expected IncompatibleNodeValues, got {refused:?}"),
    )?;
    let texts = texts_of("signed-plain-text-replace")?;
    check(texts == ["AB"], &format!("the text should be unchanged, got {texts:?}"))
}
