//! Browser tests for `Scene`'s toolbar and zoom: showing, hiding, and moving the button bar, keeping it a fixed size
//! while the content zooms, the zoom buttons themselves (by click and by keyboard), and dragging under zoom.
//!
//! Every test uses a viewport and `viewBox` of `400 x 300`, 1:1, so client pixels and user-space units coincide.

use super::common::*;
use svg_dom::root::utils::{Point, Size};
use svg_dom_graph::{
    Error,
    scene::{Scene, Side, ToolbarOptions},
};
use wasm_bindgen_test::*;

fn new_scene(id: &str) -> Result<Scene, String> {
    Scene::new(make_svg(id, Size::new(400.0, 300.0), Size::new(400.0, 300.0))).map_err(|e| e.to_string())
}

fn query(selector: &str) -> Result<Option<web_sys::Element>, String> {
    web_sys::window()
        .and_then(|w| w.document())
        .ok_or("no document")?
        .query_selector(selector)
        .map_err(|e| format!("{e:?}"))
}

fn required(selector: &str) -> Result<web_sys::Element, String> {
    query(selector)?.ok_or_else(|| format!("nothing matches {selector}"))
}

fn bar(id: &str) -> Result<web_sys::Element, String> {
    required(&format!("#{id} > [role=\"toolbar\"]"))
}

fn content(id: &str) -> Result<web_sys::Element, String> {
    required(&format!("#{id} > g.svg-dom-graph-content"))
}

/// The `n`th toolbar button: 0 is "+", 1 is "−", 2 is "Reset".
fn button(id: &str, n: usize) -> Result<web_sys::Element, String> {
    required(&format!("#{id} > [role=\"toolbar\"] > [role=\"button\"]:nth-child({})", n + 1))
}

fn attr(element: &web_sys::Element, name: &str) -> Result<String, String> {
    element.get_attribute(name).ok_or_else(|| format!("missing attribute {name}"))
}

fn click(element: &web_sys::Element) -> Result<(), String> {
    let event = web_sys::MouseEvent::new("click").map_err(|e| format!("{e:?}"))?;
    element.dispatch_event(&event).map_err(|e| format!("{e:?}"))?;
    Ok(())
}

fn keydown(element: &web_sys::Element, key: &str) -> Result<(), String> {
    let init = web_sys::KeyboardEventInit::new();
    init.set_key(key);
    let event =
        web_sys::KeyboardEvent::new_with_keyboard_event_init_dict("keydown", &init).map_err(|e| format!("{e:?}"))?;
    element.dispatch_event(&event).map_err(|e| format!("{e:?}"))?;
    Ok(())
}

/// Resolves after the browser's next animation frame. Wheel zoom and panning write the DOM once per frame, so a test
/// that reads the rendered `transform` mid-gesture must wait for it.
async fn next_frame() -> Result<(), String> {
    let promise = js_sys::Promise::new(&mut |resolve, _reject| {
        if let Some(window) = web_sys::window() {
            let _ = window.request_animation_frame(&resolve);
        }
    });
    wasm_bindgen_futures::JsFuture::from(promise)
        .await
        .map(|_| ())
        .map_err(|e| format!("{e:?}"))
}

/// Parses a `translate(tx, ty) scale(s)` attribute into `(tx, ty, s)`, so a test can compare numbers with a tolerance
/// rather than exact text — repeated multiplication does not always land on the exact decimal.
fn parse_view(transform: &str) -> Result<(f64, f64, f64), String> {
    let numbers: Vec<f64> = transform
        .split(|c: char| !(c.is_ascii_digit() || c == '.' || c == '-' || c == 'e' || c == 'E'))
        .filter(|part| !part.is_empty())
        .filter_map(|part| part.parse().ok())
        .collect();
    match numbers[..] {
        [tx, ty, scale] => Ok((tx, ty, scale)),
        _ => Err(format!("cannot parse transform {transform:?}")),
    }
}

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

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Dragging converts client pixels through the node's own screen matrix, which includes the content layer's scale. So
/// at scale 1.25, a 25 pixel drag moves the node by 25 / 1.25 = 20 user-space units.
#[wasm_bindgen_test]
fn dragging_a_node_under_zoom_moves_it_by_the_delta_divided_by_the_scale() -> Result<(), String> {
    let scene = new_scene("tb-drag")?;
    let node = scene
        .add_node(Point::new(100.0, 100.0), Size::new(60.0, 30.0), "A")
        .map_err(|e| e.to_string())?;
    scene.make_draggable(node).map_err(|e| e.to_string())?;
    scene.zoom_in().map_err(|e| e.to_string())?;

    let group = nth_group("tb-drag", 0)?;
    dispatch_pointer_event(&group, "pointerdown", 200, 200, 1)?;
    dispatch_pointer_event(&group, "pointermove", 225, 250, 1)?;
    dispatch_pointer_event(&group, "pointerup", 225, 250, 1)?;

    let (x, y) = group_translate(&group)?;
    check_close(x, 120.0)?;
    check_close(y, 140.0)
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The zoom works without any toolbar, and nodes and edges added after a zoom land in the zoomed layer.
#[wasm_bindgen_test]
fn zoom_works_without_a_toolbar_and_new_nodes_join_the_zoomed_layer() -> Result<(), String> {
    let scene = new_scene("tb-headless")?;
    scene.zoom_in().map_err(|e| e.to_string())?;
    let a = scene
        .add_node(Point::new(0.0, 0.0), Size::new(60.0, 30.0), "A")
        .map_err(|e| e.to_string())?;
    let b = scene
        .add_node(Point::new(100.0, 0.0), Size::new(60.0, 30.0), "B")
        .map_err(|e| e.to_string())?;
    scene.add_edge(a, b).map_err(|e| e.to_string())?;

    check(
        nth_group("tb-headless", 1).is_ok(),
        "the second node is not in the content layer",
    )?;
    check(
        the_connector("tb-headless").is_ok(),
        "the connector is not in the content layer",
    )?;
    check(
        content("tb-headless")?.has_attribute("transform"),
        "the content layer carries no zoom",
    )?;
    Ok(())
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
fn pan_surface(id: &str) -> Result<web_sys::Element, String> {
    required(&format!("#{id} > rect"))
}

/// The pan surface exists only while a toolbar is shown, sits directly beneath the content layer, and covers the whole
/// visible area.
#[wasm_bindgen_test]
fn a_pan_surface_exists_only_while_the_toolbar_is_shown() -> Result<(), String> {
    let scene = new_scene("tb-pan-exists")?;
    check(
        query("#tb-pan-exists > rect")?.is_none(),
        "a pan surface exists before any toolbar",
    )?;

    scene.show_toolbar(ToolbarOptions::default()).map_err(|e| e.to_string())?;
    let surface = pan_surface("tb-pan-exists")?;
    let next = surface.next_element_sibling().ok_or("nothing follows the pan surface")?;
    check(
        next.get_attribute("class").as_deref() == Some("svg-dom-graph-content"),
        "the pan surface is not directly beneath the content layer",
    )?;
    check_close(attr_f64(&surface, "width")?, 400.0)?;
    check_close(attr_f64(&surface, "height")?, 300.0)?;

    scene.hide_toolbar();
    check(
        query("#tb-pan-exists > rect")?.is_none(),
        "the pan surface outlived the toolbar",
    )
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Dragging the background moves the content by the pointer's own movement on screen, and leaves the scale alone. At
/// scale 1.25 the zoom translation is (-50, -37.5), so a (30, -20) drag gives (-20, -57.5).
#[wasm_bindgen_test]
fn dragging_the_background_pans_the_content_at_any_zoom() -> Result<(), String> {
    let scene = new_scene("tb-pan")?;
    scene.show_toolbar(ToolbarOptions::default()).map_err(|e| e.to_string())?;
    let surface = pan_surface("tb-pan")?;

    dispatch_pointer_event(&surface, "pointerdown", 100, 100, 1)?;
    dispatch_pointer_event(&surface, "pointermove", 130, 80, 1)?;
    dispatch_pointer_event(&surface, "pointerup", 130, 80, 1)?;
    let transform = attr(&content("tb-pan")?, "transform")?;
    check(
        transform == "translate(30, -20) scale(1)",
        &format!("unexpected transform: {transform}"),
    )?;

    scene.reset_view().map_err(|e| e.to_string())?;
    scene.zoom_in().map_err(|e| e.to_string())?;
    dispatch_pointer_event(&surface, "pointerdown", 100, 100, 1)?;
    dispatch_pointer_event(&surface, "pointermove", 130, 80, 1)?;
    dispatch_pointer_event(&surface, "pointerup", 130, 80, 1)?;
    let transform = attr(&content("tb-pan")?, "transform")?;
    check(
        transform == "translate(-20, -57.5) scale(1.25)",
        &format!("unexpected transform: {transform}"),
    )
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Every move applies to where the pan started, so the content follows the pointer, and finishing a pan changes
/// nothing further.
#[wasm_bindgen_test]
async fn a_pan_follows_the_pointer_and_stops_when_it_is_released() -> Result<(), String> {
    let scene = new_scene("tb-pan-follow")?;
    scene.show_toolbar(ToolbarOptions::default()).map_err(|e| e.to_string())?;
    let surface = pan_surface("tb-pan-follow")?;

    dispatch_pointer_event(&surface, "pointerdown", 100, 100, 1)?;
    dispatch_pointer_event(&surface, "pointermove", 140, 100, 1)?;
    dispatch_pointer_event(&surface, "pointermove", 110, 100, 1)?;
    next_frame().await?;
    let transform = attr(&content("tb-pan-follow")?, "transform")?;
    check(
        transform == "translate(10, 0) scale(1)",
        &format!("the pan did not follow the pointer: {transform}"),
    )?;

    dispatch_pointer_event(&surface, "pointerup", 110, 100, 1)?;
    dispatch_pointer_event(&surface, "pointermove", 300, 250, 1)?;
    let transform = attr(&content("tb-pan-follow")?, "transform")?;
    check(
        transform == "translate(10, 0) scale(1)",
        &format!("a move after pointerup still panned: {transform}"),
    )
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// A second pointer cannot steal or end the pan, a cancel ends it, and only the primary button starts one.
#[wasm_bindgen_test]
async fn a_pan_ignores_a_second_pointer_and_a_non_primary_button_and_ends_on_cancel() -> Result<(), String> {
    let scene = new_scene("tb-pan-edge")?;
    scene.show_toolbar(ToolbarOptions::default()).map_err(|e| e.to_string())?;
    let surface = pan_surface("tb-pan-edge")?;
    // The content layer has no `transform` attribute until the view first changes.
    let transform = || -> Result<String, String> {
        Ok(content("tb-pan-edge")?
            .get_attribute("transform")
            .unwrap_or_else(|| "translate(0, 0) scale(1)".into()))
    };

    dispatch_pointer_event_with_button(&surface, "pointerdown", 100, 100, 1, 2)?;
    dispatch_pointer_event(&surface, "pointermove", 150, 100, 1)?;
    check(transform()? == "translate(0, 0) scale(1)", "a right-button drag panned")?;

    dispatch_pointer_event(&surface, "pointerdown", 100, 100, 1)?;
    dispatch_pointer_event(&surface, "pointerdown", 0, 0, 2)?;
    dispatch_pointer_event(&surface, "pointermove", 0, 90, 2)?;
    dispatch_pointer_event(&surface, "pointerup", 0, 90, 2)?;
    dispatch_pointer_event(&surface, "pointermove", 120, 100, 1)?;
    next_frame().await?;
    check(
        transform()? == "translate(20, 0) scale(1)",
        &format!("a second pointer interfered: {}", transform()?),
    )?;

    dispatch_pointer_event(&surface, "pointercancel", 120, 100, 1)?;
    dispatch_pointer_event(&surface, "pointermove", 200, 200, 1)?;
    next_frame().await?;
    check(
        transform()? == "translate(20, 0) scale(1)",
        "a move after pointercancel still panned",
    )
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Dragging a node drags only that node: the node's own pointer events never reach the surface behind it.
#[wasm_bindgen_test]
fn dragging_a_node_does_not_pan_the_content() -> Result<(), String> {
    let scene = new_scene("tb-pan-node")?;
    let node = scene
        .add_node(Point::new(100.0, 100.0), Size::new(60.0, 30.0), "A")
        .map_err(|e| e.to_string())?;
    scene.make_draggable(node).map_err(|e| e.to_string())?;
    scene.show_toolbar(ToolbarOptions::default()).map_err(|e| e.to_string())?;

    let group = nth_group("tb-pan-node", 0)?;
    dispatch_pointer_event(&group, "pointerdown", 110, 110, 1)?;
    dispatch_pointer_event(&group, "pointermove", 150, 130, 1)?;
    dispatch_pointer_event(&group, "pointerup", 150, 130, 1)?;

    check(
        attr(&content("tb-pan-node")?, "transform").is_err(),
        "dragging a node panned the content",
    )?;
    let (x, y) = group_translate(&group)?;
    check_close(x, 140.0)?;
    check_close(y, 120.0)
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Panning makes Reset useful again, and Reset restores both the pan and the zoom.
#[wasm_bindgen_test]
fn reset_undoes_a_pan_and_is_enabled_by_one() -> Result<(), String> {
    let scene = new_scene("tb-pan-reset")?;
    scene.show_toolbar(ToolbarOptions::default()).map_err(|e| e.to_string())?;
    check(
        attr(&button("tb-pan-reset", 2)?, "aria-disabled")? == "true",
        "Reset is enabled before any pan",
    )?;

    let surface = pan_surface("tb-pan-reset")?;
    dispatch_pointer_event(&surface, "pointerdown", 100, 100, 1)?;
    dispatch_pointer_event(&surface, "pointermove", 160, 100, 1)?;
    dispatch_pointer_event(&surface, "pointerup", 160, 100, 1)?;
    check(
        attr(&button("tb-pan-reset", 2)?, "aria-disabled")? == "false",
        "a pan did not enable Reset",
    )?;

    click(&button("tb-pan-reset", 2)?)?;
    check(
        attr(&content("tb-pan-reset")?, "transform")? == "translate(0, 0) scale(1)",
        "Reset left the pan in place",
    )
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Dispatches a bubbling, cancelable `wheel` event at `element` and reports whether a listener cancelled it — which is
/// what stops the browser zooming the whole page for a ctrl+wheel.
fn wheel(element: &web_sys::Element, x: i32, y: i32, delta_y: f64, ctrl: bool, meta: bool) -> Result<bool, String> {
    let init = web_sys::WheelEventInit::new();
    init.set_bubbles(true);
    init.set_cancelable(true);
    init.set_client_x(x);
    init.set_client_y(y);
    init.set_delta_y(delta_y);
    init.set_ctrl_key(ctrl);
    init.set_meta_key(meta);
    let event = web_sys::WheelEvent::new_with_event_init_dict("wheel", &init).map_err(|e| format!("{e:?}"))?;
    element.dispatch_event(&event).map_err(|e| format!("{e:?}"))?;
    Ok(event.default_prevented())
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Ctrl+wheel and Cmd+wheel both zoom, about the pointer. One notch up at (100, 100) is one 1.25 step, and the content
/// point under the pointer stays put: (100 - 100 * 1.25) = -25 on each axis.
#[wasm_bindgen_test]
async fn ctrl_or_cmd_plus_wheel_zooms_about_the_pointer() -> Result<(), String> {
    let scene = new_scene("tb-wheel")?;
    scene.show_toolbar(ToolbarOptions::default()).map_err(|e| e.to_string())?;
    let surface = pan_surface("tb-wheel")?;

    // Every test's `<svg>` shares one page, so client coordinates must be built from this one's own position.
    // The client position is a whole number of pixels, but the `<svg>` itself can sit at a fractional offset. So the
    // pivot the scene sees is the difference, not exactly (100, 100).
    let bounds = surface.get_bounding_client_rect();
    let (x, y) = (bounds.left().round() as i32 + 100, bounds.top().round() as i32 + 100);
    let (pivot_x, pivot_y) = (x as f64 - bounds.left(), y as f64 - bounds.top());

    check(wheel(&surface, x, y, -100.0, true, false)?, "ctrl+wheel was not cancelled")?;
    next_frame().await?;
    let (tx, ty, scale) = parse_view(&attr(&content("tb-wheel")?, "transform")?)?;
    // One notch is one 1.25 step, and the pivot is fixed: t' = pivot - 1.25 * pivot.
    check_close(scale, 1.25)?;
    check_close(tx, pivot_x - 1.25 * pivot_x)?;
    check_close(ty, pivot_y - 1.25 * pivot_y)?;

    scene.reset_view().map_err(|e| e.to_string())?;
    check(wheel(&surface, x, y, -100.0, false, true)?, "cmd+wheel was not cancelled")?;
    check_close(scene.zoom_scale(), 1.25)
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Scrolling down zooms out, and a smaller delta zooms proportionally less.
#[wasm_bindgen_test]
fn the_wheel_direction_and_size_set_the_zoom_direction_and_amount() -> Result<(), String> {
    let scene = new_scene("tb-wheel-dir")?;
    scene.show_toolbar(ToolbarOptions::default()).map_err(|e| e.to_string())?;
    let surface = pan_surface("tb-wheel-dir")?;

    wheel(&surface, 200, 150, 100.0, true, false)?;
    check_close(scene.zoom_scale(), 0.8)?;

    scene.reset_view().map_err(|e| e.to_string())?;
    wheel(&surface, 200, 150, -10.0, true, false)?;
    check_close(scene.zoom_scale(), 1.25_f64.powf(0.1))
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Without a modifier the wheel is neither handled nor cancelled, so the page still scrolls.
#[wasm_bindgen_test]
fn the_wheel_alone_does_not_zoom_and_is_not_cancelled() -> Result<(), String> {
    let scene = new_scene("tb-wheel-plain")?;
    scene.show_toolbar(ToolbarOptions::default()).map_err(|e| e.to_string())?;
    let surface = pan_surface("tb-wheel-plain")?;

    check(!wheel(&surface, 100, 100, -100.0, false, false)?, "a plain wheel was cancelled")?;
    check_close(scene.zoom_scale(), 1.0)
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// A wheel over a node bubbles to the content layer, so zooming works over nodes as well as over empty background.
#[wasm_bindgen_test]
fn wheel_zoom_works_over_a_node() -> Result<(), String> {
    let scene = new_scene("tb-wheel-node")?;
    scene
        .add_node(Point::new(100.0, 100.0), Size::new(60.0, 30.0), "A")
        .map_err(|e| e.to_string())?;
    scene.show_toolbar(ToolbarOptions::default()).map_err(|e| e.to_string())?;

    let group = nth_group("tb-wheel-node", 0)?;
    check(
        wheel(&group, 110, 110, -100.0, true, false)?,
        "a wheel over a node was not cancelled",
    )?;
    check_close(scene.zoom_scale(), 1.25)
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Wheel zoom belongs to the toolbar, like panning. Without one, a scene leaves the wheel alone entirely, and hiding the
/// toolbar takes the wheel handling away with it.
#[wasm_bindgen_test]
fn wheel_zoom_needs_a_shown_toolbar() -> Result<(), String> {
    let scene = new_scene("tb-wheel-off")?;
    let node = scene
        .add_node(Point::new(100.0, 100.0), Size::new(60.0, 30.0), "A")
        .map_err(|e| e.to_string())?;
    let _ = node;
    let group = nth_group("tb-wheel-off", 0)?;

    check(
        !wheel(&group, 110, 110, -100.0, true, false)?,
        "a scene with no toolbar cancelled a wheel",
    )?;
    check_close(scene.zoom_scale(), 1.0)?;

    scene.show_toolbar(ToolbarOptions::default()).map_err(|e| e.to_string())?;
    scene.hide_toolbar();
    check(
        !wheel(&group, 110, 110, -100.0, true, false)?,
        "a hidden toolbar left the wheel handler behind",
    )?;
    check_close(scene.zoom_scale(), 1.0)?;

    // ...and showing it again works, without stacking a second handler that would zoom twice.
    scene.show_toolbar(ToolbarOptions::default()).map_err(|e| e.to_string())?;
    wheel(&group, 110, 110, -100.0, true, false)?;
    check_close(scene.zoom_scale(), 1.25)
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Wheel zoom stops at the same limits as the buttons, and enables Reset.
#[wasm_bindgen_test]
async fn wheel_zoom_is_limited_and_enables_reset() -> Result<(), String> {
    let scene = new_scene("tb-wheel-limit")?;
    scene.show_toolbar(ToolbarOptions::default()).map_err(|e| e.to_string())?;
    let surface = pan_surface("tb-wheel-limit")?;

    wheel(&surface, 100, 100, -100.0, true, false)?;
    next_frame().await?;
    check(
        attr(&button("tb-wheel-limit", 2)?, "aria-disabled")? == "false",
        "wheel zoom did not enable Reset",
    )?;
    for _ in 0..30 {
        wheel(&surface, 100, 100, -100.0, true, false)?;
    }
    check_close(scene.zoom_scale(), 4.0)?;
    next_frame().await?;
    check(
        attr(&button("tb-wheel-limit", 0)?, "aria-disabled")? == "true",
        "zoom-in stayed enabled at the limit",
    )
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// A trackpad pinch is many small ctrl+wheel events, not one notch. Twenty events of -10 pixels each must compose to
/// exactly two notches (1.25 squared), not twenty full steps that slam into the maximum. The DOM is not written for
/// each one: nothing is rendered until the frame, and then the final view is written once.
#[wasm_bindgen_test]
async fn a_burst_of_small_wheel_events_composes_proportionally_and_writes_the_dom_once_per_frame() -> Result<(), String>
{
    let scene = new_scene("tb-wheel-burst")?;
    scene.show_toolbar(ToolbarOptions::default()).map_err(|e| e.to_string())?;
    let surface = pan_surface("tb-wheel-burst")?;

    for _ in 0..20 {
        wheel(&surface, 100, 100, -10.0, true, false)?;
    }

    // The model already holds the composed result...
    check_close(scene.zoom_scale(), 1.5625)?;
    // ...but no event has written the DOM yet.
    check(
        content("tb-wheel-burst")?.get_attribute("transform").is_none(),
        "a wheel event wrote the DOM immediately instead of waiting for the frame",
    )?;

    next_frame().await?;
    let (_, _, scale) = parse_view(&attr(&content("tb-wheel-burst")?, "transform")?)?;
    check_close(scale, 1.5625)
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Small events and one big event zoom identically: the factors multiply.
#[wasm_bindgen_test]
fn ten_wheel_events_of_ten_pixels_zoom_exactly_as_one_of_a_hundred() -> Result<(), String> {
    let split = new_scene("tb-wheel-split")?;
    split.show_toolbar(ToolbarOptions::default()).map_err(|e| e.to_string())?;
    let surface = pan_surface("tb-wheel-split")?;
    for _ in 0..10 {
        wheel(&surface, 100, 100, -10.0, true, false)?;
    }

    let whole = new_scene("tb-wheel-whole")?;
    whole.show_toolbar(ToolbarOptions::default()).map_err(|e| e.to_string())?;
    wheel(&pan_surface("tb-wheel-whole")?, 100, 100, -100.0, true, false)?;

    check_close(split.zoom_scale(), whole.zoom_scale())?;
    check_close(split.zoom_scale(), 1.25)
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// A button pressed while a wheel burst is still waiting for its frame acts on the latest view, and the frame that
/// follows cannot resurrect the stale wheel view over it.
#[wasm_bindgen_test]
async fn a_button_pressed_mid_burst_wins_over_the_pending_frame() -> Result<(), String> {
    let scene = new_scene("tb-wheel-then-reset")?;
    scene.show_toolbar(ToolbarOptions::default()).map_err(|e| e.to_string())?;
    let surface = pan_surface("tb-wheel-then-reset")?;

    wheel(&surface, 100, 100, -100.0, true, false)?;
    click(&button("tb-wheel-then-reset", 2)?)?; // 100%
    next_frame().await?;

    check_close(scene.zoom_scale(), 1.0)?;
    let transform = attr(&content("tb-wheel-then-reset")?, "transform")?;
    check(
        transform == "translate(0, 0) scale(1)",
        &format!("a pending frame overwrote the reset: {transform}"),
    )
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// A pan is written to the DOM one frame late while it runs, but the moment the pointer is released the DOM is exact
/// — nothing is left for a later frame to catch up on.
#[wasm_bindgen_test]
fn releasing_a_pan_writes_its_final_position_immediately() -> Result<(), String> {
    let scene = new_scene("tb-pan-release")?;
    scene.show_toolbar(ToolbarOptions::default()).map_err(|e| e.to_string())?;
    let surface = pan_surface("tb-pan-release")?;

    dispatch_pointer_event(&surface, "pointerdown", 100, 100, 1)?;
    dispatch_pointer_event(&surface, "pointermove", 160, 130, 1)?;
    check(
        content("tb-pan-release")?.get_attribute("transform").is_none(),
        "a pan move wrote the DOM immediately instead of waiting for the frame",
    )?;
    dispatch_pointer_event(&surface, "pointerup", 160, 130, 1)?;
    let transform = attr(&content("tb-pan-release")?, "transform")?;
    check(
        transform == "translate(60, 30) scale(1)",
        &format!("release did not flush: {transform}"),
    )
}
