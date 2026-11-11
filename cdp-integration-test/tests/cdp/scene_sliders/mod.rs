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

mod slider_where;

use crate::common::{demo_tab::DemoTab, drag, new_demo_tab};
use crate::scene_sliders::slider_where::Where;
use headless_chrome::Tab;

const VIEWPORT: (u32, u32) = (1800, 1200);

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

mod unit_tests;
