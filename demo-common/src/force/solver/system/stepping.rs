//! Advancing the proxies, applying the constraints, and taking one fixed step.

use super::*;

impl SpringSystem {
    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    /// Advances every pinned proxy towards its target by at most `proxy_speed_limit * dt`, and lets go of any that has
    /// arrived and was to be released. A proxy that arrives is put exactly on its target, so arrival is exact.
    ///
    /// Returns which particles were let go, as a flag for each. A particle let go this step is not integrated in it: it
    /// is free from the next step, and starts from rest exactly where its proxy arrived, with no sudden change.
    pub(super) fn advance_proxies(&mut self, dt: f64) -> Vec<bool> {
        let max_step = self.params.proxy_speed_limit * dt;
        let bounds = self.bounds;
        let mut let_go = vec![false; self.particles.len()];
        let mut released = false;
        for (index, particle) in self.particles.iter_mut().enumerate() {
            let Some(pin) = particle.pin else {
                continue;
            };
            let delta = pin.committed_centre - particle.centre;
            let length = delta.length();
            if length <= max_step {
                particle.centre = pin.committed_centre;
            } else {
                particle.centre += delta * (max_step / length);
            }
            particle.centre = bounds.clamp_centre(particle.centre, particle.radius);
            particle.velocity = Vec2::ZERO;
            if pin.release_on_arrival && particle.centre == pin.committed_centre {
                particle.pin = None;
                let_go[index] = true;
                released = true;
            }
        }
        if released {
            // The released quiet window begins when the last proxy has let go, not before.
            self.quiet_streak = 0;
            if !self.any_draining() {
                self.rearmed = false;
            }
        }
        let_go
    }

    /// Keeps every free circle inside the bounds, pushes overlapping circles apart, and takes away the velocity that
    /// would carry a circle into a wall or a neighbour. A pinned proxy never gives way.
    pub(super) fn constrain(&mut self) {
        let count = self.particles.len();
        for _ in 0..CONSTRAINT_PASSES {
            for particle in &mut self.particles {
                if particle.pin.is_none() {
                    let clamped = self.bounds.clamp_centre(particle.centre, particle.radius);
                    if clamped.x != particle.centre.x {
                        particle.centre.x = clamped.x;
                    }
                    if clamped.y != particle.centre.y {
                        particle.centre.y = clamped.y;
                    }
                }
            }
            for i in 0..count {
                for j in i + 1..count {
                    let (a, b) = (self.particles[i], self.particles[j]);
                    let separation = a.radius + b.radius + self.params.collision_gap;
                    let delta = b.centre - a.centre;
                    let length = delta.length();
                    if length >= separation {
                        continue;
                    }
                    let normal = unit_or_fallback(delta, length, i, j);
                    let needed = separation - length;
                    match (a.pin.is_some(), b.pin.is_some()) {
                        (true, true) => {},
                        (true, false) => self.particles[j].centre += normal * needed,
                        (false, true) => self.particles[i].centre -= normal * needed,
                        (false, false) => {
                            self.particles[i].centre -= normal * (needed / 2.0);
                            self.particles[j].centre += normal * (needed / 2.0);
                        },
                    }
                }
            }
        }
        for particle in &mut self.particles {
            if particle.pin.is_none() {
                particle.centre = self.bounds.clamp_centre(particle.centre, particle.radius);
            }
        }
        self.remove_inward_velocity();
    }

    /// Takes away the part of a free particle's velocity that points into a wall it is touching, or into a neighbour it
    /// is touching, so that a circle at rest against something does not keep pressing into it.
    pub(super) fn remove_inward_velocity(&mut self) {
        let count = self.particles.len();
        let b = self.bounds;
        for particle in &mut self.particles {
            if particle.pin.is_some() {
                continue;
            }
            let (low_x, high_x) = (b.left + particle.radius, b.right - particle.radius);
            let (low_y, high_y) = (b.top + particle.radius, b.bottom - particle.radius);
            if particle.centre.x <= low_x + TOUCH && particle.velocity.x < 0.0 {
                particle.velocity.x = 0.0;
            }
            if particle.centre.x >= high_x - TOUCH && particle.velocity.x > 0.0 {
                particle.velocity.x = 0.0;
            }
            if particle.centre.y <= low_y + TOUCH && particle.velocity.y < 0.0 {
                particle.velocity.y = 0.0;
            }
            if particle.centre.y >= high_y - TOUCH && particle.velocity.y > 0.0 {
                particle.velocity.y = 0.0;
            }
        }
        for i in 0..count {
            for j in i + 1..count {
                let (a, bb) = (self.particles[i], self.particles[j]);
                let separation = a.radius + bb.radius + self.params.collision_gap;
                let delta = bb.centre - a.centre;
                let length = delta.length();
                if length > separation + TOUCH {
                    continue;
                }
                let normal = unit_or_fallback(delta, length, i, j);
                // The speed at which `a` approaches `bb` along the line between them.
                let approach = (a.velocity - bb.velocity).dot(normal);
                if approach <= 0.0 {
                    continue;
                }
                match (a.pin.is_some(), bb.pin.is_some()) {
                    (true, true) => {},
                    (true, false) => self.particles[j].velocity += normal * approach,
                    (false, true) => self.particles[i].velocity -= normal * approach,
                    (false, false) => {
                        self.particles[i].velocity -= normal * (approach / 2.0);
                        self.particles[j].velocity += normal * (approach / 2.0);
                    },
                }
            }
        }
    }

    /// Advances the system by `dt` seconds, clamped to a safe length.
    ///
    /// Pinned proxies move first, towards their targets and at their speed limit. Springs and contacts then act on
    /// every particle's position, and only the free ones are integrated. It does nothing, and says so, when the system
    /// is asleep, paused or unsettled, or if `dt` is not a positive finite number.
    ///
    /// If any number in the result is not finite the whole step is undone, and the system reports itself unsettled.
    pub fn step(&mut self, dt: f64) -> StepOutcome {
        if self.paused || self.unsettled || self.faulted || self.asleep || !dt.is_finite() || dt <= 0.0 {
            return StepOutcome {
                quiet_steps: self.quiet_streak,
                ..StepOutcome::default()
            };
        }
        let dt = dt.min(MAX_STEP);
        let saved = self.particles.clone();

        let let_go = self.advance_proxies(dt);
        let forces = self.forces();
        let accelerations = self.accelerations(&forces);
        for ((particle, &acceleration), &just_released) in self.particles.iter_mut().zip(&accelerations).zip(&let_go) {
            if particle.pin.is_none() && !just_released {
                particle.velocity += acceleration * dt;
                particle.centre += particle.velocity * dt;
            }
        }
        self.constrain();

        if self.particles.iter().any(|p| !p.centre.is_finite() || !p.velocity.is_finite()) {
            self.particles = saved;
            self.faulted = true;
            return StepOutcome {
                stepped: true,
                faulted: true,
                quiet_steps: self.quiet_streak,
                ..StepOutcome::default()
            };
        }

        // An unchanged target noted since the last step has had its one frame.
        self.rearmed = false;

        let quiet = self.is_quiet_now() && self.proxies_arrived() && !self.any_draining();
        self.quiet_streak = if quiet { self.quiet_streak.saturating_add(1) } else { 0 };

        let mut slept = false;
        let mut unsettled = false;
        let pinned = self.particles.iter().any(|p| p.pin.is_some());
        if !pinned && self.quiet_streak >= QUIET_STEPS {
            for particle in &mut self.particles {
                particle.velocity = Vec2::ZERO;
            }
            self.asleep = true;
            slept = true;
        }
        if !self.any_held() && !self.asleep {
            self.active_time += dt;
            if self.active_time >= WATCHDOG_SECONDS {
                self.unsettled = true;
                unsettled = true;
            }
        }
        StepOutcome {
            stepped: true,
            quiet_steps: self.quiet_streak,
            slept,
            unsettled,
            faulted: false,
        }
    }
}
