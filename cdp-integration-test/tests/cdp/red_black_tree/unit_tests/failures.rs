//! A write or a swap that fails, and what is left on the page afterwards.

use super::*;

// Faults.

#[test]
fn a_tree_that_cannot_be_built_leaves_the_old_one_and_the_step() -> Result<(), String> {
    let tab = open()?;
    seek(&tab, 10)?;
    let before = digest(&tab)?;
    let root = attribute(&tab, &format!("{STAGE} .rbt-scene"), "id")?;
    fail_writes_of(&tab, "aria-level", true)?;
    click_button(&tab, "next")?;
    fail_writes_of(&tab, "aria-level", false)?;
    check(!banner_hidden(&tab)?, "reported")?;
    check_eq(position(&tab)?, 10)?;
    check_eq(attribute(&tab, &format!("{STAGE} .rbt-scene"), "id")?, root)?;
    check_eq(digest(&tab)?, before)?;
    check_eq(count(&tab, &format!("{STAGE} .rbt-board"))?, 1)?;
    check_eq(count(&tab, "#rbt-staging svg > *")?, 0)?;
    click_button(&tab, "next")?;
    check_eq(position(&tab)?, 11)?;
    check(banner_hidden(&tab)?, "cleared by a step that works")
}

#[test]
fn a_control_that_cannot_be_written_puts_back_the_others_and_leaves_the_old_tree() -> Result<(), String> {
    for name in ["aria-valuetext", "aria-disabled", "max"] {
        let tab = open()?;
        seek(&tab, 10)?;
        let before = (
            position(&tab)?,
            field(&tab, "event")?,
            attribute(&tab, "[data-action=\"previous\"]", "aria-disabled")?,
        );
        fail_writes_of(&tab, name, true)?;
        click_button(&tab, "next")?;
        fail_writes_of(&tab, name, false)?;
        check(!banner_hidden(&tab)?, &format!("{name}: reported"))?;
        check_eq(
            (
                position(&tab)?,
                field(&tab, "event")?,
                attribute(&tab, "[data-action=\"previous\"]", "aria-disabled")?,
            ),
            before,
        )
        .map_err(|e| format!("{name}: {e}"))?;
        click_button(&tab, "next")?;
        check_eq(position(&tab)?, 11).map_err(|e| format!("{name}: {e}"))?;
    }
    Ok(())
}

#[test]
fn a_rollback_that_cannot_take_off_an_attribute_that_was_absent_leaves_the_page_dirty() -> Result<(), String> {
    let tab = open()?;
    // The Next button has no `aria-disabled` here. A write of it that is then rolled back must take it off again.
    text(
        &tab,
        "(() => { document.querySelector('[data-action=\"next\"]').removeAttribute('aria-disabled'); return 'ok'; })()",
    )?;
    fail_writes_of(&tab, "aria-valuetext", true)?;
    fail_removals_of(&tab, "aria-disabled", true)?;
    click_button(&tab, "next")?;
    fail_writes_of(&tab, "aria-valuetext", false)?;
    fail_removals_of(&tab, "aria-disabled", false)?;
    check(!banner_hidden(&tab)?, "reported")?;
    check(
        content(&tab, "#rbt-error")?.contains("could not all be put back"),
        &content(&tab, "#rbt-error")?,
    )?;
    check_eq(position(&tab)?, 0)?;
    // The run is at its first step, so Restart has nowhere to go. It still commits a page that is out of date.
    click_button(&tab, "restart")?;
    check(banner_hidden(&tab)?, "repaired by a button with nowhere to go")?;
    check_eq(live(&tab)?, "Already at the first step.".to_owned())?;
    check_eq(attribute(&tab, "[data-action=\"next\"]", "aria-disabled")?, "false".to_owned())?;
    click_button(&tab, "next")?;
    check_eq(position(&tab)?, 1)
}

#[test]
fn a_rollback_that_went_back_whole_leaves_the_page_clean() -> Result<(), String> {
    let tab = open()?;
    fail_writes_of(&tab, "aria-valuetext", true)?;
    click_button(&tab, "next")?;
    fail_writes_of(&tab, "aria-valuetext", false)?;
    check(!banner_hidden(&tab)?, "reported")?;
    check(
        !content(&tab, "#rbt-error")?.contains("could not all be put back"),
        &content(&tab, "#rbt-error")?,
    )?;
    // The page was not touched, so a button with nowhere to go does not commit it, and the line stays.
    click_button(&tab, "restart")?;
    check(!banner_hidden(&tab)?, "still reported: nothing needed repairing")
}

#[test]
fn an_error_line_that_cannot_be_hidden_fails_the_step_and_the_next_action_repairs_it() -> Result<(), String> {
    let tab = open()?;
    fail_writes_of(&tab, "aria-level", true)?;
    click_button(&tab, "next")?;
    fail_writes_of(&tab, "aria-level", false)?;
    check(!banner_hidden(&tab)?, "reported")?;
    // A step that works must now clear the line, and cannot.
    fail_writes_of(&tab, "hidden", true)?;
    click_button(&tab, "next")?;
    fail_writes_of(&tab, "hidden", false)?;
    check(!banner_hidden(&tab)?, "the line is still there")?;
    // The step was not taken as done, though the tree on the page was swapped, so the page is dirty. Previous has
    // nowhere to go, and commits the page again at the step the run is really at, which is the first.
    click_button(&tab, "previous")?;
    check(banner_hidden(&tab)?, "cleared by the repair")?;
    check_eq(live(&tab)?, "Already at the first step.".to_owned())?;
    check_eq(position(&tab)?, 0)?;
    click_button(&tab, "next")?;
    check_eq(position(&tab)?, 1)
}

#[test]
fn an_error_line_that_cannot_be_shown_is_still_said_in_the_live_region() -> Result<(), String> {
    let tab = open()?;
    fail_removals_of(&tab, "hidden", true)?;
    fail_writes_of(&tab, "aria-level", true)?;
    click_button(&tab, "next")?;
    fail_writes_of(&tab, "aria-level", false)?;
    fail_removals_of(&tab, "hidden", false)?;
    check(banner_hidden(&tab)?, "the line could not be made visible")?;
    check(
        live(&tab)?.contains("could not be drawn") && live(&tab)?.contains("error line could not be shown"),
        &live(&tab)?,
    )?;
    // Hiding it is still tried later, and the step works.
    click_button(&tab, "next")?;
    check_eq(position(&tab)?, 1)?;
    check(banner_hidden(&tab)?, "hidden")
}

#[test]
fn a_focus_lost_by_a_swap_that_could_not_put_it_back_is_restored_by_the_repair() -> Result<(), String> {
    let tab = open()?;
    click_button(&tab, "jump-2")?;
    focus_item(&tab, 5)?;
    fail_focus(&tab, true)?;
    click_button(&tab, "next")?;
    fail_focus(&tab, false)?;
    check(!banner_hidden(&tab)?, "reported")?;
    // The focus went with the element that was swapped out. A button with nowhere to go repairs the page.
    click_button(&tab, "jump-2")?;
    check(banner_hidden(&tab)?, "repaired")?;
    check_eq(focused(&tab)?, "key:5".to_owned())
}

#[test]
fn a_repair_does_not_take_the_focus_from_where_the_person_has_put_it() -> Result<(), String> {
    let tab = open()?;
    click_button(&tab, "jump-2")?;
    focus_item(&tab, 5)?;
    fail_focus(&tab, true)?;
    click_button(&tab, "next")?;
    fail_focus(&tab, false)?;
    text(&tab, "(() => { document.getElementById('rbt-seek').focus(); return 'ok'; })()")?;
    click_button(&tab, "jump-2")?;
    check(banner_hidden(&tab)?, "repaired")?;
    check_eq(focused(&tab)?, "id:rbt-seek".to_owned())
}

#[test]
fn a_swap_that_throws_before_it_changes_the_stage_leaves_the_old_tree() -> Result<(), String> {
    let tab = open()?;
    seek(&tab, 12)?;
    let before = digest(&tab)?;
    fail_replacement(&tab, true, false)?;
    click_button(&tab, "next")?;
    fail_replacement(&tab, false, false)?;
    check(!banner_hidden(&tab)?, "reported")?;
    check_eq((position(&tab)?, digest(&tab)?), (12, before))?;
    check_eq(count(&tab, &format!("{STAGE} .rbt-board"))?, 1)?;
    check_eq(count(&tab, "#rbt-staging svg > *")?, 0)?;
    click_button(&tab, "next")?;
    check_eq(position(&tab)?, 13)
}

#[test]
fn a_swap_that_throws_after_it_changed_the_stage_puts_the_old_tree_back() -> Result<(), String> {
    let tab = open()?;
    seek(&tab, 12)?;
    let before = digest(&tab)?;
    let scene = attribute(&tab, &format!("{STAGE} .rbt-scene"), "id")?;
    fail_replacement(&tab, true, true)?;
    click_button(&tab, "next")?;
    fail_replacement(&tab, false, true)?;
    check(!banner_hidden(&tab)?, "reported")?;
    check_eq(position(&tab)?, 12)?;
    check_eq(count(&tab, &format!("{STAGE} .rbt-board"))?, 1)?;
    check_eq(attribute(&tab, &format!("{STAGE} .rbt-scene"), "id")?, scene)?;
    check_eq(digest(&tab)?, before)?;
    check_eq(count(&tab, "#rbt-staging svg > *")?, 0)?;
    click_button(&tab, "next")?;
    check_eq(position(&tab)?, 13)?;
    check(banner_hidden(&tab)?, "repaired")
}

#[test]
fn a_focus_that_cannot_be_put_back_is_reported_and_the_next_action_repairs_it_even_at_an_end() -> Result<(), String> {
    let tab = open()?;
    seek(&tab, 0)?;
    click_button(&tab, "jump-2")?;
    focus_item(&tab, 5)?;
    fail_focus(&tab, true)?;
    click_button(&tab, "next")?;
    fail_focus(&tab, false)?;
    check(!banner_hidden(&tab)?, "reported")?;
    let at = position(&tab)?;
    // The step did not advance in the model, though the tree on the page is the next one.
    click_button(&tab, "restart")?;
    check_eq(position(&tab)?, 0)?;
    click_button(&tab, "previous")?; // nowhere to go, and still draws a page that is out of date
    check(banner_hidden(&tab)?, "repaired by a button with nowhere to go")?;
    check_eq(position(&tab)?, 0)?;
    check_eq(count(&tab, &format!("{STAGE} [role=treeitem]"))?, 0)?;
    check(at > 0, "it had been somewhere")
}

#[test]
fn a_play_label_that_cannot_be_written_leaves_the_page_dirty_and_a_boundary_press_repairs_it() -> Result<(), String> {
    let tab = open_manual()?;
    click_button(&tab, "play")?;
    check_eq(content(&tab, "[data-action=\"play\"]")?, "Pause".to_owned())?;
    fail_writes_of(&tab, "aria-label", true)?;
    click_button(&tab, "play")?;
    fail_writes_of(&tab, "aria-label", false)?;
    check_eq(waiting(&tab)?, 0)?;
    check(!banner_hidden(&tab)?, "reported")?;
    click_button(&tab, "previous")?; // the run is at its first step: nowhere to go
    check(banner_hidden(&tab)?, "repaired")?;
    check_eq(live(&tab)?, "Already at the first step.".to_owned())?;
    click_button(&tab, "play")?;
    check_eq(waiting(&tab)?, 1)
}

#[test]
fn a_ring_that_cannot_be_written_is_reported_and_repaired_by_the_next_step() -> Result<(), String> {
    let tab = open()?;
    click_button(&tab, "jump-1")?;
    fail_writes_of(&tab, "aria-selected", true)?;
    click_item(&tab, 10)?;
    fail_writes_of(&tab, "aria-selected", false)?;
    check(!banner_hidden(&tab)?, "reported")?;
    click_button(&tab, "next")?;
    check(banner_hidden(&tab)?, "repaired")?;
    check_eq(
        attribute(&tab, &format!("{STAGE} [aria-selected=\"true\"]"), "data-rbt-key")?,
        "10".to_owned(),
    )
}
