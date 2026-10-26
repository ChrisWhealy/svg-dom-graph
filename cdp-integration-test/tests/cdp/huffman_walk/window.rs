//! The gallery panel that opens the window.

use super::*;

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The gallery's own panel is only a description and a button that opens the window.
#[test]
fn the_gallery_panel_has_a_button_that_opens_the_window() -> Result<(), String> {
    let tab = new_demo_tab("index.html#panel-huffman", "#huffman-open", (1400, 1400))?;
    check_eq(
        text(&tab, "document.getElementById('huffman-open').textContent.trim()")?,
        "Open the demo in its own window".to_owned(),
    )?;
    check_eq(
        text(&tab, "String(document.querySelectorAll('#panel-huffman svg').length)")?,
        "0".to_owned(),
    )
}
