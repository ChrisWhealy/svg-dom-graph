//! Seeks and retries: which one is current, what replaces it, and what a failure leaves.

use super::*;

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
