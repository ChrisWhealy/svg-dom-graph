//! Which step of the walk the diagram is at, and so which boxes have been reached. Pure, so none of it needs a browser.
//!
//! The boxes form one path, and every box but the first is the result of a step. Position `0` has only the data.
//! Position `n` has reached box `n`, so the last position is one less than the number of boxes.

/// How many boxes the path has: the data, the parity bits, the codeword, the received codeword, the checks, the
/// syndrome, the error position, the corrected codeword, and the recovered data.
pub(super) const BOXES: usize = 9;

/// How many positions the walk has: one for each box, and the first box needs no step.
pub(super) const POSITIONS: usize = BOXES;

/// The position at which the received codeword is reached, since the error cannot be seen before then.
pub(super) const ARRIVAL: usize = 3;

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Whether box `index` has been reached at `position`.
pub(super) fn reached(index: usize, position: usize) -> bool {
    index <= position
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The box that `position` has just reached, if it is not the first.
pub(super) fn newest(position: usize) -> Option<usize> {
    (position > 0).then(|| position.min(BOXES - 1))
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[cfg(test)]
mod unit_tests;
