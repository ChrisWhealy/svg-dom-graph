//! The phase shift panel in the built demo gallery: the circle with point C on it, the sine plot with two waves in it,
//! and what C does to the second wave.
//!
//! Native unit tests cover what is drawn for a shift, and that the second wave is the first with the shift added. What
//! they cannot reach is the browser. These tests check the following:
//!
//! - The circle is 400 by 400 pixels and the sine 430 by 400, each holds a plot, and there is no cosine and no button.
//! - The point starts at 45°, and stays there, since it is not animated.
//! - Dragging the point or pressing a key moves it, and the angle beside the arc and the second wave follow.
//! - The first wave never moves, and the second is a different colour.
//! - The grid that holds the scenes has no scroll bar when the window is wide enough for it.
//! - The shift is marked by a dotted line, except when there is none.
//! - An arc near the middle of the circle and the angle beside it follow C, the long way round beyond 180°.
//!
//! Like [`crate::sine_cosine`], these run against the built demo gallery.

use crate::common::{demo_tab::DemoTab, drag, new_demo_tab};
use headless_chrome::Tab;
use std::time::Duration;

const CIRCLE: &str = "#phase-shift-circle-stage-svg";
const SINE: &str = "#phase-shift-sine-stage-svg";
const HANDLE: &str = "#phase-shift-circle-stage .circle-handle";
const FIRST_WAVE: &str = "[stroke=\"#2563eb\"]";
/// The dotted line that marks the shift is the colour of the second wave too, so it is left out.
const SECOND_WAVE: &str = "[stroke=\"#c2410c\"]:not([stroke-dasharray])";
const VIEWPORT: (u32, u32) = (1800, 1200);

fn open() -> Result<DemoTab, String> {
    new_demo_tab("index.html#panel-phase-shift", &format!("{SINE} g[role=\"img\"]"), VIEWPORT)
}

fn check(condition: bool, message: &str) -> Result<(), String> {
    if condition { Ok(()) } else { Err(message.to_owned()) }
}

fn check_eq<T: PartialEq + std::fmt::Debug>(got: T, expected: T) -> Result<(), String> {
    check(got == expected, &format!("expected {expected:?}, got {got:?}"))
}

/// Evaluates `expression` in the page and returns its string value.
fn text(tab: &Tab, expression: &str) -> Result<String, String> {
    tab.evaluate(expression, false)
        .map_err(|e| format!("could not evaluate {expression}: {e}"))?
        .value
        .and_then(|v| v.as_str().map(str::to_owned))
        .ok_or_else(|| format!("{expression} did not return a string"))
}

/// The `points` of the one line in the sine plot that matches `selector`.
fn wave(tab: &Tab, selector: &str) -> Result<String, String> {
    text(
        tab,
        &format!(
            "[...document.querySelectorAll('{SINE} polyline{selector}')].map(l => l.getAttribute('points')).join('|')"
        ),
    )
}

fn count(tab: &Tab, selector: &str) -> Result<usize, String> {
    text(tab, &format!("String(document.querySelectorAll('{selector}').length)"))?
        .parse()
        .map_err(|e| format!("{e}"))
}

/// The phase shift that the text beside the arc gives.
fn shown(tab: &Tab) -> Result<f64, String> {
    let line = angle_text(tab)?;
    line.strip_prefix("\u{3c6}=")
        .and_then(|rest| rest.split('\u{b0}').next())
        .and_then(|number| number.parse().ok())
        .ok_or_else(|| format!("no shift in {line:?}"))
}

fn circle_corner(tab: &Tab) -> Result<(f64, f64), String> {
    let corner = text(
        tab,
        &format!(
            "(() => {{ const r = document.querySelector('{CIRCLE}').getBoundingClientRect(); \
             return r.left + ',' + r.top; }})()"
        ),
    )?;
    let (left, top) = corner.split_once(',').ok_or("the corner is two numbers")?;
    Ok((
        left.parse().map_err(|e| format!("{e}"))?,
        top.parse().map_err(|e| format!("{e}"))?,
    ))
}

/// Where the point is on the page at `degrees`: the circle has a radius of 1 and the scene shows 2.4 across.
fn point_at(tab: &Tab, degrees: f64) -> Result<(f64, f64), String> {
    let (left, top) = circle_corner(tab)?;
    let radius = 400.0 / 2.4;
    let angle = degrees.to_radians();
    Ok((left + 200.0 + radius * angle.cos(), top + 200.0 - radius * angle.sin()))
}

/// Drags the point from where it is to `degrees`, by way of the angles between.
fn drag_to(tab: &Tab, degrees: u32) -> Result<(), String> {
    let from = shown(tab)?;
    let to = f64::from(degrees);
    let steps = 8;
    let waypoints = (0..=steps)
        .map(|n| point_at(tab, from + (to - from) * f64::from(n) / f64::from(steps)))
        .collect::<Result<Vec<_>, _>>()?;
    drag(tab, &waypoints)
}

fn handle_attribute(tab: &Tab, name: &str) -> Result<String, String> {
    text(tab, &format!("document.querySelector('{HANDLE}').getAttribute('{name}')"))
}

fn press(tab: &Tab, key: &str) -> Result<(), String> {
    tab.press_key(key).map_err(|e| format!("could not press {key}: {e}"))?;
    Ok(())
}

fn focus_handle(tab: &Tab) -> Result<(), String> {
    text(tab, &format!("document.querySelector('{HANDLE}').focus(), 'ok'")).map(|_| ())
}

/// The text of everything written in the sine plot, joined with `|`.
fn sine_labels(tab: &Tab) -> Result<String, String> {
    text(
        tab,
        &format!("[...document.querySelectorAll('{SINE} text')].map(t => t.textContent).join('|')"),
    )
}

/// The text of the angle in the circle, which starts with the letter φ.
fn angle_text(tab: &Tab) -> Result<String, String> {
    text(
        tab,
        &format!(
            "[...document.querySelectorAll('{CIRCLE} text')].map(t => t.textContent) \
             .find(t => t.startsWith('\u{3c6}')) || ''"
        ),
    )
}

/// How many points the arc has: the line in the colour of the second wave that is in the circle.
fn arc_points(tab: &Tab) -> Result<usize, String> {
    let points = text(
        tab,
        &format!(
            "(() => {{ const l = document.querySelector('{CIRCLE} polyline[stroke=\"#c2410c\"]'); \
             return l ? String(l.getAttribute('points').trim().split(/\\s+/).length) : '0'; }})()"
        ),
    )?;
    points.parse().map_err(|e| format!("{e}"))
}

mod unit_tests;
