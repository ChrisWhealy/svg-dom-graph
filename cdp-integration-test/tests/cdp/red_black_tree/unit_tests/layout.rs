//! What the page draws: the buttons, the help, the circles and links of each step, and the colours.

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
