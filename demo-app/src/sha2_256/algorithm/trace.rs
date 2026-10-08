use super::*;

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Everything one block's hash goes through, from the padded block to the digest.
pub(in crate::sha2_256) struct Trace {
    /// The padded message block.
    pub(in crate::sha2_256) block: [u32; BLOCK_WORDS],
    /// The message schedule built from it.
    pub(in crate::sha2_256) schedule: [u32; SCHEDULE_WORDS],
    /// The 64 rounds, in order. Round `i`'s `after` is round `i + 1`'s `before`.
    pub(in crate::sha2_256) rounds: [Round; ROUNDS],
    /// The working variables after the last round.
    pub(in crate::sha2_256) compressed: [u32; 8],
    /// The digest: each initial hash word plus its compressed working variable, wrapping.
    pub(in crate::sha2_256) digest: [u32; 8],
}

/// Hashes `message`, keeping every step. `None` if `message` is longer than [`MAX_MESSAGE_LEN`].
///
/// The working variables after the last round are not the digest on their own. The standard adds each one to the initial
/// hash word it started from, so that the compression feeds forward from the state it was given. That final addition is
/// what makes this a hash of a block rather than just a permutation of it.
pub(in crate::sha2_256) fn trace(message: &[u8]) -> Option<Trace> {
    let block = message_block(message)?;
    let schedule = message_schedule(&block);

    let mut state = INITIAL_HASH;
    let rounds: [Round; ROUNDS] = std::array::from_fn(|i| {
        let r = round(state, ROUND_CONSTANTS[i], schedule[i]);
        state = r.after;
        r
    });
    let digest = std::array::from_fn(|i| INITIAL_HASH[i].wrapping_add(state[i]));
    Some(Trace {
        block,
        schedule,
        rounds,
        compressed: state,
        digest,
    })
}
