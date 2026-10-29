//! The QR code panel in the built demo gallery: the matrix drawn with a fixed handful of elements, the buttons and the
//! list of roles drawn in the scene, and what each of them changes.
//!
//! The grid, the layout and the path data are covered by native unit tests. What they cannot reach is the browser: that
//! the values are written to the right elements, that going back restores exactly what going forward showed, that
//! choosing a role and stepping stay independent, that the buttons work from the keyboard and not when disabled, that
//! zooming leaves the matrix alone, and that the live region and the matrix as text follow each step.

use crate::common::{DemoTab, new_demo_tab};
use headless_chrome::Tab;
use std::time::Duration;

const DIAGRAM: &str = "#qr-code-diagram";
const LAYER: &str = "#qr-code-diagram > g.svg-dom-graph-content";

/// The length of the walk, and what each step highlights: the 30 format modules, one codeword, the last codeword, none,
/// and the 30 format modules again.
const STEPS: usize = 5;
const HIGHLIGHTED: [usize; STEPS] = [30, 8, 8, 0, 30];

fn open() -> Result<DemoTab, String> {
    new_demo_tab(
        "index.html#panel-qr-code",
        &format!("{DIAGRAM} g[data-action=\"next\"]"),
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

/// Clicks the button drawn in the scene with `action`, with a real mouse click, and waits for the redraw.
fn press(tab: &Tab, action: &str) -> Result<(), String> {
    tab.find_element(&format!("{DIAGRAM} g[data-action=\"{action}\"]"))
        .map_err(|e| format!("could not find {action}: {e}"))?
        .click()
        .map_err(|e| format!("could not click {action}: {e}"))?;
    std::thread::sleep(Duration::from_millis(150));
    Ok(())
}

/// Presses Next until the walk is at step `step`, counting from 1.
fn step_to(tab: &Tab, step: usize) -> Result<(), String> {
    for _ in 0..step {
        press(tab, "next")?;
    }
    Ok(())
}

/// Everything the matrix shows: its three colours, the outline of the role, the highlight and its accessible name.
fn shown(tab: &Tab) -> Result<Vec<String>, String> {
    let mut all = Vec::new();
    for which in ["unassigned", "light", "dark", "role", "highlight"] {
        all.push(path(tab, which)?);
    }
    all.push(attribute(tab, "#qr-code-matrix", "aria-label")?);
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

/// Whether the button `action` is disabled, as assistive technology is told.
fn disabled(tab: &Tab, action: &str) -> Result<bool, String> {
    Ok(attribute(tab, &format!("{DIAGRAM} g[data-action=\"{action}\"]"), "aria-disabled")? == "true")
}

/// Whether the role button `action` is pressed.
fn pressed(tab: &Tab, action: &str) -> Result<bool, String> {
    Ok(attribute(tab, &format!("{DIAGRAM} g[data-action=\"{action}\"]"), "aria-pressed")? == "true")
}

/// What the live region last said.
fn said(tab: &Tab) -> Result<String, String> {
    text(tab, "document.getElementById('qr-code-announcement').textContent")
}

/// All the text in the scene, a block at a time, with the lines of each block joined by spaces.
fn scene_text(tab: &Tab) -> Result<String, String> {
    text(
        tab,
        &format!(
            "[...document.querySelectorAll('{LAYER} text')] \
             .map(t => [...t.querySelectorAll('tspan')].map(s => s.textContent).join(' ')).join(' | ')"
        ),
    )
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The panel opens unstarted: every one of the 441 modules is uncoloured, so only the pale path draws any, and the
/// matrix is six elements.
#[test]
fn the_panel_opens_unstarted_with_every_module_uncoloured_in_six_elements() -> Result<(), String> {
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
    check_eq(attribute(&tab, "#qr-code-matrix", "role")?.as_str(), "img")
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Prev and Restart are disabled until the walk starts, and Next is disabled on the last step, as the toolbar does it.
#[test]
fn the_buttons_are_disabled_at_the_ends_of_the_walk() -> Result<(), String> {
    let tab = open()?;
    check_eq(
        (disabled(&tab, "prev")?, disabled(&tab, "restart")?, disabled(&tab, "next")?),
        (true, true, false),
    )?;
    press(&tab, "next")?;
    check_eq(
        (disabled(&tab, "prev")?, disabled(&tab, "restart")?, disabled(&tab, "next")?),
        (false, false, false),
    )?;
    step_to(&tab, STEPS - 1)?;
    check_eq(
        (disabled(&tab, "prev")?, disabled(&tab, "restart")?, disabled(&tab, "next")?),
        (false, false, true),
    )?;
    check_eq(
        attribute(&tab, &format!("{DIAGRAM} g[data-action=\"next\"]"), "tabindex")?.as_str(),
        "-1",
    )
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// A click on a disabled button changes nothing, since a disabled button still receives the click.
#[test]
fn a_click_on_a_disabled_button_changes_nothing() -> Result<(), String> {
    let tab = open()?;
    let before = shown(&tab)?;
    press(&tab, "prev")?;
    press(&tab, "restart")?;
    check_eq(shown(&tab)?, before)?;
    step_to(&tab, STEPS)?;
    let last = shown(&tab)?;
    press(&tab, "next")?;
    check_eq(shown(&tab)?, last)
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The first step colours the 203 function modules that have a colour already, and leaves the 30 format modules and the
/// 208 data modules without one. The matrix as text shows the top row: a finder, a separator, five uncoloured modules,
/// a separator and a finder.
#[test]
fn the_first_step_leaves_the_format_and_data_modules_uncoloured() -> Result<(), String> {
    let tab = open()?;
    press(&tab, "next")?;
    let (dark, light, none) = counts(&tab)?;
    check_eq((dark + light, none), (233 - 30, 208 + 30))?;
    check_eq(
        text(
            &tab,
            "document.getElementById('qr-code-matrix-text').textContent.split('\\n')[0]",
        )?
        .as_str(),
        "11111110.....01111111",
    )
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Each step has the colours it should: the codewords are placed, then every module has a colour, and the highlight
/// marks what the step changed.
#[test]
fn each_step_colours_and_highlights_what_it_should() -> Result<(), String> {
    let tab = open()?;
    let expected_uncoloured = [238, 230, 30, 30, 0];
    for step in 0..STEPS {
        press(&tab, "next")?;
        let (dark, light, none) = counts(&tab)?;
        check_eq(none, expected_uncoloured[step])?;
        check_eq(dark + light + none, 441)?;
        check_eq(parts(&path(&tab, "highlight")?), HIGHLIGHTED[step])?;
    }
    Ok(())
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Going back shows exactly what going forward showed at each step, down to every attribute, and Restart shows exactly
/// what the panel opened with.
#[test]
fn going_back_restores_every_earlier_state_exactly() -> Result<(), String> {
    let tab = open()?;
    let mut seen = vec![shown(&tab)?];
    for _ in 0..STEPS {
        press(&tab, "next")?;
        seen.push(shown(&tab)?);
    }
    for position in (0..STEPS).rev() {
        press(&tab, "prev")?;
        check(
            shown(&tab)? == seen[position],
            &format!("going back to position {position} shows something else"),
        )?;
    }
    step_to(&tab, 3)?;
    press(&tab, "restart")?;
    check(
        shown(&tab)? == seen[0],
        "Restart shows something else than the panel opened with",
    )
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Choosing a role outlines its modules without changing the colours or the highlight. Stepping then changes the
/// colours and the highlight without changing the outline. Choosing "No role" removes it.
#[test]
fn a_chosen_role_and_the_step_are_independent() -> Result<(), String> {
    let tab = open()?;
    press(&tab, "next")?;
    let (base, highlight) = (colours(&tab)?, path(&tab, "highlight")?);
    press(&tab, "role-finder")?;
    let outline = path(&tab, "role")?;
    check_eq(parts(&outline), 147)?;
    check_eq(
        (path(&tab, "dark")?, path(&tab, "highlight")?),
        (base.clone(), highlight.clone()),
    )?;

    press(&tab, "next")?;
    check_eq(path(&tab, "role")?, outline.clone())?;
    check(path(&tab, "highlight")? != highlight, "the highlight should follow the step")?;
    check(colours(&tab)? != base, "the colours should follow the step")?;

    press(&tab, "role-format")?;
    check_eq(parts(&path(&tab, "role")?), 30)?;
    press(&tab, "role-none")?;
    check_eq(path(&tab, "role")?.as_str(), "")?;
    check_eq(parts(&path(&tab, "highlight")?), HIGHLIGHTED[1])
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Exactly one role button is pressed, and the outline has as many squares as the role has modules.
#[test]
fn one_role_button_is_pressed_and_each_outline_has_the_modules_of_its_role() -> Result<(), String> {
    let tab = open()?;
    check_eq((pressed(&tab, "role-none")?, pressed(&tab, "role-finder")?), (true, false))?;
    step_to(&tab, STEPS)?;
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

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Choosing a role names it and explains it in the scene, and "No role" says nothing is chosen.
#[test]
fn a_chosen_role_is_named_and_explained_in_the_scene() -> Result<(), String> {
    let tab = open()?;
    check(scene_text(&tab)?.contains("No role is chosen"), "nothing is chosen at first")?;
    press(&tab, "role-finder")?;
    let shown = scene_text(&tab)?;
    check(
        shown.contains("Finder | The three large squares in the corners"),
        &format!("finder: {shown}"),
    )?;
    press(&tab, "role-timing")?;
    let shown = scene_text(&tab)?;
    check(
        shown.contains("Timing | Two lines of alternating dark and light modules"),
        &format!("timing: {shown}"),
    )?;
    check(
        !shown.contains("The three large squares"),
        "the explanation of the last role should be gone",
    )?;
    press(&tab, "role-none")?;
    check(scene_text(&tab)?.contains("No role is chosen"), "choosing no role says so")
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The buttons work from the keyboard: Enter and Space on a focused Next each take one step, and focus stays on it.
#[test]
fn the_buttons_work_from_the_keyboard() -> Result<(), String> {
    let tab = open()?;
    tab.evaluate(
        &format!("document.querySelector('{DIAGRAM} g[data-action=\"next\"]').focus()"),
        false,
    )
    .map_err(|e| format!("could not focus Next: {e}"))?;
    tab.press_key("Enter").map_err(|e| format!("could not press Enter: {e}"))?;
    std::thread::sleep(Duration::from_millis(150));
    check_eq(parts(&path(&tab, "highlight")?), HIGHLIGHTED[0])?;
    tab.press_key(" ").map_err(|e| format!("could not press Space: {e}"))?;
    std::thread::sleep(Duration::from_millis(150));
    check_eq(parts(&path(&tab, "highlight")?), HIGHLIGHTED[1])?;
    check_eq(
        attribute(&tab, "#qr-code-diagram g[data-action=\"next\"]", "aria-disabled")?.as_str(),
        "false",
    )?;
    check_eq(
        text(&tab, "document.activeElement?.getAttribute('data-action') ?? 'none'")?.as_str(),
        "next",
    )
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Zooming moves the scene's layer and leaves the matrix, and every path in it, as it was. The buttons still work.
#[test]
fn zooming_leaves_the_matrix_alone_and_the_buttons_working() -> Result<(), String> {
    let tab = open()?;
    step_to(&tab, 2)?;
    let (matrix, drawn) = (attribute(&tab, "#qr-code-matrix", "transform")?, shown(&tab)?);
    let layer = attribute(&tab, LAYER, "transform")?;
    tab.find_element(&format!("{DIAGRAM} g[aria-label=\"Zoom in\"]"))
        .map_err(|e| format!("could not find Zoom in: {e}"))?
        .click()
        .map_err(|e| format!("could not zoom in: {e}"))?;
    std::thread::sleep(Duration::from_millis(300));
    check(
        attribute(&tab, LAYER, "transform")? != layer,
        "zooming should change the layer's transform",
    )?;
    check_eq(attribute(&tab, "#qr-code-matrix", "transform")?, matrix)?;
    check_eq(shown(&tab)?, drawn)?;

    press(&tab, "next")?;
    check_eq(parts(&path(&tab, "highlight")?), HIGHLIGHTED[2])?;
    check(attribute(&tab, LAYER, "transform")? != layer, "a step should keep the zoom")
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The live region says where the walk is and what the step did, briefly. A chosen role adds how many modules it has.
#[test]
fn the_live_region_follows_the_steps_and_the_role() -> Result<(), String> {
    let tab = open()?;
    check_eq(said(&tab)?.as_str(), "Not started. Every module is empty.")?;
    press(&tab, "next")?;
    check(said(&tab)?.starts_with("Step 1 of 5. Function modules reserved."), &said(&tab)?)?;
    press(&tab, "role-finder")?;
    check(said(&tab)?.ends_with("Outlining 147 finder modules."), &said(&tab)?)?;
    press(&tab, "next")?;
    check(said(&tab)?.starts_with("Step 2 of 5. Codeword 1 placed"), &said(&tab)?)?;
    // It never reads out the matrix: a step is a sentence or two, not 441 modules.
    check(said(&tab)?.len() < 250, "a step should not be announced at length")?;
    press(&tab, "restart")?;
    check(said(&tab)?.starts_with("Not started."), &said(&tab)?)
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The matrix as text has 21 rows of 21 characters, and shows the three states: an uncoloured module as `.`, and a
/// coloured one as `0` or `1`. At the last step none is `.`.
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
    step_to(&tab, STEPS)?;
    let last = rows(&tab)?;
    check_eq((last.len(), last.iter().all(|row| row.len() == 21)), (21, true))?;
    check(
        last.iter().all(|row| !row.contains('.')),
        "every module has a colour at the last step",
    )?;
    check_eq(last[13].chars().nth(8), Some('1'))
}
