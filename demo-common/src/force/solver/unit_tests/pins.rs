//! Releasing, cancelling and re-pressing a pin, several pins at once, and a held node going quiet and waking.

use super::*;

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
