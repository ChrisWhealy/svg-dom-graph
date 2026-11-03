//! `panel-edmonds-karp`: the Edmonds-Karp maximum flow algorithm, one event at a time, on two graphs. See
//! [`build_edmonds_karp_demo`] for what the page does, [`solver`] for the algorithm, [`projection`] for what each step
//! looks like, and [`controller`] for the buttons, the range and the playback.
//!
//! The rest of this module is the browser side. [`layout`] says where the controls are in the scene, [`geometry`] where
//! the vertices, edges and residual lanes are, [`scene`] builds the SVG for one graph, and [`controls`] draws the
//! selectors, the buttons and the range. [`chrome`] finds the elements that the panel's HTML declares, [`page`] is what
//! the controller draws on, and [`scheduler`] runs the playback's ticks.

mod chrome;
mod controller;
mod controls;
mod dom;
mod fixtures;
mod geometry;
mod layout;
mod page;
mod projection;
mod scene;
mod scheduler;
mod solver;

use crate::plot_scene::listen;
use chrome::{STAGE, SVG};
use controller::{Action, Controller, Surface};
use demo_common::util::{document, ensure_svg_in, required_element};
use dom::set;
use fixtures::{CANONICAL, NAMES, by_key};
use layout::SCENE;
use page::Page;
use projection::{Pref, Prefs, Speed, Target, View, move_roving, target_order};
use scheduler::make_scheduler;
use solver::Trace;
use std::{cell::Cell, rc::Rc};
use svg_dom_graph::controls::svg_buttons;
use wasm_bindgen::JsCast;

/// This module's own full source, embedded at compile time — see `crate::source_frame`'s own doc comment for why.
pub(crate) const SOURCE: &str = include_str!("mod.rs");

/// The files behind this demo's one function: the algorithm, what each step looks like, the controls, and the browser
/// side.
pub(crate) const FILES: &[crate::SourceFile] = &[
    crate::source_file!("edmonds_karp/solver/mod.rs", "solver/mod.rs"),
    crate::source_file!("edmonds_karp/fixtures.rs", "fixtures.rs"),
    crate::source_file!("edmonds_karp/projection/mod.rs", "projection/mod.rs"),
    crate::source_file!("edmonds_karp/geometry/mod.rs", "geometry/mod.rs"),
    crate::source_file!("edmonds_karp/controller/mod.rs", "controller/mod.rs"),
    crate::source_file!("edmonds_karp/layout/mod.rs", "layout/mod.rs"),
    crate::source_file!("edmonds_karp/scene.rs", "scene.rs"),
    crate::source_file!("edmonds_karp/controls.rs", "controls.rs"),
    crate::source_file!("edmonds_karp/page.rs", "page.rs"),
    crate::source_file!("edmonds_karp/chrome.rs", "chrome.rs"),
    crate::source_file!("edmonds_karp/scheduler.rs", "scheduler.rs"),
    crate::source_file!("edmonds_karp/dom.rs", "dom.rs"),
];

thread_local! {
    /// Counted up for each demo that is built, and the number in every ID of its scenes.
    static INSTANCES: Cell<u32> = const { Cell::new(0) };
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The vertex or the edge that an event is on, from the `data-ek-*` attributes of its element. A residual lane belongs
/// to an edge, and so it gives that edge.
fn target_of(target: Option<web_sys::EventTarget>) -> Option<Target> {
    let element = target?.dyn_into::<web_sys::Element>().ok()?;
    if let Some(vertex) = element.closest("[data-ek-vertex]").ok().flatten() {
        let name = vertex.get_attribute("data-ek-vertex")?;
        return NAMES.iter().position(|&n| n == name).map(Target::Vertex);
    }
    let edge = element.closest("[data-ek-edge]").ok().flatten()?;
    edge.get_attribute("data-ek-edge")?
        .strip_prefix('e')?
        .parse()
        .ok()
        .map(Target::Edge)
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// What a press of the button for `action` asks the controller to do. Play is Pause while it plays.
fn action_for(action: &str, playing: bool) -> Option<Action> {
    match action {
        "restart" => Some(Action::Restart),
        "previous" => Some(Action::Previous),
        "next" => Some(Action::Next),
        "jump-to-cut" => Some(Action::JumpToCut),
        "previous-augmentation" => Some(Action::PreviousAugmentation),
        "next-augmentation" => Some(Action::NextAugmentation),
        "toggle-residual" => Some(Action::Toggle(Pref::Residual)),
        "toggle-queue" => Some(Action::Toggle(Pref::Queue)),
        "toggle-detail" => Some(Action::Toggle(Pref::Detail)),
        "play" => Some(if playing { Action::Pause } else { Action::Play }),
        _ => None,
    }
}

/// Whether the address asks for every draw to be a full one, which a test uses to compare a page that was diffed with
/// one that was not.
fn always_full_requested() -> bool {
    web_sys::window()
        .and_then(|window| window.location().search().ok())
        .is_some_and(|query| query.contains("always-full"))
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Builds the Edmonds-Karp demo: the algorithm that finds the maximum flow through a network, one event at a time.
///
/// Each search is a breadth-first search of the residual network, and it takes the transitions of each vertex by the ID
/// of their original edge. The first time it finds the sink, it stops, and the path it found is augmented by its
/// bottleneck all at once. A search that cannot reach the sink ends the run, and what it did reach is one side of a
/// minimum cut, whose capacity is the maximum flow.
///
/// The page shows, in one scene:
///
/// 1. Above the graph, selectors for the graph and the speed, ten buttons, and a range that goes straight to any step.
///    The buttons step, restart, play, jump between augmentations and to the cut, and turn the residual arcs, the queue
///    marks and the detail of the arcs on or off.
/// 2. The graph, which is one of two. The classic network has a maximum flow of 23. The other is built so that its
///    second path takes flow back along an edge, which a reverse residual transition shows. Every vertex and edge can
///    be reached by the keyboard, as one stop, and has a name that says its flow and both its residuals.
///
/// Beside the scene is a text panel that says all that the colours and lines say: the queue, the path, the bottleneck,
/// the change that an augmentation made, and the cut. Every step comes from one trace that is worked out before
/// anything is drawn, and the page draws only what differs from the step before. A write that fails is reported, and
/// the next button, key or move of the range draws the whole page again.
///
/// # Errors
///
/// Returns `Err` if the page is missing one of its elements, or if the scene or a listener could not be made.
pub(crate) fn build_edmonds_karp_demo() -> Result<(), String> {
    let document = document()?;
    ensure_svg_in(&document, STAGE, SVG, SCENE)?;
    let svg = required_element(&document, SVG)?;
    set(&svg, "role", "group")?;
    set(&svg, "aria-label", "Edmonds-Karp maximum flow")?;
    let trace = Trace::build(&CANONICAL.graph()).map_err(|e| e.to_string())?;
    let view = View::first(Prefs::default(), Speed::Two);
    let instance = INSTANCES.with(|n| {
        n.set(n.get() + 1);
        n.get()
    });
    let page = Rc::new(Page::new(
        &document,
        &svg,
        &CANONICAL,
        &trace,
        &view,
        instance,
        always_full_requested(),
    )?);
    let controller = Controller::new(&CANONICAL, trace, view, Rc::clone(&page) as Rc<dyn Surface>, make_scheduler()?);

    wire_buttons(&document, &controller)?;
    wire_range(&controller, &page)?;
    wire_selectors(&controller, &page)?;
    wire_graph(&svg, &controller, &page)?;
    let leaving = Rc::clone(&controller);
    listen(
        &web_sys::window().ok_or("no window")?.into(),
        "pagehide",
        move |_: web_sys::Event| leaving.teardown(),
    )?;
    Ok(())
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Makes the buttons work. A button that is only dimmed still gets its press, so that a page that is out of date is
/// drawn again even when the action then has nowhere to go.
fn wire_buttons(document: &web_sys::Document, controller: &Rc<Controller>) -> Result<(), String> {
    let controller = Rc::clone(controller);
    svg_buttons::watch(document, STAGE, move |action| {
        if let Some(action) = action_for(action, controller.is_playing()) {
            controller.act(action);
        }
    })
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Makes the range work. Moving it draws the latest place at the next frame, and letting it rest draws that place at
/// once and says where it is, once.
fn wire_range(controller: &Rc<Controller>, page: &Rc<Page>) -> Result<(), String> {
    let seek = page.controls().seek().clone();
    let value = {
        let seek = seek.clone();
        move || seek.value().parse::<usize>().unwrap_or(0)
    };
    let (moving, read) = (Rc::clone(controller), value.clone());
    listen(&seek, "input", move |_: web_sys::Event| moving.seek_input(read()))?;
    let resting = Rc::clone(controller);
    listen(&seek, "change", move |_: web_sys::Event| resting.seek_release(value()))
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Makes the selectors for the graph and the speed work. A selector says the graph that is on the page, which is the
/// old one if the new one could not be shown.
fn wire_selectors(controller: &Rc<Controller>, page: &Rc<Page>) -> Result<(), String> {
    let fixture = page.controls().fixture_selector().clone();
    let (switching, chosen) = (Rc::clone(controller), fixture.clone());
    listen(&fixture, "change", move |_: web_sys::Event| {
        if let Some(wanted) = by_key(&chosen.value()) {
            switching.act(Action::SwitchFixture(wanted));
        }
        chosen.set_value(switching.fixture().key);
    })?;
    let speed = page.controls().speed_selector().clone();
    let (controller, chosen) = (Rc::clone(controller), speed.clone());
    listen(&speed, "change", move |_: web_sys::Event| {
        if let Some(speed) = Speed::from_key(&chosen.value()) {
            controller.act(Action::SetSpeed(speed));
        }
    })
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Makes the vertices and the edges work with the keyboard and the pointer. The listeners are on the scene, which the
/// graph is replaced inside, so that a new graph needs none of its own. Moving the focus never moves the index.
fn wire_graph(svg: &web_sys::Element, controller: &Rc<Controller>, page: &Rc<Page>) -> Result<(), String> {
    let (keyed, keyed_page) = (Rc::clone(controller), Rc::clone(page));
    listen(svg, "keydown", move |e: web_sys::KeyboardEvent| {
        if target_of(e.target()).is_none() {
            return;
        }
        let order = target_order(&keyed.trace());
        if let Some(next) = move_roving(&order, keyed.view().roving, &e.key()) {
            e.prevent_default();
            keyed.act(Action::Focus(next));
            keyed_page.focus_target(next);
        }
    })?;
    let (pressed, pressed_page) = (Rc::clone(controller), Rc::clone(page));
    listen(svg, "pointerdown", move |e: web_sys::PointerEvent| {
        if let Some(target) = target_of(e.target()) {
            pressed.act(Action::Focus(target));
            pressed_page.focus_target(target);
        }
    })?;
    let gained = Rc::clone(controller);
    listen(svg, "focusin", move |e: web_sys::FocusEvent| {
        if let Some(target) = target_of(e.target()) {
            let view = gained.view();
            if view.focused != Some(target) || view.roving != target {
                gained.act(Action::Focus(target));
            }
        }
    })
}
