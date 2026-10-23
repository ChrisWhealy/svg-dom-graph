//! The spring system: its state, how it is built, and what can be asked of it.
//!
//! The rest of what it does is in the modules beside this one:
//!
//! - [`pins`] — holding a circle with a pointer, and letting go.
//! - [`tuning`] — changing weights, tension, damping and pause, and resetting.
//! - [`forces`] — the forces and accelerations, and the measures of how quiet the system is.
//! - [`stepping`] — advancing the proxies, applying the constraints, and one fixed step.

use super::*;
use std::f64::consts::TAU;

mod forces;
mod pins;
mod stepping;
mod tuning;

/// A direction from particle `i` to particle `j` when their centres coincide, so there is none to divide by. It depends
/// only on the two indices, so the same input always gives the same answer, and `(i, j)` and `(j, i)` are opposite.
pub(super) fn fallback_direction(i: usize, j: usize) -> Vec2 {
    let (low, high) = (i.min(j), i.max(j));
    let angle = TAU * ((low * 7 + high * 13) % 16) as f64 / 16.0;
    let direction = Vec2::new(angle.cos(), angle.sin());
    if i <= j { direction } else { -direction }
}

/// The unit vector of `delta`, which is `length` long, or the fallback for particles `i` and `j`.
pub(super) fn unit_or_fallback(delta: Vec2, length: f64, i: usize, j: usize) -> Vec2 {
    if length > 1e-12 { delta * (1.0 / length) } else { fallback_direction(i, j) }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// A damped spring graph.
#[derive(Clone, Debug)]
pub struct SpringSystem {
    pub(super) particles: Vec<Particle>,
    pub(super) initial: Vec<Vec2>,
    /// Each spring's rest length when the system was built. A tension is always a multiple of this, never of the last
    /// tension, so setting one again does not compound.
    pub(super) natural_lengths: Vec<f64>,
    pub(super) springs: Vec<Spring>,
    pub(super) params: Params,
    pub(super) bounds: Bounds,
    pub(super) quiet_streak: u32,
    pub(super) asleep: bool,
    pub(super) paused: bool,
    pub(super) unsettled: bool,
    pub(super) faulted: bool,
    /// Set when a pin is told its target again without it changing. The next step is then allowed to be one of zero
    /// motion, which does not need a fresh run of quiet steps.
    pub(super) rearmed: bool,
    pub(super) active_time: f64,
}

impl SpringSystem {
    /// A system of one particle for each of `centres`, with the radius in the same place of `radii`, and one spring for
    /// each of `edges`, given as `(from, to, weight)`. Each spring's rest length is the distance between its ends now.
    ///
    /// The system starts asleep if it starts at rest.
    ///
    /// # Errors
    ///
    /// Returns a [`SolverError`] for anything that is not finite, a radius that is negative, a spring that is a
    /// self-loop, has an unknown end or no length, a weight outside `0..=`[`MAX_WEIGHT`], a bad parameter, bounds with
    /// no room for a circle, or a particle that starts outside them. Nothing is built then.
    pub fn new(
        centres: &[Vec2],
        radii: &[f64],
        edges: &[(usize, usize, f64)],
        bounds: Bounds,
        params: Params,
    ) -> Result<Self, SolverError> {
        let count = centres.len();
        if count == 0 || count > MAX_PARTICLES || radii.len() != count {
            return Err(SolverError::BadParticleCount);
        }
        if edges.len() > MAX_SPRINGS {
            return Err(SolverError::TooManySprings);
        }
        params.validate()?;
        if !bounds.is_valid() {
            return Err(SolverError::InvalidBounds);
        }
        let mut particles = Vec::with_capacity(count);
        for (index, (&centre, &radius)) in centres.iter().zip(radii).enumerate() {
            if !centre.is_finite() || !radius.is_finite() || radius < 0.0 {
                return Err(SolverError::NonFinite);
            }
            if !bounds.holds_circle(radius) {
                return Err(SolverError::InvalidBounds);
            }
            let inside = bounds.clamp_centre(centre, radius);
            if (inside - centre).length() > 1e-9 {
                return Err(SolverError::OutsideBounds(index));
            }
            particles.push(Particle {
                centre,
                velocity: Vec2::ZERO,
                mass: params.mass,
                radius,
                pin: None,
            });
        }
        let mut springs = Vec::with_capacity(edges.len());
        for (index, &(from, to, weight)) in edges.iter().enumerate() {
            if from >= count || to >= count {
                return Err(SolverError::UnknownParticle(index));
            }
            if from == to {
                return Err(SolverError::SelfLoop(index));
            }
            if !weight.is_finite() || !(0.0..=MAX_WEIGHT).contains(&weight) {
                return Err(SolverError::InvalidWeight(index));
            }
            // The difference of two finite centres can overflow, and so a rest length must be checked once it is worked
            // out.
            let rest_length = (particles[to].centre - particles[from].centre).length();
            if !rest_length.is_finite() {
                return Err(SolverError::NonFiniteSpring(index));
            }
            if rest_length <= 1e-9 {
                return Err(SolverError::ZeroRestLength(index));
            }
            springs.push(Spring {
                from,
                to,
                weight,
                stiffness: params.stiffness_of(weight),
                rest_length,
            });
        }
        let mut system = Self {
            initial: centres.to_vec(),
            natural_lengths: springs.iter().map(|spring| spring.rest_length).collect(),
            particles,
            springs,
            params,
            bounds,
            quiet_streak: 0,
            asleep: false,
            paused: false,
            unsettled: false,
            faulted: false,
            rearmed: false,
            active_time: 0.0,
        };
        system.rest_if_quiet();
        Ok(system)
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    /// How many particles there are.
    pub fn len(&self) -> usize {
        self.particles.len()
    }

    /// Whether there are no particles. A system always has at least one.
    pub fn is_empty(&self) -> bool {
        self.particles.is_empty()
    }

    pub fn particle(&self, index: usize) -> Option<&Particle> {
        self.particles.get(index)
    }

    /// The centre of particle `index`, which for a pinned particle is its proxy.
    pub fn centre(&self, index: usize) -> Option<Vec2> {
        self.particles.get(index).map(|particle| particle.centre)
    }

    pub fn springs(&self) -> &[Spring] {
        &self.springs
    }

    pub fn params(&self) -> Params {
        self.params
    }

    pub fn bounds(&self) -> Bounds {
        self.bounds
    }

    /// How many quiet steps in a row there have been.
    pub fn quiet_streak(&self) -> u32 {
        self.quiet_streak
    }

    /// The simulated seconds since the last release or parameter change, in which no pin was held.
    pub fn active_time(&self) -> f64 {
        self.active_time
    }

    /// Whether particle `index` is pinned: held, or catching up after a release.
    pub fn is_pinned(&self, index: usize) -> bool {
        self.particles.get(index).is_some_and(|particle| particle.pin.is_some())
    }

    /// Whether any pin is held by a pointer, as opposed to draining after one.
    pub fn any_held(&self) -> bool {
        self.particles.iter().any(|p| p.pin.is_some_and(|pin| !pin.release_on_arrival))
    }

    /// Whether any proxy is still catching up after its pointer was released.
    pub fn any_draining(&self) -> bool {
        self.particles.iter().any(|p| p.pin.is_some_and(|pin| pin.release_on_arrival))
    }

    pub fn is_paused(&self) -> bool {
        self.paused
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    /// What the system is doing.
    pub fn status(&self) -> Status {
        if self.paused {
            Status::Paused
        } else if self.unsettled || self.faulted {
            Status::Unsettled
        } else if self.particles.iter().any(|p| p.pin.is_some()) {
            Status::Dragging
        } else if self.asleep {
            Status::Sleeping
        } else {
            Status::Running
        }
    }

    /// Whether a frame should be scheduled. It is not while paused, unsettled, asleep, or holding a pin that has
    /// arrived with 24 quiet steps behind it and nothing new to look at.
    ///
    /// A held node that stops asking for frames is *quiescent*, which is not the same as sleeping. A new target, a new
    /// pin or a release asks for frames again.
    pub fn wants_frames(&self) -> bool {
        !self.paused && !self.unsettled && !self.faulted && !self.asleep && !self.idle_held()
    }

    pub(super) fn idle_held(&self) -> bool {
        let pinned = self.particles.iter().any(|p| p.pin.is_some());
        pinned && !self.any_draining() && self.proxies_arrived() && self.quiet_streak >= QUIET_STEPS && !self.rearmed
    }

    pub(super) fn proxies_arrived(&self) -> bool {
        self.particles
            .iter()
            .all(|p| p.pin.is_none_or(|pin| pin.committed_centre == p.centre))
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    /// Notes that something changed that the system must settle again from: the quiet run, the time budget and any
    /// sleep or unsettled verdict are all begun afresh.
    pub(super) fn disturb(&mut self) {
        self.asleep = false;
        self.unsettled = false;
        self.rearmed = false;
        self.quiet_streak = 0;
        self.active_time = 0.0;
    }
}
