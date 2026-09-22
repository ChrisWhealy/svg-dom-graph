// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// A two-operand Boolean operation an [`crate::scene::Scene::add_binary_operator_node`] node represents.
///
/// This crate never evaluates the operation itself — see this module's parent doc comment. Each variant exists
/// purely to drive the rendered node's own label; a caller supplies the already-computed result separately.
///
/// Covers all six non-trivial two-input Boolean functions bitwise operators commonly name: `And`/`Or`/`Xor` and
/// their own negations `Nand`/`Nor`/`Xnor`. Deliberately excludes the degenerate functions a truth table also
/// admits — constant true/false, either operand alone or negated, and the two "ignores one operand" implications —
/// none of which read as a distinct bitwise *operator* the way these six do.
///
/// `#[non_exhaustive]`, for the same reason as [`crate::model::content::data_format::DataFormat`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum BinaryOperator {
    /// Bitwise AND.
    And,
    /// Bitwise OR.
    Or,
    /// Bitwise XOR.
    Xor,
    /// Bitwise AND, then complemented: `!(a & b)`.
    Nand,
    /// Bitwise OR, then complemented: `!(a | b)`.
    Nor,
    /// Bitwise XOR, then complemented: `!(a ^ b)` — equivalently, "every bit position where the two operands
    /// agree."
    Xnor,
}

impl BinaryOperator {
    /// A short label naming this operation, for the operator node's own rendered glyph row — `"AND"`, `"OR"`,
    /// `"XOR"`, `"NAND"`, `"NOR"`, or `"XNOR"`.
    pub(crate) fn label(self) -> &'static str {
        match self {
            Self::And => "AND",
            Self::Or => "OR",
            Self::Xor => "XOR",
            Self::Nand => "NAND",
            Self::Nor => "NOR",
            Self::Xnor => "XNOR",
        }
    }

    /// Whether swapping the two operands leaves the result unchanged. Always `true`: every `BinaryOperator`
    /// variant commutes.
    pub(crate) fn commutes(self) -> bool {
        true
    }
}
