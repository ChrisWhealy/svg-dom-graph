//! The walk's positions and what each one shows, against the quarter rounds they come from.

use super::*;
use crate::chacha20::algorithm::{initial_state, rounds};

fn check(condition: bool, msg: &str) -> Result<(), String> {
    if condition { Ok(()) } else { Err(msg.to_owned()) }
}

fn check_eq<T: PartialEq + std::fmt::Debug>(got: T, expected: T) -> Result<(), String> {
    check(got == expected, &format!("expected {expected:?}, got {got:?}"))
}

fn all_rounds() -> Rounds {
    let key: [u8; 32] = std::array::from_fn(|i| i as u8);
    rounds(&initial_state(&key, 1, &[0, 0, 0, 9, 0, 0, 0, 0x4a, 0, 0, 0, 0]))
}

#[test]
fn there_is_a_position_for_each_step_of_each_quarter_round_and_one_for_the_keystream() -> Result<(), String> {
    check_eq(POSITIONS, 80 * 12 + 1)
}

#[test]
fn positions_are_located_by_quarter_round_and_step() -> Result<(), String> {
    check_eq(locate(None), None)?;
    check_eq(locate(Some(0)), Some((0, 0)))?;
    check_eq(locate(Some(11)), Some((0, 11)))?;
    check_eq(locate(Some(12)), Some((1, 0)))?;
    check_eq(locate(Some(POSITIONS - 2)), Some((79, 11)))?;
    check_eq(locate(Some(POSITIONS - 1)), None)
}

#[test]
fn only_the_last_position_is_the_keystream() -> Result<(), String> {
    check_eq(stage(None), Stage::QuarterRound)?;
    check_eq(stage(Some(0)), Stage::QuarterRound)?;
    check_eq(stage(Some(POSITIONS - 2)), Stage::QuarterRound)?;
    check_eq(stage(Some(POSITIONS - 1)), Stage::Keystream)
}

#[test]
fn the_unstarted_walk_shows_nothing_of_the_quarter_round_but_its_state() -> Result<(), String> {
    let all = all_rounds();
    let shown = shown(&all, None);
    check_eq(shown.quarter_round, 0)?;
    check_eq(shown.before, all.initial)?;
    check_eq(shown.inputs, [0; 4])?;
    check_eq(shown.steps, [0; 12])?;
    check_eq(shown.outputs, [0; 4])?;
    check_eq(shown.unreached_steps(), (0..12).collect::<Vec<_>>())?;
    check_eq(shown.unreached_after(), vec![0, 4, 8, 12])?;
    check_eq(shown.current, None)
}

#[test]
fn a_step_shows_every_result_up_to_itself_and_zeros_after() -> Result<(), String> {
    let all = all_rounds();
    let trace = all.stages[3].trace;
    let shown = shown(&all, Some(3 * 12 + 4));
    check_eq(shown.quarter_round, 3)?;
    check_eq(shown.reached, 5)?;
    check_eq(shown.inputs, trace.input)?;
    for (index, value) in shown.steps.into_iter().enumerate() {
        check_eq(value, if index < 5 { trace.steps[index].result } else { 0 })?;
    }
    check_eq(shown.outputs, [0; 4])?;
    check_eq(shown.unreached_steps(), (5..12).collect::<Vec<_>>())?;
    check_eq(shown.current, Some(4))
}

#[test]
fn the_last_step_completes_the_quarter_round() -> Result<(), String> {
    let all = all_rounds();
    let stage = &all.stages[10];
    let shown = shown(&all, Some(10 * 12 + 11));
    check(shown.outputs_reached(), "the quarter round should be complete")?;
    check_eq(shown.outputs, stage.trace.output)?;
    check_eq(shown.after, stage.after)?;
    check_eq(shown.unreached_steps(), Vec::new())?;
    check_eq(shown.unreached_after(), Vec::new())
}

#[test]
fn the_state_after_carries_over_every_word_the_quarter_round_does_not_change() -> Result<(), String> {
    let all = all_rounds();
    let shown = shown(&all, Some(5 * 12));
    for position in 0..STATE_WORDS {
        if !shown.group.contains(&position) {
            check_eq(shown.after[position], shown.before[position])?;
        }
    }
    for position in shown.group {
        check_eq(shown.after[position], 0)?;
    }
    Ok(())
}

#[test]
fn each_title_names_its_round_and_its_kind() -> Result<(), String> {
    let all = all_rounds();
    check_eq(
        title(&all, None),
        "ChaCha20 round 1 of 20: column quarter round 1 of 4".to_owned(),
    )?;
    check_eq(
        title(&all, Some(4 * 12)),
        "ChaCha20 round 2 of 20: diagonal quarter round 1 of 4".to_owned(),
    )?;
    check_eq(title(&all, Some(POSITIONS - 1)), "ChaCha20: the keystream block".to_owned())
}
