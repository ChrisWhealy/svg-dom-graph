//! Changing the weights, the tension, the damping and the pause, and putting the system back.

use super::*;

impl SpringSystem {
    /// Gives every spring a new weight, which changes its stiffness and nothing else: not its rest length, and not any
    /// position. It is a change to the forces, so the system starts to settle again, whether or not anything moves.
    ///
    /// # Errors
    ///
    /// Returns [`SolverError::WrongWeightCount`] unless there is one weight for each spring, and
    /// [`SolverError::InvalidWeight`] for a weight that is not finite or is outside `0..=`[`MAX_WEIGHT`]. Nothing is
    /// changed then. A weight whose stiffness would not be finite gives [`SolverError::NonFiniteSpring`], and also
    /// changes nothing.
    pub fn set_weights(&mut self, weights: &[f64]) -> Result<(), SolverError> {
        if weights.len() != self.springs.len() {
            return Err(SolverError::WrongWeightCount);
        }
        if let Some(bad) = weights.iter().position(|w| !w.is_finite() || !(0.0..=MAX_WEIGHT).contains(w)) {
            return Err(SolverError::InvalidWeight(bad));
        }
        // Worked out first, so that a stiffness that overflows changes nothing.
        let stiffnesses: Vec<f64> = weights.iter().map(|&w| self.params.stiffness_of(w)).collect();
        if let Some(bad) = stiffnesses.iter().position(|k| !k.is_finite()) {
            return Err(SolverError::NonFiniteSpring(bad));
        }
        for ((spring, &weight), stiffness) in self.springs.iter_mut().zip(weights).zip(stiffnesses) {
            spring.weight = weight;
            spring.stiffness = stiffness;
        }
        self.disturb();
        Ok(())
    }

    /// Gives every spring a rest length of its natural length times the matching factor, so a factor above 1 leaves the
    /// spring stretched where the circles now are, and one below 1 leaves it squeezed. The natural length is the
    /// distance between the spring's ends when the system was built, and every call counts from that, so repeated calls
    /// do not compound. It is a change to the forces, so the system starts to settle again.
    ///
    /// # Errors
    ///
    /// Returns [`SolverError::WrongTensionCount`] unless there is one factor for each spring, and
    /// [`SolverError::InvalidTension`] for a factor that is not finite, is not above zero, or is above
    /// [`MAX_TENSION_FACTOR`]. Returns [`SolverError::NonFiniteSpring`] if a rest length would be too large to be a
    /// finite number. Nothing is changed then.
    pub fn set_tension(&mut self, factors: &[f64]) -> Result<(), SolverError> {
        if factors.len() != self.springs.len() {
            return Err(SolverError::WrongTensionCount);
        }
        if let Some(bad) = factors
            .iter()
            .position(|f| !f.is_finite() || *f <= 0.0 || *f > MAX_TENSION_FACTOR)
        {
            return Err(SolverError::InvalidTension(bad));
        }
        // Worked out first, so that a rest length that overflows changes nothing.
        let lengths: Vec<f64> = self
            .natural_lengths
            .iter()
            .zip(factors)
            .map(|(natural, factor)| natural * factor)
            .collect();
        if let Some(bad) = lengths.iter().position(|l| !l.is_finite()) {
            return Err(SolverError::NonFiniteSpring(bad));
        }
        for (spring, length) in self.springs.iter_mut().zip(lengths) {
            spring.rest_length = length;
        }
        self.disturb();
        Ok(())
    }

    /// Sets the damping, which changes the forces and so begins the settling again.
    ///
    /// # Errors
    ///
    /// Returns [`SolverError::InvalidParams`] unless `damping` is finite and not negative.
    pub fn set_damping(&mut self, damping: f64) -> Result<(), SolverError> {
        if !damping.is_finite() || damping < 0.0 {
            return Err(SolverError::InvalidParams);
        }
        self.params.damping = damping;
        self.disturb();
        Ok(())
    }

    /// Stops or resumes the system. Nothing moves while it is paused, and no pin or position is lost. Resuming begins
    /// the settling again from a fresh quiet run, but it keeps no time from before the pause against the watchdog.
    pub fn set_paused(&mut self, paused: bool) {
        if self.paused == paused {
            return;
        }
        self.paused = paused;
        if !paused {
            self.asleep = false;
            self.rearmed = false;
            self.quiet_streak = 0;
            self.active_time = 0.0;
        }
    }

    /// Puts every particle back where it started, with no velocity and no pin, keeps every rest length and weight, and
    /// puts the system to sleep if it is at rest there. It does not change whether the system is paused.
    pub fn reset(&mut self) {
        for (particle, &centre) in self.particles.iter_mut().zip(&self.initial) {
            particle.centre = centre;
            particle.velocity = Vec2::ZERO;
            particle.pin = None;
        }
        self.faulted = false;
        self.disturb();
        self.rest_if_quiet();
    }

    /// Takes the centres of every particle from `centres`, which are where the scene really has them, and clears every
    /// velocity and pin. It is for the moment after something went wrong drawing: the solver and the scene may then
    /// disagree, and the scene is the one that is right. Each centre is put inside the bounds, and the rest lengths and
    /// weights are kept. The system is left disturbed, and paused if it was.
    ///
    /// Returns `false`, changing nothing, unless there is exactly one finite centre for each particle.
    pub fn resync_centres(&mut self, centres: &[Vec2]) -> bool {
        if centres.len() != self.particles.len() || centres.iter().any(|c| !c.is_finite()) {
            return false;
        }
        for (particle, &centre) in self.particles.iter_mut().zip(centres) {
            particle.centre = self.bounds.clamp_centre(centre, particle.radius);
            particle.velocity = Vec2::ZERO;
            particle.pin = None;
        }
        self.faulted = false;
        self.disturb();
        true
    }

    /// Puts the system to sleep now if it is at rest, with a full quiet run.
    pub(super) fn rest_if_quiet(&mut self) {
        if self.particles.iter().all(|p| p.pin.is_none()) && self.is_quiet_now() {
            self.asleep = true;
            self.quiet_streak = QUIET_STEPS;
        }
    }

    /// Runs the system, ignoring time and frames, until it sleeps or the watchdog runs out. Returns whether it slept.
    /// It is for a page that asks for reduced motion, which jumps to the resting arrangement and does not animate to
    /// it.
    pub fn settle_now(&mut self) -> bool {
        let limit = (WATCHDOG_SECONDS / FIXED_DT) as usize + 1;
        for _ in 0..limit {
            if !self.wants_frames() {
                break;
            }
            self.step(FIXED_DT);
        }
        self.status() == Status::Sleeping
    }
}
