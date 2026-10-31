//! The sliders that are drawn in the scenes of the connector routing, fixing points and ellipses panels, in the built
//! demo gallery.
//!
//! Those panels had a slider of the page, outside the scene. Each now has one in the scene, which is the browser's own
//! range slider in a `foreignObject`, as the amplitude, frequency and phase panel has. Native unit tests cover where a
//! slider goes. What they cannot reach is the browser, and the panels that draw their scene again for every move of the
//! slider, which the slider has to carry on through. These tests check the following:
//!
//! - The slider is in the scene, is a range slider with the range that it should have, and the slider of the page is
//!   gone.
//! - The keys move it, and move it again after the scene has been drawn again, since the slider is the same one and so
//!   keeps the focus.
//! - The pointer takes hold of it, and a drag carries on across the scenes that are drawn again.
//! - What the slider is for follows it: the children, the clock, and the corner radius.
//! - The slider of the corner radius is dimmed for a straight connector, and its greatest value is the room that there
//!   is.
//!
//! Like [`crate::amplitude_frequency`], these run against the built demo gallery.

use crate::common::{DemoTab, drag, new_demo_tab};
use headless_chrome::Tab;

const VIEWPORT: (u32, u32) = (1800, 1200);

/// A slider of a scene: where its svg is, and where its track is in the scene.
struct Where {
    svg: &'static str,
    id: &'static str,
    left: f64,
    middle: f64,
    width: f64,
}

const FIXING_POINTS: Where = Where {
    svg: "#edge-anchors-diagram",
    id: "fixing-points",
    left: 20.0,
    middle: 292.0,
    width: 260.0,
};
const CLOCK: Where = Where {
    svg: "#ellipse-diagram",
    id: "clock-position",
    left: 20.0,
    middle: 492.0,
    width: 440.0,
};
const RADIUS: Where = Where {
    svg: "#elbow-diagram",
    id: "corner-radius",
    left: 20.0,
    middle: 332.0,
    width: 300.0,
};

fn open(panel: &str, slider: &Where) -> Result<DemoTab, String> {
    new_demo_tab(
        &format!("index.html#{panel}"),
        &format!("{} [data-slider=\"{}\"]", slider.svg, slider.id),
        VIEWPORT,
    )
}

fn check(condition: bool, message: &str) -> Result<(), String> {
    if condition { Ok(()) } else { Err(message.to_owned()) }
}

fn check_eq<T: PartialEq + std::fmt::Debug>(got: T, expected: T) -> Result<(), String> {
    check(got == expected, &format!("expected {expected:?}, got {got:?}"))
}

/// Evaluates `expression` in the page and returns its string value.
fn text(tab: &Tab, expression: &str) -> Result<String, String> {
    tab.evaluate(expression, false)
        .map_err(|e| format!("could not evaluate {expression}: {e}"))?
        .value
        .and_then(|v| v.as_str().map(str::to_owned))
        .ok_or_else(|| format!("{expression} did not return a string"))
}

/// What the slider says about itself. A native slider has its value and its range as properties, and no
/// `aria-valuenow`, so those are read from the element and not from an attribute.
fn attribute(tab: &Tab, slider: &Where, name: &str) -> Result<String, String> {
    text(
        tab,
        &format!(
            "(() => {{ const i = document.querySelector('{} input[data-slider=\"{}\"]'); \
             switch ('{name}') {{ \
             case 'aria-valuenow': return i.value; \
             case 'aria-valuemin': return i.min; \
             case 'aria-valuemax': return i.max; \
             case 'disabled': return String(i.disabled); \
             case 'type': return i.type; \
             default: return i.getAttribute('{name}'); }} }})()",
            slider.svg, slider.id
        ),
    )
}

fn focus(tab: &Tab, slider: &Where) -> Result<(), String> {
    text(
        tab,
        &format!(
            "document.querySelector('{} input[data-slider=\"{}\"]').focus(), 'ok'",
            slider.svg, slider.id
        ),
    )
    .map(|_| ())
}

fn press(tab: &Tab, key: &str) -> Result<(), String> {
    tab.press_key(key).map_err(|e| format!("could not press {key}: {e}"))?;
    Ok(())
}

/// Everything written in the scene, joined with `|`.
fn written(tab: &Tab, slider: &Where) -> Result<String, String> {
    text(
        tab,
        &format!(
            "[...document.querySelectorAll('{} text')].map(t => t.textContent).join('|')",
            slider.svg
        ),
    )
}

fn exists(tab: &Tab, selector: &str) -> Result<bool, String> {
    Ok(text(tab, &format!("String(document.querySelector('{selector}') !== null)"))? == "true")
}

/// The page position of the point at `fraction` of the way along the track of `slider`.
fn along(tab: &Tab, slider: &Where, fraction: f64) -> Result<(f64, f64), String> {
    let corner = text(
        tab,
        &format!(
            "(() => {{ const r = document.querySelector('{}').getBoundingClientRect(); \
             return r.left + ',' + r.top; }})()",
            slider.svg
        ),
    )?;
    let (x, y) = corner.split_once(',').ok_or("the corner is two numbers")?;
    let (x, y): (f64, f64) = (x.parse().map_err(|e| format!("{e}"))?, y.parse().map_err(|e| format!("{e}"))?);
    Ok((x + slider.left + slider.width * fraction, y + slider.middle))
}

fn click(tab: &Tab, slider: &Where, fraction: f64) -> Result<(), String> {
    let at = along(tab, slider, fraction)?;
    drag(tab, &[at, at])
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// How many children the fixing points scene shows: the labels that start with `Child`.
fn children(tab: &Tab) -> Result<usize, String> {
    Ok(written(tab, &FIXING_POINTS)?
        .split('|')
        .filter(|label| label.starts_with("Child"))
        .count())
}

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
