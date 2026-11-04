//! The marching squares panel in the built demo gallery: a real `Scene` with one plot node that zooms and pans, and
//! controls in the stage above it that do not.
//!
//! Native unit tests cover the algorithm, the sixteen cases, the saddles, the stitching of any set of cells, the
//! projection of a state, and every rule of the controller. What they cannot reach is the browser. These tests check
//! the following:
//!
//! - The controls are SVG in the stage, outside the swapped part and the zoom, and the selectors and the number are the
//!   browser's own controls in a `<foreignObject>`.
//! - The drawn contours are right in the page: each vertex of each polyline is on a grid edge at the place where the
//!   interpolated samples equal the level, read through the plot's own mapping, at 0.25x, 1x and 4x and after a pan.
//! - The ends of the open contours survive clipping at the boundary, and a loop has a dot at its seam.
//! - The step, the range, the buttons and the selectors reach the same state whatever came before, and a cell that has
//!   not been visited reveals nothing.
//! - A threshold edit changes the topology and editing it back restores the digest, and a refused one leaves
//!   everything.
//! - A scene rebuilt 25 times at a zoomed and panned view is the same, with unique IDs, one scene, and one handler.
//! - A burst of range moves draws once, and a late frame cannot overwrite a newer move.
//! - A write that fails, a scene that cannot be built, or a swap that throws before or after it changes the host, is
//!   reported, keeps the state, and is put right by the next action.
//!
//! Like [`crate::red_black_tree`], these run against the built demo gallery.

use crate::common::{DemoTab, new_demo_tab};
use headless_chrome::Tab;
use serde_json::Value;
use std::time::{Duration, Instant};

const STAGE: &str = "#ms-stage";
const VIEWPORT: (u32, u32) = (1400, 1700);
/// The cells of the default field, and the levels it starts with.
const CELLS: usize = 432;

fn open_at(query: &str) -> Result<DemoTab, String> {
    new_demo_tab(
        &format!("index.html{query}#panel-marching-squares"),
        &format!("{STAGE} .ms-scene-root"),
        VIEWPORT,
    )
}

fn open() -> Result<DemoTab, String> {
    open_at("")
}

/// The panel with the frames on a clock that only the test advances.
fn open_manual() -> Result<DemoTab, String> {
    open_at("?manual-clock")
}

fn check(condition: bool, message: &str) -> Result<(), String> {
    if condition { Ok(()) } else { Err(message.to_owned()) }
}

fn check_eq<T: PartialEq + std::fmt::Debug>(got: T, expected: T) -> Result<(), String> {
    check(got == expected, &format!("expected {expected:?}, got {got:?}"))
}

fn text(tab: &Tab, expression: &str) -> Result<String, String> {
    tab.evaluate(expression, false)
        .map_err(|e| format!("could not evaluate {expression}: {e}"))?
        .value
        .and_then(|v| v.as_str().map(str::to_owned))
        .ok_or_else(|| format!("{expression} did not return a string"))
}

fn json(tab: &Tab, expression: &str) -> Result<Value, String> {
    serde_json::from_str(&text(tab, expression)?).map_err(|e| format!("bad JSON from {expression}: {e}"))
}

fn count(tab: &Tab, selector: &str) -> Result<usize, String> {
    text(tab, &format!("String(document.querySelectorAll('{selector}').length)"))?
        .parse()
        .map_err(|e| format!("{e}"))
}

fn attribute(tab: &Tab, selector: &str, name: &str) -> Result<String, String> {
    text(
        tab,
        &format!("String(document.querySelector('{selector}').getAttribute('{name}'))"),
    )
}

fn content(tab: &Tab, selector: &str) -> Result<String, String> {
    text(tab, &format!("document.querySelector('{selector}').textContent"))
}

fn field(tab: &Tab, name: &str) -> Result<String, String> {
    content(tab, &format!("#ms-p-{name}"))
}

/// The number of cells visited.
fn step(tab: &Tab) -> Result<usize, String> {
    field(tab, "step")?
        .split_whitespace()
        .nth(1)
        .and_then(|n| n.parse().ok())
        .ok_or_else(|| "the step is not a number".to_owned())
}

fn live(tab: &Tab) -> Result<String, String> {
    Ok(content(tab, "#ms-live")?.replace('\u{a0}', ""))
}

fn banner_hidden(tab: &Tab) -> Result<bool, String> {
    Ok(text(tab, "String(document.getElementById('ms-error').hidden)")? == "true")
}

fn click_button(tab: &Tab, action: &str) -> Result<(), String> {
    text(
        tab,
        &format!(
            "(() => {{ document.querySelector('[data-action=\"{action}\"]').dispatchEvent(new MouseEvent('click', \
             {{ bubbles: true }})); return 'ok'; }})()"
        ),
    )
    .map(|_| ())
}

fn wait_until(tab: &Tab, seconds: u64, mut condition: impl FnMut(&Tab) -> Result<bool, String>) -> Result<(), String> {
    let start = Instant::now();
    while start.elapsed() < Duration::from_secs(seconds) {
        if condition(tab)? {
            return Ok(());
        }
        std::thread::sleep(Duration::from_millis(30));
    }
    Err("it did not happen in time".to_owned())
}

/// Moves the range to `to` as a person does who drags and lets go, and waits for the step to be shown.
fn seek(tab: &Tab, to: usize) -> Result<(), String> {
    text(
        tab,
        &format!(
            "(() => {{ const r = document.getElementById('ms-seek'); r.value = '{to}'; \
             r.dispatchEvent(new Event('input', {{ bubbles: true }})); \
             r.dispatchEvent(new Event('change', {{ bubbles: true }})); return 'ok'; }})()"
        ),
    )?;
    wait_until(tab, 5, |tab| Ok(step(tab)? == to))
}

/// Sets a selector or the number and tells the page.
fn choose(tab: &Tab, id: &str, value: &str) -> Result<(), String> {
    text(
        tab,
        &format!(
            "(() => {{ const s = document.getElementById('{id}'); s.value = '{value}'; \
             s.dispatchEvent(new Event('change', {{ bubbles: true }})); return 'ok'; }})()"
        ),
    )
    .map(|_| ())
}

fn value_of(tab: &Tab, id: &str) -> Result<String, String> {
    text(tab, &format!("document.getElementById('{id}').value"))
}

fn advance_frames(tab: &Tab, n: usize) -> Result<usize, String> {
    text(tab, &format!("String(window.__msAdvanceFrame({n}))"))?
        .parse()
        .map_err(|e| format!("{e}"))
}

fn pending_frames(tab: &Tab) -> Result<usize, String> {
    text(tab, "String(window.__msPendingFrames())")?
        .parse()
        .map_err(|e| format!("{e}"))
}

fn press(tab: &Tab, key: &str) -> Result<(), String> {
    tab.press_key(key)
        .map(|_| ())
        .map_err(|e| format!("could not press {key}: {e}"))
}

/// The digest prefix in the footer, which comes from the model and not from the view.
fn digest(tab: &Tab) -> Result<String, String> {
    field(tab, "footer")?
        .rsplit("digest ")
        .next()
        .map(str::to_owned)
        .ok_or_else(|| "no digest".to_owned())
}

fn footer_number(tab: &Tab, label: &str) -> Result<u64, String> {
    let footer = field(tab, "footer")?;
    footer
        .split(" · ")
        .find_map(|part| part.strip_prefix(label))
        .and_then(|n| n.trim().parse().ok())
        .ok_or_else(|| format!("no {label} in {footer}"))
}

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

/// Makes the browser's own `replaceChildren` throw. With `after` it throws once the node has already been changed.
fn fail_replacement(tab: &Tab, failing: bool, after: bool) -> Result<(), String> {
    let script = if failing {
        format!(
            "(() => {{ const p = Element.prototype; if (!p.__replaced) {{ p.__replaced = p.replaceChildren; }} \
             p.replaceChildren = function () {{ if ({after}) {{ p.__replaced.apply(this, arguments); }} \
             throw new Error('injected'); }}; return 'ok'; }})()"
        )
    } else {
        "(() => { const p = Element.prototype; if (p.__replaced) { p.replaceChildren = p.__replaced; } \
         return 'ok'; })()"
            .to_owned()
    };
    text(tab, &script).map(|_| ())
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
// Geometry.

/// The colours of the three levels, and of the outline of the inspected cell.
const SLOT_COLOURS: [&str; 3] = ["#1d4ed8", "#b45309", "#047857"];

/// Reads the contours as drawn and checks each vertex against the field, in the grid's own coordinates.
///
/// The plot's mapping from the grid to the scene is learned from the outline of the inspected cell `(r, c)`, whose
/// corners are whole numbers. Each vertex of each contour is then turned back into the grid's coordinates, and must be
/// on a grid edge at the place where the interpolation of the two samples gives the level. This does not use the
/// solver.
fn geometry_script(r: usize, c: usize) -> String {
    format!(
        r#"(() => {{
    const W = 25, H = 19, R = {r}, C = {c}, levels = [-0.53, 0.101, 0.52], colours = ['#1d4ed8', '#b45309', '#047857'];
    const f = (x, y) => Math.sin(1.15 * x) * Math.cos(1.35 * y) + 0.16 * x - 0.08 * y;
    const sample = (c, r) => f(-3 + 6 * c / (W - 1), 2.25 - 4.5 * r / (H - 1));
    const scene = document.querySelector('#ms-stage .ms-scene');
    const parse = p => p.getAttribute('points').trim().split(/\s+/).map(s => s.split(',').map(Number));
    const outline = [...scene.querySelectorAll('polyline')].find(p => p.getAttribute('stroke') === '#be123c');
    if (!outline) return JSON.stringify({{ error: 'no outline' }});
    const o = parse(outline);
    const ax = o[1][0] - o[0][0], ay = o[3][1] - o[0][1], top = H - 1 - R;
    const world = ([lx, ly]) => [C + (lx - o[0][0]) / ax, top - (ly - o[0][1]) / ay];
    const near = v => Math.abs(v - Math.round(v));
    const slots = colours.map((colour, slot) => {{
        const lines = [...scene.querySelectorAll('polyline')].filter(p => p.getAttribute('stroke') === colour);
        let maxEdge = 0, maxLevel = 0, maxReadback = 0, open = 0, closed = 0, boundaryOpen = 0, seams = 0, points = 0;
        for (const line of lines) {{
            const v = parse(line), isClosed = v.length > 2 && v[0][0] === v[v.length - 1][0] && v[0][1] === v[v.length - 1][1];
            if (isClosed) {{ closed++; }} else {{ open++; }}
            if (isClosed) {{
                const dot = [...scene.querySelectorAll('circle')].find(c => c.getAttribute('fill') === colour &&
                    Math.abs(+c.getAttribute('cx') - v[0][0]) < 1e-6 && Math.abs(+c.getAttribute('cy') - v[0][1]) < 1e-6);
                if (dot) seams++;
            }}
            const w = v.map(world);
            points += v.length;
            for (const [X, Y] of w) {{
                // On a vertical grid line, between two rows, or on a horizontal one, between two columns.
                let edge, level;
                if (near(X) <= near(Y)) {{
                    const c = Math.round(X), rf = H - 1 - Y, r0 = Math.min(Math.max(Math.floor(rf), 0), H - 2);
                    const a = sample(c, r0), b = sample(c, r0 + 1);
                    edge = near(X); level = Math.abs(a + (rf - r0) * (b - a) - levels[slot]) / Math.abs(b - a);
                }} else {{
                    const r = Math.round(H - 1 - Y), c0 = Math.min(Math.max(Math.floor(X), 0), W - 2);
                    const a = sample(c0, r), b = sample(c0 + 1, r);
                    edge = near(Y); level = Math.abs(a + (X - c0) * (b - a) - levels[slot]) / Math.abs(b - a);
                }}
                maxEdge = Math.max(maxEdge, edge); maxLevel = Math.max(maxLevel, level);
            }}
            // What the renderer says: the first and last points of the shape, which are floating point numbers of 32
            // bits.
            const total = line.getTotalLength(), s = line.getPointAtLength(0), e = line.getPointAtLength(total);
            maxReadback = Math.max(maxReadback, Math.hypot(s.x - v[0][0], s.y - v[0][1]) / Math.abs(ax),
                Math.hypot(e.x - v[v.length - 1][0], e.y - v[v.length - 1][1]) / Math.abs(ax));
            if (!isClosed) {{
                const onEdge = ([X, Y]) => near(X) < 1e-6 && (Math.round(X) === 0 || Math.round(X) === W - 1) ||
                    near(Y) < 1e-6 && (Math.round(Y) === 0 || Math.round(Y) === H - 1);
                if (onEdge(w[0]) && onEdge(w[w.length - 1])) boundaryOpen++;
            }}
        }}
        return {{ lines: lines.length, open, closed, boundaryOpen, seams, points, maxEdge, maxLevel, maxReadback }};
    }});
    return JSON.stringify({{ slots }});
}})()"#
    )
}

/// What the geometry check found for each level.
struct Found {
    lines: usize,
    open: usize,
    closed: usize,
    boundary_open: usize,
    seams: usize,
    max_edge: f64,
    max_level: f64,
    max_readback: f64,
}

fn geometry(tab: &Tab, cell: (usize, usize)) -> Result<Vec<Found>, String> {
    let value = json(tab, &geometry_script(cell.0, cell.1))?;
    if let Some(error) = value.get("error") {
        return Err(format!("{error}"));
    }
    let n = |v: &Value, key: &str| v[key].as_f64().unwrap_or(f64::NAN);
    Ok(value["slots"]
        .as_array()
        .ok_or("no slots")?
        .iter()
        .map(|s| Found {
            lines: n(s, "lines") as usize,
            open: n(s, "open") as usize,
            closed: n(s, "closed") as usize,
            boundary_open: n(s, "boundaryOpen") as usize,
            seams: n(s, "seams") as usize,
            max_edge: n(s, "maxEdge"),
            max_level: n(s, "maxLevel"),
            max_readback: n(s, "maxReadback"),
        })
        .collect())
}

/// Counts the polylines of one level that are drawn.
fn polylines(tab: &Tab, slot: usize) -> Result<usize, String> {
    count(tab, &format!("{STAGE} .ms-scene polyline[stroke=\"{}\"]", SLOT_COLOURS[slot]))
}

/// Chooses the inspected cell by its selectors.
fn inspect_cell(tab: &Tab, row: usize, column: usize) -> Result<(), String> {
    choose(tab, "ms-row", &row.to_string())?;
    choose(tab, "ms-column", &column.to_string())
}

/// The tolerance in grid units, from what the probe measured. The vertices are read from the `points` attribute, which
/// holds the number as text, and they were right to about 1e-13. The renderer's own readback of a shape is 32-bit
/// floating point, and it was right to about 1e-6, so the tolerance is ten times that.
const GRID_TOLERANCE: f64 = 1e-9;
const READBACK_TOLERANCE: f64 = 1e-5;
/// The tolerance on the screen, in CSS pixels.
const SCREEN_TOLERANCE: f64 = 0.5;
/// The contours of the default field at its last step: open and closed at each level.
const DEFAULT_CONTOURS: [(usize, usize); 3] = [(4, 1), (3, 0), (4, 0)];

fn check_geometry(tab: &Tab, cell: (usize, usize), when: &str) -> Result<(), String> {
    for (slot, found) in geometry(tab, cell)?.iter().enumerate() {
        let (open, closed) = DEFAULT_CONTOURS[slot];
        check_eq((found.open, found.closed, found.lines), (open, closed, open + closed))
            .map_err(|e| format!("{when}: level {slot}: {e}"))?;
        check_eq(found.boundary_open, found.open)
            .map_err(|e| format!("{when}: level {slot}: an open end was clipped: {e}"))?;
        check_eq(found.seams, found.closed)
            .map_err(|e| format!("{when}: level {slot}: a loop has no dot at its seam: {e}"))?;
        check(
            found.max_edge < GRID_TOLERANCE,
            &format!("{when}: level {slot}: a vertex is {:.3e} off a grid line", found.max_edge),
        )?;
        check(
            found.max_level < GRID_TOLERANCE,
            &format!("{when}: level {slot}: a vertex is {:.3e} off the level", found.max_level),
        )?;
        check(
            found.max_readback < READBACK_TOLERANCE,
            &format!(
                "{when}: level {slot}: the renderer is {:.3e} grid units off",
                found.max_readback
            ),
        )?;
    }
    Ok(())
}

/// How far the painted box of each contour is from the box of its vertices, through the current transform, in pixels.
const SCREEN_SCRIPT: &str = r#"(() => {
    const scene = document.querySelector('#ms-stage .ms-scene');
    const colours = ['#1d4ed8', '#b45309', '#047857'];
    let worst = 0, lines = 0;
    for (const p of scene.querySelectorAll('polyline')) {
        if (!colours.includes(p.getAttribute('stroke'))) continue;
        const m = p.getScreenCTM();
        const v = p.getAttribute('points').trim().split(/\s+/).map(s => s.split(',').map(Number))
            .map(([x, y]) => [m.a * x + m.c * y + m.e, m.b * x + m.d * y + m.f]);
        const xs = v.map(q => q[0]), ys = v.map(q => q[1]), r = p.getBoundingClientRect();
        worst = Math.max(worst, Math.abs(r.left - Math.min(...xs)), Math.abs(r.right - Math.max(...xs)),
            Math.abs(r.top - Math.min(...ys)), Math.abs(r.bottom - Math.max(...ys)));
        lines++;
    }
    return JSON.stringify({ worst, lines });
})()"#;

fn screen_deviation(tab: &Tab) -> Result<f64, String> {
    json(tab, SCREEN_SCRIPT)?["worst"]
        .as_f64()
        .ok_or_else(|| "no deviation".to_owned())
}

/// The percentage in the label of the scene's view, which the library keeps right.
fn zoom_percent(tab: &Tab) -> Result<u32, String> {
    text(
        tab,
        "document.querySelector('#ms-stage [aria-label^=\"Graph view\"]').getAttribute('aria-label')",
    )?
    .rsplit("zoom ")
    .next()
    .and_then(|p| p.trim_end_matches('%').parse().ok())
    .ok_or_else(|| "no zoom in the label".to_owned())
}

/// Clicks the centre of an element with the real mouse.
fn click_element(tab: &Tab, selector: &str) -> Result<(), String> {
    let centre = json(
        tab,
        &format!(
            "(() => {{ const e = document.querySelector('{selector}'); e.scrollIntoView({{ block: 'center' }}); \
             const r = e.getBoundingClientRect(); return JSON.stringify([r.left + r.width / 2, r.top + r.height / 2]); }})()"
        ),
    )?;
    let (x, y) = (centre[0].as_f64().unwrap_or(0.0), centre[1].as_f64().unwrap_or(0.0));
    tab.click_point(headless_chrome::browser::tab::point::Point { x, y })
        .map(|_| ())
        .map_err(|e| format!("could not click {selector}: {e}"))
}

/// Zooms with the toolbar until the label says `target` per cent. The zoom goes by steps, so 100 is reached with the
/// reset button, and the limits of 25 and 400 by pressing until they stop it.
fn zoom_to(tab: &Tab, target: u32) -> Result<(), String> {
    if target == 100 {
        click_element(tab, &format!("{STAGE} .ms-scene [aria-label=\"Reset zoom\"]"))?;
        return wait_until(tab, 3, |t| Ok(zoom_percent(t)? == 100));
    }
    for _ in 0..40 {
        let now = zoom_percent(tab)?;
        if now == target || (target >= 400 && now >= 400) || (target <= 25 && now <= 25) {
            return Ok(());
        }
        let button = if now < target { "Zoom in" } else { "Zoom out" };
        click_element(tab, &format!("{STAGE} .ms-scene [aria-label=\"{button}\"]"))?;
        wait_until(tab, 2, |t| Ok(zoom_percent(t)? != now))?;
    }
    Err(format!("did not reach {target}%"))
}

/// Pans the view with the keyboard: the scene adds a target for it, and the arrow keys move the view.
fn pan(tab: &Tab, key: &str, times: usize) -> Result<(), String> {
    text(
        tab,
        "(() => { document.querySelector('#ms-stage [aria-label^=\"Graph view\"]').focus(); return 'ok'; })()",
    )?;
    for _ in 0..times {
        press(tab, key)?;
    }
    Ok(())
}

/// The transform of the content of the scene, which is the view.
fn view_transform(tab: &Tab) -> Result<String, String> {
    text(
        tab,
        "String(document.querySelector('#ms-stage .ms-scene .svg-dom-graph-content').getAttribute('transform'))",
    )
}

/// Everything about the contours that the page draws, as one string, to compare two scenes.
fn drawn_contours(tab: &Tab) -> Result<String, String> {
    text(
        tab,
        r#"(() => JSON.stringify([...document.querySelectorAll('#ms-stage .ms-scene polyline')].map(p =>
            [p.getAttribute('stroke'), p.getAttribute('stroke-dasharray'), p.getAttribute('stroke-width'), p.getAttribute('points')])))()"#,
    )
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
// The page.

#[test]
fn the_page_opens_at_the_last_step_of_the_ripple_field_with_three_levels() -> Result<(), String> {
    let tab = open()?;
    check_eq((step(&tab)?, field(&tab, "step")?), (CELLS, format!("Step {CELLS} of {CELLS}")))?;
    check_eq(value_of(&tab, "ms-field")?, "ripple".to_owned())?;
    check_eq(value_of(&tab, "ms-threshold")?, "0.101".to_owned())?;
    check(banner_hidden(&tab)?, "no error")?;
    check(
        field(&tab, "levels")?.contains("low -0.530: 4 open, 1 closed"),
        &field(&tab, "levels")?,
    )?;
    check(
        field(&tab, "levels")?.contains("high +0.520: 4 open, 0 closed"),
        &field(&tab, "levels")?,
    )?;
    check(
        field(&tab, "legend")?.contains("The contour set is complete."),
        &field(&tab, "legend")?,
    )?;
    check_eq(count(&tab, "#ms-staging svg > *")?, 0)?;
    check_eq(count(&tab, &format!("{STAGE} .ms-scene"))?, 1)
}

#[test]
fn every_button_the_range_and_the_selectors_are_in_the_stage_svg_and_outside_the_swapped_part() -> Result<(), String> {
    let tab = open()?;
    check_eq(count(&tab, "#panel-marching-squares button")?, 0)?;
    check_eq(count(&tab, "#ms-stage-svg g.ms-controls > [role=button][data-action]")?, 5)?;
    check_eq(count(&tab, "#ms-stage-svg foreignObject select")?, 5)?;
    check_eq(
        count(&tab, "#ms-stage-svg foreignObject input[type=\"number\"]#ms-threshold")?,
        1,
    )?;
    check_eq(count(&tab, "#ms-stage-svg foreignObject input[type=\"range\"]#ms-seek")?, 1)?;
    check_eq(count(&tab, ".ms-scene-host [data-action], .ms-scene-host foreignObject")?, 0)?;
    // The controls are above the plot.
    let above = text(
        &tab,
        "(() => { const b = document.querySelector('[data-action=\"next\"]').getBoundingClientRect(); \
         const t = document.querySelector('#ms-stage .ms-scene').getBoundingClientRect(); return String(b.bottom <= t.top); })()",
    )?;
    check_eq(above, "true".to_owned())?;
    // Every selector has a name that a person can hear.
    check_eq(count(&tab, "#ms-stage-svg foreignObject select:not([aria-label])")?, 0)
}

#[test]
fn the_plot_names_its_levels_by_pattern_and_describes_the_state() -> Result<(), String> {
    let tab = open()?;
    let description = text(
        &tab,
        "document.querySelector('#ms-stage .ms-scene [role=img], #ms-stage .ms-scene [aria-label^=\"Marching\"]').getAttribute('aria-label')",
    )?;
    check(
        description.contains("Ripple plane") && description.contains("low -0.530 solid"),
        &description,
    )?;
    check(
        description.contains("middle +0.101 dashed") && description.contains("high +0.520 dotted"),
        &description,
    )?;
    check(description.contains("432 of 432 cells visited"), &description)?;
    // The legend has three entries, each with its text, and the lines are told apart by dash as well as colour.
    let legend = text(&tab, "document.querySelector('#ms-stage .ms-scene').textContent")?;
    check(
        legend.contains("low, solid") && legend.contains("middle, dashed") && legend.contains("high, dotted"),
        &legend,
    )?;
    let dashes = text(
        &tab,
        "JSON.stringify(['#1d4ed8','#b45309','#047857'].map(c => document.querySelector('#ms-stage .ms-scene polyline[stroke=\"' + c + '\"]').getAttribute('stroke-dasharray')))",
    )?;
    check_eq(dashes, "[null,\"9 5\",\"2 5\"]".to_owned())
}

#[test]
fn the_text_of_every_option_fits_inside_its_selector() -> Result<(), String> {
    let tab = open()?;
    let found = json(
        &tab,
        r#"(() => { const c = document.createElement('canvas').getContext('2d'); const out = [];
            for (const s of document.querySelectorAll('#ms-stage-svg foreignObject select, #ms-stage-svg foreignObject input')) {
                const style = getComputedStyle(s); c.font = style.fontSize + ' ' + style.fontFamily;
                const room = s.getBoundingClientRect().width - parseFloat(style.paddingLeft) - parseFloat(style.paddingRight) - 12;
                const texts = s.tagName === 'SELECT' ? [...s.options].map(o => o.textContent) : [s.value];
                for (const t of texts) out.push([s.id, t, c.measureText(t).width, room]);
            } return JSON.stringify(out); })()"#,
    )?;
    let rows = found.as_array().ok_or("no rows")?;
    check(rows.len() > 40, "every option was measured")?;
    for row in rows {
        let (id, shown, width, room) = (
            &row[0],
            &row[1],
            row[2].as_f64().unwrap_or(f64::MAX),
            row[3].as_f64().unwrap_or(0.0),
        );
        check(
            width <= room,
            &format!("{id}: {shown} is {width:.1} wide and the room is {room:.1}"),
        )?;
    }
    Ok(())
}

#[test]
fn the_name_of_the_range_is_clear_of_the_buttons_above_it_and_of_the_plot_below() -> Result<(), String> {
    let tab = open()?;
    let gaps = json(
        &tab,
        r#"(() => { const label = document.querySelector('#ms-stage-svg .scene-slider-label').getBoundingClientRect();
            const buttons = [...document.querySelectorAll('#ms-stage-svg [data-action]')].map(b => b.getBoundingClientRect().bottom);
            const plot = document.querySelector('#ms-stage .ms-scene').getBoundingClientRect().top;
            const range = document.getElementById('ms-seek').getBoundingClientRect();
            return JSON.stringify([label.top - Math.max(...buttons), plot - range.bottom, label.left]); })()"#,
    )?;
    let (above, below) = (gaps[0].as_f64().unwrap_or(-1.0), gaps[1].as_f64().unwrap_or(-1.0));
    check(above >= 6.0, &format!("the name is {above:.1} px below the buttons"))?;
    check(below >= 2.0, &format!("the range is {below:.1} px above the plot"))
}

// The geometry.

#[test]
fn the_drawn_contours_are_right_in_the_grids_own_coordinates_and_the_open_ends_survive_clipping() -> Result<(), String>
{
    let tab = open()?;
    inspect_cell(&tab, 14, 12)?;
    check_geometry(&tab, (14, 12), "at 1x")
}

#[test]
fn the_geometry_is_unchanged_by_zoom_and_pan_and_the_model_digest_does_not_move() -> Result<(), String> {
    let tab = open()?;
    inspect_cell(&tab, 14, 12)?;
    let before = (digest(&tab)?, drawn_contours(&tab)?);
    for target in [25, 400, 100] {
        zoom_to(&tab, target)?;
        check_eq(zoom_percent(&tab)?, target)?;
        check_geometry(&tab, (14, 12), &format!("at {target}%"))?;
        check(
            screen_deviation(&tab)? <= SCREEN_TOLERANCE,
            &format!("at {target}%: {}", screen_deviation(&tab)?),
        )?;
        check_eq((digest(&tab)?, drawn_contours(&tab)?), before.clone()).map_err(|e| format!("at {target}%: {e}"))?;
    }
    zoom_to(&tab, 400)?;
    let at_four = view_transform(&tab)?;
    pan(&tab, "ArrowRight", 3)?;
    pan(&tab, "ArrowDown", 2)?;
    check(view_transform(&tab)? != at_four, "the view moved")?;
    check_geometry(&tab, (14, 12), "after a pan at 400%")?;
    check(screen_deviation(&tab)? <= SCREEN_TOLERANCE, "after a pan")?;
    check_eq((digest(&tab)?, drawn_contours(&tab)?), before)
}

#[test]
fn the_controls_stay_one_size_at_every_zoom_and_the_footer_follows_the_zoom() -> Result<(), String> {
    let tab = open()?;
    let size = |tab: &Tab| {
        text(
            tab,
            "(() => { const r = document.querySelector('[data-action=\"next\"] rect').getBoundingClientRect(); \
             const s = document.getElementById('ms-seek').getBoundingClientRect(); \
             return [r.width, r.height, s.width].map(v => v.toFixed(2)).join(' '); })()",
        )
    };
    let before = size(&tab)?;
    for target in [25, 400] {
        zoom_to(&tab, target)?;
        check_eq(size(&tab)?, before.clone()).map_err(|e| format!("at {target}%: {e}"))?;
    }
    wait_until(&tab, 3, |t| Ok(field(t, "zoom")? == "400%"))?;
    zoom_to(&tab, 100)?;
    wait_until(&tab, 3, |t| Ok(field(t, "zoom")? == "100%"))
}

#[test]
fn a_loop_has_a_dot_at_its_seam_as_wide_as_the_line() -> Result<(), String> {
    let tab = open()?;
    choose(&tab, "ms-field", "twin")?;
    inspect_cell(&tab, 9, 12)?;
    let dots = text(
        &tab,
        r#"(() => { const scene = document.querySelector('#ms-stage .ms-scene');
            const loops = [...scene.querySelectorAll('polyline')].filter(p => ['#1d4ed8','#b45309','#047857'].includes(p.getAttribute('stroke')))
                .filter(p => { const v = p.getAttribute('points').trim().split(/\s+/); return v.length > 2 && v[0] === v[v.length - 1]; });
            return JSON.stringify(loops.map(p => { const first = p.getAttribute('points').trim().split(/\s+/)[0].split(',').map(Number);
                const d = [...scene.querySelectorAll('circle')].find(c => c.getAttribute('fill') === p.getAttribute('stroke') &&
                    Math.abs(+c.getAttribute('cx') - first[0]) < 1e-6 && Math.abs(+c.getAttribute('cy') - first[1]) < 1e-6);
                return d ? +d.getAttribute('r') : null; })); })()"#,
    )?;
    let radii: Vec<Option<f64>> = serde_json::from_str(&dots).map_err(|e| e.to_string())?;
    check(!radii.is_empty(), "the twin hills have loops")?;
    check(radii.iter().all(|r| *r == Some(1.25)), &format!("{radii:?}"))
}

// Stepping.

#[test]
fn restart_shows_the_samples_and_no_contour_and_next_visits_one_cell_at_a_time() -> Result<(), String> {
    let tab = open()?;
    click_button(&tab, "restart")?;
    check_eq(step(&tab)?, 0)?;
    for slot in 0..3 {
        check_eq(polylines(&tab, slot)?, 0)?;
    }
    check(
        count(&tab, &format!("{STAGE} .ms-scene circle"))? >= 475,
        "the samples are drawn",
    )?;
    check(field(&tab, "status")?.starts_with("No cell selected."), &field(&tab, "status")?)?;
    check(live(&tab)?.starts_with("No cell selected."), &live(&tab)?)?;
    click_button(&tab, "next")?;
    check_eq(step(&tab)?, 1)?;
    check(field(&tab, "status")?.starts_with("Cell (0, 0)"), &field(&tab, "status")?)?;
    check(field(&tab, "legend")?.contains("partial"), &field(&tab, "legend")?)
}

#[test]
fn the_ends_are_bounded_and_say_so_and_a_dimmed_button_still_takes_a_press() -> Result<(), String> {
    let tab = open()?;
    click_button(&tab, "next")?;
    check_eq(live(&tab)?, "Already at the last step.".to_owned())?;
    check_eq(attribute(&tab, "[data-action=\"next\"]", "aria-disabled")?, "true".to_owned())?;
    click_button(&tab, "restart")?;
    click_button(&tab, "previous")?;
    check_eq(live(&tab)?, "Already at the first step.".to_owned())?;
    check_eq(
        attribute(&tab, "[data-action=\"restart\"]", "aria-disabled")?,
        "true".to_owned(),
    )
}

#[test]
fn forward_backward_a_direct_seek_and_a_rebuild_draw_the_same_step() -> Result<(), String> {
    let tab = open()?;
    let mut forward = Vec::new();
    click_button(&tab, "restart")?;
    for k in 0..=CELLS {
        if k > 0 {
            click_button(&tab, "next")?;
        }
        if k % 24 == 0 || k == CELLS {
            forward.push((k, digest(&tab)?, drawn_contours(&tab)?));
        }
    }
    for (k, digest_k, contours_k) in forward.iter().rev() {
        seek(&tab, *k)?;
        check_eq((digest(&tab)?, drawn_contours(&tab)?), (digest_k.clone(), contours_k.clone()))
            .map_err(|e| format!("backward to {k}: {e}"))?;
    }
    for k in [192, 24, 408, 0, 96] {
        seek(&tab, k)?;
        let want = forward.iter().find(|(at, ..)| *at == k).ok_or("a step that was recorded")?;
        check_eq((digest(&tab)?, drawn_contours(&tab)?), (want.1.clone(), want.2.clone()))
            .map_err(|e| format!("direct to {k}: {e}"))?;
        click_button(&tab, "rebuild")?;
        check_eq((digest(&tab)?, drawn_contours(&tab)?), (want.1.clone(), want.2.clone()))
            .map_err(|e| format!("rebuilt at {k}: {e}"))?;
    }
    Ok(())
}

#[test]
fn a_partial_frame_has_no_loop_closed_early_and_no_join_across_unvisited_cells() -> Result<(), String> {
    let tab = open()?;
    // The low level has one closed loop. Before the last of its cells has been visited, it is not closed.
    let mut first_closed = None;
    for k in (0..=CELLS).step_by(8) {
        seek(&tab, k)?;
        let closed = text(
            &tab,
            r##"String([...document.querySelectorAll('#ms-stage .ms-scene polyline[stroke="#1d4ed8"]')].filter(p => {
                const v = p.getAttribute('points').trim().split(/\s+/); return v.length > 2 && v[0] === v[v.length - 1]; }).length)"##,
        )?;
        if closed != "0" && first_closed.is_none() {
            first_closed = Some(k);
        }
        if first_closed.is_some() {
            check_eq(closed.as_str(), "1")?;
        }
    }
    let first = first_closed.ok_or("the loop closes")?;
    check(first > 8 && first < CELLS, &format!("closed at {first}"))?;
    // At the end of the field the result is whole, and the status says so.
    check(field(&tab, "status")?.contains("Complete."), &field(&tab, "status")?)
}

// The inspector.

#[test]
fn an_unvisited_cell_reveals_nothing_and_a_visited_one_reads_its_case() -> Result<(), String> {
    let tab = open()?;
    seek(&tab, 50)?;
    inspect_cell(&tab, 14, 12)?;
    check(field(&tab, "status")?.contains("Not yet evaluated"), &field(&tab, "status")?)?;
    for hidden in ["mask", "Q ", "rho"] {
        check(
            !field(&tab, "status")?.contains(hidden),
            &format!("{hidden}: {}", field(&tab, "status")?),
        )?;
    }
    check(
        content(&tab, "#ms-inspector")?.contains("Not yet evaluated"),
        "the picture says so too",
    )?;
    seek(&tab, 14 * 24 + 13)?;
    let status = field(&tab, "status")?;
    check(
        status.starts_with("Cell (14, 12), middle level +0.101: mask 10; Q negative; rho 0.425; T–L and R–B"),
        &status,
    )?;
    check(
        content(&tab, "#ms-inspector")?.contains("mask 10"),
        &content(&tab, "#ms-inspector")?,
    )?;
    check_eq(count(&tab, "#ms-inspector line")?, 2)
}

#[test]
fn clicking_the_plot_chooses_the_cell_through_the_zoom() -> Result<(), String> {
    let tab = open()?;
    inspect_cell(&tab, 14, 12)?;
    // Where the outline of the cell is on the screen, now and at 400%.
    let centre_of_outline = |tab: &Tab| -> Result<(f64, f64), String> {
        let v = json(
            tab,
            "(() => { const r = [...document.querySelectorAll('#ms-stage .ms-scene polyline')].find(p => p.getAttribute('stroke') === '#be123c').getBoundingClientRect(); \
             return JSON.stringify([r.left + r.width / 2, r.top + r.height / 2]); })()",
        )?;
        Ok((v[0].as_f64().unwrap_or(0.0), v[1].as_f64().unwrap_or(0.0)))
    };
    // Choose another cell by a click at the centre of the cell (4, 1), whose screen place is the outline's place moved
    // by the cell size, found from the sample markers.
    let (x, y) = centre_of_outline(&tab)?;
    let step_px = json(
        &tab,
        "(() => { const o = [...document.querySelectorAll('#ms-stage .ms-scene polyline')].find(p => p.getAttribute('stroke') === '#be123c').getBoundingClientRect(); \
         return JSON.stringify([o.width, o.height]); })()",
    )?;
    let (cw, ch) = (step_px[0].as_f64().unwrap_or(1.0), step_px[1].as_f64().unwrap_or(1.0));
    let (tx, ty) = (x - 11.0 * cw, y - 10.0 * ch);
    tab.click_point(headless_chrome::browser::tab::point::Point { x: tx, y: ty })
        .map_err(|e| e.to_string())?;
    wait_until(&tab, 3, |t| Ok(value_of(t, "ms-row")? == "4"))?;
    check_eq(
        (value_of(&tab, "ms-row")?, value_of(&tab, "ms-column")?),
        ("4".to_owned(), "1".to_owned()),
    )?;
    check(field(&tab, "status")?.starts_with("Cell (4, 1)"), &field(&tab, "status")?)?;
    // At 400% only the middle of the plot is on the screen, so use a cell there, and click its neighbour on the left.
    zoom_to(&tab, 400)?;
    inspect_cell(&tab, 9, 12)?;
    let (x, y) = centre_of_outline(&tab)?;
    let (cw, _) = {
        let v = json(
            &tab,
            "(() => { const o = [...document.querySelectorAll('#ms-stage .ms-scene polyline')].find(p => p.getAttribute('stroke') === '#be123c').getBoundingClientRect(); return JSON.stringify([o.width, o.height]); })()",
        )?;
        (v[0].as_f64().unwrap_or(1.0), v[1].as_f64().unwrap_or(1.0))
    };
    check(cw > 80.0, &format!("a cell is {cw} px wide at 400%"))?;
    tab.click_point(headless_chrome::browser::tab::point::Point { x: x - cw, y })
        .map_err(|e| e.to_string())?;
    wait_until(&tab, 3, |t| Ok(value_of(t, "ms-column")? == "11"))?;
    check_eq(
        (value_of(&tab, "ms-row")?, value_of(&tab, "ms-column")?),
        ("9".to_owned(), "11".to_owned()),
    )?;
    Ok(())
}

#[test]
fn the_row_and_column_selectors_offer_only_the_cells_of_the_field() -> Result<(), String> {
    let tab = open()?;
    let enabled = |tab: &Tab, id: &str| count(tab, &format!("#{id} option:not([disabled])"));
    check_eq((enabled(&tab, "ms-row")?, enabled(&tab, "ms-column")?), (1 + 18, 1 + 24))?;
    choose(&tab, "ms-field", "coarse")?;
    check_eq((enabled(&tab, "ms-row")?, enabled(&tab, "ms-column")?), (1 + 6, 1 + 8))?;
    choose(&tab, "ms-field", "lab-tie")?;
    check_eq((enabled(&tab, "ms-row")?, enabled(&tab, "ms-column")?), (2, 2))
}

#[test]
fn the_inspected_level_sets_the_numbers_and_the_visible_levels_are_independent_of_it() -> Result<(), String> {
    let tab = open()?;
    inspect_cell(&tab, 14, 12)?;
    choose(&tab, "ms-inspect", "2")?;
    check(field(&tab, "status")?.contains("high level +0.520"), &field(&tab, "status")?)?;
    check_eq(value_of(&tab, "ms-threshold")?, "0.52".to_owned())?;
    check_eq((polylines(&tab, 0)?, polylines(&tab, 1)?, polylines(&tab, 2)?), (5, 3, 4))?;
    choose(&tab, "ms-show", "middle")?;
    check_eq((polylines(&tab, 0)?, polylines(&tab, 1)?, polylines(&tab, 2)?), (0, 3, 0))?;
    check(
        field(&tab, "status")?.contains("high level +0.520"),
        "the numbers stay at the inspected level",
    )?;
    check(
        field(&tab, "levels")?.contains("low -0.530: 4 open, 1 closed"),
        "the totals cover all three",
    )
}

// The saddles.

#[test]
fn the_saddle_lab_reads_each_named_cell_with_the_same_solver() -> Result<(), String> {
    let tab = open()?;
    for (key, text_in_status) in [
        ("lab-5-positive", "mask 5; Q positive"),
        ("lab-5-negative", "mask 5; Q negative"),
        ("lab-10-positive", "mask 10; Q positive"),
        ("lab-10-negative", "mask 10; Q negative"),
        ("lab-tie", "mask 5; Q exactly zero"),
    ] {
        choose(&tab, "ms-field", key)?;
        check_eq(step(&tab)?, 1)?;
        choose(&tab, "ms-inspect", "1")?;
        let status = field(&tab, "status")?;
        check(status.contains(text_in_status), &format!("{key}: {status}"))?;
    }
    check(field(&tab, "status")?.contains("1 saddle ties"), &field(&tab, "status")?)?;
    // The tie is two strands, and the picture shows the saddle.
    check_eq(count(&tab, "#ms-inspector line")?, 4)
}

#[test]
fn the_ripple_cells_with_a_decider_are_outlined_once_visited_and_the_decision_is_in_the_status() -> Result<(), String> {
    let tab = open()?;
    let outlines = |tab: &Tab| count(tab, &format!("{STAGE} .ms-scene polyline[stroke=\"#7c3aed\"]"));
    check_eq(outlines(&tab)?, 3)?;
    seek(&tab, 13 * 24 + 22)?;
    check_eq(outlines(&tab)?, 1)?;
    seek(&tab, 13 * 24 + 23)?;
    check_eq(outlines(&tab)?, 2)?;
    inspect_cell(&tab, 13, 22)?;
    choose(&tab, "ms-inspect", "2")?;
    check(
        field(&tab, "status")?
            .starts_with("Cell (13, 22), high level +0.520: mask 5; Q positive; rho 0.416; T–R and B–L"),
        &field(&tab, "status")?,
    )
}

// Thresholds and fields.

#[test]
fn a_threshold_edit_changes_the_topology_and_editing_it_back_restores_the_digest() -> Result<(), String> {
    let tab = open()?;
    inspect_cell(&tab, 14, 12)?;
    let original = (digest(&tab)?, drawn_contours(&tab)?);
    check(
        field(&tab, "status")?.contains("Q negative") && field(&tab, "status")?.contains("T–L and R–B"),
        &field(&tab, "status")?,
    )?;
    choose(&tab, "ms-threshold", "0.102")?;
    check(
        field(&tab, "status")?.contains("Q positive") && field(&tab, "status")?.contains("T–R and B–L"),
        &field(&tab, "status")?,
    )?;
    check(field(&tab, "status")?.contains("middle level +0.102"), &field(&tab, "status")?)?;
    check(digest(&tab)? != original.0, "the topology changed")?;
    check(drawn_contours(&tab)? != original.1, "and so did the picture")?;
    zoom_to(&tab, 400)?;
    pan(&tab, "ArrowRight", 2)?;
    choose(&tab, "ms-threshold", "0.101")?;
    check_eq((digest(&tab)?, drawn_contours(&tab)?), original)?;
    check_eq(zoom_percent(&tab)?, 400)
}

#[test]
fn a_threshold_that_is_refused_leaves_everything_and_the_number_goes_back() -> Result<(), String> {
    let tab = open()?;
    inspect_cell(&tab, 14, 12)?;
    let committed = (digest(&tab)?, drawn_contours(&tab)?, step(&tab)?);
    // Not above the level below, not below the level above, not a number, and exactly at a sample.
    for (bad, why) in [
        ("-0.9", "strictly ascending"),
        ("0.6", "strictly ascending"),
        ("", "not finite"),
        ("0.1", "exactly at the middle level"),
    ] {
        choose(&tab, "ms-threshold", bad)?;
        check(!banner_hidden(&tab)?, &format!("{bad:?} is reported"))?;
        check(
            content(&tab, "#ms-error")?.contains(why) || content(&tab, "#ms-error")?.contains("finite"),
            &format!("{bad:?}: {}", content(&tab, "#ms-error")?),
        )?;
        check_eq(value_of(&tab, "ms-threshold")?, "0.101".to_owned()).map_err(|e| format!("{bad:?}: {e}"))?;
        check_eq((digest(&tab)?, drawn_contours(&tab)?, step(&tab)?), committed.clone())?;
    }
    choose(&tab, "ms-threshold", "0.102")?;
    check(banner_hidden(&tab)?, "an edit that works clears the line")
}

#[test]
fn a_new_field_is_shown_at_its_last_step_and_the_view_is_kept_only_between_matching_axes() -> Result<(), String> {
    let tab = open()?;
    zoom_to(&tab, 400)?;
    let zoomed = zoom_percent(&tab)?;
    choose(&tab, "ms-field", "twin")?;
    check_eq((step(&tab)?, zoom_percent(&tab)?), (CELLS, zoomed))?;
    check(field(&tab, "footer")?.starts_with("Twin hills 25×19"), &field(&tab, "footer")?)?;
    check_eq(value_of(&tab, "ms-threshold")?, "0.24".to_owned())?;
    choose(&tab, "ms-field", "coarse")?;
    check_eq((step(&tab)?, zoom_percent(&tab)?), (48, 100))?;
    check(
        field(&tab, "footer")?.starts_with("Coarse saddle waves 9×7"),
        &field(&tab, "footer")?,
    )?;
    choose(&tab, "ms-field", "lab-tie")?;
    check_eq((step(&tab)?, zoom_percent(&tab)?), (1, 100))?;
    check_eq(value_of(&tab, "ms-field")?, "lab-tie".to_owned())
}

#[test]
fn the_coarse_preset_shows_its_saddles_at_one_to_one() -> Result<(), String> {
    let tab = open()?;
    choose(&tab, "ms-field", "coarse")?;
    // 6, 6 and 2 ambiguous cells at the three levels, and a cell that is ambiguous at more than one level is outlined
    // once.
    let outlines = count(&tab, &format!("{STAGE} .ms-scene polyline[stroke=\"#7c3aed\"]"))?;
    check((6..=14).contains(&outlines), &format!("{outlines} outlines"))?;
    let sizes = json(
        &tab,
        r##"(() => JSON.stringify([...document.querySelectorAll('#ms-stage .ms-scene polyline[stroke="#7c3aed"]')].map(p => {
            const r = p.getBoundingClientRect(); return Math.max(r.width, r.height); })))()"##,
    )?;
    let smallest = sizes
        .as_array()
        .and_then(|a| a.iter().filter_map(Value::as_f64).reduce(f64::min))
        .unwrap_or(0.0);
    check(smallest > 60.0, &format!("a cell is {smallest} px across at 100%"))?;
    choose(&tab, "ms-inspect", "0")?;
    inspect_cell(&tab, 1, 1)?;
    check(field(&tab, "status")?.starts_with("Cell (1, 1)"), &field(&tab, "status")?)
}

// Rebuilding.

#[test]
fn a_scene_rebuilt_twenty_five_times_at_a_zoomed_and_panned_view_is_the_same() -> Result<(), String> {
    let tab = open()?;
    inspect_cell(&tab, 14, 12)?;
    zoom_to(&tab, 400)?;
    pan(&tab, "ArrowLeft", 2)?;
    pan(&tab, "ArrowUp", 1)?;
    let view = view_transform(&tab)?;
    let first = (digest(&tab)?, drawn_contours(&tab)?, field(&tab, "status")?);
    text(
        &tab,
        "(() => { window.__nodes = [...document.querySelectorAll('#ms-stage-svg g.ms-controls > *')]; return 'ok'; })()",
    )?;
    let mut ids = std::collections::BTreeSet::new();
    let mut generations = Vec::new();
    for round in 0..25 {
        click_button(&tab, "rebuild")?;
        ids.insert(attribute(&tab, &format!("{STAGE} .ms-scene"), "id")?);
        generations.push(footer_number(&tab, "generation")?);
        check_eq((digest(&tab)?, drawn_contours(&tab)?, field(&tab, "status")?), first.clone())
            .map_err(|e| format!("round {round}: {e}"))?;
        check_eq(view_transform(&tab)?, view.clone()).map_err(|e| format!("round {round}: the view: {e}"))?;
        check_eq(zoom_percent(&tab)?, 400)?;
        check_eq(
            (count(&tab, &format!("{STAGE} .ms-scene"))?, count(&tab, "#ms-staging svg > *")?),
            (1, 0),
        )?;
    }
    check_eq(ids.len(), 25)?;
    check(generations.windows(2).all(|w| w[1] == w[0] + 1), &format!("{generations:?}"))?;
    check_geometry(&tab, (14, 12), "after 25 rebuilds")?;
    // The controls are the very same elements, and the page's own IDs are unique.
    check_eq(
        text(
            &tab,
            "String([...document.querySelectorAll('#ms-stage-svg g.ms-controls > *')].every((n, i) => n === window.__nodes[i]))",
        )?,
        "true".to_owned(),
    )?;
    let duplicates = text(
        &tab,
        "(() => { const seen = new Set(); let d = 0; for (const e of document.querySelectorAll('[id]')) { if (seen.has(e.id)) d++; seen.add(e.id); } return String(d); })()",
    )?;
    check_eq(duplicates, "0".to_owned())?;
    check_eq(count(&tab, &format!("{STAGE} .ms-scene"))?, 1)
}

#[test]
fn a_rebuild_keeps_the_step_the_cell_the_level_mode_and_says_nothing_new() -> Result<(), String> {
    let tab = open()?;
    seek(&tab, 300)?;
    inspect_cell(&tab, 14, 12)?;
    choose(&tab, "ms-show", "high")?;
    let said = live(&tab)?;
    for _ in 0..5 {
        click_button(&tab, "rebuild")?;
    }
    check_eq(
        (step(&tab)?, value_of(&tab, "ms-row")?, value_of(&tab, "ms-show")?),
        (300, "14".to_owned(), "high".to_owned()),
    )?;
    check_eq(live(&tab)?, said)?;
    check_eq((polylines(&tab, 0)?, polylines(&tab, 1)?), (0, 0))
}

#[test]
fn one_click_on_the_plot_after_many_rebuilds_is_heard_once() -> Result<(), String> {
    let tab = open()?;
    for _ in 0..10 {
        click_button(&tab, "rebuild")?;
    }
    text(
        &tab,
        "(() => { window.__heard = 0; new MutationObserver(m => { window.__heard += m.length; }).observe(document.getElementById('ms-live'), { childList: true, characterData: true, subtree: true }); return 'ok'; })()",
    )?;
    inspect_cell(&tab, 14, 12)?;
    let before = text(&tab, "String(window.__heard)")?
        .parse::<usize>()
        .map_err(|e| e.to_string())?;
    let v = json(
        &tab,
        "(() => { const o = [...document.querySelectorAll('#ms-stage .ms-scene polyline')].find(p => p.getAttribute('stroke') === '#be123c').getBoundingClientRect(); return JSON.stringify([o.left + o.width / 2 - 3 * o.width, o.top + o.height / 2]); })()",
    )?;
    tab.click_point(headless_chrome::browser::tab::point::Point {
        x: v[0].as_f64().unwrap_or(0.0),
        y: v[1].as_f64().unwrap_or(0.0),
    })
    .map_err(|e| e.to_string())?;
    wait_until(&tab, 3, |t| Ok(value_of(t, "ms-column")? == "9"))?;
    std::thread::sleep(Duration::from_millis(200));
    let after = text(&tab, "String(window.__heard)")?
        .parse::<usize>()
        .map_err(|e| e.to_string())?;
    check(
        after > before && after - before <= 3,
        &format!("{} mutations for one click", after - before),
    )
}

#[test]
fn the_focus_in_the_scene_survives_a_rebuild_by_its_name() -> Result<(), String> {
    let tab = open()?;
    text(
        &tab,
        &format!(
            "(() => {{ document.querySelector('{STAGE} .ms-scene [aria-label=\"Zoom in\"]').focus(); return 'ok'; }})()"
        ),
    )?;
    click_button(&tab, "rebuild")?;
    check_eq(
        text(
            &tab,
            "document.activeElement && document.activeElement.getAttribute && String(document.activeElement.getAttribute('aria-label'))",
        )?,
        "Zoom in".to_owned(),
    )?;
    check_eq(count(&tab, &format!("{STAGE} .ms-scene [aria-label=\"Zoom in\"]"))?, 1)
}

#[test]
fn the_focus_on_a_stage_control_is_not_moved_by_anything() -> Result<(), String> {
    let tab = open()?;
    text(&tab, "(() => { document.getElementById('ms-field').focus(); return 'ok'; })()")?;
    for _ in 0..3 {
        click_button(&tab, "rebuild")?;
    }
    check_eq(text(&tab, "document.activeElement.id")?, "ms-field".to_owned())
}

// The keyboard.

#[test]
fn a_button_takes_enter_and_space_and_the_range_takes_the_arrow_keys() -> Result<(), String> {
    let tab = open()?;
    click_button(&tab, "restart")?;
    text(
        &tab,
        "(() => { document.querySelector('[data-action=\"next\"]').focus(); return 'ok'; })()",
    )?;
    press(&tab, "Enter")?;
    check_eq(step(&tab)?, 1)?;
    press(&tab, " ")?;
    check_eq(step(&tab)?, 2)?;
    check_eq(
        text(&tab, "document.activeElement.getAttribute('data-action')")?,
        "next".to_owned(),
    )?;
    text(&tab, "(() => { document.getElementById('ms-seek').focus(); return 'ok'; })()")?;
    press(&tab, "ArrowRight")?;
    press(&tab, "ArrowRight")?;
    wait_until(&tab, 3, |t| Ok(step(t)? == 4))?;
    check_eq(attribute(&tab, "#ms-seek", "aria-valuetext")?, "4 of 432 cells".to_owned())
}

#[test]
fn every_control_is_reached_by_tab_in_order_and_a_cell_is_chosen_without_a_pointer() -> Result<(), String> {
    let tab = open()?;
    text(&tab, "(() => { document.getElementById('ms-field').focus(); return 'ok'; })()")?;
    let mut visited = Vec::new();
    for _ in 0..14 {
        press(&tab, "Tab")?;
        visited.push(text(
            &tab,
            "(() => { const a = document.activeElement; return a.id || a.getAttribute('data-action') || a.getAttribute('aria-label') || a.tagName; })()",
        )?);
    }
    let position = |name: &str| visited.iter().position(|v| v == name);
    let order = [
        "ms-show", "ms-inspect", "ms-threshold", "ms-row", "ms-column", "restart", "previous", "next", "rebuild",
        "retry", "ms-seek",
    ];
    let mut last = None;
    for name in order {
        let at = position(name).ok_or_else(|| format!("{name} is not reached: {visited:?}"))?;
        check(last.is_none_or(|l| at > l), &format!("{name} is out of order: {visited:?}"))?;
        last = Some(at);
    }
    // A cell is chosen by the selectors, which are the browser's own, with their events.
    choose(&tab, "ms-row", "3")?;
    choose(&tab, "ms-column", "5")?;
    check(field(&tab, "status")?.starts_with("Cell (3, 5)"), &field(&tab, "status")?)?;
    check(live(&tab)?.starts_with("Cell (3, 5)"), &live(&tab)?)
}

// Frames.

#[test]
fn a_burst_of_range_moves_draws_once_at_the_next_frame_and_says_nothing_until_it_rests() -> Result<(), String> {
    let tab = open_manual()?;
    click_button(&tab, "restart")?;
    let said = live(&tab)?;
    text(
        &tab,
        "(() => { const r = document.getElementById('ms-seek'); for (const v of ['30', '90', '200']) { r.value = v; \
         r.dispatchEvent(new Event('input', { bubbles: true })); } return 'ok'; })()",
    )?;
    check_eq((pending_frames(&tab)?, step(&tab)?), (1, 0))?;
    check_eq(advance_frames(&tab, 5)?, 1)?;
    check_eq((step(&tab)?, pending_frames(&tab)?), (200, 0))?;
    check_eq(live(&tab)?, said)?;
    text(
        &tab,
        "(() => { document.getElementById('ms-seek').dispatchEvent(new Event('change', { bubbles: true })); return 'ok'; })()",
    )?;
    check(live(&tab)?.starts_with("Cell ("), &live(&tab)?)
}

#[test]
fn letting_go_draws_now_and_the_frame_that_was_waiting_does_nothing() -> Result<(), String> {
    let tab = open_manual()?;
    click_button(&tab, "restart")?;
    text(
        &tab,
        "(() => { const r = document.getElementById('ms-seek'); r.value = '120'; \
         r.dispatchEvent(new Event('input', { bubbles: true })); r.value = '60'; \
         r.dispatchEvent(new Event('change', { bubbles: true })); return 'ok'; })()",
    )?;
    check_eq((step(&tab)?, pending_frames(&tab)?), (60, 0))?;
    check_eq(advance_frames(&tab, 3)?, 0)?;
    check_eq(step(&tab)?, 60)
}

#[test]
fn the_hooks_come_off_the_window_when_the_page_is_left() -> Result<(), String> {
    let tab = open_manual()?;
    check_eq(text(&tab, "typeof window.__msAdvanceFrame")?, "function".to_owned())?;
    text(&tab, "(() => { window.dispatchEvent(new Event('pagehide')); return 'ok'; })()")?;
    check_eq(
        text(&tab, "typeof window.__msAdvanceFrame + typeof window.__msPendingFrames")?,
        "undefinedundefined".to_owned(),
    )?;
    click_button(&tab, "restart")?;
    check_eq(step(&tab)?, CELLS)
}

// Faults.

#[test]
fn a_control_that_cannot_be_written_puts_back_the_others_and_leaves_the_state() -> Result<(), String> {
    for name in ["aria-valuetext", "aria-disabled", "max"] {
        let tab = open()?;
        seek(&tab, 100)?;
        let before = (step(&tab)?, field(&tab, "status")?, digest(&tab)?, drawn_contours(&tab)?);
        fail_writes_of(&tab, name, true)?;
        click_button(&tab, "next")?;
        fail_writes_of(&tab, name, false)?;
        check(!banner_hidden(&tab)?, &format!("{name}: reported"))?;
        check_eq(
            (step(&tab)?, field(&tab, "status")?, digest(&tab)?, drawn_contours(&tab)?),
            before,
        )
        .map_err(|e| format!("{name}: {e}"))?;
        click_button(&tab, "next")?;
        check_eq(step(&tab)?, 101).map_err(|e| format!("{name}: {e}"))?;
        check(banner_hidden(&tab)?, &format!("{name}: cleared"))?;
    }
    Ok(())
}

#[test]
fn a_plot_that_cannot_be_drawn_leaves_the_old_plot_and_a_clean_page() -> Result<(), String> {
    let tab = open()?;
    seek(&tab, 100)?;
    let before = (digest(&tab)?, drawn_contours(&tab)?);
    // A polyline is made with a `points` attribute, so this stops `set_plot_items`, which draws apart and then swaps.
    fail_writes_of(&tab, "points", true)?;
    click_button(&tab, "next")?;
    fail_writes_of(&tab, "points", false)?;
    check(!banner_hidden(&tab)?, "reported")?;
    check_eq((step(&tab)?, digest(&tab)?, drawn_contours(&tab)?), (100, before.0, before.1))?;
    // The page was not touched, so a button with nowhere to go does not draw it, and the line stays.
    click_button(&tab, "retry")?;
    click_button(&tab, "next")?;
    check_eq(step(&tab)?, 101)?;
    check(banner_hidden(&tab)?, "cleared by a step that works")
}

#[test]
fn a_scene_that_cannot_be_built_keeps_the_old_one_and_the_next_try_gets_a_fresh_id() -> Result<(), String> {
    let tab = open()?;
    inspect_cell(&tab, 14, 12)?;
    zoom_to(&tab, 400)?;
    let old = attribute(&tab, &format!("{STAGE} .ms-scene"), "id")?;
    let view = view_transform(&tab)?;
    fail_writes_of(&tab, "viewBox", true)?;
    click_button(&tab, "rebuild")?;
    fail_writes_of(&tab, "viewBox", false)?;
    check(!banner_hidden(&tab)?, "reported")?;
    check_eq(
        (
            attribute(&tab, &format!("{STAGE} .ms-scene"), "id")?,
            view_transform(&tab)?,
            zoom_percent(&tab)?,
        ),
        (old.clone(), view.clone(), 400),
    )?;
    check_eq(
        (count(&tab, &format!("{STAGE} .ms-scene"))?, count(&tab, "#ms-staging svg > *")?),
        (1, 0),
    )?;
    click_button(&tab, "rebuild")?;
    let fresh = attribute(&tab, &format!("{STAGE} .ms-scene"), "id")?;
    check(fresh != old, "a new ID")?;
    check_eq((view_transform(&tab)?, zoom_percent(&tab)?), (view, 400))?;
    check(banner_hidden(&tab)?, "cleared")
}

#[test]
fn a_swap_that_throws_before_or_after_it_changes_the_host_leaves_the_old_scene() -> Result<(), String> {
    for after in [false, true] {
        let tab = open()?;
        inspect_cell(&tab, 14, 12)?;
        zoom_to(&tab, 400)?;
        let old = attribute(&tab, &format!("{STAGE} .ms-scene"), "id")?;
        let before = (digest(&tab)?, drawn_contours(&tab)?, view_transform(&tab)?);
        fail_replacement(&tab, true, after)?;
        click_button(&tab, "rebuild")?;
        fail_replacement(&tab, false, after)?;
        check(!banner_hidden(&tab)?, &format!("after {after}: reported"))?;
        check_eq(attribute(&tab, &format!("{STAGE} .ms-scene"), "id")?, old.clone())
            .map_err(|e| format!("after {after}: {e}"))?;
        check_eq((digest(&tab)?, drawn_contours(&tab)?, view_transform(&tab)?), before)
            .map_err(|e| format!("after {after}: {e}"))?;
        check_eq(
            (count(&tab, &format!("{STAGE} .ms-scene"))?, count(&tab, "#ms-staging svg > *")?),
            (1, 0),
        )?;
        click_button(&tab, "rebuild")?;
        check(
            attribute(&tab, &format!("{STAGE} .ms-scene"), "id")? != old,
            "a fresh ID after the retry",
        )?;
        check(banner_hidden(&tab)?, "cleared")?;
        check_geometry(&tab, (14, 12), "after the retry")?;
    }
    Ok(())
}

#[test]
fn an_inspector_that_cannot_be_drawn_leaves_the_page_dirty_and_a_boundary_press_repairs_it() -> Result<(), String> {
    let tab = open()?;
    click_button(&tab, "restart")?;
    fail_replacement(&tab, true, false)?;
    click_button(&tab, "next")?;
    fail_replacement(&tab, false, false)?;
    check(!banner_hidden(&tab)?, "reported")?;
    // The plot already shows the first cell, but the committed state is the empty one.
    check_eq(step(&tab)?, 1).or_else(|_| check_eq(step(&tab)?, 0))?;
    // At the first step, Previous has nowhere to go, and still draws the page that is out of date.
    click_button(&tab, "previous")?;
    check(banner_hidden(&tab)?, "repaired by a button with nowhere to go")?;
    check_eq(step(&tab)?, 0)?;
    check_eq(live(&tab)?, "Already at the first step.".to_owned())?;
    for slot in 0..3 {
        check_eq(polylines(&tab, slot)?, 0)?;
    }
    Ok(())
}
