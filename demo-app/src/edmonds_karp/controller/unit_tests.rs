//! The rules of every control, the repair of a page that was half written, the playback and its stale ticks, the range,
//! and the switch of graph, on a surface that records what it is asked and a scheduler that runs a task only when told
//! to.

use super::*;
use crate::edmonds_karp::{
    fixtures::{CANONICAL, REVERSE},
    projection::{Prefs, Speed, Target},
};

/// A surface that remembers what it was asked to do, and that fails when it is told to.
#[derive(Default)]
struct Fake {
    /// Each draw as the index and whether it was a full one.
    draws: RefCell<Vec<(usize, bool)>>,
    views: RefCell<Vec<View>>,
    said: RefCell<Vec<String>>,
    failures: RefCell<Vec<String>>,
    playback_failures: RefCell<Vec<String>>,
    warnings: RefCell<Vec<String>>,
    playing: RefCell<Vec<bool>>,
    /// How many of the next draws fail.
    fail_draws: Cell<u32>,
    /// How many of the next writes of the Play button fail.
    fail_playing: Cell<u32>,
    switches: RefCell<Vec<(&'static str, u64)>>,
    switch_error: RefCell<Option<SwitchError>>,
}

impl Surface for Fake {
    fn draw(&self, _: &Trace, index: usize, view: &View, full: bool) -> Result<(), String> {
        self.draws.borrow_mut().push((index, full));
        self.views.borrow_mut().push(*view);
        if self.fail_draws.get() > 0 {
            self.fail_draws.set(self.fail_draws.get() - 1);
            return Err("the write failed".to_owned());
        }
        Ok(())
    }

    fn switch(&self, fixture: &'static Fixture, _: &Trace, _: &View, generation: u64) -> Result<(), SwitchError> {
        match self.switch_error.borrow_mut().take() {
            Some(error) => Err(error),
            None => {
                self.switches.borrow_mut().push((fixture.key, generation));
                Ok(())
            },
        }
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

    fn warn(&self, text: &str) {
        self.warnings.borrow_mut().push(text.to_owned());
    }
}

/// What a test does that stops playback.
type Stop = Box<dyn Fn(&Rig)>;

struct Rig {
    controller: Rc<Controller>,
    surface: Rc<Fake>,
    scheduler: Rc<ManualScheduler>,
}

impl Rig {
    fn new() -> Self {
        let surface = Rc::new(Fake::default());
        let scheduler = Rc::new(ManualScheduler::default());
        let trace = Trace::build(&CANONICAL.graph()).expect("a trace");
        let controller = Controller::new(
            &CANONICAL,
            trace,
            View::first(Prefs::default(), Speed::Two),
            Rc::clone(&surface) as Rc<dyn Surface>,
            Rc::clone(&scheduler) as Rc<dyn Scheduler>,
        );
        Self { controller, surface, scheduler }
    }

    fn act(&self, action: Action) {
        self.controller.act(action);
    }

    fn at(&self) -> usize {
        self.controller.selected()
    }

    fn last(&self) -> usize {
        self.controller.trace().last()
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

    fn last_said(&self) -> Option<String> {
        self.surface.said.borrow().last().cloned()
    }

    fn go_to(&self, index: usize) {
        self.act(Action::Seek(index));
    }

    /// Fails the next draw by moving to `to`, which leaves the page dirty and the index where it was.
    fn make_dirty(&self, to: usize) {
        self.surface.fail_draws.set(1);
        self.go_to(to);
        assert!(self.controller.is_dirty());
    }

    fn play_out(&self) -> usize {
        let mut ran = 0;
        while self.scheduler.advance() {
            ran += 1;
        }
        ran
    }
}

// The first draw, and one step.

#[test]
fn the_first_snapshot_is_drawn_in_full_when_the_controller_is_made() {
    let rig = Rig::new();
    assert_eq!(rig.surface.draws.borrow().as_slice(), &[(0, true)]);
    assert_eq!(
        (rig.at(), rig.controller.is_dirty(), rig.controller.is_playing()),
        (0, false, false)
    );
    assert_eq!((rig.controller.generation(), rig.controller.playback_epoch()), (0, 0));
}

#[test]
fn next_previous_and_restart_move_one_snapshot_and_back_to_the_start() {
    let rig = Rig::new();
    rig.act(Action::Next);
    rig.act(Action::Next);
    rig.act(Action::Previous);
    assert_eq!(rig.at(), 1);
    rig.go_to(40);
    rig.act(Action::Restart);
    assert_eq!(rig.at(), 0);
}

#[test]
fn a_step_draws_only_what_differs_and_is_announced_once_in_words() {
    let rig = Rig::new();
    rig.act(Action::Next);
    assert_eq!(rig.last_draw(), Some((1, false)));
    assert_eq!(rig.said().len(), 1);
    assert!(rig.said()[0].starts_with("Step 1 of 86:"), "{:?}", rig.said());
}

// The ends.

#[test]
fn a_button_with_nowhere_to_go_moves_nothing_and_says_the_pinned_words() {
    let cases: Vec<(&str, Action, usize, &str)> = vec![
        ("previous at the start", Action::Previous, 0, AT_FIRST),
        ("restart at the start", Action::Restart, 0, AT_FIRST),
        ("next at the end", Action::Next, 86, AT_LAST),
        ("jump to cut at the end", Action::JumpToCut, 86, AT_LAST),
        ("play at the end", Action::Play, 86, AT_LAST),
        (
            "previous augmentation at the start",
            Action::PreviousAugmentation,
            0,
            NO_EARLIER_AUGMENTATION,
        ),
        (
            "next augmentation at the end",
            Action::NextAugmentation,
            86,
            NO_LATER_AUGMENTATION,
        ),
    ];
    for (name, action, start, message) in cases {
        let rig = Rig::new();
        rig.go_to(start);
        let (at, draws) = (rig.at(), rig.draws());
        rig.act(action);
        assert_eq!((rig.at(), rig.draws()), (at, draws), "{name}: nothing moved or was drawn");
        assert_eq!(rig.last_said().as_deref(), Some(message), "{name}");
        assert!(!rig.controller.is_playing(), "{name}");
    }
}

#[test]
fn the_messages_are_the_pinned_ones() {
    assert_eq!(AT_FIRST, "Already at the first step");
    assert_eq!(AT_LAST, "Already at the last step");
    assert_eq!(NO_EARLIER_AUGMENTATION, "No earlier augmentation");
    assert_eq!(NO_LATER_AUGMENTATION, "No later augmentation");
}

#[test]
fn a_boundary_message_comes_only_after_the_page_has_been_repaired() {
    let rig = Rig::new();
    rig.go_to(rig.last());
    rig.make_dirty(rig.last() - 1);
    let said = rig.said().len();
    // The repair itself fails, so nothing else happens and nothing is said.
    rig.surface.fail_draws.set(1);
    rig.act(Action::Next);
    assert!(rig.controller.is_dirty());
    assert_eq!(rig.said().len(), said, "no boundary message while the page is out of date");
    // Now it works: the page is drawn in full, and then the boundary is said.
    rig.act(Action::Next);
    assert!(!rig.controller.is_dirty());
    assert_eq!(rig.last_said().as_deref(), Some(AT_LAST));
    assert_eq!(rig.at(), rig.last());
}

// Jumps.

#[test]
fn the_augmentation_buttons_move_strictly_between_the_augmentations() {
    let rig = Rig::new();
    let at = rig.controller.trace().augmentation_indices().to_vec();
    rig.act(Action::NextAugmentation);
    assert_eq!(rig.at(), at[0]);
    rig.act(Action::NextAugmentation);
    assert_eq!(rig.at(), at[1], "from an augmentation, the next one and not itself");
    rig.act(Action::PreviousAugmentation);
    assert_eq!(rig.at(), at[0]);
    rig.act(Action::NextAugmentation);
    rig.act(Action::NextAugmentation);
    assert_eq!(rig.at(), at[2]);
    rig.act(Action::NextAugmentation);
    assert_eq!(rig.at(), at[2], "no augmentation after the last");
    assert_eq!(rig.last_said().as_deref(), Some(NO_LATER_AUGMENTATION));
}

#[test]
fn jump_to_cut_lands_on_the_last_snapshot_and_is_announced_like_a_step() {
    let rig = Rig::new();
    rig.act(Action::JumpToCut);
    assert_eq!(rig.at(), 86);
    assert!(rig.last_said().expect("a message").starts_with("Step 86 of 86: MinCutShown"));
    assert!(!rig.controller.is_playing());
}

// A page that is half written.

#[test]
fn a_draw_that_fails_leaves_the_index_marks_the_page_dirty_and_stops_playback() {
    let rig = Rig::new();
    rig.act(Action::Play);
    rig.surface.fail_draws.set(1);
    rig.scheduler.advance();
    assert_eq!(rig.at(), 0, "the index did not advance");
    assert!(rig.controller.is_dirty());
    assert!(!rig.controller.is_playing());
    assert_eq!(rig.scheduler.pending(), 0, "no further tick was scheduled");
    assert_eq!(rig.surface.failures.borrow().len(), 1);
}

#[test]
fn nothing_is_announced_for_a_step_whose_write_failed() {
    let rig = Rig::new();
    rig.surface.fail_draws.set(1);
    rig.act(Action::Next);
    assert_eq!(rig.at(), 0);
    assert!(rig.said().is_empty(), "{:?}", rig.said());
}

#[test]
fn every_action_that_has_nowhere_to_go_still_repairs_a_dirty_page_first() {
    let actions: Vec<(&str, Action, usize)> = vec![
        ("previous at the start", Action::Previous, 0),
        ("restart at the start", Action::Restart, 0),
        ("next at the end", Action::Next, 86),
        ("play at the end", Action::Play, 86),
        ("jump to cut at the end", Action::JumpToCut, 86),
        ("a seek to where it is", Action::Seek(0), 0),
        ("previous augmentation with none", Action::PreviousAugmentation, 0),
        ("next augmentation with none", Action::NextAugmentation, 86),
        ("a toggle", Action::Toggle(Pref::Queue), 5),
        ("a focus", Action::Focus(Target::Edge(2)), 5),
        ("the graph that is already shown", Action::SwitchFixture(&CANONICAL), 5),
    ];
    for (name, action, start) in actions {
        let rig = Rig::new();
        rig.go_to(start);
        let other = if start == 0 { 1 } else { start - 1 };
        rig.make_dirty(other);
        assert_eq!(rig.at(), start, "{name}");
        rig.act(action);
        assert!(!rig.controller.is_dirty(), "{name}: repaired");
        assert!(
            rig.surface.draws.borrow().iter().any(|&(i, full)| full && i == start),
            "{name}: drawn in full"
        );
        assert_eq!(rig.at(), start, "{name}: and did not move");
    }
}

#[test]
fn a_failed_repair_keeps_the_page_dirty_and_the_index_and_does_nothing_else() {
    let rig = Rig::new();
    rig.go_to(10);
    rig.make_dirty(11);
    rig.surface.fail_draws.set(1);
    rig.act(Action::Next);
    assert_eq!((rig.at(), rig.controller.is_dirty()), (10, true));
    assert_eq!(
        rig.last_draw(),
        Some((10, true)),
        "it tried to draw the page in full, and not a step"
    );
    rig.act(Action::Next);
    assert_eq!(
        (rig.at(), rig.controller.is_dirty()),
        (11, false),
        "the next try repairs and then steps"
    );
}

#[test]
fn play_does_not_start_on_a_page_that_cannot_be_repaired() {
    let rig = Rig::new();
    rig.make_dirty(3);
    rig.surface.fail_draws.set(1);
    rig.act(Action::Play);
    assert!(!rig.controller.is_playing());
    assert_eq!(rig.scheduler.pending(), 0);
}

// The view.

#[test]
fn a_toggle_is_committed_only_when_the_page_has_drawn_it_and_does_not_move_the_index() {
    let rig = Rig::new();
    rig.go_to(20);
    rig.act(Action::Toggle(Pref::Residual));
    assert!(!rig.controller.view().prefs.show_residual);
    assert_eq!(rig.at(), 20);
    rig.surface.fail_draws.set(1);
    rig.act(Action::Toggle(Pref::Queue));
    assert!(
        rig.controller.view().prefs.show_queue,
        "a toggle that was not drawn is not committed"
    );
    assert!(rig.controller.is_dirty());
}

#[test]
fn a_toggle_and_a_focus_do_not_stop_playback() {
    let rig = Rig::new();
    rig.act(Action::Play);
    rig.act(Action::Toggle(Pref::Detail));
    rig.act(Action::Focus(Target::Vertex(3)));
    assert!(rig.controller.is_playing());
    assert_eq!(rig.scheduler.pending(), 1);
}

#[test]
fn a_focus_moves_the_roving_target_and_never_the_index() {
    let rig = Rig::new();
    rig.go_to(12);
    rig.act(Action::Focus(Target::Edge(4)));
    let view = rig.controller.view();
    assert_eq!((view.focused, view.roving), (Some(Target::Edge(4)), Target::Edge(4)));
    assert_eq!(rig.at(), 12);
}

// Playback.

#[test]
fn play_waits_for_a_tick_after_the_interval_of_the_speed_and_claims_to_play_once_it_is_waiting() {
    let rig = Rig::new();
    rig.act(Action::Play);
    assert!(rig.controller.is_playing());
    assert_eq!((rig.scheduler.pending(), rig.scheduler.delay()), (1, Some(200)));
    assert_eq!(rig.surface.playing.borrow().as_slice(), &[true]);
}

#[test]
fn each_tick_shows_one_snapshot_and_asks_for_the_next_only_after_it_is_drawn() {
    let rig = Rig::new();
    rig.act(Action::Play);
    for expected in 1..=10 {
        assert_eq!(rig.scheduler.pending(), 1, "exactly one tick is ever waiting");
        assert!(rig.scheduler.advance());
        assert_eq!(rig.at(), expected);
        assert_eq!(rig.last_draw(), Some((expected, false)));
    }
}

#[test]
fn playing_a_whole_run_reaches_the_end_and_stops_with_nothing_waiting() {
    let rig = Rig::new();
    rig.act(Action::Play);
    assert_eq!(rig.play_out(), 86);
    assert_eq!(rig.at(), 86);
    assert!(!rig.controller.is_playing());
    assert_eq!(rig.scheduler.pending(), 0);
    assert_eq!(rig.surface.playing.borrow().as_slice(), &[true, false]);
}

#[test]
fn consuming_a_tick_on_the_way_is_not_a_stop_but_the_terminal_step_is() {
    let rig = Rig::new();
    rig.act(Action::Play);
    let epoch = rig.controller.playback_epoch();
    for _ in 0..5 {
        rig.scheduler.advance();
    }
    assert_eq!(
        rig.controller.playback_epoch(),
        epoch,
        "a tick on the way does not count as a stop"
    );
    rig.play_out();
    assert_eq!(rig.controller.playback_epoch(), epoch + 1, "the end does");
}

#[test]
fn only_a_path_an_augmentation_and_the_cut_are_announced_while_playing() {
    let rig = Rig::new();
    rig.act(Action::Play);
    rig.play_out();
    let said = rig.said();
    assert_eq!(said.len(), 7, "three paths, three augmentations and the cut: {said:?}");
    for text in &said {
        assert!(
            ["PathFound", "FlowAugmented", "MinCutShown"].iter().any(|k| text.contains(k)),
            "{text}"
        );
    }
}

#[test]
fn pause_stops_the_playback_takes_the_tick_back_and_says_where_it_is() {
    let rig = Rig::new();
    rig.act(Action::Play);
    for _ in 0..3 {
        rig.scheduler.advance();
    }
    rig.act(Action::Pause);
    assert!(!rig.controller.is_playing());
    assert_eq!(rig.scheduler.pending(), 0);
    assert!(rig.last_said().expect("a message").starts_with("Paused at step 3 of 86"));
    let said = rig.said().len();
    rig.act(Action::Pause);
    assert_eq!(rig.said().len(), said, "pausing when paused says nothing");
}

#[test]
fn play_while_playing_does_not_start_a_second_loop() {
    let rig = Rig::new();
    rig.act(Action::Play);
    rig.act(Action::Play);
    assert_eq!(rig.scheduler.pending(), 1);
}

#[test]
fn every_logical_stop_counts_even_when_nothing_was_playing() {
    let stops: Vec<(&str, Stop)> = vec![
        ("pause", Box::new(|r| r.act(Action::Pause))),
        ("next", Box::new(|r| r.act(Action::Next))),
        ("restart", Box::new(|r| r.act(Action::Restart))),
        ("a jump", Box::new(|r| r.act(Action::NextAugmentation))),
        ("a seek", Box::new(|r| r.controller.seek_input(5))),
        ("teardown", Box::new(|r| r.controller.teardown())),
    ];
    for (name, stop) in stops {
        let rig = Rig::new();
        rig.act(Action::Play);
        let before = rig.controller.playback_epoch();
        stop(&rig);
        assert!(rig.controller.playback_epoch() > before, "{name}");
        assert!(!rig.controller.is_playing(), "{name}");
        assert_eq!(rig.scheduler.pending(), 0, "{name}");
    }
}

#[test]
fn a_boundary_press_stops_playback_too() {
    let rig = Rig::new();
    rig.act(Action::Play);
    rig.act(Action::Previous);
    assert!(!rig.controller.is_playing());
    assert_eq!(rig.at(), 0);
}

#[test]
fn a_tick_from_before_a_pause_does_nothing_even_when_taking_it_back_failed() {
    let rig = Rig::new();
    rig.scheduler.keep_cancelled();
    rig.act(Action::Play);
    rig.act(Action::Pause);
    rig.act(Action::Play);
    assert_eq!(rig.scheduler.pending(), 2, "the stale tick and the new one");
    rig.scheduler.advance();
    assert_eq!(rig.at(), 0, "the stale tick did nothing");
    rig.scheduler.advance();
    assert_eq!(rig.at(), 1);
}

#[test]
fn a_new_speed_replaces_the_tick_that_is_waiting_and_a_stale_one_is_ignored() {
    let rig = Rig::new();
    rig.scheduler.keep_cancelled();
    rig.act(Action::Play);
    rig.act(Action::SetSpeed(Speed::Four));
    assert_eq!(rig.controller.view().speed, Speed::Four);
    assert_eq!(rig.scheduler.pending(), 2);
    rig.scheduler.advance();
    assert_eq!(rig.at(), 0, "the tick of the old speed is stale");
    assert_eq!(rig.scheduler.delay(), Some(100));
    rig.scheduler.advance();
    assert_eq!(rig.at(), 1);
}

#[test]
fn a_new_speed_while_paused_starts_nothing() {
    let rig = Rig::new();
    rig.act(Action::SetSpeed(Speed::Half));
    assert_eq!(rig.scheduler.pending(), 0);
    rig.act(Action::Play);
    assert_eq!(rig.scheduler.delay(), Some(800));
}

// A timer that cannot be set.

#[test]
fn play_does_not_claim_to_be_playing_when_the_first_tick_cannot_be_scheduled() {
    let rig = Rig::new();
    rig.scheduler.fail_next_timers(1);
    rig.act(Action::Play);
    assert!(!rig.controller.is_playing());
    assert_eq!(rig.scheduler.pending(), 0);
    assert!(rig.surface.playing.borrow().is_empty(), "the page was never told that it plays");
    assert_eq!(rig.surface.playback_failures.borrow().len(), 1);
    assert_eq!((rig.at(), rig.controller.is_dirty()), (0, false));
    assert!(rig.surface.failures.borrow().is_empty(), "it is not a failure to draw");
    rig.act(Action::Play);
    assert!(rig.controller.is_playing(), "Play works when pressed again");
}

#[test]
fn playback_stops_and_says_so_when_a_later_tick_cannot_be_scheduled() {
    let rig = Rig::new();
    rig.act(Action::Play);
    rig.scheduler.advance();
    rig.scheduler.fail_next_timers(1);
    rig.scheduler.advance();
    assert_eq!(rig.at(), 2, "the step that was drawn is kept");
    assert!(!rig.controller.is_playing());
    assert_eq!(rig.scheduler.pending(), 0);
    assert_eq!(rig.surface.playing.borrow().as_slice(), &[true, false]);
    assert_eq!(rig.surface.playback_failures.borrow().len(), 1);
    assert!(!rig.controller.is_dirty());
}

#[test]
fn a_new_speed_that_cannot_be_scheduled_stops_playback_and_says_so() {
    let rig = Rig::new();
    rig.act(Action::Play);
    rig.scheduler.fail_next_timers(1);
    rig.act(Action::SetSpeed(Speed::One));
    assert!(!rig.controller.is_playing());
    assert_eq!(rig.surface.playback_failures.borrow().len(), 1);
    assert_eq!(rig.controller.view().speed, Speed::One, "the speed that was chosen is kept");
}

// The range.

#[test]
fn moving_the_range_many_times_draws_once_at_the_latest_place_at_the_next_frame() {
    let rig = Rig::new();
    let draws = rig.draws();
    for index in [10, 40, 25] {
        rig.controller.seek_input(index);
    }
    assert_eq!(rig.scheduler.pending_frames(), 1, "one frame is asked for");
    assert_eq!(rig.draws(), draws, "nothing is drawn until it comes");
    rig.scheduler.advance_frame();
    assert_eq!((rig.at(), rig.draws()), (25, draws + 1));
    assert!(rig.said().is_empty(), "the range does not speak until it rests");
}

#[test]
fn the_first_move_of_the_range_stops_playback() {
    let rig = Rig::new();
    rig.act(Action::Play);
    rig.controller.seek_input(30);
    assert!(!rig.controller.is_playing());
    assert_eq!(rig.scheduler.pending(), 0, "no tick can come between the move and its frame");
}

#[test]
fn letting_go_of_the_range_draws_what_is_waiting_and_says_where_it_is_once() {
    let rig = Rig::new();
    rig.controller.seek_input(30);
    rig.controller.seek_release(30);
    assert_eq!(rig.at(), 30);
    assert_eq!(
        rig.scheduler.pending_frames(),
        0,
        "the frame was taken back, so it does not draw twice"
    );
    assert_eq!(rig.said().len(), 1);
    assert!(rig.said()[0].starts_with("Step 30 of 86"));
    // The frame that came first, and then the release, say it once.
    let rig = Rig::new();
    rig.controller.seek_input(30);
    rig.scheduler.advance_frame();
    rig.controller.seek_release(30);
    assert_eq!(rig.said().len(), 1);
}

#[test]
fn letting_go_without_a_move_goes_to_where_the_range_is() {
    let rig = Rig::new();
    rig.controller.seek_release(17);
    assert_eq!(rig.at(), 17);
}

#[test]
fn nothing_is_said_when_the_page_could_not_draw_where_the_range_rests() {
    let rig = Rig::new();
    rig.controller.seek_input(30);
    rig.surface.fail_draws.set(1);
    rig.controller.seek_release(30);
    assert_eq!(rig.at(), 0);
    assert!(rig.controller.is_dirty());
    assert!(rig.said().is_empty());
}

#[test]
fn a_frame_that_cannot_be_asked_for_draws_the_latest_place_at_once_and_warns() {
    let rig = Rig::new();
    rig.scheduler.fail_next_frames(1);
    rig.controller.seek_input(30);
    assert_eq!(rig.at(), 30, "committed synchronously, once");
    assert_eq!(rig.surface.warnings.borrow().len(), 1);
    assert_eq!(
        rig.scheduler.pending_frames(),
        0,
        "no seek is left waiting for a frame that never comes"
    );
    assert!(!rig.controller.is_dirty());
}

#[test]
fn a_frame_that_cannot_be_asked_for_and_a_write_that_fails_leave_the_index_and_a_dirty_page() {
    let rig = Rig::new();
    rig.scheduler.fail_next_frames(1);
    rig.surface.fail_draws.set(1);
    rig.controller.seek_input(30);
    assert_eq!(rig.at(), 0);
    assert!(rig.controller.is_dirty());
    assert!(rig.said().is_empty(), "nothing is announced");
}

#[test]
fn a_move_of_the_range_made_against_an_earlier_graph_is_dropped() {
    let rig = Rig::new();
    rig.controller.seek_input(70);
    rig.act(Action::SwitchFixture(&REVERSE));
    // The frame that was waiting comes now, for a graph that is gone. The new graph has 44 snapshots, and 70 means
    // nothing.
    assert_eq!(rig.scheduler.pending_frames(), 0, "it was taken back with the switch");
    rig.scheduler.advance_frame();
    assert_eq!(rig.at(), 0);
}

#[test]
fn a_stale_frame_that_survives_cancelling_does_nothing() {
    let rig = Rig::new();
    rig.scheduler.keep_cancelled();
    rig.controller.seek_input(70);
    rig.act(Action::SwitchFixture(&REVERSE));
    assert_eq!(rig.scheduler.pending_frames(), 1);
    rig.scheduler.advance_frame();
    assert_eq!(
        (rig.at(), rig.controller.trace().last()),
        (0, 43),
        "the new graph is where it was put"
    );
}

// The switch of graph.

#[test]
fn a_switch_resets_to_the_first_step_counts_the_generation_once_and_keeps_what_a_person_chose() {
    let rig = Rig::new();
    rig.act(Action::Toggle(Pref::Residual));
    rig.act(Action::SetSpeed(Speed::Four));
    rig.act(Action::Focus(Target::Edge(3)));
    rig.go_to(40);
    rig.act(Action::SwitchFixture(&REVERSE));
    assert_eq!(rig.controller.fixture().key, "reverse");
    assert_eq!((rig.at(), rig.controller.generation()), (0, 1));
    assert_eq!(rig.controller.trace().last(), 43);
    let view = rig.controller.view();
    assert!(
        !view.prefs.show_residual && view.speed == Speed::Four,
        "preferences and speed are kept"
    );
    assert_eq!(
        (view.focused, view.roving),
        (None, Target::Vertex(0)),
        "the focus is cleared, and roves from s"
    );
    assert_eq!(
        rig.surface.switches.borrow().as_slice(),
        &[("reverse", 1)],
        "its IDs carry generation + 1"
    );
}

#[test]
fn a_switch_to_the_graph_that_is_shown_does_nothing() {
    let rig = Rig::new();
    rig.go_to(9);
    rig.act(Action::SwitchFixture(&CANONICAL));
    assert_eq!((rig.at(), rig.controller.generation()), (9, 0));
    assert!(rig.surface.switches.borrow().is_empty());
}

#[test]
fn a_switch_stops_playback_and_a_tick_from_before_it_does_nothing() {
    let rig = Rig::new();
    rig.scheduler.keep_cancelled();
    rig.act(Action::Play);
    rig.scheduler.advance();
    rig.act(Action::SwitchFixture(&REVERSE));
    assert!(!rig.controller.is_playing());
    let waiting = rig.scheduler.pending();
    rig.scheduler.advance();
    assert_eq!(rig.at(), 0, "the old graph's tick is stale ({waiting} was waiting)");
}

#[test]
fn a_switch_that_failed_before_touching_the_page_changes_nothing_and_stops_playback() {
    let rig = Rig::new();
    rig.go_to(20);
    rig.act(Action::Play);
    *rig.surface.switch_error.borrow_mut() = Some(SwitchError::Detached("the new graph could not be drawn".into()));
    rig.act(Action::SwitchFixture(&REVERSE));
    assert_eq!(rig.controller.fixture().key, "canonical");
    assert_eq!(
        (rig.at(), rig.controller.generation(), rig.controller.trace().last()),
        (20, 0, 86)
    );
    assert!(!rig.controller.is_playing(), "every failed switch pauses");
    assert!(!rig.controller.is_dirty(), "nothing live was touched");
    assert_eq!(rig.surface.failures.borrow().len(), 1);
    assert_eq!(rig.scheduler.pending(), 0);
}

#[test]
fn a_switch_that_failed_after_touching_the_page_leaves_it_dirty_and_the_next_action_repairs_the_old_graph() {
    let rig = Rig::new();
    rig.go_to(20);
    *rig.surface.switch_error.borrow_mut() = Some(SwitchError::Live("a control could not be written".into()));
    rig.act(Action::SwitchFixture(&REVERSE));
    assert_eq!(
        (rig.controller.fixture().key, rig.at(), rig.controller.generation()),
        ("canonical", 20, 0)
    );
    assert!(rig.controller.is_dirty());
    rig.act(Action::Next);
    assert!(!rig.controller.is_dirty());
    let draws = rig.surface.draws.borrow().clone();
    assert_eq!(
        &draws[draws.len() - 2..],
        &[(20, true), (21, false)],
        "repair of the old graph, then the step"
    );
}

#[test]
fn a_failed_switch_does_not_use_up_a_generation() {
    let rig = Rig::new();
    *rig.surface.switch_error.borrow_mut() = Some(SwitchError::Detached("no".into()));
    rig.act(Action::SwitchFixture(&REVERSE));
    assert_eq!(rig.controller.generation(), 0);
    rig.act(Action::SwitchFixture(&REVERSE));
    assert_eq!(rig.controller.generation(), 1);
    assert_eq!(
        rig.surface.switches.borrow().as_slice(),
        &[("reverse", 1)],
        "the same candidate number, now committed"
    );
}

#[test]
fn a_failed_switch_takes_back_a_move_of_the_range_that_was_waiting() {
    let rig = Rig::new();
    rig.controller.seek_input(50);
    *rig.surface.switch_error.borrow_mut() = Some(SwitchError::Detached("no".into()));
    rig.act(Action::SwitchFixture(&REVERSE));
    assert_eq!(rig.scheduler.pending_frames(), 0);
    rig.scheduler.advance_frame();
    assert_eq!(rig.at(), 0, "the move was cancelled, and the old graph stays where it is");
}

#[test]
fn every_failed_or_successful_switch_counts_as_a_stop() {
    for error in [
        None,
        Some(SwitchError::Detached("x".into())),
        Some(SwitchError::Live("x".into())),
    ] {
        let rig = Rig::new();
        let before = rig.controller.playback_epoch();
        *rig.surface.switch_error.borrow_mut() = error;
        rig.act(Action::SwitchFixture(&REVERSE));
        assert!(rig.controller.playback_epoch() > before);
    }
}

// The end.

#[test]
fn after_teardown_nothing_a_person_does_has_any_effect_and_no_callback_runs() {
    let rig = Rig::new();
    rig.scheduler.keep_cancelled();
    rig.act(Action::Play);
    rig.controller.seek_input(9);
    rig.controller.teardown();
    assert_eq!(rig.scheduler.teardowns(), 1);
    let draws = rig.draws();
    rig.act(Action::Next);
    rig.act(Action::Play);
    rig.controller.seek_input(20);
    rig.controller.seek_release(20);
    assert_eq!(
        (rig.draws(), rig.at(), rig.scheduler.pending(), rig.scheduler.pending_frames()),
        (draws, 0, 0, 0)
    );
    assert!(!rig.scheduler.advance() && !rig.scheduler.advance_frame());
}

#[test]
fn a_manual_scheduler_runs_timers_and_frames_in_order_and_only_when_told() {
    let scheduler = ManualScheduler::default();
    let order = Rc::new(RefCell::new(Vec::new()));
    for name in ["a", "b"] {
        let order = Rc::clone(&order);
        scheduler
            .schedule_once(5, Box::new(move || order.borrow_mut().push(name)))
            .expect("scheduled");
    }
    let frame_order = Rc::clone(&order);
    scheduler
        .request_frame(Box::new(move || frame_order.borrow_mut().push("frame")))
        .expect("scheduled");
    assert!(order.borrow().is_empty());
    let handle = scheduler.schedule_once(5, Box::new(|| {})).expect("scheduled");
    scheduler.cancel(handle);
    assert_eq!((scheduler.pending(), scheduler.pending_frames()), (2, 1));
    assert!(scheduler.advance() && scheduler.advance() && !scheduler.advance());
    assert!(scheduler.advance_frame() && !scheduler.advance_frame());
    assert_eq!(order.borrow().as_slice(), &["a", "b", "frame"]);
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[test]
fn a_frame_that_was_taken_back_but_arrives_does_not_consume_a_newer_seek() {
    let rig = Rig::new();
    rig.scheduler.keep_cancelled();
    // Seek A asks for frame A, which is then taken back by letting go of the range, but arrives anyway.
    rig.controller.seek_input(10);
    rig.controller.seek_release(10);
    assert_eq!(rig.at(), 10);
    // Seek B, in the same graph, asks for frame B.
    rig.controller.seek_input(40);
    assert_eq!(rig.scheduler.pending_frames(), 2, "the taken-back frame A, and frame B");
    // Frame A arrives first. It is not the frame that is waiting, so it draws nothing.
    rig.scheduler.advance_frame();
    assert_eq!(rig.at(), 10, "frame A did not draw seek B");
    // Frame B is still being waited for, and draws it.
    rig.scheduler.advance_frame();
    assert_eq!(rig.at(), 40);
}

#[test]
fn a_seek_whose_frame_was_taken_back_by_a_switch_is_not_drawn_by_that_frame_in_the_next_graph() {
    let rig = Rig::new();
    rig.scheduler.keep_cancelled();
    rig.controller.seek_input(20);
    rig.act(Action::SwitchFixture(&REVERSE));
    rig.controller.seek_input(5);
    rig.scheduler.advance_frame();
    assert_eq!(rig.at(), 0, "the old frame is not the one that is waiting");
    rig.scheduler.advance_frame();
    assert_eq!(rig.at(), 5);
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[test]
fn a_pause_whose_label_could_not_be_written_leaves_the_page_dirty_and_a_boundary_press_repairs_it() {
    let rig = Rig::new();
    rig.act(Action::Play);
    assert!(rig.controller.is_playing());
    rig.surface.fail_playing.set(1);
    rig.act(Action::Restart);
    assert!(rig.controller.is_dirty(), "the button may still say Pause");
    assert_eq!(rig.surface.failures.borrow().len(), 1);
    let draws = rig.surface.draws.borrow().len();
    // The range is at its first step, so Previous has nowhere to go, but it draws the page that is out of date first.
    rig.act(Action::Previous);
    assert_eq!(
        rig.surface.draws.borrow().get(draws),
        Some(&(0, true)),
        "a full draw before the no-op"
    );
    assert!(!rig.controller.is_dirty());
}

#[test]
fn a_play_whose_label_could_not_be_written_does_not_start_and_leaves_no_tick_waiting() {
    let rig = Rig::new();
    rig.surface.fail_playing.set(1);
    rig.act(Action::Play);
    assert!(!rig.controller.is_playing());
    assert_eq!(rig.scheduler.pending(), 0, "the tick was taken back");
    assert_eq!(rig.surface.playback_failures.borrow().len(), 1);
    assert_eq!(
        *rig.surface.playing.borrow(),
        vec![true, false],
        "the button is written back to Play"
    );
    assert!(!rig.controller.is_dirty(), "the page is right");
    rig.act(Action::Play);
    assert!(rig.controller.is_playing(), "a later Play works");
}

#[test]
fn a_play_whose_label_cannot_be_written_back_either_leaves_the_page_dirty() {
    let rig = Rig::new();
    rig.surface.fail_playing.set(2);
    rig.act(Action::Play);
    assert!(!rig.controller.is_playing());
    assert_eq!(rig.scheduler.pending(), 0);
    assert!(rig.controller.is_dirty());
}
