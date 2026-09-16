// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// A single-operand bitwise operation an [`crate::scene::Scene::add_unary_operator_node`] node represents.
///
/// This crate never evaluates the operation itself — see this module's parent doc comment. Each variant exists
/// purely to drive the rendered node's own label; a caller supplies the already-computed result separately.
///
/// `#[non_exhaustive]`, for the same reason as [`super::DataFormat`]. A plausible future addition — `Not`'s
/// bit-count-aware cousins, say — should stay additive.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum UnaryOperator {
    /// Bitwise complement: every bit flips.
    Not,
    /// Logical shift left by the wrapped bit count. Bits shifted past the type's own width are discarded.
    ShiftLeft(u8),
    /// Logical shift right by the wrapped bit count. Bits shifted past the type's own width are discarded.
    ShiftRight(u8),
    /// Rotate left by the wrapped bit count. Bits shifted out one end reappear at the other.
    RotateLeft(u8),
    /// Rotate right by the wrapped bit count. Bits shifted out one end reappear at the other.
    RotateRight(u8),
}

impl UnaryOperator {
    /// A short label naming this operation, for the operator node's own rendered glyph row — e.g. `"NOT"`,
    /// `"SHL 3"`, `"ROTR 1"`.
    pub(crate) fn label(self) -> String {
        match self {
            Self::Not => "NOT".to_owned(),
            Self::ShiftLeft(n) => format!("SHL {n}"),
            Self::ShiftRight(n) => format!("SHR {n}"),
            Self::RotateLeft(n) => format!("ROTL {n}"),
            Self::RotateRight(n) => format!("ROTR {n}"),
        }
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// A two-operand Boolean operation an [`crate::scene::Scene::add_binary_operator_node`] node represents.
///
/// This crate never evaluates the operation itself — see this module's parent doc comment. Each variant exists
/// purely to drive the rendered node's own label; a caller supplies the already-computed result separately.
///
/// `#[non_exhaustive]`, for the same reason as [`super::DataFormat`]. A plausible future addition — `Nand`, `Nor`,
/// `Xnor` — should stay additive.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum BinaryOperator {
    /// Bitwise AND.
    And,
    /// Bitwise OR.
    Or,
    /// Bitwise XOR.
    Xor,
}

impl BinaryOperator {
    /// A short label naming this operation, for the operator node's own rendered glyph row — `"AND"`, `"OR"`, or
    /// `"XOR"`.
    pub(crate) fn label(self) -> &'static str {
        match self {
            Self::And => "AND",
            Self::Or => "OR",
            Self::Xor => "XOR",
        }
    }
}
