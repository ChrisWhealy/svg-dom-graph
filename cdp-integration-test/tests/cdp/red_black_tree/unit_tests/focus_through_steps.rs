//! Where the focus goes when its key is rotated, deleted, replaced or not yet inserted.

use super::*;

// The selection.

#[test]
fn a_selected_15_shows_as_20_after_the_deletion_and_comes_back_on_reversing() -> Result<(), String> {
    let tab = open()?;
    click_button(&tab, "jump-2")?;
    click_item(&tab, 15)?;
    check_eq(
        attribute(&tab, &format!("{STAGE} [aria-selected=\"true\"]"), "data-rbt-key")?,
        "15".to_owned(),
    )?;
    click_button(&tab, "next")?;
    check_eq(
        attribute(&tab, &format!("{STAGE} [aria-selected=\"true\"]"), "data-rbt-key")?,
        "20".to_owned(),
    )?;
    check(
        field(&tab, "note")?.contains("took the place of your selected 15"),
        &field(&tab, "note")?,
    )?;
    check_eq(field(&tab, "selected")?, "20, red".to_owned())?;
    let last = last_step(&tab)?;
    seek(&tab, last)?;
    check_eq(
        attribute(&tab, &format!("{STAGE} [aria-selected=\"true\"]"), "data-rbt-key")?,
        "20".to_owned(),
    )?;
    click_button(&tab, "jump-2")?;
    check_eq(
        attribute(&tab, &format!("{STAGE} [aria-selected=\"true\"]"), "data-rbt-key")?,
        "15".to_owned(),
    )?;
    check_eq(field(&tab, "note")?, "\u{2014}".to_owned())
}

#[test]
fn choosing_20_after_the_deletion_stops_the_return_to_15() -> Result<(), String> {
    let tab = open()?;
    click_button(&tab, "jump-2")?;
    click_item(&tab, 15)?;
    click_button(&tab, "next")?;
    click_item(&tab, 20)?;
    click_button(&tab, "previous")?;
    check_eq(
        attribute(&tab, &format!("{STAGE} [aria-selected=\"true\"]"), "data-rbt-key")?,
        "20".to_owned(),
    )?;
    check_eq(
        count(&tab, &format!("{STAGE} [data-rbt-key=\"15\"][aria-selected=\"true\"]"))?,
        0,
    )
}

#[test]
fn a_key_that_is_not_yet_inserted_falls_back_to_the_root() -> Result<(), String> {
    let tab = open()?;
    click_button(&tab, "jump-2")?;
    click_item(&tab, 12)?;
    seek(&tab, 5)?;
    check_eq(count(&tab, &format!("{STAGE} [aria-selected=\"true\"]"))?, 1)?;
    check(
        field(&tab, "note")?.contains("not in the tree at this step"),
        &field(&tab, "note")?,
    )?;
    click_button(&tab, "jump-2")?;
    check_eq(
        attribute(&tab, &format!("{STAGE} [aria-selected=\"true\"]"), "data-rbt-key")?,
        "12".to_owned(),
    )
}

// The focus.

#[test]
fn the_focus_follows_its_key_through_a_rotation() -> Result<(), String> {
    let tab = open()?;
    click_button(&tab, "jump-1")?; // 10 is a red leaf-side node here, and the next steps change its place
    focus_item(&tab, 10)?;
    let before = attribute(&tab, &format!("{STAGE} [data-rbt-key=\"10\"]"), "aria-label")?;
    click_button(&tab, "next")?;
    check_eq(focused(&tab)?, "key:10".to_owned())?;
    check_eq(position(&tab)?, 17)?;
    click_button(&tab, "previous")?;
    check_eq(focused(&tab)?, "key:10".to_owned())?;
    check_eq(
        attribute(&tab, &format!("{STAGE} [data-rbt-key=\"10\"]"), "aria-label")?,
        before,
    )
}

#[test]
fn focus_on_15_goes_to_20_when_15_is_deleted_and_back_to_15_on_reversing() -> Result<(), String> {
    let tab = open()?;
    click_button(&tab, "jump-2")?;
    focus_item(&tab, 15)?;
    click_button(&tab, "next")?;
    check_eq(focused(&tab)?, "key:20".to_owned())?;
    check_eq(
        attribute(&tab, &format!("{STAGE} .rbt-overlay [tabindex=\"0\"]"), "data-rbt-key")?,
        "20".to_owned(),
    )?;
    click_button(&tab, "previous")?;
    check_eq(focused(&tab)?, "key:15".to_owned())?;
    check_eq(
        attribute(&tab, &format!("{STAGE} .rbt-overlay [tabindex=\"0\"]"), "data-rbt-key")?,
        "15".to_owned(),
    )
}

#[test]
fn tabbing_onto_a_replacement_does_not_forget_the_key_it_replaced() -> Result<(), String> {
    let tab = open()?;
    click_button(&tab, "jump-2")?;
    focus_item(&tab, 15)?;
    click_button(&tab, "next")?;
    // The tab stop is on 20. Leave the tree and Tab back in: it lands on 20, and the intent stays 15.
    text(&tab, "(() => { document.getElementById('rbt-seek').focus(); return 'ok'; })()")?;
    press(&tab, "Tab")?;
    check_eq(focused(&tab)?, "key:20".to_owned())?;
    click_button(&tab, "previous")?;
    check_eq(focused(&tab)?, "key:15".to_owned())
}

#[test]
fn a_node_that_is_not_yet_inserted_sends_the_focus_to_the_root() -> Result<(), String> {
    let tab = open()?;
    click_button(&tab, "jump-2")?;
    focus_item(&tab, 12)?;
    // At step 5 only 10 and 5 are in the tree, and the root is 10.
    seek(&tab, 5)?;
    check_eq(focused(&tab)?, "key:10".to_owned())
}

#[test]
fn an_empty_tree_sends_the_focus_to_the_tree_itself() -> Result<(), String> {
    let tab = open()?;
    click_button(&tab, "jump-0")?;
    focus_item(&tab, 5)?;
    seek(&tab, 0)?;
    check_eq(focused(&tab)?, "tree".to_owned())
}

#[test]
fn a_button_keeps_the_focus_through_every_step_and_the_ends_say_so() -> Result<(), String> {
    let tab = open()?;
    text(
        &tab,
        "(() => { document.querySelector('[data-action=\"next\"]').focus(); return 'ok'; })()",
    )?;
    let button = text(&tab, "(() => { window.__next = document.activeElement; return 'ok'; })()")?;
    check_eq(button, "ok".to_owned())?;
    let last = last_step(&tab)?;
    for step in 1..=last {
        click_button(&tab, "next")?;
        check_eq(position(&tab)?, step)?;
        check_eq(
            text(&tab, "String(document.activeElement === window.__next)")?,
            "true".to_owned(),
        )
        .map_err(|e| format!("step {step}: {e}"))?;
    }
    click_button(&tab, "next")?;
    check_eq(live(&tab)?, "Already at the last step.".to_owned())?;
    check_eq(attribute(&tab, "[data-action=\"next\"]", "aria-disabled")?, "true".to_owned())?;
    check_eq(
        text(&tab, "String(document.activeElement === window.__next)")?,
        "true".to_owned(),
    )?;
    click_button(&tab, "restart")?;
    click_button(&tab, "previous")?;
    check_eq(live(&tab)?, "Already at the first step.".to_owned())
}

#[test]
fn nothing_steals_the_focus_from_elsewhere_on_the_page() -> Result<(), String> {
    let tab = open()?;
    text(&tab, "(() => { document.getElementById('rbt-seek').focus(); return 'ok'; })()")?;
    for _ in 0..4 {
        click_button(&tab, "next")?;
    }
    check_eq(focused(&tab)?, "id:rbt-seek".to_owned())
}
