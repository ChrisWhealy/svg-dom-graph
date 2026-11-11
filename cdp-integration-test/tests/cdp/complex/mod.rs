//! The complex numbers panel in the built demo gallery: the Argand diagrams are plot nodes of the library, and a real
//! click on the one beside the sliders puts the number where it was clicked.
//!
//! The native tests of the demo cover what is drawn on the diagram, and the library's own browser tests cover how a
//! plot node turns a click into values. What neither reaches is the whole path with a real mouse: the browser's hit
//! test, a click on a plot node, the demo moving its sliders, and the diagram and the seven nodes following.
//!
//! Like [`crate::dijkstra`], these run against the built demo gallery.

use crate::common::{demo_tab::DemoTab, mouse_event, new_demo_tab};
use headless_chrome::{Tab, protocol::cdp::Input};
use std::time::{Duration, Instant};

const DIAGRAM: &str = "#complex-diagram";
const ROOTS: &str = "#complex-roots-diagram";
const VIEWPORT: (u32, u32) = (1900, 2900);

fn open() -> Result<DemoTab, String> {
    new_demo_tab("index.html#panel-complex", &format!("{ROOTS} g[role=\"img\"]"), VIEWPORT)
}

fn check(condition: bool, message: &str) -> Result<(), String> {
    if condition { Ok(()) } else { Err(message.to_owned()) }
}

/// Evaluates `expression` in the page and returns its string value.
fn text(tab: &Tab, expression: &str) -> Result<String, String> {
    tab.evaluate(expression, false)
        .map_err(|e| format!("could not evaluate {expression}: {e}"))?
        .value
        .and_then(|v| v.as_str().map(str::to_owned))
        .ok_or_else(|| format!("{expression} did not return a string"))
}

fn number(tab: &Tab, expression: &str) -> Result<f64, String> {
    text(tab, &format!("String({expression})"))?
        .parse()
        .map_err(|e| format!("{expression} is not a number: {e}"))
}

/// Waits until `condition` holds, or fails after five seconds.
fn wait_for(what: &str, mut condition: impl FnMut() -> Result<bool, String>) -> Result<(), String> {
    let deadline = Instant::now() + Duration::from_secs(5);
    while Instant::now() < deadline {
        if condition()? {
            return Ok(());
        }
        std::thread::sleep(Duration::from_millis(20));
    }
    Err(format!("timed out waiting for {what}"))
}

/// The accessible name of the diagram beside the sliders.
fn name(tab: &Tab) -> Result<String, String> {
    text(
        tab,
        &format!("document.querySelector('{DIAGRAM} g[role=\"img\"]').getAttribute('aria-label')"),
    )
}

/// Where the point `(x, y)` of the plane is on the page, read from the plotted area as the browser draws it. The area
/// is the second rectangle of the plot node, and the plane shows `-5.5` to `5.5` along each axis.
fn page_point(tab: &Tab, x: f64, y: f64) -> Result<(f64, f64), String> {
    let at: [f64; 2] = serde_json::from_str(&text(
        tab,
        &format!(
            "(() => {{ const r = document.querySelectorAll('{DIAGRAM} g[role=\"img\"] > rect')[1].getBoundingClientRect(); \
             return JSON.stringify([r.left + (({x} + 5.5) / 11) * r.width, r.top + ((5.5 - ({y})) / 11) * r.height]); }})()"
        ),
    )?)
    .map_err(|e| format!("could not read where the point is: {e}"))?;
    Ok((at[0], at[1]))
}

/// Clicks at `point` with a real mouse.
fn click(tab: &Tab, point: (f64, f64)) -> Result<(), String> {
    mouse_event(tab, Input::DispatchMouseEventTypeOption::MouseMoved, point, None)?;
    mouse_event(tab, Input::DispatchMouseEventTypeOption::MousePressed, point, Some(1))?;
    mouse_event(tab, Input::DispatchMouseEventTypeOption::MouseReleased, point, Some(0))
}

mod unit_tests;
