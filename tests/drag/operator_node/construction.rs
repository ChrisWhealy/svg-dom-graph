//! Rendering, auto-wiring, and operand/result validation for a unary, binary, or arithmetic operator node.

use crate::common::{attr_f64, check, connector_count, make_svg, nth_group};
use svg_dom::root::utils::{Point, Size};
use svg_dom_graph::{
    Error,
    scene::{ArithmeticOperator, BinaryOperator, DataFormat, DataNodeContent, NodeValues, Scene, UnaryOperator},
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

/// `<title>` child text, or `None` if `element` has no direct `<title>` child.
fn title_of(element: &web_sys::Element) -> Result<Option<String>, String> {
    Ok(element
        .query_selector(":scope > title")
        .map_err(|e| format!("{e:?}"))?
        .map(|title| title.text_content().unwrap_or_default()))
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// An operator node's own `aria-label` reads `"{label} result = {value}"` — the operator that produced it, and its
/// own real formatted result value as text, rather than just the result's own type. Its `<title>` — the browser's
/// own mouse-hover tooltip — carries that exact same text, so hovering shows the same thing a screen reader
/// announces.
#[wasm_bindgen_test]
fn an_operator_nodes_own_aria_label_names_the_operator_and_the_real_result_value() -> Result<(), String> {
    let svg = make_svg("operator-aria-label", Size::new(400.0, 260.0), Size::new(400.0, 260.0));
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

    let group = nth_group("operator-aria-label", 1)?;
    check(
        group.get_attribute("aria-label").as_deref() == Some("NOT result = F0 F0 F0 F0"),
        &format!("unexpected aria-label: {:?}", group.get_attribute("aria-label")),
    )?;
    check(
        title_of(&group)?.as_deref() == Some("NOT result = F0 F0 F0 F0"),
        &format!("unexpected <title>: {:?}", title_of(&group)?),
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
