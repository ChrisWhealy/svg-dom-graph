// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// A node representing a two-operand arithmetic operation. Represented by the
/// [`crate::scene::Scene::add_arithmetic_operator_node`] and [`crate::scene::Scene::add_arithmetic_operator_node_with`]
/// constructors
///
/// This crate never calculates the result of the operation. See this module's parent doc comment. Each variant exists
/// purely to drive the rendered node's own label; the caller must supply the correctly-calculated result.
///
/// This `enum` covers the five plain arithmetic operators: addition, subtraction, multiplication, division and
/// modulus. They apply to integers, signed or unsigned, and to floating point values, which is unlike the unary and
/// binary operators, whose operands must be integers.
///
/// Unlike all of the [`super::binary::BinaryOperator`]s, the operands for the arithmetic operators `subtract`, `divide`
/// and `modulo` are not commutative. When passing the `input` tuple to
/// [`crate::scene::Scene::add_arithmetic_operator_node_with`], `inputs.0` is always the left-hand operand and
/// `inputs.1` always the right-hand one.
///
/// Division and modulus of an integer by zero panic in plain Rust arithmetic, and an overflowing integer operation
/// panics in a debug build. Like computing every other operator's result, avoiding them is the caller's responsibility.
/// A float divided by zero is an infinity, or `NaN`, which a float node can show.
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
    /// A short label naming this operation, for the operator node's own rendered glyph row — `"ADD"`, `"SUB"`, `"MUL"`,
    /// `"DIV"`, or `"MOD"`.
    pub(crate) fn label(self) -> &'static str {
        match self {
            Self::Add => "ADD",
            Self::Subtract => "SUB",
            Self::Multiply => "MUL",
            Self::Divide => "DIV",
            Self::Modulus => "MOD",
        }
    }

    /// Whether swapping the two operands leaves the result unchanged. `true` for `Add` and `Multiply`, `false` for
    /// everything else
    pub(crate) fn commutes(self) -> bool {
        matches!(self, Self::Add | Self::Multiply)
    }
}
