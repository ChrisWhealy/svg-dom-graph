//! `panel-marching-squares`: the marching squares algorithm on a sampled scalar field, one cell at a time, at three
//! levels. See [`build_marching_squares_demo`] for what the page does, [`solver`] for the algorithm, [`projection`] for
//! what each step looks like, [`field`] for the fields, and [`controller`] for the controls.
//!
//! The rest of this module is the browser side. [`page`] builds the scene, which is a real `Scene` with one plot node,
//! swaps it in, and draws the inspector. [`chrome`] draws the controls in the stage, outside the part that zooms, and
//! [`layout`] says where they are. [`scheduler`] runs the range's frames.

mod chrome;
mod controller;
mod dom;
mod field;
mod layout;
mod page;
mod projection;
mod scheduler;
mod solver;

use crate::plot_scene::listen;
use chrome::{PANEL, STAGE};
use controller::{Action, Controller, Surface};
use demo_common::util::{document, required_element};
use field::FieldId;
use page::Page;
use projection::{Show, State};
use scheduler::make_scheduler;
use solver::Trace;
use std::{cell::Cell, rc::Rc};
use svg_dom_graph::controls::svg_buttons;
use wasm_bindgen::JsCast;

/// This module's own full source, embedded at compile time — see `crate::source_frame`'s own doc comment for why.
pub(crate) const SOURCE: &str = include_str!("mod.rs");

/// The files behind this demo's one function: the algorithm, the fields, what each step looks like, the controller, and
/// the browser side.
pub(crate) const FILES: &[crate::SourceFile] = &[
    crate::source_file!("marching_squares/solver/mod.rs", "solver/mod.rs"),
    crate::source_file!("marching_squares/solver/stitch.rs", "solver/stitch.rs"),
    crate::source_file!("marching_squares/field.rs", "field.rs"),
    crate::source_file!("marching_squares/projection.rs", "projection.rs"),
    crate::source_file!("marching_squares/controller/mod.rs", "controller/mod.rs"),
    crate::source_file!("marching_squares/layout.rs", "layout.rs"),
    crate::source_file!("marching_squares/page.rs", "page.rs"),
    crate::source_file!("marching_squares/chrome.rs", "chrome.rs"),
    crate::source_file!("marching_squares/scheduler.rs", "scheduler.rs"),
    crate::source_file!("marching_squares/dom.rs", "dom.rs"),
];

thread_local! {
    /// Counted up for each demo that is built, and the number in every ID of its scenes.
    static INSTANCES: Cell<u32> = const { Cell::new(0) };
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The action that the button called `action` asks for.
fn action_for(action: &str) -> Option<Action> {
    match action {
        "restart" => Some(Action::Restart),
        "previous" => Some(Action::Previous),
        "next" => Some(Action::Next),
        "rebuild" => Some(Action::Rebuild),
        "retry" => Some(Action::Retry),
        _ => None,
    }
}

/// The cell that the row and column selectors name. A selector left at "none" while the other is not is taken as the
/// first, so that choosing a row alone still chooses a cell.
fn cell_from(row: &str, column: &str) -> Option<(usize, usize)> {
    match (row.parse().ok(), column.parse().ok()) {
        (None, None) => None,
        (r, c) => Some((r.unwrap_or(0), c.unwrap_or(0))),
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Builds the marching squares demo: the algorithm that turns a grid of numbers into contour lines.
///
/// Each cell of the grid has four corners, and each corner is above or below a level. Those four yes-or-no answers make
/// a number from 0 to 15, which says which sides of the cell the contour crosses and how the crossings are joined.
/// Where a contour crosses a side, it is placed by interpolating between the two samples, and the two cells that share
/// the side use the same point. Two cases are ambiguous, since the corners above the level could be joined or
/// separated: they are settled by the sign of `Q`, which is what the surface between the corners does at its saddle.
///
/// The page visits the cells in order and draws the contours of the cells so far, at three levels at once, in a scene
/// that can be zoomed and panned. Any cell can be inspected, and its four samples, case, crossings and decision are
/// shown enlarged. Every state comes from one trace that is worked out before anything is drawn, and a threshold can be
/// edited, which works out a new trace before anything is changed.
///
/// # Errors
///
/// Returns `Err` if the page is missing one of its elements, or if the first scene or a listener could not be made.
pub(crate) fn build_marching_squares_demo() -> Result<(), String> {
    let document = document()?;
    required_element(&document, PANEL)?;
    let field = FieldId::Ripple;
    let trace = Trace::build(field.grid().map_err(|e| e.to_string())?, field.levels()).map_err(|e| e.to_string())?;
    let instance = INSTANCES.with(|n| {
        n.set(n.get() + 1);
        n.get()
    });
    let page = Rc::new(Page::new(&document, instance, trace.grid.cell_count())?);
    let state = State {
        step: trace.grid.cell_count(),
        ..State::first(field)
    };
    let controller = Controller::new(trace, state, Rc::clone(&page) as Rc<dyn Surface>, make_scheduler()?);
    let weak = Rc::downgrade(&controller);
    page.set_click_handler(Rc::new(move |x, y| {
        if let Some(controller) = weak.upgrade() {
            if let Some(cell) = page::cell_of_click(&controller.trace(), x, y) {
                controller.act(Action::Select(Some(cell)));
            }
        }
    }));
    // The first scene is drawn the way every other is, so that a page that cannot draw it says so.
    page.first(&controller.trace(), &controller.state(), controller.meta())
        .map_err(|e| match e {
            controller::CommitError::Detached(p) | controller::CommitError::Live(p) => p,
        })?;

    wire_buttons(&document, &controller)?;
    wire_range(&controller, &page)?;
    wire_selectors(&controller, &page)?;
    wire_zoom(&page)?;
    wire_pan(&page)?;
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
/// drawn again even when the action then has nowhere to go. The library's listeners are on the stage, and also hear the
/// Enter and Space keys.
fn wire_buttons(document: &web_sys::Document, controller: &Rc<Controller>) -> Result<(), String> {
    let controller = Rc::clone(controller);
    svg_buttons::watch(document, STAGE, move |action| {
        if let Some(action) = action_for(action) {
            controller.act(action);
        }
    })
}

/// Makes the range work. Moving it draws the latest place at the next frame, and letting it rest draws that place at
/// once and says where it is, once.
fn wire_range(controller: &Rc<Controller>, page: &Rc<Page>) -> Result<(), String> {
    let seek = page.chrome().seek.clone();
    let value = {
        let seek = seek.clone();
        move || seek.value().parse::<usize>().unwrap_or(0)
    };
    let (moving, read) = (Rc::clone(controller), value.clone());
    listen(&seek, "input", move |_: web_sys::Event| moving.seek_input(read()))?;
    let resting = Rc::clone(controller);
    listen(&seek, "change", move |_: web_sys::Event| resting.seek_release(value()))
}

/// Makes the selectors and the number work. A control shows the committed state again after its change, which is the
/// old value if the change was refused.
fn wire_selectors(controller: &Rc<Controller>, page: &Rc<Page>) -> Result<(), String> {
    let c = page.chrome();
    let again = |controller: &Rc<Controller>, page: &Rc<Page>| {
        let _ = Surface::sync_controls(page.as_ref(), &controller.state());
    };
    let (field_ctl, field_page, select) = (Rc::clone(controller), Rc::clone(page), c.field.clone());
    listen(&c.field, "change", move |_: web_sys::Event| {
        if let Some(field) = FieldId::from_key(&select.value()) {
            field_ctl.act(Action::SetField(field));
        }
        again(&field_ctl, &field_page);
    })?;
    let (show_ctl, show_page, select) = (Rc::clone(controller), Rc::clone(page), c.show.clone());
    listen(&c.show, "change", move |_: web_sys::Event| {
        if let Some(show) = Show::from_key(&select.value()) {
            show_ctl.act(Action::SetShow(show));
        }
        again(&show_ctl, &show_page);
    })?;
    let (inspect_ctl, inspect_page, select) = (Rc::clone(controller), Rc::clone(page), c.inspect.clone());
    listen(&c.inspect, "change", move |_: web_sys::Event| {
        if let Ok(slot) = select.value().parse() {
            inspect_ctl.act(Action::SetInspected(slot));
        }
        again(&inspect_ctl, &inspect_page);
    })?;
    let (number_ctl, number_page, input) = (Rc::clone(controller), Rc::clone(page), c.threshold.clone());
    listen(&c.threshold, "change", move |_: web_sys::Event| {
        number_ctl.act(Action::SetThreshold(input.value().trim().parse().unwrap_or(f64::NAN)));
        again(&number_ctl, &number_page);
    })?;
    for control in [c.row.clone(), c.column.clone()] {
        let (cell_ctl, cell_page) = (Rc::clone(controller), Rc::clone(page));
        let (row, column) = (c.row.clone(), c.column.clone());
        listen(&control, "change", move |_: web_sys::Event| {
            cell_ctl.act(Action::Select(cell_from(&row.value(), &column.value())));
            again(&cell_ctl, &cell_page);
        })?;
    }
    Ok(())
}

/// Keeps the zoom in the footer right. The toolbar, the wheel and a drag change the view without a control of the page,
/// so the footer is told a moment after any of them.
fn wire_zoom(page: &Rc<Page>) -> Result<(), String> {
    let stage = page.chrome().stage.clone();
    for event in ["click", "wheel", "pointerup", "keyup"] {
        let page = Rc::clone(page);
        listen(&stage, event, move |_: web_sys::Event| {
            let page = Rc::clone(&page);
            let later = wasm_bindgen::closure::Closure::once_into_js(move || page.refresh_zoom());
            if let Some(window) = web_sys::window() {
                let _ = window.set_timeout_with_callback_and_timeout_and_arguments_0(later.unchecked_ref(), 60);
            }
        })?;
    }
    Ok(())
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Makes dragging the plot pan the view. The plot fills the scene, so the library's own surface for panning is behind
/// it and is never pressed. The press is heard on the host, which every scene is swapped into, and the moves and the
/// release on the window. The click that ends a drag is swallowed before the plot hears it, so that panning does not
/// choose a cell.
fn wire_pan(page: &Rc<Page>) -> Result<(), String> {
    let window: web_sys::EventTarget = web_sys::window().ok_or("no window")?.into();
    let host = page.chrome().host.clone();
    let pressed = Rc::clone(page);
    listen(&host, "pointerdown", move |e: web_sys::PointerEvent| {
        if e.button() != 0 {
            return;
        }
        if let Some(target) = e.target().and_then(|t| t.dyn_into::<web_sys::Element>().ok()) {
            pressed.pan_start(&target, f64::from(e.client_x()), f64::from(e.client_y()));
        }
    })?;
    let moved = Rc::clone(page);
    listen(&window, "pointermove", move |e: web_sys::PointerEvent| {
        moved.pan_move(f64::from(e.client_x()), f64::from(e.client_y()));
    })?;
    for event in ["pointerup", "pointercancel"] {
        let released = Rc::clone(page);
        listen(&window, event, move |_: web_sys::PointerEvent| released.pan_end())?;
    }
    // In the capture phase, so that it runs before the plot's own listener, which is below the host.
    let clicked = Rc::clone(page);
    let swallow = wasm_bindgen::closure::Closure::<dyn FnMut(web_sys::Event)>::new(move |e: web_sys::Event| {
        if clicked.take_swallowed_click() {
            e.stop_propagation();
        }
    });
    host.add_event_listener_with_callback_and_bool("click", swallow.as_ref().unchecked_ref(), true)
        .map_err(|e| format!("could not attach the click listener: {e:?}"))?;
    swallow.forget();
    Ok(())
}
