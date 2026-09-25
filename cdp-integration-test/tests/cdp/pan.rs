//! Panning and node dragging share one pointer, and must never be confused for each other.
//!
//! `wasm-bindgen-test`'s synthetic events are dispatched straight at a chosen element, so they cannot prove what
//! matters here: that once the browser has captured the pointer for one gesture, every further event belongs to that
//! gesture *wherever the pointer then travels*. Only real, CDP-driven input goes through the browser's own hit-testing
//! and pointer capture.
//!
//! The fixture has panning switched on with no toolbar. Its background is clear of every node and connector at
//! `(450, 200)` and `(150, 90)`. `solo` is the node at `(20, 20)`, `80 x 40`.

use crate::common::{group_translate, mouse_event, new_tab};
use headless_chrome::{Tab, protocol::cdp::Input};

const PRESS: Input::DispatchMouseEventTypeOption = Input::DispatchMouseEventTypeOption::MousePressed;
const MOVE: Input::DispatchMouseEventTypeOption = Input::DispatchMouseEventTypeOption::MouseMoved;
const RELEASE: Input::DispatchMouseEventTypeOption = Input::DispatchMouseEventTypeOption::MouseReleased;

const SURFACE: &str = "#diagram > rect";
const SOLO: &str = "#diagram > g.svg-dom-graph-content > g:nth-of-type(1)";

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The content layer's `transform`, as `(tx, ty)`, or `None` if it has never been panned or zoomed.
fn content_translate(tab: &Tab) -> Result<Option<(f64, f64)>, String> {
    let content = tab
        .find_element("#diagram > g.svg-dom-graph-content")
        .map_err(|e| format!("could not find the content layer: {e}"))?;
    let Some(value) = content.get_attribute_value("transform").map_err(|e| format!("{e}"))? else {
        return Ok(None);
    };
    let inner = value
        .strip_prefix("translate(")
        .and_then(|s| s.split(')').next())
        .ok_or_else(|| format!("transform {value:?} is not a translate(...) expression"))?;
    let mut parts = inner.split(',').map(|part| part.trim().parse::<f64>());
    match (parts.next(), parts.next()) {
        (Some(Ok(x)), Some(Ok(y))) => Ok(Some((x, y))),
        _ => Err(format!("transform {value:?} did not parse")),
    }
}

/// Whether `selector`'s element currently holds pointer capture for the mouse pointer (id 1).
fn has_capture(tab: &Tab, selector: &str) -> Result<bool, String> {
    let script = format!("document.querySelector('{selector}').hasPointerCapture(1)");
    let result = tab
        .evaluate(&script, false)
        .map_err(|e| format!("could not query pointer capture: {e}"))?;
    result
        .value
        .and_then(|v| v.as_bool())
        .ok_or_else(|| format!("hasPointerCapture for {selector} did not return a boolean"))
}

fn close(got: f64, expected: f64) -> bool {
    (got - expected).abs() <= 1.0
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Dragging empty background pans the whole scene by the pointer's own movement. The surface holds pointer capture for
/// as long as the button is down, and lets go the moment it comes up.
#[test]
fn dragging_empty_background_pans_the_scene_and_holds_then_releases_pointer_capture() -> Result<(), String> {
    let tab = new_tab()?;
    check(
        content_translate(&tab)?.is_none(),
        "the scene was already panned before any drag",
    )?;

    mouse_event(&tab, MOVE, (450.0, 200.0), None)?;
    mouse_event(&tab, PRESS, (450.0, 200.0), Some(1))?;
    mouse_event(&tab, MOVE, (420.0, 190.0), Some(1))?;
    mouse_event(&tab, MOVE, (400.0, 180.0), Some(1))?;

    check(has_capture(&tab, SURFACE)?, "the surface does not hold pointer capture mid-pan")?;

    mouse_event(&tab, RELEASE, (400.0, 180.0), Some(0))?;

    check(
        !has_capture(&tab, SURFACE)?,
        "the surface still holds pointer capture after the button came up",
    )?;
    let (x, y) = content_translate(&tab)?.ok_or("the scene did not pan")?;
    check(
        close(x, -50.0) && close(y, -20.0),
        &format!("expected a pan of (-50, -20), got ({x}, {y})"),
    )
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// A pan that starts on background and then sweeps across a node is still a pan: the node neither moves nor gets the
/// events, because the surface has captured the pointer.
#[test]
fn a_pan_that_crosses_over_a_node_stays_a_pan() -> Result<(), String> {
    let tab = new_tab()?;
    let solo = tab.find_element(SOLO).map_err(|e| format!("could not find solo: {e}"))?;
    let (solo_x, solo_y) = group_translate(&solo)?;

    // Starts on clear background, sweeps left across `solo` at (20..100, 20..60), and finishes on background below it.
    mouse_event(&tab, MOVE, (150.0, 90.0), None)?;
    mouse_event(&tab, PRESS, (150.0, 90.0), Some(1))?;
    mouse_event(&tab, MOVE, (100.0, 60.0), Some(1))?;
    mouse_event(&tab, MOVE, (60.0, 40.0), Some(1))?; // directly over solo
    mouse_event(&tab, MOVE, (30.0, 100.0), Some(1))?;
    check(
        has_capture(&tab, SURFACE)?,
        "the surface lost pointer capture while over a node",
    )?;
    mouse_event(&tab, RELEASE, (30.0, 100.0), Some(0))?;

    let (x, y) = content_translate(&tab)?.ok_or("the scene did not pan")?;
    check(
        close(x, -120.0) && close(y, 10.0),
        &format!("expected a pan of (-120, 10), got ({x}, {y})"),
    )?;

    // The node's own transform is untouched: it moved only because its parent layer did.
    let solo = tab.find_element(SOLO).map_err(|e| format!("could not re-find solo: {e}"))?;
    let (after_x, after_y) = group_translate(&solo)?;
    check(
        close(after_x, solo_x) && close(after_y, solo_y),
        &format!("solo's own position changed from ({solo_x}, {solo_y}) to ({after_x}, {after_y})"),
    )
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// A node drag that starts on a node and then travels across empty background is still a node drag: the scene never
/// pans. The node holds pointer capture during the drag, and the surface never does.
#[test]
fn a_node_drag_that_crosses_over_background_stays_a_node_drag() -> Result<(), String> {
    let tab = new_tab()?;
    let solo = tab.find_element(SOLO).map_err(|e| format!("could not find solo: {e}"))?;
    let (before_x, before_y) = group_translate(&solo)?;

    // Presses on `solo`'s centre, then travels well beyond its own edges, over background.
    mouse_event(&tab, MOVE, (60.0, 40.0), None)?;
    mouse_event(&tab, PRESS, (60.0, 40.0), Some(1))?;
    mouse_event(&tab, MOVE, (120.0, 60.0), Some(1))?;
    mouse_event(&tab, MOVE, (180.0, 80.0), Some(1))?;
    check(has_capture(&tab, SOLO)?, "the node does not hold pointer capture mid-drag")?;
    check(
        !has_capture(&tab, SURFACE)?,
        "the pan surface captured the pointer during a node drag",
    )?;
    mouse_event(&tab, RELEASE, (180.0, 80.0), Some(0))?;

    check(
        !has_capture(&tab, SOLO)?,
        "the node still holds pointer capture after the button came up",
    )?;
    check(content_translate(&tab)?.is_none(), "a node drag panned the scene")?;

    let solo = tab.find_element(SOLO).map_err(|e| format!("could not re-find solo: {e}"))?;
    let (after_x, after_y) = group_translate(&solo)?;
    check(
        close(after_x, before_x + 120.0) && close(after_y, before_y + 40.0),
        &format!(
            "expected solo at ({}, {}), got ({after_x}, {after_y})",
            before_x + 120.0,
            before_y + 40.0
        ),
    )
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Pressing on a node never starts a pan, even though the pan surface lies behind it.
#[test]
fn pressing_on_a_node_never_starts_a_pan() -> Result<(), String> {
    let tab = new_tab()?;

    mouse_event(&tab, MOVE, (60.0, 40.0), None)?;
    mouse_event(&tab, PRESS, (60.0, 40.0), Some(1))?;
    check(
        !has_capture(&tab, SURFACE)?,
        "the pan surface captured a press that landed on a node",
    )?;
    mouse_event(&tab, RELEASE, (60.0, 40.0), Some(0))?;
    check(content_translate(&tab)?.is_none(), "a press on a node panned the scene")
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
fn check(condition: bool, msg: &str) -> Result<(), String> {
    if condition { Ok(()) } else { Err(msg.into()) }
}
