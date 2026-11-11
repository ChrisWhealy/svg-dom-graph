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
//! - A real Ctrl wheel about an off-centre pointer keeps the point under it still, a real drag pans the view by its
//!   distance without choosing a cell, a click still does, the toolbar does not move, and a rebuild keeps all of it.
//! - A scene rebuilt 25 times at a zoomed and panned view is the same, with unique IDs, one scene, and one handler.
//! - A burst of range moves draws once, and a late frame cannot overwrite a newer move.
//! - A write that fails, a scene that cannot be built, or a swap that throws before or after it changes the host, is
//!   reported, keeps the state, and is put right by the next action.
//!
//! Like [`crate::red_black_tree`], these run against the built demo gallery.

mod found;

use crate::common::{ctrl_wheel, demo_tab::DemoTab, drag, new_demo_tab};
use crate::marching_squares::found::Found;
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

/// Makes `focus()` do nothing on SVG elements, which stands in for a focus that cannot be put back.
fn fail_focus(tab: &Tab, failing: bool) -> Result<(), String> {
    let script = if failing {
        "(() => { const p = SVGElement.prototype; if (!p.__focused) { p.__focused = p.focus; } \
         p.focus = function () {}; return 'ok'; })()"
    } else {
        "(() => { const p = SVGElement.prototype; if (p.__focused) { p.focus = p.__focused; } return 'ok'; })()"
    };
    text(tab, script).map(|_| ())
}

/// Lets the next `allow` calls of `replaceChildren` work, and makes every one after them throw, or stops it.
fn fail_replacement_after(tab: &Tab, failing: bool, allow: usize) -> Result<(), String> {
    let script = if failing {
        format!(
            "(() => {{ const p = Element.prototype; if (!p.__replaced) {{ p.__replaced = p.replaceChildren; }} \
             window.__allow = {allow}; p.replaceChildren = function () {{ if (window.__allow-- > 0) {{ \
             return p.__replaced.apply(this, arguments); }} throw new Error('injected'); }}; return 'ok'; }})()"
        )
    } else {
        "(() => { const p = Element.prototype; if (p.__replaced) { p.replaceChildren = p.__replaced; } return 'ok'; })()"
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

/// Where the first corner of the outline of the inspected cell is on the screen, which a zoom about a pointer over it
/// must not move, and which a pan moves by the distance of the drag.
fn outline_corner(tab: &Tab) -> Result<(f64, f64), String> {
    let v = json(
        tab,
        r#"(() => { const p = [...document.querySelectorAll('#ms-stage .ms-scene polyline')].find(e => e.getAttribute('stroke') === '#be123c');
            const m = p.getScreenCTM(), [x, y] = p.getAttribute('points').trim().split(/\s+/)[0].split(',').map(Number);
            return JSON.stringify([m.a * x + m.c * y + m.e, m.b * x + m.d * y + m.f]); })()"#,
    )?;
    Ok((v[0].as_f64().unwrap_or(f64::NAN), v[1].as_f64().unwrap_or(f64::NAN)))
}

/// Where the toolbar is, as its left, top, width and height from the top left corner of the stage. The stage is
/// measured and not the scene, since the box of the scene's `<svg>` grows with its zoomed content.
fn toolbar_place(tab: &Tab) -> Result<Vec<f64>, String> {
    let v = json(
        tab,
        r#"(() => { const scene = document.getElementById('ms-stage-svg').getBoundingClientRect();
            const bar = document.querySelector('#ms-stage .ms-scene [aria-label="Scene controls"]').getBoundingClientRect();
            return JSON.stringify([bar.left - scene.left, bar.top - scene.top, bar.width, bar.height]); })()"#,
    )?;
    Ok(v.as_array()
        .map(|a| a.iter().filter_map(Value::as_f64).collect())
        .unwrap_or_default())
}

fn near(a: (f64, f64), b: (f64, f64), tolerance: f64) -> bool {
    (a.0 - b.0).abs() <= tolerance && (a.1 - b.1).abs() <= tolerance
}

/// Whether the scene on the page is the one for the 25 by 19 field: it is described as that field, and it has a sample
/// for each of the 475 points of the grid, which the 9 by 7 field's scene, with 63, does not.
fn has_ripple_axes(tab: &Tab) -> Result<bool, String> {
    let found = json(
        tab,
        r#"(() => { const scene = document.querySelector('#ms-stage .ms-scene');
            const described = [...scene.querySelectorAll('[aria-label]')].some(e => e.getAttribute('aria-label').includes('Ripple plane'));
            return JSON.stringify([described, scene.querySelectorAll('circle').length]); })()"#,
    )?;
    Ok(found[0].as_bool() == Some(true) && found[1].as_u64().unwrap_or(0) >= 475)
}

mod unit_tests;
