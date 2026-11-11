//! The sine and cosine panel in the built demo gallery: three scenes of the sizes that were asked for, a point that
//! goes round by itself, the button in the circle that stops it, and dragging the point round the circle.
//!
//! Native unit tests cover what is drawn for an angle, and that the plots share their scales. What they cannot reach is
//! the browser. These tests check the following:
//!
//! - The circle is 400 by 400 pixels, the sine 430 by 400 and the cosine 400 by 430, and each holds a plot.
//! - The labels of the circle stop at 1, the radius.
//! - Dragging the point puts it at an angle, and the numbers beside it follow. A drag that starts away from the point
//!   changes nothing.
//! - Point C is a slider in the tab order, over C itself, and the arrow, page, Home and End keys move it.
//! - The readout is three lines, which the page shows as three lines.
//! - The angle is never written as a negative zero, however C is taken across 0°.
//! - The point starts at 0° and goes round from there.
//! - The grid that holds the scenes has no scroll bar when the window is wide enough for it.
//! - The sine and the cosine each have a title in the plot.
//! - Each plot has its dotted line to an axis.
//! - The point goes round by itself, and the button in the circle stops it and starts it again.
//!
//! Like [`crate::fft`], these run against the built demo gallery.

use crate::common::{demo_tab::DemoTab, drag, new_demo_tab};
use headless_chrome::Tab;
use std::time::{Duration, Instant};

const CIRCLE: &str = "#sine-cosine-circle-stage-svg";
const SINE: &str = "#sine-cosine-sine-stage-svg";
const COSINE: &str = "#sine-cosine-cosine-stage-svg";
const VIEWPORT: (u32, u32) = (1800, 1800);

fn open() -> Result<DemoTab, String> {
    new_demo_tab("index.html#panel-sine-cosine", &format!("{COSINE} g[role=\"img\"]"), VIEWPORT)
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

fn readout(tab: &Tab) -> Result<String, String> {
    text(tab, "document.getElementById('sine-cosine-readout').textContent")
}

/// What the button in the circle says it will do next.
fn button(tab: &Tab) -> Result<String, String> {
    text(
        tab,
        "document.querySelector('#sine-cosine-circle-stage [data-action=\"play\"]').getAttribute('aria-label')",
    )
}

/// How many dotted lines `svg` has: the lines with a dash pattern, which the plot grid does not use.
fn dotted(tab: &Tab, svg: &str) -> Result<String, String> {
    text(
        tab,
        &format!("String(document.querySelectorAll('{svg} [stroke-dasharray=\"1 5\"]').length)"),
    )
}

/// Waits until `read` gives something other than `before`, for up to three seconds.
fn changes(tab: &Tab, before: &str, read: fn(&Tab) -> Result<String, String>) -> Result<bool, String> {
    let start = Instant::now();
    while start.elapsed() < Duration::from_secs(3) {
        if read(tab)? != before {
            return Ok(true);
        }
        std::thread::sleep(Duration::from_millis(50));
    }
    Ok(false)
}

fn press_play(tab: &Tab) -> Result<(), String> {
    text(
        tab,
        "document.querySelector('#sine-cosine-circle-stage [data-action=\"play\"]') \
         .dispatchEvent(new MouseEvent('click', { bubbles: true })), 'ok'",
    )
    .map(|_| ())
}

/// Where the circle is on the page: the left and top of its scene, in pixels.
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

/// The angle that the readout gives.
fn shown_angle(tab: &Tab) -> Result<f64, String> {
    let line = readout(tab)?;
    line.strip_prefix("Angle ")
        .and_then(|rest| rest.split('\u{b0}').next())
        .and_then(|number| number.parse().ok())
        .ok_or_else(|| format!("no angle in {line:?}"))
}

/// Stops the point, if it is going round, and drags it from where it is to `degrees`, by way of the angles between.
fn drag_to(tab: &Tab, degrees: u32) -> Result<(), String> {
    if button(tab)? == "Pause" {
        press_play(tab)?;
    }
    let from = shown_angle(tab)?;
    let to = f64::from(degrees);
    let steps = 8;
    let waypoints = (0..=steps)
        .map(|n| point_at(tab, from + (to - from) * f64::from(n) / f64::from(steps)))
        .collect::<Result<Vec<_>, _>>()?;
    drag(tab, &waypoints)
}

const HANDLE: &str = "#sine-cosine-circle-stage .circle-handle";

fn focus_handle(tab: &Tab) -> Result<(), String> {
    text(tab, &format!("document.querySelector('{HANDLE}').focus(), 'ok'")).map(|_| ())
}

fn handle_attribute(tab: &Tab, name: &str) -> Result<String, String> {
    text(tab, &format!("document.querySelector('{HANDLE}').getAttribute('{name}')"))
}

fn press(tab: &Tab, key: &str) -> Result<(), String> {
    tab.press_key(key).map_err(|e| format!("could not press {key}: {e}"))?;
    Ok(())
}

mod unit_tests;
