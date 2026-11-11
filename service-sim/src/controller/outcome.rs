//! What a request came to.

/// What a request came to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Outcome {
    /// The slice that was asked for is on the page and is the committed one.
    Done,
    /// There was nowhere to go: the first slice or the last. A page that was dirty was repaired first.
    AtBoundary,
    /// A seek was asked for and will arrive at a frame.
    Pending,
    /// The host is hidden, detached or disposed, so nothing was drawn and nothing was committed.
    Unavailable,
    /// A write or a schedule failed, or the engine refused. The cursor is where it was.
    Failed,
}
