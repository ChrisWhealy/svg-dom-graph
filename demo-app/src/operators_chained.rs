//! `panel-operators-chained` / `#operators-chained-diagram`: operator nodes feeding further operator nodes, so an
//! operator's own [`DataNodeContent`] result is used as another operator's operand — see
//! [`build_chained_operator_demo`]'s own doc comment.

use crate::util::{stringify, view_box_rect};
use std::cell::RefCell;
use svg_dom::{SvgRoot, root::utils::Point};
use svg_dom_graph::{
    NodeId,
    scene::{BinaryOperator, DataFormat, DataNodeContent, DragOptions, NodeValues, Scene, UnaryOperator},
};

/// This module's own full source, embedded at compile time — see `crate::source_frame`'s own doc comment for why.
pub(crate) const SOURCE: &str = include_str!("operators_chained.rs");

thread_local! {
    // Same reasoning as `tree::SCENE`'s own doc comment, for this demo's own, separate `Scene`.
    static SCENE: RefCell<Option<Scene>> = const { RefCell::new(None) };
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Builds the demo scene: three examples of an operator node's own result feeding a further operator node, rather
/// than every row standing alone the way [`crate::operators_unary`]/[`crate::operators_binary`]'s rows do.
///
/// Every result shown is computed right here, with plain Rust integer operators. `svg_dom_graph` itself never
/// evaluates an operator — see [`Scene::add_binary_operator_node`]'s own doc comment for why — so this function's
/// job is exactly the one a real caller would have: compute each stage's real value, then hand it to the library
/// alongside the operator that produced it. Feeding one operator node's id back in as another's operand is nothing
/// special to the library — its own [`DataNodeContent`] result is a valid operand like any other data node's.
///
/// Three chains, each one stage deeper than the last:
///
/// - `B` rotated right by one bit, then XOR'ed with `A` — the two-step chain from the feature request that started
///   this whole panel.
/// - `XOR(w0, AND(NOT(w1), w2))` for three `u32` values — a three-stage chain built from a `NOT` feeding an `AND`
///   feeding an `XOR`.
/// - SHA-256's own "Choose" function, `Ch(x, y, z) = (x AND y) XOR (NOT(x) AND z)` — `x` feeds two different
///   operator nodes (its own `NOT`, and the `AND` with `y`), so this row also demonstrates one operand feeding more
///   than one operator.
///
/// Every leaf operand — never an intermediate operator's own result, which already carries the operator's own
/// label — is a [`Scene::add_named_data_node`] node, labelled with its own variable name from the prose above
/// (`"A"`/`"B"`, `"w0"`/`"w1"`/`"w2"`, `"x"`/`"y"`/`"z"`), so the rendered diagram reads directly against the
/// formula it draws.
///
/// # Errors
///
/// Returns `Err` if any library call fails, or if `index.html` is missing `#operators-chained-diagram`.
pub(crate) fn build_chained_operator_demo() -> Result<(), String> {
    let svg = SvgRoot::attach("operators-chained-diagram").map_err(stringify)?;
    let bounds = view_box_rect(&svg)?;
    let scene = Scene::new(svg).map_err(stringify)?;
    let drag_options = DragOptions::default().with_bounds(Some(bounds));

    // Every chain's leftmost operand column shares this x; a unary stage sits one column over, and each further
    // stage moves one more column to the right to make room for the operator node it feeds.
    const X_OPERAND: f64 = 20.0;
    const X_UNARY: f64 = 260.0;
    const X_STAGE_2: f64 = 500.0;
    const X_STAGE_3: f64 = 740.0;

    let place_operand = |x: f64, y: f64, name: &str, content: DataNodeContent| -> Result<NodeId, String> {
        let node = scene.add_named_data_node(Point::new(x, y), name, content).map_err(stringify)?;
        scene.make_draggable_with(node, drag_options).map_err(stringify)?;
        Ok(node)
    };

    // Chain 1 — `B` rotated right by one bit, then XOR'ed with `A`. The ROR node's own result feeds the XOR node
    // as its second operand.
    let a: u64 = 0x0123_4567_89AB_CDEF;
    let b: u64 = 0xFEDC_BA98_7654_3210;
    let b_node = place_operand(
        X_OPERAND,
        20.0,
        "B",
        DataNodeContent::new(NodeValues::U64(vec![b]), DataFormat::Hexadecimal),
    )?;
    let ror_b = b.rotate_right(1);
    let ror_b_node = scene
        .add_unary_operator_node(
            Point::new(X_UNARY, 20.0),
            UnaryOperator::RotateRight(1),
            b_node,
            DataNodeContent::new(NodeValues::U64(vec![ror_b]), DataFormat::Hexadecimal),
        )
        .map_err(stringify)?;
    scene.make_draggable_with(ror_b_node, drag_options).map_err(stringify)?;

    let a_node = place_operand(
        X_OPERAND,
        140.0,
        "A",
        DataNodeContent::new(NodeValues::U64(vec![a]), DataFormat::Hexadecimal),
    )?;
    let chain1_result = scene
        .add_binary_operator_node(
            Point::new(X_STAGE_2, 80.0),
            BinaryOperator::Xor,
            (a_node, ror_b_node),
            DataNodeContent::new(NodeValues::U64(vec![a ^ ror_b]), DataFormat::Hexadecimal),
        )
        .map_err(stringify)?;
    scene.make_draggable_with(chain1_result, drag_options).map_err(stringify)?;

    // Chain 2 — `XOR(w0, AND(NOT(w1), w2))`. `NOT(w1)` feeds the `AND` as its first operand, and that `AND`'s own
    // result feeds the final `XOR` as its second operand.
    let w0: u32 = 0x1234_5678;
    let w1: u32 = 0x0F0F_0F0F;
    let w2: u32 = 0xFF00_FF00;

    let w1_node = place_operand(
        X_OPERAND,
        280.0,
        "w1",
        DataNodeContent::new(NodeValues::U32(vec![w1]), DataFormat::Hexadecimal),
    )?;
    let not_w1 = !w1;
    let not_w1_node = scene
        .add_unary_operator_node(
            Point::new(X_UNARY, 280.0),
            UnaryOperator::Not,
            w1_node,
            DataNodeContent::new(NodeValues::U32(vec![not_w1]), DataFormat::Hexadecimal),
        )
        .map_err(stringify)?;
    scene.make_draggable_with(not_w1_node, drag_options).map_err(stringify)?;

    let w2_node = place_operand(
        X_OPERAND,
        400.0,
        "w2",
        DataNodeContent::new(NodeValues::U32(vec![w2]), DataFormat::Hexadecimal),
    )?;
    let and_val = not_w1 & w2;
    let and_node = scene
        .add_binary_operator_node(
            Point::new(X_STAGE_2, 340.0),
            BinaryOperator::And,
            (not_w1_node, w2_node),
            DataNodeContent::new(NodeValues::U32(vec![and_val]), DataFormat::Hexadecimal),
        )
        .map_err(stringify)?;
    scene.make_draggable_with(and_node, drag_options).map_err(stringify)?;

    let w0_node = place_operand(
        X_OPERAND,
        520.0,
        "w0",
        DataNodeContent::new(NodeValues::U32(vec![w0]), DataFormat::Hexadecimal),
    )?;
    let chain2_result = scene
        .add_binary_operator_node(
            Point::new(X_STAGE_3, 430.0),
            BinaryOperator::Xor,
            (w0_node, and_node),
            DataNodeContent::new(NodeValues::U32(vec![w0 ^ and_val]), DataFormat::Hexadecimal),
        )
        .map_err(stringify)?;
    scene.make_draggable_with(chain2_result, drag_options).map_err(stringify)?;

    // Chain 3 — SHA-256's own "Choose" function: `Ch(x, y, z) = (x AND y) XOR (NOT(x) AND z)`. `x` feeds two
    // separate operator nodes (its own `NOT`, and the `AND` with `y`) — one operand, more than one operator.
    //
    // `NOT(x)` sits in the operand column, directly below `x`, rather than in the usual unary-operator column
    // beside it. `x`'s other outgoing edge, to `AND(x, y)`, leaves `x`'s east side — putting `NOT(x)` there too
    // would put it directly in that edge's path, exactly the leftover-single-operator-page column scheme this
    // three-way fan-out doesn't fit.
    //
    // `x` sits directly above `NOT(x)`, with `y` above `x` rather than between them — so `x`'s own straight drop
    // to `NOT(x)` has no other node's box in its way.
    let x: u32 = 0xAAAA_AAAA;
    let y: u32 = 0xCCCC_CCCC;
    let z: u32 = 0xF0F0_F0F0;

    let y_node = place_operand(
        X_OPERAND,
        660.0,
        "y",
        DataNodeContent::new(NodeValues::U32(vec![y]), DataFormat::Hexadecimal),
    )?;
    let x_node = place_operand(
        X_OPERAND,
        750.0,
        "x",
        DataNodeContent::new(NodeValues::U32(vec![x]), DataFormat::Hexadecimal),
    )?;
    let not_x = !x;
    let not_x_node = scene
        .add_unary_operator_node(
            Point::new(X_OPERAND, 860.0),
            UnaryOperator::Not,
            x_node,
            DataNodeContent::new(NodeValues::U32(vec![not_x]), DataFormat::Hexadecimal),
        )
        .map_err(stringify)?;
    scene.make_draggable_with(not_x_node, drag_options).map_err(stringify)?;

    let z_node = place_operand(
        X_OPERAND,
        970.0,
        "z",
        DataNodeContent::new(NodeValues::U32(vec![z]), DataFormat::Hexadecimal),
    )?;

    let and_xy = x & y;
    let and_xy_node = scene
        .add_binary_operator_node(
            Point::new(X_STAGE_2, 700.0),
            BinaryOperator::And,
            (x_node, y_node),
            DataNodeContent::new(NodeValues::U32(vec![and_xy]), DataFormat::Hexadecimal),
        )
        .map_err(stringify)?;
    scene.make_draggable_with(and_xy_node, drag_options).map_err(stringify)?;

    // A clear vertical gap from `and_xy_node` above — otherwise the two `AND` nodes, stacked in the same column,
    // read as one combined node rather than two independent stages feeding the final `XOR`.
    let and_notx_z = not_x & z;
    let and_notx_z_node = scene
        .add_binary_operator_node(
            Point::new(X_STAGE_2, 910.0),
            BinaryOperator::And,
            (not_x_node, z_node),
            DataNodeContent::new(NodeValues::U32(vec![and_notx_z]), DataFormat::Hexadecimal),
        )
        .map_err(stringify)?;
    scene.make_draggable_with(and_notx_z_node, drag_options).map_err(stringify)?;

    let chain3_result = scene
        .add_binary_operator_node(
            Point::new(X_STAGE_3, 805.0),
            BinaryOperator::Xor,
            (and_xy_node, and_notx_z_node),
            DataNodeContent::new(NodeValues::U32(vec![and_xy ^ and_notx_z]), DataFormat::Hexadecimal),
        )
        .map_err(stringify)?;
    scene.make_draggable_with(chain3_result, drag_options).map_err(stringify)?;

    // Keeps this Scene's only strong handle alive for the page's lifetime — see SCENE's own doc comment.
    SCENE.with_borrow_mut(|slot| *slot = Some(scene));

    Ok(())
}
