//! Where everything is, in the scene's own coordinates. The projection does not use it, and the surface does.

use crate::projection::QUEUE_SLOTS;

/// The scene is drawn in a view box of this size.
pub const VIEW: (f64, f64) = (1000.0, 480.0);

/// The queue window is a column of twelve slots, with the "more" line under it.
pub const QUEUE_ORIGIN: (f64, f64) = (24.0, 64.0);
pub const QUEUE_SLOT: (f64, f64) = (210.0, 26.0);
pub const QUEUE_GAP: f64 = 4.0;

/// Where the top of slot `index` is.
#[must_use]
pub fn queue_slot_at(index: usize) -> (f64, f64) {
    (QUEUE_ORIGIN.0, QUEUE_ORIGIN.1 + index as f64 * (QUEUE_SLOT.1 + QUEUE_GAP))
}

/// Where the "more" line is: under the last slot.
#[must_use]
pub fn more_at() -> (f64, f64) {
    queue_slot_at(QUEUE_SLOTS)
}

/// The node that stands for the waiting queue, and the servers it feeds.
pub const QUEUE_NODE: ((f64, f64), (f64, f64)) = ((300.0, 190.0), (120.0, 56.0));
pub const SERVER_SIZE: (f64, f64) = (150.0, 64.0);
pub const SERVER_X: f64 = 500.0;
pub const SERVER_PITCH: f64 = 92.0;

/// Where the top left of server `index` is.
#[must_use]
pub fn server_at(index: usize) -> (f64, f64) {
    (SERVER_X, 64.0 + index as f64 * SERVER_PITCH)
}

/// The data node: eight cells of counters.
pub const DATA_AT: (f64, f64) = (720.0, 64.0);

/// The counters under it, one line each.
pub const COUNTERS_AT: (f64, f64) = (730.0, 210.0);
pub const COUNTER_PITCH: f64 = 24.0;

/// The clock and the selected job, across the top and the bottom.
pub const CLOCK_AT: (f64, f64) = (24.0, 30.0);
pub const SELECTED_AT: (f64, f64) = (24.0, 470.0);
