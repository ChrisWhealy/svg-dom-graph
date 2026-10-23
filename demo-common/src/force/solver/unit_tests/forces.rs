//! Forces and damping: a spring at its length, symmetry, linear stiffness, energy and damping, and a system that
//! updates the same way whatever order it is given.

use super::*;

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
// Zero extension and force symmetry
// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -

#[test]
fn a_spring_at_its_rest_length_has_no_force_whatever_its_weight() {
    for weight in [0.0, 1.0, 10.0, 100.0] {
        let system = pair(weight, 150.0);
        assert_eq!(system.spring_tension(0), Some(0.0));
        assert!(system.forces().iter().all(|f| f.length() == 0.0));
    }
}

#[test]
fn stretching_and_compressing_a_spring_give_opposite_signs_and_equal_and_opposite_end_forces() {
    let mut system = pair(4.0, 150.0);
    system.particles[1].centre = v(500.0 + 180.0, 500.0);
    let stretched = system.spring_tension(0).expect("a spring");
    assert!(stretched > 0.0);
    let forces = system.forces();
    // A stretched spring pulls the ends together.
    assert!(forces[0].x > 0.0 && forces[1].x < 0.0);
    assert_eq!(forces[0] + forces[1], Vec2::ZERO);

    system.particles[1].centre = v(500.0 + 120.0, 500.0);
    let compressed = system.spring_tension(0).expect("a spring");
    assert!(compressed < 0.0);
    let forces = system.forces();
    assert!(forces[0].x < 0.0 && forces[1].x > 0.0);
    assert_eq!(forces[0] + forces[1], Vec2::ZERO);
    // 30 units each way, so the same size.
    assert!((stretched + compressed).abs() < 1e-9);
}

#[test]
fn swapping_the_two_ends_of_a_spring_changes_no_physics() {
    let build = |from: usize, to: usize| {
        SpringSystem::new(
            &[v(300.0, 300.0), v(450.0, 300.0), v(375.0, 430.0)],
            &[20.0; 3],
            &[(from, to, 3.0), (1, 2, 5.0), (2, 0, 2.0)],
            BIG,
            Params::default(),
        )
        .expect("a valid system")
    };
    let (mut forward, mut backward) = (build(0, 1), build(1, 0));
    for system in [&mut forward, &mut backward] {
        system.begin_pin(0, v(300.0, 300.0));
        system.update_pin_target(0, v(340.0, 330.0));
        run(system, 200);
    }
    for i in 0..3 {
        let (a, b) = (forward.centre(i).expect("a centre"), backward.centre(i).expect("a centre"));
        assert!((a - b).length() < 1e-9, "particle {i}: {a:?} and {b:?}");
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
// Linear stiffness
// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -

#[test]
fn at_equal_extension_a_weight_of_ten_gives_ten_times_the_force_of_a_weight_of_one() {
    let (mut light, mut heavy) = (pair(1.0, 150.0), pair(10.0, 150.0));
    for system in [&mut light, &mut heavy] {
        system.particles[1].centre = v(500.0 + 175.0, 500.0);
    }
    let (light_force, heavy_force) = (light.forces()[0].length(), heavy.forces()[0].length());
    assert!((heavy_force / light_force - 10.0).abs() < 1e-9);
    assert!((light.spring_tension(0).expect("a spring") - 25.0 * 25.0).abs() < 1e-9);
}

#[test]
fn stiffness_follows_the_weight_through_the_reference_weight_and_a_weight_change_keeps_rest_lengths() {
    let mut system = triangle(1.0);
    let rest: Vec<f64> = system.springs().iter().map(|s| s.rest_length).collect();
    system.set_weights(&[2.0, 4.0, 8.0]).expect("valid weights");
    let stiffness: Vec<f64> = system.springs().iter().map(|s| s.stiffness).collect();
    assert_eq!(stiffness, vec![50.0, 100.0, 200.0]);
    assert_eq!(system.springs().iter().map(|s| s.rest_length).collect::<Vec<_>>(), rest);
}

#[test]
fn changing_weights_alone_does_not_move_a_system_at_rest_and_does_not_wake_it_for_ever() {
    let mut system = triangle(1.0);
    assert_eq!(system.status(), Status::Sleeping);
    let before: Vec<Vec2> = (0..3).map(|i| system.centre(i).expect("a centre")).collect();
    system.set_weights(&[10.0, 1.0, 5.0]).expect("valid weights");
    // The change is a disturbance, so it is Running until it has been seen to be quiet again.
    assert_eq!(system.status(), Status::Running);
    assert!(system.wants_frames());
    let steps = run_until(&mut system, 100, |s| s.status() == Status::Sleeping);
    assert!(steps >= QUIET_STEPS as usize, "{steps}");
    let after: Vec<Vec2> = (0..3).map(|i| system.centre(i).expect("a centre")).collect();
    assert_eq!(before, after);
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
// Energy and damping
// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -

#[test]
fn free_velocities_decay_and_energy_never_rises_beyond_numerical_slack() {
    let mut system = triangle(3.0);
    system.particles[0].centre = v(160.0, 230.0);
    system.asleep = false;
    let mut last = energy(&system);
    assert!(last > 0.0);
    for step in 0..2400 {
        system.step(FIXED_DT);
        let now = energy(&system);
        // Semi-implicit Euler is not exactly conservative, so a rise is allowed to within a documented slack: one part
        // in a million of the current energy plus a floor.
        assert!(now <= last * (1.0 + 1e-6) + 1e-9, "step {step}: {last} then {now}");
        last = now;
    }
    assert!(last < 0.05, "energy {last} did not decay");
    assert!(system.max_free_speed() < 0.4);
}

#[test]
fn with_no_damping_a_disturbed_spring_keeps_moving_and_never_sleeps() {
    let params = Params {
        damping: 0.0,
        ..Params::default()
    };
    let mut system = SpringSystem::new(&[v(500.0, 500.0), v(650.0, 500.0)], &[20.0; 2], &[(0, 1, 5.0)], BIG, params)
        .expect("a valid system");
    system.particles[1].centre = v(690.0, 500.0);
    system.disturb();
    run(&mut system, 600);
    assert_ne!(system.status(), Status::Sleeping);
    assert!(system.max_free_speed() > 0.4);
}

#[test]
fn a_weight_ten_spring_is_underdamped_at_the_defaults_and_a_weight_one_spring_is_critically_damped() {
    // For two free unit masses the spring sees a reduced mass of one half, so the relative coordinate obeys x'' = -2k x
    // - c x'. The damping ratio is c / (2 sqrt(2k)). Weight 1: k = 25, ratio = 10 / (2 * 7.07) = 0.707. Weight 10:
    // k = 250, ratio = 0.224. Both are under one, so both overshoot, and the heavier one more.
    let overshoot = |weight: f64| {
        let mut system = pair(weight, 150.0);
        system.particles[1].centre = v(650.0 + 30.0, 500.0);
        system.disturb();
        let mut most_compressed: f64 = 0.0;
        for _ in 0..600 {
            system.step(FIXED_DT);
            let tension = system.spring_tension(0).expect("a spring");
            most_compressed = most_compressed.min(tension / system.springs()[0].stiffness);
        }
        -most_compressed
    };
    let (light, heavy) = (overshoot(1.0), overshoot(10.0));
    assert!(heavy > light, "heavy {heavy}, light {light}");
    assert!(heavy > 1.0, "the heavy spring overshoots by {heavy}");
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
// Coincidence and deterministic updates
// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -

#[test]
fn the_fallback_direction_is_a_unit_vector_that_depends_only_on_the_pair_and_flips_with_it() {
    for i in 0..8 {
        for j in 0..8 {
            if i == j {
                continue;
            }
            let forward = fallback_direction(i, j);
            assert!((forward.length() - 1.0).abs() < 1e-12);
            assert_eq!(forward, -fallback_direction(j, i));
            assert_eq!(forward, fallback_direction(i, j));
        }
    }
}

#[test]
fn particles_at_the_same_place_are_pushed_apart_the_same_way_every_time_with_nothing_not_finite() {
    let outcome = || {
        let mut system = triangle(2.0);
        system.particles[1].centre = system.particles[0].centre;
        system.disturb();
        run(&mut system, 120);
        assert!(system.particles.iter().all(|p| p.centre.is_finite() && p.velocity.is_finite()));
        (0..3).map(|i| system.centre(i).expect("a centre")).collect::<Vec<_>>()
    };
    assert_eq!(outcome(), outcome());
}

#[test]
fn the_order_the_edges_are_listed_in_does_not_change_where_the_system_settles() {
    let build = |edges: &[(usize, usize, f64)]| {
        let mut system = SpringSystem::new(
            &[v(200.0, 200.0), v(400.0, 200.0), v(300.0, 380.0)],
            &[20.0; 3],
            edges,
            BIG,
            Params::default(),
        )
        .expect("a valid system");
        system.particles[2].centre = v(330.0, 360.0);
        system.disturb();
        run(&mut system, 1200);
        (0..3).map(|i| system.centre(i).expect("a centre")).collect::<Vec<_>>()
    };
    let a = build(&[(0, 1, 2.0), (1, 2, 5.0), (2, 0, 9.0)]);
    let b = build(&[(2, 0, 9.0), (0, 1, 2.0), (1, 2, 5.0)]);
    for (x, y) in a.iter().zip(&b) {
        assert!((*x - *y).length() < 1e-6, "{x:?} and {y:?}");
    }
}
