//! The rules of every button, the repair of a page that was half written, and the playback, on a surface that records
//! what it is asked to do and a scheduler that runs a task only when it is told to.

use super::*;
use crate::n_queens::projection::{Kind, project};

/// A surface that remembers what it was asked to draw and say, and that fails when it is told to.
#[derive(Default)]
struct Fake {
    /// Each draw as the index and whether it was a full one.
    draws: RefCell<Vec<(usize, bool)>>,
    said: RefCell<Vec<String>>,
    failures: RefCell<Vec<String>>,
    playing: RefCell<Vec<bool>>,
    rebuilds: RefCell<Vec<usize>>,
    /// How many of the next draws fail.
    fail_draws: Cell<u32>,
    fail_rebuild: Cell<bool>,
    scheduler: RefCell<Option<Rc<ManualScheduler>>>,
    /// How many ticks were waiting at each draw, which must be none, since a tick is asked for only after a draw.
    waiting_at_draw: RefCell<Vec<usize>>,
}

impl Surface for Fake {
    fn draw(&self, _: &Trace, index: usize, full: bool) -> Result<(), String> {
        if let Some(scheduler) = self.scheduler.borrow().as_ref() {
            self.waiting_at_draw.borrow_mut().push(scheduler.pending());
        }
        self.draws.borrow_mut().push((index, full));
        if self.fail_draws.get() > 0 {
            self.fail_draws.set(self.fail_draws.get() - 1);
            return Err("the write failed".to_owned());
        }
        Ok(())
    }

    fn rebuild(&self, trace: &Trace) -> Result<(), String> {
        if self.fail_rebuild.get() {
            return Err("the board could not be made".to_owned());
        }
        self.rebuilds.borrow_mut().push(trace.size());
        Ok(())
    }

    fn playing_changed(&self, playing: bool) {
        self.playing.borrow_mut().push(playing);
    }

    fn say(&self, text: &str) {
        self.said.borrow_mut().push(text.to_owned());
    }

    fn failed(&self, problem: &str) {
        self.failures.borrow_mut().push(problem.to_owned());
    }
}

struct Rig {
    controller: Rc<Controller>,
    surface: Rc<Fake>,
    scheduler: Rc<ManualScheduler>,
}

impl Rig {
    fn new(n: usize) -> Self {
        let surface = Rc::new(Fake::default());
        let scheduler = Rc::new(ManualScheduler::default());
        *surface.scheduler.borrow_mut() = Some(Rc::clone(&scheduler));
        let trace = Trace::build(n).expect("a trace");
        let controller = Controller::new(trace, Rc::clone(&surface) as Rc<dyn Surface>, Rc::clone(&scheduler) as _, 20);
        Self { controller, surface, scheduler }
    }

    fn act(&self, action: Action) {
        self.controller.act(action);
    }

    fn at(&self) -> usize {
        self.controller.selected()
    }

    fn last(&self) -> usize {
        self.controller.trace().len()
    }

    fn draws(&self) -> usize {
        self.surface.draws.borrow().len()
    }

    fn last_draw(&self) -> Option<(usize, bool)> {
        self.surface.draws.borrow().last().copied()
    }

    fn said(&self) -> Vec<String> {
        self.surface.said.borrow().clone()
    }

    /// Fails the next `count` draws, and moves to the snapshot `to` with a Seek, so the page is left dirty.
    fn make_dirty(&self, to: usize) {
        self.surface.fail_draws.set(1);
        self.act(Action::Seek(to));
        assert!(self.controller.is_dirty());
    }

    /// Runs ticks until playback stops, or `limit` have run, and says how many ran.
    fn play_out(&self, limit: usize) -> usize {
        let mut ran = 0;
        while ran < limit && self.scheduler.advance() {
            ran += 1;
        }
        ran
    }
}

// The first draw.

#[test]
fn the_first_snapshot_is_drawn_in_full_when_the_controller_is_made() {
    let rig = Rig::new(4);
    assert_eq!(rig.surface.draws.borrow().as_slice(), &[(0, true)]);
    assert_eq!(
        (rig.at(), rig.controller.is_dirty(), rig.controller.is_playing()),
        (0, false, false)
    );
}

// One step.

#[test]
fn next_and_previous_move_one_snapshot_and_a_round_trip_returns() {
    let rig = Rig::new(4);
    rig.act(Action::Next);
    assert_eq!(rig.at(), 1);
    rig.act(Action::Next);
    rig.act(Action::Previous);
    assert_eq!(rig.at(), 1);
    rig.act(Action::Previous);
    assert_eq!(rig.at(), 0);
}

#[test]
fn restart_goes_to_the_empty_board() {
    let rig = Rig::new(4);
    rig.act(Action::Seek(30));
    rig.act(Action::Restart);
    assert_eq!(rig.at(), 0);
}

#[test]
fn a_step_that_changes_the_snapshot_draws_only_what_differs() {
    let rig = Rig::new(4);
    rig.act(Action::Next);
    assert_eq!(rig.last_draw(), Some((1, false)));
}

#[test]
fn previous_at_the_start_and_next_at_the_end_move_nothing_and_say_so() {
    let rig = Rig::new(4);
    rig.act(Action::Previous);
    rig.act(Action::Restart);
    assert_eq!((rig.at(), rig.draws()), (0, 1), "nothing was drawn");
    assert_eq!(rig.said(), vec![AT_FIRST.to_owned(), AT_FIRST.to_owned()]);
    rig.act(Action::Seek(usize::MAX));
    assert_eq!(rig.at(), rig.last());
    let before = rig.draws();
    rig.act(Action::Next);
    assert_eq!((rig.at(), rig.draws()), (rig.last(), before));
    assert_eq!(rig.said().last().map(String::as_str), Some(AT_LAST));
}

#[test]
fn a_step_is_announced_once_in_words() {
    let rig = Rig::new(4);
    rig.act(Action::Next);
    let said = rig.said();
    assert_eq!(said.len(), 1);
    assert!(said[0].starts_with("Step 1 of"), "{said:?}");
}

// Seeking.

#[test]
fn a_seek_goes_straight_to_the_snapshot_without_drawing_those_between() {
    let rig = Rig::new(8);
    rig.act(Action::Seek(20_000));
    assert_eq!(rig.at(), 20_000);
    assert_eq!(rig.draws(), 2, "the first snapshot, and the one it went to");
}

#[test]
fn a_seek_past_the_end_goes_to_the_last_snapshot_and_a_seek_is_not_announced() {
    let rig = Rig::new(4);
    rig.act(Action::Seek(1_000_000));
    assert_eq!(rig.at(), rig.last());
    assert!(
        rig.said().is_empty(),
        "the range announces itself, and the page speaks when it comes to rest"
    );
}

#[test]
fn a_seek_to_where_it_is_draws_nothing_and_says_nothing() {
    let rig = Rig::new(4);
    rig.act(Action::Seek(0));
    assert_eq!((rig.draws(), rig.said().len()), (1, 0));
}

#[test]
fn the_seek_request_keeps_the_latest_and_asks_for_one_frame() {
    let request = SeekRequest::default();
    assert!(request.request(5), "the first request needs a frame");
    assert!(!request.request(9) && !request.request(7), "later ones wait for the same frame");
    assert_eq!(request.take(), Some(7));
    assert_eq!(request.take(), None);
    assert!(request.request(1), "after a frame, the next request needs another");
}

#[test]
fn after_a_seek_that_comes_to_rest_the_page_says_where_it_is() {
    let rig = Rig::new(4);
    rig.act(Action::Seek(10));
    rig.controller.announce_current();
    assert_eq!(rig.said().len(), 1);
    assert!(rig.said()[0].starts_with("Step 10 of"));
}

// Solutions.

#[test]
fn the_solution_buttons_move_strictly_between_the_solutions() {
    let rig = Rig::new(4);
    let trace = rig.controller.trace();
    let [first, second] = [trace.solution_indices()[0], trace.solution_indices()[1]];
    rig.act(Action::NextSolution);
    assert_eq!(rig.at(), first);
    rig.act(Action::NextSolution);
    assert_eq!(rig.at(), second, "from a solution to the next, not to itself");
    rig.act(Action::PreviousSolution);
    assert_eq!(rig.at(), first);
}

#[test]
fn a_solution_button_with_nowhere_to_go_does_nothing_and_says_so() {
    let rig = Rig::new(4);
    rig.act(Action::PreviousSolution);
    assert_eq!(rig.at(), 0);
    assert_eq!(rig.said().last().map(String::as_str), Some(NO_EARLIER_SOLUTION));
    rig.act(Action::Seek(rig.last()));
    rig.act(Action::NextSolution);
    assert_eq!(rig.at(), rig.last());
    assert_eq!(rig.said().last().map(String::as_str), Some(NO_LATER_SOLUTION));
}

#[test]
fn every_solution_of_eight_is_reached_by_jumping_and_lands_on_a_solution() {
    let rig = Rig::new(8);
    let mut jumps = 0;
    loop {
        let before = rig.at();
        rig.act(Action::NextSolution);
        if rig.at() == before {
            break;
        }
        jumps += 1;
        let view = project(&rig.controller.trace(), rig.at()).expect("a view");
        assert_eq!((view.kind, view.ordinal), (Kind::Solution, Some(jumps)));
    }
    assert_eq!(jumps, 92);
}

// A page that is half written.

#[test]
fn a_draw_that_fails_leaves_the_index_where_it_was_and_marks_the_page_dirty() {
    let rig = Rig::new(4);
    rig.act(Action::Next);
    rig.surface.fail_draws.set(1);
    rig.act(Action::Next);
    assert_eq!(rig.at(), 1, "the index did not move");
    assert!(rig.controller.is_dirty());
    assert_eq!(rig.surface.failures.borrow().len(), 1);
}

#[test]
fn the_next_action_draws_the_selected_snapshot_in_full_before_it_moves() {
    let rig = Rig::new(4);
    rig.act(Action::Next);
    rig.surface.fail_draws.set(1);
    rig.act(Action::Next);
    rig.act(Action::Next);
    let draws = rig.surface.draws.borrow().clone();
    let n = draws.len();
    assert_eq!(&draws[n - 2..], &[(1, true), (2, false)], "repair, then the step");
    assert_eq!((rig.at(), rig.controller.is_dirty()), (2, false));
}

#[test]
fn a_failed_repair_keeps_the_page_dirty_and_the_index_and_moves_nothing() {
    let rig = Rig::new(4);
    rig.make_dirty(5);
    rig.surface.fail_draws.set(1);
    rig.act(Action::Next);
    assert_eq!((rig.at(), rig.controller.is_dirty()), (0, true));
    assert_eq!(
        rig.last_draw(),
        Some((0, true)),
        "it tried to draw the selected snapshot, and not a step"
    );
    rig.act(Action::Next);
    assert_eq!(
        (rig.at(), rig.controller.is_dirty()),
        (1, false),
        "the next try repairs, and then it steps"
    );
}

#[test]
fn every_action_that_has_nowhere_to_go_still_repairs_a_dirty_page() {
    let boundary = [
        ("previous at the start", Action::Previous, 0),
        ("restart at the start", Action::Restart, 0),
        ("next at the end", Action::Next, usize::MAX),
        ("play at the end", Action::Play, usize::MAX),
        ("a seek to where it is", Action::Seek(0), 0),
        ("previous solution with none before", Action::PreviousSolution, 0),
        ("next solution with none after", Action::NextSolution, usize::MAX),
    ];
    for (name, action, start) in boundary {
        let rig = Rig::new(4);
        let at = start.min(rig.last());
        if at > 0 {
            rig.act(Action::Seek(at));
        }
        rig.surface.fail_draws.set(1);
        // A step that fails marks the page dirty without moving, so go via a seek to the other end and back.
        let other = if at == 0 { 1 } else { at - 1 };
        rig.act(Action::Seek(other));
        assert!(rig.controller.is_dirty(), "{name}");
        rig.act(Action::Seek(at));
        let seek_target = rig.at();
        assert_eq!(seek_target, at, "{name}: the seek repaired and then moved");
        // Now dirty again, with the index at `at`, and the action under test has nowhere to go.
        rig.surface.fail_draws.set(1);
        let draws = rig.draws();
        let probe = if at == 0 { Action::Seek(1) } else { Action::Seek(at - 1) };
        rig.act(probe);
        assert!(rig.controller.is_dirty(), "{name}");
        assert_eq!(rig.at(), at, "{name}");
        let before = rig.draws();
        assert!(before > draws);
        rig.act(action);
        assert!(!rig.controller.is_dirty(), "{name}: repaired");
        assert_eq!(rig.at(), at, "{name}: and did not move");
        assert_eq!(
            rig.last_draw(),
            Some((at, true)),
            "{name}: by drawing the selected snapshot in full"
        );
        assert!(!rig.controller.is_playing(), "{name}");
    }
}

#[test]
fn a_dirty_page_at_the_last_snapshot_is_repaired_by_next_and_the_index_stays() {
    let rig = Rig::new(4);
    let last = rig.last();
    rig.act(Action::Seek(last));
    rig.make_dirty(last - 1);
    // The failed seek left the index at `last` and the page dirty.
    assert_eq!(rig.at(), last);
    rig.act(Action::Next);
    assert_eq!((rig.at(), rig.controller.is_dirty()), (last, false));
    assert_eq!(rig.last_draw(), Some((last, true)));
}

#[test]
fn play_does_not_start_on_a_page_that_cannot_be_repaired() {
    let rig = Rig::new(4);
    rig.make_dirty(3);
    rig.surface.fail_draws.set(1);
    rig.act(Action::Play);
    assert!(!rig.controller.is_playing());
    assert_eq!(rig.scheduler.pending(), 0);
}

// Playing.

#[test]
fn play_waits_for_a_tick_that_is_one_over_the_speed_in_milliseconds() {
    let rig = Rig::new(4);
    rig.act(Action::Play);
    assert!(rig.controller.is_playing());
    assert_eq!((rig.scheduler.pending(), rig.scheduler.delay()), (1, Some(50)), "20 a second");
    assert_eq!(rig.surface.playing.borrow().as_slice(), &[true]);
}

#[test]
fn each_tick_shows_one_snapshot_and_then_asks_for_the_next_so_that_only_one_is_ever_waiting() {
    let rig = Rig::new(4);
    rig.act(Action::Play);
    for expected in 1..=10 {
        assert_eq!(rig.scheduler.pending(), 1);
        assert!(rig.scheduler.advance());
        assert_eq!(rig.at(), expected);
    }
    assert!(
        rig.surface.waiting_at_draw.borrow().iter().skip(1).all(|&waiting| waiting == 0),
        "a draw never overlaps"
    );
}

#[test]
fn playing_reaches_the_end_stops_there_and_asks_for_nothing_more() {
    for n in [4, 8, 9] {
        let rig = Rig::new(n);
        rig.act(Action::Play);
        let ticks = rig.play_out(1_000_000);
        assert_eq!(ticks, rig.last(), "N={n}");
        assert_eq!(rig.at(), rig.last(), "N={n}");
        assert!(!rig.controller.is_playing(), "N={n}");
        assert_eq!(rig.scheduler.pending(), 0, "N={n}");
        assert_eq!(rig.surface.playing.borrow().as_slice(), &[true, false], "N={n}");
        let view = project(&rig.controller.trace(), rig.at()).expect("a view");
        assert_eq!(view.kind, Kind::Exhausted);
    }
}

#[test]
fn play_at_the_end_does_nothing_and_does_not_start_again() {
    let rig = Rig::new(4);
    rig.act(Action::Seek(usize::MAX));
    rig.act(Action::Play);
    assert!(!rig.controller.is_playing());
    assert_eq!((rig.scheduler.pending(), rig.at()), (0, rig.last()));
    assert_eq!(rig.said().last().map(String::as_str), Some(AT_LAST));
}

#[test]
fn play_while_playing_does_not_start_a_second_loop() {
    let rig = Rig::new(4);
    rig.act(Action::Play);
    rig.act(Action::Play);
    assert_eq!(rig.scheduler.pending(), 1);
}

#[test]
fn every_action_but_pause_stops_playback_and_takes_back_the_tick() {
    let actions = [
        Action::Next,
        Action::Previous,
        Action::Restart,
        Action::Seek(7),
        Action::PreviousSolution,
        Action::NextSolution,
    ];
    for action in actions {
        let rig = Rig::new(4);
        rig.act(Action::Seek(20));
        rig.act(Action::Play);
        assert_eq!(rig.scheduler.pending(), 1);
        rig.act(action);
        assert!(!rig.controller.is_playing(), "{action:?}");
        assert_eq!(rig.scheduler.pending(), 0, "{action:?}");
    }
}

#[test]
fn pause_stops_playback_and_says_where_the_search_is() {
    let rig = Rig::new(4);
    rig.act(Action::Play);
    rig.scheduler.advance();
    rig.act(Action::Pause);
    assert!(!rig.controller.is_playing());
    assert_eq!(rig.scheduler.pending(), 0);
    let said = rig.said();
    assert!(
        said.last().is_some_and(|text| text.starts_with("Paused at step 1 of")),
        "{said:?}"
    );
    rig.act(Action::Pause);
    assert_eq!(rig.said().len(), said.len(), "pausing when paused says nothing");
}

#[test]
fn a_tick_that_was_already_on_its_way_when_playback_stopped_does_nothing() {
    let rig = Rig::new(4);
    rig.scheduler.keep_cancelled();
    rig.act(Action::Play);
    rig.act(Action::Pause);
    assert_eq!(rig.scheduler.pending(), 1, "the stale tick is still queued");
    let draws = rig.draws();
    rig.scheduler.advance();
    assert_eq!((rig.at(), rig.draws(), rig.scheduler.pending()), (0, draws, 0));
}

#[test]
fn a_tick_from_an_earlier_run_does_not_move_a_later_one() {
    let rig = Rig::new(4);
    rig.scheduler.keep_cancelled();
    rig.act(Action::Play);
    rig.act(Action::Pause);
    rig.act(Action::Play);
    assert_eq!(rig.scheduler.pending(), 2, "the stale one and the new one");
    rig.scheduler.advance();
    assert_eq!(rig.at(), 0, "the stale one did nothing");
    rig.scheduler.advance();
    assert_eq!(rig.at(), 1);
}

#[test]
fn only_a_solution_and_the_end_are_announced_while_playing() {
    let rig = Rig::new(4);
    rig.act(Action::Play);
    rig.play_out(1_000);
    let said = rig.said();
    assert_eq!(said.len(), 3, "two solutions and the end: {said:?}");
    assert!(said[0].contains("solution 1 of 2") && said[1].contains("solution 2 of 2"));
    assert!(said[2].contains("finished"));
}

#[test]
fn a_draw_that_fails_during_play_stops_playback_and_reports_it() {
    let rig = Rig::new(4);
    rig.act(Action::Play);
    rig.scheduler.advance();
    rig.surface.fail_draws.set(1);
    rig.scheduler.advance();
    assert!(!rig.controller.is_playing());
    assert_eq!(rig.scheduler.pending(), 0);
    assert_eq!((rig.at(), rig.controller.is_dirty()), (1, true));
    assert_eq!(rig.surface.failures.borrow().len(), 1);
}

#[test]
fn a_slow_page_makes_no_backlog_since_the_next_tick_is_asked_for_after_the_draw() {
    let rig = Rig::new(4);
    rig.act(Action::Play);
    // However long the draw took, only one tick is ever waiting once it has run.
    for _ in 0..5 {
        rig.scheduler.advance();
        assert_eq!(rig.scheduler.pending(), 1);
    }
}

#[test]
fn a_new_speed_replaces_the_tick_that_is_waiting() {
    let rig = Rig::new(4);
    rig.act(Action::Play);
    rig.controller.set_speed(1);
    assert_eq!((rig.scheduler.pending(), rig.scheduler.delay()), (1, Some(1000)));
    rig.controller.set_speed(50);
    assert_eq!((rig.scheduler.pending(), rig.scheduler.delay()), (1, Some(20)));
    rig.controller.set_speed(0);
    assert_eq!(rig.controller.speed(), 1, "a speed of nothing is one a second");
}

#[test]
fn a_new_speed_while_paused_starts_nothing() {
    let rig = Rig::new(4);
    rig.controller.set_speed(5);
    assert_eq!(rig.scheduler.pending(), 0);
}

// A board of another size.

#[test]
fn changing_the_size_stops_playback_and_starts_again_at_the_first_snapshot() {
    let rig = Rig::new(8);
    rig.act(Action::Play);
    rig.scheduler.advance();
    rig.controller.change_size(Trace::build(5).expect("a trace"));
    assert!(!rig.controller.is_playing());
    assert_eq!((rig.scheduler.pending(), rig.at(), rig.controller.trace().size()), (0, 0, 5));
    assert_eq!(rig.surface.rebuilds.borrow().as_slice(), &[5]);
}

#[test]
fn changing_the_size_of_a_dirty_page_makes_it_right() {
    let rig = Rig::new(4);
    rig.make_dirty(9);
    rig.controller.change_size(Trace::build(6).expect("a trace"));
    assert!(!rig.controller.is_dirty());
}

#[test]
fn a_board_that_cannot_be_made_leaves_the_old_one_and_reports() {
    let rig = Rig::new(8);
    rig.act(Action::Seek(100));
    rig.surface.fail_rebuild.set(true);
    let changed = rig.controller.change_size(Trace::build(5).expect("a trace"));
    assert!(!changed);
    assert_eq!((rig.at(), rig.controller.trace().size()), (100, 8));
    assert_eq!(rig.surface.failures.borrow().len(), 1);
    assert!(rig.surface.rebuilds.borrow().is_empty());
}

#[test]
fn a_size_that_is_changed_says_so_and_a_size_that_is_not_does_not() {
    let rig = Rig::new(8);
    assert!(rig.controller.change_size(Trace::build(6).expect("a trace")));
    rig.surface.fail_rebuild.set(true);
    assert!(!rig.controller.change_size(Trace::build(5).expect("a trace")));
    assert_eq!(rig.controller.trace().size(), 6, "the board it was on");
}

#[test]
fn a_failed_new_board_leaves_the_page_dirty_so_that_the_next_action_draws_the_old_board_in_full() {
    let rig = Rig::new(8);
    rig.act(Action::Seek(100));
    rig.surface.fail_rebuild.set(true);
    rig.controller.change_size(Trace::build(5).expect("a trace"));
    // The controls may be part way between the two boards, so nothing is trusted until it is drawn again.
    assert!(rig.controller.is_dirty());
    rig.surface.fail_rebuild.set(false);
    // An action with nowhere to go still repairs, and does so with the trace and the index that were kept.
    rig.act(Action::Seek(100));
    assert!(!rig.controller.is_dirty());
    assert_eq!(rig.last_draw(), Some((100, true)));
    assert_eq!(
        (rig.at(), rig.controller.trace().size(), rig.controller.trace().len()),
        (100, 8, 33_589)
    );
}

#[test]
fn a_failed_new_board_stops_the_playback_and_the_old_board_can_play_on() {
    let rig = Rig::new(8);
    rig.act(Action::Play);
    rig.scheduler.advance();
    rig.surface.fail_rebuild.set(true);
    rig.controller.change_size(Trace::build(5).expect("a trace"));
    assert!(!rig.controller.is_playing());
    assert_eq!(rig.scheduler.pending(), 0);
    rig.surface.fail_rebuild.set(false);
    rig.act(Action::Play);
    assert!(rig.controller.is_playing());
    rig.scheduler.advance();
    assert_eq!(rig.at(), 2, "it carried on from where it was, on the old board");
}

// The end.

#[test]
fn after_teardown_nothing_a_person_does_has_any_effect_and_no_tick_runs() {
    let rig = Rig::new(4);
    rig.scheduler.keep_cancelled();
    rig.act(Action::Play);
    rig.controller.teardown();
    assert!(!rig.controller.is_playing());
    let draws = rig.draws();
    rig.scheduler.advance();
    rig.act(Action::Next);
    rig.act(Action::Play);
    assert_eq!((rig.draws(), rig.at(), rig.scheduler.pending()), (draws, 0, 0));
}

#[test]
fn a_manual_scheduler_runs_tasks_in_order_and_only_when_told() {
    let scheduler = ManualScheduler::default();
    let order = Rc::new(RefCell::new(Vec::new()));
    for name in ["a", "b"] {
        let order = Rc::clone(&order);
        scheduler.schedule(5, Box::new(move || order.borrow_mut().push(name)));
    }
    assert!(order.borrow().is_empty());
    let handle = scheduler.schedule(5, Box::new(|| {}));
    scheduler.cancel(handle);
    assert_eq!(scheduler.pending(), 2);
    assert!(scheduler.advance() && scheduler.advance() && !scheduler.advance());
    assert_eq!(order.borrow().as_slice(), &["a", "b"]);
}
