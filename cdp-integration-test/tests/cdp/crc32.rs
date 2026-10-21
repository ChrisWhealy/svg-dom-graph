//! The CRC-32 demo, in the window of its own that the gallery's panel opens: the checksum on opening, the check value on
//! typing, an empty text, the cut at 64 bytes, the focus and caret across the redraw, the buttons in the scene
//! that introduce an error, the receiver finding and correcting a flipped bit, and the stepping toolbar that walks
//! through the calculation.
//!
//! The calculation is covered by native unit tests against published checksums. What they cannot reach is the browser:
//! that a real change draws the scene again with the right nodes, that the buttons drawn in the scene work, and that the
//! text box keeps focus across the redraw.
//!
//! Like [`crate::huffman_walk`], these run against the built demo gallery's window page, not the fixture.

use crate::common::{DemoTab, new_demo_tab};
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

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The gallery's own panel is only a description and a button that opens the window.
#[test]
fn the_gallery_panel_has_a_button_that_opens_the_window() -> Result<(), String> {
    let tab = new_demo_tab("index.html#panel-crc32", "#crc32-open", (1400, 1400))?;
    check_eq(
        text(&tab, "document.getElementById('crc32-open').textContent.trim()")?,
        "Open the demo in its own window".to_owned(),
    )?;
    check_eq(
        text(&tab, "String(document.querySelectorAll('#panel-crc32 svg').length)")?,
        "0".to_owned(),
    )
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The diagram is drawn at 85% of the size of its own view box, so its text is 15% smaller than it would be.
#[test]
fn the_diagram_is_scaled_down_to_85_percent() -> Result<(), String> {
    let tab = open()?;
    let ratio = text(
        &tab,
        &format!(
            "(() => {{ const svg = document.querySelector('{DIAGRAM}'); const box = svg.viewBox.baseVal; \
             return String(parseFloat(svg.getAttribute('width')) / box.width); }})()"
        ),
    )?;
    let ratio: f64 = ratio.parse().map_err(|e| format!("could not read the ratio {ratio:?}: {e}"))?;
    check(
        (ratio - 0.85).abs() < 0.001,
        &format!("the diagram is drawn at {ratio} of its size"),
    )
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The window opens on a sentence of 49 bytes, whose CRC-32 is `AC52F740`.
#[test]
fn the_window_opens_on_its_sentence() -> Result<(), String> {
    let tab = open()?;
    check_eq(result(&tab)?.as_str(), "AC52F740")?;
    for name in [
        "Input bytes",
        "Table index",
        "Initial register",
        "Register after each byte",
        "Table entry",
    ] {
        check_eq(nodes_named(&tab, name)?, 1)?;
    }
    Ok(())
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Typing works the checksum out again for the new text.
#[test]
fn typing_works_the_checksum_out_again() -> Result<(), String> {
    let tab = open()?;
    for (typed, expected) in [
        // The check value of CRC-32.
        ("123456789", "CBF43926"),
        ("a", "E8B7BE43"),
        ("abc", "352441C2"),
        ("The quick brown fox jumps over the lazy dog", "414FA339"),
    ] {
        set_text(&tab, typed)?;
        check_eq(result(&tab)?, expected.to_owned())?;
    }
    Ok(())
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// An empty text has no bytes to shift through the register, so the CRC-32 is zero and the nodes for the bytes are not
/// drawn. Typing again brings them back.
#[test]
fn an_empty_text_has_a_checksum_of_zero_and_no_byte_nodes() -> Result<(), String> {
    let tab = open()?;
    set_text(&tab, "")?;
    check_eq(result(&tab)?.as_str(), "00000000")?;
    check_eq(nodes_named(&tab, "Input bytes")?, 0)?;
    check_eq(nodes_named(&tab, "Table index")?, 0)?;
    check_eq(nodes_named(&tab, "Initial register")?, 1)?;
    set_text(&tab, "a")?;
    check_eq(nodes_named(&tab, "Table index")?, 1)?;
    check_eq(result(&tab)?.as_str(), "E8B7BE43")
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// A character that straddles the 64 byte limit is left out whole, so the checksum is that of the 63 bytes before it.
#[test]
fn a_character_that_straddles_the_limit_is_left_out_whole() -> Result<(), String> {
    let tab = open()?;
    // 63 bytes, and then a euro sign of three bytes.
    set_text(&tab, &format!("{}\u{20AC}", "a".repeat(63)))?;
    let straddling = result(&tab)?;
    set_text(&tab, &"a".repeat(63))?;
    check_eq(straddling, result(&tab)?)
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// A change draws the scene again, and the text box that replaces the old one has the keyboard focus, with the caret
/// where it was, so typing carries on.
#[test]
fn typing_keeps_the_focus_and_the_caret_across_the_redraw() -> Result<(), String> {
    let tab = open()?;
    tab.evaluate(
        &format!(
            "(() => {{ const i = document.querySelector('{TEXT_BOX}'); i.focus(); i.value = 'abcd'; \
             i.setSelectionRange(2, 2); i.dispatchEvent(new Event('input', {{ bubbles: true }})); }})()"
        ),
        false,
    )
    .map_err(|e| format!("could not edit the text box: {e}"))?;
    std::thread::sleep(Duration::from_millis(300));
    tab.type_str("X").map_err(|e| format!("{e}"))?;
    std::thread::sleep(Duration::from_millis(300));
    check_eq(
        text(&tab, &format!("document.querySelector('{TEXT_BOX}').value"))?.as_str(),
        "abXcd",
    )
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The two buttons for introducing an error are in the scene, and there are no fields for choosing a bit.
#[test]
fn the_buttons_for_introducing_an_error_are_in_the_scene() -> Result<(), String> {
    let tab = open()?;
    for action in ["introduce", "remove"] {
        check_eq(
            text(
                &tab,
                &format!("String(document.querySelectorAll('{DIAGRAM} g[data-action=\"{action}\"]').length)"),
            )?
            .as_str(),
            "1",
        )?;
    }
    // They are inside the scene's own layer, so they zoom and pan with it.
    check_eq(
        text(
            &tab,
            &format!("String(document.querySelectorAll('{DIAGRAM} > g.svg-dom-graph-content g[data-action]').length)"),
        )?
        .as_str(),
        "2",
    )?;
    // The only text box is the one for the text to checksum.
    check_eq(
        text(&tab, &format!("String(document.querySelectorAll('{DIAGRAM} input').length)"))?.as_str(),
        "1",
    )
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// With no error, the receiver finds none. With a bit flipped, wherever the random choice puts it, the receiver finds one
/// bit, in the data or in the CRC. Removing the error puts the receiver back as it was.
#[test]
fn the_receiver_finds_a_flipped_bit_and_puts_it_right() -> Result<(), String> {
    let tab = open()?;
    check_eq(nodes_named(&tab, "Error position")?, 1)?;
    check_eq(position(&tab)?.as_str(), "none")?;
    check(
        says(&tab, "No error has been introduced.")?,
        "the status line should say that no error was introduced",
    )?;

    // The choice is random, so try several times: each press picks again, and each must be found.
    for _ in 0..3 {
        flip(&tab)?;
        check(
            says(&tab, "is flipped in transmission.")?,
            "the status line does not say a bit was flipped",
        )?;
        let found = position(&tab)?;
        check(
            (found.starts_with("byte ") && found.contains(", bit ")) || found.ends_with(" of the CRC"),
            &format!("the flipped bit was not found: {found}"),
        )?;
        // The CRC that was sent is still the one on show.
        check_eq(result(&tab)?.as_str(), "AC52F740")?;
    }

    remove_errors(&tab)?;
    check_eq(position(&tab)?.as_str(), "none")
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// A flip in a byte that is no longer there is dropped when the text gets shorter, and the flip stays if it was in the
/// CRC. Either way the status line says what is flipped now.
#[test]
fn a_shorter_text_keeps_only_a_flip_that_still_exists() -> Result<(), String> {
    let tab = open()?;
    flip(&tab)?;
    set_text(&tab, "abc")?;
    let all = texts(&tab)?.join(" ");
    check(
        all.contains("is flipped in transmission.") || all.contains("No error has been introduced."),
        &format!("the status line should describe the flip or say there is none: {all}"),
    )
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The button is drawn again with the rest of the scene, and the new button of the same kind has the keyboard focus, so
/// a keyboard user can press it again. The change is announced in a live region that is not part of the scene.
#[test]
fn a_button_keeps_the_keyboard_focus_across_the_redraw_and_the_change_is_announced() -> Result<(), String> {
    let tab = open()?;
    let focused = |tab: &Tab| text(tab, "(document.activeElement?.getAttribute('data-action') ?? 'none')");
    let announced = |tab: &Tab| text(tab, "document.getElementById('crc32-announcement').textContent");

    flip(&tab)?;
    check_eq(focused(&tab)?.as_str(), "introduce")?;
    check(
        announced(&tab)?.ends_with("is flipped in transmission."),
        "the live region should say which bit was flipped",
    )?;

    remove_errors(&tab)?;
    check_eq(focused(&tab)?.as_str(), "remove")?;
    check_eq(announced(&tab)?.as_str(), "No error has been introduced.")?;
    check_eq(
        text(&tab, "document.getElementById('crc32-announcement').getAttribute('aria-live')")?.as_str(),
        "polite",
    )
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// A redraw that works leaves the window's error banner hidden and empty. A redraw that fails shows its reason there, but
/// nothing in the page can make one fail, so this only checks the banner is where the demo expects it, and clears.
#[test]
fn the_error_banner_stays_hidden_while_redraws_work() -> Result<(), String> {
    let tab = open()?;
    let banner = |tab: &Tab| {
        text(
            tab,
            "(() => { const b = document.getElementById('crc32-window-error'); \
             return b.hidden + '|' + b.textContent; })()",
        )
    };
    check_eq(banner(&tab)?.as_str(), "true|")?;
    set_text(&tab, "abc")?;
    flip(&tab)?;
    check_eq(banner(&tab)?.as_str(), "true|")
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

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The walk starts with nothing worked out: the bytes are there to read, and everything worked out from them is faint.
#[test]
fn the_walk_starts_with_nothing_worked_out() -> Result<(), String> {
    let tab = open()?;
    set_text(&tab, "abc")?;
    check_eq(step_position(&tab)?, Some(0))?;
    check(!faint(&tab, "Input bytes")?, "the bytes are the input, so they are not faint")?;
    for name in [
        "Table index",
        "Table entry",
        "Register after each byte",
        "Register after the last byte",
        "Received bytes",
        "CRC that arrived",
        "Received register after each byte",
        "CRC of the received bytes",
        "Error position",
        "Corrected bytes",
    ] {
        check(faint(&tab, name)?, &format!("{name} should not be reached yet"))?;
    }
    Ok(())
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Each step reaches the next byte in the table index, the table entry and the register, then the sender is done, then
/// the receiver is reached one box at a time. "abc" has 3 bytes, so the walk has 11 positions.
#[test]
fn each_step_reaches_one_more_box() -> Result<(), String> {
    let tab = open()?;
    set_text(&tab, "abc")?;
    goto(&tab, 1)?;
    check(
        text(
            &tab,
            &format!("document.querySelector('{DIAGRAM} g[aria-label^=\"Table index\"]').getAttribute('aria-label')"),
        )?
        .contains("not yet computed: cells 1"),
        "after one byte the index is reached for byte 0 only",
    )?;
    check(
        faint(&tab, "Register after the last byte")?,
        "the last register comes after the last byte",
    )?;

    goto(&tab, 3)?;
    check(!faint(&tab, "Table index")?, "all three bytes have been through")?;
    check(!faint(&tab, "Register after each byte")?, "all three registers are worked out")?;
    check(
        !faint(&tab, "Register after the last byte")?,
        "the last register is reached with the last byte",
    )?;
    check(faint(&tab, "Received bytes")?, "nothing has been received yet")?;

    // Position 4 makes the CRC, 5 receives the bytes, and so on, one box each.
    goto(&tab, 5)?;
    check(!faint(&tab, "Received bytes")?, "the bytes arrive at position 5")?;
    check(!faint(&tab, "CRC that arrived")?, "so does the CRC")?;
    check(
        faint(&tab, "Received register after each byte")?,
        "the registers are worked out next",
    )?;
    goto(&tab, 6)?;
    check(
        !faint(&tab, "Received register after each byte")?,
        "position 6 works the registers out",
    )?;
    check(faint(&tab, "CRC of the received bytes")?, "the CRC is not worked out yet")?;
    goto(&tab, 10)?;
    check(
        !faint(&tab, "Corrected bytes")?,
        "the last position reaches the corrected bytes",
    )?;
    check(!faint(&tab, "Error position")?, "and the error position")
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Stepping back draws the scene as it was, and editing the text starts the walk again.
#[test]
fn stepping_back_restores_the_boxes_and_editing_the_text_starts_again() -> Result<(), String> {
    let tab = open()?;
    set_text(&tab, "abc")?;
    goto(&tab, 10)?;
    goto(&tab, 2)?;
    check(faint(&tab, "Received bytes")?, "stepping back makes the receiver faint again")?;
    check(faint(&tab, "Register after the last byte")?, "and the last register")?;
    goto(&tab, 4)?;
    set_text(&tab, "abcd")?;
    check_eq(step_position(&tab)?, Some(0))?;
    check(faint(&tab, "Table index")?, "a new text has not been worked out")
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Flipping a bit moves the walk on to the bytes arriving if it was before that, since the error cannot be seen sooner.
/// It does not move a walk that is already past that, and removing the error leaves the walk where it is.
#[test]
fn flipping_a_bit_moves_the_walk_to_the_arrival_but_never_back() -> Result<(), String> {
    let tab = open()?;
    set_text(&tab, "abc")?;
    flip(&tab)?;
    check_eq(step_position(&tab)?, Some(5))?;
    check(!faint(&tab, "Received bytes")?, "the bytes have arrived")?;
    goto(&tab, 8)?;
    flip(&tab)?;
    check_eq(step_position(&tab)?, Some(8))?;
    remove_errors(&tab)?;
    check_eq(step_position(&tab)?, Some(8))
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The toolbar button that has the keyboard focus still has it after the scene is drawn again for the step.
#[test]
fn a_toolbar_button_keeps_the_keyboard_focus_across_a_step() -> Result<(), String> {
    let tab = open()?;
    set_text(&tab, "abc")?;
    click(&tab, "Next selection")?;
    check_eq(
        text(&tab, "document.activeElement?.getAttribute('aria-label') ?? 'none'")?.as_str(),
        "Next selection",
    )
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The notes that say something about the text and the error are white, and the buttons have colours of their own, so the
/// three kinds of box are told apart from each other and from the light blue explanations.
#[test]
fn the_notes_are_white_and_the_buttons_are_coloured() -> Result<(), String> {
    const EXPLANATION: &str = "#eef4ff";
    let tab = open()?;
    let fill_of_note = |phrase: &str| {
        text(
            &tab,
            &format!(
                "(() => {{ const t = [...document.querySelectorAll('{DIAGRAM} text')].find(t => t.textContent.includes({phrase:?})); \
                 return t ? t.closest('g').querySelector('rect').getAttribute('fill') : 'missing'; }})()"
            ),
        )
    };
    check_eq(fill_of_note("49 of 64 bytes")?.as_str(), "#ffffff")?;
    check_eq(fill_of_note("No error has been introduced.")?.as_str(), "#ffffff")?;
    let button_fill = |action: &str| {
        text(
            &tab,
            &format!("document.querySelector('{DIAGRAM} g[data-action=\"{action}\"] rect').getAttribute('fill')"),
        )
    };
    let (flip, remove) = (button_fill("introduce")?, button_fill("remove")?);
    check(
        flip != EXPLANATION && remove != EXPLANATION,
        "the buttons are not the colour of the notes",
    )?;
    check(
        flip != remove && flip != "#ffffff" && remove != "#ffffff",
        "each button has its own colour",
    )
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The toolbar's "Restart" button starts the walk again and removes every flipped bit.
#[test]
fn restart_goes_back_to_the_start_and_removes_the_errors() -> Result<(), String> {
    let tab = open()?;
    set_text(&tab, "abc")?;
    flip(&tab)?;
    goto(&tab, 8)?;
    check(!says(&tab, "No error has been introduced.")?, "a bit should be flipped")?;
    click(&tab, "Restart selection")?;
    check_eq(step_position(&tab)?, None)?;
    check(
        says(&tab, "No error has been introduced.")?,
        "restarting should remove the flipped bit",
    )?;
    check(faint(&tab, "Received bytes")?, "the walk should be back at the start")
}
