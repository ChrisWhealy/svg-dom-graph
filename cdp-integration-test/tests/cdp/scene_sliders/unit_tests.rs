//! The tests of [`super`], which holds the helpers they use.

use super::*;

#[test]
fn the_fixing_points_slider_is_in_the_scene_and_the_slider_of_the_page_is_gone() -> Result<(), String> {
    let tab = open("panel-edge-anchors", &FIXING_POINTS)?;
    check_eq(attribute(&tab, &FIXING_POINTS, "type")?, "range".to_owned())?;
    check_eq(attribute(&tab, &FIXING_POINTS, "aria-label")?, "Fixing points".to_owned())?;
    check_eq(attribute(&tab, &FIXING_POINTS, "aria-valuemin")?, "0".to_owned())?;
    check_eq(attribute(&tab, &FIXING_POINTS, "aria-valuemax")?, "5".to_owned())?;
    check_eq(attribute(&tab, &FIXING_POINTS, "aria-valuenow")?, "0".to_owned())?;
    check(!exists(&tab, "#edge-anchors-fixing-points")?, "the slider of the page is gone")?;
    check(
        !exists(&tab, "#edge-anchors-fixing-points-value")?,
        "and so is the number beside it",
    )
}

#[test]
fn the_keys_move_the_fixing_points_slider_and_the_scene_follows_with_the_focus_kept() -> Result<(), String> {
    let tab = open("panel-edge-anchors", &FIXING_POINTS)?;
    check_eq(children(&tab)?, 1)?;
    focus(&tab, &FIXING_POINTS)?;
    for expected in [1, 2, 3] {
        press(&tab, "ArrowRight")?;
        check_eq(attribute(&tab, &FIXING_POINTS, "aria-valuenow")?, expected.to_string())?;
        check_eq(children(&tab)?, expected)?;
    }
    press(&tab, "End")?;
    check_eq(children(&tab)?, 5)?;
    press(&tab, "ArrowRight")?;
    check_eq(attribute(&tab, &FIXING_POINTS, "aria-valuenow")?, "5".to_owned())?;
    press(&tab, "Home")?;
    check_eq(attribute(&tab, &FIXING_POINTS, "aria-valuenow")?, "0".to_owned())?;
    check_eq(children(&tab)?, 1)
}

#[test]
fn the_pointer_moves_the_fixing_points_slider_and_a_drag_carries_on_across_the_scenes_that_are_drawn_again()
-> Result<(), String> {
    let tab = open("panel-edge-anchors", &FIXING_POINTS)?;
    click(&tab, &FIXING_POINTS, 1.0)?;
    check_eq(attribute(&tab, &FIXING_POINTS, "aria-valuenow")?, "5".to_owned())?;
    check_eq(children(&tab)?, 5)?;
    let path: Vec<(f64, f64)> = [1.0, 0.8, 0.6, 0.4, 0.2, 0.0]
        .into_iter()
        .map(|fraction| along(&tab, &FIXING_POINTS, fraction))
        .collect::<Result<_, _>>()?;
    drag(&tab, &path)?;
    check_eq(attribute(&tab, &FIXING_POINTS, "aria-valuenow")?, "0".to_owned())?;
    check_eq(children(&tab)?, 1)
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[test]
fn the_clock_slider_is_in_the_scene_and_the_slider_of_the_page_is_gone() -> Result<(), String> {
    let tab = open("panel-ellipse", &CLOCK)?;
    check_eq(attribute(&tab, &CLOCK, "type")?, "range".to_owned())?;
    check_eq(attribute(&tab, &CLOCK, "aria-label")?, "Clock position".to_owned())?;
    check_eq(attribute(&tab, &CLOCK, "aria-valuemin")?, "0".to_owned())?;
    check_eq(attribute(&tab, &CLOCK, "aria-valuemax")?, "59".to_owned())?;
    check_eq(attribute(&tab, &CLOCK, "aria-valuenow")?, "10".to_owned())?;
    check(
        written(&tab, &CLOCK)?.contains("10 \u{b7} 60\u{b0} clockwise from north"),
        "the minute is written",
    )?;
    check(!exists(&tab, "#ellipse-minute")?, "the slider of the page is gone")?;
    check(!exists(&tab, "#ellipse-minute-bearing")?, "and so is the text beside it")
}

#[test]
fn the_keys_move_the_clock_and_the_circle_at_the_end_of_its_connector_is_named_for_the_minute() -> Result<(), String> {
    let tab = open("panel-ellipse", &CLOCK)?;
    focus(&tab, &CLOCK)?;
    press(&tab, "ArrowRight")?;
    check_eq(attribute(&tab, &CLOCK, "aria-valuenow")?, "11".to_owned())?;
    check(
        written(&tab, &CLOCK)?.split('|').any(|label| label == "11"),
        "the circle says 11",
    )?;
    press(&tab, "ArrowRight")?;
    check_eq(attribute(&tab, &CLOCK, "aria-valuenow")?, "12".to_owned())?;
    press(&tab, "Home")?;
    check(
        written(&tab, &CLOCK)?.contains("0 \u{b7} 0\u{b0} clockwise from north (north)"),
        &written(&tab, &CLOCK)?,
    )?;
    press(&tab, "End")?;
    check_eq(attribute(&tab, &CLOCK, "aria-valuenow")?, "59".to_owned())?;
    press(&tab, "PageDown")?;
    let after: f64 = attribute(&tab, &CLOCK, "aria-valuenow")?.parse().map_err(|e| format!("{e}"))?;
    check(after < 59.0, "a page key moves it further than a step")
}

#[test]
fn the_pointer_puts_the_clock_where_it_is_pressed() -> Result<(), String> {
    let tab = open("panel-ellipse", &CLOCK)?;
    // The minute 30 is 30 of the 59 steps along the slider.
    click(&tab, &CLOCK, 30.0 / 59.0)?;
    check_eq(attribute(&tab, &CLOCK, "aria-valuenow")?, "30".to_owned())?;
    check(written(&tab, &CLOCK)?.contains("(south)"), "30 is south")?;
    let path: Vec<(f64, f64)> = [0.5, 0.6, 0.8, 1.0]
        .into_iter()
        .map(|fraction| along(&tab, &CLOCK, fraction))
        .collect::<Result<_, _>>()?;
    drag(&tab, &path)?;
    check_eq(attribute(&tab, &CLOCK, "aria-valuenow")?, "59".to_owned())
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[test]
fn the_corner_radius_slider_is_in_the_scene_and_the_slider_of_the_page_is_gone() -> Result<(), String> {
    let tab = open("panel-elbow", &RADIUS)?;
    check_eq(attribute(&tab, &RADIUS, "type")?, "range".to_owned())?;
    check_eq(attribute(&tab, &RADIUS, "aria-label")?, "Corner radius".to_owned())?;
    check_eq(attribute(&tab, &RADIUS, "aria-valuemin")?, "0".to_owned())?;
    check_eq(attribute(&tab, &RADIUS, "aria-valuenow")?, "0".to_owned())?;
    check(!exists(&tab, "#corner-radius")?, "the slider of the page is gone")?;
    check(!exists(&tab, "#corner-radius-value")?, "and so is the number beside it")
}

#[test]
fn the_greatest_corner_radius_is_the_room_that_the_boxes_leave_and_not_the_first_guess() -> Result<(), String> {
    let tab = open("panel-elbow", &RADIUS)?;
    let greatest: f64 = attribute(&tab, &RADIUS, "aria-valuemax")?.parse().map_err(|e| format!("{e}"))?;

    check(
        greatest > 0.0 && greatest < 80.0,
        &format!("the room of the boxes at the start, {greatest}"),
    )?;
    focus(&tab, &RADIUS)?;
    press(&tab, "End")?;
    check_eq(attribute(&tab, &RADIUS, "aria-valuenow")?, greatest.to_string())?;
    press(&tab, "ArrowRight")?;
    check_eq(attribute(&tab, &RADIUS, "aria-valuenow")?, greatest.to_string())?;
    press(&tab, "Home")?;
    check_eq(attribute(&tab, &RADIUS, "aria-valuenow")?, "0".to_owned())
}

#[test]
fn the_pointer_moves_the_corner_radius_slider_and_the_connector_is_rounded() -> Result<(), String> {
    let tab = open("panel-elbow", &RADIUS)?;
    let greatest: f64 = attribute(&tab, &RADIUS, "aria-valuemax")?.parse().map_err(|e| format!("{e}"))?;
    let connector = "document.querySelector('#elbow-diagram > g.svg-dom-graph-content > path').getAttribute('d')";
    check(!text(&tab, connector)?.contains(" A "), "the corners are sharp at first")?;
    click(&tab, &RADIUS, 1.0)?;
    check_eq(attribute(&tab, &RADIUS, "aria-valuenow")?, greatest.to_string())?;
    check(text(&tab, connector)?.contains(" A "), "and rounded when the radius is raised")
}

#[test]
fn the_corner_radius_slider_is_dimmed_for_a_straight_connector_and_answers_nothing() -> Result<(), String> {
    let tab = open("panel-elbow", &RADIUS)?;
    focus(&tab, &RADIUS)?;
    press(&tab, "ArrowRight")?;
    check_eq(attribute(&tab, &RADIUS, "aria-valuenow")?, "1".to_owned())?;
    text(&tab, "document.getElementById('connector-type-straight').click(), 'ok'")?;
    check_eq(attribute(&tab, &RADIUS, "disabled")?, "true".to_owned())?;
    click(&tab, &RADIUS, 1.0)?;
    check_eq(attribute(&tab, &RADIUS, "aria-valuenow")?, "1".to_owned())?;
    text(&tab, "document.getElementById('connector-type-elbow').click(), 'ok'")?;
    check_eq(attribute(&tab, &RADIUS, "disabled")?, "false".to_owned())
}
