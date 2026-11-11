//! The hex dump example in the gallery's Cell selection panel: a named hex dump node whose own selection toolbar is
//! drawn inside its box, and which marks the selected byte twice, by its hex digits and by its character.
//!
//! Like [`crate::huffman_walk`], this runs against the built demo gallery, not the fixture.

use crate::common::{demo_tab::DemoTab, new_demo_tab};
use headless_chrome::Tab;
use std::time::Duration;

const DIAGRAM: &str = "#selection-hexdump-diagram";
/// The fill of the block over the selected byte's hex digits: the primary selection colour.
const PRIMARY: &str = "#ff6b4a";
/// The fill of the block over its character: the secondary selection colour.
const SECONDARY: &str = "#7fd8be";

fn open() -> Result<DemoTab, String> {
    new_demo_tab(
        "index.html#panel-selection-hexdump",
        &format!("{DIAGRAM} g[aria-label^=\"hexdump\"]"),
        (1400, 3000),
    )
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

/// Presses a toolbar button `times` times, with the events a real click sends, and lets the redraw finish.
fn press(tab: &Tab, label: &str, times: usize) -> Result<(), String> {
    let js = format!(
        "(() => {{ const b = document.querySelector('{DIAGRAM} g[aria-label=\"{label}\"]'); \
         for (let i = 0; i < {times}; i++) {{ for (const t of ['pointerdown','mousedown','pointerup','mouseup','click']) \
         b.dispatchEvent(new MouseEvent(t, {{bubbles: true}})); }} return ''; }})()"
    );
    text(tab, &js)?;
    std::thread::sleep(Duration::from_millis(200));
    Ok(())
}

/// The status line the example shows beside the diagram.
fn status(tab: &Tab) -> Result<String, String> {
    text(tab, "document.getElementById('selection-hexdump-index').textContent")
}

/// For each of the two marks, its block's fill and the text over it, as `fill=text`, for the marks that are visible.
fn marks(tab: &Tab) -> Result<Vec<String>, String> {
    let json = text(
        tab,
        &format!(
            "JSON.stringify([...document.querySelectorAll('{DIAGRAM} rect')] \
             .filter(r => r.getAttribute('visibility') === 'visible') \
             .map(r => r.getAttribute('fill') + '=' + r.nextElementSibling.textContent))"
        ),
    )?;
    serde_json::from_str(&json).map_err(|e| format!("could not read the marks: {e}"))
}

mod unit_tests;
