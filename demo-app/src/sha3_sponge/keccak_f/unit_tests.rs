use super::{
    ROUND_CONSTANTS, ROUND_COUNT, SHA3_256_RATE_LANES, from_theta_grid, keccak_f, keccak_round, round_trace,
    round_traces, sha3_256_block, sha3_256_run, theta, to_theta_grid,
};
use crate::sha3_sponge::{chi::chi, iota::iota, pi::pi, rho::rho};

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[test]
fn keccak_f_of_the_zero_state_starts_with_the_published_lane() {
    assert_eq!(keccak_f([0; 25])[0], 0xF125_8F79_40E1_DDE7);
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[test]
fn the_traces_chain_and_end_where_keccak_f_ends() {
    let seed: [u64; 25] = std::array::from_fn(|i| 0x9E37_79B9_7F4A_7C15_u64.wrapping_mul(i as u64 + 1));
    let traces = round_traces(seed);
    assert_eq!(traces[0].input, seed);
    for pair in traces.windows(2) {
        assert_eq!(
            pair[1].input, pair[0].output,
            "a round starts from the previous round's own output"
        );
    }
    assert_eq!(traces[23].output, keccak_f(seed));
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[test]
fn theta_agrees_with_the_theta_scenes_own_calculation() {
    let state: [u64; 25] = std::array::from_fn(|i| 0x0123_4567_89AB_CDEF_u64.rotate_left(i as u32) ^ i as u64);
    assert_eq!(from_theta_grid(to_theta_grid(state)), state);
    let from_scene = from_theta_grid(super::super::theta::output(to_theta_grid(state)));
    assert_eq!(theta(state), from_scene);
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
//   Acceptance tests, at four levels. Each higher level is checked against something the lower ones are not made from,
//   so the diagram's own data flow and the real SHA3 calculation cannot drift apart unnoticed:
//
// 1. every step function, against a second implementation written straight from FIPS 202's own formulas;
// 2. one complete round, against the same;
// 3. all 24 rounds, against the same, and against the zero state's published first lane;
// 4. one complete `SHA3-256` digest, against published known-answer vectors — and the diagram's own run of it.
//
// The reference below deliberately shares nothing with the code it checks. It indexes `A[x][y]` as the spec does,
// derives `Rho`'s offsets by walking the lanes (Algorithm 2) instead of reading the table, and derives the round
// constants from the spec's own linear-feedback shift register (Algorithm 5) instead of the list.

type Grid = [[u64; 5]; 5];

fn grid(state: [u64; 25]) -> Grid {
    std::array::from_fn(|x| std::array::from_fn(|y| state[x + 5 * y]))
}

fn flat(grid: Grid) -> [u64; 25] {
    std::array::from_fn(|lane| grid[lane % 5][lane / 5])
}

fn reference_theta(a: Grid) -> Grid {
    let c: [u64; 5] = std::array::from_fn(|x| a[x].iter().fold(0, |acc, lane| acc ^ lane));
    let d: [u64; 5] = std::array::from_fn(|x| c[(x + 4) % 5] ^ c[(x + 1) % 5].rotate_left(1));
    std::array::from_fn(|x| std::array::from_fn(|y| a[x][y] ^ d[x]))
}

/// FIPS 202 Algorithm 2: starting at `(1, 0)`, lane `t` is rotated by `(t + 1)(t + 2) / 2`, then the walk moves to `(y,
/// (2x + 3y) mod 5)`.
fn reference_rho(mut a: Grid) -> Grid {
    let (mut x, mut y) = (1, 0);
    for t in 0..24u32 {
        a[x][y] = a[x][y].rotate_left((t + 1) * (t + 2) / 2 % 64);
        (x, y) = (y, (2 * x + 3 * y) % 5);
    }
    a
}

/// FIPS 202 Algorithm 3: `A'[x, y] = A[(x + 3y) mod 5, x]`.
fn reference_pi(a: Grid) -> Grid {
    std::array::from_fn(|x| std::array::from_fn(|y| a[(x + 3 * y) % 5][x]))
}

/// FIPS 202 Algorithm 4.
fn reference_chi(a: Grid) -> Grid {
    std::array::from_fn(|x| std::array::from_fn(|y| a[x][y] ^ (!a[(x + 1) % 5][y] & a[(x + 2) % 5][y])))
}

/// FIPS 202 Algorithm 5: one bit of the round-constant LFSR, `x^8 + x^6 + x^5 + x^4 + 1`.
fn reference_rc_bit(t: usize) -> u64 {
    let mut register = 1u16;
    for _ in 0..t % 255 {
        register <<= 1;
        if register & 0x100 != 0 {
            register ^= 0x171;
        }
    }
    u64::from(register & 1)
}

/// FIPS 202 Algorithm 6: the round constant is seven LFSR bits, scattered to lane bits `2^j - 1`.
fn reference_round_constant(round: usize) -> u64 {
    (0..=6).fold(0, |rc, j| rc | reference_rc_bit(j + 7 * round) << ((1 << j) - 1))
}

fn reference_iota(mut a: Grid, round: usize) -> Grid {
    a[0][0] ^= reference_round_constant(round);
    a
}

fn reference_round(a: Grid, round: usize) -> Grid {
    reference_iota(reference_chi(reference_pi(reference_rho(reference_theta(a)))), round)
}

/// A handful of unrelated, full-width states, so a bug that only shows on a particular bit pattern cannot hide.
fn states() -> Vec<[u64; 25]> {
    (1..=6u64)
        .map(|seed| {
            std::array::from_fn(|lane| {
                let mut x =
                    seed.wrapping_mul(0x9E37_79B9_7F4A_7C15) ^ (lane as u64).wrapping_mul(0xBF58_476D_1CE4_E5B9);
                x ^= x >> 31;
                x = x.wrapping_mul(0x94D0_49BB_1331_11EB);
                x ^ (x >> 29)
            })
        })
        .collect()
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[test]
fn the_round_constant_table_matches_the_specs_own_lfsr() {
    for round in 0..ROUND_COUNT {
        assert_eq!(ROUND_CONSTANTS[round], reference_round_constant(round), "RC[{round}]");
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Level 1: each step function.
#[test]
fn each_step_function_matches_the_reference_written_from_the_spec() {
    for state in states() {
        let a = grid(state);
        assert_eq!(theta(state), flat(reference_theta(a)), "theta");
        assert_eq!(rho(state), flat(reference_rho(a)), "rho");
        assert_eq!(pi(state), flat(reference_pi(a)), "pi");
        assert_eq!(chi(state), flat(reference_chi(a)), "chi");
        for round in [0, 1, 7, 23] {
            assert_eq!(iota(state, round), flat(reference_iota(a, round)), "iota, round {round}");
        }
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Level 2: one complete round, and every intermediate state the nested scenes show.
#[test]
fn one_complete_round_and_each_of_its_intermediate_states_match_the_reference() {
    for state in states() {
        for round in [0, 5, 17, 23] {
            let trace = round_trace(state, round);
            let after_theta = reference_theta(grid(state));
            let after_rho = reference_rho(after_theta);
            let after_pi = reference_pi(after_rho);
            let after_chi = reference_chi(after_pi);
            assert_eq!(trace.input, state);
            assert_eq!(trace.theta, flat(after_theta), "round {round}: after theta");
            assert_eq!(trace.rho, flat(after_rho), "round {round}: after rho");
            assert_eq!(trace.pi, flat(after_pi), "round {round}: after pi");
            assert_eq!(trace.chi, flat(after_chi), "round {round}: after chi");
            assert_eq!(
                trace.output,
                flat(reference_iota(after_chi, round)),
                "round {round}: after iota"
            );
            assert_eq!(keccak_round(state, round), flat(reference_round(grid(state), round)));
        }
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Level 3: all 24 rounds, and every one of the 24 states a nested scene can be entered on.
#[test]
fn all_twenty_four_rounds_match_the_reference_and_every_traced_round_agrees() {
    for state in states() {
        let mut expected = grid(state);
        let traces = round_traces(state);
        for (round, trace) in traces.iter().enumerate() {
            assert_eq!(trace.input, flat(expected), "the state entering round {round}");
            expected = reference_round(expected, round);
            assert_eq!(trace.output, flat(expected), "the state leaving round {round}");
        }
        assert_eq!(keccak_f(state), flat(expected));
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Level 4: complete `SHA3-256` digests against published known answers — NIST's own example for the empty message and
/// for `"abc"`, and the widely quoted digest of a longer sentence — through the same `sha3_256_run` the diagram draws
/// from.
#[test]
fn sha3_256_digests_match_the_published_known_answers() {
    for (message, expected) in [
        (&b""[..], "a7ffc6f8bf1ed76651c14756a061d662f580ff4de43b49fa82d80a4b80f8434a"),
        (b"abc", "3a985da74fe225b2045c172d6bd390bd855f086e3e9d525b46bfe24511431532"),
        (
            b"The quick brown fox jumps over the lazy dog",
            "69070dda01975c8c120c3aada1b282394e7f032fa9cf32f4cb2259a0897dfc04",
        ),
    ] {
        let digest: String = sha3_256_run(message).digest().iter().map(|b| format!("{b:02x}")).collect();
        assert_eq!(digest, expected, "SHA3-256({:?})", String::from_utf8_lossy(message));
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The diagram's own data flow: the state the Keccak scene starts from is the sponge's own absorbed state, and the
/// state its last round leaves is the sponge's own row 3. So a reader following the diagram down through the nested
/// scenes is following the calculation that produced the digest.
#[test]
fn the_keccak_scenes_rounds_start_at_the_sponges_absorbed_state_and_end_at_its_output() {
    for message in [&b""[..], b"abc", b"The quick brown fox jumps over the lazy dog"] {
        let run = sha3_256_run(message);
        assert_eq!(
            run.absorbed[..SHA3_256_RATE_LANES],
            sha3_256_block(message),
            "the rate holds the block"
        );
        assert_eq!(
            run.absorbed[SHA3_256_RATE_LANES..],
            [0; 25 - SHA3_256_RATE_LANES],
            "the capacity is untouched"
        );
        let traces = round_traces(run.absorbed);
        assert_eq!(traces[0].input, run.absorbed);
        assert_eq!(traces[ROUND_COUNT - 1].output, run.permuted);
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The padded block as bytes, the way the sponge absorbs it: each lane little-endian.
fn block_bytes(message: &[u8]) -> Vec<u8> {
    sha3_256_block(message).iter().flat_map(|lane| lane.to_le_bytes()).collect()
}

/// `pad10*1` with SHA3's `06` suffix: the padding byte follows the message, and the final byte of the 136-byte block
/// has its top bit set. They are separate bytes for every message length but one, and these are the edge cases a
/// rewrite of the padding is most likely to break.
#[test]
fn padding_puts_06_after_the_message_and_80_at_the_end_of_the_block() {
    let bytes = block_bytes(b"");
    assert_eq!((bytes[0], bytes[135]), (0x06, 0x80), "an empty message");
    assert!(bytes[1..135].iter().all(|b| *b == 0), "everything between is zero");

    let message = [0xAB; 100];
    let bytes = block_bytes(&message);
    assert_eq!(bytes[..100], message, "the message is copied unchanged");
    assert_eq!((bytes[100], bytes[135]), (0x06, 0x80));
    assert!(bytes[101..135].iter().all(|b| *b == 0));
}

#[test]
fn a_message_one_byte_short_of_a_full_block_shares_the_last_byte_between_06_and_80() {
    // 135 bytes: the `06` and the `80` land on the same, final byte, which is therefore `86`.
    let message = [0x11; 135];
    let bytes = block_bytes(&message);
    assert_eq!(bytes[..135], message);
    assert_eq!(bytes[135], 0x86);
}

#[test]
fn a_message_two_bytes_short_of_a_full_block_ends_06_80() {
    // 134 bytes: the last two bytes are `06` then `80`.
    let message = [0x22; 134];
    let bytes = block_bytes(&message);
    assert_eq!(bytes[..134], message);
    assert_eq!(bytes[134..], [0x06, 0x80]);
}

#[test]
#[should_panic(expected = "needs more than one absorb")]
fn a_message_of_a_full_block_is_rejected_rather_than_padded_wrongly() {
    // A 136-byte message needs a whole second block of padding; this function pads only one.
    let _ = sha3_256_block(&[0x33; 136]);
}
