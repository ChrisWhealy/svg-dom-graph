//! The Markdown node and the text input node, in the built demo gallery's "Markdown node" and "Text input node" panels.
//!
//! The Markdown parser and its layout are covered by native unit tests. What they cannot reach is the browser: that the
//! lines are drawn inside the box, that bold, italic and code runs are drawn as such, that real typing reaches the text
//! input node's callback, and that typing in it, or a drag of the node, does not disturb the rest of the scene.

mod measured;
mod runs;

use crate::common::{demo_tab::DemoTab, drag, new_demo_tab};
use crate::text_nodes::{measured::Measured, runs::Runs};
use headless_chrome::Tab;
use std::time::Duration;

const MARKDOWN_DIAGRAM: &str = "#markdown-diagram";
const DIAGRAM: &str = "#text-input-diagram";

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The Markdown panel, once its node is drawn.
fn open_markdown() -> Result<DemoTab, String> {
    new_demo_tab(
        "index.html#panel-markdown",
        &format!("{MARKDOWN_DIAGRAM} g[aria-label^=\"Text\"]"),
        (1400, 1400),
    )
}

/// The text input panel, once its text box is drawn.
fn open() -> Result<DemoTab, String> {
    new_demo_tab("index.html#panel-text-input", &format!("{DIAGRAM} input"), (1400, 1400))
}

fn check(condition: bool, message: &str) -> Result<(), String> {
    if condition { Ok(()) } else { Err(message.to_owned()) }
}

fn check_eq<T: PartialEq + std::fmt::Debug>(got: T, expected: T) -> Result<(), String> {
    check(got == expected, &format!("expected {expected:?}, got {got:?}"))
}

fn text(tab: &Tab, expression: &str) -> Result<String, String> {
    tab.evaluate(expression, false)
        .map_err(|e| format!("could not evaluate {expression}: {e}"))?
        .value
        .and_then(|v| v.as_str().map(str::to_owned))
        .ok_or_else(|| format!("{expression} did not return a string"))
}

/// The scene's content layer `transform`, which is its zoom and pan, or an empty string if it has none.
fn view(tab: &Tab) -> Result<String, String> {
    text(
        tab,
        &format!("(document.querySelector('{DIAGRAM} > g.svg-dom-graph-content')?.getAttribute('transform') ?? '')"),
    )
}

fn echo(tab: &Tab) -> Result<String, String> {
    text(tab, "document.getElementById('text-input-echo').textContent")
}

/// The Markdown node's `<g>`: the first node group, which is the one holding the headings.
const MARKDOWN_GROUP: &str = "g[aria-label^=\"Text\"]";

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The text input node's `<g>`, found by its label.
const INPUT_GROUP: &str = "g[aria-label^=\"Text to echo\"]";

/// The `(left, top, right, bottom)` of the first element matching `selector`, in page pixels.
fn bounds(tab: &Tab, selector: &str) -> Result<(f64, f64, f64, f64), String> {
    let json = text(
        tab,
        &format!(
            "(() => {{ const r = document.querySelector('{selector}').getBoundingClientRect(); \
                return JSON.stringify([r.left, r.top, r.right, r.bottom]); }})()"
        ),
    )?;
    serde_json::from_str(&json).map_err(|e| format!("{e}"))
}

fn input_group_transform(tab: &Tab) -> Result<String, String> {
    text(
        tab,
        &format!("document.querySelector('{DIAGRAM} {INPUT_GROUP}').getAttribute('transform')"),
    )
}

fn caret(tab: &Tab) -> Result<(u32, u32), String> {
    let json = text(
        tab,
        &format!(
            "(() => {{ const i = document.querySelector('{DIAGRAM} input'); \
                return JSON.stringify([i.selectionStart, i.selectionEnd]); }})()"
        ),
    )?;
    serde_json::from_str(&json).map_err(|e| format!("{e}"))
}

fn calls(tab: &Tab) -> Result<String, String> {
    text(tab, "document.getElementById('text-input-calls').textContent")
}

mod unit_tests;
