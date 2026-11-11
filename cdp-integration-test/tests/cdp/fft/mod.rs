//! The FFT panel in the built demo gallery: the graph of three waves and the sliders that set them, the table of what the
//! transform found, the overview, the stepping toolbar, the butterfly inspector, and the lists that choose a size and a
//! butterfly.
//!
//! Native unit tests cover the arithmetic, against an independent transform from the definition. Others cover the
//! cursor that undoes a failed step, and inject every failure. What they cannot reach is the browser. These tests check
//! the following:
//!
//! - The graph follows a slider as it moves, and the scenes are drawn again only when it is let go.
//! - The table finds the amplitude and phase that were set, from the samples alone.
//! - The right number of nodes and connectors are drawn and named.
//! - A stage the toolbar has not reached is marked as not computed.
//! - The numbers on the page are those of a transform worked out here, from the definition.
//! - Choosing a butterfly moves only the inspector and the emphasis.
//! - A rebuild keeps the zoom, and the place the butterfly was dragged to.
//!
//! Like [`crate::dijkstra`], these run against the built demo gallery.

use crate::common::{demo_tab::DemoTab, drag, new_demo_tab};
use headless_chrome::Tab;
use serde::Deserialize;
use std::time::{Duration, Instant};

const OVERVIEW: &str = "#fft-overview-diagram";
const INSPECTOR: &str = "#fft-inspector-diagram";
const VIEWPORT: (u32, u32) = (1800, 3800);
/// Every lane node of the overview: a node whose accessible name starts `S`, a boundary, `[`, a lane and `]`.
const LANES: &str = "[...document.querySelectorAll('#fft-overview-diagram g[aria-label]')] \
     .filter(g => /^S\\d+\\[\\d+\\]/.test(g.getAttribute('aria-label')))";
/// Every connector of the overview: the paths of its content layer that end in an arrow.
const CONNECTORS: &str = "[...document.querySelectorAll('#fft-overview-diagram g.svg-dom-graph-content > path')] \
     .filter(p => p.hasAttribute('marker-end'))";

fn open() -> Result<DemoTab, String> {
    new_demo_tab(
        "index.html#panel-fft",
        &format!("{OVERVIEW} g[aria-label^=\"Step:\"]"),
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

/// Evaluates `expression`, which gives a JSON-serialisable value, and reads it as a `T`.
fn json<T: for<'de> Deserialize<'de>>(tab: &Tab, expression: &str) -> Result<T, String> {
    let raw = text(tab, &format!("JSON.stringify({expression})"))?;
    serde_json::from_str(&raw).map_err(|e| format!("could not read {raw}: {e}"))
}

fn number(tab: &Tab, expression: &str) -> Result<usize, String> {
    json(tab, expression)
}

/// What the panel's status line says.
fn status(tab: &Tab) -> Result<String, String> {
    text(tab, "document.getElementById('fft-status').textContent")
}

/// The title above the inspector.
fn title(tab: &Tab) -> Result<String, String> {
    text(tab, "document.getElementById('fft-inspector-title').textContent")
}

/// The accessible name of the lane node `name`, such as `S3[1]`: its value is in it.
fn lane(tab: &Tab, name: &str) -> Result<String, String> {
    text(
        tab,
        &format!("document.querySelector('{OVERVIEW} g[aria-label^=\"{name}:\"]').getAttribute('aria-label')"),
    )
}

/// The accessible name of the inspector's node `name`.
fn inspector_node(tab: &Tab, name: &str) -> Result<String, String> {
    text(
        tab,
        &format!("document.querySelector('{INSPECTOR} g[aria-label^=\"{name}\"]').getAttribute('aria-label')"),
    )
}

/// The toolbar's position, read from the hidden driver's own selection. `None` before the first stage.
fn position(tab: &Tab) -> Result<Option<usize>, String> {
    let name = text(
        tab,
        &format!("(document.querySelector('{OVERVIEW} g[aria-label^=\"Step:\"]')?.getAttribute('aria-label') ?? '')"),
    )?;
    let Some((before, _)) = name.split_once(" selected") else {
        return Ok(None);
    };
    before
        .rsplit("cell ")
        .next()
        .and_then(|n| n.parse().ok())
        .map(Some)
        .ok_or_else(|| format!("could not read a cell from {name:?}"))
}

/// Waits until `condition` holds, or fails after five seconds.
fn wait_for(what: &str, mut condition: impl FnMut() -> Result<bool, String>) -> Result<(), String> {
    let deadline = Instant::now() + Duration::from_secs(5);
    while Instant::now() < deadline {
        if condition()? {
            return Ok(());
        }
        std::thread::sleep(Duration::from_millis(20));
    }
    Err(format!("timed out waiting for {what}"))
}

/// Presses a toolbar button of the overview with a real mouse click, and waits for the status line to change.
fn click(tab: &Tab, name: &str) -> Result<(), String> {
    let before = status(tab)?;
    tab.find_element(&format!("{OVERVIEW} g[aria-label=\"{name}\"]"))
        .map_err(|e| format!("could not find the {name:?} button: {e}"))?
        .click()
        .map_err(|e| format!("could not click {name:?}: {e}"))?;
    wait_for(&format!("the status to change after {name:?}"), || Ok(status(tab)? != before))
}

fn next(tab: &Tab) -> Result<(), String> {
    click(tab, "Next selection")
}

/// Chooses an entry of the list `id`, as a person would, and waits for `done`.
fn choose(tab: &Tab, id: &str, value: &str) -> Result<(), String> {
    tab.evaluate(
        &format!(
            "(() => {{ const s = document.getElementById('{id}'); s.value = '{value}'; \
             s.dispatchEvent(new Event('change', {{ bubbles: true }})); }})()"
        ),
        false,
    )
    .map_err(|e| format!("could not choose {value:?} in #{id}: {e}"))?;
    Ok(())
}

/// The content layer's `transform` of a scene, which is its zoom and pan.
fn view(tab: &Tab, diagram: &str) -> Result<String, String> {
    text(
        tab,
        &format!("(document.querySelector('{diagram} > g.svg-dom-graph-content')?.getAttribute('transform') ?? '')"),
    )
}

/// How many nodes the overview says are not yet computed.
fn uncomputed(tab: &Tab) -> Result<usize, String> {
    number(
        tab,
        &format!("{LANES}.filter(g => g.getAttribute('aria-label').includes('not yet computed')).length"),
    )
}

/// How many connectors are emphasised.
fn emphasised(tab: &Tab) -> Result<usize, String> {
    number(
        tab,
        &format!("{CONNECTORS}.filter(p => p.getAttribute('aria-current') === 'true').length"),
    )
}

/// The routes of the emphasised connectors.
fn emphasised_routes(tab: &Tab) -> Result<Vec<String>, String> {
    json(
        tab,
        &format!(
            "{CONNECTORS}.filter(p => p.getAttribute('aria-current') === 'true').map(p => p.getAttribute('d')).sort()"
        ),
    )
}

/// The accessible name of every lane node, in the order they are drawn.
fn all_lanes(tab: &Tab) -> Result<Vec<String>, String> {
    json(tab, &format!("{LANES}.map(g => g.getAttribute('aria-label'))"))
}

/// The ids of the sliders, in the order amplitude A, phase A, amplitude B, phase B, amplitude C, phase C.
const SLIDERS: [&str; 6] = [
    "fft-a-amplitude",
    "fft-a-phase",
    "fft-b-amplitude",
    "fft-b-phase",
    "fft-c-amplitude",
    "fft-c-phase",
];

/// The waves the panel starts with, as (amplitude, phase in degrees).
const INITIAL: [(f64, f64); 3] = [(1.0, 0.0), (0.6, 90.0), (0.4, 210.0)];

/// Moves a slider as a person does: an `input` event while it moves, and a `change` event when it is let go.
fn slide(tab: &Tab, id: &str, value: f64, release: bool) -> Result<(), String> {
    tab.evaluate(
        &format!(
            "(() => {{ const s = document.getElementById('{id}'); s.value = '{value}'; \
             s.dispatchEvent(new Event('input', {{ bubbles: true }})); \
             if ({release}) {{ s.dispatchEvent(new Event('change', {{ bubbles: true }})); }} }})()"
        ),
        false,
    )
    .map_err(|e| format!("could not move #{id}: {e}"))?;
    Ok(())
}

/// Sets all three waves and lets the sliders go, one at a time. Waits for the last announcement.
fn set_waves(tab: &Tab, waves: [(f64, f64); 3]) -> Result<(), String> {
    for (index, (amplitude, phase)) in waves.iter().enumerate() {
        slide(tab, SLIDERS[index * 2], *amplitude, true)?;
        slide(tab, SLIDERS[index * 2 + 1], *phase, true)?;
    }
    wait_for("the waves to be announced", || {
        Ok(status(tab)?.starts_with("The transform found"))
    })
}

/// The sum of the waves at sample `i` of `n`.
fn sample(waves: &[(f64, f64); 3], i: usize, n: usize) -> f64 {
    waves
        .iter()
        .enumerate()
        .map(|(index, (amplitude, phase))| {
            let cycles = (index + 1) as f64;
            amplitude * (std::f64::consts::TAU * cycles * i as f64 / n as f64 + phase.to_radians()).sin()
        })
        .sum()
}

/// The transform of the samples of `waves`, straight from the definition `X[k] = Σ x[n]·e^(−2πikn/N)`.
fn transform(waves: &[(f64, f64); 3], n: usize) -> Vec<(f64, f64)> {
    (0..n)
        .map(|k| {
            (0..n).fold((0.0, 0.0), |(re, im), i| {
                let angle = -std::f64::consts::TAU * (k * i) as f64 / n as f64;
                let x = sample(waves, i, n);
                (re + x * angle.cos(), im + x * angle.sin())
            })
        })
        .collect()
}

/// A complex number as a node's name writes it: three decimals, a zero for anything that rounds to one.
fn written((re, im): (f64, f64)) -> String {
    let tidy = |x: f64| if x.abs() < 5e-4 { 0.0 } else { x };
    let (re, im) = (tidy(re), tidy(im));
    format!("{re:.3} {} {:.3}i", if im < 0.0 { '-' } else { '+' }, im.abs())
}

/// The number in the accessible name of the lane `name`, such as `S3[1]`.
fn value_of(tab: &Tab, name: &str) -> Result<String, String> {
    text(
        tab,
        &format!(
            "document.querySelector('{OVERVIEW} g[aria-label^=\"{name}:\"]').getAttribute('aria-label') \
             .match(/-?\\d+\\.\\d{{3}} [+-] \\d+\\.\\d{{3}}i/)[0]"
        ),
    )
}

/// The text of the cell in row `row` and column `column` (both from 0, the header row not counted) of the table.
fn cell(tab: &Tab, row: usize, column: usize) -> Result<String, String> {
    text(
        tab,
        &format!("document.querySelectorAll('#fft-findings tbody tr')[{row}].children[{column}].textContent"),
    )
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The frame of the inspector's butterfly: the centre of its title, which is inside the frame and on nothing else.
fn frame_title_point(tab: &Tab) -> Result<(f64, f64), String> {
    let (x, y): (f64, f64) = {
        let point: [f64; 2] = json(
            tab,
            &format!(
                "(() => {{ const g = [...document.querySelectorAll('{INSPECTOR} g.svg-dom-graph-content > g')] \
                 .find(g => g.querySelector(':scope > text')?.textContent === 'Radix-2 Butterfly'); \
                 const r = g.querySelector(':scope > text').getBoundingClientRect(); \
                 return [r.left + r.width / 2, r.top + r.height / 2]; }})()"
            ),
        )?;
        (point[0], point[1])
    };
    Ok((x, y))
}

/// Where the frame's rectangle is on the page.
fn frame_corner(tab: &Tab) -> Result<(f64, f64), String> {
    let point: [f64; 2] = json(
        tab,
        &format!(
            "(() => {{ const g = [...document.querySelectorAll('{INSPECTOR} g.svg-dom-graph-content > g')] \
             .find(g => g.querySelector(':scope > text')?.textContent === 'Radix-2 Butterfly'); \
             const r = g.getBoundingClientRect(); return [r.left, r.top]; }})()"
        ),
    )?;
    Ok((point[0], point[1]))
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Makes the `n`th attempt, from now, to show a replacement scene throw. A replacement is shown by taking the `style` that
/// hides it off its staging `<svg>`, which is the one write of a rebuild that a browser can refuse. The overview's is the
/// first attempt and the inspector's is the second.
fn fail_showing_replacement(tab: &Tab, n: u32) -> Result<(), String> {
    tab.evaluate(
        &format!(
            "(() => {{ const proto = Element.prototype; const original = proto.removeAttribute; let calls = 0; \
             proto.removeAttribute = function (name) {{ \
                 if (name === 'style' && this.id && this.id.endsWith('-staging')) {{ \
                     calls += 1; if (calls === {n}) {{ throw new Error('injected failure showing ' + this.id); }} }} \
                 return original.apply(this, arguments); }}; \
             window.__restoreRemoveAttribute = () => {{ proto.removeAttribute = original; }}; }})()"
        ),
        false,
    )
    .map_err(|e| format!("could not inject the failure: {e}"))?;
    Ok(())
}

fn stop_failing(tab: &Tab) -> Result<(), String> {
    tab.evaluate("window.__restoreRemoveAttribute()", false)
        .map_err(|e| format!("could not remove the injected failure: {e}"))?;
    Ok(())
}

mod unit_tests;
