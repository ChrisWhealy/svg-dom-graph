//! Where everything is, in the scene's own coordinates. The projection does not use it, and the surface does.

use crate::projection::QUEUE_SLOTS;

/// The scene is drawn in a view box of this size.
pub const VIEW: (f64, f64) = (1000.0, 480.0);

/// The queue window is a column of twelve slots, with the "more" line under it.
pub const QUEUE_ORIGIN: (f64, f64) = (24.0, 64.0);
pub const QUEUE_SLOT: (f64, f64) = (210.0, 26.0);
pub const QUEUE_GAP: f64 = 4.0;

/// The node that stands for the waiting queue, and the servers it feeds.
pub const QUEUE_NODE_X: f64 = 300.0;
pub const QUEUE_NODE_SIZE: (f64, f64) = (120.0, 56.0);
pub const SERVER_SIZE: (f64, f64) = (150.0, 64.0);
pub const SERVER_X: f64 = 500.0;
pub const SERVER_PITCH: f64 = 92.0;
const SERVER_TOP: f64 = 64.0;

/// The data node: eight cells of counters.
pub const DATA_AT: (f64, f64) = (700.0, 64.0);

/// What each row of the data node's cells is, left to right. The cells are two to a row, and each row is named beside
/// it.
pub const DATA_ROWS: [&str; 4] = [
    "snapshot · tick",
    "waiting · busy",
    "completed · timed out",
    "cancelled · rejected",
];
/// The cells are as wide as this number is when it is drawn, which is room for six digits.
pub const DATA_CELL_SAMPLE: u64 = 999_999;

/// The counters under it, one line each. The value is this far to the right of its name, which is room for the
/// longest name.
pub const COUNTERS_AT: (f64, f64) = (730.0, 278.0);
pub const COUNTER_PITCH: f64 = 21.0;
pub const COUNTER_VALUE_DX: f64 = 176.0;

/// The clock and the selected job, across the top and the bottom.
pub const CLOCK_AT: (f64, f64) = (24.0, 30.0);
pub const SELECTED_AT: (f64, f64) = (24.0, 470.0);

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Where the top of slot `index` is.
#[must_use]
pub fn queue_slot_at(index: usize) -> (f64, f64) {
    (QUEUE_ORIGIN.0, QUEUE_ORIGIN.1 + index as f64 * (QUEUE_SLOT.1 + QUEUE_GAP))
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Where the "more" line is: under the last slot.
#[must_use]
pub fn more_at() -> (f64, f64) {
    queue_slot_at(QUEUE_SLOTS)
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Where the top left of the dispatch node is, for a surface with `servers` servers.
///
/// Its horizontal midline is the common midline of the column of servers: halfway between the top of the first and the
/// bottom of the last. So it is a server's own midline when the number is odd, and between two when it is even.
#[must_use]
pub fn queue_node_at(servers: usize) -> (f64, f64) {
    let last = servers.max(1) - 1;
    let centre = SERVER_TOP + (last as f64 * SERVER_PITCH + SERVER_SIZE.1) / 2.0;
    (QUEUE_NODE_X, centre - QUEUE_NODE_SIZE.1 / 2.0)
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Where the top left of server `index` is.
#[must_use]
pub fn server_at(index: usize) -> (f64, f64) {
    (SERVER_X, SERVER_TOP + index as f64 * SERVER_PITCH)
}
