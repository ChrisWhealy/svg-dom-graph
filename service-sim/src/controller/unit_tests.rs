use super::*;
use crate::projection::{View, extract, project};
use crate::scheduler::ManualScheduler;
use service_sim_engine::trace::Scenario;
use service_sim_engine::{Engine, Limits};
use std::cell::RefCell;
use std::rc::Rc;

const FIXTURES: &str = include_str!("../../../dess/s1/fixtures.json");

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// What a fake host did, shared with the test that drives it.
struct Log {
    presence: Presence,
    /// Every draw, as the hash of what it drew and whether it was full.
    draws: Vec<(String, bool)>,
    /// What the page holds, if it is known: a delta is written onto it, and a failed draw makes it unknown.
    page: Option<Projection>,
    /// The draw, counted from 1, that fails with a write error, and the one that detaches the host in its middle and
    /// returns Ok.
    fail_at: Vec<usize>,
    detach_at: Option<usize>,
    announces: Vec<String>,
    reports: Vec<String>,
    playing: Vec<bool>,
    deltas_on_unknown: usize,
}

struct Fake(Rc<RefCell<Log>>);

impl Host for Fake {
    fn presence(&self) -> Presence {
        self.0.borrow().presence
    }

    fn draw(&mut self, p: &Projection, full: bool) -> Result<(), DrawFailure> {
        let mut log = self.0.borrow_mut();
        let n = log.draws.len() + 1;
        log.draws.push((p.sha256(), full));
        if log.detach_at == Some(n) {
            // A detached element goes on accepting writes, so the draw reports no error, and the page is partly
            // written.
            log.presence = Presence::Detached;
            log.page = None;
            return Ok(());
        }
        if log.fail_at.contains(&n) {
            log.page = None;
            return Err(DrawFailure::Failed("a write failed".into()));
        }
        if !full && log.page.is_none() {
            log.deltas_on_unknown += 1;
        }
        log.page = Some(p.clone());
        Ok(())
    }

    fn announce(&mut self, text: &str) {
        self.0.borrow_mut().announces.push(text.to_owned());
    }

    fn report(&mut self, text: &str) {
        self.0.borrow_mut().reports.push(text.to_owned());
    }

    fn playing_changed(&mut self, playing: bool) {
        self.0.borrow_mut().playing.push(playing);
    }
}

type C = Controller<Fake, ManualScheduler>;

fn log() -> Rc<RefCell<Log>> {
    Rc::new(RefCell::new(Log {
        presence: Presence::Live,
        draws: Vec::new(),
        page: None,
        fail_at: Vec::new(),
        detach_at: None,
        announces: Vec::new(),
        reports: Vec::new(),
        playing: Vec::new(),
        deltas_on_unknown: 0,
    }))
}

fn factory_of(scenario: Scenario) -> Factory {
    Rc::new(move || {
        Engine::new(
            &scenario.config_id,
            scenario.seed,
            scenario.config,
            &scenario.script,
            scenario.params.as_ref(),
            Limits::default(),
        )
    })
}

fn e0() -> Factory {
    let fixtures: serde_json::Value = serde_json::from_str(FIXTURES).expect("fixtures");
    factory_of(Scenario::fixture("E0", &fixtures["E0"]).expect("a fixture"))
}

fn generated(seed: u64) -> Factory {
    factory_of(Scenario::panel(seed).expect("a panel"))
}

fn start(factory: Factory) -> (C, Rc<RefCell<Log>>) {
    let log = log();
    let c = Controller::new(Fake(Rc::clone(&log)), ManualScheduler::default(), factory, View::default())
        .expect("a controller");
    (c, log)
}

/// The projection of slice `k`, made straight from an engine that was stepped to it, which is what every route has to
/// arrive at.
fn straight(factory: &Factory, k: u64, view: &View) -> Projection {
    let mut engine = factory().expect("an engine");
    for _ in 0..k {
        engine.step().expect("a slice").expect("a slice that exists");
    }
    project(&extract(&engine, view), view)
}

fn page(log: &Rc<RefCell<Log>>) -> Option<String> {
    log.borrow().page.as_ref().map(Projection::canonical)
}

fn run_frames(c: &C) {
    while c.scheduler().advance_frame() {}
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[test]
fn a_new_controller_draws_its_first_slice_in_full_and_is_clean() {
    let (c, log) = start(e0());
    assert_eq!((c.committed_slice(), c.is_dirty(), c.is_playing()), (0, false, false));
    assert_eq!(log.borrow().draws.len(), 1);
    assert!(log.borrow().draws[0].1, "the first draw is a full one");
    assert_eq!(page(&log), Some(c.committed().canonical()));
}

#[test]
fn next_walks_to_the_last_slice_and_has_nowhere_to_go_there() {
    let factory = e0();
    let (c, log) = start(Rc::clone(&factory));
    for k in 1..=12 {
        assert_eq!(c.next(), Outcome::Done, "slice {k}");
        assert_eq!(c.committed_slice(), k);
        assert_eq!(c.committed().canonical(), straight(&factory, k, &View::default()).canonical());
        assert_eq!(page(&log), Some(c.committed().canonical()));
    }
    let draws = log.borrow().draws.len();
    assert_eq!(c.next(), Outcome::AtBoundary);
    assert_eq!(c.last_slice(), Some(12));
    assert_eq!(log.borrow().draws.len(), draws, "a boundary draws nothing");
    assert!(
        log.borrow().draws[1..].iter().all(|d| !d.1),
        "every slice after the first is a delta"
    );
}

#[test]
fn full_step_and_seek_arrive_at_the_same_slice_with_the_same_page() {
    let factory = generated(3);
    for k in [0, 1, 2, 7, 15, 31] {
        let want = straight(&factory, k, &View::default());
        // By steps.
        let (c, log) = start(Rc::clone(&factory));
        for _ in 0..k {
            assert_eq!(c.next(), Outcome::Done);
        }
        assert_eq!(c.committed().canonical(), want.canonical(), "stepping to {k}");
        assert_eq!(page(&log), Some(want.canonical()));
        // By a seek from the start.
        let (c, log) = start(Rc::clone(&factory));
        if k > 0 {
            assert_eq!(c.seek(k), Outcome::Pending);
            run_frames(&c);
        }
        assert_eq!(c.committed_slice(), k);
        assert_eq!(c.committed().canonical(), want.canonical(), "seeking to {k}");
        assert_eq!(page(&log), Some(want.canonical()));
        // By a seek from somewhere else, backwards and forwards.
        let (c, log) = start(Rc::clone(&factory));
        c.seek(20);
        run_frames(&c);
        if k != 20 {
            c.seek(k);
            run_frames(&c);
        }
        assert_eq!(c.committed().canonical(), want.canonical(), "seeking back or on to {k}");
        assert_eq!(page(&log), Some(want.canonical()));
    }
}

#[test]
fn previous_and_restart_replay_and_the_first_slice_is_a_boundary() {
    let factory = e0();
    let (c, log) = start(Rc::clone(&factory));
    assert_eq!(c.previous(), Outcome::AtBoundary);
    assert_eq!(c.restart(), Outcome::AtBoundary);
    for _ in 0..5 {
        c.next();
    }
    assert_eq!(c.previous(), Outcome::Pending);
    run_frames(&c);
    assert_eq!(c.committed_slice(), 4);
    assert_eq!(page(&log), Some(straight(&factory, 4, &View::default()).canonical()));
    assert_eq!(c.restart(), Outcome::Pending);
    run_frames(&c);
    assert_eq!(c.committed_slice(), 0);
    assert_eq!(page(&log), Some(straight(&factory, 0, &View::default()).canonical()));
}

#[test]
fn a_seek_past_the_end_stops_at_the_last_slice() {
    let (c, _) = start(e0());
    assert_eq!(c.seek(500), Outcome::Pending);
    run_frames(&c);
    assert_eq!(c.committed_slice(), 12);
    assert_eq!(c.last_slice(), Some(12));
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[test]
fn playback_commits_one_slice_a_delay_and_schedules_the_next_only_after_the_commit() {
    let (c, log) = start(e0());
    c.set_speed(250);
    assert_eq!(c.play(), Outcome::Done);
    assert!(c.is_playing());
    assert_eq!((c.scheduler().pending(), c.scheduler().next_delay()), (1, Some(250)));
    c.scheduler().advance_by(249);
    assert_eq!(c.committed_slice(), 0, "nothing comes before the delay");
    c.scheduler().advance_by(1);
    assert_eq!(c.committed_slice(), 1);
    assert_eq!(c.scheduler().pending(), 1, "exactly one timer is waiting");
    c.scheduler().advance_by(250 * 20);
    assert_eq!(c.committed_slice(), 12, "it stops at the end");
    assert!(!c.is_playing());
    assert_eq!(c.scheduler().pending(), 0);
    assert_eq!(log.borrow().playing, vec![true, false]);
}

#[test]
fn pause_takes_back_the_timer_and_a_kept_one_that_runs_is_inert() {
    let (c, log) = start(e0());
    c.scheduler().keep_cancelled(true);
    c.set_speed(100);
    c.play();
    c.pause();
    assert!(!c.is_playing());
    assert_eq!((c.scheduler().pending(), c.scheduler().held()), (0, 1));
    let draws = log.borrow().draws.len();
    c.scheduler().advance_by(1_000);
    assert_eq!(c.committed_slice(), 0);
    assert_eq!(
        log.borrow().draws.len(),
        draws,
        "the callback that was already on its way drew nothing"
    );
    c.play();
    c.scheduler().advance_by(100);
    assert_eq!(c.committed_slice(), 1, "play after pause carries on from the committed slice");
}

#[test]
fn a_speed_change_while_playing_replaces_the_timer_and_a_slice_does_not_come_twice() {
    let (c, _) = start(generated(3));
    c.scheduler().keep_cancelled(true);
    c.set_speed(400);
    c.play();
    c.scheduler().advance_by(100);
    c.set_speed(50);
    assert_eq!(c.scheduler().pending(), 1);
    assert_eq!(c.scheduler().next_delay(), Some(50));
    c.scheduler().advance_by(50);
    assert_eq!(c.committed_slice(), 1, "the old timer's callback, though kept, did nothing");
    c.scheduler().advance_by(500);
    assert_eq!(c.committed_slice(), 11, "ten more at 50 ms each, and no more");
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[test]
fn one_callback_queued_before_a_hide_is_inert_with_or_without_the_hint() {
    for hinted in [true, false] {
        let (c, log) = start(e0());
        c.scheduler().keep_cancelled(true);
        c.set_speed(100);
        c.play();
        let (draws, epoch) = (log.borrow().draws.len(), c.epoch());
        log.borrow_mut().presence = Presence::Hidden;
        if hinted {
            c.presence_changed();
            assert!(c.epoch() > epoch, "the hint invalidates what was scheduled");
        }
        c.scheduler().advance_by(1_000);
        assert_eq!(c.committed_slice(), 0, "hinted {hinted}");
        assert_eq!(log.borrow().draws.len(), draws, "it drew nothing, hinted {hinted}");
        assert!(!c.is_playing());
        assert_eq!(log.borrow().announces.len(), 1, "one announcement, hinted {hinted}");
        assert!(log.borrow().reports.is_empty(), "hiding is not an error");
        assert_eq!((c.scheduler().pending(), c.scheduler().pending_frames()), (0, 0));
    }
}

#[test]
fn showing_again_repairs_a_dirty_page_and_never_resumes_playback() {
    let (c, log) = start(e0());
    c.set_speed(100);
    c.next();
    c.play();
    log.borrow_mut().presence = Presence::Hidden;
    c.presence_changed();
    assert!(c.is_dirty() && !c.is_playing());
    log.borrow_mut().page = None;
    log.borrow_mut().presence = Presence::Live;
    c.presence_changed();
    assert!(!c.is_dirty(), "the committed slice was drawn again");
    assert_eq!(page(&log), Some(c.committed().canonical()));
    assert!(log.borrow().draws.last().expect("a draw").1, "the repair is a full draw");
    c.scheduler().advance_by(10_000);
    assert_eq!(c.committed_slice(), 1, "it stays paused, and resumes only when asked");
    assert_eq!(c.play(), Outcome::Done);
    c.scheduler().advance_by(100);
    assert_eq!(c.committed_slice(), 2);
    assert_eq!(
        log.borrow().deltas_on_unknown,
        0,
        "no delta was ever written onto a page that was not known"
    );
}

#[test]
fn a_host_that_is_hidden_or_detached_before_a_request_gets_no_draw_and_no_commit() {
    for presence in [Presence::Hidden, Presence::Detached, Presence::Disposed] {
        let (c, log) = start(e0());
        let draws = log.borrow().draws.len();
        log.borrow_mut().presence = presence;
        for outcome in [c.next(), c.play(), c.select_job(Some(0)), c.repair()] {
            assert_eq!(outcome, Outcome::Unavailable, "{presence:?}");
        }
        assert_eq!(c.seek(3), Outcome::Unavailable);
        assert_eq!(c.committed_slice(), 0);
        assert_eq!(log.borrow().draws.len(), draws, "nothing was drawn, {presence:?}");
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[test]
fn a_host_that_leaves_in_the_middle_of_a_draw_is_never_committed_and_is_repaired_on_return() {
    let factory = e0();
    let (c, log) = start(Rc::clone(&factory));
    c.next();
    c.set_speed(50);
    c.play();
    // The next draw, which is the third, detaches the host and reports no error, as a detached element would.
    log.borrow_mut().detach_at = Some(3);
    c.scheduler().advance_by(50);
    assert_eq!(
        c.committed_slice(),
        1,
        "the slice that was drawn as the host left is not committed"
    );
    assert!(c.is_dirty() && !c.is_playing());
    assert_eq!(log.borrow().announces.len(), 1);
    let draws = log.borrow().draws.len();
    c.scheduler().advance_by(10_000);
    assert_eq!(log.borrow().draws.len(), draws, "no draw follows the departure");

    // Back again: the committed slice is drawn in full and playback stays stopped.
    log.borrow_mut().presence = Presence::Live;
    c.presence_changed();
    assert!(!c.is_dirty());
    assert_eq!(page(&log), Some(straight(&factory, 1, &View::default()).canonical()));
    assert_eq!(c.next(), Outcome::Done);
    assert_eq!(c.committed_slice(), 2, "the slice that was staged is not stepped twice");
    assert_eq!(c.committed().canonical(), straight(&factory, 2, &View::default()).canonical());
}

#[test]
fn a_host_that_leaves_in_the_middle_of_a_seek_never_commits_it() {
    let (c, log) = start(generated(3));
    assert_eq!(c.seek(9), Outcome::Pending);
    log.borrow_mut().detach_at = Some(2);
    run_frames(&c);
    assert_eq!(c.committed_slice(), 0);
    assert!(c.is_dirty());
    assert!(log.borrow().reports.is_empty(), "a host that left is not a failed write");
}

#[test]
fn a_failed_write_commits_nothing_reports_once_repairs_once_and_stays_stopped() {
    let factory = e0();
    let (c, log) = start(Rc::clone(&factory));
    c.next();
    c.set_speed(50);
    c.play();
    log.borrow_mut().fail_at = vec![3];
    c.scheduler().advance_by(50);
    assert_eq!(c.committed_slice(), 1);
    assert!(!c.is_playing(), "playback stopped");
    assert_eq!(log.borrow().reports.len(), 1, "one report");
    let repair = log.borrow().draws.last().cloned().expect("a draw");
    assert_eq!(log.borrow().draws.len(), 4, "one failed draw and one repair, and no retry loop");
    assert!(repair.1, "the repair is full");
    assert!(!c.is_dirty(), "the repair succeeded");
    assert_eq!(page(&log), Some(c.committed().canonical()));
    c.scheduler().advance_by(10_000);
    assert_eq!(c.committed_slice(), 1, "the failed request is not retried by itself");
    assert_eq!(c.next(), Outcome::Done);
    assert_eq!(c.committed().canonical(), straight(&factory, 2, &View::default()).canonical());
}

#[test]
fn a_repair_that_fails_leaves_the_page_dirty_and_a_later_request_repairs_it_even_at_a_boundary() {
    let (c, log) = start(e0());
    // Draws are counted from 1: the first slice, the failed one, and the repair, which also fails.
    log.borrow_mut().fail_at = vec![2, 3];
    assert_eq!(c.next(), Outcome::Failed);
    assert!(c.is_dirty());
    assert_eq!(log.borrow().reports.len(), 1);
    assert_eq!(c.committed_slice(), 0);
    // A restart at the first slice has nowhere to go, and still draws the committed slice again first.
    assert_eq!(c.restart(), Outcome::AtBoundary);
    assert!(!c.is_dirty(), "the boundary request repaired the page");
    assert_eq!(page(&log), Some(c.committed().canonical()));
    assert_eq!(log.borrow().reports.len(), 1, "no second report for the same episode");
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[test]
fn a_seek_replaces_the_one_before_it_and_a_kept_frame_of_the_first_is_inert() {
    let factory = generated(3);
    let (c, log) = start(Rc::clone(&factory));
    c.scheduler().keep_cancelled(true);
    c.seek(10);
    let first = c.seek_token();
    c.seek(4);
    assert!(c.seek_token() > first);
    assert_eq!(
        c.scheduler().pending_frames(),
        1,
        "one live frame, and one held that is cancelled"
    );
    assert_eq!(c.scheduler().held(), 2);
    run_frames(&c);
    assert_eq!(c.committed_slice(), 4);
    assert_eq!(
        log.borrow().draws.len(),
        2,
        "the first page and the second seek, and nothing for the first"
    );
    assert_eq!(c.committed().canonical(), straight(&factory, 4, &View::default()).canonical());
}

#[test]
fn a_forced_one_slice_seek_can_be_cancelled_between_frames_and_changes_nothing_that_is_shown() {
    let factory = generated(3);
    let (c, log) = start(Rc::clone(&factory));
    c.force_chunk(Some(1));
    c.seek(6);
    let shown = page(&log);
    for _ in 0..3 {
        assert!(c.scheduler().advance_frame());
        assert_eq!(c.committed_slice(), 0, "nothing is committed until the seek completes");
        assert_eq!(page(&log), shown, "the page is untouched by a stage that is not done");
        assert!(c.scheduler().pending_frames() <= 1);
    }
    // A new seek replaces it half done.
    c.seek(2);
    assert_eq!(c.scheduler().pending_frames(), 1);
    let mut frames = 0;
    while c.scheduler().advance_frame() {
        frames += 1;
        assert!(c.scheduler().pending_frames() <= 1);
    }
    assert_eq!(
        frames, 2,
        "one slice a frame, and the frame that replays the last one commits it"
    );
    assert_eq!(c.committed_slice(), 2);
    assert_eq!(c.committed().canonical(), straight(&factory, 2, &View::default()).canonical());
    assert_eq!(log.borrow().draws.len(), 2, "one draw for the seek that completed");
}

#[test]
fn a_seek_whose_frame_cannot_be_asked_for_keeps_the_cursor_and_can_be_retried() {
    let factory = e0();
    let (c, _) = start(Rc::clone(&factory));
    c.next();
    c.scheduler().fail_next_frames(1);
    assert_eq!(c.seek(6), Outcome::Failed);
    assert_eq!((c.committed_slice(), c.pending_retry()), (1, Some(6)));
    assert_eq!(c.retry(), Outcome::Pending);
    run_frames(&c);
    assert_eq!(c.committed_slice(), 6);
    assert_eq!(c.pending_retry(), None);
}

#[test]
fn a_timer_that_cannot_be_scheduled_stops_playback_and_keeps_the_display() {
    let (c, log) = start(e0());
    c.scheduler().fail_next_timers(1);
    assert_eq!(c.play(), Outcome::Failed);
    assert!(!c.is_playing());
    assert_eq!(log.borrow().reports.len(), 1);
    // Play, and let the second timer fail after a slice was committed.
    c.set_speed(10);
    assert_eq!(c.play(), Outcome::Done);
    c.scheduler().fail_next_timers(1);
    c.scheduler().advance_by(10);
    assert_eq!(c.committed_slice(), 1, "the slice that was drawn stays");
    assert!(!c.is_playing(), "and playback stopped, as there was no timer for the next");
    assert_eq!(page(&log), Some(c.committed().canonical()));
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[test]
fn a_scenario_switch_invalidates_what_was_scheduled_and_draws_the_new_history_in_full() {
    let (c, log) = start(e0());
    c.scheduler().keep_cancelled(true);
    c.set_speed(100);
    c.next();
    c.next();
    c.play();
    c.seek(8);
    let generation = c.generation();
    let other = e0();
    assert_eq!(c.switch_scenario(Rc::clone(&other)).expect("a scenario"), Outcome::Done);
    assert!(c.generation() > generation);
    assert_eq!((c.committed_slice(), c.is_playing()), (0, false));
    assert!(log.borrow().draws.last().expect("a draw").1, "the new history is drawn in full");
    let draws = log.borrow().draws.len();
    c.scheduler().advance_by(10_000);
    run_frames(&c);
    assert_eq!(c.committed_slice(), 0, "the timer and the frame of the old history did nothing");
    assert_eq!(log.borrow().draws.len(), draws);
    assert_eq!(c.next(), Outcome::Done);
}

#[test]
fn selecting_a_job_redraws_the_same_slice_with_the_highlight_and_a_later_slice_keeps_it() {
    let factory = e0();
    let (c, log) = start(Rc::clone(&factory));
    c.next();
    c.next();
    let plain = c.committed().canonical();
    assert_eq!(c.select_job(Some(0)), Outcome::Done);
    assert_eq!(c.committed_slice(), 2, "the slice did not change");
    assert_ne!(c.committed().canonical(), plain);
    let view = View { selected: Some(0) };
    assert_eq!(c.committed().canonical(), straight(&factory, 2, &view).canonical());
    assert_eq!(c.next(), Outcome::Done);
    assert_eq!(c.committed().canonical(), straight(&factory, 3, &view).canonical());
    assert_eq!(page(&log), Some(c.committed().canonical()));
    assert_eq!(c.select_job(None), Outcome::Done);
    assert_eq!(c.committed().canonical(), straight(&factory, 3, &View::default()).canonical());
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[test]
fn a_disposed_controller_draws_nothing_and_late_callbacks_are_inert() {
    let (c, log) = start(e0());
    c.scheduler().keep_cancelled(true);
    c.set_speed(10);
    c.play();
    c.seek(5);
    c.dispose();
    let draws = log.borrow().draws.len();
    c.scheduler().advance_by(1_000);
    run_frames(&c);
    assert_eq!(log.borrow().draws.len(), draws);
    assert_eq!(c.committed_slice(), 0);
    assert!(!c.is_playing());
    assert_eq!(c.next(), Outcome::Unavailable);
    assert_eq!(c.play(), Outcome::Unavailable);
}

#[test]
fn scheduled_callbacks_hold_no_strong_reference_to_the_controller() {
    let (c, _) = start(e0());
    c.set_speed(10);
    c.play();
    assert_eq!(c.scheduler().held(), 1);
    assert_eq!(
        Rc::strong_count(&c.inner),
        1,
        "a timer is waiting, and it does not hold the controller"
    );
    // A seek takes the timer back and waits for a frame, which does not hold the controller either.
    c.seek(4);
    assert_eq!((c.scheduler().held(), c.scheduler().pending_frames()), (1, 1));
    assert_eq!(
        Rc::strong_count(&c.inner),
        1,
        "a frame is waiting, and it does not hold the controller"
    );
    // Dropping the controller while its callback is held lets it go, and the callback finds nothing to act on.
    let probe = Rc::downgrade(&c.inner);
    drop(c);
    assert!(probe.upgrade().is_none(), "the controller was released");
}

#[test]
fn at_most_one_timer_and_one_frame_are_live_through_a_busy_sequence() {
    let (c, _) = start(generated(3));
    c.set_speed(30);
    for step in 0..40_u64 {
        match step % 5 {
            0 => {
                c.play();
            },
            1 => {
                c.seek(step);
            },
            2 => {
                c.set_speed(20 + step as u32);
            },
            3 => c.pause(),
            _ => {
                c.scheduler().advance_by(100);
                c.scheduler().advance_frame();
            },
        }
        assert!(c.scheduler().pending() <= 1, "timers at step {step}");
        assert!(c.scheduler().pending_frames() <= 1, "frames at step {step}");
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[test]
fn a_frame_of_a_seek_that_was_replaced_does_no_work_for_the_new_one() {
    let factory = generated(3);
    let (c, _) = start(Rc::clone(&factory));
    c.scheduler().keep_cancelled(true);
    c.force_chunk(Some(1));
    c.seek(6);
    assert!(c.scheduler().advance_frame(), "the first seek makes some progress");
    // The seek is replaced. Its next frame was taken back and is kept, as a callback already on its way would be.
    c.seek(3);
    let mut frames = 0;
    while c.scheduler().advance_frame() {
        frames += 1;
        if frames == 3 {
            assert_eq!(
                c.committed_slice(),
                0,
                "the stale frame took no step, so three slices are not replayed in three frames"
            );
        }
    }
    assert_eq!(
        frames, 4,
        "one stale frame that does nothing, and three that replay three slices one a frame"
    );
    assert_eq!(c.committed_slice(), 3);
    assert_eq!(c.committed().canonical(), straight(&factory, 3, &View::default()).canonical());
}

#[test]
fn a_slice_that_was_staged_is_not_worked_out_again_after_a_failed_draw() {
    let made = Rc::new(std::cell::Cell::new(0_u32));
    let counted: Factory = {
        let (inner, made) = (e0(), Rc::clone(&made));
        Rc::new(move || {
            made.set(made.get() + 1);
            inner()
        })
    };
    let (c, log) = start(counted);
    c.next();
    let engines = made.get();
    log.borrow_mut().fail_at = vec![3];
    assert_eq!(c.next(), Outcome::Failed);
    assert_eq!(made.get(), engines, "the failed draw made no engine");
    assert_eq!(c.next(), Outcome::Done);
    assert_eq!(c.committed_slice(), 2);
    assert_eq!(
        made.get(),
        engines,
        "the staged slice was committed, and the run was not replayed to find it again"
    );
}

#[test]
fn a_hidden_host_is_announced_once_for_the_whole_episode_and_again_after_it_was_shown() {
    let (c, log) = start(e0());
    log.borrow_mut().presence = Presence::Hidden;
    c.next();
    c.next();
    c.play();
    c.seek(3);
    c.select_job(Some(0));
    c.repair();
    c.presence_changed();
    assert_eq!(log.borrow().announces.len(), 1, "seven requests and one announcement");
    log.borrow_mut().presence = Presence::Live;
    c.presence_changed();
    log.borrow_mut().presence = Presence::Hidden;
    c.next();
    assert_eq!(log.borrow().announces.len(), 2, "a second episode is announced again");
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
// Navigation has one owner at a time: a manual request first cancels what is playing and what is seeking, so that the
// last request a person made is the one that wins. The callbacks of what was cancelled are kept, as ones already on
// their way would be.

#[test]
fn a_seek_back_to_the_committed_slice_cancels_the_seek_that_was_pending() {
    let (c, log) = start(e0());
    c.scheduler().keep_cancelled(true);
    assert_eq!(c.seek(10), Outcome::Pending);
    let token = c.seek_token();
    assert_eq!(c.seek(0), Outcome::AtBoundary, "the committed slice is slice 0");
    assert!(
        c.seek_token() > token,
        "the request that ended at a boundary still replaced the seek"
    );
    let draws = log.borrow().draws.len();
    run_frames(&c);
    assert_eq!(c.committed_slice(), 0, "the earlier seek did not override the later request");
    assert_eq!(log.borrow().draws.len(), draws);
    assert_eq!(c.scheduler().pending_frames(), 0);
}

#[test]
fn restart_and_previous_at_the_first_slice_cancel_a_pending_seek_too() {
    for which in 0..2 {
        let (c, _) = start(e0());
        c.scheduler().keep_cancelled(true);
        c.seek(7);
        let outcome = if which == 0 { c.restart() } else { c.previous() };
        assert_eq!(outcome, Outcome::AtBoundary);
        run_frames(&c);
        assert_eq!(c.committed_slice(), 0, "request {which}");
    }
}

#[test]
fn next_while_playing_stops_playback_and_the_timer_that_was_waiting_does_nothing() {
    let (c, log) = start(generated(3));
    c.scheduler().keep_cancelled(true);
    c.set_speed(100);
    c.play();
    assert_eq!(c.next(), Outcome::Done);
    assert!(!c.is_playing(), "a manual step ends playback");
    assert_eq!(c.scheduler().pending(), 0);
    c.scheduler().advance_by(10_000);
    assert_eq!(
        c.committed_slice(),
        1,
        "the timer of the playback that was stopped did not step again"
    );
    assert_eq!(log.borrow().playing, vec![true, false]);
}

#[test]
fn a_seek_while_playing_stops_playback_and_the_seek_is_the_only_thing_that_moves_the_slice() {
    let (c, _) = start(generated(3));
    c.scheduler().keep_cancelled(true);
    c.set_speed(100);
    c.play();
    c.scheduler().advance_by(100);
    assert_eq!(c.committed_slice(), 1);
    assert_eq!(c.seek(6), Outcome::Pending);
    assert!(!c.is_playing());
    c.scheduler().advance_by(10_000);
    assert_eq!(
        c.committed_slice(),
        1,
        "no tick of the stopped playback moved the slice before the seek arrived"
    );
    run_frames(&c);
    assert_eq!(c.committed_slice(), 6);
    c.scheduler().advance_by(10_000);
    assert_eq!(c.committed_slice(), 6, "and none after it");
}

#[test]
fn play_while_a_seek_is_pending_cancels_the_seek_and_plays_on_from_the_committed_slice() {
    let (c, _) = start(generated(3));
    c.scheduler().keep_cancelled(true);
    c.set_speed(100);
    c.seek(8);
    assert_eq!(c.play(), Outcome::Done);
    run_frames(&c);
    assert_eq!(c.committed_slice(), 0, "the seek's frame, though it still ran, did nothing");
    c.scheduler().advance_by(100);
    assert_eq!(
        c.committed_slice(),
        1,
        "playback went on from the committed slice, one slice a delay"
    );
    c.scheduler().advance_by(100);
    assert_eq!(c.committed_slice(), 2);
}

#[test]
fn a_seek_while_another_is_pending_and_a_next_while_one_is_pending_each_leave_one_owner() {
    let (c, _) = start(generated(3));
    c.scheduler().keep_cancelled(true);
    c.seek(9);
    assert_eq!(c.next(), Outcome::Done, "a step cancels the seek before it");
    run_frames(&c);
    assert_eq!(
        c.committed_slice(),
        1,
        "the seek that was pending did not move the slice after the step"
    );
    assert_eq!(c.scheduler().pending_frames(), 0);
}

#[test]
fn pause_stops_playback_and_leaves_a_pending_seek_alone() {
    let (c, _) = start(generated(3));
    c.set_speed(100);
    c.seek(5);
    c.play();
    // Play cancelled the seek, so ask for it again, and then pause: a pause is not a navigation, and the seek arrives.
    c.seek(5);
    c.pause();
    run_frames(&c);
    assert_eq!(c.committed_slice(), 5, "a pause does not cancel a seek");
    assert!(!c.is_playing());
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
// An engine that fails is not an engine that has finished. The run is made to fail by a limit of five slices, which the
// sixth exceeds.

/// A generated run whose engine refuses the sixth slice.
fn failing_after_five() -> Factory {
    let scenario = Scenario::panel(3).expect("a panel");
    Rc::new(move || {
        Engine::new(
            &scenario.config_id,
            scenario.seed,
            scenario.config,
            &scenario.script,
            scenario.params.as_ref(),
            Limits {
                max_slices: 5,
                ..Limits::default()
            },
        )
    })
}

#[test]
fn an_engine_error_during_a_seek_commits_nothing_and_does_not_end_the_run() {
    for stepped in [0, 3] {
        let factory = failing_after_five();
        let (c, log) = start(Rc::clone(&factory));
        for _ in 0..stepped {
            assert_eq!(c.next(), Outcome::Done);
        }
        let (slice, shown, draws) = (c.committed_slice(), c.committed().canonical(), log.borrow().draws.len());
        assert_eq!(c.seek(9), Outcome::Pending);
        run_frames(&c);
        assert_eq!(c.committed_slice(), slice, "the cursor stayed, from slice {stepped}");
        assert_eq!(c.committed().canonical(), shown, "the committed projection stayed");
        assert_eq!(c.last_slice(), None, "a failure is not the end of the run");
        assert_eq!(
            log.borrow().draws.len(),
            draws,
            "nothing was drawn for a slice that was not reached"
        );
        assert_eq!(page(&log), Some(shown));
        assert_eq!(log.borrow().reports.len(), 1, "the error is reported once");
        assert!(log.borrow().reports[0].contains("limit") || !log.borrow().reports[0].is_empty());
        assert_eq!(c.scheduler().pending_frames(), 0, "the seek is over");
        assert!(!c.is_dirty());
    }
}

#[test]
fn an_engine_error_on_a_step_commits_nothing_and_does_not_end_the_run() {
    let (c, log) = start(failing_after_five());
    for _ in 0..5 {
        assert_eq!(c.next(), Outcome::Done);
    }
    let (shown, draws) = (c.committed().canonical(), log.borrow().draws.len());
    assert_eq!(c.next(), Outcome::Failed);
    assert_eq!((c.committed_slice(), c.last_slice()), (5, None));
    assert_eq!(c.committed().canonical(), shown);
    assert_eq!(log.borrow().draws.len(), draws);
    assert_eq!(log.borrow().reports.len(), 1);
}

#[test]
fn an_engine_error_while_playing_stops_playback_and_keeps_the_last_committed_slice() {
    let (c, log) = start(failing_after_five());
    c.set_speed(10);
    c.play();
    c.scheduler().advance_by(1_000);
    assert_eq!((c.committed_slice(), c.last_slice()), (5, None));
    assert!(!c.is_playing(), "playback stopped at the failure");
    assert_eq!(log.borrow().reports.len(), 1);
    assert_eq!(page(&log), Some(c.committed().canonical()));
}

#[test]
fn a_run_that_comes_to_its_natural_end_is_still_known_to_have_ended() {
    let (c, _) = start(e0());
    c.seek(500);
    run_frames(&c);
    assert_eq!((c.committed_slice(), c.last_slice()), (12, Some(12)));
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
// A selection and a scenario switch are staged and published only after the draw of them succeeded, so that what the
// controller calls committed is always what was shown.

#[test]
fn a_selection_whose_draw_failed_is_not_published_and_a_later_step_does_not_use_it() {
    let factory = e0();
    let (c, log) = start(Rc::clone(&factory));
    c.next();
    c.next();
    let (shown, draws) = (c.committed().canonical(), log.borrow().draws.len());
    // The draw of the selection fails, and so does the repair that follows it.
    log.borrow_mut().fail_at = vec![draws + 1, draws + 2];
    assert_eq!(c.select_job(Some(0)), Outcome::Failed);
    assert_eq!(c.selected(), None, "the selection was not published");
    assert_eq!(
        c.committed().canonical(),
        shown,
        "and the committed projection is the one that was shown"
    );
    assert_eq!(c.committed_slice(), 2);
    // The next slice is drawn without the selection that failed.
    assert_eq!(c.next(), Outcome::Done);
    assert_eq!(c.committed().canonical(), straight(&factory, 3, &View::default()).canonical());
    assert_eq!(page(&log), Some(c.committed().canonical()));
    assert_eq!(c.selected(), None);
    // A selection that is drawn is published, and then it applies.
    assert_eq!(c.select_job(Some(0)), Outcome::Done);
    assert_eq!(c.selected(), Some(0));
}

#[test]
fn a_selection_asked_of_a_host_that_is_not_there_is_not_published() {
    let (c, log) = start(e0());
    c.next();
    log.borrow_mut().presence = Presence::Hidden;
    assert_eq!(c.select_job(Some(0)), Outcome::Unavailable);
    assert_eq!(c.selected(), None);
    log.borrow_mut().presence = Presence::Live;
    c.presence_changed();
    assert_eq!(c.next(), Outcome::Done);
    assert_eq!(
        c.selected(),
        None,
        "the selection that was refused did not come back with the next slice"
    );
}

#[test]
fn a_failed_draw_of_a_selection_changed_in_the_middle_of_a_draw_is_not_published_either() {
    let (c, log) = start(e0());
    c.next();
    let draws = log.borrow().draws.len();
    log.borrow_mut().detach_at = Some(draws + 1);
    assert_eq!(c.select_job(Some(1)), Outcome::Unavailable);
    assert_eq!(c.selected(), None);
}

#[test]
fn a_scenario_switch_whose_draw_failed_leaves_the_old_scenario_committed_and_repaired() {
    let old = e0();
    let (c, log) = start(Rc::clone(&old));
    c.next();
    c.next();
    c.select_job(Some(0));
    let (slice, shown, generation, draws) = (
        c.committed_slice(),
        c.committed().canonical(),
        c.generation(),
        log.borrow().draws.len(),
    );
    // The draw of the new scenario fails, and the repair of the old one that follows it succeeds.
    log.borrow_mut().fail_at = vec![draws + 1];
    assert_eq!(c.switch_scenario(generated(3)).expect("an engine"), Outcome::Failed);
    assert_eq!(c.generation(), generation, "the history was not changed");
    assert_eq!((c.committed_slice(), c.selected()), (slice, Some(0)));
    assert_eq!(c.committed().canonical(), shown, "what is committed is what was shown");
    assert!(!c.is_dirty(), "the page was repaired to the old scenario");
    assert_eq!(page(&log), Some(shown));
    assert!(!c.is_playing(), "a switch is a navigation, so playback had stopped");
    // The old scenario carries on: the next slice is the old run's.
    assert_eq!(c.next(), Outcome::Done);
    assert_eq!(
        c.committed().canonical(),
        straight(&old, 3, &View { selected: Some(0) }).canonical()
    );
}

#[test]
fn a_scenario_switch_asked_of_a_host_that_is_not_there_changes_nothing_and_can_be_asked_again() {
    let (c, log) = start(e0());
    c.next();
    let (shown, generation) = (c.committed().canonical(), c.generation());
    log.borrow_mut().presence = Presence::Hidden;
    assert_eq!(c.switch_scenario(generated(3)).expect("an engine"), Outcome::Unavailable);
    assert_eq!((c.generation(), c.committed().canonical()), (generation, shown));
    log.borrow_mut().presence = Presence::Live;
    c.presence_changed();
    let other = generated(3);
    assert_eq!(c.switch_scenario(Rc::clone(&other)).expect("an engine"), Outcome::Done);
    assert_eq!(c.generation(), generation + 1);
    assert_eq!((c.committed_slice(), c.selected()), (0, None));
    assert_eq!(c.committed().canonical(), straight(&other, 0, &View::default()).canonical());
    assert_eq!(page(&log), Some(c.committed().canonical()));
}

#[test]
fn a_scenario_switch_that_leaves_in_the_middle_of_its_draw_is_not_published() {
    let (c, log) = start(e0());
    c.next();
    let (shown, generation, draws) = (c.committed().canonical(), c.generation(), log.borrow().draws.len());
    log.borrow_mut().detach_at = Some(draws + 1);
    assert_eq!(c.switch_scenario(generated(3)).expect("an engine"), Outcome::Unavailable);
    assert_eq!((c.generation(), c.committed().canonical()), (generation, shown));
    assert!(c.is_dirty());
    log.borrow_mut().presence = Presence::Live;
    c.presence_changed();
    assert_eq!(
        page(&log),
        Some(c.committed().canonical()),
        "the old scenario is drawn again on return"
    );
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[test]
fn a_hide_and_a_show_between_two_ticks_stop_playback_and_it_is_not_resumed() {
    let (c, log) = start(generated(3));
    c.scheduler().keep_cancelled(true);
    c.set_speed(100);
    c.play();
    c.scheduler().advance_by(100);
    assert_eq!(c.committed_slice(), 1);
    // The page is hidden and shown again before the next tick, and each is told as it happens.
    log.borrow_mut().presence = Presence::Hidden;
    c.presence_changed();
    log.borrow_mut().presence = Presence::Live;
    c.presence_changed();
    assert!(!c.is_playing(), "the hide was seen, so playback is stopped");
    c.scheduler().advance_by(10_000);
    assert_eq!(
        c.committed_slice(),
        1,
        "the tick that was already on its way found the epoch moved, and did nothing"
    );
    assert_eq!(log.borrow().playing, vec![true, false]);
}

#[test]
fn suspend_stops_playback_and_seeks_without_asking_whether_the_page_is_visible() {
    let (c, log) = start(generated(3));
    c.scheduler().keep_cancelled(true);
    c.set_speed(100);
    c.play();
    let epoch = c.epoch();
    c.suspend();
    assert!(!c.is_playing());
    assert!(c.epoch() > epoch);
    assert!(!c.is_dirty(), "a page that is being left is not marked dirty");
    c.scheduler().advance_by(10_000);
    assert_eq!(c.committed_slice(), 0);
    assert_eq!(
        log.borrow().presence,
        Presence::Live,
        "the host still reads as live, as it does during pagehide"
    );
    c.seek(5);
    c.suspend();
    run_frames(&c);
    assert_eq!(c.committed_slice(), 0, "a seek that was pending is cancelled too");
    // A page that is shown again stays paused, and playback starts only when asked for.
    c.presence_changed();
    assert!(!c.is_playing());
    assert_eq!(c.play(), Outcome::Done);
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[test]
fn a_boundary_request_whose_repair_fails_is_a_failure_and_not_a_no_op() {
    // A request that has nowhere to go repairs a dirty page first. Four ways to reach a boundary, each with a page made
    // dirty by a failed draw and a repair that fails, and then a repair that is made to fail again by the boundary
    // request itself.
    type Ask = fn(&C) -> Outcome;
    let asks: [(&str, u64, Ask); 4] = [
        ("restart at the first slice", 0, |c| c.restart()),
        ("previous at the first slice", 0, |c| c.previous()),
        ("a seek to the committed slice", 0, |c| c.seek(0)),
        ("next at the last slice", 12, |c| c.next()),
    ];
    for (what, at, ask) in asks {
        let (c, log) = start(e0());
        if at > 0 {
            c.seek(at);
            run_frames(&c);
            assert_eq!(c.next(), Outcome::AtBoundary, "{what}: the last slice is known");
        }
        let draws = log.borrow().draws.len();
        // The draw that makes the page dirty fails, and so does the repair that follows it.
        log.borrow_mut().fail_at = vec![draws + 1, draws + 2];
        c.select_job(Some(0));
        assert!(c.is_dirty(), "{what}: the page is dirty");
        // The repair that the boundary request makes fails too.
        log.borrow_mut().fail_at = vec![draws + 3];
        assert_eq!(ask(&c), Outcome::Failed, "{what}: a failed repair is not a boundary no-op");
        assert!(c.is_dirty(), "{what}: it is still dirty");
        // And when the repair succeeds, it is the no-op that it was.
        assert_eq!(
            ask(&c),
            Outcome::AtBoundary,
            "{what}: a repair that worked leaves a boundary no-op"
        );
        assert!(!c.is_dirty(), "{what}: the page is repaired");
        assert_eq!(page(&log), Some(c.committed().canonical()));
    }
}

#[test]
fn a_boundary_request_to_a_host_that_is_not_there_is_unavailable_and_not_a_failure() {
    let (c, log) = start(e0());
    log.borrow_mut().fail_at = vec![2, 3];
    c.next();
    assert!(c.is_dirty());
    log.borrow_mut().presence = Presence::Hidden;
    assert_eq!(c.restart(), Outcome::Unavailable);
    assert!(c.is_dirty());
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[test]
fn a_forced_chunk_of_zero_is_one_slice_a_frame_and_a_seek_still_makes_progress() {
    let factory = generated(3);
    let (c, _) = start(Rc::clone(&factory));
    c.force_chunk(Some(0));
    c.seek(3);
    // A chunk that did no work would ask for a frame for ever, so the frames are counted and the count is bounded.
    let mut frames = 0;
    while c.scheduler().advance_frame() {
        frames += 1;
        assert!(frames <= 10, "the seek is not making progress");
    }
    assert_eq!(frames, 3, "three slices at one a frame, as for a chunk of one");
    assert_eq!(c.committed_slice(), 3);
    assert_eq!(c.committed().canonical(), straight(&factory, 3, &View::default()).canonical());
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[test]
fn a_retry_that_a_later_navigation_made_obsolete_is_discarded() {
    type Ask = fn(&C) -> Outcome;
    let asks: [(&str, Ask); 5] = [
        ("next", |c| c.next()),
        ("play", |c| c.play()),
        ("restart", |c| c.restart()),
        ("previous", |c| c.previous()),
        ("a seek that works", |c| c.seek(3)),
    ];
    for (what, ask) in asks {
        let (c, _) = start(generated(3));
        c.next();
        c.scheduler().fail_next_frames(1);
        assert_eq!(c.seek(6), Outcome::Failed);
        assert_eq!(c.pending_retry(), Some(6), "{what}: the failed seek offers a retry");
        ask(&c);
        assert_eq!(
            c.pending_retry(),
            None,
            "{what}: a later navigation discarded the retry that it superseded"
        );
        let slice = c.committed_slice();
        c.pause();
        assert_eq!(c.retry(), Outcome::Done, "{what}: there is nothing to retry");
        run_frames(&c);
        assert_ne!(c.committed_slice(), 6, "{what}: the superseded seek was not started");
        let _ = slice;
    }
}

#[test]
fn a_seek_that_fails_in_its_turn_replaces_the_retry_with_its_own_target() {
    let (c, _) = start(generated(3));
    c.scheduler().fail_next_frames(2);
    assert_eq!(c.seek(6), Outcome::Failed);
    assert_eq!(c.seek(3), Outcome::Failed);
    assert_eq!(c.pending_retry(), Some(3), "the retry is the latest request, and not the first");
    assert_eq!(c.retry(), Outcome::Pending);
    run_frames(&c);
    assert_eq!(c.committed_slice(), 3);
    assert_eq!(c.pending_retry(), None);
}

#[test]
fn a_retry_survives_a_hide_since_a_hide_is_not_a_navigation() {
    let (c, log) = start(generated(3));
    c.scheduler().fail_next_frames(1);
    c.seek(5);
    log.borrow_mut().presence = Presence::Hidden;
    c.presence_changed();
    log.borrow_mut().presence = Presence::Live;
    c.presence_changed();
    assert_eq!(c.pending_retry(), Some(5), "the latest request is still the seek that failed");
    assert_eq!(c.retry(), Outcome::Pending);
    run_frames(&c);
    assert_eq!(c.committed_slice(), 5);
}
