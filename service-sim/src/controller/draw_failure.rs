//! Why a draw did not succeed.

/// Why a draw did not succeed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DrawFailure {
    /// The host was not in the document when the draw looked.
    Detached,
    /// A write failed, or the projection has the wrong shape. The page may be partly written.
    Failed(String),
}
