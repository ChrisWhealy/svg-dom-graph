//! The tests of [`super`], which holds the helpers they use.

use super::*;

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
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

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
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

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
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

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
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

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
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

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
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

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
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

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// With no error, the receiver finds none. With a bit flipped, wherever the random choice puts it, the receiver finds
/// one bit, in the data or in the CRC. Removing the error puts the receiver back as it was.
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

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
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

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
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

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// A redraw that works leaves the window's error banner hidden and empty. A redraw that fails shows its reason there,
/// but nothing in the page can make one fail, so this only checks the banner is where the demo expects it, and clears.
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

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
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

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
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

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// An explanation is faint until the walk arrives at the box it explains. "abc" has three explanations for the boxes of
/// each byte (index, entry, register), one for the last register, and six for the receiver.
#[test]
fn an_explanation_is_faint_until_the_walk_reaches_its_box() -> Result<(), String> {
    let tab = open()?;
    set_text(&tab, "abc")?;
    check_eq(faint_notes(&tab)?, 10)?;
    goto(&tab, 1)?;
    check_eq(faint_notes(&tab)?, 7)?;
    goto(&tab, 3)?;
    check_eq(faint_notes(&tab)?, 6)?;
    goto(&tab, 10)?;
    check_eq(faint_notes(&tab)?, 0)
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
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

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
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

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
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

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The toolbar's "Restart" button starts the walk again and removes every flipped bit.
#[test]
fn restart_goes_back_to_the_start_and_removes_the_errors() -> Result<(), String> {
    let tab = open()?;
    set_text(&tab, "abc")?;
    flip(&tab)?;
    goto(&tab, 8)?;
    check(!says(&tab, "No error has been introduced.")?, "a bit should be flipped")?;
    click(&tab, "Restart selection")?;
    check_eq(step_position(&tab)?, Some(0))?;
    check(
        says(&tab, "No error has been introduced.")?,
        "restarting should remove the flipped bit",
    )?;
    check(faint(&tab, "Received bytes")?, "the walk should be back at the start")
}
