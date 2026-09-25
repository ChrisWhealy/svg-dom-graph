//! Browser tests for `Scene`'s toolbar and zoom: showing, hiding, and moving the button bar, keeping it a fixed size
//! while the content zooms, the zoom buttons themselves (by click and by keyboard), and dragging under zoom.
//!
//! Every test uses a viewport and `viewBox` of `400 x 300`, 1:1, so client pixels and user-space units coincide.

use super::common::*;
use svg_dom::root::utils::{Point, Size};
use svg_dom_graph::{
    Error,
    scene::{DataFormat, DataNodeContent, GridLayout, InputMode, NodeValues, Scene, Selection, Side, ToolbarOptions},
};
use wasm_bindgen::JsCast;
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

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
// Pan and wheel zoom are independent of the toolbar. Each has its own `InputMode`: `WithToolbar` (the default, which
// follows the toolbar), `On`, or `Off`.

/// Ctrl+wheel one notch up, then reports whether the scene zoomed. Uses a bounding-box-relative pointer position.
fn ctrl_wheel_zooms(scene: &Scene, id: &str) -> Result<bool, String> {
    let before = scene.zoom_scale();
    let target = query(&format!("#{id} > rect"))?.unwrap_or(required(&format!("#{id}"))?);
    wheel(&target, 100, 100, -100.0, true, false)?;
    Ok((scene.zoom_scale() - before).abs() > 1e-9)
}

/// Drags the background 40 pixels right, then reports whether the content moved. Needs a pan surface to exist.
fn background_drag_pans(id: &str) -> Result<bool, String> {
    let surface = match query(&format!("#{id} > rect"))? {
        Some(surface) => surface,
        None => return Ok(false),
    };
    let content = content(id)?;
    let before = content.get_attribute("transform");
    dispatch_pointer_event(&surface, "pointerdown", 100, 100, 1)?;
    dispatch_pointer_event(&surface, "pointermove", 140, 100, 1)?;
    dispatch_pointer_event(&surface, "pointerup", 140, 100, 1)?;
    Ok(content.get_attribute("transform") != before)
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The default follows the toolbar exactly as before: off with no toolbar, on while one is shown, off again once hidden.
#[wasm_bindgen_test]
fn by_default_both_gestures_follow_the_toolbar() -> Result<(), String> {
    let scene = new_scene("im-default")?;
    check(
        scene.pan_mode() == InputMode::WithToolbar,
        "pan does not default to WithToolbar",
    )?;
    check(
        scene.wheel_zoom_mode() == InputMode::WithToolbar,
        "wheel zoom does not default to WithToolbar",
    )?;
    check(
        !scene.pan_enabled() && !scene.wheel_zoom_enabled(),
        "a gesture is active with no toolbar",
    )?;

    scene.show_toolbar(ToolbarOptions::default()).map_err(|e| e.to_string())?;
    check(
        scene.pan_enabled() && scene.wheel_zoom_enabled(),
        "showing the toolbar did not activate the gestures",
    )?;

    scene.hide_toolbar();
    check(
        !scene.pan_enabled() && !scene.wheel_zoom_enabled(),
        "hiding the toolbar left a gesture active",
    )?;
    check(query("#im-default > rect")?.is_none(), "the surface outlived the toolbar")
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The motivating case: an application supplies its own controls, hides the stock toolbar, and still wants both
/// gestures.
#[wasm_bindgen_test]
fn both_gestures_can_be_forced_on_with_no_toolbar_at_all() -> Result<(), String> {
    let scene = new_scene("im-own-controls")?;
    scene.set_pan_mode(InputMode::On).map_err(|e| e.to_string())?;
    scene.set_wheel_zoom_mode(InputMode::On).map_err(|e| e.to_string())?;

    check(!scene.has_toolbar(), "test setup: a toolbar is shown")?;
    check(
        scene.pan_enabled() && scene.wheel_zoom_enabled(),
        "a forced gesture is not enabled",
    )?;
    check(background_drag_pans("im-own-controls")?, "dragging the background did not pan")?;
    check(ctrl_wheel_zooms(&scene, "im-own-controls")?, "ctrl+wheel did not zoom")
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// A forced-on gesture survives the toolbar being shown and hidden, while one following the toolbar does not.
#[wasm_bindgen_test]
fn a_forced_gesture_survives_hiding_the_toolbar_and_a_following_one_does_not() -> Result<(), String> {
    let scene = new_scene("im-mixed")?;
    scene.set_wheel_zoom_mode(InputMode::On).map_err(|e| e.to_string())?;
    scene.show_toolbar(ToolbarOptions::default()).map_err(|e| e.to_string())?;
    check(
        scene.pan_enabled() && scene.wheel_zoom_enabled(),
        "both should be active with the toolbar",
    )?;

    scene.hide_toolbar();
    check(!scene.pan_enabled(), "pan followed the toolbar away but is still active")?;
    check(
        scene.wheel_zoom_enabled(),
        "a forced wheel zoom was switched off with the toolbar",
    )?;
    check(
        ctrl_wheel_zooms(&scene, "im-mixed")?,
        "wheel zoom stopped working after the toolbar was hidden",
    )?;
    check(
        !background_drag_pans("im-mixed")?,
        "the background still pans with no pan mode active",
    )
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// `Off` beats a shown toolbar, and the two gestures are independent of each other.
#[wasm_bindgen_test]
fn a_gesture_can_be_forced_off_even_while_the_toolbar_is_shown() -> Result<(), String> {
    let scene = new_scene("im-off")?;
    scene.set_pan_mode(InputMode::Off).map_err(|e| e.to_string())?;
    scene.show_toolbar(ToolbarOptions::default()).map_err(|e| e.to_string())?;

    check(!scene.pan_enabled(), "pan is active despite being forced off")?;
    check(scene.wheel_zoom_enabled(), "wheel zoom was affected by the pan mode")?;
    check(
        !background_drag_pans("im-off")?,
        "dragging the background panned despite pan being off",
    )?;
    check(ctrl_wheel_zooms(&scene, "im-off")?, "ctrl+wheel stopped zooming")?;
    check(
        query("#im-off > rect")?.and_then(|r| r.get_attribute("style")).is_none(),
        "a surface with panning off still shows a grab cursor",
    )
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Both `Off` with a toolbar shown leaves no surface at all, so nothing sits behind the content.
#[wasm_bindgen_test]
fn with_both_gestures_off_there_is_no_surface_even_with_a_toolbar() -> Result<(), String> {
    let scene = new_scene("im-both-off")?;
    scene.set_pan_mode(InputMode::Off).map_err(|e| e.to_string())?;
    scene.set_wheel_zoom_mode(InputMode::Off).map_err(|e| e.to_string())?;
    scene.show_toolbar(ToolbarOptions::default()).map_err(|e| e.to_string())?;

    check(scene.has_toolbar(), "the toolbar is not shown")?;
    check(
        query("#im-both-off > rect")?.is_none(),
        "a surface exists with both gestures off",
    )?;
    check(!ctrl_wheel_zooms(&scene, "im-both-off")?, "wheel zoom worked with the mode off")
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Pan on, wheel zoom untouched: only the requested gesture is wired.
#[wasm_bindgen_test]
fn forcing_only_pan_on_leaves_wheel_zoom_off() -> Result<(), String> {
    let scene = new_scene("im-pan-only")?;
    scene.set_pan_mode(InputMode::On).map_err(|e| e.to_string())?;

    check(background_drag_pans("im-pan-only")?, "dragging the background did not pan")?;
    check(
        !ctrl_wheel_zooms(&scene, "im-pan-only")?,
        "wheel zoom worked though it was never enabled",
    )?;
    check(scene.zoom_scale() == 1.0, "the scale changed")
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Going back to `WithToolbar`, or to `Off`, tears the gesture down completely: no surface, no leftover wheel listener
/// on the content layer, so a wheel over a node is no longer cancelled.
#[wasm_bindgen_test]
fn switching_a_gesture_back_off_removes_its_surface_and_listener() -> Result<(), String> {
    let scene = new_scene("im-teardown")?;
    scene
        .add_node(Point::new(100.0, 100.0), Size::new(60.0, 30.0), "A")
        .map_err(|e| e.to_string())?;
    let node = nth_group("im-teardown", 0)?;

    scene.set_wheel_zoom_mode(InputMode::On).map_err(|e| e.to_string())?;
    check(
        wheel(&node, 110, 110, -100.0, true, false)?,
        "a wheel over a node was not cancelled while on",
    )?;

    scene.set_wheel_zoom_mode(InputMode::WithToolbar).map_err(|e| e.to_string())?;
    check(
        query("#im-teardown > rect")?.is_none(),
        "the surface remained after switching back",
    )?;
    let before = scene.zoom_scale();
    check(
        !wheel(&node, 110, 110, -100.0, true, false)?,
        "a leftover wheel listener still cancels",
    )?;
    check_close(scene.zoom_scale(), before)
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Setting the same mode again changes nothing — in particular it does not rebuild the surface.
#[wasm_bindgen_test]
fn setting_the_same_mode_again_keeps_the_same_surface() -> Result<(), String> {
    let scene = new_scene("im-idempotent")?;
    scene.set_pan_mode(InputMode::On).map_err(|e| e.to_string())?;
    let first = required("#im-idempotent > rect")?;
    scene.set_pan_mode(InputMode::On).map_err(|e| e.to_string())?;
    let second = required("#im-idempotent > rect")?;
    check(first.is_same_node(Some(&second)), "an unchanged mode rebuilt the surface")
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// `refresh_layout` resizes the surface with no toolbar shown, and `refresh_toolbar_layout` still works as before.
#[wasm_bindgen_test]
fn refresh_layout_resizes_the_surface_with_no_toolbar() -> Result<(), String> {
    let scene = new_scene("im-refresh")?;
    let svg_element = required("#im-refresh")?;
    scene.set_pan_mode(InputMode::On).map_err(|e| e.to_string())?;
    let surface = required("#im-refresh > rect")?;
    check_close(attr_f64(&surface, "width")?, 400.0)?;

    // The scene cannot observe a `viewBox` change, which is why `refresh_layout` exists.
    svg_element
        .set_attribute("viewBox", "0 0 800 600")
        .map_err(|e| format!("{e:?}"))?;
    scene.refresh_layout().map_err(|e| e.to_string())?;
    check_close(attr_f64(&surface, "width")?, 800.0)?;
    check_close(attr_f64(&surface, "height")?, 600.0)?;

    svg_element
        .set_attribute("viewBox", "0 0 500 350")
        .map_err(|e| format!("{e:?}"))?;
    scene.refresh_toolbar_layout().map_err(|e| e.to_string())?;
    check_close(attr_f64(&surface, "width")?, 500.0)
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Dragging a node that is currently selected drags only that node. Its selection — the text it exposes to assistive
/// technology, and every cell's colour — is untouched by the drag, and the scene does not pan.
#[wasm_bindgen_test]
fn dragging_a_selected_node_moves_it_keeps_its_selection_and_does_not_pan() -> Result<(), String> {
    let scene = new_scene("tb-selected-drag")?;
    let node = scene
        .add_data_node(
            Point::new(100.0, 100.0),
            DataNodeContent::new(NodeValues::U8(vec![1, 2, 3, 4]), DataFormat::Decimal)
                .with_layout(GridLayout::Rows(2)),
        )
        .map_err(|e| e.to_string())?;
    scene.make_draggable(node).map_err(|e| e.to_string())?;
    scene
        .set_selection(node, Selection::Row { row: 1, col: Some(0) })
        .map_err(|e| e.to_string())?;
    scene.show_toolbar(ToolbarOptions::default()).map_err(|e| e.to_string())?;

    let group = nth_group("tb-selected-drag", 0)?;
    let fills = |group: &web_sys::Element| -> Result<Vec<Option<String>>, String> {
        let rects = group.query_selector_all("rect").map_err(|e| format!("{e:?}"))?;
        Ok((0..rects.length())
            .filter_map(|i| rects.get(i))
            .filter_map(|n| n.dyn_into::<web_sys::Element>().ok())
            .map(|r| r.get_attribute("fill"))
            .collect())
    };
    let title = |group: &web_sys::Element| -> Result<Option<String>, String> {
        Ok(group
            .query_selector(":scope > title")
            .map_err(|e| format!("{e:?}"))?
            .and_then(|t| t.text_content()))
    };
    let (label_before, fills_before) = (title(&group)?, fills(&group)?);
    check(
        fills_before.iter().any(|f| f.as_deref() == Some("#ff6b4a")),
        "test setup: no cell shows the focused colour",
    )?;

    dispatch_pointer_event(&group, "pointerdown", 110, 110, 1)?;
    dispatch_pointer_event(&group, "pointermove", 140, 130, 1)?;
    dispatch_pointer_event(&group, "pointerup", 140, 130, 1)?;

    let (x, y) = group_translate(&group)?;
    check_close(x, 130.0)?;
    check_close(y, 120.0)?;
    check(
        title(&group)? == label_before,
        "the drag changed the node's accessible description",
    )?;
    check(fills(&group)? == fills_before, "the drag changed a cell's colour")?;
    check(
        content("tb-selected-drag")?.get_attribute("transform").is_none(),
        "dragging a selected node panned the scene",
    )
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The surface's cursor tracks the pan: grabbing while the button is down, back to grab after a release or a cancel.
/// Real pointer capture is checked separately, against real input, in the CDP suite — a synthetic pointer id is not one
/// the browser will let a test capture.
#[wasm_bindgen_test]
fn the_pan_cursor_returns_to_idle_after_release_and_after_cancel() -> Result<(), String> {
    let scene = new_scene("tb-pan-cursor")?;
    scene.show_toolbar(ToolbarOptions::default()).map_err(|e| e.to_string())?;
    let surface = pan_surface("tb-pan-cursor")?;
    let style = || attr(&surface, "style");

    check(
        style()?.contains("cursor: grab;"),
        "the idle surface does not show a grab cursor",
    )?;

    for end in ["pointerup", "pointercancel"] {
        dispatch_pointer_event(&surface, "pointerdown", 100, 100, 1)?;
        check(
            style()?.contains("cursor: grabbing;"),
            "a pan in progress does not show a grabbing cursor",
        )?;
        dispatch_pointer_event(&surface, end, 120, 100, 1)?;
        check(
            style()?.contains("cursor: grab;"),
            &format!("the cursor did not return to grab after {end}"),
        )?;
    }
    Ok(())
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// However a pan ends — released, or cancelled — the next one starts cleanly and adds to the last, so no stale gesture
/// state is left behind.
#[wasm_bindgen_test]
fn a_pan_can_be_started_again_after_any_way_of_ending_one() -> Result<(), String> {
    let scene = new_scene("tb-pan-reuse")?;
    scene.show_toolbar(ToolbarOptions::default()).map_err(|e| e.to_string())?;
    let surface = pan_surface("tb-pan-reuse")?;

    let mut expected = 0.0;
    for end in ["pointerup", "pointercancel", "pointerup"] {
        dispatch_pointer_event(&surface, "pointerdown", 100, 100, 1)?;
        dispatch_pointer_event(&surface, "pointermove", 110, 100, 1)?;
        dispatch_pointer_event(&surface, end, 110, 100, 1)?;
        // `pointerup` flushes at once. A cancel does too, since it also ends the gesture.
        expected += 10.0;
        let (tx, _, _) = parse_view(&attr(&content("tb-pan-reuse")?, "transform")?)?;
        check_close(tx, expected)?;
    }
    Ok(())
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The toolbar's own buttons sit above the pan surface, so pressing one never starts a pan — but does still click.
#[wasm_bindgen_test]
fn pressing_a_toolbar_button_does_not_start_a_pan() -> Result<(), String> {
    let scene = new_scene("tb-button-pan")?;
    scene.show_toolbar(ToolbarOptions::default()).map_err(|e| e.to_string())?;
    let plus = button("tb-button-pan", 0)?;

    dispatch_pointer_event(&plus, "pointerdown", 100, 100, 1)?;
    dispatch_pointer_event(&plus, "pointermove", 160, 100, 1)?;
    dispatch_pointer_event(&plus, "pointerup", 160, 100, 1)?;
    check(
        content("tb-button-pan")?.get_attribute("transform").is_none(),
        "pressing a toolbar button panned the scene",
    )?;

    click(&plus)?;
    check_close(scene.zoom_scale(), 1.25)
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Showing and hiding the toolbar repeatedly must never leave a handler behind. Every listener lives on the surface,
/// which is removed with the gestures, except the wheel listener on the content layer.
///
/// A leaked wheel listener cannot zoom, since it only holds a weak reference to a surface that no longer exists. What it
/// can still do is cancel the event, so the test that catches a leak is the last one: with everything torn down, a
/// modified wheel over a node must no longer be cancelled by anything.
#[wasm_bindgen_test]
fn showing_and_hiding_the_toolbar_repeatedly_installs_no_duplicate_handlers() -> Result<(), String> {
    let scene = new_scene("tb-cycles")?;
    scene
        .add_node(Point::new(100.0, 100.0), Size::new(60.0, 30.0), "A")
        .map_err(|e| e.to_string())?;
    let node = nth_group("tb-cycles", 0)?;
    let count = |selector: &str| -> Result<u32, String> {
        Ok(web_sys::window()
            .and_then(|w| w.document())
            .ok_or("no document")?
            .query_selector_all(selector)
            .map_err(|e| format!("{e:?}"))?
            .length())
    };

    for _ in 0..5 {
        scene.show_toolbar(ToolbarOptions::default()).map_err(|e| e.to_string())?;
        scene.hide_toolbar();
    }
    scene.show_toolbar(ToolbarOptions::default()).map_err(|e| e.to_string())?;
    // Showing again with a toolbar already up must replace it, not add a second.
    scene
        .show_toolbar(ToolbarOptions::new(Side::South))
        .map_err(|e| e.to_string())?;

    check(count("#tb-cycles > rect")? == 1, "there is not exactly one pan surface")?;
    check(
        count("#tb-cycles > [role=\"toolbar\"]")? == 1,
        "there is not exactly one toolbar",
    )?;

    // One notch over a node reaches the content layer's own listener; over the background, the surface's.
    wheel(&node, 110, 110, -100.0, true, false)?;
    check_close(scene.zoom_scale(), 1.25)?;
    scene.reset_view().map_err(|e| e.to_string())?;
    wheel(&pan_surface("tb-cycles")?, 100, 100, -100.0, true, false)?;
    check_close(scene.zoom_scale(), 1.25)?;

    // Tear it all down: nothing left from any earlier cycle may still react to the wheel.
    scene.hide_toolbar();
    check(count("#tb-cycles > rect")? == 0, "a surface survived hiding the toolbar")?;
    check(
        !wheel(&node, 110, 110, -100.0, true, false)?,
        "a listener left by an earlier cycle still cancels the wheel",
    )
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The same, for a mode toggled back and forth: no accumulated surfaces or listeners.
#[wasm_bindgen_test]
fn toggling_input_modes_repeatedly_installs_no_duplicate_handlers() -> Result<(), String> {
    let scene = new_scene("im-cycles")?;
    scene
        .add_node(Point::new(100.0, 100.0), Size::new(60.0, 30.0), "A")
        .map_err(|e| e.to_string())?;
    let node = nth_group("im-cycles", 0)?;

    for _ in 0..8 {
        for mode in [InputMode::On, InputMode::Off, InputMode::WithToolbar] {
            scene.set_wheel_zoom_mode(mode).map_err(|e| e.to_string())?;
            scene.set_pan_mode(mode).map_err(|e| e.to_string())?;
        }
    }
    scene.set_wheel_zoom_mode(InputMode::On).map_err(|e| e.to_string())?;
    scene.set_pan_mode(InputMode::On).map_err(|e| e.to_string())?;

    let surfaces = web_sys::window()
        .and_then(|w| w.document())
        .ok_or("no document")?
        .query_selector_all("#im-cycles > rect")
        .map_err(|e| format!("{e:?}"))?
        .length();
    check(surfaces == 1, &format!("expected one surface, found {surfaces}"))?;
    wheel(&node, 110, 110, -100.0, true, false)?;
    check_close(scene.zoom_scale(), 1.25)?;

    // Tear it all down: nothing left from any earlier toggle may still react to the wheel.
    scene.set_wheel_zoom_mode(InputMode::WithToolbar).map_err(|e| e.to_string())?;
    scene.set_pan_mode(InputMode::WithToolbar).map_err(|e| e.to_string())?;
    check(
        !wheel(&node, 110, 110, -100.0, true, false)?,
        "a listener left by an earlier toggle still cancels the wheel",
    )
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
// Responsive layouts. The toolbar and the pan surface are laid out in the `<svg>`'s own user space, and the scene
// cannot observe that changing.

/// Builds an `<svg>` sized purely by CSS, with no `width`, `height`, or `viewBox` attribute, and returns its id.
///
/// `svg-dom` caches an attribute-less `<svg>`'s size as `0 x 0`, since it never measures the rendered size. The layout
/// must not be fooled by that.
fn css_sized_svg(id: &str, width: u32, height: u32) -> Result<svg_dom::SvgRoot, String> {
    let document = web_sys::window().and_then(|w| w.document()).ok_or("no document")?;
    let svg = document
        .create_element_ns(Some("http://www.w3.org/2000/svg"), "svg")
        .map_err(|e| format!("{e:?}"))?;
    svg.set_id(id);
    svg.set_attribute("style", &format!("display: block; width: {width}px; height: {height}px;"))
        .map_err(|e| format!("{e:?}"))?;
    let body = document
        .query_selector("body")
        .map_err(|e| format!("{e:?}"))?
        .ok_or("no body")?;
    body.append_child(&svg).map_err(|e| format!("{e:?}"))?;
    svg_dom::SvgRoot::attach(id).map_err(|e| e.to_string())
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// With no `viewBox` and no size attributes, the layout uses the rendered size rather than the cached `0 x 0`. A
/// 300 x 200 north bar of default width 134 sits at x = (300 - 134) / 2 = 83, and the pan surface covers the whole box.
#[wasm_bindgen_test]
fn a_css_sized_svg_with_no_view_box_is_laid_out_against_its_rendered_size() -> Result<(), String> {
    let svg = css_sized_svg("rl-css-sized", 300, 200)?;
    check(svg.width() == 0.0, "test setup: the cached width is not zero")?;
    let scene = Scene::new(svg).map_err(|e| e.to_string())?;
    scene.show_toolbar(ToolbarOptions::default()).map_err(|e| e.to_string())?;

    let (x, y) = group_translate(&bar("rl-css-sized")?)?;
    check_close(x, 83.0)?;
    check_close(y, 8.0)?;

    let surface = pan_surface("rl-css-sized")?;
    check_close(attr_f64(&surface, "width")?, 300.0)?;
    check_close(attr_f64(&surface, "height")?, 200.0)
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The layout does not follow a resize by itself: it stays where it was until `refresh_layout` is called. That is the
/// documented requirement, and this test pins it down so it is not mistaken for a bug.
#[wasm_bindgen_test]
fn a_resize_is_only_picked_up_by_refresh_layout() -> Result<(), String> {
    let scene = Scene::new(css_sized_svg("rl-resize", 300, 200)?).map_err(|e| e.to_string())?;
    scene.show_toolbar(ToolbarOptions::default()).map_err(|e| e.to_string())?;
    let svg_element = required("#rl-resize")?;

    svg_element
        .set_attribute("style", "display: block; width: 500px; height: 200px;")
        .map_err(|e| format!("{e:?}"))?;
    let (x, _) = group_translate(&bar("rl-resize")?)?;
    check_close(x, 83.0)?; // Still laid out for 300 wide.

    scene.refresh_layout().map_err(|e| e.to_string())?;
    let (x, _) = group_translate(&bar("rl-resize")?)?;
    check_close(x, 183.0)?; // (500 - 134) / 2
    check_close(attr_f64(&pan_surface("rl-resize")?, "width")?, 500.0)
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// With a `viewBox`, the layout is in the `viewBox`'s own units, and the browser scales the whole `<svg>` — toolbar
/// included — when only its CSS size changes. So a responsive page needs no refresh at all. Nothing about the toolbar's
/// own position changes, and it is still where it should be relative to the content.
#[wasm_bindgen_test]
fn with_a_view_box_a_css_resize_needs_no_refresh() -> Result<(), String> {
    let scene = new_scene("rl-view-box")?;
    scene.show_toolbar(ToolbarOptions::default()).map_err(|e| e.to_string())?;
    let before = attr(&bar("rl-view-box")?, "transform")?;

    required("#rl-view-box")?
        .set_attribute("style", "display: block; width: 800px; height: 600px;")
        .map_err(|e| format!("{e:?}"))?;

    check(
        attr(&bar("rl-view-box")?, "transform")? == before,
        "a CSS resize moved the toolbar in user space",
    )?;
    // It is still centred in the same `400 x 300` user space, which the browser has simply scaled up.
    let (x, y) = group_translate(&bar("rl-view-box")?)?;
    check_close(x, 133.0)?;
    check_close(y, 8.0)
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
// Non-zero `viewBox` origins, mismatched aspect ratios, and CSS scaling. The scene must work in the `<svg>`'s own user
// space — the coordinates its content is drawn in — wherever the browser places that space on screen.

/// Builds an `<svg>` with the given CSS size, `viewBox`, and optional `preserveAspectRatio`, and returns its scene.
fn scene_in_svg(
    id: &str,
    css_width: u32,
    css_height: u32,
    view_box: &str,
    preserve_aspect_ratio: Option<&str>,
) -> Result<Scene, String> {
    let document = web_sys::window().and_then(|w| w.document()).ok_or("no document")?;
    let svg = document
        .create_element_ns(Some("http://www.w3.org/2000/svg"), "svg")
        .map_err(|e| format!("{e:?}"))?;
    svg.set_id(id);
    svg.set_attribute("viewBox", view_box).map_err(|e| format!("{e:?}"))?;
    if let Some(value) = preserve_aspect_ratio {
        svg.set_attribute("preserveAspectRatio", value).map_err(|e| format!("{e:?}"))?;
    }
    svg.set_attribute(
        "style",
        &format!("display: block; width: {css_width}px; height: {css_height}px;"),
    )
    .map_err(|e| format!("{e:?}"))?;
    let body = document
        .query_selector("body")
        .map_err(|e| format!("{e:?}"))?
        .ok_or("no body")?;
    body.append_child(&svg).map_err(|e| format!("{e:?}"))?;
    Scene::new(svg_dom::SvgRoot::attach(id).map_err(|e| e.to_string())?).map_err(|e| e.to_string())
}

/// The surface's `(x, y, width, height)`.
fn surface_rect(id: &str) -> Result<(f64, f64, f64, f64), String> {
    let surface = pan_surface(id)?;
    Ok((
        attr_f64(&surface, "x")?,
        attr_f64(&surface, "y")?,
        attr_f64(&surface, "width")?,
        attr_f64(&surface, "height")?,
    ))
}

fn check_rect(got: (f64, f64, f64, f64), expected: (f64, f64, f64, f64)) -> Result<(), String> {
    check_close(got.0, expected.0)?;
    check_close(got.1, expected.1)?;
    check_close(got.2, expected.2)?;
    check_close(got.3, expected.3)
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// A `viewBox` centred on the origin: `-500 -300 1000 600`. The visible area's centre is (0, 0), not (500, 300), so a
/// zoom about the centre leaves the translation at zero. An implementation that assumed a `(0, 0)` origin would zoom
/// about (500, 300) instead, and translate by (-125, -75).
#[wasm_bindgen_test]
fn a_view_box_centred_on_the_origin_zooms_about_its_own_centre() -> Result<(), String> {
    let scene = scene_in_svg("vb-centred", 400, 240, "-500 -300 1000 600", None)?;
    scene.show_toolbar(ToolbarOptions::default()).map_err(|e| e.to_string())?;

    // The surface covers the visible area, origin included.
    check_rect(surface_rect("vb-centred")?, (-500.0, -300.0, 1000.0, 600.0))?;
    // A north bar: centred along the top edge, `margin` below it. (1000 - 134) / 2 - 500 = -67.
    let (x, y) = group_translate(&bar("vb-centred")?)?;
    check_close(x, -67.0)?;
    check_close(y, -292.0)?;

    scene.zoom_in().map_err(|e| e.to_string())?;
    let (tx, ty, scale) = parse_view(&attr(&content("vb-centred")?, "transform")?)?;
    check_close(scale, 1.25)?;
    check_close(tx, 0.0)?;
    check_close(ty, 0.0)?;

    scene.reset_view().map_err(|e| e.to_string())?;
    check(
        attr(&content("vb-centred")?, "transform")? == "translate(0, 0) scale(1)",
        "reset did not restore the identity",
    )
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// A `viewBox` whose origin is positive on both axes: `100 50 400 300`, centre (300, 200). Zooming in by 1.25 about
/// it gives a translation of (300 - 1.25 * 300, 200 - 1.25 * 200) = (-75, -50).
#[wasm_bindgen_test]
fn a_view_box_with_a_positive_origin_zooms_about_its_own_centre() -> Result<(), String> {
    let scene = scene_in_svg("vb-offset", 400, 300, "100 50 400 300", None)?;
    scene
        .show_toolbar(ToolbarOptions::new(Side::South))
        .map_err(|e| e.to_string())?;

    check_rect(surface_rect("vb-offset")?, (100.0, 50.0, 400.0, 300.0))?;
    // South: (400 - 134) / 2 + 100 = 233, and 50 + 300 - 8 - 28 = 314.
    let (x, y) = group_translate(&bar("vb-offset")?)?;
    check_close(x, 233.0)?;
    check_close(y, 314.0)?;

    scene.zoom_in().map_err(|e| e.to_string())?;
    let (tx, ty, _) = parse_view(&attr(&content("vb-offset")?, "transform")?)?;
    check_close(tx, -75.0)?;
    check_close(ty, -50.0)
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Pointer-centred wheel zoom under a non-zero origin *and* a CSS scale (here 0.4 pixels per user unit): the content
/// point under the pointer must not move.
#[wasm_bindgen_test]
async fn pointer_centred_zoom_holds_its_point_under_a_non_zero_origin_and_a_css_scale() -> Result<(), String> {
    let scene = scene_in_svg("vb-pointer", 400, 240, "-500 -300 1000 600", None)?;
    scene.show_toolbar(ToolbarOptions::default()).map_err(|e| e.to_string())?;
    let surface = pan_surface("vb-pointer")?;

    // Aim at user point (-200, -100): 300 units from the left, at 0.4 px per unit, is 120 px; 200 from the top is 80.
    let bounds = surface.get_bounding_client_rect();
    let (x, y) = (bounds.left().round() as i32 + 120, bounds.top().round() as i32 + 80);
    // Read back the user point the integer pixel position really is, since the `<svg>` may sit at a fractional offset.
    let pivot_x = (x as f64 - bounds.left()) / 0.4 - 500.0;
    let pivot_y = (y as f64 - bounds.top()) / 0.4 - 300.0;

    wheel(&surface, x, y, -100.0, true, false)?;
    next_frame().await?;

    // From the identity, zooming by 1.25 about the pivot gives t' = pivot - 1.25 * pivot. Those are user units, so they
    // come out the same whatever the CSS scale — and they are not what a `(0, 0)`-origin assumption would give.
    let (tx, ty, scale) = parse_view(&attr(&content("vb-pointer")?, "transform")?)?;
    check_close(scale, 1.25)?;
    check_close(tx, pivot_x - 1.25 * pivot_x)?;
    check_close(ty, pivot_y - 1.25 * pivot_y)
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// With the default `preserveAspectRatio` (`xMidYMid meet`), an `<svg>` wider than its `viewBox` shows *more* than the
/// `viewBox`. Here `0 0 400 400` in a 400 x 200 box scales by 0.5 and is centred, so the visible user-space area is
/// x from -200 to 600 and y from 0 to 400. The bar belongs on the real top edge of that, not the `viewBox`'s.
#[wasm_bindgen_test]
fn with_meet_the_visible_area_is_wider_than_the_view_box() -> Result<(), String> {
    let scene = scene_in_svg("vb-meet", 400, 200, "0 0 400 400", None)?;
    scene.show_toolbar(ToolbarOptions::default()).map_err(|e| e.to_string())?;

    check_rect(surface_rect("vb-meet")?, (-200.0, 0.0, 800.0, 400.0))?;
    // (800 - 134) / 2 - 200 = 133.
    let (x, y) = group_translate(&bar("vb-meet")?)?;
    check_close(x, 133.0)?;
    check_close(y, 8.0)?;

    // The centre is (200, 200) either way, so zooming about it gives (200 - 250, 200 - 250).
    scene.zoom_in().map_err(|e| e.to_string())?;
    let (tx, ty, _) = parse_view(&attr(&content("vb-meet")?, "transform")?)?;
    check_close(tx, -50.0)?;
    check_close(ty, -50.0)
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// With `slice`, the same box *crops* the `viewBox`: it scales by 1 and shows y from 100 to 300 only. A bar placed at
/// the `viewBox`'s own top edge (y = 8) would be cropped out of view entirely. It must sit on the visible top edge.
#[wasm_bindgen_test]
fn with_slice_the_toolbar_stays_inside_the_cropped_visible_area() -> Result<(), String> {
    let scene = scene_in_svg("vb-slice", 400, 200, "0 0 400 400", Some("xMidYMid slice"))?;
    scene.show_toolbar(ToolbarOptions::default()).map_err(|e| e.to_string())?;

    check_rect(surface_rect("vb-slice")?, (0.0, 100.0, 400.0, 200.0))?;
    let (x, y) = group_translate(&bar("vb-slice")?)?;
    check_close(x, 133.0)?;
    check_close(y, 108.0)
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// A CSS scale other than 1 changes how many pixels a pointer moves per user unit. Panning by 40 pixels at 2 pixels per
/// user unit moves the content by 20.
#[wasm_bindgen_test]
fn panning_under_a_css_scale_moves_the_content_by_user_units_not_pixels() -> Result<(), String> {
    let scene = scene_in_svg("vb-css-pan", 800, 600, "0 0 400 300", None)?;
    scene.show_toolbar(ToolbarOptions::default()).map_err(|e| e.to_string())?;
    let surface = pan_surface("vb-css-pan")?;

    dispatch_pointer_event(&surface, "pointerdown", 100, 100, 1)?;
    dispatch_pointer_event(&surface, "pointermove", 140, 100, 1)?;
    dispatch_pointer_event(&surface, "pointerup", 140, 100, 1)?;
    let (tx, ty, _) = parse_view(&attr(&content("vb-css-pan")?, "transform")?)?;
    check_close(tx, 20.0)?;
    check_close(ty, 0.0)
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
// Accessibility. A keyboard user can zoom from the toolbar, so they must also be able to move the view afterwards, and
// see where focus is.

/// A bubbling, cancelable `keydown`, optionally with Shift, Ctrl, or Meta held. Reports whether a listener cancelled it.
fn key(element: &web_sys::Element, key: &str, shift: bool, ctrl: bool, meta: bool) -> Result<bool, String> {
    let init = web_sys::KeyboardEventInit::new();
    init.set_key(key);
    init.set_bubbles(true);
    init.set_cancelable(true);
    init.set_shift_key(shift);
    init.set_ctrl_key(ctrl);
    init.set_meta_key(meta);
    let event =
        web_sys::KeyboardEvent::new_with_keyboard_event_init_dict("keydown", &init).map_err(|e| format!("{e:?}"))?;
    element.dispatch_event(&event).map_err(|e| format!("{e:?}"))?;
    Ok(event.default_prevented())
}

/// The content layer's `(tx, ty)`, or `(0, 0)` if it has never moved.
fn content_translate(id: &str) -> Result<(f64, f64), String> {
    match content(id)?.get_attribute("transform") {
        Some(transform) => parse_view(&transform).map(|(tx, ty, _)| (tx, ty)),
        None => Ok((0.0, 0.0)),
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Every button exposes the button role, a name that does not depend on its visible text, and a place in the Tab order.
#[wasm_bindgen_test]
fn every_toolbar_button_has_the_button_role_an_explicit_name_and_a_tab_stop() -> Result<(), String> {
    let scene = new_scene("a11y-buttons")?;
    scene.show_toolbar(ToolbarOptions::default()).map_err(|e| e.to_string())?;

    // "Reset" is drawn as "100%", so its name must not be read from the visible text.
    for (n, name) in ["Zoom in", "Zoom out", "Reset zoom"].iter().enumerate() {
        let b = button("a11y-buttons", n)?;
        check(
            attr(&b, "role")? == "button",
            &format!("button {n} does not have the button role"),
        )?;
        check(attr(&b, "aria-label")? == *name, &format!("button {n} is not named {name:?}"))?;
        check(attr(&b, "tabindex")? == "0", &format!("button {n} is not in the Tab order"))?;
    }
    Ok(())
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Keyboard focus is drawn explicitly — a thicker, differently coloured border — rather than left to whatever outline a
/// browser draws for a focused SVG element, and goes away again on blur.
#[wasm_bindgen_test]
fn a_focused_toolbar_button_shows_an_obvious_focus_ring_that_blur_removes() -> Result<(), String> {
    let scene = new_scene("a11y-focus")?;
    scene.show_toolbar(ToolbarOptions::default()).map_err(|e| e.to_string())?;
    let plus = button("a11y-focus", 0)?;
    let rect = plus.first_element_child().ok_or("a button has no rect")?;
    let resting = (attr(&rect, "stroke")?, attr_f64(&rect, "stroke-width")?);

    plus.dispatch_event(&web_sys::FocusEvent::new("focus").map_err(|e| format!("{e:?}"))?.into())
        .map_err(|e| format!("{e:?}"))?;
    check(attr(&rect, "stroke")? != resting.0, "focus did not change the border colour")?;
    check(attr_f64(&rect, "stroke-width")? > resting.1, "focus did not thicken the border")?;

    plus.dispatch_event(&web_sys::FocusEvent::new("blur").map_err(|e| format!("{e:?}"))?.into())
        .map_err(|e| format!("{e:?}"))?;
    check(attr(&rect, "stroke")? == resting.0, "blur did not restore the border colour")?;
    check_close(attr_f64(&rect, "stroke-width")?, resting.1)
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Activating a control marked `aria-disabled` does nothing at all — by click or by keyboard. In particular it writes
/// nothing to the DOM, so the content layer still has no `transform`.
#[wasm_bindgen_test]
fn activating_an_aria_disabled_button_does_nothing() -> Result<(), String> {
    let scene = new_scene("a11y-disabled")?;
    scene.show_toolbar(ToolbarOptions::default()).map_err(|e| e.to_string())?;

    // Unzoomed, "Reset" has nothing to do.
    let reset = button("a11y-disabled", 2)?;
    check(attr(&reset, "aria-disabled")? == "true", "test setup: Reset is not disabled")?;
    click(&reset)?;
    keydown(&reset, "Enter")?;
    keydown(&reset, " ")?;
    check(
        content("a11y-disabled")?.get_attribute("transform").is_none(),
        "activating a disabled Reset changed the view",
    )?;

    // At maximum zoom, "zoom in" has nothing to do.
    for _ in 0..30 {
        scene.zoom_in().map_err(|e| e.to_string())?;
    }
    let plus = button("a11y-disabled", 0)?;
    check(
        attr(&plus, "aria-disabled")? == "true",
        "test setup: zoom in is not disabled at the limit",
    )?;
    let before = attr(&content("a11y-disabled")?, "transform")?;
    click(&plus)?;
    keydown(&plus, "Enter")?;
    check(
        attr(&content("a11y-disabled")?, "transform")? == before,
        "activating a disabled zoom in changed the view",
    )?;
    check_close(scene.zoom_scale(), 4.0)
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// With panning on, the `<svg>` itself is a keyboard target with a role and a name that says how far it is zoomed.
#[wasm_bindgen_test]
async fn the_scene_is_a_named_keyboard_target_that_reports_its_zoom() -> Result<(), String> {
    let scene = new_scene("a11y-root")?;
    let root = required("#a11y-root")?;
    check(
        !root.has_attribute("tabindex"),
        "the scene is a keyboard target before anything asks for it",
    )?;

    scene.show_toolbar(ToolbarOptions::default()).map_err(|e| e.to_string())?;
    check(attr(&root, "tabindex")? == "0", "the scene is not in the Tab order")?;
    check(
        attr(&root, "role")? == "application",
        "the scene does not have the application role",
    )?;
    check(
        attr(&root, "aria-label")? == "Graph view, zoom 100%",
        "the scene's name does not report its zoom",
    )?;
    check(
        attr(&root, "aria-description")?.contains("Arrow keys pan"),
        "the scene does not say how to pan it",
    )?;

    scene.zoom_in().map_err(|e| e.to_string())?;
    check(
        attr(&root, "aria-label")? == "Graph view, zoom 125%",
        "the name did not follow a button zoom",
    )?;

    // The name is updated by a zoom that is deferred to an animation frame, too.
    scene.reset_view().map_err(|e| e.to_string())?;
    wheel(&pan_surface("a11y-root")?, 100, 100, -100.0, true, false)?;
    next_frame().await?;
    check(
        attr(&root, "aria-label")? == "Graph view, zoom 125%",
        "the name did not follow a wheel zoom",
    )
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Arrow keys move the view like scrolling: pressing right moves the view right, so the content moves left. One press is
/// 40 units, and Shift makes it five times further. This is the keyboard way back to content that zooming pushed out of
/// view.
#[wasm_bindgen_test]
fn arrow_keys_pan_the_view_and_shift_pans_further() -> Result<(), String> {
    let scene = new_scene("a11y-arrows")?;
    scene.show_toolbar(ToolbarOptions::default()).map_err(|e| e.to_string())?;
    let root = required("#a11y-arrows")?;

    check(
        key(&root, "ArrowRight", false, false, false)?,
        "a handled arrow key was not cancelled",
    )?;
    check(
        content_translate("a11y-arrows")? == (-40.0, 0.0),
        "ArrowRight did not move the content left by 40",
    )?;
    key(&root, "ArrowDown", false, false, false)?;
    check(
        content_translate("a11y-arrows")? == (-40.0, -40.0),
        "ArrowDown did not move the content up by 40",
    )?;
    key(&root, "ArrowLeft", false, false, false)?;
    key(&root, "ArrowUp", false, false, false)?;
    check(
        content_translate("a11y-arrows")? == (0.0, 0.0),
        "opposite arrows did not cancel out",
    )?;

    key(&root, "ArrowLeft", true, false, false)?;
    check(
        content_translate("a11y-arrows")? == (200.0, 0.0),
        "Shift did not make a press five times further",
    )?;
    // The 100% button is enabled by a pan, and undoes it.
    check(
        attr(&button("a11y-arrows", 2)?, "aria-disabled")? == "false",
        "a keyboard pan did not enable Reset",
    )?;
    click(&button("a11y-arrows", 2)?)?;
    check(
        content_translate("a11y-arrows")? == (0.0, 0.0),
        "Reset did not undo a keyboard pan",
    )
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The pan step is in the `<svg>`'s own units, so it looks the same on screen however far the content is zoomed.
#[wasm_bindgen_test]
fn a_keyboard_pan_step_is_the_same_distance_at_any_zoom() -> Result<(), String> {
    let scene = new_scene("a11y-arrows-zoom")?;
    scene.show_toolbar(ToolbarOptions::default()).map_err(|e| e.to_string())?;
    for _ in 0..4 {
        scene.zoom_in().map_err(|e| e.to_string())?;
    }
    let (before, _) = content_translate("a11y-arrows-zoom")?;
    key(&required("#a11y-arrows-zoom")?, "ArrowRight", false, false, false)?;
    let (after, _) = content_translate("a11y-arrows-zoom")?;
    check_close(after - before, -40.0)
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Plus, minus, and zero zoom from the keyboard, about the centre of the visible area.
#[wasm_bindgen_test]
fn plus_minus_and_zero_zoom_from_the_keyboard() -> Result<(), String> {
    let scene = new_scene("a11y-zoom-keys")?;
    scene.show_toolbar(ToolbarOptions::default()).map_err(|e| e.to_string())?;
    let root = required("#a11y-zoom-keys")?;

    check(key(&root, "+", false, false, false)?, "a handled zoom key was not cancelled")?;
    check_close(scene.zoom_scale(), 1.25)?;
    key(&root, "=", false, false, false)?; // the unshifted key that shares a cap with "+"
    check_close(scene.zoom_scale(), 1.5625)?;
    key(&root, "-", false, false, false)?;
    check_close(scene.zoom_scale(), 1.25)?;
    key(&root, "0", false, false, false)?;
    check_close(scene.zoom_scale(), 1.0)?;
    check(
        attr(&content("a11y-zoom-keys")?, "transform")? == "translate(0, 0) scale(1)",
        "zero did not restore the identity",
    )
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Keys the scene does not use, and keys held with Ctrl, Cmd, or Alt, are neither acted on nor cancelled — so the
/// browser's own page zoom (Ctrl or Cmd with plus or minus) and other shortcuts keep working.
#[wasm_bindgen_test]
fn unrelated_keys_and_browser_shortcuts_are_left_alone() -> Result<(), String> {
    let scene = new_scene("a11y-shortcuts")?;
    scene.show_toolbar(ToolbarOptions::default()).map_err(|e| e.to_string())?;
    let root = required("#a11y-shortcuts")?;

    check(!key(&root, "a", false, false, false)?, "an unrelated key was cancelled")?;
    check(
        !key(&root, "Tab", false, false, false)?,
        "Tab was cancelled, which would trap keyboard focus",
    )?;
    check(!key(&root, "ArrowRight", false, true, false)?, "Ctrl+Arrow was cancelled")?;
    check(
        !key(&root, "+", false, true, false)?,
        "Ctrl+plus was cancelled, which would block browser page zoom",
    )?;
    check(
        !key(&root, "-", false, false, true)?,
        "Cmd+minus was cancelled, which would block browser page zoom",
    )?;
    check(
        scene.zoom_scale() == 1.0 && content_translate("a11y-shortcuts")? == (0.0, 0.0),
        "an ignored key moved the view",
    )
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// A key pressed on a toolbar button bubbles up through the `<svg>`, but is the button's, not the scene's: an arrow key
/// on a focused button must not pan the view.
#[wasm_bindgen_test]
fn a_key_pressed_on_a_toolbar_button_does_not_pan_the_view() -> Result<(), String> {
    let scene = new_scene("a11y-bubble")?;
    scene.show_toolbar(ToolbarOptions::default()).map_err(|e| e.to_string())?;

    check(
        !key(&button("a11y-bubble", 0)?, "ArrowRight", false, false, false)?,
        "a button's arrow key was cancelled",
    )?;
    check(
        content_translate("a11y-bubble")? == (0.0, 0.0),
        "an arrow key on a button panned the view",
    )?;
    check_close(scene.zoom_scale(), 1.0)
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Each key group follows its own mode: the arrows are the keyboard side of panning, and plus, minus, and zero the
/// keyboard side of wheel zoom.
#[wasm_bindgen_test]
fn the_arrow_keys_follow_the_pan_mode_and_the_zoom_keys_follow_the_wheel_zoom_mode() -> Result<(), String> {
    let scene = new_scene("a11y-modes")?;
    scene.set_pan_mode(InputMode::On).map_err(|e| e.to_string())?;
    let root = required("#a11y-modes")?;

    // Panning on, wheel zoom off: arrows work, zoom keys do not.
    key(&root, "ArrowRight", false, false, false)?;
    check(content_translate("a11y-modes")? == (-40.0, 0.0), "the arrow keys did not pan")?;
    check(
        !key(&root, "+", false, false, false)?,
        "a zoom key was handled with wheel zoom off",
    )?;
    check_close(scene.zoom_scale(), 1.0)?;
    check(
        !attr(&root, "aria-description")?.contains("Plus and minus"),
        "the description mentions keys that are off",
    )?;

    // Wheel zoom on, panning off: the reverse.
    scene.set_pan_mode(InputMode::Off).map_err(|e| e.to_string())?;
    scene.set_wheel_zoom_mode(InputMode::On).map_err(|e| e.to_string())?;
    let before = content_translate("a11y-modes")?;
    check(
        !key(&root, "ArrowRight", false, false, false)?,
        "an arrow key was handled with panning off",
    )?;
    check(
        content_translate("a11y-modes")? == before,
        "an arrow key panned with panning off",
    )?;
    key(&root, "+", false, false, false)?;
    check_close(scene.zoom_scale(), 1.25)
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The `<svg>` is only a keyboard target while a gesture that needs it is active. Once none is, everything this added —
/// the tab stop, the role, the name, the description, and the key listener — is gone again.
#[wasm_bindgen_test]
fn hiding_the_toolbar_takes_the_keyboard_handling_off_the_scene() -> Result<(), String> {
    let scene = new_scene("a11y-teardown")?;
    scene.show_toolbar(ToolbarOptions::default()).map_err(|e| e.to_string())?;
    let root = required("#a11y-teardown")?;
    check(root.has_attribute("tabindex"), "test setup: no keyboard handling was added")?;

    scene.hide_toolbar();
    for name in ["tabindex", "role", "aria-label", "aria-description"] {
        check(
            !root.has_attribute(name),
            &format!("{name} was left on the scene after the toolbar was hidden"),
        )?;
    }
    check(
        !key(&root, "ArrowRight", false, false, false)?,
        "a key listener was left behind",
    )?;
    check(
        content_translate("a11y-teardown")? == (0.0, 0.0),
        "a leftover key listener panned the view",
    )
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Zoom changes must not cause disruptive announcements. Nothing here is a live region, and the state attributes are only
/// rewritten when they actually change, so a run of zoom steps that changes no button's state touches none of them.
#[wasm_bindgen_test]
fn zooming_adds_no_live_region_and_rewrites_no_unchanged_button_state() -> Result<(), String> {
    let scene = new_scene("a11y-quiet")?;
    scene.show_toolbar(ToolbarOptions::default()).map_err(|e| e.to_string())?;
    scene.zoom_in().map_err(|e| e.to_string())?;

    let live = web_sys::window()
        .and_then(|w| w.document())
        .ok_or("no document")?
        .query_selector_all("#a11y-quiet [aria-live], #a11y-quiet [role=\"status\"], #a11y-quiet [role=\"alert\"]")
        .map_err(|e| format!("{e:?}"))?
        .length();
    check(live == 0, "a live region was added, which would announce every zoom step")?;

    // Watch every button's state attributes through more zoom steps that leave every button's state as it was.
    let observed = ["aria-disabled", "opacity"];
    let before: Vec<Vec<String>> = (0..3)
        .map(|n| {
            button("a11y-quiet", n).map(|b| observed.iter().map(|a| b.get_attribute(a).unwrap_or_default()).collect())
        })
        .collect::<Result<_, _>>()?;
    scene.zoom_in().map_err(|e| e.to_string())?;
    scene.zoom_out().map_err(|e| e.to_string())?;
    scene.zoom_in().map_err(|e| e.to_string())?;
    let after: Vec<Vec<String>> = (0..3)
        .map(|n| {
            button("a11y-quiet", n).map(|b| observed.iter().map(|a| b.get_attribute(a).unwrap_or_default()).collect())
        })
        .collect::<Result<_, _>>()?;
    check(
        before == after,
        "a zoom step that changed no button's state changed its attributes",
    )
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
// Performance. Zoom and pan should cost the same however big the graph is: one group's `transform`, written at most once
// per animation frame, with nothing beneath it touched.

/// One recorded DOM mutation, reduced to what a test needs: whether it hit `content` itself, its type, and which
/// attribute it wrote.
type Mutation = (bool, String, Option<String>);

/// Records every DOM mutation anywhere at or below a target: attributes, children, and text.
///
/// The browser delivers records to the observer's callback at the next microtask checkpoint, which any `await` reaches.
/// So the callback is where they are collected. [`Recorder::pending`] reads only those not yet delivered, for a check
/// made before the test has yielded.
struct Recorder {
    observer: web_sys::MutationObserver,
    content: web_sys::Element,
    log: std::rc::Rc<std::cell::RefCell<Vec<Mutation>>>,
    // Kept alive for as long as the observer may call it.
    _callback: wasm_bindgen::closure::Closure<dyn FnMut(js_sys::Array, web_sys::MutationObserver)>,
}

fn summarise(records: &js_sys::Array, content: &web_sys::Element) -> Vec<Mutation> {
    (0..records.length())
        .map(|i| records.get(i).unchecked_into::<web_sys::MutationRecord>())
        .map(|record| {
            let on_content = record.target().is_some_and(|t| content.is_same_node(Some(&t)));
            (on_content, record.type_(), record.attribute_name())
        })
        .collect()
}

impl Recorder {
    fn watch(target: &web_sys::Element) -> Result<Self, String> {
        let log: std::rc::Rc<std::cell::RefCell<Vec<Mutation>>> = Default::default();
        let (sink, content) = (log.clone(), target.clone());
        let callback = wasm_bindgen::closure::Closure::<dyn FnMut(js_sys::Array, web_sys::MutationObserver)>::new(
            move |records: js_sys::Array, _observer| sink.borrow_mut().extend(summarise(&records, &content)),
        );

        let observer =
            web_sys::MutationObserver::new(callback.as_ref().unchecked_ref()).map_err(|e| format!("{e:?}"))?;
        let init = web_sys::MutationObserverInit::new();
        init.set_attributes(true);
        init.set_child_list(true);
        init.set_subtree(true);
        init.set_character_data(true);
        observer.observe_with_options(target, &init).map_err(|e| format!("{e:?}"))?;
        Ok(Self {
            observer,
            content: target.clone(),
            log,
            _callback: callback,
        })
    }

    /// Mutations made but not yet delivered — that is, made since the test last yielded.
    fn pending(&self) -> Vec<Mutation> {
        summarise(&self.observer.take_records(), &self.content)
    }

    /// Every mutation delivered so far. Call after an `await`, which is what delivers them.
    fn delivered(&self) -> Vec<Mutation> {
        self.log.borrow().clone()
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// A graph of nodes and connectors, then every way of zooming and panning it: buttons, wheel, drag, and keyboard. The
/// only thing that may change anywhere at or below the content layer is that layer's own `transform`. No node moves, no
/// connector is rerouted, and no label or marker is rewritten — which is what makes the cost independent of graph size.
#[wasm_bindgen_test]
async fn zooming_and_panning_change_only_the_content_layers_own_transform() -> Result<(), String> {
    let scene = new_scene("perf-only-transform")?;
    let mut nodes = Vec::new();
    for i in 0..6 {
        let x = 20.0 + 60.0 * f64::from(i % 3);
        let y = 20.0 + 70.0 * f64::from(i / 3);
        nodes.push(
            scene
                .add_node(Point::new(x, y), Size::new(40.0, 24.0), format!("n{i}"))
                .map_err(|e| e.to_string())?,
        );
    }
    for pair in nodes.windows(2) {
        scene.add_edge(pair[0], pair[1]).map_err(|e| e.to_string())?;
    }
    scene.show_toolbar(ToolbarOptions::default()).map_err(|e| e.to_string())?;

    let content_element = content("perf-only-transform")?;
    let surface = pan_surface("perf-only-transform")?;
    let root = required("#perf-only-transform")?;
    let recorder = Recorder::watch(&content_element)?;

    // Every route into the view.
    scene.zoom_in().map_err(|e| e.to_string())?;
    scene.zoom_out().map_err(|e| e.to_string())?;
    click(&button("perf-only-transform", 0)?)?;
    for _ in 0..12 {
        wheel(&surface, 100, 100, -10.0, true, false)?;
    }
    dispatch_pointer_event(&surface, "pointerdown", 100, 100, 1)?;
    dispatch_pointer_event(&surface, "pointermove", 150, 130, 1)?;
    dispatch_pointer_event(&surface, "pointerup", 150, 130, 1)?;
    key(&root, "ArrowRight", false, false, false)?;
    key(&root, "+", false, false, false)?;
    click(&button("perf-only-transform", 2)?)?;
    next_frame().await?;

    let mutations = recorder.delivered();
    check(
        !mutations.is_empty(),
        "test setup: nothing was recorded, so this proves nothing",
    )?;
    for (on_content, kind, attribute) in &mutations {
        check(*on_content, "a zoom or pan changed something beneath the content layer")?;
        check(kind == "attributes", &format!("a zoom or pan made a {kind:?} mutation"))?;
        check(
            attribute.as_deref() == Some("transform"),
            &format!("a zoom or pan wrote {attribute:?}"),
        )?;
    }
    Ok(())
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The cost of a burst is one write, not one per event. Twenty wheel events and a run of pan moves, all inside one frame,
/// are a single mutation of the content layer's `transform`.
#[wasm_bindgen_test]
async fn a_burst_of_wheel_and_pan_events_is_one_transform_write() -> Result<(), String> {
    let scene = new_scene("perf-one-write")?;
    scene.show_toolbar(ToolbarOptions::default()).map_err(|e| e.to_string())?;
    let content_element = content("perf-one-write")?;
    let surface = pan_surface("perf-one-write")?;
    next_frame().await?; // Let anything from setup settle, so the frame below is a clean one.

    let recorder = Recorder::watch(&content_element)?;
    for _ in 0..20 {
        wheel(&surface, 100, 100, -10.0, true, false)?;
    }
    dispatch_pointer_event(&surface, "pointerdown", 100, 100, 1)?;
    for step in 1..=10 {
        dispatch_pointer_event(&surface, "pointermove", 100 + 5 * step, 100, 1)?;
    }
    // Still inside the same frame: none of those thirty events has written the DOM yet.
    check(recorder.pending().is_empty(), "an event wrote the DOM ahead of its frame")?;

    next_frame().await?;
    let writes = recorder.delivered();
    check(
        writes.len() == 1,
        &format!("expected one write for the whole burst, found {}", writes.len()),
    )?;
    dispatch_pointer_event(&surface, "pointerup", 150, 100, 1)?;
    Ok(())
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Zoom and pan do not invalidate routing: every node's own position and every connector's own path are byte-for-byte
/// what they were, however far the view has been zoomed and panned.
#[wasm_bindgen_test]
fn zoom_and_pan_leave_every_node_position_and_connector_path_untouched() -> Result<(), String> {
    let scene = new_scene("perf-routing")?;
    let mut nodes = Vec::new();
    for i in 0..4 {
        nodes.push(
            scene
                .add_node(
                    Point::new(20.0 + 90.0 * f64::from(i), 40.0 + 30.0 * f64::from(i % 2)),
                    Size::new(50.0, 24.0),
                    format!("n{i}"),
                )
                .map_err(|e| e.to_string())?,
        );
    }
    for pair in nodes.windows(2) {
        scene.add_edge(pair[0], pair[1]).map_err(|e| e.to_string())?;
    }
    scene.show_toolbar(ToolbarOptions::default()).map_err(|e| e.to_string())?;

    let snapshot = || -> Result<Vec<String>, String> {
        let mut out = Vec::new();
        for n in 0..4 {
            out.push(attr(&nth_group("perf-routing", n)?, "transform")?);
        }
        for n in 0..3 {
            out.push(path_d(&nth_connector("perf-routing", n)?)?);
        }
        Ok(out)
    };
    let before = snapshot()?;

    for _ in 0..5 {
        scene.zoom_in().map_err(|e| e.to_string())?;
    }
    let surface = pan_surface("perf-routing")?;
    dispatch_pointer_event(&surface, "pointerdown", 100, 100, 1)?;
    dispatch_pointer_event(&surface, "pointermove", 160, 60, 1)?;
    dispatch_pointer_event(&surface, "pointerup", 160, 60, 1)?;
    key(&required("#perf-routing")?, "ArrowLeft", true, false, false)?;

    check(
        snapshot()? == before,
        "zooming and panning changed a node's position or a connector's path",
    )
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
// Lifecycle. Showing the toolbar builds the buttons, the pan surface, and the pointer, wheel, and keyboard handling.
// Hiding it takes them away. Do that repeatedly, and any handler that was not taken away is now stacked on the new one:
// one input would then do its job twice.

/// Sends exactly one of each kind of input to a scene, and checks that each did its job exactly once. Every input is
/// measured from a reset view, so a doubled handler shows up as double the distance or double the zoom.
async fn every_input_does_exactly_one_thing(scene: &Scene, id: &str) -> Result<(), String> {
    let surface = pan_surface(id)?;
    let root = required(&format!("#{id}"))?;

    // A toolbar button: one click is one step, not two.
    scene.reset_view().map_err(|e| e.to_string())?;
    click(&button(id, 0)?)?;
    check_close(scene.zoom_scale(), 1.25)?;

    // The keyboard: one press is one 40-unit step or one zoom step.
    scene.reset_view().map_err(|e| e.to_string())?;
    key(&root, "ArrowRight", false, false, false)?;
    check(
        content_translate(id)? == (-40.0, 0.0),
        "one arrow key press did not move the view exactly 40 units",
    )?;
    scene.reset_view().map_err(|e| e.to_string())?;
    key(&root, "+", false, false, false)?;
    check_close(scene.zoom_scale(), 1.25)?;

    // The wheel: one notch is one step.
    scene.reset_view().map_err(|e| e.to_string())?;
    wheel(&surface, 100, 100, -100.0, true, false)?;
    check_close(scene.zoom_scale(), 1.25)?;

    // A pan: 40 pixels of drag is 40 units of movement.
    scene.reset_view().map_err(|e| e.to_string())?;
    dispatch_pointer_event(&surface, "pointerdown", 100, 100, 1)?;
    dispatch_pointer_event(&surface, "pointermove", 140, 100, 1)?;
    dispatch_pointer_event(&surface, "pointerup", 140, 100, 1)?;
    check(
        content_translate(id)? == (40.0, 0.0),
        "a 40 pixel drag did not move the view exactly 40 units",
    )?;

    next_frame().await?;
    scene.reset_view().map_err(|e| e.to_string())
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The sequence itself: show, hide, show, hide, show. After it, the toolbar and its gestures behave exactly as they would
/// have after a single show.
#[wasm_bindgen_test]
async fn show_hide_show_hide_show_then_every_input_does_exactly_one_thing() -> Result<(), String> {
    let scene = new_scene("lifecycle-sequence")?;

    // A fresh scene is the baseline.
    scene.show_toolbar(ToolbarOptions::default()).map_err(|e| e.to_string())?;
    every_input_does_exactly_one_thing(&scene, "lifecycle-sequence").await?;
    scene.hide_toolbar();

    scene.show_toolbar(ToolbarOptions::default()).map_err(|e| e.to_string())?;
    scene.hide_toolbar();
    scene.show_toolbar(ToolbarOptions::default()).map_err(|e| e.to_string())?;
    scene.hide_toolbar();
    scene.show_toolbar(ToolbarOptions::default()).map_err(|e| e.to_string())?;

    every_input_does_exactly_one_thing(&scene, "lifecycle-sequence").await
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Many more cycles, mixing in a toolbar moved to every edge, replaced while shown, and re-shown with new options. One
/// stale listener from any of them would double an input.
#[wasm_bindgen_test]
async fn many_lifecycle_changes_still_leave_every_input_doing_exactly_one_thing() -> Result<(), String> {
    let scene = new_scene("lifecycle-many")?;

    for edge in [Side::North, Side::East, Side::South, Side::West, Side::North] {
        scene.show_toolbar(ToolbarOptions::new(edge)).map_err(|e| e.to_string())?;
        // Replace it while it is shown, which tears the old one down and builds another.
        scene.show_toolbar(ToolbarOptions::new(edge)).map_err(|e| e.to_string())?;
        scene.set_toolbar_edge(Side::East).map_err(|e| e.to_string())?;
        scene.hide_toolbar();
    }
    scene.show_toolbar(ToolbarOptions::default()).map_err(|e| e.to_string())?;

    every_input_does_exactly_one_thing(&scene, "lifecycle-many").await
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The same for the input modes: one gesture switched on, off, and on again, over and over, with the toolbar showing
/// throughout.
#[wasm_bindgen_test]
async fn many_mode_changes_still_leave_every_input_doing_exactly_one_thing() -> Result<(), String> {
    let scene = new_scene("lifecycle-modes")?;
    scene.show_toolbar(ToolbarOptions::default()).map_err(|e| e.to_string())?;

    for _ in 0..6 {
        for mode in [InputMode::Off, InputMode::On, InputMode::WithToolbar] {
            scene.set_pan_mode(mode).map_err(|e| e.to_string())?;
            scene.set_wheel_zoom_mode(mode).map_err(|e| e.to_string())?;
        }
    }
    check(
        scene.pan_enabled() && scene.wheel_zoom_enabled(),
        "test setup: the gestures are not active",
    )?;

    every_input_does_exactly_one_thing(&scene, "lifecycle-modes").await
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Once every `Scene` handle is dropped, no input route does anything at all: not a button, not the wheel, not a key, not a
/// drag. If a listener were kept alive by a reference cycle, the scene it points at would still be alive and would still
/// answer, moving the view. Nothing here has a handle left to read the zoom from, so the rendered DOM is what is checked.
#[wasm_bindgen_test]
fn a_dropped_scene_answers_no_input_of_any_kind() -> Result<(), String> {
    let scene = new_scene("lifecycle-dropped")?;
    scene.show_toolbar(ToolbarOptions::default()).map_err(|e| e.to_string())?;
    // Show and hide first, so the listeners left behind by earlier cycles, if any, are in play too.
    scene.hide_toolbar();
    scene.show_toolbar(ToolbarOptions::default()).map_err(|e| e.to_string())?;

    let plus = button("lifecycle-dropped", 0)?;
    let surface = pan_surface("lifecycle-dropped")?;
    let root = required("#lifecycle-dropped")?;
    drop(scene);

    click(&plus)?;
    keydown(&plus, "Enter")?;
    wheel(&surface, 100, 100, -100.0, true, false)?;
    key(&root, "ArrowRight", false, false, false)?;
    key(&root, "+", false, false, false)?;
    dispatch_pointer_event(&surface, "pointerdown", 100, 100, 1)?;
    dispatch_pointer_event(&surface, "pointermove", 160, 100, 1)?;
    dispatch_pointer_event(&surface, "pointerup", 160, 100, 1)?;

    check(
        content("lifecycle-dropped")?.get_attribute("transform").is_none(),
        "a dropped scene still answered input: a listener is holding it alive",
    )
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
// Changing the view in the middle of a pointer gesture. The view can change at any moment — the wheel, the keyboard, a
// toolbar button, or the application calling `zoom_in` — and a node drag or a pan that is already under way must go on
// tracking the pointer correctly. Both would otherwise rest on a picture of the view taken when the gesture began.

/// A node at (100, 100), draggable, in a scene with the toolbar shown.
fn draggable_node_scene(id: &str) -> Result<Scene, String> {
    let scene = new_scene(id)?;
    let node = scene
        .add_node(Point::new(100.0, 100.0), Size::new(60.0, 30.0), "A")
        .map_err(|e| e.to_string())?;
    scene.make_draggable(node).map_err(|e| e.to_string())?;
    scene.show_toolbar(ToolbarOptions::default()).map_err(|e| e.to_string())?;
    Ok(scene)
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Drag a node, zoom with the wheel at the pointer, then keep dragging. A wheel at the pointer holds the grabbed point
/// still, so the second move of 25 pixels at 1.25x is 20 units of content, not 25: the node ends at 100 + 25 + 20 = 145.
/// Using the matrix from the start of the drag would give 150.
#[wasm_bindgen_test]
async fn a_node_drag_carries_on_correctly_after_a_wheel_zoom_in_the_middle() -> Result<(), String> {
    let _scene = draggable_node_scene("mid-drag-wheel")?;
    let group = nth_group("mid-drag-wheel", 0)?;

    dispatch_pointer_event(&group, "pointerdown", 110, 110, 1)?;
    dispatch_pointer_event(&group, "pointermove", 135, 110, 1)?;
    wheel(&group, 135, 110, -100.0, true, false)?;
    dispatch_pointer_event(&group, "pointermove", 160, 120, 1)?;
    dispatch_pointer_event(&group, "pointerup", 160, 120, 1)?;
    next_frame().await?;

    let (x, y) = group_translate(&group)?;
    check_close(x, 145.0)?;
    check_close(y, 108.0)
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The same for a zoom that comes from the application rather than from an input: `zoom_in` is public, so a drag cannot
/// assume only the wheel or keyboard will change the view. That zoom is about the centre of the view, not the pointer,
/// so the grabbed point moves on screen. The node must follow the pointer to wherever it now is: the point of content
/// under the pointer, less the offset it was grabbed at.
#[wasm_bindgen_test]
async fn a_node_drag_carries_on_correctly_after_the_application_zooms_in_the_middle() -> Result<(), String> {
    let scene = draggable_node_scene("mid-drag-api")?;
    let group = nth_group("mid-drag-api", 0)?;
    let bounds = required("#mid-drag-api")?.get_bounding_client_rect();
    let (left, top) = (bounds.left(), bounds.top());

    // Grabbed 10 units in from the node's corner, on both axes.
    let (grab_x, grab_y) = ((left + 110.0).round() as i32, (top + 110.0).round() as i32);
    let offset = (grab_x as f64 - left - 100.0, grab_y as f64 - top - 100.0);

    dispatch_pointer_event(&group, "pointerdown", grab_x, grab_y, 1)?;
    dispatch_pointer_event(&group, "pointermove", grab_x + 25, grab_y, 1)?;
    scene.zoom_in().map_err(|e| e.to_string())?;
    let (end_x, end_y) = (grab_x + 40, grab_y + 20);
    dispatch_pointer_event(&group, "pointermove", end_x, end_y, 1)?;
    dispatch_pointer_event(&group, "pointerup", end_x, end_y, 1)?;
    next_frame().await?;

    // Zoom about the centre (200, 150) from the identity: t = centre - 1.25 * centre = -50, -37.5, at scale 1.25.
    let (tx, ty, scale) = (-50.0, -37.5, 1.25);
    let (pointer_x, pointer_y) = (end_x as f64 - left, end_y as f64 - top);
    let expected = ((pointer_x - tx) / scale - offset.0, (pointer_y - ty) / scale - offset.1);

    let (x, y) = group_translate(&group)?;
    check_close(x, expected.0)?;
    check_close(y, expected.1)
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// A wheel zoom is written to the DOM one frame late. If a drag begins inside that frame, the node's screen matrix, read
/// from the DOM, still shows the old zoom while the view already has the new one. The drag must not be built on that
/// stale matrix: at 1.25x, 25 pixels is 20 units.
#[wasm_bindgen_test]
async fn a_drag_that_starts_before_a_pending_zoom_has_been_drawn_uses_the_new_zoom() -> Result<(), String> {
    let _scene = draggable_node_scene("mid-drag-pending")?;
    let group = nth_group("mid-drag-pending", 0)?;
    let root_bounds = required("#mid-drag-pending")?.get_bounding_client_rect();
    let pivot = (
        (root_bounds.left() + 5.0).round() as i32,
        (root_bounds.top() + 5.0).round() as i32,
    );

    // Zoom about a corner, far from the node, and press straight away: no frame has passed.
    wheel(&pan_surface("mid-drag-pending")?, pivot.0, pivot.1, -100.0, true, false)?;
    let node = group.query_selector("rect").map_err(|e| format!("{e:?}"))?.ok_or("no rect")?;
    let _ = node;
    dispatch_pointer_event(&group, "pointerdown", 150, 150, 1)?;
    dispatch_pointer_event(&group, "pointermove", 175, 150, 1)?;
    dispatch_pointer_event(&group, "pointerup", 175, 150, 1)?;
    next_frame().await?;

    let (x, _) = group_translate(&group)?;
    check_close(x, 120.0) // 100 + 25 / 1.25
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Pan, zoom with the wheel at the pointer, then keep panning. The zoom must not be thrown away by the next move: the
/// pan is 30 pixels, then a zoom of 1.25 about the pointer, then 30 more pixels across and 10 down.
#[wasm_bindgen_test]
async fn a_pan_carries_on_correctly_after_a_wheel_zoom_in_the_middle() -> Result<(), String> {
    let scene = new_scene("mid-pan-wheel")?;
    scene.show_toolbar(ToolbarOptions::default()).map_err(|e| e.to_string())?;
    let surface = pan_surface("mid-pan-wheel")?;
    let bounds = surface.get_bounding_client_rect();
    let (left, top) = (bounds.left(), bounds.top());
    let (x0, y0) = ((left + 100.0).round() as i32, (top + 100.0).round() as i32);

    dispatch_pointer_event(&surface, "pointerdown", x0, y0, 1)?;
    dispatch_pointer_event(&surface, "pointermove", x0 + 30, y0, 1)?;
    wheel(&surface, x0 + 30, y0, -100.0, true, false)?;
    dispatch_pointer_event(&surface, "pointermove", x0 + 60, y0 + 10, 1)?;
    dispatch_pointer_event(&surface, "pointerup", x0 + 60, y0 + 10, 1)?;
    next_frame().await?;

    // After the first move the translation is (30, 0). Zooming by 1.25 about the pointer at (px, py) gives
    // p - 1.25 * (p - t). The second move then adds (30, 10).
    let (px, py) = ((x0 + 30) as f64 - left, y0 as f64 - top);
    let expected = (px - 1.25 * (px - 30.0) + 30.0, py - 1.25 * (py - 0.0) + 10.0);

    let (tx, ty, scale) = parse_view(&attr(&content("mid-pan-wheel")?, "transform")?)?;
    check_close(scale, 1.25)?;
    check_close(tx, expected.0)?;
    check_close(ty, expected.1)
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The same for a zoom the application makes: pan, `zoom_in`, pan. It zooms about the centre of the view, (200, 150).
#[wasm_bindgen_test]
async fn a_pan_carries_on_correctly_after_the_application_zooms_in_the_middle() -> Result<(), String> {
    let scene = new_scene("mid-pan-api")?;
    scene.show_toolbar(ToolbarOptions::default()).map_err(|e| e.to_string())?;
    let surface = pan_surface("mid-pan-api")?;
    let bounds = surface.get_bounding_client_rect();
    let (x0, y0) = ((bounds.left() + 100.0).round() as i32, (bounds.top() + 100.0).round() as i32);

    dispatch_pointer_event(&surface, "pointerdown", x0, y0, 1)?;
    dispatch_pointer_event(&surface, "pointermove", x0 + 30, y0, 1)?;
    scene.zoom_in().map_err(|e| e.to_string())?;
    dispatch_pointer_event(&surface, "pointermove", x0 + 60, y0 + 10, 1)?;
    dispatch_pointer_event(&surface, "pointerup", x0 + 60, y0 + 10, 1)?;
    next_frame().await?;

    // Pan to (30, 0), zoom about the centre (200, 150), then pan by (30, 10).
    let expected = (200.0 - 1.25 * (200.0 - 30.0) + 30.0, 150.0 - 1.25 * (150.0 - 0.0) + 10.0);
    let (tx, ty, scale) = parse_view(&attr(&content("mid-pan-api")?, "transform")?)?;
    check_close(scale, 1.25)?;
    check_close(tx, expected.0)?;
    check_close(ty, expected.1)
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Zoom in the middle of a pan, from the keyboard, and reset in the middle of a node drag. Whatever changes the view, the
/// gesture composes with it.
#[wasm_bindgen_test]
async fn the_keyboard_and_a_toolbar_button_also_compose_with_a_gesture_in_progress() -> Result<(), String> {
    // A pan, with the keyboard zooming in the middle: `+` zooms about the centre (200, 150).
    let scene = new_scene("mid-pan-key")?;
    scene.show_toolbar(ToolbarOptions::default()).map_err(|e| e.to_string())?;
    let surface = pan_surface("mid-pan-key")?;
    let bounds = surface.get_bounding_client_rect();
    let (x0, y0) = ((bounds.left() + 100.0).round() as i32, (bounds.top() + 100.0).round() as i32);

    dispatch_pointer_event(&surface, "pointerdown", x0, y0, 1)?;
    dispatch_pointer_event(&surface, "pointermove", x0 + 30, y0, 1)?;
    key(&required("#mid-pan-key")?, "+", false, false, false)?;
    dispatch_pointer_event(&surface, "pointermove", x0 + 50, y0, 1)?;
    dispatch_pointer_event(&surface, "pointerup", x0 + 50, y0, 1)?;
    next_frame().await?;
    let (tx, _, scale) = parse_view(&attr(&content("mid-pan-key")?, "transform")?)?;
    check_close(scale, 1.25)?;
    check_close(tx, 200.0 - 1.25 * (200.0 - 30.0) + 20.0)?;

    // A node drag, with the "100%" button pressed in the middle: the view goes back to the identity, so the pointer is over
    // a different point of content. The node must follow the pointer to it, still held at the same offset.
    let scene = draggable_node_scene("mid-drag-button")?;
    scene.zoom_in().map_err(|e| e.to_string())?;
    next_frame().await?;
    let group = nth_group("mid-drag-button", 0)?;
    let bounds = required("#mid-drag-button")?.get_bounding_client_rect();
    let (left, top) = (bounds.left(), bounds.top());
    let (gx, gy) = ((left + 85.0).round() as i32, (top + 97.0).round() as i32);

    // Zoomed by 1.25 about the centre, the view is scale 1.25 and translation (-50, -37.5). The pointer is grabbing the
    // node at this offset from its corner, in content units.
    let (tx, ty, scale) = (-50.0, -37.5, 1.25);
    let offset = ((gx as f64 - left - tx) / scale - 100.0, (gy as f64 - top - ty) / scale - 100.0);

    dispatch_pointer_event(&group, "pointerdown", gx, gy, 1)?;
    dispatch_pointer_event(&group, "pointermove", gx + 25, gy, 1)?;
    click(&button("mid-drag-button", 2)?)?;
    check_close(scene.zoom_scale(), 1.0)?;
    dispatch_pointer_event(&group, "pointermove", gx + 50, gy, 1)?;
    dispatch_pointer_event(&group, "pointerup", gx + 50, gy, 1)?;
    next_frame().await?;

    // At the identity, content and the `<svg>`'s user space are the same, so the pointer is at (gx + 50 - left, gy - top).
    let (x, y) = group_translate(&group)?;
    check_close(x, (gx as f64 + 50.0 - left) - offset.0)?;
    check_close(y, (gy as f64 - top) - offset.1)
}
