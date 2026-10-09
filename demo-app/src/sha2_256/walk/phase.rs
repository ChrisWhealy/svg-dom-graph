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
    /// The digest, written out as a hexadecimal hash value.
    Hash,
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
        p if p == FINAL => Phase::Final,
        _ => Phase::Hash,
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The two halves of the hash. The diagram shows only what the half in progress uses.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(in crate::sha2_256) enum Stage {
    /// Building the message schedule: the message, its block, and the schedule's 64 words. The working variables, the
    /// initial hash, the round constants and the digest are not yet in use.
    Expansion,
    /// The 64 rounds of compression, and the digest they lead to. The schedule is complete.
    Compression,
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Which half of the hash `position` is in. A position past the end is in the second.
pub(in crate::sha2_256) fn stage(position: usize) -> Stage {
    if position < ROUND_START { Stage::Expansion } else { Stage::Compression }
}
