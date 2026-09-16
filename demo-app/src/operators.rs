//! `panel-operators` / `#operators-diagram`: one operand node (or two, for a binary operator) feeding an operator
//! node, for every unary and binary operator this crate names. Every result shown is computed here with plain Rust
//! integer ops, never by the library itself — see [`build_operator_demo`]'s own doc comment.

use crate::util::{stringify, view_box_rect};
use std::cell::RefCell;
use svg_dom::{SvgRoot, root::utils::Point};
use svg_dom_graph::{
    NodeId,
    scene::{BinaryOperator, DataFormat, DataNodeContent, DragOptions, NodeValues, Scene, UnaryOperator},
};

/// This module's own full source, embedded at compile time — see `crate::source_frame`'s own doc comment for why.
pub(crate) const SOURCE: &str = include_str!("operators.rs");

thread_local! {
    // Same reasoning as `tree::SCENE`'s own doc comment, for this demo's own, separate `Scene`.
    static SCENE: RefCell<Option<Scene>> = const { RefCell::new(None) };
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Builds the demo scene: one operand node (or two, for a binary operator) feeding an operator node, for every
/// [`UnaryOperator`]/[`BinaryOperator`] this crate names.
///
/// Every result shown is computed right here, with plain Rust integer operators (`!`, `<<`, `>>`, `rotate_left`,
/// `rotate_right`, `&`, `|`, `^`). `svg_dom_graph` itself never evaluates an operator — see
/// [`Scene::add_unary_operator_node`]'s own doc comment for why — so this function's job is exactly the one a real
/// caller would have: compute the real value, then hand it to the library alongside the operator that produced it.
///
/// The last row is the two-step chain from the feature request that started this: `B` rotated right by one bit,
/// then XOR'ed with `A`. It shows an operator node feeding a second operator node — its own [`DataNodeContent`]
/// result is a valid operand like any other data node's.
///
/// # Errors
///
/// Returns `Err` if any library call fails, or if `index.html` is missing `#operators-diagram`.
pub(crate) fn build_operator_demo() -> Result<(), String> {
    let svg = SvgRoot::attach("operators-diagram").map_err(stringify)?;
    let bounds = view_box_rect(&svg)?;
    let scene = Scene::new(svg).map_err(stringify)?;
    let drag_options = DragOptions::default().with_bounds(Some(bounds));

    // Every row's operand(s) share this left-hand x; every row's operator node shares one of these two, depending
    // on how far right it needs to sit to receive its own operand(s).
    const X_OPERAND: f64 = 20.0;
    const X_OPERATOR: f64 = 260.0;

    let place_operand = |x: f64, y: f64, content: DataNodeContent| -> Result<NodeId, String> {
        let node = scene.add_data_node(Point::new(x, y), content).map_err(stringify)?;
        scene.make_draggable_with(node, drag_options).map_err(stringify)?;
        Ok(node)
    };

    // NOT — u8, `DataFormat::Binary` so the bit flip reads digit by digit.
    let not_input: u8 = 0b1010_1010;
    let not_operand = place_operand(
        X_OPERAND,
        20.0,
        DataNodeContent::new(NodeValues::U8(vec![not_input]), DataFormat::Binary),
    )?;
    let not_node = scene
        .add_unary_operator_node(
            Point::new(X_OPERATOR, 20.0),
            UnaryOperator::Not,
            not_operand,
            DataNodeContent::new(NodeValues::U8(vec![!not_input]), DataFormat::Binary),
        )
        .map_err(stringify)?;
    scene.make_draggable_with(not_node, drag_options).map_err(stringify)?;

    // Shift left 3 — u16, hexadecimal.
    let shl_input: u16 = 0x00FF;
    let shl_operand = place_operand(
        X_OPERAND,
        130.0,
        DataNodeContent::new(NodeValues::U16(vec![shl_input]), DataFormat::Hexadecimal),
    )?;
    let shl_node = scene
        .add_unary_operator_node(
            Point::new(X_OPERATOR, 130.0),
            UnaryOperator::ShiftLeft(3),
            shl_operand,
            DataNodeContent::new(NodeValues::U16(vec![shl_input << 3]), DataFormat::Hexadecimal),
        )
        .map_err(stringify)?;
    scene.make_draggable_with(shl_node, drag_options).map_err(stringify)?;

    // Shift right 4 — u32, hexadecimal.
    let shr_input: u32 = 0xDEAD_BEEF;
    let shr_operand = place_operand(
        X_OPERAND,
        240.0,
        DataNodeContent::new(NodeValues::U32(vec![shr_input]), DataFormat::Hexadecimal),
    )?;
    let shr_node = scene
        .add_unary_operator_node(
            Point::new(X_OPERATOR, 240.0),
            UnaryOperator::ShiftRight(4),
            shr_operand,
            DataNodeContent::new(NodeValues::U32(vec![shr_input >> 4]), DataFormat::Hexadecimal),
        )
        .map_err(stringify)?;
    scene.make_draggable_with(shr_node, drag_options).map_err(stringify)?;

    // Rotate left 4 — u8, binary, so the nybble swap this particular rotate amount produces reads clearly.
    let rol_input: u8 = 0b0001_1010;
    let rol_operand = place_operand(
        X_OPERAND,
        350.0,
        DataNodeContent::new(NodeValues::U8(vec![rol_input]), DataFormat::Binary),
    )?;
    let rol_node = scene
        .add_unary_operator_node(
            Point::new(X_OPERATOR, 350.0),
            UnaryOperator::RotateLeft(4),
            rol_operand,
            DataNodeContent::new(NodeValues::U8(vec![rol_input.rotate_left(4)]), DataFormat::Binary),
        )
        .map_err(stringify)?;
    scene.make_draggable_with(rol_node, drag_options).map_err(stringify)?;

    // Rotate right 5 — u16, hexadecimal.
    let ror_input: u16 = 0x0007;
    let ror_operand = place_operand(
        X_OPERAND,
        460.0,
        DataNodeContent::new(NodeValues::U16(vec![ror_input]), DataFormat::Hexadecimal),
    )?;
    let ror_node = scene
        .add_unary_operator_node(
            Point::new(X_OPERATOR, 460.0),
            UnaryOperator::RotateRight(5),
            ror_operand,
            DataNodeContent::new(NodeValues::U16(vec![ror_input.rotate_right(5)]), DataFormat::Hexadecimal),
        )
        .map_err(stringify)?;
    scene.make_draggable_with(ror_node, drag_options).map_err(stringify)?;

    // AND — u32, two operands stacked in one column, feeding one operator node between them.
    let and_a: u32 = 0xFF00_FF00;
    let and_b: u32 = 0x0F0F_0F0F;
    let and_a_node = place_operand(
        X_OPERAND,
        570.0,
        DataNodeContent::new(NodeValues::U32(vec![and_a]), DataFormat::Hexadecimal),
    )?;
    let and_b_node = place_operand(
        X_OPERAND,
        660.0,
        DataNodeContent::new(NodeValues::U32(vec![and_b]), DataFormat::Hexadecimal),
    )?;
    let and_node = scene
        .add_binary_operator_node(
            Point::new(X_OPERATOR, 615.0),
            BinaryOperator::And,
            (and_a_node, and_b_node),
            DataNodeContent::new(NodeValues::U32(vec![and_a & and_b]), DataFormat::Hexadecimal),
        )
        .map_err(stringify)?;
    scene.make_draggable_with(and_node, drag_options).map_err(stringify)?;

    // OR — u64, two operands stacked, same layout as AND above.
    let or_a: u64 = 0x0000_0000_FFFF_0000;
    let or_b: u64 = 0x1234_5678_0000_ABCD;
    let or_a_node = place_operand(
        X_OPERAND,
        740.0,
        DataNodeContent::new(NodeValues::U64(vec![or_a]), DataFormat::Hexadecimal),
    )?;
    let or_b_node = place_operand(
        X_OPERAND,
        830.0,
        DataNodeContent::new(NodeValues::U64(vec![or_b]), DataFormat::Hexadecimal),
    )?;
    let or_node = scene
        .add_binary_operator_node(
            Point::new(X_OPERATOR, 785.0),
            BinaryOperator::Or,
            (or_a_node, or_b_node),
            DataNodeContent::new(NodeValues::U64(vec![or_a | or_b]), DataFormat::Hexadecimal),
        )
        .map_err(stringify)?;
    scene.make_draggable_with(or_node, drag_options).map_err(stringify)?;

    // XOR chain — the motivating example: B rotated right by one bit, then XOR'ed with A. The ROR node's own
    // result feeds the XOR node as its second operand, a further x-coordinate over to make room for it.
    let a: u64 = 0x0123_4567_89AB_CDEF;
    let b: u64 = 0xFEDC_BA98_7654_3210;
    let b_node = place_operand(
        X_OPERAND,
        910.0,
        DataNodeContent::new(NodeValues::U64(vec![b]), DataFormat::Hexadecimal),
    )?;
    let ror_b = b.rotate_right(1);
    let ror_b_node = scene
        .add_unary_operator_node(
            Point::new(X_OPERATOR, 910.0),
            UnaryOperator::RotateRight(1),
            b_node,
            DataNodeContent::new(NodeValues::U64(vec![ror_b]), DataFormat::Hexadecimal),
        )
        .map_err(stringify)?;
    scene.make_draggable_with(ror_b_node, drag_options).map_err(stringify)?;

    let a_node = place_operand(
        X_OPERAND,
        1030.0,
        DataNodeContent::new(NodeValues::U64(vec![a]), DataFormat::Hexadecimal),
    )?;
    let xor_node = scene
        .add_binary_operator_node(
            Point::new(500.0, 970.0),
            BinaryOperator::Xor,
            (a_node, ror_b_node),
            DataNodeContent::new(NodeValues::U64(vec![a ^ ror_b]), DataFormat::Hexadecimal),
        )
        .map_err(stringify)?;
    scene.make_draggable_with(xor_node, drag_options).map_err(stringify)?;

    // Keeps this Scene's only strong handle alive for the page's lifetime — see SCENE's own doc comment.
    SCENE.with_borrow_mut(|slot| *slot = Some(scene));

    Ok(())
}
