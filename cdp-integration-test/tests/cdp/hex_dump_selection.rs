//! The hex dump example in the gallery's Cell selection panel: a named hex dump node whose own selection toolbar is
//! drawn inside its box, and which marks the selected byte twice, by its hex digits and by its character.
//!
//! Like [`crate::huffman_walk`], this runs against the built demo gallery, not the fixture.

use crate::common::{DemoTab, new_demo_tab};
use headless_chrome::Tab;
use std::time::Duration;

const DIAGRAM: &str = "#selection-hexdump-diagram";
/// The fill of the block over the selected byte's hex digits: the primary selection colour.
const PRIMARY: &str = "#ff6b4a";
/// The fill of the block over its character: the secondary selection colour.
const SECONDARY: &str = "#7fd8be";

fn open() -> Result<DemoTab, String> {
    new_demo_tab(
        "index.html#panel-selection",
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

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The toolbar is drawn inside the node, under its name and above its black text area, and not at the scene's edge.
#[test]
fn the_toolbar_is_docked_inside_the_nodes_own_box_between_its_name_and_its_text() -> Result<(), String> {
    let tab = open()?;
    let inside = text(
        &tab,
        &format!(
            "String(document.querySelectorAll('{DIAGRAM} g[aria-label^=\"hexdump\"] g[aria-label=\"Next selection\"]').length)"
        ),
    )?;
    check_eq(inside.as_str(), "1")?;

    let geometry = text(
        &tab,
        &format!(
            "(() => {{ const node = document.querySelector('{DIAGRAM} g[aria-label^=\"hexdump\"]'); \
             const box = node.getBoundingClientRect(); \
             const bar = node.querySelector('g[aria-label=\"Next selection\"]').getBoundingClientRect(); \
             const name = [...node.querySelectorAll('text')].find(t => t.textContent === 'hexdump -C').getBoundingClientRect(); \
             const black = [...node.querySelectorAll('rect')].find(r => r.getAttribute('fill') === '#000000').getBoundingClientRect(); \
             return [name.bottom <= bar.top, bar.bottom <= black.top, bar.left >= box.left && bar.right <= box.right].join(); }})()"
        ),
    )?;
    check_eq(geometry.as_str(), "true,true,true")
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Stepping selects a byte at a time, and marks its hex digits and its character with the primary and secondary colours.
#[test]
fn a_selected_byte_is_marked_by_its_hex_digits_and_by_its_character() -> Result<(), String> {
    let tab = open()?;
    check_eq(status(&tab)?.as_str(), "not started")?;
    check_eq(marks(&tab)?, Vec::<String>::new())?;

    // Byte 13 of the text is the `y` of "by": 0x79.
    press(&tab, "Next selection", 14)?;
    check_eq(status(&tab)?.as_str(), "byte 13 (offset 0000000d), value 79")?;
    check_eq(marks(&tab)?, vec![format!("{PRIMARY}=79"), format!("{SECONDARY}=y")])?;

    // A byte that is not printable is a dot in the character column. Bytes 32 to 37 are 00 01 02 7f fe ff.
    press(&tab, "Next selection", 19)?;
    check_eq(status(&tab)?.as_str(), "byte 32 (offset 00000020), value 00")?;
    check_eq(marks(&tab)?, vec![format!("{PRIMARY}=00"), format!("{SECONDARY}=.")])
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Stepping back moves the marks to the earlier byte, and Restart clears them.
#[test]
fn stepping_back_and_restarting_move_and_clear_the_marks() -> Result<(), String> {
    let tab = open()?;
    press(&tab, "Next selection", 14)?;
    press(&tab, "Previous selection", 1)?;
    check_eq(status(&tab)?.as_str(), "byte 12 (offset 0000000c), value 62")?;
    check_eq(marks(&tab)?, vec![format!("{PRIMARY}=62"), format!("{SECONDARY}=b")])?;

    press(&tab, "Restart selection", 1)?;
    check_eq(status(&tab)?.as_str(), "not started")?;
    check_eq(marks(&tab)?, Vec::<String>::new())
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The marks sit over the glyphs they name, wherever the byte is in its line: at the ends of a line, on either side of the
/// extra gap after the eighth byte, and across the change of line. This is measured against where the browser draws each
/// character, and not against the library's own idea of the columns.
#[test]
fn the_marks_sit_over_the_right_glyphs_at_the_edges_of_a_group_and_of_a_line() -> Result<(), String> {
    use crate::common::{ascii_column, dump_marks, hex_column};
    let tab = open()?;
    let scope = format!("{DIAGRAM} g[aria-label^=\"hexdump\"]");
    // The example has 65 bytes: four full lines and a line of one byte.
    let mut at = 0;
    for byte in [0, 7, 8, 15, 16, 31, 32, 64] {
        press(&tab, "Next selection", byte + 1 - at)?;
        at = byte + 1;
        let (line, place) = (byte / 16, byte % 16);
        let marks = dump_marks(&tab, &scope)?;
        let hex = marks
            .iter()
            .find(|m| m.text.len() == 2)
            .ok_or(format!("byte {byte} has no hex mark"))?;
        let ascii = marks
            .iter()
            .find(|m| m.text.len() == 1)
            .ok_or(format!("byte {byte} has no character mark"))?;
        check_eq(
            (hex.column, hex.line, hex.over_its_glyphs),
            (hex_column(place, true), line, true),
        )?;
        check_eq(
            (ascii.column, ascii.line, ascii.over_its_glyphs),
            (ascii_column(place, true), line, true),
        )?;
    }
    Ok(())
}
