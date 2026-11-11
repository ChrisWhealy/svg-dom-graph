//! The discrete-event service simulator, as a gallery panel:
//!
//! - `panel-service-simulator` / `#dess-stage` — [`build_service_simulator_demo`]: a queue and one to four servers,
//!   with jobs that arrive, wait, run, time out, retry, fail and are cancelled, drawn on one long-lived `Scene` with an
//!   overlay and a data node of eight counters.
//!
//! None of the simulator is here. The engine is `service-sim-engine`, the projection, the surface, the controller and
//! the session coordinator are `service-sim`, and the scenarios that are offered are its catalogue. This module is the
//! markup's controls and inspector, and the page side of a session.
//!
//! The controls are native HTML. Each is one request to the controller, and the page shows what the controller
//! committed. The scenario is replayed to its end before anything is built, so that the range has its true maximum and
//! a run that is longer than it should be is refused.
//!
//! One pair of page hooks, a watch on the gallery's stage and the page's lifecycle listeners, is installed when the
//! panel is built, and each calls whichever controller is current. They hold the coordinator's target weakly, so that
//! nothing owns anything in a circle.

mod controls;
mod dom;

use crate::source_file;
use controls::{Hook, Listeners, Shared, Ui, counter_names, queue_sync, sync, wire};
use demo_common::util::{clear_error, document, report_error, required_element};
use dom::{DomSessions, Notify, Voices};
use service_sim::catalogue::catalogue;
use service_sim::host::{GalleryHint, PageLifecycle};
use service_sim::session::{Coordinator, SwitchOutcome};
use std::cell::RefCell;
use std::rc::Rc;
use wasm_bindgen::JsCast;

/// This module's own full source, embedded at compile time — see `crate::source_frame`'s own doc comment for why.
pub(crate) const SOURCE: &str = include_str!("mod.rs");

/// The files behind this demo: the controls and the page side of a session, then the simulator's own crates, from the
/// catalogue to the surface.
pub(crate) const FILES: &[crate::SourceFile] = &[
    source_file!("service_sim/controls.rs", "controls.rs"),
    source_file!("service_sim/dom.rs", "dom.rs"),
    source_file!("service-sim/src/catalogue.rs", "../../../service-sim/src/catalogue.rs"),
    source_file!("service-sim/src/session/mod.rs", "../../../service-sim/src/session/mod.rs"),
    source_file!(
        "service-sim/src/controller/mod.rs",
        "../../../service-sim/src/controller/mod.rs"
    ),
    source_file!(
        "service-sim/src/projection/mod.rs",
        "../../../service-sim/src/projection/mod.rs"
    ),
    source_file!("service-sim/src/surface.rs", "../../../service-sim/src/surface.rs"),
    source_file!("service-sim/src/host.rs", "../../../service-sim/src/host.rs"),
];

const ERROR: &str = "dess-error";

thread_local! {
    /// What a built panel holds for the life of the page. The gallery builds a panel once and only hides it afterwards,
    /// so it is never dropped.
    static PANEL: RefCell<Option<Panel>> = const { RefCell::new(None) };
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The panel's owner: the coordinator, the one pair of hooks and the listeners of the controls.
#[allow(dead_code)]
struct Panel {
    coordinator: Coordinator<DomSessions>,
    shared: Rc<Shared>,
    gallery: GalleryHint,
    lifecycle: PageLifecycle,
    listeners: Listeners,
    notify: Rc<dyn Fn()>,
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
fn find(document: &web_sys::Document, id: &str) -> Result<web_sys::Element, String> {
    required_element(document, id)
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
fn find_as<T: JsCast>(document: &web_sys::Document, id: &str) -> Result<T, String> {
    find(document, id)?
        .dyn_into::<T>()
        .map_err(|_| format!("#{id} is not the kind of element that the panel needs"))
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Writes the rows of the ten counters, from the projection's own names, and returns the elements that hold the values.
fn counter_rows(document: &web_sys::Document, list: &web_sys::Element) -> Result<Vec<web_sys::Element>, String> {
    let make = |tag: &str| {
        document
            .create_element(tag)
            .map_err(|e| format!("could not make a {tag}: {e:?}"))
    };
    let mut values = Vec::new();
    for (key, name) in counter_names() {
        let term = make("dt")?;
        term.set_text_content(Some(name));
        let value = make("dd")?;
        value.set_id(&format!("dess-c-{key}"));
        value.set_text_content(Some("—"));
        list.append_child(&term).map_err(|e| format!("{e:?}"))?;
        list.append_child(&value).map_err(|e| format!("{e:?}"))?;
        values.push(value);
    }
    Ok(values)
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
fn ui(document: &web_sys::Document) -> Result<Ui, String> {
    Ok(Ui {
        error: find(document, ERROR)?,
        live: find(document, "dess-live")?,
        scenario: find(document, "dess-scenario")?,
        snapshot: find(document, "dess-snapshot")?,
        previous: find(document, "dess-prev")?,
        next: find(document, "dess-next")?,
        restart: find(document, "dess-restart")?,
        play: find(document, "dess-play")?,
        speed: find_as(document, "dess-speed")?,
        repair: find(document, "dess-repair")?,
        previous_reason: find(document, "dess-prev-reason")?,
        next_reason: find(document, "dess-next-reason")?,
        seek: find_as(document, "dess-seek")?,
        seek_text: find(document, "dess-seek-text")?,
        seek_description: find(document, "dess-seek-desc")?,
        job: find_as(document, "dess-job")?,
        i_snapshot: find(document, "dess-i-snapshot")?,
        i_tick: find(document, "dess-i-tick")?,
        i_waiting: find(document, "dess-i-waiting")?,
        i_busy: find(document, "dess-i-busy")?,
        i_selected: find(document, "dess-i-selected")?,
        i_state: find(document, "dess-i-state")?,
        i_display: find(document, "dess-i-display")?,
        counters: counter_rows(document, &find(document, "dess-counters")?)?,
    })
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Builds the panel: replays the first scenario to its end, builds its scene, and puts the controls and the page's
/// hooks on it. The gallery calls this once, the first time that the panel is selected.
///
/// # Errors
///
/// Returns `Err` if an element of the fragment is missing, the scenario is refused, or its scene could not be built and
/// drawn.
pub(crate) fn build_service_simulator_demo() -> Result<(), String> {
    let document = document()?;
    clear_error(ERROR);
    build(&document).inspect_err(|message| report_error(ERROR, message))
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
fn build(document: &web_sys::Document) -> Result<(), String> {
    let ui = ui(document)?;
    let entry = catalogue()
        .map_err(|e| e.message())?
        .into_iter()
        .next()
        .ok_or("the catalogue is empty")?;
    // The end of the run comes from a replay of a separate engine that draws nothing, and it is what the range's
    // maximum is.
    let extent = entry.spec.preflight().map_err(|e| format!("The scenario was refused: {e:?}"))?;

    let notify_slot: Notify = Rc::new(RefCell::new(None));
    let sessions = DomSessions::new(
        document.clone(),
        find(document, "dess-stage")?,
        Voices {
            status: ui.live.clone(),
            problems: ui.error.clone(),
            play: ui.play.clone(),
        },
        Rc::clone(&notify_slot),
    );
    let mut coordinator = Coordinator::new(sessions);
    let hook: Hook = coordinator.target().hook();
    let shared = Rc::new(Shared::new(ui, hook.clone()));
    shared.end.set(extent.slices);

    // The panel is told after each draw and each change of playback, and reads what the controller committed once that
    // call has returned. The controller's own first draw is made before the panel can be told, so the first reading is
    // made here.
    let weak = Rc::downgrade(&shared);
    let notify: Rc<dyn Fn()> = Rc::new(move || {
        if let Some(shared) = weak.upgrade() {
            queue_sync(&shared);
        }
    });
    *notify_slot.borrow_mut() = Some(Rc::downgrade(&notify));

    let label = entry.spec.label.clone();
    match coordinator.switch(entry.spec) {
        SwitchOutcome::Replaced => {},
        other => return Err(format!("The first scenario could not be shown: {other:?}")),
    }
    shared.ui.scenario.set_text_content(Some(&label));
    shared.ui.seek.set_max(&extent.slices.to_string());
    if let Some(controller) = hook.controller() {
        controller.set_speed(shared.ui.speed.value().parse().unwrap_or(1_000));
    }

    // One watch on the gallery's stage and one set of page listeners, whatever scenario is on the page.
    let stage = document.get_element_by_id("stage").ok_or("the gallery's stage is missing")?;
    let (changed, shown, left) = (hook.clone(), Rc::downgrade(&shared), hook.clone());
    let gallery = GalleryHint::install(&stage, {
        let changed = changed.clone();
        let shown = shown.clone();
        move || {
            changed.presence_changed();
            if let Some(shared) = shown.upgrade() {
                sync(&shared);
            }
        }
    })?;
    let lifecycle = PageLifecycle::install(
        move || {
            changed.presence_changed();
            if let Some(shared) = shown.upgrade() {
                sync(&shared);
            }
        },
        move || left.suspend(),
    )?;

    let listeners = wire(&shared)?;
    sync(&shared);
    PANEL.with(|panel| {
        *panel.borrow_mut() = Some(Panel {
            coordinator,
            shared,
            gallery,
            lifecycle,
            listeners,
            notify,
        });
    });

    Ok(())
}
