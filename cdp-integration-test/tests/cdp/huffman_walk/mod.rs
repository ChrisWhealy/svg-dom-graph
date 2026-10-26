//! The Huffman demo's own window: its stepping toolbar and its text input node, driven by real clicks and key presses
//! in Chrome.
//!
//! The algorithm and the walk are covered by native unit tests. What they cannot reach is the browser-facing behaviour:
//! what the window shows at each position, that a step backwards restores what a step forwards drew, that editing the
//! text starts the walk again, which nodes the route through the tree marks, and what a redraw keeps (zoom, and
//! keyboard focus on a button or in the text box). Every step draws the whole scene again, so every step is a place
//! where a redraw could drop one of those.
//!
//! Most scenarios set the text to "abracadabra", which is small enough to work by hand. It has 4 merges and 5 distinct
//! bytes, so its walk has 11 positions: `0` counts, `1..=4` merge, `5` reads the lengths and `6..=10` assign the
//! canonical codes. Pass two continues from there, with one position for each input byte and a last one for the flush.
//! The demo's own default text is the first sentence of *Pride and Prejudice*, which has its own scenarios below.
//!
//! Like [`crate::chacha20_walk`], these run against the built demo gallery's window page, not the fixture.

use crate::common::{DemoTab, new_demo_tab};
use headless_chrome::Tab;
use std::time::{Duration, Instant};

const DIAGRAM: &str = "#huffman-diagram";
const VIEWPORT: (u32, u32) = (1800, 2400);
const LAST: usize = 10;
/// "abracadabra" has 11 bytes, so pass two has 11 positions of packing and a last one for the flush.
const FLUSH: usize = LAST + 12;
/// The fill the library gives a cell marked by a secondary selection.
const SECONDARY_FILL: &str = "#7fd8be";

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The demo as it opens, with its own default text.
fn open_default() -> Result<DemoTab, String> {
    new_demo_tab("huffman-window.html", &format!("{DIAGRAM} g[aria-label^=\"Step:\"]"), VIEWPORT)
}

/// The demo with the text set to "abracadabra".
fn open() -> Result<DemoTab, String> {
    let tab = open_default()?;
    set_text(&tab, "abracadabra")?;
    Ok(tab)
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

/// The text of every box in the diagram, in the order they were drawn.
fn texts(tab: &Tab) -> Result<Vec<String>, String> {
    let json = text(
        tab,
        &format!("JSON.stringify([...document.querySelectorAll('{DIAGRAM} text')].map(t => t.textContent))"),
    )?;
    serde_json::from_str(&json).map_err(|e| format!("could not read the box texts: {e}"))
}

/// The lines of the `index`th Markdown node, joined with a space, which rebuilds its text. The first is the status line
/// and the second is the explanation of the step.
fn markdown(tab: &Tab, index: usize) -> Result<String, String> {
    text(
        tab,
        &format!(
            "[...document.querySelectorAll('{DIAGRAM} g[aria-label=\"Text\"]')][{index}] \
             ?.querySelectorAll('text') ? [...[...document.querySelectorAll('{DIAGRAM} g[aria-label=\"Text\"]')][{index}] \
             .querySelectorAll('text')].map(t => t.textContent).join(' ') : ''"
        ),
    )
}

/// What the step explains.
fn explanation(tab: &Tab) -> Result<String, String> {
    markdown(tab, 1)
}

/// What the status line says.
fn status(tab: &Tab) -> Result<String, String> {
    markdown(tab, 0)
}

/// The code table's five columns for each distinct byte, in the order shown, without the headings.
fn table(tab: &Tab) -> Result<Vec<Vec<String>>, String> {
    let all = texts(tab)?;
    let start = all.iter().position(|t| t == "Byte").ok_or("the table has no heading")?;
    // The hidden "Step" driver is drawn straight after the table, and is the first box that is not part of it.
    let cells: Vec<String> = all[start + 5..].iter().take_while(|c| *c != "Step").cloned().collect();
    Ok(cells.chunks(5).map(<[String]>::to_vec).collect())
}

/// How many leaves and branches the forest shows. A leaf's box reads "x n" for the byte `x` and its count `n`, and a
/// branch's reads only its weight, which is at least 2. The boxes that label a link read "0" or "1".
fn forest(tab: &Tab) -> Result<(usize, usize), String> {
    let all = texts(tab)?;
    let end = all.iter().position(|t| t == "Byte").ok_or("the table has no heading")?;
    let forest = &all[..end];
    let leaves = forest
        .iter()
        .filter(|t| {
            t.rsplit_once(" x")
                .is_some_and(|(_, n)| !n.is_empty() && n.bytes().all(|b| b.is_ascii_digit()))
        })
        .count();
    let branches = forest.iter().filter(|t| t.parse::<u32>().is_ok_and(|w| w >= 2)).count();
    Ok((leaves, branches))
}

/// The texts of the boxes the library has marked with a secondary selection, sorted. The marked box of a node with one
/// value is its outer rectangle, which is a sibling of its text.
fn marked(tab: &Tab) -> Result<Vec<String>, String> {
    let json = text(
        tab,
        &format!(
            "JSON.stringify([...document.querySelectorAll('{DIAGRAM} rect')] \
                 .filter(r => r.getAttribute('fill') === '{SECONDARY_FILL}') \
                 .map(r => r.parentElement.querySelector('text')?.textContent ?? '').sort())"
        ),
    )?;
    serde_json::from_str(&json).map_err(|e| format!("could not read the marked boxes: {e}"))
}

/// The walk's position, read from the hidden "Step" driver's own selection.
fn position(tab: &Tab) -> Result<Option<usize>, String> {
    let name = text(
        tab,
        &format!("(document.querySelector('{DIAGRAM} g[aria-label^=\"Step:\"]')?.getAttribute('aria-label') ?? '')"),
    )?;
    let Some((before, _)) = name.split_once(" selected") else {
        return Ok(None);
    };
    before
        .rsplit("cell ")
        .next()
        .and_then(|n| n.parse().ok())
        .map(Some)
        .ok_or_else(|| format!("could not read a cell from {name:?}"))
}

/// Waits until `position` returns something other than `from`, or fails after five seconds.
fn wait_for_move(tab: &Tab, from: Option<usize>) -> Result<(), String> {
    let deadline = Instant::now() + Duration::from_secs(5);
    while Instant::now() < deadline {
        if position(tab)? != from {
            // Every move draws the scene again. Give the redraw its own moment to finish attaching.
            std::thread::sleep(Duration::from_millis(100));
            return Ok(());
        }
        std::thread::sleep(Duration::from_millis(20));
    }
    Err(format!("the walk never left {from:?}"))
}

fn button<'a>(tab: &'a Tab, name: &str) -> Result<headless_chrome::Element<'a>, String> {
    tab.find_element(&format!("{DIAGRAM} g[aria-label=\"{name}\"]"))
        .map_err(|e| format!("could not find the {name:?} button: {e}"))
}

/// Presses a toolbar button with a real mouse click and waits for the walk to move.
fn click(tab: &Tab, name: &str) -> Result<(), String> {
    let from = position(tab)?;
    button(tab, name)?
        .click()
        .map_err(|e| format!("could not click {name:?}: {e}"))?;
    wait_for_move(tab, from)
}

/// Walks to `target` one step at a time.
fn goto(tab: &Tab, target: usize) -> Result<(), String> {
    for _ in 0..80 {
        let now = position(tab)?.unwrap_or(0);
        if now == target {
            return Ok(());
        }
        click(tab, if now < target { "Next selection" } else { "Previous selection" })?;
    }
    Err(format!("could not reach position {target}, stopped at {:?}", position(tab)?))
}

/// Replaces the text box's value and fires the `input` event a real keystroke would.
fn set_text(tab: &Tab, value: &str) -> Result<(), String> {
    tab.evaluate(
        &format!(
            "(() => {{ const i = document.querySelector('{DIAGRAM} input'); i.value = {value:?}; \
             i.dispatchEvent(new Event('input', {{ bubbles: true }})); }})()"
        ),
        false,
    )
    .map_err(|e| format!("could not edit the text box: {e}"))?;
    std::thread::sleep(Duration::from_millis(250));
    Ok(())
}

fn row(symbol: &str, count: &str, path: &str, length: &str, code: &str) -> Vec<String> {
    [symbol, count, path, length, code].map(str::to_owned).to_vec()
}

mod editing;
mod explanation;
mod pass_two;
mod redraw;
mod routes;
mod walk;
mod window;
