//! Operator labels and commutativity: `UnaryOperator`, `BinaryOperator` and `ArithmeticOperator`.

use super::super::*;
use super::support::check_eq;

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
//   UnaryOperator::label / BinaryOperator::label
// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -

#[test]
fn unary_operator_label_names_not_with_no_operand() -> Result<(), String> {
    check_eq(UnaryOperator::Not.label(), "NOT".to_owned())
}

#[test]
fn unary_operator_label_includes_the_shift_or_rotate_amount() -> Result<(), String> {
    check_eq(UnaryOperator::ShiftLeft(3).label(), "SHL 3".to_owned())?;
    check_eq(UnaryOperator::ShiftRight(5).label(), "SHR 5".to_owned())?;
    check_eq(UnaryOperator::RotateLeft(1).label(), "ROTL 1".to_owned())?;
    check_eq(UnaryOperator::RotateRight(1).label(), "ROTR 1".to_owned())
}

#[test]
fn unary_operator_label_names_reverse_bits_and_swap_bytes_with_no_operand() -> Result<(), String> {
    check_eq(UnaryOperator::ReverseBits.label(), "RBIT".to_owned())?;
    check_eq(UnaryOperator::SwapBytes.label(), "BSWAP".to_owned())
}

#[test]
fn binary_operator_label_names_each_variant() -> Result<(), String> {
    check_eq(BinaryOperator::And.label(), "AND")?;
    check_eq(BinaryOperator::Or.label(), "OR")?;
    check_eq(BinaryOperator::Xor.label(), "XOR")?;
    check_eq(BinaryOperator::Nand.label(), "NAND")?;
    check_eq(BinaryOperator::Nor.label(), "NOR")?;
    check_eq(BinaryOperator::Xnor.label(), "XNOR")
}

#[test]
fn arithmetic_operator_label_names_each_variant() -> Result<(), String> {
    check_eq(ArithmeticOperator::Add.label(), "ADD")?;
    check_eq(ArithmeticOperator::Subtract.label(), "SUB")?;
    check_eq(ArithmeticOperator::Multiply.label(), "MUL")?;
    check_eq(ArithmeticOperator::Divide.label(), "DIV")?;
    check_eq(ArithmeticOperator::Modulus.label(), "MOD")
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
//   BinaryOperator::commutes / ArithmeticOperator::commutes
// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -

#[test]
fn every_binary_operator_variant_commutes() -> Result<(), String> {
    check_eq(BinaryOperator::And.commutes(), true)?;
    check_eq(BinaryOperator::Or.commutes(), true)?;
    check_eq(BinaryOperator::Xor.commutes(), true)?;
    check_eq(BinaryOperator::Nand.commutes(), true)?;
    check_eq(BinaryOperator::Nor.commutes(), true)?;
    check_eq(BinaryOperator::Xnor.commutes(), true)
}

#[test]
fn only_add_and_multiply_commute_among_arithmetic_operators() -> Result<(), String> {
    check_eq(ArithmeticOperator::Add.commutes(), true)?;
    check_eq(ArithmeticOperator::Multiply.commutes(), true)?;
    check_eq(ArithmeticOperator::Subtract.commutes(), false)?;
    check_eq(ArithmeticOperator::Divide.commutes(), false)?;
    check_eq(ArithmeticOperator::Modulus.commutes(), false)
}
