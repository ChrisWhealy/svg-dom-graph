//! `Scene::show_scene_title`/`hide_scene_title`/`has_scene_title`: a heading-style title for the whole scene, fixed to
//! one edge of its own visible area — drawn attributes, edge placement, replacing an existing title, and option
//! validation.

use crate::common::{attr_f64, check, check_close, make_svg};
use svg_dom::root::utils::Size;
use svg_dom_graph::{
    Error,
    scene::{Scene, SceneTitleOptions, Side},
};
use wasm_bindgen_test::wasm_bindgen_test;

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The rendered `<text role="heading">`, if any, under `container_id`.
fn heading(container_id: &str) -> Result<Option<web_sys::Element>, String> {
    web_sys::window()
        .and_then(|w| w.document())
        .ok_or("no document")?
        .query_selector(&format!("#{container_id} > text[role=\"heading\"]"))
        .map_err(|e| format!("{e:?}"))
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[wasm_bindgen_test]
fn there_is_no_scene_title_until_one_is_shown_and_none_after_it_is_hidden() -> Result<(), String> {
    let svg = make_svg("title-show-hide", Size::new(400.0, 300.0), Size::new(400.0, 300.0));
    let scene = Scene::new(svg).map_err(|e| e.to_string())?;
    check(!scene.has_scene_title(), "a new scene already has a title")?;
    check(
        heading("title-show-hide")?.is_none(),
        "a title is in the DOM before being shown",
    )?;

    scene
        .show_scene_title("SHA3 Theta Function", SceneTitleOptions::default())
        .map_err(|e| e.to_string())?;
    check(scene.has_scene_title(), "has_scene_title is false after show_scene_title")?;
    heading("title-show-hide")?.ok_or("no heading found after show_scene_title")?;

    scene.hide_scene_title();
    check(!scene.has_scene_title(), "has_scene_title is true after hide_scene_title")?;
    check(
        heading("title-show-hide")?.is_none(),
        "the title is still in the DOM after hide_scene_title",
    )
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The default options draw the title's own text content, `role="heading"`, the default `aria-level`, and bold and
/// underlined styling. That is the "heading-style text" the feature request asked default options to look like.
#[wasm_bindgen_test]
fn show_scene_title_with_default_options_draws_a_bold_underlined_heading() -> Result<(), String> {
    let svg = make_svg("title-defaults", Size::new(400.0, 300.0), Size::new(400.0, 300.0));
    let scene = Scene::new(svg).map_err(|e| e.to_string())?;
    scene
        .show_scene_title("SHA3 Theta Function", SceneTitleOptions::default())
        .map_err(|e| e.to_string())?;

    let text = heading("title-defaults")?.ok_or("no heading found")?;
    check(
        text.text_content().as_deref() == Some("SHA3 Theta Function"),
        &format!("unexpected text: {:?}", text.text_content()),
    )?;
    check(
        text.get_attribute("aria-level").as_deref() == Some("2"),
        &format!("unexpected aria-level: {:?}", text.get_attribute("aria-level")),
    )?;
    check(
        text.get_attribute("font-weight").as_deref() == Some("bold"),
        "default options did not draw a bold title",
    )?;
    check(
        text.get_attribute("text-decoration").as_deref() == Some("underline"),
        "default options did not draw an underlined title",
    )?;
    check_close(attr_f64(&text, "font-size")?, 20.0)
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// `bold`/`underline` set to `false` omit their own attribute entirely, rather than writing e.g. `font-weight="normal"`
/// — there is nothing to override the browser's own default with.
#[wasm_bindgen_test]
fn show_scene_title_without_bold_or_underline_omits_those_attributes() -> Result<(), String> {
    let svg = make_svg("title-plain", Size::new(400.0, 300.0), Size::new(400.0, 300.0));
    let scene = Scene::new(svg).map_err(|e| e.to_string())?;
    let options = SceneTitleOptions::default().with_bold(false).with_underline(false);
    scene.show_scene_title("Plain", options).map_err(|e| e.to_string())?;

    let text = heading("title-plain")?.ok_or("no heading found")?;
    check(text.get_attribute("font-weight").is_none(), "font-weight was written anyway")?;
    check(
        text.get_attribute("text-decoration").is_none(),
        "text-decoration was written anyway",
    )
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// A North or South title is centred horizontally on the `<svg>`'s own visible area, exactly at its own midpoint. This
/// holds regardless of the title's own rendered width, since `text-anchor="middle"` positioning cancels it out. A West
/// or East title is centred vertically the same way.
#[wasm_bindgen_test]
fn each_edge_centres_the_title_along_the_visible_areas_own_opposite_axis() -> Result<(), String> {
    let svg = make_svg("title-edges", Size::new(400.0, 300.0), Size::new(400.0, 300.0));
    let scene = Scene::new(svg).map_err(|e| e.to_string())?;

    for edge in [Side::North, Side::South] {
        scene
            .show_scene_title("Title", SceneTitleOptions::default().with_edge(edge))
            .map_err(|e| e.to_string())?;
        let text = heading("title-edges")?.ok_or("no heading found")?;
        check_close(attr_f64(&text, "x")?, 200.0)?;
    }
    for edge in [Side::West, Side::East] {
        scene
            .show_scene_title("Title", SceneTitleOptions::default().with_edge(edge))
            .map_err(|e| e.to_string())?;
        let text = heading("title-edges")?.ok_or("no heading found")?;
        check_close(attr_f64(&text, "y")?, 150.0)?;
    }
    Ok(())
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// `North` sits in the visible area's own top half, and `South` in its own bottom half. The check is qualitative, since
/// the exact offset depends on the title's own rendered height, which this crate measures rather than assumes.
#[wasm_bindgen_test]
fn north_and_south_sit_on_their_own_expected_half_of_the_visible_area() -> Result<(), String> {
    let svg = make_svg("title-halves", Size::new(400.0, 300.0), Size::new(400.0, 300.0));
    let scene = Scene::new(svg).map_err(|e| e.to_string())?;

    scene
        .show_scene_title("Title", SceneTitleOptions::default().with_edge(Side::North))
        .map_err(|e| e.to_string())?;
    let north_y = attr_f64(&heading("title-halves")?.ok_or("no heading found")?, "y")?;
    check(
        north_y < 150.0,
        &format!("North title's own y ({north_y}) was not in the top half"),
    )?;

    scene
        .show_scene_title("Title", SceneTitleOptions::default().with_edge(Side::South))
        .map_err(|e| e.to_string())?;
    let south_y = attr_f64(&heading("title-halves")?.ok_or("no heading found")?, "y")?;
    check(
        south_y > 150.0,
        &format!("South title's own y ({south_y}) was not in the bottom half"),
    )
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Showing a second title replaces the first, rather than stacking a second one.
#[wasm_bindgen_test]
fn showing_a_scene_title_twice_leaves_exactly_one() -> Result<(), String> {
    let svg = make_svg("title-twice", Size::new(400.0, 300.0), Size::new(400.0, 300.0));
    let scene = Scene::new(svg).map_err(|e| e.to_string())?;
    scene
        .show_scene_title("First", SceneTitleOptions::default())
        .map_err(|e| e.to_string())?;
    scene
        .show_scene_title("Second", SceneTitleOptions::default())
        .map_err(|e| e.to_string())?;

    let document = web_sys::window().and_then(|w| w.document()).ok_or("no document")?;
    let all = document
        .query_selector_all("#title-twice > text[role=\"heading\"]")
        .map_err(|e| format!("{e:?}"))?;
    check(
        all.length() == 1,
        &format!("expected exactly one title, found {}", all.length()),
    )?;

    let text = heading("title-twice")?.ok_or("no heading found")?;
    check(
        text.text_content().as_deref() == Some("Second"),
        &format!("expected the second title's own text, got {:?}", text.text_content()),
    )
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Invalid options are rejected — a non-positive `font_size`, a negative `margin`, or an `aria_level` of `0` — and
/// leave any title already shown exactly as it was.
#[wasm_bindgen_test]
fn invalid_scene_title_options_are_rejected_and_leave_an_existing_title_alone() -> Result<(), String> {
    let svg = make_svg("title-invalid", Size::new(400.0, 300.0), Size::new(400.0, 300.0));
    let scene = Scene::new(svg).map_err(|e| e.to_string())?;
    scene
        .show_scene_title("Kept", SceneTitleOptions::default())
        .map_err(|e| e.to_string())?;

    let attempts = [
        SceneTitleOptions::default().with_font_size(0.0),
        SceneTitleOptions::default().with_font_size(f64::NAN),
        SceneTitleOptions::default().with_margin(-1.0),
        SceneTitleOptions::default().with_aria_level(0),
    ];
    for options in attempts {
        let result = scene.show_scene_title("Rejected", options);
        check(
            matches!(result, Err(Error::InvalidSceneTitleOptions(_))),
            &format!(
                "{options:?} should have been rejected as Err(Error::InvalidSceneTitleOptions(_)), got {result:?}"
            ),
        )?;
    }

    let text = heading("title-invalid")?.ok_or("no heading found after every call was rejected")?;
    check(
        text.text_content().as_deref() == Some("Kept"),
        &format!(
            "the existing title changed despite every call being rejected: {:?}",
            text.text_content()
        ),
    )
}
