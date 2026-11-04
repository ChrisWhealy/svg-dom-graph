//! `panel-petri-net`: a bounded buffer that four producers fill and three consumers empty, drawn as a Petri net. See
//! [`build_petri_net_demo`] for what the page does, [`net`] for the rules, [`fixtures`] for the three scripts,
//! [`projection`] for what each snapshot looks like, and [`controller`] for the buttons, the manual branch and the
//! playback.
//!
//! The rest of this module is the browser side. [`layout`] says where everything is, [`scene`] builds the real nodes
//! and connectors and the overlay on them, and [`controls`] draws the selectors, the buttons and the range. [`chrome`]
//! finds the elements that the panel's HTML declares, [`page`] is what the controller draws on, and [`scheduler`] runs
//! the playback's ticks.

mod chrome;
mod controller;
mod controls;
mod dom;
mod fixtures;
mod layout;
mod listeners;
mod net;
mod page;
mod projection;
mod scene;
mod scheduler;
mod trace;

use chrome::{STAGE, SVG};
use controller::{Action, Controller, Surface};
use controls::ControlState;
use demo_common::util::{document, ensure_svg_in, required_element};
use dom::set;
use fixtures::Scenario;
use layout::SCENE;
use listeners::Listeners;
use net::{Net, PlaceId, TransitionId};
use page::Page;
use projection::{Speed, Target, View, Wiring, bounds, move_roving, target_order};
use scheduler::make_scheduler;
use std::{
    cell::{Cell, RefCell},
    rc::{Rc, Weak},
};
use wasm_bindgen::{JsCast, closure::Closure};

/// This module's own full source, embedded at compile time — see `crate::source_frame`'s own doc comment for why.
pub(crate) const SOURCE: &str = include_str!("mod.rs");

/// The files behind this demo's one function: the rules, the scripts, the history, what each snapshot looks like, the
/// controls, and the browser side.
pub(crate) const FILES: &[crate::SourceFile] = &[
    crate::source_file!("petri_net/net/mod.rs", "net/mod.rs"),
    crate::source_file!("petri_net/fixtures/mod.rs", "fixtures/mod.rs"),
    crate::source_file!("petri_net/trace/mod.rs", "trace/mod.rs"),
    crate::source_file!("petri_net/projection/mod.rs", "projection/mod.rs"),
    crate::source_file!("petri_net/controller/mod.rs", "controller/mod.rs"),
    crate::source_file!("petri_net/layout/mod.rs", "layout/mod.rs"),
    crate::source_file!("petri_net/scene.rs", "scene.rs"),
    crate::source_file!("petri_net/controls.rs", "controls.rs"),
    crate::source_file!("petri_net/page.rs", "page.rs"),
    crate::source_file!("petri_net/chrome.rs", "chrome.rs"),
    crate::source_file!("petri_net/scheduler.rs", "scheduler.rs"),
    crate::source_file!("petri_net/dom.rs", "dom.rs"),
    crate::source_file!("petri_net/listeners.rs", "listeners.rs"),
];

thread_local! {
    /// Counted up for each demo that is built, and the number in every ID of its scene.
    static INSTANCES: Cell<u32> = const { Cell::new(0) };
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The place or the transition that an event is on, from the `data-pn-*` attributes of its element.
fn target_of(target: Option<web_sys::EventTarget>) -> Option<Target> {
    let element = target?.dyn_into::<web_sys::Element>().ok()?;
    if let Some(place) = element.closest("[data-pn-place]").ok().flatten() {
        let name = place.get_attribute("data-pn-place")?;
        return PlaceId::all().find(|p| p.name() == name).map(Target::Place);
    }
    let gate = element.closest("[data-pn-transition]").ok().flatten()?;
    let name = gate.get_attribute("data-pn-transition")?;
    TransitionId::all().find(|t| t.name() == name).map(Target::Transition)
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// What a press of the button for `action` asks the controller to do. Play is Pause while it plays, and the manual
/// button enters or leaves manual mode.
fn action_for(action: &str, playing: bool, manual: bool, wiring: Wiring) -> Option<Action> {
    match action {
        "restart" => Some(Action::Restart),
        "previous" => Some(Action::Previous),
        "next" => Some(Action::Next),
        "play" => Some(if playing { Action::Pause } else { Action::Play }),
        "manual" => Some(if manual { Action::ReturnToScript } else { Action::EnterManual }),
        "fire" => Some(Action::FireSelected),
        "toggle-focused" => Some(Action::SetWiring(if wiring == Wiring::Focused {
            Wiring::Overview
        } else {
            Wiring::Focused
        })),
        _ => None,
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Whether the address asks for every draw to be a full one, which a test uses to compare a page that was diffed with
/// one that was not.
fn always_full_requested() -> bool {
    web_sys::window()
        .and_then(|window| window.location().search().ok())
        .is_some_and(|query| query.contains("always-full"))
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Builds the Petri net demo: a bounded buffer that four producers fill and three consumers empty.
///
/// The net is an ordinary place/transition net. A producer prepares an item and puts it into the buffer, and a consumer
/// takes an item out and processes it. The buffer is two places, `Free` and `Used`, so a `Put` needs a free slot and a
/// `Take` needs a stored item, and no check in the page stops a full buffer from taking another item. The net does.
///
/// The page shows, in one scene:
///
/// 1. Above the graph, selectors for the scenario, a checkpoint and the speed, seven buttons, and a range that goes
///    straight to any snapshot. The buttons step, restart and play a script, enter and leave manual mode, fire the
///    selected transition and emphasise the wiring of the selected target.
/// 2. The net, with a ring for each of the four producers on the left, the two buffer places in the middle and a ring
///    for each of the three consumers on the right. Every place and every transition can be reached by the keyboard,
///    as one stop, and has a name that says its tokens or its status and what it is missing.
///
/// Beside the scene is a text panel that says all that the picture says: what fired, the tokens it moved, which
/// transitions it enabled and disabled, who waits on the buffer, and the counts. Every snapshot comes from firing the
/// ordinary transitions, and the page draws only what differs from the snapshot before. A write that fails is reported,
/// and the next button, key or move of the range draws the whole page again.
///
/// # Errors
///
/// Returns `Err` if the page is missing one of its elements, or if the scene or a listener could not be made.
pub(crate) fn build_petri_net_demo() -> Result<(), String> {
    let document = document()?;
    ensure_svg_in(&document, STAGE, SVG, SCENE)?;
    let svg = required_element(&document, SVG)?;
    set(&svg, "role", "group")?;
    set(&svg, "aria-label", "Petri net of a bounded buffer")?;
    let net = Net::bounded_buffer();
    let view = View::first();
    let scenario = Scenario::Walkthrough;
    let trace = scenario.trace(&net).map_err(|e| e.to_string())?;
    let instance = INSTANCES.with(|n| {
        n.set(n.get() + 1);
        n.get()
    });
    let source = projection::Source::Script(scenario);
    let state = ControlState {
        bounds: bounds(&net, &trace, 0, source, &view),
        wiring: view.wiring,
        index: 0,
        last: trace.last(),
        scenario,
        manual: false,
        speed: view.speed,
        playing: false,
    };
    let page = Rc::new(Page::new(&document, &svg, &net, &state, instance, always_full_requested())?);
    let controller = Controller::new(net, scenario, view, Rc::clone(&page) as Rc<dyn Surface>, make_scheduler()?);

    let listeners = match wire_all(&document, &svg, &controller, &page) {
        Ok(listeners) => listeners,
        Err(problem) => {
            // What was added is taken off again as the registry is dropped, and the controller lets go of its clock.
            controller.teardown();
            return Err(problem);
        },
    };
    LIVE.with(|live| {
        live.borrow_mut().push(Demo {
            instance,
            controller,
            _page: page,
            listeners,
        });
    });
    // This one listener is deliberately kept for the life of the page. It holds only a number, so that it frees nothing
    // that it must not and holds nothing that is torn down, and it does nothing the second time that it is called.
    let leaving = Closure::<dyn FnMut(web_sys::Event)>::new(move |_: web_sys::Event| shut_down(instance));
    web_sys::window()
        .ok_or("no window")?
        .add_event_listener_with_callback("pagehide", leaving.as_ref().unchecked_ref())
        .map_err(|e| format!("could not attach the pagehide listener: {e:?}"))?;
    leaving.forget();
    Ok(())
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// A demo that is on the page: the controller, the page that it draws on, and every listener that was added for them.
///
/// The listeners hold the controller and the page weakly, so that this is what keeps them alive, from the moment that
/// the demo is built to the moment that it is shut down.
struct Demo {
    instance: u32,
    controller: Rc<Controller>,
    _page: Rc<Page>,
    listeners: Listeners,
}

thread_local! {
    /// The demos that are on the page, which is where each one is owned.
    static LIVE: RefCell<Vec<Demo>> = const { RefCell::new(Vec::new()) };
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Shuts down the demo with this instance number: stops its playback, takes off its clock's hooks and every listener
/// that was added for it, and lets go of the controller, the page and the scene. Shutting down twice does nothing.
fn shut_down(instance: u32) {
    let demo = LIVE.with(|live| {
        let mut live = live.borrow_mut();
        live.iter().position(|d| d.instance == instance).map(|at| live.remove(at))
    });
    if let Some(mut demo) = demo {
        demo.controller.teardown();
        demo.listeners.detach_all();
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Calls `f` with what `weak` points to, if it is still there. A listener that is called after its demo was shut down
/// finds nothing, and does nothing.
fn with<T>(weak: &Weak<T>, f: impl FnOnce(&T)) {
    if let Some(strong) = weak.upgrade() {
        f(&strong);
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Adds every listener that the page needs, and gives the registry that holds them.
///
/// # Errors
///
/// Returns `Err` if the stage is missing, or a listener could not be attached. The registry is then dropped, which
/// takes off what had been added.
fn wire_all(
    document: &web_sys::Document,
    svg: &web_sys::Element,
    controller: &Rc<Controller>,
    page: &Rc<Page>,
) -> Result<Listeners, String> {
    let mut listeners = Listeners::default();
    let (controller, page) = (Rc::downgrade(controller), Rc::downgrade(page));
    wire_buttons(&mut listeners, document, &controller)?;
    wire_range(&mut listeners, &controller, &page)?;
    wire_selectors(&mut listeners, &controller, &page)?;
    wire_graph(&mut listeners, svg, &controller, &page)?;
    Ok(listeners)
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The action of the button that an event is on, from its `data-action`.
fn button_action(target: Option<web_sys::EventTarget>) -> Option<String> {
    target
        .and_then(|t| t.dyn_into::<web_sys::Element>().ok())
        .and_then(|e| e.closest("[data-action]").ok().flatten())
        .and_then(|button| button.get_attribute("data-action"))
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Makes the buttons work, with one listener for clicks and one for the Enter and Space keys on the stage, which find
/// the button by its `data-action`. A button that is only dimmed still gets its press, so that a page that is out of
/// date is drawn again even when the action then has nowhere to go.
fn wire_buttons(
    listeners: &mut Listeners,
    document: &web_sys::Document,
    controller: &Weak<Controller>,
) -> Result<(), String> {
    let stage: web_sys::EventTarget = required_element(document, STAGE)?.into();
    let press = |controller: &Weak<Controller>, action: &str| {
        with(controller, |controller| {
            let view = controller.view();
            if let Some(action) = action_for(action, controller.is_playing(), controller.is_manual(), view.wiring) {
                controller.act(action);
            }
        });
    };
    let clicked = controller.clone();
    listeners.add(&stage, "click", move |e: web_sys::Event| {
        if let Some(action) = button_action(e.target()) {
            press(&clicked, &action);
        }
    })?;
    let keyed = controller.clone();
    listeners.add(&stage, "keydown", move |e: web_sys::KeyboardEvent| {
        if e.key() == "Enter" || e.key() == " " {
            if let Some(action) = button_action(e.target()) {
                e.prevent_default();
                press(&keyed, &action);
            }
        }
    })
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Makes the range work. Moving it draws the latest place at the next frame, and letting it rest draws that place at
/// once and says where it is, once.
fn wire_range(listeners: &mut Listeners, controller: &Weak<Controller>, page: &Weak<Page>) -> Result<(), String> {
    let Some(seek) = page.upgrade().map(|page| page.controls().seek().clone()) else {
        return Err("the page was dropped before its range was wired".to_owned());
    };
    let value = {
        let seek = seek.clone();
        move || seek.value().parse::<usize>().unwrap_or(0)
    };
    let (moving, read) = (controller.clone(), value.clone());
    listeners.add(&seek, "input", move |_: web_sys::Event| {
        with(&moving, |controller| controller.seek_input(read()));
    })?;
    let resting = controller.clone();
    listeners.add(&seek, "change", move |_: web_sys::Event| {
        with(&resting, |controller| controller.seek_release(value()));
    })
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Makes the selectors work. A selector says what is on the page, which is the old value if the new one could not be
/// shown.
fn wire_selectors(listeners: &mut Listeners, controller: &Weak<Controller>, page: &Weak<Page>) -> Result<(), String> {
    let Some((scenario, checkpoint, speed)) = page.upgrade().map(|page| {
        let controls = page.controls();
        (
            controls.scenario_selector().clone(),
            controls.checkpoint_selector().clone(),
            controls.speed_selector().clone(),
        )
    }) else {
        return Err("the page was dropped before its selectors were wired".to_owned());
    };
    let (switching, chosen) = (controller.clone(), scenario.clone());
    listeners.add(&scenario, "change", move |_: web_sys::Event| {
        with(&switching, |controller| {
            if let Some(wanted) = Scenario::from_key(&chosen.value()) {
                controller.act(Action::SelectScenario(wanted));
            }
            chosen.set_value(controller.scenario().key());
        });
    })?;
    let (jumping, chosen) = (controller.clone(), checkpoint.clone());
    listeners.add(&checkpoint, "change", move |_: web_sys::Event| {
        with(&jumping, |controller| {
            if let Ok(n) = chosen.value().parse::<usize>() {
                controller.act(Action::Checkpoint(n));
            }
        });
        // The prompt is chosen again, so that the same checkpoint can be chosen twice.
        chosen.set_value("");
    })?;
    let (setting, chosen) = (controller.clone(), speed.clone());
    listeners.add(&speed, "change", move |_: web_sys::Event| {
        with(&setting, |controller| {
            if let Some(speed) = Speed::from_key(&chosen.value()) {
                controller.act(Action::SetSpeed(speed));
            }
        });
    })
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Makes the places and the transitions work with the keyboard and the pointer. The listeners are on the scene, so that
/// each target needs none of its own.
///
/// Moving the focus selects, and never fires. A click only selects. A double click on a transition, and Enter or Space
/// on a focused one, fire it in manual mode, and in a script they say why not.
fn wire_graph(
    listeners: &mut Listeners,
    svg: &web_sys::Element,
    controller: &Weak<Controller>,
    page: &Weak<Page>,
) -> Result<(), String> {
    let svg: &web_sys::EventTarget = svg;
    let (keyed, keyed_page) = (controller.clone(), page.clone());
    listeners.add(svg, "keydown", move |e: web_sys::KeyboardEvent| {
        let Some(from) = target_of(e.target()) else {
            return;
        };
        let key = e.key();
        if matches!(key.as_str(), "Enter" | " ") {
            e.prevent_default();
            // A held key repeats, and a repeat must not fire again.
            if e.repeat() {
                return;
            }
            with(&keyed, |controller| match from {
                Target::Transition(t) if controller.is_manual() => controller.act(Action::Fire(t)),
                other => controller.act(Action::Select(other)),
            });
            with(&keyed_page, |page| page.focus_target(from));
            return;
        }
        with(&keyed, |controller| {
            if let Some(next) = move_roving(&target_order(), controller.view().roving, &key) {
                e.prevent_default();
                controller.act(Action::Select(next));
                with(&keyed_page, |page| page.focus_target(next));
            }
        });
    })?;
    let clicked = controller.clone();
    listeners.add(svg, "click", move |e: web_sys::MouseEvent| {
        if let Some(target) = target_of(e.target()) {
            with(&clicked, |controller| controller.act(Action::Select(target)));
        }
    })?;
    let double = controller.clone();
    listeners.add(svg, "dblclick", move |e: web_sys::MouseEvent| {
        if let Some(Target::Transition(t)) = target_of(e.target()) {
            with(&double, |controller| controller.act(Action::Fire(t)));
        }
    })?;
    let gained = controller.clone();
    listeners.add(svg, "focusin", move |e: web_sys::FocusEvent| {
        if let Some(target) = target_of(e.target()) {
            with(&gained, |controller| {
                let view = controller.view();
                if view.selected != Some(target) || view.roving != target {
                    controller.act(Action::Select(target));
                }
            });
        }
    })
}
