//! What a pop did.

/// What a pop did.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Outcome {
    /// It changed something.
    Effective,
    /// It carries a token, and found its job or attempt gone.
    Stale,
    /// It named something that had nothing to act on.
    NoOp,
}

impl Outcome {
    pub(super) fn letter(self) -> char {
        match self {
            Self::Effective => 'e',
            Self::Stale => 's',
            Self::NoOp => 'n',
        }
    }

    #[must_use]
    pub fn code(self) -> u8 {
        match self {
            Self::Effective => 0,
            Self::Stale => 1,
            Self::NoOp => 2,
        }
    }
}
