//! The reasons a [`BitGrid`](super::BitGrid) refuses a change.

use std::fmt;

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Why a [`BitGrid`](super::BitGrid) refused a change. The grid is unchanged whenever one of these is returned.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum GridError {
    OutOfBounds {
        row: usize,
        col: usize,
    },
    /// The module already has a role or a colour, so it cannot be reserved or assigned.
    Occupied {
        row: usize,
        col: usize,
    },
    /// The module is not reserved for the role the caller expected.
    NotReserved {
        row: usize,
        col: usize,
    },
    /// The module is not a coloured data or error correction module, so it cannot be flipped.
    NotFlippable {
        row: usize,
        col: usize,
    },
}

impl fmt::Display for GridError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {
            Self::OutOfBounds { row, col } => write!(f, "module ({row}, {col}) is outside the grid"),
            Self::Occupied { row, col } => write!(f, "module ({row}, {col}) already has a role or a colour"),
            Self::NotReserved { row, col } => write!(f, "module ({row}, {col}) is not reserved for that role"),
            Self::NotFlippable { row, col } => write!(f, "module ({row}, {col}) holds no coloured payload bit"),
        }
    }
}

impl std::error::Error for GridError {}
