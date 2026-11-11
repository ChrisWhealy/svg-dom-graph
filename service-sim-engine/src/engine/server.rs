//! What one server is doing: nothing, serving one job until a tick, or down.

/// What one server is doing: nothing, serving one job until a tick, or down.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Server {
    Idle,
    Busy { job: u64, due: u64 },
    Down,
}
