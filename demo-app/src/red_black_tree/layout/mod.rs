//! Where each key is. Pure: nothing here touches the DOM.
//!
//! A key's place along the row is fixed by its rank in the keys of the fixture, so that a key never moves sideways
//! because another one is gone. Its depth is worked out again for each snapshot.

use super::solver::{Key, Snapshot};

/// The size of the scene, and the radius a node starts from before its label makes it larger.
pub(crate) const VIEW_WIDTH: f64 = 760.0;
pub(crate) const VIEW_HEIGHT: f64 = 540.0;
pub(crate) const RADIUS: f64 = 28.0;

/// The keys of the fixture, whose ranks give the places along the row.
pub(crate) const UNIVERSE: [Key; 6] = [1, 5, 10, 12, 15, 20];

/// The strip above the tree that holds the controls, and the size of the whole stage.
pub(crate) const CONTROLS_HEIGHT: f64 = 132.0;
pub(crate) const STAGE_HEIGHT: f64 = CONTROLS_HEIGHT + VIEW_HEIGHT;

/// Each button: its action, its label, and where it is, as the left edge, the top, and the width. The height of every
/// button is [`BUTTON_HEIGHT`]. The first row steps and plays, and the second jumps.
pub(crate) const BUTTONS: [(&str, &str, f64, f64, f64); 7] = [
    ("restart", "Restart", 20.0, 12.0, 110.0),
    ("previous", "Previous", 140.0, 12.0, 110.0),
    ("next", "Next", 260.0, 12.0, 110.0),
    ("play", "Play", 380.0, 12.0, 110.0),
    ("jump-0", "Insert 1: right-rotate at 10", 20.0, 52.0, 232.0),
    ("jump-1", "Insert 20: left-rotate at 10", 262.0, 52.0, 232.0),
    ("jump-2", "Delete 15: choose the successor", 504.0, 52.0, 236.0),
];
pub(crate) const BUTTON_HEIGHT: f64 = 30.0;

/// The range that goes to a step: the left end and the length of its track, and the height of its middle.
pub(crate) const RANGE_LEFT: f64 = 20.0;
pub(crate) const RANGE_LENGTH: f64 = VIEW_WIDTH - 2.0 * RANGE_LEFT;
pub(crate) const RANGE_MIDDLE: f64 = 110.0;

const LEFT: f64 = 100.0;
const COLUMN: f64 = 106.0;
const TOP: f64 = 96.0;
const ROW: f64 = 105.0;

/// The centre of every key of `s`, in key order. A tree whose keys are not all in the fixture is spread by the rank of
/// each key among the keys that are in the tree.
pub(crate) fn centres(s: &Snapshot) -> Vec<(Key, (f64, f64))> {
    let in_fixture = s.nodes.iter().all(|n| UNIVERSE.contains(&n.key));
    s.nodes
        .iter()
        .enumerate()
        .map(|(rank, n)| {
            let rank = if in_fixture {
                UNIVERSE.iter().position(|&k| k == n.key).unwrap_or(rank)
            } else {
                rank
            };
            (n.key, (LEFT + rank as f64 * COLUMN, TOP + depth(s, n.key) as f64 * ROW))
        })
        .collect()
}

/// The number of parents above `key`.
fn depth(s: &Snapshot, key: Key) -> usize {
    let mut depth = 0;
    let mut at = s.node(key).and_then(|n| n.parent);
    while let Some(p) = at {
        depth += 1;
        // A cycle cannot occur in a snapshot, and this stops a malformed one all the same.
        if depth > s.nodes.len() {
            break;
        }
        at = s.node(p).and_then(|n| n.parent);
    }
    depth
}

#[cfg(test)]
mod unit_tests;
