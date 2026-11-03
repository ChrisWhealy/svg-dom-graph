//! `panel-red-black-tree`: a red-black tree built by six insertions and then a deletion of a node with two children,
//! one event at a time. See [`build_red_black_tree_demo`] for what the page does, [`solver`] for the algorithm,
//! [`a11y`] for the keyboard and the accessible tree, [`layout`] for where each key is, and [`controller`] for the
//! buttons, the range and the playback.
//!
//! The rest of this module is the browser side. [`page`] builds each snapshot as a real `Scene` with a semantic overlay
//! above it, swaps it in, and puts the focus back. [`chrome`] finds the elements that the panel's HTML declares, and
//! [`scheduler`] runs the playback's ticks.

mod a11y;
mod chrome;
mod controller;
mod dom;
mod layout;
mod page;
mod scheduler;
mod solver;

use crate::plot_scene::listen;
use a11y::Nav;
use chrome::{PANEL, STAGE};
use controller::{Action, Controller, Surface};
use demo_common::util::{document, required_element};
use page::Page;
use scheduler::make_scheduler;
use solver::{Event, Key, Op, Phase, Trace};
use std::{cell::Cell, rc::Rc};
use svg_dom_graph::controls::svg_buttons;
use wasm_bindgen::JsCast;

/// This module's own full source, embedded at compile time — see `crate::source_frame`'s own doc comment for why.
pub(crate) const SOURCE: &str = include_str!("mod.rs");

/// The files behind this demo's one function: the algorithm, the accessible tree and keyboard, the layout, the
/// controller, and the browser side.
pub(crate) const FILES: &[crate::SourceFile] = &[
    crate::source_file!("red_black_tree/solver/mod.rs", "solver/mod.rs"),
    crate::source_file!("red_black_tree/a11y/mod.rs", "a11y/mod.rs"),
    crate::source_file!("red_black_tree/layout/mod.rs", "layout/mod.rs"),
    crate::source_file!("red_black_tree/controller/mod.rs", "controller/mod.rs"),
    crate::source_file!("red_black_tree/page.rs", "page.rs"),
    crate::source_file!("red_black_tree/chrome.rs", "chrome.rs"),
    crate::source_file!("red_black_tree/scheduler.rs", "scheduler.rs"),
    crate::source_file!("red_black_tree/dom.rs", "dom.rs"),
];

/// Six insertions, then the deletion of 15, which has two children.
const RUN: [Op; 7] = [
    Op::Insert(10),
    Op::Insert(5),
    Op::Insert(1),
    Op::Insert(15),
    Op::Insert(20),
    Op::Insert(12),
    Op::Delete(15),
];

thread_local! {
    /// Counted up for each demo that is built, and the number in every ID of its scenes.
    static INSTANCES: Cell<u32> = const { Cell::new(0) };
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The snapshots that the three jump buttons go to: the right rotation of the construction, its left rotation, and the
/// first event of the deletion. Each is found by what it is, and not by its number.
fn jump_targets(trace: &Trace) -> Result<Vec<usize>, String> {
    let find = |phase, wanted: &dyn Fn(&Event) -> bool, what: &str| {
        trace.find(phase, wanted).ok_or_else(|| format!("the run has no {what}"))
    };
    Ok(vec![
        find(Phase::Insert, &|e| matches!(e, Event::RotateRight(10)), "right rotation at 10")?,
        find(Phase::Insert, &|e| matches!(e, Event::RotateLeft(10)), "left rotation at 10")?,
        find(
            Phase::Delete,
            &|e| matches!(e, Event::ChooseSuccessor { deleted: 15, .. }),
            "choice of a successor for 15",
        )?,
    ])
}

/// The action that the button called `action` asks for. Play is Pause while it plays.
fn action_for(action: &str, playing: bool) -> Option<Action> {
    match action {
        "restart" => Some(Action::Restart),
        "previous" => Some(Action::Previous),
        "next" => Some(Action::Next),
        "play" => Some(if playing { Action::Pause } else { Action::Play }),
        other => other.strip_prefix("jump-")?.parse().ok().map(Action::Jump),
    }
}

/// The key of the item that an event is on.
fn key_of(target: Option<web_sys::EventTarget>) -> Option<Key> {
    let element = target?.dyn_into::<web_sys::Element>().ok()?;
    element
        .closest("[data-rbt-key]")
        .ok()
        .flatten()?
        .get_attribute("data-rbt-key")?
        .parse()
        .ok()
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Builds the red-black tree demo: a binary search tree that keeps itself balanced by colouring each node red or black,
/// and by rotating.
///
/// The run inserts 10, 5, 1, 15, 20 and 12, which shows a right rotation, a recolouring that moves a conflict up, and a
/// left rotation. It then deletes 15, which has two children. The node that takes its place is the original 20, moved
/// and not copied, and the repair that follows rotates twice more.
///
/// The picture is a real `Scene` of circles and connectors, drawn again for every step. Above it is a tree that a
/// keyboard and a screen reader can use: Tab enters it once, the arrow keys move through it, and Enter or Space
/// selects. What you select stays selected as a choice, so if you select 15 and the run passes its deletion, 20 is
/// shown as selected, and going back shows 15 again. Every step comes from one trace that is worked out before anything
/// is drawn.
///
/// # Errors
///
/// Returns `Err` if the page is missing one of its elements, or if the first tree or a listener could not be made.
pub(crate) fn build_red_black_tree_demo() -> Result<(), String> {
    let document = document()?;
    required_element(&document, PANEL)?;
    required_element(&document, "rbt-help")?.set_text_content(Some(a11y::KEYBOARD_HELP));
    let trace = Rc::new(Trace::build(&RUN).map_err(|e| e.to_string())?);
    let instance = INSTANCES.with(|n| {
        n.set(n.get() + 1);
        n.get()
    });
    let page = Rc::new(Page::new(&document, instance, trace.last())?);
    let controller = Controller::new(
        Rc::clone(&trace),
        jump_targets(&trace)?,
        Rc::clone(&page) as Rc<dyn Surface>,
        make_scheduler()?,
    );
    // The first tree is drawn the way every other is, so that a page that cannot draw it says so.
    page.commit(&trace, 0, &controller.view()).map_err(|e| match e {
        controller::CommitError::Detached(p) | controller::CommitError::Live(p) => p,
    })?;

    wire_buttons(&document, &controller)?;
    wire_range(&controller, &page)?;
    wire_tree(&controller, &page)?;
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
        if let Some(action) = action_for(action, controller.is_playing()) {
            controller.act(action);
        }
    })
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Makes the range work. Moving it goes to that step without a word, and letting it rest says where it is, once.
fn wire_range(controller: &Rc<Controller>, page: &Rc<Page>) -> Result<(), String> {
    let seek = page.chrome().seek.clone();
    let value = {
        let seek = seek.clone();
        move || seek.value().parse::<usize>().unwrap_or(0)
    };
    let (moving, read) = (Rc::clone(controller), value);
    listen(&seek, "input", move |_: web_sys::Event| moving.act(Action::SeekQuiet(read())))?;
    let resting = Rc::clone(controller);
    listen(&seek, "change", move |_: web_sys::Event| resting.seek_release())
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Makes the tree work with the keyboard and the pointer. The listeners are on the stage, which each tree is swapped
/// into, so that a new tree needs none of its own.
fn wire_tree(controller: &Rc<Controller>, page: &Rc<Page>) -> Result<(), String> {
    let stage = page.chrome().stage.clone();
    debug_assert_eq!(stage.id(), STAGE);
    let keyed = Rc::clone(controller);
    listen(&stage, "keydown", move |e: web_sys::KeyboardEvent| {
        let Some(key) = key_of(e.target()) else { return };
        if e.ctrl_key() || e.meta_key() || e.alt_key() {
            return;
        }
        if let Some(nav) = Nav::from_key(&e.key()) {
            e.prevent_default();
            keyed.act(Action::Navigate(nav));
        } else if e.key() == "Enter" || e.key() == " " {
            e.prevent_default();
            keyed.act(Action::Choose(key));
        }
    })?;
    let clicked = Rc::clone(controller);
    listen(&stage, "click", move |e: web_sys::MouseEvent| {
        if let Some(key) = key_of(e.target()) {
            clicked.act(Action::Choose(key));
        }
    })?;
    let (gained, own) = (Rc::clone(controller), Rc::clone(page));
    listen(&stage, "focusin", move |e: web_sys::FocusEvent| {
        if own.is_restoring() {
            return;
        }
        if let Some(key) = key_of(e.target()) {
            gained.act(Action::FocusMoved(key));
        }
    })
}
