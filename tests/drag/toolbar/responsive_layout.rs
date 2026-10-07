//! Responsive layouts. The toolbar and the pan surface are laid out in the `<svg>`'s own user space, and the scene
//! cannot observe that changing.

use super::support::*;
use crate::common::{attr_f64, check, check_close, group_translate};
use svg_dom_graph::scene::{Scene, ToolbarOptions};
use wasm_bindgen_test::*;

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

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// With no `viewBox` and no size attributes, the layout uses the rendered size rather than the cached `0 x 0`. A 300 x
/// 200 north bar of default width 134 sits at x = (300 - 134) / 2 = 83, and the pan surface covers the whole box.
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

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
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

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
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
