use super::*;
use crate::springs::fixture::Fixture;
use demo_common::force::solver::QUIET_STEPS;

/// Nine weights, and a tension that leaves every spring as long as it is.
const NEW_WEIGHTS: [f64; 9] = [1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0];
const FLAT: [f64; 9] = [1.0; 9];

fn core() -> Core {
    Core::new(Fixture::dijkstra().system().expect("a valid system"))
}

fn b_centre(core: &Core) -> Vec2 {
    core.system().centre(1).expect("a centre")
}

/// A frame time, in milliseconds, that is `n` fixed steps after `base`.
fn after(base: f64, steps: f64) -> f64 {
    base + steps * FIXED_DT * 1000.0 + 0.01
}

/// Runs frames, one display frame of two steps each, until `done`, and panics after `limit` frames.
fn run_until(core: &mut Core, limit: usize, done: impl Fn(&Core) -> bool) -> f64 {
    let mut now = 1000.0;
    core.frame(now);
    for _ in 0..limit {
        if done(core) {
            return now;
        }
        now = after(now, 2.0);
        let moves = core.frame(now);
        for (index, _) in moves {
            core.confirm_drawn(index);
        }
    }
    panic!("not done after {limit} frames; status {:?}", core.status());
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[test]
fn a_new_core_is_asleep_asks_for_no_frame_and_has_nothing_to_draw() {
    let mut core = core();
    assert_eq!(core.status(), Status::Sleeping);
    assert!(!core.wants_frame());
    assert!(core.reset_enabled());
    assert!(core.frame(1000.0).is_empty());
    assert_eq!(core.weights().len(), 9);
}

#[test]
fn a_held_circle_is_never_in_the_list_of_circles_to_draw_and_its_neighbours_are() {
    let mut core = core();
    let start = b_centre(&core);
    core.drag_start(1, start);
    core.drag_applied(1, start + Vec2::new(60.0, 25.0));
    assert!(core.wants_frame());
    let mut drawn_any = false;
    let mut now = 1000.0;
    core.frame(now);
    for _ in 0..40 {
        now = after(now, 2.0);
        let moves = core.frame(now);
        assert!(
            moves.iter().all(|&(index, _)| index != 1),
            "the held circle was written by the physics"
        );
        for (index, _) in &moves {
            core.confirm_drawn(*index);
        }
        drawn_any |= !moves.is_empty();
    }
    assert!(drawn_any, "no neighbour reacted");
}

#[test]
fn a_circle_whose_proxy_is_catching_up_after_release_is_not_written_until_it_has_arrived() {
    let mut core = core();
    let start = b_centre(&core);
    core.drag_start(1, start);
    let target = start + Vec2::new(90.0, 0.0);
    core.drag_applied(1, target);
    core.drag_end(1, target);
    assert!(core.system().any_draining());
    let mut now = 1000.0;
    core.frame(now);
    // 90 units at six a step is fifteen steps: until then it is draining and never written.
    for _ in 0..7 {
        now = after(now, 2.0);
        let moves = core.frame(now);
        assert!(moves.iter().all(|&(index, _)| index != 1));
        for (index, _) in moves {
            core.confirm_drawn(index);
        }
    }
    assert!(core.system().is_pinned(1));
    run_until(&mut core, 200, |c| !c.system().is_pinned(1));
}

#[test]
fn each_drag_event_updates_the_pin_and_what_the_core_believes_is_drawn() {
    let mut core = core();
    let start = b_centre(&core);
    core.drag_start(1, start);
    assert!(core.system().any_held());
    core.drag_applied(1, start + Vec2::new(10.0, 0.0));
    core.drag_end(1, start + Vec2::new(10.0, 0.0));
    assert!(core.system().any_draining() && !core.system().any_held());
    core.drag_start(1, start + Vec2::new(10.0, 0.0));
    core.drag_cancel(1, start + Vec2::new(10.0, 0.0));
    assert!(core.system().any_draining());
    // Events for a circle that is not there are ignored.
    core.drag_start(99, start);
    core.drag_applied(99, start);
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[test]
fn reset_is_rejected_while_a_circle_is_held_and_says_why() {
    let mut core = core();
    core.drag_start(1, b_centre(&core));
    assert!(!core.reset_enabled());
    let (outcome, moves) = core.reset();
    assert_eq!(outcome, ApplyOutcome::Rejected);
    assert!(moves.is_empty());
    assert_eq!(core.feedback(), Some(Feedback::ResetUnavailable));
    assert_eq!(
        core.feedback().map(Feedback::message),
        Some("Reset unavailable while dragging or finishing input.")
    );
    // Nothing was changed: the pin is still held.
    assert!(core.system().any_held());
}

#[test]
fn reset_is_rejected_while_a_released_proxy_is_draining_and_allowed_once_it_has_arrived() {
    let mut core = core();
    let start = b_centre(&core);
    core.drag_start(1, start);
    core.drag_applied(1, start + Vec2::new(90.0, 0.0));
    core.drag_end(1, start + Vec2::new(90.0, 0.0));
    assert_eq!(core.reset().0, ApplyOutcome::Rejected);
    assert!(core.feedback().is_some());
    // Run until the proxy lets go: the feedback clears by itself and Reset is back, without being run for the user.
    run_until(&mut core, 200, |c| c.reset_enabled());
    assert!(core.feedback().is_none(), "the rejection message stayed");
    assert!(core.reset_enabled());
    // The rejected Reset was not deferred: the circle has not been put back.
    assert_ne!(b_centre(&core), start);
}

#[test]
fn a_reset_that_runs_when_it_is_allowed_puts_every_circle_back_and_returns_what_to_draw() {
    let mut core = core();
    let start = b_centre(&core);
    core.drag_start(1, start);
    core.drag_applied(1, start + Vec2::new(60.0, 25.0));
    core.drag_end(1, start + Vec2::new(60.0, 25.0));
    run_until(&mut core, 600, |c| c.status() == Status::Sleeping);
    let (outcome, moves) = core.reset();
    assert_eq!(outcome, ApplyOutcome::Applied);
    assert_eq!(b_centre(&core), start);
    assert!(!moves.is_empty());
    // Putting the lengths back wakes the system for a moment, and with every spring at its length nothing pulls, so it
    // goes back to sleep where it was put.
    run_until(&mut core, 100, |c| c.status() == Status::Sleeping);
    assert_eq!(b_centre(&core), start);
    // Every circle that was away from its place is in the list, to its starting place.
    assert!(moves.iter().any(|&(index, centre)| index == 1 && centre == start));
}

#[test]
fn reset_when_nothing_has_moved_has_nothing_to_draw() {
    let mut core = core();
    let (outcome, moves) = core.reset();
    assert_eq!(outcome, ApplyOutcome::Applied);
    assert!(moves.is_empty());
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[test]
fn randomising_weights_changes_stiffness_only_and_does_not_move_a_graph_at_rest() {
    let mut core = core();
    let before = b_centre(&core);
    assert_eq!(
        core.randomise(&[10.0, 1.0, 3.0, 7.0, 2.0, 9.0, 4.0, 6.0, 8.0], &FLAT),
        ApplyOutcome::Applied
    );
    assert_eq!(core.weights()[0], 10.0);
    // A force change wakes it for a quiet check, and it then goes back to sleep without having moved.
    assert!(core.wants_frame());
    run_until(&mut core, 200, |c| c.status() == Status::Sleeping);
    assert_eq!(b_centre(&core), before);
}

#[test]
fn weights_the_solver_refuses_are_rejected_with_feedback_and_change_nothing() {
    let mut core = core();
    assert_eq!(core.randomise(&[1.0, 2.0], &FLAT), ApplyOutcome::Rejected);
    assert_eq!(core.feedback(), Some(Feedback::WeightsRefused));
    assert_eq!(core.weights(), vec![4.0, 2.0, 1.0, 5.0, 8.0, 10.0, 2.0, 6.0, 3.0]);
    assert_eq!(core.randomise(&[3.0; 9], &FLAT), ApplyOutcome::Applied);
    assert_eq!(core.feedback(), None);
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[test]
fn pausing_stops_frames_and_resuming_starts_from_a_fresh_clock_so_nothing_catches_up() {
    let mut core = core();
    let start = b_centre(&core);
    core.drag_start(1, start);
    core.drag_applied(1, start + Vec2::new(30.0, 10.0));
    let mut now = 1000.0;
    core.frame(now);
    for _ in 0..3 {
        now = after(now, 2.0);
        for (index, _) in core.frame(now) {
            core.confirm_drawn(index);
        }
    }
    core.set_user_paused(true);
    assert_eq!(core.status(), Status::Paused);
    assert!(!core.wants_frame());
    assert!(core.is_user_paused());
    core.set_user_paused(false);
    assert!(core.wants_frame());
    // The first frame after the resume is a new baseline even if an hour has passed.
    let moves = core.frame(1_000_000_000.0);
    assert!(moves.is_empty());
}

#[test]
fn a_hidden_page_pauses_and_says_so_and_showing_it_again_resumes_unless_the_user_paused() {
    let mut core = core();
    core.randomise(&[5.0; 9], &FLAT);
    core.set_hidden(true);
    assert_eq!(core.status(), Status::Paused);
    assert_eq!(core.status_text(), "Paused: the page is hidden.");
    core.set_hidden(false);
    assert_eq!(core.status(), Status::Running);

    core.set_user_paused(true);
    core.set_hidden(true);
    core.set_hidden(false);
    assert_eq!(core.status(), Status::Paused, "showing the page resumed a user pause");
    assert_eq!(core.status_text(), "Paused.");
}

#[test]
fn a_frame_after_the_system_went_idle_is_only_a_baseline() {
    let mut core = core();
    core.randomise(&[5.0; 9], &FLAT);
    run_until(&mut core, 300, |c| c.status() == Status::Sleeping);
    core.settled_down();
    core.randomise(&[2.0; 9], &FLAT);
    // Asleep for a long time, then woken: the first frame must not run the steps of the whole gap.
    assert!(core.frame(9_000_000.0).is_empty());
    assert!(core.system().quiet_streak() < QUIET_STEPS);
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[test]
fn with_reduced_motion_no_frame_is_ever_wanted_and_a_disturbance_settles_at_once() {
    let mut core = core();
    core.set_reduced_motion(true);
    assert!(core.is_reduced_motion());
    let start = b_centre(&core);
    core.drag_start(1, start);
    core.drag_applied(1, start + Vec2::new(60.0, 25.0));
    assert!(!core.wants_frame());
    core.drag_end(1, start + Vec2::new(60.0, 25.0));
    let moves = core.settle();
    assert!(!moves.is_empty(), "nothing was moved by settling");
    assert_eq!(core.status(), Status::Sleeping);
    assert!(!core.wants_frame());
}

#[test]
fn settling_does_nothing_without_reduced_motion() {
    let mut core = core();
    core.randomise(&[5.0; 9], &FLAT);
    assert!(core.settle().is_empty());
    assert_eq!(core.status(), Status::Running);
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[test]
fn a_drawing_failure_resyncs_to_where_the_page_really_is_pauses_and_says_why() {
    let mut core = core();
    let start = b_centre(&core);
    core.drag_start(1, start);
    let really: Vec<Vec2> = (0..6).map(|i| core.system().centre(i).expect("a centre")).collect();
    core.fail(&really, "a write failed".to_owned());
    assert!(!core.system().any_held());
    assert_eq!(core.status(), Status::Paused);
    assert!(core.status_text().starts_with("Physics stopped: a write failed"));
    assert_eq!(core.fault(), Some("a write failed"));
    // Resuming clears the fault.
    core.set_user_paused(false);
    assert!(core.fault().is_none());
    assert_ne!(core.status(), Status::Paused);
}

#[test]
fn the_status_text_names_each_state_in_words() {
    let mut core = core();
    assert!(core.status_text().starts_with("Sleeping"));
    core.randomise(&[5.0; 9], &FLAT);
    assert!(core.status_text().starts_with("Running"));
    let start = b_centre(&core);
    core.drag_start(1, start);
    assert_eq!(core.status_text(), "Dragging.");
    core.drag_end(1, start);
    assert!(core.status_text().starts_with("Finishing input"));
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[test]
fn randomising_applies_the_weights_and_a_tension_that_makes_the_network_settle_by_itself() {
    let mut core = core();
    assert_eq!(core.status(), Status::Sleeping);
    let start = b_centre(&core);
    let tension = [0.7, 1.5, 1.0, 1.3, 0.8, 1.0, 1.6, 0.6, 1.2];
    assert_eq!(core.randomise(&NEW_WEIGHTS, &tension), ApplyOutcome::Applied);
    assert_eq!(core.weights(), NEW_WEIGHTS.to_vec());
    // Nothing had been dragged and no circle was moved, yet the springs now pull, so the network is awake.
    assert_eq!(b_centre(&core), start);
    assert_ne!(core.status(), Status::Sleeping);
    assert!(core.wants_frame());
    run_until(&mut core, 2000, |c| c.status() == Status::Sleeping);
    assert_ne!(b_centre(&core), start, "the circles never moved");
}

#[test]
fn randomising_with_no_tension_goes_back_to_sleep_without_moving() {
    let mut core = core();
    let start = b_centre(&core);
    assert_eq!(core.randomise(&NEW_WEIGHTS, &FLAT), ApplyOutcome::Applied);
    run_until(&mut core, 100, |c| c.status() == Status::Sleeping);
    assert_eq!(b_centre(&core), start);
}

#[test]
fn a_randomise_the_solver_will_not_take_changes_nothing_and_says_so() {
    let mut core = core();
    let before = core.weights();
    // Good weights but a bad tension: the weights must not have been applied either.
    let outcome = core.randomise(&NEW_WEIGHTS, &[1.0, 1.0, 0.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0]);
    assert_eq!(outcome, ApplyOutcome::Rejected);
    assert_eq!(core.weights(), before);
    assert_eq!(core.status(), Status::Sleeping);
    assert_eq!(core.feedback(), Some(Feedback::WeightsRefused));
    // A good one afterwards clears the feedback.
    assert_eq!(core.randomise(&NEW_WEIGHTS, &FLAT), ApplyOutcome::Applied);
    assert_eq!(core.feedback(), None);
}

#[test]
fn reset_keeps_the_weights_and_puts_every_spring_back_to_its_length_so_the_layout_stays_put() {
    let mut core = core();
    let start = b_centre(&core);
    let natural: Vec<f64> = core.system().springs().iter().map(|s| s.rest_length).collect();
    core.randomise(&NEW_WEIGHTS, &[0.7, 1.5, 1.0, 1.3, 0.8, 1.0, 1.6, 0.6, 1.2]);
    run_until(&mut core, 2000, |c| c.status() == Status::Sleeping);
    assert_ne!(b_centre(&core), start);
    let (outcome, moves) = core.reset();
    assert_eq!(outcome, ApplyOutcome::Applied);
    assert!(!moves.is_empty());
    // The weights are as randomised, and the lengths are the starting ones.
    assert_eq!(core.weights(), NEW_WEIGHTS.to_vec());
    let now: Vec<f64> = core.system().springs().iter().map(|s| s.rest_length).collect();
    assert_eq!(now, natural);
    // And the network does not wander off again: it is at rest where it started.
    run_until(&mut core, 100, |c| c.status() == Status::Sleeping);
    assert_eq!(b_centre(&core), start);
}

#[test]
fn random_tension_stays_in_range_whatever_the_source_gives() {
    let given = [0.0, 1.0, 0.5, -3.0, 7.0, f64::NAN, f64::INFINITY, f64::NEG_INFINITY];
    let mut next = given.iter().cycle();
    let factors = random_tension(16, || *next.next().expect("endless"));
    assert_eq!(factors.len(), 16);
    let (low, high) = TENSION_RANGE;
    assert!(factors.iter().all(|f| (low..=high).contains(f)));
    assert!(factors.contains(&low) && factors.contains(&high));
}

#[test]
fn the_tension_chosen_for_a_randomise_always_settles_in_time_and_is_in_range() {
    let mut state = 987_654_321_u64;
    let mut unit = move || {
        state = state
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        ((state >> 11) as f64) / ((1_u64 << 53) as f64)
    };
    let mut core = core();
    let (low, high) = TENSION_RANGE;
    for _ in 0..60 {
        let weights: Vec<f64> = (0..9).map(|_| 1.0 + (unit() * 10.0).floor()).collect();
        let tension = core.choose_tension(&weights, &mut unit);
        assert_eq!(tension.len(), 9);
        assert!(tension.iter().all(|f| (low..=high).contains(f)), "{tension:?}");
        assert!(core.settles(&weights, &tension), "{tension:?} was chosen but does not settle");
        // The choice is what a randomise then runs, from wherever the last one left the circles, and the network does
        // come to rest.
        assert_eq!(core.randomise(&weights, &tension), ApplyOutcome::Applied);
        run_until(&mut core, 2000, |c| c.status() == Status::Sleeping);
    }
}

#[test]
fn a_tension_that_never_settles_is_not_chosen_and_the_fallback_leaves_every_spring_as_long_as_it_is() {
    let core = core();
    let weights = [5.0; 9];
    // A source that always gives the top of the range: every spring at its most stretched.
    let tension = core.choose_tension_in(&weights, || 1.0, 3, 0);
    // With no steps allowed in the trial nothing can settle by moving, so only a flat network passes.
    assert_eq!(tension, vec![1.0; 9]);
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[test]
fn two_queued_toggles_are_a_pause_and_a_resume_and_leave_the_original_state() {
    use crate::springs::controller::ControllerCommand;
    use demo_common::force::bridge::BridgeCommand;

    // A toggle is never merged into another command, or two of them into one.
    let mut tail = ControllerCommand::TogglePaused;
    assert!(!ControllerCommand::merge_into_tail(&mut tail, &ControllerCommand::TogglePaused));

    // Each runs against the state it finds, so the second undoes the first.
    let mut core = core();
    assert!(!core.is_user_paused());
    core.toggle_user_paused();
    assert!(core.is_user_paused());
    core.toggle_user_paused();
    assert!(!core.is_user_paused());

    // The same from a state that began paused.
    core.set_user_paused(true);
    core.toggle_user_paused();
    core.toggle_user_paused();
    assert!(core.is_user_paused());
}
