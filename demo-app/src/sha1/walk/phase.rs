use super::*;

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Which step of the hash a position is.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Phase {
    Message,
    Block,
    Copy,
    /// Building schedule word `n`, `16..80`.
    Expand(usize),
    /// Running round `i`, `0..80`.
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
/// The part of the hash the diagram is drawn for. The diagram draws only what that part uses.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(in crate::sha1) enum Stage {
    /// Building the message schedule: the message, its block, and the schedule's 80 words. The working variables, the
    /// initial hash and the digest are not yet in use.
    Expansion,
    /// One section of 20 rounds of compression. Every round in a section uses the same round function and the same
    /// round constant, so the diagram names them. The last section also holds the digest the 80 rounds lead to.
    Compression(usize),
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Which stage `position` is in. A position past the end is in the last section.
pub(in crate::sha1) fn stage(position: usize) -> Stage {
    match phase(position) {
        Phase::Message | Phase::Block | Phase::Copy | Phase::Expand(_) => Stage::Expansion,
        Phase::Round(i) => Stage::Compression(section_of(i)),
        Phase::Final | Phase::Hash => Stage::Compression(SECTIONS - 1),
    }
}
