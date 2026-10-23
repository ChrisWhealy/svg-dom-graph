//! The forces on the circles, and the measures of how quiet the system is.

use super::*;

impl SpringSystem {
    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    /// The force on each particle: the springs, and a push apart for any two circles that overlap.
    pub(in crate::force::solver) fn forces(&self) -> Vec<Vec2> {
        let mut forces = vec![Vec2::ZERO; self.particles.len()];
        for spring in &self.springs {
            let (i, j) = (spring.from, spring.to);
            let delta = self.particles[j].centre - self.particles[i].centre;
            let length = delta.length();
            let unit = unit_or_fallback(delta, length, i, j);
            let force = unit * (spring.stiffness * (length - spring.rest_length));
            forces[i] += force;
            forces[j] -= force;
        }
        for i in 0..self.particles.len() {
            for j in i + 1..self.particles.len() {
                let (a, b) = (&self.particles[i], &self.particles[j]);
                let separation = a.radius + b.radius + self.params.collision_gap;
                let delta = b.centre - a.centre;
                let length = delta.length();
                if length < separation {
                    let push =
                        unit_or_fallback(delta, length, i, j) * (self.params.contact_stiffness * (separation - length));
                    forces[i] -= push;
                    forces[j] += push;
                }
            }
        }
        forces
    }

    /// The acceleration of each free particle for `forces`. A pinned particle has none.
    pub(super) fn accelerations(&self, forces: &[Vec2]) -> Vec<Vec2> {
        self.particles
            .iter()
            .zip(forces)
            .map(|(particle, &force)| {
                if particle.pin.is_some() {
                    Vec2::ZERO
                } else {
                    force * (1.0 / particle.mass) - particle.velocity * self.params.damping
                }
            })
            .collect()
    }

    /// `acceleration` of particle `index` with any part that pushes it further into a wall or another circle that it is
    /// already touching taken away. A wall or a neighbour holds it, so that part does not move it.
    pub(in crate::force::solver) fn projected(&self, index: usize, acceleration: Vec2) -> Vec2 {
        let particle = &self.particles[index];
        let mut result = acceleration;
        let b = &self.bounds;
        let (low_x, high_x) = (b.left + particle.radius, b.right - particle.radius);
        let (low_y, high_y) = (b.top + particle.radius, b.bottom - particle.radius);
        if particle.centre.x <= low_x + TOUCH && result.x < 0.0 {
            result.x = 0.0;
        }
        if particle.centre.x >= high_x - TOUCH && result.x > 0.0 {
            result.x = 0.0;
        }
        if particle.centre.y <= low_y + TOUCH && result.y < 0.0 {
            result.y = 0.0;
        }
        if particle.centre.y >= high_y - TOUCH && result.y > 0.0 {
            result.y = 0.0;
        }
        for (other_index, other) in self.particles.iter().enumerate() {
            if other_index == index {
                continue;
            }
            let separation = particle.radius + other.radius + self.params.collision_gap;
            let delta = other.centre - particle.centre;
            let length = delta.length();
            if length <= separation + TOUCH {
                let normal = unit_or_fallback(delta, length, index, other_index);
                let into = result.dot(normal);
                if into > 0.0 {
                    result -= normal * into;
                }
            }
        }
        result
    }

    /// The largest speed of a free particle.
    pub fn max_free_speed(&self) -> f64 {
        self.particles
            .iter()
            .filter(|p| p.pin.is_none())
            .map(|p| p.velocity.length())
            .fold(0.0, f64::max)
    }

    /// The largest projected acceleration of a free particle.
    pub fn max_projected_acceleration(&self) -> f64 {
        let accelerations = self.accelerations(&self.forces());
        (0..self.particles.len())
            .filter(|&i| self.particles[i].pin.is_none())
            .map(|i| self.projected(i, accelerations[i]).length())
            .fold(0.0, f64::max)
    }

    /// The deepest any two circles overlap, measured to the collision gap, or zero if none does.
    pub fn max_penetration(&self) -> f64 {
        let mut deepest: f64 = 0.0;
        for i in 0..self.particles.len() {
            for j in i + 1..self.particles.len() {
                let (a, b) = (&self.particles[i], &self.particles[j]);
                let separation = a.radius + b.radius + self.params.collision_gap;
                deepest = deepest.max(separation - (b.centre - a.centre).length());
            }
        }
        deepest
    }

    /// The force of spring `index`, as a positive number for a stretched spring and a negative one for a compressed
    /// one.
    pub fn spring_tension(&self, index: usize) -> Option<f64> {
        let spring = self.springs.get(index)?;
        let length = (self.particles[spring.to].centre - self.particles[spring.from].centre).length();
        Some(spring.stiffness * (length - spring.rest_length))
    }

    /// Whether, in the state it is in now, every free particle is slow and settled and nothing overlaps too far.
    pub(super) fn is_quiet_now(&self) -> bool {
        self.max_free_speed() <= MAX_FREE_SPEED
            && self.max_projected_acceleration() <= MAX_PROJECTED_ACCELERATION
            && self.max_penetration() <= MAX_PENETRATION
    }
}
