//! The walk through ChaCha20: which position is on show, and what each part of the diagram shows there.
//!
//! A position is one step of one quarter round. Each quarter round has [`STEPS_PER_QUARTER_ROUND`] steps, and there are
//! [`QUARTER_ROUNDS`] of them. One last position follows them all, the keystream block, which the 20 rounds lead to.
//! The walk is also "unstarted" before the first position, when nothing has been worked out yet.
//!
//! [`shown`] is a pure function of the position alone. So stepping backwards writes exactly what stepping forwards
//! wrote, and a restart is the same write as any other step.

use super::algorithm::{QUARTER_ROUNDS, QuarterRoundStage, RoundKind, Rounds, STATE_WORDS, STEPS};

/// Steps in one quarter round.
pub(super) const STEPS_PER_QUARTER_ROUND: usize = STEPS.len();

/// Every position of the walk: the steps of all the quarter rounds, then the keystream block.
pub(super) const POSITIONS: usize = QUARTER_ROUNDS * STEPS_PER_QUARTER_ROUND + 1;

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Which diagram a position belongs to.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Stage {
    /// One quarter round, drawn as four columns. Every quarter round has the same layout, so they share one diagram.
    QuarterRound,
    /// The state after 20 rounds, the initial state, and the keystream block that they make together.
    Keystream,
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The diagram `position` belongs to. The unstarted walk is shown on the first quarter round.
pub(super) fn stage(position: Option<usize>) -> Stage {
    match position {
        Some(position) if position >= QUARTER_ROUNDS * STEPS_PER_QUARTER_ROUND => Stage::Keystream,
        _ => Stage::QuarterRound,
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The quarter round, from 0, and the step within it, from 0, that `position` is at. `None` for the unstarted walk and
/// for the keystream block, neither of which is a step.
pub(super) fn locate(position: Option<usize>) -> Option<(usize, usize)> {
    let position = position?;
    (position < QUARTER_ROUNDS * STEPS_PER_QUARTER_ROUND)
        .then_some((position / STEPS_PER_QUARTER_ROUND, position % STEPS_PER_QUARTER_ROUND))
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The title of the diagram for `position`.
pub(super) fn title(rounds: &Rounds, position: Option<usize>) -> String {
    let Some(stage) = rounds
        .stages
        .get(locate(position).map_or(0, |(quarter_round, _)| quarter_round))
    else {
        return "ChaCha20".to_owned();
    };
    if self::stage(position) == Stage::Keystream {
        return "ChaCha20: the keystream block".to_owned();
    }
    let kind = match stage.kind {
        RoundKind::Column => "column",
        RoundKind::Diagonal => "diagonal",
    };
    format!(
        "ChaCha20 round {} of {}: {kind} quarter round {} of 4",
        stage.round,
        super::algorithm::ROUNDS,
        stage.number
    )
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// What the quarter round diagram shows at one position.
///
/// A value the walk has not reached is zero, and its node is marked as not yet computed, so a placeholder zero is not
/// mistaken for a result. Until the walk reaches the first step, nothing of the quarter round has been worked out.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct Shown {
    /// The quarter round on show, from 0.
    pub(super) quarter_round: usize,
    /// How many of its steps the walk has reached: none before it starts, and all 12 once the quarter round is done.
    pub(super) reached: usize,
    /// The state the quarter round starts from. It is the cipher's own input, so it is always shown.
    pub(super) before: [u32; STATE_WORDS],
    /// The positions in that state of the words `a`, `b`, `c` and `d`.
    pub(super) group: [usize; 4],
    /// The four words going in. They are read by the first step, so they are shown from there.
    pub(super) inputs: [u32; 4],
    /// The result of each step.
    pub(super) steps: [u32; STEPS_PER_QUARTER_ROUND],
    /// The four words coming out. They are shown once the last step has run.
    pub(super) outputs: [u32; 4],
    /// The state after the quarter round: `before`, with the four words replaced once the last step has run.
    pub(super) after: [u32; STATE_WORDS],
    /// The step the walk stands at, which is the last one it has reached.
    pub(super) current: Option<usize>,
}

impl Shown {
    /// Whether the four input words have been worked out.
    pub(super) fn inputs_reached(&self) -> bool {
        self.reached > 0
    }

    /// Whether the quarter round is complete, so its four output words have been worked out.
    pub(super) fn outputs_reached(&self) -> bool {
        self.reached == STEPS_PER_QUARTER_ROUND
    }

    /// The steps the walk has not reached.
    pub(super) fn unreached_steps(&self) -> Vec<usize> {
        (self.reached..STEPS_PER_QUARTER_ROUND).collect()
    }

    /// The cells of the state after the quarter round that the walk has not reached: its four new words, until the
    /// quarter round is complete. The other twelve are carried over from the state before, so they are known.
    pub(super) fn unreached_after(&self) -> Vec<usize> {
        if self.outputs_reached() {
            Vec::new()
        } else {
            let mut cells = self.group.to_vec();
            cells.sort_unstable();
            cells
        }
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// What the quarter round diagram shows at `position`. The unstarted walk (`None`) shows the first quarter round with
/// nothing reached.
pub(super) fn shown(rounds: &Rounds, position: Option<usize>) -> Shown {
    let (quarter_round, current) = match locate(position) {
        Some((quarter_round, step)) => (quarter_round, Some(step)),
        // The unstarted walk is on the first quarter round. A position past the last step has no step to show, and its
        // own diagram is not this one.
        None => (0, None),
    };
    let stage: &QuarterRoundStage = &rounds.stages[quarter_round];
    let reached = current.map_or(0, |step| step + 1);

    let mut steps = [0; STEPS_PER_QUARTER_ROUND];
    for (slot, step) in steps.iter_mut().zip(&stage.trace.steps).take(reached) {
        *slot = step.result;
    }
    let outputs = if reached == STEPS_PER_QUARTER_ROUND { stage.trace.output } else { [0; 4] };

    let mut after = stage.before;
    for (position, word) in stage.indices.into_iter().zip(outputs) {
        after[position] = word;
    }

    Shown {
        quarter_round,
        reached,
        before: stage.before,
        group: stage.indices,
        inputs: if reached > 0 { stage.trace.input } else { [0; 4] },
        steps,
        outputs,
        after,
        current,
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[cfg(test)]
mod unit_tests;
