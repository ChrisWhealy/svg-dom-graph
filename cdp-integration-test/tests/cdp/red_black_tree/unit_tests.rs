//! The tests of [`super`], which holds the helpers they use.

use super::*;

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
// The page.

#[test]
fn the_page_opens_at_the_empty_tree_with_the_picture_hidden_from_the_tree_of_roles() -> Result<(), String> {
    let tab = open()?;
    check_eq((position(&tab)?, last_step(&tab)?), (0, 29))?;
    check_eq(field(&tab, "state")?, "Settled: a valid red-black tree".to_owned())?;
    check_eq(count(&tab, &format!("{STAGE} [role=treeitem]"))?, 0)?;
    check_eq(attribute(&tab, &format!("{STAGE} .rbt-overlay"), "role")?, "tree".to_owned())?;
    check_eq(
        attribute(&tab, &format!("{STAGE} .rbt-overlay"), "aria-label")?,
        "Red-black tree, empty".to_owned(),
    )?;
    check_eq(
        attribute(&tab, &format!("{STAGE} .rbt-scene"), "aria-hidden")?,
        "true".to_owned(),
    )?;
    for (action, disabled) in [("restart", "true"), ("previous", "true"), ("next", "false"), ("play", "false")] {
        check_eq(
            attribute(&tab, &format!("[data-action=\"{action}\"]"), "aria-disabled")?,
            disabled.to_owned(),
        )?;
    }
    check(banner_hidden(&tab)?, "no error")?;
    check_eq(count(&tab, "#rbt-staging svg > *")?, 0)
}

#[test]
fn every_button_and_the_range_are_svg_in_the_stage_and_the_range_is_html_in_a_foreign_object() -> Result<(), String> {
    let tab = open()?;
    check_eq(count(&tab, "#panel-red-black-tree button")?, 0)?;
    check_eq(count(&tab, "#rbt-stage-svg g.rbt-controls > [role=button][data-action]")?, 7)?;
    check_eq(count(&tab, "#rbt-stage-svg g.rbt-controls > [role=button][tabindex]")?, 7)?;
    check_eq(count(&tab, "#rbt-stage-svg foreignObject input[type=\"range\"]#rbt-seek")?, 1)?;
    check_eq(count(&tab, "#panel-red-black-tree input")?, 1)?;
    // The controls are above the tree and outside the part that is swapped.
    let outside = text(
        &tab,
        "String(!document.querySelector('#rbt-stage .rbt-board-host').querySelector('[data-action], foreignObject'))",
    )?;
    check_eq(outside, "true".to_owned())?;
    let above = text(
        &tab,
        "(() => { const b = document.querySelector('[data-action=\"next\"]').getBoundingClientRect(); \
         const t = document.querySelector('#rbt-stage .rbt-scene').getBoundingClientRect(); \
         return String(b.bottom <= t.top); })()",
    )?;
    check_eq(above, "true".to_owned())?;
    // A swap does not replace them: the same elements are there after a step.
    text(
        &tab,
        "(() => { window.__b = document.querySelector('[data-action=\"next\"]'); window.__r = document.getElementById('rbt-seek'); return 'ok'; })()",
    )?;
    click_button(&tab, "next")?;
    check_eq(
        text(
            &tab,
            "String(window.__b === document.querySelector('[data-action=\"next\"]') && window.__r === document.getElementById('rbt-seek'))",
        )?,
        "true".to_owned(),
    )?;
    check_eq(attribute(&tab, "[data-action=\"play\"]", "aria-label")?, "Play".to_owned())
}

#[test]
fn every_button_label_fits_inside_its_button_with_room_at_each_end() -> Result<(), String> {
    let tab = open()?;
    let report = json(
        &tab,
        r#"(() => JSON.stringify([...document.querySelectorAll('#rbt-stage g.rbt-controls > [data-action]')].map(g => {
            const r = g.querySelector('rect').getBoundingClientRect(), t = g.querySelector('text').getBoundingClientRect();
            return [g.getAttribute('data-action'), t.left - r.left, r.right - t.right,
                parseFloat(g.querySelector('text').getAttribute('font-size'))];
        })))()"#,
    )?;
    let rows = report.as_array().ok_or("no report")?;
    check_eq(rows.len(), 7)?;
    for row in rows {
        let (name, left, right, size) = (
            &row[0],
            row[1].as_f64().unwrap_or(-1.0),
            row[2].as_f64().unwrap_or(-1.0),
            row[3].as_f64().unwrap_or(0.0),
        );
        check(
            left >= 4.0 && right >= 4.0,
            &format!("{name}: the label has {left:.1} and {right:.1} units of room"),
        )?;
        check(size >= 9.0, &format!("{name}: the label is too small to read: {size}"))?;
    }
    // The long ones are made smaller, and the short ones are left at the library's size.
    let size_of = |i: usize| rows[i][3].as_f64().unwrap_or(0.0);
    check(
        size_of(4) < 14.0 || size_of(5) < 14.0 || size_of(6) < 14.0,
        "a long label shrinks",
    )?;
    check_eq(size_of(0), 14.0)
}

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

#[test]
fn the_keyboard_help_is_visible_and_says_the_right_key_limit() -> Result<(), String> {
    let tab = open()?;
    let help = content(&tab, "#rbt-help")?;
    check(
        help.contains("Right goes to the first available child (left before right)"),
        &help,
    )?;
    check(help.contains("use Up/Down to reach the other child"), &help)
}

// The picture.

#[test]
fn every_step_draws_one_circle_per_key_and_one_connector_per_link_at_the_circles() -> Result<(), String> {
    let tab = open()?;
    let last = last_step(&tab)?;
    for step in 0..=last {
        seek(&tab, step)?;
        let keys = count(&tab, &format!("{STAGE} [role=treeitem]"))?;
        check_eq(count(&tab, &format!("{STAGE} .rbt-scene ellipse"))?, keys)
            .map_err(|e| format!("step {step}: {e}"))?;
        let links = drawn_links(&tab).map_err(|e| format!("step {step}: {e}"))?;
        check_eq(links.len(), keys.saturating_sub(1)).map_err(|e| format!("step {step}: {e}"))?;
        check_eq(links, tree_links(&tab)?).map_err(|e| format!("step {step}: the picture and the tree differ: {e}"))?;
        check(overlay_error(&tab)? < 0.6, &format!("step {step}: the tree is off its circles"))?;
        check_eq(count(&tab, "#rbt-staging svg > *")?, 0).map_err(|e| format!("step {step}: {e}"))?;
    }
    Ok(())
}

#[test]
fn the_checkpoints_have_exactly_the_golden_links() -> Result<(), String> {
    let tab = open()?;
    click_button(&tab, "jump-0")?;
    check_eq(drawn_links(&tab)?, vec![(5, 1), (5, 10)])?;
    check(field(&tab, "event")?.contains("Right-rotate at 10"), &field(&tab, "event")?)?;
    click_button(&tab, "jump-1")?;
    check_eq(drawn_links(&tab)?, vec![(5, 1), (5, 15), (15, 10), (15, 20)])?;
    check(field(&tab, "event")?.contains("Left-rotate at 10"), &field(&tab, "event")?)?;
    let last = last_step(&tab)?;
    seek(&tab, last)?;
    check_eq(drawn_links(&tab)?, vec![(5, 1), (5, 12), (12, 10), (12, 20)])?;
    check_eq(count(&tab, &format!("{STAGE} [data-rbt-key=\"15\"]"))?, 0)?;
    check(
        !content(&tab, &format!("{STAGE} .rbt-scene"))?.contains("15 "),
        "no circle says 15",
    )
}

#[test]
fn the_transplant_moves_the_original_node_and_the_old_connectors_are_gone() -> Result<(), String> {
    let tab = open()?;
    click_button(&tab, "jump-2")?;
    check_eq(drawn_links(&tab)?, vec![(5, 1), (5, 15), (10, 12), (15, 10), (15, 20)])?;
    click_button(&tab, "next")?;
    check(field(&tab, "event")?.contains("Transplant"), &field(&tab, "event")?)?;
    check_eq(
        field(&tab, "state")?,
        "Repair in progress: not yet a valid red-black tree".to_owned(),
    )?;
    check_eq(drawn_links(&tab)?, vec![(5, 1), (5, 20), (10, 12), (20, 10)])?;
    check_eq(
        attribute(&tab, &format!("{STAGE} [data-rbt-key=\"20\"]"), "data-rbt-colour")?,
        "red".to_owned(),
    )
}

#[test]
fn forward_backward_and_a_direct_seek_draw_the_same_step() -> Result<(), String> {
    let tab = open()?;
    let last = last_step(&tab)?;
    let mut forward = Vec::new();
    for step in 0..=last {
        seek(&tab, step)?;
        forward.push(digest(&tab)?);
    }
    for step in (0..=last).rev() {
        seek(&tab, step)?;
        check_eq(digest(&tab)?, forward[step].clone()).map_err(|e| format!("backward at {step}: {e}"))?;
    }
    for step in [17, 3, 28, 0, 22, 9, 29, 14] {
        seek(&tab, step)?;
        check_eq(digest(&tab)?, forward[step].clone()).map_err(|e| format!("direct to {step}: {e}"))?;
    }
    Ok(())
}

#[test]
fn a_black_node_and_a_red_node_say_their_colour_in_the_label_and_not_only_in_the_fill() -> Result<(), String> {
    let tab = open()?;
    click_button(&tab, "jump-1")?;
    let labels = text(
        &tab,
        r#"(() => JSON.stringify([...document.querySelectorAll('#rbt-stage .rbt-scene ellipse')].map(e =>
            [e.parentElement.querySelector('text').textContent, e.getAttribute('fill')]).sort()))()"#,
    )?;
    check(
        labels.contains("\"10 R\",\"#fecaca\"") && labels.contains("\"15 B\",\"#94a3b8\""),
        &labels,
    )
}

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

// The controls.

#[test]
fn the_three_jumps_land_just_after_their_event_and_the_construction_left_rotation_is_the_first() -> Result<(), String> {
    let tab = open()?;
    click_button(&tab, "jump-0")?;
    let a = position(&tab)?;
    check(field(&tab, "event")?.contains("Right-rotate at 10"), &field(&tab, "event")?)?;
    click_button(&tab, "jump-1")?;
    let b = position(&tab)?;
    check(field(&tab, "event")?.contains("Left-rotate at 10"), &field(&tab, "event")?)?;
    click_button(&tab, "jump-2")?;
    let c = position(&tab)?;
    check(field(&tab, "event")?.contains("successor"), &field(&tab, "event")?)?;
    check(a < b && b < c, "in order")?;
    check(live(&tab)?.starts_with(&format!("Step {c} of")), &live(&tab)?)
}

#[test]
fn the_range_goes_to_any_step_and_says_where_it_rests_once() -> Result<(), String> {
    let tab = open()?;
    text(
        &tab,
        "(() => { const r = document.getElementById('rbt-seek'); for (const v of ['3', '9', '12']) { r.value = v; \
         r.dispatchEvent(new Event('input', { bubbles: true })); } return 'ok'; })()",
    )?;
    check_eq(position(&tab)?, 12)?;
    check_eq(live(&tab)?, String::new())?;
    text(
        &tab,
        "(() => { document.getElementById('rbt-seek').dispatchEvent(new Event('change', { bubbles: true })); return 'ok'; })()",
    )?;
    check(live(&tab)?.starts_with("Step 12 of"), &live(&tab)?)
}

#[test]
fn a_transitional_step_says_repair_in_progress_and_a_finished_one_says_it_is_valid() -> Result<(), String> {
    let tab = open()?;
    click_button(&tab, "jump-2")?;
    click_button(&tab, "next")?;
    check(
        live(&tab)?.contains("Repair in progress, not yet a valid red-black tree"),
        &live(&tab)?,
    )?;
    let last = last_step(&tab)?;
    seek(&tab, last)?;
    click_button(&tab, "previous")?;
    click_button(&tab, "next")?;
    check(!live(&tab)?.contains("Repair in progress"), &live(&tab)?)?;
    check(
        live(&tab)?.contains("Delete 15 complete: a valid red-black tree of 5 keys, root 5"),
        &live(&tab)?,
    )
}

// Play.

#[test]
fn play_runs_one_tick_at_a_time_and_announces_only_the_big_moves() -> Result<(), String> {
    let tab = open_manual()?;
    click_button(&tab, "play")?;
    check_eq(
        (waiting(&tab)?, content(&tab, "[data-action=\"play\"]")?),
        (1, "Pause".to_owned()),
    )?;
    check_eq(attribute(&tab, "[data-action=\"play\"]", "aria-label")?, "Pause".to_owned())?;
    check_eq(advance(&tab, 1)?, 1)?;
    check_eq(position(&tab)?, 1)?;
    check_eq(live(&tab)?, String::new())?; // an insertion event is a small move
    let last = last_step(&tab)?;
    check_eq(advance(&tab, 1000)?, last - 1)?;
    check_eq((position(&tab)?, waiting(&tab)?), (last, 0))?;
    check_eq(content(&tab, "[data-action=\"play\"]")?, "Play".to_owned())?;
    check(live(&tab)?.contains("Delete 15 complete"), &live(&tab)?)
}

#[test]
fn pausing_says_where_the_run_is_and_a_stale_tick_does_nothing() -> Result<(), String> {
    let tab = open_manual()?;
    click_button(&tab, "play")?;
    advance(&tab, 1)?;
    click_button(&tab, "play")?;
    check_eq((waiting(&tab)?, position(&tab)?), (0, 1))?;
    check(live(&tab)?.starts_with("Step 1 of"), &live(&tab)?)?;
    check_eq(advance(&tab, 5)?, 0)?;
    check_eq(position(&tab)?, 1)
}

#[test]
fn every_other_control_stops_the_playback() -> Result<(), String> {
    for action in ["next", "previous", "restart", "jump-0"] {
        let tab = open_manual()?;
        seek(&tab, 4)?;
        click_button(&tab, "play")?;
        check_eq(waiting(&tab)?, 1)?;
        click_button(&tab, action)?;
        check_eq(
            (waiting(&tab)?, content(&tab, "[data-action=\"play\"]")?),
            (0, "Play".to_owned()),
        )
        .map_err(|e| format!("{action}: {e}"))?;
    }
    let tab = open_manual()?;
    click_button(&tab, "play")?;
    seek(&tab, 7)?;
    check_eq(waiting(&tab)?, 0)
}

#[test]
fn the_hooks_come_off_the_window_when_the_page_is_left() -> Result<(), String> {
    let tab = open_manual()?;
    check_eq(text(&tab, "typeof window.__rbtAdvance")?, "function".to_owned())?;
    text(&tab, "(() => { window.dispatchEvent(new Event('pagehide')); return 'ok'; })()")?;
    check_eq(
        text(&tab, "typeof window.__rbtAdvance + typeof window.__rbtPending")?,
        "undefinedundefined".to_owned(),
    )?;
    click_button(&tab, "next")?;
    check_eq(position(&tab)?, 0)
}

// Pan and zoom.

#[test]
fn pan_and_zoom_are_off_so_the_picture_cannot_leave_the_tree() -> Result<(), String> {
    let tab = open()?;
    click_button(&tab, "jump-1")?;
    let before = text(
        &tab,
        &format!(
            "document.querySelector('{STAGE} .svg-dom-graph-content').outerHTML.length + '|' + document.querySelector('{STAGE} .svg-dom-graph-content').getAttribute('transform')"
        ),
    )?;
    text(
        &tab,
        &format!(
            "(() => {{ const s = document.querySelector('{STAGE} .rbt-scene'); \
             s.dispatchEvent(new WheelEvent('wheel', {{ deltaY: -400, ctrlKey: true, bubbles: true, cancelable: true }})); \
             return 'ok'; }})()"
        ),
    )?;
    let after = text(
        &tab,
        &format!(
            "document.querySelector('{STAGE} .svg-dom-graph-content').outerHTML.length + '|' + document.querySelector('{STAGE} .svg-dom-graph-content').getAttribute('transform')"
        ),
    )?;
    check_eq(after, before)?;
    check_eq(overlay_error(&tab)? < 0.6, true)?;
    check_eq(count(&tab, &format!("{STAGE} [role=application]"))?, 0)
}

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
