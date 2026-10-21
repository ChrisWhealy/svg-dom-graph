//! Bit arrays (`NodeValues::Bits`) in a real browser: every width from 1 to 7 draws exactly that many binary digits
//! inside its cell, a node says its width, and a replacement of the same width rewrites the cells while one of another
//! width is refused.
//!
//! The fixture has one array of each width, `bits_1` to `bits_7`, each holding zero and the largest value that fits.
//! Native unit tests cover how a value is formatted. What they cannot reach is the drawing and measuring.

use crate::common::new_tab;
use headless_chrome::Tab;

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

/// The text of every cell of the node called `name`, in order.
fn cells(tab: &Tab, name: &str) -> Result<Vec<String>, String> {
    let json = text(
        tab,
        &format!(
            "JSON.stringify([...document.querySelectorAll('#diagram g[aria-label^=\"{name}\"] text')] \
             .map(t => t.textContent))"
        ),
    )?;
    serde_json::from_str(&json).map_err(|e| format!("could not read the cells of {name}: {e}"))
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Zero is as many zeros as the width, and the largest value is as many ones. There is no gap, whatever the width.
#[test]
fn binary_writes_exactly_the_width_in_digits_for_every_width() -> Result<(), String> {
    let tab = new_tab()?;
    for width in 1..=7 {
        let all = cells(&tab, &format!("bits_{width}"))?;
        // The name of the node is drawn too, so the cells are the last two texts.
        let values = &all[all.len().saturating_sub(2)..];
        check_eq(values, &["0".repeat(width), "1".repeat(width)][..])?;
    }
    Ok(())
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The digits of every cell are drawn inside the cell's own box, so the node was measured wide enough for its widest
/// value at every width.
#[test]
fn the_digits_of_every_width_fit_inside_their_cells() -> Result<(), String> {
    let tab = new_tab()?;
    for width in 1..=7 {
        let json = text(
            &tab,
            &format!(
                "JSON.stringify([...document.querySelectorAll('#diagram g[aria-label^=\"bits_{width}\"] text')] \
                 .filter(t => t.previousElementSibling?.tagName === 'rect' && /^[01]+$/.test(t.textContent)) \
                 .map(t => {{ const r = t.previousElementSibling.getBBox(); const b = t.getBBox(); \
                              return [b.x - r.x, r.x + r.width - (b.x + b.width)]; }}))"
            ),
        )?;
        let margins: Vec<[f64; 2]> =
            serde_json::from_str(&json).map_err(|e| format!("bits_{width}: could not read the margins: {e}"))?;
        check_eq(margins.len(), 2)?;
        for [left, right] in margins {
            check(
                left >= -0.5 && right >= -0.5,
                &format!("bits_{width}: the digits spill out of their cell ({left}, {right})"),
            )?;
        }
    }
    Ok(())
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// A node says its width in its tooltip or its accessible name, since its colour is the same for every width.
#[test]
fn a_node_says_its_width() -> Result<(), String> {
    let tab = new_tab()?;
    for width in 1..=7 {
        let said = text(
            &tab,
            &format!(
                "(() => {{ const g = document.querySelector('#diagram g[aria-label^=\"bits_{width}\"]'); \
                   return (g.getAttribute('aria-label') ?? '') + ' | ' + (g.querySelector('title')?.textContent ?? ''); }})()"
            ),
        )?;
        check(
            said.contains(&format!("{width}-bit")),
            &format!("bits_{width} says only {said:?}"),
        )?;
    }
    Ok(())
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Replacing the values of a rendered array with others of the same width rewrites its cells.
#[test]
fn a_replacement_of_the_same_width_rewrites_the_cells() -> Result<(), String> {
    let tab = new_tab()?;
    check_eq(
        text(&tab, "window.fixture.replace_bits('bits_6', 6, new Uint8Array([19, 63]))")?.as_str(),
        "ok",
    )?;
    let all = cells(&tab, "bits_6")?;
    check_eq(&all[all.len() - 2..], &["010011".to_owned(), "111111".to_owned()][..])
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// A replacement of another width is refused, and so is a value that does not fit. Neither changes a cell.
#[test]
fn a_replacement_of_another_width_or_too_wide_a_value_is_refused_and_changes_nothing() -> Result<(), String> {
    let tab = new_tab()?;
    let before = cells(&tab, "bits_6")?;

    let other_width = text(&tab, "window.fixture.replace_bits('bits_6', 5, new Uint8Array([1, 2]))")?;
    check(
        other_width.contains("cannot show those values"),
        &format!("width 5 was not refused: {other_width:?}"),
    )?;

    let too_wide = text(&tab, "window.fixture.replace_bits('bits_6', 6, new Uint8Array([64, 1]))")?;
    check(
        // The wording of the message is not the point, so this looks for what it must name: the value and the width.
        too_wide.contains("index 0") && too_wide.contains("6 bits"),
        &format!("64 was not refused: {too_wide:?}"),
    )?;

    check_eq(cells(&tab, "bits_6")?, before)
}
