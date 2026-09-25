//! `panel-operators-unary` / `#operators-unary-diagram`: one operand node feeding an operator node, for every
//! [`UnaryOperator`] this crate names. Every result shown is computed here with plain Rust integer ops, never by
//! the library itself — see [`build_unary_operator_demo`]'s own doc comment.

use crate::util::{stringify, view_box_rect};
use std::cell::RefCell;
use svg_dom::{SvgRoot, root::utils::Point};
use svg_dom_graph::{
    NodeId,
    scene::{DataFormat, DataNodeContent, DragOptions, NodeValues, Scene, Side, ToolbarOptions, UnaryOperator},
};

/// This module's own full source, embedded at compile time — see `crate::source_frame`'s own doc comment for why.
pub(crate) const SOURCE: &str = include_str!("operators_unary.rs");

thread_local! {
    // Same reasoning as `tree::SCENE`'s own doc comment, for this demo's own, separate `Scene`.
    static SCENE: RefCell<Option<Scene>> = const { RefCell::new(None) };
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Builds the demo scene: one operand node feeding an operator node, for every [`UnaryOperator`] this crate names.
///
/// Each operand is a [`Scene::add_named_data_node`] node, labelled `"A"` — the raw value's own outer box, wrapping
/// its value cell, the same way an operator node's own outer box wraps its result. Naming the operand this way,
/// rather than leaving it a plain [`Scene::add_data_node`] box, gives the reader an unambiguous handle for the
/// value the operator acts on.
///
/// Every result shown is computed right here, with plain Rust integer operators (`!`, `<<`, `>>`, `rotate_left`,
/// `rotate_right`, `reverse_bits`, `swap_bytes`). `svg_dom_graph` itself never evaluates an operator — see
/// [`Scene::add_unary_operator_node`]'s own doc comment for why — so this function's job is exactly the one a real
/// caller would have: compute the real value, then hand it to the library alongside the operator that produced it.
///
/// See [`crate::operators_binary::build_binary_operator_demo`] for this demo's own binary-operator counterpart, and
/// [`crate::operators_chained::build_chained_operator_demo`] for operator nodes feeding further operator nodes.
///
/// # Errors
///
/// Returns `Err` if any library call fails, or if `index.html` is missing `#operators-unary-diagram`.
pub(crate) fn build_unary_operator_demo() -> Result<(), String> {
    let svg = SvgRoot::attach("operators-unary-diagram").map_err(stringify)?;
    let bounds = view_box_rect(&svg)?;
    let scene = Scene::new(svg).map_err(stringify)?;
    let drag_options = DragOptions::default().with_bounds(Some(bounds));

    // Every row's operand shares this left-hand x; every row's operator node shares this one, a fixed distance to
    // its right.
    const X_OPERAND: f64 = 20.0;
    const X_OPERATOR: f64 = 260.0;

    let place_operand = |x: f64, y: f64, content: DataNodeContent| -> Result<NodeId, String> {
        let node = scene.add_named_data_node(Point::new(x, y), "A", content).map_err(stringify)?;
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

    // Reverse bits — u8, binary, so the end-to-end bit reversal reads digit by digit, the same reason NOT above
    // uses binary too.
    let rbit_input: u8 = 0b1100_0010;
    let rbit_operand = place_operand(
        X_OPERAND,
        570.0,
        DataNodeContent::new(NodeValues::U8(vec![rbit_input]), DataFormat::Binary),
    )?;
    let rbit_node = scene
        .add_unary_operator_node(
            Point::new(X_OPERATOR, 570.0),
            UnaryOperator::ReverseBits,
            rbit_operand,
            DataNodeContent::new(NodeValues::U8(vec![rbit_input.reverse_bits()]), DataFormat::Binary),
        )
        .map_err(stringify)?;
    scene.make_draggable_with(rbit_node, drag_options).map_err(stringify)?;

    // Swap bytes — u32, hexadecimal, so the reversed byte-group order reads clearly.
    let bswap_input: u32 = 0x1234_5678;
    let bswap_operand = place_operand(
        X_OPERAND,
        680.0,
        DataNodeContent::new(NodeValues::U32(vec![bswap_input]), DataFormat::Hexadecimal),
    )?;
    let bswap_node = scene
        .add_unary_operator_node(
            Point::new(X_OPERATOR, 680.0),
            UnaryOperator::SwapBytes,
            bswap_operand,
            DataNodeContent::new(NodeValues::U32(vec![bswap_input.swap_bytes()]), DataFormat::Hexadecimal),
        )
        .map_err(stringify)?;
    scene.make_draggable_with(bswap_node, drag_options).map_err(stringify)?;

    scene.show_toolbar(ToolbarOptions::new(Side::East)).map_err(stringify)?;

    // Keeps this Scene's only strong handle alive for the page's lifetime — see SCENE's own doc comment.
    SCENE.with_borrow_mut(|slot| *slot = Some(scene));

    Ok(())
}
