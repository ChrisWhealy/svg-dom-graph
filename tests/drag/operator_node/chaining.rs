//! An operator node's own result feeding a further operator node as one of its own operands.

use crate::common::{check, connector_count, dispatch_pointer_event, make_svg, nth_group};
use svg_dom::root::utils::{Point, Size};
use svg_dom_graph::scene::{BinaryOperator, DataFormat, DataNodeContent, NodeValues, Scene};
use wasm_bindgen_test::wasm_bindgen_test;

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// A chain — `A`/`B` feed `XOR`, whose own result feeds `AND` alongside `C` — auto-wires all four edges from a plain
/// data node's own value, an operator node's own single-value result, or a mix of both. `add_binary_operator_node`
/// makes no distinction: `XOR`'s own [`NodeId`] is just as valid an operand as any [`DataNodeContent`] node's own,
/// since an operator node's own result is itself stored as `NodeContent::Data`.
///
/// Dragging each of `A`, `XOR`, and `AND` in turn then proves the routing this chain depends on: an edge always
/// reroutes when either of its own two endpoints moves, and — just as important — never touches an edge whose own
/// endpoints did not, whether that edge sits upstream or downstream of the node that actually moved.
#[wasm_bindgen_test]
fn dragging_each_node_in_an_operator_to_operator_chain_reroutes_only_its_own_incident_connectors() -> Result<(), String>
{
    let svg = make_svg("operator-chain-drag", Size::new(700.0, 600.0), Size::new(700.0, 600.0));
    let scene = Scene::new(svg).map_err(|e| e.to_string())?;

    let a = scene
        .add_data_node(
            Point::new(140.0, 10.0),
            DataNodeContent::new(NodeValues::U8(vec![0b1010_1010]), DataFormat::Binary),
        )
        .map_err(|e| e.to_string())?;
    let b = scene
        .add_data_node(
            Point::new(320.0, 10.0),
            DataNodeContent::new(NodeValues::U8(vec![0b0101_0101]), DataFormat::Binary),
        )
        .map_err(|e| e.to_string())?;
    let c = scene
        .add_data_node(
            Point::new(550.0, 420.0),
            DataNodeContent::new(NodeValues::U8(vec![0b0000_1111]), DataFormat::Binary),
        )
        .map_err(|e| e.to_string())?;

    let xor_value = 0b1010_1010 ^ 0b0101_0101;
    let xor = scene
        .add_binary_operator_node(
            Point::new(220.0, 150.0),
            BinaryOperator::Xor,
            (a, b),
            DataNodeContent::new(NodeValues::U8(vec![xor_value]), DataFormat::Binary),
        )
        .map_err(|e| e.to_string())?;

    // `xor` — an operator node's own id, not a plain data node's — is `AND`'s own first operand here: this is the
    // "operator feeding operator" wiring the demos show but no prior test exercises directly.
    let and = scene
        .add_binary_operator_node(
            Point::new(220.0, 400.0),
            BinaryOperator::And,
            (xor, c),
            DataNodeContent::new(NodeValues::U8(vec![xor_value & 0b0000_1111]), DataFormat::Binary),
        )
        .map_err(|e| e.to_string())?;

    scene.make_draggable(a).map_err(|e| e.to_string())?;
    scene.make_draggable(xor).map_err(|e| e.to_string())?;
    scene.make_draggable(and).map_err(|e| e.to_string())?;

    check(
        connector_count("operator-chain-drag")? == 4,
        &format!(
            "expected 4 auto-wired connectors, found {}",
            connector_count("operator-chain-drag")?
        ),
    )?;

    // Connector indices, in creation order: 0 = A→XOR, 1 = B→XOR, 2 = XOR→AND, 3 = C→AND.
    let path = |n: u32| -> Result<String, String> {
        crate::common::path_d(&crate::common::nth_connector("operator-chain-drag", n)?)
    };

    // `C` feeds `AND` from a different side than `XOR` does — `XOR` sits directly above `AND`, `C` sits far to its
    // east — so each of `AND`'s own two inputs resolves its own anchor independently, and every "must stay exactly
    // as it was" check below holds regardless of how the same-side anti-crossing split behaves elsewhere in the
    // chain.

    // --- Drag A: only A's own edge into XOR touches A at all. ---
    let (path_xor_and_before, path_c_and_before) = (path(2)?, path(3)?);
    let path_a_xor_before = path(0)?;
    let a_group = nth_group("operator-chain-drag", 0)?;
    dispatch_pointer_event(&a_group, "pointerdown", 100, 100, 1)?;
    dispatch_pointer_event(&a_group, "pointermove", 160, 180, 1)?;
    dispatch_pointer_event(&a_group, "pointerup", 160, 180, 1)?;

    check(
        path(0)? != path_a_xor_before,
        "expected A's own edge into XOR to reroute after dragging A",
    )?;
    check(
        path(2)? == path_xor_and_before,
        "expected XOR's own edge into AND to stay put while only A moved",
    )?;
    check(
        path(3)? == path_c_and_before,
        "expected C's own edge into AND to stay put while only A moved",
    )?;

    // --- Drag XOR: both of its own inputs, and its own output into AND, all touch XOR. C's own edge into AND does
    // not — proving a moved *intermediate* operator correctly reroutes both its incoming and its outgoing edges,
    // without disturbing a sibling edge into the same downstream node. ---
    let (path_a_xor_before, path_b_xor_before, path_xor_and_before, path_c_and_before) =
        (path(0)?, path(1)?, path(2)?, path(3)?);
    let xor_group = nth_group("operator-chain-drag", 3)?;
    dispatch_pointer_event(&xor_group, "pointerdown", 100, 100, 1)?;
    dispatch_pointer_event(&xor_group, "pointermove", 60, 40, 1)?;
    dispatch_pointer_event(&xor_group, "pointerup", 60, 40, 1)?;

    check(
        path(0)? != path_a_xor_before,
        "expected A's own edge into XOR to reroute after dragging XOR",
    )?;
    check(
        path(1)? != path_b_xor_before,
        "expected B's own edge into XOR to reroute after dragging XOR",
    )?;
    check(
        path(2)? != path_xor_and_before,
        "expected XOR's own edge into AND to reroute after dragging XOR",
    )?;
    check(
        path(3)? == path_c_and_before,
        "expected C's own edge into AND to stay put while only XOR moved",
    )?;

    // --- Drag AND: both of its own inputs touch AND. Neither of XOR's own inputs does — proving a moved
    // *downstream* operator never disturbs the edges feeding the operator upstream of it. ---
    let (path_a_xor_before, path_b_xor_before, path_xor_and_before, path_c_and_before) =
        (path(0)?, path(1)?, path(2)?, path(3)?);
    let and_group = nth_group("operator-chain-drag", 4)?;
    dispatch_pointer_event(&and_group, "pointerdown", 100, 100, 1)?;
    dispatch_pointer_event(&and_group, "pointermove", 150, 60, 1)?;
    dispatch_pointer_event(&and_group, "pointerup", 150, 60, 1)?;

    check(
        path(2)? != path_xor_and_before,
        "expected XOR's own edge into AND to reroute after dragging AND",
    )?;
    check(
        path(3)? != path_c_and_before,
        "expected C's own edge into AND to reroute after dragging AND",
    )?;
    check(
        path(0)? == path_a_xor_before,
        "expected A's own edge into XOR to stay put while only AND moved",
    )?;
    check(
        path(1)? == path_b_xor_before,
        "expected B's own edge into XOR to stay put while only AND moved",
    )
}
