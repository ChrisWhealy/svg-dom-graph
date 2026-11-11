//! What a controller changes as it runs: the cursor, the engine and the counters that keep a stale callback inert.

use super::{Factory, SeekJob, Staged};
use crate::projection::{Projection, View};
use service_sim_engine::{Engine, Model};
use std::collections::VecDeque;

/// What a controller changes as it runs: the cursor, the engine and the counters that keep a stale callback inert.
pub(super) struct State {
    pub(super) factory: Factory,
    /// The engine, which may be ahead of the committed slice after a draw that failed. It is only ever read at the
    /// slice it is at.
    pub(super) engine: Engine,
    pub(super) engine_at: u64,
    pub(super) staged: Option<Staged>,
    /// Slices worked out in advance by [`Controller::prepare_ahead`], oldest first, each for the view it was made for.
    pub(super) ahead: VecDeque<Staged>,
    pub(super) committed_slice: u64,
    pub(super) committed_model: Model,
    pub(super) committed: Projection,
    pub(super) view: View,
    pub(super) last_slice: Option<u64>,
    pub(super) generation: u64,
    pub(super) epoch: u64,
    pub(super) seek_token: u64,
    pub(super) timer: Option<u64>,
    pub(super) frame: Option<u64>,
    pub(super) playing: bool,
    pub(super) dirty: bool,
    pub(super) disposed: bool,
    pub(super) reported: bool,
    pub(super) announced: bool,
    pub(super) speed_ms: u32,
    pub(super) chunk: Option<u64>,
    pub(super) seek: Option<SeekJob>,
    pub(super) retry: Option<u64>,
}
