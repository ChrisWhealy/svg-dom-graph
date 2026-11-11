//! The Tarjan window in the built demo gallery: a strip of controls, the text of the step, and one Scene that holds the
//! graph of 64 vertices and 114 edges with an overlay, the stack, the roster and the condensation.
//!
//! Native unit tests cover the algorithm, the fixture, the condensation, the layout, the view rules, what each step
//! shows and every rule of the controller. What they cannot reach is the browser. These tests check the following:
//!
//! - The Phase 0 measurements, made on the real rendered paths: how many connectors pass through a vertex that is not
//!   theirs, that no two connectors coincide, and that the six antiparallel connectors run side by side.
//! - The scene is built once and never replaced, with the vertex targets above the edge targets, and one tab
//!   stop.
//! - Following keeps the search in view as it plays and as a person steps. A seek never moves the view, a jump to a
//!   component centres it, and a pan or a zoom by hand turns following off.
//! - The stack, the roster and the condensation are drawn from the one step, and a condensation node never moves.
//! - A write that fails is reported, and put right by the next action.
//! - A page that was diffed step by step is the same, as the DOM, as one that was drawn in full at every step.
//! - Leaving the page takes off every listener that the demo added.
//!
//! Like [`crate::petri_net`], these run against the built demo gallery.

mod dag;
mod offending;
mod phase0;

use crate::common::{demo_tab::DemoTab, new_demo_tab};
use crate::tarjan::{dag::Dag, phase0::Phase0};
use headless_chrome::Tab;
use std::time::{Duration, Instant};

const SOURCE: &str = "#tj-scene svg";
/// The height of the strip of controls that covers the top of the scene.
const STRIP: f64 = 150.0;
const VIEWPORT: (u32, u32) = (1000, 1000);

fn open_at(query: &str) -> Result<DemoTab, String> {
    new_demo_tab(
        &format!("tarjan-scc-window.html{query}"),
        &format!("{SOURCE} .tj-overlay"),
        VIEWPORT,
    )
}

fn open() -> Result<DemoTab, String> {
    open_at("")
}

fn open_manual() -> Result<DemoTab, String> {
    open_at("?manual-clock")
}

/// The page opened and then zoomed to the first vertex, so that most of the graph is out of view and following has
/// something to do.
fn open_zoomed() -> Result<DemoTab, String> {
    let tab = open()?;
    click(&tab, "tj-fit")?;
    Ok(tab)
}

fn open_manual_zoomed() -> Result<DemoTab, String> {
    let tab = open_manual()?;
    click(&tab, "tj-fit")?;
    Ok(tab)
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
    content(tab, &format!("#tj-p-{name}"))
}

fn live(tab: &Tab) -> Result<String, String> {
    Ok(content(tab, "#tj-live")?.replace('\u{a0}', ""))
}

fn position(tab: &Tab) -> Result<usize, String> {
    field(tab, "snapshot")?
        .split(" of ")
        .next()
        .and_then(|n| n.parse().ok())
        .ok_or_else(|| "the step field has no number".to_owned())
}

fn click(tab: &Tab, id: &str) -> Result<(), String> {
    text(
        tab,
        &format!("(() => {{ document.getElementById('{id}').dispatchEvent(new MouseEvent('click', {{ bubbles: true }})); return 'ok'; }})()"),
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

fn seek(tab: &Tab, to: usize) -> Result<(), String> {
    text(
        tab,
        &format!(
            "(() => {{ const r = document.getElementById('tj-seek'); r.value = '{to}'; \
             r.dispatchEvent(new Event('input', {{ bubbles: true }})); \
             r.dispatchEvent(new Event('change', {{ bubbles: true }})); return 'ok'; }})()"
        ),
    )?;
    wait_until(tab, 5, |tab| Ok(position(tab)? == to))
}

fn next(tab: &Tab) -> Result<(), String> {
    let at = position(tab)?;
    click(tab, "tj-next")?;
    wait_until(tab, 5, |tab| Ok(position(tab)? == at + 1))
}

fn previous(tab: &Tab) -> Result<(), String> {
    let at = position(tab)?;
    click(tab, "tj-previous")?;
    wait_until(tab, 5, |tab| Ok(position(tab)? + 1 == at))
}

fn focus(tab: &Tab, selector: &str) -> Result<(), String> {
    text(tab, &format!("document.querySelector('{selector}').focus(), 'ok'")).map(|_| ())
}

fn advance(tab: &Tab, n: usize) -> Result<usize, String> {
    text(tab, &format!("String(window.__tjAdvance({n}))"))?
        .parse()
        .map_err(|e| format!("{e}"))
}

fn advance_frames(tab: &Tab, n: usize) -> Result<usize, String> {
    text(tab, &format!("String(window.__tjAdvanceFrame({n}))"))?
        .parse()
        .map_err(|e| format!("{e}"))
}

fn waiting(tab: &Tab) -> Result<usize, String> {
    text(tab, "String(window.__tjPending())")?.parse().map_err(|e| format!("{e}"))
}

fn waiting_frames(tab: &Tab) -> Result<usize, String> {
    text(tab, "String(window.__tjPendingFrames())")?
        .parse()
        .map_err(|e| format!("{e}"))
}

fn disabled(tab: &Tab, id: &str) -> Result<bool, String> {
    Ok(attribute(tab, &format!("#{id}"), "aria-disabled")? == "true")
}

fn banner_hidden(tab: &Tab) -> Result<bool, String> {
    Ok(attribute(tab, "#tj-error", "hidden")? != "null")
}

fn follow_on(tab: &Tab) -> Result<bool, String> {
    Ok(attribute(tab, "#tj-follow", "aria-pressed")? == "true")
}

fn vertex(v: usize) -> String {
    format!("{SOURCE} [data-tj-vertex=\"v{v:02}\"]")
}

fn shoot(tab: &Tab, name: &str) -> Result<(), String> {
    let Ok(dir) = std::env::var("TJ_SHOT") else {
        return Ok(());
    };
    tab.evaluate("document.querySelector('#tj-scene').scrollIntoView()", false)
        .map_err(|e| format!("{e}"))?;
    let png = tab
        .capture_screenshot(
            headless_chrome::protocol::cdp::Page::CaptureScreenshotFormatOption::Png,
            None,
            None,
            true,
        )
        .map_err(|e| format!("could not take a screenshot: {e}"))?;
    std::fs::write(format!("{dir}/{name}.png"), png).map_err(|e| format!("{e}"))
}

/// The view of the scene, as `(scale, tx, ty)`, from the transform of its content group.
fn view(tab: &Tab) -> Result<(f64, f64, f64), String> {
    let t = text(
        tab,
        "document.querySelector('#tj-scene .svg-dom-graph-content').getAttribute('transform') || 'translate(0, 0) scale(1)'",
    )?;
    let numbers: Vec<f64> = t
        .split(|c: char| !(c.is_ascii_digit() || c == '.' || c == '-' || c == 'e'))
        .filter(|s| !s.is_empty() && *s != "-" && *s != ".")
        .filter_map(|s| s.parse().ok())
        .collect();
    match numbers.as_slice() {
        [tx, ty, scale] => Ok((*scale, *tx, *ty)),
        other => Err(format!("cannot read the transform {t:?}: {other:?}")),
    }
}

/// Where the centre of a vertex's box is on the screen of the scene's viewport, as a fraction of its width and height.
fn vertex_in_pane(tab: &Tab, v: usize) -> Result<(f64, f64), String> {
    let (scale, tx, ty) = view(tab)?;
    let r = text(
        tab,
        &format!(
            "(() => {{ const e = document.querySelector('{}'); return JSON.stringify([+e.getAttribute('x') + e.getAttribute('width') / 2, \
             +e.getAttribute('y') + e.getAttribute('height') / 2]); }})()",
            vertex(v)
        ),
    )?;
    let p: Vec<f64> = serde_json::from_str(&r).map_err(|e| e.to_string())?;
    let (w, h) = viewport(tab)?;
    Ok(((scale * p[0] + tx) / w, (scale * p[1] + ty - STRIP) / h))
}

/// The size of the scene's viewport, in CSS pixels.
fn viewport(tab: &Tab) -> Result<(f64, f64), String> {
    let r = text(
        tab,
        &format!(
            "JSON.stringify([document.querySelector('{SOURCE}').clientWidth, document.querySelector('{SOURCE}').clientHeight - {STRIP}])"
        ),
    )?;
    let p: Vec<f64> = serde_json::from_str(&r).map_err(|e| e.to_string())?;
    Ok((p[0], p[1]))
}

fn inside_safe(f: (f64, f64)) -> bool {
    // A little more than rounding, since the boxes are written to a tenth of a unit.
    (0.15 - 1e-3..=0.85 + 1e-3).contains(&f.0) && (0.15 - 1e-3..=0.85 + 1e-3).contains(&f.1)
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The Phase 0 measurements, from the rendered paths: for each of the 114 connectors, the vertices other than its own
/// whose box, expanded by 4 scene units, any sampled point of it lies in, and how the connectors compare with each
/// other.
const PHASE_0: &str = r#"(() => {
  const svg = document.querySelector('#tj-scene svg');
  const starts = [0, 11, 21, 30, 38, 45, 51, 56, 60, 63], sizes = [11, 10, 9, 8, 7, 6, 5, 4, 3, 1];
  const region = v => starts.filter(s => s <= v).length - 1;
  const boxes = [...svg.querySelectorAll('.tj-vertex')].map(e => ({x: +e.getAttribute('x') - 4, y: +e.getAttribute('y') - 4,
      w: +e.getAttribute('width') + 8, h: +e.getAttribute('height') + 8}));
  const ends = [...svg.querySelectorAll('.tj-edge-hit')].map(e => e.getAttribute('aria-label').match(/from v(\d+) to v(\d+)/).slice(1).map(Number));
  const paths = [...svg.querySelectorAll('path[data-tj-edge]')];
  const sample = (p, n) => { const L = p.getTotalLength(), out = []; for (let i = 0; i <= n; i++) { const q = p.getPointAtLength(L * i / n); out.push([q.x, q.y]); } return out; };
  const dense = paths.map(p => sample(p, Math.max(2, Math.ceil(p.getTotalLength() / 2))));
  const offending = [];
  dense.forEach((pts, e) => {
    const [a, b] = ends[e], hit = [];
    boxes.forEach((r, v) => { if (v === a || v === b) return;
      if (pts.some(([x, y]) => x >= r.x && x <= r.x + r.w && y >= r.y && y <= r.y + r.h)) hit.push(v); });
    if (hit.length) offending.push({e, a, b, hit, kind: region(a) !== region(b) ? 'bridge' : ((b - starts[region(a)] - (a - starts[region(a)]) + sizes[region(a)]) % sizes[region(a)] === 1 ? 'ring' : 'chord')});
  });
  const few = paths.map(p => sample(p, 20));
  let coincident = 0;
  for (let i = 0; i < few.length; i++) for (let j = i + 1; j < few.length; j++) {
    const same = few[i].every((q, k) => Math.hypot(q[0] - few[j][k][0], q[1] - few[j][k][1]) <= 1);
    const reversed = few[i].every((q, k) => Math.hypot(q[0] - few[j][20 - k][0], q[1] - few[j][20 - k][1]) <= 1);
    if (same || reversed) coincident++;
  }
  const pairs = [[56, 58], [60, 62], [61, 62]], separation = {};
  for (const [a, b] of pairs) {
    const ia = ends.findIndex(([x, y]) => x === a && y === b), ib = ends.findIndex(([x, y]) => x === b && y === a);
    const mid = pts => pts.slice(Math.floor(pts.length * 0.3), Math.ceil(pts.length * 0.7) + 1);
    const far = (from, to) => Math.min(...mid(dense[from]).map(([x, y]) => Math.min(...dense[to].map(([u, w]) => Math.hypot(x - u, y - w)))));
    separation[a + '-' + b] = Math.min(far(ia, ib), far(ib, ia));
  }
  return JSON.stringify({offending, coincident, separation});
})()"#;

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
fn drag(tab: &Tab, from: (f64, f64), to: (f64, f64)) -> Result<(), String> {
    use headless_chrome::protocol::cdp::Input::{
        DispatchMouseEvent, DispatchMouseEventTypeOption as Type, MouseButton,
    };
    let send = |kind: Type, at: (f64, f64), buttons: Option<u32>| {
        tab.call_method(DispatchMouseEvent {
            Type: kind,
            x: at.0,
            y: at.1,
            modifiers: None,
            timestamp: None,
            button: Some(MouseButton::Left),
            buttons,
            click_count: Some(1),
            force: None,
            tangential_pressure: None,
            tilt_x: None,
            tilt_y: None,
            twist: None,
            delta_x: None,
            delta_y: None,
            pointer_Type: None,
        })
        .map(|_| ())
        .map_err(|e| format!("{e}"))
    };
    send(Type::MousePressed, from, Some(1))?;
    for step in 1..=8 {
        let f = f64::from(step) / 8.0;
        send(
            Type::MouseMoved,
            (from.0 + (to.0 - from.0) * f, from.1 + (to.1 - from.1) * f),
            Some(1),
        )?;
    }
    send(Type::MouseReleased, to, Some(0))
}

/// A point of the scene's background, on the screen, where no vertex, edge or outline is.
fn background_point(tab: &Tab) -> Result<(f64, f64), String> {
    let r = text(
        tab,
        "(() => { const s = document.querySelector('#tj-scene svg'); const b = s.getBoundingClientRect();
           for (let y = b.y + 170; y < b.bottom - 20; y += 20) for (let x = b.x + 20; x < b.right - 80; x += 20) {
             const e = document.elementFromPoint(x, y); if (e && e.tagName === 'rect' && e.getAttribute('class') === null || e === s) return JSON.stringify([x, y]); }
           return JSON.stringify([b.x + 30, b.bottom - 30]); })()",
    )?;
    let p: Vec<f64> = serde_json::from_str(&r).map_err(|e| e.to_string())?;
    Ok((p[0], p[1]))
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// How many cells of the stack's node are in use, which is the ones that have a vertex in them.
fn stack_depth(tab: &Tab) -> Result<usize, String> {
    count_texts(tab, &format!("{SOURCE} g[data-tj-stack] text"), "·")
}

fn count_texts(tab: &Tab, selector: &str, containing: &str) -> Result<usize, String> {
    text(
        tab,
        &format!(
            "String([...document.querySelectorAll('{selector}')].filter(e => e.textContent.includes('{containing}')).length)"
        ),
    )?
    .parse()
    .map_err(|e| format!("{e}"))
}

fn dag_nodes(tab: &Tab) -> Result<usize, String> {
    count(tab, &format!("{SOURCE} [data-tj-dag-node]:not([data-tj-hidden=\"true\"])"))
}

fn dag_arcs(tab: &Tab) -> Result<usize, String> {
    count(tab, &format!("{SOURCE} [data-tj-dag-edge]:not([data-tj-hidden=\"true\"])"))
}

fn rows(tab: &Tab) -> Result<usize, String> {
    count(tab, &format!("{SOURCE} [data-tj-roster]:not([data-tj-hidden=\"true\"])"))
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Makes every write of the attribute `name` fail with an exception, or puts the browser's own write back.
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

fn first_difference(a: &str, b: &str) -> String {
    let at = a.chars().zip(b.chars()).take_while(|(x, y)| x == y).count();
    let around = |s: &str| s.chars().skip(at.saturating_sub(80)).take(200).collect::<String>();
    format!("at {at}: diffed ...{}... but full ...{}...", around(a), around(b))
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// How many listeners of each type Chrome says are on the object that `expression` evaluates to.
fn listener_counts(tab: &Tab, expression: &str) -> Result<std::collections::BTreeMap<String, usize>, String> {
    use headless_chrome::protocol::cdp::DOMDebugger::GetEventListeners;
    let object_id = tab
        .evaluate(expression, false)
        .map_err(|e| format!("could not evaluate {expression}: {e}"))?
        .object_id
        .ok_or_else(|| format!("{expression} is not an object"))?;
    let listeners = tab
        .call_method(GetEventListeners {
            object_id,
            depth: None,
            pierce: None,
        })
        .map_err(|e| format!("could not ask for the listeners of {expression}: {e}"))?
        .listeners;
    let mut counts = std::collections::BTreeMap::new();
    for listener in listeners {
        *counts.entry(listener.Type).or_insert(0) += 1;
    }
    Ok(counts)
}

/// Where each node of the condensation is, as the numbers in its transform, in the order of its component, and the
/// paths of the arrows that are shown.
const DAG_STATE: &str = r#"(() => {
    const svg = document.querySelector('#tj-scene svg');
    const nodes = [...svg.querySelectorAll('[data-tj-dag-node]')].sort((a, b) => a.getAttribute('data-tj-component') - b.getAttribute('data-tj-component'))
        .map(g => (g.getAttribute('transform').match(/-?[\d.]+/g) || []).map(Number).slice(0, 2));
    const arcs = [...svg.querySelectorAll('[data-tj-dag-edge]:not([data-tj-hidden="true"])')].map(p => p.getAttribute('data-tj-dag-edge').split('-').map(Number));
    return JSON.stringify({ nodes, arcs, state: svg.getAttribute('data-tj-springs') });
})()"#;

fn dag(tab: &Tab) -> Result<Dag, String> {
    serde_json::from_str(&text(tab, DAG_STATE)?).map_err(|e| e.to_string())
}

mod unit_tests;
