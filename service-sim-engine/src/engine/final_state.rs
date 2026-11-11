//! The state of every job and server at the end of a run.

use super::Counters;

/// The state of every job and server at the end of a run.
#[derive(Debug, Clone)]
pub struct Final {
    pub counters: Counters,
    pub jobs: Vec<(u64, String, String)>,
    pub servers: Vec<&'static str>,
    pub event_pop_sha256: String,
}
