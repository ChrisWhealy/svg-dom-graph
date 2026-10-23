//! Sleep, the watchdog, pause, reset, settling at once, and a step that goes wrong.

use super::*;

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
