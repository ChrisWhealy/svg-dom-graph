//! Pressing the buttons and the range from the keyboard.

use super::*;

#[test]
fn a_button_takes_enter_and_space_from_the_keyboard() -> Result<(), String> {
    let tab = open()?;
    text(
        &tab,
        "(() => { document.querySelector('[data-action=\"next\"]').focus(); return 'ok'; })()",
    )?;
    press(&tab, "Enter")?;
    check_eq(position(&tab)?, 1)?;
    press(&tab, " ")?;
    check_eq(position(&tab)?, 2)?;
    check_eq(focused(&tab)?, "button:next".to_owned())
}

#[test]
fn the_range_keys_move_one_step_at_a_time_and_its_arrow_keys_work() -> Result<(), String> {
    let tab = open()?;
    text(&tab, "(() => { document.getElementById('rbt-seek').focus(); return 'ok'; })()")?;
    press(&tab, "ArrowRight")?;
    press(&tab, "ArrowRight")?;
    check_eq(position(&tab)?, 2)?;
    check_eq(attribute(&tab, "#rbt-seek", "aria-valuetext")?, "Step 2 of 29".to_owned())?;
    check_eq(content(&tab, "#rbt-stage .scene-slider-value")?, "Step 2 of 29".to_owned())
}
