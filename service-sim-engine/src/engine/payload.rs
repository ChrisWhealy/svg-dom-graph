//! What an event carries, which depends on its kind.

/// What an event carries, which depends on its kind.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum Payload {
    Complete {
        job: u64,
        token: u64,
    },
    Fail {
        server: u64,
    },
    Cancel {
        key: String,
    },
    Deadline {
        job: u64,
    },
    Recover {
        server: u64,
    },
    Retry {
        job: u64,
        token: u64,
    },
    Arrive {
        key: String,
        generated: bool,
        priority: u8,
        durations: Vec<u64>,
        deadline_or_slack: u64,
        cancel_delay: u64,
    },
}
