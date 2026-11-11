//! The Base64 panel in the built demo gallery: typing reaches the encoding, both alphabets, the final groups of one and
//! two bytes, an empty text, the cut at 256 bytes falling between characters, and the text box keeping focus and its
//! caret across the redraw that every change makes.
//!
//! The encoding itself is covered by native unit tests against the RFC 4648 vectors. What they cannot reach is the
//! browser: that a real change redraws the scene with the right nodes, that the radio buttons draw it again, and what
//! the redraw keeps.
//!
//! A change redraws the scene, which replaces the text box, and a composition with an input method editor does not
//! survive that. So the demo waits until the composition ends. The last test drives a composition through CDP's own
//! `Input.imeSetComposition`.

use crate::common::{demo_tab::DemoTab, new_demo_tab};
use headless_chrome::Tab;
use std::time::Duration;

const DIAGRAM: &str = "#base64-diagram";

fn open() -> Result<DemoTab, String> {
    new_demo_tab("index.html#panel-base64", &format!("{DIAGRAM} input"), (1800, 2400))
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

/// Sets the text box to `value` as a change the user made.
fn set_text(tab: &Tab, value: &str) -> Result<(), String> {
    tab.evaluate(
        &format!(
            "(() => {{ const i = document.querySelector('{DIAGRAM} input'); i.value = {value:?}; \
             i.dispatchEvent(new Event('input', {{ bubbles: true }})); }})()"
        ),
        false,
    )
    .map_err(|e| format!("could not edit the text box: {e}"))?;
    std::thread::sleep(Duration::from_millis(300));
    Ok(())
}

/// Chooses the alphabet whose radio button has the id `id`.
fn choose(tab: &Tab, id: &str) -> Result<(), String> {
    tab.evaluate(&format!("document.getElementById('{id}').click()"), false)
        .map_err(|e| format!("could not choose {id}: {e}"))?;
    std::thread::sleep(Duration::from_millis(300));
    Ok(())
}

/// The Base64 text on the line under the diagram.
fn result(tab: &Tab) -> Result<String, String> {
    text(tab, "document.getElementById('base64-result').textContent")
}

/// The text of every box in the diagram, in the order they were drawn.
fn texts(tab: &Tab) -> Result<Vec<String>, String> {
    let json = text(
        tab,
        &format!("JSON.stringify([...document.querySelectorAll('{DIAGRAM} text')].map(t => t.textContent))"),
    )?;
    serde_json::from_str(&json).map_err(|e| format!("could not read the box texts: {e}"))
}

/// How many nodes of the diagram have an accessible name starting with `name`.
fn nodes_named(tab: &Tab, name: &str) -> Result<usize, String> {
    text(
        tab,
        &format!("String(document.querySelectorAll('{DIAGRAM} g[aria-label^=\"{name}\"]').length)"),
    )?
    .parse()
    .map_err(|e| format!("could not count {name}: {e}"))
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The left and right edges of the node whose accessible name starts with `name`, and of the diagram itself.
fn edges(tab: &Tab, name: &str) -> Result<(f64, f64, f64), String> {
    let json = text(
        tab,
        &format!(
            "(() => {{ const r = document \
             .querySelector('{DIAGRAM} g[aria-label^=\"{name}\"]').getBoundingClientRect(); \
             const d = document.querySelector('{DIAGRAM}').getBoundingClientRect(); \
             return JSON.stringify([r.left, r.right, d.left]); }})()"
        ),
    )?;
    let [left, right, diagram]: [f64; 3] =
        serde_json::from_str(&json).map_err(|e| format!("could not read the edges of {name}: {e}"))?;
    Ok((left, right, diagram))
}

mod unit_tests;
