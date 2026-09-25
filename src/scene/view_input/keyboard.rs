//! Keyboard control of the view, for someone who cannot use a mouse or trackpad.
//!
//! Zooming is reachable from the keyboard through the toolbar, but zooming in can push content out of view. Without a
//! keyboard way to move the view, that content would be somewhere the same person can no longer reach. So when panning
//! or wheel zoom is active the scene adds a keyboard focus target. Its arrow keys pan the view, and its plus, minus and
//! zero keys zoom when wheel zoom is active.
//!
//! # A dedicated element, not the application's `<svg>`
//!
//! The focus target is a transparent `<rect>` that the scene creates and removes, and that is the only thing given a role,
//! a name, a description, or a `tabindex`. The application's own `<svg>` is never touched. So whatever role, name, or
//! `tabindex` the application gave it — often a description of the whole graph — is left exactly as it was, and is there
//! again when the keyboard control goes.
//!
//! It also keeps the `application` role, which asks a screen reader to pass keys through instead of keeping them for
//! reading, confined to one small control. The nodes inside the `<svg>` keep their ordinary accessible descriptions and
//! are read as normal.
//!
//! The target has `pointer-events="none"`, so it never gets in the way of the pan surface beneath it. Nothing is drawn
//! for it until it has keyboard focus. Then it outlines the visible area, so it is obvious where focus is.

use super::super::Scene;
use crate::colours::FOCUS_RING;
use crate::{error::Error, scene::SceneInner};
use std::{cell::RefCell, rc::Weak};
use svg_dom::SvgNode;
use web_sys::KeyboardEvent;

/// How far one arrow key press moves the view, in the `<svg>`'s own user space. That is the same distance on screen at
/// any zoom, so panning feels the same whether zoomed in or out.
const PAN_STEP: f64 = 40.0;

/// How many times further a press moves the view while Shift is held.
const SHIFT_FACTOR: f64 = 5.0;

/// The width of the outline drawn around the visible area while the target has focus, in the `<svg>`'s own units. Half
/// of it falls outside the visible area and is clipped, so this is twice the width that shows.
const FOCUS_OUTLINE_WIDTH: f64 = 6.0;

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The accessible name of the scene while it is a keyboard target: what it is, and how far it is zoomed.
///
/// The zoom is part of the *name*, not a live region, so changing it never interrupts a screen reader that is reading
/// something else. It is read the next time the scene takes focus.
///
/// Writes into a caller-owned buffer, replacing its content, so a run of zoom updates reuses one allocation instead of
/// building a new `String` for every animation frame. See [`label`] for a version that returns one.
pub(super) fn write_label(scale: f64, out: &mut String) {
    use std::fmt::Write as _;
    out.clear();
    let _ = write!(out, "Graph view, zoom {}%", percent(scale));
}

/// The zoom as the whole percentage the label shows.
///
/// A pan never changes it, and a zoom usually does. Remembering the last one lets a frame that has not changed it skip
/// building and writing the label altogether.
pub(super) fn percent(scale: f64) -> i64 {
    (scale * 100.0).round() as i64
}

/// [`write_label`] into a new `String`, for the one-off case where nothing is being reused.
pub(super) fn label(scale: f64) -> String {
    let mut out = String::new();
    write_label(scale, &mut out);
    out
}

/// What the keys do, for the scene's accessible description.
fn description(pan: bool, zoom: bool) -> String {
    let mut parts = Vec::new();
    if pan {
        parts.push("Arrow keys pan the view. Hold Shift to pan further.");
    }
    if zoom {
        parts.push("Plus and minus zoom. Zero restores the original zoom.");
    }
    parts.join(" ")
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Makes `target`, a `<rect>` the scene has created for the purpose, a keyboard target and wires its keys.
///
/// `pan` enables the arrow keys and `zoom` enables plus, minus and zero. `scale` is the current zoom, for the name.
///
/// The listeners hold only a `Weak` reference to the scene's shared state, for the same reason
/// [`Scene::make_draggable_with`](crate::scene::Scene::make_draggable_with)'s do. They go when `target` does.
pub(super) fn install(
    target: &SvgNode,
    inner: &Weak<RefCell<SceneInner>>,
    pan: bool,
    zoom: bool,
    scale: f64,
) -> Result<(), Error> {
    // Nothing is drawn until it has focus, and it never takes a pointer event from the surface beneath it.
    target.set_fill("none")?;
    target.set_stroke("none")?;
    target.set_attr("pointer-events", "none")?;

    target.set_attr("role", "application")?;
    target.set_attr("tabindex", "0")?;
    target.set_attr("aria-label", &label(scale))?;
    target.set_attr("aria-description", &description(pan, zoom))?;

    // Keyboard focus must be obvious, so outline the visible area while it has it.
    let focused = target.downgrade();
    target.on_focus(move |_| {
        if let Some(target) = focused.upgrade() {
            let _ = target.set_stroke(FOCUS_RING);
            let _ = target.set_stroke_width(FOCUS_OUTLINE_WIDTH);
        }
    })?;
    let blurred = target.downgrade();
    target.on_blur(move |_| {
        if let Some(target) = blurred.upgrade() {
            let _ = target.set_stroke("none");
        }
    })?;

    let inner = inner.clone();
    target.on_keydown(move |event| on_key(&inner, pan, zoom, &event))?;
    Ok(())
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
fn on_key(inner: &Weak<RefCell<SceneInner>>, pan: bool, zoom: bool, event: &KeyboardEvent) {
    // Only this element receives the event, since it has no children and nothing else in the `<svg>` is its ancestor. A
    // key pressed on a toolbar button, for one, never reaches it.
    // Leave browser and assistive-technology shortcuts alone, such as Ctrl and Cmd plus plus or minus for page zoom.
    if event.ctrl_key() || event.meta_key() || event.alt_key() {
        return;
    }
    let Some(inner) = inner.upgrade() else { return };

    let step = if event.shift_key() { PAN_STEP * SHIFT_FACTOR } else { PAN_STEP };
    // An arrow key moves the *view* that way, like scrolling, so the content moves the opposite way.
    let moved = match event.key().as_str() {
        "ArrowLeft" if pan => Some((step, 0.0)),
        "ArrowRight" if pan => Some((-step, 0.0)),
        "ArrowUp" if pan => Some((0.0, step)),
        "ArrowDown" if pan => Some((0.0, -step)),
        _ => None,
    };

    let scene = Scene { inner };
    // A listener has nowhere to report an error to, and a failed DOM write leaves the previous view in place.
    let handled = if let Some((dx, dy)) = moved {
        let mut inner = scene.inner.borrow_mut();
        let view = inner.view.translated(dx, dy);
        let _ = inner.set_view(view);
        true
    } else if zoom {
        match event.key().as_str() {
            "+" | "=" => {
                let _ = scene.zoom_in();
                true
            },
            "-" | "_" => {
                let _ = scene.zoom_out();
                true
            },
            "0" => {
                let _ = scene.reset_view();
                true
            },
            _ => false,
        }
    } else {
        false
    };

    if handled {
        // Stops the arrow keys and Space-like keys scrolling the page underneath.
        event.prevent_default();
    }
}
