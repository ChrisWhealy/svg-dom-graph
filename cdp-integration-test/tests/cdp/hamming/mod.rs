//! The Hamming(7,4) demo in the window the gallery's panel opens: the path of boxes laid out as a block, the buttons
//! that choose the data and flip a bit, and the stepping toolbar that walks along the path.
//!
//! The code itself and the layout arithmetic are covered by native unit tests. What they cannot reach is the browser:
//! that the scene is drawn with the nine boxes where the layout says, that the buttons drawn in the scene work, and
//! that each step makes the right boxes faint.
//!
//! Like [`crate::crc32`], these run against the built demo gallery's window page, not the fixture.

use crate::common::{demo_tab::DemoTab, new_demo_tab};
use headless_chrome::Tab;
use std::time::{Duration, Instant};

const DIAGRAM: &str = "#hamming-diagram";
const VIEWPORT: (u32, u32) = (2000, 1800);
/// The last position of the walk: nine boxes, and the first needs no step.
const LAST: usize = 8;
/// The boxes of the path, in order, by the start of their accessible names.
const BOXES: [&str; 9] = [
    "Data bits",
    "Parity bits",
    "Codeword",
    "Received codeword",
    "Check bits",
    "Syndrome",
    "Error position",
    "Corrected codeword",
    "Recovered data",
];

fn open() -> Result<DemoTab, String> {
    open_with("")
}

/// Opens the window with `query` on its address, such as `?error=5`, which opens it with that bit already flipped.
fn open_with(query: &str) -> Result<DemoTab, String> {
    new_demo_tab(
        &format!("hamming-window.html{query}"),
        &format!("{DIAGRAM} g[aria-label^=\"Data bits\"]"),
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

/// Presses the button drawn in the scene with `action`.
fn press(tab: &Tab, action: &str) -> Result<(), String> {
    tab.evaluate(
        &format!(
            "document.querySelector('{DIAGRAM} g[data-action=\"{action}\"]') \
             .dispatchEvent(new MouseEvent('click', {{ bubbles: true }}))"
        ),
        false,
    )
    .map_err(|e| format!("could not press {action}: {e}"))?;
    std::thread::sleep(Duration::from_millis(400));
    Ok(())
}

/// The accessible name of the button drawn in the scene with `action`.
fn label_of(tab: &Tab, action: &str) -> Result<String, String> {
    text(
        tab,
        &format!("document.querySelector('{DIAGRAM} g[data-action=\"{action}\"]').getAttribute('aria-label')"),
    )
}

/// Whether the box whose accessible name starts with `name` says that it is not yet computed.
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
    let deadline = Instant::now() + Duration::from_secs(5);
    while Instant::now() < deadline {
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
    for _ in 0..30 {
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

/// The rectangle `[x, y, width, height]` of each box of the path, in order, in the page.
fn rects(tab: &Tab) -> Result<Vec<[f64; 4]>, String> {
    let names = serde_json::to_string(&BOXES).map_err(|e| e.to_string())?;
    let json = text(
        tab,
        &format!(
            "JSON.stringify({names}.map(n => {{ const r = document.querySelector('{DIAGRAM} g[aria-label^=\"' + n + '\"]') \
             .getBoundingClientRect(); return [r.x, r.y, r.width, r.height]; }}))"
        ),
    )?;
    serde_json::from_str(&json).map_err(|e| format!("could not read the boxes: {e}"))
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The accessible names of the notes, which are the nodes of Markdown text, in the order they were drawn. The first
/// node of Markdown text is the status line beside the buttons, and it is not a note.
fn note_names(tab: &Tab) -> Result<Vec<String>, String> {
    let json = text(
        tab,
        &format!(
            "JSON.stringify([...document.querySelectorAll('{DIAGRAM} g[aria-label^=\"Text\"]')].map(g => g.getAttribute('aria-label')))"
        ),
    )?;
    let mut names: Vec<String> = serde_json::from_str(&json).map_err(|e| format!("could not read the notes: {e}"))?;
    names.remove(0);
    Ok(names)
}

/// The accessible name of the box that starts with `name`.
fn box_label(tab: &Tab, name: &str) -> Result<String, String> {
    text(
        tab,
        &format!("document.querySelector('{DIAGRAM} g[aria-label^=\"{name}\"]').getAttribute('aria-label')"),
    )
}

/// The scene's content layer `transform`, which is its zoom and pan, or an empty string if it has none.
fn view(tab: &Tab) -> Result<String, String> {
    text(
        tab,
        &format!("(document.querySelector('{DIAGRAM} > g.svg-dom-graph-content')?.getAttribute('transform') ?? '')"),
    )
}

/// Clicks the button `name` with a real mouse click, and waits a moment, without waiting for the walk to move. For a
/// button that leaves the walk where it is.
fn click_and_settle(tab: &Tab, name: &str) -> Result<(), String> {
    tab.find_element(&format!("{DIAGRAM} g[aria-label=\"{name}\"]"))
        .map_err(|e| format!("could not find the {name:?} button: {e}"))?
        .click()
        .map_err(|e| format!("could not click {name:?}: {e}"))?;
    std::thread::sleep(Duration::from_millis(400));
    Ok(())
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// At `position`, the live region outside the scene says which step the walk is at and what it has reached, so that a
/// screen reader can follow every stage.
fn announces(position: usize) -> Result<(), String> {
    let tab = open()?;
    goto(&tab, position)?;
    let said = text(&tab, "document.getElementById('hamming-announcement').textContent")?;
    check(
        said.starts_with(&format!("Step {} of 9: {}", position + 1, BOXES[position])),
        &format!("position {position} announced {said:?}"),
    )
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// At `position`, the note under a box is faint until the walk reaches the box, and says so to a screen reader, so that
/// it can be read ahead of the walk but not mistaken for a step that has been done.
fn notes_follow_the_walk(position: usize) -> Result<(), String> {
    let tab = open()?;
    goto(&tab, position)?;
    let names = note_names(&tab)?;
    check_eq(names.len(), BOXES.len())?;
    for (index, name) in names.iter().enumerate() {
        check_eq(name.contains("not yet computed"), index > position)?;
    }
    Ok(())
}
// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Restarting from `position`, after changing the data and introducing an error, returns to the start: the error is
/// gone, the later boxes are faint again, and the data bit that was changed stays changed. At position 0 the walk is
/// already at the start, and Restart leaves it there.
fn restart_from(position: usize) -> Result<(), String> {
    let tab = open()?;
    press(&tab, "toggle-1")?;
    press(&tab, "introduce")?;
    goto(&tab, position)?;
    if position == 0 {
        click_and_settle(&tab, "Restart selection")?;
    } else {
        click(&tab, "Restart selection")?;
    }
    check_eq(step_position(&tab)?, Some(0))?;
    check(
        says(&tab, "No error has been introduced.")?,
        "restarting should remove the flipped bit",
    )?;
    for name in &BOXES[1..] {
        check(faint(&tab, name)?, &format!("{name} should be faint again"))?;
    }
    check_eq(label_of(&tab, "toggle-1")?.as_str(), "d2 = 1")
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// With the bit at `error` flipped, which the page's address chooses, the status line names it, and the receiver finds
/// that same position at the end of the path and recovers the data. It is deterministic, unlike the button.
fn finds_error_at(error: usize) -> Result<(), String> {
    let tab = open_with(&format!("?error={error}"))?;
    check(
        says(&tab, &format!("The bit at position {error} is flipped in transmission."))?,
        "the status line should name the flipped bit",
    )?;
    goto(&tab, LAST)?;
    let found = text(
        &tab,
        &format!(
            "[...document.querySelector('{DIAGRAM} g[aria-label^=\"Error position\"]').querySelectorAll('text')].at(-1).textContent"
        ),
    )?;
    check_eq(found, error.to_string())?;
    check(says(&tab, "They are the data that was sent.")?, "the data should be recovered")?;
    // The flipped bit is marked where it arrived, and where it was put right.
    let cell = error - 1;
    check(
        box_label(&tab, "Received codeword")?.contains(&format!("also highlighted: cell {cell}")),
        "the received codeword should mark the flipped bit",
    )?;
    check(
        box_label(&tab, "Corrected codeword")?.contains(&format!("also highlighted: cell {cell}")),
        "the corrected codeword should mark the bit put right",
    )
}

mod unit_tests;
