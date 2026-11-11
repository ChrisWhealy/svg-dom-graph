//! A seek that is being replayed, a slice or a frame's worth at a time.

use service_sim_engine::Engine;

/// A seek that is being replayed, a slice or a frame's worth at a time.
pub(super) struct SeekJob {
    pub(super) target: u64,
    pub(super) engine: Engine,
    pub(super) at: u64,
}
