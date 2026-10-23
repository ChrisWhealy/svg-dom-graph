//! The small systems and helpers that every group of solver tests builds on.

use super::super::*;

pub(super) const BIG: Bounds = Bounds {
    left: 0.0,
    top: 0.0,
    right: 1000.0,
    bottom: 1000.0,
};

pub(super) fn v(x: f64, y: f64) -> Vec2 {
    Vec2::new(x, y)
}

/// Three circles of radius 20 in a triangle, with a spring of `weight` along each side.
pub(super) fn triangle(weight: f64) -> SpringSystem {
    SpringSystem::new(
        &[v(200.0, 200.0), v(400.0, 200.0), v(300.0, 380.0)],
        &[20.0; 3],
        &[(0, 1, weight), (1, 2, weight), (2, 0, weight)],
        BIG,
        Params::default(),
    )
    .expect("a valid system")
}

/// Two circles joined by one spring of `weight`, `gap` apart, with the first of them at `(500, 500)`.
pub(super) fn pair(weight: f64, length: f64) -> SpringSystem {
    SpringSystem::new(
        &[v(500.0, 500.0), v(500.0 + length, 500.0)],
        &[20.0; 2],
        &[(0, 1, weight)],
        BIG,
        Params::default(),
    )
    .expect("a valid system")
}

/// Total energy: the kinetic energy of the free particles and the energy stored in the springs.
pub(super) fn energy(system: &SpringSystem) -> f64 {
    let kinetic: f64 = system
        .particles
        .iter()
        .filter(|p| p.pin.is_none())
        .map(|p| 0.5 * p.mass * p.velocity.dot(p.velocity))
        .sum();
    let potential: f64 = (0..system.springs.len())
        .map(|i| {
            let spring = system.springs[i];
            let extension = system.spring_tension(i).expect("a spring") / spring.stiffness.max(1e-12);
            0.5 * spring.stiffness * extension * extension
        })
        .sum();
    kinetic + potential
}

pub(super) fn run(system: &mut SpringSystem, steps: usize) {
    for _ in 0..steps {
        system.step(FIXED_DT);
    }
}

/// Steps until `done` holds, or panics after `limit` steps.
pub(super) fn run_until(system: &mut SpringSystem, limit: usize, done: impl Fn(&SpringSystem) -> bool) -> usize {
    for step in 0..limit {
        if done(system) {
            return step;
        }
        system.step(FIXED_DT);
    }
    panic!("not done after {limit} steps; status {:?}", system.status());
}
