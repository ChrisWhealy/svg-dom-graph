//! The Base64 panel in the built demo gallery: typing reaches the encoding, both alphabets, the final groups of one and
//! two bytes, an empty text, the cut at 256 bytes falling between characters, and the text box keeping focus and its
//! caret across the redraw that every change makes.
//!
//! The encoding itself is covered by native unit tests against the RFC 4648 vectors. What they cannot reach is the
//! browser: that a real change redraws the scene with the right nodes, that the radio buttons draw it again, and what
//! the redraw keeps.
//!
//! A change redraws the scene, which replaces the text box, and a composition with an input method editor does not
//! survive that. So the demo waits until the composition ends. The last test drives a composition through CDP's own
//! `Input.imeSetComposition`.

use crate::common::{DemoTab, new_demo_tab};
use headless_chrome::Tab;
use std::time::Duration;

const DIAGRAM: &str = "#base64-diagram";

fn open() -> Result<DemoTab, String> {
    new_demo_tab("index.html#panel-base64", &format!("{DIAGRAM} input"), (1800, 2400))
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
            "(() => {{ const i = document.querySelector('{DIAGRAM} input'); i.value = {value:?}; \
             i.dispatchEvent(new Event('input', {{ bubbles: true }})); }})()"
        ),
        false,
    )
    .map_err(|e| format!("could not edit the text box: {e}"))?;
    std::thread::sleep(Duration::from_millis(300));
    Ok(())
}

/// Chooses the alphabet whose radio button has the id `id`.
fn choose(tab: &Tab, id: &str) -> Result<(), String> {
    tab.evaluate(&format!("document.getElementById('{id}').click()"), false)
        .map_err(|e| format!("could not choose {id}: {e}"))?;
    std::thread::sleep(Duration::from_millis(300));
    Ok(())
}

/// The Base64 text on the line under the diagram.
fn result(tab: &Tab) -> Result<String, String> {
    text(tab, "document.getElementById('base64-result').textContent")
}

/// The text of every box in the diagram, in the order they were drawn.
fn texts(tab: &Tab) -> Result<Vec<String>, String> {
    let json = text(
        tab,
        &format!("JSON.stringify([...document.querySelectorAll('{DIAGRAM} text')].map(t => t.textContent))"),
    )?;
    serde_json::from_str(&json).map_err(|e| format!("could not read the box texts: {e}"))
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

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The panel opens on a sentence of 62 bytes: 20 whole groups and a last group of two, so one padding character.
#[test]
fn the_panel_opens_on_its_sentence_with_one_padding_character() -> Result<(), String> {
    let tab = open()?;
    check_eq(
        result(&tab)?.as_str(),
        "VGhpcyBpcyB3aGF0IHNvbWUgdGV4dCBsb29rcyBsaWtlIHdoZW4gZW5jb2RlZCB1c2luZyBCYXNlNjQuLi4=",
    )?;
    check_eq(nodes_named(&tab, "Six-bit values")?, 1)?;
    check_eq(nodes_named(&tab, "Base64 text")?, 1)
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Typing encodes the new text, with the padding that a last group of one byte, of two, and of three needs.
#[test]
fn typing_encodes_the_new_text_with_the_padding_each_final_group_needs() -> Result<(), String> {
    let tab = open()?;
    for (typed, expected) in [
        ("f", "Zg=="),
        ("fo", "Zm8="),
        ("foo", "Zm9v"),
        ("foob", "Zm9vYg=="),
        ("foobar", "Zm9vYmFy"),
    ] {
        set_text(&tab, typed)?;
        check_eq(result(&tab)?, expected.to_owned())?;
    }
    Ok(())
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// An empty text has nothing to encode, so the six-bit values and the Base64 text are not drawn, and typing again brings
/// them back.
#[test]
fn an_empty_text_draws_neither_the_six_bit_values_nor_the_base64_text() -> Result<(), String> {
    let tab = open()?;
    set_text(&tab, "")?;
    check_eq(result(&tab)?.as_str(), "")?;
    check_eq(nodes_named(&tab, "Six-bit values")?, 0)?;
    check_eq(nodes_named(&tab, "Base64 text")?, 0)?;
    check_eq(nodes_named(&tab, "Input bytes")?, 1)?;
    set_text(&tab, "f")?;
    check_eq(nodes_named(&tab, "Six-bit values")?, 1)?;
    check_eq(result(&tab)?.as_str(), "Zg==")
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The URL-safe alphabet writes `-` and `_` where the standard one writes `+` and `/`, for the same text, and choosing
/// either draws the same text again.
#[test]
fn the_url_safe_alphabet_changes_only_the_characters_for_62_and_63() -> Result<(), String> {
    let tab = open()?;
    // "???" is the six-bit values 15, 51, 60 and 63.
    set_text(&tab, "???")?;
    check_eq(result(&tab)?.as_str(), "Pz8/")?;
    choose(&tab, "base64-alphabet-url")?;
    check_eq(result(&tab)?.as_str(), "Pz8_")?;
    check_eq(
        text(&tab, "document.getElementById('base64-alphabet').textContent")?.ends_with("-_"),
        true,
    )?;
    choose(&tab, "base64-alphabet-standard")?;
    check_eq(result(&tab)?.as_str(), "Pz8/")?;
    // The text box was not touched.
    check_eq(
        text(&tab, &format!("document.querySelector('{DIAGRAM} input').value"))?.as_str(),
        "???",
    )
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Text that is one byte too long for the limit because its last character takes three bytes is cut before that
/// character, not inside it, and the note beside the text box says so.
#[test]
fn a_character_that_straddles_the_256_byte_limit_is_left_out_whole() -> Result<(), String> {
    let tab = open()?;
    // 255 bytes and then a euro sign of three: 258 bytes, of which only 255 can be encoded without splitting it.
    set_text(&tab, &format!("{}\u{20AC}", "a".repeat(255)))?;
    // 255 bytes are 85 groups of three, which is 340 characters, with no padding.
    check_eq(result(&tab)?.len(), 340)?;
    check(!result(&tab)?.contains('='), "255 bytes need no padding")?;
    let note = texts(&tab)?.join(" ");
    check(
        note.contains("258 bytes typed"),
        &format!("the note does not say how much was typed: {note}"),
    )?;
    check(
        note.contains("first 255"),
        &format!("the note does not say how much was encoded: {note}"),
    )?;
    // 256 bytes exactly are all encoded, with the padding that 256 = 85 * 3 + 1 needs.
    set_text(&tab, &"a".repeat(256))?;
    check(result(&tab)?.ends_with("YQ=="), "256 bytes end in a group of one byte")?;
    let note = texts(&tab)?.join(" ");
    check(
        note.contains("256 of 256 bytes"),
        &format!("the note does not give the length: {note}"),
    )
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// A change redraws the whole scene, and the text box that replaces the old one has the keyboard focus, with the caret
/// where it was, so typing carries on.
#[test]
fn typing_keeps_the_focus_and_the_caret_across_the_redraw() -> Result<(), String> {
    let tab = open()?;
    tab.evaluate(
        &format!(
            "(() => {{ const i = document.querySelector('{DIAGRAM} input'); i.focus(); i.value = 'abcd'; \
             i.setSelectionRange(2, 2); i.dispatchEvent(new Event('input', {{ bubbles: true }})); }})()"
        ),
        false,
    )
    .map_err(|e| format!("could not edit the text box: {e}"))?;
    std::thread::sleep(Duration::from_millis(300));
    let state = text(
        &tab,
        &format!(
            "(() => {{ const i = document.querySelector('{DIAGRAM} input'); \
             return i.selectionStart + ',' + i.selectionEnd + ',' + (document.activeElement === i); }})()"
        ),
    )?;
    check_eq(state.as_str(), "2,2,true")?;
    tab.type_str("X").map_err(|e| format!("{e}"))?;
    std::thread::sleep(Duration::from_millis(300));
    check_eq(
        text(&tab, &format!("document.querySelector('{DIAGRAM} input').value"))?.as_str(),
        "abXcd",
    )?;
    check_eq(result(&tab)?.as_str(), "YWJYY2Q=")
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// While text is being composed, the scene is not drawn again, so the text box is the same element and the composition
/// goes on. When the composition is committed, the scene is drawn for the composed text and the text box has the focus.
#[test]
fn a_composition_in_progress_is_not_interrupted_by_a_redraw() -> Result<(), String> {
    use headless_chrome::protocol::cdp::Input::{ImeSetComposition, InsertText};

    let tab = open()?;
    let before = result(&tab)?;
    // Put the caret at the end of the text and remember which element is the text box.
    tab.evaluate(
        &format!(
            "(() => {{ const i = document.querySelector('{DIAGRAM} input'); i.focus(); \
             i.setSelectionRange(i.value.length, i.value.length); window.composingBox = i; }})()"
        ),
        false,
    )
    .map_err(|e| format!("could not focus the text box: {e}"))?;

    tab.call_method(ImeSetComposition {
        text: "\u{306B}\u{307B}".to_owned(),
        selection_start: 2,
        selection_end: 2,
        replacement_start: None,
        replacement_end: None,
    })
    .map_err(|e| format!("could not start the composition: {e}"))?;
    std::thread::sleep(Duration::from_millis(400));
    let still_there = text(
        &tab,
        &format!(
            "String(window.composingBox.isConnected && \
             document.querySelector('{DIAGRAM} input') === window.composingBox)"
        ),
    )?;
    check_eq(still_there.as_str(), "true")?;
    check_eq(result(&tab)?, before.clone())?;

    tab.call_method(InsertText {
        text: "\u{65E5}\u{672C}".to_owned(),
    })
    .map_err(|e| format!("could not commit the composition: {e}"))?;
    std::thread::sleep(Duration::from_millis(400));
    let value = text(&tab, &format!("document.querySelector('{DIAGRAM} input').value"))?;
    check(
        value.ends_with("\u{65E5}\u{672C}"),
        &format!("the composed text is not in the box: {value:?}"),
    )?;
    check(result(&tab)? != before, "the scene was not drawn for the composed text")?;
    check_eq(
        text(
            &tab,
            &format!("String(document.activeElement === document.querySelector('{DIAGRAM} input'))"),
        )?
        .as_str(),
        "true",
    )
}
