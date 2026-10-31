//! The sine and cosine panel in the built demo gallery: three scenes of the sizes that were asked for, a point that
//! goes round by itself, the button in the circle that stops it, and dragging the point round the circle.
//!
//! Native unit tests cover what is drawn for an angle, and that the plots share their scales. What they cannot reach is
//! the browser. These tests check the following:
//!
//! - The circle is 400 by 400 pixels, the sine 430 by 400 and the cosine 400 by 430, and each holds a plot.
//! - The labels of the circle stop at 1, the radius.
//! - Dragging the point puts it at an angle, and the numbers beside it follow. A drag that starts away from the point
//!   changes nothing.
//! - Point C is a slider in the tab order, over C itself, and the arrow, page, Home and End keys move it.
//! - The readout is three lines, which the page shows as three lines.
//! - The angle is never written as a negative zero, however C is taken across 0°.
//! - The point starts at 0° and goes round from there.
//! - The sine and the cosine each have a title in the plot.
//! - Each plot has its dotted line to an axis.
//! - The point goes round by itself, and the button in the circle stops it and starts it again.
//!
//! Like [`crate::fft`], these run against the built demo gallery.

use crate::common::{DemoTab, drag, new_demo_tab};
use headless_chrome::Tab;
use std::time::{Duration, Instant};

const CIRCLE: &str = "#sine-cosine-circle-stage-svg";
const SINE: &str = "#sine-cosine-sine-stage-svg";
const COSINE: &str = "#sine-cosine-cosine-stage-svg";
const VIEWPORT: (u32, u32) = (1800, 1800);

fn open() -> Result<DemoTab, String> {
    new_demo_tab("index.html#panel-sine-cosine", &format!("{COSINE} g[role=\"img\"]"), VIEWPORT)
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

fn readout(tab: &Tab) -> Result<String, String> {
    text(tab, "document.getElementById('sine-cosine-readout').textContent")
}

/// What the button in the circle says it will do next.
fn button(tab: &Tab) -> Result<String, String> {
    text(
        tab,
        "document.querySelector('#sine-cosine-circle-stage [data-action=\"play\"]').getAttribute('aria-label')",
    )
}

/// How many dotted lines `svg` has: the lines with a dash pattern, which the plot grid does not use.
fn dotted(tab: &Tab, svg: &str) -> Result<String, String> {
    text(
        tab,
        &format!("String(document.querySelectorAll('{svg} [stroke-dasharray=\"1 5\"]').length)"),
    )
}

/// Waits until `read` gives something other than `before`, for up to three seconds.
fn changes(tab: &Tab, before: &str, read: fn(&Tab) -> Result<String, String>) -> Result<bool, String> {
    let start = Instant::now();
    while start.elapsed() < Duration::from_secs(3) {
        if read(tab)? != before {
            return Ok(true);
        }
        std::thread::sleep(Duration::from_millis(50));
    }
    Ok(false)
}

fn press_play(tab: &Tab) -> Result<(), String> {
    text(
        tab,
        "document.querySelector('#sine-cosine-circle-stage [data-action=\"play\"]') \
         .dispatchEvent(new MouseEvent('click', { bubbles: true })), 'ok'",
    )
    .map(|_| ())
}

/// Where the circle is on the page: the left and top of its scene, in pixels.
fn circle_corner(tab: &Tab) -> Result<(f64, f64), String> {
    let corner = text(
        tab,
        &format!(
            "(() => {{ const r = document.querySelector('{CIRCLE}').getBoundingClientRect(); \
             return r.left + ',' + r.top; }})()"
        ),
    )?;
    let (left, top) = corner.split_once(',').ok_or("the corner is two numbers")?;
    Ok((
        left.parse().map_err(|e| format!("{e}"))?,
        top.parse().map_err(|e| format!("{e}"))?,
    ))
}

/// Where the point is on the page at `degrees`: the circle has a radius of 1 and the scene shows 2.4 across.
fn point_at(tab: &Tab, degrees: f64) -> Result<(f64, f64), String> {
    let (left, top) = circle_corner(tab)?;
    let radius = 400.0 / 2.4;
    let angle = degrees.to_radians();
    Ok((left + 200.0 + radius * angle.cos(), top + 200.0 - radius * angle.sin()))
}

/// The angle that the readout gives.
fn shown_angle(tab: &Tab) -> Result<f64, String> {
    let line = readout(tab)?;
    line.strip_prefix("Angle ")
        .and_then(|rest| rest.split('\u{b0}').next())
        .and_then(|number| number.parse().ok())
        .ok_or_else(|| format!("no angle in {line:?}"))
}

/// Stops the point, if it is going round, and drags it from where it is to `degrees`, by way of the angles between.
fn drag_to(tab: &Tab, degrees: u32) -> Result<(), String> {
    if button(tab)? == "Pause" {
        press_play(tab)?;
    }
    let from = shown_angle(tab)?;
    let to = f64::from(degrees);
    let steps = 8;
    let waypoints = (0..=steps)
        .map(|n| point_at(tab, from + (to - from) * f64::from(n) / f64::from(steps)))
        .collect::<Result<Vec<_>, _>>()?;
    drag(tab, &waypoints)
}

#[test]
fn the_three_scenes_have_the_sizes_that_were_asked_for() -> Result<(), String> {
    let tab = open()?;
    for (svg, size) in [(CIRCLE, "400x400"), (SINE, "430x400"), (COSINE, "400x430")] {
        let got = text(
            &tab,
            &format!(
                "(() => {{ const s = document.querySelector('{svg}'); \
                 return s.getAttribute('width') + 'x' + s.getAttribute('height'); }})()"
            ),
        )?;
        check_eq(got, size.to_owned())?;
        check(
            text(
                &tab,
                &format!("String(document.querySelectorAll('{svg} g[role=\"img\"]').length)"),
            )? == "1",
            &format!("{svg} holds one plot"),
        )?;
    }
    Ok(())
}

#[test]
fn the_labels_of_the_circle_stop_at_1() -> Result<(), String> {
    let tab = open()?;
    let labels = text(
        &tab,
        &format!("[...document.querySelectorAll('{CIRCLE} text')].map(t => t.textContent).join(' ')"),
    )?;
    check(!labels.contains("1.2"), &format!("no label at the margin: {labels:?}"))?;
    check(
        labels.split(' ').any(|label| label == "1"),
        &format!("a label at 1: {labels:?}"),
    )
}

#[test]
fn dragging_the_point_puts_it_at_an_angle_and_the_numbers_follow() -> Result<(), String> {
    let tab = open()?;
    drag_to(&tab, 90)?;
    check_eq(
        readout(&tab)?,
        "Angle 90\u{b0}\nWidth  W = cos = 0.00\nHeight H = sin = 1.00".to_owned(),
    )?;
    drag_to(&tab, 180)?;
    check_eq(
        readout(&tab)?,
        "Angle 180\u{b0}\nWidth  W = cos = -1.00\nHeight H = sin = 0.00".to_owned(),
    )?;
    drag_to(&tab, 100)?;
    check_eq(
        readout(&tab)?,
        "Angle 100\u{b0}\nWidth  W = cos = -0.17\nHeight H = sin = 0.98".to_owned(),
    )
}

#[test]
fn the_point_can_be_dragged_backwards_as_well_as_forwards() -> Result<(), String> {
    let tab = open()?;
    drag_to(&tab, 200)?;
    check_eq(shown_angle(&tab)?, 200.0)?;
    drag_to(&tab, 20)?;
    check_eq(shown_angle(&tab)?, 20.0)
}

#[test]
fn holding_the_point_stops_it_and_the_button_says_play() -> Result<(), String> {
    let tab = open()?;
    drag_to(&tab, 120)?;
    check_eq(button(&tab)?, "Play".to_owned())?;
    std::thread::sleep(Duration::from_millis(500));
    check_eq(shown_angle(&tab)?, 120.0)
}

#[test]
fn a_drag_that_starts_away_from_the_point_changes_nothing() -> Result<(), String> {
    let tab = open()?;
    drag_to(&tab, 90)?;
    let (left, top) = circle_corner(&tab)?;
    // The point is at the top of the circle. This starts in the lower left, well away from it.
    drag(
        &tab,
        &[
            (left + 60.0, top + 300.0),
            (left + 90.0, top + 320.0),
            (left + 120.0, top + 340.0),
        ],
    )?;
    check_eq(shown_angle(&tab)?, 90.0)
}

#[test]
fn each_plot_has_a_dotted_line_to_an_axis_and_the_circle_has_two() -> Result<(), String> {
    let tab = open()?;
    drag_to(&tab, 60)?;
    check_eq(dotted(&tab, CIRCLE)?, "2".to_owned())?;
    check_eq(dotted(&tab, SINE)?, "1".to_owned())?;
    check_eq(dotted(&tab, COSINE)?, "1".to_owned())
}

#[test]
fn the_point_goes_round_by_itself_and_the_button_stops_it_and_starts_it_again() -> Result<(), String> {
    let tab = open()?;
    check_eq(button(&tab)?, "Pause".to_owned())?;
    let start = readout(&tab)?;
    check(changes(&tab, &start, readout)?, "the point moves without being touched")?;

    press_play(&tab)?;
    check_eq(button(&tab)?, "Play".to_owned())?;
    let stopped = readout(&tab)?;
    std::thread::sleep(Duration::from_millis(600));
    check_eq(readout(&tab)?, stopped.clone())?;

    press_play(&tab)?;
    check_eq(button(&tab)?, "Pause".to_owned())?;
    check(changes(&tab, &stopped, readout)?, "the point moves again")
}

const HANDLE: &str = "#sine-cosine-circle-stage .sine-cosine-handle";

fn focus_handle(tab: &Tab) -> Result<(), String> {
    text(tab, &format!("document.querySelector('{HANDLE}').focus(), 'ok'")).map(|_| ())
}

fn handle_attribute(tab: &Tab, name: &str) -> Result<String, String> {
    text(tab, &format!("document.querySelector('{HANDLE}').getAttribute('{name}')"))
}

fn press(tab: &Tab, key: &str) -> Result<(), String> {
    tab.press_key(key).map_err(|e| format!("could not press {key}: {e}"))?;
    Ok(())
}

#[test]
fn point_c_is_a_slider_that_the_tab_key_reaches() -> Result<(), String> {
    let tab = open()?;
    check_eq(handle_attribute(&tab, "role")?, "slider".to_owned())?;
    check_eq(handle_attribute(&tab, "tabindex")?, "0".to_owned())?;
    check_eq(handle_attribute(&tab, "aria-valuemin")?, "0".to_owned())?;
    check_eq(handle_attribute(&tab, "aria-valuemax")?, "359".to_owned())?;
    drag_to(&tab, 90)?;
    check_eq(handle_attribute(&tab, "aria-valuenow")?, "90".to_owned())?;
    check_eq(handle_attribute(&tab, "aria-valuetext")?, "90 degrees".to_owned())
}

#[test]
fn the_handle_is_over_point_c_wherever_it_is() -> Result<(), String> {
    let tab = open()?;
    for degrees in [0, 90, 215] {
        drag_to(&tab, degrees)?;
        let (x, y) = point_at(&tab, f64::from(degrees))?;
        let (left, top) = circle_corner(&tab)?;
        let transform = handle_attribute(&tab, "transform")?;
        let numbers: Vec<f64> = transform
            .trim_start_matches("translate(")
            .trim_end_matches(')')
            .split(' ')
            .map(|n| n.parse().map_err(|e| format!("{transform:?}: {e}")))
            .collect::<Result<_, _>>()?;
        check(
            (left + numbers[0] - x).abs() < 0.5 && (top + numbers[1] - y).abs() < 0.5,
            &format!("at {degrees}: {transform}"),
        )?;
    }
    Ok(())
}

#[test]
fn the_arrow_keys_move_c_a_degree_and_the_page_keys_fifteen() -> Result<(), String> {
    let tab = open()?;
    drag_to(&tab, 90)?;
    focus_handle(&tab)?;
    press(&tab, "ArrowRight")?;
    check_eq(shown_angle(&tab)?, 91.0)?;
    press(&tab, "ArrowLeft")?;
    press(&tab, "ArrowLeft")?;
    check_eq(shown_angle(&tab)?, 89.0)?;
    press(&tab, "PageUp")?;
    check_eq(shown_angle(&tab)?, 104.0)?;
    press(&tab, "PageDown")?;
    check_eq(shown_angle(&tab)?, 89.0)?;
    check_eq(handle_attribute(&tab, "aria-valuenow")?, "89".to_owned())
}

#[test]
fn home_and_end_go_to_0_and_359_and_the_angle_wraps() -> Result<(), String> {
    let tab = open()?;
    drag_to(&tab, 100)?;
    focus_handle(&tab)?;
    press(&tab, "Home")?;
    check_eq(shown_angle(&tab)?, 0.0)?;
    press(&tab, "ArrowLeft")?;
    check_eq(shown_angle(&tab)?, 359.0)?;
    press(&tab, "ArrowRight")?;
    check_eq(shown_angle(&tab)?, 0.0)?;
    press(&tab, "End")?;
    check_eq(shown_angle(&tab)?, 359.0)
}

#[test]
fn a_key_stops_c_going_round_by_itself() -> Result<(), String> {
    let tab = open()?;
    check_eq(button(&tab)?, "Pause".to_owned())?;
    focus_handle(&tab)?;
    press(&tab, "ArrowRight")?;
    check_eq(button(&tab)?, "Play".to_owned())?;
    let stopped = readout(&tab)?;
    std::thread::sleep(Duration::from_millis(500));
    check_eq(readout(&tab)?, stopped)
}

#[test]
fn the_sine_and_the_cosine_have_titles() -> Result<(), String> {
    let tab = open()?;
    for (svg, words) in [(SINE, ["Sine:", "height H"]), (COSINE, ["Cosine:", "width W"])] {
        let labels = text(
            &tab,
            &format!("[...document.querySelectorAll('{svg} text')].map(t => t.textContent).join('|')"),
        )?;
        for word in words {
            check(labels.contains(word), &format!("{svg} has {word:?} in {labels:?}"))?;
        }
    }
    Ok(())
}

#[test]
fn the_angle_is_never_written_as_a_negative_zero() -> Result<(), String> {
    let tab = open()?;
    drag_to(&tab, 10)?;
    // Slightly below 0° is a negative angle that rounds to a negative zero.
    let way: Vec<(f64, f64)> = [10.0, 5.0, 2.0, -0.3]
        .into_iter()
        .map(|degrees| point_at(&tab, degrees))
        .collect::<Result<_, _>>()?;
    drag(&tab, &way)?;
    let line = readout(&tab)?;
    check(line.starts_with("Angle 0\u{b0}\n"), &format!("0 and not -0: {line:?}"))?;
    check_eq(handle_attribute(&tab, "aria-valuenow")?, "0".to_owned())?;
    check_eq(handle_attribute(&tab, "aria-valuetext")?, "0 degrees".to_owned())
}

#[test]
fn the_point_starts_at_0_degrees() -> Result<(), String> {
    // It goes round at 30 degrees a second, so by the time the page can be asked it is still only a few degrees on.
    let tab = open()?;
    let angle = shown_angle(&tab)?;
    check(
        angle <= 15.0,
        &format!("the point starts at 0 degrees, and it is at {angle} a moment later"),
    )
}

#[test]
fn the_readout_is_shown_as_three_lines() -> Result<(), String> {
    let tab = open()?;
    drag_to(&tab, 60)?;
    check_eq(readout(&tab)?.lines().count(), 3)?;
    // `innerText` follows what is drawn, so it only has the line breaks if the page honours the ones in the text.
    let drawn = text(&tab, "document.getElementById('sine-cosine-readout').innerText")?;
    check_eq(drawn.lines().count(), 3)?;
    check(!drawn.contains("<br>"), &format!("no markup is shown: {drawn:?}"))
}
