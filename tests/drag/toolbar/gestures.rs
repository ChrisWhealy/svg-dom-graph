//! Pan and wheel-zoom gesture mechanics. They cover dragging a node under zoom, the pan surface itself, and
//! following/releasing/ cancelling a pan. They also cover ctrl/cmd-plus-wheel, wheel direction and amount, coalescing a
//! burst into one frame, and a toolbar button winning over a pending frame.

use super::support::*;
use crate::common::{
    attr_f64, check, check_close, dispatch_pointer_event, dispatch_pointer_event_with_button, group_translate,
    nth_group, the_connector,
};
use svg_dom::root::utils::{Point, Size};
use svg_dom_graph::scene::ToolbarOptions;
use wasm_bindgen_test::*;

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

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The pan surface exists only while a toolbar is shown, sits directly beneath the content layer, and covers the whole
/// visible area.
#[wasm_bindgen_test]
fn a_pan_surface_exists_only_while_the_toolbar_is_shown() -> Result<(), String> {
    let scene = new_scene("tb-pan-exists")?;
    check(
        query("#tb-pan-exists > rect[aria-hidden=\"true\"]")?.is_none(),
        "a pan surface exists before any toolbar",
    )?;

    scene.show_toolbar(ToolbarOptions::default()).map_err(|e| e.to_string())?;
    let surface = pan_surface("tb-pan-exists")?;
    // Beneath the content layer, with the keyboard focus target between the two: surface, target, content.
    let target = surface.next_element_sibling().ok_or("nothing follows the pan surface")?;
    check(
        target.get_attribute("role").as_deref() == Some("application"),
        "the keyboard target does not directly follow the pan surface",
    )?;
    let next = target.next_element_sibling().ok_or("nothing follows the keyboard target")?;
    check(
        next.get_attribute("class").as_deref() == Some("svg-dom-graph-content"),
        "the pan surface and keyboard target are not directly beneath the content layer",
    )?;
    check_close(attr_f64(&surface, "width")?, 400.0)?;
    check_close(attr_f64(&surface, "height")?, 300.0)?;

    scene.hide_toolbar();
    check(
        query("#tb-pan-exists > rect[aria-hidden=\"true\"]")?.is_none(),
        "the pan surface outlived the toolbar",
    )
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
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

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Every move applies to where the pan started, so the content follows the pointer, and finishing a pan changes nothing
/// further.
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

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
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

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
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

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
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

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Ctrl+wheel and Cmd+wheel both zoom, about the pointer. One notch up at (100, 100) is one 1.25 step, and the content
/// point under the pointer stays put: (100 - 100 * 1.25) = -25 on each axis.
#[wasm_bindgen_test]
async fn ctrl_or_cmd_plus_wheel_zooms_about_the_pointer() -> Result<(), String> {
    let scene = new_scene("tb-wheel")?;
    scene.show_toolbar(ToolbarOptions::default()).map_err(|e| e.to_string())?;
    let surface = pan_surface("tb-wheel")?;

    // Every test's `<svg>` shares one page, so client coordinates must be built from this one's own position. The
    // client position is a whole number of pixels, but the `<svg>` itself can sit at a fractional offset. So the pivot
    // the scene sees is the difference, not exactly (100, 100).
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

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
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

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Without a modifier the wheel is neither handled nor cancelled, so the page still scrolls.
#[wasm_bindgen_test]
fn the_wheel_alone_does_not_zoom_and_is_not_cancelled() -> Result<(), String> {
    let scene = new_scene("tb-wheel-plain")?;
    scene.show_toolbar(ToolbarOptions::default()).map_err(|e| e.to_string())?;
    let surface = pan_surface("tb-wheel-plain")?;

    check(!wheel(&surface, 100, 100, -100.0, false, false)?, "a plain wheel was cancelled")?;
    check_close(scene.zoom_scale(), 1.0)
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
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

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Wheel zoom belongs to the toolbar, like panning. Without one, a scene leaves the wheel alone entirely, and hiding
/// the toolbar takes the wheel handling away with it.
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

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
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

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
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

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
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

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// A button pressed while a wheel burst is still waiting for its frame acts on the latest view. The frame that follows
/// cannot resurrect the stale wheel view over it.
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

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// A pan is written to the DOM one frame late while it runs. The moment the pointer is released, the DOM is exact.
/// Nothing is left for a later frame to catch up on.
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
