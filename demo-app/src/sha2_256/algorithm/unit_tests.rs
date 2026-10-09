//! SHA-256 against its published values: the standard's own constants, and known digests including the empty message,
//! `"abc"`, the demo's own message and the longest single-block message.

use super::*;

fn check(condition: bool, msg: &str) -> Result<(), String> {
    if condition { Ok(()) } else { Err(msg.to_owned()) }
}

fn check_eq<T: PartialEq + std::fmt::Debug>(got: T, expected: T) -> Result<(), String> {
    check(got == expected, &format!("expected {expected:?}, got {got:?}"))
}

fn digest_of(message: &[u8]) -> Result<String, String> {
    let trace = trace(message).ok_or("the message fits one block")?;
    Ok(digest_hex(&trace.digest))
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
//   Constants
// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[test]
fn the_first_primes_end_where_the_standard_says() -> Result<(), String> {
    check_eq(first_primes(8), vec![2, 3, 5, 7, 11, 13, 17, 19])?;
    check_eq(first_primes(64).last().copied(), Some(311))
}

#[test]
fn the_initial_hash_is_the_standards_own() -> Result<(), String> {
    check_eq(INITIAL_HASH[0], 0x6a09_e667)?;
    check_eq(INITIAL_HASH[7], 0x5be0_cd19)
}

#[test]
fn the_round_constants_start_and_end_as_the_standard_says() -> Result<(), String> {
    check_eq(&ROUND_CONSTANTS[..4], &[0x428a_2f98, 0x7137_4491, 0xb5c0_fbcf, 0xe9b5_dba5][..])?;
    check_eq(
        &ROUND_CONSTANTS[60..],
        &[0x90be_fffa, 0xa450_6ceb, 0xbef9_a3f7, 0xc671_78f2][..],
    )
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
//   Recomputing the constants
//
//   The tables are fixed by the standard and declared as constants. These recompute them from the primes, with exact
//   integer arithmetic, so a mistyped digit cannot go unnoticed.
// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -

/// The first `n` prime numbers.
fn first_primes(n: usize) -> Vec<u32> {
    let mut primes: Vec<u32> = Vec::with_capacity(n);
    let mut candidate = 2;
    while primes.len() < n {
        if primes.iter().take_while(|&&p| p * p <= candidate).all(|&p| candidate % p != 0) {
            primes.push(candidate);
        }
        candidate += 1;
    }
    primes
}

/// The largest `r` with `r.pow(degree) <= value`, by binary search.
fn integer_root(value: u128, degree: u32) -> u128 {
    let (mut low, mut high) = (0_u128, 1_u128 << 64);
    while low < high {
        let mid = low + (high - low).div_ceil(2);
        if mid.checked_pow(degree).is_some_and(|power| power <= value) {
            low = mid;
        } else {
            high = mid - 1;
        }
    }

    low
}

/// The first 32 bits of the fractional part of the square root of `prime`. `floor(sqrt(prime) * 2^32)` is exactly
/// `isqrt(prime * 2^64)`, and its low 32 bits are the fraction.
fn square_root_fraction(prime: u32) -> u32 {
    (integer_root(u128::from(prime) << 64, 2) & 0xFFFF_FFFF) as u32
}

/// The first 32 bits of the fractional part of the cube root of `prime`. `floor(cbrt(prime) * 2^32)` is exactly
/// `icbrt(prime * 2^96)`, and its low 32 bits are the fraction.
fn cube_root_fraction(prime: u32) -> u32 {
    (integer_root(u128::from(prime) << 96, 3) & 0xFFFF_FFFF) as u32
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[test]
fn the_initial_hash_table_is_the_square_root_fractions_of_the_first_eight_primes() -> Result<(), String> {
    let recomputed: Vec<u32> = first_primes(8).into_iter().map(square_root_fraction).collect();
    check_eq(INITIAL_HASH.to_vec(), recomputed)
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[test]
fn the_round_constant_table_is_the_cube_root_fractions_of_the_first_sixty_four_primes() -> Result<(), String> {
    let recomputed: Vec<u32> = first_primes(ROUNDS).into_iter().map(cube_root_fraction).collect();
    check_eq(ROUND_CONSTANTS.to_vec(), recomputed)
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
//   The functions
// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[test]
fn choice_takes_f_where_e_is_set_and_g_where_it_is_clear() -> Result<(), String> {
    check_eq(choice(0xFFFF_0000, 0x1234_5678, 0x9ABC_DEF0), 0x1234_DEF0)
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[test]
fn majority_takes_the_value_at_least_two_inputs_share() -> Result<(), String> {
    check_eq(majority(0b1100, 0b1010, 0b0110), 0b1110)
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[test]
fn the_sigmas_are_their_own_rotations_and_shifts() -> Result<(), String> {
    let w = 0x1234_5678;
    check_eq(sigma0(w), w.rotate_right(7) ^ w.rotate_right(18) ^ (w >> 3))?;
    check_eq(sigma1(w), w.rotate_right(17) ^ w.rotate_right(19) ^ (w >> 10))?;
    check_eq(big_sigma0(w), w.rotate_right(2) ^ w.rotate_right(13) ^ w.rotate_right(22))?;
    check_eq(big_sigma1(w), w.rotate_right(6) ^ w.rotate_right(11) ^ w.rotate_right(25))
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
//   The block and the schedule
// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[test]
fn the_block_is_the_message_then_the_terminator_then_the_bit_length() -> Result<(), String> {
    let block = message_block(b"abc").ok_or("fits one block")?;
    check_eq(block[0], 0x6162_6380)?;
    check_eq(&block[1..15], &[0_u32; 14][..])?;
    check_eq(block[15], 24)
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[test]
fn a_message_that_fills_the_last_word_before_the_length_still_fits() -> Result<(), String> {
    let message = [b'x'; MAX_MESSAGE_LEN];
    let block = message_block(&message).ok_or("55 bytes fit one block")?;
    // 55 message bytes put the terminator in the last byte of word 13. Word 14 is zero and word 15 the length.
    check_eq(block[13] & 0xFF, 0x80)?;
    check_eq(block[14], 0)?;
    check_eq(block[15], 440)
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[test]
fn a_longer_message_is_refused() -> Result<(), String> {
    check(
        message_block(&[0_u8; MAX_MESSAGE_LEN + 1]).is_none(),
        "56 bytes do not fit one block with its padding",
    )
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[test]
fn the_schedule_starts_with_the_block_and_expands_from_it() -> Result<(), String> {
    let block = message_block(b"abc").ok_or("fits one block")?;
    let schedule = message_schedule(&block);
    check_eq(&schedule[..BLOCK_WORDS], &block[..])?;
    // The standard's own worked example for "abc" gives these first expanded words.
    check_eq(schedule[16], 0x6162_6380)?;
    check_eq(schedule[17], 0x000f_0000)?;
    check_eq(schedule[18], 0x7da8_6405)?;
    check_eq(schedule[19], 0x6000_03c6)
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
//   The rounds and the digest
// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[test]
fn a_round_shifts_the_working_variables_down_and_computes_a_and_e() -> Result<(), String> {
    let state = [1, 2, 3, 4, 5, 6, 7, 8];
    let r = round(state, 0x428a_2f98, 0x6162_6380);
    check_eq(&r.after[1..4], &state[0..3])?;
    check_eq(&r.after[5..], &state[4..7])?;
    check_eq(r.after[0], r.temp1.wrapping_add(r.temp2))?;
    check_eq(r.after[4], state[3].wrapping_add(r.temp1))?;
    check_eq(r.temp2, r.big_sigma0.wrapping_add(r.majority))
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[test]
fn each_rounds_output_is_the_next_rounds_input_and_the_last_feeds_the_digest() -> Result<(), String> {
    let trace = trace(b"abc").ok_or("fits one block")?;
    for pair in trace.rounds.windows(2) {
        check_eq(pair[0].after, pair[1].before)?;
    }
    // The digest is that last state added to the initial hash, word by word.
    let last = trace.rounds[ROUNDS - 1].after;
    check_eq(trace.digest, std::array::from_fn(|i| INITIAL_HASH[i].wrapping_add(last[i])))
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[test]
fn the_digest_of_the_empty_message_is_the_published_one() -> Result<(), String> {
    check_eq(
        digest_of(b"")?,
        "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855".to_owned(),
    )
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[test]
fn the_digest_of_abc_is_the_published_one() -> Result<(), String> {
    check_eq(
        digest_of(b"abc")?,
        "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad".to_owned(),
    )
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[test]
fn the_digest_of_the_demos_own_message_is_the_published_one() -> Result<(), String> {
    check_eq(
        digest_of(b"The quick brown fox jumps over the lazy dog")?,
        "d7a8fbb307d7809469ca9abcb0082e4f8d5651e46d3cdb762d02d0bf37c9e592".to_owned(),
    )
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[test]
fn the_digest_of_the_longest_single_block_message_is_the_published_one() -> Result<(), String> {
    // 55 bytes of 'a', checked against `sha256sum`.
    check_eq(
        digest_of(&[b'a'; MAX_MESSAGE_LEN])?,
        "9f4390f8d30c2dd92ec9f095b65e2b9ae9b0a925a5258e241c9f1e910f734318".to_owned(),
    )
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[test]
fn the_compressed_state_alone_is_not_the_digest() -> Result<(), String> {
    let trace = trace(b"abc").ok_or("fits one block")?;
    check(
        trace.rounds[ROUNDS - 1].after != trace.digest,
        "the final feed-forward addition changes the result",
    )
}

#[test]
fn the_hex_digest_is_sixty_four_lowercase_digits_with_leading_zeros_kept() -> Result<(), String> {
    let hex = digest_hex(&[0x0000_000a, 0, 0, 0, 0, 0, 0, 0xffff_ffff]);
    check_eq(hex.len(), 64)?;
    check(hex.starts_with("0000000a") && hex.ends_with("ffffffff"), &hex)
}
