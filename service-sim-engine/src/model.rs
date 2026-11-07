//! A read-only picture of the engine between two slices, which is what a projection draws.
//!
//! It holds nothing that the engine needs to run and nothing that a hash covers, so reading it cannot change a run. The engine's own
//! types stay private, and these are copies of what a reader may see.

use crate::engine::Counters;

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Where a job is, in the nine states of design section 3.2 that a job can be in once it has arrived.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JobState {
    Queued,
    Running,
    RetryWait,
    Completed,
    TimedOut,
    Cancelled,
    Rejected,
    /// The last attempt failed and no attempt is left.
    FailedAttempts,
    /// A retry found the waiting queue full more often than the cap allows.
    FailedDeferrals,
}

impl JobState {
    /// The word for the state, as the final report spells it with no reason.
    #[must_use]
    pub fn word(self) -> &'static str {
        match self {
            Self::Queued => "queued",
            Self::Running => "running",
            Self::RetryWait => "waiting to retry",
            Self::Completed => "completed",
            Self::TimedOut => "timed out",
            Self::Cancelled => "cancelled",
            Self::Rejected => "rejected",
            Self::FailedAttempts => "failed, no attempts left",
            Self::FailedDeferrals => "failed, retry deferred too often",
        }
    }

    /// Whether the job has not reached a terminal state.
    #[must_use]
    pub fn active(self) -> bool {
        matches!(self, Self::Queued | Self::Running | Self::RetryWait)
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// One job that has arrived.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JobView {
    pub id: u64,
    pub key: String,
    /// 1 is high, and 0 is low.
    pub priority: u8,
    pub state: JobState,
    /// The attempt that is running, or the last one that ran. Zero before the first.
    pub attempt: u32,
    /// The server that a running job holds.
    pub server: Option<usize>,
    /// The tick at which a running job completes.
    pub due: Option<u64>,
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// One server.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ServerView {
    Idle,
    Busy { job: u64, due: u64 },
    Down,
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The engine after a whole slice, or before the first one.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Model {
    /// The tick of the last slice, or 0 before the first.
    pub tick: u64,
    /// The number of slices done. This is the snapshot index: 0 is the initial state.
    pub slice: u64,
    pub counters: Counters,
    pub servers: Vec<ServerView>,
    /// The ids of the waiting jobs, in the order that the next dispatch takes them.
    pub waiting: Vec<u64>,
    /// Every job that has arrived, in the order of their ids.
    pub jobs: Vec<JobView>,
    pub queue_capacity: usize,
}

impl Model {
    /// The job with this id.
    #[must_use]
    pub fn job(&self, id: u64) -> Option<&JobView> {
        usize::try_from(id).ok().and_then(|i| self.jobs.get(i))
    }

    /// How many jobs are in `state`.
    #[must_use]
    pub fn count(&self, state: JobState) -> usize {
        self.jobs.iter().filter(|j| j.state == state).count()
    }
}
