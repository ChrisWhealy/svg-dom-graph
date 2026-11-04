//! The rules of every control, the repair of a page that was half written, the coalesced range and its stale frames,
//! the threshold edit and the switch of field, on a surface that records what it is asked and a scheduler that runs a
//! frame only when told to.

use super::*;
use crate::marching_squares::field::LabCase;

#[derive(Default)]
struct Fake {
    draws: RefCell<Vec<(DrawKind, State, Meta)>>,
    said: RefCell<Vec<String>>,
    failures: RefCell<Vec<String>>,
    warnings: RefCell<Vec<String>>,
    syncs: Cell<u32>,
    /// The failures of the next draws, taken in turn.
    draw_failures: RefCell<Vec<CommitError>>,
    fail_sync: Cell<bool>,
}

impl Surface for Fake {
    fn draw(&self, _: &Trace, state: &State, kind: DrawKind, meta: Meta) -> Result<(), CommitError> {
        self.draws.borrow_mut().push((kind, *state, meta));
        let mut failures = self.draw_failures.borrow_mut();
        if failures.is_empty() { Ok(()) } else { Err(failures.remove(0)) }
    }

    fn sync_controls(&self, _: &State) -> Result<(), String> {
        self.syncs.set(self.syncs.get() + 1);
        if self.fail_sync.get() { Err("not written".to_owned()) } else { Ok(()) }
    }

    fn say(&self, text: &str) {
        self.said.borrow_mut().push(text.to_owned());
    }

    fn failed(&self, problem: &str) {
        self.failures.borrow_mut().push(problem.to_owned());
    }

    fn warn(&self, text: &str) {
        self.warnings.borrow_mut().push(text.to_owned());
    }
}

struct Rig {
    controller: Rc<Controller>,
    surface: Rc<Fake>,
    scheduler: Rc<ManualScheduler>,
}

impl Rig {
    fn new() -> Self {
        Self::with(FieldId::Ripple)
    }

    fn with(field: FieldId) -> Self {
        let trace = Trace::build(field.grid().expect("grid"), field.levels()).expect("trace");
        let surface = Rc::new(Fake::default());
        let scheduler = Rc::new(ManualScheduler::default());
        let controller = Controller::new(
            trace,
            State::first(field),
            Rc::clone(&surface) as Rc<dyn Surface>,
            Rc::clone(&scheduler) as Rc<dyn Scheduler>,
        );
        Self { controller, surface, scheduler }
    }

    fn act(&self, action: Action) {
        self.controller.act(action);
    }

    fn step(&self) -> usize {
        self.controller.state().step
    }

    fn said(&self) -> Vec<String> {
        self.surface.said.borrow().clone()
    }

    fn draws(&self) -> usize {
        self.surface.draws.borrow().len()
    }

    fn fail_next(&self, error: CommitError) {
        self.surface.draw_failures.borrow_mut().push(error);
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[test]
fn the_page_starts_at_no_cells_visited() {
    let rig = Rig::new();
    assert_eq!(rig.step(), 0);
    assert!(!rig.controller.is_dirty());
    assert_eq!(rig.draws(), 0, "the first draw is the page's own");
}

#[test]
fn next_and_previous_move_one_cell_and_say_where_it_is() {
    let rig = Rig::new();
    rig.act(Action::Next);
    assert_eq!(rig.step(), 1);
    assert!(rig.said()[0].contains("Cell (0, 0)"), "{:?}", rig.said());
    rig.act(Action::Previous);
    assert_eq!(rig.step(), 0);
    assert!(rig.said()[1].starts_with("No cell selected."), "{:?}", rig.said());
}

#[test]
fn the_ends_are_bounded_and_say_so() {
    let rig = Rig::new();
    rig.act(Action::Previous);
    rig.act(Action::Restart);
    assert_eq!((rig.step(), rig.said()), (0, vec![AT_FIRST.to_owned(), AT_FIRST.to_owned()]));
    assert_eq!(rig.draws(), 0, "a step with nowhere to go draws nothing");
    rig.act(Action::Seek(10_000));
    assert_eq!(rig.step(), 432);
    rig.surface.said.borrow_mut().clear();
    rig.act(Action::Next);
    assert_eq!(rig.said(), vec![AT_LAST.to_owned()]);
}

#[test]
fn restart_and_seek_reach_the_same_state_whatever_came_before() {
    let a = Rig::new();
    a.act(Action::Seek(100));
    a.act(Action::Restart);
    a.act(Action::Seek(200));
    let b = Rig::new();
    b.act(Action::Seek(200));
    assert_eq!(a.controller.state(), b.controller.state());
}

#[test]
fn showing_one_level_or_inspecting_another_changes_only_that() {
    let rig = Rig::new();
    rig.act(Action::Seek(300));
    let before = rig.controller.state();
    rig.act(Action::SetShow(Show::One(2)));
    rig.act(Action::SetInspected(0));
    let after = rig.controller.state();
    assert_eq!(
        (after.step, after.levels, after.field),
        (before.step, before.levels, before.field)
    );
    assert_eq!((after.show, after.inspected), (Show::One(2), 0));
    let draws = rig.draws();
    rig.act(Action::SetShow(Show::One(2)));
    rig.act(Action::SetInspected(0));
    assert_eq!(rig.draws(), draws, "the same choice draws nothing");
    rig.act(Action::SetInspected(7));
    assert_eq!(rig.controller.state().inspected, 0);
}

#[test]
fn choosing_a_cell_is_independent_of_stepping_and_a_cell_out_of_range_is_refused() {
    let rig = Rig::new();
    rig.act(Action::Select(Some((14, 12))));
    assert_eq!(rig.controller.state().selected, Some((14, 12)));
    rig.act(Action::Next);
    rig.act(Action::Seek(300));
    assert_eq!(rig.controller.state().selected, Some((14, 12)), "stepping does not move it");
    rig.act(Action::Select(None));
    assert_eq!(rig.controller.state().selected, None);
    rig.act(Action::Select(Some((18, 0))));
    rig.act(Action::Select(Some((0, 24))));
    assert_eq!(rig.surface.failures.borrow().len(), 2);
    assert_eq!(rig.controller.state().selected, None);
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[test]
fn a_draw_that_fails_before_the_page_is_touched_keeps_the_state_and_the_page_clean() {
    let rig = Rig::new();
    rig.fail_next(CommitError::Detached("no plot".to_owned()));
    rig.act(Action::Next);
    assert_eq!(rig.step(), 0);
    assert!(!rig.controller.is_dirty());
    assert_eq!(rig.surface.failures.borrow().len(), 1);
    rig.act(Action::Next);
    assert_eq!(rig.step(), 1, "the next try works");
}

#[test]
fn a_draw_that_fails_after_the_page_was_touched_leaves_the_state_and_a_dirty_page() {
    let rig = Rig::new();
    rig.act(Action::Seek(5));
    rig.fail_next(CommitError::Live("inspector".to_owned()));
    rig.act(Action::Next);
    assert_eq!((rig.step(), rig.controller.is_dirty()), (5, true));
}

#[test]
fn the_next_action_repairs_the_committed_state_first_even_if_it_has_nowhere_to_go() {
    let rig = Rig::new();
    rig.fail_next(CommitError::Live("x".to_owned()));
    rig.act(Action::Next);
    assert!(rig.controller.is_dirty());
    let draws = rig.draws();
    rig.act(Action::Previous); // at the first step: nowhere to go
    assert_eq!(rig.draws(), draws + 1, "drawn again before the no-op");
    assert!(!rig.controller.is_dirty());
    let (_, repaired, _) = rig.surface.draws.borrow().last().copied().expect("a draw");
    assert_eq!(repaired.step, 0, "the committed state, and not the one that failed");
    assert_eq!(rig.said().last(), Some(&AT_FIRST.to_owned()));
}

#[test]
fn a_repair_that_fails_stops_the_action_and_a_retry_can_try_again() {
    let rig = Rig::new();
    rig.fail_next(CommitError::Live("a".to_owned()));
    rig.act(Action::Next);
    rig.fail_next(CommitError::Live("b".to_owned()));
    rig.act(Action::Next);
    assert_eq!(rig.step(), 0);
    assert!(rig.controller.is_dirty());
    rig.act(Action::Retry);
    assert!(!rig.controller.is_dirty());
    assert_eq!(rig.step(), 0, "a retry only repairs");
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[test]
fn a_new_threshold_builds_a_new_trace_and_keeps_the_view_and_the_step() {
    let rig = Rig::new();
    rig.act(Action::Seek(200));
    let before = rig.controller.meta();
    rig.act(Action::SetThreshold(0.102));
    let state = rig.controller.state();
    assert_eq!((state.levels[1], state.step), (0.102, 200));
    assert_eq!(rig.surface.draws.borrow().last().map(|d| d.0), Some(DrawKind::Step));
    assert_eq!(
        rig.controller.meta().generation,
        before.generation + 1,
        "the trace is a new one"
    );
}

#[test]
fn a_threshold_that_is_refused_leaves_everything_and_puts_the_control_back() {
    let rig = Rig::new();
    rig.act(Action::Seek(200));
    let committed = rig.controller.state();
    let draws = rig.draws();
    // Out of order, equal to another level, not finite, and exactly at a sample.
    for bad in [-0.9, 0.52, f64::NAN, 0.1] {
        rig.act(Action::SetThreshold(bad));
    }
    assert_eq!(rig.controller.state(), committed);
    assert_eq!(rig.draws(), draws, "nothing was drawn");
    assert_eq!(rig.surface.failures.borrow().len(), 4);
    assert_eq!(rig.surface.syncs.get(), 4);
    assert!(
        rig.surface.failures.borrow()[3].contains("exactly at the middle level"),
        "{:?}",
        rig.surface.failures.borrow()
    );
    assert!(!rig.controller.is_dirty());
}

#[test]
fn a_threshold_that_is_refused_and_a_control_that_cannot_be_written_back_leaves_the_page_dirty() {
    let rig = Rig::new();
    rig.surface.fail_sync.set(true);
    rig.act(Action::SetThreshold(0.52));
    assert!(rig.controller.is_dirty());
    rig.act(Action::Retry);
    assert!(!rig.controller.is_dirty());
}

#[test]
fn a_threshold_edit_and_back_restores_the_digest() {
    let rig = Rig::new();
    rig.act(Action::Seek(432));
    let digest = |rig: &Rig| rig.controller.trace().digest(432);
    let first = digest(&rig);
    rig.act(Action::SetThreshold(0.102));
    assert_ne!(digest(&rig), first);
    rig.act(Action::SetThreshold(0.101));
    assert_eq!(digest(&rig), first);
}

#[test]
fn a_new_field_is_shown_at_its_last_step_and_keeps_the_view_only_between_matching_axes() {
    let rig = Rig::new();
    rig.act(Action::SetShow(Show::One(1)));
    rig.act(Action::Seek(50));
    rig.act(Action::SetField(FieldId::Twin));
    let (kind, state, _) = rig.surface.draws.borrow().last().copied().expect("a draw");
    assert_eq!(kind, DrawKind::Scene { keep_view: true }, "both are 25 by 19");
    assert_eq!((state.field, state.step, state.show), (FieldId::Twin, 432, Show::One(1)));
    assert_eq!(state.levels, FieldId::Twin.levels());
    rig.act(Action::SetField(FieldId::Coarse));
    assert_eq!(
        rig.surface.draws.borrow().last().map(|d| d.0),
        Some(DrawKind::Scene { keep_view: false })
    );
    assert_eq!(rig.step(), 48, "8 by 6 cells");
    rig.act(Action::SetField(FieldId::Lab(LabCase::Tie)));
    assert_eq!(
        rig.surface.draws.borrow().last().map(|d| d.0),
        Some(DrawKind::Scene { keep_view: false })
    );
    assert_eq!(rig.step(), 1);
    rig.act(Action::SetField(FieldId::Lab(LabCase::FivePositive)));
    assert_eq!(
        rig.surface.draws.borrow().last().map(|d| d.0),
        Some(DrawKind::Scene { keep_view: true })
    );
}

#[test]
fn a_field_that_cannot_be_drawn_leaves_the_old_one() {
    let rig = Rig::new();
    rig.fail_next(CommitError::Detached("the scene could not be built".to_owned()));
    rig.act(Action::SetField(FieldId::Coarse));
    assert_eq!(rig.controller.state().field, FieldId::Ripple);
    assert_eq!(rig.controller.trace().grid.width, 25, "the trace is the old one too");
    assert!(!rig.controller.is_dirty());
    rig.act(Action::SetField(FieldId::Coarse));
    assert_eq!(rig.controller.state().field, FieldId::Coarse);
}

#[test]
fn a_rebuild_keeps_the_view_the_state_and_says_nothing() {
    let rig = Rig::new();
    rig.act(Action::Seek(120));
    rig.act(Action::Select(Some((3, 4))));
    let state = rig.controller.state();
    let said = rig.said().len();
    let before = rig.controller.meta().generation;
    rig.act(Action::Rebuild);
    assert_eq!(rig.controller.state(), state);
    assert_eq!(
        rig.surface.draws.borrow().last().map(|d| d.0),
        Some(DrawKind::Scene { keep_view: true })
    );
    assert_eq!(rig.said().len(), said);
    assert_eq!(rig.controller.meta().generation, before + 1, "a fresh generation each time");
}

#[test]
fn a_rebuild_that_fails_keeps_the_old_scene_and_the_next_gets_a_fresh_generation() {
    let rig = Rig::new();
    rig.fail_next(CommitError::Detached("the swap failed".to_owned()));
    let before = rig.controller.meta().generation;
    rig.act(Action::Rebuild);
    assert_eq!(rig.controller.meta().generation, before, "nothing was replaced");
    rig.act(Action::Rebuild);
    let metas: Vec<u64> = rig.surface.draws.borrow().iter().map(|d| d.2.generation).collect();
    assert_eq!(
        metas,
        vec![before + 1, before + 1],
        "each attempt is drawn as the generation it would become"
    );
    assert_eq!(rig.controller.meta().generation, before + 1);
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[test]
fn a_burst_of_moves_draws_once_at_the_next_frame_and_says_nothing_until_it_rests() {
    let rig = Rig::new();
    for step in [10, 40, 90, 120] {
        rig.controller.seek_input(step);
    }
    assert_eq!((rig.scheduler.pending_frames(), rig.draws()), (1, 0));
    rig.scheduler.advance_frame();
    assert_eq!((rig.step(), rig.draws(), rig.said().len()), (120, 1, 0));
    rig.controller.seek_release(120);
    assert_eq!(rig.said().len(), 1, "said once when it rests");
}

#[test]
fn letting_go_draws_now_and_drops_what_was_waiting() {
    let rig = Rig::new();
    rig.controller.seek_input(30);
    rig.controller.seek_release(77);
    assert_eq!(rig.step(), 77);
    assert_eq!(rig.scheduler.pending_frames(), 0);
    rig.scheduler.advance_frame();
    assert_eq!(rig.step(), 77);
}

#[test]
fn a_frame_that_was_taken_back_but_arrives_does_not_consume_a_newer_request() {
    let rig = Rig::new();
    rig.scheduler.keep_cancelled();
    rig.controller.seek_input(10); // frame A
    rig.controller.seek_release(10); // takes A back, but it arrives anyway
    rig.controller.seek_input(40); // frame B
    assert_eq!(rig.scheduler.pending_frames(), 2);
    rig.scheduler.advance_frame();
    assert_eq!(rig.step(), 10, "frame A did not draw the newer request");
    rig.scheduler.advance_frame();
    assert_eq!(rig.step(), 40);
}

#[test]
fn a_frame_from_before_a_new_trace_does_nothing() {
    let rig = Rig::new();
    rig.scheduler.keep_cancelled();
    rig.controller.seek_input(200);
    rig.act(Action::SetField(FieldId::Coarse));
    rig.controller.seek_input(5);
    rig.scheduler.advance_frame();
    assert_eq!(rig.step(), 48, "the old frame is neither the generation nor the token");
    rig.scheduler.advance_frame();
    assert_eq!(rig.step(), 5);
}

#[test]
fn a_frame_that_cannot_be_asked_for_draws_the_latest_place_at_once_and_warns() {
    let rig = Rig::new();
    rig.scheduler.fail_next_frames(1);
    rig.controller.seek_input(30);
    assert_eq!(rig.step(), 30);
    assert_eq!(rig.surface.warnings.borrow().len(), 1);
    assert_eq!(rig.scheduler.pending_frames(), 0);
}

#[test]
fn a_move_that_arrives_while_the_page_is_dirty_repairs_it_first() {
    let rig = Rig::new();
    rig.fail_next(CommitError::Live("x".to_owned()));
    rig.act(Action::Next);
    rig.controller.seek_input(60);
    rig.scheduler.advance_frame();
    assert_eq!(rig.step(), 60);
    assert!(!rig.controller.is_dirty());
}

#[test]
fn after_teardown_nothing_has_any_effect() {
    let rig = Rig::new();
    rig.controller.seek_input(9);
    rig.controller.teardown();
    assert_eq!((rig.scheduler.pending_frames(), rig.scheduler.teardowns()), (0, 1));
    rig.act(Action::Next);
    rig.controller.seek_input(3);
    rig.controller.seek_release(3);
    assert_eq!(rig.step(), 0);
}

#[test]
fn releasing_where_it_already_is_still_says_where_it_is() {
    let rig = Rig::new();
    rig.act(Action::Seek(20));
    let said = rig.said().len();
    rig.controller.seek_release(20);
    assert_eq!(rig.said().len(), said + 1);
}
