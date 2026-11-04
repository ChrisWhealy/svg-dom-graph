use super::*;
use crate::tarjan::fixture;
use std::cell::RefCell;

const VIEWPORT: (f64, f64) = (400.0, 360.0);

/// The view that the tests start from: zoomed in on vertex 0, so that other vertices are out of view.
fn start(layout: &Layout) -> Camera {
    view::zoom_to(VIEWPORT, layout, 0).camera
}

/// One draw that the fake surface was asked for.
#[derive(Debug, Clone, PartialEq)]
struct Drawn {
    index: usize,
    full: bool,
    camera: Camera,
    follow: bool,
    component: Option<usize>,
    notice: String,
}

struct Fake {
    camera: Cell<Camera>,
    viewport: Cell<(f64, f64)>,
    draws: RefCell<Vec<Drawn>>,
    said: RefCell<Vec<String>>,
    failures: RefCell<Vec<String>>,
    playback_failures: RefCell<Vec<String>>,
    warnings: RefCell<Vec<String>>,
    playing: RefCell<Vec<bool>>,
    writes: RefCell<Vec<Camera>>,
    fail_draws: Cell<u32>,
    fail_writes: Cell<u32>,
    fail_playing: Cell<bool>,
}

impl Surface for Fake {
    fn draw(&self, frame: &Frame, full: bool) -> Result<(), String> {
        if self.fail_draws.get() > 0 {
            self.fail_draws.set(self.fail_draws.get() - 1);
            return Err("a write failed".to_owned());
        }
        let p = present(
            &crate::tarjan::projection::Context {
                graph: frame.graph,
                edges: frame.edges,
                trace: frame.trace,
                layout: frame.layout,
                camera: frame.camera,
                viewport: frame.viewport,
            },
            frame.index,
            frame.choices,
        )
        .expect("a step");
        self.draws.borrow_mut().push(Drawn {
            index: frame.index,
            full,
            camera: frame.camera,
            follow: frame.choices.follow,
            component: frame.choices.component,
            notice: p.panel[3].clone(),
        });
        Ok(())
    }

    fn camera(&self) -> Camera {
        self.camera.get()
    }

    fn viewport(&self) -> (f64, f64) {
        self.viewport.get()
    }

    fn write_camera(&self, camera: Camera) -> Result<(), String> {
        if self.fail_writes.get() > 0 {
            self.fail_writes.set(self.fail_writes.get() - 1);
            return Err("the view could not be written".to_owned());
        }
        self.camera.set(camera);
        self.writes.borrow_mut().push(camera);
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
    c: Rc<Controller>,
    layout: Layout,
}

fn rig() -> Rig {
    let layout = Layout::build(101.3, 91.8);
    let fake = Rc::new(Fake {
        camera: Cell::new(start(&layout)),
        viewport: Cell::new(VIEWPORT),
        draws: RefCell::default(),
        said: RefCell::default(),
        failures: RefCell::default(),
        playback_failures: RefCell::default(),
        warnings: RefCell::default(),
        playing: RefCell::default(),
        writes: RefCell::default(),
        fail_draws: Cell::new(0),
        fail_writes: Cell::new(0),
        fail_playing: Cell::new(false),
    });
    let clock = Rc::new(ManualScheduler::default());
    let graph = fixture::graph();
    let trace = Trace::build(&graph).unwrap();
    let c = Controller::new(
        graph,
        trace,
        layout.clone(),
        Rc::clone(&fake) as Rc<dyn Surface>,
        Rc::clone(&clock) as Rc<dyn Scheduler>,
    );
    Rig { fake, clock, c, layout }
}

impl Rig {
    fn act(&self, a: Action) {
        self.c.act(a);
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

    fn camera(&self) -> Camera {
        self.fake.camera.get()
    }

    fn see(&self, v: usize) -> Visibility {
        visibility(&self.camera(), VIEWPORT, &self.layout, v)
    }

    /// What a person does with the wheel or the mouse: change the view without the controller.
    fn person_pans(&self, dx: f64) {
        let c = self.camera();
        self.fake.camera.set(Camera { tx: c.tx + dx, ..c });
    }

    fn at(&self, index: usize) {
        self.act(Action::Seek(index));
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[test]
fn the_first_step_is_drawn_in_full_at_once_with_vertex_zero_in_view() {
    let r = rig();
    let d = r.last_draw();
    assert_eq!((d.index, d.full, d.follow), (0, true, true));
    assert_eq!(r.see(0), Visibility::Inside);
    assert!(!r.c.is_dirty() && d.notice.is_empty());
}

#[test]
fn next_previous_and_restart_move_one_step_and_say_what_happened() {
    let r = rig();
    r.act(Action::Next);
    r.act(Action::Next);
    assert_eq!(r.c.index(), 2);
    assert!(r.last_said().starts_with("Step 2 of 252."), "{}", r.last_said());
    r.act(Action::Previous);
    assert_eq!(r.c.index(), 1);
    r.act(Action::Restart);
    assert_eq!(r.c.index(), 0);
}

#[test]
fn a_button_with_nowhere_to_go_says_so_and_draws_nothing() {
    let r = rig();
    let draws = r.fake.draws.borrow().len();
    r.act(Action::Previous);
    r.act(Action::Restart);
    r.act(Action::PreviousComponent);
    assert_eq!(r.said(), vec![AT_FIRST, AT_FIRST, NO_EARLIER_COMPONENT]);
    r.at(252);
    r.act(Action::Next);
    r.act(Action::NextComponent);
    r.act(Action::JumpFinal);
    assert_eq!(&r.said()[3..], &[AT_LAST, NO_LATER_COMPONENT, AT_LAST]);
    assert_eq!(r.fake.draws.borrow().len(), draws + 1, "only the seek drew");
}

#[test]
fn seek_goes_straight_to_a_step_and_is_clamped_to_the_end() {
    let r = rig();
    r.at(100);
    assert_eq!(r.c.index(), 100);
    r.at(9999);
    assert_eq!(r.c.index(), 252);
}

#[test]
fn selecting_never_changes_the_step_or_the_view() {
    let r = rig();
    r.at(40);
    let camera = r.camera();
    r.act(Action::Select(Target::Vertex(5)));
    r.act(Action::Select(Target::Edge(7)));
    assert_eq!((r.c.index(), r.camera(), r.fake.writes.borrow().len()), (40, camera, 0));
    assert_eq!(r.c.choices().selected, Some(Target::Edge(7)));
    assert_eq!(r.c.choices().roving, Target::Edge(7));
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[test]
fn next_from_34_to_35_brings_the_new_vertex_into_view_and_previous_brings_the_old_one_back() {
    let r = rig();
    r.at(34);
    assert_eq!(r.camera(), start(&r.layout), "a seek never moves the view");
    r.act(Action::Next);
    assert_eq!(r.see(33), Visibility::Inside);
    assert_eq!(r.camera().scale, 1.0, "following never zooms");
    r.act(Action::Previous);
    assert_eq!(r.see(3), Visibility::Inside);
}

#[test]
fn with_following_off_a_step_never_moves_the_view_but_the_notice_still_runs() {
    let r = rig();
    r.act(Action::SetFollow(false));
    r.at(34);
    let camera = r.camera();
    r.act(Action::Next);
    assert_eq!(r.camera(), camera);
    assert_eq!(
        r.last_draw().notice,
        "Active vertex v33 is off-screen; use Zoom to active vertex."
    );
}

#[test]
fn a_seek_leaves_the_view_and_shows_the_notice_whether_following_is_on_or_off() {
    for follow in [true, false] {
        let r = rig();
        r.act(Action::SetFollow(follow));
        r.at(74);
        assert_eq!(r.camera(), start(&r.layout));
        assert_eq!(
            r.last_draw().notice,
            "Active vertex v63 is off-screen; use Zoom to active vertex."
        );
    }
}

#[test]
fn the_first_next_after_a_far_seek_puts_a_vertex_that_is_off_the_screen_in_the_middle() {
    let r = rig();
    r.at(74);
    r.act(Action::Next);
    let v = r.c.trace().snapshot(75).unwrap().attention().unwrap();
    assert_eq!(r.see(v), Visibility::Inside);
}

#[test]
fn restart_is_stationary_and_says_if_the_vertex_is_far_away() {
    let r = rig();
    r.act(Action::SetFollow(false));
    r.at(74);
    r.act(Action::Restart);
    assert_eq!(r.c.index(), 0);
    assert_eq!(r.camera(), start(&r.layout));
}

#[test]
fn next_component_centres_the_closure_region_even_with_following_off_and_does_not_turn_following_on() {
    let r = rig();
    r.act(Action::SetFollow(false));
    r.act(Action::NextComponent);
    assert_eq!(r.c.index(), 74);
    assert_eq!(r.see(63), Visibility::Inside);
    assert!(!r.c.choices().follow && !r.c.memory().follow);
    assert!(
        r.last_said().starts_with("Step 74 of 252. Component C1 closes at v63"),
        "{}",
        r.last_said()
    );
}

#[test]
fn previous_component_and_jump_final_centre_their_destinations() {
    let r = rig();
    r.at(77);
    r.act(Action::PreviousComponent);
    assert_eq!(r.c.index(), 74);
    r.act(Action::JumpFinal);
    assert_eq!(r.c.index(), 252);
    let members = r.c.trace().snapshot(252).unwrap().components.last().unwrap().clone();
    let middle = r.camera().to_screen(r.layout.bounds_of(&members, 0.0).unwrap().centre());
    assert!((middle.0 - VIEWPORT.0 / 2.0).abs() < 1e-6 && (middle.1 - VIEWPORT.1 / 2.0).abs() < 1e-6);
    assert_eq!(r.camera().scale, 1.0, "a jump keeps the zoom");
}

#[test]
fn a_landmark_jump_keeps_a_zoom_that_a_person_chose() {
    let r = rig();
    r.fake.camera.set(Camera { scale: 1.6, ..r.camera() });
    r.act(Action::NextComponent);
    assert_eq!(r.camera().scale, 1.6);
    assert_eq!(r.see(63), Visibility::Inside);
}

#[test]
fn a_jump_that_is_a_boundary_does_not_move_the_view() {
    let r = rig();
    r.at(251);
    let camera = r.camera();
    r.act(Action::NextComponent);
    assert_eq!((r.camera(), r.last_said()), (camera, NO_LATER_COMPONENT.to_owned()));
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[test]
fn a_view_that_a_person_changed_turns_following_off_once_and_says_so() {
    let r = rig();
    r.at(34);
    r.person_pans(120.0);
    r.act(Action::Next);
    assert!(!r.c.choices().follow);
    assert_eq!(r.said().iter().filter(|s| *s == FOLLOW_OFF).count(), 1);
    assert_eq!(
        r.fake.writes.borrow().len(),
        0,
        "the controller did not pan over the person's pan"
    );
    r.person_pans(40.0);
    r.act(Action::Next);
    assert_eq!(r.said().iter().filter(|s| *s == FOLLOW_OFF).count(), 1, "said once");
}

#[test]
fn a_pan_by_the_controller_never_turns_following_off() {
    let r = rig();
    r.at(34);
    r.act(Action::Next);
    r.act(Action::Next);
    r.act(Action::Previous);
    assert!(r.c.choices().follow && r.c.memory().follow);
    assert!(!r.said().iter().any(|s| s == FOLLOW_OFF));
    assert!(!r.fake.writes.borrow().is_empty());
}

#[test]
fn a_change_of_view_between_two_ticks_wins_before_the_next_tick_pans() {
    let r = rig();
    r.at(33);
    r.act(Action::Play);
    r.clock.advance();
    r.clock.advance();
    r.person_pans(-80.0);
    let written = r.fake.writes.borrow().len();
    r.clock.advance();
    assert!(!r.c.choices().follow);
    assert_eq!(r.fake.writes.borrow().len(), written, "no pan after the person's");
    assert!(r.c.is_playing(), "play goes on at the chosen speed");
}

#[test]
fn play_does_not_turn_following_back_on_and_neither_do_pause_seek_or_restart() {
    let r = rig();
    r.person_pans(10.0);
    r.act(Action::Next);
    assert!(!r.c.choices().follow);
    for a in [Action::Play, Action::Pause, Action::Seek(30), Action::Restart] {
        r.act(a);
        assert!(!r.c.choices().follow, "{a:?}");
    }
    r.act(Action::SetFollow(true));
    assert!(r.c.choices().follow && r.c.memory().follow);
}

#[test]
fn a_person_s_pan_turns_following_off_and_it_stays_off_until_the_person_turns_it_on() {
    let r = rig();
    r.person_pans(10.0);
    r.act(Action::Next);
    assert!(!r.c.choices().follow);
    r.act(Action::SetFollow(true));
    r.act(Action::Next);
    assert!(r.c.choices().follow);
}

#[test]
fn while_a_gesture_is_unsettled_the_controller_makes_no_pan_and_a_step_still_commits() {
    let r = rig();
    r.at(34);
    r.c.gesture(true);
    r.act(Action::Next);
    assert_eq!(r.c.index(), 35);
    assert!(r.fake.writes.borrow().is_empty());
    assert_eq!(r.see(33), Visibility::OffScreen);
    r.c.gesture(false);
    assert!(r.fake.writes.borrow().is_empty(), "no surprise pan after the gesture ends");
}

#[test]
fn the_end_of_a_gesture_that_moved_the_view_turns_following_off() {
    let r = rig();
    r.c.gesture(true);
    r.person_pans(60.0);
    r.c.gesture(false);
    assert!(!r.c.choices().follow);
    assert!(r.said().iter().any(|s| s == FOLLOW_OFF));
}

#[test]
fn the_end_of_a_gesture_that_did_not_move_the_view_leaves_following_on() {
    let r = rig();
    r.c.gesture(true);
    r.c.gesture(false);
    assert!(r.c.choices().follow && r.said().is_empty());
}

#[test]
fn observing_the_view_after_a_wheel_or_a_key_does_the_same_as_a_step_would() {
    let r = rig();
    r.c.observe_view();
    assert!(r.c.choices().follow, "no change, so nothing happens");
    r.person_pans(5.0);
    r.c.observe_view();
    assert!(!r.c.choices().follow);
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[test]
fn zoom_puts_the_attention_vertex_in_the_middle_at_a_readable_scale_and_leaves_following_alone() {
    let r = rig();
    r.at(35);
    r.act(Action::Fit);
    assert_eq!(r.camera().scale, view::MAX_FIT);
    assert!(r.c.choices().follow);
    assert_eq!(r.see(33), Visibility::Inside);
    assert!(r.c.memory().follow);
    assert_eq!(r.last_said(), "Zoomed to v33.");
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[test]
fn a_view_notice_is_said_when_the_state_changes_to_one_that_is_not_inside_and_not_at_every_step() {
    let r = rig();
    r.act(Action::SetFollow(false));
    r.at(34);
    let attention = |k: usize| r.c.trace().snapshot(k).unwrap().attention().unwrap();
    let state = |k: usize| visibility(&r.camera(), VIEWPORT, &r.layout, attention(k));
    let before = r.said().len();
    let mut expected = 0;
    let mut last = state(34);
    for k in 35..=40 {
        r.act(Action::Next);
        let now = state(k);
        if now != last && now != Visibility::Inside {
            expected += 1;
        }
        last = now;
    }
    let notices = r.said()[before..]
        .iter()
        .filter(|s| s.starts_with("Active vertex") && s.contains("use Zoom to active vertex"))
        .count();
    assert_eq!(
        notices, expected,
        "said exactly when the state changed to near the edge or off-screen"
    );
    assert!(r.said().len() - before > notices, "the steps themselves are said as well");
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[test]
fn play_steps_once_per_tick_follows_and_stops_at_the_end_without_looping() {
    let r = rig();
    r.act(Action::Play);
    assert!(r.c.is_playing());
    assert_eq!(*r.fake.playing.borrow(), vec![true]);
    let mut ticks = 0;
    while r.clock.advance() {
        ticks += 1;
        let v = r.c.trace().snapshot(r.c.index()).unwrap().attention().unwrap();
        assert_eq!(r.see(v), Visibility::Inside, "step {}", r.c.index());
    }
    assert_eq!((ticks, r.c.index()), (252, 252));
    assert!(!r.c.is_playing());
    assert_eq!(r.clock.pending(), 0);
    r.act(Action::Play);
    assert_eq!(r.last_said(), AT_LAST);
}

#[test]
fn playback_says_only_closures_and_the_end_aloud() {
    let r = rig();
    r.act(Action::Play);
    while r.clock.advance() {}
    let spoken: Vec<String> = r.said().into_iter().filter(|s| s.starts_with("Step ")).collect();
    assert_eq!(spoken.len(), 11, "ten closures and the end");
    assert!(spoken.iter().take(10).all(|s| s.contains("closes at")));
    assert!(spoken[10].contains("Complete"));
}

#[test]
fn the_default_speed_is_400_ms_and_a_new_speed_replaces_the_waiting_tick() {
    let r = rig();
    r.act(Action::Play);
    assert_eq!(r.clock.delay(), Some(400));
    r.act(Action::SetSpeed(Speed::Faster));
    assert_eq!((r.clock.pending(), r.clock.delay()), (1, Some(100)));
    r.act(Action::Pause);
    r.act(Action::SetSpeed(Speed::Slow));
    assert_eq!(r.clock.pending(), 0);
}

#[test]
fn pausing_says_where_and_cancels_the_tick_and_a_stale_tick_does_nothing() {
    let r = rig();
    r.act(Action::Play);
    r.clock.keep_cancelled();
    r.clock.advance();
    r.act(Action::Pause);
    assert!(r.last_said().starts_with("Paused at step 1 of 252"));
    r.clock.advance();
    assert_eq!(r.c.index(), 1);
    assert!(!r.c.is_playing());
}

#[test]
fn every_stop_counts_in_the_epoch() {
    let r = rig();
    r.act(Action::Play);
    let epoch = r.c.playback_epoch();
    r.act(Action::Next);
    assert!(r.c.playback_epoch() > epoch);
    r.act(Action::Play);
    let epoch = r.c.playback_epoch();
    r.act(Action::Seek(10));
    assert!(r.c.playback_epoch() > epoch);
}

#[test]
fn a_timer_that_cannot_be_scheduled_leaves_playback_paused_and_says_so() {
    let r = rig();
    r.clock.fail_next_timers(1);
    r.act(Action::Play);
    assert!(!r.c.is_playing() && !r.c.is_dirty());
    assert_eq!(r.fake.playback_failures.borrow().len(), 1);
}

#[test]
fn a_play_button_that_cannot_be_written_leaves_playback_paused() {
    let r = rig();
    r.fake.fail_playing.set(true);
    r.act(Action::Play);
    assert!(!r.c.is_playing());
    assert_eq!((r.clock.pending(), r.fake.playback_failures.borrow().len()), (0, 1));
}

#[test]
fn moving_the_range_many_times_between_frames_draws_once() {
    let r = rig();
    let draws = r.fake.draws.borrow().len();
    for k in [3, 9, 14, 21] {
        r.c.seek_input(k);
    }
    assert_eq!((r.clock.pending_frames(), r.fake.draws.borrow().len()), (1, draws));
    r.clock.advance_frame();
    assert_eq!((r.c.index(), r.fake.draws.borrow().len()), (21, draws + 1));
}

#[test]
fn letting_the_range_go_draws_at_once_and_says_where_it_is_once() {
    let r = rig();
    r.c.seek_input(10);
    r.c.seek_release(12);
    assert_eq!((r.c.index(), r.clock.pending_frames()), (10, 0));
    assert_eq!(
        r.said().iter().filter(|s| s.starts_with("Step 10 of 252")).count(),
        1,
        "where it is, said once"
    );
}

#[test]
fn a_frame_that_cannot_be_asked_for_draws_at_once_and_warns() {
    let r = rig();
    r.clock.fail_next_frames(1);
    r.c.seek_input(8);
    assert_eq!((r.c.index(), r.fake.warnings.borrow().len()), (8, 1));
}

#[test]
fn a_seek_never_moves_the_view_however_far_it_goes() {
    let r = rig();
    let camera = r.camera();
    for k in [10, 74, 150, 252, 3] {
        r.c.seek_input(k);
        r.clock.advance_frame();
        assert_eq!(r.camera(), camera);
    }
    assert!(r.fake.writes.borrow().is_empty());
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[test]
fn a_selected_component_is_let_go_of_in_the_same_step_that_removes_it_and_the_change_is_said() {
    let r = rig();
    r.at(81);
    r.act(Action::SelectComponent(Some(3)));
    assert_eq!(r.c.choices().component, Some(3));
    assert_eq!(r.last_draw().component, Some(3));
    r.act(Action::Previous);
    assert_eq!((r.c.choices().component, r.last_draw().component), (None, None));
    assert!(
        r.last_said().contains("Component C3 does not exist at this step"),
        "{}",
        r.last_said()
    );
}

#[test]
fn only_an_emitted_component_can_be_selected() {
    let r = rig();
    r.at(80);
    r.act(Action::SelectComponent(Some(3)));
    assert_eq!(r.c.choices().component, None);
    r.at(81);
    r.act(Action::SelectComponent(Some(3)));
    r.act(Action::SelectComponent(None));
    assert_eq!(r.c.choices().component, None);
}

#[test]
fn selecting_a_component_changes_neither_the_step_nor_the_view() {
    let r = rig();
    r.at(100);
    let camera = r.camera();
    r.act(Action::SelectComponent(Some(2)));
    assert_eq!((r.c.index(), r.camera()), (100, camera));
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[test]
fn a_failed_draw_keeps_the_step_marks_the_page_dirty_stops_playback_and_the_next_action_repairs_in_full() {
    let r = rig();
    r.at(20);
    r.act(Action::Play);
    r.fake.fail_draws.set(1);
    r.clock.advance();
    assert_eq!(r.c.index(), 20);
    assert!(r.c.is_dirty() && !r.c.is_playing());
    r.act(Action::Next);
    let draws = r.fake.draws.borrow();
    let (repair, step) = (&draws[draws.len() - 2], &draws[draws.len() - 1]);
    assert_eq!((repair.index, repair.full, step.index, step.full), (20, true, 21, false));
    assert!(!r.c.is_dirty());
}

#[test]
fn a_press_with_nowhere_to_go_still_repairs() {
    let r = rig();
    r.fake.fail_draws.set(1);
    r.act(Action::Select(Target::Vertex(2)));
    assert!(r.c.is_dirty());
    r.act(Action::Previous);
    assert!(!r.c.is_dirty());
    assert_eq!(r.last_said(), AT_FIRST);
    assert!(r.last_draw().full);
}

#[test]
fn a_repair_that_fails_stops_everything_and_the_third_try_works() {
    let r = rig();
    r.fake.fail_draws.set(2);
    r.act(Action::Next);
    r.act(Action::Next);
    assert_eq!(r.c.index(), 0);
    r.act(Action::Next);
    assert_eq!(r.c.index(), 1);
}

#[test]
fn a_view_that_cannot_be_written_keeps_the_step_and_the_old_view_and_does_not_advance_the_baseline() {
    let r = rig();
    r.at(34);
    let (camera, memory) = (r.camera(), r.c.memory());
    r.fake.fail_writes.set(1);
    r.act(Action::Next);
    assert_eq!(r.c.index(), 34, "the step was not published");
    assert_eq!(r.camera(), camera, "the old view is back");
    assert_eq!(r.c.memory(), memory, "the baseline did not move");
    assert!(r.c.is_dirty());
    assert_eq!(r.fake.failures.borrow().len(), 1);
    // The next action draws the committed step in full first, and then the step goes through with its pan.
    r.act(Action::Next);
    assert_eq!(r.c.index(), 35);
    assert!(!r.c.is_dirty());
}

#[test]
fn a_failed_landmark_pan_keeps_the_step_and_is_repaired_by_the_next_action() {
    let r = rig();
    r.fake.fail_writes.set(1);
    r.act(Action::NextComponent);
    assert_eq!(r.c.index(), 0);
    assert!(r.c.is_dirty());
    r.act(Action::NextComponent);
    assert_eq!(r.c.index(), 74);
    assert_eq!(r.see(63), Visibility::Inside);
}

#[test]
fn a_fit_that_cannot_be_written_puts_the_view_back_and_marks_the_page_dirty() {
    let r = rig();
    r.at(35);
    let camera = r.camera();
    r.fake.fail_writes.set(1);
    r.act(Action::Fit);
    assert_eq!(r.camera(), camera);
    assert!(r.c.is_dirty());
}

#[test]
fn a_failed_selection_stops_playback_so_that_a_waiting_tick_cannot_draw_on_a_page_that_is_out_of_date() {
    let r = rig();
    r.act(Action::Play);
    r.clock.keep_cancelled();
    r.fake.fail_draws.set(1);
    r.act(Action::Select(Target::Vertex(1)));
    assert!(r.c.is_dirty() && !r.c.is_playing());
    let draws = r.fake.draws.borrow().len();
    r.clock.advance();
    assert_eq!((r.c.index(), r.fake.draws.borrow().len()), (0, draws));
}

#[test]
fn teardown_stops_everything_and_makes_every_later_call_do_nothing() {
    let r = rig();
    r.act(Action::Play);
    r.c.seek_input(9);
    r.c.teardown();
    assert_eq!(r.clock.teardowns(), 1);
    let draws = r.fake.draws.borrow().len();
    r.act(Action::Next);
    r.c.seek_input(3);
    r.c.seek_release(3);
    r.c.gesture(true);
    r.c.observe_view();
    r.clock.advance();
    r.clock.advance_frame();
    assert_eq!((r.c.index(), r.fake.draws.borrow().len()), (0, draws));
}
