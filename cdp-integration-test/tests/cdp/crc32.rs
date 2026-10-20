//! The CRC-32 demo, in the window of its own that the gallery's panel opens: the checksum on opening, the check value on
//! typing, an empty text, the cut at 64 bytes, the focus and caret across the redraw, the fields and buttons in the scene
//! that introduce an error, and the receiver finding and correcting a flipped bit.
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
/// The text box that holds the text to checksum. The two fields for choosing a bit are text boxes too.
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

/// Types `byte` and `bit` into the two fields in the scene, exactly as given, then presses the button that introduces the
/// error.
fn introduce_text(tab: &Tab, byte: &str, bit: &str) -> Result<(), String> {
    tab.evaluate(
        &format!(
            "(() => {{ for (const [label, value] of [['Byte', {byte:?}], ['Bit (0 to 7)', {bit:?}]]) {{ \
                 const i = document.querySelector('{DIAGRAM} g[aria-label^=\"' + label + '\"] input'); \
                 i.value = value; i.dispatchEvent(new Event('input', {{ bubbles: true }})); }} \
             document.querySelector('{DIAGRAM} g[data-action=\"introduce\"]') \
                 .dispatchEvent(new MouseEvent('click', {{ bubbles: true }})); }})()"
        ),
        false,
    )
    .map_err(|e| format!("could not introduce the error: {e}"))?;
    std::thread::sleep(Duration::from_millis(400));
    Ok(())
}

/// Chooses bit `bit` of byte `byte` in the two fields in the scene, then presses the button that introduces the error.
fn introduce(tab: &Tab, byte: usize, bit: u8) -> Result<(), String> {
    introduce_text(tab, &byte.to_string(), &bit.to_string())
}

/// The fill of the box of the field called `label`, and whether its text box is marked invalid.
fn field_marks(tab: &Tab, label: &str) -> Result<(String, String), String> {
    let marks = text(
        tab,
        &format!(
            "(() => {{ const g = document.querySelector('{DIAGRAM} g[aria-label^=\"{label}\"]'); \
             return (g.querySelector('rect').getAttribute('fill') ?? '') + '|' + \
                    (g.querySelector('input').getAttribute('aria-invalid') ?? ''); }})()"
        ),
    )?;
    let (fill, invalid) = marks.split_once('|').ok_or("no marks returned")?;
    Ok((fill.to_owned(), invalid.to_owned()))
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
/// The fields and buttons for introducing an error are in the scene, and the fields keep what was typed in them when the
/// scene is drawn again.
#[test]
fn the_controls_for_introducing_an_error_are_in_the_scene() -> Result<(), String> {
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
    check_eq(nodes_named(&tab, "Byte")?, 1)?;
    check_eq(nodes_named(&tab, "Bit (0 to 7)")?, 1)?;
    // They are inside the scene's own layer, so they zoom and pan with it.
    check_eq(
        text(
            &tab,
            &format!("String(document.querySelectorAll('{DIAGRAM} > g.svg-dom-graph-content g[data-action]').length)"),
        )?
        .as_str(),
        "2",
    )?;
    introduce(&tab, 2, 5)?;
    check_eq(
        text(
            &tab,
            &format!("document.querySelector('{DIAGRAM} g[aria-label^=\"Byte\"] input').value"),
        )?
        .as_str(),
        "2",
    )
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// With no error, the receiver finds none. With one bit flipped, it finds that bit, and flipping it back gives the bytes
/// that were sent. Removing the error puts the receiver back as it was.
#[test]
fn the_receiver_finds_a_flipped_bit_and_puts_it_right() -> Result<(), String> {
    let tab = open()?;
    check_eq(nodes_named(&tab, "Error position")?, 1)?;
    check(
        texts(&tab)?.iter().any(|t| t == "none"),
        "no error should be found when none was introduced",
    )?;
    check(
        says(&tab, "No error has been introduced.")?,
        "the status line should say that no error was introduced",
    )?;

    introduce(&tab, 2, 5)?;
    check(
        says(&tab, "Bit 5 of byte 2 is flipped in transmission.")?,
        "the status line does not say which bit",
    )?;
    let all = texts(&tab)?;
    check(
        all.iter().any(|t| t == "byte 2, bit 5"),
        &format!("the flipped bit was not found: {all:?}"),
    )?;
    // The CRC that was sent is still the one on show.
    check_eq(result(&tab)?.as_str(), "AC52F740")?;

    tab.evaluate(
        &format!(
            "document.querySelector('{DIAGRAM} g[data-action=\"remove\"]') \
             .dispatchEvent(new MouseEvent('click', {{ bubbles: true }}))"
        ),
        false,
    )
    .map_err(|e| format!("could not remove the error: {e}"))?;
    std::thread::sleep(Duration::from_millis(400));
    check(
        texts(&tab)?.iter().any(|t| t == "none"),
        "removing the error should leave none to find",
    )
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// An error in a byte that is no longer there is dropped when the text gets shorter.
#[test]
fn an_error_in_a_byte_that_goes_away_is_dropped() -> Result<(), String> {
    let tab = open()?;
    introduce(&tab, 8, 0)?;
    set_text(&tab, "abc")?;
    check(
        says(&tab, "No error has been introduced.")?,
        "the error should have been dropped with its byte",
    )
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The buttons are drawn again with the rest of the scene, and the new button of the same kind has the keyboard focus, so
/// a keyboard user can press it again. The change is announced in a live region that is not part of the scene.
#[test]
fn a_button_keeps_the_keyboard_focus_across_the_redraw_and_the_change_is_announced() -> Result<(), String> {
    let tab = open()?;
    let focused = |tab: &Tab| text(tab, "(document.activeElement?.getAttribute('data-action') ?? 'none')");
    let announced = |tab: &Tab| text(tab, "document.getElementById('crc32-announcement').textContent");

    introduce(&tab, 3, 1)?;
    check_eq(focused(&tab)?.as_str(), "introduce")?;
    check_eq(announced(&tab)?.as_str(), "Bit 1 of byte 3 is flipped in transmission.")?;

    tab.evaluate(
        &format!(
            "document.querySelector('{DIAGRAM} g[data-action=\"remove\"]') \
             .dispatchEvent(new MouseEvent('click', {{ bubbles: true }}))"
        ),
        false,
    )
    .map_err(|e| format!("could not remove the error: {e}"))?;
    std::thread::sleep(Duration::from_millis(400));
    check_eq(focused(&tab)?.as_str(), "remove")?;
    check_eq(announced(&tab)?.as_str(), "No error has been introduced.")?;
    check_eq(
        text(&tab, "document.getElementById('crc32-announcement').getAttribute('aria-live')")?.as_str(),
        "polite",
    )
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// A field that is not a whole number, or is out of range, is checked when the button is pressed. It is marked in the
/// colour for what is wrong with it, and no other field is. The status line and the live region say which field and what
/// is wrong, and no bit is flipped.
#[test]
fn a_field_that_is_wrong_is_marked_in_the_colour_for_what_is_wrong_and_nothing_is_flipped() -> Result<(), String> {
    const NOT_A_NUMBER: &str = "#f5b7b1";
    const OUT_OF_RANGE: &str = "#fad7a0";
    let tab = open()?;
    let (before, _) = field_marks(&tab, "Bit (0 to 7)")?;

    // Letters in the bit field, and a byte past the end of the 49 bytes.
    introduce_text(&tab, "70", "abc")?;
    let (bit_fill, bit_invalid) = field_marks(&tab, "Bit (0 to 7)")?;
    let (byte_fill, byte_invalid) = field_marks(&tab, "Byte")?;
    check_eq(bit_fill, NOT_A_NUMBER.to_owned())?;
    check_eq(bit_invalid, "true".to_owned())?;
    check_eq(byte_fill, OUT_OF_RANGE.to_owned())?;
    check_eq(byte_invalid, "true".to_owned())?;
    check(
        says(&tab, "Bit: \"abc\" is not a whole number")?,
        "the status line does not say what is wrong with the bit",
    )?;
    check(
        says(&tab, "Byte: 70 is more than 48")?,
        "the status line does not say what is wrong with the byte",
    )?;
    check(texts(&tab)?.iter().any(|t| t == "none"), "no bit should have been flipped")?;
    check(
        text(&tab, "document.getElementById('crc32-announcement').textContent")?.contains("not a whole number"),
        "the live region does not say what is wrong",
    )?;

    // A number that is too big is marked in the other colour, and a field that is right is not marked.
    introduce_text(&tab, "3", "9")?;
    check_eq(field_marks(&tab, "Bit (0 to 7)")?.0, OUT_OF_RANGE.to_owned())?;
    let (byte_fill, byte_invalid) = field_marks(&tab, "Byte")?;
    check_eq(byte_invalid, String::new())?;
    check(
        byte_fill != NOT_A_NUMBER && byte_fill != OUT_OF_RANGE,
        "the byte is right, so it must not be marked",
    )?;

    // Editing a marked field takes its mark off at once, and the other mark stays.
    set_field(&tab, "Bit (0 to 7)", "5")?;
    let (fill, invalid) = field_marks(&tab, "Bit (0 to 7)")?;
    check_eq(fill, before)?;
    check_eq(invalid, String::new())?;

    // Right values flip the bit and leave no marks.
    introduce_text(&tab, "3", "5")?;
    check(
        says(&tab, "Bit 5 of byte 3 is flipped in transmission.")?,
        "the bit was not flipped",
    )?;
    check_eq(field_marks(&tab, "Bit (0 to 7)")?.1, String::new())?;
    check_eq(field_marks(&tab, "Byte")?.1, String::new())
}

/// Types `value` into the field called `label` in the scene.
fn set_field(tab: &Tab, label: &str, value: &str) -> Result<(), String> {
    tab.evaluate(
        &format!(
            "(() => {{ const i = document.querySelector('{DIAGRAM} g[aria-label^=\"{label}\"] input'); \
             i.value = {value:?}; i.dispatchEvent(new Event('input', {{ bubbles: true }})); }})()"
        ),
        false,
    )
    .map_err(|e| format!("could not edit the {label} field: {e}"))?;
    std::thread::sleep(Duration::from_millis(200));
    Ok(())
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
    introduce(&tab, 1, 1)?;
    check_eq(banner(&tab)?.as_str(), "true|")
}
