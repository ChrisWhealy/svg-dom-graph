//! The controls and the inspector of the panel: what a click asks of the controller, and what the page says about the
//! state that it committed.
//!
//! The controls are native HTML, outside the scene, with no shortcut of their own. Every one of them is a request to
//! the controller, and what the page shows is read from the controller's committed state after the request has been
//! made, never from the request. A request that has nowhere to go is still made. A button for it says so with
//! `aria-disabled` and a reason that is read with it, and is never `disabled`, which would take the click away from a
//! controller that may have a dirty page to repair.

use super::{
    dom::{DomSessions, PanelController, PanelHost},
    switching,
};
use service_sim::{
    catalogue::Entry as Scenario,
    controller::{Host, Outcome, Presence},
    host::BrowserScheduler,
    projection::{COUNTER_NAMES, counter_values},
    session::Coordinator,
    session::TargetHook,
};
use service_sim_engine::model::{JobState, Model};
use std::{
    any::Any,
    cell::{Cell, RefCell},
    rc::{Rc, Weak},
};
use wasm_bindgen::{JsCast, closure::Closure, convert::FromWasmAbi};

pub(super) type Hook = TargetHook<PanelHost, BrowserScheduler>;

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The elements of the panel that the controls read and write.
pub(super) struct Ui {
    pub(super) error: web_sys::Element,
    pub(super) live: web_sys::Element,
    pub(super) scenario: web_sys::HtmlSelectElement,
    pub(super) notice: web_sys::Element,
    pub(super) snapshot: web_sys::Element,
    pub(super) previous: web_sys::Element,
    pub(super) next: web_sys::Element,
    pub(super) restart: web_sys::Element,
    pub(super) play: web_sys::Element,
    pub(super) speed: web_sys::HtmlSelectElement,
    pub(super) repair: web_sys::Element,
    pub(super) previous_reason: web_sys::Element,
    pub(super) next_reason: web_sys::Element,
    pub(super) seek: web_sys::HtmlInputElement,
    pub(super) seek_text: web_sys::Element,
    pub(super) seek_description: web_sys::Element,
    pub(super) job: web_sys::HtmlSelectElement,
    pub(super) i_snapshot: web_sys::Element,
    pub(super) i_tick: web_sys::Element,
    pub(super) i_waiting: web_sys::Element,
    pub(super) i_busy: web_sys::Element,
    pub(super) i_selected: web_sys::Element,
    pub(super) i_state: web_sys::Element,
    pub(super) i_display: web_sys::Element,
    pub(super) counters: Vec<web_sys::Element>,
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// What the controls share: the way to the current controller, and the little that the page itself remembers.
pub(super) struct Shared {
    pub(super) ui: Ui,
    pub(super) hook: Hook,
    /// The one coordinator of the panel. It is borrowed only for the length of a request for another scenario.
    pub(super) coordinator: Rc<RefCell<Coordinator<DomSessions>>>,
    /// The scenarios that the panel offers, in the order of its list.
    pub(super) scenarios: Vec<Scenario>,
    /// Whether playback was stopped by the panel or the page being hidden, which is said until a person acts. It is a
    /// presentation of that interruption, and not a state of the controller.
    pub(super) interrupted: Cell<bool>,
    /// The last snapshot of the scenario, which a preflight replay found, so that it is known before the run has been
    /// reached.
    pub(super) end: Cell<u64>,
    /// Whether a person asked for something that will arrive at a frame, which is announced when it has.
    announce_when_committed: Cell<bool>,
    /// Whether a refresh of the inspector is already waiting for the end of the call that drew.
    queued: Cell<bool>,
    /// The options that the job list holds, so that it is built again only when they change.
    job_options: RefCell<String>,
    /// The snapshot that was last shown, so that a commit is known from a refresh. A failure that the controller
    /// recovered from stays on the page until a commit or a repair, and is not hidden by the recovery.
    shown_slice: Cell<u64>,
    /// Whether the next refresh may clear a failure, because a person asked for a repair.
    repaired: Cell<bool>,
}

impl Shared {
    pub(super) fn new(
        ui: Ui,
        hook: Hook,
        coordinator: Rc<RefCell<Coordinator<DomSessions>>>,
        scenarios: Vec<Scenario>,
    ) -> Self {
        Self {
            ui,
            hook,
            coordinator,
            scenarios,
            interrupted: Cell::new(false),
            end: Cell::new(0),
            announce_when_committed: Cell::new(false),
            queued: Cell::new(false),
            job_options: RefCell::new(String::new()),
            shown_slice: Cell::new(0),
            repaired: Cell::new(false),
        }
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// One listener that is on a target, with the closure that it calls, so that it can be taken off again.
struct Entry {
    target: web_sys::EventTarget,
    event: &'static str,
    function: js_sys::Function,
    _closure: Box<dyn Any>,
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Every listener that the panel has put on its controls, which are taken off again when it is dropped.
#[derive(Default)]
pub(super) struct Listeners {
    entries: Vec<Entry>,
}

impl Listeners {
    fn add<E: FromWasmAbi + 'static>(
        &mut self,
        target: &web_sys::EventTarget,
        event: &'static str,
        handler: impl FnMut(E) + 'static,
    ) -> Result<(), String> {
        let closure = Closure::<dyn FnMut(E)>::new(handler);
        let function: js_sys::Function = closure.as_ref().unchecked_ref::<js_sys::Function>().clone();
        target
            .add_event_listener_with_callback(event, &function)
            .map_err(|e| format!("could not attach the {event} listener: {e:?}"))?;
        self.entries.push(Entry {
            target: target.clone(),
            event,
            function,
            _closure: Box::new(closure),
        });
        Ok(())
    }
}

impl Drop for Listeners {
    fn drop(&mut self) {
        while let Some(entry) = self.entries.pop() {
            let _ = entry.target.remove_event_listener_with_callback(entry.event, &entry.function);
        }
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
fn set_text(element: &web_sys::Element, text: &str) {
    if element.text_content().as_deref() != Some(text) {
        element.set_text_content(Some(text));
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Marks a button as one that has nowhere to go, with the reason that is read with it. It says whether the write of the
/// attribute failed, in which case the button may advertise the wrong boundary until the next write succeeds.
fn mark_boundary(button: &web_sys::Element, reason: &web_sys::Element, at_boundary: bool, text: &str) -> bool {
    let written = if at_boundary {
        button.set_attribute("aria-disabled", "true")
    } else {
        button.remove_attribute("aria-disabled")
    };
    set_text(reason, if at_boundary { text } else { "" });
    written.is_err()
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
fn announce(ui: &Ui, text: &str) {
    ui.live.set_text_content(Some(text));
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The sentence that says what a job is and where, from the committed model.
fn describe_job(model: &Model, id: u64) -> String {
    let Some(job) = model.job(id) else {
        return format!("Job {id} is not in the picture");
    };
    let priority = if job.priority == 0 { "low" } else { "high" };
    let place = match (job.state, job.server, job.due) {
        (JobState::Running, Some(server), Some(due)) => format!(" on S{server}, due at tick {due}"),
        _ => String::new(),
    };
    format!(
        "{}: {priority} priority, {}{place}, attempt {}",
        job.key,
        job.state.word(),
        job.attempt
    )
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Writes the committed state of the controller to everything that shows it.
///
/// Nothing here comes from a request. A seek that has not arrived is shown as what was asked for beside the snapshot
/// that is still the committed one, and the range is moved back to the committed one when no seek is left.
pub(super) fn sync(shared: &Shared) {
    let ui = &shared.ui;
    let Some(controller) = shared.hook.controller() else {
        return;
    };
    let model = controller.committed_model();
    let slice = controller.committed_slice();
    let end = shared.end.get();
    let pending = controller.pending_seek();
    let playing = controller.is_playing();

    set_text(&ui.snapshot, &format!("Snapshot {slice} of {end}"));
    set_text(&ui.i_snapshot, &format!("{slice} of {end}"));
    set_text(&ui.i_tick, &model.tick.to_string());
    set_text(&ui.i_waiting, &format!("{} of {}", model.waiting_total, model.queue_capacity));
    let busy = model
        .servers
        .iter()
        .filter(|s| matches!(s, service_sim_engine::model::ServerView::Busy { .. }))
        .count();
    set_text(&ui.i_busy, &format!("{busy} of {}", model.servers.len()));
    for (element, value) in ui.counters.iter().zip(counter_values(&model)) {
        set_text(element, &value.to_string());
    }
    let selected = controller.selected();
    set_text(
        &ui.i_selected,
        &selected.map_or_else(|| "none".to_owned(), |id| describe_job(&model, id)),
    );

    // The job list holds the jobs that a person can see in the picture, and the selected one wherever it is.
    let options: Vec<(u64, String)> = model
        .jobs
        .iter()
        .filter(|job| job.state.active() || Some(job.id) == selected)
        .map(|job| (job.id, describe_job(&model, job.id)))
        .collect();
    let key = format!("{options:?}");
    if *shared.job_options.borrow() != key {
        ui.job.set_inner_html("");
        if let Some(document) = ui.job.owner_document() {
            let add = |value: &str, label: &str| {
                if let Ok(option) = document.create_element("option") {
                    let _ = option.set_attribute("value", value);
                    option.set_text_content(Some(label));
                    let _ = ui.job.append_child(&option);
                }
            };
            add("", "None");
            for (id, label) in &options {
                add(&id.to_string(), label);
            }
        }
        *shared.job_options.borrow_mut() = key;
    }
    ui.job.set_value(&selected.map_or_else(String::new, |id| id.to_string()));

    // The range shows what was asked for while a seek is on its way, and the committed snapshot otherwise.
    let shown = pending.unwrap_or(slice);
    ui.seek.set_value(&shown.to_string());
    let _ = ui.seek.set_attribute("aria-valuetext", &format!("Snapshot {shown} of {end}"));
    match pending {
        Some(requested) => {
            set_text(&ui.seek_text, &format!("Requested {requested}, committed {slice}"));
            set_text(&ui.seek_description, &format!("Snapshot {slice} is still the committed one."));
        },
        None => {
            set_text(&ui.seek_text, &format!("Committed {slice}"));
            set_text(&ui.seek_description, "");
        },
    }

    let boundary_failed = mark_boundary(&ui.previous, &ui.previous_reason, slice == 0, "Already at first snapshot")
        | mark_boundary(&ui.next, &ui.next_reason, slice >= end, "Already at last snapshot");

    let visible = match controller.with_host(|host| host.presence()) {
        Presence::Live => "visible",
        Presence::Hidden => "hidden",
        Presence::Detached => "detached",
        Presence::Disposed => "disposed",
    };
    let state = if playing {
        "Playing"
    } else if slice >= end {
        "Paused, end of scenario"
    } else {
        "Paused"
    };
    set_text(&ui.i_state, &format!("{state} / {visible}"));
    // A page needs repair if a draw failed, and also if the Play control could not be written, which leaves playback
    // going under a button that says Play. A write of a boundary button that failed is the same: the controller drew
    // what it should, and a control says something else. Every refresh writes them again, so a repair, which refreshes,
    // puts them right.
    let dirty = controller.is_dirty() || controller.with_host(|host| host.controls_stale()) || boundary_failed;
    set_text(&ui.i_display, if dirty { "Needs repair" } else { "Up to date" });
    let committed = shared.shown_slice.replace(slice) != slice;
    if !dirty && (committed || shared.repaired.replace(false)) {
        let _ = ui.error.set_attribute("hidden", "");
        ui.error.set_text_content(None);
    }
    if shared.interrupted.get() && !playing {
        let _ = ui.notice.remove_attribute("hidden");
        set_text(&ui.notice, "Paused while hidden. Press Play to resume.");
    } else {
        let _ = ui.notice.set_attribute("hidden", "");
        set_text(&ui.notice, "");
    }
    if shared.announce_when_committed.get() && pending.is_none() {
        shared.announce_when_committed.set(false);
        announce(ui, &format!("Snapshot {slice} of {end}"));
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Asks for [`sync`] when the call that drew has returned, which is when the controller's own state is committed and
/// can be read.
pub(super) fn queue_sync(shared: &Rc<Shared>) {
    if shared.queued.replace(true) {
        return;
    }
    let weak: Weak<Shared> = Rc::downgrade(shared);
    let task = Closure::once_into_js(move || {
        if let Some(shared) = weak.upgrade() {
            shared.queued.set(false);
            sync(&shared);
        }
    });
    if let Some(window) = web_sys::window() {
        window.queue_microtask(task.unchecked_ref());
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Says what a request came to, in the live region. A seek and a step back arrive at a frame, and are said when they
/// have.
fn say(shared: &Shared, controller: &PanelController, outcome: Outcome, at_boundary: &str) {
    let ui = &shared.ui;
    match outcome {
        Outcome::Done => announce(
            ui,
            &format!("Snapshot {} of {}", controller.committed_slice(), shared.end.get()),
        ),
        Outcome::AtBoundary => announce(ui, at_boundary),
        Outcome::Pending => shared.announce_when_committed.set(true),
        Outcome::Unavailable => announce(ui, "The panel is hidden or detached, so nothing changed."),
        Outcome::Failed => announce(ui, "The display could not be updated. Use Repair display."),
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Runs a request against the current controller, and then writes what it committed.
fn request(shared: &Rc<Shared>, act: impl FnOnce(&PanelController, &Shared) -> Outcome, at_boundary: &str) {
    let Some(controller) = shared.hook.controller() else {
        return;
    };
    shared.interrupted.set(false);
    let outcome = act(&controller, shared);
    say(shared, &controller, outcome, at_boundary);
    sync(shared);
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Puts the controls on their buttons, once.
///
/// # Errors
///
/// Returns `Err` if a listener could not be attached, and then none is left.
pub(super) fn wire(shared: &Rc<Shared>) -> Result<Listeners, String> {
    let ui = &shared.ui;
    let mut listeners = Listeners::default();
    let on = |listeners: &mut Listeners, element: &web_sys::Element, event: &'static str, run: Rc<dyn Fn()>| {
        listeners.add::<web_sys::Event>(element.unchecked_ref(), event, move |_| run())
    };

    let s = Rc::clone(shared);
    on(
        &mut listeners,
        &ui.next,
        "click",
        Rc::new(move || {
            request(&s, |c, _| c.next(), "End of scenario");
        }),
    )?;
    let s = Rc::clone(shared);
    on(
        &mut listeners,
        &ui.previous,
        "click",
        Rc::new(move || {
            request(&s, |c, _| c.previous(), "Already at the first snapshot");
        }),
    )?;
    let s = Rc::clone(shared);
    on(
        &mut listeners,
        &ui.restart,
        "click",
        Rc::new(move || {
            request(&s, |c, _| c.restart(), "Already at the first snapshot");
        }),
    )?;
    let s = Rc::clone(shared);
    on(
        &mut listeners,
        &ui.repair,
        "click",
        Rc::new(move || {
            request(
                &s,
                |c, shared| {
                    shared.repaired.set(true);
                    c.repair()
                },
                "Display repaired",
            );
        }),
    )?;

    // One toggle. At the end that is known from the preflight, Play asks for a step, which the controller answers at
    // once with a boundary and a repair of a dirty page, and starts no timer, so the button never reads Pause. Anywhere
    // else it sets the speed and plays.
    let s = Rc::clone(shared);
    on(
        &mut listeners,
        &ui.play,
        "click",
        Rc::new(move || {
            let Some(controller) = s.hook.controller() else {
                return;
            };
            if controller.is_playing() {
                controller.pause();
                announce(&s.ui, "Paused");
                sync(&s);
            } else if controller.committed_slice() >= s.end.get() {
                request(&s, |c, _| c.next(), "End of scenario");
            } else {
                s.interrupted.set(false);
                let ms: u32 = s.ui.speed.value().parse().unwrap_or(1_000);
                controller.set_speed(ms);
                let outcome = controller.play();
                if outcome == Outcome::Done {
                    announce(&s.ui, &format!("Playing at {} slices/s", 1_000 / ms.max(1)));
                } else {
                    say(&s, &controller, outcome, "End of scenario");
                }
                sync(&s);
            }
        }),
    )?;

    let s = Rc::clone(shared);
    on(
        &mut listeners,
        ui.speed.unchecked_ref(),
        "change",
        Rc::new(move || {
            let Some(controller) = s.hook.controller() else {
                return;
            };
            controller.set_speed(s.ui.speed.value().parse().unwrap_or(1_000));
            sync(&s);
        }),
    )?;

    // Each distinct value of the range is a request, and the arrow keys make one each. Only `input` is listened for,
    // since `change` at the end of a drag would repeat the last request.
    let s = Rc::clone(shared);
    on(
        &mut listeners,
        ui.seek.unchecked_ref(),
        "input",
        Rc::new(move || {
            let Ok(target) = s.ui.seek.value().parse::<u64>() else {
                return;
            };
            request(&s, |c, _| c.seek(target), "Already there");
        }),
    )?;

    let s = Rc::clone(shared);
    on(
        &mut listeners,
        ui.scenario.unchecked_ref(),
        "change",
        Rc::new(move || switching::change(&s)),
    )?;

    let s = Rc::clone(shared);
    on(
        &mut listeners,
        ui.job.unchecked_ref(),
        "change",
        Rc::new(move || {
            let chosen = s.ui.job.value().parse::<u64>().ok();
            request(&s, |c, _| c.select_job(chosen), "Selection unchanged");
            if let Some(c) = s.hook.controller() {
                let text = match c.selected() {
                    Some(_) => s.ui.i_selected.text_content().unwrap_or_default(),
                    None => "No job is selected".to_owned(),
                };
                announce(&s.ui, &text);
            }
        }),
    )?;
    Ok(listeners)
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The elements of the ten counters, named from the projection's own list, so that the inspector says what the overlay
/// says.
pub(super) fn counter_names() -> impl Iterator<Item = (&'static str, &'static str)> {
    COUNTER_NAMES.iter().copied()
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Tells the current controller that the page or the panel changed, and says so if that stopped playback.
///
/// A stop is the controller's own act, and the page only says that it happened, once, and keeps saying it until a
/// person acts. A hide and a show in one batch are seen by the observer as a panel that is shown, so playback goes on,
/// and nothing is said.
pub(super) fn lifecycle(shared: &Shared, tell: impl FnOnce(&Hook)) {
    let playing = shared.hook.controller().is_some_and(|c| c.is_playing());
    tell(&shared.hook);
    let stopped = playing && shared.hook.controller().is_some_and(|c| !c.is_playing());
    if stopped && !shared.interrupted.replace(true) {
        announce(&shared.ui, "Paused while hidden. Press Play to resume.");
    }
    sync(shared);
}
