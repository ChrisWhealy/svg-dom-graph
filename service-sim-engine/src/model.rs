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
/// The engine after a whole slice, or before the first, as far as a reader can see it.
///
/// It does not hold every job that has arrived. A run can have tens of thousands of them, and copying them all for each displayed
/// slice would make the cost of a draw grow with the length of the run and not with what is on the screen. So it holds the jobs that
/// a projection can name: those in the first `window` places of the queue, those that are running, and the one that was selected.
/// The rest is in aggregates that the engine keeps as it goes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Model {
    /// The tick of the last slice, or 0 before the first.
    pub tick: u64,
    /// The number of slices done. This is the snapshot index: 0 is the initial state.
    pub slice: u64,
    pub counters: Counters,
    pub servers: Vec<ServerView>,
    /// The ids of the first waiting jobs, in the order that the next dispatch takes them, and no more than the window asked for.
    pub waiting: Vec<u64>,
    /// How many jobs wait, which is the length of the queue and not of `waiting`.
    pub waiting_total: usize,
    /// How many jobs wait to retry.
    pub retry_wait: usize,
    /// How many jobs have arrived. Their ids are `0..arrived`.
    pub arrived: u64,
    /// The jobs a projection can name, in the order of their ids: the window, the running jobs and the selected one.
    pub jobs: Vec<JobView>,
    /// The job that this model was extracted for. It is in `jobs` if it has arrived.
    pub selected: Option<u64>,
    pub queue_capacity: usize,
}

impl Model {
    /// The job with this id, if it is one that the model holds.
    #[must_use]
    pub fn job(&self, id: u64) -> Option<&JobView> {
        self.jobs.binary_search_by_key(&id, |j| j.id).ok().map(|i| &self.jobs[i])
    }
}
