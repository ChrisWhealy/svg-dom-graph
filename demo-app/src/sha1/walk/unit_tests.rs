//! The walk's own bookkeeping: which step a position is, and what is shown once it is reached.

use super::*;
use crate::sha1::algorithm::{INITIAL_HASH, trace};

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
    // Message, block, copy, 64 schedule words, 80 rounds, the digest, and the digest as a hash value.
    check_eq(STEPS, 3 + 64 + 80 + 1 + 1)
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[test]
fn positions_map_onto_the_steps_in_order() -> Result<(), String> {
    check_eq(phase(0), Phase::Message)?;
    check_eq(phase(1), Phase::Block)?;
    check_eq(phase(2), Phase::Copy)?;
    check_eq(phase(3), Phase::Expand(16))?;
    check_eq(phase(66), Phase::Expand(79))?;
    check_eq(phase(67), Phase::Round(0))?;
    check_eq(phase(146), Phase::Round(79))?;
    check_eq(phase(147), Phase::Final)?;
    check_eq(phase(148), Phase::Hash)?;
    check_eq(phase(10_000), Phase::Hash)
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[test]
fn the_stage_changes_only_at_a_section_boundary() -> Result<(), String> {
    check_eq(stage(0), Stage::Expansion)?;
    check_eq(stage(66), Stage::Expansion)?;
    check_eq(stage(67), Stage::Compression(0))?;
    check_eq(stage(67 + 19), Stage::Compression(0))?;
    check_eq(stage(67 + 20), Stage::Compression(1))?;
    check_eq(stage(67 + 60), Stage::Compression(3))?;
    check_eq(stage(FINAL), Stage::Compression(3))?;
    check_eq(stage(FINAL + 1), Stage::Compression(3))?;
    check_eq(stage(10_000), Stage::Compression(3))
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[test]
fn nothing_has_been_written_at_the_start() -> Result<(), String> {
    let shown = shown(&the_trace()?, 0);
    check_eq(shown.ring, Some(Ring::Message))?;
    check_eq(shown.block, [0; BLOCK_WORDS])?;
    check_eq(shown.schedule, [0; SCHEDULE_WORDS])?;
    check_eq(shown.digest, [0; 5])
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
    let shown = shown(&trace, EXPAND_START);
    check_eq(shown.schedule_focus, Some(16))?;
    check_eq(shown.schedule_secondary, vec![13, 8, 2, 0])?;
    let e = shown.expansion.ok_or("expansion is shown")?;
    check_eq(
        e.inputs,
        [trace.schedule[13], trace.schedule[8], trace.schedule[2], trace.schedule[0]],
    )?;
    check_eq(e.xor.rotate_left(1), e.sum)?;
    check_eq(e.sum, trace.schedule[16])
}

#[test]
fn only_the_words_built_so_far_are_shown() -> Result<(), String> {
    let trace = the_trace()?;
    let shown = shown(&trace, EXPAND_START + 4);
    check_eq(shown.schedule[20], trace.schedule[20])?;
    check_eq(shown.schedule[21], 0)?;
    check_eq(shown.schedule_unreached.first().copied(), Some(21))
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[test]
fn a_round_shows_its_working_variables_and_schedule_word() -> Result<(), String> {
    let trace = the_trace()?;
    let shown = shown(&trace, ROUND_START);
    check_eq(shown.working, INITIAL_HASH)?;
    check_eq(shown.round.map(|r| r.before), Some(INITIAL_HASH))?;
    check_eq(shown.schedule_focus, Some(0))?;
    check_eq(shown.round.map(|r| r.terms[4]), Some(trace.schedule[0]))?;
    check(shown.expansion.is_none(), "a round builds no schedule word")
}

#[test]
fn the_digest_appears_at_its_own_step_and_the_hash_after_it() -> Result<(), String> {
    let trace = the_trace()?;
    check_eq(shown(&trace, FINAL - 1).digest, [0; 5])?;
    check_eq(shown(&trace, FINAL).digest, trace.digest)?;
    check_eq(shown(&trace, FINAL).ring, Some(Ring::Digest))?;
    check_eq(shown(&trace, FINAL).hash, "0".repeat(40))?;
    check_eq(
        shown(&trace, FINAL + 1).hash,
        "2fd4e1c67a2d28fced849ee1bb76e7391b93eb12".to_owned(),
    )?;
    check_eq(shown(&trace, FINAL + 1).ring, Some(Ring::Hash))
}

#[test]
fn the_last_round_stays_on_show_for_the_digest() -> Result<(), String> {
    let trace = the_trace()?;
    check_eq(shown(&trace, FINAL).round, Some(trace.rounds[ROUNDS - 1]))?;
    check_eq(shown(&trace, FINAL).working, trace.rounds[ROUNDS - 1].before)
}

#[test]
fn a_position_past_the_end_is_the_last_step() -> Result<(), String> {
    let trace = the_trace()?;
    check_eq(shown(&trace, STEPS + 50).hash, shown(&trace, STEPS - 1).hash)
}
