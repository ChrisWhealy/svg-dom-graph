//! Changing the view in the middle of a pointer gesture. The view can change at any moment: the wheel, the keyboard, a
//! toolbar button, or the application calling `zoom_in`. A node drag or a pan that is already under way must go on
//! tracking the pointer correctly. Both would otherwise rest on a picture of the view taken when the gesture began.

use super::support::*;
use crate::common::{check_close, dispatch_pointer_event, group_translate, nth_group};
use svg_dom_graph::scene::ToolbarOptions;
use wasm_bindgen_test::*;

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Drag a node, zoom with the wheel at the pointer, then keep dragging. A wheel at the pointer holds the grabbed point
/// still. So the second move of 25 pixels at 1.25x is 20 units of content, not 25. The node ends at 100 + 25 + 20 =
/// 145.
/// 145. Using the matrix from the start of the drag would give 150.
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

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The same holds for a zoom that comes from the application rather than from an input. `zoom_in` is public, so a drag
/// cannot assume only the wheel or keyboard will change the view. That zoom is about the centre of the view, not the
/// pointer, so the grabbed point moves on screen. The node must follow the pointer to wherever it now is: the point of
/// content under the pointer, less the offset it was grabbed at.
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

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// A wheel zoom is written to the DOM one frame late. If a drag begins inside that frame, the node's screen matrix,
/// read from the DOM, still shows the old zoom while the view already has the new one. The drag must not be built on
/// that stale matrix: at 1.25x, 25 pixels is 20 units.
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

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Pan, zoom with the wheel at the pointer, then keep panning. The zoom must not be thrown away by the next move. The
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

    // After the first move the translation is (30, 0). Zooming by 1.25 about the pointer at (px, py) gives p - 1.25 *
    // (p - t). The second move then adds (30, 10).
    let (px, py) = ((x0 + 30) as f64 - left, y0 as f64 - top);
    let expected = (px - 1.25 * (px - 30.0) + 30.0, py - 1.25 * (py - 0.0) + 10.0);

    let (tx, ty, scale) = parse_view(&attr(&content("mid-pan-wheel")?, "transform")?)?;
    check_close(scale, 1.25)?;
    check_close(tx, expected.0)?;
    check_close(ty, expected.1)
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
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

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Zoom in the middle of a pan, from the keyboard, and reset in the middle of a node drag. Whatever changes the view,
/// the gesture composes with it.
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
    key(&focus_target("mid-pan-key")?, "+", false, false, false)?;
    dispatch_pointer_event(&surface, "pointermove", x0 + 50, y0, 1)?;
    dispatch_pointer_event(&surface, "pointerup", x0 + 50, y0, 1)?;
    next_frame().await?;
    let (tx, _, scale) = parse_view(&attr(&content("mid-pan-key")?, "transform")?)?;
    check_close(scale, 1.25)?;
    check_close(tx, 200.0 - 1.25 * (200.0 - 30.0) + 20.0)?;

    // A node drag, with the "100%" button pressed in the middle: the view goes back to the identity, so the pointer is
    // over a different point of content. The node must follow the pointer to it, still held at the same offset.
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

    // At the identity, content and the `<svg>`'s user space are the same, so the pointer is at (gx + 50 - left, gy -
    // top).
    let (x, y) = group_translate(&group)?;
    check_close(x, (gx as f64 + 50.0 - left) - offset.0)?;
    check_close(y, (gy as f64 - top) - offset.1)
}
