//! Panning and node dragging share one pointer, and must never be confused for each other.
//!
//! `wasm-bindgen-test`'s synthetic events are dispatched straight at a chosen element, so they cannot prove what
//! matters here. Once the browser has captured the pointer for one gesture, every further event belongs to that gesture
//! *wherever the pointer then travels*. Only real, CDP-driven input goes through the browser's own hit-testing and
//! pointer capture.
//!
//! The fixture has panning switched on with no toolbar. Its background is clear of every node and connector at `(450,
//! 200)` and `(150, 90)`. `solo` is the node at `(20, 20)`, `80 x 40`.

use crate::common::{ctrl_wheel, group_translate, mouse_event, new_tab};
use headless_chrome::{Tab, protocol::cdp::Input};

const PRESS: Input::DispatchMouseEventTypeOption = Input::DispatchMouseEventTypeOption::MousePressed;
const MOVE: Input::DispatchMouseEventTypeOption = Input::DispatchMouseEventTypeOption::MouseMoved;
const RELEASE: Input::DispatchMouseEventTypeOption = Input::DispatchMouseEventTypeOption::MouseReleased;

const SURFACE: &str = "#diagram > rect";
const SOLO: &str = "#diagram > g.svg-dom-graph-content > g:nth-of-type(1)";

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
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

/// Where the `<svg>`'s top-left corner is on the page, in the same pixels as the mouse coordinates. The page has a body
/// margin, so this is not `(0, 0)`, and a zoom's pivot — which is a point of the `<svg>` — depends on it.
fn svg_origin(tab: &Tab) -> Result<(f64, f64), String> {
    let read = |property: &str| -> Result<f64, String> {
        let script = format!("document.querySelector('#diagram').getBoundingClientRect().{property}");
        tab.evaluate(&script, false)
            .map_err(|e| format!("could not read the <svg>'s position: {e}"))?
            .value
            .and_then(|v| v.as_f64())
            .ok_or_else(|| format!("the <svg>'s {property} was not a number"))
    };
    Ok((read("left")?, read("top")?))
}

fn close(got: f64, expected: f64) -> bool {
    (got - expected).abs() <= 1.0
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
fn check(condition: bool, msg: &str) -> Result<(), String> {
    if condition { Ok(()) } else { Err(msg.into()) }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The `role` of the element that currently has keyboard focus, or an empty string if nothing does.
///
/// The scene's focus target is an element of its own with no id, so it is recognised by its role. The application's own
/// `<svg>`, which has an id but never a role from the scene, is what must *not* be the one focused.
fn focused_role(tab: &Tab) -> Result<String, String> {
    let result = tab
        .evaluate(
            "document.activeElement && document.activeElement.getAttribute('role') || ''",
            false,
        )
        .map_err(|e| format!("could not read the focused element: {e}"))?;
    Ok(result.value.and_then(|v| v.as_str().map(str::to_owned)).unwrap_or_default())
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Zooming with the real wheel in the middle of a real node drag. The pointer holds `solo` at (60, 40), 40 in from its
/// corner. A wheel at the pointer holds that point still, so the second move of 25 pixels at 1.25x is 20 units of
/// content. Using the matrix from the start of the drag would move it 25.
fn a_node_drag_carries_on_correctly_after_a_real_wheel_zoom() -> Result<(), String> {
    let tab = new_tab()?;
    let solo = tab.find_element(SOLO).map_err(|e| format!("could not find solo: {e}"))?;
    let (before_x, before_y) = group_translate(&solo)?;

    mouse_event(&tab, MOVE, (60.0, 40.0), None)?;
    mouse_event(&tab, PRESS, (60.0, 40.0), Some(1))?;
    mouse_event(&tab, MOVE, (85.0, 40.0), Some(1))?;
    ctrl_wheel(&tab, (85.0, 40.0), -100.0)?;
    mouse_event(&tab, MOVE, (110.0, 40.0), Some(1))?;
    mouse_event(&tab, RELEASE, (110.0, 40.0), Some(0))?;

    let solo = tab.find_element(SOLO).map_err(|e| format!("could not re-find solo: {e}"))?;
    let (after_x, after_y) = group_translate(&solo)?;
    // 25 units at 1.0x, then 25 pixels at 1.25x, which is 20 units.
    check(
        close(after_x, before_x + 25.0 + 20.0) && close(after_y, before_y),
        &format!("expected solo at ({}, {before_y}), got ({after_x}, {after_y})", before_x + 45.0),
    )?;
    check(
        !has_capture(&tab, SOLO)?,
        "the node still holds pointer capture after the button came up",
    )
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The same for a pan: pan 30 pixels, zoom with the real wheel at the pointer, pan 30 more. The zoom is kept.
fn a_pan_carries_on_correctly_after_a_real_wheel_zoom() -> Result<(), String> {
    let tab = new_tab()?;

    mouse_event(&tab, MOVE, (450.0, 200.0), None)?;
    mouse_event(&tab, PRESS, (450.0, 200.0), Some(1))?;
    mouse_event(&tab, MOVE, (420.0, 200.0), Some(1))?;
    ctrl_wheel(&tab, (420.0, 200.0), -100.0)?;
    mouse_event(&tab, MOVE, (390.0, 190.0), Some(1))?;
    mouse_event(&tab, RELEASE, (390.0, 190.0), Some(0))?;

    // Pan to (-30, 0), zoom 1.25 about the pointer: t' = p - 1.25 * (p - t), then pan by (-30, -10). The pointer is at
    // (420, 200) on the page, which is `p` relative to the `<svg>`.
    let (left, top) = svg_origin(&tab)?;
    let (px, py) = (420.0 - left, 200.0 - top);
    let expected = (px - 1.25 * (px + 30.0) - 30.0, py - 1.25 * py - 10.0);
    let (x, y) = content_translate(&tab)?.ok_or("the scene did not pan")?;
    check(
        close(x, expected.0) && close(y, expected.1),
        &format!("expected a view at ({}, {}), got ({x}, {y})", expected.0, expected.1),
    )
}

mod unit_tests;
