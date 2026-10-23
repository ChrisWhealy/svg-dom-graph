//! Why a system could not be built or changed.

use super::{MAX_SPRINGS, MAX_TENSION_FACTOR, MAX_WEIGHT};
use std::fmt;

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
