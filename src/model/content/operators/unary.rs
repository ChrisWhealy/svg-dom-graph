// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// A single-operand bitwise operation an [`crate::scene::Scene::add_unary_operator_node`] node represents.
///
/// This crate never evaluates the operation itself — see this module's parent doc comment. Each variant exists
/// purely to drive the rendered node's own label; a caller supplies the already-computed result separately.
///
/// Covers every same-width bitwise transform a single unsigned operand supports: complement, shift, rotate, bit
/// reversal, and byte-order reversal. Deliberately excludes a bit-counting operation such as population count or
/// leading/trailing zero count — those produce a small count, not a same-width value, so they would not fit this
/// crate's own "operand, result, and every input share one `NodeValues` width" contract
/// [`crate::model::content::data_node_content::DataNodeContent`] enforces.
///
/// `#[non_exhaustive]`, for the same reason as [`crate::model::content::data_format::DataFormat`].
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
    /// Reverses the operand's own bit order end to end — the type's own most significant bit becomes its least
    /// significant, and so on inward. The same transform ARM's own `RBIT` instruction names.
    ReverseBits,
    /// Reverses the operand's own byte order end to end, leaving each byte's own bits untouched — the same transform
    /// x86's own `BSWAP` instruction names, and [`crate::model::content::byte_order::ByteOrder`] applies implicitly
    /// when/ formatting [`crate::model::content::data_format::DataFormat::Hexadecimal`] /
    /// [`crate::model::content::data_format::DataFormat::Binary`] under
    /// [`crate::model::content::byte_order::ByteOrder::LittleEndian`]. Here, unlike there, the reversed byte order is
    /// the value itself, not just how it is displayed.
    SwapBytes,
}

impl UnaryOperator {
    /// A short label naming this operation, for the operator node's own rendered glyph row — e.g. `"NOT"`,
    /// `"SHL 3"`, `"ROTR 1"`, `"RBIT"`, `"BSWAP"`.
    pub(crate) fn label(self) -> String {
        match self {
            Self::Not => "NOT".to_owned(),
            Self::ShiftLeft(n) => format!("SHL {n}"),
            Self::ShiftRight(n) => format!("SHR {n}"),
            Self::RotateLeft(n) => format!("ROTL {n}"),
            Self::RotateRight(n) => format!("ROTR {n}"),
            Self::ReverseBits => "RBIT".to_owned(),
            Self::SwapBytes => "BSWAP".to_owned(),
        }
    }
}
