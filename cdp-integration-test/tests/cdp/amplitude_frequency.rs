//! The amplitude, frequency and phase panel in the built demo gallery: one scene, 800 pixels wide and 400 high, with
//! three sliders drawn in it that set the wave that is plotted below them.
//!
//! Native unit tests cover the wave, the values of the sliders and the keys. What they cannot reach is the browser.
//! These tests check the following:
//!
//! - The scene is 800 by 400 pixels, and has one plot and three sliders with the ranges that were asked for.
//! - The keys move a slider a step, ten steps, and to its ends, and the value beside it and the wave follow.
//! - The pointer takes hold of a slider and puts it where it is, and the wave follows.
//! - A wave of more frequency has more cycles in the same second, and the time axis does not change.
//! - A wave of no amplitude is flat, and has no dotted lines.
//! - The formula is written in the plot, and follows the sliders.
//!
//! Like [`crate::phase_shift`], these run against the built demo gallery.

use crate::common::{DemoTab, drag, new_demo_tab};
use headless_chrome::Tab;

const SVG: &str = "#amplitude-frequency-stage-svg";
const WAVE: &str = "polyline[stroke=\"#2563eb\"]:not([stroke-dasharray])";
const VIEWPORT: (u32, u32) = (1800, 1000);

/// Evaluates to the left and the top of the scene, in pixels, written as `left,top`.
const CORNER: &str = "(() => { \
    const r = document.querySelector('#amplitude-frequency-stage-svg').getBoundingClientRect(); \
    return r.left + ',' + r.top; })()";

/// Where each slider is in the scene: the left end of its track, and the height of its middle. The track is 220 long.
const AMPLITUDE: (f64, f64) = (20.0, 56.0);
const FREQUENCY: (f64, f64) = (290.0, 56.0);
const TRACK: f64 = 220.0;

fn open() -> Result<DemoTab, String> {
    new_demo_tab(
        "index.html#panel-amplitude-frequency",
        &format!("{SVG} [data-slider=\"phase\"]"),
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

fn count(tab: &Tab, selector: &str) -> Result<usize, String> {
    text(tab, &format!("String(document.querySelectorAll('{selector}').length)"))?
        .parse()
        .map_err(|e| format!("{e}"))
}

/// What the slider `id` says about itself. A native slider has its value and its range as properties, and no
/// `aria-valuenow`, so those are read from the element and not from an attribute.
fn slider(tab: &Tab, id: &str, attribute: &str) -> Result<String, String> {
    text(
        tab,
        &format!(
            "(() => {{ const i = document.querySelector('{SVG} input[data-slider=\"{id}\"]'); \
             switch ('{attribute}') {{ \
             case 'aria-valuenow': return i.value; \
             case 'aria-valuemin': return i.min; \
             case 'aria-valuemax': return i.max; \
             case 'disabled': return String(i.disabled); \
             case 'type': return i.type; \
             default: return i.getAttribute('{attribute}'); }} }})()"
        ),
    )
}

fn focus(tab: &Tab, id: &str) -> Result<(), String> {
    text(
        tab,
        &format!("document.querySelector('{SVG} input[data-slider=\"{id}\"]').focus(), 'ok'"),
    )
    .map(|_| ())
}

fn press(tab: &Tab, key: &str) -> Result<(), String> {
    tab.press_key(key).map_err(|e| format!("could not press {key}: {e}"))?;
    Ok(())
}

/// Everything written in the scene, joined with `|`.
fn written(tab: &Tab) -> Result<String, String> {
    text(
        tab,
        &format!("[...document.querySelectorAll('{SVG} text')].map(t => t.textContent).join('|')"),
    )
}

/// The `points` of the wave, as numbers: the heights, from the top of the scene down.
fn heights(tab: &Tab) -> Result<Vec<f64>, String> {
    let points = text(tab, &format!("document.querySelector('{SVG} {WAVE}').getAttribute('points')"))?;
    points
        .split_whitespace()
        .map(|pair| {
            pair.split(',')
                .nth(1)
                .ok_or_else(|| format!("no height in {pair:?}"))?
                .parse::<f64>()
                .map_err(|e| format!("{e}"))
        })
        .collect()
}

/// How many times the wave turns round: its tops and its bottoms that are not at its ends.
fn turns(tab: &Tab) -> Result<usize, String> {
    let y = heights(tab)?;
    let rises: Vec<bool> = y.windows(2).map(|pair| pair[1] > pair[0] + 1e-9).collect();
    let falls: Vec<bool> = y.windows(2).map(|pair| pair[1] < pair[0] - 1e-9).collect();
    let mut direction = None;
    let mut turned = 0;
    for (up, down) in rises.into_iter().zip(falls) {
        let now = if up {
            Some(true)
        } else if down {
            Some(false)
        } else {
            None
        };
        if let (Some(before), Some(after)) = (direction, now) {
            if before != after {
                turned += 1;
            }
        }
        if now.is_some() {
            direction = now;
        }
    }
    Ok(turned)
}

/// Presses the pointer on the track of a slider at `fraction` of the way along it, and lets go.
fn click(tab: &Tab, (left, middle): (f64, f64), fraction: f64) -> Result<(), String> {
    let corner = text(tab, CORNER)?;
    let (x, y) = corner.split_once(',').ok_or("the corner is two numbers")?;
    let (x, y): (f64, f64) = (x.parse().map_err(|e| format!("{e}"))?, y.parse().map_err(|e| format!("{e}"))?);
    let at = (x + left + TRACK * fraction, y + middle);
    drag(tab, &[at, at])
}

#[test]
fn the_scene_is_800_by_400_and_has_one_plot_and_three_sliders() -> Result<(), String> {
    let tab = open()?;
    let size = text(
        &tab,
        &format!(
            "(() => {{ const s = document.querySelector('{SVG}'); \
             return s.getAttribute('width') + 'x' + s.getAttribute('height'); }})()"
        ),
    )?;
    check_eq(size, "800x400".to_owned())?;
    check_eq(count(&tab, &format!("{SVG} g[role=\"img\"]"))?, 1)?;
    check_eq(count(&tab, &format!("{SVG} input[type=\"range\"]"))?, 3)?;
    check_eq(count(&tab, "#panel-amplitude-frequency svg")?, 1)
}

#[test]
fn the_sliders_have_the_ranges_and_the_first_values_that_were_asked_for() -> Result<(), String> {
    let tab = open()?;
    for (id, name, min, max, now, shown) in [
        ("amplitude", "Amplitude", "0", "1", "1", "1.00"),
        ("frequency", "Frequency", "1", "8", "2", "2.0 Hz"),
        ("phase", "Phase", "0", "360", "0", "0\u{b0}"),
    ] {
        check_eq(slider(&tab, id, "aria-label")?, name.to_owned())?;
        check_eq(slider(&tab, id, "aria-valuemin")?, min.to_owned())?;
        check_eq(slider(&tab, id, "aria-valuemax")?, max.to_owned())?;
        check_eq(slider(&tab, id, "aria-valuenow")?, now.to_owned())?;
        check_eq(slider(&tab, id, "aria-valuetext")?, shown.to_owned())?;
        check_eq(slider(&tab, id, "type")?, "range".to_owned())?;
    }
    let all = written(&tab)?;
    check(
        all.contains("2.0 Hz") && all.contains("1.00") && all.contains("Frequency"),
        &format!("{all:?}"),
    )
}

#[test]
fn the_keys_move_the_frequency_by_the_browser_and_to_the_ends() -> Result<(), String> {
    let tab = open()?;
    let value = |tab: &Tab| -> Result<f64, String> {
        slider(tab, "frequency", "aria-valuenow")?.parse().map_err(|e| format!("{e}"))
    };
    focus(&tab, "frequency")?;
    press(&tab, "ArrowRight")?;
    check_eq(slider(&tab, "frequency", "aria-valuenow")?, "2.1".to_owned())?;
    check_eq(slider(&tab, "frequency", "aria-valuetext")?, "2.1 Hz".to_owned())?;
    press(&tab, "PageUp")?;
    check(value(&tab)? > 2.1, "a page key moves it on further than a step")?;
    press(&tab, "ArrowLeft")?;
    press(&tab, "End")?;
    check_eq(slider(&tab, "frequency", "aria-valuenow")?, "8".to_owned())?;
    press(&tab, "ArrowRight")?;
    check_eq(slider(&tab, "frequency", "aria-valuenow")?, "8".to_owned())?;
    press(&tab, "Home")?;
    check_eq(slider(&tab, "frequency", "aria-valuenow")?, "1".to_owned())?;
    press(&tab, "ArrowLeft")?;
    check_eq(slider(&tab, "frequency", "aria-valuenow")?, "1".to_owned())?;
    check(written(&tab)?.contains("1.0 Hz"), "the value beside the slider follows")
}

#[test]
fn a_higher_frequency_has_more_cycles_in_the_same_second_and_the_time_axis_does_not_change() -> Result<(), String> {
    let tab = open()?;
    let axis = written(&tab)?
        .split('|')
        .filter(|label| label.ends_with(" s"))
        .map(str::to_owned)
        .collect::<Vec<_>>();
    check_eq(axis.len(), 10)?;
    check_eq(turns(&tab)?, 4)?;
    focus(&tab, "frequency")?;
    press(&tab, "End")?;
    check_eq(turns(&tab)?, 16)?;
    press(&tab, "Home")?;
    check_eq(turns(&tab)?, 2)?;
    let after = written(&tab)?
        .split('|')
        .filter(|label| label.ends_with(" s") && !label.contains("Hz"))
        .map(str::to_owned)
        .collect::<Vec<_>>();
    check_eq(after, axis)
}

#[test]
fn a_wave_of_no_amplitude_is_flat_and_has_no_dotted_lines() -> Result<(), String> {
    let tab = open()?;
    check_eq(count(&tab, &format!("{SVG} [stroke-dasharray=\"1 5\"]"))?, 2)?;
    focus(&tab, "amplitude")?;
    press(&tab, "Home")?;
    check_eq(slider(&tab, "amplitude", "aria-valuenow")?, "0".to_owned())?;
    let y = heights(&tab)?;
    check(y.iter().all(|height| (height - y[0]).abs() < 1e-9), "the wave is flat")?;
    check_eq(turns(&tab)?, 0)?;
    check_eq(count(&tab, &format!("{SVG} [stroke-dasharray=\"1 5\"]"))?, 0)
}

#[test]
fn the_phase_moves_the_wave_and_the_formula_says_so() -> Result<(), String> {
    let tab = open()?;
    let before = heights(&tab)?;
    focus(&tab, "phase")?;
    for _ in 0..10 {
        press(&tab, "ArrowRight")?;
    }
    check_eq(slider(&tab, "phase", "aria-valuenow")?, "10".to_owned())?;
    check_eq(slider(&tab, "phase", "aria-valuetext")?, "10\u{b0}".to_owned())?;
    check(heights(&tab)? != before, "the wave has moved")?;
    check(written(&tab)?.contains("t + 10\u{b0})"), "the formula has the phase")?;
    press(&tab, "End")?;
    check_eq(slider(&tab, "phase", "aria-valuenow")?, "360".to_owned())
}

#[test]
fn the_pointer_puts_a_slider_where_it_is_pressed_and_the_wave_follows() -> Result<(), String> {
    let tab = open()?;
    click(&tab, AMPLITUDE, 0.5)?;
    check_eq(slider(&tab, "amplitude", "aria-valuenow")?, "0.5".to_owned())?;
    check_eq(slider(&tab, "amplitude", "aria-valuetext")?, "0.50".to_owned())?;
    check(written(&tab)?.contains("y = 0.50 \u{b7}"), "the formula has the amplitude")?;
    click(&tab, FREQUENCY, 1.0)?;
    check_eq(slider(&tab, "frequency", "aria-valuenow")?, "8".to_owned())?;
    check_eq(turns(&tab)?, 16)?;
    click(&tab, FREQUENCY, 0.0)?;
    check_eq(slider(&tab, "frequency", "aria-valuenow")?, "1".to_owned())
}

#[test]
fn the_pointer_can_drag_a_thumb_along_its_track() -> Result<(), String> {
    let tab = open()?;
    let corner = text(&tab, CORNER)?;
    let (x, y) = corner.split_once(',').ok_or("two numbers")?;
    let (x, y): (f64, f64) = (x.parse().map_err(|e| format!("{e}"))?, y.parse().map_err(|e| format!("{e}"))?);
    let at = |fraction: f64| (x + AMPLITUDE.0 + TRACK * fraction, y + AMPLITUDE.1);
    drag(&tab, &[at(1.0), at(0.75), at(0.25), at(0.1)])?;
    check_eq(slider(&tab, "amplitude", "aria-valuenow")?, "0.1".to_owned())
}

#[test]
fn the_formula_is_written_in_the_plot_and_the_first_one_has_the_first_numbers() -> Result<(), String> {
    let tab = open()?;
    check(
        written(&tab)?.contains("y = 1.00 \u{b7} sin(2\u{3c0} \u{b7} 2.0 \u{b7} t + 0\u{b0})"),
        &format!("{:?}", written(&tab)?),
    )
}

#[test]
fn the_scene_needs_no_scroll_bar_in_a_window_that_is_wide_enough() -> Result<(), String> {
    let tab = open()?;
    let sizes = text(
        &tab,
        "(() => { const g = document.querySelector('.amplitude-frequency-wrap'); \
         return [g.scrollWidth, g.clientWidth, g.scrollHeight, g.clientHeight].join(','); })()",
    )?;
    let n: Vec<i64> = sizes
        .split(',')
        .map(|n| n.parse().map_err(|e| format!("{e}")))
        .collect::<Result<_, _>>()?;
    check_eq(n[0], n[1])?;
    check_eq(n[2], n[3])
}
