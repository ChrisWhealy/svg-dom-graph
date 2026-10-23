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

use std::{
    f64::consts::TAU,
    fmt,
    ops::{Add, AddAssign, Mul, Neg, Sub, SubAssign},
};

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
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

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// A point or a vector in the scene's user space.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Vec2 {
    pub x: f64,
    pub y: f64,
}

impl Vec2 {
    pub const ZERO: Self = Self { x: 0.0, y: 0.0 };

    pub const fn new(x: f64, y: f64) -> Self {
        Self { x, y }
    }

    pub fn length(self) -> f64 {
        self.x.hypot(self.y)
    }

    pub fn dot(self, other: Self) -> f64 {
        self.x * other.x + self.y * other.y
    }

    pub fn is_finite(self) -> bool {
        self.x.is_finite() && self.y.is_finite()
    }
}

impl Add for Vec2 {
    type Output = Self;

    fn add(self, other: Self) -> Self {
        Self::new(self.x + other.x, self.y + other.y)
    }
}

impl Sub for Vec2 {
    type Output = Self;

    fn sub(self, other: Self) -> Self {
        Self::new(self.x - other.x, self.y - other.y)
    }
}

impl Mul<f64> for Vec2 {
    type Output = Self;

    fn mul(self, factor: f64) -> Self {
        Self::new(self.x * factor, self.y * factor)
    }
}

impl Neg for Vec2 {
    type Output = Self;

    fn neg(self) -> Self {
        Self::new(-self.x, -self.y)
    }
}

impl AddAssign for Vec2 {
    fn add_assign(&mut self, other: Self) {
        *self = *self + other;
    }
}

impl SubAssign for Vec2 {
    fn sub_assign(&mut self, other: Self) {
        *self = *self - other;
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The rectangle a particle's whole circle must stay inside: the same rectangle the scene confines a dragged node to.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Bounds {
    pub left: f64,
    pub top: f64,
    pub right: f64,
    pub bottom: f64,
}

impl Bounds {
    /// The bounds of a rectangle with its top-left corner at `(left, top)`.
    pub fn from_origin_and_size(left: f64, top: f64, width: f64, height: f64) -> Self {
        Self {
            left,
            top,
            right: left + width,
            bottom: top + height,
        }
    }

    fn is_valid(&self) -> bool {
        // The width and height are derived, and two finite edges can be further apart than any number can say.
        [self.left, self.top, self.right, self.bottom].iter().all(|v| v.is_finite())
            && self.right > self.left
            && self.bottom > self.top
            && (self.right - self.left).is_finite()
            && (self.bottom - self.top).is_finite()
    }

    /// `centre` moved inside the rectangle that a circle of `radius` can have its centre in.
    pub fn clamp_centre(&self, centre: Vec2, radius: f64) -> Vec2 {
        Vec2::new(
            clamp(centre.x, self.left + radius, self.right - radius),
            clamp(centre.y, self.top + radius, self.bottom - radius),
        )
    }

    fn holds_circle(&self, radius: f64) -> bool {
        self.right - self.left > 2.0 * radius && self.bottom - self.top > 2.0 * radius
    }
}

/// `value` limited to `low..=high`. If `high < low` the result is `low`, so a region too small for a circle still gives
/// a finite answer.
fn clamp(value: f64, low: f64, high: f64) -> f64 {
    value.min(high).max(low)
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
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
    fn stiffness_of(&self, weight: f64) -> f64 {
        self.base_stiffness * weight / self.reference_weight
    }

    fn validate(&self) -> Result<(), SolverError> {
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

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Why a system could not be built or changed.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum SolverError {
    /// There are no particles, too many, or the slices describing them differ in length.
    BadParticleCount,
    /// There are more springs than [`MAX_SPRINGS`].
    TooManySprings,
    /// A centre, radius or bound is not finite, or a radius is negative.
    NonFinite,
    /// A spring joins a particle to itself, which is not supported.
    SelfLoop(usize),
    /// A spring names a particle that is not there.
    UnknownParticle(usize),
    /// A spring's ends are at the same place, so it has no rest length to recover.
    ZeroRestLength(usize),
    /// A weight is not finite, is negative, or is above [`MAX_WEIGHT`].
    InvalidWeight(usize),
    /// A parameter is not finite or is outside its range.
    InvalidParams,
    /// The bounds have no area, or a circle cannot fit inside them.
    InvalidBounds,
    /// A particle starts outside the bounds it must stay inside.
    OutsideBounds(usize),
    /// The number of weights given is not the number of springs.
    WrongWeightCount,
    /// A tension factor is not finite, is not above zero, or is above [`MAX_TENSION_FACTOR`].
    InvalidTension(usize),
    /// The number of tension factors given is not the number of springs.
    WrongTensionCount,
    /// A number worked out for a spring, its rest length or its stiffness, is not finite even though everything it was
    /// worked out from is. The positions are too far apart, or the values too large, to be told apart.
    NonFiniteSpring(usize),
}

impl fmt::Display for SolverError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SolverError::BadParticleCount => write!(f, "the particles are missing, too many, or do not match"),
            SolverError::TooManySprings => write!(f, "there are more than {MAX_SPRINGS} springs"),
            SolverError::NonFinite => write!(f, "a position, radius or bound is not a finite number"),
            SolverError::SelfLoop(spring) => write!(f, "spring {spring} joins a particle to itself"),
            SolverError::UnknownParticle(spring) => write!(f, "spring {spring} names a particle that is not there"),
            SolverError::ZeroRestLength(spring) => write!(f, "spring {spring} has both ends at the same place"),
            SolverError::InvalidWeight(spring) => {
                write!(f, "the weight of spring {spring} is not between 0 and {MAX_WEIGHT}")
            },
            SolverError::InvalidParams => write!(f, "a physics parameter is not finite or is out of range"),
            SolverError::InvalidBounds => write!(f, "the bounds have no area, or a circle cannot fit inside them"),
            SolverError::OutsideBounds(particle) => write!(f, "particle {particle} starts outside the bounds"),
            SolverError::WrongWeightCount => write!(f, "the number of weights is not the number of springs"),
            SolverError::InvalidTension(spring) => {
                write!(
                    f,
                    "the tension of spring {spring} is not above 0 and at most {MAX_TENSION_FACTOR}"
                )
            },
            SolverError::WrongTensionCount => write!(f, "the number of tensions is not the number of springs"),
            SolverError::NonFiniteSpring(spring) => {
                write!(
                    f,
                    "the length or stiffness of spring {spring} is too large to be a finite number"
                )
            },
        }
    }
}

impl std::error::Error for SolverError {}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Where a pinned particle is being pulled to, and whether it lets go on arriving.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PinTarget {
    /// The position of the vertex as the scene last committed it. The proxy moves towards this.
    pub committed_centre: Vec2,
    /// Whether the pin is released when the proxy arrives. Set when the pointer is released.
    pub release_on_arrival: bool,
}

/// One circle.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Particle {
    pub centre: Vec2,
    pub velocity: Vec2,
    pub mass: f64,
    pub radius: f64,
    /// `Some` while the particle is kinematic: held by a pointer, or still catching up after one.
    pub pin: Option<PinTarget>,
}

/// One spring, joining the particles at indices `from` and `to`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Spring {
    pub from: usize,
    pub to: usize,
    pub weight: f64,
    pub stiffness: f64,
    pub rest_length: f64,
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// What the system is doing, as the panel reports it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Status {
    /// At rest, and asking for no frames.
    Sleeping,
    /// Free particles are moving after a disturbance.
    Running,
    /// A pointer holds a vertex, or its proxy is still catching up after the release.
    Dragging,
    /// Stopped by the user or because the page is hidden.
    Paused,
    /// The watchdog ran out without the system coming to rest. Never to be reported as convergence.
    Unsettled,
}

impl Status {
    /// The word the panel writes for this state.
    pub fn as_str(self) -> &'static str {
        match self {
            Status::Sleeping => "sleeping",
            Status::Running => "running",
            Status::Dragging => "dragging",
            Status::Paused => "paused",
            Status::Unsettled => "unsettled",
        }
    }
}

/// What one call to [`SpringSystem::step`] did.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct StepOutcome {
    /// Whether the system took a step. It does not when it is asleep, paused, unsettled or faulted.
    pub stepped: bool,
    /// How many quiet steps in a row there have now been.
    pub quiet_steps: u32,
    /// Whether this step put the system to sleep.
    pub slept: bool,
    /// Whether this step ran the watchdog out.
    pub unsettled: bool,
    /// Whether the step produced a number that is not finite. The state was put back as it was.
    pub faulted: bool,
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// A direction from particle `i` to particle `j` when their centres coincide, so there is none to divide by. It depends
/// only on the two indices, so the same input always gives the same answer, and `(i, j)` and `(j, i)` are opposite.
fn fallback_direction(i: usize, j: usize) -> Vec2 {
    let (low, high) = (i.min(j), i.max(j));
    let angle = TAU * ((low * 7 + high * 13) % 16) as f64 / 16.0;
    let direction = Vec2::new(angle.cos(), angle.sin());
    if i <= j { direction } else { -direction }
}

/// The unit vector of `delta`, which is `length` long, or the fallback for particles `i` and `j`.
fn unit_or_fallback(delta: Vec2, length: f64, i: usize, j: usize) -> Vec2 {
    if length > 1e-12 { delta * (1.0 / length) } else { fallback_direction(i, j) }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// A damped spring graph.
#[derive(Clone, Debug)]
pub struct SpringSystem {
    particles: Vec<Particle>,
    initial: Vec<Vec2>,
    /// Each spring's rest length when the system was built. A tension is always a multiple of this, never of the last
    /// tension, so setting one again does not compound.
    natural_lengths: Vec<f64>,
    springs: Vec<Spring>,
    params: Params,
    bounds: Bounds,
    quiet_streak: u32,
    asleep: bool,
    paused: bool,
    unsettled: bool,
    faulted: bool,
    /// Set when a pin is told its target again without it changing. The next step is then allowed to be one of zero
    /// motion, which does not need a fresh run of quiet steps.
    rearmed: bool,
    active_time: f64,
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

    fn idle_held(&self) -> bool {
        let pinned = self.particles.iter().any(|p| p.pin.is_some());
        pinned && !self.any_draining() && self.proxies_arrived() && self.quiet_streak >= QUIET_STEPS && !self.rearmed
    }

    fn proxies_arrived(&self) -> bool {
        self.particles
            .iter()
            .all(|p| p.pin.is_none_or(|pin| pin.committed_centre == p.centre))
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    /// Notes that something changed that the system must settle again from: the quiet run, the time budget and any
    /// sleep or unsettled verdict are all begun afresh.
    fn disturb(&mut self) {
        self.asleep = false;
        self.unsettled = false;
        self.rearmed = false;
        self.quiet_streak = 0;
        self.active_time = 0.0;
    }

    /// Pins particle `index` for a held vertex whose circle is now at `actual`.
    ///
    /// A particle that is free is pinned where `actual` is, with no velocity. One that is already pinned, as when a new
    /// press comes while the proxy is still catching up, keeps its proxy where it is and only has its target changed,
    /// so nothing jumps. Either way a pending release is cancelled.
    ///
    /// Returns `false`, and changes nothing, for an unknown particle or a position that is not finite.
    pub fn begin_pin(&mut self, index: usize, actual: Vec2) -> bool {
        if !actual.is_finite() || index >= self.particles.len() {
            return false;
        }
        let target = self.bounds.clamp_centre(actual, self.particles[index].radius);
        let particle = &mut self.particles[index];
        if particle.pin.is_none() {
            particle.centre = target;
        }
        particle.velocity = Vec2::ZERO;
        particle.pin = Some(PinTarget {
            committed_centre: target,
            release_on_arrival: false,
        });
        self.disturb();
        true
    }

    /// Tells a held particle that its vertex is now at `actual`, which is where the scene committed it.
    ///
    /// If that is the target it already has, the quiet run is kept and the system is asked for one frame to check, and
    /// a target that has changed begins a new run. A particle that is not held is left alone.
    ///
    /// Returns whether a held pin was updated.
    pub fn update_pin_target(&mut self, index: usize, actual: Vec2) -> bool {
        if !actual.is_finite() || index >= self.particles.len() {
            return false;
        }
        let radius = self.particles[index].radius;
        let target = self.bounds.clamp_centre(actual, radius);
        let Some(pin) = self.particles[index].pin.as_mut() else {
            return false;
        };
        if pin.release_on_arrival {
            return false;
        }
        if pin.committed_centre == target {
            self.rearmed = true;
        } else {
            pin.committed_centre = target;
            self.quiet_streak = 0;
            self.asleep = false;
            self.rearmed = false;
        }
        true
    }

    /// The pointer was released and the vertex is at `actual`. The proxy keeps its pin while it catches up, and lets go
    /// when it arrives. Returns whether a pin was there.
    pub fn end_pin(&mut self, index: usize, actual: Vec2) -> bool {
        self.release_pin(index, actual)
    }

    /// The drag was cancelled and the last committed position of the vertex is `actual`. It behaves like
    /// [`end_pin`](Self::end_pin): the proxy catches up to that position and then lets go, and no position newer than
    /// it is used.
    pub fn cancel_pin(&mut self, index: usize, actual: Vec2) -> bool {
        self.release_pin(index, actual)
    }

    fn release_pin(&mut self, index: usize, actual: Vec2) -> bool {
        if !actual.is_finite() || index >= self.particles.len() || self.particles[index].pin.is_none() {
            return false;
        }
        let target = self.bounds.clamp_centre(actual, self.particles[index].radius);
        self.particles[index].pin = Some(PinTarget {
            committed_centre: target,
            release_on_arrival: true,
        });
        self.disturb();
        true
    }

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
    fn rest_if_quiet(&mut self) {
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

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    /// The force on each particle: the springs, and a push apart for any two circles that overlap.
    fn forces(&self) -> Vec<Vec2> {
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
    fn accelerations(&self, forces: &[Vec2]) -> Vec<Vec2> {
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
    fn projected(&self, index: usize, acceleration: Vec2) -> Vec2 {
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
    fn is_quiet_now(&self) -> bool {
        self.max_free_speed() <= MAX_FREE_SPEED
            && self.max_projected_acceleration() <= MAX_PROJECTED_ACCELERATION
            && self.max_penetration() <= MAX_PENETRATION
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    /// Advances every pinned proxy towards its target by at most `proxy_speed_limit * dt`, and lets go of any that has
    /// arrived and was to be released. A proxy that arrives is put exactly on its target, so arrival is exact.
    ///
    /// Returns which particles were let go, as a flag for each. A particle let go this step is not integrated in it: it
    /// is free from the next step, and starts from rest exactly where its proxy arrived, with no sudden change.
    fn advance_proxies(&mut self, dt: f64) -> Vec<bool> {
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
    fn constrain(&mut self) {
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
    fn remove_inward_velocity(&mut self) {
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

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[cfg(test)]
mod unit_tests;
