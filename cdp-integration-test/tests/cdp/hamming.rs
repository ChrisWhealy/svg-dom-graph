//! The Hamming(7,4) demo in the window the gallery's panel opens: the path of boxes laid out as a block, the buttons that
//! choose the data and flip a bit, and the stepping toolbar that walks along the path.
//!
//! The code itself and the layout arithmetic are covered by native unit tests. What they cannot reach is the browser:
//! that the scene is drawn with the nine boxes where the layout says, that the buttons drawn in the scene work, and that
//! each step makes the right boxes faint.
//!
//! Like [`crate::crc32`], these run against the built demo gallery's window page, not the fixture.

use crate::common::{DemoTab, new_demo_tab};
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
    new_demo_tab(
        "hamming-window.html",
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

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
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

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The window opens with the nine boxes of the path and the controls, and the walk at its start.
#[test]
fn the_window_opens_with_the_path_the_controls_and_the_walk_at_the_start() -> Result<(), String> {
    let tab = open()?;
    for name in BOXES {
        check(
            !text(
                &tab,
                &format!("String(!document.querySelector('{DIAGRAM} g[aria-label^=\"{name}\"]'))"),
            )?
            .starts_with("true"),
            &format!("{name} should be drawn"),
        )?;
    }
    for action in ["toggle-0", "toggle-1", "toggle-2", "toggle-3", "introduce", "remove"] {
        check_eq(
            text(
                &tab,
                &format!("String(document.querySelectorAll('{DIAGRAM} g[data-action=\"{action}\"]').length)"),
            )?
            .as_str(),
            "1",
        )?;
    }
    check_eq(step_position(&tab)?, Some(0))?;
    check(says(&tab, "No error has been introduced.")?, "no error to begin with")?;
    // Only the data is there to read.
    check(!faint(&tab, "Data bits")?, "the data is the input")?;
    for name in &BOXES[1..] {
        check(faint(&tab, name)?, &format!("{name} should not be reached yet"))?;
    }
    Ok(())
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The path runs along a row, and back along the next: a block, not a line. The boxes of a row are level, the first row
/// runs left to right and the second right to left, and no two boxes overlap.
#[test]
fn the_path_snakes_into_a_block() -> Result<(), String> {
    let tab = open()?;
    let boxes = rects(&tab)?;
    let middle_y = |b: &[f64; 4]| (b[1] + b[3] / 2.0).round();
    let mut rows: Vec<Vec<usize>> = Vec::new();
    for (index, b) in boxes.iter().enumerate() {
        match rows.last_mut() {
            Some(row) if (middle_y(&boxes[row[0]]) - middle_y(b)).abs() < 4.0 => row.push(index),
            _ => rows.push(vec![index]),
        }
    }
    check(
        (2..=4).contains(&rows.len()),
        &format!("expected a block of 2 to 4 rows, got {}", rows.len()),
    )?;
    for (n, row) in rows.iter().enumerate() {
        let xs: Vec<f64> = row.iter().map(|&i| boxes[i][0]).collect();
        let forward = n % 2 == 0;
        check(
            xs.windows(2).all(|w| if forward { w[0] < w[1] } else { w[0] > w[1] }),
            &format!(
                "row {n} should run {}: {xs:?}",
                if forward { "left to right" } else { "right to left" }
            ),
        )?;
    }
    for (i, a) in boxes.iter().enumerate() {
        for b in &boxes[i + 1..] {
            let apart = a[0] + a[2] <= b[0] || b[0] + b[2] <= a[0] || a[1] + a[3] <= b[1] || b[1] + b[3] <= a[1];
            check(apart, "two boxes overlap")?;
        }
    }
    Ok(())
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// A data bit's button flips it, says so in its name and its pressed state, and keeps the keyboard focus across the redraw.
#[test]
fn a_data_bit_button_toggles_its_bit_and_keeps_the_focus() -> Result<(), String> {
    let tab = open()?;
    check_eq(label_of(&tab, "toggle-0")?.as_str(), "d1 = 1")?;
    check_eq(
        text(
            &tab,
            &format!("document.querySelector('{DIAGRAM} g[data-action=\"toggle-0\"]').getAttribute('aria-pressed')"),
        )?
        .as_str(),
        "true",
    )?;
    press(&tab, "toggle-0")?;
    check_eq(label_of(&tab, "toggle-0")?.as_str(), "d1 = 0")?;
    check_eq(
        text(
            &tab,
            &format!("document.querySelector('{DIAGRAM} g[data-action=\"toggle-0\"]').getAttribute('aria-pressed')"),
        )?
        .as_str(),
        "false",
    )?;
    check_eq(
        text(&tab, "document.activeElement?.getAttribute('data-action') ?? 'none'")?.as_str(),
        "toggle-0",
    )?;
    check_eq(
        text(&tab, "document.getElementById('hamming-announcement').textContent")?.as_str(),
        "The data bits are now 0 0 1 1.",
    )?;
    // The other buttons are untouched.
    check_eq(label_of(&tab, "toggle-1")?.as_str(), "d2 = 0")?;
    Ok(())
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Each step reaches one more box of the path.
#[test]
fn each_step_reaches_the_next_box() -> Result<(), String> {
    let tab = open()?;
    for position in 1..=LAST {
        goto(&tab, position)?;
        for (index, name) in BOXES.iter().enumerate() {
            check_eq(faint(&tab, name)?, index > position)?;
        }
    }
    // Stepping back makes the later boxes faint again.
    goto(&tab, 2)?;
    check(
        faint(&tab, "Received codeword")?,
        "stepping back should make the receiver faint again",
    )
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Flipping a bit moves the walk to the received codeword, and the receiver finds exactly that bit: the error position
/// at the end of the path is the position the status line says was flipped. The data is recovered whichever bit it was.
#[test]
fn the_receiver_finds_whichever_bit_was_flipped() -> Result<(), String> {
    let tab = open()?;
    // The choice is random, so try several times.
    for _ in 0..4 {
        press(&tab, "introduce")?;
        check_eq(step_position(&tab)?.map(|p| p.max(3)), step_position(&tab)?)?;
        let status = texts(&tab)?
            .join(" ")
            .split("The bit at position ")
            .nth(1)
            .and_then(|rest| rest.split(' ').next().map(str::to_owned))
            .ok_or("the status line does not say which bit")?;
        goto(&tab, LAST)?;
        // The box's own texts are its title, then its value.
        let found = text(
            &tab,
            &format!(
                "[...document.querySelector('{DIAGRAM} g[aria-label^=\"Error position\"]').querySelectorAll('text')].map(t => t.textContent).join('')"
            ),
        )?;
        check_eq(found.strip_prefix("Error position").unwrap_or(&found), status.as_str())?;
        check(says(&tab, "They are the data that was sent.")?, "the data should be recovered")?;
        goto(&tab, 0)?;
    }
    Ok(())
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Flipping a bit moves a walk that is before the received codeword on to it, and never moves one that is past it.
/// Removing the error leaves the walk where it is.
#[test]
fn flipping_a_bit_moves_the_walk_to_the_received_codeword_but_never_back() -> Result<(), String> {
    let tab = open()?;
    press(&tab, "introduce")?;
    check_eq(step_position(&tab)?, Some(3))?;
    check(!faint(&tab, "Received codeword")?, "the received codeword should be reached")?;
    goto(&tab, 6)?;
    press(&tab, "introduce")?;
    check_eq(step_position(&tab)?, Some(6))?;
    press(&tab, "remove")?;
    check_eq(step_position(&tab)?, Some(6))?;
    check(says(&tab, "No error has been introduced.")?, "the error should be gone")
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The toolbar's "Restart" starts the walk again and removes the flipped bit, but keeps the data bits as they were set.
#[test]
fn restart_goes_back_to_the_start_and_removes_the_error_but_keeps_the_data() -> Result<(), String> {
    let tab = open()?;
    press(&tab, "toggle-1")?;
    press(&tab, "introduce")?;
    goto(&tab, 5)?;
    click(&tab, "Restart selection")?;
    check_eq(step_position(&tab)?, None)?;
    check(
        says(&tab, "No error has been introduced.")?,
        "restarting should remove the flipped bit",
    )?;
    check(faint(&tab, "Received codeword")?, "the walk should be back at the start")?;
    check_eq(label_of(&tab, "toggle-1")?.as_str(), "d2 = 1")
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The status line is white, and the buttons have colours of their own that are not the light blue of the notes.
#[test]
fn the_status_line_is_white_and_the_buttons_are_coloured() -> Result<(), String> {
    const NOTE: &str = "#eef4ff";
    let tab = open()?;
    let status_fill = text(
        &tab,
        &format!(
            "(() => {{ const t = [...document.querySelectorAll('{DIAGRAM} text')].find(t => t.textContent.includes('No error')); \
             return t ? t.closest('g').querySelector('rect').getAttribute('fill') : 'missing'; }})()"
        ),
    )?;
    check_eq(status_fill.as_str(), "#ffffff")?;
    for action in ["toggle-0", "toggle-1", "introduce", "remove"] {
        let fill = text(
            &tab,
            &format!("document.querySelector('{DIAGRAM} g[data-action=\"{action}\"] rect').getAttribute('fill')"),
        )?;
        check(
            fill != NOTE && fill != "#ffffff",
            &format!("{action} should have a colour of its own"),
        )?;
    }
    Ok(())
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The error banner stays hidden while redraws work.
#[test]
fn the_error_banner_stays_hidden_while_redraws_work() -> Result<(), String> {
    let tab = open()?;
    press(&tab, "toggle-2")?;
    press(&tab, "introduce")?;
    goto(&tab, 4)?;
    check_eq(
        text(&tab, "(() => { const b = document.getElementById('hamming-window-error'); return b.hidden + '|' + b.textContent; })()")?
            .as_str(),
        "true|",
    )
}
