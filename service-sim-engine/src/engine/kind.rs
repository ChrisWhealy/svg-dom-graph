//! The kind of an event.

/// The kind of an event. The number is its rank and also its code in the canonical encoding.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    Complete = 0,
    Fail = 1,
    Cancel = 2,
    Deadline = 3,
    Recover = 4,
    Retry = 5,
    Arrive = 6,
}

impl Kind {
    #[must_use]
    pub fn name(self) -> &'static str {
        match self {
            Self::Complete => "complete",
            Self::Fail => "fail",
            Self::Cancel => "cancel",
            Self::Deadline => "deadline",
            Self::Recover => "recover",
            Self::Retry => "retry",
            Self::Arrive => "arrive",
        }
    }
}
