use super::*;
use crate::controller::DrawFailure;
use crate::projection::{Projection, View};
use crate::scheduler::ManualScheduler;
use service_sim_engine::trace::Scenario;
use service_sim_engine::{Engine, Limits};
use std::cell::Cell;
use std::collections::{BTreeMap, BTreeSet};

const FIXTURES: &str = include_str!("../../../dess/s1/fixtures.json");

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// What one fake page holds: how its draws go, what it shows, and whether the host that it belongs to has been dropped.
struct Page {
    fail_draws: Vec<usize>,
    draws: usize,
    presence: Presence,
    /// The canonical form of what is on the page, or `None` after a draw that failed.
    drawn: Option<String>,
}

struct FakeHost {
    page: Rc<RefCell<Page>>,
    dropped: Rc<Cell<usize>>,
}

impl Host for FakeHost {
    fn presence(&self) -> Presence {
        self.page.borrow().presence
    }

    fn draw(&mut self, p: &Projection, _full: bool) -> Result<(), DrawFailure> {
        let mut page = self.page.borrow_mut();
        page.draws += 1;
        if page.fail_draws.contains(&page.draws) {
            page.drawn = None;
            return Err(DrawFailure::Failed("a write failed".into()));
        }
        page.drawn = Some(p.canonical());
        Ok(())
    }

    fn announce(&mut self, _text: &str) {}

    fn report(&mut self, _text: &str) {}

    fn playing_changed(&mut self, _playing: bool) {}
}

impl Drop for FakeHost {
    fn drop(&mut self) {
        self.dropped.set(self.dropped.get() + 1);
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The page that the fake sessions share: what is shown, where the focus is, what was built and removed, and what is
/// made to fail.
#[derive(Default)]
struct World {
    next_token: u32,
    built: Vec<u32>,
    discarded: Vec<u32>,
    finalized: Vec<u32>,
    visible: BTreeSet<u32>,
    focus: Option<u32>,
    pages: BTreeMap<u32, Rc<RefCell<Page>>>,
    hosts_dropped: Rc<Cell<usize>>,
    undos: usize,
    fail_build: bool,
    first_draw_fails: bool,
    fail_verify: bool,
    /// The step of the handoff, from 0, that fails: 0 shows the new page, 1 hides the old, and 2 moves the focus.
    fail_publish_step: Option<usize>,
    fail_undo: bool,
    /// The handoff leaves the new page hidden, as a browser might if it were hidden in the middle.
    hide_candidate_on_publish: bool,
    /// The step of the handoff, from 0, that is skipped without a word: `publish` returns `Ok` and has not done it.
    skipped_publish_step: Option<usize>,
}

struct Fake(Rc<RefCell<World>>);

impl SessionHost for Fake {
    type H = FakeHost;
    type S = ManualScheduler;
    type Token = u32;

    fn build(&mut self, spec: &Spec) -> Result<Candidate<FakeHost, ManualScheduler, u32>, String> {
        let (id, page, dropped) = {
            let mut w = self.0.borrow_mut();
            if w.fail_build {
                return Err("the host could not be staged".to_owned());
            }
            let id = w.next_token;
            w.next_token += 1;
            let page = Rc::new(RefCell::new(Page {
                fail_draws: if w.first_draw_fails { vec![1] } else { vec![] },
                draws: 0,
                presence: Presence::Live,
                drawn: None,
            }));
            (id, page, Rc::clone(&w.hosts_dropped))
        };
        let host = FakeHost {
            page: Rc::clone(&page),
            dropped,
        };
        let controller = Controller::new(host, ManualScheduler::default(), Rc::clone(&spec.factory), View::default())
            .map_err(|e| e.message())?;
        let mut w = self.0.borrow_mut();
        w.pages.insert(id, page);
        w.built.push(id);
        Ok(Candidate { controller, token: id })
    }

    fn verify(&mut self, _candidate: &Candidate<FakeHost, ManualScheduler, u32>) -> Result<(), String> {
        if self.0.borrow().fail_verify {
            Err("the data cells are not on the page".to_owned())
        } else {
            Ok(())
        }
    }

    fn publish(&mut self, from: Option<&u32>, to: &u32) -> Result<(), String> {
        let mut w = self.0.borrow_mut();
        let steps: [&dyn Fn(&mut World); 3] = [
            &|w| {
                w.visible.insert(*to);
            },
            &|w| {
                if let Some(from) = from {
                    w.visible.remove(from);
                }
            },
            &|w| w.focus = Some(*to),
        ];
        for (index, step) in steps.iter().enumerate() {
            if w.fail_publish_step == Some(index) {
                return Err(format!("step {index} of the handoff failed"));
            }
            if w.skipped_publish_step == Some(index) {
                continue;
            }
            step(&mut w);
        }
        if w.hide_candidate_on_publish {
            w.pages[to].borrow_mut().presence = Presence::Hidden;
        }
        Ok(())
    }

    fn verify_published(&mut self, from: Option<&u32>, to: &u32) -> Result<(), String> {
        let w = self.0.borrow();
        if !w.visible.contains(to) {
            return Err("the new page is not shown: it is still staged".to_owned());
        }
        if from.is_some_and(|from| w.visible.contains(from)) || w.visible.len() != 1 {
            return Err("the old page is still shown beside the new one".to_owned());
        }
        if w.focus != Some(*to) {
            return Err("the focus is not in the new page".to_owned());
        }
        Ok(())
    }

    fn undo_publish(&mut self, from: Option<&u32>, to: &u32) -> Result<(), String> {
        let mut w = self.0.borrow_mut();
        w.undos += 1;
        if w.fail_undo {
            return Err("the handoff could not be undone".to_owned());
        }
        w.visible.remove(to);
        if let Some(from) = from {
            w.visible.insert(*from);
        }
        w.focus = from.copied();
        Ok(())
    }

    fn discard(&mut self, token: u32) {
        let mut w = self.0.borrow_mut();
        w.discarded.push(token);
        w.visible.remove(&token);
        w.pages.remove(&token);
    }

    fn finalize(&mut self, token: u32) {
        let mut w = self.0.borrow_mut();
        w.finalized.push(token);
        w.visible.remove(&token);
        w.pages.remove(&token);
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
fn fixture(name: &str, servers: usize, slices: u64, pops: u64) -> Spec {
    let fixtures: serde_json::Value = serde_json::from_str(FIXTURES).expect("fixtures");
    let scenario = Scenario::fixture(name, &fixtures[name]).expect("a fixture");
    Spec {
        id: name.to_owned(),
        label: name.to_owned(),
        servers,
        factory: Rc::new(move || {
            Engine::new(
                &scenario.config_id,
                scenario.seed,
                scenario.config,
                &scenario.script,
                None,
                Limits::default(),
            )
        }),
        expected: Expected {
            slices,
            pops,
            max_slices: slices * 4,
            max_pops: pops * 4,
        },
    }
}

/// E0: two servers, twelve slices and nineteen pops.
fn e0() -> Spec {
    fixture("E0", 2, 12, 19)
}

/// E3-NOOP: one server, nine slices and nine pops.
fn noop() -> Spec {
    fixture("E3-NOOP", 1, 9, 9)
}

/// The frozen generator at the manifest seed with 360 arrivals: three servers, 487 slices and 1,009 pops.
fn scaled() -> Spec {
    let mut scenario = Scenario::mixed_outage_7(20_261_106).expect("a scenario");
    scenario.params.as_mut().expect("parameters").arrivals = 360;
    Spec {
        id: "scaled".into(),
        label: "scaled".into(),
        servers: 3,
        factory: Rc::new(move || {
            Engine::new(
                &scenario.config_id,
                scenario.seed,
                scenario.config,
                &scenario.script,
                scenario.params.as_ref(),
                Limits::default(),
            )
        }),
        expected: Expected {
            slices: 487,
            pops: 1009,
            max_slices: 1000,
            max_pops: 2000,
        },
    }
}

type Session = Coordinator<Fake>;

/// A coordinator that has E0 on the page, and the world it is in.
fn started() -> (Session, Rc<RefCell<World>>) {
    let world = Rc::new(RefCell::new(World::default()));
    let mut session = Coordinator::new(Fake(Rc::clone(&world)));
    assert_eq!(session.switch(e0()), SwitchOutcome::Replaced);
    (session, world)
}

fn current(session: &Session) -> Controller<FakeHost, ManualScheduler> {
    session.target().current().expect("a controller")
}

/// What the old session has to be after any failure: where it was, with the page showing it.
fn assert_old_is_untouched(
    session: &Session,
    world: &Rc<RefCell<World>>,
    old: &Controller<FakeHost, ManualScheduler>,
    slice: u64,
) {
    assert_eq!(session.active_id(), Some("E0"));
    assert_eq!((old.committed_slice(), old.selected()), (slice, old.selected()));
    assert_eq!(current(session).committed_slice(), slice);
    let w = world.borrow();
    assert_eq!(w.visible, BTreeSet::from([0]), "the old page is the one that is shown");
    assert_eq!(w.focus, Some(0), "and has the focus");
    assert!(w.finalized.is_empty(), "nothing was removed as replaced");
    assert!(!old.is_dirty());
    assert_eq!(
        w.pages[&0].borrow().drawn,
        Some(old.committed().canonical()),
        "and shows its committed slice"
    );
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[test]
fn a_scenario_is_run_to_its_end_and_checked_against_what_it_is_known_to_come_to() {
    assert_eq!(e0().preflight(), Ok(Extent { slices: 12, pops: 19 }));
    assert_eq!(noop().preflight(), Ok(Extent { slices: 9, pops: 9 }));
    assert_eq!(scaled().preflight(), Ok(Extent { slices: 487, pops: 1009 }));
}

#[test]
fn a_scenario_that_is_refused_is_refused_for_a_reason_and_without_building_anything() {
    let mut none = e0();
    none.servers = 0;
    assert_eq!(none.preflight(), Err(PreflightError::Servers(0)));
    let mut many = e0();
    many.servers = 5;
    assert_eq!(many.preflight(), Err(PreflightError::Servers(5)));
    let mut slices = e0();
    slices.expected.max_slices = 5;
    assert_eq!(slices.preflight(), Err(PreflightError::Ceiling { what: "slices", limit: 5 }));
    let mut pops = e0();
    pops.expected.max_pops = 3;
    assert_eq!(pops.preflight(), Err(PreflightError::Ceiling { what: "pops", limit: 3 }));
    let mut wrong = e0();
    wrong.expected.slices = 13;
    assert_eq!(
        wrong.preflight(),
        Err(PreflightError::Mismatch {
            found: Extent { slices: 12, pops: 19 },
            expected: Extent { slices: 13, pops: 19 }
        })
    );
    let mut broken = e0();
    broken.factory = Rc::new(|| Err(service_sim_engine::Error::Invalid("no engine".into())));
    assert!(matches!(broken.preflight(), Err(PreflightError::Engine(_))));
}

#[test]
fn the_first_scenario_is_built_published_and_made_the_target() {
    let (session, world) = started();
    assert_eq!(session.active_id(), Some("E0"));
    assert_eq!(session.state(), State::Committed);
    assert_eq!(current(&session).committed_slice(), 0);
    let w = world.borrow();
    assert_eq!(
        (w.built.clone(), w.visible.clone(), w.focus),
        (vec![0], BTreeSet::from([0]), Some(0))
    );
}

#[test]
fn a_scenario_with_the_same_number_of_servers_switches_in_place_and_builds_nothing() {
    let (mut session, world) = started();
    let old = current(&session);
    old.next();
    old.next();
    let mut again = e0();
    again.id = "E0-again".into();
    assert_eq!(session.switch(again), SwitchOutcome::InPlace(Outcome::Done));
    assert_eq!(session.active_id(), Some("E0-again"));
    assert_eq!(world.borrow().built, vec![0], "no second surface was built");
    assert_eq!(current(&session).committed_slice(), 0, "the history starts again");
    assert!(!old.is_disposed(), "and it is the same controller");
}

#[test]
fn a_same_size_switch_whose_draw_fails_keeps_the_old_scenario() {
    let (mut session, world) = started();
    let old = current(&session);
    old.next();
    let draws = world.borrow().pages[&0].borrow().draws;
    world.borrow().pages[&0].borrow_mut().fail_draws.push(draws + 1);
    let mut again = e0();
    again.id = "E0-again".into();
    assert_eq!(session.switch(again), SwitchOutcome::InPlace(Outcome::Failed));
    assert_eq!(
        session.active_id(),
        Some("E0"),
        "the label did not move to a scenario that was not drawn"
    );
    assert_eq!(old.committed_slice(), 1, "and the cursor is where it was");
}

#[test]
fn a_different_number_of_servers_replaces_the_session_and_keeps_the_speed_without_playing() {
    let (mut session, world) = started();
    let old = current(&session);
    old.set_speed(200);
    old.play();
    old.select_job(Some(0));
    assert_eq!(session.switch(noop()), SwitchOutcome::Replaced);
    let new = current(&session);
    assert_eq!(session.active_id(), Some("E3-NOOP"));
    assert_eq!(
        (new.committed_slice(), new.speed_ms(), new.is_playing(), new.selected()),
        (0, 200, false, None)
    );
    assert!(old.is_disposed() && !new.is_disposed());
    let w = world.borrow();
    assert_eq!(
        (w.built.clone(), w.finalized.clone()),
        (vec![0, 1], vec![0]),
        "the old page was removed, once"
    );
    assert_eq!((w.visible.clone(), w.focus), (BTreeSet::from([1]), Some(1)));
    assert_eq!(session.state(), State::Committed);
}

#[test]
fn a_request_that_is_not_valid_changes_nothing_and_does_not_stop_the_playback() {
    let (mut session, world) = started();
    let old = current(&session);
    old.play();
    let mut bad = noop();
    bad.servers = 9;
    assert_eq!(session.switch(bad), SwitchOutcome::Rejected(PreflightError::Servers(9)));
    assert!(old.is_playing(), "a request that was never valid does not own navigation");
    assert_eq!(session.state(), State::Committed);
    assert_eq!(world.borrow().built, vec![0]);
}

/// Each way that a candidate can fail before the handoff, with the stage that it is reported at and whether a candidate
/// had been built.
type StagingFailure = (&'static str, Stage, fn(&mut World), bool);

fn staging_failures() -> Vec<StagingFailure> {
    vec![
        ("the host cannot be staged", Stage::Build, |w| w.fail_build = true, false),
        (
            "the first draw of the candidate fails and leaves it dirty",
            Stage::Validate,
            |w| w.first_draw_fails = true,
            true,
        ),
        (
            "the readback of the candidate's page fails",
            Stage::Validate,
            |w| w.fail_verify = true,
            true,
        ),
    ]
}

#[test]
fn a_candidate_that_fails_before_the_handoff_leaves_the_old_session_as_it_was() {
    for (what, stage, make_fail, built) in staging_failures() {
        let (mut session, world) = started();
        let old = current(&session);
        old.next();
        old.next();
        old.next();
        old.select_job(Some(0));
        let selected = old.selected();
        old.set_speed(500);
        old.play();
        make_fail(&mut world.borrow_mut());
        let outcome = session.switch(noop());
        let SwitchOutcome::RolledBack(rollback) = outcome else {
            panic!("{what}: {outcome:?}")
        };
        assert_eq!(rollback.stage, stage, "{what}");
        assert!(!rollback.undo_failed, "{what}: nothing was handed off, so nothing was undone");
        assert_eq!(rollback.repair, None, "{what}: a clean page is not repaired");
        assert_eq!(session.state(), State::RolledBack, "{what}");
        assert_old_is_untouched(&session, &world, &old, 3);
        assert_eq!(old.selected(), selected, "{what}: the selection is kept");
        assert!(
            !old.is_playing(),
            "{what}: the request stopped the old playback, which is not restored"
        );
        assert_eq!(old.speed_ms(), 500, "{what}: and the speed is kept");
        let w = world.borrow();
        assert_eq!(
            w.discarded.len(),
            usize::from(built),
            "{what}: a candidate that was built is removed"
        );
        assert_eq!(w.undos, 0, "{what}");
    }
}

#[test]
fn a_candidate_that_was_built_dirty_is_refused_although_its_constructor_succeeded() {
    let (mut session, world) = started();
    world.borrow_mut().first_draw_fails = true;
    let SwitchOutcome::RolledBack(rollback) = session.switch(noop()) else {
        panic!("rolled back")
    };
    assert_eq!(rollback.stage, Stage::Validate);
    assert!(rollback.reason.contains("dirty"), "{}", rollback.reason);
}

#[test]
fn a_handoff_that_fails_at_any_step_is_undone_and_the_old_page_is_repaired() {
    for step in 0..3 {
        let (mut session, world) = started();
        let old = current(&session);
        old.next();
        old.next();
        world.borrow_mut().fail_publish_step = Some(step);
        let SwitchOutcome::RolledBack(rollback) = session.switch(noop()) else {
            panic!("step {step}")
        };
        assert_eq!(rollback.stage, Stage::Publish, "step {step}");
        assert!(!rollback.undo_failed, "step {step}");
        assert_eq!(
            rollback.repair,
            Some(Outcome::Done),
            "step {step}: the old picture is drawn again, since the handoff may have disturbed it"
        );
        assert_old_is_untouched(&session, &world, &old, 2);
        let w = world.borrow();
        assert_eq!(
            (w.undos, w.discarded.clone()),
            (1, vec![1]),
            "step {step}: the handoff was undone and the candidate removed"
        );
        assert_eq!(w.built, vec![0, 1]);
    }
}

#[test]
fn a_handoff_that_reports_success_and_is_incomplete_is_not_committed() {
    // Each step in turn is skipped without an error: the page is not shown, the old page is not hidden, the focus is
    // not moved. The candidate is clean, at slice 0 and Live to the controller the whole time, since a staged host is
    // as live as a shown one, so only the readback sees it.
    for (step, expected) in [
        (0, "still staged"),
        (1, "still shown beside"),
        (2, "focus is not in the new page"),
    ] {
        let (mut session, world) = started();
        let old = current(&session);
        old.next();
        world.borrow_mut().skipped_publish_step = Some(step);
        let SwitchOutcome::RolledBack(rollback) = session.switch(noop()) else {
            panic!("step {step}")
        };
        assert_eq!(rollback.stage, Stage::Publish, "step {step}");
        assert!(rollback.reason.contains(expected), "step {step}: {}", rollback.reason);
        assert!(!rollback.undo_failed, "step {step}");
        assert_eq!(
            rollback.repair,
            Some(Outcome::Done),
            "step {step}: the old page is repaired, since it was handed off"
        );
        assert_old_is_untouched(&session, &world, &old, 1);
        let w = world.borrow();
        assert_eq!(
            (w.undos, w.discarded.clone()),
            (1, vec![1]),
            "step {step}: the handoff was undone and the candidate removed"
        );
        assert_eq!(session.state(), State::RolledBack, "step {step}");
        assert!(
            !old.is_disposed(),
            "step {step}: and the old controller is still the one in use"
        );
    }
}

#[test]
fn a_handoff_that_cannot_be_undone_is_reported_and_the_old_session_is_still_the_active_one() {
    let (mut session, world) = started();
    let old = current(&session);
    {
        let mut w = world.borrow_mut();
        w.fail_publish_step = Some(2);
        w.fail_undo = true;
    }
    let SwitchOutcome::RolledBack(rollback) = session.switch(noop()) else {
        panic!("rolled back")
    };
    assert!(rollback.undo_failed, "the failure to restore the focus is not hidden");
    assert_eq!(session.active_id(), Some("E0"));
    assert_eq!(current(&session).committed_slice(), old.committed_slice());
    assert_eq!(world.borrow().discarded, vec![1], "the candidate is removed all the same");
    assert!(!old.is_disposed());
}

#[test]
fn a_candidate_that_is_not_live_after_the_handoff_is_rolled_back() {
    let (mut session, world) = started();
    world.borrow_mut().hide_candidate_on_publish = true;
    let SwitchOutcome::RolledBack(rollback) = session.switch(noop()) else {
        panic!("rolled back")
    };
    assert_eq!(rollback.stage, Stage::Publish);
    assert!(rollback.reason.contains("Hidden"), "{}", rollback.reason);
    assert_eq!(world.borrow().undos, 1);
    assert_eq!(session.active_id(), Some("E0"));
}

#[test]
fn a_rolled_back_request_can_be_followed_by_a_good_one() {
    let (mut session, world) = started();
    world.borrow_mut().fail_build = true;
    assert!(matches!(session.switch(noop()), SwitchOutcome::RolledBack(_)));
    world.borrow_mut().fail_build = false;
    assert_eq!(session.switch(noop()), SwitchOutcome::Replaced);
    assert_eq!((session.active_id(), session.state()), (Some("E3-NOOP"), State::Committed));
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[test]
fn a_hook_that_was_made_once_reaches_the_controller_that_is_current_after_a_replacement() {
    let (mut session, _world) = started();
    let hook = session.target().hook();
    assert_eq!(session.switch(noop()), SwitchOutcome::Replaced);
    let new = current(&session);
    new.set_speed(100);
    new.play();
    assert!(new.is_playing());
    hook.suspend();
    assert!(
        !new.is_playing(),
        "the hook that was made before the replacement stopped the new controller"
    );
}

#[test]
fn a_rolled_back_replacement_leaves_the_hook_pointing_at_the_old_controller() {
    let (mut session, world) = started();
    let hook = session.target().hook();
    world.borrow_mut().fail_publish_step = Some(1);
    assert!(matches!(session.switch(noop()), SwitchOutcome::RolledBack(_)));
    let old = current(&session);
    old.set_speed(100);
    old.play();
    assert!(old.is_playing());
    hook.suspend();
    assert!(
        !old.is_playing(),
        "the hook still reaches the old controller, which is the active one"
    );
}

#[test]
fn a_hook_tells_the_current_controller_that_the_panel_was_hidden_and_shown() {
    let (session, world) = started();
    let hook = session.target().hook();
    let old = current(&session);
    let page = Rc::clone(&world.borrow().pages[&0]);
    let draws = page.borrow().draws;
    // The draw fails, and so does the one repair that follows it, which leaves the page dirty.
    page.borrow_mut().fail_draws.extend([draws + 1, draws + 2]);
    assert_eq!(old.next(), Outcome::Failed);
    assert!(
        old.is_dirty(),
        "a draw that failed, and a repair that failed, leave the page dirty"
    );
    page.borrow_mut().presence = Presence::Hidden;
    hook.presence_changed();
    assert!(
        old.is_dirty(),
        "a hidden panel is not drawn on, so nothing is repaired while it is hidden"
    );
    page.borrow_mut().presence = Presence::Live;
    hook.presence_changed();
    assert!(!old.is_dirty(), "and showing the panel repairs it, through the hook");
    assert!(!old.is_playing(), "without resuming anything");
}

#[test]
fn the_old_session_is_disposed_and_removed_exactly_once() {
    let (mut session, world) = started();
    let old = current(&session);
    assert_eq!(session.switch(noop()), SwitchOutcome::Replaced);
    assert!(old.is_disposed());
    assert_eq!(world.borrow().finalized, vec![0]);
    let new = current(&session);
    session.dispose();
    session.dispose();
    assert!(new.is_disposed());
    assert_eq!(
        world.borrow().finalized,
        vec![0, 1],
        "the new one is removed once, and a second dispose removes nothing"
    );
    assert!(session.target().current().is_none() && session.active_id().is_none());
}

#[test]
fn nothing_owns_anything_in_a_circle_so_every_host_is_dropped_with_its_session() {
    let world = Rc::new(RefCell::new(World::default()));
    let dropped = Rc::clone(&world.borrow().hosts_dropped);
    let hook;
    {
        let mut session = Coordinator::new(Fake(Rc::clone(&world)));
        assert_eq!(session.switch(e0()), SwitchOutcome::Replaced);
        hook = session.target().hook();
        assert_eq!(session.switch(noop()), SwitchOutcome::Replaced);
        assert_eq!(session.switch(scaled()), SwitchOutcome::Replaced);
        assert_eq!(dropped.get(), 2, "each replaced session's host is dropped as it is replaced");
        session.dispose();
    }
    assert_eq!(dropped.get(), 3, "and the last one when the coordinator is");
    hook.presence_changed();
    hook.suspend();
    assert!(
        hook.controller().is_none(),
        "a hook that outlives the coordinator reaches nothing"
    );
}

#[test]
fn a_timer_and_a_frame_that_were_on_their_way_when_the_session_was_replaced_do_nothing() {
    let (mut session, _world) = started();
    let old = current(&session);
    old.scheduler().keep_cancelled(true);
    old.set_speed(100);
    old.play();
    old.seek(5);
    assert_eq!(session.switch(noop()), SwitchOutcome::Replaced);
    let new = current(&session);
    old.scheduler().advance_by(1000);
    while old.scheduler().advance_frame() {}
    assert_eq!(
        old.committed_slice(),
        0,
        "the old controller commits nothing after it was replaced"
    );
    assert_eq!(
        (new.committed_slice(), new.is_playing()),
        (0, false),
        "and the new one is unmoved"
    );
}

#[test]
fn scenarios_can_be_replaced_through_every_size_and_back() {
    let (mut session, world) = started();
    for spec in [noop(), scaled(), e0()] {
        let id = spec.id.clone();
        assert_eq!(session.switch(spec), SwitchOutcome::Replaced);
        assert_eq!(session.active_id(), Some(id.as_str()));
        assert_eq!(
            world.borrow().visible.len(),
            1,
            "exactly one page is shown after each replacement"
        );
    }
    assert_eq!(world.borrow().finalized, vec![0, 1, 2]);
}
