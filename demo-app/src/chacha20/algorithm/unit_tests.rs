//! ChaCha20 against the values RFC 8439 publishes: its quarter round test vector, the block function's example and the
//! encryption example.

use super::*;

fn check(condition: bool, msg: &str) -> Result<(), String> {
    if condition { Ok(()) } else { Err(msg.to_owned()) }
}

fn check_eq<T: PartialEq + std::fmt::Debug>(got: T, expected: T) -> Result<(), String> {
    check(got == expected, &format!("expected {expected:?}, got {got:?}"))
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

/// The key every RFC 8439 example uses: the bytes `00 01 02 ... 1f`.
fn rfc_key() -> [u8; KEY_LEN] {
    std::array::from_fn(|i| i as u8)
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
//   The quarter round
// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// RFC 8439 section 2.1.1.
#[test]
fn the_quarter_round_matches_the_rfcs_test_vector() -> Result<(), String> {
    let output = quarter_round_trace([0x1111_1111, 0x0102_0304, 0x9b8d_6f43, 0x0123_4567]).output;
    check_eq(output, [0xea2a_92f4, 0xcb1c_f8ce, 0x4581_472e, 0x5881_c4bb])
}

#[test]
fn the_trace_has_twelve_steps_and_ends_at_the_quarter_rounds_output() -> Result<(), String> {
    let trace = quarter_round_trace([0x1111_1111, 0x0102_0304, 0x9b8d_6f43, 0x0123_4567]);
    check_eq(trace.steps.len(), 12)?;

    // Each word's last step is the value that comes out.
    for word in Word::ALL {
        let last = trace.steps.iter().rev().find(|step| step.word == word).map(|step| step.result);
        check_eq(last, Some(trace.output[word.index()]))?;
    }
    Ok(())
}

#[test]
fn the_steps_update_each_word_the_number_of_times_the_rfc_says() -> Result<(), String> {
    let count = |word: Word| STEPS.iter().filter(|(w, _)| *w == word).count();
    check_eq(count(Word::A), 2)?;
    check_eq(count(Word::B), 4)?;
    check_eq(count(Word::C), 2)?;
    check_eq(count(Word::D), 4)
}

#[test]
fn the_rotations_are_16_12_8_and_7_bits() -> Result<(), String> {
    let rotations: Vec<u8> = STEPS
        .iter()
        .filter_map(|(_, operation)| match operation {
            Operation::RotateLeft(bits) => Some(*bits),
            _ => None,
        })
        .collect();
    check_eq(rotations, vec![16, 12, 8, 7])
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
//   The state and the block function
// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// RFC 8439 section 2.3.2.
const BLOCK_NONCE: [u8; NONCE_LEN] = [0, 0, 0, 9, 0, 0, 0, 0x4a, 0, 0, 0, 0];

#[test]
fn the_constants_spell_expand_32_byte_k() -> Result<(), String> {
    let bytes: Vec<u8> = CONSTANTS.iter().flat_map(|word| word.to_le_bytes()).collect();
    check_eq(bytes, b"expand 32-byte k".to_vec())
}

#[test]
fn the_initial_state_matches_the_rfcs_example() -> Result<(), String> {
    let state = initial_state(&rfc_key(), 1, &BLOCK_NONCE);
    check_eq(
        state,
        [
            0x6170_7865, 0x3320_646e, 0x7962_2d32, 0x6b20_6574, //
            0x0302_0100, 0x0706_0504, 0x0b0a_0908, 0x0f0e_0d0c, //
            0x1312_1110, 0x1716_1514, 0x1b1a_1918, 0x1f1e_1d1c, //
            0x0000_0001, 0x0900_0000, 0x4a00_0000, 0x0000_0000,
        ],
    )
}

#[test]
fn the_block_function_matches_the_rfcs_example() -> Result<(), String> {
    let state = initial_state(&rfc_key(), 1, &BLOCK_NONCE);
    check_eq(
        block(&state),
        [
            0xe4e7_f110, 0x1559_3bd1, 0x1fdd_0f50, 0xc471_20a3, //
            0xc7f4_d1c7, 0x0368_c033, 0x9aaa_2204, 0x4e6c_d4c3, //
            0x4664_82d2, 0x09aa_9f07, 0x05d7_c214, 0xa202_8bd9, //
            0xd19c_12b5, 0xb94e_16de, 0xe883_d0cb, 0x4e3c_50a2,
        ],
    )
}

#[test]
fn the_keystream_is_the_blocks_words_written_little_endian() -> Result<(), String> {
    let state = initial_state(&rfc_key(), 1, &BLOCK_NONCE);
    check_eq(
        hex(&keystream_block(&state)),
        "10f1e7e4d13b5915500fdd1fa32071c4c7d1f4c733c0680304 22aa9ac3d46c4ed2826446079faa0914c2d705d98b02a2b5129cd1de164eb9cbd083e8a2503c4e"
            .replace(' ', ""),
    )
}

#[test]
fn the_rounds_alone_do_not_give_the_block() -> Result<(), String> {
    let state = initial_state(&rfc_key(), 1, &BLOCK_NONCE);
    check(run_rounds(&state) != block(&state), "the initial state must be added back in")
}

#[test]
fn the_columns_and_diagonals_each_cover_every_word_once() -> Result<(), String> {
    for groups in [COLUMNS, DIAGONALS] {
        let mut seen: Vec<usize> = groups.iter().flatten().copied().collect();
        seen.sort_unstable();
        check_eq(seen, (0..STATE_WORDS).collect::<Vec<_>>())?;
    }
    Ok(())
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
//   Encryption
// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// RFC 8439 section 2.4.2.
const PLAINTEXT: &[u8] = b"Ladies and Gentlemen of the class of '99: If I could offer you only one tip for the \
future, sunscreen would be it.";
const ENCRYPTION_NONCE: [u8; NONCE_LEN] = [0, 0, 0, 0, 0, 0, 0, 0x4a, 0, 0, 0, 0];

#[test]
fn encryption_matches_the_rfcs_example() -> Result<(), String> {
    let cipher = encrypt(&rfc_key(), 1, &ENCRYPTION_NONCE, PLAINTEXT);
    check_eq(
        hex(&cipher),
        "6e2e359a2568f98041ba0728dd0d6981e97e7aec1d4360c20a27afccfd9fae0bf91b65c5524733ab8f593dabcd62b3571639d624e65152ab\
8f530c359f0861d807ca0dbf500d6a6156a38e088a22b65e52bc514d16ccf806818ce91ab77937365af90bbf74a35be6b40b8eedf2785e42874d"
            .to_owned(),
    )
}

#[test]
fn decryption_is_the_same_operation() -> Result<(), String> {
    let cipher = encrypt(&rfc_key(), 1, &ENCRYPTION_NONCE, PLAINTEXT);
    check_eq(encrypt(&rfc_key(), 1, &ENCRYPTION_NONCE, &cipher), PLAINTEXT.to_vec())
}

#[test]
fn a_message_of_no_bytes_encrypts_to_no_bytes() -> Result<(), String> {
    check_eq(encrypt(&rfc_key(), 1, &ENCRYPTION_NONCE, &[]), Vec::new())
}

#[test]
fn the_second_block_of_a_long_message_uses_the_next_counter() -> Result<(), String> {
    let zeros = [0_u8; BLOCK_LEN + 3];
    let cipher = encrypt(&rfc_key(), 1, &ENCRYPTION_NONCE, &zeros);
    let second = keystream_block(&initial_state(&rfc_key(), 2, &ENCRYPTION_NONCE));
    check_eq(cipher[BLOCK_LEN..].to_vec(), second[..3].to_vec())
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
//   The twenty rounds
// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[test]
fn there_are_eighty_quarter_rounds_four_to_each_of_twenty_rounds() -> Result<(), String> {
    let all = rounds(&initial_state(&rfc_key(), 1, &BLOCK_NONCE));
    check_eq(all.stages.len(), QUARTER_ROUNDS)?;
    check_eq(QUARTER_ROUNDS, 80)?;
    for (index, stage) in all.stages.iter().enumerate() {
        check_eq(stage.round, index / QUARTER_ROUNDS_PER_ROUND + 1)?;
        check_eq(stage.number, index % QUARTER_ROUNDS_PER_ROUND + 1)?;
    }
    Ok(())
}

#[test]
fn odd_rounds_mix_columns_and_even_rounds_mix_diagonals() -> Result<(), String> {
    let all = rounds(&initial_state(&rfc_key(), 1, &BLOCK_NONCE));
    for stage in &all.stages {
        let (kind, groups) = if stage.round % 2 == 1 {
            (RoundKind::Column, COLUMNS)
        } else {
            (RoundKind::Diagonal, DIAGONALS)
        };
        check_eq(stage.kind, kind)?;
        check_eq(stage.indices, groups[stage.number - 1])?;
    }
    Ok(())
}

#[test]
fn each_quarter_round_starts_where_the_last_one_stopped_and_changes_only_its_four_words() -> Result<(), String> {
    let initial = initial_state(&rfc_key(), 1, &BLOCK_NONCE);
    let all = rounds(&initial);
    check_eq(all.stages[0].before, initial)?;
    for pair in all.stages.windows(2) {
        check_eq(pair[0].after, pair[1].before)?;
    }
    for stage in &all.stages {
        let [a, b, c, d] = stage.indices;
        check_eq(
            stage.trace.input,
            [stage.before[a], stage.before[b], stage.before[c], stage.before[d]],
        )?;
        for position in 0..STATE_WORDS {
            if !stage.indices.contains(&position) {
                check_eq(stage.after[position], stage.before[position])?;
            }
        }
        for (position, word) in stage.indices.into_iter().zip(stage.trace.output) {
            check_eq(stage.after[position], word)?;
        }
    }
    Ok(())
}

#[test]
fn the_last_quarter_round_leaves_the_state_the_block_function_adds_back_to() -> Result<(), String> {
    let initial = initial_state(&rfc_key(), 1, &BLOCK_NONCE);
    let all = rounds(&initial);
    check_eq(all.stages[QUARTER_ROUNDS - 1].after, all.mixed)?;
    check_eq(run_rounds(&initial), all.mixed)?;
    check_eq(block(&initial), all.keystream)
}
