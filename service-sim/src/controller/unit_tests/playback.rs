//! Playback: the timer, its speed, pausing, and what stops it.

use super::*;

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

#[test]
fn a_timer_that_cannot_be_scheduled_stops_playback_and_keeps_the_display() {
    let (c, log) = start(e0());
    c.scheduler().fail_next_timers(1);
    assert_eq!(c.play(), Outcome::Failed);
    assert!(!c.is_playing());
    assert_eq!(c.scheduler().pending(), 0, "and no timer waits");
    assert_eq!(log.borrow().reports.len(), 1);
    // Play, and let the second timer fail after a slice was committed.
    c.set_speed(10);
    assert_eq!(c.play(), Outcome::Done);
    c.scheduler().fail_next_timers(1);
    c.scheduler().advance_by(10);
    assert_eq!(c.committed_slice(), 1, "the slice that was drawn stays");
    assert!(!c.is_playing(), "and playback stopped, as there was no timer for the next");
    assert_eq!(c.scheduler().pending(), 0, "and no timer waits");
    assert_eq!(page(&log), Some(c.committed().canonical()));
}

#[test]
fn a_timer_that_cannot_be_scheduled_for_a_new_speed_stops_playback_and_leaves_no_timer() {
    let (c, log) = start(e0());
    c.set_speed(100);
    assert_eq!(c.play(), Outcome::Done);
    assert_eq!(c.scheduler().pending(), 1);
    let epoch = c.epoch();
    c.scheduler().fail_next_timers(1);
    assert_eq!(c.set_speed(50), Outcome::Failed);
    assert!(!c.is_playing(), "playback is not left marked as running without a timer");
    assert_eq!(
        c.scheduler().pending(),
        0,
        "the timer that was taken back is not replaced, and none waits"
    );
    assert!(c.epoch() > epoch, "a callback that was already on its way is stale");
    assert_eq!(c.committed_slice(), 0);
    assert_eq!(log.borrow().reports.len(), 1);
    assert_eq!(
        log.borrow().playing.last(),
        Some(&false),
        "and the host was told that it stopped"
    );
    // Nothing runs on its own afterwards, and Play starts it again.
    c.scheduler().advance_by(1000);
    assert_eq!(c.committed_slice(), 0);
    assert_eq!(c.play(), Outcome::Done);
    assert!(c.is_playing());
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
