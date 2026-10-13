//! `NodeValues::Bool`, `Usize` and `Isize`: each `BoolFormat`, the marks and what a screen reader is told for them, a
//! Boolean's cells that stay wide enough when a value flips, the logical operators, and the pointer-sized integers.

use super::support::{check_fits, texts_of, title_of};
use crate::common::{check, make_svg, nth_group};
use svg_dom::root::utils::{Point, Size};
use svg_dom_graph::{
    Error, NodeId,
    scene::{
        ArithmeticOperator, BinaryOperator, BoolFormat, DataFormat, DataNodeContent, GridLayout, NodeValues, Scene,
        UnaryOperator,
    },
};
use wasm_bindgen_test::wasm_bindgen_test;

const CHECK: &str = "\u{2714}";
const CROSS: &str = "\u{2718}";

fn scene_and_node(id: &str, content: DataNodeContent) -> Result<(Scene, NodeId), String> {
    let svg = make_svg(id, Size::new(900.0, 400.0), Size::new(900.0, 400.0));
    let scene = Scene::new(svg).map_err(|e| e.to_string())?;
    let node = scene
        .add_data_node(Point::new(10.0, 10.0), content)
        .map_err(|e| e.to_string())?;
    Ok((scene, node))
}

fn flags(values: Vec<bool>, format: BoolFormat) -> DataNodeContent {
    DataNodeContent::new(NodeValues::Bool(values), DataFormat::Bool(format)).with_layout(GridLayout::Columns(1))
}

/// How one `<text>` of a node is drawn and named.
#[derive(Debug)]
struct Cell {
    fill: Option<String>,
    name: Option<String>,
}

/// Each `<text>` of node `id`, in order.
fn fills_and_names(id: &str) -> Result<Vec<Cell>, String> {
    let group = nth_group(id, 0)?;
    let texts = group.query_selector_all("text").map_err(|e| format!("{e:?}"))?;
    let mut out = Vec::new();
    for i in 0..texts.length() {
        let el: web_sys::Element = wasm_bindgen::JsCast::dyn_into(texts.get(i).ok_or("missing text")?)
            .map_err(|_| "text is not an Element".to_owned())?;
        out.push(Cell {
            fill: el.get_attribute("fill"),
            name: el.get_attribute("aria-label"),
        });
    }
    Ok(out)
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Each format writes a Boolean its own way, and every cell fits.
#[wasm_bindgen_test]
fn every_bool_format_is_written_and_fits() -> Result<(), String> {
    let cases = [
        ("bool-words", BoolFormat::TrueFalse, ["true", "false"]),
        ("bool-bits", BoolFormat::Bit, ["1", "0"]),
        ("bool-icons", BoolFormat::Icon, [CHECK, CROSS]),
    ];
    for (id, format, expected) in cases {
        let _scene = scene_and_node(id, flags(vec![true, false], format))?;
        let texts = texts_of(id)?;
        check(texts == expected, &format!("{id}: {texts:?}"))?;
        check_fits(id)?;
    }
    Ok(())
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// A check mark is green and a cross is red, and neither is spoken as a glyph: the accessible name of each cell says
/// `true` or `false`. A word is drawn in the ordinary text colour.
#[wasm_bindgen_test]
fn icons_are_coloured_and_spoken_as_words() -> Result<(), String> {
    let _a = scene_and_node("bool-icon-ink", flags(vec![true, false], BoolFormat::Icon))?;
    let _b = scene_and_node("bool-word-ink", flags(vec![true, false], BoolFormat::TrueFalse))?;
    let icons = fills_and_names("bool-icon-ink")?;
    let words = fills_and_names("bool-word-ink")?;
    check(
        icons[0].fill != icons[1].fill,
        "a check mark and a cross should have different fills",
    )?;
    check(
        icons[0].fill != words[0].fill && icons[1].fill != words[0].fill,
        "an icon is not drawn in the text colour",
    )?;
    check(words[0].fill == words[1].fill, "words share the text colour")?;
    check(
        icons[0].name.as_deref() == Some("row 0, column 0: true")
            && icons[1].name.as_deref() == Some("row 1, column 0: false"),
        &format!("the names of the icons were {icons:?}"),
    )
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// A node drawn with only `true` is wide enough to be changed to `false`, in every format. The colour and the
/// accessible name follow the new value, and nothing overflows.
#[wasm_bindgen_test]
fn flipping_every_value_keeps_the_cells_wide_enough_and_updates_colour_and_name() -> Result<(), String> {
    for (id, format) in [
        ("flip-words", BoolFormat::TrueFalse),
        ("flip-bits", BoolFormat::Bit),
        ("flip-icons", BoolFormat::Icon),
    ] {
        let (scene, node) = scene_and_node(id, flags(vec![true, true], format))?;
        let rect_before = scene.node_rect(node).map_err(|e| e.to_string())?;
        let ink_before = fills_and_names(id)?;
        scene
            .set_data_values(node, NodeValues::Bool(vec![false, false]))
            .map_err(|e| e.to_string())?;
        check_fits(id)?;
        check(
            scene.node_rect(node).map_err(|e| e.to_string())? == rect_before,
            &format!("{id}: the node's rect changed"),
        )?;
        let names = fills_and_names(id)?;
        let spoken = if format == BoolFormat::Bit { "0" } else { "false" };
        check(
            names[0].name.as_deref() == Some(&format!("row 0, column 0: {spoken}")),
            &format!("{id}: {names:?}"),
        )?;
        if format == BoolFormat::Icon {
            check(names[0].fill != ink_before[0].fill, "the mark's colour should follow its value")?;
        }
    }
    Ok(())
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// A single Boolean names its type and value, in words even for an icon, in the label and the tooltip, and follows a
/// change.
#[wasm_bindgen_test]
fn a_single_icon_names_its_value_in_words_and_follows_a_change() -> Result<(), String> {
    let (scene, node) = scene_and_node(
        "bool-single",
        DataNodeContent::new(NodeValues::Bool(vec![true]), DataFormat::Bool(BoolFormat::Icon)),
    )?;
    let group = nth_group("bool-single", 0)?;
    for (value, mark, expected) in [
        (true, CHECK, "bool = true"),
        (false, CROSS, "bool = false"),
        (true, CHECK, "bool = true"),
    ] {
        scene
            .set_data_values(node, NodeValues::Bool(vec![value]))
            .map_err(|e| e.to_string())?;
        check(texts_of("bool-single")? == [mark], &format!("the mark for {value}"))?;
        check(
            group.get_attribute("aria-label").as_deref() == Some(expected),
            &format!("aria-label {:?}, expected {expected:?}", group.get_attribute("aria-label")),
        )?;
        check(title_of(&group)?.as_deref() == Some(expected), "the tooltip should match")?;
    }
    check_fits("bool-single")
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// A Boolean format asks for Booleans, and anything else is rejected before anything is drawn.
#[wasm_bindgen_test]
fn a_bool_format_is_rejected_for_anything_but_booleans() -> Result<(), String> {
    let svg = make_svg("bool-invalid", Size::new(400.0, 200.0), Size::new(400.0, 200.0));
    let scene = Scene::new(svg).map_err(|e| e.to_string())?;
    for values in [NodeValues::U8(vec![1]), NodeValues::Usize(vec![1]), NodeValues::F64(vec![1.0])] {
        let outcome = scene.add_data_node(
            Point::new(10.0, 10.0),
            DataNodeContent::new(values, DataFormat::Bool(BoolFormat::TrueFalse)),
        );
        check(
            matches!(outcome, Err(Error::InvalidBoolFormat)),
            &format!("expected InvalidBoolFormat, got {outcome:?}"),
        )?;
    }
    check(nth_group("bool-invalid", 0).is_err(), "a rejected call should draw nothing")
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The logical operators accept Booleans, and the result of one names its value in words. A shift, a rotation, and an
/// arithmetic operator refuse them. Nothing is drawn for a rejected call.
#[wasm_bindgen_test]
fn logical_operators_accept_booleans_and_the_others_refuse_them() -> Result<(), String> {
    let svg = make_svg("bool-operators", Size::new(700.0, 400.0), Size::new(700.0, 400.0));
    let scene = Scene::new(svg).map_err(|e| e.to_string())?;
    let flag = |y: f64, value: bool| {
        scene
            .add_data_node(
                Point::new(10.0, y),
                DataNodeContent::new(NodeValues::Bool(vec![value]), DataFormat::Bool(BoolFormat::Icon)),
            )
            .map_err(|e| e.to_string())
    };
    let (a, b) = (flag(10.0, true)?, flag(110.0, false)?);
    let result = |value: bool| DataNodeContent::new(NodeValues::Bool(vec![value]), DataFormat::Bool(BoolFormat::Icon));

    scene
        .add_binary_operator_node(Point::new(300.0, 60.0), BinaryOperator::And, (a, b), result(false))
        .map_err(|e| e.to_string())?;
    let and = nth_group("bool-operators", 2)?;
    let label = and.get_attribute("aria-label");
    check(
        label.as_deref().is_some_and(|l| l.starts_with("AND result: bool = false.")),
        &format!("aria-label {label:?}"),
    )?;
    scene
        .add_unary_operator_node(Point::new(300.0, 200.0), UnaryOperator::Not, a, result(false))
        .map_err(|e| e.to_string())?;

    let drawn = |i: u32| nth_group("bool-operators", i).is_ok();
    check(drawn(3) && !drawn(4), "only the AND and the NOT should have been added")?;

    let shift = scene.add_unary_operator_node(Point::new(300.0, 300.0), UnaryOperator::ShiftLeft(1), a, result(true));
    check(
        matches!(&shift, Err(Error::OperatorOnBool { operator }) if operator == "SHL 1"),
        &format!("expected OperatorOnBool for SHL 1, got {shift:?}"),
    )?;
    let add =
        scene.add_arithmetic_operator_node(Point::new(300.0, 300.0), ArithmeticOperator::Add, (a, b), result(true));
    check(
        matches!(&add, Err(Error::OperatorOnBool { operator }) if operator == "ADD"),
        &format!("expected OperatorOnBool for ADD, got {add:?}"),
    )?;
    check(!drawn(4), "a rejected operator should draw nothing")
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// A pointer-sized integer writes its whole range, and its cells fit. This runs on the `wasm32` target, where it is 32
/// bits wide, and it asks the target and not a constant for the range.
#[wasm_bindgen_test]
fn pointer_sized_integers_write_their_whole_range_and_fit() -> Result<(), String> {
    let _a = scene_and_node(
        "size-usize",
        DataNodeContent::new(NodeValues::Usize(vec![0, usize::MAX]), DataFormat::Decimal)
            .with_layout(GridLayout::Columns(1)),
    )?;
    let _b = scene_and_node(
        "size-isize",
        DataNodeContent::new(NodeValues::Isize(vec![isize::MIN, 0, isize::MAX]), DataFormat::Decimal)
            .with_layout(GridLayout::Columns(1)),
    )?;
    check(texts_of("size-usize")? == ["0".to_owned(), usize::MAX.to_string()], "usize")?;
    check(
        texts_of("size-isize")? == [isize::MIN.to_string(), "0".to_owned(), isize::MAX.to_string()],
        "isize",
    )?;
    check_fits("size-usize")?;
    check_fits("size-isize")
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// A pointer-sized integer replaced across zero keeps its node's size, and its type is named.
#[wasm_bindgen_test]
fn a_pointer_sized_integer_can_be_replaced_and_names_its_type() -> Result<(), String> {
    let (scene, node) = scene_and_node(
        "size-replace",
        DataNodeContent::new(NodeValues::Isize(vec![isize::MIN]), DataFormat::Decimal),
    )?;
    let group = nth_group("size-replace", 0)?;
    for value in [isize::MAX, 0, -1, isize::MIN] {
        scene
            .set_data_values(node, NodeValues::Isize(vec![value]))
            .map_err(|e| e.to_string())?;
        let expected = format!("isize = {value}");
        check(
            group.get_attribute("aria-label").as_deref() == Some(expected.as_str()),
            &format!("aria-label {:?}, expected {expected:?}", group.get_attribute("aria-label")),
        )?;
    }
    check_fits("size-replace")?;
    let wrong = scene.set_data_values(node, NodeValues::Usize(vec![1]));
    check(
        matches!(wrong, Err(Error::IncompatibleNodeValues(_))),
        &format!("a usize is not an isize, got {wrong:?}"),
    )
}
