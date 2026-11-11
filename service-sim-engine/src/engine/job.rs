//! One job, from its arrival to its end, with every counter its rules need.

use super::State;

/// One job, from its arrival to its end, with every counter its rules need.
#[derive(Debug, Clone)]
pub(super) struct Job {
    pub(super) id: u64,
    pub(super) key: String,
    pub(super) priority: u8,
    pub(super) durations: Vec<u64>,
    pub(super) state: State,
    pub(super) attempt: u32,
    pub(super) failures: u32,
    pub(super) token: u64,
    pub(super) defers: u32,
    pub(super) entered: u64,
    pub(super) server: usize,
}
