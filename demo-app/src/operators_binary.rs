//! `panel-operators-binary` / `#operators-binary-diagram`: two operand nodes feeding an operator node, for every
//! [`BinaryOperator`] this crate names. Every result shown is computed here with plain Rust integer ops, never by
//! the library itself — see [`build_binary_operator_demo`]'s own doc comment.

use crate::util::{stringify, view_box_rect};
use std::cell::RefCell;
use svg_dom::{SvgRoot, root::utils::Point};
use svg_dom_graph::{
    NodeId,
    scene::{BinaryOperator, DataFormat, DataNodeContent, DragOptions, NodeValues, Scene},
};

/// This module's own full source, embedded at compile time — see `crate::source_frame`'s own doc comment for why.
pub(crate) const SOURCE: &str = include_str!("operators_binary.rs");

thread_local! {
    // Same reasoning as `tree::SCENE`'s own doc comment, for this demo's own, separate `Scene`.
    static SCENE: RefCell<Option<Scene>> = const { RefCell::new(None) };
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Builds the demo scene: two operand nodes feeding an operator node, for every [`BinaryOperator`] this crate
/// names.
///
/// Each operand is a [`Scene::add_named_data_node`] node, labelled `"A"`/`"B"` — the raw value's own outer box,
/// wrapping its value cell, the same way an operator node's own outer box wraps its result — rather than a plain
/// [`Scene::add_data_node`] box.
///
/// Every result shown is computed right here, with plain Rust integer operators (`&`, `|`, `^`, and their own
/// complements). `svg_dom_graph` itself never evaluates an operator — see [`Scene::add_binary_operator_node`]'s
/// own doc comment for why — so this function's job is exactly the one a real caller would have: compute the real
/// value, then hand it to the library alongside the operator that produced it.
///
/// See [`crate::operators_chained::build_chained_operator_demo`] for operator nodes feeding further operator
/// nodes — an operator's own [`DataNodeContent`] result is a valid operand like any other data node's, which this
/// page's six rows don't show on their own.
///
/// # Errors
///
/// Returns `Err` if any library call fails, or if `index.html` is missing `#operators-binary-diagram`.
pub(crate) fn build_binary_operator_demo() -> Result<(), String> {
    let svg = SvgRoot::attach("operators-binary-diagram").map_err(stringify)?;
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

    // AND — u32, two operands stacked in one column, feeding one operator node between them.
    let and_a: u32 = 0xFF00_FF00;
    let and_b: u32 = 0x0F0F_0F0F;
    let and_a_node = place_operand(
        X_OPERAND,
        20.0,
        "A",
        DataNodeContent::new(NodeValues::U32(vec![and_a]), DataFormat::Hexadecimal),
    )?;
    let and_b_node = place_operand(
        X_OPERAND,
        110.0,
        "B",
        DataNodeContent::new(NodeValues::U32(vec![and_b]), DataFormat::Hexadecimal),
    )?;
    let and_node = scene
        .add_binary_operator_node(
            Point::new(X_OPERATOR, 65.0),
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
        190.0,
        "A",
        DataNodeContent::new(NodeValues::U64(vec![or_a]), DataFormat::Hexadecimal),
    )?;
    let or_b_node = place_operand(
        X_OPERAND,
        280.0,
        "B",
        DataNodeContent::new(NodeValues::U64(vec![or_b]), DataFormat::Hexadecimal),
    )?;
    let or_node = scene
        .add_binary_operator_node(
            Point::new(X_OPERATOR, 235.0),
            BinaryOperator::Or,
            (or_a_node, or_b_node),
            DataNodeContent::new(NodeValues::U64(vec![or_a | or_b]), DataFormat::Hexadecimal),
        )
        .map_err(stringify)?;
    scene.make_draggable_with(or_node, drag_options).map_err(stringify)?;

    // NAND — u16, two operands stacked, same layout as AND/OR above.
    let nand_a: u16 = 0xFF0F;
    let nand_b: u16 = 0x0FFF;
    let nand_a_node = place_operand(
        X_OPERAND,
        360.0,
        "A",
        DataNodeContent::new(NodeValues::U16(vec![nand_a]), DataFormat::Hexadecimal),
    )?;
    let nand_b_node = place_operand(
        X_OPERAND,
        450.0,
        "B",
        DataNodeContent::new(NodeValues::U16(vec![nand_b]), DataFormat::Hexadecimal),
    )?;
    let nand_node = scene
        .add_binary_operator_node(
            Point::new(X_OPERATOR, 405.0),
            BinaryOperator::Nand,
            (nand_a_node, nand_b_node),
            DataNodeContent::new(NodeValues::U16(vec![!(nand_a & nand_b)]), DataFormat::Hexadecimal),
        )
        .map_err(stringify)?;
    scene.make_draggable_with(nand_node, drag_options).map_err(stringify)?;

    // NOR — u8, binary, so the negated-OR bit pattern reads digit by digit.
    let nor_a: u8 = 0b1010_0000;
    let nor_b: u8 = 0b0000_1010;
    let nor_a_node = place_operand(
        X_OPERAND,
        530.0,
        "A",
        DataNodeContent::new(NodeValues::U8(vec![nor_a]), DataFormat::Binary),
    )?;
    let nor_b_node = place_operand(
        X_OPERAND,
        620.0,
        "B",
        DataNodeContent::new(NodeValues::U8(vec![nor_b]), DataFormat::Binary),
    )?;
    let nor_node = scene
        .add_binary_operator_node(
            Point::new(X_OPERATOR, 575.0),
            BinaryOperator::Nor,
            (nor_a_node, nor_b_node),
            DataNodeContent::new(NodeValues::U8(vec![!(nor_a | nor_b)]), DataFormat::Binary),
        )
        .map_err(stringify)?;
    scene.make_draggable_with(nor_node, drag_options).map_err(stringify)?;

    // XNOR — u32, two complementary bit patterns: XOR would be all ones, so XNOR (its negation) is all zeros.
    let xnor_a: u32 = 0xAAAA_AAAA;
    let xnor_b: u32 = 0x5555_5555;
    let xnor_a_node = place_operand(
        X_OPERAND,
        700.0,
        "A",
        DataNodeContent::new(NodeValues::U32(vec![xnor_a]), DataFormat::Hexadecimal),
    )?;
    let xnor_b_node = place_operand(
        X_OPERAND,
        790.0,
        "B",
        DataNodeContent::new(NodeValues::U32(vec![xnor_b]), DataFormat::Hexadecimal),
    )?;
    let xnor_node = scene
        .add_binary_operator_node(
            Point::new(X_OPERATOR, 745.0),
            BinaryOperator::Xnor,
            (xnor_a_node, xnor_b_node),
            DataNodeContent::new(NodeValues::U32(vec![!(xnor_a ^ xnor_b)]), DataFormat::Hexadecimal),
        )
        .map_err(stringify)?;
    scene.make_draggable_with(xnor_node, drag_options).map_err(stringify)?;

    // Keeps this Scene's only strong handle alive for the page's lifetime — see SCENE's own doc comment.
    SCENE.with_borrow_mut(|slot| *slot = Some(scene));

    Ok(())
}
