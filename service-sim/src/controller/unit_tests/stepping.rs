//! Drawing the first slice, and walking, replaying and seeking to others without a timer.

use super::*;

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

#[test]
fn the_engine_that_the_controller_owns_is_observed_directly_and_agrees_with_a_replay() {
    let factory = e0();
    let (c, log) = start(Rc::clone(&factory));
    let reference = |slice: u64| {
        let mut engine = factory().expect("engine");
        for _ in 0..slice {
            engine.step().expect("step").expect("a slice");
        }
        let model = extract(&engine, &View::default());
        (model.clone(), engine.checkpoint(model.tick, model.slice))
    };
    assert_eq!(c.live_engine().sha256, reference(0).1, "at the start");
    c.next();
    c.next();
    let live = c.live_engine();
    assert_eq!((live.recorded_at, live.slice), (2, 2));
    assert_eq!(live.sha256, reference(2).1);
    assert_eq!(c.committed_model(), reference(2).0);
    log.borrow_mut().fail_at = vec![4];
    assert_eq!(c.next(), Outcome::Failed);
    let ahead = c.live_engine();
    assert_eq!(c.committed_slice(), 2, "the failed draw committed nothing");
    assert_eq!(c.committed_model(), reference(2).0, "and left the model as it was");
    assert_eq!(
        (ahead.recorded_at, ahead.slice),
        (3, 3),
        "the engine is one slice ahead, on purpose, and says so"
    );
    assert_eq!(ahead.sha256, reference(3).1, "and what it holds is what a replay holds there");
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
fn a_run_that_comes_to_its_natural_end_is_still_known_to_have_ended() {
    let (c, _) = start(e0());
    c.seek(500);
    run_frames(&c);
    assert_eq!((c.committed_slice(), c.last_slice()), (12, Some(12)));
}
