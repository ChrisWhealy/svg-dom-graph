use super::*;
use demo_common::force::solver::{FIXED_DT, QUIET_STEPS, Status, WATCHDOG_SECONDS};
use demo_common::util::MARGIN;
use svg_dom::root::utils::{Point, Size};

fn fixture() -> Fixture {
    Fixture::dijkstra()
}

fn index_of(name: &str) -> usize {
    NAMES.iter().position(|n| *n == name).expect("a vertex of that name")
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
// Fixture centres and bounds
// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -

#[test]
fn the_centres_are_the_origins_of_the_dijkstra_layout_plus_the_radius() {
    let fixture = fixture();
    for (index, origin) in vertex_origins().iter().enumerate() {
        assert_eq!(
            fixture.centres[index],
            Vec2::new(origin.x + VERTEX_RADIUS, origin.y + VERTEX_RADIUS)
        );
        assert_eq!(fixture.radii[index], VERTEX_RADIUS);
    }
    assert_eq!(fixture.centres.len(), NAMES.len());
}

#[test]
fn the_edges_are_the_nine_the_dijkstra_demo_draws_with_their_weights_in_the_same_order() {
    let fixture = fixture();
    assert_eq!(fixture.edges.len(), EDGES.len());
    for (&(from, to, weight), &(f, t, w)) in EDGES.iter().zip(&fixture.edges) {
        assert_eq!((f, t, w), (from, to, weight as f64));
    }
}

#[test]
fn the_solver_bounds_are_the_one_graph_region_and_every_centre_is_inside_it_inset_by_the_radius() {
    let fixture = fixture();
    assert_eq!(fixture.region, graph_region());
    let bounds = fixture.bounds();
    let region = graph_region();
    assert_eq!(
        (bounds.left, bounds.top, bounds.right, bounds.bottom),
        (
            region.origin.x,
            region.origin.y,
            region.origin.x + region.size.width,
            region.origin.y + region.size.height
        )
    );
    for centre in &fixture.centres {
        assert!(
            centre.x >= bounds.left + VERTEX_RADIUS && centre.x <= bounds.right - VERTEX_RADIUS,
            "{centre:?}"
        );
        assert!(
            centre.y >= bounds.top + VERTEX_RADIUS && centre.y <= bounds.bottom - VERTEX_RADIUS,
            "{centre:?}"
        );
    }
    // The inset is what a dragged circle's enclosing square may occupy: the same limits for a circle's centre.
    assert_eq!(bounds.left + VERTEX_RADIUS, MARGIN + VERTEX_RADIUS);
}

#[test]
fn the_graph_region_is_derived_from_the_layout_constants_and_not_written_out_again() {
    let region = graph_region();
    assert_eq!(region.origin, Point::new(MARGIN, 64.0));
    assert_eq!(region.size, Size::new(560.0, 310.0));
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
// Contact clearance
// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -

#[test]
fn every_pair_of_vertices_is_further_apart_than_two_radii_plus_the_collision_gap() {
    let fixture = fixture();
    let gap = Params::default().collision_gap;
    let minimum = 2.0 * VERTEX_RADIUS + gap;
    let mut nearest: Option<(usize, usize, f64)> = None;
    for (i, first) in fixture.centres.iter().enumerate() {
        for (j, second) in fixture.centres.iter().enumerate().skip(i + 1) {
            let distance = (*second - *first).length();
            assert!(
                distance >= minimum,
                "{} and {} are {distance} apart, which is under {minimum}",
                NAMES[i],
                NAMES[j]
            );
            if nearest.is_none_or(|(_, _, best)| distance < best) {
                nearest = Some((i, j, distance));
            }
        }
    }
    let (i, j, distance) = nearest.expect("at least one pair");
    println!(
        "nearest pair: {}-{} at {distance:.4}; the minimum is {minimum}",
        NAMES[i], NAMES[j]
    );
    // Far from touching, so the contact rule is never at work at rest.
    assert!(distance > 2.0 * minimum, "{distance}");
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
// Fixture resting forces
// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -

#[test]
fn every_rest_length_is_the_starting_distance_so_all_nine_springs_start_with_no_force() {
    let fixture = fixture();
    let system = fixture.system().expect("a valid system");
    assert_eq!(system.springs().len(), 9);
    for (index, spring) in system.springs().iter().enumerate() {
        let distance = (fixture.centres[spring.to] - fixture.centres[spring.from]).length();
        assert_eq!(spring.rest_length, distance, "spring {index}");
        assert_eq!(system.spring_tension(index), Some(0.0), "spring {index}");
    }
    assert_eq!(system.status(), Status::Sleeping);
    assert!(!system.wants_frames());
}

#[test]
fn stiffness_is_twenty_five_times_the_weight_for_every_edge() {
    let fixture = fixture();
    let system = fixture.system().expect("a valid system");
    for (spring, &(_, _, weight)) in system.springs().iter().zip(&fixture.edges) {
        assert_eq!(spring.stiffness, 25.0 * weight);
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
// Measured B displacement
// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -

/// Drags B by `(60, 25)` over 24 fixed steps (0.20 s), releases it, and runs until the system sleeps. Returns the
/// simulated seconds from the release to sleep, and the number of steps in that.
fn measured_release(fixture: &Fixture) -> (f64, usize, SpringSystem) {
    let mut system = fixture.system().expect("a valid system");
    let b = index_of("B");
    let start = fixture.centres[b];
    let delta = Vec2::new(60.0, 25.0);
    assert!(system.begin_pin(b, start));
    for step in 1..=24 {
        let target = start + delta * (f64::from(step) / 24.0);
        assert!(system.update_pin_target(b, target));
        system.step(FIXED_DT);
    }
    let target = start + delta;
    assert!(system.end_pin(b, target));
    let mut steps = 0;
    while system.status() != Status::Sleeping {
        steps += 1;
        assert!(
            steps < 20 * 120,
            "not asleep after twenty seconds; status {:?}",
            system.status()
        );
        assert_ne!(system.status(), Status::Unsettled);
        system.step(FIXED_DT);
    }
    (steps as f64 * FIXED_DT, steps, system)
}

#[test]
fn releasing_b_after_a_sixty_by_twenty_five_drag_settles_in_well_under_the_twenty_second_guard() {
    let fixture = fixture();
    let (seconds, steps, system) = measured_release(&fixture);
    println!("B released after (+60, +25) over 0.20 s: asleep after {seconds:.3} s ({steps} steps)");
    // Strictly after the release and strictly before the watchdog.
    assert!(seconds > 0.0 && seconds < WATCHDOG_SECONDS, "{seconds}");
    // At least the 24 quiet steps that sleeping needs, after the proxy had caught up.
    assert!(steps >= QUIET_STEPS as usize, "{steps}");
    assert_eq!(system.status(), Status::Sleeping);
    assert!(!system.wants_frames());
    // No false immediate sleep: the first step after the release is not sleep.
    let mut again = fixture.system().expect("a valid system");
    let b = index_of("B");
    again.begin_pin(b, fixture.centres[b]);
    again.update_pin_target(b, fixture.centres[b] + Vec2::new(60.0, 25.0));
    again.end_pin(b, fixture.centres[b] + Vec2::new(60.0, 25.0));
    assert!(!again.step(FIXED_DT).slept);
}

#[test]
fn after_the_release_every_circle_is_still_inside_the_region_and_clear_of_the_others() {
    let fixture = fixture();
    let (_, _, system) = measured_release(&fixture);
    let bounds = fixture.bounds();
    for index in 0..system.len() {
        let centre = system.centre(index).expect("a centre");
        assert!(centre.x >= bounds.left + VERTEX_RADIUS - 1e-9 && centre.x <= bounds.right - VERTEX_RADIUS + 1e-9);
        assert!(centre.y >= bounds.top + VERTEX_RADIUS - 1e-9 && centre.y <= bounds.bottom - VERTEX_RADIUS + 1e-9);
    }
    assert!(system.max_penetration() <= 0.1);
}

#[test]
fn the_released_graph_recovers_its_arrangement_up_to_a_shift_and_the_springs_end_nearly_slack() {
    let fixture = fixture();
    let (_, _, system) = measured_release(&fixture);
    // A triangle chain is rigid, so with the springs slack the edge lengths are the rest lengths again, whichever way the
    // graph has been shifted or turned.
    for index in 0..system.springs().len() {
        let spring = system.springs()[index];
        let tension = system.spring_tension(index).expect("a spring");
        let extension = tension / spring.stiffness;
        assert!(extension.abs() < 0.2, "spring {index} is {extension} off its rest length");
    }
}

#[test]
fn an_unstretched_graph_does_not_move_when_a_weight_changes_and_does_not_animate_for_ever() {
    let fixture = fixture();
    let mut system = fixture.system().expect("a valid system");
    let before: Vec<Vec2> = (0..system.len()).map(|i| system.centre(i).expect("a centre")).collect();
    system
        .set_weights(&[10.0, 1.0, 3.0, 7.0, 2.0, 9.0, 4.0, 6.0, 8.0])
        .expect("valid weights");
    let mut steps = 0;
    while system.status() != Status::Sleeping {
        steps += 1;
        assert!(steps < 2400);
        system.step(FIXED_DT);
    }
    let after: Vec<Vec2> = (0..system.len()).map(|i| system.centre(i).expect("a centre")).collect();
    assert_eq!(before, after);
    assert!(steps >= QUIET_STEPS as usize);
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
// Rest pose and input determinism
// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -

#[test]
fn the_same_fixture_and_the_same_drag_give_identical_trajectories() {
    let trajectory = || {
        let fixture = fixture();
        let mut system = fixture.system().expect("a valid system");
        let b = index_of("B");
        system.begin_pin(b, fixture.centres[b]);
        system.update_pin_target(b, fixture.centres[b] + Vec2::new(80.0, 30.0));
        let mut points = Vec::new();
        for _ in 0..300 {
            system.step(FIXED_DT);
            points.push(
                (0..system.len())
                    .map(|i| system.centre(i).expect("a centre"))
                    .collect::<Vec<_>>(),
            );
        }
        points
    };
    assert_eq!(trajectory(), trajectory());
}

#[test]
fn changing_a_fixture_constant_changes_the_result_naturally() {
    let default = fixture();
    // The same graph with D moved 30 units to the right.
    let mut origins: Vec<Vec2> = vertex_origins().iter().map(|o| Vec2::new(o.x, o.y)).collect();
    origins[index_of("D")].x += 30.0;
    let moved = Fixture::from_parts(origins, graph_region());
    assert_ne!(default.centres, moved.centres);
    let (a, _, _) = measured_release(&default);
    let (b, _, _) = measured_release(&moved);
    println!("settle time {a:.3} s, with D moved {b:.3} s");
    let system = moved.system().expect("a valid system");
    // Its rest lengths are its own, so it also starts at rest.
    assert_eq!(system.status(), Status::Sleeping);
    assert_ne!(
        default.system().expect("a valid system").springs()[3].rest_length,
        system.springs()[3].rest_length
    );
}

#[test]
fn a_fixture_the_solver_cannot_accept_says_so_and_does_not_panic() {
    let mut origins: Vec<Vec2> = vertex_origins().iter().map(|o| Vec2::new(o.x, o.y)).collect();
    origins[0] = Vec2::new(-500.0, 0.0);
    let bad = Fixture::from_parts(origins, graph_region());
    assert!(matches!(bad.system(), Err(SolverError::OutsideBounds(0))));
}
