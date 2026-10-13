//! 128-bit values: `NodeValues::U128`/`I128` and `NodeValues::Uuid` with `DataFormat::Uuid`. Each writes what it
//! should, every cell is wide enough for its text, a UUID format is rejected for anything but UUIDs, and no operator
//! accepts a UUID.

use super::support::{check_fits, texts_of, title_of};
use crate::common::{check, make_svg, mutations_during, nth_group};
use svg_dom::root::utils::{Point, Size};
use svg_dom_graph::{
    Error, NodeId,
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

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// A node of `content`, drawn alone, and the node it is.
fn scene_and_node(id: &str, content: DataNodeContent) -> Result<(Scene, NodeId), String> {
    let svg = make_svg(id, Size::new(900.0, 400.0), Size::new(900.0, 400.0));
    let scene = Scene::new(svg).map_err(|e| e.to_string())?;
    let node = scene
        .add_data_node(Point::new(10.0, 10.0), content)
        .map_err(|e| e.to_string())?;
    Ok((scene, node))
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Replacing `u128` values with the smallest and then the largest keeps the node's size, rewrites every cell, and no
/// text overflows its cell, since the node was drawn wide enough for the largest.
#[wasm_bindgen_test]
fn extreme_u128_values_replace_in_place_and_still_fit() -> Result<(), String> {
    let (scene, node) = scene_and_node(
        "set-u128",
        DataNodeContent::new(NodeValues::U128(vec![u128::MAX, 1]), DataFormat::Decimal)
            .with_layout(GridLayout::Columns(1)),
    )?;
    let rect_before = scene.node_rect(node).map_err(|e| e.to_string())?;
    for (values, expected) in [
        (vec![0, u128::MAX], ["0", "340282366920938463463374607431768211455"]),
        (vec![u128::MAX, 0], ["340282366920938463463374607431768211455", "0"]),
        (
            vec![1 << 64, 1 << 127],
            ["18446744073709551616", "170141183460469231731687303715884105728"],
        ),
    ] {
        scene
            .set_data_values(node, NodeValues::U128(values))
            .map_err(|e| e.to_string())?;
        let texts = texts_of("set-u128")?;
        check(texts == expected, &format!("expected {expected:?}, got {texts:?}"))?;
        check_fits("set-u128")?;
        check(
            scene.node_rect(node).map_err(|e| e.to_string())? == rect_before,
            "the node's own rect changed",
        )?;
    }
    Ok(())
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// An `i128` crosses zero and both extremes, and the sign is written only when the value is negative. The node was
/// drawn for the most negative value, which is the widest, so every replacement fits.
#[wasm_bindgen_test]
fn extreme_i128_values_replace_across_zero_and_still_fit() -> Result<(), String> {
    let (scene, node) = scene_and_node(
        "set-i128",
        DataNodeContent::new(NodeValues::I128(vec![i128::MIN, i128::MAX]), DataFormat::Decimal)
            .with_layout(GridLayout::Columns(1)),
    )?;
    for (values, expected) in [
        (
            vec![i128::MAX, i128::MIN],
            [
                "170141183460469231731687303715884105727",
                "-170141183460469231731687303715884105728",
            ],
        ),
        (vec![-1, 0], ["-1", "0"]),
        (vec![0, 1], ["0", "1"]),
    ] {
        scene
            .set_data_values(node, NodeValues::I128(values))
            .map_err(|e| e.to_string())?;
        let texts = texts_of("set-i128")?;
        check(texts == expected, &format!("expected {expected:?}, got {texts:?}"))?;
        check_fits("set-i128")?;
    }
    Ok(())
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// A single `i128` is replaced in place across zero and both extremes, and its accessible name and tooltip follow it.
#[wasm_bindgen_test]
fn a_single_i128_follows_its_value_in_text_name_and_tooltip() -> Result<(), String> {
    let (scene, node) = scene_and_node(
        "set-i128-single",
        DataNodeContent::new(NodeValues::I128(vec![i128::MIN]), DataFormat::Decimal),
    )?;
    let group = nth_group("set-i128-single", 0)?;
    for value in [i128::MAX, 0, -1, i128::MIN] {
        scene
            .set_data_values(node, NodeValues::I128(vec![value]))
            .map_err(|e| e.to_string())?;
        let expected = format!("i128 = {value}");
        check(
            texts_of("set-i128-single")? == [value.to_string()],
            &format!("text for {value}"),
        )?;
        check(
            group.get_attribute("aria-label").as_deref() == Some(expected.as_str()),
            &format!("aria-label {:?}, expected {expected:?}", group.get_attribute("aria-label")),
        )?;
        check(
            title_of(&group)?.as_deref() == Some(expected.as_str()),
            "the tooltip should match",
        )?;
    }
    check_fits("set-i128-single")
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// A node of UUIDs is replaced by others, and the text of every cell is as wide as before, whatever the UUID. That is
/// what lets one measurement serve a node's every UUID. Unchanged UUIDs are not rewritten, and values of another type
/// are rejected.
#[wasm_bindgen_test]
fn replacing_uuids_rewrites_only_what_changed_and_never_changes_the_width() -> Result<(), String> {
    let first = Uuid::from_u128(EXAMPLE);
    let (scene, node) = scene_and_node(
        "set-uuid",
        DataNodeContent::new(
            NodeValues::Uuid(vec![first, Uuid::from_u128(0)]),
            DataFormat::Uuid(UuidFormat::new()),
        )
        .with_layout(GridLayout::Columns(1)),
    )?;
    let group = nth_group("set-uuid", 0)?;
    let rect_before = scene.node_rect(node).map_err(|e| e.to_string())?;

    // The first UUID is unchanged, so only the second cell is written.
    let seen = mutations_during(&group, || {
        scene
            .set_data_values(node, NodeValues::Uuid(vec![first, Uuid::from_u128(u128::MAX)]))
            .map_err(|e| e.to_string())
    })?;
    check(seen > 0, "the changed UUID should have been written")?;
    check(
        texts_of("set-uuid")? == ["123e4567-e89b-12d3-a456-426614174000", "ffffffff-ffff-ffff-ffff-ffffffffffff"],
        &format!("{:?}", texts_of("set-uuid")?),
    )?;

    let seen = mutations_during(&group, || {
        scene
            .set_data_values(node, NodeValues::Uuid(vec![first, Uuid::from_u128(u128::MAX)]))
            .map_err(|e| e.to_string())
    })?;
    check(
        seen == 0,
        &format!("identical UUIDs should change nothing, saw {seen} mutations"),
    )?;

    scene
        .set_data_values(node, NodeValues::Uuid(vec![Uuid::from_u128(0), Uuid::from_u128(1)]))
        .map_err(|e| e.to_string())?;
    check_fits("set-uuid")?;
    check(
        scene.node_rect(node).map_err(|e| e.to_string())? == rect_before,
        "the node's own rect changed",
    )?;

    let rejected = scene.set_data_values(node, NodeValues::U128(vec![1, 2]));
    check(
        matches!(rejected, Err(Error::IncompatibleNodeValues(_))),
        &format!("a u128 is not a UUID, got {rejected:?}"),
    )?;
    check(
        texts_of("set-uuid")? == ["00000000-0000-0000-0000-000000000000", "00000000-0000-0000-0000-000000000001"],
        "a rejected replacement should change nothing",
    )
}
