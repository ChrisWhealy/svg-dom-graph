//! The weighted springs panel in the built demo gallery: the Dijkstra graph as damped springs. A real mouse drags a
//! circle and its neighbours follow, the network settles, and at rest the page has no animation frame pending.
//!
//! The physics itself, the frame clock, the command bridge and the controller's decisions are covered by native unit
//! tests. What they cannot reach is the browser: that a real drag reaches the observer, that frames are really
//! requested while the graph moves and really stop when it rests, that the controls are unavailable when they must be,
//! and that a page that is hidden, suspended or left stops cleanly.
//!
//! A real mouse cannot be held by this suite's helper, which presses, moves and releases in one call. The tests that
//! need a circle held, or two held at once, dispatch pointer events in the page instead, as the library's own drag
//! listeners receive them. The panel publishes `data-physics` and `data-physics-pending` for exactly these tests to
//! read: the second is `1` only while the physics controller has an animation frame requested.
//!
//! Like [`crate::dijkstra`], these run against the built demo gallery.

use crate::common::{demo_tab::DemoTab, drag, new_demo_tab};
use headless_chrome::Tab;
use std::time::{Duration, Instant};

const VIEWPORT: (u32, u32) = (1800, 1800);
const PANEL: &str = "#panel-springs";
const VERTICES: [&str; 6] = ["A", "B", "C", "D", "E", "F"];

fn open() -> Result<DemoTab, String> {
    new_demo_tab(
        "index.html#panel-springs",
        "#springs-diagram g[aria-label^=\"Zoom in\"]",
        VIEWPORT,
    )
}

fn check(condition: bool, message: &str) -> Result<(), String> {
    if condition { Ok(()) } else { Err(message.to_owned()) }
}

/// Evaluates `expression` in the page, which gives a string.
fn text(tab: &Tab, expression: &str) -> Result<String, String> {
    tab.evaluate(expression, false)
        .map_err(|e| format!("could not evaluate {expression}: {e}"))?
        .value
        .and_then(|v| v.as_str().map(str::to_owned))
        .ok_or_else(|| format!("{expression} did not return a string"))
}

/// Runs an async script in the page, which returns a string, and waits for it.
fn run(tab: &Tab, body: &str) -> Result<String, String> {
    let script = format!("(async () => {{ {body} }})()");
    tab.evaluate(&script, true)
        .map_err(|e| format!("script failed: {e}"))?
        .value
        .and_then(|v| v.as_str().map(str::to_owned))
        .ok_or_else(|| "the script did not return a string".to_owned())
}

/// Presses the button of the scene with this action. An SVG group has no `click()`, so the event is sent to it.
fn press(tab: &Tab, action: &str) -> Result<(), String> {
    text(
        tab,
        &format!(
            "document.querySelector('#springs-diagram [data-action={action}]') \
                .dispatchEvent(new MouseEvent('click', {{ bubbles: true }})), 'clicked'"
        ),
    )
    .map(drop)
}

fn physics(tab: &Tab) -> Result<String, String> {
    text(tab, &format!("document.querySelector('{PANEL}').dataset.physics"))
}

fn pending(tab: &Tab) -> Result<String, String> {
    text(tab, &format!("document.querySelector('{PANEL}').dataset.physicsPending"))
}

/// Waits until the panel says `state`, or fails after `seconds`.
fn wait_for_state(tab: &Tab, state: &str, seconds: u64) -> Result<(), String> {
    let deadline = Instant::now() + Duration::from_secs(seconds);
    loop {
        if physics(tab)? == state {
            return Ok(());
        }
        if Instant::now() > deadline {
            return Err(format!("the panel never said {state:?}; it says {:?}", physics(tab)?));
        }
        std::thread::sleep(Duration::from_millis(50));
    }
}

/// The `transform` of each vertex, by name, as a JSON object.
const POSITIONS: &str = "JSON.stringify(Object.fromEntries([...document.querySelectorAll( \
     '#springs-diagram > g.svg-dom-graph-content > g')].filter(g => g.querySelector(':scope > ellipse')) \
     .map(g => [g.querySelector('text').textContent, g.getAttribute('transform')])))";

fn positions(tab: &Tab) -> Result<String, String> {
    text(tab, POSITIONS)
}

fn weights(tab: &Tab) -> Result<String, String> {
    text(
        tab,
        "[...document.querySelectorAll('#springs-diagram [data-svg-dom-graph-edge-label] text')] \
         .map(t => t.textContent).join(',')",
    )
}

/// The centre of vertex `name` in the page, in viewport pixels.
fn centre(tab: &Tab, name: &str) -> Result<(f64, f64), String> {
    let raw = text(
        tab,
        &format!(
            "(() => {{ const g = [...document.querySelectorAll('#springs-diagram > g.svg-dom-graph-content > g')] \
             .find(g => g.querySelector(':scope > ellipse') && g.querySelector('text').textContent === '{name}'); \
             const r = g.querySelector('ellipse').getBoundingClientRect(); \
             return (r.left + r.width / 2) + ',' + (r.top + r.height / 2); }})()"
        ),
    )?;
    let (x, y) = raw.split_once(',').ok_or("a centre")?;
    Ok((x.parse().map_err(|e| format!("{e}"))?, y.parse().map_err(|e| format!("{e}"))?))
}

/// JavaScript that finds vertex `name` and defines `press`, `move_to` and `release` for pointer `id` on it.
fn pointer_helpers(name: &str, id: u32) -> String {
    format!(
        "const g = [...document.querySelectorAll('#springs-diagram > g.svg-dom-graph-content > g')] \
         .find(g => g.querySelector(':scope > ellipse') && g.querySelector('text').textContent === '{name}'); \
         const r = g.querySelector('ellipse').getBoundingClientRect(); \
         const cx = r.left + r.width / 2, cy = r.top + r.height / 2; \
         const fire = (type, x, y) => g.dispatchEvent(new PointerEvent(type, \
             {{ bubbles: true, clientX: x, clientY: y, button: 0, pointerId: {id}, pointerType: 'mouse' }})); \
         const sleep = ms => new Promise(r => setTimeout(r, ms));"
    )
}

/// Drags B with a real mouse and says whether its neighbour A was pulled along.
fn a_drag_of_b_moves_a(tab: &Tab) -> Result<bool, String> {
    let before = centre(tab, "A")?;
    let (x, y) = centre(tab, "B")?;
    drag(tab, &[(x, y), (x + 30.0, y + 10.0), (x + 60.0, y + 25.0)])?;
    wait_for_state(tab, "sleeping", 25)?;
    Ok(centre(tab, "A")? != before)
}

mod unit_tests;
