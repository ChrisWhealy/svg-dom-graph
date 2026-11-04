use super::*;
use crate::petri_net::net::{PlaceId, TransitionId as T};

/// One draw that the fake surface was asked for.
#[derive(Debug, Clone, PartialEq)]
struct Drawn {
    index: usize,
    last: usize,
    source: Source,
    full: bool,
    notice: String,
    selected: Option<Target>,
}

#[derive(Default)]
struct Fake {
    draws: RefCell<Vec<Drawn>>,
    said: RefCell<Vec<String>>,
    failures: RefCell<Vec<String>>,
    playback_failures: RefCell<Vec<String>>,
    warnings: RefCell<Vec<String>>,
    playing: RefCell<Vec<bool>>,
    /// How many of the next draws fail.
    fail_draws: Cell<u32>,
    /// Whether the Play button write fails.
    fail_playing: Cell<bool>,
}

impl Surface for Fake {
    fn draw(&self, frame: &Frame, full: bool) -> Result<(), String> {
        if self.fail_draws.get() > 0 {
            self.fail_draws.set(self.fail_draws.get() - 1);
            return Err("a write failed".to_owned());
        }
        self.draws.borrow_mut().push(Drawn {
            index: frame.index,
            last: frame.trace.last(),
            source: frame.source,
            full,
            notice: frame.notice.to_owned(),
            selected: frame.view.selected,
        });
        Ok(())
    }

    fn playing_changed(&self, playing: bool) -> Result<(), String> {
        if self.fail_playing.get() {
            return Err("the button could not be written".to_owned());
        }
        self.playing.borrow_mut().push(playing);
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

struct Rig {
    fake: Rc<Fake>,
    clock: Rc<ManualScheduler>,
    controller: Rc<Controller>,
}

fn rig_for(scenario: Scenario) -> Rig {
    let (fake, clock) = (Rc::new(Fake::default()), Rc::new(ManualScheduler::default()));
    let controller = Controller::new(
        Net::bounded_buffer(),
        scenario,
        View::first(),
        Rc::clone(&fake) as Rc<dyn Surface>,
        Rc::clone(&clock) as Rc<dyn Scheduler>,
    );
    Rig { fake, clock, controller }
}

fn rig() -> Rig {
    rig_for(Scenario::Walkthrough)
}

impl Rig {
    fn act(&self, action: Action) {
        self.controller.act(action);
    }

    fn last_draw(&self) -> Drawn {
        self.fake.draws.borrow().last().cloned().expect("a draw")
    }

    fn said(&self) -> Vec<String> {
        self.fake.said.borrow().clone()
    }

    fn last_said(&self) -> String {
        self.said().last().cloned().unwrap_or_default()
    }

    fn select(&self, t: TransitionId) {
        self.act(Action::Select(Target::Transition(t)));
    }

    fn manual_at(&self, snapshot: usize) {
        self.act(Action::Seek(snapshot));
        self.act(Action::EnterManual);
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[test]
fn the_first_snapshot_of_the_first_script_is_drawn_in_full_at_once() {
    let r = rig();
    let drawn = r.last_draw();
    assert_eq!((drawn.index, drawn.last, drawn.full), (0, 50, true));
    assert_eq!(drawn.source, Source::Script(Scenario::Walkthrough));
    assert_eq!((r.controller.cursor(), r.controller.is_dirty()), (0, false));
}

#[test]
fn next_previous_and_restart_move_one_snapshot_and_say_what_happened() {
    let r = rig();
    r.act(Action::Next);
    r.act(Action::Next);
    assert_eq!(r.controller.cursor(), 2);
    assert!(r.last_said().starts_with("Finish[0] consumed"), "{}", r.last_said());
    r.act(Action::Previous);
    assert_eq!(r.controller.cursor(), 1);
    r.act(Action::Restart);
    assert_eq!(r.controller.cursor(), 0);
}

#[test]
fn a_button_with_nowhere_to_go_says_so_and_draws_nothing() {
    let r = rig();
    let before = r.fake.draws.borrow().len();
    r.act(Action::Previous);
    r.act(Action::Restart);
    assert_eq!(r.said(), vec![AT_FIRST, AT_FIRST]);
    r.act(Action::Seek(50));
    r.act(Action::Next);
    assert_eq!(r.last_said(), AT_LAST);
    assert_eq!(r.fake.draws.borrow().len(), before + 1, "only the seek drew");
}

#[test]
fn seek_goes_straight_to_a_snapshot_and_is_clamped_to_the_end() {
    let r = rig();
    r.act(Action::Seek(30));
    assert_eq!(r.controller.cursor(), 30);
    r.act(Action::Seek(500));
    assert_eq!(r.controller.cursor(), 50);
}

#[test]
fn a_checkpoint_goes_to_the_snapshot_of_the_design_without_replaying() {
    let r = rig();
    let before = r.fake.draws.borrow().len();
    for (n, snapshot) in [(0, 24), (1, 26), (2, 27), (3, 45)] {
        r.act(Action::Checkpoint(n));
        assert_eq!(r.controller.cursor(), snapshot);
    }
    assert_eq!(r.fake.draws.borrow().len(), before + 4, "one draw each");
    r.act(Action::Checkpoint(9));
    assert_eq!(r.controller.cursor(), 45, "there is no such checkpoint");
}

#[test]
fn selecting_a_place_or_a_transition_never_changes_the_snapshot_or_fires() {
    let r = rig();
    r.act(Action::Seek(5));
    r.select(T::put(0));
    r.act(Action::Select(Target::Place(PlaceId::FREE)));
    assert_eq!(r.controller.cursor(), 5);
    assert_eq!(r.controller.view().selected, Some(Target::Place(PlaceId::FREE)));
    assert_eq!(r.controller.view().roving, Target::Place(PlaceId::FREE));
    assert_eq!(r.last_draw().selected, Some(Target::Place(PlaceId::FREE)));
    assert!(!r.controller.is_manual());
}

#[test]
fn the_wiring_changes_only_the_view() {
    let r = rig();
    r.act(Action::Seek(7));
    r.act(Action::SetWiring(Wiring::Focused));
    assert_eq!(r.controller.view().wiring, Wiring::Focused);
    assert_eq!((r.controller.cursor(), r.controller.last()), (7, 50));
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[test]
fn play_steps_once_per_tick_and_stops_at_the_end_without_looping() {
    let r = rig_for(Scenario::Contention);
    r.act(Action::Play);
    assert!(r.controller.is_playing());
    assert_eq!(r.clock.delay(), Some(500));
    assert_eq!(*r.fake.playing.borrow(), vec![true]);
    let mut ticks = 0;
    while r.clock.advance() {
        ticks += 1;
    }
    assert_eq!((ticks, r.controller.cursor()), (28, 28));
    assert!(!r.controller.is_playing());
    assert_eq!(*r.fake.playing.borrow(), vec![true, false]);
    assert_eq!(r.clock.pending(), 0, "no further tick");
    assert_eq!(
        r.last_said(),
        summary(r.controller.net(), &Scenario::Contention.trace(r.controller.net()).unwrap(), 28)
    );
    r.act(Action::Play);
    assert_eq!(r.last_said(), AT_LAST, "Play at the end does not restart or loop");
    assert!(!r.controller.is_playing());
}

#[test]
fn playback_says_only_the_milestones_aloud() {
    let r = rig();
    r.act(Action::Seek(23));
    let before = r.said().len();
    r.act(Action::Play);
    for _ in 0..4 {
        r.clock.advance();
    }
    // Snapshots 24 to 27: Put[3] fills the buffer, then Start[0] and Finish[0] block a producer, then Take[0] releases
    // it.
    assert_eq!(r.controller.cursor(), 27);
    let heard = &r.said()[before..];
    assert_eq!(heard.len(), 2, "{heard:?}");
    assert!(heard[0].starts_with("Finish[0]") && heard[1].starts_with("Take[0]"));
}

#[test]
fn pausing_says_where_it_stopped_and_cancels_the_tick() {
    let r = rig();
    r.act(Action::Play);
    r.clock.advance();
    r.act(Action::Pause);
    assert!(!r.controller.is_playing());
    assert_eq!(r.clock.pending(), 0);
    assert!(r.last_said().starts_with("Paused at snapshot 1 of 50."), "{}", r.last_said());
    r.act(Action::Pause);
    assert_eq!(
        r.said().iter().filter(|s| s.starts_with("Paused")).count(),
        1,
        "pausing twice says it once"
    );
}

#[test]
fn every_stop_counts_and_a_tick_from_before_it_does_nothing() {
    let r = rig();
    r.controller.act(Action::Play);
    let epoch = r.controller.playback_epoch();
    r.clock.keep_cancelled();
    r.act(Action::Next);
    assert!(r.controller.playback_epoch() > epoch);
    assert_eq!(r.controller.cursor(), 1);
    r.clock.advance();
    assert_eq!(r.controller.cursor(), 1, "the stale tick was ignored");
    assert!(!r.controller.is_playing());
}

#[test]
fn a_speed_change_while_playing_replaces_the_tick_with_one_at_the_new_interval() {
    let r = rig();
    r.act(Action::Play);
    r.act(Action::SetSpeed(Speed::Ten));
    assert_eq!((r.clock.pending(), r.clock.delay()), (1, Some(100)));
    assert!(r.controller.is_playing());
    r.act(Action::Pause);
    r.act(Action::SetSpeed(Speed::One));
    assert_eq!(r.clock.pending(), 0, "a speed change does not start playback");
}

#[test]
fn a_timer_that_cannot_be_scheduled_leaves_playback_paused_and_says_so() {
    let r = rig();
    r.clock.fail_next_timers(1);
    r.act(Action::Play);
    assert!(!r.controller.is_playing());
    assert_eq!(r.fake.playback_failures.borrow().len(), 1);
    assert!(!r.controller.is_dirty(), "the page is right");
    r.act(Action::Play);
    r.clock.fail_next_timers(1);
    r.clock.advance();
    assert_eq!(r.controller.cursor(), 1, "the tick drew its snapshot");
    assert!(!r.controller.is_playing());
    assert_eq!(r.fake.playback_failures.borrow().len(), 2);
}

#[test]
fn a_play_button_that_cannot_be_written_leaves_playback_paused() {
    let r = rig();
    r.fake.fail_playing.set(true);
    r.act(Action::Play);
    assert!(!r.controller.is_playing());
    assert_eq!(r.clock.pending(), 0);
    assert_eq!(r.fake.playback_failures.borrow().len(), 1);
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[test]
fn moving_the_range_many_times_between_frames_draws_once() {
    let r = rig();
    let before = r.fake.draws.borrow().len();
    for index in [3, 9, 14, 21] {
        r.controller.seek_input(index);
    }
    assert_eq!((r.clock.pending_frames(), r.fake.draws.borrow().len()), (1, before));
    r.clock.advance_frame();
    assert_eq!((r.controller.cursor(), r.fake.draws.borrow().len()), (21, before + 1));
}

#[test]
fn letting_the_range_go_draws_at_once_and_says_where_it_is_once() {
    let r = rig();
    r.controller.seek_input(10);
    r.controller.seek_release(12);
    assert_eq!(r.controller.cursor(), 10, "the latest request was the one drawn");
    assert_eq!(r.clock.pending_frames(), 0);
    assert_eq!(r.said().len(), 1);
    r.controller.seek_release(12);
    assert_eq!(
        r.controller.cursor(),
        12,
        "a release with nothing waiting goes to where it was let go"
    );
}

#[test]
fn a_frame_that_cannot_be_asked_for_draws_at_once_and_warns() {
    let r = rig();
    r.clock.fail_next_frames(1);
    r.controller.seek_input(8);
    assert_eq!(r.controller.cursor(), 8);
    assert_eq!(r.fake.warnings.borrow().len(), 1);
}

#[test]
fn a_seek_asked_for_in_one_history_is_dropped_in_the_next() {
    let r = rig();
    r.controller.seek_input(20);
    r.clock.keep_cancelled();
    r.act(Action::SelectScenario(Scenario::Interleaving));
    r.clock.advance_frame();
    assert_eq!((r.controller.scenario(), r.controller.cursor()), (Scenario::Interleaving, 0));
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[test]
fn entering_manual_forks_from_the_shown_snapshot_at_its_own_index_zero() {
    let r = rig();
    r.manual_at(26);
    assert!(r.controller.is_manual());
    let drawn = r.last_draw();
    assert_eq!((drawn.index, drawn.last), (0, 0));
    assert_eq!(
        drawn.source,
        Source::Manual {
            origin: Scenario::Walkthrough,
            fork: 26
        }
    );
    assert_eq!(r.controller.script_index(), 26);
    assert!(
        r.last_said().starts_with("Manual mode. Forked from Walkthrough"),
        "{}",
        r.last_said()
    );
}

#[test]
fn entering_manual_stops_playback_and_a_late_tick_does_nothing() {
    let r = rig();
    r.act(Action::Play);
    r.clock.keep_cancelled();
    r.act(Action::EnterManual);
    r.clock.advance();
    assert_eq!(r.controller.cursor(), 0);
    assert!(!r.controller.is_playing());
}

#[test]
fn firing_an_enabled_transition_appends_a_snapshot_with_the_inherited_counters() {
    let r = rig();
    r.manual_at(24);
    r.select(T::take(0));
    r.act(Action::FireSelected);
    assert_eq!((r.controller.cursor(), r.controller.last()), (1, 1));
    assert!(
        r.last_said().starts_with("Take[0] consumed C0.Idle and Used"),
        "{}",
        r.last_said()
    );
    assert_eq!(r.last_draw().index, 1);
    assert!(r.last_said().contains("Used 7 of 8, Free 1"));
    // Put[3] had been fired in the script, so the branch starts with the script's 8 Puts and has 1 Take.
    r.select(T::complete(0));
    r.act(Action::FireSelected);
    r.select(T::put(0));
    r.act(Action::FireSelected);
    assert!(r.last_said().starts_with("Put[0] is not enabled"), "{}", r.last_said());
    assert_eq!(r.controller.last(), 2);
}

#[test]
fn a_disabled_attempt_changes_no_snapshot_and_names_the_missing_input() {
    let r = rig();
    r.manual_at(26);
    let draws = r.fake.draws.borrow().len();
    r.select(T::put(0));
    let draws_after_select = r.fake.draws.borrow().len();
    assert_eq!(draws_after_select, draws + 1);
    r.act(Action::FireSelected);
    assert_eq!(r.last_said(), "Put[0] is not enabled: Free has no token. Nothing changed.");
    assert_eq!((r.controller.cursor(), r.controller.last()), (0, 0));
    assert_eq!(r.fake.draws.borrow().len(), draws_after_select, "nothing was drawn");
    assert!(!r.controller.is_playing());
}

#[test]
fn firing_with_nothing_or_a_place_selected_says_why() {
    let r = rig();
    r.act(Action::EnterManual);
    r.act(Action::FireSelected);
    assert_eq!(r.last_said(), FIRE_NOTHING_SELECTED);
    r.act(Action::Select(Target::Place(PlaceId::USED)));
    r.act(Action::FireSelected);
    assert_eq!(r.last_said(), FIRE_A_PLACE);
    assert_eq!(r.controller.last(), 0);
}

#[test]
fn a_script_cannot_fire_by_any_route() {
    let r = rig();
    r.select(T::start(0));
    r.act(Action::FireSelected);
    assert_eq!(r.last_said(), FIRE_IN_SCRIPT);
    r.act(Action::Fire(T::start(1)));
    assert_eq!(r.last_said(), FIRE_IN_SCRIPT);
    assert_eq!((r.controller.cursor(), r.controller.last()), (0, 50));
}

#[test]
fn the_quick_route_selects_the_transition_and_fires_it_once() {
    let r = rig();
    r.act(Action::EnterManual);
    r.act(Action::Fire(T::start(2)));
    assert_eq!(r.controller.view().selected, Some(Target::Transition(T::start(2))));
    assert_eq!((r.controller.cursor(), r.controller.last()), (1, 1));
    r.act(Action::Fire(T::start(2)));
    assert_eq!(r.controller.last(), 1, "Start[2] is not enabled again");
}

#[test]
fn in_manual_mode_previous_next_restart_and_seek_browse_without_recomputing() {
    let r = rig();
    r.act(Action::EnterManual);
    for t in [T::start(0), T::finish(0), T::put(0)] {
        r.act(Action::Fire(t));
    }
    assert_eq!(r.controller.cursor(), 3);
    r.act(Action::Previous);
    assert_eq!(r.controller.cursor(), 2);
    r.act(Action::Restart);
    assert_eq!(r.controller.cursor(), 0);
    r.act(Action::Next);
    r.act(Action::Next);
    assert_eq!((r.controller.cursor(), r.controller.last()), (2, 3));
    r.act(Action::Seek(99));
    assert_eq!(r.controller.cursor(), 3);
    r.act(Action::Next);
    assert_eq!(r.last_said(), AT_LAST, "next cannot create a firing");
}

#[test]
fn firing_from_an_older_manual_snapshot_cuts_off_its_future_and_changes_the_history() {
    let r = rig();
    r.act(Action::EnterManual);
    for t in [T::start(0), T::finish(0), T::put(0)] {
        r.act(Action::Fire(t));
    }
    let generation = r.controller.history_generation();
    r.act(Action::Seek(1));
    r.act(Action::Fire(T::start(1)));
    assert_eq!((r.controller.cursor(), r.controller.last()), (2, 2));
    assert_eq!(
        r.controller.history_generation(),
        generation + 1,
        "a cut is a change of history"
    );
    r.act(Action::Fire(T::finish(1)));
    assert_eq!(r.controller.history_generation(), generation + 1, "an append at the tip is not");
}

#[test]
fn checkpoints_and_play_are_unavailable_in_manual_mode_and_say_so() {
    let r = rig();
    r.act(Action::EnterManual);
    r.act(Action::Checkpoint(1));
    assert_eq!(r.last_said(), NO_CHECKPOINTS_IN_MANUAL);
    assert_eq!(r.controller.cursor(), 0);
    r.act(Action::Fire(T::start(0)));
    r.act(Action::Play);
    assert_eq!(r.last_said(), PLAY_IN_MANUAL);
    assert!(!r.controller.is_playing());
    assert_eq!(r.clock.pending(), 0);
}

#[test]
fn returning_restores_the_script_at_the_fork_index_and_discards_the_branch() {
    let r = rig();
    r.manual_at(27);
    for t in [T::complete(0), T::start(1)] {
        r.act(Action::Fire(t));
    }
    r.act(Action::Previous);
    assert_eq!(r.controller.cursor(), 1);
    r.act(Action::ReturnToScript);
    assert!(!r.controller.is_manual());
    assert_eq!(
        (r.controller.cursor(), r.controller.last()),
        (27, 50),
        "the fork index, whatever the branch did"
    );
    assert_eq!(r.last_draw().source, Source::Script(Scenario::Walkthrough));
    r.act(Action::EnterManual);
    assert_eq!(
        (r.controller.cursor(), r.controller.last()),
        (0, 0),
        "a fresh entry is a fresh branch"
    );
}

#[test]
fn the_script_is_never_changed_by_a_branch() {
    let r = rig();
    let before = (*r.controller.script.borrow()).clone();
    r.manual_at(10);
    for t in [T::finish(1), T::put(1), T::take(0)] {
        r.act(Action::Fire(t));
    }
    r.act(Action::ReturnToScript);
    assert_eq!(*r.controller.script.borrow(), before);
}

#[test]
fn every_change_of_history_makes_a_late_tick_and_a_late_seek_do_nothing() {
    let r = rig();
    for change in [
        Action::EnterManual,
        Action::ReturnToScript,
        Action::SelectScenario(Scenario::Contention),
    ] {
        r.act(Action::Play);
        r.controller.seek_input(15);
        let generation = r.controller.history_generation();
        r.clock.keep_cancelled();
        r.act(change);
        assert!(r.controller.history_generation() > generation, "{change:?}");
        let at = r.controller.cursor();
        r.clock.advance();
        r.clock.advance_frame();
        assert_eq!(r.controller.cursor(), at, "{change:?}");
    }
}

#[test]
fn selecting_a_scenario_shows_snapshot_zero_of_it_on_the_same_scene() {
    let r = rig();
    r.act(Action::Seek(30));
    r.act(Action::SelectScenario(Scenario::Interleaving));
    let drawn = r.last_draw();
    assert_eq!((drawn.index, drawn.last, drawn.full), (0, 30, false), "a diff on the one scene");
    assert_eq!(r.controller.scenario(), Scenario::Interleaving);
    assert_eq!(r.last_said(), "Showing Interleaving: overlapping worker states, at snapshot 0.");
}

#[test]
fn the_selection_survives_a_change_of_scenario() {
    let r = rig();
    r.select(T::put(2));
    r.act(Action::SelectScenario(Scenario::Contention));
    assert_eq!(r.controller.view().selected, Some(Target::Transition(T::put(2))));
    assert_eq!(r.last_draw().selected, Some(Target::Transition(T::put(2))));
}

#[test]
fn choosing_a_scenario_from_manual_discards_the_branch_and_says_so_on_the_page_and_aloud() {
    let r = rig();
    r.manual_at(24);
    r.act(Action::Fire(T::take(0)));
    r.act(Action::SelectScenario(Scenario::Walkthrough));
    let notice = "Manual branch discarded. Walkthrough: fill, block, release, drain loaded at snapshot 0.";
    assert!(!r.controller.is_manual());
    assert_eq!((r.controller.cursor(), r.controller.script_index()), (0, 0));
    assert_eq!(r.last_draw().notice, notice);
    assert_eq!(r.last_said(), notice);
    r.act(Action::Next);
    assert_eq!(r.last_draw().notice, "", "the next successful action replaces it");
}

#[test]
fn the_notice_is_written_again_by_a_repair() {
    let r = rig();
    r.act(Action::EnterManual);
    r.act(Action::SelectScenario(Scenario::Contention));
    r.fake.fail_draws.set(1);
    r.act(Action::Select(Target::Place(PlaceId::FREE)));
    assert!(r.controller.is_dirty());
    r.act(Action::Next);
    let repair = r.fake.draws.borrow().iter().rev().find(|d| d.full).cloned().unwrap();
    assert!(repair.notice.starts_with("Manual branch discarded."), "{}", repair.notice);
}

#[test]
fn a_scenario_that_cannot_be_drawn_leaves_the_old_one_and_the_branch() {
    let r = rig();
    r.manual_at(24);
    r.act(Action::Fire(T::take(0)));
    r.fake.fail_draws.set(1);
    r.act(Action::SelectScenario(Scenario::Contention));
    assert!(r.controller.is_dirty());
    assert_eq!(r.controller.scenario(), Scenario::Walkthrough);
    assert!(r.controller.is_manual());
    assert_eq!(
        (r.controller.cursor(), r.controller.last(), r.controller.script_index()),
        (1, 1, 24)
    );
    assert_eq!(r.fake.failures.borrow().len(), 1);
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[test]
fn a_failed_write_keeps_the_index_marks_the_page_dirty_and_stops_playback() {
    let r = rig();
    r.act(Action::Play);
    r.fake.fail_draws.set(1);
    r.clock.advance();
    assert_eq!(r.controller.cursor(), 0);
    assert!(r.controller.is_dirty() && !r.controller.is_playing());
    assert_eq!(r.fake.failures.borrow().len(), 1);
    assert_eq!(r.clock.pending(), 0);
}

#[test]
fn a_failed_selection_stops_playback_so_that_a_tick_that_was_waiting_cannot_draw_on_a_page_that_is_out_of_date() {
    let r = rig();
    r.act(Action::Seek(4));
    r.act(Action::Play);
    assert!(r.controller.is_playing());
    r.clock.keep_cancelled();
    r.fake.fail_draws.set(1);
    r.select(T::put(0));
    assert!(r.controller.is_dirty());
    assert!(!r.controller.is_playing(), "the failed write stopped playback");
    assert!(!*r.fake.playing.borrow().last().unwrap(), "and the Play button says so");
    let draws = r.fake.draws.borrow().len();
    r.clock.advance();
    assert_eq!(r.controller.cursor(), 4, "the old tick did nothing");
    assert_eq!(r.fake.draws.borrow().len(), draws, "and drew nothing");
    assert!(
        r.controller.is_dirty(),
        "the page is still repaired by the next action, not by a tick"
    );
    r.act(Action::Next);
    assert!(!r.controller.is_dirty() && r.controller.cursor() == 5);
}

#[test]
fn a_failed_change_of_wiring_stops_playback_in_the_same_way() {
    let r = rig();
    r.act(Action::Play);
    r.clock.keep_cancelled();
    r.fake.fail_draws.set(1);
    r.act(Action::SetWiring(Wiring::Focused));
    assert!(r.controller.is_dirty() && !r.controller.is_playing());
    r.clock.advance();
    assert_eq!(r.controller.cursor(), 0);
    assert_eq!(r.controller.view().wiring, Wiring::Overview, "the view was not committed");
}

#[test]
fn the_next_action_draws_the_committed_snapshot_in_full_before_anything_else() {
    let r = rig();
    r.act(Action::Seek(4));
    r.fake.fail_draws.set(1);
    r.act(Action::Next);
    assert_eq!(r.controller.cursor(), 4);
    r.act(Action::Next);
    let draws = r.fake.draws.borrow();
    let (repair, step) = (&draws[draws.len() - 2], &draws[draws.len() - 1]);
    assert_eq!((repair.index, repair.full), (4, true));
    assert_eq!((step.index, step.full), (5, false));
    assert!(!r.controller.is_dirty());
}

#[test]
fn a_press_with_nowhere_to_go_still_repairs() {
    let r = rig();
    r.fake.fail_draws.set(1);
    r.act(Action::Select(Target::Place(PlaceId::USED)));
    assert!(r.controller.is_dirty());
    r.act(Action::Previous);
    assert!(!r.controller.is_dirty());
    assert_eq!(r.last_said(), AT_FIRST);
    assert!(r.fake.draws.borrow().last().is_some_and(|d| d.full));
}

#[test]
fn selecting_the_same_index_again_repairs_too() {
    let r = rig();
    r.act(Action::Seek(6));
    r.fake.fail_draws.set(1);
    r.act(Action::Seek(7));
    assert!(r.controller.is_dirty());
    r.act(Action::Seek(6));
    assert!(!r.controller.is_dirty());
    assert!(r.fake.draws.borrow().last().is_some_and(|d| d.full && d.index == 6));
}

#[test]
fn a_repair_that_fails_stops_everything() {
    let r = rig();
    r.fake.fail_draws.set(2);
    r.act(Action::Next);
    assert!(r.controller.is_dirty());
    r.act(Action::Next);
    assert_eq!(r.controller.cursor(), 0);
    assert_eq!(r.fake.failures.borrow().len(), 2);
    r.act(Action::Next);
    assert_eq!(r.controller.cursor(), 1, "and the third try works");
}

#[test]
fn a_firing_that_cannot_be_drawn_is_not_kept() {
    let r = rig();
    r.act(Action::EnterManual);
    r.select(T::start(0));
    r.fake.fail_draws.set(1);
    r.act(Action::FireSelected);
    assert_eq!((r.controller.cursor(), r.controller.last()), (0, 0), "no speculative snapshot");
    assert!(r.controller.is_dirty());
    r.act(Action::FireSelected);
    assert_eq!(
        (r.controller.cursor(), r.controller.last()),
        (1, 1),
        "the repair came first, then the firing"
    );
}

#[test]
fn entering_manual_that_cannot_be_drawn_stays_in_the_script() {
    let r = rig();
    r.act(Action::Seek(5));
    r.fake.fail_draws.set(1);
    r.act(Action::EnterManual);
    assert!(!r.controller.is_manual() && r.controller.is_dirty());
    assert_eq!(r.controller.cursor(), 5);
}

#[test]
fn returning_that_cannot_be_drawn_stays_in_the_branch() {
    let r = rig();
    r.act(Action::EnterManual);
    r.act(Action::Fire(T::start(0)));
    r.fake.fail_draws.set(1);
    r.act(Action::ReturnToScript);
    assert!(r.controller.is_manual() && r.controller.is_dirty());
    assert_eq!((r.controller.cursor(), r.controller.last()), (1, 1));
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[test]
fn a_branch_at_the_event_budget_refuses_a_firing_and_says_so_before_changing_anything() {
    let r = rig();
    r.act(Action::EnterManual);
    while r.controller.last() + 5 <= MAX_EVENTS {
        for t in [T::start(0), T::finish(0), T::put(0), T::take(0), T::complete(0)] {
            r.act(Action::Fire(t));
        }
    }
    assert_eq!(r.controller.last(), MAX_EVENTS);
    let draws = r.fake.draws.borrow().len();
    r.act(Action::Fire(T::start(1)));
    assert!(
        r.last_said().starts_with("The manual branch is full at 10000 events."),
        "{}",
        r.last_said()
    );
    assert_eq!(
        (r.controller.last(), r.fake.draws.borrow().len() - draws),
        (MAX_EVENTS, 1),
        "only the selection drew"
    );
}

#[test]
fn teardown_stops_playback_and_makes_every_later_call_do_nothing() {
    let r = rig();
    r.act(Action::Play);
    r.controller.seek_input(9);
    r.controller.teardown();
    assert_eq!(r.clock.teardowns(), 1);
    let draws = r.fake.draws.borrow().len();
    r.act(Action::Next);
    r.controller.seek_input(3);
    r.controller.seek_release(3);
    r.clock.advance();
    r.clock.advance_frame();
    assert_eq!((r.controller.cursor(), r.fake.draws.borrow().len()), (0, draws));
    assert!(!r.controller.is_playing());
}
