//! The tests of [`super`], which holds the helpers they use.

use super::*;

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
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

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
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

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
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

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
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
