//! The non-commutative `SUB`/`DIV`/`MOD` "L"/"R" port marker: which operators draw one, its accessible name, and
//! that its own identity survives a near/far reassignment or a full operand-position exchange.

use crate::common::{
    check, check_close, dispatch_pointer_event, group_translate, make_svg, nth_group, nth_port_marker,
    port_marker_count,
};
use svg_dom::root::utils::{Point, Size};
use svg_dom_graph::scene::{
    ArithmeticOperator, BinaryOperator, CollisionPolicy, DataFormat, DataNodeContent, DragOptions, NodeValues, Scene,
};
use wasm_bindgen_test::wasm_bindgen_test;

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// A commutative binary operator — swapping the operands never changes the result — draws no "L"/"R" port marker
/// at all: there is nothing ambiguous about operand order for a reader to be told apart.
#[wasm_bindgen_test]
fn a_commutative_binary_operator_node_draws_no_port_markers() -> Result<(), String> {
    let svg = make_svg(
        "operator-markers-commutative-binary",
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
            DataNodeContent::new(NodeValues::U8(vec![2]), DataFormat::Decimal),
        )
        .map_err(|e| e.to_string())?;
    let result = DataNodeContent::new(NodeValues::U8(vec![3]), DataFormat::Decimal);
    scene
        .add_binary_operator_node(Point::new(220.0, 60.0), BinaryOperator::Xor, (a, b), result)
        .map_err(|e| e.to_string())?;

    check(
        port_marker_count("operator-markers-commutative-binary")? == 0,
        &format!(
            "expected no port markers for a commutative operator, found {}",
            port_marker_count("operator-markers-commutative-binary")?
        ),
    )
}

/// `Add` and `Multiply` are the only two commutative [`ArithmeticOperator`] variants — neither draws a port marker.
#[wasm_bindgen_test]
fn commutative_arithmetic_operators_draw_no_port_markers() -> Result<(), String> {
    let svg = make_svg(
        "operator-markers-commutative-arithmetic",
        Size::new(400.0, 260.0),
        Size::new(400.0, 260.0),
    );
    let scene = Scene::new(svg).map_err(|e| e.to_string())?;
    for operator in [ArithmeticOperator::Add, ArithmeticOperator::Multiply] {
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
        let result = DataNodeContent::new(NodeValues::U8(vec![3]), DataFormat::Decimal);
        scene
            .add_arithmetic_operator_node(Point::new(220.0, 60.0), operator, (a, b), result)
            .map_err(|e| e.to_string())?;
    }

    check(
        port_marker_count("operator-markers-commutative-arithmetic")? == 0,
        &format!(
            "expected no port markers for Add or Multiply, found {}",
            port_marker_count("operator-markers-commutative-arithmetic")?
        ),
    )
}

/// `Subtract`, `Divide`, and `Modulus` each draw exactly two port markers — one per input — labelled "L"/"R", with
/// an `aria-label` naming which operand each one is, and `role="img"` so assistive technology announces that name
/// rather than reading the bare glyph.
#[wasm_bindgen_test]
fn non_commutative_arithmetic_operators_draw_labelled_port_markers() -> Result<(), String> {
    for operator in [
        ArithmeticOperator::Subtract,
        ArithmeticOperator::Divide,
        ArithmeticOperator::Modulus,
    ] {
        let id = format!("operator-markers-non-commutative-{operator:?}");
        let svg = make_svg(&id, Size::new(400.0, 260.0), Size::new(400.0, 260.0));
        let scene = Scene::new(svg).map_err(|e| e.to_string())?;
        let a = scene
            .add_data_node(
                Point::new(10.0, 10.0),
                DataNodeContent::new(NodeValues::U8(vec![9]), DataFormat::Decimal),
            )
            .map_err(|e| e.to_string())?;
        let b = scene
            .add_data_node(
                Point::new(10.0, 120.0),
                DataNodeContent::new(NodeValues::U8(vec![3]), DataFormat::Decimal),
            )
            .map_err(|e| e.to_string())?;
        let result = DataNodeContent::new(NodeValues::U8(vec![3]), DataFormat::Decimal);
        scene
            .add_arithmetic_operator_node(Point::new(220.0, 60.0), operator, (a, b), result)
            .map_err(|e| e.to_string())?;

        check(
            port_marker_count(&id)? == 2,
            &format!("expected 2 port markers for {operator:?}, found {}", port_marker_count(&id)?),
        )?;

        let left = nth_port_marker(&id, 0)?;
        check(
            left.text_content().as_deref() == Some("L"),
            "expected the first port marker's own glyph to be \"L\"",
        )?;
        check(
            left.get_attribute("aria-label").as_deref() == Some("left operand"),
            "expected the first port marker's own aria-label to be \"left operand\"",
        )?;
        check(
            left.get_attribute("role").as_deref() == Some("img"),
            "expected the first port marker's own role to be \"img\"",
        )?;

        let right = nth_port_marker(&id, 1)?;
        check(
            right.text_content().as_deref() == Some("R"),
            "expected the second port marker's own glyph to be \"R\"",
        )?;
        check(
            right.get_attribute("aria-label").as_deref() == Some("right operand"),
            "expected the second port marker's own aria-label to be \"right operand\"",
        )?;
    }
    Ok(())
}

/// The core regression this feature exists to fix: dragging one operand onto its sibling's own side forces the
/// anti-crossing router to reassign which operand's connector lands on the near/far slot — see
/// `dragging_an_operand_onto_its_siblings_side_re_splits_both_connectors_live`, the matching test for the plain
/// connector routing this builds on. Despite that reassignment, the "L" marker must stay bound to `inputs.0`'s own
/// connector and the "R" marker to `inputs.1`'s — never swapped just because the router moved one of them to a
/// different visual slot.
#[wasm_bindgen_test]
fn dragging_an_operand_re_splits_the_connectors_but_each_port_markers_own_identity_stays_correct() -> Result<(), String>
{
    let svg = make_svg(
        "operator-markers-live-identity",
        Size::new(500.0, 400.0),
        Size::new(500.0, 400.0),
    );
    let scene = Scene::new(svg).map_err(|e| e.to_string())?;
    let a = scene
        .add_data_node(
            Point::new(140.0, 10.0),
            DataNodeContent::new(NodeValues::U8(vec![9]), DataFormat::Decimal),
        )
        .map_err(|e| e.to_string())?;
    let b = scene
        .add_data_node(
            Point::new(450.0, 300.0),
            DataNodeContent::new(NodeValues::U8(vec![3]), DataFormat::Decimal),
        )
        .map_err(|e| e.to_string())?;
    scene.make_draggable(b).map_err(|e| e.to_string())?;
    let result = DataNodeContent::new(NodeValues::U8(vec![3]), DataFormat::Decimal);
    scene
        .add_arithmetic_operator_node(Point::new(220.0, 300.0), ArithmeticOperator::Subtract, (a, b), result)
        .map_err(|e| e.to_string())?;

    // Marker 0 ("L") stays bound to `a` == `inputs.0`'s own edge (connector 0), marker 1 ("R") to connector 1 —
    // never swapped — both before the drag below (where `a` and `b` start on different operator sides) and after
    // (where the drag forces them onto the same side, triggering a near/far reassignment). See
    // `check_port_marker_identity`'s own doc comment for how it stays exact in both cases.
    crate::common::check_port_marker_identity("operator-markers-live-identity")?;

    // Drag `b` to join `a` above the operator, forcing the same-side split — and, for at least one of the two
    // input rows, a near/far reassignment relative to before the drag.
    let b_group = nth_group("operator-markers-live-identity", 1)?;
    dispatch_pointer_event(&b_group, "pointerdown", 100, 100, 1)?;
    dispatch_pointer_event(&b_group, "pointermove", -50, -190, 1)?;
    dispatch_pointer_event(&b_group, "pointerup", -50, -190, 1)?;

    crate::common::check_port_marker_identity("operator-markers-live-identity")
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Physically exchanging `SUB`'s own two operand nodes' positions — not just dragging one across, but swapping
/// which one sits where — must never disturb which port marker names which operand: `inputs.0`'s own edge keeps the
/// "L" marker and `inputs.1`'s own edge keeps "R", wherever each one now physically sits. This is the concrete case
/// the original report calls out: a diagram reader cannot tell `A - B` from `B - A` by position alone, so the
/// marker's own identity must survive exactly this kind of exchange.
#[wasm_bindgen_test]
fn exchanging_the_two_operand_positions_of_a_subtract_node_preserves_lhs_rhs_identity() -> Result<(), String> {
    let svg = make_svg(
        "operator-markers-exchange-identity",
        Size::new(500.0, 400.0),
        Size::new(500.0, 400.0),
    );
    let scene = Scene::new(svg).map_err(|e| e.to_string())?;
    let lhs = scene
        .add_data_node(
            Point::new(140.0, 10.0),
            DataNodeContent::new(NodeValues::U8(vec![9]), DataFormat::Decimal),
        )
        .map_err(|e| e.to_string())?;
    let rhs = scene
        .add_data_node(
            Point::new(320.0, 10.0),
            DataNodeContent::new(NodeValues::U8(vec![3]), DataFormat::Decimal),
        )
        .map_err(|e| e.to_string())?;
    // `CollisionPolicy::Allow`: the exchange below drags `lhs` onto `rhs`'s own (still current) position and vice
    // versa, so the two nodes briefly overlap mid-exchange — the default push-clear policy would fight that.
    let drag_options = DragOptions::default().with_collision(CollisionPolicy::Allow);
    scene.make_draggable_with(lhs, drag_options).map_err(|e| e.to_string())?;
    scene.make_draggable_with(rhs, drag_options).map_err(|e| e.to_string())?;
    let result = DataNodeContent::new(NodeValues::U8(vec![9 - 3]), DataFormat::Decimal);
    scene
        .add_arithmetic_operator_node(Point::new(220.0, 220.0), ArithmeticOperator::Subtract, (lhs, rhs), result)
        .map_err(|e| e.to_string())?;

    // Marker 0 ("L") stays bound to `lhs` == `inputs.0`'s own edge (connector 0), marker 1 ("R") to `rhs`'s
    // (connector 1) — never swapped. See `check_port_marker_identity`'s own doc comment for how it stays exact
    // through the exchange below.
    crate::common::check_port_marker_identity("operator-markers-exchange-identity")?;

    // Exchange the two operands' own physical positions outright: `lhs` moves to where `rhs` started, and `rhs`
    // moves to where `lhs` started.
    let (lhs_x, lhs_y) = group_translate(&nth_group("operator-markers-exchange-identity", 0)?)?;
    let (rhs_x, rhs_y) = group_translate(&nth_group("operator-markers-exchange-identity", 1)?)?;

    let lhs_group = nth_group("operator-markers-exchange-identity", 0)?;
    let lhs_end = (100.0 + (rhs_x - lhs_x), 100.0 + (rhs_y - lhs_y));
    dispatch_pointer_event(&lhs_group, "pointerdown", 100, 100, 1)?;
    dispatch_pointer_event(&lhs_group, "pointermove", lhs_end.0 as i32, lhs_end.1 as i32, 1)?;
    dispatch_pointer_event(&lhs_group, "pointerup", lhs_end.0 as i32, lhs_end.1 as i32, 1)?;

    let rhs_group = nth_group("operator-markers-exchange-identity", 1)?;
    let rhs_end = (100.0 + (lhs_x - rhs_x), 100.0 + (lhs_y - rhs_y));
    dispatch_pointer_event(&rhs_group, "pointerdown", 100, 100, 1)?;
    dispatch_pointer_event(&rhs_group, "pointermove", rhs_end.0 as i32, rhs_end.1 as i32, 1)?;
    dispatch_pointer_event(&rhs_group, "pointerup", rhs_end.0 as i32, rhs_end.1 as i32, 1)?;

    // Confirm the exchange actually happened: `lhs` is now where `rhs` started, and vice versa.
    check_close(group_translate(&nth_group("operator-markers-exchange-identity", 0)?)?.0, rhs_x)?;
    check_close(group_translate(&nth_group("operator-markers-exchange-identity", 1)?)?.0, lhs_x)?;

    crate::common::check_port_marker_identity("operator-markers-exchange-identity")
}
