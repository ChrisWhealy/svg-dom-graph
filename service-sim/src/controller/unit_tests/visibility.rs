//! A host that is hidden, detached, or leaves in the middle of a draw.

use super::*;

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
