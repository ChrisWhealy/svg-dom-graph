//! Where the FFT panel is: which stage is complete, which butterfly is inspected, and what to do when a repaint fails.
//! Pure: the repaints are closures, so every failure can be injected in a native test.
//!
//! The stepping toolbar owns the stage. Its selection is `None` for "unstarted" and `Some(i)` for "stages 1 to `i + 1`
//! are complete". This cursor keeps the last position that was drawn successfully, only so that a failed repaint can be
//! undone and so that an out-of-step callback can be detected. It never drives ordinary stepping by itself.
//!
//! A repaint writes many nodes and is not atomic. So a failed step puts the toolbar back, and draws the last good state
//! again. If that fails too, the panel is in a repair state: it refuses further steps until a retry draws the last good
//! state and puts the toolbar right.

use super::algorithm::{FftTrace, first_informative_pair};

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The toolbar selection that means `stage` stages are complete.
pub(super) const fn selection_of(stage: usize) -> Option<usize> {
    stage.checked_sub(1)
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// How many stages are complete when the toolbar's selection is `selection`.
pub(super) const fn stage_of(selection: Option<usize>) -> usize {
    match selection {
        Some(index) => index + 1,
        None => 0,
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The stage whose butterflies the inspector offers when `completed` stages are complete. Before any stage is complete
/// it previews stage 1.
pub(super) const fn shown_stage(completed: usize) -> usize {
    if completed == 0 { 1 } else { completed }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// What became of a request to change the display.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) enum Outcome {
    /// The display shows the new state, and the cursor has moved.
    Committed,
    /// The repaint failed, and the last good state is drawn again. Carries the first error.
    RolledBack(String),
    /// The repaint failed, and so did drawing the last good state. Carries both errors.
    NeedsRepair(String),
    /// The request was refused because a repair is outstanding, and the display was left alone.
    Refused,
    /// The toolbar was not where the cursor said, so the display was put back to the cursor's state first.
    Resynchronised,
}

/// The last good state of the display.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct Cursor {
    stages: usize,
    completed: usize,
    pair: usize,
    /// The pair chosen at each stage, from stage 1, once a repaint of it has succeeded.
    cache: Vec<Option<usize>>,
    repair: bool,
}

impl Cursor {
    /// A cursor at "unstarted" for a transform of `stages` stages, on pair 0 with nothing chosen. The display starts on
    /// [`unstarted`](Self::unstarted), which knows which pair that is.
    pub(super) fn new(stages: usize) -> Self {
        Self {
            stages,
            completed: 0,
            pair: 0,
            cache: vec![None; stages],
            repair: false,
        }
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    /// A cursor at "unstarted" for `trace`, whose pair is the one the display starts on, and which is remembered for the
    /// preview of stage 1. That is the pair to draw first, so a failed repaint, or a retry, goes back to it and not to the
    /// first pair, which is only the same when the first butterfly is the first one with a nonzero input.
    pub(super) fn unstarted(trace: &FftTrace) -> Self {
        let mut cursor = Self::new(trace.stages());
        let pair = cursor.pair_for(trace, 0);
        cursor.remember(0, pair);
        cursor
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    /// How many stages are complete in the last good state.
    pub(super) const fn completed(&self) -> usize {
        self.completed
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    /// The pair the inspector showed in the last good state, counted from 0.
    pub(super) const fn pair(&self) -> usize {
        self.pair
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    #[cfg(test)]
    pub(super) const fn needs_repair(&self) -> bool {
        self.repair
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    /// The pair to show at `completed` stages: the one chosen there before, otherwise the first with a nonzero input.
    pub(super) fn pair_for(&self, trace: &FftTrace, completed: usize) -> usize {
        let stage = shown_stage(completed);
        let pairs = trace.stage_pairs(stage);
        self.cache
            .get(stage - 1)
            .copied()
            .flatten()
            .filter(|&pair| pair < pairs.len())
            .unwrap_or_else(|| first_informative_pair(pairs))
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    /// Forgets every pair that was chosen, but stays where it is. Used when the display is about to return to
    /// "unstarted", so that the pair shown is the default again.
    pub(super) fn forget_choices(&mut self) {
        self.cache.iter_mut().for_each(|slot| *slot = None);
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    fn remember(&mut self, completed: usize, pair: usize) {
        self.completed = completed;
        self.pair = pair;
        if let Some(slot) = self.cache.get_mut(shown_stage(completed) - 1) {
            *slot = Some(pair);
        }
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Handles the stepping toolbar's callback. `from` and `to` are the transition it reported. `render(completed, pair)`
/// draws the whole display, and `set_driver(completed)` puts the toolbar's selection where `completed` stages means.
pub(super) fn apply_transition(
    cursor: &mut Cursor,
    trace: &FftTrace,
    from: Option<usize>,
    to: Option<usize>,
    render: &mut dyn FnMut(usize, usize) -> Result<(), String>,
    set_driver: &mut dyn FnMut(usize) -> Result<(), String>,
) -> Outcome {
    if cursor.repair {
        let _ = set_driver(cursor.completed);
        return Outcome::Refused;
    }
    if from != selection_of(cursor.completed) {
        // The toolbar and the display disagree. Do not apply the move, and do not skip a stage.
        return match set_driver(cursor.completed).and_then(|()| render(cursor.completed, cursor.pair)) {
            Ok(()) => Outcome::Resynchronised,
            Err(e) => {
                cursor.repair = true;
                Outcome::NeedsRepair(e)
            },
        };
    }
    let wanted = stage_of(to).min(cursor.stages);
    let pair = cursor.pair_for(trace, wanted);
    match render(wanted, pair) {
        Ok(()) => {
            cursor.remember(wanted, pair);
            Outcome::Committed
        },
        Err(first) => undo(cursor, first, render, set_driver),
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Handles a change of the pair selector to `pair`.
pub(super) fn apply_pair(
    cursor: &mut Cursor,
    pair: usize,
    render: &mut dyn FnMut(usize, usize) -> Result<(), String>,
) -> Outcome {
    if cursor.repair {
        return Outcome::Refused;
    }
    match render(cursor.completed, pair) {
        Ok(()) => {
            cursor.remember(cursor.completed, pair);
            Outcome::Committed
        },
        Err(first) => match render(cursor.completed, cursor.pair) {
            Ok(()) => Outcome::RolledBack(first),
            Err(second) => {
                cursor.repair = true;
                Outcome::NeedsRepair(format!("{first}; then {second}"))
            },
        },
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Draws the last good state again, puts the toolbar right, and ends the repair state if both work.
pub(super) fn retry(
    cursor: &mut Cursor,
    render: &mut dyn FnMut(usize, usize) -> Result<(), String>,
    set_driver: &mut dyn FnMut(usize) -> Result<(), String>,
) -> Result<(), String> {
    render(cursor.completed, cursor.pair).and_then(|()| set_driver(cursor.completed))?;
    cursor.repair = false;
    Ok(())
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
fn undo(
    cursor: &mut Cursor,
    first: String,
    render: &mut dyn FnMut(usize, usize) -> Result<(), String>,
    set_driver: &mut dyn FnMut(usize) -> Result<(), String>,
) -> Outcome {
    // The toolbar has already moved, so it goes back first.
    match set_driver(cursor.completed).and_then(|()| render(cursor.completed, cursor.pair)) {
        Ok(()) => Outcome::RolledBack(first),
        Err(second) => {
            cursor.repair = true;
            Outcome::NeedsRepair(format!("{first}; then {second}"))
        },
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[cfg(test)]
mod unit_tests;
