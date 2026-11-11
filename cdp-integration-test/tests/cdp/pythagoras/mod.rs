//! The Pythagoras panel in the built demo gallery: the unit circle with a right triangle in it and a square on each of
//! its sides, which are drawn again as point C is dragged round.
//!
//! Native unit tests cover the triangle and the squares for every angle. What they cannot reach is the browser. These
//! tests check the following:
//!
//! - The scene is 560 pixels square, with no plot, axes or grid in it, and no slider or readout outside it.
//! - C starts at 45°, and the squares are the colours that were asked for: a blue one on the hypotenuse, and two green
//!   ones, each that lets what is under it be seen.
//! - Dragging C, or pressing a key, draws the squares again, and the two green ones add up to the blue one.
//! - The area of each square is written in it, and the sum is written at the top.
//!
//! Like [`crate::phase_shift`], these run against the built demo gallery.

use crate::common::{demo_tab::DemoTab, drag, new_demo_tab};
use headless_chrome::Tab;

const SVG: &str = "#pythagoras-stage-svg";
const HANDLE: &str = "#pythagoras-stage .circle-handle";
const VIEWPORT: (u32, u32) = (1800, 1200);

/// Where the middle of the circle is in the scene, and its radius, in pixels.
const MIDDLE: (f64, f64) = (280.0, 310.0);
const RADIUS: f64 = 140.0;

fn open() -> Result<DemoTab, String> {
    new_demo_tab(
        "index.html#panel-pythagoras",
        &format!("{SVG} polygon.square-hypotenuse"),
        VIEWPORT,
    )
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

fn count(tab: &Tab, selector: &str) -> Result<usize, String> {
    text(tab, &format!("String(document.querySelectorAll('{selector}').length)"))?
        .parse()
        .map_err(|e| format!("{e}"))
}

fn attribute(tab: &Tab, selector: &str, name: &str) -> Result<String, String> {
    text(
        tab,
        &format!("document.querySelector('{SVG} {selector}').getAttribute('{name}')"),
    )
}

fn written(tab: &Tab, selector: &str) -> Result<String, String> {
    text(tab, &format!("document.querySelector('{SVG} {selector}').textContent"))
}

/// The corners of a polygon, as numbers.
fn corners(tab: &Tab, selector: &str) -> Result<Vec<(f64, f64)>, String> {
    attribute(tab, selector, "points")?
        .split_whitespace()
        .map(|pair| {
            let (x, y) = pair.split_once(',').ok_or_else(|| format!("no comma in {pair:?}"))?;
            Ok((x.parse().map_err(|e| format!("{e}"))?, y.parse().map_err(|e| format!("{e}"))?))
        })
        .collect()
}

/// The area of the square with these corners, in pixels.
fn area(square: &[(f64, f64)]) -> f64 {
    let side = (square[1].0 - square[0].0, square[1].1 - square[0].1);
    side.0 * side.0 + side.1 * side.1
}

/// The areas of the squares on the width, the height and the hypotenuse, in units of the circle.
fn areas(tab: &Tab) -> Result<(f64, f64, f64), String> {
    let unit = RADIUS * RADIUS;
    Ok((
        area(&corners(tab, "polygon.square-width")?) / unit,
        area(&corners(tab, "polygon.square-height")?) / unit,
        area(&corners(tab, "polygon.square-hypotenuse")?) / unit,
    ))
}

fn handle(tab: &Tab, name: &str) -> Result<String, String> {
    text(tab, &format!("document.querySelector('{HANDLE}').getAttribute('{name}')"))
}

fn focus_handle(tab: &Tab) -> Result<(), String> {
    text(tab, &format!("document.querySelector('{HANDLE}').focus(), 'ok'")).map(|_| ())
}

fn press(tab: &Tab, key: &str) -> Result<(), String> {
    tab.press_key(key).map_err(|e| format!("could not press {key}: {e}"))?;
    Ok(())
}

/// Where C is on the page at `degrees`, to the nearest whole pixel.
///
/// The scene is now beside a column of text, so its corner is at a fractional position that depends on the fonts of the
/// machine. A mouse position that is then cut down to whole pixels, rather than rounded, always moves C a little the
/// same way round the circle, which can be the whole degree that these tests read back. Rounding here makes the error
/// at most half a pixel each way, which is under a third of a degree.
fn point_at(tab: &Tab, degrees: f64) -> Result<(f64, f64), String> {
    let corner = text(
        tab,
        &format!(
            "(() => {{ const r = document.querySelector('{SVG}').getBoundingClientRect(); \
             return r.left + ',' + r.top; }})()"
        ),
    )?;
    let (left, top) = corner.split_once(',').ok_or("the corner is two numbers")?;
    let (left, top): (f64, f64) = (
        left.parse().map_err(|e| format!("{e}"))?,
        top.parse().map_err(|e| format!("{e}"))?,
    );
    let angle = degrees.to_radians();
    Ok((
        (left + MIDDLE.0 + RADIUS * angle.cos()).round(),
        (top + MIDDLE.1 - RADIUS * angle.sin()).round(),
    ))
}

/// Drags C from where it is to `degrees`, by way of the angles between.
fn drag_to(tab: &Tab, degrees: u32) -> Result<(), String> {
    let from: f64 = handle(tab, "aria-valuenow")?.parse().map_err(|e| format!("{e}"))?;
    let to = f64::from(degrees);
    let steps = 8;
    let waypoints = (0..=steps)
        .map(|n| point_at(tab, from + (to - from) * f64::from(n) / f64::from(steps)))
        .collect::<Result<Vec<_>, _>>()?;
    drag(tab, &waypoints)
}

mod unit_tests;
