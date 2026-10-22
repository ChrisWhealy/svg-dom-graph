//! The Hamming(7,4) demo in the window the gallery's panel opens: the path of boxes laid out as a block, the buttons
//! that choose the data and flip a bit, and the stepping toolbar that walks along the path.
//!
//! The code itself and the layout arithmetic are covered by native unit tests. What they cannot reach is the browser:
//! that the scene is drawn with the nine boxes where the layout says, that the buttons drawn in the scene work, and
//! that each step makes the right boxes faint.
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

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
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

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// A data bit's button flips it, says so in its name and its pressed state, and keeps the keyboard focus across the
/// redraw.
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

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
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

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
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
        // The box draws its title first and its one value last.
        let found = text(
            &tab,
            &format!(
                "[...document.querySelector('{DIAGRAM} g[aria-label^=\"Error position\"]').querySelectorAll('text')].at(-1).textContent"
            ),
        )?;
        check_eq(found.as_str(), status.as_str())?;
        check(says(&tab, "They are the data that was sent.")?, "the data should be recovered")?;
        goto(&tab, 0)?;
    }
    Ok(())
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
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

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The toolbar's "Restart" starts the walk again and removes the flipped bit, but keeps the data bits as they were set.
#[test]
fn restart_goes_back_to_the_start_and_removes_the_error_but_keeps_the_data() -> Result<(), String> {
    let tab = open()?;
    press(&tab, "toggle-1")?;
    press(&tab, "introduce")?;
    goto(&tab, 5)?;
    click(&tab, "Restart selection")?;
    check_eq(step_position(&tab)?, Some(0))?;
    check(
        says(&tab, "No error has been introduced.")?,
        "restarting should remove the flipped bit",
    )?;
    check(faint(&tab, "Received codeword")?, "the walk should be back at the start")?;
    check_eq(label_of(&tab, "toggle-1")?.as_str(), "d2 = 1")
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Defines one test for each position listed, calling `$run` with it. Each is a test of its own, with its own tab, so
/// the CI deals them out to different shards, instead of one long test that walks every position in a single shard.
macro_rules! at_each_position {
    ($run:ident: $($name:ident => $position:expr),+ $(,)?) => {
        $(
            #[test]
            fn $name() -> Result<(), String> {
                $run($position)
            }
        )+
    };
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

at_each_position! { announces:
    the_walk_announces_step_2 => 1,
    the_walk_announces_step_3 => 2,
    the_walk_announces_step_4 => 3,
    the_walk_announces_step_5 => 4,
    the_walk_announces_step_6 => 5,
    the_walk_announces_step_7 => 6,
    the_walk_announces_step_8 => 7,
    the_walk_announces_step_9 => 8,
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

at_each_position! { notes_follow_the_walk:
    the_notes_at_position_0_follow_the_walk => 0,
    the_notes_at_position_1_follow_the_walk => 1,
    the_notes_at_position_2_follow_the_walk => 2,
    the_notes_at_position_3_follow_the_walk => 3,
    the_notes_at_position_4_follow_the_walk => 4,
    the_notes_at_position_5_follow_the_walk => 5,
    the_notes_at_position_6_follow_the_walk => 6,
    the_notes_at_position_7_follow_the_walk => 7,
    the_notes_at_position_8_follow_the_walk => 8,
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

at_each_position! { restart_from:
    restart_from_position_0_returns_to_the_start => 0,
    restart_from_position_1_returns_to_the_start => 1,
    restart_from_position_2_returns_to_the_start => 2,
    restart_from_position_3_returns_to_the_start => 3,
    restart_from_position_4_returns_to_the_start => 4,
    restart_from_position_5_returns_to_the_start => 5,
    restart_from_position_6_returns_to_the_start => 6,
    restart_from_position_7_returns_to_the_start => 7,
    restart_from_position_8_returns_to_the_start => 8,
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Restart makes one announcement that says both that the error is gone and that the walk is back at the first step.
/// Two announcements made together would leave a screen reader saying only the second.
#[test]
fn restart_announces_the_removed_error_and_the_first_step_together() -> Result<(), String> {
    let tab = open_with("?error=3")?;
    goto(&tab, 5)?;
    click(&tab, "Restart selection")?;
    let said = text(&tab, "document.getElementById('hamming-announcement').textContent")?;
    check(
        said.contains("No error has been introduced.") && said.contains("Step 1 of 9: Data bits"),
        &format!("restart announced {said:?}"),
    )
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// "Previous selection" at position 0 has nowhere earlier to go, and the toolbar reports it exactly as it reports
/// "Restart". So the walk stays at the start, a flipped bit is removed as a restart removes it, the data stays, and
/// nothing fails.
#[test]
fn previous_at_position_zero_is_a_restart() -> Result<(), String> {
    let tab = open_with("?error=5")?;
    check(
        says(&tab, "The bit at position 5 is flipped in transmission.")?,
        "the page should open with a flipped bit",
    )?;
    click_and_settle(&tab, "Previous selection")?;
    check_eq(step_position(&tab)?, Some(0))?;
    check(
        says(&tab, "No error has been introduced.")?,
        "stepping back from the start restarts, which removes the flipped bit",
    )?;
    check(!faint(&tab, "Data bits")?, "the data should still be there")?;
    check(faint(&tab, "Parity bits")?, "nothing else should be reached")?;
    check_eq(
        text(&tab, "String(document.getElementById('hamming-window-error').hidden)")?.as_str(),
        "true",
    )
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

at_each_position! { finds_error_at:
    the_receiver_finds_an_error_at_position_1 => 1,
    the_receiver_finds_an_error_at_position_2 => 2,
    the_receiver_finds_an_error_at_position_3 => 3,
    the_receiver_finds_an_error_at_position_4 => 4,
    the_receiver_finds_an_error_at_position_5 => 5,
    the_receiver_finds_an_error_at_position_6 => 6,
    the_receiver_finds_an_error_at_position_7 => 7,
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Stepping back from a box that shows the error to one before it takes the mark away, since a mark is drawn at full
/// strength even in a faint box, and would give away what the walk has not reached.
#[test]
fn stepping_back_takes_the_error_marks_away_from_boxes_not_reached() -> Result<(), String> {
    let tab = open_with("?error=5")?;
    goto(&tab, LAST)?;
    check(
        box_label(&tab, "Received codeword")?.contains("also highlighted"),
        "marked once reached",
    )?;
    check(
        box_label(&tab, "Corrected codeword")?.contains("also highlighted"),
        "marked once reached",
    )?;
    goto(&tab, 6)?;
    check(
        !box_label(&tab, "Corrected codeword")?.contains("also highlighted"),
        "the corrected codeword is not reached, so shows no mark",
    )?;
    check(
        box_label(&tab, "Received codeword")?.contains("also highlighted"),
        "the received codeword is reached, so keeps its mark",
    )?;
    goto(&tab, 2)?;
    check(
        !box_label(&tab, "Received codeword")?.contains("also highlighted"),
        "the received codeword is not reached, so shows no mark",
    )
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The zoom stays through redraws of every kind: a data bit changed, an error introduced and removed, a step forward
/// and back, and a restart.
#[test]
fn the_zoom_survives_every_kind_of_redraw() -> Result<(), String> {
    let tab = open()?;
    click_and_settle(&tab, "Zoom in")?;
    let zoomed = view(&tab)?;
    check(!zoomed.is_empty(), "zooming in should change the view")?;
    press(&tab, "toggle-0")?;
    press(&tab, "introduce")?;
    press(&tab, "remove")?;
    goto(&tab, 5)?;
    goto(&tab, 1)?;
    click(&tab, "Restart selection")?;
    check_eq(view(&tab)?, zoomed)
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
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
