use super::*;

const BIG: Bounds = Bounds {
    left: 0.0,
    top: 0.0,
    right: 1000.0,
    bottom: 1000.0,
};

fn v(x: f64, y: f64) -> Vec2 {
    Vec2::new(x, y)
}

/// Three circles of radius 20 in a triangle, with a spring of `weight` along each side.
fn triangle(weight: f64) -> SpringSystem {
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
fn pair(weight: f64, length: f64) -> SpringSystem {
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
fn energy(system: &SpringSystem) -> f64 {
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

fn run(system: &mut SpringSystem, steps: usize) {
    for _ in 0..steps {
        system.step(FIXED_DT);
    }
}

/// Steps until `done` holds, or panics after `limit` steps.
fn run_until(system: &mut SpringSystem, limit: usize, done: impl Fn(&SpringSystem) -> bool) -> usize {
    for step in 0..limit {
        if done(system) {
            return step;
        }
        system.step(FIXED_DT);
    }
    panic!("not done after {limit} steps; status {:?}", system.status());
}

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

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
// Bounds, contact and radius
// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -

#[test]
fn every_free_centre_is_held_inside_the_bounds_inset_by_its_radius() {
    let mut system = pair(1.0, 150.0);
    system.particles[0].centre = v(-500.0, 5000.0);
    system.particles[1].centre = v(9000.0, -20.0);
    system.disturb();
    for _ in 0..50 {
        system.step(FIXED_DT);
        for particle in &system.particles {
            assert!(particle.centre.x >= 20.0 && particle.centre.x <= 980.0, "{:?}", particle.centre);
            assert!(particle.centre.y >= 20.0 && particle.centre.y <= 980.0, "{:?}", particle.centre);
        }
    }
}

#[test]
fn a_circle_against_a_wall_has_no_velocity_into_it_and_a_zero_projected_acceleration_into_it() {
    let mut system = pair(1.0, 150.0);
    system.particles[0].centre = v(20.0, 500.0);
    system.particles[1].centre = v(170.0, 500.0);
    system.particles[0].velocity = v(-50.0, 0.0);
    system.disturb();
    system.step(FIXED_DT);
    assert!(system.particles[0].velocity.x >= 0.0);
    assert_eq!(system.centre(0), Some(v(20.0, 500.0)));
    // An acceleration into the wall is taken away, and one along it is kept.
    let projected = system.projected(0, v(-10.0, 3.0));
    assert_eq!((projected.x, projected.y), (0.0, 3.0));
}

#[test]
fn overlapping_circles_are_pushed_apart_to_the_collision_gap_and_penetration_resolves_to_tolerance() {
    let mut system = SpringSystem::new(
        &[v(300.0, 300.0), v(380.0, 300.0), v(600.0, 600.0)],
        &[20.0; 3],
        &[(0, 2, 1.0)],
        BIG,
        Params::default(),
    )
    .expect("a valid system");
    // Pushed into one another, closer than their radii and the gap of 6.
    system.particles[1].centre = v(310.0, 300.0);
    system.disturb();
    assert!(system.max_penetration() > 30.0);
    run(&mut system, 10);
    assert!(system.max_penetration() <= 0.1, "{}", system.max_penetration());
}

#[test]
fn a_pinned_proxy_never_yields_to_contact_and_the_free_circle_gives_way_instead() {
    let mut system = SpringSystem::new(&[v(300.0, 300.0), v(360.0, 300.0)], &[20.0; 2], &[], BIG, Params::default())
        .expect("a valid system");
    // 60 apart is more than 20 + 20 + 6, so they start clear. The held one is dragged towards the free one.
    system.begin_pin(0, v(300.0, 300.0));
    system.update_pin_target(0, v(340.0, 300.0));
    run(&mut system, 40);
    assert_eq!(system.centre(0), Some(v(340.0, 300.0)), "the proxy was pushed back");
    let gap = system.centre(1).expect("a centre").x - 340.0;
    assert!(gap >= 46.0 - 0.1, "the free circle is {gap} away");
}

#[test]
fn a_far_away_committed_target_is_not_a_second_collision_body() {
    let mut system = SpringSystem::new(&[v(300.0, 300.0), v(700.0, 300.0)], &[20.0; 2], &[], BIG, Params::default())
        .expect("a valid system");
    // The held vertex is sent 400 units away, right on top of the free one, but its proxy is still where it was.
    system.begin_pin(0, v(300.0, 300.0));
    system.update_pin_target(0, v(700.0, 300.0));
    system.step(FIXED_DT);
    // One step later the proxy has moved six units, and the free circle has not been pushed by the distant target.
    assert_eq!(system.centre(0), Some(v(306.0, 300.0)));
    assert_eq!(system.centre(1), Some(v(700.0, 300.0)));
    assert_eq!(system.max_penetration(), 0.0);
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
// Pinned proxy pursuit
// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -

#[test]
fn a_target_a_hundred_units_away_is_reached_by_steps_of_no_more_than_six_units() {
    let mut system = triangle(10.0);
    system.begin_pin(0, v(200.0, 200.0));
    system.update_pin_target(0, v(300.0, 200.0));
    let mut steps = 0;
    let mut longest: f64 = 0.0;
    loop {
        let before = system.centre(0).expect("a centre");
        system.step(FIXED_DT);
        let after = system.centre(0).expect("a centre");
        longest = longest.max((after - before).length());
        steps += 1;
        if after == v(300.0, 200.0) {
            break;
        }
        assert!(steps < 100);
    }
    // 100 / 6 = 16.7, so seventeen steps, none longer than the limit of 720 * (1/120) = 6.
    assert_eq!(steps, 17);
    assert!(longest <= 6.0 + 1e-9, "{longest}");
    // The proxy has no velocity, so no momentum is handed to its neighbours by the move.
    assert_eq!(system.particle(0).expect("a particle").velocity, Vec2::ZERO);
}

#[test]
fn a_neighbour_never_sees_a_single_step_spring_extension_of_the_whole_jump() {
    let mut system = triangle(10.0);
    system.begin_pin(0, v(200.0, 200.0));
    system.update_pin_target(0, v(300.0, 200.0));
    let stiffness = system.springs()[0].stiffness;
    let mut worst_change: f64 = 0.0;
    let mut last = system.spring_tension(0).expect("a spring") / stiffness;
    for _ in 0..30 {
        system.step(FIXED_DT);
        let now = system.spring_tension(0).expect("a spring") / stiffness;
        worst_change = worst_change.max((now - last).abs());
        last = now;
    }
    // The extension changes by the proxy's six units a step at most, plus a little for the neighbour moving. A single
    // snap to the target would change it by the whole 100.
    assert!(worst_change <= 7.0, "{worst_change}");
}

#[test]
fn a_pin_is_put_inside_the_bounds_and_cannot_be_dragged_out_of_them() {
    let mut system = triangle(1.0);
    system.begin_pin(0, v(200.0, 200.0));
    system.update_pin_target(0, v(-5000.0, 99_999.0));
    run(&mut system, 1000);
    let centre = system.centre(0).expect("a centre");
    assert_eq!(centre, v(20.0, 980.0));
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
// End, Cancel, second Start
// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -

#[test]
fn a_release_never_teleports_the_proxy_and_lets_go_only_on_arrival() {
    let mut system = triangle(2.0);
    system.begin_pin(0, v(200.0, 200.0));
    system.update_pin_target(0, v(320.0, 200.0));
    system.end_pin(0, v(320.0, 200.0));
    assert!(system.any_draining() && !system.any_held());
    assert_eq!(system.status(), Status::Dragging);
    let mut last = system.centre(0).expect("a centre");
    let mut released_at = None;
    for step in 0..40 {
        system.step(FIXED_DT);
        let now = system.centre(0).expect("a centre");
        assert!((now - last).length() <= 6.0 + 1e-9, "step {step} moved {:?}", now - last);
        last = now;
        if !system.is_pinned(0) {
            released_at = Some(step);
            break;
        }
    }
    // 120 units at six a step is twenty steps, and the last of them is the arrival, so the pin goes then.
    assert_eq!(released_at, Some(19));
    // It arrived exactly on its target, and the step that let go did not integrate it, so it is free from rest right
    // there.
    assert_eq!(last, v(320.0, 200.0));
    // After letting go the particle is free, with no velocity.
    assert_eq!(system.particle(0).expect("a particle").velocity, Vec2::ZERO);
    assert_eq!(system.status(), Status::Running);
}

#[test]
fn cancel_uses_the_last_committed_position_and_ignores_any_newer_uncommitted_one() {
    let mut system = triangle(2.0);
    system.begin_pin(0, v(200.0, 200.0));
    system.update_pin_target(0, v(230.0, 200.0));
    // The pointer was at 400, but that was never committed. The scene says the circle is at 230.
    system.cancel_pin(0, v(230.0, 200.0));
    let pin = system.particle(0).and_then(|p| p.pin).expect("still pinned while it drains");
    assert_eq!(pin.committed_centre, v(230.0, 200.0));
    assert!(pin.release_on_arrival);
    run(&mut system, 10);
    assert!(!system.is_pinned(0));
    assert!((system.centre(0).expect("a centre") - v(230.0, 200.0)).length() < 5.0);
}

#[test]
fn a_second_start_during_the_drain_clears_the_pending_release_without_a_jump() {
    let mut system = triangle(2.0);
    system.begin_pin(0, v(200.0, 200.0));
    system.update_pin_target(0, v(320.0, 200.0));
    system.end_pin(0, v(320.0, 200.0));
    run(&mut system, 5);
    let proxy = system.centre(0).expect("a centre");
    assert!(proxy.x < 320.0 && system.any_draining());
    // The pointer comes down again on the circle, which the scene has at 320.
    assert!(system.begin_pin(0, v(320.0, 200.0)));
    assert!(system.any_held() && !system.any_draining());
    assert_eq!(system.centre(0), Some(proxy), "the proxy jumped");
    // It carries on following the new commits and is never released by the old drain.
    run(&mut system, 40);
    assert!(system.is_pinned(0));
    assert_eq!(system.centre(0), Some(v(320.0, 200.0)));
}

#[test]
fn several_particles_can_be_pinned_at_once_each_with_its_own_proxy() {
    let mut system = triangle(2.0);
    system.begin_pin(0, v(200.0, 200.0));
    system.begin_pin(1, v(400.0, 200.0));
    system.update_pin_target(0, v(180.0, 220.0));
    system.update_pin_target(1, v(420.0, 190.0));
    system.end_pin(1, v(420.0, 190.0));
    assert!(system.any_held() && system.any_draining());
    run(&mut system, 20);
    assert!(system.is_pinned(0) && !system.is_pinned(1));
    assert_eq!(system.centre(0), Some(v(180.0, 220.0)));
    // Released on arrival, so free, and still near where its proxy arrived. The springs have had a few steps since.
    assert!((system.centre(1).expect("a centre") - v(420.0, 190.0)).length() < 40.0);
}

#[test]
fn a_pin_for_an_unknown_particle_or_a_position_that_is_not_finite_changes_nothing() {
    let mut system = triangle(2.0);
    assert!(!system.begin_pin(9, v(1.0, 1.0)));
    assert!(!system.begin_pin(0, v(f64::NAN, 1.0)));
    assert!(!system.update_pin_target(0, v(1.0, 1.0)), "particle 0 is not held");
    assert!(!system.end_pin(0, v(1.0, 1.0)));
    assert_eq!(system.status(), Status::Sleeping);
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
// Held-state quiescence and wake-up
// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -

/// A held triangle whose proxy has arrived and whose neighbours have gone quiet: idle while held.
fn held_and_idle() -> SpringSystem {
    let mut system = triangle(2.0);
    system.begin_pin(0, v(200.0, 200.0));
    system.update_pin_target(0, v(240.0, 200.0));
    run_until(&mut system, 4000, |s| !s.wants_frames());
    system
}

#[test]
fn a_held_node_that_has_arrived_and_gone_quiet_stops_asking_for_frames_but_is_never_sleeping() {
    let system = held_and_idle();
    assert_eq!(system.status(), Status::Dragging);
    assert!(!system.wants_frames());
    assert!(system.quiet_streak() >= QUIET_STEPS);
    assert!(system.any_held());
}

#[test]
fn a_held_node_does_not_go_idle_before_its_proxy_has_arrived_or_its_neighbours_are_quiet() {
    let mut system = triangle(2.0);
    system.begin_pin(0, v(200.0, 200.0));
    system.update_pin_target(0, v(320.0, 200.0));
    // The proxy is still on its way, so there is always a reason to ask for the next frame.
    for _ in 0..15 {
        system.step(FIXED_DT);
        assert!(system.wants_frames());
    }
    // And it needs 24 quiet steps after arriving, not one.
    run_until(&mut system, 4000, |s| s.proxies_arrived());
    assert!(system.wants_frames());
}

#[test]
fn a_changed_target_wakes_a_quiescent_held_node_and_starts_a_new_run_of_quiet_steps() {
    let mut system = held_and_idle();
    assert!(system.update_pin_target(0, v(260.0, 200.0)));
    assert!(system.wants_frames());
    assert_eq!(system.quiet_streak(), 0);
    let steps = run_until(&mut system, 4000, |s| !s.wants_frames());
    assert!(steps >= QUIET_STEPS as usize, "{steps}");
    assert_eq!(system.centre(0), Some(v(260.0, 200.0)));
}

#[test]
fn the_same_target_again_asks_for_one_frame_and_keeps_the_quiet_run_so_it_goes_idle_at_once() {
    let mut system = held_and_idle();
    let streak = system.quiet_streak();
    assert!(system.update_pin_target(0, v(240.0, 200.0)));
    // It asks for a frame, even though nothing changed.
    assert!(system.wants_frames());
    assert_eq!(system.quiet_streak(), streak, "the streak was reset");
    system.step(FIXED_DT);
    // One zero-motion step later it is idle again, without earning a second 24.
    assert!(!system.wants_frames());
    assert!(system.quiet_streak() >= QUIET_STEPS);
    assert_eq!(system.status(), Status::Dragging);
}

#[test]
fn a_release_wakes_an_idle_held_node_and_begins_a_fresh_released_quiet_window_after_the_drain() {
    let mut system = held_and_idle();
    system.end_pin(0, v(240.0, 200.0));
    assert!(system.wants_frames());
    assert_eq!(system.quiet_streak(), 0);
    // The proxy is already there, so the first step lets go. From then 24 quiet steps are needed again.
    system.step(FIXED_DT);
    assert!(!system.is_pinned(0));
    // The window begins when the proxy lets go: at most this one step of it has been counted.
    assert!(system.quiet_streak() <= 1);
    assert_eq!(system.status(), Status::Running);
    let steps = run_until(&mut system, 4000, |s| s.status() == Status::Sleeping);
    assert!(steps >= QUIET_STEPS as usize - 2, "{steps}");
}

#[test]
fn a_force_affecting_parameter_change_resets_the_held_quiet_streak() {
    let mut system = held_and_idle();
    system.set_weights(&[5.0, 5.0, 5.0]).expect("valid weights");
    assert_eq!(system.quiet_streak(), 0);
    assert!(system.wants_frames());
    assert_eq!(system.status(), Status::Dragging);
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
// Sleep and the watchdog
// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -

#[test]
fn a_system_built_at_rest_is_asleep_and_asks_for_nothing() {
    let system = triangle(3.0);
    assert_eq!(system.status(), Status::Sleeping);
    assert!(!system.wants_frames());
    assert_eq!(system.quiet_streak(), QUIET_STEPS);
}

#[test]
fn a_step_of_a_sleeping_system_does_nothing_and_says_so() {
    let mut system = triangle(3.0);
    let outcome = system.step(FIXED_DT);
    assert!(!outcome.stepped);
    assert!(!outcome.slept);
}

#[test]
fn it_does_not_sleep_until_it_has_been_quiet_for_twenty_four_steps_in_a_row() {
    let mut system = triangle(3.0);
    system.particles[0].centre = v(150.0, 250.0);
    system.disturb();
    let mut slept_at = None;
    for step in 1..=6000 {
        let outcome = system.step(FIXED_DT);
        if outcome.slept {
            slept_at = Some(step);
            assert!(outcome.quiet_steps >= QUIET_STEPS);
            break;
        }
        assert_ne!(system.status(), Status::Sleeping, "slept at step {step} without saying so");
    }
    let slept_at = slept_at.expect("the system never slept");
    // It took longer than the quiet run alone, because it had to stop moving first.
    assert!(slept_at > QUIET_STEPS as usize);
    assert_eq!(system.status(), Status::Sleeping);
    assert!(!system.wants_frames());
    // On sleeping, every velocity is cleared.
    assert!(system.particles.iter().all(|p| p.velocity == Vec2::ZERO));
}

#[test]
fn one_noisy_step_in_the_middle_of_a_quiet_run_resets_the_count() {
    let mut system = triangle(3.0);
    system.particles[0].centre = v(190.0, 205.0);
    system.disturb();
    run_until(&mut system, 4000, |s| s.quiet_streak() >= 10);
    // A kick.
    system.particles[1].velocity = v(30.0, 0.0);
    system.step(FIXED_DT);
    assert_eq!(system.quiet_streak(), 0);
    assert_ne!(system.status(), Status::Sleeping);
}

#[test]
fn a_system_that_never_settles_is_unsettled_after_twenty_active_seconds_and_stops_asking_for_frames() {
    let params = Params {
        damping: 0.0,
        ..Params::default()
    };
    let mut system = SpringSystem::new(&[v(500.0, 500.0), v(650.0, 500.0)], &[20.0; 2], &[(0, 1, 5.0)], BIG, params)
        .expect("a valid system");
    system.particles[1].centre = v(690.0, 500.0);
    system.disturb();
    let mut unsettled_at = None;
    for step in 1..=3000 {
        if system.step(FIXED_DT).unsettled {
            unsettled_at = Some(step);
            break;
        }
    }
    assert!(
        matches!(unsettled_at, Some(2400 | 2401)),
        "twenty seconds is 2400 steps: {unsettled_at:?}"
    );
    assert_eq!(system.status(), Status::Unsettled);
    assert!(!system.wants_frames());
    // It is never reported as convergence, and stepping it does nothing.
    assert!(!system.step(FIXED_DT).stepped);
    assert_ne!(system.status(), Status::Sleeping);
}

#[test]
fn time_spent_holding_a_pin_does_not_count_against_the_watchdog() {
    let params = Params {
        damping: 0.0,
        ..Params::default()
    };
    let mut system = SpringSystem::new(
        &[v(500.0, 500.0), v(650.0, 500.0), v(575.0, 630.0)],
        &[20.0; 3],
        &[(0, 1, 5.0), (1, 2, 5.0), (2, 0, 5.0)],
        BIG,
        params,
    )
    .expect("a valid system");
    system.begin_pin(0, v(500.0, 500.0));
    system.update_pin_target(0, v(480.0, 500.0));
    // A minute of simulated time with the pin held.
    run(&mut system, 7200);
    assert_ne!(system.status(), Status::Unsettled);
    assert_eq!(system.active_time(), 0.0);
}

#[test]
fn a_disturbance_clears_an_unsettled_verdict_and_restarts_the_budget() {
    let params = Params {
        damping: 0.0,
        ..Params::default()
    };
    let mut system = SpringSystem::new(&[v(500.0, 500.0), v(650.0, 500.0)], &[20.0; 2], &[(0, 1, 5.0)], BIG, params)
        .expect("a valid system");
    system.particles[1].centre = v(690.0, 500.0);
    system.disturb();
    run(&mut system, 2500);
    assert_eq!(system.status(), Status::Unsettled);
    system.set_weights(&[6.0]).expect("valid weights");
    assert_eq!(system.status(), Status::Running);
    assert_eq!(system.active_time(), 0.0);
    assert!(system.wants_frames());
}

#[test]
fn pausing_stops_the_system_keeps_its_state_and_resuming_starts_a_fresh_quiet_run() {
    let mut system = triangle(3.0);
    system.particles[0].centre = v(150.0, 250.0);
    system.disturb();
    run(&mut system, 30);
    let frozen: Vec<Vec2> = (0..3).map(|i| system.centre(i).expect("a centre")).collect();
    system.set_paused(true);
    assert_eq!(system.status(), Status::Paused);
    assert!(!system.wants_frames());
    assert!(!system.step(FIXED_DT).stepped);
    let still: Vec<Vec2> = (0..3).map(|i| system.centre(i).expect("a centre")).collect();
    assert_eq!(frozen, still);

    system.set_paused(false);
    assert_eq!(system.status(), Status::Running);
    assert!(system.wants_frames());
    assert_eq!(system.quiet_streak(), 0);
    assert_eq!(system.active_time(), 0.0);
    run_until(&mut system, 6000, |s| s.status() == Status::Sleeping);
}

#[test]
fn pausing_a_held_pin_keeps_it_and_does_not_lose_the_position() {
    let mut system = triangle(3.0);
    system.begin_pin(0, v(200.0, 200.0));
    system.update_pin_target(0, v(260.0, 200.0));
    system.set_paused(true);
    system.set_paused(true);
    assert!(system.is_pinned(0));
    system.set_paused(false);
    run_until(&mut system, 4000, |s| s.proxies_arrived());
    assert_eq!(system.centre(0), Some(v(260.0, 200.0)));
}

#[test]
fn reset_restores_the_start_clears_pins_and_velocities_keeps_weights_and_sleeps() {
    let mut system = triangle(3.0);
    let start: Vec<Vec2> = (0..3).map(|i| system.centre(i).expect("a centre")).collect();
    system.set_weights(&[4.0, 5.0, 6.0]).expect("valid weights");
    system.begin_pin(0, v(200.0, 200.0));
    system.update_pin_target(0, v(300.0, 300.0));
    run(&mut system, 50);
    system.reset();
    let after: Vec<Vec2> = (0..3).map(|i| system.centre(i).expect("a centre")).collect();
    assert_eq!(start, after);
    assert!(!system.is_pinned(0));
    assert!(system.particles.iter().all(|p| p.velocity == Vec2::ZERO));
    assert_eq!(system.status(), Status::Sleeping);
    assert_eq!(
        system.springs().iter().map(|s| s.weight).collect::<Vec<_>>(),
        vec![4.0, 5.0, 6.0]
    );
}

#[test]
fn reset_does_not_change_whether_the_system_is_paused() {
    let mut system = triangle(3.0);
    system.set_paused(true);
    system.reset();
    assert_eq!(system.status(), Status::Paused);
}

#[test]
fn settle_now_runs_to_rest_without_frames_and_reports_it() {
    let mut system = triangle(3.0);
    system.particles[0].centre = v(150.0, 250.0);
    system.disturb();
    assert!(system.settle_now());
    assert_eq!(system.status(), Status::Sleeping);
}

#[test]
fn a_step_that_produces_a_number_that_is_not_finite_is_refused_and_flagged() {
    let mut system = triangle(3.0);
    system.particles[0].velocity = v(f64::INFINITY, 0.0);
    system.disturb();
    let outcome = system.step(FIXED_DT);
    assert!(outcome.faulted);
    assert_eq!(system.status(), Status::Unsettled);
    assert!(!system.wants_frames());
    // A reset puts a good state back.
    system.reset();
    assert_eq!(system.status(), Status::Sleeping);
}

#[test]
fn a_step_longer_than_the_limit_is_clamped_and_a_step_that_is_not_positive_is_ignored() {
    let mut system = triangle(3.0);
    system.particles[0].centre = v(150.0, 250.0);
    system.disturb();
    assert!(!system.step(0.0).stepped);
    assert!(!system.step(-1.0).stepped);
    assert!(!system.step(f64::NAN).stepped);
    let outcome = system.step(1e9);
    assert!(outcome.stepped && !outcome.faulted);
    assert!(system.particles.iter().all(|p| p.centre.is_finite()));
}

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

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Two circles of radius 20 at the two ends of `x`, on the line `y = 0`, in bounds that reach `reach` either way.
fn far_pair(x: f64, reach: f64, params: Params) -> Result<SpringSystem, SolverError> {
    SpringSystem::new(
        &[v(-x, 0.0), v(x, 0.0)],
        &[20.0; 2],
        &[(0, 1, 1.0)],
        Bounds {
            left: -reach,
            top: -reach,
            right: reach,
            bottom: reach,
        },
        params,
    )
}

#[test]
fn parameters_that_would_make_a_permitted_weight_overflow_are_refused() {
    let big = Params {
        base_stiffness: 1e308,
        ..Params::default()
    };
    assert_eq!(far_pair(100.0, 1000.0, big).err(), Some(SolverError::InvalidParams));
    // Each is finite and so is their quotient for the reference weight, but not for the heaviest weight.
    let tiny_reference = Params {
        base_stiffness: 1e10,
        reference_weight: 1e-300,
        ..Params::default()
    };
    assert_eq!(far_pair(100.0, 1000.0, tiny_reference).err(), Some(SolverError::InvalidParams));
}

#[test]
fn bounds_wider_than_any_number_can_say_are_refused() {
    // Finite edges, but the width is not.
    assert_eq!(
        far_pair(100.0, 1.5e308, Params::default()).err(),
        Some(SolverError::InvalidBounds)
    );
}

#[test]
fn a_rest_length_that_overflows_is_refused_and_one_that_is_merely_huge_is_not() {
    // Corner to corner, 1.6e308 either way: each difference is finite and the distance is not.
    let corners = SpringSystem::new(
        &[v(-8e307 + 20.0, -8e307 + 20.0), v(8e307 - 20.0, 8e307 - 20.0)],
        &[20.0; 2],
        &[(0, 1, 1.0)],
        Bounds {
            left: -8e307,
            top: -8e307,
            right: 8e307,
            bottom: 8e307,
        },
        Params::default(),
    );
    assert_eq!(corners.err(), Some(SolverError::NonFiniteSpring(0)));
    // A distance whose square is out of range is still a distance: `hypot` does not square it.
    let huge = far_pair(1e200, 2e200, Params::default()).expect("a long spring is still a spring");
    assert_eq!(huge.springs()[0].rest_length, 2e200);
}

#[test]
fn a_tension_that_overflows_a_rest_length_is_refused_and_changes_nothing() {
    let mut system = far_pair(3e307 - 20.0, 3e307, Params::default()).expect("a valid system");
    let natural = system.springs()[0].rest_length;
    assert!(natural.is_finite());
    // 6e307 times four is past the largest number.
    assert_eq!(system.set_tension(&[MAX_TENSION_FACTOR]), Err(SolverError::NonFiniteSpring(0)));
    assert_eq!(system.springs()[0].rest_length, natural);
    assert_eq!(system.status(), Status::Sleeping);
    // A factor that stays in range is taken.
    system.set_tension(&[1.5]).expect("in range");
    assert_eq!(system.springs()[0].rest_length, natural * 1.5);
}
