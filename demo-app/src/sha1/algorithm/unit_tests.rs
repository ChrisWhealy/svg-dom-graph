//! SHA-1 against its published values: the standard's own constants, and known digests including the empty message,
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

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
//   Constants
// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[test]
fn the_initial_hash_is_the_standards_own() -> Result<(), String> {
    check_eq(digest_hex(&INITIAL_HASH), "67452301efcdab8998badcfe10325476c3d2e1f0".to_owned())
}

/// `sqrt(n) * 2^31`, rounded down, found with exact integer arithmetic: the largest `r` with `r * r <= n * 2^62`.
fn scaled_root(n: u64) -> u64 {
    let target = u128::from(n) << 62;
    let (mut low, mut high) = (0_u64, u64::MAX >> 1);
    while low < high {
        let mid = low + (high - low).div_ceil(2);
        if u128::from(mid) * u128::from(mid) <= target {
            low = mid;
        } else {
            high = mid - 1;
        }
    }
    low
}

#[test]
fn the_round_constants_are_the_square_roots_of_2_3_5_and_10() -> Result<(), String> {
    // Each constant is `sqrt(n) * 2^30`, rounded down, so one bit less than `scaled_root` carries.
    let recomputed: Vec<u32> = [2_u64, 3, 5, 10].iter().map(|&n| (scaled_root(n) >> 1) as u32).collect();
    check_eq(recomputed, ROUND_CONSTANTS.to_vec())
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
//   Pieces
// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[test]
fn sections_map_onto_their_round_functions() -> Result<(), String> {
    check_eq(RoundFunction::of_section(section_of(0)), RoundFunction::Choice)?;
    check_eq(RoundFunction::of_section(section_of(19)), RoundFunction::Choice)?;
    check_eq(RoundFunction::of_section(section_of(20)), RoundFunction::Parity)?;
    check_eq(RoundFunction::of_section(section_of(40)), RoundFunction::Majority)?;
    check_eq(RoundFunction::of_section(section_of(59)), RoundFunction::Majority)?;
    check_eq(RoundFunction::of_section(section_of(60)), RoundFunction::Parity)?;
    check_eq(RoundFunction::of_section(section_of(79)), RoundFunction::Parity)
}

#[test]
fn the_round_functions_agree_with_their_definitions() -> Result<(), String> {
    let (b, c, d) = (0xf0f0_f0f0_u32, 0xcccc_cccc, 0xaaaa_aaaa);
    check_eq(RoundFunction::Choice.apply(b, c, d), (b & c) | (!b & d))?;
    check_eq(RoundFunction::Parity.apply(b, c, d), b ^ c ^ d)?;
    check_eq(RoundFunction::Majority.apply(b, c, d), (b & c) | (b & d) | (c & d))
}

#[test]
fn the_message_block_holds_the_terminator_and_the_length() -> Result<(), String> {
    let block = message_block(b"abc").ok_or("fits")?;
    check_eq(block[0], 0x6162_6380)?;
    check_eq(block[BLOCK_WORDS - 1], 24)?;
    check_eq(message_block(&[0; MAX_MESSAGE_LEN + 1]), None)
}

#[test]
fn a_schedule_word_is_the_rotated_xor_of_four_earlier_ones() -> Result<(), String> {
    let block = message_block(b"abc").ok_or("fits")?;
    let schedule = message_schedule(&block);
    check_eq(&schedule[..BLOCK_WORDS], &block[..])?;
    let inputs = expansion_inputs(&schedule, 16);
    check_eq(inputs, [schedule[13], schedule[8], schedule[2], schedule[0]])?;
    check_eq(schedule[16], expansion_xor(&inputs).rotate_left(1))
}

#[test]
fn a_round_shifts_the_working_variables_along() -> Result<(), String> {
    let r = round([1, 2, 3, 4, 5], 0, 6);
    check_eq(r.after, [r.temp, 1, 2_u32.rotate_left(30), 3, 4])?;
    check_eq(r.temp, r.terms.iter().fold(0_u32, |s, t| s.wrapping_add(*t)))
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
//   Digests
// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[test]
fn the_empty_message_hashes_as_published() -> Result<(), String> {
    check_eq(digest_of(b"")?, "da39a3ee5e6b4b0d3255bfef95601890afd80709".to_owned())
}

#[test]
fn abc_hashes_as_published() -> Result<(), String> {
    check_eq(digest_of(b"abc")?, "a9993e364706816aba3e25717850c26c9cd0d89d".to_owned())
}

#[test]
fn the_demos_own_message_hashes_as_published() -> Result<(), String> {
    check_eq(
        digest_of(b"The quick brown fox jumps over the lazy dog")?,
        "2fd4e1c67a2d28fced849ee1bb76e7391b93eb12".to_owned(),
    )
}

#[test]
fn the_longest_single_block_message_hashes_as_a_reference_implementation_does() -> Result<(), String> {
    // 55 'a' characters, hashed with `sha1sum`.
    let message = [b'a'; MAX_MESSAGE_LEN];
    check_eq(digest_of(&message)?, "c1c8bbdc22796e28c0e15163d20899b65621d65a".to_owned())
}

#[test]
fn a_message_that_is_too_long_has_no_trace() -> Result<(), String> {
    check(
        trace(&[0; MAX_MESSAGE_LEN + 1]).is_none(),
        "a 56-byte message needs a second block",
    )
}

#[test]
fn each_rounds_output_is_the_next_rounds_input() -> Result<(), String> {
    let t = trace(b"abc").ok_or("fits")?;
    check_eq(t.rounds[0].before, INITIAL_HASH)?;
    for pair in t.rounds.windows(2) {
        check_eq(pair[0].after, pair[1].before)?;
    }
    Ok(())
}
