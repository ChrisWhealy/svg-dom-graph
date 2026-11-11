//! The tests of [`super`], which holds the helpers they use.

use super::*;

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The panel opens on the one-byte character, with a button for each of the four, the first pressed, and one column of
/// four nodes.
#[test]
fn the_panel_opens_on_the_one_byte_character_with_four_buttons() -> Result<(), String> {
    let tab = open()?;
    check_eq(result(&tab)?.as_str(), "A U+0041 is 41")?;
    for index in 0..4 {
        let pressed = text(
            &tab,
            &format!(
                "document.querySelector('{DIAGRAM} g[data-action=\"choose-{index}\"]').getAttribute('aria-pressed')"
            ),
        )?;
        check_eq(pressed.as_str(), if index == 0 { "true" } else { "false" })?;
    }
    // "Code point" starts the names of two nodes: the code point, and its bits.
    check_eq(nodes_named(&tab, "Code point")?, 2)?;
    for name in ["Code point bits", "UTF-8 bits", "UTF-8 bytes"] {
        check_eq(nodes_named(&tab, name)?, 1)?;
    }
    Ok(())
}

#[test]
fn a_one_byte_character_has_one_template_bit() -> Result<(), String> {
    encodes_to(0, "41", &["cell 0"])
}

#[test]
fn a_two_byte_character_has_the_template_of_two_bytes() -> Result<(), String> {
    encodes_to(1, "C2 A3", &["cells 0 to 2", "cells 0, 1"])
}

#[test]
fn a_three_byte_character_has_the_template_of_three_bytes() -> Result<(), String> {
    encodes_to(2, "E2 82 AC", &["cells 0 to 3", "cells 0, 1", "cells 0, 1"])
}

#[test]
fn a_four_byte_character_has_the_template_of_four_bytes() -> Result<(), String> {
    encodes_to(3, "F0 9F 98 80", &["cells 0 to 4", "cells 0, 1", "cells 0, 1", "cells 0, 1"])
}

#[test]
fn the_four_nodes_of_a_one_byte_character_share_a_midline() -> Result<(), String> {
    rows_are_centred(0, 1)
}

#[test]
fn the_gaps_of_a_two_byte_character_are_on_the_midline() -> Result<(), String> {
    rows_are_centred(1, 2)
}

#[test]
fn the_middle_nodes_of_a_three_byte_character_share_a_midline() -> Result<(), String> {
    rows_are_centred(2, 3)
}

#[test]
fn the_gaps_of_a_four_byte_character_are_on_the_midline() -> Result<(), String> {
    rows_are_centred(3, 4)
}

#[test]
fn the_buttons_the_note_and_a_one_byte_graph_share_a_midline() -> Result<(), String> {
    shares_a_midline(0)
}

#[test]
fn the_buttons_the_note_and_a_four_byte_graph_share_a_midline() -> Result<(), String> {
    shares_a_midline(3)
}

#[test]
fn a_one_byte_character_fits_inside_the_diagram() -> Result<(), String> {
    fits_inside_the_diagram(0)
}

#[test]
fn a_four_byte_character_fits_inside_the_diagram() -> Result<(), String> {
    fits_inside_the_diagram(3)
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Choosing characters again and again, in an order that goes up and down in length, draws the right scene each time,
/// and the zoom is kept. The scene is drawn from scratch for every choice, so a node, a button or a listener left
/// behind by one would show in the next. After each choice there is exactly one button pressed, the one chosen, the
/// nodes are those of the character, and their cells hold its bits: the bits of the code point in the groups, and every
/// bit of the bytes in the byte nodes. The expected bits are worked out here from the character, not read from the
/// page.
#[test]
fn choosing_characters_again_and_again_draws_each_one_and_keeps_the_zoom() -> Result<(), String> {
    const CHARACTERS: [char; 4] = ['A', '£', '€', '😀'];
    let tab = open()?;
    tab.find_element(&format!("{DIAGRAM} g[aria-label=\"Zoom in\"]"))
        .map_err(|e| format!("could not find Zoom in: {e}"))?
        .click()
        .map_err(|e| format!("could not zoom in: {e}"))?;
    std::thread::sleep(Duration::from_millis(300));
    let zoomed = view(&tab)?;
    check(!zoomed.is_empty(), "zooming in should change the view")?;

    // From one byte to four, down to two, to one, and on, twice round.
    for index in [0, 3, 1, 0, 2, 3, 1, 2, 0, 3] {
        press(&tab, &format!("choose-{index}"))?;
        let c = CHARACTERS[index];
        let mut encoded = [0_u8; 4];
        let bytes = c.encode_utf8(&mut encoded).as_bytes();
        let len = bytes.len();
        let hex = bytes.iter().map(|b| format!("{b:02X}")).collect::<Vec<_>>().join(" ");

        check_eq(pressed(&tab)?, format!("1:choose-{index}"))?;
        check(result(&tab)?.ends_with(&hex), &format!("{c}: the result is not {hex}"))?;
        check_eq(nodes_named(&tab, "UTF-8 bits")?, len)?;
        check_eq(nodes_named(&tab, if len == 1 { "Code point bits" } else { "Bits " })?, len)?;
        check_eq(nodes_named(&tab, "UTF-8 bytes")?, 1)?;
        check_eq(nodes_named(&tab, "Code point:")?, 1)?;
        check_eq(view(&tab)?, zoomed.clone())?;

        // Every bit of the bytes, and the bits of the code point that go into them: 7, 11, 16 or 21.
        let bits: String = bytes.iter().map(|b| format!("{b:08b}")).collect();
        check_eq(digits_of(&tab, "UTF-8 bits")?, bits)?;
        let payload_len = [7, 11, 16, 21][len - 1];
        let payload = format!("{:0payload_len$b}", u32::from(c));
        check_eq(digits_of(&tab, if len == 1 { "Code point bits" } else { "Bits " })?, payload)?;
    }
    Ok(())
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// A button keeps the keyboard focus across the redraw that choosing it makes, and shows as the pressed one.
#[test]
fn a_button_keeps_the_keyboard_focus_across_the_redraw() -> Result<(), String> {
    let tab = open()?;
    tab.evaluate(
        &format!("document.querySelector('{DIAGRAM} g[data-action=\"choose-2\"]').focus()"),
        false,
    )
    .map_err(|e| format!("could not focus the button: {e}"))?;
    tab.press_key("Enter").map_err(|e| format!("could not press Enter: {e}"))?;
    std::thread::sleep(Duration::from_millis(300));
    check_eq(
        text(&tab, "document.activeElement?.getAttribute('data-action') ?? 'none'")?.as_str(),
        "choose-2",
    )?;
    check_eq(
        text(
            &tab,
            &format!("document.querySelector('{DIAGRAM} g[data-action=\"choose-2\"]').getAttribute('aria-pressed')"),
        )?
        .as_str(),
        "true",
    )
}
