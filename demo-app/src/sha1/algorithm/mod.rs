//! SHA-1 (FIPS 180-4) for one message block, in plain Rust, split into the same named pieces the demo draws.
//!
//! Every function here is a direct reading of the standard's own definitions, and every value the demo shows is one of
//! these results. The two tables of constants, [`INITIAL_HASH`] and [`ROUND_CONSTANTS`], are fixed tables, not computed
//! at run time. The tests check them against the published values and check the digests of several messages.
//!
//! # Scope
//!
//! One block only: a message of up to [`MAX_MESSAGE_LEN`] bytes, so that the `0x80` terminator and the 64-bit length
//! still fit in the 64-byte block. A longer message needs several blocks, each chained into the next, which this does
//! not do.

mod round;
mod trace;

pub(super) use round::{Round, round};
pub(super) use trace::{Trace, trace};

/// Words in the message block.
pub(super) const BLOCK_WORDS: usize = 16;
/// Words in the message schedule.
pub(super) const SCHEDULE_WORDS: usize = 80;
/// Rounds of compression, one per schedule word.
pub(super) const ROUNDS: usize = 80;
/// Rounds that share one round function and one round constant.
pub(super) const ROUNDS_PER_SECTION: usize = 20;
/// Sections of rounds: four, each with its own round function and round constant.
pub(super) const SECTIONS: usize = ROUNDS / ROUNDS_PER_SECTION;
/// The longest message that fits one block: 64 bytes, less the terminator byte and the 8-byte length.
pub(super) const MAX_MESSAGE_LEN: usize = 55;

/// The initial hash value `H`. `H[0]` is working variable `a`, and `H[4]` is `e`.
pub(super) const INITIAL_HASH: [u32; 5] = [0x6745_2301, 0xefcd_ab89, 0x98ba_dcfe, 0x1032_5476, 0xc3d2_e1f0];

/// The four round constants `K`, one for each section of 20 rounds. The standard derives them from the square roots of
/// 2, 3, 5 and 10.
pub(super) const ROUND_CONSTANTS: [u32; SECTIONS] = [0x5a82_7999, 0x6ed9_eba1, 0x8f1b_bcdc, 0xca62_c1d6];

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The round function a section of rounds uses.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum RoundFunction {
    /// Rounds 0 to 19: each bit of `b` picks the matching bit of `c` where it is set, and of `d` where it is clear.
    Choice,
    /// Rounds 20 to 39 and 60 to 79: the XOR of `b`, `c` and `d`.
    Parity,
    /// Rounds 40 to 59: each result bit is the value held by at least two of the matching bits of `b`, `c` and `d`.
    Majority,
}

impl RoundFunction {
    /// The round function section `section` uses.
    pub(super) fn of_section(section: usize) -> Self {
        match section {
            0 => Self::Choice,
            2 => Self::Majority,
            _ => Self::Parity,
        }
    }

    /// Applies this function to `b`, `c` and `d`.
    pub(super) fn apply(self, b: u32, c: u32, d: u32) -> u32 {
        match self {
            Self::Choice => (b & c) ^ (!b & d),
            Self::Parity => b ^ c ^ d,
            Self::Majority => (b & c) ^ (b & d) ^ (c & d),
        }
    }
}

/// The section round `i` belongs to.
pub(super) fn section_of(round: usize) -> usize {
    (round / ROUNDS_PER_SECTION).min(SECTIONS - 1)
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The padded message block: `message`'s bytes, then the `0x80` terminator, then zeros, with the message's length in
/// bits as the last word. Words are big-endian, so the first byte of the message is the top byte of word 0.
///
/// Returns `None` if `message` is longer than [`MAX_MESSAGE_LEN`].
pub(super) fn message_block(message: &[u8]) -> Option<[u32; BLOCK_WORDS]> {
    if message.len() > MAX_MESSAGE_LEN {
        return None;
    }
    let mut bytes = [0_u8; BLOCK_WORDS * 4];
    bytes[..message.len()].copy_from_slice(message);
    bytes[message.len()] = 0x80;
    let mut block: [u32; BLOCK_WORDS] = std::array::from_fn(|i| {
        u32::from_be_bytes([bytes[4 * i], bytes[4 * i + 1], bytes[4 * i + 2], bytes[4 * i + 3]])
    });
    // The length is 64 bits wide. A single block's message is far shorter than 2^32 bits, so its top word stays zero.
    block[BLOCK_WORDS - 1] = (message.len() as u32) * 8;
    Some(block)
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The four earlier schedule words schedule word `n` is built from, in the order the standard names them: `ms[n-3]`,
/// `ms[n-8]`, `ms[n-14]` and `ms[n-16]`.
pub(super) fn expansion_inputs(schedule: &[u32; SCHEDULE_WORDS], n: usize) -> [u32; 4] {
    [schedule[n - 3], schedule[n - 8], schedule[n - 14], schedule[n - 16]]
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Schedule word `n`, for `n >= 16`, from the earlier words of `schedule`: the XOR of the four words
/// [`expansion_inputs`] names, rotated left by one bit.
pub(super) fn expanded_word(schedule: &[u32; SCHEDULE_WORDS], n: usize) -> u32 {
    expansion_xor(&expansion_inputs(schedule, n)).rotate_left(1)
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The XOR of the four words schedule word `n` is built from, before the rotation.
pub(super) fn expansion_xor(inputs: &[u32; 4]) -> u32 {
    inputs.iter().fold(0, |acc, w| acc ^ w)
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The message schedule: the block's 16 words, then 64 more, each from [`expanded_word`].
pub(super) fn message_schedule(block: &[u32; BLOCK_WORDS]) -> [u32; SCHEDULE_WORDS] {
    let mut schedule = [0_u32; SCHEDULE_WORDS];
    schedule[..BLOCK_WORDS].copy_from_slice(block);
    for n in BLOCK_WORDS..SCHEDULE_WORDS {
        schedule[n] = expanded_word(&schedule, n);
    }
    schedule
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The digest as the 40 lowercase hexadecimal digits `sha1sum` prints: its 5 words, each as 8 digits, joined.
pub(super) fn digest_hex(digest: &[u32; 5]) -> String {
    digest.iter().map(|word| format!("{word:08x}")).collect()
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[cfg(test)]
mod unit_tests;
