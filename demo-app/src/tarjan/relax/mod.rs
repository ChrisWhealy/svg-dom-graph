//! The condensation as springs, once the search is over. Pure: nothing here touches the page.
//!
//! When the last step is shown, every arrow of the condensation becomes a spring and its nodes are let go, so that they
//! settle into a new arrangement. There is no value in the calculation that a spring's tension could follow: the
//! algorithm gives each arrow nothing but its two ends, and every arrow here comes from one bridge. So every spring has
//! the same weight and the same rest length.

use demo_common::force::solver::{Bounds, MAX_TENSION_FACTOR, Params, SpringSystem, Vec2};

/// The radius of the circle that stands for a node's box, which is large enough to keep two boxes apart.
pub(crate) const RADIUS: f64 = 54.0;

/// How long every spring is when it is at rest, in scene units, and its weight.
pub(crate) const REST: f64 = 190.0;
pub(crate) const WEIGHT: f64 = 1.0;

/// How long the springs are left to move, in seconds of the physics, if they have not come to rest before.
pub(crate) const RUN_SECONDS: f64 = 6.0;

/// The system of springs for nodes whose centres are `centres`, with one spring for each of `arcs`, which are pairs of
/// C-IDs counted from 1. Every spring has the same tension. The nodes cannot leave `bounds`.
///
/// # Errors
///
/// Returns `Err` if the solver refuses what it is given.
pub(crate) fn build(centres: &[(f64, f64)], arcs: &[(usize, usize)], bounds: Bounds) -> Result<SpringSystem, String> {
    let at: Vec<Vec2> = centres.iter().map(|&(x, y)| Vec2::new(x, y)).collect();
    let springs: Vec<(usize, usize, f64)> = arcs.iter().map(|&(a, b)| (a - 1, b - 1, WEIGHT)).collect();
    let mut system = SpringSystem::new(&at, &vec![RADIUS; at.len()], &springs, bounds, Params::default())
        .map_err(|e| format!("could not make the springs: {e:?}"))?;
    let factors: Vec<f64> = system
        .springs()
        .iter()
        .map(|spring| (REST / spring.rest_length).clamp(0.05, MAX_TENSION_FACTOR))
        .collect();
    system
        .set_tension(&factors)
        .map_err(|e| format!("could not set the tension: {e:?}"))?;
    Ok(system)
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[cfg(test)]
mod unit_tests;
