//! Reaching and pressing every control from the keyboard.

use super::*;

#[test]
fn the_focus_on_a_stage_control_is_not_moved_by_anything() -> Result<(), String> {
    let tab = open()?;
    text(&tab, "(() => { document.getElementById('ms-field').focus(); return 'ok'; })()")?;
    for _ in 0..3 {
        click_button(&tab, "rebuild")?;
    }
    check_eq(text(&tab, "document.activeElement.id")?, "ms-field".to_owned())
}

// The keyboard.

#[test]
fn a_button_takes_enter_and_space_and_the_range_takes_the_arrow_keys() -> Result<(), String> {
    let tab = open()?;
    click_button(&tab, "restart")?;
    text(
        &tab,
        "(() => { document.querySelector('[data-action=\"next\"]').focus(); return 'ok'; })()",
    )?;
    press(&tab, "Enter")?;
    check_eq(step(&tab)?, 1)?;
    press(&tab, " ")?;
    check_eq(step(&tab)?, 2)?;
    check_eq(
        text(&tab, "document.activeElement.getAttribute('data-action')")?,
        "next".to_owned(),
    )?;
    text(&tab, "(() => { document.getElementById('ms-seek').focus(); return 'ok'; })()")?;
    press(&tab, "ArrowRight")?;
    press(&tab, "ArrowRight")?;
    wait_until(&tab, 3, |t| Ok(step(t)? == 4))?;
    check_eq(attribute(&tab, "#ms-seek", "aria-valuetext")?, "4 of 432 cells".to_owned())
}

#[test]
fn every_control_is_reached_by_tab_in_order_and_a_cell_is_chosen_without_a_pointer() -> Result<(), String> {
    let tab = open()?;
    text(&tab, "(() => { document.getElementById('ms-field').focus(); return 'ok'; })()")?;
    let mut visited = Vec::new();
    for _ in 0..14 {
        press(&tab, "Tab")?;
        visited.push(text(
            &tab,
            "(() => { const a = document.activeElement; return a.id || a.getAttribute('data-action') || a.getAttribute('aria-label') || a.tagName; })()",
        )?);
    }
    let position = |name: &str| visited.iter().position(|v| v == name);
    let order = [
        "ms-show", "ms-inspect", "ms-threshold", "ms-row", "ms-column", "restart", "previous", "next", "rebuild",
        "retry", "ms-seek",
    ];
    let mut last = None;
    for name in order {
        let at = position(name).ok_or_else(|| format!("{name} is not reached: {visited:?}"))?;
        check(last.is_none_or(|l| at > l), &format!("{name} is out of order: {visited:?}"))?;
        last = Some(at);
    }
    // A cell is chosen by the selectors, which are the browser's own, with their events.
    choose(&tab, "ms-row", "3")?;
    choose(&tab, "ms-column", "5")?;
    check(field(&tab, "status")?.starts_with("Cell (3, 5)"), &field(&tab, "status")?)?;
    check(live(&tab)?.starts_with("Cell (3, 5)"), &live(&tab)?)
}
