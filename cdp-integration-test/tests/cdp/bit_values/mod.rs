//! Bit arrays (`NodeValues::Bits`) in a real browser: every width from 1 to 7 draws exactly that many binary digits
//! inside its cell, a node says its width, and a replacement of the same width rewrites the cells while one of another
//! width is refused.
//!
//! The fixture has one array of each width, `bits_1` to `bits_7`, each holding zero and the largest value that fits.
//! Native unit tests cover how a value is formatted. What they cannot reach is the drawing and measuring.

use crate::common::new_tab;
use headless_chrome::Tab;

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

/// The text of every cell of the node called `name`, in order.
fn cells(tab: &Tab, name: &str) -> Result<Vec<String>, String> {
    let json = text(
        tab,
        &format!(
            "JSON.stringify([...document.querySelectorAll('#diagram g[aria-label^=\"{name}\"] text')] \
             .map(t => t.textContent))"
        ),
    )?;
    serde_json::from_str(&json).map_err(|e| format!("could not read the cells of {name}: {e}"))
}

mod unit_tests;
