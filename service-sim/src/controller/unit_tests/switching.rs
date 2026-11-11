//! Selecting a job and changing the scenario, and what a failed draw leaves published.

use super::*;

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
