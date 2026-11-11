//! The CRC-32 demo, in the window of its own that the gallery's panel opens: the checksum on opening, the check value
//! on typing, an empty text, the cut at 64 bytes, the focus and caret across the redraw, the buttons in the scene that
//! introduce an error, the receiver finding and correcting a flipped bit, and the stepping toolbar that walks through
//! the calculation.
//!
//! The calculation is covered by native unit tests against published checksums. What they cannot reach is the browser:
//! that a real change draws the scene again with the right nodes, that the buttons drawn in the scene work, and that
//! the text box keeps focus across the redraw.
//!
//! Like [`crate::huffman_walk`], these run against the built demo gallery's window page, not the fixture.

use crate::common::{demo_tab::DemoTab, new_demo_tab};
use headless_chrome::Tab;
use std::time::Duration;

const DIAGRAM: &str = "#crc32-diagram";
/// The text box that holds the text to checksum.
const TEXT_BOX: &str = "#crc32-diagram g[aria-label^=\"Text to checksum\"] input";
const VIEWPORT: (u32, u32) = (2600, 2400);

fn open() -> Result<DemoTab, String> {
    new_demo_tab("crc32-window.html", TEXT_BOX, VIEWPORT)
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
            "(() => {{ const i = document.querySelector('{TEXT_BOX}'); i.value = {value:?}; \
             i.dispatchEvent(new Event('input', {{ bubbles: true }})); }})()"
        ),
        false,
    )
    .map_err(|e| format!("could not edit the text box: {e}"))?;
    std::thread::sleep(Duration::from_millis(300));
    Ok(())
}

/// The CRC-32 the sender's XOR shows, as eight hex digits. It is the value of the first XOR node in the scene.
fn result(tab: &Tab) -> Result<String, String> {
    text(
        tab,
        &format!(
            "(() => {{ const label = [...document.querySelectorAll('{DIAGRAM} text')].find(t => t.textContent === 'XOR'); \
             if (!label) return ''; \
             return [...label.closest('g').querySelectorAll('text')].map(t => t.textContent) \
                 .filter(t => /^[0-9A-F]{{2}}( [0-9A-F]{{2}}){{3}}$/.test(t)).join('').replaceAll(' ', ''); }})()"
        ),
    )
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

/// The texts of every box in the diagram, in the order they were drawn.
fn texts(tab: &Tab) -> Result<Vec<String>, String> {
    let json = text(
        tab,
        &format!("JSON.stringify([...document.querySelectorAll('{DIAGRAM} text')].map(t => t.textContent))"),
    )?;
    serde_json::from_str(&json).map_err(|e| format!("could not read the box texts: {e}"))
}

/// Whether the diagram says `phrase` somewhere, in one box or across the lines of a note.
fn says(tab: &Tab, phrase: &str) -> Result<bool, String> {
    Ok(texts(tab)?.join(" ").contains(phrase))
}

/// Presses the button that flips one bit at random.
fn flip(tab: &Tab) -> Result<(), String> {
    tab.evaluate(
        &format!(
            "document.querySelector('{DIAGRAM} g[data-action=\"introduce\"]') \
             .dispatchEvent(new MouseEvent('click', {{ bubbles: true }}))"
        ),
        false,
    )
    .map_err(|e| format!("could not flip a bit: {e}"))?;
    std::thread::sleep(Duration::from_millis(400));
    Ok(())
}

/// Presses the button that puts every flipped bit back.
fn remove_errors(tab: &Tab) -> Result<(), String> {
    tab.evaluate(
        &format!(
            "document.querySelector('{DIAGRAM} g[data-action=\"remove\"]') \
             .dispatchEvent(new MouseEvent('click', {{ bubbles: true }}))"
        ),
        false,
    )
    .map_err(|e| format!("could not remove the errors: {e}"))?;
    std::thread::sleep(Duration::from_millis(400));
    Ok(())
}

/// What the error-position box says: the one text in the diagram that is in the form of a position.
fn position(tab: &Tab) -> Result<String, String> {
    let is_digits = |t: &str| !t.is_empty() && t.bytes().all(|b| b.is_ascii_digit());
    let is_position = |t: &str| {
        t == "none"
            || t == "more than one bit"
            || t.strip_prefix("byte ")
                .and_then(|rest| rest.split_once(", bit "))
                .is_some_and(|(byte, bit)| is_digits(byte) && is_digits(bit))
            || t.strip_prefix("bit ")
                .and_then(|rest| rest.strip_suffix(" of the CRC"))
                .is_some_and(is_digits)
    };
    texts(tab)?
        .into_iter()
        .find(|t| is_position(t))
        .ok_or_else(|| "no error position in the diagram".to_owned())
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The walk's position, read from the hidden "Step" driver's own selection. `None` before the walk has started.
fn step_position(tab: &Tab) -> Result<Option<usize>, String> {
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

/// Waits until `step_position` returns something other than `from`, or fails after five seconds.
fn wait_for_move(tab: &Tab, from: Option<usize>) -> Result<(), String> {
    let deadline = std::time::Instant::now() + Duration::from_secs(5);
    while std::time::Instant::now() < deadline {
        if step_position(tab)? != from {
            // Every move draws the scene again. Give the redraw its own moment to finish attaching.
            std::thread::sleep(Duration::from_millis(100));
            return Ok(());
        }
        std::thread::sleep(Duration::from_millis(20));
    }
    Err(format!("the walk never left {from:?}"))
}

/// Presses a toolbar button with a real mouse click and waits for the walk to move.
fn click(tab: &Tab, name: &str) -> Result<(), String> {
    let from = step_position(tab)?;
    tab.find_element(&format!("{DIAGRAM} g[aria-label=\"{name}\"]"))
        .map_err(|e| format!("could not find the {name:?} button: {e}"))?
        .click()
        .map_err(|e| format!("could not click {name:?}: {e}"))?;
    wait_for_move(tab, from)
}

/// Walks to `target` one step at a time.
fn goto(tab: &Tab, target: usize) -> Result<(), String> {
    for _ in 0..80 {
        let now = step_position(tab)?.unwrap_or(0);
        if now == target {
            return Ok(());
        }
        click(tab, if now < target { "Next selection" } else { "Previous selection" })?;
    }
    Err(format!(
        "could not reach position {target}, stopped at {:?}",
        step_position(tab)?
    ))
}

/// Whether the node whose accessible name starts with `name` says that it is not yet computed.
fn faint(tab: &Tab, name: &str) -> Result<bool, String> {
    Ok(text(
        tab,
        &format!(
            "(document.querySelector('{DIAGRAM} g[aria-label^=\"{name}\"]')?.getAttribute('aria-label') ?? 'missing')"
        ),
    )?
    .contains("not yet computed"))
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// How many explanations say that they are not yet computed, to a screen reader as well as by being faint.
fn faint_notes(tab: &Tab) -> Result<usize, String> {
    text(
        tab,
        &format!("String(document.querySelectorAll('{DIAGRAM} g[aria-label=\"Text, not yet computed\"]').length)"),
    )?
    .parse()
    .map_err(|e| format!("could not count the notes: {e}"))
}

mod unit_tests;
