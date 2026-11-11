//! A write or a draw that fails, and what is left on the page afterwards.

use super::*;

// Faults.

#[test]
fn a_control_that_cannot_be_written_puts_back_the_others_and_leaves_the_state() -> Result<(), String> {
    for name in ["aria-valuetext", "aria-disabled", "max"] {
        let tab = open()?;
        seek(&tab, 100)?;
        let before = (step(&tab)?, field(&tab, "status")?, digest(&tab)?, drawn_contours(&tab)?);
        fail_writes_of(&tab, name, true)?;
        click_button(&tab, "next")?;
        fail_writes_of(&tab, name, false)?;
        check(!banner_hidden(&tab)?, &format!("{name}: reported"))?;
        check_eq(
            (step(&tab)?, field(&tab, "status")?, digest(&tab)?, drawn_contours(&tab)?),
            before,
        )
        .map_err(|e| format!("{name}: {e}"))?;
        click_button(&tab, "next")?;
        check_eq(step(&tab)?, 101).map_err(|e| format!("{name}: {e}"))?;
        check(banner_hidden(&tab)?, &format!("{name}: cleared"))?;
    }
    Ok(())
}

#[test]
fn a_plot_that_cannot_be_drawn_leaves_the_old_plot_and_a_clean_page() -> Result<(), String> {
    let tab = open()?;
    seek(&tab, 100)?;
    let before = (digest(&tab)?, drawn_contours(&tab)?);
    // A polyline is made with a `points` attribute, so this stops `set_plot_items`, which draws apart and then swaps.
    fail_writes_of(&tab, "points", true)?;
    click_button(&tab, "next")?;
    fail_writes_of(&tab, "points", false)?;
    check(!banner_hidden(&tab)?, "reported")?;
    check_eq((step(&tab)?, digest(&tab)?, drawn_contours(&tab)?), (100, before.0, before.1))?;
    // The page was not touched, so a button with nowhere to go does not draw it, and the line stays.
    click_button(&tab, "retry")?;
    click_button(&tab, "next")?;
    check_eq(step(&tab)?, 101)?;
    check(banner_hidden(&tab)?, "cleared by a step that works")
}

#[test]
fn a_scene_that_cannot_be_built_keeps_the_old_one_and_the_next_try_gets_a_fresh_id() -> Result<(), String> {
    let tab = open()?;
    inspect_cell(&tab, 14, 12)?;
    zoom_to(&tab, 400)?;
    let old = attribute(&tab, &format!("{STAGE} .ms-scene"), "id")?;
    let view = view_transform(&tab)?;
    let (generation, attempt) = (footer_number(&tab, "generation")?, footer_number(&tab, "attempt")?);
    fail_writes_of(&tab, "viewBox", true)?;
    click_button(&tab, "rebuild")?;
    fail_writes_of(&tab, "viewBox", false)?;
    check(!banner_hidden(&tab)?, "reported")?;
    check_eq(
        (
            attribute(&tab, &format!("{STAGE} .ms-scene"), "id")?,
            view_transform(&tab)?,
            zoom_percent(&tab)?,
        ),
        (old.clone(), view.clone(), 400),
    )?;
    check_eq(
        (count(&tab, &format!("{STAGE} .ms-scene"))?, count(&tab, "#ms-staging svg > *")?),
        (1, 0),
    )?;
    click_button(&tab, "rebuild")?;
    let fresh = attribute(&tab, &format!("{STAGE} .ms-scene"), "id")?;
    check(fresh != old, "a new ID")?;
    // The failed try used a number of its own: the committed generation moved once, and the attempt twice.
    check_eq(
        (footer_number(&tab, "generation")?, footer_number(&tab, "attempt")?),
        (generation + 1, attempt + 2),
    )?;
    check_eq((view_transform(&tab)?, zoom_percent(&tab)?), (view, 400))?;
    check(banner_hidden(&tab)?, "cleared")
}

#[test]
fn a_swap_that_throws_before_or_after_it_changes_the_host_leaves_the_old_scene() -> Result<(), String> {
    for after in [false, true] {
        let tab = open()?;
        inspect_cell(&tab, 14, 12)?;
        zoom_to(&tab, 400)?;
        let old = attribute(&tab, &format!("{STAGE} .ms-scene"), "id")?;
        let before = (digest(&tab)?, drawn_contours(&tab)?, view_transform(&tab)?);
        fail_replacement(&tab, true, after)?;
        click_button(&tab, "rebuild")?;
        fail_replacement(&tab, false, after)?;
        check(!banner_hidden(&tab)?, &format!("after {after}: reported"))?;
        check_eq(attribute(&tab, &format!("{STAGE} .ms-scene"), "id")?, old.clone())
            .map_err(|e| format!("after {after}: {e}"))?;
        check_eq((digest(&tab)?, drawn_contours(&tab)?, view_transform(&tab)?), before)
            .map_err(|e| format!("after {after}: {e}"))?;
        check_eq(
            (count(&tab, &format!("{STAGE} .ms-scene"))?, count(&tab, "#ms-staging svg > *")?),
            (1, 0),
        )?;
        click_button(&tab, "rebuild")?;
        check(
            attribute(&tab, &format!("{STAGE} .ms-scene"), "id")? != old,
            "a fresh ID after the retry",
        )?;
        check(banner_hidden(&tab)?, "cleared")?;
        check_geometry(&tab, (14, 12), "after the retry")?;
    }
    Ok(())
}

#[test]
fn a_write_that_fails_after_a_swap_to_another_grid_puts_the_old_scene_back_with_its_axes_and_view() -> Result<(), String>
{
    let tab = open()?;
    inspect_cell(&tab, 14, 12)?;
    zoom_to(&tab, 400)?;
    let view = view_transform(&tab)?;
    let before = (digest(&tab)?, drawn_contours(&tab)?);
    check(has_ripple_axes(&tab)?, "the default field has its own axes")?;
    // The focus is in the scene, so that putting it back is a write that follows the swap, and it fails.
    text(
        &tab,
        &format!(
            "(() => {{ document.querySelector('{STAGE} .ms-scene [aria-label=\"Zoom in\"]').focus(); return 'ok'; }})()"
        ),
    )?;
    fail_focus(&tab, true)?;
    choose(&tab, "ms-field", "coarse")?;
    fail_focus(&tab, false)?;
    check(!banner_hidden(&tab)?, "reported")?;
    check(
        content(&tab, "#ms-error")?.contains("earlier scene was put back"),
        &content(&tab, "#ms-error")?,
    )?;
    // The committed field is still the old one, and so are the axes, the contours and the view.
    check_eq((value_of(&tab, "ms-field")?, step(&tab)?), ("ripple".to_owned(), CELLS))?;
    check(has_ripple_axes(&tab)?, "the old scene's axes")?;
    check_eq(
        (digest(&tab)?, drawn_contours(&tab)?, view_transform(&tab)?, zoom_percent(&tab)?),
        (before.0.clone(), before.1.clone(), view.clone(), 400),
    )?;
    check_eq(
        (count(&tab, &format!("{STAGE} .ms-scene"))?, count(&tab, "#ms-staging svg > *")?),
        (1, 0),
    )?;
    // The page is dirty. Any action draws the committed state again first, and it is a step on the old scene.
    click_button(&tab, "retry")?;
    check(banner_hidden(&tab)?, "repaired")?;
    check(has_ripple_axes(&tab)?, "still the old axes")?;
    check_geometry(&tab, (14, 12), "after the repair")?;
    check_eq((view_transform(&tab)?, zoom_percent(&tab)?), (view, 400))?;
    choose(&tab, "ms-field", "coarse")?;
    check_eq(step(&tab)?, 48)?;
    check(!has_ripple_axes(&tab)?, "the switch works once nothing fails")
}

#[test]
fn a_swap_that_cannot_be_put_back_is_built_again_from_the_committed_field_with_its_view() -> Result<(), String> {
    let tab = open()?;
    inspect_cell(&tab, 14, 12)?;
    zoom_to(&tab, 400)?;
    let view = view_transform(&tab)?;
    let before = (digest(&tab)?, drawn_contours(&tab)?);
    text(
        &tab,
        &format!(
            "(() => {{ document.querySelector('{STAGE} .ms-scene [aria-label=\"Zoom in\"]').focus(); return 'ok'; }})()"
        ),
    )?;
    // The swap works, then the focus cannot be put back, and then the old scene cannot be put back either.
    fail_focus(&tab, true)?;
    fail_replacement_after(&tab, true, 1)?;
    choose(&tab, "ms-field", "coarse")?;
    fail_replacement_after(&tab, false, 0)?;
    fail_focus(&tab, false)?;
    check(!banner_hidden(&tab)?, "reported")?;
    check(
        content(&tab, "#ms-error")?.contains("could not be put back"),
        &content(&tab, "#ms-error")?,
    )?;
    check_eq((value_of(&tab, "ms-field")?, step(&tab)?), ("ripple".to_owned(), CELLS))?;
    check(
        !has_ripple_axes(&tab)?,
        "the scene on the page is the new one, for the wrong grid",
    )?;
    check_eq(count(&tab, "#ms-staging svg > *")?, 0)?;
    // The next action builds the committed field again, and does not draw its contours into the wrong axes.
    click_button(&tab, "retry")?;
    check(banner_hidden(&tab)?, "repaired")?;
    check(has_ripple_axes(&tab)?, "the committed field's axes")?;
    check_eq((digest(&tab)?, drawn_contours(&tab)?), before)?;
    check_geometry(&tab, (14, 12), "after the reconstruction")?;
    check_eq((view_transform(&tab)?, zoom_percent(&tab)?), (view, 400))?;
    check_eq(count(&tab, &format!("{STAGE} .ms-scene"))?, 1)?;
    choose(&tab, "ms-field", "coarse")?;
    check_eq(step(&tab)?, 48)
}

#[test]
fn a_write_that_fails_after_a_swap_to_a_grid_of_the_same_size_leaves_a_scene_that_a_step_can_repair()
-> Result<(), String> {
    let tab = open()?;
    inspect_cell(&tab, 14, 12)?;
    zoom_to(&tab, 400)?;
    let view = view_transform(&tab)?;
    text(
        &tab,
        &format!(
            "(() => {{ document.querySelector('{STAGE} .ms-scene [aria-label=\"Zoom in\"]').focus(); return 'ok'; }})()"
        ),
    )?;
    fail_focus(&tab, true)?;
    choose(&tab, "ms-field", "twin")?;
    fail_focus(&tab, false)?;
    check(!banner_hidden(&tab)?, "reported")?;
    check_eq(value_of(&tab, "ms-field")?, "ripple".to_owned())?;
    click_button(&tab, "retry")?;
    check(banner_hidden(&tab)?, "repaired")?;
    check_geometry(&tab, (14, 12), "after the repair")?;
    check_eq((view_transform(&tab)?, zoom_percent(&tab)?), (view, 400))
}

#[test]
fn an_inspector_that_cannot_be_drawn_leaves_the_page_dirty_and_a_boundary_press_repairs_it() -> Result<(), String> {
    let tab = open()?;
    click_button(&tab, "restart")?;
    fail_replacement(&tab, true, false)?;
    click_button(&tab, "next")?;
    fail_replacement(&tab, false, false)?;
    check(!banner_hidden(&tab)?, "reported")?;
    // The plot already shows the first cell, but the committed state is the empty one.
    check_eq(step(&tab)?, 1).or_else(|_| check_eq(step(&tab)?, 0))?;
    // At the first step, Previous has nowhere to go, and still draws the page that is out of date.
    click_button(&tab, "previous")?;
    check(banner_hidden(&tab)?, "repaired by a button with nowhere to go")?;
    check_eq(step(&tab)?, 0)?;
    check_eq(live(&tab)?, "Already at the first step.".to_owned())?;
    for slot in 0..3 {
        check_eq(polylines(&tab, slot)?, 0)?;
    }
    Ok(())
}
