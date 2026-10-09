//! SHA-256 (FIPS 180-4) for one message block, in plain Rust, split into the same named pieces the demo draws.
//!
//! Every function here is a direct reading of the standard's own definitions, and every value the demo shows is one of
//! these results. The two tables of constants, [`INITIAL_HASH`] and [`ROUND_CONSTANTS`], are fixed tables, not computed
//! at run time. Their values are recomputed independently from the primes, with exact integer arithmetic, and verified
//! by the unit tests.
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
pub(super) const SCHEDULE_WORDS: usize = 64;
/// Rounds of compression, one per schedule word.
pub(super) const ROUNDS: usize = 64;
/// The longest message that fits one block: 64 bytes, less the terminator byte and the 8-byte length.
pub(super) const MAX_MESSAGE_LEN: usize = 55;

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The initial hash value `H`: the first 32 bits of the fractional parts of the square roots of the first 8 primes.
/// `H[0]` is working variable `a`, and `H[7]` is `h`.
///
/// Fixed by the standard, so it is a constant rather than something computed at run time. The tests recompute it from
/// the primes and check that this table agrees.
#[rustfmt::skip]
pub(super) const INITIAL_HASH: [u32; 8] = [
    0x6a09_e667, 0xbb67_ae85,
    0x3c6e_f372, 0xa54f_f53a,
    0x510e_527f, 0x9b05_688c,
    0x1f83_d9ab, 0x5be0_cd19,
];

/// The round constants `K`: the first 32 bits of the fractional parts of the cube roots of the first 64 primes. Fixed
/// by the standard, and checked against a recomputation in the tests, as [`INITIAL_HASH`] is.
#[rustfmt::skip]
pub(super) const ROUND_CONSTANTS: [u32; ROUNDS] = [
    0x428a_2f98, 0x7137_4491, 0xb5c0_fbcf, 0xe9b5_dba5,
    0x3956_c25b, 0x59f1_11f1, 0x923f_82a4, 0xab1c_5ed5,
    0xd807_aa98, 0x1283_5b01, 0x2431_85be, 0x550c_7dc3,
    0x72be_5d74, 0x80de_b1fe, 0x9bdc_06a7, 0xc19b_f174,
    0xe49b_69c1, 0xefbe_4786, 0x0fc1_9dc6, 0x240c_a1cc,
    0x2de9_2c6f, 0x4a74_84aa, 0x5cb0_a9dc, 0x76f9_88da,
    0x983e_5152, 0xa831_c66d, 0xb003_27c8, 0xbf59_7fc7,
    0xc6e0_0bf3, 0xd5a7_9147, 0x06ca_6351, 0x1429_2967,
    0x27b7_0a85, 0x2e1b_2138, 0x4d2c_6dfc, 0x5338_0d13,
    0x650a_7354, 0x766a_0abb, 0x81c2_c92e, 0x9272_2c85,
    0xa2bf_e8a1, 0xa81a_664b, 0xc24b_8b70, 0xc76c_51a3,
    0xd192_e819, 0xd699_0624, 0xf40e_3585, 0x106a_a070,
    0x19a4_c116, 0x1e37_6c08, 0x2748_774c, 0x34b0_bcb5,
    0x391c_0cb3, 0x4ed8_aa4a, 0x5b9c_ca4f, 0x682e_6ff3,
    0x748f_82ee, 0x78a5_636f, 0x84c8_7814, 0x8cc7_0208,
    0x90be_fffa, 0xa450_6ceb, 0xbef9_a3f7, 0xc671_78f2,
];

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The message schedule's general purpose small sigma
fn sigma_using(w: u32, r1: u32, r2: u32, sh: u32) -> u32 {
    w.rotate_right(r1) ^ w.rotate_right(r2) ^ w >> sh
}

/// The message schedule's small sigma 0: `ROTR(w, 7) XOR ROTR(w, 18) XOR SHR(w, 3)`.
pub(super) fn sigma0(w: u32) -> u32 {
    sigma_using(w, 7, 18, 3)
}

/// The message schedule's small sigma 1: `ROTR(w, 17) XOR ROTR(w, 19) XOR SHR(w, 10)`.
pub(super) fn sigma1(w: u32) -> u32 {
    sigma_using(w, 17, 19, 10)
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Compression's general purpose big
fn big_sigma_using(w: u32, r1: u32, r2: u32, r3: u32) -> u32 {
    w.rotate_right(r1) ^ w.rotate_right(r2) ^ w.rotate_right(r3)
}

/// The compression's big sigma 0, applied to `a`: `ROTR(w, 2) XOR ROTR(w, 13) XOR ROTR(w, 22)`.
pub(super) fn big_sigma0(w: u32) -> u32 {
    big_sigma_using(w, 2, 13, 22)
}

/// The compression's big sigma 1, applied to `e`: `ROTR(w, 6) XOR ROTR(w, 11) XOR ROTR(w, 25)`.
pub(super) fn big_sigma1(w: u32) -> u32 {
    big_sigma_using(w, 6, 11, 25)
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Choose: each bit of `e` picks the matching bit of `f` where it is set, and of `g` where it is clear.
pub(super) fn choice(e: u32, f: u32, g: u32) -> u32 {
    (e & f) ^ (!e & g)
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Majority: each result bit is the value held by at least two of the matching bits of `a`, `b` and `c`.
pub(super) fn majority(a: u32, b: u32, c: u32) -> u32 {
    (a & b) ^ (a & c) ^ (b & c)
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
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

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Schedule word `n`, for `n >= 16`, from the earlier words of `schedule`: `ms[n-16] + sigma0(ms[n-15]) + ms[n-7] +
/// sigma1(ms[n-2])`, with overflow ignored.
pub(super) fn expanded_word(schedule: &[u32; SCHEDULE_WORDS], n: usize) -> u32 {
    schedule[n - 16]
        .wrapping_add(sigma0(schedule[n - 15]))
        .wrapping_add(schedule[n - 7])
        .wrapping_add(sigma1(schedule[n - 2]))
}

/// The message schedule: the block's 16 words, then 48 more, each from [`expanded_word`].
pub(super) fn message_schedule(block: &[u32; BLOCK_WORDS]) -> [u32; SCHEDULE_WORDS] {
    let mut schedule = [0_u32; SCHEDULE_WORDS];
    schedule[..BLOCK_WORDS].copy_from_slice(block);
    for n in BLOCK_WORDS..SCHEDULE_WORDS {
        schedule[n] = expanded_word(&schedule, n);
    }
    schedule
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The digest as the 64 lowercase hexadecimal digits `sha256sum` prints: its 8 words, each as 8 digits, joined.
pub(super) fn digest_hex(digest: &[u32; 8]) -> String {
    digest.iter().map(|word| format!("{word:08x}")).collect()
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[cfg(test)]
mod unit_tests;
