//! Where the marks over a hex dump's bytes sit, with and without the offsets, and how little a change to a set touches.
//!
//! These run against the fixture, which has the same 65 bytes as two dumps, `dump_off` and `dump_on`, and a large dump,
//! `dump_big`, with every byte unreached.

use crate::common::{ascii_column, dump_marks, hex_column, new_tab};

fn check_eq<T: PartialEq + std::fmt::Debug>(got: T, expected: T) -> Result<(), String> {
    if got == expected {
        Ok(())
    } else {
        Err(format!("expected {expected:?}, got {got:?}"))
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The marks sit over the glyphs they name at the edges of a group of eight, at the edges of a line, across the change
/// of line, and at the last byte, whether or not the dump shows offsets. Measured against where the browser draws each
/// character, and not against the library's own idea of the columns.
#[test]
fn the_marks_sit_over_the_right_glyphs_with_and_without_offsets() -> Result<(), String> {
    let tab = new_tab()?;
    for (name, offsets) in [("dump_off", false), ("dump_on", true)] {
        let scope = format!("#diagram g[aria-label^=\"{name}\"]");
        // 65 bytes: four full lines and a line of one byte.
        for byte in [0, 7, 8, 15, 16, 31, 32, 64] {
            tab.evaluate(&format!("window.fixture.select_dump_byte({name:?}, {byte})"), false)
                .map_err(|e| format!("could not select byte {byte}: {e}"))?;
            let (line, place) = (byte / 16, byte % 16);
            let marks = dump_marks(&tab, &scope)?;
            let hex = marks
                .iter()
                .find(|m| m.text.len() == 2)
                .ok_or(format!("{name}: byte {byte} has no hex mark"))?;
            let ascii = marks
                .iter()
                .find(|m| m.text.len() == 1)
                .ok_or(format!("{name}: byte {byte} has no character mark"))?;
            check_eq(
                (hex.column, hex.line, hex.over_its_glyphs),
                (hex_column(place, offsets), line, true),
            )?;
            check_eq(
                (ascii.column, ascii.line, ascii.over_its_glyphs),
                (ascii_column(place, offsets), line, true),
            )?;
        }
    }
    Ok(())
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Reaching one byte of 256 unreached ones changes only that byte's two blocks in the document. The other 255 are neither
/// moved nor shown again. The page records every change to the node while it happens.
#[test]
fn reaching_one_byte_of_many_changes_only_that_bytes_blocks() -> Result<(), String> {
    let tab = new_tab()?;
    let js = "(() => { \
        const g = document.querySelector('#diagram g[aria-label^=\"dump_big\"]'); \
        const watch = new MutationObserver(() => {}); \
        watch.observe(g, { attributes: true, subtree: true, childList: true, characterData: true }); \
        window.fixture.reach_the_first_byte_of_the_big_dump(); \
        const records = watch.takeRecords(); watch.disconnect(); \
        const rects = new Set(records.filter(r => r.target.tagName === 'rect').map(r => r.target)); \
        return JSON.stringify([records.length, rects.size, \
          records.filter(r => r.attributeName === 'x' || r.attributeName === 'width').length]); })()";
    let json = tab
        .evaluate(js, false)
        .map_err(|e| format!("could not watch the node: {e}"))?
        .value
        .and_then(|v| v.as_str().map(str::to_owned))
        .ok_or("no counts returned")?;
    let [records, rects, moved]: [usize; 3] = serde_json::from_str(&json).map_err(|e| format!("bad counts: {e}"))?;
    // The two blocks of byte 0 are hidden. The node's name and tooltip are rewritten, and that is all.
    check_eq(rects, 2)?;
    check_eq(moved, 0)?;
    if records > 10 {
        return Err(format!("{records} changes were made to the document for one byte"));
    }
    Ok(())
}
