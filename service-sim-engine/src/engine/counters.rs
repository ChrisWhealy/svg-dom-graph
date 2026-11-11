//! The twelve counters, in the order of the canonical encoding.

/// The twelve counters, in the order of the canonical encoding.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Counters {
    pub pops: u64,
    pub stale: u64,
    pub noops: u64,
    pub slices: u64,
    pub arrivals: u64,
    pub admitted: u64,
    pub rejected: u64,
    pub completed: u64,
    pub timed_out: u64,
    pub cancelled: u64,
    pub failed_attempts: u64,
    pub failed_deferrals: u64,
}

impl Counters {
    #[must_use]
    pub fn as_array(&self) -> [u64; 12] {
        [
            self.pops,
            self.stale,
            self.noops,
            self.slices,
            self.arrivals,
            self.admitted,
            self.rejected,
            self.completed,
            self.timed_out,
            self.cancelled,
            self.failed_attempts,
            self.failed_deferrals,
        ]
    }

    /// The names the trace uses, in the same order.
    pub const NAMES: [&'static str; 12] = [
        "pops",
        "stale",
        "noops",
        "slices",
        "arrivals",
        "admitted",
        "rejected",
        "completed",
        "timedOut",
        "cancelled",
        "failedAttempts",
        "failedDeferrals",
    ];
}
