// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// A node representing a two-operand arithmetic operation. Represented by the
/// [`crate::scene::Scene::add_arithmetic_operator_node`] and [`crate::scene::Scene::add_arithmetic_operator_node_with`]
/// constructors
///
/// This crate never calculates the result of the operation. See this module's parent doc comment. Each variant exists
/// purely to drive the rendered node's own label; the caller must supply the correctly-calculated result.
///
/// Covers the five plain arithmetic operators on unsigned integers: addition, subtraction, multiplication,
/// division and modulus.
///
/// Unlike all of the [`super::binary::BinaryOperator`]s, the operands are commutative:
/// When passing the `input` tuple to [`crate::scene::Scene::add_arithmetic_operator_node_with`], `inputs.0` is always
/// the left-hand operand and `inputs.1` always the right-hand one.
///
/// Division and modulus by zero panic in plain Rust arithmetic. Like computing every other operator's result, avoiding
/// division-by-zero is the caller's responsibility.
///
/// `#[non_exhaustive]`, for the same reason as [`crate::model::content::data_format::DataFormat`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum ArithmeticOperator {
    /// Addition: `a + b`.
    Add,
    /// Subtraction: `a - b`.
    Subtract,
    /// Multiplication: `a * b`.
    Multiply,
    /// Division: `a / b`.
    Divide,
    /// Modulus (remainder): `a % b`.
    Modulus,
}

impl ArithmeticOperator {
    /// A short label naming this operation, for the operator node's own rendered glyph row — `"ADD"`, `"SUB"`,
    /// `"MUL"`, `"DIV"`, or `"MOD"`.
    pub(crate) fn label(self) -> &'static str {
        match self {
            Self::Add => "ADD",
            Self::Subtract => "SUB",
            Self::Multiply => "MUL",
            Self::Divide => "DIV",
            Self::Modulus => "MOD",
        }
    }
}
