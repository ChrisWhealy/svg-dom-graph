//! Slices that were worked out ahead, and when they may be used.

use super::*;

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

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// A factory that counts the engines that it makes, and the shared count.
fn counted(inner: Factory) -> (Factory, Rc<std::cell::Cell<u32>>) {
    let made = Rc::new(std::cell::Cell::new(0_u32));
    let counted: Factory = {
        let made = Rc::clone(&made);
        Rc::new(move || {
            made.set(made.get() + 1);
            inner()
        })
    };
    (counted, made)
}

#[test]
fn slices_prepared_ahead_are_committed_by_next_without_working_them_out_again() {
    let factory = generated(3);
    let (counting, made) = counted(Rc::clone(&factory));
    let (c, log) = start(counting);
    for _ in 0..3 {
        c.next();
    }
    assert_eq!(c.prepare_ahead(4), Ok(4));
    assert_eq!(c.committed_slice(), 3, "preparing commits nothing");
    assert_eq!(
        log.borrow().draws.len(),
        4,
        "and draws nothing: the first draw and three slices"
    );
    let engines = made.get();
    for k in 4..=7 {
        assert_eq!(c.next(), Outcome::Done);
        assert_eq!(c.committed_slice(), k);
        assert_eq!(c.committed().canonical(), straight(&factory, k, &View::default()).canonical());
        assert_eq!(page(&log), Some(c.committed().canonical()));
    }
    assert_eq!(
        made.get(),
        engines,
        "no engine was made, and so no run was replayed to find them"
    );
    assert_eq!(c.live_engine().slice, 7, "the engine stepped once for each, before the draws");
    assert_eq!(c.next(), Outcome::Done, "and the one after them is worked out in the usual way");
    assert_eq!(c.committed().canonical(), straight(&factory, 8, &View::default()).canonical());
    assert_eq!(c.live_engine().sha256, {
        let mut engine = factory().expect("an engine");
        for _ in 0..8 {
            engine.step().expect("a slice");
        }
        let model = extract(&engine, &View::default());
        engine.checkpoint(model.tick, model.slice)
    });
}

#[test]
fn a_slice_prepared_for_one_selection_is_not_used_for_another() {
    let factory = generated(5);
    let (c, _) = start(Rc::clone(&factory));
    c.next();
    assert_eq!(c.prepare_ahead(3), Ok(3));
    let job = c.committed_model().waiting.first().copied().or_else(|| {
        c.committed_model().servers.iter().find_map(|s| match s {
            service_sim_engine::ServerView::Busy { job, .. } => Some(*job),
            _ => None,
        })
    });
    assert!(job.is_some(), "a job to select");
    let selected = View { selected: job };
    assert_eq!(c.select_job(job), Outcome::Done);
    assert_eq!(c.next(), Outcome::Done);
    assert_eq!(
        c.committed().canonical(),
        straight(&factory, 2, &selected).canonical(),
        "the slice that came is drawn for the selection that is committed, and not for the one it was prepared for"
    );
}

#[test]
fn slices_prepared_ahead_do_not_survive_a_change_of_scenario_or_a_move_of_the_cursor() {
    let factory = generated(3);
    let (c, _) = start(Rc::clone(&factory));
    c.prepare_ahead(4).expect("prepared");
    let other = generated(4);
    assert_eq!(c.switch_scenario(Rc::clone(&other)).expect("a scenario"), Outcome::Done);
    assert_eq!(c.next(), Outcome::Done);
    assert_eq!(
        c.committed().canonical(),
        straight(&other, 1, &View::default()).canonical(),
        "the first slice of the new history"
    );
    c.prepare_ahead(4).expect("prepared");
    c.next();
    c.next();
    assert_eq!(c.committed_slice(), 3);
    c.previous();
    run_frames(&c);
    assert_eq!(c.committed_slice(), 2);
    assert_eq!(c.next(), Outcome::Done);
    assert_eq!(c.committed_slice(), 3);
    assert_eq!(c.committed().canonical(), straight(&other, 3, &View::default()).canonical());
    c.seek(6);
    run_frames(&c);
    c.prepare_ahead(2).expect("prepared");
    c.next();
    assert_eq!(
        c.committed().canonical(),
        straight(&other, c.committed_slice(), &View::default()).canonical()
    );
}

#[test]
fn preparing_ahead_at_the_end_of_a_run_makes_fewer_than_asked() {
    let (c, _) = start(e0());
    assert_eq!(c.prepare_ahead(100), Ok(12), "E0 has twelve slices");
    for k in 1..=12 {
        assert_eq!(c.next(), Outcome::Done);
        assert_eq!(c.committed_slice(), k);
    }
    assert_eq!(c.next(), Outcome::AtBoundary);
}

#[test]
fn the_checkpoints_of_slices_prepared_ahead_are_the_engines_own_after_each_slice() {
    let factory = generated(3);
    let (c, _) = start(Rc::clone(&factory));
    c.next();
    let hashes = c.prepare_ahead_with_checkpoints(4).expect("prepared");
    assert_eq!(hashes.len(), 4);
    let mut engine = factory().expect("an engine");
    engine.step().expect("a slice");
    for hash in &hashes {
        let record = engine.step().expect("a slice").expect("a slice that exists");
        assert_eq!(
            hash, &record.checkpoint,
            "the hash after each slice, and not the last four times"
        );
    }
    assert_eq!(
        hashes.iter().collect::<std::collections::HashSet<_>>().len(),
        4,
        "and each is its own"
    );
}
