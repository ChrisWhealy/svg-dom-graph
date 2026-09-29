//! Showing, hiding, and placing the button bar, and its buttons by click and by keyboard: existence, sibling
//! ordering, edge placement, activation, disabled state at the zoom limits, and option validation.

use super::support::*;
use crate::common::{check, check_close, group_translate};
use svg_dom::root::utils::{Point, Size};
use svg_dom_graph::{
    Error,
    scene::{Side, ToolbarOptions},
};
use wasm_bindgen_test::*;

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[wasm_bindgen_test]
fn there_is_no_toolbar_until_one_is_shown_and_none_after_it_is_hidden() -> Result<(), String> {
    let scene = new_scene("tb-show-hide")?;
    check(!scene.has_toolbar(), "a new scene already has a toolbar")?;
    check(
        query("#tb-show-hide > [role=\"toolbar\"]")?.is_none(),
        "a toolbar is in the DOM before being shown",
    )?;

    scene.show_toolbar(ToolbarOptions::default()).map_err(|e| e.to_string())?;
    check(scene.has_toolbar(), "has_toolbar is false after show_toolbar")?;
    bar("tb-show-hide")?;

    scene.hide_toolbar();
    check(!scene.has_toolbar(), "has_toolbar is true after hide_toolbar")?;
    check(
        query("#tb-show-hide > [role=\"toolbar\"]")?.is_none(),
        "the toolbar is still in the DOM after hide_toolbar",
    )
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Showing a second time replaces the first, rather than stacking a second bar.
#[wasm_bindgen_test]
fn showing_a_toolbar_twice_leaves_exactly_one() -> Result<(), String> {
    let scene = new_scene("tb-twice")?;
    scene.show_toolbar(ToolbarOptions::default()).map_err(|e| e.to_string())?;
    scene
        .show_toolbar(ToolbarOptions::new(Side::South))
        .map_err(|e| e.to_string())?;

    let count = web_sys::window()
        .and_then(|w| w.document())
        .ok_or("no document")?
        .query_selector_all("#tb-twice > [role=\"toolbar\"]")
        .map_err(|e| format!("{e:?}"))?
        .length();
    check(count == 1, &format!("expected one toolbar, found {count}"))
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The bar and the content layer are siblings, and the bar comes later so it draws on top.
#[wasm_bindgen_test]
fn the_toolbar_is_a_sibling_after_the_content_layer_not_inside_it() -> Result<(), String> {
    let scene = new_scene("tb-sibling")?;
    scene
        .add_node(Point::new(10.0, 10.0), Size::new(60.0, 30.0), "A")
        .map_err(|e| e.to_string())?;
    scene.show_toolbar(ToolbarOptions::default()).map_err(|e| e.to_string())?;

    check(
        query("#tb-sibling > g.svg-dom-graph-content [role=\"toolbar\"]")?.is_none(),
        "the toolbar is inside the content layer",
    )?;
    let content = content("tb-sibling")?;
    let next = content.next_element_sibling().ok_or("nothing follows the content layer")?;
    check(
        next.get_attribute("role").as_deref() == Some("toolbar"),
        "the toolbar does not follow the content layer",
    )
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// With default options a horizontal bar is 134 wide (28 + 4 + 28 + 4 + 70) and 28 tall. A vertical bar is 70 wide, as
/// wide as its widest button, and 92 tall (28 + 4 + 28 + 4 + 28). Every bar is inset by 8 and centred along its edge.
#[wasm_bindgen_test]
fn each_edge_places_the_bar_against_that_edge_of_the_view_box() -> Result<(), String> {
    let scene = new_scene("tb-edges")?;
    let expected = [
        (Side::North, (133.0, 8.0)),
        (Side::South, (133.0, 264.0)),
        (Side::West, (8.0, 104.0)),
        (Side::East, (322.0, 104.0)),
    ];

    scene.show_toolbar(ToolbarOptions::default()).map_err(|e| e.to_string())?;
    for (edge, (x, y)) in expected {
        scene.set_toolbar_edge(edge).map_err(|e| e.to_string())?;
        let (got_x, got_y) = group_translate(&bar("tb-edges")?)?;
        check_close(got_x, x)?;
        check_close(got_y, y)?;
    }
    Ok(())
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Zooming scales the content layer only. The bar's own position, and every button's size, never change.
#[wasm_bindgen_test]
fn zooming_changes_the_content_layer_but_never_the_toolbar() -> Result<(), String> {
    let scene = new_scene("tb-fixed")?;
    scene
        .add_node(Point::new(10.0, 10.0), Size::new(60.0, 30.0), "A")
        .map_err(|e| e.to_string())?;
    scene.show_toolbar(ToolbarOptions::default()).map_err(|e| e.to_string())?;

    let bar_before = attr(&bar("tb-fixed")?, "transform")?;
    let plus_before = (attr(&button("tb-fixed", 0)?.first_element_child().ok_or("no rect")?, "width")?,);

    scene.zoom_in().map_err(|e| e.to_string())?;
    scene.zoom_in().map_err(|e| e.to_string())?;

    check(
        attr(&content("tb-fixed")?, "transform")?.contains("scale(1.5625)"),
        "the content did not scale",
    )?;
    check(
        attr(&bar("tb-fixed")?, "transform")? == bar_before,
        "the toolbar's own transform changed",
    )?;
    let plus_after = (attr(&button("tb-fixed", 0)?.first_element_child().ok_or("no rect")?, "width")?,);
    check(plus_before == plus_after, "a button's size changed with the zoom")
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Zooming keeps the centre of the visible area fixed: at scale 1.25 about (200, 150), the translation is
/// (200 - 200 * 1.25, 150 - 150 * 1.25) = (-50, -37.5).
#[wasm_bindgen_test]
fn zoom_in_keeps_the_centre_of_the_visible_area_fixed() -> Result<(), String> {
    let scene = new_scene("tb-pivot")?;
    scene.zoom_in().map_err(|e| e.to_string())?;
    let transform = attr(&content("tb-pivot")?, "transform")?;
    check(
        transform == "translate(-50, -37.5) scale(1.25)",
        &format!("unexpected transform: {transform}"),
    )
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[wasm_bindgen_test]
fn the_buttons_zoom_and_reset_by_click() -> Result<(), String> {
    let scene = new_scene("tb-click")?;
    scene.show_toolbar(ToolbarOptions::default()).map_err(|e| e.to_string())?;

    click(&button("tb-click", 0)?)?; // +
    check_close(scene.zoom_scale(), 1.25)?;
    click(&button("tb-click", 1)?)?; // −
    check_close(scene.zoom_scale(), 1.0)?;
    click(&button("tb-click", 0)?)?;
    click(&button("tb-click", 2)?)?; // Reset
    check_close(scene.zoom_scale(), 1.0)?;
    let transform = attr(&content("tb-click")?, "transform")?;
    check(transform == "translate(0, 0) scale(1)", &format!("reset left {transform}"))
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[wasm_bindgen_test]
fn enter_and_space_activate_a_button_and_other_keys_do_not() -> Result<(), String> {
    let scene = new_scene("tb-keys")?;
    scene.show_toolbar(ToolbarOptions::default()).map_err(|e| e.to_string())?;
    let plus = button("tb-keys", 0)?;

    keydown(&plus, "Enter")?;
    check_close(scene.zoom_scale(), 1.25)?;
    keydown(&plus, " ")?;
    check_close(scene.zoom_scale(), 1.5625)?;
    keydown(&plus, "a")?;
    keydown(&plus, "Tab")?;
    check_close(scene.zoom_scale(), 1.5625)
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Every button is reachable and named for assistive technology, and the bar itself is a labelled toolbar.
#[wasm_bindgen_test]
fn buttons_are_focusable_named_and_the_bar_reports_its_orientation() -> Result<(), String> {
    let scene = new_scene("tb-a11y")?;
    scene.show_toolbar(ToolbarOptions::default()).map_err(|e| e.to_string())?;

    let names = ["Zoom in", "Zoom out", "Reset zoom"];
    for (n, name) in names.iter().enumerate() {
        let b = button("tb-a11y", n)?;
        check(attr(&b, "tabindex")? == "0", "a button is not focusable")?;
        check(
            attr(&b, "aria-label")? == *name,
            &format!("button {n} has the wrong accessible name"),
        )?;
    }
    check(
        attr(&bar("tb-a11y")?, "aria-orientation")? == "horizontal",
        "a North bar is not horizontal",
    )?;
    scene.set_toolbar_edge(Side::East).map_err(|e| e.to_string())?;
    check(
        attr(&bar("tb-a11y")?, "aria-orientation")? == "vertical",
        "an East bar is not vertical",
    )
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// A button that could do nothing says so: dimmed, and `aria-disabled`.
#[wasm_bindgen_test]
fn a_button_at_its_limit_reports_aria_disabled() -> Result<(), String> {
    let scene = new_scene("tb-limits")?;
    scene.show_toolbar(ToolbarOptions::default()).map_err(|e| e.to_string())?;

    // Unzoomed: Reset has nothing to do; both zoom buttons do.
    check(
        attr(&button("tb-limits", 2)?, "aria-disabled")? == "true",
        "Reset is enabled when unzoomed",
    )?;
    check(
        attr(&button("tb-limits", 0)?, "aria-disabled")? == "false",
        "zoom-in is disabled when unzoomed",
    )?;

    for _ in 0..30 {
        scene.zoom_in().map_err(|e| e.to_string())?;
    }
    check_close(scene.zoom_scale(), 4.0)?;
    check(
        attr(&button("tb-limits", 0)?, "aria-disabled")? == "true",
        "zoom-in is enabled at maximum zoom",
    )?;
    check(
        attr(&button("tb-limits", 2)?, "aria-disabled")? == "false",
        "Reset is disabled while zoomed",
    )?;

    for _ in 0..60 {
        scene.zoom_out().map_err(|e| e.to_string())?;
    }
    check_close(scene.zoom_scale(), 0.25)?;
    check(
        attr(&button("tb-limits", 1)?, "aria-disabled")? == "true",
        "zoom-out is enabled at minimum zoom",
    )
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[wasm_bindgen_test]
fn invalid_toolbar_options_are_rejected_and_leave_an_existing_toolbar_alone() -> Result<(), String> {
    let scene = new_scene("tb-invalid")?;
    scene.show_toolbar(ToolbarOptions::default()).map_err(|e| e.to_string())?;

    for change in [
        (|o: &mut ToolbarOptions| o.button_height = 0.0) as fn(&mut ToolbarOptions),
        |o| o.button_height = f64::NAN,
        |o| o.gap = -1.0,
        |o| o.margin = f64::INFINITY,
    ] {
        let mut bad = ToolbarOptions::default();
        change(&mut bad);
        match scene.show_toolbar(bad) {
            Err(Error::InvalidToolbarOptions(_)) => {},
            other => return Err(format!("{bad:?} was not rejected: {other:?}")),
        }
    }
    check(scene.has_toolbar(), "a rejected call removed the existing toolbar")?;
    bar("tb-invalid")?;
    Ok(())
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Dropping the last `Scene` handle silences the buttons rather than leaking or panicking.
#[wasm_bindgen_test]
fn a_button_does_nothing_once_every_scene_handle_is_dropped() -> Result<(), String> {
    let scene = new_scene("tb-dropped")?;
    scene.show_toolbar(ToolbarOptions::default()).map_err(|e| e.to_string())?;
    let plus = button("tb-dropped", 0)?;
    drop(scene);
    click(&plus)?;
    keydown(&plus, "Enter")
}
