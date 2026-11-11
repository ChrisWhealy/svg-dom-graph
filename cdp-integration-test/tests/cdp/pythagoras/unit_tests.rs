//! The tests of [`super`], which holds the helpers they use.

use super::*;

#[test]
fn the_scene_is_560_pixels_square_with_no_plot_no_slider_and_nothing_outside_it() -> Result<(), String> {
    let tab = open()?;
    let size = text(
        &tab,
        &format!(
            "(() => {{ const s = document.querySelector('{SVG}'); \
             return s.getAttribute('width') + 'x' + s.getAttribute('height'); }})()"
        ),
    )?;
    check_eq(size, "560x560".to_owned())?;
    check_eq(count(&tab, "#panel-pythagoras svg")?, 1)?;
    check_eq(count(&tab, &format!("{SVG} g[role=\"img\"]"))?, 0)?;
    check_eq(count(&tab, "#panel-pythagoras input")?, 0)?;
    check_eq(count(&tab, &format!("{SVG} line"))?, 0)
}

#[test]
fn the_squares_are_a_blue_one_on_the_hypotenuse_and_two_green_ones_that_can_be_seen_through() -> Result<(), String> {
    let tab = open()?;
    for (selector, colour) in [
        ("polygon.square-hypotenuse", "#2563eb"),
        ("polygon.square-width", "#1f7a37"),
        ("polygon.square-height", "#1f7a37"),
    ] {
        check_eq(attribute(&tab, selector, "fill")?, colour.to_owned())?;
        let opacity: f64 = attribute(&tab, selector, "fill-opacity")?.parse().map_err(|e| format!("{e}"))?;
        check(
            opacity > 0.0 && opacity < 1.0,
            &format!("{selector} is semi-transparent: {opacity}"),
        )?;
    }
    Ok(())
}

#[test]
fn c_starts_at_45_degrees_and_the_two_green_squares_are_a_half_each() -> Result<(), String> {
    let tab = open()?;
    check_eq(handle(&tab, "aria-valuenow")?, "45".to_owned())?;
    let (width, height, hypotenuse) = areas(&tab)?;
    check(
        (width - 0.5).abs() < 1e-6 && (height - 0.5).abs() < 1e-6,
        &format!("{width} and {height}"),
    )?;
    check((hypotenuse - 1.0).abs() < 1e-6, &format!("the blue square is {hypotenuse}"))?;
    check_eq(written(&tab, ".equation")?, "a\u{b2} + b\u{b2} = c\u{b2}".to_owned())?;
    check_eq(written(&tab, ".numbers")?, "0.50 + 0.50 = 1.00".to_owned())?;
    check_eq(written(&tab, ".text-hypotenuse")?, "c\u{b2} = 1.00".to_owned())?;
    check_eq(written(&tab, ".text-width")?, "a\u{b2} = 0.50".to_owned())?;
    check_eq(written(&tab, ".text-height")?, "b\u{b2} = 0.50".to_owned())
}

#[test]
fn dragging_c_draws_the_squares_again_and_the_green_ones_always_add_up_to_the_blue_one() -> Result<(), String> {
    let tab = open()?;
    let before = attribute(&tab, "polygon.square-width", "points")?;
    for degrees in [20, 90, 135, 200, 300, 45] {
        drag_to(&tab, degrees)?;
        check_eq(handle(&tab, "aria-valuenow")?, degrees.to_string())?;
        let (width, height, hypotenuse) = areas(&tab)?;
        let radians = f64::from(degrees).to_radians();
        check(
            (width - radians.cos().powi(2)).abs() < 1e-6,
            &format!("the width at {degrees}: {width}"),
        )?;
        check(
            (height - radians.sin().powi(2)).abs() < 1e-6,
            &format!("the height at {degrees}: {height}"),
        )?;
        check(
            (width + height - hypotenuse).abs() < 1e-6,
            &format!("{width} + {height} = {hypotenuse} at {degrees}"),
        )?;
    }
    drag_to(&tab, 120)?;
    check(
        attribute(&tab, "polygon.square-width", "points")? != before,
        "the square has been drawn again",
    )
}

#[test]
fn the_area_of_each_square_is_written_in_it_and_the_sum_is_written_at_the_top() -> Result<(), String> {
    let tab = open()?;
    drag_to(&tab, 90)?;
    check_eq(written(&tab, ".text-width")?, "a\u{b2} = 0.00".to_owned())?;
    check_eq(written(&tab, ".text-height")?, "b\u{b2} = 1.00".to_owned())?;
    check_eq(written(&tab, ".numbers")?, "0.00 + 1.00 = 1.00".to_owned())?;
    drag_to(&tab, 60)?;
    check_eq(written(&tab, ".numbers")?, "0.25 + 0.75 = 1.00".to_owned())
}

#[test]
fn the_keys_move_c_and_the_squares_follow() -> Result<(), String> {
    let tab = open()?;
    focus_handle(&tab)?;
    press(&tab, "Home")?;
    check_eq(handle(&tab, "aria-valuenow")?, "0".to_owned())?;
    check_eq(written(&tab, ".numbers")?, "1.00 + 0.00 = 1.00".to_owned())?;
    let (width, height, hypotenuse) = areas(&tab)?;
    check(
        (width - 1.0).abs() < 1e-6 && height.abs() < 1e-6 && (hypotenuse - 1.0).abs() < 1e-6,
        "the width is the radius",
    )?;
    press(&tab, "PageUp")?;
    press(&tab, "PageUp")?;
    check_eq(handle(&tab, "aria-valuenow")?, "30".to_owned())?;
    check_eq(written(&tab, ".numbers")?, "0.75 + 0.25 = 1.00".to_owned())
}

#[test]
fn the_squares_are_drawn_without_taking_the_pointer_and_the_handle_is_over_them() -> Result<(), String> {
    let tab = open()?;
    check_eq(attribute(&tab, "g", "style")?, "pointer-events: none;".to_owned())?;
    // The handle is the last thing in the scene, so it is over everything that is drawn.
    let last = text(
        &tab,
        &format!("document.querySelector('{SVG}').lastElementChild.getAttribute('class')"),
    )?;
    check_eq(last, "circle-handle".to_owned())
}

#[test]
fn the_scene_says_what_it_is_for_assistive_technology() -> Result<(), String> {
    let tab = open()?;
    let label = text(&tab, &format!("document.querySelector('{SVG}').getAttribute('aria-label')"))?;
    check(label.contains("45 degrees") && label.contains("0.50 + 0.50 = 1.00"), &label)?;
    check_eq(
        text(&tab, &format!("document.querySelector('{SVG}').getAttribute('role')"))?,
        "img".to_owned(),
    )
}
