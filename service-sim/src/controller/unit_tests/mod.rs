use super::*;
use crate::projection::{View, extract, project};
use crate::scheduler::ManualScheduler;
use service_sim_engine::{Engine, Limits, trace::Scenario};
use std::{cell::RefCell, rc::Rc};

const FIXTURES: &str = include_str!("../../../../dess/s1/fixtures.json");

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// What a fake host did, shared with the test that drives it.
struct Log {
    presence: Presence,
    /// Every draw, as the hash of what it drew and whether it was full.
    draws: Vec<(String, bool)>,
    /// What the page holds, if it is known: a delta is written onto it, and a failed draw makes it unknown.
    page: Option<Projection>,
    /// The draw, counted from 1, that fails with a write error, and the one that detaches the host in its middle and
    /// returns Ok.
    fail_at: Vec<usize>,
    detach_at: Option<usize>,
    announces: Vec<String>,
    reports: Vec<String>,
    playing: Vec<bool>,
    deltas_on_unknown: usize,
}

struct Fake(Rc<RefCell<Log>>);

impl Host for Fake {
    fn presence(&self) -> Presence {
        self.0.borrow().presence
    }

    fn draw(&mut self, p: &Projection, full: bool) -> Result<(), DrawFailure> {
        let mut log = self.0.borrow_mut();
        let n = log.draws.len() + 1;
        log.draws.push((p.sha256(), full));
        if log.detach_at == Some(n) {
            // A detached element goes on accepting writes, so the draw reports no error, and the page is partly
            // written.
            log.presence = Presence::Detached;
            log.page = None;
            return Ok(());
        }
        if log.fail_at.contains(&n) {
            log.page = None;
            return Err(DrawFailure::Failed("a write failed".into()));
        }
        if !full && log.page.is_none() {
            log.deltas_on_unknown += 1;
        }
        log.page = Some(p.clone());
        Ok(())
    }

    fn announce(&mut self, text: &str) {
        self.0.borrow_mut().announces.push(text.to_owned());
    }

    fn report(&mut self, text: &str) {
        self.0.borrow_mut().reports.push(text.to_owned());
    }

    fn playing_changed(&mut self, playing: bool) {
        self.0.borrow_mut().playing.push(playing);
    }
}

type C = Controller<Fake, ManualScheduler>;

fn log() -> Rc<RefCell<Log>> {
    Rc::new(RefCell::new(Log {
        presence: Presence::Live,
        draws: Vec::new(),
        page: None,
        fail_at: Vec::new(),
        detach_at: None,
        announces: Vec::new(),
        reports: Vec::new(),
        playing: Vec::new(),
        deltas_on_unknown: 0,
    }))
}

fn factory_of(scenario: Scenario) -> Factory {
    Rc::new(move || {
        Engine::new(
            &scenario.config_id,
            scenario.seed,
            scenario.config,
            &scenario.script,
            scenario.params.as_ref(),
            Limits::default(),
        )
    })
}

fn e0() -> Factory {
    let fixtures: serde_json::Value = serde_json::from_str(FIXTURES).expect("fixtures");
    factory_of(Scenario::fixture("E0", &fixtures["E0"]).expect("a fixture"))
}

fn generated(seed: u64) -> Factory {
    factory_of(Scenario::panel(seed).expect("a panel"))
}

fn start(factory: Factory) -> (C, Rc<RefCell<Log>>) {
    let log = log();
    let c = Controller::new(Fake(Rc::clone(&log)), ManualScheduler::default(), factory, View::default())
        .expect("a controller");
    (c, log)
}

/// The projection of slice `k`, made straight from an engine that was stepped to it, which is what every route has to
/// arrive at.
fn straight(factory: &Factory, k: u64, view: &View) -> Projection {
    let mut engine = factory().expect("an engine");
    for _ in 0..k {
        engine.step().expect("a slice").expect("a slice that exists");
    }
    project(&extract(&engine, view), view)
}

fn page(log: &Rc<RefCell<Log>>) -> Option<String> {
    log.borrow().page.as_ref().map(Projection::canonical)
}

fn run_frames(c: &C) {
    while c.scheduler().advance_frame() {}
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
// An engine that fails is not an engine that has finished. The run is made to fail by a limit of five slices, which the
// sixth exceeds.

/// A generated run whose engine refuses the sixth slice.
fn failing_after_five() -> Factory {
    let scenario = Scenario::panel(3).expect("a panel");
    Rc::new(move || {
        Engine::new(
            &scenario.config_id,
            scenario.seed,
            scenario.config,
            &scenario.script,
            scenario.params.as_ref(),
            Limits {
                max_slices: 5,
                ..Limits::default()
            },
        )
    })
}

mod disposal;
mod playback;
mod prepared;
mod repair;
mod seeking;
mod stepping;
mod switching;
mod visibility;
