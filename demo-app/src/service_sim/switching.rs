//! Choosing another scenario: the request, what it came to, and what the page says about it.
//!
//! None of the decision is here. The coordinator decides whether the scenario that was asked for replaces the one on
//! the page, in place if it has the same number of servers and by a staged scene if it has not, and rolls back if
//! anything fails. This writes what it decided: the new scenario and its range if it committed, and the reason and the
//! scenario that is still shown if it did not.

use super::controls::{Shared, sync};
use service_sim::{controller::Outcome, session::SwitchOutcome};
use std::rc::Rc;

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// What a request that did not commit said, in words that a person can act on.
fn refusal(outcome: &SwitchOutcome) -> Option<String> {
    let reason = match outcome {
        SwitchOutcome::Replaced | SwitchOutcome::InPlace(Outcome::Done) => return None,
        SwitchOutcome::Rejected(problem) => format!("it was refused: {problem:?}"),
        SwitchOutcome::RolledBack(rollback) => {
            let undone = if rollback.undo_failed {
                ", and the page could not be put back as it was"
            } else {
                ""
            };
            format!("{:?} failed: {}{undone}", rollback.stage, rollback.reason)
        },
        SwitchOutcome::InPlace(Outcome::Unavailable) => "the panel is hidden or unavailable".to_owned(),
        SwitchOutcome::InPlace(other) => format!("the display could not be updated ({other:?})"),
    };
    Some(format!(
        "The scenario could not be changed: {reason}. The previous scenario is still shown."
    ))
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Handles a change of the scenario list.
pub(super) fn change(shared: &Rc<Shared>) {
    let ui = &shared.ui;
    let id = ui.scenario.value();
    let Some(entry) = shared.scenarios.iter().find(|entry| entry.spec.id == id) else {
        return;
    };
    // The exact end of the run is found before anything is built, and it is what the range will run to.
    let outcome = match entry.spec.preflight() {
        Ok(extent) => {
            let outcome = shared.coordinator.borrow_mut().switch(entry.spec.clone());
            (outcome, Some(extent))
        },
        Err(problem) => (SwitchOutcome::Rejected(problem), None),
    };
    match (refusal(&outcome.0), outcome.1) {
        (None, Some(extent)) => {
            shared.end.set(extent.slices);
            ui.seek.set_max(&extent.slices.to_string());
            shared.interrupted.set(false);
            let _ = ui.error.set_attribute("hidden", "");
            ui.error.set_text_content(None);
            ui.live
                .set_text_content(Some(&format!("{}: snapshot 0 of {}", entry.spec.label, extent.slices)));
        },
        (message, _) => {
            // The list goes back to the scenario that is on the page, since nothing changed.
            let active = shared.coordinator.borrow().active_id().map(str::to_owned);
            if let Some(active) = active {
                ui.scenario.set_value(&active);
            }
            let message = message.unwrap_or_default();
            ui.error.set_text_content(Some(&message));
            let _ = ui.error.remove_attribute("hidden");
            ui.live.set_text_content(Some(&message));
        },
    }
    sync(shared);
}
