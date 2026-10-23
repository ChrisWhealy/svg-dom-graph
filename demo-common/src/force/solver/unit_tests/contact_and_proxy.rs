//! Bounds, contact and radius, and a pinned proxy's pursuit of its target.

use super::*;

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
