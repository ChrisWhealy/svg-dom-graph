//! The scenarios that the gallery offers, as specs that a coordinator can show.
//!
//! The simulator's rules are the engine's, and this only says which runs are offered, under what name, and what each is
//! known to come to. A scenario that is offered is replayed to its end before anything is built (design D2), and the
//! ceilings are the totals that it is known to come to, so a run that is longer than it should be is refused after that
//! much work and not after any amount of time.

use crate::controller::Factory;
use crate::session::{Expected, Spec};
use service_sim_engine::trace::Scenario;
use service_sim_engine::{Engine, Error, Limits};
use std::rc::Rc;

/// The seed of the frozen generator's revision, which the scaled runs use.
pub const SEED: u64 = 20_261_106;

/// The fixtures of the engine's known-answer ledger, from which the two scripted scenarios are taken.
const FIXTURES: &str = include_str!("../../dess/s1/fixtures.json");

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// A scenario that is offered, and whether it belongs to the advanced path.
#[derive(Clone)]
pub struct Entry {
    pub spec: Spec,
    /// The run is long, is not a benchmark, and is offered as an advanced choice.
    pub advanced: bool,
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
fn factory(scenario: Scenario) -> Factory {
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

fn fixture(name: &str) -> Result<Scenario, Error> {
    let all: serde_json::Value =
        serde_json::from_str(FIXTURES).map_err(|e| Error::Invalid(format!("the fixtures are not JSON: {e}")))?;
    Scenario::fixture(name, &all[name])
}

/// The frozen generator at the manifest's seed, with this many arrivals.
fn scaled(arrivals: u64) -> Result<Scenario, Error> {
    let mut scenario = Scenario::mixed_outage_7(SEED)?;
    if let Some(params) = scenario.params.as_mut() {
        params.arrivals = arrivals;
    }
    Ok(scenario)
}

fn entry(id: &str, label: &str, scenario: Scenario, (slices, pops): (u64, u64), advanced: bool) -> Entry {
    Entry {
        spec: Spec {
            id: id.to_owned(),
            label: label.to_owned(),
            servers: scenario.config.servers,
            expected: Expected {
                slices,
                pops,
                max_slices: slices,
                max_pops: pops,
            },
            factory: factory(scenario),
        },
        advanced,
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The four scenarios, in the order that the gallery lists them. The first is the one that it opens with.
///
/// # Errors
///
/// Returns an [`Error`] if a scenario is refused, which would be a defect.
pub fn catalogue() -> Result<Vec<Entry>, Error> {
    Ok(vec![
        entry("E0", "Walkthrough - default", fixture("E0")?, (12, 19), false),
        entry("E3-NOOP", "No-op and stale events", fixture("E3-NOOP")?, (9, 9), false),
        entry("scaled", "Busy service / outages", scaled(360)?, (487, 1_009), false),
        entry("W6", "W6 reference trace - advanced", scaled(4_000)?, (5_464, 11_214), true),
    ])
}

#[cfg(test)]
mod unit_tests;
