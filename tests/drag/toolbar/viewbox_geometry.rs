//! Non-zero `viewBox` origins, mismatched aspect ratios, and CSS scaling. The scene must work in the `<svg>`'s own
//! user space — the coordinates its content is drawn in — wherever the browser places that space on screen.

use super::support::*;
use crate::common::{attr_f64, check, check_close, dispatch_pointer_event, group_translate};
use svg_dom_graph::scene::{Side, ToolbarOptions};
use wasm_bindgen_test::*;

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

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
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

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
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

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
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

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
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

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
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

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
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
