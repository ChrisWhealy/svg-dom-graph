//! The convolution panel in the built demo gallery: four scenes, with a slider in the first that sets the frequency that
//! a signal of two sine waves is probed with, two products that are shaded above and below the axis, and the spectrum.
//!
//! Native unit tests cover the sums and what is drawn for each frequency, and the library's own tests cover how an area
//! is shaded. What they cannot reach is the browser. These tests check the following:
//!
//! - The four scenes have the sizes that were asked for, each holds one plot, and the slider is in the first.
//! - The slider starts at 3 Hz, where there is none of either wave, takes the keys and the pointer, and the writing in every scene follows it.
//! - The products are shaded in two colours, and the area of the shading in the browser is the area that is written.
//! - The dot in the spectrum is at the amplitude found for the frequency of the slider.
//!
//! Like [`crate::amplitude_frequency`], these run against the built demo gallery.

use crate::common::{demo_tab::DemoTab, drag, new_demo_tab};
use headless_chrome::Tab;

const SIGNAL: &str = "#convolution-stage-svg";
const COSINE: &str = "#convolution-cosine-stage-svg";
const SINE: &str = "#convolution-sine-stage-svg";
const SPECTRUM: &str = "#convolution-spectrum-stage-svg";
const VIEWPORT: (u32, u32) = (1800, 1400);

/// Where the slider is in the first scene: the left end of its track, the height of its middle, and how long it is.
const SLIDER: (f64, f64) = (20.0, 43.0);
const TRACK: f64 = 320.0;

/// The colours that a product is shaded in, above its axis and below it.
const ABOVE: &str = "#2563eb";
const BELOW: &str = "#ea580c";

fn open() -> Result<DemoTab, String> {
    new_demo_tab(
        "index.html#panel-convolution",
        &format!("{SIGNAL} [data-slider=\"probe\"]"),
        VIEWPORT,
    )
}

fn check(condition: bool, message: &str) -> Result<(), String> {
    if condition { Ok(()) } else { Err(message.to_owned()) }
}

fn check_eq<T: PartialEq + std::fmt::Debug>(got: T, expected: T) -> Result<(), String> {
    check(got == expected, &format!("expected {expected:?}, got {got:?}"))
}

fn near(a: f64, b: f64, tolerance: f64) -> bool {
    (a - b).abs() <= tolerance
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

/// A property of the slider, or its attribute if the name has a hyphen in it.
fn slider(tab: &Tab, property: &str) -> Result<String, String> {
    let read = if property.contains('-') {
        format!("i.getAttribute('{property}')")
    } else {
        format!("i.{property}")
    };
    text(
        tab,
        &format!(
            "(() => {{ const i = document.querySelector('{SIGNAL} input[data-slider=\"probe\"]'); return String({read}); }})()"
        ),
    )
}

fn focus(tab: &Tab) -> Result<(), String> {
    text(
        tab,
        &format!("document.querySelector('{SIGNAL} input[data-slider=\"probe\"]').focus(), 'ok'"),
    )
    .map(|_| ())
}

fn press(tab: &Tab, key: &str) -> Result<(), String> {
    tab.press_key(key).map_err(|e| format!("could not press {key}: {e}"))?;
    Ok(())
}

/// Everything written in the plot of `scene`, joined with `|`.
fn written(tab: &Tab, scene: &str) -> Result<String, String> {
    text(
        tab,
        &format!("[...document.querySelectorAll('{scene} text')].map(t => t.textContent).join('|')"),
    )
}

/// The size of `scene`, as `WxH`.
fn size(tab: &Tab, scene: &str) -> Result<String, String> {
    text(
        tab,
        &format!(
            "(() => {{ const s = document.querySelector('{scene}'); \
             return s.getAttribute('width') + 'x' + s.getAttribute('height'); }})()"
        ),
    )
}

/// The corners of every polygon of `scene` that is filled with `fill`, as numbers.
fn polygons(tab: &Tab, scene: &str, fill: &str) -> Result<Vec<Vec<(f64, f64)>>, String> {
    let all = text(
        tab,
        &format!(
            "[...document.querySelectorAll('{scene} polygon[fill=\"{fill}\"]')].map(p => p.getAttribute('points')).join('|')"
        ),
    )?;
    all.split('|')
        .filter(|points| !points.is_empty())
        .map(|points| {
            points
                .split_whitespace()
                .map(|pair| {
                    let (x, y) = pair.split_once(',').ok_or_else(|| format!("no comma in {pair:?}"))?;
                    Ok((x.parse().map_err(|e| format!("{e}"))?, y.parse().map_err(|e| format!("{e}"))?))
                })
                .collect()
        })
        .collect()
}

/// The area of a polygon, by the shoelace formula, in pixels.
fn area(corners: &[(f64, f64)]) -> f64 {
    let n = corners.len();
    (0..n)
        .map(|i| {
            let (a, b) = (corners[i], corners[(i + 1) % n]);
            a.0 * b.1 - b.0 * a.1
        })
        .sum::<f64>()
        .abs()
        / 2.0
}

/// The number written before `after` in the writing of `scene`, for example the `0.43` in `+0.43 above`.
fn figure(tab: &Tab, scene: &str, after: &str) -> Result<f64, String> {
    let all = written(tab, scene)?;
    let item = all
        .split('|')
        .find(|t| t.ends_with(after))
        .ok_or_else(|| format!("nothing ends with {after:?} in {all:?}"))?;
    item.trim_end_matches(after)
        .trim()
        .replace('\u{2212}', "-")
        .parse()
        .map_err(|e| format!("{e} in {item:?}"))
}

/// Presses the pointer on the track of the slider at `fraction` of the way along it, and lets go.
fn click(tab: &Tab, fraction: f64) -> Result<(), String> {
    let corner = text(
        tab,
        &format!(
            "(() => {{ const r = document.querySelector('{SIGNAL}').getBoundingClientRect(); \
             return r.left + ',' + r.top; }})()"
        ),
    )?;
    let (x, y) = corner.split_once(',').ok_or("the corner is two numbers")?;
    let (x, y): (f64, f64) = (x.parse().map_err(|e| format!("{e}"))?, y.parse().map_err(|e| format!("{e}"))?);
    let at = (x + SLIDER.0 + TRACK * fraction, y + SLIDER.1);
    drag(tab, &[at, at])
}

/// Moves the slider by `steps` tenths of a hertz, which is to the right if it is above 0.
fn step(tab: &Tab, steps: i32) -> Result<(), String> {
    focus(tab)?;
    for _ in 0..steps.abs() {
        press(tab, if steps > 0 { "ArrowRight" } else { "ArrowLeft" })?;
    }
    Ok(())
}

/// Makes every write of the attribute `name` fail with an exception, as the browser can when a page is under pressure,
/// or puts the browser's own write back.
fn fail_writes_of(tab: &Tab, name: &str, failing: bool) -> Result<(), String> {
    let script = if failing {
        format!(
            "(() => {{ const p = Element.prototype; if (!p.__written) {{ p.__written = p.setAttribute; }} \
             p.setAttribute = function (n, v) {{ if (n === '{name}') {{ throw new Error('injected'); }} \
             return p.__written.apply(this, arguments); }}; return 'ok'; }})()"
        )
    } else {
        "(() => { const p = Element.prototype; if (p.__written) { p.setAttribute = p.__written; } return 'ok'; })()"
            .to_owned()
    };
    text(tab, &script).map(|_| ())
}

/// Whether the error banner of the page is hidden.
fn banner_hidden(tab: &Tab) -> Result<bool, String> {
    Ok(text(tab, "String(document.querySelector('#convolution-error').hidden)")? == "true")
}

mod unit_tests;
