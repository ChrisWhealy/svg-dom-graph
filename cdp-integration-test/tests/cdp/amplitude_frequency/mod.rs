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

use crate::common::{demo_tab::DemoTab, drag, new_demo_tab};
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

        if let (Some(before), Some(after)) = (direction, now)
            && before != after
        {
            turned += 1;
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

mod unit_tests;
