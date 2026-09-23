//! `panel-operators-arithmetic` / `#operators-arithmetic-diagram`: two operand nodes feeding an operator node, for
//! every [`ArithmeticOperator`] this crate names. Every result shown is computed here with plain Rust integer ops,
//! never by the library itself — see [`build_arithmetic_operator_demo`]'s own doc comment.

use crate::util::{stringify, view_box_rect};
use std::cell::RefCell;
use svg_dom::{SvgRoot, root::utils::Point};
use svg_dom_graph::{
    NodeId,
    scene::{ArithmeticOperator, DataFormat, DataNodeContent, DragOptions, NodeValues, Scene},
};

/// This module's own full source, embedded at compile time — see `crate::source_frame`'s own doc comment for why.
pub(crate) const SOURCE: &str = include_str!("operators_arithmetic.rs");

thread_local! {
    // Same reasoning as `tree::SCENE`'s own doc comment, for this demo's own, separate `Scene`.
    static SCENE: RefCell<Option<Scene>> = const { RefCell::new(None) };
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Builds the demo scene: two operand nodes feeding an operator node, for every [`ArithmeticOperator`] this crate
/// names.
///
/// Each operand is a [`Scene::add_named_data_node`] node, labelled `"A"`/`"B"` — the raw value's own outer box,
/// wrapping its value cell, the same way an operator node's own outer box wraps its result — rather than a plain
/// [`Scene::add_data_node`] box. Naming `inputs.0`/`inputs.1` this way pairs naturally with the non-commutative
/// operators' own "L"/"R" port markers: `A` is always the left-hand operand, `B` the right-hand one.
///
/// Every result shown is computed right here, with plain Rust integer operators (`+`, `-`, `*`, `/`, `%`).
/// `svg_dom_graph` itself never evaluates an operator — see [`Scene::add_arithmetic_operator_node`]'s own doc
/// comment for why — so this function's job is exactly the one a real caller would have: compute the real value,
/// then hand it to the library alongside the operator that produced it.
///
/// Each row uses a different operand width, so between them all four this crate supports — `u8`, `u16`, `u32`,
/// `u64` — appear at least once. `Subtract` never underflows (the first operand is always `>=` the second), and
/// neither `Divide` nor `Modulus` ever divides by zero — both are the caller's own responsibility, never checked by
/// the library, since an operator node's own displayed value is always whatever the caller already computed. See
/// [`ArithmeticOperator`]'s own doc comment.
///
/// # Errors
///
/// Returns `Err` if any library call fails, or if `index.html` is missing `#operators-arithmetic-diagram`.
pub(crate) fn build_arithmetic_operator_demo() -> Result<(), String> {
    let svg = SvgRoot::attach("operators-arithmetic-diagram").map_err(stringify)?;
    let bounds = view_box_rect(&svg)?;
    let scene = Scene::new(svg).map_err(stringify)?;
    let drag_options = DragOptions::default().with_bounds(Some(bounds));

    // Every row's operand(s) share this left-hand x; every row's operator node shares this one, a fixed distance
    // to its right.
    const X_OPERAND: f64 = 20.0;
    const X_OPERATOR: f64 = 260.0;

    let place_operand = |x: f64, y: f64, name: &str, content: DataNodeContent| -> Result<NodeId, String> {
        let node = scene.add_named_data_node(Point::new(x, y), name, content).map_err(stringify)?;
        scene.make_draggable_with(node, drag_options).map_err(stringify)?;
        Ok(node)
    };

    // ADD — u8, two operands stacked in one column, feeding one operator node between them.
    let add_a: u8 = 180;
    let add_b: u8 = 45;
    let add_a_node = place_operand(
        X_OPERAND,
        20.0,
        "A",
        DataNodeContent::new(NodeValues::U8(vec![add_a]), DataFormat::Decimal),
    )?;
    let add_b_node = place_operand(
        X_OPERAND,
        110.0,
        "B",
        DataNodeContent::new(NodeValues::U8(vec![add_b]), DataFormat::Decimal),
    )?;
    let add_node = scene
        .add_arithmetic_operator_node(
            Point::new(X_OPERATOR, 65.0),
            ArithmeticOperator::Add,
            (add_a_node, add_b_node),
            DataNodeContent::new(NodeValues::U8(vec![add_a + add_b]), DataFormat::Decimal),
        )
        .map_err(stringify)?;
    scene.make_draggable_with(add_node, drag_options).map_err(stringify)?;

    // SUB — u16, two operands stacked, same layout as ADD above. The first operand is always the larger, so the
    // unsigned subtraction never underflows.
    let sub_a: u16 = 50_000;
    let sub_b: u16 = 12_345;
    let sub_a_node = place_operand(
        X_OPERAND,
        190.0,
        "A",
        DataNodeContent::new(NodeValues::U16(vec![sub_a]), DataFormat::Decimal),
    )?;
    let sub_b_node = place_operand(
        X_OPERAND,
        280.0,
        "B",
        DataNodeContent::new(NodeValues::U16(vec![sub_b]), DataFormat::Decimal),
    )?;
    let sub_node = scene
        .add_arithmetic_operator_node(
            Point::new(X_OPERATOR, 235.0),
            ArithmeticOperator::Subtract,
            (sub_a_node, sub_b_node),
            DataNodeContent::new(NodeValues::U16(vec![sub_a - sub_b]), DataFormat::Decimal),
        )
        .map_err(stringify)?;
    scene.make_draggable_with(sub_node, drag_options).map_err(stringify)?;

    // MUL — u32, two operands stacked, same layout as ADD/SUB above. Both factors are kept well below `u32::MAX`'s
    // own square root headroom, so the product cannot overflow.
    let mul_a: u32 = 50_000;
    let mul_b: u32 = 70_000;
    let mul_a_node = place_operand(
        X_OPERAND,
        360.0,
        "A",
        DataNodeContent::new(NodeValues::U32(vec![mul_a]), DataFormat::Decimal),
    )?;
    let mul_b_node = place_operand(
        X_OPERAND,
        450.0,
        "B",
        DataNodeContent::new(NodeValues::U32(vec![mul_b]), DataFormat::Decimal),
    )?;
    let mul_node = scene
        .add_arithmetic_operator_node(
            Point::new(X_OPERATOR, 405.0),
            ArithmeticOperator::Multiply,
            (mul_a_node, mul_b_node),
            DataNodeContent::new(NodeValues::U32(vec![mul_a * mul_b]), DataFormat::Decimal),
        )
        .map_err(stringify)?;
    scene.make_draggable_with(mul_node, drag_options).map_err(stringify)?;

    // DIV — u64, two operands stacked, same layout as ADD/SUB/MUL above. The divisor is never zero.
    let div_a: u64 = 100_000_000_000;
    let div_b: u64 = 4;
    let div_a_node = place_operand(
        X_OPERAND,
        530.0,
        "A",
        DataNodeContent::new(NodeValues::U64(vec![div_a]), DataFormat::Decimal),
    )?;
    let div_b_node = place_operand(
        X_OPERAND,
        620.0,
        "B",
        DataNodeContent::new(NodeValues::U64(vec![div_b]), DataFormat::Decimal),
    )?;
    let div_node = scene
        .add_arithmetic_operator_node(
            Point::new(X_OPERATOR, 575.0),
            ArithmeticOperator::Divide,
            (div_a_node, div_b_node),
            DataNodeContent::new(NodeValues::U64(vec![div_a / div_b]), DataFormat::Decimal),
        )
        .map_err(stringify)?;
    scene.make_draggable_with(div_node, drag_options).map_err(stringify)?;

    // MOD — u8 again, this time paired with a different divisor than ADD's own operands, so this row is not just a
    // visual repeat of the first one.
    let mod_a: u8 = 200;
    let mod_b: u8 = 7;
    let mod_a_node = place_operand(
        X_OPERAND,
        700.0,
        "A",
        DataNodeContent::new(NodeValues::U8(vec![mod_a]), DataFormat::Decimal),
    )?;
    let mod_b_node = place_operand(
        X_OPERAND,
        790.0,
        "B",
        DataNodeContent::new(NodeValues::U8(vec![mod_b]), DataFormat::Decimal),
    )?;
    let mod_node = scene
        .add_arithmetic_operator_node(
            Point::new(X_OPERATOR, 745.0),
            ArithmeticOperator::Modulus,
            (mod_a_node, mod_b_node),
            DataNodeContent::new(NodeValues::U8(vec![mod_a % mod_b]), DataFormat::Decimal),
        )
        .map_err(stringify)?;
    scene.make_draggable_with(mod_node, drag_options).map_err(stringify)?;

    // Keeps this Scene's only strong handle alive for the page's lifetime — see SCENE's own doc comment.
    SCENE.with_borrow_mut(|slot| *slot = Some(scene));

    Ok(())
}
