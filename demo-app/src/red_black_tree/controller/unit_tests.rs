//! The rules of every control, the repair of a page that was half written, the playback and its stale ticks, the keys,
//! and the choice that outlives the key, on a surface that records what it is asked and a scheduler that runs a task
//! only when told to.

use super::*;
use crate::red_black_tree::solver::{Event, Op, Phase};

/// A surface that remembers what it was asked to do, and that fails when it is told to.
#[derive(Default)]
struct Fake {
    commits: RefCell<Vec<usize>>,
    projects: Cell<u32>,
    focused: RefCell<Vec<Key>>,
    said: RefCell<Vec<String>>,
    failures: RefCell<Vec<String>>,
    playback_failures: RefCell<Vec<String>>,
    playing: RefCell<Vec<bool>>,
    /// The failures of the next commits, taken in turn.
    commit_failures: RefCell<Vec<CommitError>>,
    fail_project: Cell<u32>,
    fail_playing: Cell<u32>,
    fail_focus: Cell<u32>,
    /// The number of the call of `project` that fails, counted from one, or zero for none.
    fail_project_call: Cell<u32>,
}

impl Surface for Fake {
    fn commit(&self, _: &Trace, index: usize, _: &View) -> Result<(), CommitError> {
        self.commits.borrow_mut().push(index);
        let mut failures = self.commit_failures.borrow_mut();
        if failures.is_empty() { Ok(()) } else { Err(failures.remove(0)) }
    }

    fn project(&self, _: &Trace, _: usize, _: &View) -> Result<(), String> {
        self.projects.set(self.projects.get() + 1);
        if self.fail_project_call.get() == self.projects.get() {
            return Err("the ring could not be written".to_owned());
        }
        if self.fail_project.get() > 0 {
            self.fail_project.set(self.fail_project.get() - 1);
            return Err("the ring could not be written".to_owned());
        }
        Ok(())
    }

    fn focus_key(&self, key: Key) -> Result<(), String> {
        self.focused.borrow_mut().push(key);
        if self.fail_focus.get() > 0 {
            self.fail_focus.set(self.fail_focus.get() - 1);
            return Err("the focus could not be moved".to_owned());
        }
        Ok(())
    }

    fn playing_changed(&self, playing: bool) -> Result<(), String> {
        self.playing.borrow_mut().push(playing);
        if self.fail_playing.get() > 0 {
            self.fail_playing.set(self.fail_playing.get() - 1);
            return Err("the label could not be written".to_owned());
        }
        Ok(())
    }

    fn say(&self, text: &str) {
        self.said.borrow_mut().push(text.to_owned());
    }

    fn failed(&self, problem: &str) {
        self.failures.borrow_mut().push(problem.to_owned());
    }

    fn playback_failed(&self, problem: &str) {
        self.playback_failures.borrow_mut().push(problem.to_owned());
    }
}

struct Rig {
    controller: Rc<Controller>,
    surface: Rc<Fake>,
    scheduler: Rc<ManualScheduler>,
    trace: Rc<Trace>,
}

impl Rig {
    fn new() -> Self {
        let trace = Rc::new(
            Trace::build(&[
                Op::Insert(10),
                Op::Insert(5),
                Op::Insert(1),
                Op::Insert(15),
                Op::Insert(20),
                Op::Insert(12),
                Op::Delete(15),
            ])
            .expect("valid"),
        );
        let jumps = vec![
            trace
                .find(Phase::Insert, |e| matches!(e, Event::RotateRight(10)))
                .expect("right"),
            trace.find(Phase::Insert, |e| matches!(e, Event::RotateLeft(10))).expect("left"),
            trace
                .find(Phase::Delete, |e| matches!(e, Event::ChooseSuccessor { .. }))
                .expect("choose"),
        ];
        let surface = Rc::new(Fake::default());
        let scheduler = Rc::new(ManualScheduler::default());
        let controller = Controller::new(
            Rc::clone(&trace),
            jumps,
            Rc::clone(&surface) as Rc<dyn Surface>,
            Rc::clone(&scheduler) as Rc<dyn Scheduler>,
        );
        Self {
            controller,
            surface,
            scheduler,
            trace,
        }
    }

    fn act(&self, action: Action) {
        self.controller.act(action);
    }

    fn at(&self) -> usize {
        self.controller.index()
    }

    fn said(&self) -> Vec<String> {
        self.surface.said.borrow().clone()
    }

    fn goto(&self, i: usize) {
        self.act(Action::SeekQuiet(i));
        self.surface.said.borrow_mut().clear();
    }

    fn transplant(&self) -> usize {
        self.trace
            .find(Phase::Delete, |e| matches!(e, Event::Transplant { .. }))
            .expect("transplant")
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[test]
fn the_page_starts_at_the_empty_tree() {
    let rig = Rig::new();
    assert_eq!(rig.at(), 0);
    assert!(!rig.controller.is_dirty() && !rig.controller.is_playing());
}

#[test]
fn next_and_previous_move_one_snapshot_and_say_where_it_is() {
    let rig = Rig::new();
    rig.act(Action::Next);
    assert_eq!(rig.at(), 1);
    assert!(rig.said()[0].starts_with("Step 1 of"), "{:?}", rig.said());
    rig.act(Action::Previous);
    assert_eq!(rig.at(), 0);
}

#[test]
fn the_first_next_goes_from_empty_to_the_first_insertion() {
    let rig = Rig::new();
    rig.act(Action::Next);
    assert_eq!(rig.trace.snapshot(rig.at()).map(|s| s.event.clone()), Some(Event::Insert(10)));
}

#[test]
fn the_ends_are_bounded_and_say_so() {
    let rig = Rig::new();
    rig.act(Action::Previous);
    rig.act(Action::Restart);
    assert_eq!((rig.at(), rig.said()), (0, vec![AT_FIRST.to_owned(), AT_FIRST.to_owned()]));
    let last = rig.trace.last();
    rig.goto(last);
    rig.act(Action::Next);
    assert_eq!((rig.at(), rig.said()), (last, vec![AT_LAST.to_owned()]));
}

#[test]
fn restart_goes_back_to_the_empty_tree() {
    let rig = Rig::new();
    rig.goto(30);
    rig.act(Action::Restart);
    assert_eq!(rig.at(), 0);
}

#[test]
fn a_seek_beyond_the_end_goes_to_the_last_snapshot() {
    let rig = Rig::new();
    rig.act(Action::SeekQuiet(10_000));
    assert_eq!(rig.at(), rig.trace.last());
}

#[test]
fn a_quiet_seek_says_nothing_until_the_range_rests() {
    let rig = Rig::new();
    rig.act(Action::SeekQuiet(12));
    assert_eq!((rig.at(), rig.said().len()), (12, 0));
    rig.controller.seek_release();
    assert_eq!(rig.said().len(), 1);
}

#[test]
fn the_three_jumps_land_on_the_state_after_their_event() {
    let rig = Rig::new();
    let expected = [
        Event::RotateRight(10),
        Event::RotateLeft(10),
        Event::ChooseSuccessor { deleted: 15, successor: 20 },
    ];
    for (n, event) in expected.iter().enumerate() {
        rig.act(Action::Jump(n));
        let s = rig.trace.snapshot(rig.at()).expect("a snapshot");
        assert_eq!(&s.event, event);
    }
    assert_eq!(rig.trace.snapshot(rig.at()).map(|s| s.phase), Some(Phase::Delete));
}

#[test]
fn the_second_left_rotation_is_not_what_the_construction_jump_finds() {
    let rig = Rig::new();
    rig.act(Action::Jump(1));
    assert_eq!(rig.trace.snapshot(rig.at()).map(|s| s.phase), Some(Phase::Insert));
}

#[test]
fn a_jump_to_where_it_already_is_does_nothing_and_an_unknown_jump_does_nothing() {
    let rig = Rig::new();
    rig.act(Action::Jump(0));
    let before = rig.surface.commits.borrow().len();
    rig.act(Action::Jump(0));
    rig.act(Action::Jump(9));
    assert_eq!(rig.surface.commits.borrow().len(), before);
}

#[test]
fn every_action_that_moves_stops_the_playback() {
    for action in [
        Action::Next,
        Action::Previous,
        Action::Restart,
        Action::SeekQuiet(4),
        Action::Jump(0),
        Action::Pause,
    ] {
        let rig = Rig::new();
        rig.goto(3);
        rig.act(Action::Play);
        assert!(rig.controller.is_playing());
        rig.act(action);
        assert!(!rig.controller.is_playing(), "{action:?}");
        assert_eq!(rig.scheduler.pending(), 0, "{action:?}");
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[test]
fn playing_steps_one_snapshot_at_a_tick_and_stops_at_the_end() {
    let rig = Rig::new();
    rig.act(Action::Play);
    assert!(rig.controller.is_playing());
    assert_eq!(*rig.surface.playing.borrow(), vec![true]);
    let mut ticks = 0;
    while rig.scheduler.advance() {
        ticks += 1;
        assert!(ticks <= rig.trace.last(), "it keeps going");
    }
    assert_eq!((rig.at(), rig.controller.is_playing()), (rig.trace.last(), false));
    assert_eq!(ticks, rig.trace.last());
    assert_eq!(*rig.surface.playing.borrow(), vec![true, false]);
}

#[test]
fn playing_announces_only_the_big_moves() {
    let rig = Rig::new();
    rig.act(Action::Play);
    while rig.scheduler.advance() {}
    let said = rig.said();
    assert!(
        !said.is_empty() && said.len() < rig.trace.last(),
        "{} of {}",
        said.len(),
        rig.trace.last()
    );
    let big = rig
        .trace
        .snapshots()
        .iter()
        .filter(|s| announced_while_playing(&s.event))
        .count();
    assert_eq!(said.len(), big);
}

#[test]
fn pausing_announces_the_state_even_if_it_is_a_small_step() {
    let rig = Rig::new();
    rig.act(Action::Play);
    rig.scheduler.advance();
    let small = rig.trace.snapshot(rig.at()).map(|s| s.event.clone());
    assert_eq!(small, Some(Event::Insert(10)));
    rig.surface.said.borrow_mut().clear();
    rig.act(Action::Pause);
    assert_eq!(rig.said().len(), 1);
    rig.act(Action::Pause);
    assert_eq!(rig.said().len(), 1, "pausing what is not playing says nothing");
}

#[test]
fn play_at_the_end_says_so_and_does_not_start() {
    let rig = Rig::new();
    rig.goto(rig.trace.last());
    rig.act(Action::Play);
    assert!(!rig.controller.is_playing());
    assert_eq!(rig.said(), vec![AT_LAST.to_owned()]);
}

#[test]
fn a_second_play_does_not_queue_a_second_tick() {
    let rig = Rig::new();
    rig.act(Action::Play);
    rig.act(Action::Play);
    assert_eq!(rig.scheduler.pending(), 1);
}

#[test]
fn a_tick_that_survives_cancelling_does_nothing() {
    let rig = Rig::new();
    rig.scheduler.keep_cancelled();
    rig.act(Action::Play);
    rig.act(Action::Pause);
    assert_eq!(rig.scheduler.pending(), 1, "the stale tick is still on its way");
    rig.scheduler.advance();
    assert_eq!(rig.at(), 0);
    assert_eq!(rig.scheduler.pending(), 0, "and it asked for no tick of its own");
}

#[test]
fn a_tick_that_cannot_be_scheduled_stops_the_playback_and_says_so() {
    let rig = Rig::new();
    rig.act(Action::Play);
    rig.scheduler.fail_next(1);
    rig.scheduler.advance();
    assert!(!rig.controller.is_playing());
    assert_eq!(rig.surface.playback_failures.borrow().len(), 1);
    assert!(!rig.controller.is_dirty(), "the page is right");
}

#[test]
fn a_first_tick_that_cannot_be_scheduled_does_not_claim_to_play() {
    let rig = Rig::new();
    rig.scheduler.fail_next(1);
    rig.act(Action::Play);
    assert!(!rig.controller.is_playing());
    assert!(rig.surface.playing.borrow().is_empty());
    assert_eq!(rig.surface.playback_failures.borrow().len(), 1);
}

#[test]
fn a_play_whose_label_could_not_be_written_does_not_start() {
    let rig = Rig::new();
    rig.surface.fail_playing.set(1);
    rig.act(Action::Play);
    assert!(!rig.controller.is_playing());
    assert_eq!(rig.scheduler.pending(), 0);
    assert_eq!(*rig.surface.playing.borrow(), vec![true, false]);
    assert!(!rig.controller.is_dirty());
    rig.surface.fail_playing.set(2);
    rig.act(Action::Play);
    assert!(rig.controller.is_dirty(), "the label could not be written back either");
}

#[test]
fn a_pause_whose_label_could_not_be_written_leaves_the_page_dirty_and_a_no_op_repairs_it() {
    let rig = Rig::new();
    rig.act(Action::Play);
    rig.surface.fail_playing.set(1);
    rig.act(Action::Pause);
    assert!(rig.controller.is_dirty());
    let before = rig.surface.commits.borrow().len();
    rig.act(Action::Previous);
    assert_eq!(
        rig.surface.commits.borrow().len(),
        before + 1,
        "committed again before the no-op"
    );
    assert!(!rig.controller.is_dirty());
    assert_eq!(rig.said().last(), Some(&AT_FIRST.to_owned()));
}

#[test]
fn after_teardown_nothing_has_any_effect() {
    let rig = Rig::new();
    rig.act(Action::Play);
    rig.controller.teardown();
    assert_eq!((rig.scheduler.pending(), rig.scheduler.teardowns()), (0, 1));
    rig.act(Action::Next);
    assert_eq!(rig.at(), 0);
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[test]
fn a_commit_that_fails_before_the_page_is_touched_keeps_the_index_and_the_page_clean() {
    let rig = Rig::new();
    rig.goto(5);
    rig.surface
        .commit_failures
        .borrow_mut()
        .push(CommitError::Detached("no circle".to_owned()));
    rig.act(Action::Next);
    assert_eq!(rig.at(), 5);
    assert!(!rig.controller.is_dirty());
    assert_eq!(rig.surface.failures.borrow().len(), 1);
    rig.act(Action::Next);
    assert_eq!(rig.at(), 6, "the next try works");
}

#[test]
fn a_commit_that_fails_after_the_page_was_touched_leaves_the_index_and_a_dirty_page() {
    let rig = Rig::new();
    rig.goto(5);
    rig.surface
        .commit_failures
        .borrow_mut()
        .push(CommitError::Live("focus failed".to_owned()));
    rig.act(Action::Next);
    assert_eq!((rig.at(), rig.controller.is_dirty()), (5, true));
}

#[test]
fn the_next_action_repairs_a_dirty_page_at_the_index_it_was_at() {
    let rig = Rig::new();
    rig.goto(5);
    rig.surface.commit_failures.borrow_mut().push(CommitError::Live("x".to_owned()));
    rig.act(Action::Next);
    rig.act(Action::Next);
    assert_eq!(*rig.surface.commits.borrow().last().expect("commits"), 6);
    assert_eq!(
        rig.surface.commits.borrow()[rig.surface.commits.borrow().len() - 2],
        5,
        "repaired first"
    );
    assert!(!rig.controller.is_dirty());
}

#[test]
fn a_repair_that_fails_stops_the_action_that_asked_for_it() {
    let rig = Rig::new();
    rig.goto(5);
    rig.surface
        .commit_failures
        .borrow_mut()
        .extend([CommitError::Live("a".to_owned()), CommitError::Live("b".to_owned())]);
    rig.act(Action::Next);
    rig.act(Action::Next);
    assert_eq!(rig.at(), 5);
    assert!(rig.controller.is_dirty());
}

#[test]
fn a_failed_step_while_playing_stops_the_playback() {
    let rig = Rig::new();
    rig.act(Action::Play);
    rig.surface
        .commit_failures
        .borrow_mut()
        .push(CommitError::Detached("x".to_owned()));
    rig.scheduler.advance();
    assert!(!rig.controller.is_playing());
    assert_eq!(rig.at(), 0);
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[test]
fn choosing_a_key_remembers_the_choice_and_moves_the_tab_stop_there() {
    let rig = Rig::new();
    rig.goto(rig.trace.last());
    rig.act(Action::Choose(12));
    assert_eq!(
        rig.controller.view(),
        View {
            chosen: Some(12),
            roving: Some(12)
        }
    );
    assert!(rig.said()[0].contains("12, red"), "{:?}", rig.said());
    assert_eq!(rig.surface.projects.get(), 1);
}

#[test]
fn a_key_that_is_not_in_the_tree_cannot_be_chosen() {
    let rig = Rig::new();
    rig.goto(rig.trace.last());
    rig.act(Action::Choose(15));
    assert_eq!(rig.controller.view(), View::default());
}

#[test]
fn the_choice_survives_every_step_and_seek() {
    let rig = Rig::new();
    rig.goto(rig.transplant() - 1);
    rig.act(Action::Choose(15));
    for target in [rig.transplant(), rig.trace.last(), 0, 3, rig.transplant() - 1] {
        rig.act(Action::SeekQuiet(target));
        assert_eq!(rig.controller.view().chosen, Some(15), "at {target}");
    }
}

#[test]
fn playing_through_the_deletion_does_not_change_the_choice() {
    let rig = Rig::new();
    rig.goto(rig.transplant() - 1);
    rig.act(Action::Choose(15));
    rig.act(Action::Play);
    while rig.scheduler.advance() {}
    assert_eq!(
        rig.controller.view(),
        View {
            chosen: Some(15),
            roving: Some(15)
        }
    );
}

#[test]
fn the_keys_move_the_tab_stop_and_the_focus() {
    let rig = Rig::new();
    rig.goto(rig.trace.last());
    // The tab stop starts at the root, 5, whose left child is 1 and whose right child is 12.
    rig.act(Action::Navigate(Nav::Right));
    assert_eq!(rig.controller.view().roving, Some(1));
    rig.act(Action::Navigate(Nav::Down));
    assert_eq!(rig.controller.view().roving, Some(5));
    rig.act(Action::Navigate(Nav::Down));
    assert_eq!(rig.controller.view().roving, Some(10));
    assert_eq!(*rig.surface.focused.borrow(), vec![1, 5, 10]);
    assert_eq!(rig.controller.view().chosen, None, "moving never selects");
}

#[test]
fn a_key_that_goes_nowhere_does_nothing() {
    let rig = Rig::new();
    rig.goto(rig.trace.last());
    // The tab stop is on the root, which has no parent.
    rig.act(Action::Navigate(Nav::Left));
    assert!(rig.surface.focused.borrow().is_empty());
    assert_eq!(rig.controller.view().roving, None);
    rig.act(Action::Navigate(Nav::End));
    let projects = rig.surface.projects.get();
    rig.act(Action::Navigate(Nav::Down));
    rig.act(Action::Navigate(Nav::End));
    assert_eq!(*rig.surface.focused.borrow(), vec![20], "only the first End moved");
    assert_eq!(rig.surface.projects.get(), projects, "and the others wrote nothing");
}

#[test]
fn a_key_on_an_empty_tree_does_nothing() {
    let rig = Rig::new();
    rig.act(Action::Navigate(Nav::Down));
    assert!(rig.surface.focused.borrow().is_empty());
}

#[test]
fn a_focus_that_lands_on_the_tab_stop_changes_nothing_and_one_elsewhere_moves_it() {
    let rig = Rig::new();
    rig.goto(rig.trace.last());
    rig.act(Action::FocusMoved(5));
    assert_eq!(
        (rig.controller.view().roving, rig.surface.projects.get()),
        (None, 0),
        "Tab lands on the root"
    );
    rig.act(Action::FocusMoved(10));
    assert_eq!(rig.controller.view().roving, Some(10));
}

#[test]
fn a_focus_that_lands_on_a_replacement_does_not_forget_the_key_it_replaced() {
    let rig = Rig::new();
    rig.goto(rig.transplant() - 1);
    rig.act(Action::Choose(15));
    rig.act(Action::SeekQuiet(rig.trace.last()));
    // 15 is gone, so the tab stop is on 20. Tab lands there.
    rig.act(Action::FocusMoved(20));
    assert_eq!(rig.controller.view().roving, Some(15), "the intent stays 15");
}

#[test]
fn focus_moves_do_not_repair_or_rebuild() {
    let rig = Rig::new();
    rig.goto(rig.trace.last());
    let commits = rig.surface.commits.borrow().len();
    rig.act(Action::FocusMoved(10));
    assert_eq!(rig.surface.commits.borrow().len(), commits);
}

#[test]
fn a_ring_that_could_not_be_written_leaves_the_page_dirty_for_the_next_action() {
    let rig = Rig::new();
    rig.goto(rig.trace.last());
    rig.surface.fail_project.set(1);
    rig.act(Action::Choose(12));
    assert!(rig.controller.is_dirty());
    rig.act(Action::Previous);
    assert!(!rig.controller.is_dirty());
}

#[test]
fn a_focus_that_did_not_move_puts_the_tab_stop_back_and_reports_it() {
    let rig = Rig::new();
    rig.goto(rig.trace.last());
    rig.act(Action::Navigate(Nav::Right)); // from the root 5 to 1
    assert_eq!(rig.controller.view().roving, Some(1));
    rig.surface.fail_focus.set(1);
    let projects = rig.surface.projects.get();
    rig.act(Action::Navigate(Nav::Down)); // 1 to 5 cannot take the focus
    assert_eq!(
        rig.controller.view().roving,
        Some(1),
        "the tab stop is where the focus really is"
    );
    assert_eq!(rig.surface.projects.get(), projects + 2, "written, and then written back");
    assert_eq!(rig.surface.failures.borrow().len(), 1);
    assert!(!rig.controller.is_dirty());
    // The next key moves from where the focus really is.
    rig.act(Action::Navigate(Nav::Down));
    assert_eq!(rig.controller.view().roving, Some(5));
    assert_eq!(*rig.surface.focused.borrow(), vec![1, 5, 5]);
}

#[test]
fn a_focus_that_did_not_move_and_a_tab_stop_that_cannot_be_written_back_leaves_the_page_dirty() {
    let rig = Rig::new();
    rig.goto(rig.trace.last());
    rig.surface.fail_focus.set(1);
    // The first write of the tab stop works, and the one that puts it back is the next call.
    rig.surface.fail_project_call.set(rig.surface.projects.get() + 2);
    rig.act(Action::Navigate(Nav::Right));
    assert_eq!(rig.controller.view().roving, None, "the intent is back where it was");
    assert!(rig.controller.is_dirty(), "but what the page shows may not be");
    rig.act(Action::Previous);
    assert!(!rig.controller.is_dirty(), "the next action commits the page again");
}

#[test]
fn choosing_a_key_moves_the_focus_to_it_even_if_another_key_has_it() {
    let rig = Rig::new();
    rig.goto(rig.trace.last());
    rig.act(Action::Navigate(Nav::Right)); // the focus is on 1
    rig.act(Action::Choose(12));
    assert_eq!(rig.surface.focused.borrow().last(), Some(&12));
    assert_eq!(
        rig.controller.view(),
        View {
            chosen: Some(12),
            roving: Some(12)
        }
    );
    // The next key goes on from the key that was chosen.
    rig.act(Action::Navigate(Nav::Up));
    assert_eq!(rig.controller.view().roving, Some(10));
}

#[test]
fn a_choice_whose_focus_cannot_move_keeps_the_choice_and_the_tab_stop_where_the_focus_is() {
    let rig = Rig::new();
    rig.goto(rig.trace.last());
    rig.act(Action::Navigate(Nav::Right)); // the focus is on 1
    rig.surface.fail_focus.set(1);
    rig.act(Action::Choose(12));
    assert_eq!(
        rig.controller.view(),
        View {
            chosen: Some(12),
            roving: Some(1)
        }
    );
    assert_eq!(rig.surface.failures.borrow().len(), 1);
    assert!(!rig.controller.is_dirty());
    rig.act(Action::Navigate(Nav::Down));
    assert_eq!(rig.controller.view().roving, Some(5), "from 1, where the focus is");
}
