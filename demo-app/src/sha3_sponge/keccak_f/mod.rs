//! SHA3's `Keccak-f\[1600\]` permutation as plain, pure Rust: each of its five step functions is an ordinary
//! transformation of an explicitly supplied 25-lane state, with no DOM and no demo data in sight.
//!
//! ```text
//! theta(state) -> rho(state) -> pi(state) -> chi(state) -> iota(state, round)
//! ```
//!
//! Every nested scene is given the state it displays from here, so every number a child scene shows is derived from the
//! number immediately upstream of it: [`round_trace`] returns each intermediate state of one round, and [`keccak_f`]
//! chains 24 of them. Lane `x + 5y` holds `A[x, y]`, as in FIPS 202, and the bytes of a lane are little-endian.

use super::{chi::chi, iota::iota, pi::pi, rho::rho};

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// How many rounds `Keccak-f\[1600\]` actually runs.
pub(crate) const ROUND_COUNT: usize = 24;

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The real Keccak-f\[1600\] round constants, `RC[0..24]` — see keccak.team's own specification summary
/// (<https://keccak.team/keccak_specs_summary.html>) and FIPS 202 section 3.2.5.
pub(super) const ROUND_CONSTANTS: [u64; ROUND_COUNT] = [
    0x0000_0000_0000_0001,
    0x0000_0000_0000_8082,
    0x8000_0000_0000_808a,
    0x8000_0000_8000_8000,
    0x0000_0000_0000_808b,
    0x0000_0000_8000_0001,
    0x8000_0000_8000_8081,
    0x8000_0000_0000_8009,
    0x0000_0000_0000_008a,
    0x0000_0000_0000_0088,
    0x0000_0000_8000_8009,
    0x0000_0000_8000_000a,
    0x0000_0000_8000_808b,
    0x8000_0000_0000_008b,
    0x8000_0000_0000_8089,
    0x8000_0000_0000_8003,
    0x8000_0000_0000_8002,
    0x8000_0000_0000_0080,
    0x0000_0000_0000_800a,
    0x8000_0000_8000_000a,
    0x8000_0000_8000_8081,
    0x8000_0000_0000_8080,
    0x0000_0000_8000_0001,
    0x8000_0000_8000_8008,
];

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// SHA3's real `Theta`: every lane is XORed with `D[x]`, where `D[x] = C[x - 1] ^ rotl(C[x + 1], 1)` and `C[x]` is the
/// XOR of column `x`.
pub(super) fn theta(state: [u64; 25]) -> [u64; 25] {
    let c: [u64; 5] = std::array::from_fn(|x| (0..5).fold(0, |acc, y| acc ^ state[x + 5 * y]));
    let d: [u64; 5] = std::array::from_fn(|x| c[(x + 4) % 5] ^ c[(x + 1) % 5].rotate_left(1));
    std::array::from_fn(|lane| state[lane] ^ d[lane % 5])
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The state at the start of one round and after each of its five functions.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct RoundTrace {
    /// The state the round starts from — `Theta`'s own input.
    pub(super) input: [u64; 25],
    /// After `Theta` — `Rho`'s own input.
    pub(super) theta: [u64; 25],
    /// After `Rho` — `Pi`'s own input.
    pub(super) rho: [u64; 25],
    /// After `Pi` — `Chi`'s own input.
    pub(super) pi: [u64; 25],
    /// After `Chi` — `Iota`'s own input.
    pub(super) chi: [u64; 25],
    /// After `Iota` — the round's own output, and the next round's own input.
    pub(super) output: [u64; 25],
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Runs round `round` over `input`, keeping every intermediate state.
pub(super) fn round_trace(input: [u64; 25], round: usize) -> RoundTrace {
    let after_theta = theta(input);
    let after_rho = rho(after_theta);
    let after_pi = pi(after_rho);
    let after_chi = chi(after_pi);
    RoundTrace {
        input,
        theta: after_theta,
        rho: after_rho,
        pi: after_pi,
        chi: after_chi,
        output: iota(after_chi, round),
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// One round of `Keccak-f\[1600\]`.
#[cfg_attr(not(test), allow(dead_code))] // The demo walks rounds through `round_traces`; tests check the whole.
pub(super) fn keccak_round(state: [u64; 25], round: usize) -> [u64; 25] {
    round_trace(state, round).output
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Every round's own [`RoundTrace`], chained from `seed`: round `0` starts from `seed`, and every later round starts
/// from the previous round's own output.
pub(super) fn round_traces(seed: [u64; 25]) -> [RoundTrace; ROUND_COUNT] {
    let mut current = seed;
    std::array::from_fn(|round| {
        let trace = round_trace(current, round);
        current = trace.output;
        trace
    })
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The whole `Keccak-f\[1600\]` permutation: all 24 rounds.
pub(super) fn keccak_f(state: [u64; 25]) -> [u64; 25] {
    (0..ROUND_COUNT).fold(state, keccak_round)
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// `A[x][y]`, as `Theta`'s own nested scenes index their input: the state's lane `x + 5y` is `grid[x][y]`. Those scenes
/// work along `x` — "row `n`" there is column `n` of the state — so a state is handed to them transposed.
pub(super) fn to_theta_grid(state: [u64; 25]) -> [[u64; 5]; 5] {
    std::array::from_fn(|x| std::array::from_fn(|y| state[x + 5 * y]))
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The inverse of [`to_theta_grid`].
#[cfg_attr(not(test), allow(dead_code))] // The demo walks rounds through `round_traces`; tests check the whole.
pub(super) fn from_theta_grid(grid: [[u64; 5]; 5]) -> [u64; 25] {
    std::array::from_fn(|lane| grid[lane % 5][lane / 5])
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// `SHA3-256`'s own rate, in lanes: 1088 bits.
pub(super) const SHA3_256_RATE_LANES: usize = 17;

/// The one 136-byte block `SHA3-256` absorbs for `message`, as 17 little-endian lanes: the message, then the `0x06`
/// domain-separation byte, then zeros, with the final byte's top bit set (FIPS 202's `pad10*1`). Only for a message
/// short enough to fit one block — fewer than 136 bytes.
pub(super) fn sha3_256_block(message: &[u8]) -> [u64; SHA3_256_RATE_LANES] {
    assert!(
        message.len() < 8 * SHA3_256_RATE_LANES,
        "a message of one block or more needs more than one absorb"
    );
    let mut block = [0u8; 8 * SHA3_256_RATE_LANES];
    block[..message.len()].copy_from_slice(message);
    block[message.len()] ^= 0x06;
    block[8 * SHA3_256_RATE_LANES - 1] ^= 0x80;
    std::array::from_fn(|lane| u64::from_le_bytes(block[8 * lane..8 * lane + 8].try_into().expect("8 bytes")))
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// One complete `SHA3-256` of a short message, as the sponge diagram shows it: the padded block, the state after it is
/// absorbed, and the state after `Keccak-f\[1600\]`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct Sha3_256Run {
    /// The padded block — "Input block".
    pub(super) block: [u64; SHA3_256_RATE_LANES],
    /// The state entering `Keccak-f\[1600\]`: the block XORed into the rate of an all-zero state — the Keccak scene's
    /// own round `0` input.
    pub(super) absorbed: [u64; 25],
    /// The state leaving it — the Keccak scene's own last round's output, and the sponge's own row 3.
    pub(super) permuted: [u64; 25],
}

impl Sha3_256Run {
    /// The 32-byte digest: the first four lanes of `permuted`, little-endian.
    #[cfg_attr(not(test), allow(dead_code))] // The diagram shows the lanes; tests compare the bytes with published digests.
    pub(super) fn digest(&self) -> [u8; 32] {
        let mut digest = [0u8; 32];
        for (lane, bytes) in digest.chunks_mut(8).enumerate() {
            bytes.copy_from_slice(&self.permuted[lane].to_le_bytes());
        }
        digest
    }
}

/// Runs `SHA3-256` over a message shorter than one block.
pub(super) fn sha3_256_run(message: &[u8]) -> Sha3_256Run {
    let block = sha3_256_block(message);
    let mut absorbed = [0u64; 25];
    absorbed[..SHA3_256_RATE_LANES].copy_from_slice(&block);
    Sha3_256Run {
        block,
        absorbed,
        permuted: keccak_f(absorbed),
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[cfg(test)]
mod unit_tests;
