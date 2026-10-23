//! The part of the weighted springs controller that needs no DOM: what each command does to the physics, and what that
//! means for the circles on the page.
//!
//! The controller in [`super::controller`] draws. This decides. It keeps the solver, the frame clock, where every
//! circle is *drawn*, whether the user or the page has paused, and the feedback a refused command leaves, and it
//! answers with plain data: which circles to move and to where, and what to say. So the rules of the demo are tested
//! with `cargo test`, and the DOM part only carries them out.
//!
//! # Who writes a circle
//!
//! A circle that a pointer holds is written by the library's own drag, never by this. A circle whose proxy is still
//! catching up after the pointer was released is not written either. Only free circles are, and only when the physics
//! has moved them from where they were last drawn.

use demo_common::force::{
    bridge::ApplyOutcome,
    clock::FrameClock,
    solver::{FIXED_DT, SolverError, SpringSystem, Status, Vec2},
};

/// The range of the random factors that give the springs their tension on a reset. Below 1 a spring is squeezed where the
/// circles are, and above 1 it is stretched.
pub(super) const TENSION_RANGE: (f64, f64) = (0.6, 1.6);

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// What a refused command leaves for the user to read.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Feedback {
    /// Reset was asked for while a pointer held a circle or a released one was still catching up.
    ResetUnavailable,
    /// New weights were refused, because the solver would not take them.
    WeightsRefused,
    /// A reset was refused, because the solver would not take the new weights or tension.
    ResetRefused,
}

impl Feedback {
    /// The sentence shown for it.
    pub(super) fn message(self) -> &'static str {
        match self {
            Feedback::ResetUnavailable => "Reset unavailable while dragging or finishing input.",
            Feedback::WeightsRefused => "The new weights were refused and nothing changed.",
            Feedback::ResetRefused => "The reset was refused and nothing changed.",
        }
    }
}

/// How long a trial of a tension may run before it is said not to settle: ten seconds of fixed steps, half the
/// watchdog, so a network that is slow to rest is not chosen.
const TRIAL_STEPS: usize = 1200;

/// How many random tensions are tried before giving up and leaving the springs as long as they are.
const TRIAL_ATTEMPTS: usize = 12;

/// One random tension factor for each of `count` springs, from `unit`, which gives numbers from 0 to 1. A number outside
/// that range, or not a number at all, still gives a factor in [`TENSION_RANGE`].
pub(super) fn random_tension(count: usize, mut unit: impl FnMut() -> f64) -> Vec<f64> {
    let (low, high) = TENSION_RANGE;
    (0..count)
        .map(|_| {
            let fraction = unit();
            let fraction = if fraction.is_nan() { 0.0 } else { fraction.clamp(0.0, 1.0) };
            low + fraction * (high - low)
        })
        .collect()
}

/// The decisions of the weighted springs demo.
pub(super) struct Core {
    system: SpringSystem,
    clock: FrameClock,
    user_paused: bool,
    hidden: bool,
    reduced_motion: bool,
    /// The centre each circle is drawn at, as far as this knows.
    drawn: Vec<Vec2>,
    feedback: Option<Feedback>,
    /// A fault that stopped the physics, in words, until it is cleared by a resume or a reset.
    fault: Option<String>,
}

impl Core {
    pub(super) fn new(system: SpringSystem) -> Self {
        let drawn = (0..system.len()).map(|i| system.centre(i).unwrap_or(Vec2::ZERO)).collect();
        Self {
            system,
            clock: FrameClock::new(),
            user_paused: false,
            hidden: false,
            reduced_motion: false,
            drawn,
            feedback: None,
            fault: None,
        }
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    pub(super) fn system(&self) -> &SpringSystem {
        &self.system
    }

    pub(super) fn status(&self) -> Status {
        self.system.status()
    }

    pub(super) fn feedback(&self) -> Option<Feedback> {
        self.feedback
    }

    pub(super) fn fault(&self) -> Option<&str> {
        self.fault.as_deref()
    }

    pub(super) fn is_user_paused(&self) -> bool {
        self.user_paused
    }

    #[cfg(test)]
    pub(super) fn is_reduced_motion(&self) -> bool {
        self.reduced_motion
    }

    /// Whether a frame should be requested. Never with reduced motion, which settles at once instead of animating.
    pub(super) fn wants_frame(&self) -> bool {
        !self.reduced_motion && self.system.wants_frames()
    }

    /// Whether the Reset control is enabled: no pointer holds a circle and none is still catching up.
    pub(super) fn reset_enabled(&self) -> bool {
        !self.system.any_held() && !self.system.any_draining()
    }

    /// The weights now, one for each spring, in the order of the edges.
    #[cfg(test)]
    pub(super) fn weights(&self) -> Vec<f64> {
        self.system.springs().iter().map(|s| s.weight).collect()
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    /// A pointer pressed circle `index`, which the page has at `centre`.
    pub(super) fn drag_start(&mut self, index: usize, centre: Vec2) {
        if self.system.begin_pin(index, centre) {
            self.drawn[index] = centre;
        }
    }

    /// The library drew a move of circle `index`, so it is now at `centre`.
    pub(super) fn drag_applied(&mut self, index: usize, centre: Vec2) {
        if self.system.update_pin_target(index, centre) {
            self.drawn[index] = centre;
        }
    }

    /// The pointer released circle `index`, which the page has at `centre`.
    pub(super) fn drag_end(&mut self, index: usize, centre: Vec2) {
        if self.system.end_pin(index, centre) {
            self.drawn[index] = centre;
        }
        self.refresh_feedback();
    }

    /// The drag of circle `index` was cancelled, and the page has it at `centre`.
    pub(super) fn drag_cancel(&mut self, index: usize, centre: Vec2) {
        if self.system.cancel_pin(index, centre) {
            self.drawn[index] = centre;
        }
        self.refresh_feedback();
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    /// A frame arrived at `timestamp_ms`. Runs as many fixed steps as the clock allows, and returns the circles to move.
    pub(super) fn frame(&mut self, timestamp_ms: f64) -> Vec<(usize, Vec2)> {
        let steps = self.clock.advance(timestamp_ms);
        for _ in 0..steps {
            if !self.system.wants_frames() {
                break;
            }
            self.system.step(FIXED_DT);
        }
        self.refresh_feedback();
        self.moves()
    }

    /// With reduced motion, runs the physics to rest at once, if it has anything to do, and returns the circles to move.
    pub(super) fn settle(&mut self) -> Vec<(usize, Vec2)> {
        if self.reduced_motion && self.system.wants_frames() {
            self.system.settle_now();
            self.refresh_feedback();
        }
        self.moves()
    }

    /// The free circles that are not where they were last drawn. A held circle, and one whose proxy is catching up, is
    /// never in the list, since the library's drag is its only writer.
    fn moves(&self) -> Vec<(usize, Vec2)> {
        (0..self.system.len())
            .filter(|&index| !self.system.is_pinned(index))
            .filter_map(|index| {
                let centre = self.system.centre(index)?;
                (centre != self.drawn[index]).then_some((index, centre))
            })
            .collect()
    }

    /// Circle `index` was drawn at its present centre without an error.
    pub(super) fn confirm_drawn(&mut self, index: usize) {
        if let Some(centre) = self.system.centre(index) {
            self.drawn[index] = centre;
        }
    }

    /// Called after every batch of commands. A system that will ask for no more frames forgets the clock's baseline, so
    /// the next time it wakes the first frame is only a new baseline and nothing is caught up.
    pub(super) fn settled_down(&mut self) {
        if !self.system.wants_frames() {
            self.clock.restart();
        }
        self.refresh_feedback();
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    /// Puts every circle back where it started, with `weights` for the springs and `tension` for their rest lengths. A
    /// tension that is not 1 leaves a spring stretched or squeezed where the circles are, so the network starts to
    /// settle by itself.
    ///
    /// It is refused, changing nothing, while a pointer holds a circle or a released one is still catching up, and the
    /// feedback says so. It is refused too if the solver will not take the weights or the tension, and nothing is
    /// changed then either. Returns the outcome and the circles to move.
    pub(super) fn reset(&mut self, weights: &[f64], tension: &[f64]) -> (ApplyOutcome, Vec<(usize, Vec2)>) {
        if !self.reset_enabled() {
            self.feedback = Some(Feedback::ResetUnavailable);
            return (ApplyOutcome::Rejected, Vec::new());
        }
        // Tried on a copy first, so a refusal part-way through leaves the real system as it was.
        let mut next = self.system.clone();
        next.reset();
        if next.set_weights(weights).and_then(|()| next.set_tension(tension)).is_err() {
            self.feedback = Some(Feedback::ResetRefused);
            return (ApplyOutcome::Rejected, Vec::new());
        }
        if matches!(self.feedback, Some(Feedback::ResetUnavailable | Feedback::ResetRefused)) {
            self.feedback = None;
        }
        self.system = next;
        self.fault = None;
        self.clock.restart();
        (ApplyOutcome::Applied, self.moves())
    }

    /// Whether a network with these `weights` and this `tension`, put back where it started, comes to rest within
    /// `steps` fixed steps. It is tried on a copy, so nothing here changes.
    ///
    /// A tension that stretches the springs against the edge of the region can leave the circles creeping so slowly that
    /// the watchdog would run out first, and then the panel would say it never settled. So a reset only takes a tension
    /// that has been seen to settle.
    pub(super) fn settles_within(&self, weights: &[f64], tension: &[f64], steps: usize) -> bool {
        let mut trial = self.system.clone();
        trial.reset();
        // A paused system takes no steps, and a pause is the user's, not part of what is being asked.
        trial.set_paused(false);
        if trial.set_weights(weights).and_then(|()| trial.set_tension(tension)).is_err() {
            return false;
        }
        for _ in 0..steps {
            if !trial.wants_frames() {
                break;
            }
            trial.step(FIXED_DT);
        }
        trial.status() == Status::Sleeping
    }

    /// [`Self::settles_within`] for the longest a reset allows.
    #[cfg(test)]
    pub(super) fn settles(&self, weights: &[f64], tension: &[f64]) -> bool {
        self.settles_within(weights, tension, TRIAL_STEPS)
    }

    /// A random tension for a reset with these `weights`, drawn from `unit` (numbers from 0 to 1) and known to settle.
    /// If none of several tries does, every spring is left as long as it is, which always does.
    pub(super) fn choose_tension(&self, weights: &[f64], unit: impl FnMut() -> f64) -> Vec<f64> {
        self.choose_tension_in(weights, unit, TRIAL_ATTEMPTS, TRIAL_STEPS)
    }

    /// [`Self::choose_tension`] with its number of tries and the length of a trial given.
    pub(super) fn choose_tension_in(
        &self,
        weights: &[f64],
        mut unit: impl FnMut() -> f64,
        attempts: usize,
        steps: usize,
    ) -> Vec<f64> {
        let count = self.system.springs().len();
        for _ in 0..attempts {
            let tension = random_tension(count, &mut unit);
            if self.settles_within(weights, &tension, steps) {
                return tension;
            }
        }
        vec![1.0; count]
    }

    /// Gives the springs these weights. Refused, changing nothing, if the solver will not take them.
    pub(super) fn randomise(&mut self, weights: &[f64]) -> ApplyOutcome {
        match self.system.set_weights(weights) {
            Ok(()) => {
                if self.feedback == Some(Feedback::WeightsRefused) {
                    self.feedback = None;
                }
                ApplyOutcome::Applied
            },
            Err(SolverError::WrongWeightCount | SolverError::InvalidWeight(_)) | Err(_) => {
                self.feedback = Some(Feedback::WeightsRefused);
                ApplyOutcome::Rejected
            },
        }
    }

    /// The user paused or resumed. A resume starts from a fresh clock and keeps no time against the watchdog.
    pub(super) fn set_user_paused(&mut self, paused: bool) {
        self.user_paused = paused;
        if !paused {
            self.fault = None;
        }
        self.apply_pause();
    }

    /// The page was hidden or shown again.
    pub(super) fn set_hidden(&mut self, hidden: bool) {
        self.hidden = hidden;
        self.apply_pause();
    }

    fn apply_pause(&mut self) {
        let paused = self.user_paused || self.hidden;
        let was_paused = self.system.is_paused();
        self.system.set_paused(paused);
        if was_paused && !paused {
            self.clock.restart();
        }
    }

    /// The page asks for reduced motion, or no longer does.
    pub(super) fn set_reduced_motion(&mut self, reduced: bool) {
        self.reduced_motion = reduced;
    }

    /// Drawing failed. The solver's circles are put where the page really has them, the physics is paused, and `message`
    /// says why. Resuming, or resetting, clears it.
    pub(super) fn fail(&mut self, centres: &[Vec2], message: String) {
        if self.system.resync_centres(centres) {
            self.drawn = centres.to_vec();
        }
        self.user_paused = true;
        self.apply_pause();
        self.fault = Some(message);
    }

    fn refresh_feedback(&mut self) {
        if self.feedback == Some(Feedback::ResetUnavailable) && self.reset_enabled() {
            self.feedback = None;
        }
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    /// What the status line says, in words.
    pub(super) fn status_text(&self) -> String {
        if let Some(fault) = &self.fault {
            return format!("Physics stopped: {fault}");
        }
        match self.status() {
            Status::Sleeping => "Sleeping: the graph is at rest.".to_owned(),
            Status::Running => "Running: the graph is settling.".to_owned(),
            Status::Dragging if self.system.any_draining() && !self.system.any_held() => {
                "Finishing input: the released circle is catching up.".to_owned()
            },
            Status::Dragging => "Dragging.".to_owned(),
            Status::Paused if self.hidden && !self.user_paused => "Paused: the page is hidden.".to_owned(),
            Status::Paused => "Paused.".to_owned(),
            Status::Unsettled => "Unsettled: the graph did not come to rest within 20 seconds.".to_owned(),
        }
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[cfg(test)]
mod unit_tests;
