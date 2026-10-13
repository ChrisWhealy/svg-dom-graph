//! The pure part of drawing an operator box: where everything sits, given the two measured text widths. Neither touches
//! the DOM, so both run under a plain `cargo test`.

use super::*;
use crate::{
    scene::{DataFormat, NodeValues},
    test_support::check,
};

fn check_eq<T: PartialEq + std::fmt::Debug>(got: T, expected: T) -> Result<(), String> {
    check(got == expected, &format!("expected {expected:?}, got {got:?}"))
}

#[test]
fn a_wide_label_sets_the_box_width() -> Result<(), String> {
    let layout = OperatorLayout::new(200.0, 50.0);
    // The label's own width plus padding beats the value cell plus outer padding.
    check_eq(layout.size.width, 200.0 + 2.0 * CELL_PADDING)
}

#[test]
fn a_wide_value_sets_the_box_width() -> Result<(), String> {
    let layout = OperatorLayout::new(10.0, 200.0);
    check_eq(layout.size.width, 200.0 + 2.0 * CELL_PADDING + 2.0 * OUTER_PADDING)
}

#[test]
fn the_value_cell_is_centred_and_never_reaches_a_side_edge() -> Result<(), String> {
    for (label, value) in [(200.0, 50.0), (10.0, 200.0), (80.0, 80.0)] {
        let layout = OperatorLayout::new(label, value);
        let left = layout.value_cell_origin.x;
        let right = layout.size.width - (left + layout.value_cell_size.width);
        check_eq(left, right)?;
        check(
            left >= OUTER_PADDING,
            "the value cell is clear of the side edges by at least the outer padding",
        )?;
    }
    Ok(())
}

#[test]
fn the_box_is_one_label_row_one_value_cell_and_the_outer_padding_tall() -> Result<(), String> {
    let layout = OperatorLayout::new(100.0, 100.0);
    check_eq(layout.value_cell_origin.y, LABEL_ROW_HEIGHT)?;
    check_eq(
        layout.size.height,
        LABEL_ROW_HEIGHT + CELL_HEIGHT + 2.0 * CELL_PADDING + OUTER_PADDING,
    )
}

#[test]
fn the_texts_are_centred_on_the_box() -> Result<(), String> {
    let layout = OperatorLayout::new(100.0, 100.0);
    check_eq(layout.centre_x(), layout.size.width / 2.0)
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
//   check_operand_types: every operand and the result must share one integer width.
// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -

fn content(values: NodeValues) -> DataNodeContent {
    DataNodeContent::new(values, DataFormat::Hexadecimal)
}

fn rect() -> Rect {
    Rect {
        origin: Point::origin(),
        size: Size::new(10.0, 10.0),
    }
}

#[test]
fn matching_operands_and_result_are_accepted() -> Result<(), String> {
    let mut graph = Graph::new();
    let a = graph.add_node(rect(), content(NodeValues::U16(vec![1])));
    let b = graph.add_node(rect(), content(NodeValues::U16(vec![2])));
    let result = content(NodeValues::U16(vec![3]));
    check(
        check_operand_types(&graph, &[a, b], &result).is_ok(),
        "same width throughout is valid",
    )?;
    check(
        check_operand_types(&graph, &[a], &result).is_ok(),
        "a unary operator is valid too",
    )
}

#[test]
fn the_second_operand_disagreeing_names_both_widths() -> Result<(), String> {
    let mut graph = Graph::new();
    let a = graph.add_node(rect(), content(NodeValues::U8(vec![1])));
    let b = graph.add_node(rect(), content(NodeValues::U32(vec![2])));
    let result = content(NodeValues::U8(vec![3]));
    check(
        matches!(
            check_operand_types(&graph, &[a, b], &result),
            Err(Error::OperatorTypeMismatch { expected: "u8", found: "u32" })
        ),
        "expected an OperatorTypeMismatch naming u8 and u32",
    )
}

#[test]
fn a_result_of_another_width_is_rejected_against_the_operands() -> Result<(), String> {
    let mut graph = Graph::new();
    let a = graph.add_node(rect(), content(NodeValues::U8(vec![1])));
    let b = graph.add_node(rect(), content(NodeValues::U8(vec![2])));
    let result = content(NodeValues::U64(vec![3]));
    check(
        matches!(
            check_operand_types(&graph, &[a, b], &result),
            Err(Error::OperatorTypeMismatch { expected: "u8", found: "u64" })
        ),
        "expected an OperatorTypeMismatch naming u8 and u64",
    )
}

#[test]
fn an_unknown_operand_is_reported_as_such() -> Result<(), String> {
    let mut graph = Graph::new();
    let a = graph.add_node(rect(), content(NodeValues::U8(vec![1])));
    let other = Graph::new().add_node(rect(), content(NodeValues::U8(vec![1])));
    let result = content(NodeValues::U8(vec![3]));
    check(
        matches!(check_operand_types(&graph, &[a, other], &result), Err(Error::UnknownNode(_))),
        "an id from another graph is unknown",
    )
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
//   check_integer_operands: a unary or binary operator needs integer operands.
// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -

#[test]
fn integer_operands_are_accepted_by_a_bitwise_operator() -> Result<(), String> {
    let mut graph = Graph::new();
    let a = graph.add_node(rect(), content(NodeValues::I16(vec![1])));
    let b = graph.add_node(rect(), content(NodeValues::U64(vec![2])));
    check(
        check_integer_operands(&graph, &[a, b], "XOR").is_ok(),
        "signed and unsigned integers are integers",
    )
}

#[test]
fn a_float_operand_is_rejected_by_a_bitwise_operator_and_named() -> Result<(), String> {
    let mut graph = Graph::new();
    let a = graph.add_node(rect(), content(NodeValues::U8(vec![1])));
    let b = graph.add_node(rect(), content(NodeValues::F64(vec![2.0])));
    check(
        matches!(
            check_integer_operands(&graph, &[a, b], "AND"),
            Err(Error::OperatorNeedsIntegers { ref operator, found: "f64" }) if operator == "AND"
        ),
        "expected OperatorNeedsIntegers naming AND and f64",
    )?;
    check(
        matches!(
            check_integer_operands(&graph, &[b], "ROTL 3"),
            Err(Error::OperatorNeedsIntegers { found: "f64", .. })
        ),
        "a unary operator is checked too",
    )
}

#[test]
fn a_uuid_operand_is_rejected_by_every_kind_of_operator_and_a_u128_is_not() -> Result<(), String> {
    use crate::scene::Uuid;
    let mut graph = Graph::new();
    let id = graph.add_node(rect(), content(NodeValues::Uuid(vec![Uuid::from_u128(1)])));
    let number = graph.add_node(rect(), content(NodeValues::U128(vec![1])));
    check(
        matches!(
            check_not_uuid_operands(&graph, &[id], "XOR"),
            Err(Error::OperatorOnUuid { ref operator }) if operator == "XOR"
        ),
        "expected OperatorOnUuid naming XOR",
    )?;
    check(
        matches!(
            check_not_uuid_operands(&graph, &[number, id], "ADD"),
            Err(Error::OperatorOnUuid { .. })
        ),
        "a UUID as the second operand is rejected too",
    )?;
    check(
        check_not_uuid_operands(&graph, &[number], "ADD").is_ok(),
        "a 128-bit number is an ordinary operand",
    )
}

#[test]
fn an_unknown_operand_is_still_reported_as_unknown_by_the_integer_check() -> Result<(), String> {
    let mut graph = Graph::new();
    let a = graph.add_node(rect(), content(NodeValues::U8(vec![1])));
    let other = Graph::new().add_node(rect(), content(NodeValues::U8(vec![1])));
    check(
        matches!(check_integer_operands(&graph, &[a, other], "NOT"), Err(Error::UnknownNode(_))),
        "an id from another graph is unknown",
    )
}
