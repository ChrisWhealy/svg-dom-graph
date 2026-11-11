//! Where a job is in its life, and whether it can still change.

use super::Failure;

/// Where a job is in its life, and whether it can still change.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum State {
    Queued,
    Running,
    RetryWait,
    Completed,
    TimedOut,
    Cancelled,
    Rejected,
    Failed(Failure),
}

impl State {
    pub(super) fn active(self) -> bool {
        matches!(self, Self::Queued | Self::Running | Self::RetryWait)
    }

    pub(super) fn text(self) -> String {
        match self {
            Self::Queued => "queued".into(),
            Self::Running => "running".into(),
            Self::RetryWait => "retrywait".into(),
            Self::Completed => "completed".into(),
            Self::TimedOut => "timedout".into(),
            Self::Cancelled => "cancelled".into(),
            Self::Rejected => "rejected".into(),
            Self::Failed(Failure::AttemptsExhausted) => "failed(attempts_exhausted)".into(),
            Self::Failed(Failure::DeferralsExhausted) => "failed(retry_deferrals_exhausted)".into(),
        }
    }

    pub(super) fn code(self) -> u8 {
        match self {
            Self::Queued => 1,
            Self::Running => 2,
            _ => 3,
        }
    }
}
