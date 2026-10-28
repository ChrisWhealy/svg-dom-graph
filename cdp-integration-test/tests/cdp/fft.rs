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

use crate::common::{DemoTab, drag, new_demo_tab};
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
/// The panel opens unstarted, on eight samples of the three waves. The overview has four columns of eight lanes, so 32
/// nodes, and 48 connectors, one more node that is hidden. Only the input column holds numbers: the other 24 are marked
/// as not computed. The inspector previews stage 1 at its default pair, the first, since the first sample is not zero.
#[test]
fn the_panel_opens_unstarted_with_the_documented_counts() -> Result<(), String> {
    let tab = open()?;
    check_eq(number(&tab, &format!("{LANES}.length"))?, 32)?;
    check_eq(number(&tab, &format!("{CONNECTORS}.length"))?, 48)?;
    check_eq(
        number(
            &tab,
            &format!("document.querySelectorAll('{OVERVIEW} g[aria-label^=\"Step:\"]').length"),
        )?,
        1,
    )?;
    check_eq(uncomputed(&tab)?, 24)?;
    check_eq(position(&tab)?, None)?;

    check(
        status(&tab)?.ends_with(
            "FFT ready: N=8 samples of the summed waves. No stages completed. Previewing stage 1, pair 1, lanes 0 and 1.",
        ),
        &format!("the status says that nothing is complete: {:?}", status(&tab)?),
    )?;
    check_eq(title(&tab)?.as_str(), "Preview: stage 1, pair 1")?;
    check_eq(number(&tab, "document.getElementById('fft-pair').options.length")?, 4)?;
    check_eq(number(&tab, "document.getElementById('fft-pair').selectedIndex")?, 0)?;

    // The input column is bit-reversed: lane 1 starts with x[4], and lane 4 with x[1].
    for (lane_number, source) in [(0, 0), (1, 4), (2, 2), (4, 1)] {
        let expected = written((sample(&INITIAL, source, 8), 0.0));
        check_eq(value_of(&tab, &format!("S0[{lane_number}]=x[{source}]"))?, expected)?;
    }
    check(
        lane(&tab, "S1[4]")?.ends_with("not yet computed"),
        "a stage that has not run is marked",
    )?;
    check_eq(emphasised(&tab)?, 4)?;
    Ok(())
}

/// Each press of Next completes one whole stage. After stage 1 the first two columns hold numbers and the last two do
/// not. After the third, nothing is marked, and the last column is the transform, in order, as the definition gives it.
/// Stage 1 joins the pairs of the input column with a twiddle factor of 1, so lane 0 is `x[0] + x[4]`.
#[test]
fn next_completes_one_stage_and_the_last_column_is_the_transform() -> Result<(), String> {
    let tab = open()?;
    next(&tab)?;
    check_eq(position(&tab)?, Some(0))?;
    check_eq(uncomputed(&tab)?, 16)?;
    let (x0, x4) = (sample(&INITIAL, 0, 8), sample(&INITIAL, 4, 8));
    check_eq(value_of(&tab, "S1[0]")?, written((x0 + x4, 0.0)))?;
    check_eq(value_of(&tab, "S1[1]")?, written((x0 - x4, 0.0)))?;
    check(
        status(&tab)?
            .starts_with("Stage 1 of 3 complete; block width 2; pair 1 of 4; lanes 0 and 1; twiddle W2^0 = 1."),
        &format!("the stage is announced: {:?}", status(&tab)?),
    )?;
    check_eq(title(&tab)?.as_str(), "Stage 1 of 3, pair 1 of 4")?;

    next(&tab)?;
    check_eq(uncomputed(&tab)?, 8)?;
    next(&tab)?;
    check_eq(position(&tab)?, Some(2))?;
    check_eq(uncomputed(&tab)?, 0)?;
    check(
        status(&tab)?.contains("This is the transform"),
        "the last stage says that it is the transform",
    )?;

    for (k, expected) in transform(&INITIAL, 8).into_iter().enumerate() {
        check_eq(value_of(&tab, &format!("S3[{k}]"))?, written(expected))?;
    }
    Ok(())
}

/// Previous steps back and shows the stored state again. Previous from the first stage, and Restart, both return to
/// unstarted, with the same waves and the default pair.
#[test]
fn previous_and_restart_return_to_the_stored_states() -> Result<(), String> {
    let tab = open()?;
    next(&tab)?;
    next(&tab)?;
    let at_two = all_lanes(&tab)?;
    next(&tab)?;
    click(&tab, "Previous selection")?;
    check_eq(position(&tab)?, Some(1))?;
    check_eq(all_lanes(&tab)?, at_two)?;

    click(&tab, "Restart selection")?;
    check_eq(position(&tab)?, None)?;
    check_eq(uncomputed(&tab)?, 24)?;
    check(
        status(&tab)?.starts_with("FFT ready: N=8 samples of the summed waves."),
        "restart says that nothing is complete",
    )?;
    check_eq(title(&tab)?.as_str(), "Preview: stage 1, pair 1")?;

    next(&tab)?;
    click(&tab, "Previous selection")?;
    check_eq(position(&tab)?, None)?;
    Ok(())
}

/// Choosing another butterfly of the stage changes the inspector, the emphasis and the status, and no node of the
/// overview. Pair 2 of stage 3 joins lanes 1 and 5 with `W8^1`. Its inputs are those two lanes of the stage before, and
/// its outputs are those two lanes of stage 3.
#[test]
fn choosing_a_butterfly_moves_only_the_inspector_and_the_emphasis() -> Result<(), String> {
    let tab = open()?;
    next(&tab)?;
    next(&tab)?;
    next(&tab)?;
    let lanes_before = all_lanes(&tab)?;
    let routes_before = emphasised_routes(&tab)?;
    let said = status(&tab)?;

    choose(&tab, "fft-pair", "1")?;
    wait_for("the status to name the pair", || Ok(status(&tab)? != said))?;
    check(
        status(&tab)?.starts_with("Stage 3, pair 2 of 4: lanes 1 and 5; W8^1 = 0.707 \u{2212} 0.707i."),
        &format!("the pair is announced: {:?}", status(&tab)?),
    )?;
    check_eq(title(&tab)?.as_str(), "Stage 3 of 3, pair 2 of 4")?;
    check_eq(all_lanes(&tab)?, lanes_before)?;
    check_eq(emphasised(&tab)?, 4)?;
    check(
        emphasised_routes(&tab)? != routes_before,
        "the emphasis moved to the new butterfly's connectors",
    )?;

    check(
        inspector_node(&tab, "a:")?.contains(&value_of(&tab, "S2[1]")?),
        "a is lane 1 of stage 2",
    )?;
    check(
        inspector_node(&tab, "b:")?.contains(&value_of(&tab, "S2[5]")?),
        "b is lane 5 of stage 2",
    )?;
    check(inspector_node(&tab, "w:")?.contains("0.707 - 0.707i"), "w is W8^1")?;
    check(
        inspector_node(&tab, "ADD result")?.contains(&value_of(&tab, "S3[1]")?),
        "the sum is lane 1 of stage 3",
    )?;
    check(
        inspector_node(&tab, "SUB result")?.contains(&value_of(&tab, "S3[5]")?),
        "the difference is lane 5 of stage 3",
    )?;
    check_eq(number(&tab, "document.getElementById('fft-pair').selectedIndex")?, 1)?;
    Ok(())
}

/// A pair chosen at one stage is found again after stepping away and back, and Restart forgets it.
#[test]
fn a_chosen_butterfly_is_remembered_for_its_stage() -> Result<(), String> {
    let tab = open()?;
    next(&tab)?;
    choose(&tab, "fft-pair", "0")?;
    wait_for("the pair to change", || {
        Ok(number(&tab, "document.getElementById('fft-pair').selectedIndex")? == 0
            && title(&tab)?.contains("pair 1 of"))
    })?;
    next(&tab)?;
    check_eq(number(&tab, "document.getElementById('fft-pair').selectedIndex")?, 0)?;
    choose(&tab, "fft-pair", "3")?;
    wait_for("the pair to change", || {
        Ok(number(&tab, "document.getElementById('fft-pair').selectedIndex")? == 3
            && title(&tab)?.contains("pair 4 of"))
    })?;
    click(&tab, "Previous selection")?;
    check_eq(number(&tab, "document.getElementById('fft-pair').selectedIndex")?, 0)?;
    next(&tab)?;
    check_eq(number(&tab, "document.getElementById('fft-pair').selectedIndex")?, 3)?;

    click(&tab, "Restart selection")?;
    check_eq(number(&tab, "document.getElementById('fft-pair').selectedIndex")?, 0)?;
    Ok(())
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Choosing sixteen samples draws 80 lane nodes and 128 connectors, and a pair list of eight, and the toolbar has four
/// stages. A wave A of amplitude 1 and phase 90°, with the others silent, is a cosine that fits the window once. It has
/// two peaks of `N/2`, in bins 1 and 15.
#[test]
fn sixteen_samples_draw_eighty_nodes_and_a_lone_wave_peaks_in_two_bins() -> Result<(), String> {
    let tab = open()?;
    choose(&tab, "fft-length", "16")?;
    wait_for("sixteen lanes", || Ok(number(&tab, &format!("{LANES}.length"))? == 80))?;
    check_eq(number(&tab, &format!("{CONNECTORS}.length"))?, 128)?;
    check_eq(
        number(
            &tab,
            &format!("document.querySelectorAll('{OVERVIEW} g[aria-label^=\"Step:\"]').length"),
        )?,
        1,
    )?;
    check_eq(number(&tab, "document.getElementById('fft-pair').options.length")?, 8)?;
    check(status(&tab)?.contains("N=16"), "the status names the new size")?;
    check_eq(position(&tab)?, None)?;
    check_eq(uncomputed(&tab)?, 64)?;

    set_waves(&tab, [(1.0, 90.0), (0.0, 0.0), (0.0, 0.0)])?;
    check_eq(position(&tab)?, None)?;
    for _ in 0..4 {
        next(&tab)?;
    }
    check_eq(uncomputed(&tab)?, 0)?;
    for (bin, expected) in [(0, "0.000"), (1, "8.000"), (2, "0.000"), (15, "8.000")] {
        let name = lane(&tab, &format!("S4[{bin}]"))?;
        check(
            name.contains(&format!("{expected} +")) || name.contains(&format!("{expected} -")),
            &format!("bin {bin}: {name}"),
        )?;
    }
    Ok(())
}

/// Letting go of a slider restarts the walk, in the same scene: the toolbar is unstarted and the numbers are the new
/// waves'. A lone wave A of amplitude 0.5 puts `N × 0.5 / 2 = 2` in bin 1, and nothing in bin 0 or bin 2.
#[test]
fn letting_go_of_a_slider_restarts_the_walk() -> Result<(), String> {
    let tab = open()?;
    next(&tab)?;
    next(&tab)?;
    set_waves(&tab, [(0.5, 90.0), (0.0, 0.0), (0.0, 0.0)])?;
    check_eq(position(&tab)?, None)?;
    check_eq(uncomputed(&tab)?, 24)?;
    check(
        title(&tab)?.starts_with("Preview: stage 1, pair"),
        "the title is a preview again",
    )?;
    for _ in 0..3 {
        next(&tab)?;
    }
    check_eq(value_of(&tab, "S3[1]")?.as_str(), "2.000 + 0.000i")?;
    check_eq(value_of(&tab, "S3[7]")?.as_str(), "2.000 + 0.000i")?;
    check_eq(value_of(&tab, "S3[0]")?.as_str(), "0.000 + 0.000i")?;
    check_eq(value_of(&tab, "S3[2]")?.as_str(), "0.000 + 0.000i")?;
    Ok(())
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The graph has the three waves and the sum as four curves, and one mark for each sample, and one more in its key. The
/// vertical axis stands at the left edge of the graph, and nothing is drawn left of it but its labels.
#[test]
fn the_graph_plots_three_waves_and_their_sum_with_one_mark_for_each_sample() -> Result<(), String> {
    let tab = open()?;
    check_eq(
        number(&tab, "document.querySelectorAll('#fft-wave-plot svg polyline').length")?,
        4,
    )?;
    check_eq(
        number(&tab, "document.querySelectorAll('#fft-wave-plot svg circle').length")?,
        9,
    )?;
    check_eq(
        text(
            &tab,
            "document.querySelector('#fft-wave-plot g[role=\"img\"]').getAttribute('role')",
        )?
        .as_str(),
        "img",
    )?;
    check(
        text(
            &tab,
            "document.querySelector('#fft-wave-plot g[role=\"img\"]').getAttribute('aria-label')",
        )?
        .contains("wave B, 2 cycles, amplitude 0.60, phase 90 degrees"),
        "the graph is described",
    )?;
    let (left, right): (f64, f64) = {
        let extent: [f64; 2] = json(
            &tab,
            "(() => { const xs = [...document.querySelectorAll('#fft-wave-plot svg polyline')] \
             .flatMap(p => p.getAttribute('points').split(' ').map(q => Number(q.split(',')[0]))); \
             return [Math.min(...xs), Math.max(...xs)]; })()",
        )?;
        (extent[0], extent[1])
    };
    check(
        left >= 52.0 - 1e-6,
        &format!("no curve starts left of the vertical axis: {left}"),
    )?;
    check(right > left, "time runs to the right")?;
    choose(&tab, "fft-length", "16")?;
    wait_for("sixteen samples", || {
        Ok(number(&tab, "document.querySelectorAll('#fft-wave-plot svg circle').length")? == 17)
    })?;
    Ok(())
}

/// The graph, the numbers beside the sliders and the table follow a slider as it moves, and the scenes do not. They are
/// drawn again, as new elements, when the slider is let go.
#[test]
fn the_graph_follows_a_moving_slider_and_the_scenes_wait_until_it_is_let_go() -> Result<(), String> {
    let tab = open()?;
    let sum_before = text(
        &tab,
        "document.querySelectorAll('#fft-wave-plot svg polyline')[3].getAttribute('points')",
    )?;
    check_eq(
        text(&tab, "document.getElementById('fft-b-amplitude-out').textContent")?.as_str(),
        "0.60",
    )?;
    check_eq(
        text(&tab, "document.getElementById('fft-b-phase-out').textContent")?.as_str(),
        "90\u{00B0}",
    )?;
    tab.evaluate(
        "window.__overview = document.getElementById('fft-overview-diagram'); \
         window.__plot = document.querySelector('#fft-wave-plot svg');",
        false,
    )
    .map_err(|e| format!("could not mark the scene: {e}"))?;

    slide(&tab, "fft-b-amplitude", 0.2, false)?;
    check_eq(
        text(&tab, "document.getElementById('fft-b-amplitude-out').textContent")?.as_str(),
        "0.20",
    )?;
    check(
        text(
            &tab,
            "document.querySelectorAll('#fft-wave-plot svg polyline')[3].getAttribute('points')",
        )? != sum_before,
        "the sum was drawn again",
    )?;
    check_eq(cell(&tab, 1, 3)?.as_str(), "0.20")?;
    check_eq(cell(&tab, 1, 4)?.as_str(), "0.20")?;
    check_eq(
        text(
            &tab,
            "String(window.__overview === document.getElementById('fft-overview-diagram'))",
        )?
        .as_str(),
        "true",
    )?;

    slide(&tab, "fft-b-amplitude", 0.2, true)?;
    wait_for("the scene to be replaced", || {
        Ok(text(
            &tab,
            "String(window.__overview === document.getElementById('fft-overview-diagram'))",
        )? == "false")
    })?;
    Ok(())
}

/// The table finds, from the samples alone, the amplitude and phase that were set, for every kind of wave: a phase
/// either side of the turn, an amplitude of zero, and a different number of samples.
#[test]
fn the_table_finds_the_amplitude_and_phase_that_were_set() -> Result<(), String> {
    let tab = open()?;
    for (row, (amplitude, phase)) in INITIAL.iter().enumerate() {
        check_eq(cell(&tab, row, 3)?, format!("{amplitude:.2}"))?;
        check_eq(cell(&tab, row, 4)?, format!("{amplitude:.2}"))?;
        check_eq(cell(&tab, row, 5)?, format!("{phase:.0}\u{00B0}"))?;
        check_eq(cell(&tab, row, 6)?, format!("{phase:.0}\u{00B0}"))?;
        check_eq(cell(&tab, row, 7)?.as_str(), "Yes")?;
    }
    check(
        text(&tab, "document.querySelector('#fft-findings p').textContent")?
            .ends_with("The transform found nothing that was not put in."),
        "nothing else is found",
    )?;

    choose(&tab, "fft-length", "16")?;
    wait_for("sixteen lanes", || Ok(number(&tab, &format!("{LANES}.length"))? == 80))?;
    for waves in [
        [(0.35, 345.0), (0.8, 15.0), (1.0, 180.0)],
        [(0.0, 120.0), (0.5, 270.0), (0.05, 0.0)],
    ] {
        set_waves(&tab, waves)?;
        for (row, (amplitude, phase)) in waves.iter().enumerate() {
            check_eq(cell(&tab, row, 4)?, format!("{amplitude:.2}"))?;
            if *amplitude == 0.0 {
                check_eq(cell(&tab, row, 6)?.as_str(), "\u{2014}")?;
            } else {
                check_eq(cell(&tab, row, 6)?, format!("{phase:.0}\u{00B0}"))?;
            }
            check_eq(cell(&tab, row, 7)?.as_str(), "Yes")?;
        }
    }
    Ok(())
}

/// The sliders are native range inputs with a label each, and each has an output beside it that is tied to it.
#[test]
fn every_slider_is_labelled_and_has_its_number_beside_it() -> Result<(), String> {
    let tab = open()?;
    for id in SLIDERS {
        check_eq(text(&tab, &format!("document.getElementById('{id}').type"))?.as_str(), "range")?;
        check_eq(
            number(&tab, &format!("document.querySelectorAll('label[for=\"{id}\"]').length"))?,
            1,
        )?;
        check_eq(
            number(&tab, &format!("document.querySelectorAll('output[for=\"{id}\"]').length"))?,
            1,
        )?;
    }
    Ok(())
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

/// A rebuild keeps the overview's zoom, and keeps the place the inspector's butterfly was dragged to.
#[test]
fn a_rebuild_keeps_the_zoom_and_where_the_butterfly_was_dragged() -> Result<(), String> {
    let tab = open()?;
    tab.find_element(&format!("{OVERVIEW} g[aria-label=\"Zoom in\"]"))
        .map_err(|e| format!("could not find Zoom in: {e}"))?
        .click()
        .map_err(|e| format!("could not click Zoom in: {e}"))?;
    wait_for("the zoom", || Ok(view(&tab, OVERVIEW)?.contains("scale")))?;
    let zoomed = view(&tab, OVERVIEW)?;

    let (x, y) = frame_title_point(&tab)?;
    let (left, top) = frame_corner(&tab)?;
    // The frame fills the canvas from top to bottom but for a margin, so there is little room to move it down.
    drag(&tab, &[(x, y), (x + 20.0, y + 4.0), (x + 60.0, y + 10.0)])?;
    let (dragged_left, dragged_top) = frame_corner(&tab)?;
    check(
        (dragged_left - left - 60.0).abs() < 2.0 && (dragged_top - top - 10.0).abs() < 2.0,
        &format!("the frame moved by the drag: ({left}, {top}) to ({dragged_left}, {dragged_top})"),
    )?;

    set_waves(&tab, [(0.5, 90.0), (0.0, 0.0), (0.0, 0.0)])?;
    check_eq(view(&tab, OVERVIEW)?, zoomed)?;
    let (kept_left, kept_top) = frame_corner(&tab)?;
    check(
        (kept_left - dragged_left).abs() < 2.0 && (kept_top - dragged_top).abs() < 2.0,
        &format!("the frame stayed where it was dragged: ({dragged_left}, {dragged_top}) to ({kept_left}, {kept_top})"),
    )?;
    Ok(())
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The connectors are real paths with no accessible name and no tab stop, and the lane nodes are not tab stops either.
/// The topology is read from the nodes' names instead: each lane says what it feeds, and what feeds it. So the page
/// does not offer 128 more things to tab through at sixteen samples.
#[test]
fn the_connectors_are_not_tab_stops_and_the_nodes_name_their_neighbours() -> Result<(), String> {
    let tab = open()?;
    check_eq(
        number(
            &tab,
            &format!("{CONNECTORS}.filter(p => p.hasAttribute('tabindex') || p.hasAttribute('aria-label')).length"),
        )?,
        0,
    )?;
    check_eq(
        number(&tab, &format!("{LANES}.filter(g => g.hasAttribute('tabindex')).length"))?,
        0,
    )?;
    check(
        lane(&tab, "S1[4]")?.contains("Input from S0[4]=x[1]. Input from S0[5]=x[5]."),
        "a lane names its inputs",
    )?;
    check(
        lane(&tab, "S1[4]")?.contains("Output to S2[4]. Output to S2[6]."),
        "a lane names its outputs",
    )?;
    check_eq(
        text(
            &tab,
            &format!("document.querySelector('{OVERVIEW} g[aria-label^=\"Step:\"]').getAttribute('aria-hidden')"),
        )?
        .as_str(),
        "true",
    )?;
    let labelled = number(
        &tab,
        "document.querySelectorAll('label[for=\"fft-pair\"], label[for=\"fft-length\"]').length",
    )?;
    check_eq(labelled, 2)?;
    Ok(())
}

/// The two lists are native, and the live region is one persistent element outside both scenes.
#[test]
fn the_lists_are_native_and_the_status_is_one_live_region_outside_the_scenes() -> Result<(), String> {
    let tab = open()?;
    for id in ["fft-length", "fft-pair"] {
        check_eq(
            text(&tab, &format!("document.getElementById('{id}').tagName"))?.as_str(),
            "SELECT",
        )?;
    }
    check_eq(
        text(&tab, "document.getElementById('fft-status').getAttribute('aria-live')")?.as_str(),
        "polite",
    )?;
    check_eq(
        text(&tab, "String(document.getElementById('fft-status').closest('svg') === null)")?.as_str(),
        "true",
    )?;
    let before = text(&tab, "String(document.getElementById('fft-status').isConnected)")?;
    choose(&tab, "fft-length", "16")?;
    wait_for("sixteen lanes", || Ok(number(&tab, &format!("{LANES}.length"))? == 80))?;
    check_eq(before.as_str(), "true")?;
    check_eq(
        text(&tab, "String(document.getElementById('fft-status').isConnected)")?.as_str(),
        "true",
    )?;
    // No staging copy is left behind by a rebuild.
    check_eq(
        number(
            &tab,
            "document.querySelectorAll('#fft-overview-staging, #fft-inspector-staging').length",
        )?,
        0,
    )?;
    check_eq(
        number(
            &tab,
            "document.querySelectorAll('#fft-overview-diagram, #fft-inspector-diagram').length",
        )?,
        2,
    )?;
    Ok(())
}

/// At sixteen samples, Chrome's own accessibility tree shows no connector as a named or focusable item, so the page
/// does not offer 128 more things to read or tab through. The stops of the keyboard are the toolbars of the two scenes,
/// and nothing else in them.
#[test]
fn at_sixteen_samples_the_browser_exposes_no_connector_as_a_named_or_focusable_item() -> Result<(), String> {
    use crate::step_driver_accessibility::{ax_node, ax_string};
    use headless_chrome::protocol::cdp::Accessibility;

    let tab = open()?;
    tab.call_method(Accessibility::Enable(None))
        .map_err(|e| format!("Accessibility.enable failed: {e}"))?;
    choose(&tab, "fft-length", "16")?;
    wait_for("sixteen lanes", || Ok(number(&tab, &format!("{LANES}.length"))? == 80))?;

    let connectors = tab
        .find_elements(&format!("{OVERVIEW} g.svg-dom-graph-content > path[marker-end]"))
        .map_err(|e| format!("could not find the connectors: {e}"))?;
    check_eq(connectors.len(), 128)?;
    let mut offending = Vec::new();
    for (index, connector) in connectors.iter().enumerate() {
        let Some(node) = ax_node(&tab, connector)? else { continue };
        if node["ignored"].as_bool() == Some(true) {
            continue;
        }
        let named = ax_string(&node, "name").is_some_and(|name| !name.is_empty());
        let focusable = node["properties"].as_array().is_some_and(|properties| {
            properties
                .iter()
                .any(|p| p["name"] == "focusable" && p["value"]["value"].as_bool() == Some(true))
        });
        if named || focusable {
            offending.push((index, named, focusable));
        }
    }
    check(
        offending.is_empty(),
        &format!("connectors exposed as named or focusable: {offending:?}"),
    )?;

    // Every keyboard stop in the panel is a toolbar button, or the scene itself.
    let stops: Vec<String> = json(
        &tab,
        "[...document.querySelectorAll('#panel-fft [tabindex]')] \
         .filter(e => e.getAttribute('tabindex') !== '-1' && !e.closest('[aria-hidden=\"true\"]')) \
         .map(e => e.getAttribute('aria-label') ?? e.tagName)",
    )?;
    let allowed = [
        "Zoom in",
        "Zoom out",
        "Reset zoom",
        "Previous selection",
        "Next selection",
        "Restart selection",
    ];
    let strangers: Vec<&String> = stops
        .iter()
        .filter(|name| !allowed.contains(&name.as_str()) && !name.starts_with("Graph view"))
        .collect();
    check(
        strangers.is_empty(),
        &format!("unexpected keyboard stops: {strangers:?} in {stops:?}"),
    )?;
    Ok(())
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

/// A rebuild that cannot show its overview, or cannot show its inspector, leaves the old diagrams exactly as they were: the
/// same elements, still on the page and still working. No half-shown replacement or staging copy is left, the controls go
/// back to what the diagrams show, and the error is on the page. A later rebuild, with nothing failing, then succeeds.
#[test]
fn a_rebuild_that_cannot_be_shown_leaves_the_old_diagrams_in_place_and_working() -> Result<(), String> {
    for failing in [1, 2] {
        let tab = open()?;
        next(&tab)?;
        let lanes = all_lanes(&tab)?;
        tab.evaluate(
            "window.__old = [document.getElementById('fft-overview-diagram'), \
             document.getElementById('fft-inspector-diagram')]",
            false,
        )
        .map_err(|e| format!("could not mark the old diagrams: {e}"))?;
        let same = |tab: &Tab| {
            text(
                tab,
                "String(window.__old[0] === document.getElementById('fft-overview-diagram') \
                 && window.__old[1] === document.getElementById('fft-inspector-diagram') \
                 && window.__old[0].isConnected && window.__old[1].isConnected)",
            )
        };

        fail_showing_replacement(&tab, failing)?;
        slide(&tab, "fft-b-amplitude", 0.2, true)?;
        wait_for("the error to be shown", || {
            Ok(text(&tab, "String(!document.getElementById('fft-error').hidden)")? == "true")
        })?;
        stop_failing(&tab)?;

        check_eq(same(&tab)?.as_str(), "true")?;
        check(
            text(&tab, "document.getElementById('fft-error').textContent")?.contains("old one is kept"),
            "the error says that the old diagram is kept",
        )?;
        check_eq(
            number(
                &tab,
                "document.querySelectorAll('#fft-overview-staging, #fft-inspector-staging').length",
            )?,
            0,
        )?;
        check_eq(text(&tab, "document.getElementById('fft-b-amplitude').value")?.as_str(), "0.6")?;
        check_eq(all_lanes(&tab)?, lanes)?;
        check_eq(position(&tab)?, Some(0))?;

        // The old diagram still steps, and a rebuild with nothing failing is made.
        next(&tab)?;
        check_eq(position(&tab)?, Some(1))?;
        slide(&tab, "fft-b-amplitude", 0.2, true)?;
        wait_for("the diagrams to be replaced", || Ok(same(&tab)? == "false"))?;
        check_eq(position(&tab)?, None)?;
        check_eq(number(&tab, &format!("{LANES}.length"))?, 32)?;
    }
    Ok(())
}
