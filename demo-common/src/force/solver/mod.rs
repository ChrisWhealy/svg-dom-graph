//! The numerical model of a damped spring graph, with no DOM: particles joined by springs whose stiffness is
//! proportional to a weight, a kinematic proxy for a held particle, contact and boundary projection, and the rules that
//! decide when the system is asleep. Nothing here knows about a scene, a node id or an element, so all of it is tested
//! with a plain `cargo test`.
//!
//! # The model
//!
//! Every spring has a rest length, captured from the distance between its ends when the system is built. A spring's
//! force is `stiffness * (length - rest_length)` along its line, equal and opposite on its two ends, whichever way the
//! edge it stands for was drawn. Its stiffness is `base_stiffness * weight / reference_weight`, so a heavier edge is
//! harder to stretch. Weights scale stiffness only. They never change a rest length, so changing a weight alone does
//! not change the arrangement the springs try to recover.
//!
//! A free particle is integrated with semi-implicit Euler, `a = F / m - damping * v`, at a fixed step.
//!
//! # A held particle is a proxy
//!
//! A particle that is pinned is **kinematic**. It is never integrated. Each step it moves towards the position it was
//! last told the vertex is at, by no more than `proxy_speed_limit * dt`, and its velocity stays zero. Springs and
//! contacts read the proxy's position, never the target. A pointer that jumps a long way therefore reaches the
//! neighbours as a bounded sequence of small moves, and not as one enormous stretch.
//!
//! # Sleep is an observed result
//!
//! After each step the free particles are inspected. The system is quiet for a step only if every free particle is
//! slow, has a small acceleration after the constraints are taken into account, and nothing penetrates by more than a
//! tenth of a unit. After 24 quiet steps in a row with no pin, it sleeps and wants no more frames. With a pin held and
//! its proxy arrived, the same 24 quiet steps let it stop asking for frames while still reporting [`Status::Dragging`].
//! See [`SpringSystem::wants_frames`].
//!
//! A watchdog counts the simulated time since the last release or parameter change in which no pin was held. If it
//! reaches 20 seconds the system is [`Status::Unsettled`], which is never reported as convergence.

mod bounds;
mod error;
mod params;
mod particle;
mod spring;
mod status;
mod system;
mod vec2;

pub use bounds::Bounds;
pub use error::SolverError;
pub use params::Params;
pub use particle::{Particle, PinTarget};
pub use spring::Spring;
pub use status::{Status, StepOutcome};
pub use system::SpringSystem;
#[cfg(test)]
use system::fallback_direction;
pub use vec2::Vec2;

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The fixed step, in seconds.
pub const FIXED_DT: f64 = 1.0 / 120.0;
/// How many quiet steps in a row make the system quiet enough to rest.
pub const QUIET_STEPS: u32 = 24;
/// Simulated seconds without sleep, since the last disturbance, after which the system is [`Status::Unsettled`].
pub const WATCHDOG_SECONDS: f64 = 20.0;
/// The most particles and springs a system accepts. A contact pass compares every pair, so this is a budget.
pub const MAX_PARTICLES: usize = 64;
pub const MAX_SPRINGS: usize = 256;
/// The heaviest weight a spring can have.
pub const MAX_WEIGHT: f64 = 100.0;

/// The largest factor [`SpringSystem::set_tension`] accepts. A spring may be given any rest length from just above
/// nothing up to this many times its natural length.
pub const MAX_TENSION_FACTOR: f64 = 4.0;

const MAX_FREE_SPEED: f64 = 0.4;
const MAX_PROJECTED_ACCELERATION: f64 = 2.0;
const MAX_PENETRATION: f64 = 0.1;
/// How close to a wall or another particle counts as touching it, for working out the projected acceleration.
const TOUCH: f64 = 1e-9;
/// How many times the constraints are applied each step, so that pushing one pair apart can settle against another.
const CONSTRAINT_PASSES: usize = 6;
/// The largest step a caller can ask for. A longer one is clamped, so a stalled frame cannot blow the system up.
const MAX_STEP: f64 = 0.05;

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[cfg(test)]
mod unit_tests;
