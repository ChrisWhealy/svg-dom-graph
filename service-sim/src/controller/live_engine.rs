//! What the controller's own engine holds.

/// What the controller's own engine holds. The slice that its counters say is read from the engine, and the slice that
/// the controller believes it is at is the controller's own record, so a disagreement between them is a defect of the
/// bookkeeping.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LiveEngine {
    /// The slice that the controller recorded when it last stepped or replaced the engine.
    pub recorded_at: u64,
    /// The slice that the engine's own counters say it has done.
    pub slice: u64,
    /// The hash that the engine makes of its whole state, as it does at the end of that slice.
    pub sha256: String,
}
