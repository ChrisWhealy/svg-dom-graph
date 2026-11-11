//! One pop, as it is recorded for comparison.

use super::{Kind, Outcome};

/// One pop, as it is recorded for comparison.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PopRecord {
    pub tick: u64,
    pub kind: Kind,
    pub seq: u64,
    pub outcome: Outcome,
    /// The job number, or the server index, or -1 for a cancel of a key with no job.
    pub subject: i64,
}
