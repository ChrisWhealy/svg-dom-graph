use super::*;

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Which step of the hash a position is.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Phase {
    Message,
    Block,
    Copy,
    /// Building schedule word `n`, `16..64`.
    Expand(usize),
    /// Running round `i`, `0..64`.
    Round(usize),
    Final,
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The step `position` is. A position past the end is the last step.
pub(super) fn phase(position: usize) -> Phase {
    match position {
        0 => Phase::Message,
        1 => Phase::Block,
        2 => Phase::Copy,
        p if p < ROUND_START => Phase::Expand(BLOCK_WORDS + (p - EXPAND_START)),
        p if p < FINAL => Phase::Round(p - ROUND_START),
        _ => Phase::Final,
    }
}
