//! `panel-tarjan`: Tarjan's strongly connected components on a fixed graph of 64 vertices and 114 edges, one event at a
//! time. See [`build_tarjan_demo`] for what the page does, [`solver`] for the algorithm, [`projection`] for what each
//! step looks like, [`view`] for when the source view moves, and [`controller`] for the buttons, the follow and the
//! playback.
//!
//! The rest of this module is the browser side. [`layout`] and [`ui_layout`] say where everything is, [`scene`] builds
//! the scene, which holds the graph, the stack, the roster and the condensation, and [`controls`] draws the selector,
//! the buttons and the range. [`chrome`] finds the elements that the panel's HTML declares, [`page`] is what the
//! controller draws on, [`scheduler`] runs the playback's ticks, and [`listeners`] keeps every listener so that it can
//! be taken off.

mod chrome;
mod condensation;
mod controller;
mod controls;
mod dom;
mod fixture;
mod layout;
mod listeners;
mod motion;
mod page;
mod projection;
mod relax;
mod scene;
mod scheduler;
mod solver;
mod ui_layout;
mod view;

use chrome::CONTROLS;
use controller::{Action, Controller, Surface};
use demo_common::util::{document, required_element};
use listeners::Listeners;
use page::Page;
use projection::{Speed, Target, move_roving, target_order};
use scheduler::make_scheduler;
use solver::Trace;
use std::{
    cell::{Cell, RefCell},
    rc::{Rc, Weak},
};
use wasm_bindgen::{JsCast, JsValue, closure::Closure};

/// This module's own full source, embedded at compile time — see `crate::source_frame`'s own doc comment for why.
pub(crate) const SOURCE: &str = include_str!("mod.rs");

/// The files behind this demo's one function: the fixture, the algorithm, the condensation, what each step looks like,
/// where the view goes, the controls, and the browser side.
pub(crate) const FILES: &[crate::SourceFile] = &[
    crate::source_file!("tarjan/fixture/mod.rs", "fixture/mod.rs"),
    crate::source_file!("tarjan/solver/mod.rs", "solver/mod.rs"),
    crate::source_file!("tarjan/condensation/mod.rs", "condensation/mod.rs"),
    crate::source_file!("tarjan/projection/mod.rs", "projection/mod.rs"),
    crate::source_file!("tarjan/view/mod.rs", "view/mod.rs"),
    crate::source_file!("tarjan/controller/mod.rs", "controller/mod.rs"),
    crate::source_file!("tarjan/layout/mod.rs", "layout/mod.rs"),
    crate::source_file!("tarjan/ui_layout.rs", "ui_layout.rs"),
    crate::source_file!("tarjan/scene.rs", "scene.rs"),
    crate::source_file!("tarjan/relax/mod.rs", "relax/mod.rs"),
    crate::source_file!("tarjan/motion.rs", "motion.rs"),
    crate::source_file!("tarjan/controls.rs", "controls.rs"),
    crate::source_file!("tarjan/page.rs", "page.rs"),
    crate::source_file!("tarjan/chrome.rs", "chrome.rs"),
    crate::source_file!("tarjan/scheduler.rs", "scheduler.rs"),
    crate::source_file!("tarjan/listeners.rs", "listeners.rs"),
    crate::source_file!("tarjan/dom.rs", "dom.rs"),
];

thread_local! {
    /// Counted up for each demo that is built, and the number in every ID of its scenes.
    static INSTANCES: Cell<u32> = const { Cell::new(0) };
    /// The demos that are on the page, which is where each one is owned.
    static LIVE: RefCell<Vec<Demo>> = const { RefCell::new(Vec::new()) };
}

/// A demo that is on the page: the controller, the page that it draws on, and every listener that was added for them.
/// The listeners hold the controller and the page weakly, so this is what keeps them alive, until the demo is shut
/// down.
struct Demo {
    instance: u32,
    controller: Rc<Controller>,
    _page: Rc<Page>,
    listeners: Listeners,
}

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

/// Calls `f` with what `weak` points to, if it is still there.
fn with<T>(weak: &Weak<T>, f: impl FnOnce(&T)) {
    if let Some(strong) = weak.upgrade() {
        f(&strong);
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The vertex or the edge that an event is on, from the `data-tj-*` attributes of its hit target.
fn target_of(target: Option<web_sys::EventTarget>) -> Option<Target> {
    let element = target?.dyn_into::<web_sys::Element>().ok()?;
    if let Some(vertex) = element.closest("[data-tj-vertex]").ok().flatten() {
        let name = vertex.get_attribute("data-tj-vertex")?;
        return name.strip_prefix('v')?.parse().ok().map(Target::Vertex);
    }
    let edge = element.closest("[data-tj-edge-hit]").ok().flatten()?;
    edge.get_attribute("data-tj-edge-hit")?
        .strip_prefix('e')?
        .parse()
        .ok()
        .map(Target::Edge)
}

/// What a press of the button for `action` asks the controller to do. Play is Pause while it plays, and the follow
/// button turns following on or off.
fn action_for(action: &str, playing: bool, follow: bool) -> Option<Action> {
    match action {
        "restart" => Some(Action::Restart),
        "previous" => Some(Action::Previous),
        "next" => Some(Action::Next),
        "previous-component" => Some(Action::PreviousComponent),
        "next-component" => Some(Action::NextComponent),
        "jump-final" => Some(Action::JumpFinal),
        "fit" => Some(Action::Fit),
        "follow" => Some(Action::SetFollow(!follow)),
        "play" => Some(if playing { Action::Pause } else { Action::Play }),
        _ => None,
    }
}

/// Whether the springs are left out, which a test asks for so that the last step holds still.
fn springs_left_out() -> bool {
    web_sys::window()
        .and_then(|window| window.location().search().ok())
        .is_some_and(|query| query.contains("no-springs"))
}

fn always_full_requested() -> bool {
    web_sys::window()
        .and_then(|window| window.location().search().ok())
        .is_some_and(|query| query.contains("always-full"))
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The Tarjan scene is large, so the gallery panel keeps only the description, plus a button that opens the scene in a
/// window of its own, as large as the screen allows: `tarjan-scc-window.html`, which runs [`build_tarjan_window`]
/// there. A named window is reused.
///
/// # Errors
///
/// Returns `Err` if `index.html` is missing `#tj-open`, or if a listener could not be attached to it.
pub(crate) fn build_tarjan_demo() -> Result<(), String> {
    let button = required_element(&document()?, "tj-open")?;
    let open = Closure::<dyn FnMut()>::new(move || {
        let Some(window) = web_sys::window() else { return };
        // The window opens as large as the screen allows, since the picture is shown whole, scaled to fit it.
        let (width, height) = window
            .screen()
            .ok()
            .map(|s| (s.avail_width().unwrap_or(1600), s.avail_height().unwrap_or(900)))
            .unwrap_or((1600, 900));
        let features = format!("popup,left=0,top=0,width={width},height={height}");
        // A blocked pop-up is the user's own choice, and there is nowhere to report it to. It is ignored.
        if let Ok(Some(popup)) = window.open_with_url_and_target_and_features(WINDOW_PAGE, WINDOW_NAME, &features) {
            let _ = popup.focus();
        }
    });
    button
        .add_event_listener_with_callback("click", open.as_ref().unchecked_ref())
        .map_err(|e| format!("could not attach the open-window listener to #tj-open: {e:?}"))?;
    open.forget();
    Ok(())
}

/// The page [`build_tarjan_demo`]'s button opens.
const WINDOW_PAGE: &str = "tarjan-scc-window.html";
/// The window's own name, so a second click reuses it.
const WINDOW_NAME: &str = "svg-dom-graph-tarjan";

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Builds the Tarjan demo in its window: the strongly connected components of a directed graph, found one event at a
/// time.
///
/// A depth-first search gives each vertex an index and a lowlink, and keeps a stack of the vertices that are not yet in
/// a component. When a vertex's lowlink is still its own index, everything above it on the stack is one component, and
/// all of it is popped at once. The page shows, from one snapshot of the run, in one scene:
///
/// 1. The graph of 64 vertices and 114 edges, in ten regions, with each vertex showing its index and its lowlink, and a
///    dashed outline round each component as soon as it is found.
/// 2. At the left of the graph, the stack of the algorithm, which is not the chain of calls, and next to it the roster
///    of the components found so far.
/// 3. Below the graph, the condensation, with one node for each component and an arrow wherever the graph has an edge
///    from one to another. Its nodes are placed by a fixed table, so none moves once it is shown.
///
/// Buttons step, jump between the components, play, and fit the region that the search is in, and a range goes to any
/// step. Following keeps the search in view while it plays and while a person steps, and a pan or a zoom by hand turns
/// it off. Every step comes from one trace that is worked out before anything is drawn, and the page draws only what
/// differs from the step before.
///
/// # Errors
///
/// Returns `Err` if the window is missing one of its elements, or if the scene or a listener could not be made.
pub(crate) fn build_tarjan_window() -> Result<(), String> {
    let document = document()?;
    let graph = fixture::graph();
    let edges = graph.edges();
    let trace = Trace::build(&graph).map_err(|e| e.to_string())?;
    let instance = INSTANCES.with(|n| {
        n.set(n.get() + 1);
        n.get()
    });
    let page = Rc::new(Page::new(
        &document,
        &edges,
        &trace,
        instance,
        always_full_requested(),
        !springs_left_out(),
    )?);
    let layout = page.board().layout.clone();
    page.board().set_camera(view::initial(&layout, page.board().viewport()))?;
    let controller = Controller::new(graph, trace, layout, Rc::clone(&page) as Rc<dyn Surface>, make_scheduler()?);

    let listeners = match wire_all(&document, &controller, &page) {
        Ok(listeners) => listeners,
        Err(problem) => {
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
    // This one listener is deliberately kept for the life of the page. It holds only a number, so it frees nothing that
    // it must not, and it does nothing the second time that it is called.
    let leaving = Closure::<dyn FnMut(web_sys::Event)>::new(move |_: web_sys::Event| shut_down(instance));
    web_sys::window()
        .ok_or("no window")?
        .add_event_listener_with_callback("pagehide", leaving.as_ref().unchecked_ref())
        .map_err(|e| format!("could not attach the pagehide listener: {e:?}"))?;
    leaving.forget();
    Ok(())
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Adds every listener that the page needs, and gives the registry that holds them.
///
/// # Errors
///
/// Returns `Err` if a listener could not be attached. The registry is then dropped, which takes off what had been
/// added.
fn wire_all(document: &web_sys::Document, controller: &Rc<Controller>, page: &Rc<Page>) -> Result<Listeners, String> {
    let mut listeners = Listeners::default();
    let (controller, page) = (Rc::downgrade(controller), Rc::downgrade(page));
    wire_buttons(&mut listeners, document, &controller)?;
    wire_range(&mut listeners, &controller, &page)?;
    wire_speed(&mut listeners, &controller, &page)?;
    wire_graph(&mut listeners, &controller, &page)?;
    wire_view_input(&mut listeners, &controller, &page)?;
    wire_resize(&mut listeners, &page)?;
    wire_lists(&mut listeners, &controller, &page)?;
    Ok(listeners)
}

fn button_action(target: Option<web_sys::EventTarget>) -> Option<String> {
    target
        .and_then(|t| t.dyn_into::<web_sys::Element>().ok())
        .and_then(|e| e.closest("[data-action]").ok().flatten())
        .and_then(|button| button.get_attribute("data-action"))
}

/// Makes the buttons work, with one listener for clicks and one for the Enter and Space keys on the strip. A button
/// that is only dimmed still gets its press, so that a page that is out of date is drawn again.
fn wire_buttons(
    listeners: &mut Listeners,
    document: &web_sys::Document,
    controller: &Weak<Controller>,
) -> Result<(), String> {
    let strip: web_sys::EventTarget = required_element(document, CONTROLS)?.into();
    let press = |controller: &Weak<Controller>, action: &str| {
        with(controller, |controller| {
            let choices = controller.choices();
            if let Some(action) = action_for(action, controller.is_playing(), choices.follow) {
                controller.act(action);
            }
        });
    };
    let clicked = controller.clone();
    listeners.add(&strip, "click", move |e: web_sys::Event| {
        if let Some(action) = button_action(e.target()) {
            press(&clicked, &action);
        }
    })?;
    let keyed = controller.clone();
    listeners.add(&strip, "keydown", move |e: web_sys::KeyboardEvent| {
        if e.key() == "Enter" || e.key() == " " {
            if let Some(action) = button_action(e.target()) {
                e.prevent_default();
                press(&keyed, &action);
            }
        }
    })
}

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

fn wire_speed(listeners: &mut Listeners, controller: &Weak<Controller>, page: &Weak<Page>) -> Result<(), String> {
    let Some(speed) = page.upgrade().map(|page| page.controls().speed_selector().clone()) else {
        return Err("the page was dropped before its selector was wired".to_owned());
    };
    let (setting, chosen) = (controller.clone(), speed.clone());
    listeners.add(&speed, "change", move |_: web_sys::Event| {
        with(&setting, |controller| {
            if let Some(speed) = Speed::from_key(&chosen.value()) {
                controller.act(Action::SetSpeed(speed));
            }
        });
    })
}

/// Makes the vertices and the edges work with the keyboard and the pointer. The listeners are on the scene, so that
/// each target needs none of its own. Moving the focus selects, and a click selects, and neither moves the step.
fn wire_graph(listeners: &mut Listeners, controller: &Weak<Controller>, page: &Weak<Page>) -> Result<(), String> {
    let Some(svg) = page.upgrade().map(|page| page.board().svg().clone()) else {
        return Err("the page was dropped before its graph was wired".to_owned());
    };
    let svg: web_sys::EventTarget = svg.into();
    let (keyed, keyed_page) = (controller.clone(), page.clone());
    listeners.add(&svg, "keydown", move |e: web_sys::KeyboardEvent| {
        let Some(from) = target_of(e.target()) else {
            return;
        };
        let key = e.key();
        if matches!(key.as_str(), "Enter" | " ") {
            e.prevent_default();
            with(&keyed, |controller| controller.act(Action::Select(from)));
            with(&keyed_page, |page| page.focus_target(from));
            return;
        }
        with(&keyed, |controller| {
            let order = target_order(fixture::VERTICES, fixture::EDGES);
            if let Some(next) = move_roving(&order, controller.choices().roving, &key) {
                e.prevent_default();
                controller.act(Action::Select(next));
                with(&keyed_page, |page| page.focus_target(next));
            }
        });
    })?;
    let clicked = controller.clone();
    listeners.add(&svg, "click", move |e: web_sys::MouseEvent| {
        if let Some(target) = target_of(e.target()) {
            with(&clicked, |controller| controller.act(Action::Select(target)));
        }
    })?;
    let gained = controller.clone();
    listeners.add(&svg, "focusin", move |e: web_sys::FocusEvent| {
        if let Some(target) = target_of(e.target()) {
            with(&gained, |controller| {
                let choices = controller.choices();
                if choices.selected != Some(target) || choices.roving != target {
                    controller.act(Action::Select(target));
                }
            });
        }
    })
}

/// Runs `f` after the current handlers, and again at the next animation frame, which is when a library that updates the
/// view in a frame of its own has done it.
fn after_input(f: impl Fn() + Clone + 'static) {
    let soon = f.clone();
    let microtask = Closure::once(move |_: JsValue| soon());
    let _ = js_sys::Promise::resolve(&JsValue::UNDEFINED).then(&microtask);
    microtask.forget();
    if let Some(window) = web_sys::window() {
        let later = Closure::once_into_js(f);
        let _ = window.request_animation_frame(later.unchecked_ref());
    }
}

/// Makes the controller notice a pan or a zoom by hand. A wheel turn, a key and a click on the zoom toolbar ask for a
/// look at the view after the handlers have run. A pointer that goes down on the scene starts a gesture, which the
/// controller does not move the view under, and which ends when the pointer is let go, cancelled or loses its capture,
/// one frame after.
fn wire_view_input(listeners: &mut Listeners, controller: &Weak<Controller>, page: &Weak<Page>) -> Result<(), String> {
    let Some(svg) = page.upgrade().map(|page| page.board().svg().clone()) else {
        return Err("the page was dropped before its view input was wired".to_owned());
    };
    let svg: web_sys::EventTarget = svg.into();
    for event in ["wheel", "keydown", "click"] {
        let watching = controller.clone();
        listeners.add(&svg, event, move |_: web_sys::Event| {
            let watching = watching.clone();
            after_input(move || with(&watching, |controller| controller.observe_view()));
        })?;
    }
    let started = controller.clone();
    listeners.add(&svg, "pointerdown", move |_: web_sys::PointerEvent| {
        with(&started, |controller| controller.gesture(true));
    })?;
    let window: web_sys::EventTarget = web_sys::window().ok_or("no window")?.into();
    for (target, event) in [(&window, "pointerup"), (&window, "pointercancel"), (&svg, "lostpointercapture")] {
        let ending = controller.clone();
        listeners.add(target, event, move |_: web_sys::PointerEvent| {
            let ending = ending.clone();
            after_input(move || with(&ending, |controller| controller.gesture(false)));
        })?;
    }
    Ok(())
}

/// Keeps the toolbar and the surface for panning at the edge of the scene when the window is resized, which the scene
/// cannot see for itself.
fn wire_resize(listeners: &mut Listeners, page: &Weak<Page>) -> Result<(), String> {
    let window: web_sys::EventTarget = web_sys::window().ok_or("no window")?.into();
    let resized = page.clone();
    listeners.add(&window, "resize", move |_: web_sys::Event| {
        with(&resized, |page| {
            // A layout that cannot be written is left as it was. The next resize tries again.
            let _ = page.board().refresh_layout();
        });
    })
}

/// Makes a component in the roster or in the condensation pick it out, with a click or Enter or Space, and the button
/// next to the notice about the view fit the region.
fn wire_lists(listeners: &mut Listeners, controller: &Weak<Controller>, page: &Weak<Page>) -> Result<(), String> {
    let Some(svg) = page.upgrade().map(|page| page.board().svg().clone()) else {
        return Err("the page was dropped before its lists were wired".to_owned());
    };
    let svg: web_sys::EventTarget = svg.into();
    let pick = |controller: &Weak<Controller>, target: Option<web_sys::EventTarget>| -> bool {
        let Some(component) = target
            .and_then(|t| t.dyn_into::<web_sys::Element>().ok())
            .and_then(|e| e.closest("[data-tj-roster], [data-tj-dag-node]").ok().flatten())
            .and_then(|e| e.get_attribute("data-tj-component"))
            .and_then(|c| c.parse::<usize>().ok())
        else {
            return false;
        };
        with(controller, |controller| {
            let now = controller.choices().component;
            controller.act(Action::SelectComponent(if now == Some(component) {
                None
            } else {
                Some(component)
            }));
        });
        true
    };
    let picking = controller.clone();
    listeners.add(&svg, "click", move |e: web_sys::MouseEvent| {
        pick(&picking, e.target());
    })?;
    let keyed = controller.clone();
    listeners.add(&svg, "keydown", move |e: web_sys::KeyboardEvent| {
        if (e.key() == "Enter" || e.key() == " ") && pick(&keyed, e.target()) {
            e.prevent_default();
        }
    })?;
    let fit: web_sys::EventTarget = required_element(&document()?, "tj-fit-inline")?.into();
    let fitting = controller.clone();
    listeners.add(&fit, "click", move |_: web_sys::Event| {
        with(&fitting, |controller| controller.act(Action::Fit));
    })
}
