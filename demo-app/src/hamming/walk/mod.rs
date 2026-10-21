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
/// What each position is, in a few words: the name of its box and what is done to arrive there.
const STAGES: [&str; BOXES] = [
    "Data bits. The four bits to protect.",
    "Parity bits. Each makes its group of bits hold an even number of 1s.",
    "Codeword. The data and parity bits in positions 1 to 7.",
    "Received codeword. The channel may have flipped one bit.",
    "Check bits. The three groups worked out again over the bits that arrived.",
    "Syndrome. The three checks read as a binary number.",
    "Error position. The syndrome, which is the position of the flipped bit, or 0 if none.",
    "Corrected codeword. The bit at the error position is flipped back.",
    "Recovered data. Read from positions 3, 5, 6 and 7.",
];

/// What a screen reader is told when the walk arrives at `position`, which it cannot see: how far along it is, and what
/// the box it has reached is.
pub(super) fn announcement(position: usize) -> String {
    let index = position.min(BOXES - 1);
    format!("Step {} of {BOXES}: {}", index + 1, STAGES[index])
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[cfg(test)]
mod unit_tests;
