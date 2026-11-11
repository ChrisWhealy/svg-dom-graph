//! The tree as the accessibility tree has it, and how the arrow, tab and click keys move the focus.

use super::*;

// The semantic tree.

#[test]
fn the_tree_has_truthful_roles_levels_positions_names_and_one_tab_stop() -> Result<(), String> {
    let tab = open()?;
    click_button(&tab, "jump-2")?;
    let rows = json(
        &tab,
        r#"(() => JSON.stringify([...document.querySelectorAll('#rbt-stage [role=treeitem]')].map(g => [
            +g.getAttribute('data-rbt-key'), g.getAttribute('aria-level'), g.getAttribute('aria-posinset'),
            g.getAttribute('aria-setsize'), g.getAttribute('aria-expanded'), g.getAttribute('aria-label'),
            g.getAttribute('aria-selected')])))()"#,
    )?;
    let row = |key: u64| -> Result<Vec<Option<String>>, String> {
        rows.as_array()
            .and_then(|a| a.iter().find(|r| r[0].as_u64() == Some(key)))
            .map(|r| {
                r.as_array()
                    .map(|c| c.iter().map(|v| v.as_str().map(str::to_owned)).collect())
                    .unwrap_or_default()
            })
            .ok_or_else(|| format!("no item {key}"))
    };
    let s = |v: &str| Some(v.to_owned());
    // The tree at the choice of a successor: 5 has 1 and 15, 15 has 10 and 20, and 10 has the right child 12.
    check_eq(
        row(5)?[1..6].to_vec(),
        vec![s("1"), s("1"), s("1"), s("true"), s("5, black, root")],
    )?;
    check_eq(
        row(15)?[1..6].to_vec(),
        vec![s("2"), s("2"), s("2"), s("true"), s("15, red, right child of 5")],
    )?;
    check_eq(
        row(10)?[1..6].to_vec(),
        vec![s("3"), s("1"), s("2"), s("true"), s("10, black, left child of 15")],
    )?;
    check_eq(
        row(20)?[1..6].to_vec(),
        vec![s("3"), s("2"), s("2"), None, s("20, black, right child of 15")],
    )?;
    check_eq(
        row(12)?[1..6].to_vec(),
        vec![s("4"), s("1"), s("1"), None, s("12, red, right child of 10")],
    )?;
    check_eq(count(&tab, &format!("{STAGE} .rbt-overlay [tabindex=\"0\"]"))?, 1)?;
    check_eq(
        attribute(&tab, &format!("{STAGE} .rbt-overlay [tabindex=\"0\"]"), "data-rbt-key")?,
        "5".to_owned(),
    )?;
    // Only a key with children holds a group: 5, 15 and 10.
    check_eq(count(&tab, &format!("{STAGE} [role=group]"))?, 3)?;
    check_eq(count(&tab, &format!("{STAGE} .rbt-scene [tabindex]"))?, 0)?;
    check_eq(count(&tab, &format!("{STAGE} [role=application]"))?, 0)
}

#[test]
fn every_item_has_aria_selected_true_or_false() -> Result<(), String> {
    let tab = open()?;
    click_button(&tab, "jump-1")?;
    check_eq(count(&tab, &format!("{STAGE} [role=treeitem]:not([aria-selected])"))?, 0)?;
    check_eq(count(&tab, &format!("{STAGE} [aria-selected=\"true\"]"))?, 0)
}

// The keyboard.

#[test]
fn tab_enters_the_tree_once_and_leaves_it_and_the_arrows_go_where_they_should() -> Result<(), String> {
    let tab = open()?;
    click_button(&tab, "jump-2")?;
    let step = position(&tab)?;
    // From the range, one Tab lands on the tab stop, which is the root, and another leaves the tree.
    text(&tab, "(() => { document.getElementById('rbt-seek').focus(); return 'ok'; })()")?;
    press(&tab, "Tab")?;
    check_eq(focused(&tab)?, "key:5".to_owned())?;
    press(&tab, "Tab")?;
    check(!focused(&tab)?.starts_with("key:") && focused(&tab)? != "tree", &focused(&tab)?)?;
    // Back in the tree: 5 has the children 1 and 15.
    focus_item(&tab, 5)?;
    press(&tab, "ArrowRight")?;
    check_eq(focused(&tab)?, "key:1".to_owned())?;
    press(&tab, "ArrowRight")?;
    check_eq(focused(&tab)?, "key:1".to_owned())?; // a leaf goes nowhere
    press(&tab, "ArrowLeft")?;
    check_eq(focused(&tab)?, "key:5".to_owned())?;
    press(&tab, "ArrowLeft")?;
    check_eq(focused(&tab)?, "key:5".to_owned())?; // the root has no parent
    press(&tab, "ArrowDown")?;
    check_eq(focused(&tab)?, "key:10".to_owned())?; // sorted order: 1, 5, 10, 12, 15, 20
    press(&tab, "ArrowDown")?;
    press(&tab, "ArrowDown")?;
    check_eq(focused(&tab)?, "key:15".to_owned())?;
    press(&tab, "ArrowRight")?;
    check_eq(focused(&tab)?, "key:10".to_owned())?; // two children: the left first
    press(&tab, "ArrowUp")?;
    check_eq(focused(&tab)?, "key:5".to_owned())?;
    press(&tab, "End")?;
    check_eq(focused(&tab)?, "key:20".to_owned())?;
    press(&tab, "Home")?;
    check_eq(focused(&tab)?, "key:1".to_owned())?;
    check_eq(count(&tab, &format!("{STAGE} .rbt-overlay [tabindex=\"0\"]"))?, 1)?;
    check_eq(
        attribute(&tab, &format!("{STAGE} .rbt-overlay [tabindex=\"0\"]"), "data-rbt-key")?,
        "1".to_owned(),
    )?;
    check_eq(position(&tab)?, step)
}

#[test]
fn an_arrow_key_whose_focus_cannot_move_keeps_the_tab_stop_where_the_focus_is() -> Result<(), String> {
    let tab = open()?;
    click_button(&tab, "jump-2")?;
    focus_item(&tab, 5)?;
    fail_focus(&tab, true)?;
    press(&tab, "ArrowRight")?;
    fail_focus(&tab, false)?;
    check_eq(focused(&tab)?, "key:5".to_owned())?;
    check(!banner_hidden(&tab)?, "reported")?;
    check(
        content(&tab, "#rbt-error")?.contains("could not be moved to 1"),
        &content(&tab, "#rbt-error")?,
    )?;
    check_eq(count(&tab, &format!("{STAGE} .rbt-overlay [tabindex=\"0\"]"))?, 1)?;
    check_eq(
        attribute(&tab, &format!("{STAGE} .rbt-overlay [tabindex=\"0\"]"), "data-rbt-key")?,
        "5".to_owned(),
    )?;
    // The next key moves from where the focus really is, to 1, and not from the key that the failed move aimed at.
    press(&tab, "ArrowRight")?;
    check_eq(focused(&tab)?, "key:1".to_owned())?;
    check(banner_hidden(&tab)?, "a move that works clears it")?;
    press(&tab, "ArrowDown")?;
    check_eq(focused(&tab)?, "key:5".to_owned())
}

#[test]
fn a_click_that_selects_another_key_moves_the_focus_there_and_the_next_arrow_goes_on_from_it() -> Result<(), String> {
    let tab = open()?;
    click_button(&tab, "jump-1")?;
    focus_item(&tab, 5)?;
    // A synthetic click does not move the focus the way a real one does, so the page has to.
    click_item(&tab, 20)?;
    check_eq(focused(&tab)?, "key:20".to_owned())?;
    check_eq(
        attribute(&tab, &format!("{STAGE} [aria-selected=\"true\"]"), "data-rbt-key")?,
        "20".to_owned(),
    )?;
    check_eq(
        attribute(&tab, &format!("{STAGE} .rbt-overlay [tabindex=\"0\"]"), "data-rbt-key")?,
        "20".to_owned(),
    )?;
    press(&tab, "ArrowUp")?;
    check_eq(focused(&tab)?, "key:15".to_owned())
}

#[test]
fn a_click_whose_focus_cannot_move_selects_but_leaves_the_tab_stop_on_the_focused_key() -> Result<(), String> {
    let tab = open()?;
    click_button(&tab, "jump-1")?;
    focus_item(&tab, 5)?;
    fail_focus(&tab, true)?;
    click_item(&tab, 20)?;
    fail_focus(&tab, false)?;
    check_eq(focused(&tab)?, "key:5".to_owned())?;
    check_eq(
        attribute(&tab, &format!("{STAGE} [aria-selected=\"true\"]"), "data-rbt-key")?,
        "20".to_owned(),
    )?;
    check_eq(
        attribute(&tab, &format!("{STAGE} .rbt-overlay [tabindex=\"0\"]"), "data-rbt-key")?,
        "5".to_owned(),
    )?;
    check(!banner_hidden(&tab)?, "reported")?;
    press(&tab, "ArrowRight")?;
    check_eq(focused(&tab)?, "key:1".to_owned())
}

#[test]
fn a_focus_that_is_not_at_the_tab_stop_stays_with_its_key_through_a_step() -> Result<(), String> {
    let tab = open()?;
    click_button(&tab, "jump-1")?;
    // The tab stop is on the root, 5. The focus is moved to 10 without the page hearing of it.
    text(
        &tab,
        "(() => { window.__mute = e => e.stopPropagation(); document.addEventListener('focusin', window.__mute, true); \
         return 'ok'; })()",
    )?;
    focus_item(&tab, 10)?;
    text(
        &tab,
        "(() => { document.removeEventListener('focusin', window.__mute, true); return 'ok'; })()",
    )?;
    check_eq(
        attribute(&tab, &format!("{STAGE} .rbt-overlay [tabindex=\"0\"]"), "data-rbt-key")?,
        "5".to_owned(),
    )?;
    click_button(&tab, "next")?;
    check_eq(focused(&tab)?, "key:10".to_owned())?;
    // The keys go on from the key that has the focus, and not from the tab stop, which is still on the root.
    press(&tab, "ArrowDown")?;
    check_eq(focused(&tab)?, "key:15".to_owned())?;
    check_eq(
        attribute(&tab, &format!("{STAGE} .rbt-overlay [tabindex=\"0\"]"), "data-rbt-key")?,
        "15".to_owned(),
    )
}

#[test]
fn the_right_child_of_a_node_with_two_children_is_reached_with_down() -> Result<(), String> {
    let tab = open()?;
    click_button(&tab, "jump-2")?;
    focus_item(&tab, 15)?;
    press(&tab, "ArrowRight")?;
    check_eq(focused(&tab)?, "key:10".to_owned())?;
    // Down from 10 goes to 12 in sorted order, and then 15 and 20: the right child 20 is two Downs from 15's left.
    focus_item(&tab, 15)?;
    press(&tab, "ArrowDown")?;
    check_eq(focused(&tab)?, "key:20".to_owned())
}

#[test]
fn enter_and_space_select_the_focused_key_and_navigation_never_does() -> Result<(), String> {
    let tab = open()?;
    click_button(&tab, "jump-1")?;
    let step = position(&tab)?;
    focus_item(&tab, 5)?;
    press(&tab, "ArrowRight")?;
    check_eq(count(&tab, &format!("{STAGE} [aria-selected=\"true\"]"))?, 0)?;
    press(&tab, "Enter")?;
    check_eq(count(&tab, &format!("{STAGE} [aria-selected=\"true\"]"))?, 1)?;
    check_eq(
        attribute(&tab, &format!("{STAGE} [aria-selected=\"true\"]"), "data-rbt-key")?,
        "1".to_owned(),
    )?;
    check_eq(field(&tab, "selected")?, "1, black".to_owned())?;
    press(&tab, "ArrowLeft")?;
    press(&tab, " ")?;
    check_eq(
        attribute(&tab, &format!("{STAGE} [aria-selected=\"true\"]"), "data-rbt-key")?,
        "5".to_owned(),
    )?;
    check(live(&tab)?.contains("Selected 5, black, root"), &live(&tab)?)?;
    check_eq(position(&tab)?, step)
}

#[test]
fn the_navigation_keys_do_not_scroll_the_page() -> Result<(), String> {
    let tab = open()?;
    click_button(&tab, "jump-1")?;
    focus_item(&tab, 5)?;
    let prevented = text(
        &tab,
        r#"(() => { const out = []; for (const key of ['ArrowDown', 'Home', 'End', ' ', 'a', 'Tab']) {
            const e = new KeyboardEvent('keydown', { key, bubbles: true, cancelable: true });
            document.querySelector('#rbt-stage [data-rbt-key="5"]').dispatchEvent(e); out.push(e.defaultPrevented); }
            return JSON.stringify(out); })()"#,
    )?;
    check_eq(prevented, "[true,true,true,true,false,false]".to_owned())
}
