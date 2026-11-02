//! The convolution panel in the built demo gallery: four scenes, with a slider in the first that sets the frequency that
//! a signal of two sine waves is probed with, two products that are shaded above and below the axis, and the spectrum.
//!
//! Native unit tests cover the sums and what is drawn for each frequency, and the library's own tests cover how an area
//! is shaded. What they cannot reach is the browser. These tests check the following:
//!
//! - The four scenes have the sizes that were asked for, each holds one plot, and the slider is in the first.
//! - The slider starts at 2 Hz, takes the keys and the pointer, and the writing in every scene follows it.
//! - The products are shaded in two colours, and the area of the shading in the browser is the area that is written.
//! - The dot in the spectrum is at the amplitude found for the frequency of the slider.
//!
//! Like [`crate::amplitude_frequency`], these run against the built demo gallery.

use crate::common::{DemoTab, drag, new_demo_tab};
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

#[test]
fn the_four_scenes_have_their_sizes_and_one_plot_each_and_the_slider_is_in_the_first() -> Result<(), String> {
    let tab = open()?;
    check_eq(size(&tab, SIGNAL)?, "800x330".to_owned())?;
    check_eq(size(&tab, COSINE)?, "400x250".to_owned())?;
    check_eq(size(&tab, SINE)?, "400x250".to_owned())?;
    check_eq(size(&tab, SPECTRUM)?, "800x240".to_owned())?;
    for scene in [SIGNAL, COSINE, SINE, SPECTRUM] {
        check(
            count(&tab, &format!("{scene} polyline"))? > 0,
            &format!("{scene} has a plot with a line in it"),
        )?;
    }
    check_eq(count(&tab, "#panel-convolution input[data-slider]")?, 1)?;
    check_eq(count(&tab, &format!("{SIGNAL} input[data-slider]"))?, 1)
}

#[test]
fn the_slider_goes_from_0_to_10_hz_in_tenths_and_starts_at_2_hz() -> Result<(), String> {
    let tab = open()?;
    check_eq(slider(&tab, "min")?, "0".to_owned())?;
    check_eq(slider(&tab, "max")?, "10".to_owned())?;
    check_eq(slider(&tab, "step")?, "0.1".to_owned())?;
    check_eq(slider(&tab, "value")?, "2".to_owned())?;
    check_eq(slider(&tab, "aria-valuetext")?, "2.0 Hz".to_owned())
}

#[test]
fn the_first_probe_finds_the_wave_of_2_hz_and_says_so() -> Result<(), String> {
    let tab = open()?;
    let spectrum = written(&tab, SPECTRUM)?;
    check(spectrum.contains("2.0 Hz:   a = +0.64   b = +0.77"), &spectrum)?;
    check(
        spectrum.contains("amplitude = \u{221A}(a\u{00B2} + b\u{00B2}) = 1.00"),
        &spectrum,
    )?;
    check(spectrum.contains("phase = 40\u{00B0}"), &spectrum)?;
    check(written(&tab, COSINE)?.contains("integral +0.32"), &written(&tab, COSINE)?)?;
    check(written(&tab, SINE)?.contains("integral +0.38"), &written(&tab, SINE)?)?;
    check(
        written(&tab, SIGNAL)?.contains("cos(2\u{03C0} \u{00B7} 2.0 \u{00B7} t)"),
        "the probe is named",
    )
}

#[test]
fn each_product_is_shaded_blue_above_the_axis_and_orange_below_it_and_can_be_seen_through() -> Result<(), String> {
    let tab = open()?;
    for scene in [COSINE, SINE] {
        check(!polygons(&tab, scene, ABOVE)?.is_empty(), &format!("{scene} is shaded above"))?;
        check(!polygons(&tab, scene, BELOW)?.is_empty(), &format!("{scene} is shaded below"))?;
        let opacity = text(
            &tab,
            &format!("document.querySelector('{scene} polygon[fill-opacity]').getAttribute('fill-opacity')"),
        )?;
        check_eq(opacity, "0.4".to_owned())?;
    }
    // The arrow heads of the axes are polygons too, but they are not shaded.
    check_eq(count(&tab, &format!("{SIGNAL} polygon[fill-opacity]"))?, 0)?;
    check_eq(count(&tab, &format!("{SPECTRUM} polygon[fill-opacity]"))?, 0)
}

#[test]
fn the_area_of_the_shading_in_the_browser_is_the_area_that_is_written() -> Result<(), String> {
    let tab = open()?;
    // The plot is 400 wide with 40 at the left and 16 at the right, and 250 high, for 1 second and 3.6 of height.
    let per_unit = (400.0 - 40.0 - 16.0) * (250.0 / 3.6);
    for scene in [COSINE, SINE] {
        let above: f64 = polygons(&tab, scene, ABOVE)?.iter().map(|p| area(p)).sum::<f64>() / per_unit;
        let below: f64 = polygons(&tab, scene, BELOW)?.iter().map(|p| area(p)).sum::<f64>() / per_unit;
        let said_above = figure(&tab, scene, " above")?;
        let said_below = figure(&tab, scene, " below")?;
        check(
            near(above, said_above, 0.01),
            &format!("{scene}: shaded {above:.3} above, written {said_above}"),
        )?;
        check(
            near(below, -said_below, 0.01),
            &format!("{scene}: shaded {below:.3} below, written {said_below}"),
        )?;
    }
    Ok(())
}

#[test]
fn the_keys_move_the_slider_and_every_scene_follows() -> Result<(), String> {
    let tab = open()?;
    focus(&tab)?;
    press(&tab, "ArrowRight")?;
    check_eq(slider(&tab, "value")?, "2.1".to_owned())?;
    check_eq(slider(&tab, "aria-valuetext")?, "2.1 Hz".to_owned())?;
    for scene in [SIGNAL, COSINE, SINE, SPECTRUM] {
        check(written(&tab, scene)?.contains("2.1"), &format!("{scene} says 2.1"))?;
    }
    press(&tab, "End")?;
    check_eq(slider(&tab, "value")?, "10".to_owned())?;
    press(&tab, "ArrowRight")?;
    check_eq(slider(&tab, "value")?, "10".to_owned())?;
    press(&tab, "Home")?;
    check_eq(slider(&tab, "value")?, "0".to_owned())?;
    check(written(&tab, SPECTRUM)?.contains("0.0 Hz:"), "the spectrum says 0.0 Hz")
}

#[test]
fn a_frequency_that_is_not_in_the_signal_has_areas_that_cancel_and_no_phase() -> Result<(), String> {
    let tab = open()?;
    focus(&tab)?;
    // Three steps of a tenth from 2.0 would be 2.3. To get to 3.0 from 2.0, press ten times.
    for _ in 0..10 {
        press(&tab, "ArrowRight")?;
    }
    check_eq(slider(&tab, "value")?, "3".to_owned())?;
    for scene in [COSINE, SINE] {
        check(written(&tab, scene)?.contains("integral 0.00"), &written(&tab, scene)?)?;
        check(
            figure(&tab, scene, " above")? > 0.1,
            "there is shading, so the areas cancel and are not absent",
        )?;
    }
    let spectrum = written(&tab, SPECTRUM)?;
    check(spectrum.contains("= 0.00   phase = \u{2014}"), &spectrum)
}

#[test]
fn the_probe_of_5_hz_finds_the_second_wave() -> Result<(), String> {
    let tab = open()?;
    focus(&tab)?;
    for _ in 0..30 {
        press(&tab, "ArrowRight")?;
    }
    check_eq(slider(&tab, "value")?, "5".to_owned())?;
    let spectrum = written(&tab, SPECTRUM)?;
    check(
        spectrum.contains("amplitude = \u{221A}(a\u{00B2} + b\u{00B2}) = 0.60"),
        &spectrum,
    )?;
    check(spectrum.contains("phase = 110\u{00B0}"), &spectrum)
}

#[test]
fn the_dot_in_the_spectrum_moves_with_the_slider() -> Result<(), String> {
    let tab = open()?;
    let dot = |tab: &Tab| -> Result<f64, String> {
        text(
            tab,
            &format!("document.querySelector('{SPECTRUM} circle[fill=\"#d63c32\"]').getAttribute('cx')"),
        )?
        .parse()
        .map_err(|e| format!("{e}"))
    };
    let at_two = dot(&tab)?;
    focus(&tab)?;
    for _ in 0..30 {
        press(&tab, "ArrowRight")?;
    }
    let at_five = dot(&tab)?;
    // The plot is 800 wide, with 40 at the left and 30 at the right, for 10 Hz.
    let per_hertz = (800.0 - 40.0 - 30.0) / 10.0;
    check(near(at_five - at_two, 3.0 * per_hertz, 1.0), &format!("{at_two} to {at_five}"))
}

#[test]
fn the_pointer_puts_the_slider_where_it_is_pressed_and_the_scenes_follow() -> Result<(), String> {
    let tab = open()?;
    click(&tab, 0.5)?;
    let value: f64 = slider(&tab, "value")?.parse().map_err(|e| format!("{e}"))?;
    check(near(value, 5.0, 0.3), &format!("half way along is about 5 Hz, not {value}"))?;
    check(
        written(&tab, SPECTRUM)?.contains(&format!("{value:.1} Hz:")),
        "the spectrum follows",
    )
}

#[test]
fn the_plots_describe_themselves_and_the_description_follows_the_slider() -> Result<(), String> {
    let tab = open()?;
    let label = |tab: &Tab, scene: &str| -> Result<String, String> {
        text(
            tab,
            &format!(
                "(() => {{ const s = document.querySelector('{scene}'); \
                 const d = [...s.querySelectorAll('[aria-label]')].map(e => e.getAttribute('aria-label')).join(' ') \
                 + ' ' + [...s.querySelectorAll('desc')].map(e => e.textContent).join(' '); return d; }})()"
            ),
        )
    };
    check(label(&tab, COSINE)?.contains("integral is +0.32"), &label(&tab, COSINE)?)?;
    focus(&tab)?;
    press(&tab, "End")?;
    check(label(&tab, COSINE)?.contains("integral is"), "the description is written again")?;
    check(
        !label(&tab, COSINE)?.contains("integral is +0.32"),
        "and it is no longer the first one",
    )
}
