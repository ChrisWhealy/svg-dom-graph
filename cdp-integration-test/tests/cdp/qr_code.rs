//! The QR code demo in the window the gallery's panel opens: the graph of the stages, the matrix of 21 by 21 modules,
//! the 59 steps, the buttons that jump to a stage, the list of roles, and the text node.
//!
//! The encoder, the snapshots and the path data are covered by native unit tests, and the encoder is compared with two
//! independent ones. What they cannot reach is the browser: that the scene is built once and only written to after
//! that, that going back shows exactly what going forward showed, that choosing a role and stepping stay independent,
//! that the toolbar and the buttons work from the keyboard, that zooming leaves the matrix alone, that typing starts
//! the walk again, that text which cannot be encoded leaves the symbol on show alone, and that the live region and the
//! matrix as text follow each step.

use crate::common::{DemoTab, new_demo_tab};
use headless_chrome::Tab;
use std::time::Duration;

const DIAGRAM: &str = "#qr-code-diagram";
const LAYER: &str = "#qr-code-diagram > g.svg-dom-graph-content";

/// The number of steps, and what each stage's first step is.
const STEPS: usize = 62;
const RS: usize = 7;
const FUNCTIONS: usize = 26;
const PLACEMENT: usize = 27;
const MASKS: usize = 53;
const FINAL: usize = 61;

/// The text the window opens with, which is 17 characters long, and so as long as a text can be.
const DEFAULT_TEXT: &str = "This is a QR code";

fn open() -> Result<DemoTab, String> {
    new_demo_tab(
        "qr-code-window.html",
        &format!("{DIAGRAM} g[data-action=\"jump-0\"]"),
        (2400, 2400),
    )
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

/// The attribute `name` of the first element that matches `selector`.
fn attribute(tab: &Tab, selector: &str, name: &str) -> Result<String, String> {
    text(
        tab,
        &format!("document.querySelector('{selector}').getAttribute('{name}') ?? ''"),
    )
}

/// The `d` of one of the matrix's paths, which is `base` for the three colours, `role` and `highlight`.
fn path(tab: &Tab, which: &str) -> Result<String, String> {
    attribute(tab, &format!("#qr-code-{which}"), "d")
}

/// The three colour paths together, which change whenever any module's colour does.
fn colours(tab: &Tab) -> Result<String, String> {
    Ok(["unassigned", "light", "dark"]
        .iter()
        .map(|which| path(tab, which))
        .collect::<Result<Vec<_>, _>>()?
        .join(" | "))
}

/// How many rectangles, outlines or modules a path draws. Every one of them starts with `M`.
fn parts(path: &str) -> usize {
    path.matches('M').count()
}

/// Clicks the element that matches `selector` with a real mouse click, and waits for the redraw.
fn click(tab: &Tab, selector: &str) -> Result<(), String> {
    tab.find_element(selector)
        .map_err(|e| format!("could not find {selector}: {e}"))?
        .click()
        .map_err(|e| format!("could not click {selector}: {e}"))?;
    std::thread::sleep(Duration::from_millis(120));
    Ok(())
}

/// Presses the button drawn in the scene with `action`.
fn press(tab: &Tab, action: &str) -> Result<(), String> {
    click(tab, &format!("{DIAGRAM} g[data-action=\"{action}\"]"))
}

/// Presses a button of the stepping toolbar by its accessible name: `Next selection`, `Previous selection` or
/// `Restart selection`.
fn toolbar(tab: &Tab, name: &str) -> Result<(), String> {
    click(tab, &format!("{DIAGRAM} g[aria-label=\"{name}\"]"))
}

/// Presses Next `count` times.
fn next(tab: &Tab, count: usize) -> Result<(), String> {
    for _ in 0..count {
        toolbar(tab, "Next selection")?;
    }
    Ok(())
}

/// Goes to snapshot `index` by the button that jumps to it, which must be the first of a stage.
fn jump(tab: &Tab, index: usize) -> Result<(), String> {
    press(tab, &format!("jump-{index}"))
}

/// Everything the matrix shows: its three colours, the outline of the role, the highlight and its accessible name.
fn shown(tab: &Tab) -> Result<Vec<String>, String> {
    let mut all = Vec::new();
    for which in ["unassigned", "light", "dark", "role", "highlight"] {
        all.push(path(tab, which)?);
    }
    all.push(attribute(tab, "#qr-code-matrix", "aria-label")?);
    all.push(said(tab)?);
    Ok(all)
}

/// How many modules are dark, light and uncoloured, read from the matrix's accessible name.
fn counts(tab: &Tab) -> Result<(usize, usize, usize), String> {
    let found = text(
        tab,
        "(() => { const m = document.getElementById('qr-code-matrix').getAttribute('aria-label') \
         .match(/(\\d+) are dark, (\\d+) are light and (\\d+) have no colour/); \
         return m ? m.slice(1).join(',') : ''; })()",
    )?;
    let numbers: Vec<usize> = found.split(',').filter_map(|n| n.parse().ok()).collect();
    match numbers[..] {
        [dark, light, none] => Ok((dark, light, none)),
        _ => Err(format!("the accessible name of the matrix has no counts: {found:?}")),
    }
}

/// Whether a button, in the scene or on the toolbar, is disabled, as assistive technology is told.
fn disabled(tab: &Tab, selector: &str) -> Result<bool, String> {
    Ok(attribute(tab, selector, "aria-disabled")? == "true")
}

/// Whether the toolbar button with this accessible name is disabled.
fn toolbar_disabled(tab: &Tab, name: &str) -> Result<bool, String> {
    disabled(tab, &format!("{DIAGRAM} g[aria-label=\"{name}\"]"))
}

/// Whether the button `action` is pressed.
fn pressed(tab: &Tab, action: &str) -> Result<bool, String> {
    Ok(attribute(tab, &format!("{DIAGRAM} g[data-action=\"{action}\"]"), "aria-pressed")? == "true")
}

/// What the live region last said.
fn said(tab: &Tab) -> Result<String, String> {
    text(tab, "document.getElementById('qr-code-announcement').textContent")
}

/// The accessible name of the node whose name starts with `name`, which says which of its cells are not yet computed.
fn node(tab: &Tab, name: &str) -> Result<String, String> {
    attribute(tab, &format!("{DIAGRAM} g[aria-label^=\"{name}\"]"), "aria-label")
}

/// All the text in the scene, a block at a time, with the lines of each block joined by spaces and the unused lines of
/// a block left off.
fn scene_text(tab: &Tab) -> Result<String, String> {
    text(
        tab,
        &format!(
            "[...document.querySelectorAll('{LAYER} text')] \
             .map(t => [...t.querySelectorAll('tspan')].map(s => s.textContent).join(' ').trim()).join(' | ')"
        ),
    )
}

/// Replaces the text in the text node with `value`, as typing would, and waits for the scene to take it in.
fn type_text(tab: &Tab, value: &str) -> Result<(), String> {
    tab.evaluate(
        &format!(
            "(() => {{ const i = document.querySelector('{DIAGRAM} input'); i.focus(); i.value = {value:?}; \
             i.dispatchEvent(new Event('input', {{ bubbles: true }})); }})()"
        ),
        false,
    )
    .map_err(|e| format!("could not edit the text box: {e}"))?;
    std::thread::sleep(Duration::from_millis(300));
    Ok(())
}

/// What the text node holds now.
fn input_value(tab: &Tab) -> Result<String, String> {
    text(tab, &format!("document.querySelector('{DIAGRAM} input').value"))
}

/// Where the caret is in the text node, as the start and the end of the selection.
fn input_caret(tab: &Tab) -> Result<(u32, u32), String> {
    let found = text(
        tab,
        &format!(
            "(() => {{ const i = document.querySelector('{DIAGRAM} input'); \
             return i.selectionStart + ',' + i.selectionEnd; }})()"
        ),
    )?;
    let (start, end) = found.split_once(',').ok_or_else(|| format!("no caret in {found:?}"))?;
    Ok((
        start.parse().map_err(|e| format!("{e}"))?,
        end.parse().map_err(|e| format!("{e}"))?,
    ))
}

/// The error banner: its text, or an empty string while it is hidden.
fn banner(tab: &Tab) -> Result<String, String> {
    text(
        tab,
        "(() => { const b = document.getElementById('qr-code-error'); return b.hidden ? '' : b.textContent; })()",
    )
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The window opens before the first step. The matrix is empty, so every one of its 441 modules is uncoloured, and it
/// is six elements. Prev and Restart are disabled, and the nodes after the input are dimmed.
#[test]
fn the_window_opens_unstarted_with_an_empty_matrix_in_six_elements() -> Result<(), String> {
    let tab = open()?;
    check_eq(counts(&tab)?, (0, 0, 441))?;
    check_eq(parts(&path(&tab, "unassigned")?), 21)?;
    for which in ["light", "dark", "role", "highlight"] {
        check_eq(path(&tab, which)?.as_str(), "")?;
    }
    check_eq(
        text(&tab, "String(document.querySelectorAll('#qr-code-matrix *').length)")?.as_str(),
        "6",
    )?;
    check_eq(said(&tab)?.as_str(), "Not started. Every module is empty.")?;
    check_eq(
        (
            toolbar_disabled(&tab, "Previous selection")?,
            toolbar_disabled(&tab, "Restart selection")?,
            toolbar_disabled(&tab, "Next selection")?,
        ),
        (true, true, false),
    )?;
    check(
        node(&tab, "Data codewords")?.contains("not yet computed"),
        "the data codewords are not yet computed",
    )?;
    check_eq(
        text(&tab, &format!("document.querySelector('{DIAGRAM} input').value"))?.as_str(),
        DEFAULT_TEXT,
    )?;
    check_eq(DEFAULT_TEXT.len(), 17)
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The first seven steps are the input, its bytes, the four parts of the bitstream and the data codewords, and the data
/// codewords are dimmed until the last of them.
#[test]
fn the_first_seven_steps_are_the_input_the_bytes_the_bitstream_in_four_parts_and_the_data_codewords()
-> Result<(), String> {
    let tab = open()?;
    let starts = [
        "Step 1 of 62. Input:",
        "Step 2 of 62. Bytes:",
        "Step 3 of 62. Bitstream, mode:",
        "Step 4 of 62. Bitstream, count:",
        "Step 5 of 62. Bitstream, data:",
        "Step 6 of 62. Bitstream, terminator:",
        "Step 7 of 62. Data codewords:",
    ];
    for (position, start) in starts.iter().enumerate() {
        next(&tab, 1)?;
        check(said(&tab)?.starts_with(start), &format!("step {position}: {}", said(&tab)?))?;
        check_eq(node(&tab, "Data codewords")?.contains("not yet computed"), position < 6)?;
    }
    Ok(())
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// A jump button goes to the first step of its stage and draws it, though the toolbar's own callback is not called by
/// the selection being set. The toolbar follows, and the button of the stage is the one pressed.
#[test]
fn a_jump_goes_to_the_first_step_of_a_stage_and_the_toolbar_follows() -> Result<(), String> {
    let tab = open()?;
    jump(&tab, PLACEMENT)?;
    check(said(&tab)?.starts_with("Step 28 of 62. Codeword 1 of 26"), &said(&tab)?)?;
    check_eq(parts(&path(&tab, "highlight")?), 8)?;
    check_eq(toolbar_disabled(&tab, "Previous selection")?, false)?;
    check_eq((pressed(&tab, "jump-27")?, pressed(&tab, "jump-0")?), (true, false))?;

    // The toolbar carries on from where the jump put it.
    next(&tab, 1)?;
    check(said(&tab)?.starts_with("Step 29 of 62. Codeword 2 of 26"), &said(&tab)?)?;

    jump(&tab, FINAL)?;
    check(said(&tab)?.starts_with("Step 62 of 62. Final symbol:"), &said(&tab)?)?;
    check_eq(toolbar_disabled(&tab, "Next selection")?, true)?;
    jump(&tab, 0)?;
    check(said(&tab)?.starts_with("Step 1 of 62. Input:"), &said(&tab)?)?;
    check_eq(toolbar_disabled(&tab, "Previous selection")?, false)
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Each stage draws the matrix it should: empty to the function modules, then the codewords one at a time, then a whole
/// candidate for each mask, with every module coloured.
#[test]
fn each_stage_draws_the_matrix_it_should() -> Result<(), String> {
    let tab = open()?;
    jump(&tab, FUNCTIONS)?;
    // 233 function modules, of which the 30 for the format information have no colour yet, and 208 data modules.
    check_eq(counts(&tab)?.2, 441 - 203)?;
    check_eq(parts(&path(&tab, "highlight")?), 30)?;
    jump(&tab, PLACEMENT)?;
    check_eq(counts(&tab)?.2, 441 - 203 - 8)?;
    next(&tab, 25)?;
    check(said(&tab)?.starts_with("Step 53 of 62. Codeword 26 of 26"), &said(&tab)?)?;
    check_eq(counts(&tab)?.2, 30)?;
    jump(&tab, MASKS)?;
    check_eq(counts(&tab)?.2, 0)?;
    check_eq(parts(&path(&tab, "highlight")?), 30)?;
    check(said(&tab)?.starts_with("Step 54 of 62. Mask 0:"), &said(&tab)?)?;
    jump(&tab, FINAL)?;
    check_eq(counts(&tab)?.2, 0)
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The scores of the masks are reached a row at a time, and the format information from the first mask.
#[test]
fn the_scores_and_the_format_information_are_reached_with_the_masks() -> Result<(), String> {
    let tab = open()?;
    jump(&tab, PLACEMENT)?;
    check(
        node(&tab, "Mask scores")?.contains("not yet computed"),
        "the scores are not yet computed",
    )?;
    check(
        node(&tab, "Format information")?.contains("not yet computed"),
        "the format is not yet computed",
    )?;
    jump(&tab, MASKS)?;
    check(
        node(&tab, "Mask scores")?.contains("not yet computed"),
        "only the first row is computed",
    )?;
    check(
        !node(&tab, "Format information")?.contains("not yet computed"),
        "the format is computed",
    )?;
    jump(&tab, FINAL)?;
    check(
        !node(&tab, "Mask scores")?.contains("not yet computed"),
        "every row is computed",
    )
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Going back shows exactly what going forward showed at each step, down to every attribute and what was announced, and
/// Restart shows exactly what the window opened with.
#[test]
fn going_back_restores_every_earlier_state_exactly() -> Result<(), String> {
    let tab = open()?;
    let mut seen = vec![shown(&tab)?];
    for _ in 0..STEPS {
        next(&tab, 1)?;
        seen.push(shown(&tab)?);
    }
    check_eq(toolbar_disabled(&tab, "Next selection")?, true)?;
    for position in (0..STEPS).rev() {
        toolbar(&tab, "Previous selection")?;
        check(
            shown(&tab)? == seen[position],
            &format!("going back to position {position} shows something else"),
        )?;
    }
    jump(&tab, MASKS)?;
    toolbar(&tab, "Restart selection")?;
    check(
        shown(&tab)? == seen[0],
        "Restart shows something else than the window opened with",
    )
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The scene is built once. An element marked at the start is still the same element after the whole walk, going back
/// and new text, so nothing was drawn again.
#[test]
fn nothing_is_drawn_again_by_a_step_a_jump_or_new_text() -> Result<(), String> {
    let tab = open()?;
    tab.evaluate(
        &format!(
            "window.marked = [...document.querySelectorAll('{DIAGRAM} g[aria-label]')]; \
             window.markedMatrix = document.getElementById('qr-code-matrix');"
        ),
        false,
    )
    .map_err(|e| format!("could not mark the elements: {e}"))?;
    jump(&tab, FINAL)?;
    jump(&tab, PLACEMENT)?;
    type_text(&tab, "Hi")?;
    next(&tab, 3)?;
    let kept = text(
        &tab,
        &format!(
            "String(window.marked.length > 10 && window.marked.every(g => g.isConnected) \
             && window.markedMatrix === document.getElementById('qr-code-matrix') \
             && document.querySelectorAll('{DIAGRAM} g[aria-label]').length === window.marked.length)"
        ),
    )?;
    check_eq(kept.as_str(), "true")
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Choosing a role outlines its modules without changing the colours or the highlight. Stepping then changes the
/// colours and the highlight without changing the outline. Choosing "No role" removes it.
#[test]
fn a_chosen_role_and_the_step_are_independent() -> Result<(), String> {
    let tab = open()?;
    jump(&tab, FUNCTIONS)?;
    let (base, highlight) = (colours(&tab)?, path(&tab, "highlight")?);
    press(&tab, "role-finder")?;
    let outline = path(&tab, "role")?;
    check_eq(parts(&outline), 147)?;
    check_eq((colours(&tab)?, path(&tab, "highlight")?), (base.clone(), highlight.clone()))?;

    next(&tab, 1)?;
    check_eq(path(&tab, "role")?, outline.clone())?;
    check(path(&tab, "highlight")? != highlight, "the highlight should follow the step")?;
    check(colours(&tab)? != base, "the colours should follow the step")?;

    press(&tab, "role-format")?;
    check_eq(parts(&path(&tab, "role")?), 30)?;
    press(&tab, "role-none")?;
    check_eq(path(&tab, "role")?.as_str(), "")?;
    check_eq(parts(&path(&tab, "highlight")?), 8)
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Exactly one role button is pressed, and the outline has as many squares as the role has modules.
#[test]
fn one_role_button_is_pressed_and_each_outline_has_the_modules_of_its_role() -> Result<(), String> {
    let tab = open()?;
    check_eq((pressed(&tab, "role-none")?, pressed(&tab, "role-finder")?), (true, false))?;
    jump(&tab, FINAL)?;
    let roles = [
        ("finder", 147),
        ("separator", 45),
        ("timing", 10),
        ("dark-module", 1),
        ("format", 30),
        ("data", 152),
        ("error-correction", 56),
    ];
    for (key, modules) in roles {
        press(&tab, &format!("role-{key}"))?;
        check_eq(parts(&path(&tab, "role")?), modules)?;
        for (other, _) in roles {
            check_eq(pressed(&tab, &format!("role-{other}"))?, other == key)?;
        }
        check_eq(pressed(&tab, "role-none")?, false)?;
    }
    Ok(())
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Choosing a role names it and explains it in the scene, and "No role" says nothing is chosen. The stage is explained
/// too, and the explanation changes with the stage.
#[test]
fn a_chosen_role_and_the_stage_are_named_and_explained_in_the_scene() -> Result<(), String> {
    let tab = open()?;
    check(scene_text(&tab)?.contains("No role is chosen"), "nothing is chosen at first")?;
    check(scene_text(&tab)?.contains("Not started | Press Next"), &scene_text(&tab)?)?;
    press(&tab, "role-finder")?;
    check(
        scene_text(&tab)?.contains("Finder | The three large squares in the corners"),
        &scene_text(&tab)?,
    )?;
    press(&tab, "role-timing")?;
    check(
        !scene_text(&tab)?.contains("The three large squares"),
        "the explanation of the last role should be gone",
    )?;
    jump(&tab, RS)?;
    check(
        scene_text(&tab)?.contains("Reed-Solomon | Error correction codewords let a scanner"),
        &scene_text(&tab)?,
    )?;
    jump(&tab, MASKS)?;
    check(scene_text(&tab)?.contains("Masks | Large plain areas"), &scene_text(&tab)?)?;
    press(&tab, "role-none")?;
    check(scene_text(&tab)?.contains("No role is chosen"), "choosing no role says so")
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Typing new text goes back to the start, with an empty matrix, and the walk then follows the new text.
#[test]
fn typing_new_text_goes_back_to_the_start_and_the_walk_follows_it() -> Result<(), String> {
    let tab = open()?;
    jump(&tab, PLACEMENT)?;
    check(counts(&tab)?.2 < 441, "the matrix should have something in it")?;
    type_text(&tab, "Hi")?;
    check_eq(said(&tab)?.as_str(), "Not started. Every module is empty.")?;
    check_eq(counts(&tab)?, (0, 0, 441))?;
    check_eq(
        (
            toolbar_disabled(&tab, "Previous selection")?,
            toolbar_disabled(&tab, "Next selection")?,
        ),
        (true, false),
    )?;
    next(&tab, 2)?;
    check_eq(said(&tab)?.as_str(), "Step 2 of 62. Bytes: 2 of 17, 48 69.")?;
    check(banner(&tab)?.is_empty(), "no error is shown for text that fits")
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Text that cannot be encoded is refused with the reason, and what was on show stays, and the text node goes back to
/// the text that made it, so that it always agrees with the symbol. Text that can be encoded again clears the reason
/// and starts the walk.
#[test]
fn text_that_cannot_be_encoded_leaves_the_symbol_on_show_alone() -> Result<(), String> {
    let tab = open()?;
    // Only the first snapshot of a stage has a button, so the walk goes on from there to the sixth codeword.
    jump(&tab, PLACEMENT)?;
    next(&tab, 5)?;
    let on_show = shown(&tab)?;

    type_text(&tab, "abcdefghijklmnopqr")?;
    check(banner(&tab)?.contains("18") && banner(&tab)?.contains("17"), &banner(&tab)?)?;
    check(shown(&tab)? == on_show, "a text that is too long changed what is on show")?;
    check_eq(input_value(&tab)?.as_str(), DEFAULT_TEXT)?;

    type_text(&tab, "Hello £")?;
    check(banner(&tab)?.contains("not ASCII"), &banner(&tab)?)?;
    check(shown(&tab)? == on_show, "a text that is not ASCII changed what is on show")?;
    check_eq(input_value(&tab)?.as_str(), DEFAULT_TEXT)?;
    // The walk goes on where it was.
    next(&tab, 1)?;
    check(said(&tab)?.starts_with("Step 34 of 62."), &said(&tab)?)?;

    type_text(&tab, "Fine")?;
    check(banner(&tab)?.is_empty(), "the reason should go once the text can be encoded")?;
    check_eq(input_value(&tab)?.as_str(), "Fine")?;
    check_eq(said(&tab)?.as_str(), "Not started. Every module is empty.")
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Typing a character that cannot be encoded does not take it. The reason is shown, the text node holds what it held,
/// the caret stays where it was so that typing can go on, and the next character that can be encoded is taken.
#[test]
fn a_character_that_cannot_be_encoded_is_not_taken_and_typing_goes_on() -> Result<(), String> {
    let tab = open()?;
    type_text(&tab, "Hi")?;
    next(&tab, 1)?;
    let on_show = shown(&tab)?;
    // The caret is at the end of the text, and a pound sign is typed.
    tab.evaluate(
        &format!(
            "(() => {{ const i = document.querySelector('{DIAGRAM} input'); i.focus(); i.setSelectionRange(2, 2); }})()"
        ),
        false,
    )
    .map_err(|e| format!("could not place the caret: {e}"))?;
    tab.type_str("£").map_err(|e| format!("could not type: {e}"))?;
    std::thread::sleep(Duration::from_millis(300));
    check(banner(&tab)?.contains("not ASCII"), &banner(&tab)?)?;
    check_eq(input_value(&tab)?.as_str(), "Hi")?;
    check_eq(input_caret(&tab)?, (2, 2))?;
    check(shown(&tab)? == on_show, "a refused character changed what is on show")?;
    check_eq(text(&tab, "document.activeElement?.tagName ?? 'none'")?.as_str(), "INPUT")?;

    // Typing goes on from the same place, and a character that can be encoded is taken, which clears the reason.
    tab.type_str("!").map_err(|e| format!("could not type: {e}"))?;
    std::thread::sleep(Duration::from_millis(300));
    check_eq(input_value(&tab)?.as_str(), "Hi!")?;
    check(banner(&tab)?.is_empty(), "the reason should go")?;
    check_eq(said(&tab)?.as_str(), "Not started. Every module is empty.")
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The toolbar and the buttons work from the keyboard: Enter and Space on a focused Next each take one step, Enter on a
/// jump button goes to its stage, and focus stays where it was.
#[test]
fn the_toolbar_and_the_buttons_work_from_the_keyboard() -> Result<(), String> {
    let tab = open()?;
    let focus = |selector: &str| -> Result<(), String> {
        tab.evaluate(&format!("document.querySelector('{selector}').focus()"), false)
            .map_err(|e| format!("could not focus {selector}: {e}"))?;
        Ok(())
    };
    let pause = || std::thread::sleep(Duration::from_millis(150));

    focus(&format!("{DIAGRAM} g[aria-label=\"Next selection\"]"))?;
    tab.press_key("Enter").map_err(|e| format!("could not press Enter: {e}"))?;
    pause();
    check(said(&tab)?.starts_with("Step 1 of 62."), &said(&tab)?)?;
    tab.press_key(" ").map_err(|e| format!("could not press Space: {e}"))?;
    pause();
    check(said(&tab)?.starts_with("Step 2 of 62."), &said(&tab)?)?;
    check_eq(
        text(&tab, "document.activeElement?.getAttribute('aria-label') ?? 'none'")?.as_str(),
        "Next selection",
    )?;

    focus(&format!("{DIAGRAM} g[data-action=\"jump-{MASKS}\"]"))?;
    tab.press_key("Enter").map_err(|e| format!("could not press Enter: {e}"))?;
    pause();
    check(said(&tab)?.starts_with("Step 54 of 62."), &said(&tab)?)?;
    check_eq(
        text(&tab, "document.activeElement?.getAttribute('data-action') ?? 'none'")?.as_str(),
        "jump-53",
    )
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Zooming moves the scene's layer and leaves the matrix, and every path in it, as it was. A step still works.
#[test]
fn zooming_leaves_the_matrix_alone_and_the_buttons_working() -> Result<(), String> {
    let tab = open()?;
    jump(&tab, PLACEMENT)?;
    let (matrix, drawn) = (attribute(&tab, "#qr-code-matrix", "transform")?, shown(&tab)?);
    let layer = attribute(&tab, LAYER, "transform")?;
    click(&tab, &format!("{DIAGRAM} g[aria-label=\"Zoom in\"]"))?;
    std::thread::sleep(Duration::from_millis(200));
    check(
        attribute(&tab, LAYER, "transform")? != layer,
        "zooming should change the layer's transform",
    )?;
    check_eq(attribute(&tab, "#qr-code-matrix", "transform")?, matrix)?;
    check_eq(shown(&tab)?, drawn)?;

    next(&tab, 1)?;
    check(said(&tab)?.starts_with("Step 29 of 62."), &said(&tab)?)?;
    check(attribute(&tab, LAYER, "transform")? != layer, "a step should keep the zoom")
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The live region says where the walk is and what the step did, briefly, and a chosen role adds how many modules it
/// has.
#[test]
fn the_live_region_follows_the_steps_and_the_role() -> Result<(), String> {
    let tab = open()?;
    check_eq(said(&tab)?.as_str(), "Not started. Every module is empty.")?;
    jump(&tab, FUNCTIONS)?;
    check(said(&tab)?.starts_with("Step 27 of 62. Function modules:"), &said(&tab)?)?;
    press(&tab, "role-finder")?;
    check(said(&tab)?.ends_with("Outlining 147 finder modules."), &said(&tab)?)?;
    next(&tab, 1)?;
    check(said(&tab)?.starts_with("Step 28 of 62. Codeword 1 of 26"), &said(&tab)?)?;
    // It never reads out the matrix: a step is a sentence or two, not 441 modules.
    check(said(&tab)?.len() < 300, "a step should not be announced at length")?;
    toolbar(&tab, "Restart selection")?;
    check(said(&tab)?.starts_with("Not started."), &said(&tab)?)
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The matrix as text has 21 rows of 21 characters. It is all `.` at first, shows the finders and the uncoloured format
/// modules at the function step, and has no `.` at the last.
#[test]
fn the_matrix_as_text_follows_the_steps() -> Result<(), String> {
    let tab = open()?;
    let rows = |tab: &Tab| -> Result<Vec<String>, String> {
        Ok(text(tab, "document.getElementById('qr-code-matrix-text').textContent")?
            .split('\n')
            .map(str::to_owned)
            .collect())
    };
    let empty = rows(&tab)?;
    check_eq((empty.len(), empty.iter().all(|row| row == &".".repeat(21))), (21, true))?;
    jump(&tab, FUNCTIONS)?;
    check_eq(rows(&tab)?[0].as_str(), "11111110.....01111111")?;
    jump(&tab, FINAL)?;
    let last = rows(&tab)?;
    check_eq((last.len(), last.iter().all(|row| row.len() == 21)), (21, true))?;
    check(
        last.iter().all(|row| !row.contains('.')),
        "every module has a colour at the last step",
    )?;
    check_eq(last[13].chars().nth(8), Some('1'))
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The text node holds at most 17 characters. Typing more than that is not taken, and the text on show is the first 17.
#[test]
fn the_text_node_takes_no_more_than_17_characters() -> Result<(), String> {
    let tab = open()?;
    check_eq(attribute(&tab, &format!("{DIAGRAM} input"), "maxlength")?.as_str(), "17")?;
    tab.evaluate(
        &format!("(() => {{ const i = document.querySelector('{DIAGRAM} input'); i.focus(); i.select(); }})()"),
        false,
    )
    .map_err(|e| format!("could not select the text: {e}"))?;
    tab.type_str("abcdefghijklmnopqrstuvwxyz")
        .map_err(|e| format!("could not type: {e}"))?;
    std::thread::sleep(Duration::from_millis(400));
    check_eq(
        text(&tab, &format!("document.querySelector('{DIAGRAM} input').value"))?.as_str(),
        "abcdefghijklmnopq",
    )?;
    check(
        banner(&tab)?.is_empty(),
        "typing past the limit should not be an error, just not taken",
    )?;
    next(&tab, 2)?;
    check_eq(
        said(&tab)?.as_str(),
        "Step 2 of 62. Bytes: 17 of 17, 61 62 63 64 65 66 67 68 69 6A 6B 6C 6D 6E 6F 70 71.",
    )
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The text node and the hex dump of its bytes share one horizontal midline, so the edge between them is straight.
#[test]
fn the_text_node_and_the_hex_dump_share_one_midline() -> Result<(), String> {
    let tab = open()?;
    let midlines = text(
        &tab,
        "(() => { const box = (name) => document \
         .querySelector('#qr-code-diagram g[aria-label^=\"' + name + '\"] rect').getBoundingClientRect(); \
         const [a, b] = [box('ASCII text'), box('Input bytes')]; \
         return [a.top + a.height / 2, b.top + b.height / 2, a.height, b.height].join(','); })()",
    )?;
    let numbers: Vec<f64> = midlines.split(',').filter_map(|n| n.parse().ok()).collect();
    check_eq(numbers.len(), 4)?;
    check(
        (numbers[0] - numbers[1]).abs() < 1.0,
        &format!("the midlines differ: {midlines}"),
    )?;
    // They are not just the same height: the hex dump is taller, so the text node had to move.
    check(
        numbers[2] != numbers[3],
        &format!("the two boxes are the same height: {midlines}"),
    )?;
    // The edge between them is one horizontal line.
    let edge = text(
        &tab,
        "(() => { const p = [...document.querySelectorAll('#qr-code-diagram path')].filter(p => !p.id)[0]; \
         const b = p.getBoundingClientRect(); return b.height.toFixed(1); })()",
    )?;
    check_eq(edge.as_str(), "0.0")
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The hex dump is a node of its own kind, with a line for each 16 bytes and the unused places dimmed.
#[test]
fn the_bytes_are_a_hex_dump_and_the_unused_places_are_dimmed() -> Result<(), String> {
    let tab = open()?;
    check(node(&tab, "Input bytes")?.contains("hex dump"), &node(&tab, "Input bytes")?)?;
    next(&tab, 2)?;
    check(
        !node(&tab, "Input bytes")?.contains("not yet computed"),
        "17 bytes use every place",
    )?;
    type_text(&tab, "Hi")?;
    next(&tab, 2)?;
    check(
        node(&tab, "Input bytes")?.contains("not yet computed"),
        "2 bytes leave the other places unused",
    )
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Each part of the bitstream is marked in turn, with a note to the right that says what it holds, and the bits after
/// it are not on show yet. The default text gives 4 bits of mode, 8 of count, 136 of data and 4 of terminator.
#[test]
fn the_bitstream_is_marked_a_part_at_a_time_with_a_note_to_say_what_it_holds() -> Result<(), String> {
    let tab = open()?;
    let parts = [
        ("Mode indicator, 4 bits | 0100 says", "cells 0 to 3", "cells 4 to 151"),
        ("Character count, 8 bits | 17 in binary", "cells 4 to 11", "cells 12 to 151"),
        ("Data, 136 bits | The 17 bytes", "cells 12 to 147", "cells 148 to 151"),
        ("Terminator, 4 bits | Zero bits", "cells 148 to 151", ""),
    ];
    next(&tab, 2)?;
    for (note, marked, not_yet) in parts {
        next(&tab, 1)?;
        let shown = scene_text(&tab)?;
        check(shown.contains(note), &format!("the note {note:?} is not in {shown}"))?;
        let label = node(&tab, "Bitstream")?;
        check(
            label.contains(&format!("also highlighted: {marked}")),
            &format!("{marked} is not marked: {label}"),
        )?;
        check_eq(
            label.contains(&format!("not yet computed: {not_yet}")) || not_yet.is_empty(),
            true,
        )?;
    }
    // Once the data codewords are made, the note goes and nothing in the bitstream is marked.
    next(&tab, 1)?;
    check(
        !node(&tab, "Bitstream")?.contains("also highlighted"),
        "nothing should be marked now",
    )?;
    check(!scene_text(&tab)?.contains("Terminator, 4 bits"), "the note should go")
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// At each Reed-Solomon step the data codeword that is taken in is marked in the data codewords, and it is the next one
/// at the next step.
#[test]
fn the_data_codeword_that_the_division_takes_in_is_marked() -> Result<(), String> {
    let tab = open()?;
    check(
        !node(&tab, "Data codewords")?.contains("also highlighted"),
        "nothing is marked before the division",
    )?;
    jump(&tab, RS)?;
    check(
        node(&tab, "Data codewords")?.contains("also highlighted: cell 0"),
        &node(&tab, "Data codewords")?,
    )?;
    next(&tab, 1)?;
    check(
        node(&tab, "Data codewords")?.contains("also highlighted: cell 1"),
        &node(&tab, "Data codewords")?,
    )?;
    next(&tab, 17)?;
    check(
        node(&tab, "Data codewords")?.contains("also highlighted: cell 18"),
        &node(&tab, "Data codewords")?,
    )?;
    next(&tab, 1)?;
    check(
        !node(&tab, "Data codewords")?.contains("also highlighted"),
        "the mark should go with the division",
    )
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Two connectors arrive on the north edge of the codewords. The one from the data codewords lands over the middle of
/// the 19 data codewords, and the one from the error correction lands over the middle of the last 7 places, which it
/// fills.
#[test]
fn the_connectors_into_the_codewords_land_over_the_data_and_over_the_error_correction() -> Result<(), String> {
    let tab = open()?;
    let found = text(
        &tab,
        "(() => { const edges = [...document.querySelectorAll('#qr-code-diagram path')].filter(p => !p.id); \
         const end = (p) => p.getPointAtLength(p.getTotalLength()).matrixTransform(p.getScreenCTM()); \
         const group = document.querySelector('#qr-code-diagram g[aria-label^=\"Codewords\"]'); \
         const cells = [...group.querySelectorAll('text')].filter(t => /^[0-9A-F]{2}$/.test(t.textContent)) \
             .map(t => t.getBoundingClientRect()); \
         const top = group.querySelector('rect').getBoundingClientRect().top; \
         const [d, e] = [end(edges[5]), end(edges[6])]; \
         const centre = (r) => r.left + r.width / 2; \
         return [edges.length, cells.length, d.x, e.x, d.y - top, e.y - top, centre(cells[9]), centre(cells[19]), \
                 centre(cells[25]), cells[19].left, cells[25].right].join(','); })()",
    )?;
    let n: Vec<f64> = found.split(',').filter_map(|v| v.parse().ok()).collect();
    check_eq(n.len(), 11)?;
    let [
        edges,
        cells,
        data_end,
        error_end,
        data_y,
        error_y,
        data_middle,
        first,
        last,
        left,
        right,
    ] = [n[0], n[1], n[2], n[3], n[4], n[5], n[6], n[7], n[8], n[9], n[10]];
    check_eq((edges, cells), (9.0, 26.0))?;
    // Both end on the top edge of the node.
    check(
        data_y.abs() < 2.0 && error_y.abs() < 2.0,
        &format!("not on the north edge: {found}"),
    )?;
    // The left one is over the middle of the 19 data codewords, which is the tenth.
    check(
        (data_end - data_middle).abs() < 2.0,
        &format!("the data connector is not over the data: {found}"),
    )?;
    // The right one is over the middle of the last 7 places, and so over the last of them, not over the data.
    check(
        (error_end - (first + last) / 2.0).abs() < 2.0,
        &format!("not over the middle of the 7: {found}"),
    )?;
    check(
        error_end > left && error_end < right,
        &format!("not over the error correction: {found}"),
    )?;
    check(error_end > data_end, &format!("the two are the wrong way round: {found}"))
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The note that says what the marked part of the bitstream holds is to the right of the bitstream node, level with its
/// top, and within the width of the graph, which the error correction node is the widest part of, so that the note
/// makes the scene no wider.
#[test]
fn the_note_is_to_the_right_of_the_bitstream_and_within_the_width_of_the_graph() -> Result<(), String> {
    let tab = open()?;
    next(&tab, 3)?;
    let found = text(
        &tab,
        "(() => { const rect = (el) => el.getBoundingClientRect(); \
         const box = (name) => rect(document \
             .querySelector('#qr-code-diagram g[aria-label^=\"' + name + '\"] rect')); \
         const note = [...document.querySelectorAll('#qr-code-diagram > g.svg-dom-graph-content text')] \
             .find(t => t.textContent.startsWith('Mode indicator')); \
         const [bitstream, bytes, errors, text] = [box('Bitstream'), box('Input bytes'), \
             box('Error correction codewords'), rect(note)]; \
         const graph_right = Math.max(bytes.right, errors.right); \
         return [text.left, text.right, text.top, bitstream.right, bitstream.top, graph_right].join(','); })()",
    )?;
    let n: Vec<f64> = found.split(',').filter_map(|v| v.parse().ok()).collect();
    check_eq(n.len(), 6)?;
    let [note_left, note_right, note_top, bitstream_right, bitstream_top, graph_right] =
        [n[0], n[1], n[2], n[3], n[4], n[5]];
    check(
        note_left > bitstream_right,
        &format!("the note is not to the right of the bitstream: {found}"),
    )?;
    check(note_right <= graph_right, &format!("the note makes the scene wider: {found}"))?;
    check(
        note_top >= bitstream_top - 1.0 && note_top < bitstream_top + 60.0,
        &format!("not level with the top: {found}"),
    )
}
