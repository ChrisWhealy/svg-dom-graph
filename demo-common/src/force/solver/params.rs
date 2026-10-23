//! The physics parameters, and the checks that make them safe to use.

use super::{MAX_WEIGHT, SolverError};

/// The numbers that shape the physics.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Params {
    /// The mass of every particle.
    pub mass: f64,
    /// Stiffness of a spring of the reference weight, in s⁻² for a mass of one.
    pub base_stiffness: f64,
    /// The weight that has exactly `base_stiffness`.
    pub reference_weight: f64,
    /// Velocity damping, in s⁻¹.
    pub damping: f64,
    /// Clear space kept between two circles, beyond their radii.
    pub collision_gap: f64,
    /// The fastest a pinned proxy moves, in units per second.
    pub proxy_speed_limit: f64,
    /// How hard overlapping circles push each other apart, per unit of penetration.
    pub contact_stiffness: f64,
}

impl Default for Params {
    fn default() -> Self {
        Self {
            mass: 1.0,
            base_stiffness: 25.0,
            reference_weight: 1.0,
            damping: 10.0,
            collision_gap: 6.0,
            proxy_speed_limit: 720.0,
            contact_stiffness: 400.0,
        }
    }
}

impl Params {
    /// The stiffness of a spring of `weight`.
    pub(super) fn stiffness_of(&self, weight: f64) -> f64 {
        self.base_stiffness * weight / self.reference_weight
    }

    pub(super) fn validate(&self) -> Result<(), SolverError> {
        let ok = |value: f64, positive: bool| value.is_finite() && if positive { value > 0.0 } else { value >= 0.0 };
        let valid = ok(self.mass, true)
            && ok(self.base_stiffness, false)
            && ok(self.reference_weight, true)
            && ok(self.damping, false)
            && ok(self.collision_gap, false)
            && ok(self.proxy_speed_limit, true)
            && ok(self.contact_stiffness, false)
            // A stiffness is `base_stiffness * weight / reference_weight`, and each of those is finite on its own. So
            // the stiffest spring that a weight can ask for must be finite too, or a permitted weight could overflow.
            && self.stiffness_of(MAX_WEIGHT).is_finite();
        if valid { Ok(()) } else { Err(SolverError::InvalidParams) }
    }
}
