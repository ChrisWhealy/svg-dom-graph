//! Why a job ended as failed.

/// Why a job ended as failed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Failure {
    AttemptsExhausted,
    DeferralsExhausted,
}
