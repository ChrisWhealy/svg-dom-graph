//! Everything the solver refuses to build or change, and says why, plus resyncing to the scene's centres.

use super::*;

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
// Validation
// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -

fn build(centres: &[Vec2], radii: &[f64], edges: &[(usize, usize, f64)]) -> Result<SpringSystem, SolverError> {
    SpringSystem::new(centres, radii, edges, BIG, Params::default())
}

#[test]
fn a_system_that_cannot_be_built_says_why() {
    let c = [v(100.0, 100.0), v(300.0, 100.0)];
    assert_eq!(build(&[], &[], &[]).err(), Some(SolverError::BadParticleCount));
    assert_eq!(build(&c, &[20.0], &[]).err(), Some(SolverError::BadParticleCount));
    assert_eq!(build(&c, &[20.0; 2], &[(0, 0, 1.0)]).err(), Some(SolverError::SelfLoop(0)));
    assert_eq!(
        build(&c, &[20.0; 2], &[(0, 5, 1.0)]).err(),
        Some(SolverError::UnknownParticle(0))
    );
    assert_eq!(
        build(&c, &[20.0; 2], &[(0, 1, -1.0)]).err(),
        Some(SolverError::InvalidWeight(0))
    );
    assert_eq!(
        build(&c, &[20.0; 2], &[(0, 1, f64::NAN)]).err(),
        Some(SolverError::InvalidWeight(0))
    );
    assert_eq!(
        build(&c, &[20.0; 2], &[(0, 1, MAX_WEIGHT + 1.0)]).err(),
        Some(SolverError::InvalidWeight(0))
    );
    assert_eq!(
        build(&[v(100.0, 100.0), v(100.0, 100.0)], &[20.0; 2], &[(0, 1, 1.0)]).err(),
        Some(SolverError::ZeroRestLength(0))
    );
    assert_eq!(
        build(&[v(f64::NAN, 1.0), v(300.0, 100.0)], &[20.0; 2], &[]).err(),
        Some(SolverError::NonFinite)
    );
    assert_eq!(build(&c, &[-1.0, 20.0], &[]).err(), Some(SolverError::NonFinite));
    assert_eq!(
        build(&[v(5.0, 100.0), v(300.0, 100.0)], &[20.0; 2], &[]).err(),
        Some(SolverError::OutsideBounds(0))
    );
}

#[test]
fn bad_parameters_and_bounds_are_refused() {
    let c = [v(100.0, 100.0), v(300.0, 100.0)];
    for params in [
        Params { mass: 0.0, ..Params::default() },
        Params {
            damping: -1.0,
            ..Params::default()
        },
        Params {
            base_stiffness: f64::NAN,
            ..Params::default()
        },
        Params {
            reference_weight: 0.0,
            ..Params::default()
        },
        Params {
            collision_gap: -1.0,
            ..Params::default()
        },
        Params {
            proxy_speed_limit: 0.0,
            ..Params::default()
        },
        Params {
            contact_stiffness: f64::INFINITY,
            ..Params::default()
        },
    ] {
        assert_eq!(
            SpringSystem::new(&c, &[20.0; 2], &[], BIG, params).err(),
            Some(SolverError::InvalidParams),
            "{params:?}"
        );
    }
    let small = Bounds::from_origin_and_size(0.0, 0.0, 30.0, 30.0);
    assert_eq!(
        SpringSystem::new(&[v(15.0, 15.0)], &[20.0], &[], small, Params::default()).err(),
        Some(SolverError::InvalidBounds)
    );
    let flat = Bounds::from_origin_and_size(0.0, 0.0, 100.0, 0.0);
    assert_eq!(
        SpringSystem::new(&c, &[20.0; 2], &[], flat, Params::default()).err(),
        Some(SolverError::InvalidBounds)
    );
}

#[test]
fn too_many_particles_or_springs_are_refused() {
    let many: Vec<Vec2> = (0..=MAX_PARTICLES).map(|i| v(10.0 + 15.0 * i as f64, 10.0)).collect();
    let wide = Bounds::from_origin_and_size(0.0, 0.0, 5000.0, 100.0);
    assert_eq!(
        SpringSystem::new(&many, &vec![2.0; many.len()], &[], wide, Params::default()).err(),
        Some(SolverError::BadParticleCount)
    );
    let two = [v(100.0, 100.0), v(300.0, 100.0)];
    let springs = vec![(0, 1, 1.0); MAX_SPRINGS + 1];
    assert_eq!(build(&two, &[20.0; 2], &springs).err(), Some(SolverError::TooManySprings));
}

#[test]
fn a_tension_stretches_the_springs_from_their_natural_length_and_wakes_the_system() {
    let mut system = triangle(3.0);
    let natural: Vec<f64> = system.springs().iter().map(|s| s.rest_length).collect();
    assert_eq!(system.status(), Status::Sleeping);
    system.set_tension(&[0.5, 1.0, 2.0]).expect("valid factors");
    assert_ne!(system.status(), Status::Sleeping);
    assert!(system.wants_frames());
    let now: Vec<f64> = system.springs().iter().map(|s| s.rest_length).collect();
    assert_eq!(now, vec![natural[0] * 0.5, natural[1], natural[2] * 2.0]);
    // Counted from the natural length each time, so asking again does not compound.
    system.set_tension(&[0.5, 1.0, 2.0]).expect("valid factors");
    let again: Vec<f64> = system.springs().iter().map(|s| s.rest_length).collect();
    assert_eq!(again, now);
    // The circles have not been moved by it, and settle once the steps are run.
    assert_eq!(system.centre(0), Some(v(200.0, 200.0)));
    for _ in 0..20_000 {
        if !system.wants_frames() {
            break;
        }
        system.step(FIXED_DT);
    }
    assert_eq!(system.status(), Status::Sleeping);
    assert_ne!(system.centre(0), Some(v(200.0, 200.0)));
}

#[test]
fn bad_tensions_are_refused_and_change_nothing() {
    let mut system = triangle(3.0);
    let before: Vec<f64> = system.springs().iter().map(|s| s.rest_length).collect();
    assert_eq!(system.set_tension(&[1.0, 1.0]), Err(SolverError::WrongTensionCount));
    assert_eq!(system.set_tension(&[1.0, f64::NAN, 1.0]), Err(SolverError::InvalidTension(1)));
    assert_eq!(system.set_tension(&[1.0, 1.0, 0.0]), Err(SolverError::InvalidTension(2)));
    assert_eq!(system.set_tension(&[-1.0, 1.0, 1.0]), Err(SolverError::InvalidTension(0)));
    assert_eq!(
        system.set_tension(&[1.0, MAX_TENSION_FACTOR + 0.1, 1.0]),
        Err(SolverError::InvalidTension(1))
    );
    assert_eq!(system.status(), Status::Sleeping);
    assert_eq!(system.springs().iter().map(|s| s.rest_length).collect::<Vec<_>>(), before);
}

#[test]
fn bad_weights_and_dampings_are_refused_and_change_nothing() {
    let mut system = triangle(3.0);
    assert_eq!(system.set_weights(&[1.0, 2.0]), Err(SolverError::WrongWeightCount));
    assert_eq!(system.set_weights(&[1.0, f64::NAN, 2.0]), Err(SolverError::InvalidWeight(1)));
    assert_eq!(system.set_weights(&[1.0, 2.0, -3.0]), Err(SolverError::InvalidWeight(2)));
    assert_eq!(system.set_damping(-1.0), Err(SolverError::InvalidParams));
    assert_eq!(system.status(), Status::Sleeping);
    assert!(system.springs().iter().all(|s| s.weight == 3.0));
}

#[test]
fn the_errors_say_what_is_wrong_in_words() {
    assert_eq!(SolverError::SelfLoop(2).to_string(), "spring 2 joins a particle to itself");
    assert!(SolverError::OutsideBounds(4).to_string().contains("particle 4"));
    assert_eq!(Status::Unsettled.as_str(), "unsettled");
    assert_eq!(
        [Status::Sleeping, Status::Running, Status::Dragging, Status::Paused].map(Status::as_str),
        ["sleeping", "running", "dragging", "paused"]
    );
}

#[test]
fn resyncing_takes_the_scenes_centres_clears_pins_and_velocities_and_keeps_the_springs() {
    let mut system = triangle(3.0);
    system.begin_pin(0, v(200.0, 200.0));
    system.update_pin_target(0, v(260.0, 200.0));
    run(&mut system, 3);
    let rest: Vec<f64> = system.springs().iter().map(|s| s.rest_length).collect();
    assert!(system.resync_centres(&[v(210.0, 205.0), v(405.0, 200.0), v(300.0, 380.0)]));
    assert!(!system.any_held() && !system.any_draining());
    assert_eq!(system.centre(0), Some(v(210.0, 205.0)));
    assert!(system.particles.iter().all(|p| p.velocity == Vec2::ZERO));
    assert_eq!(system.springs().iter().map(|s| s.rest_length).collect::<Vec<_>>(), rest);
    assert_eq!(system.status(), Status::Running);
    // Wrong count or a centre that is not finite changes nothing.
    assert!(!system.resync_centres(&[v(1.0, 1.0)]));
    assert!(!system.resync_centres(&[v(f64::NAN, 1.0), v(1.0, 1.0), v(1.0, 1.0)]));
    assert_eq!(system.centre(0), Some(v(210.0, 205.0)));
}
