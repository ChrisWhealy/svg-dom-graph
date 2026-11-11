//! A failed write or a repair, and what is shown and reported afterwards.

use super::*;

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
// A repair is a draw, and a host can leave in the middle of one as in the middle of any other. Every path that repairs
// has to treat that as the commit does: stop, invalidate, announce once, and say that the host is unavailable, and not
// that the write failed.

#[test]
fn an_explicit_repair_that_the_host_leaves_in_the_middle_of_is_unavailable_and_stops_playback() {
    let (c, log) = start(generated(3));
    c.scheduler().keep_cancelled(true);
    c.set_speed(100);
    c.play();
    let draws = log.borrow().draws.len();
    log.borrow_mut().detach_at = Some(draws + 1);
    assert_eq!(
        c.repair(),
        Outcome::Unavailable,
        "the host is gone, which is not a failed write"
    );
    assert!(!c.is_playing(), "playback stopped at once");
    assert!(c.is_dirty());
    assert_eq!(log.borrow().announces.len(), 1, "one announcement");
    assert!(log.borrow().reports.is_empty(), "and no error");
    assert_eq!(c.scheduler().pending(), 0);
    c.scheduler().advance_by(10_000);
    assert_eq!(c.committed_slice(), 0, "the timer that was waiting did nothing");
    assert_eq!(log.borrow().draws.len(), draws + 1);
}

#[test]
fn a_repair_inside_play_whose_host_leaves_is_unavailable() {
    let (c, log) = start(e0());
    log.borrow_mut().fail_at = vec![2, 3];
    c.next();
    assert!(c.is_dirty());
    log.borrow_mut().detach_at = Some(4);
    assert_eq!(c.play(), Outcome::Unavailable);
    assert!(!c.is_playing());
    assert_eq!(log.borrow().announces.len(), 1);
}

#[test]
fn a_repair_that_a_show_makes_when_the_host_leaves_again_is_a_new_episode_and_is_announced_once() {
    let (c, log) = start(e0());
    log.borrow_mut().fail_at = vec![2, 3];
    c.next();
    log.borrow_mut().presence = Presence::Hidden;
    c.presence_changed();
    log.borrow_mut().presence = Presence::Live;
    let draws = log.borrow().draws.len();
    // The show repairs the page, and the host leaves in the middle of that repair.
    log.borrow_mut().detach_at = Some(draws + 1);
    c.presence_changed();
    assert!(c.is_dirty());
    assert_eq!(
        log.borrow().announces.len(),
        2,
        "the hide was announced, the show began a new episode, and the host leaving in its repair is announced once more"
    );
}

#[test]
fn a_repair_that_works_still_clears_the_dirty_mark_and_never_starts_playback() {
    let (c, log) = start(e0());
    log.borrow_mut().fail_at = vec![2, 3];
    c.next();
    assert_eq!(c.repair(), Outcome::Done);
    assert!(!c.is_dirty() && !c.is_playing());
    assert_eq!(page(&log), Some(c.committed().canonical()));
}
