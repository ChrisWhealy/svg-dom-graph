//! The walk's own bookkeeping: which step a position is, and what is shown once it is reached.

use super::*;
use crate::sha2_256::algorithm::{INITIAL_HASH, trace};

fn check(condition: bool, msg: &str) -> Result<(), String> {
    if condition { Ok(()) } else { Err(msg.to_owned()) }
}

fn check_eq<T: PartialEq + std::fmt::Debug>(got: T, expected: T) -> Result<(), String> {
    check(got == expected, &format!("expected {expected:?}, got {got:?}"))
}

fn the_trace() -> Result<Trace, String> {
    trace(MESSAGE).ok_or_else(|| "the message fits one block".to_owned())
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[test]
fn the_walk_has_one_position_per_step() -> Result<(), String> {
    // Message, block, copy, 48 schedule words, 64 rounds, and the digest.
    check_eq(STEPS, 3 + 48 + 64 + 1)
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[test]
fn positions_map_onto_the_steps_in_order() -> Result<(), String> {
    check_eq(phase(0), Phase::Message)?;
    check_eq(phase(1), Phase::Block)?;
    check_eq(phase(2), Phase::Copy)?;
    check_eq(phase(3), Phase::Expand(16))?;
    check_eq(phase(50), Phase::Expand(63))?;
    check_eq(phase(51), Phase::Round(0))?;
    check_eq(phase(114), Phase::Round(63))?;
    check_eq(phase(115), Phase::Final)?;
    check_eq(phase(10_000), Phase::Final)
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[test]
fn nothing_has_been_written_at_the_start() -> Result<(), String> {
    let shown = shown(&the_trace()?, 0);
    check_eq(shown.ring, Some(Ring::Message))?;
    check_eq(shown.block, [0; BLOCK_WORDS])?;
    check_eq(shown.schedule, [0; SCHEDULE_WORDS])?;
    check_eq(shown.digest, [0; 8])
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[test]
fn the_block_appears_at_its_own_step_and_stays() -> Result<(), String> {
    let trace = the_trace()?;
    check_eq(shown(&trace, 1).block, trace.block)?;
    check_eq(shown(&trace, 100).block, trace.block)
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[test]
fn the_copy_step_fills_only_the_first_sixteen_schedule_words() -> Result<(), String> {
    let trace = the_trace()?;
    let shown = shown(&trace, 2);
    check_eq(&shown.schedule[..BLOCK_WORDS], &trace.block[..])?;
    check_eq(&shown.schedule[BLOCK_WORDS..], &[0_u32; SCHEDULE_WORDS - BLOCK_WORDS][..])
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[test]
fn building_a_word_marks_the_four_words_it_reads() -> Result<(), String> {
    let trace = the_trace()?;
    let shown = shown(&trace, 3);
    check_eq(shown.schedule_focus, Some(16))?;
    check_eq(shown.schedule_secondary, vec![0, 1, 9, 14])?;
    // The word just built is shown, and the ones after it are not.
    check_eq(shown.schedule[16], trace.schedule[16])?;
    check_eq(shown.schedule[17], 0)
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[test]
fn the_first_round_starts_from_the_initial_hash() -> Result<(), String> {
    let shown = shown(&the_trace()?, 51);
    check_eq(shown.working, INITIAL_HASH)?;
    check_eq(shown.k_focus, Some(0))?;
    check_eq(shown.schedule_focus, Some(0))?;
    check(shown.round.is_some(), "a round is shown")
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[test]
fn the_last_step_shows_the_digest_and_the_final_working_variables() -> Result<(), String> {
    let trace = the_trace()?;
    let shown = shown(&trace, STEPS - 1);
    check_eq(shown.ring, Some(Ring::Digest))?;
    check_eq(shown.digest, trace.digest)?;
    check_eq(shown.working, trace.compressed)?;
    check(shown.round.is_none(), "no round is running at the last step")
}
