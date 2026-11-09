//! `panel-n-queens`: the whole depth-first search of the N-Queens puzzle, one event at a time. See
//! [`build_n_queens_demo`] for what the page does, [`solver`] for the search, [`projection`] for what each step looks
//! like, and [`controller`] for the buttons, the range and the playback.
//!
//! The rest of this module is the browser side. [`layout`] says where everything is in the scene, [`controls`] draws
//! the selectors, the buttons and the range in it, and [`board`] draws the layers of one board. [`chrome`] finds the
//! elements that the panel's HTML declares, [`page`] is what the controller draws on, and [`scheduler`] runs the
//! playback's ticks.

mod board;
mod chrome;
mod controller;
mod controls;
mod dom;
mod layout;
mod page;
mod projection;
mod scheduler;
mod solver;

use crate::plot_scene::listen;
use controller::{Action, Controller, SeekRequest, Surface};
use controls::{SIZES, SPEEDS};
use demo_common::{
    force::animation::AnimationLoop,
    util::{document, ensure_svg_in, required_element},
};
use dom::set;
use layout::SCENE;
use page::Page;
use projection::move_active;
use scheduler::make_scheduler;
use solver::Trace;
use std::rc::Rc;
use svg_dom_graph::controls::svg_buttons;
use wasm_bindgen::JsCast;

/// This module's own full source, embedded at compile time — see `crate::source_frame`'s own doc comment for why.
pub(crate) const SOURCE: &str = include_str!("mod.rs");

/// The files behind this demo's one function: the search, what each step looks like, the controls, and the browser
/// side.
pub(crate) const FILES: &[crate::SourceFile] = &[
    crate::source_file!("n_queens/solver/mod.rs", "solver/mod.rs"),
    crate::source_file!("n_queens/projection/mod.rs", "projection/mod.rs"),
    crate::source_file!("n_queens/controller/mod.rs", "controller/mod.rs"),
    crate::source_file!("n_queens/layout/mod.rs", "layout/mod.rs"),
    crate::source_file!("n_queens/controls.rs", "controls.rs"),
    crate::source_file!("n_queens/board.rs", "board.rs"),
    crate::source_file!("n_queens/page.rs", "page.rs"),
    crate::source_file!("n_queens/chrome.rs", "chrome.rs"),
    crate::source_file!("n_queens/scheduler.rs", "scheduler.rs"),
    crate::source_file!("n_queens/dom.rs", "dom.rs"),
];

/// The ids of the elements that the page declares. The stage is empty, and gets an `<svg>` of the same name.
const STAGE: &str = "n-queens-stage";
const SVG: &str = "n-queens-stage-svg";

/// The size of the board when the page opens, and how many events are shown in each second at first.
const DEFAULT_SIZE: usize = 8;
const FIRST_SPEED: u32 = 20;

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The square that a pointer or focus event is on, from the `data-row` and `data-col` of its gridcell.
fn cell_of(target: Option<web_sys::EventTarget>) -> Option<(usize, usize)> {
    let cell = target
        .and_then(|t| t.dyn_into::<web_sys::Element>().ok())?
        .closest("[role=gridcell]")
        .ok()
        .flatten()?;
    let number = |name: &str| cell.get_attribute(name).and_then(|value| value.parse().ok());
    Some((number("data-row")?, number("data-col")?))
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Whether `target` is inside a square of the grid.
fn in_grid(target: Option<web_sys::EventTarget>) -> bool {
    cell_of(target).is_some()
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Builds the N-Queens demo: the whole depth-first search of the puzzle, one event at a time.
///
/// The search tries the rows in order, and the columns of a row in order. Each square is a Try, which is followed at
/// once by a Reject, if a queen already attacks it, or a Place. A Remove takes a queen back when the search returns.
/// Nothing is skipped because of a symmetry, so the page shows every one of the 33,589 events of a board of eight, and
/// its 92 solutions.
///
/// The page shows:
///
/// 1. The board of 4 to 9 queens, as a grid whose squares a person can move between with the arrow keys. A Try outlines
///    its square and draws the three lines through it, whether or not the square is free. A Reject marks the square,
///    and draws the line that is the reason heavier than the other two. A solution is framed.
/// 2. The status: the step, the event, the square, the queens placed, the reason, the solutions found, and the
///    playback.
/// 3. Above the board, in the same scene: selectors for the size of the board and the speed, buttons for Restart,
///    Previous, Next, Play or Pause and the previous and the next solution, and a range that goes straight to any step.
///    The buttons and the range are the library's own, and a selector is the browser's own `<select>`.
///
/// Every step comes from the one trace that is worked out when the board is chosen, and the page draws only what
/// differs from the step before. A write to the page that fails is reported, and the next button, key or move of the
/// range draws the whole board again.
///
/// # Errors
///
/// Returns `Err` if the page is missing one of its elements, or if the board or a listener could not be made.
pub(crate) fn build_n_queens_demo() -> Result<(), String> {
    let document = document()?;
    ensure_svg_in(&document, STAGE, SVG, SCENE)?;
    let svg = required_element(&document, SVG)?;
    set(&svg, "role", "group")?;
    set(&svg, "aria-label", "N-Queens search board")?;
    let trace = Trace::build(DEFAULT_SIZE).map_err(|e| e.to_string())?;
    let page = Rc::new(Page::new(&document, &svg, &trace, FIRST_SPEED)?);
    let controller = Controller::new(trace, Rc::clone(&page) as Rc<dyn Surface>, make_scheduler()?, FIRST_SPEED);

    wire_buttons(&document, &controller)?;
    wire_seek(&controller, &page)?;
    wire_selectors(&controller, &page)?;
    wire_board(&svg, &page)?;
    let leaving = Rc::clone(&controller);
    listen(
        &web_sys::window().ok_or("no window")?.into(),
        "pagehide",
        move |_: web_sys::Event| {
            leaving.teardown();
        },
    )?;
    Ok(())
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// What a press of the button for `action` asks the controller to do. Play is Pause while it plays.
fn action_for(action: &str, playing: bool) -> Option<Action> {
    match action {
        "restart" => Some(Action::Restart),
        "previous" => Some(Action::Previous),
        "next" => Some(Action::Next),
        "previous-solution" => Some(Action::PreviousSolution),
        "next-solution" => Some(Action::NextSolution),
        "play" => Some(if playing { Action::Pause } else { Action::Play }),
        _ => None,
    }
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
/// Makes the range work. Moving it asks for the latest place to be drawn at the next frame, and letting it rest draws
/// that place at once, so the page never says a step that is not on the board.
fn wire_seek(controller: &Rc<Controller>, page: &Rc<Page>) -> Result<(), String> {
    let request = Rc::new(SeekRequest::default());
    let frame = {
        let (controller, request) = (Rc::clone(controller), Rc::clone(&request));
        AnimationLoop::new(move |_| {
            if let Some(index) = request.take(controller.generation()) {
                controller.act(Action::Seek(index));
            }
        })?
    };
    let value = {
        let page = Rc::clone(page);
        move || page.controls().seek().value().parse::<usize>().unwrap_or(0)
    };
    let moved = value.clone();
    let (asked, asking, moving) = (Rc::clone(&request), Rc::clone(&frame), Rc::clone(controller));
    listen(&page.controls().seek().clone(), "input", move |_: web_sys::Event| {
        if asked.request(moved(), moving.generation()) {
            asking.request();
        }
    })?;
    let controller = Rc::clone(controller);
    listen(&page.controls().seek().clone(), "change", move |_: web_sys::Event| {
        frame.cancel();
        let index = request.take(controller.generation()).unwrap_or_else(&value);
        controller.act(Action::Seek(index));
        if !controller.is_dirty() {
            controller.announce_current();
        }
    })
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Makes the selectors for the size of the board and the speed work.
fn wire_selectors(controller: &Rc<Controller>, page: &Rc<Page>) -> Result<(), String> {
    let size = page.controls().size_selector().clone();
    let speed = page.controls().speed_selector().clone();
    let (controller_for_size, page_for_size, chosen) = (Rc::clone(controller), Rc::clone(page), size.clone());
    listen(&size, "change", move |_: web_sys::Event| {
        let Some(n) = chosen.value().parse::<usize>().ok().filter(|n| SIZES.contains(n)) else {
            return;
        };
        match Trace::build(n) {
            Ok(trace) => {
                controller_for_size.change_size(trace);
            },
            Err(problem) => page_for_size.failed(&problem.to_string()),
        }
        // The selector says the board that is on the page, which is the old one if the new one could not be made.
        chosen.set_value(&page_for_size.size().to_string());
        page_for_size.show_playback();
    })?;
    let (controller, page, chosen) = (Rc::clone(controller), Rc::clone(page), speed.clone());
    listen(&speed, "change", move |_: web_sys::Event| {
        if let Some(per_second) = chosen.value().parse::<u32>().ok().filter(|n| SPEEDS.contains(n)) {
            controller.set_speed(per_second);
            page.set_speed(per_second);
        }
    })
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Makes the squares work with the keyboard and the pointer. The listeners are on the scene, which the board is
/// replaced inside, so that a new size needs none of its own.
fn wire_board(svg: &web_sys::Element, page: &Rc<Page>) -> Result<(), String> {
    let typed = Rc::clone(page);
    listen(svg, "keydown", move |e: web_sys::KeyboardEvent| {
        if !in_grid(e.target()) {
            return;
        }
        if let Some(to) = move_active(&e.key(), e.ctrl_key(), typed.active(), typed.size()) {
            e.prevent_default();
            typed.move_to(to);
        }
    })?;
    let pressed = Rc::clone(page);
    listen(svg, "pointerdown", move |e: web_sys::PointerEvent| {
        if let Some(cell) = cell_of(e.target()) {
            pressed.move_to(cell);
        }
    })?;
    let gained = Rc::clone(page);
    listen(svg, "focusin", move |e: web_sys::FocusEvent| {
        if let Some(cell) = cell_of(e.target()) {
            gained.set_active(cell);
            gained.show_ring(true);
        }
    })?;
    let lost = Rc::clone(page);
    listen(svg, "focusout", move |e: web_sys::FocusEvent| {
        if !in_grid(e.related_target()) {
            lost.show_ring(false);
        }
    })
}
