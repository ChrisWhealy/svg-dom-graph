//! `Scene::measure_data_node`/`measure_named_data_node`/`measure_operator_box` return the size a data or operator node
//! would render at in *this* `Scene`, without ever adding it. See the design proposal's own "Acceptance tests" for the
//! full list this file implements against.
//!
//! The central property under test throughout is *exact* equality. `measure_*` must report precisely what `node_rect`
//! on the equivalent `add_*` call would report. "Close enough" is not enough, and neither is "some nonzero size." Both
//! values come from the same browser text measurement, the same `draw_*` implementation and the same arithmetic. A
//! difference of even a thousandth of a pixel would mean the two code paths have silently diverged. `check_close`'s own
//! tolerance would hide that.

use crate::common::{check, make_svg};
use svg_dom::root::utils::{Point, Size};
use svg_dom_graph::{
    Error,
    scene::{
        ArithmeticOperator, BinaryOperator, DataFormat, DataNodeContent, GridLayout, LabellingStyle, NodeValues, Scene,
        UnaryOperator,
    },
};
use wasm_bindgen::JsCast;
use wasm_bindgen_test::wasm_bindgen_test;

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Exact equality, not [`crate::common::check_close`]'s tolerance — see this module's own doc comment for why.
fn assert_size_exact(measured: Size, actual: Size) -> Result<(), String> {
    check(
        measured == actual,
        &format!("measured size {measured:?} does not exactly equal the real rendered size {actual:?}"),
    )
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The number of elements anywhere under `root`, not just its own direct children. `Element::child_element_count` alone
/// would miss a leaked element nested inside `root`'s own persistent content (a `<g class="scene-content">` wrapper,
/// say). It would count only one sitting directly under the `<svg>` root itself.
fn descendant_element_count(root: &web_sys::Element) -> Result<u32, String> {
    root.query_selector_all("*")
        .map(|nodes| nodes.length())
        .map_err(|e| format!("{e:?}"))
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Acceptance test 1: `measure_data_node` against a single-value data node.
#[wasm_bindgen_test]
fn measure_data_node_matches_a_single_value_node() -> Result<(), String> {
    let svg = make_svg("measure-data-single", Size::new(400.0, 260.0), Size::new(400.0, 260.0));
    let scene = Scene::new(svg).map_err(|e| e.to_string())?;
    let content = DataNodeContent::new(NodeValues::U64(vec![0xF0E1D2C3B4A59687]), DataFormat::Hexadecimal);

    let measured = scene.measure_data_node(&content).map_err(|e| e.to_string())?;
    let id = scene
        .add_data_node(Point::new(10.0, 10.0), content)
        .map_err(|e| e.to_string())?;
    let actual = scene.node_rect(id).map_err(|e| e.to_string())?.size;

    assert_size_exact(measured, actual)
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Acceptance test 2: the same equality for a multi-cell/grid data node.
#[wasm_bindgen_test]
fn measure_data_node_matches_a_grid_data_node() -> Result<(), String> {
    let svg = make_svg("measure-data-grid", Size::new(400.0, 260.0), Size::new(400.0, 260.0));
    let scene = Scene::new(svg).map_err(|e| e.to_string())?;
    let content = DataNodeContent::new(NodeValues::U8((1..=10).collect()), DataFormat::Decimal);

    let measured = scene.measure_data_node(&content).map_err(|e| e.to_string())?;
    let id = scene
        .add_data_node(Point::new(10.0, 10.0), content)
        .map_err(|e| e.to_string())?;
    let actual = scene.node_rect(id).map_err(|e| e.to_string())?.size;

    assert_size_exact(measured, actual)
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Acceptance test 3: `measure_named_data_node` against `add_named_data_node` — the extra label row included.
#[wasm_bindgen_test]
fn measure_named_data_node_matches_a_named_data_node() -> Result<(), String> {
    let svg = make_svg("measure-data-named", Size::new(400.0, 260.0), Size::new(400.0, 260.0));
    let scene = Scene::new(svg).map_err(|e| e.to_string())?;
    let content = DataNodeContent::new(NodeValues::U64(vec![0x1234_5678_9ABC_DEF0]), DataFormat::Hexadecimal);

    let measured = scene.measure_named_data_node("Checksum", &content).map_err(|e| e.to_string())?;
    let id = scene
        .add_named_data_node(Point::new(10.0, 10.0), "Checksum", content)
        .map_err(|e| e.to_string())?;
    let actual = scene.node_rect(id).map_err(|e| e.to_string())?.size;

    assert_size_exact(measured, actual)
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Acceptance test 4 (unary): `measure_operator_box` against `add_unary_operator_node` — no real operand needed.
#[wasm_bindgen_test]
fn measure_operator_box_matches_a_unary_operator_node() -> Result<(), String> {
    let svg = make_svg("measure-op-unary", Size::new(400.0, 260.0), Size::new(400.0, 260.0));
    let scene = Scene::new(svg).map_err(|e| e.to_string())?;
    let operand = scene
        .add_data_node(
            Point::new(10.0, 10.0),
            DataNodeContent::new(NodeValues::U32(vec![0x0F0F_0F0F]), DataFormat::Hexadecimal),
        )
        .map_err(|e| e.to_string())?;
    let result = DataNodeContent::new(NodeValues::U32(vec![!0x0F0F_0F0Fu32]), DataFormat::Hexadecimal);

    let measured = scene.measure_operator_box("NOT", &result).map_err(|e| e.to_string())?;
    let id = scene
        .add_unary_operator_node(Point::new(200.0, 10.0), UnaryOperator::Not, operand, result)
        .map_err(|e| e.to_string())?;
    let actual = scene.node_rect(id).map_err(|e| e.to_string())?.size;

    assert_size_exact(measured, actual)
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Acceptance test 4 (binary).
#[wasm_bindgen_test]
fn measure_operator_box_matches_a_binary_operator_node() -> Result<(), String> {
    let svg = make_svg("measure-op-binary", Size::new(400.0, 260.0), Size::new(400.0, 260.0));
    let scene = Scene::new(svg).map_err(|e| e.to_string())?;
    let a = scene
        .add_data_node(
            Point::new(10.0, 10.0),
            DataNodeContent::new(NodeValues::U64(vec![10]), DataFormat::Hexadecimal),
        )
        .map_err(|e| e.to_string())?;
    let b = scene
        .add_data_node(
            Point::new(10.0, 100.0),
            DataNodeContent::new(NodeValues::U64(vec![20]), DataFormat::Hexadecimal),
        )
        .map_err(|e| e.to_string())?;
    let result = DataNodeContent::new(NodeValues::U64(vec![30]), DataFormat::Hexadecimal);

    let measured = scene.measure_operator_box("XOR", &result).map_err(|e| e.to_string())?;
    let id = scene
        .add_binary_operator_node(Point::new(200.0, 50.0), BinaryOperator::Xor, (a, b), result)
        .map_err(|e| e.to_string())?;
    let actual = scene.node_rect(id).map_err(|e| e.to_string())?.size;

    assert_size_exact(measured, actual)
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Acceptance test 4 (arithmetic).
#[wasm_bindgen_test]
fn measure_operator_box_matches_an_arithmetic_operator_node() -> Result<(), String> {
    let svg = make_svg("measure-op-arith", Size::new(400.0, 260.0), Size::new(400.0, 260.0));
    let scene = Scene::new(svg).map_err(|e| e.to_string())?;
    let a = scene
        .add_data_node(
            Point::new(10.0, 10.0),
            DataNodeContent::new(NodeValues::U32(vec![7]), DataFormat::Decimal),
        )
        .map_err(|e| e.to_string())?;
    let b = scene
        .add_data_node(
            Point::new(10.0, 100.0),
            DataNodeContent::new(NodeValues::U32(vec![3]), DataFormat::Decimal),
        )
        .map_err(|e| e.to_string())?;
    let result = DataNodeContent::new(NodeValues::U32(vec![10]), DataFormat::Decimal);

    let measured = scene.measure_operator_box("ADD", &result).map_err(|e| e.to_string())?;
    let id = scene
        .add_arithmetic_operator_node(Point::new(200.0, 50.0), ArithmeticOperator::Add, (a, b), result)
        .map_err(|e| e.to_string())?;
    let actual = scene.node_rect(id).map_err(|e| e.to_string())?.size;

    assert_size_exact(measured, actual)
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Acceptance test 5/6: measurement leaves the `Scene`'s own rendered, persistent content unchanged. The count of
/// *every* element anywhere under the `<svg>` root, not just its own direct children, is identical before and after a
/// `measure_*` call. That proves the throwaway group was actually removed from wherever in the tree it was drawn, not
/// merely hidden or left nested inside the content layer.
#[wasm_bindgen_test]
fn measurement_leaves_the_scenes_own_rendered_content_unchanged() -> Result<(), String> {
    let svg = make_svg("measure-no-residue", Size::new(400.0, 260.0), Size::new(400.0, 260.0));
    let root = svg.root.clone();
    let scene = Scene::new(svg).map_err(|e| e.to_string())?;
    scene
        .add_data_node(
            Point::new(10.0, 10.0),
            DataNodeContent::new(NodeValues::U8(vec![1, 2, 3]), DataFormat::Decimal),
        )
        .map_err(|e| e.to_string())?;

    let before = descendant_element_count(&root)?;
    let content = DataNodeContent::new(NodeValues::U64(vec![0x1122_3344_5566_7788]), DataFormat::Hexadecimal);
    scene.measure_data_node(&content).map_err(|e| e.to_string())?;
    let after = descendant_element_count(&root)?;

    check(
        before == after,
        &format!("measure_data_node changed the <svg>'s own descendant count: {before} before, {after} after"),
    )
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Acceptance test 8: repeated measurement calls leave no accumulated DOM elements behind, anywhere in the tree.
#[wasm_bindgen_test]
fn repeated_measurement_calls_leave_no_accumulated_dom_elements() -> Result<(), String> {
    let svg = make_svg("measure-repeated", Size::new(400.0, 260.0), Size::new(400.0, 260.0));
    let root = svg.root.clone();
    let scene = Scene::new(svg).map_err(|e| e.to_string())?;
    let content = DataNodeContent::new(NodeValues::U64(vec![0xAABB_CCDD_EEFF_0011]), DataFormat::Hexadecimal);

    let before = descendant_element_count(&root)?;
    for _ in 0..20 {
        scene.measure_data_node(&content).map_err(|e| e.to_string())?;
        scene.measure_operator_box("XOR", &content).map_err(|e| e.to_string())?;
    }
    let after = descendant_element_count(&root)?;

    check(
        before == after,
        &format!("20 repeated measurements left the <svg>'s own descendant count at {after}, started at {before}"),
    )
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Acceptance test 7 (data node, empty content): the same error class from `measure_data_node` as `add_data_node`
/// already produces.
#[wasm_bindgen_test]
fn measure_data_node_rejects_empty_content_like_add_data_node_does() -> Result<(), String> {
    let svg = make_svg("measure-data-empty", Size::new(400.0, 260.0), Size::new(400.0, 260.0));
    let scene = Scene::new(svg).map_err(|e| e.to_string())?;
    let empty = DataNodeContent::new(NodeValues::U64(vec![]), DataFormat::Hexadecimal);

    check(
        matches!(scene.measure_data_node(&empty), Err(Error::EmptyNodeContent)),
        "measure_data_node did not reject empty content with Err(Error::EmptyNodeContent)",
    )?;
    check(
        matches!(scene.add_data_node(Point::new(0.0, 0.0), empty), Err(Error::EmptyNodeContent)),
        "add_data_node did not reject the same empty content with Err(Error::EmptyNodeContent)",
    )
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Acceptance test 7 (data node, invalid grid layout): the same error class from `measure_data_node` as `add_data_node`
/// already produces for a `GridLayout` that wraps `0`.
#[wasm_bindgen_test]
fn measure_data_node_rejects_an_invalid_grid_layout_like_add_data_node_does() -> Result<(), String> {
    let svg = make_svg("measure-data-bad-layout", Size::new(400.0, 260.0), Size::new(400.0, 260.0));
    let scene = Scene::new(svg).map_err(|e| e.to_string())?;
    let bad_layout =
        DataNodeContent::new(NodeValues::U8(vec![1, 2, 3]), DataFormat::Decimal).with_layout(GridLayout::Columns(0));

    check(
        matches!(scene.measure_data_node(&bad_layout), Err(Error::InvalidGridLayout(_))),
        "measure_data_node did not reject Columns(0) with Err(Error::InvalidGridLayout(_))",
    )?;
    check(
        matches!(
            scene.add_data_node(Point::new(0.0, 0.0), bad_layout),
            Err(Error::InvalidGridLayout(_))
        ),
        "add_data_node did not reject the same layout with Err(Error::InvalidGridLayout(_))",
    )
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Acceptance test 7 (named data node, blank name): the same error class from `measure_named_data_node` as
/// `add_named_data_node` already produces for an empty/whitespace-only name.
#[wasm_bindgen_test]
fn measure_named_data_node_rejects_a_blank_name_like_add_named_data_node_does() -> Result<(), String> {
    let svg = make_svg("measure-data-blank-name", Size::new(400.0, 260.0), Size::new(400.0, 260.0));
    let scene = Scene::new(svg).map_err(|e| e.to_string())?;
    let content = DataNodeContent::new(NodeValues::U64(vec![1]), DataFormat::Hexadecimal);

    check(
        matches!(scene.measure_named_data_node("   ", &content), Err(Error::EmptyNodeName)),
        "measure_named_data_node did not reject a blank name with Err(Error::EmptyNodeName)",
    )?;
    check(
        matches!(
            scene.add_named_data_node(Point::new(0.0, 0.0), "   ", content),
            Err(Error::EmptyNodeName)
        ),
        "add_named_data_node did not reject the same blank name with Err(Error::EmptyNodeName)",
    )
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Acceptance test 7 (operator node, multi-value result): the same error class from `measure_operator_box` as
/// `add_binary_operator_node` already produces.
#[wasm_bindgen_test]
fn measure_operator_box_rejects_a_multi_value_result_like_add_binary_operator_node_does() -> Result<(), String> {
    let svg = make_svg("measure-op-multi", Size::new(400.0, 260.0), Size::new(400.0, 260.0));
    let scene = Scene::new(svg).map_err(|e| e.to_string())?;
    let a = scene
        .add_data_node(
            Point::new(10.0, 10.0),
            DataNodeContent::new(NodeValues::U64(vec![1]), DataFormat::Hexadecimal),
        )
        .map_err(|e| e.to_string())?;
    let b = scene
        .add_data_node(
            Point::new(10.0, 100.0),
            DataNodeContent::new(NodeValues::U64(vec![2]), DataFormat::Hexadecimal),
        )
        .map_err(|e| e.to_string())?;
    let multi_value = DataNodeContent::new(NodeValues::U64(vec![1, 2]), DataFormat::Hexadecimal);

    check(
        matches!(
            scene.measure_operator_box("XOR", &multi_value),
            Err(Error::OperatorResultNotSingleValue(2))
        ),
        "measure_operator_box did not reject a 2-value result with Err(Error::OperatorResultNotSingleValue(2))",
    )?;
    check(
        matches!(
            scene.add_binary_operator_node(Point::new(200.0, 50.0), BinaryOperator::Xor, (a, b), multi_value),
            Err(Error::OperatorResultNotSingleValue(2))
        ),
        "add_binary_operator_node did not reject the same result with Err(Error::OperatorResultNotSingleValue(2))",
    )
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Acceptance test 7 (operator node, invalid grid layout): the same error class from `measure_operator_box` as a real
/// operator node's own `result` validation already produces for a `GridLayout` that wraps `0`.
#[wasm_bindgen_test]
fn measure_operator_box_rejects_an_invalid_grid_layout_like_add_binary_operator_node_does() -> Result<(), String> {
    let svg = make_svg("measure-op-bad-layout", Size::new(400.0, 260.0), Size::new(400.0, 260.0));
    let scene = Scene::new(svg).map_err(|e| e.to_string())?;
    let a = scene
        .add_data_node(
            Point::new(10.0, 10.0),
            DataNodeContent::new(NodeValues::U64(vec![1]), DataFormat::Hexadecimal),
        )
        .map_err(|e| e.to_string())?;
    let b = scene
        .add_data_node(
            Point::new(10.0, 100.0),
            DataNodeContent::new(NodeValues::U64(vec![2]), DataFormat::Hexadecimal),
        )
        .map_err(|e| e.to_string())?;
    let bad_layout =
        DataNodeContent::new(NodeValues::U64(vec![1, 2]), DataFormat::Hexadecimal).with_layout(GridLayout::Columns(0));

    check(
        matches!(scene.measure_operator_box("XOR", &bad_layout), Err(Error::InvalidGridLayout(_))),
        "measure_operator_box did not reject Columns(0) with Err(Error::InvalidGridLayout(_))",
    )?;
    check(
        matches!(
            scene.add_binary_operator_node(Point::new(200.0, 50.0), BinaryOperator::Xor, (a, b), bad_layout),
            Err(Error::InvalidGridLayout(_))
        ),
        "add_binary_operator_node did not reject the same layout with Err(Error::InvalidGridLayout(_))",
    )
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Acceptance test 9 (weak form; see the design proposal's own caveat). Two `Scene`s, each with no special CSS applied,
/// measure the same content to the same size, independently. Nothing about measuring in one `Scene` leaks into or
/// depends on shared global state.
#[wasm_bindgen_test]
fn two_scenes_measure_the_same_content_independently_and_consistently() -> Result<(), String> {
    let svg_a = make_svg("measure-two-scenes-a", Size::new(400.0, 260.0), Size::new(400.0, 260.0));
    let scene_a = Scene::new(svg_a).map_err(|e| e.to_string())?;
    let svg_b = make_svg("measure-two-scenes-b", Size::new(400.0, 260.0), Size::new(400.0, 260.0));
    let scene_b = Scene::new(svg_b).map_err(|e| e.to_string())?;
    let content = DataNodeContent::new(NodeValues::U64(vec![0x0102_0304_0506_0708]), DataFormat::Hexadecimal);

    let size_a = scene_a.measure_data_node(&content).map_err(|e| e.to_string())?;
    let size_b = scene_b.measure_data_node(&content).map_err(|e| e.to_string())?;

    assert_size_exact(size_a, size_b)
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Exact equality for the content features that change a node's own size: row labels, column groups, an `Ascii` or
/// `PlainText` format, and a name wider than the content.
#[wasm_bindgen_test]
fn measure_matches_nodes_with_labels_groups_and_other_formats() -> Result<(), String> {
    let svg = make_svg("measure-features", Size::new(900.0, 900.0), Size::new(900.0, 900.0));
    let scene = Scene::new(svg).map_err(|e| e.to_string())?;
    let contents = [
        DataNodeContent::new(NodeValues::U32(vec![1; 8]), DataFormat::Binary)
            .with_layout(GridLayout::Columns(1))
            .with_labelling_style(LabellingStyle::Alphabetic),
        DataNodeContent::new(NodeValues::U8((0..40).collect()), DataFormat::Decimal)
            .with_layout(GridLayout::Columns(8))
            .with_column_groups(4)
            .with_labels(),
        DataNodeContent::new(NodeValues::U8(b"hello world".to_vec()), DataFormat::Ascii),
        DataNodeContent::new(NodeValues::U8(b"hello world".to_vec()), DataFormat::PlainText),
    ];
    for (i, content) in contents.into_iter().enumerate() {
        for name in [None, Some("A rather long name for a small node, to be the wider of the two")] {
            let measured = match name {
                Some(name) => scene.measure_named_data_node(name, &content),
                None => scene.measure_data_node(&content),
            }
            .map_err(|e| e.to_string())?;
            let id = match name {
                Some(name) => scene.add_named_data_node(Point::new(10.0, 10.0), name, content.clone()),
                None => scene.add_data_node(Point::new(10.0, 10.0), content.clone()),
            }
            .map_err(|e| e.to_string())?;
            let actual = scene.node_rect(id).map_err(|e| e.to_string())?.size;
            assert_size_exact(measured, actual).map_err(|e| format!("content {i}, named {}: {e}", name.is_some()))?;
        }
    }
    Ok(())
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Measuring builds no cell grid. However many values the content holds, only the handful of elements measuring needs
/// is ever added to the document. A grid drawn for real adds two elements per cell.
#[wasm_bindgen_test]
fn measuring_a_large_grid_adds_only_a_few_elements_to_the_document() -> Result<(), String> {
    let svg = make_svg("measure-cost", Size::new(400.0, 260.0), Size::new(400.0, 260.0));
    let scene = Scene::new(svg).map_err(|e| e.to_string())?;
    let root = web_sys::window()
        .and_then(|w| w.document())
        .and_then(|d| d.get_element_by_id("measure-cost"))
        .ok_or("no svg root")?;

    let ignore = wasm_bindgen::closure::Closure::wrap(Box::new(|_: js_sys::Array, _: web_sys::MutationObserver| {})
        as Box<dyn FnMut(js_sys::Array, web_sys::MutationObserver)>);
    let observer = web_sys::MutationObserver::new(ignore.as_ref().unchecked_ref()).map_err(|e| format!("{e:?}"))?;
    let options = web_sys::MutationObserverInit::new();
    options.set_child_list(true);
    options.set_subtree(true);
    observer.observe_with_options(&root, &options).map_err(|e| format!("{e:?}"))?;

    let content = DataNodeContent::new(NodeValues::U16((0..1000).collect()), DataFormat::Decimal)
        .with_layout(GridLayout::Columns(10))
        .with_labels();
    scene.measure_named_data_node("Big", &content).map_err(|e| e.to_string())?;

    let added: u32 = observer
        .take_records()
        .iter()
        .map(|r| r.unchecked_into::<web_sys::MutationRecord>().added_nodes().length())
        .sum();
    observer.disconnect();
    check(
        added <= 10,
        &format!("measuring 1000 values added {added} elements; a drawn grid would add over 2000"),
    )
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Measuring an operator box draws no boxes and no cells: only its two measuring texts are ever added to the document,
/// and none stays. Drawing the same box adds its group, two rectangles and two texts.
#[wasm_bindgen_test]
fn measuring_an_operator_box_adds_only_its_two_texts_and_leaves_nothing() -> Result<(), String> {
    let svg = make_svg("measure-operator-cost", Size::new(400.0, 260.0), Size::new(400.0, 260.0));
    let scene = Scene::new(svg).map_err(|e| e.to_string())?;
    let root = web_sys::window()
        .and_then(|w| w.document())
        .and_then(|d| d.get_element_by_id("measure-operator-cost"))
        .ok_or("no svg root")?;
    let before = descendant_element_count(&root)?;

    let ignore = wasm_bindgen::closure::Closure::wrap(Box::new(|_: js_sys::Array, _: web_sys::MutationObserver| {})
        as Box<dyn FnMut(js_sys::Array, web_sys::MutationObserver)>);
    let observer = web_sys::MutationObserver::new(ignore.as_ref().unchecked_ref()).map_err(|e| format!("{e:?}"))?;
    let options = web_sys::MutationObserverInit::new();
    options.set_child_list(true);
    options.set_subtree(true);
    observer.observe_with_options(&root, &options).map_err(|e| format!("{e:?}"))?;

    let result = DataNodeContent::new(NodeValues::U32(vec![7]), DataFormat::Hexadecimal);
    scene.measure_operator_box("XOR", &result).map_err(|e| e.to_string())?;

    let added: u32 = observer
        .take_records()
        .iter()
        .map(|r| r.unchecked_into::<web_sys::MutationRecord>().added_nodes().length())
        .sum();
    observer.disconnect();
    check(
        added <= 3,
        &format!("expected a group and two texts at most, {added} elements were added"),
    )?;
    check(descendant_element_count(&root)? == before, "measuring left elements behind")
}
