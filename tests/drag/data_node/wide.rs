//! 128-bit values: `NodeValues::U128`/`I128` and `NodeValues::Uuid` with `DataFormat::Uuid`. Each writes what it
//! should, every cell is wide enough for its text, a UUID format is rejected for anything but UUIDs, and no operator
//! accepts a UUID.

use super::support::{rect_children, text_children, title_of};
use crate::common::{check, make_svg, nth_group};
use svg_dom::root::utils::{Point, Size};
use svg_dom_graph::{
    Error,
    scene::{
        ArithmeticOperator, BinaryOperator, DataFormat, DataNodeContent, GridLayout, NodeValues, Scene, UnaryOperator,
        Uuid, UuidCase, UuidFormat, UuidLayout, UuidStyle,
    },
};
use wasm_bindgen_test::wasm_bindgen_test;

const EXAMPLE: u128 = 0x123e4567_e89b_12d3_a456_426614174000;

fn scene_with(id: &str, content: DataNodeContent) -> Result<Scene, String> {
    let svg = make_svg(id, Size::new(900.0, 400.0), Size::new(900.0, 400.0));
    let scene = Scene::new(svg).map_err(|e| e.to_string())?;
    scene
        .add_data_node(Point::new(10.0, 10.0), content)
        .map_err(|e| e.to_string())?;
    Ok(scene)
}

fn texts_of(id: &str) -> Result<Vec<String>, String> {
    let group = nth_group(id, 0)?;
    Ok(text_children(&group)?
        .iter()
        .map(|t| t.text_content().unwrap_or_default())
        .collect())
}

/// Every text of node `id` is inside its own cell, which is inside the node.
fn check_fits(id: &str) -> Result<(), String> {
    let group = nth_group(id, 0)?;
    let texts = text_children(&group)?;
    let rects = rect_children(&group)?;
    // A single value has one box, and several have an outer box and one for each cell.
    let cells = if rects.len() == texts.len() { &rects[..] } else { &rects[1..] };
    check(
        cells.len() == texts.len(),
        &format!("{id}: {} rects for {} texts", rects.len(), texts.len()),
    )?;
    for (text, rect) in texts.iter().zip(cells) {
        let (needed, available) = (text.get_bounding_client_rect().width(), rect.get_bounding_client_rect().width());
        check(
            needed <= available,
            &format!("{id}: {:?} needs {needed} but its cell is {available}", text.text_content()),
        )?;
    }
    Ok(())
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// A `u128` and an `i128` write their whole range, a sign only when negative, and fit their cells.
#[wasm_bindgen_test]
fn the_whole_range_of_a_128_bit_integer_is_written_and_fits() -> Result<(), String> {
    let _a = scene_with(
        "wide-u128",
        DataNodeContent::new(NodeValues::U128(vec![0, u128::MAX]), DataFormat::Decimal)
            .with_layout(GridLayout::Columns(1)),
    )?;
    let _b = scene_with(
        "wide-i128",
        DataNodeContent::new(NodeValues::I128(vec![7, i128::MIN]), DataFormat::Decimal)
            .with_layout(GridLayout::Columns(1)),
    )?;
    check(
        texts_of("wide-u128")? == ["0", "340282366920938463463374607431768211455"],
        &format!("u128 {:?}", texts_of("wide-u128")?),
    )?;
    check(
        texts_of("wide-i128")? == ["7", "-170141183460469231731687303715884105728"],
        &format!("i128 {:?}", texts_of("wide-i128")?),
    )?;
    check_fits("wide-u128")?;
    check_fits("wide-i128")
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Each style of UUID writes the digits its own way, and its node is wide enough for them.
#[wasm_bindgen_test]
fn every_uuid_style_is_written_and_fits() -> Result<(), String> {
    let cases = [
        ("uuid-hyphenated", UuidStyle::Hyphenated, "123e4567-e89b-12d3-a456-426614174000"),
        ("uuid-simple", UuidStyle::Simple, "123e4567e89b12d3a456426614174000"),
        ("uuid-braced", UuidStyle::Braced, "{123e4567-e89b-12d3-a456-426614174000}"),
        ("uuid-urn", UuidStyle::Urn, "urn:uuid:123e4567-e89b-12d3-a456-426614174000"),
    ];
    for (id, style, expected) in cases {
        let format = DataFormat::Uuid(UuidFormat::new().with_style(style));
        let _scene = scene_with(
            id,
            DataNodeContent::new(NodeValues::Uuid(vec![Uuid::from_u128(EXAMPLE)]), format),
        )?;
        let texts = texts_of(id)?;
        check(texts == [expected], &format!("{id}: {texts:?}"))?;
        check_fits(id)?;
    }
    Ok(())
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Case and layout reach the page too, and a node of several UUIDs fits every one of them.
#[wasm_bindgen_test]
fn uppercase_and_the_guid_layout_reach_the_page_and_every_uuid_fits() -> Result<(), String> {
    let in_memory = Uuid::from_bytes([
        0x33, 0x22, 0x11, 0x00, 0x55, 0x44, 0x77, 0x66, 0x88, 0x99, 0xAA, 0xBB, 0xCC, 0xDD, 0xEE, 0xFF,
    ]);
    let format = UuidFormat::new().with_case(UuidCase::Upper).with_layout(UuidLayout::Guid);
    let _scene = scene_with(
        "uuid-many",
        DataNodeContent::new(
            NodeValues::Uuid(vec![in_memory, Uuid::from_u128(0), Uuid::from_u128(u128::MAX)]),
            DataFormat::Uuid(format),
        )
        .with_layout(GridLayout::Columns(1)),
    )?;
    let texts = texts_of("uuid-many")?;
    check(
        texts
            == [
                "00112233-4455-6677-8899-AABBCCDDEEFF",
                "00000000-0000-0000-0000-000000000000",
                "FFFFFFFF-FFFF-FFFF-FFFF-FFFFFFFFFFFF",
            ],
        &format!("{texts:?}"),
    )?;
    check_fits("uuid-many")
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// A UUID names its own type and the text it shows, to assistive technology and to the tooltip.
#[wasm_bindgen_test]
fn a_uuid_node_names_its_type_and_the_text_it_shows() -> Result<(), String> {
    let _scene = scene_with(
        "uuid-name",
        DataNodeContent::new(
            NodeValues::Uuid(vec![Uuid::from_u128(EXAMPLE)]),
            DataFormat::Uuid(UuidFormat::new()),
        ),
    )?;
    let group = nth_group("uuid-name", 0)?;
    let expected = "uuid = 123e4567-e89b-12d3-a456-426614174000";
    check(
        group.get_attribute("aria-label").as_deref() == Some(expected),
        &format!("aria-label {:?}", group.get_attribute("aria-label")),
    )?;
    check(title_of(&group)?.as_deref() == Some(expected), "the tooltip should match")
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// A UUID format asks for UUIDs, and anything else is rejected before anything is drawn.
#[wasm_bindgen_test]
fn a_uuid_format_is_rejected_for_anything_but_uuids() -> Result<(), String> {
    let svg = make_svg("uuid-invalid", Size::new(400.0, 200.0), Size::new(400.0, 200.0));
    let scene = Scene::new(svg).map_err(|e| e.to_string())?;
    for values in [NodeValues::U128(vec![1]), NodeValues::U64(vec![1]), NodeValues::F32(vec![1.0])] {
        let outcome = scene.add_data_node(
            Point::new(10.0, 10.0),
            DataNodeContent::new(values, DataFormat::Uuid(UuidFormat::new())),
        );
        check(
            matches!(outcome, Err(Error::InvalidUuidFormat)),
            &format!("expected InvalidUuidFormat, got {outcome:?}"),
        )?;
    }
    check(nth_group("uuid-invalid", 0).is_err(), "a rejected call should draw nothing")
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// A 128-bit number is an ordinary operand, and a UUID is refused by every kind of operator. Nothing is drawn for a
/// rejected call.
#[wasm_bindgen_test]
fn operators_accept_128_bit_numbers_and_refuse_uuids() -> Result<(), String> {
    let svg = make_svg("uuid-operators", Size::new(700.0, 400.0), Size::new(700.0, 400.0));
    let scene = Scene::new(svg).map_err(|e| e.to_string())?;
    let data = |y: f64, values: NodeValues| {
        scene
            .add_data_node(Point::new(10.0, y), DataNodeContent::new(values, DataFormat::Decimal))
            .map_err(|e| e.to_string())
    };
    let uuid = |n: u128| NodeValues::Uuid(vec![Uuid::from_u128(n)]);
    let (n1, n2) = (data(10.0, NodeValues::U128(vec![5]))?, data(110.0, NodeValues::U128(vec![7]))?);
    let (u1, u2) = (data(210.0, uuid(1))?, data(310.0, uuid(2))?);

    scene
        .add_arithmetic_operator_node(
            Point::new(300.0, 60.0),
            ArithmeticOperator::Add,
            (n1, n2),
            DataNodeContent::new(NodeValues::U128(vec![12]), DataFormat::Decimal),
        )
        .map_err(|e| e.to_string())?;
    let drawn = |i: u32| nth_group("uuid-operators", i).is_ok();
    // Four data nodes and one operator are drawn so far.
    check(drawn(4) && !drawn(5), "only the arithmetic node should have been added")?;

    let result = || DataNodeContent::new(uuid(3), DataFormat::Decimal);
    let arithmetic =
        scene.add_arithmetic_operator_node(Point::new(300.0, 260.0), ArithmeticOperator::Add, (u1, u2), result());
    check(
        matches!(&arithmetic, Err(Error::OperatorOnUuid { operator }) if operator == "ADD"),
        &format!("expected OperatorOnUuid for ADD, got {arithmetic:?}"),
    )?;
    let binary = scene.add_binary_operator_node(Point::new(300.0, 260.0), BinaryOperator::Xor, (u1, u2), result());
    check(
        matches!(&binary, Err(Error::OperatorOnUuid { operator }) if operator == "XOR"),
        &format!("expected OperatorOnUuid for XOR, got {binary:?}"),
    )?;
    let unary = scene.add_unary_operator_node(Point::new(300.0, 260.0), UnaryOperator::Not, u1, result());
    check(
        matches!(&unary, Err(Error::OperatorOnUuid { operator }) if operator == "NOT"),
        &format!("expected OperatorOnUuid for NOT, got {unary:?}"),
    )?;
    check(!drawn(5), "a rejected operator should draw nothing")
}
