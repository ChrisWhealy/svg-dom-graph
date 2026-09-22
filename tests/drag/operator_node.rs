//! `Scene::add_unary_operator_node`/`add_binary_operator_node`: a node labelled with the operation that produced its
//! single value, wired with an auto-drawn edge from each of its operand(s). Covers rendering (label row plus an
//! inset value cell), the auto-wired input edge(s), operand-type validation, and dragging.

use crate::common::{
    attr_f64, check, check_close, connector_count, dispatch_pointer_event, group_translate, make_svg, nth_group,
};
use svg_dom::root::utils::{Point, Size};
use svg_dom_graph::{
    Error,
    scene::{
        ArithmeticOperator, BinaryOperator, DataFormat, DataNodeContent, EdgeAnchors, NodeOptions, NodeValues, Scene,
        UnaryOperator,
    },
};
use wasm_bindgen::JsCast;
use wasm_bindgen_test::wasm_bindgen_test;

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
fn text_children(group: &web_sys::Element) -> Result<Vec<web_sys::Element>, String> {
    elements_matching(group, "text")
}

fn rect_children(group: &web_sys::Element) -> Result<Vec<web_sys::Element>, String> {
    elements_matching(group, "rect")
}

fn elements_matching(group: &web_sys::Element, selector: &str) -> Result<Vec<web_sys::Element>, String> {
    let nodes = group.query_selector_all(selector).map_err(|e| format!("{e:?}"))?;
    let mut out = Vec::with_capacity(nodes.length() as usize);
    for i in 0..nodes.length() {
        let el = nodes
            .get(i)
            .ok_or("query_selector_all reported a length longer than it could actually return")?
            .dyn_into::<web_sys::Element>()
            .map_err(|_| format!("{selector} is not an Element"))?;
        out.push(el);
    }
    Ok(out)
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// A unary operator node renders a label row naming the operator, and its single result value in its own inset
/// cell, coloured by the result's own type — one outer rect plus one value-cell rect, one label text plus one
/// value text.
#[wasm_bindgen_test]
fn a_unary_operator_node_renders_its_label_and_value_rows() -> Result<(), String> {
    let svg = make_svg("operator-unary", Size::new(400.0, 260.0), Size::new(400.0, 260.0));
    let scene = Scene::new(svg).map_err(|e| e.to_string())?;
    let operand = scene
        .add_data_node(
            Point::new(10.0, 10.0),
            DataNodeContent::new(NodeValues::U32(vec![0x0F0F_0F0F]), DataFormat::Hexadecimal),
        )
        .map_err(|e| e.to_string())?;
    let result = DataNodeContent::new(NodeValues::U32(vec![!0x0F0F_0F0Fu32]), DataFormat::Hexadecimal);
    scene
        .add_unary_operator_node(Point::new(200.0, 10.0), UnaryOperator::Not, operand, result)
        .map_err(|e| e.to_string())?;

    let group = nth_group("operator-unary", 1)?;
    let rects = rect_children(&group)?;
    let texts = text_children(&group)?;
    check(
        rects.len() == 2,
        &format!("expected 1 outer + 1 value-cell rect, found {}", rects.len()),
    )?;
    check(
        texts.len() == 2,
        &format!("expected 1 label + 1 value text, found {}", texts.len()),
    )?;
    check(
        texts[0].text_content().as_deref() == Some("NOT"),
        &format!("expected the label row to read \"NOT\", got {:?}", texts[0].text_content()),
    )?;
    check(
        texts[1].text_content().as_deref() == Some("F0 F0 F0 F0"),
        &format!(
            "expected the value row to read \"F0 F0 F0 F0\", got {:?}",
            texts[1].text_content()
        ),
    )
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// A binary operator node auto-wires an edge from each of its two operands — no separate `add_edge` call is
/// needed, or even possible to get wrong.
#[wasm_bindgen_test]
fn a_binary_operator_node_auto_wires_both_input_edges() -> Result<(), String> {
    let svg = make_svg("operator-binary", Size::new(400.0, 260.0), Size::new(400.0, 260.0));
    let scene = Scene::new(svg).map_err(|e| e.to_string())?;
    let a = scene
        .add_data_node(
            Point::new(10.0, 10.0),
            DataNodeContent::new(NodeValues::U16(vec![0xFF00]), DataFormat::Hexadecimal),
        )
        .map_err(|e| e.to_string())?;
    let b = scene
        .add_data_node(
            Point::new(10.0, 120.0),
            DataNodeContent::new(NodeValues::U16(vec![0x00FF]), DataFormat::Hexadecimal),
        )
        .map_err(|e| e.to_string())?;
    let result = DataNodeContent::new(NodeValues::U16(vec![0xFFFF]), DataFormat::Hexadecimal);
    scene
        .add_binary_operator_node(Point::new(220.0, 60.0), BinaryOperator::Xor, (a, b), result)
        .map_err(|e| e.to_string())?;

    check(
        connector_count("operator-binary")? == 2,
        &format!(
            "expected 2 auto-wired connectors, found {}",
            connector_count("operator-binary")?
        ),
    )
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The value cell drawn inside an operator node's own box never touches that box's own left, right, or bottom
/// edge. A connector's own anchor point is always somewhere on the outer box's own perimeter — never on some
/// inner sub-region — so an inset value cell keeps every such anchor visually attached to the "named operation"
/// box, rather than looking like it terminates at the result cell instead.
#[wasm_bindgen_test]
fn operator_node_value_cell_is_inset_from_every_outer_edge() -> Result<(), String> {
    let svg = make_svg("operator-value-cell-inset", Size::new(400.0, 260.0), Size::new(400.0, 260.0));
    let scene = Scene::new(svg).map_err(|e| e.to_string())?;
    let a = scene
        .add_data_node(
            Point::new(10.0, 10.0),
            DataNodeContent::new(NodeValues::U32(vec![0xFF00_FF00]), DataFormat::Hexadecimal),
        )
        .map_err(|e| e.to_string())?;
    let b = scene
        .add_data_node(
            Point::new(10.0, 120.0),
            DataNodeContent::new(NodeValues::U32(vec![0x0F0F_0F0F]), DataFormat::Hexadecimal),
        )
        .map_err(|e| e.to_string())?;
    let result = DataNodeContent::new(NodeValues::U32(vec![0xFF00_FF00 & 0x0F0F_0F0F]), DataFormat::Hexadecimal);
    scene
        .add_binary_operator_node(Point::new(220.0, 60.0), BinaryOperator::And, (a, b), result)
        .map_err(|e| e.to_string())?;

    let group = nth_group("operator-value-cell-inset", 2)?;
    let rects = rect_children(&group)?;
    let outer = &rects[0];
    let value_cell = &rects[1];

    let outer_width = attr_f64(outer, "width")?;
    let outer_height = attr_f64(outer, "height")?;
    let cell_x = attr_f64(value_cell, "x")?;
    let cell_y = attr_f64(value_cell, "y")?;
    let cell_width = attr_f64(value_cell, "width")?;
    let cell_height = attr_f64(value_cell, "height")?;

    check(
        cell_x > 0.0,
        &format!("expected the value cell's left edge inset from the outer box's own, got x={cell_x}"),
    )?;
    check(
        cell_x + cell_width < outer_width,
        &format!(
            "expected the value cell's right edge inset from the outer box's own, got {} against an outer width of \
             {outer_width}",
            cell_x + cell_width
        ),
    )?;
    check(
        cell_y + cell_height < outer_height,
        &format!(
            "expected the value cell's bottom edge inset from the outer box's own, got {} against an outer height \
             of {outer_height}",
            cell_y + cell_height
        ),
    )
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// An arithmetic operator node renders and auto-wires exactly like a [`BinaryOperator`] one — same box shape, same
/// two auto-wired input edges — via the two-input operator construction the two share.
#[wasm_bindgen_test]
fn an_arithmetic_operator_node_auto_wires_both_input_edges() -> Result<(), String> {
    let svg = make_svg("operator-arithmetic", Size::new(400.0, 260.0), Size::new(400.0, 260.0));
    let scene = Scene::new(svg).map_err(|e| e.to_string())?;
    let dividend = scene
        .add_data_node(
            Point::new(10.0, 10.0),
            DataNodeContent::new(NodeValues::U8(vec![17]), DataFormat::Decimal),
        )
        .map_err(|e| e.to_string())?;
    let divisor = scene
        .add_data_node(
            Point::new(10.0, 120.0),
            DataNodeContent::new(NodeValues::U8(vec![5]), DataFormat::Decimal),
        )
        .map_err(|e| e.to_string())?;
    let result = DataNodeContent::new(NodeValues::U8(vec![17 % 5]), DataFormat::Decimal);
    scene
        .add_arithmetic_operator_node(
            Point::new(220.0, 60.0),
            ArithmeticOperator::Modulus,
            (dividend, divisor),
            result,
        )
        .map_err(|e| e.to_string())?;

    let group = nth_group("operator-arithmetic", 2)?;
    let texts = text_children(&group)?;
    check(
        texts[0].text_content().as_deref() == Some("MOD"),
        &format!("expected the label row to read \"MOD\", got {:?}", texts[0].text_content()),
    )?;
    check(
        texts[1].text_content().as_deref() == Some("2"),
        &format!("expected the value row to read \"2\", got {:?}", texts[1].text_content()),
    )?;
    check(
        connector_count("operator-arithmetic")? == 2,
        &format!(
            "expected 2 auto-wired connectors, found {}",
            connector_count("operator-arithmetic")?
        ),
    )
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Both operands to an arithmetic operator node must share one [`NodeValues`] width, exactly like a
/// [`BinaryOperator`] node's — validated by the same shared construction path.
#[wasm_bindgen_test]
fn add_arithmetic_operator_node_rejects_mismatched_operand_widths() -> Result<(), String> {
    let svg = make_svg(
        "operator-arithmetic-type-mismatch",
        Size::new(400.0, 260.0),
        Size::new(400.0, 260.0),
    );
    let scene = Scene::new(svg).map_err(|e| e.to_string())?;
    let a = scene
        .add_data_node(
            Point::new(10.0, 10.0),
            DataNodeContent::new(NodeValues::U8(vec![1]), DataFormat::Decimal),
        )
        .map_err(|e| e.to_string())?;
    let b = scene
        .add_data_node(
            Point::new(10.0, 120.0),
            DataNodeContent::new(NodeValues::U16(vec![1]), DataFormat::Decimal),
        )
        .map_err(|e| e.to_string())?;
    let result = DataNodeContent::new(NodeValues::U8(vec![1]), DataFormat::Decimal);

    let outcome =
        scene.add_arithmetic_operator_node(Point::new(220.0, 60.0), ArithmeticOperator::Modulus, (a, b), result);
    check(
        matches!(outcome, Err(Error::OperatorTypeMismatch { .. })),
        &format!("expected Err(Error::OperatorTypeMismatch {{ .. }}), got {outcome:?}"),
    )
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Both operands to a binary operator node must share one [`NodeValues`] width. A `u8` and a `u16` operand are
/// rejected before anything is drawn or wired.
#[wasm_bindgen_test]
fn add_binary_operator_node_rejects_mismatched_operand_widths() -> Result<(), String> {
    let svg = make_svg("operator-type-mismatch", Size::new(400.0, 260.0), Size::new(400.0, 260.0));
    let scene = Scene::new(svg).map_err(|e| e.to_string())?;
    let a = scene
        .add_data_node(
            Point::new(10.0, 10.0),
            DataNodeContent::new(NodeValues::U8(vec![1]), DataFormat::Decimal),
        )
        .map_err(|e| e.to_string())?;
    let b = scene
        .add_data_node(
            Point::new(10.0, 120.0),
            DataNodeContent::new(NodeValues::U16(vec![1]), DataFormat::Decimal),
        )
        .map_err(|e| e.to_string())?;
    let result = DataNodeContent::new(NodeValues::U8(vec![1]), DataFormat::Decimal);

    let outcome = scene.add_binary_operator_node(Point::new(220.0, 60.0), BinaryOperator::And, (a, b), result);
    check(
        matches!(outcome, Err(Error::OperatorTypeMismatch { .. })),
        &format!("expected Err(Error::OperatorTypeMismatch {{ .. }}), got {outcome:?}"),
    )
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// A binary operator's own two operands must be distinct nodes — combining a node with itself has no second
/// "other side" to route a connector to, and the router could never tell its two auto-wired edges apart anyway.
#[wasm_bindgen_test]
fn add_binary_operator_node_rejects_duplicate_operands() -> Result<(), String> {
    let svg = make_svg("operator-duplicate-operands", Size::new(400.0, 260.0), Size::new(400.0, 260.0));
    let scene = Scene::new(svg).map_err(|e| e.to_string())?;
    let a = scene
        .add_data_node(
            Point::new(10.0, 10.0),
            DataNodeContent::new(NodeValues::U8(vec![1]), DataFormat::Decimal),
        )
        .map_err(|e| e.to_string())?;
    let result = DataNodeContent::new(NodeValues::U8(vec![1]), DataFormat::Decimal);

    let outcome = scene.add_binary_operator_node(Point::new(220.0, 60.0), BinaryOperator::And, (a, a), result);
    check(
        matches!(outcome, Err(Error::DuplicateOperands(id)) if id == a),
        &format!("expected Err(Error::DuplicateOperands(a)), got {outcome:?}"),
    )
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// An operator's operand must itself be a [`DataNodeContent`] node — a plain label node has no value width an
/// operator could act on.
#[wasm_bindgen_test]
fn add_unary_operator_node_rejects_a_label_node_as_its_operand() -> Result<(), String> {
    let svg = make_svg("operator-operand-not-data", Size::new(400.0, 260.0), Size::new(400.0, 260.0));
    let scene = Scene::new(svg).map_err(|e| e.to_string())?;
    let label = scene
        .add_node(Point::new(10.0, 10.0), Size::new(90.0, 50.0), "Not data")
        .map_err(|e| e.to_string())?;
    let result = DataNodeContent::new(NodeValues::U8(vec![0]), DataFormat::Decimal);

    let outcome = scene.add_unary_operator_node(Point::new(220.0, 10.0), UnaryOperator::Not, label, result);
    check(
        matches!(outcome, Err(Error::OperandNotData(_))),
        &format!("expected Err(Error::OperandNotData(_)), got {outcome:?}"),
    )
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// An operator always produces exactly one value, never a grid of them.
#[wasm_bindgen_test]
fn add_binary_operator_node_rejects_a_multi_value_result() -> Result<(), String> {
    let svg = make_svg("operator-result-not-single", Size::new(400.0, 260.0), Size::new(400.0, 260.0));
    let scene = Scene::new(svg).map_err(|e| e.to_string())?;
    let a = scene
        .add_data_node(
            Point::new(10.0, 10.0),
            DataNodeContent::new(NodeValues::U8(vec![1]), DataFormat::Decimal),
        )
        .map_err(|e| e.to_string())?;
    let b = scene
        .add_data_node(
            Point::new(10.0, 120.0),
            DataNodeContent::new(NodeValues::U8(vec![2]), DataFormat::Decimal),
        )
        .map_err(|e| e.to_string())?;
    let result = DataNodeContent::new(NodeValues::U8(vec![1, 2]), DataFormat::Decimal);

    let outcome = scene.add_binary_operator_node(Point::new(220.0, 60.0), BinaryOperator::Or, (a, b), result);
    check(
        matches!(outcome, Err(Error::OperatorResultNotSingleValue(2))),
        &format!("expected Err(Error::OperatorResultNotSingleValue(2)), got {outcome:?}"),
    )
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Dragging an operator node moves its own box, exactly like any other node, and its auto-wired input connector
/// reroutes to follow it.
#[wasm_bindgen_test]
fn dragging_an_operator_node_moves_it_and_reroutes_its_input_connector() -> Result<(), String> {
    let svg = make_svg("operator-drag", Size::new(400.0, 260.0), Size::new(400.0, 260.0));
    let scene = Scene::new(svg).map_err(|e| e.to_string())?;
    let operand = scene
        .add_data_node(
            Point::new(10.0, 10.0),
            DataNodeContent::new(NodeValues::U8(vec![0b1010_1010]), DataFormat::Binary),
        )
        .map_err(|e| e.to_string())?;
    let result = DataNodeContent::new(NodeValues::U8(vec![0b0101_0101]), DataFormat::Binary);
    let op = scene
        .add_unary_operator_node(Point::new(220.0, 60.0), UnaryOperator::Not, operand, result)
        .map_err(|e| e.to_string())?;
    scene.make_draggable(op).map_err(|e| e.to_string())?;

    let group = nth_group("operator-drag", 1)?;
    let path_before = crate::common::path_d(&crate::common::the_connector("operator-drag")?)?;
    let group_xy_before = group_translate(&group)?;

    dispatch_pointer_event(&group, "pointerdown", 100, 100, 1)?;
    dispatch_pointer_event(&group, "pointermove", 130, 140, 1)?;
    dispatch_pointer_event(&group, "pointerup", 130, 140, 1)?;

    let (group_x_after, group_y_after) = group_translate(&group)?;
    check_close(group_x_after, group_xy_before.0 + 30.0)?;
    check_close(group_y_after, group_xy_before.1 + 40.0)?;

    let path_after = crate::common::path_d(&crate::common::the_connector("operator-drag")?)?;
    check(
        path_after != path_before,
        "expected the auto-wired connector's path to change after dragging the operator node",
    )?;

    // Every child stays in local coordinates — only the group's own transform moved.
    let rects = rect_children(&group)?;
    check(
        attr_f64(&rects[0], "x")? == 0.0 && attr_f64(&rects[0], "y")? == 0.0,
        "expected the outer rect's local origin to stay at (0, 0)",
    )
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Two operands both above a binary operator node land on two distinct points on its own north side, not the one
/// shared midpoint a single operand there would use.
#[wasm_bindgen_test]
fn two_operands_above_a_binary_operator_node_split_to_distinct_points() -> Result<(), String> {
    let svg = make_svg("operator-anchor-split", Size::new(500.0, 400.0), Size::new(500.0, 400.0));
    let scene = Scene::new(svg).map_err(|e| e.to_string())?;
    let a = scene
        .add_data_node(
            Point::new(140.0, 10.0),
            DataNodeContent::new(NodeValues::U8(vec![1]), DataFormat::Decimal),
        )
        .map_err(|e| e.to_string())?;
    let b = scene
        .add_data_node(
            Point::new(320.0, 10.0),
            DataNodeContent::new(NodeValues::U8(vec![2]), DataFormat::Decimal),
        )
        .map_err(|e| e.to_string())?;
    let result = DataNodeContent::new(NodeValues::U8(vec![3]), DataFormat::Decimal);
    scene
        .add_binary_operator_node(Point::new(220.0, 300.0), BinaryOperator::Or, (a, b), result)
        .map_err(|e| e.to_string())?;

    let end_a = crate::common::last_point_of_path(&crate::common::path_d(&crate::common::nth_connector(
        "operator-anchor-split",
        0,
    )?)?)?;
    let end_b = crate::common::last_point_of_path(&crate::common::path_d(&crate::common::nth_connector(
        "operator-anchor-split",
        1,
    )?)?)?;

    check_close(end_a.1, end_b.1)?;
    check(
        (end_a.0 - end_b.0).abs() > 1.0,
        &format!("expected two distinct anchor x positions, got {end_a:?} and {end_b:?}"),
    )
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Dragging a binary operator node itself — not either of its own operands — reroutes both of its own auto-wired
/// input connectors, and the same-side split the two operands above it share stays intact at the new position.
///
/// `SceneInner::move_node` handles this case specially: `id` (the dragged node) is itself the binary operator, so
/// both its own input edges are redrawn together via `redraw_binary_operator_inputs`, rather than via two separate
/// `incident_edges` iterations that would each independently recompute the shared pair geometry.
#[wasm_bindgen_test]
fn dragging_a_binary_operator_node_reroutes_and_keeps_the_split_on_both_its_input_connectors() -> Result<(), String> {
    let svg = make_svg("operator-drag-binary", Size::new(500.0, 500.0), Size::new(500.0, 500.0));
    let scene = Scene::new(svg).map_err(|e| e.to_string())?;
    let a = scene
        .add_data_node(
            Point::new(140.0, 10.0),
            DataNodeContent::new(NodeValues::U8(vec![1]), DataFormat::Decimal),
        )
        .map_err(|e| e.to_string())?;
    let b = scene
        .add_data_node(
            Point::new(320.0, 10.0),
            DataNodeContent::new(NodeValues::U8(vec![2]), DataFormat::Decimal),
        )
        .map_err(|e| e.to_string())?;
    let result = DataNodeContent::new(NodeValues::U8(vec![3]), DataFormat::Decimal);
    let op = scene
        .add_binary_operator_node(Point::new(220.0, 300.0), BinaryOperator::Or, (a, b), result)
        .map_err(|e| e.to_string())?;
    scene.make_draggable(op).map_err(|e| e.to_string())?;

    let path_a_before = crate::common::path_d(&crate::common::nth_connector("operator-drag-binary", 0)?)?;
    let path_b_before = crate::common::path_d(&crate::common::nth_connector("operator-drag-binary", 1)?)?;

    let op_group = nth_group("operator-drag-binary", 2)?;
    dispatch_pointer_event(&op_group, "pointerdown", 100, 100, 1)?;
    dispatch_pointer_event(&op_group, "pointermove", 150, 250, 1)?;
    dispatch_pointer_event(&op_group, "pointerup", 150, 250, 1)?;

    let path_a_after = crate::common::path_d(&crate::common::nth_connector("operator-drag-binary", 0)?)?;
    let path_b_after = crate::common::path_d(&crate::common::nth_connector("operator-drag-binary", 1)?)?;
    check(
        path_a_after != path_a_before,
        "expected a's own connector to reroute after dragging the operator node",
    )?;
    check(
        path_b_after != path_b_before,
        "expected b's own connector to reroute after dragging the operator node",
    )?;

    // Both operands are still above the operator's new position — the same-side split
    // `two_operands_above_a_binary_operator_node_split_to_distinct_points` proves at creation must survive moving
    // the operator itself, not just moving an operand.
    let end_a = crate::common::last_point_of_path(&path_a_after)?;
    let end_b = crate::common::last_point_of_path(&path_b_after)?;
    check_close(end_a.1, end_b.1)?;
    check(
        (end_a.0 - end_b.0).abs() > 1.0,
        &format!("expected two distinct anchor x positions after dragging the operator, got {end_a:?} and {end_b:?}"),
    )
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Two operands on *different* sides of a binary operator node each still keep today's plain midpoint — this
/// feature only ever changes anything when both inputs actually collide on the same side.
#[wasm_bindgen_test]
fn operands_on_different_sides_of_a_binary_operator_node_keep_the_plain_midpoint() -> Result<(), String> {
    let svg = make_svg("operator-anchor-no-split", Size::new(500.0, 400.0), Size::new(500.0, 400.0));
    let scene = Scene::new(svg).map_err(|e| e.to_string())?;
    let above = scene
        .add_data_node(
            Point::new(200.0, 10.0),
            DataNodeContent::new(NodeValues::U8(vec![1]), DataFormat::Decimal),
        )
        .map_err(|e| e.to_string())?;
    let beside = scene
        .add_data_node(
            Point::new(450.0, 300.0),
            DataNodeContent::new(NodeValues::U8(vec![2]), DataFormat::Decimal),
        )
        .map_err(|e| e.to_string())?;
    let result = DataNodeContent::new(NodeValues::U8(vec![3]), DataFormat::Decimal);
    scene
        .add_binary_operator_node(Point::new(220.0, 300.0), BinaryOperator::Or, (above, beside), result)
        .map_err(|e| e.to_string())?;

    let op_group = nth_group("operator-anchor-no-split", 2)?;
    let (op_x, op_y) = group_translate(&op_group)?;
    let outer_rect = rect_children(&op_group)?
        .into_iter()
        .next()
        .ok_or("operator node has no outer rect")?;
    let centre_x = op_x + attr_f64(&outer_rect, "width")? / 2.0;
    let centre_y = op_y + attr_f64(&outer_rect, "height")? / 2.0;

    let end_above = crate::common::last_point_of_path(&crate::common::path_d(&crate::common::nth_connector(
        "operator-anchor-no-split",
        0,
    )?)?)?;
    let end_beside = crate::common::last_point_of_path(&crate::common::path_d(&crate::common::nth_connector(
        "operator-anchor-no-split",
        1,
    )?)?)?;

    // The operand straight above lands on the north side's own plain midpoint: centred horizontally.
    check_close(end_above.0, centre_x)?;
    // The operand to the side lands on the east/west side's own plain midpoint: centred vertically.
    check_close(end_beside.1, centre_y)
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Dragging one operand from a side of its own onto its sibling's side re-splits both connectors live, not just
/// once at creation.
#[wasm_bindgen_test]
fn dragging_an_operand_onto_its_siblings_side_re_splits_both_connectors_live() -> Result<(), String> {
    let svg = make_svg("operator-anchor-live-split", Size::new(500.0, 400.0), Size::new(500.0, 400.0));
    let scene = Scene::new(svg).map_err(|e| e.to_string())?;
    let a = scene
        .add_data_node(
            Point::new(140.0, 10.0),
            DataNodeContent::new(NodeValues::U8(vec![1]), DataFormat::Decimal),
        )
        .map_err(|e| e.to_string())?;
    let b = scene
        .add_data_node(
            Point::new(450.0, 300.0),
            DataNodeContent::new(NodeValues::U8(vec![2]), DataFormat::Decimal),
        )
        .map_err(|e| e.to_string())?;
    scene.make_draggable(b).map_err(|e| e.to_string())?;
    let result = DataNodeContent::new(NodeValues::U8(vec![3]), DataFormat::Decimal);
    scene
        .add_binary_operator_node(Point::new(220.0, 300.0), BinaryOperator::Or, (a, b), result)
        .map_err(|e| e.to_string())?;

    // Before: `b` sits beside the operator, so `a`'s own connector alone occupies the north side's plain midpoint.
    let end_a_before = crate::common::last_point_of_path(&crate::common::path_d(&crate::common::nth_connector(
        "operator-anchor-live-split",
        0,
    )?)?)?;

    // Drag `b` to sit above the operator too, joining `a` on the same (north) side.
    let b_group = nth_group("operator-anchor-live-split", 1)?;
    dispatch_pointer_event(&b_group, "pointerdown", 100, 100, 1)?;
    dispatch_pointer_event(&b_group, "pointermove", -50, -190, 1)?;
    dispatch_pointer_event(&b_group, "pointerup", -50, -190, 1)?;

    let end_a_after = crate::common::last_point_of_path(&crate::common::path_d(&crate::common::nth_connector(
        "operator-anchor-live-split",
        0,
    )?)?)?;
    let end_b_after = crate::common::last_point_of_path(&crate::common::path_d(&crate::common::nth_connector(
        "operator-anchor-live-split",
        1,
    )?)?)?;

    check(
        end_a_after != end_a_before,
        "expected a's own connector to move off the shared midpoint once b joined it on the same side",
    )?;
    check_close(end_a_after.1, end_b_after.1)?;
    check(
        (end_a_after.0 - end_b_after.0).abs() > 1.0,
        &format!("expected two distinct anchor x positions after the drag, got {end_a_after:?} and {end_b_after:?}"),
    )
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Every `M`/`L` point a path's own `d` attribute visits, in order — the corner-rounding arcs this file never
/// exercises (every test here uses `ConnectorOptions::default`'s own sharp `corner_radius: 0.0`) would need their
/// own parsing, which this deliberately does not attempt.
fn all_points(d: &str) -> Result<Vec<(f64, f64)>, String> {
    let mut numbers = Vec::new();
    for token in d.split_whitespace() {
        if token.chars().next().is_some_and(char::is_alphabetic) {
            continue;
        }
        numbers.push(
            token
                .parse::<f64>()
                .map_err(|e| format!("token {token:?} in path {d:?} did not parse as f64: {e}"))?,
        );
    }
    Ok(numbers.chunks_exact(2).map(|pair| (pair[0], pair[1])).collect())
}

/// `true` if axis-aligned segments `a1`-`a2` and `b1`-`b2` touch anywhere. Every route this crate ever produces is
/// Manhattan (each segment purely horizontal or purely vertical), so plain bounding-box overlap is a complete,
/// exact intersection test here — see the matching helper in `src/geometry/unit_tests.rs`.
fn segments_touch(a1: (f64, f64), a2: (f64, f64), b1: (f64, f64), b2: (f64, f64)) -> bool {
    let (a_min_x, a_max_x) = (a1.0.min(a2.0), a1.0.max(a2.0));
    let (a_min_y, a_max_y) = (a1.1.min(a2.1), a1.1.max(a2.1));
    let (b_min_x, b_max_x) = (b1.0.min(b2.0), b1.0.max(b2.0));
    let (b_min_y, b_max_y) = (b1.1.min(b2.1), b1.1.max(b2.1));
    a_min_x <= b_max_x && b_min_x <= a_max_x && a_min_y <= b_max_y && b_min_y <= a_max_y
}

fn check_routes_do_not_cross(a: &[(f64, f64)], b: &[(f64, f64)]) -> Result<(), String> {
    for pair_a in a.windows(2) {
        for pair_b in b.windows(2) {
            if segments_touch(pair_a[0], pair_a[1], pair_b[0], pair_b[1]) {
                return Err(format!(
                    "routes cross: {a:?}'s segment {pair_a:?} touches {b:?}'s segment {pair_b:?}"
                ));
            }
        }
    }
    Ok(())
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Dragging the *near* operand of a same-side binary operator pair down past the *far* operand's own anchor row —
/// the concrete repro for this report — must not leave the two connectors crossing.
#[wasm_bindgen_test]
fn dragging_the_near_operand_past_the_far_operands_row_does_not_cross_the_connectors() -> Result<(), String> {
    let svg = make_svg("operator-anchor-no-cross", Size::new(600.0, 500.0), Size::new(600.0, 500.0));
    let scene = Scene::new(svg).map_err(|e| e.to_string())?;
    let near = scene
        .add_data_node(
            Point::new(140.0, 10.0),
            DataNodeContent::new(NodeValues::U8(vec![1]), DataFormat::Decimal),
        )
        .map_err(|e| e.to_string())?;
    let far = scene
        .add_data_node(
            Point::new(140.0, 300.0),
            DataNodeContent::new(NodeValues::U8(vec![2]), DataFormat::Decimal),
        )
        .map_err(|e| e.to_string())?;
    scene.make_draggable(near).map_err(|e| e.to_string())?;
    let result = DataNodeContent::new(NodeValues::U8(vec![3]), DataFormat::Decimal);
    scene
        .add_binary_operator_node(Point::new(400.0, 150.0), BinaryOperator::Or, (near, far), result)
        .map_err(|e| e.to_string())?;

    // Read the operator's own real rendered geometry back, so the drag target below is worked out from its actual
    // candidate rows rather than a guessed pixel offset.
    let op_group = nth_group("operator-anchor-no-cross", 2)?;
    let (_, op_y) = group_translate(&op_group)?;
    let outer_rect = rect_children(&op_group)?
        .into_iter()
        .next()
        .ok_or("operator node has no outer rect")?;
    let op_height = attr_f64(&outer_rect, "height")?;
    let far_target_y = op_y + op_height * 3.0 / 4.0;

    // Drag `near` well past `far`'s own target row — the exact "one input moved too far" scenario reported.
    let near_group = nth_group("operator-anchor-no-cross", 0)?;
    let (near_x_before, near_y_before) = group_translate(&near_group)?;
    let drag_dy = far_target_y - near_y_before + 80.0;
    dispatch_pointer_event(&near_group, "pointerdown", 100, 100, 1)?;
    dispatch_pointer_event(&near_group, "pointermove", 100, (100.0 + drag_dy) as i32, 1)?;
    dispatch_pointer_event(&near_group, "pointerup", 100, (100.0 + drag_dy) as i32, 1)?;
    check(
        group_translate(&near_group)?.1 > far_target_y,
        &format!(
            "expected the drag to move `near` past y = {far_target_y}, got {:?} (started at {near_x_before}, \
             {near_y_before})",
            group_translate(&near_group)?
        ),
    )?;

    let near_route = all_points(&crate::common::path_d(&crate::common::nth_connector(
        "operator-anchor-no-cross",
        0,
    )?)?)?;
    let far_route = all_points(&crate::common::path_d(&crate::common::nth_connector(
        "operator-anchor-no-cross",
        1,
    )?)?)?;
    check_routes_do_not_cross(&near_route, &far_route)
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The real, rendered centre of the `index`th `<g>` under `container_id` — `group_translate`'s own origin plus
/// half of its outer `<rect>`'s own measured size.
fn group_centre(container_id: &str, index: u32) -> Result<(f64, f64), String> {
    let group = nth_group(container_id, index)?;
    let (x, y) = group_translate(&group)?;
    let outer_rect = rect_children(&group)?
        .into_iter()
        .next()
        .ok_or_else(|| format!("group {index} under #{container_id} has no outer rect"))?;
    Ok((
        x + attr_f64(&outer_rect, "width")? / 2.0,
        y + attr_f64(&outer_rect, "height")? / 2.0,
    ))
}

/// Drags the `index`th `<g>` under `container_id` so its own centre ends up at `target`.
fn drag_group_centre_to(container_id: &str, index: u32, target: (f64, f64)) -> Result<(), String> {
    let group = nth_group(container_id, index)?;
    let (from_x, from_y) = group_centre(container_id, index)?;
    let (dx, dy) = (target.0 - from_x, target.1 - from_y);
    let end = (100.0 + dx, 100.0 + dy);
    dispatch_pointer_event(&group, "pointerdown", 100, 100, 1)?;
    dispatch_pointer_event(&group, "pointermove", end.0 as i32, end.1 as i32, 1)?;
    dispatch_pointer_event(&group, "pointerup", end.0 as i32, end.1 as i32, 1)
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Two distinct operands on the exact same ray from a binary operator's own centre — different nodes, but an
/// identical crossing position on its shared side — must still land on two distinct anchor points, not one on top
/// of the other. Comparing the crossings alone cannot tell them apart; the fix is a stable first/second identity.
#[wasm_bindgen_test]
fn two_distinct_operands_with_an_identical_crossing_do_not_overlap() -> Result<(), String> {
    let svg = make_svg("operator-equal-crossing", Size::new(700.0, 500.0), Size::new(700.0, 500.0));
    let scene = Scene::new(svg).map_err(|e| e.to_string())?;
    let near = scene
        .add_data_node(
            Point::new(10.0, 10.0),
            DataNodeContent::new(NodeValues::U8(vec![1]), DataFormat::Decimal),
        )
        .map_err(|e| e.to_string())?;
    let far = scene
        .add_data_node(
            Point::new(10.0, 120.0),
            DataNodeContent::new(NodeValues::U8(vec![2]), DataFormat::Decimal),
        )
        .map_err(|e| e.to_string())?;
    scene.make_draggable(near).map_err(|e| e.to_string())?;
    scene.make_draggable(far).map_err(|e| e.to_string())?;
    let result = DataNodeContent::new(NodeValues::U8(vec![3]), DataFormat::Decimal);
    scene
        .add_binary_operator_node(Point::new(500.0, 260.0), BinaryOperator::Or, (near, far), result)
        .map_err(|e| e.to_string())?;

    // Drag both operands onto the exact same ray from the operator's own centre — direction (-100, -1), at t = 1
    // and t = 2 — so their crossing positions on the operator's own west side are identical (the crossing formula
    // is `centre + dy * half_w / dx`, which depends only on the ray's own direction, not how far along it a point
    // sits).
    let (op_cx, op_cy) = group_centre("operator-equal-crossing", 2)?;
    drag_group_centre_to("operator-equal-crossing", 0, (op_cx - 100.0, op_cy - 1.0))?;
    drag_group_centre_to("operator-equal-crossing", 1, (op_cx - 200.0, op_cy - 2.0))?;

    let end_near = crate::common::last_point_of_path(&crate::common::path_d(&crate::common::nth_connector(
        "operator-equal-crossing",
        0,
    )?)?)?;
    let end_far = crate::common::last_point_of_path(&crate::common::path_d(&crate::common::nth_connector(
        "operator-equal-crossing",
        1,
    )?)?)?;

    // Both land on the operator's own west edge (same x) — but at two distinct y positions, not one on top of
    // the other.
    check_close(end_near.0, end_far.0)?;
    check(
        (end_near.1 - end_far.1).abs() > 1.0,
        &format!(
            "expected two distinct anchor y positions despite identical crossings, got {end_near:?} and {end_far:?}"
        ),
    )
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Same shape as `two_operands_above_a_binary_operator_node_split_to_distinct_points`, but the operator node has
/// `EdgeAnchors(5)` configured. The split must land on the outer two of the node's own *five* configured
/// candidates, not the unconfigured default's fixed outer-two-of-three — otherwise `EdgeAnchors` would be silently
/// ignored for a binary operator's own two automatically-wired operand edges, even though it is honoured for every
/// other edge into the same node.
#[wasm_bindgen_test]
fn two_operands_above_a_binary_operator_node_split_to_its_configured_fixing_points() -> Result<(), String> {
    let svg = make_svg(
        "operator-anchor-split-configured",
        Size::new(500.0, 400.0),
        Size::new(500.0, 400.0),
    );
    let scene = Scene::new(svg).map_err(|e| e.to_string())?;
    let a = scene
        .add_data_node(
            Point::new(140.0, 10.0),
            DataNodeContent::new(NodeValues::U8(vec![1]), DataFormat::Decimal),
        )
        .map_err(|e| e.to_string())?;
    let b = scene
        .add_data_node(
            Point::new(320.0, 10.0),
            DataNodeContent::new(NodeValues::U8(vec![2]), DataFormat::Decimal),
        )
        .map_err(|e| e.to_string())?;
    let result = DataNodeContent::new(NodeValues::U8(vec![3]), DataFormat::Decimal);
    scene
        .add_binary_operator_node_with(
            Point::new(220.0, 300.0),
            BinaryOperator::Or,
            (a, b),
            result,
            NodeOptions::default().with_edge_anchors(Some(EdgeAnchors(5))),
        )
        .map_err(|e| e.to_string())?;

    let op_group = nth_group("operator-anchor-split-configured", 2)?;
    let (op_x, op_y) = group_translate(&op_group)?;
    let outer_rect = rect_children(&op_group)?
        .into_iter()
        .next()
        .ok_or("operator node has no outer rect")?;
    let op_width = attr_f64(&outer_rect, "width")?;

    let end_a = crate::common::last_point_of_path(&crate::common::path_d(&crate::common::nth_connector(
        "operator-anchor-split-configured",
        0,
    )?)?)?;
    let end_b = crate::common::last_point_of_path(&crate::common::path_d(&crate::common::nth_connector(
        "operator-anchor-split-configured",
        1,
    )?)?)?;

    check_close(end_a.1, op_y)?;
    check_close(end_b.1, op_y)?;
    // Outer two of five candidates on a side divided into six equal segments: 1/6 and 5/6 of the side's width —
    // not 1/4 and 3/4, which is what the unconfigured default's outer-two-of-three split would give.
    check_close(end_a.0, op_x + op_width / 6.0)?;
    check_close(end_b.0, op_x + op_width * 5.0 / 6.0)
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Same shape again, but with `EdgeAnchors(1)` configured. A single candidate has no second position to split to,
/// so both operands land on the same point — the side's own plain midpoint, exactly like `EdgeAnchors(1)` on any
/// other node. This is `EdgeAnchors`' own documented "not reserved, nothing stops two connectors sharing a point"
/// contract playing out for a binary operator's own two operands, not a bug: see `binary_operator_anchor`'s own
/// doc comment for why `None` (not `Some(1)`) is what still gets the outer-two-of-three split.
#[wasm_bindgen_test]
fn two_operands_above_a_binary_operator_node_with_one_configured_fixing_point_share_it() -> Result<(), String> {
    let svg = make_svg(
        "operator-anchor-split-one-anchor",
        Size::new(500.0, 400.0),
        Size::new(500.0, 400.0),
    );
    let scene = Scene::new(svg).map_err(|e| e.to_string())?;
    let a = scene
        .add_data_node(
            Point::new(140.0, 10.0),
            DataNodeContent::new(NodeValues::U8(vec![1]), DataFormat::Decimal),
        )
        .map_err(|e| e.to_string())?;
    let b = scene
        .add_data_node(
            Point::new(320.0, 10.0),
            DataNodeContent::new(NodeValues::U8(vec![2]), DataFormat::Decimal),
        )
        .map_err(|e| e.to_string())?;
    let result = DataNodeContent::new(NodeValues::U8(vec![3]), DataFormat::Decimal);
    scene
        .add_binary_operator_node_with(
            Point::new(220.0, 300.0),
            BinaryOperator::Or,
            (a, b),
            result,
            NodeOptions::default().with_edge_anchors(Some(EdgeAnchors(1))),
        )
        .map_err(|e| e.to_string())?;

    let op_group = nth_group("operator-anchor-split-one-anchor", 2)?;
    let (op_x, op_y) = group_translate(&op_group)?;
    let outer_rect = rect_children(&op_group)?
        .into_iter()
        .next()
        .ok_or("operator node has no outer rect")?;
    let op_width = attr_f64(&outer_rect, "width")?;

    let end_a = crate::common::last_point_of_path(&crate::common::path_d(&crate::common::nth_connector(
        "operator-anchor-split-one-anchor",
        0,
    )?)?)?;
    let end_b = crate::common::last_point_of_path(&crate::common::path_d(&crate::common::nth_connector(
        "operator-anchor-split-one-anchor",
        1,
    )?)?)?;

    check_close(end_a.1, op_y)?;
    check_close(end_b.1, op_y)?;
    check_close(end_a.0, op_x + op_width / 2.0)?;
    check_close(end_b.0, op_x + op_width / 2.0)
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Same shape again, but with `EdgeAnchors(2)` configured — the minimal case where the two operands actually do
/// split apart: both of the node's own two candidates are used, with no middle one to skip.
#[wasm_bindgen_test]
fn two_operands_above_a_binary_operator_node_with_two_configured_fixing_points_use_both() -> Result<(), String> {
    let svg = make_svg(
        "operator-anchor-split-two-anchors",
        Size::new(500.0, 400.0),
        Size::new(500.0, 400.0),
    );
    let scene = Scene::new(svg).map_err(|e| e.to_string())?;
    let a = scene
        .add_data_node(
            Point::new(140.0, 10.0),
            DataNodeContent::new(NodeValues::U8(vec![1]), DataFormat::Decimal),
        )
        .map_err(|e| e.to_string())?;
    let b = scene
        .add_data_node(
            Point::new(320.0, 10.0),
            DataNodeContent::new(NodeValues::U8(vec![2]), DataFormat::Decimal),
        )
        .map_err(|e| e.to_string())?;
    let result = DataNodeContent::new(NodeValues::U8(vec![3]), DataFormat::Decimal);
    scene
        .add_binary_operator_node_with(
            Point::new(220.0, 300.0),
            BinaryOperator::Or,
            (a, b),
            result,
            NodeOptions::default().with_edge_anchors(Some(EdgeAnchors(2))),
        )
        .map_err(|e| e.to_string())?;

    let op_group = nth_group("operator-anchor-split-two-anchors", 2)?;
    let (op_x, op_y) = group_translate(&op_group)?;
    let outer_rect = rect_children(&op_group)?
        .into_iter()
        .next()
        .ok_or("operator node has no outer rect")?;
    let op_width = attr_f64(&outer_rect, "width")?;

    let end_a = crate::common::last_point_of_path(&crate::common::path_d(&crate::common::nth_connector(
        "operator-anchor-split-two-anchors",
        0,
    )?)?)?;
    let end_b = crate::common::last_point_of_path(&crate::common::path_d(&crate::common::nth_connector(
        "operator-anchor-split-two-anchors",
        1,
    )?)?)?;

    check_close(end_a.1, op_y)?;
    check_close(end_b.1, op_y)?;
    // Two candidates on a side divided into three equal segments: 1/3 and 2/3 of the side's width.
    check_close(end_a.0, op_x + op_width / 3.0)?;
    check_close(end_b.0, op_x + op_width * 2.0 / 3.0)
}
