//! What a redraw keeps: the zoom, the keyboard focus, and the caret in the text box.

use super::*;

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The zoom survives a step, which draws the scene again.
#[test]
fn the_zoom_survives_a_redraw() -> Result<(), String> {
    let tab = open()?;
    button(&tab, "Zoom in")?.click().map_err(|e| format!("{e}"))?;
    std::thread::sleep(Duration::from_millis(200));
    let view = |tab: &Tab| {
        text(
            tab,
            &format!(
                "(document.querySelector('{DIAGRAM} > g.svg-dom-graph-content')?.getAttribute('transform') ?? '')"
            ),
        )
    };
    let zoomed = view(&tab)?;
    check(!zoomed.is_empty(), "zooming in did nothing")?;

    click(&tab, "Next selection")?;
    check_eq(view(&tab)?, zoomed)
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The keyboard focus stays on the toolbar button that was pressed, so a keyboard user can press it again.
#[test]
fn keyboard_focus_stays_on_the_pressed_button() -> Result<(), String> {
    let tab = open()?;
    button(&tab, "Next selection")?.focus().map_err(|e| format!("{e}"))?;
    let from = position(&tab)?;
    tab.press_key("Enter").map_err(|e| format!("{e}"))?;
    wait_for_move(&tab, from)?;
    check_eq(position(&tab)?, Some(1))?;
    check_eq(
        text(&tab, "document.activeElement?.getAttribute('aria-label') ?? ''")?,
        "Next selection".to_owned(),
    )?;

    // Pressed again without touching the mouse, it moves on.
    let from = position(&tab)?;
    tab.press_key("Enter").map_err(|e| format!("{e}"))?;
    wait_for_move(&tab, from)?;
    check_eq(position(&tab)?, Some(2))
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Typing in the text box, with a real keyboard, draws the scene again and leaves the keyboard focus and the caret in
/// the new text box, so that the next key goes where the last did.
#[test]
fn typing_keeps_the_focus_and_the_caret_through_each_redraw() -> Result<(), String> {
    let tab = open()?;
    let input = tab
        .find_element(&format!("{DIAGRAM} input"))
        .map_err(|e| format!("no text box: {e}"))?;
    input.click().map_err(|e| format!("{e}"))?;
    tab.press_key("End").map_err(|e| format!("{e}"))?;
    tab.type_str("xyz").map_err(|e| format!("{e}"))?;
    std::thread::sleep(Duration::from_millis(300));

    let value = text(&tab, &format!("document.querySelector('{DIAGRAM} input').value"))?;
    check_eq(value.as_str(), "abracadabraxyz")?;
    check_eq(status(&tab)?, "14 bytes, 8 distinct, 7 merges.".to_owned())?;
    check_eq(text(&tab, "document.activeElement.tagName.toLowerCase()")?, "input".to_owned())?;

    // The caret is where the typing left it, not at the start or the end of a fresh box. Move it, then type.
    tab.press_key("ArrowLeft").map_err(|e| format!("{e}"))?;
    tab.press_key("ArrowLeft").map_err(|e| format!("{e}"))?;
    tab.type_str("!").map_err(|e| format!("{e}"))?;
    std::thread::sleep(Duration::from_millis(300));
    check_eq(
        text(&tab, &format!("document.querySelector('{DIAGRAM} input').value"))?.as_str(),
        "abracadabrax!yz",
    )
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The caret is kept in UTF-16 code units, as the browser counts them. After a character that is two units, such as an
/// emoji, the redrawn text box has the caret just after it, not in the middle of it.
#[test]
fn the_caret_survives_a_redraw_after_a_character_of_two_utf16_units() -> Result<(), String> {
    let tab = open()?;
    // "a", an emoji of two units, then "b", with the caret after the emoji: unit 3.
    text(
        &tab,
        &format!(
            "(() => {{ const i = document.querySelector('{DIAGRAM} input'); i.focus(); \
                i.value = 'a\\u{{1F600}}b'; i.setSelectionRange(3, 3); \
                i.dispatchEvent(new Event('input', {{ bubbles: true }})); return 'ok'; }})()"
        ),
    )?;
    std::thread::sleep(Duration::from_millis(300));
    let caret = text(
        &tab,
        &format!(
            "(() => {{ const i = document.querySelector('{DIAGRAM} input'); \
                return i.selectionStart + ',' + i.selectionEnd + ',' + (document.activeElement === i); }})()"
        ),
    )?;
    check_eq(caret.as_str(), "3,3,true")?;
    tab.type_str("!").map_err(|e| format!("{e}"))?;
    std::thread::sleep(Duration::from_millis(300));
    check_eq(
        text(&tab, &format!("document.querySelector('{DIAGRAM} input').value"))?.as_str(),
        "a\u{1F600}!b",
    )
}
