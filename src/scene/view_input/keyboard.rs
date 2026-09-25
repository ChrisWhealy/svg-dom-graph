//! Keyboard control of the view, for someone who cannot use a mouse or trackpad.
//!
//! Zooming is reachable from the keyboard through the toolbar, but zooming in can push content out of view. Without a
//! keyboard way to move the view, that content would be somewhere the same person can no longer reach. So when panning
//! is active the `<svg>` itself becomes a keyboard focus target. Its arrow keys pan the view, and its plus, minus and
//! zero keys zoom when wheel zoom is active.
//!
//! The `<svg>` is given `role="application"`, since it handles its own keys. Screen readers that would otherwise keep
//! arrow keys for reading then pass them through to it.
//!
//! Only a key pressed while the `<svg>` itself has focus is handled. A key pressed on a toolbar button, which sits inside
//! the `<svg>` and so sees the same event bubble up, is left to the button.

use super::super::Scene;
use crate::{error::Error, scene::SceneInner};
use std::{cell::RefCell, rc::Weak};
use svg_dom::SvgNode;
use web_sys::KeyboardEvent;

/// How far one arrow key press moves the view, in the `<svg>`'s own user space. That is the same distance on screen at
/// any zoom, so panning feels the same whether zoomed in or out.
const PAN_STEP: f64 = 40.0;

/// How many times further a press moves the view while Shift is held.
const SHIFT_FACTOR: f64 = 5.0;

/// The attributes this module sets on the `<svg>`, and which [`remove`] takes away again.
const ATTRIBUTES: [&str; 4] = ["role", "tabindex", "aria-label", "aria-description"];

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
    let _ = write!(out, "Graph view, zoom {}%", (scale * 100.0).round());
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
/// Makes `root`, the `<svg>` itself, a keyboard target and wires its keys.
///
/// `pan` enables the arrow keys and `zoom` enables plus, minus and zero. `scale` is the current zoom, for the name.
///
/// The listener holds only a `Weak` reference to the scene's shared state, for the same reason
/// [`Scene::make_draggable_with`](crate::scene::Scene::make_draggable_with)'s do.
pub(super) fn install(
    root: &SvgNode,
    inner: &Weak<RefCell<SceneInner>>,
    pan: bool,
    zoom: bool,
    scale: f64,
) -> Result<(), Error> {
    root.set_attr("role", "application")?;
    root.set_attr("tabindex", "0")?;
    root.set_attr("aria-label", &label(scale))?;
    root.set_attr("aria-description", &description(pan, zoom))?;

    let target = root.downgrade();
    let inner = inner.clone();
    root.on_keydown(move |event| on_key(&target, &inner, pan, zoom, &event))?;
    Ok(())
}

/// Takes the keyboard handling off `root`: its listener, and the attributes [`install`] set.
pub(super) fn remove(root: &SvgNode) {
    root.remove_listeners("keydown");
    for name in ATTRIBUTES {
        let _ = root.remove_attr(name);
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
fn on_key(
    root: &svg_dom::WeakSvgNode,
    inner: &Weak<RefCell<SceneInner>>,
    pan: bool,
    zoom: bool,
    event: &KeyboardEvent,
) {
    // Only when the scene itself has focus: a key pressed on a toolbar button bubbles up to here too.
    let Some(root) = root.upgrade() else { return };
    if event.target().as_ref() != Some(root.as_element().as_ref()) {
        return;
    }
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
