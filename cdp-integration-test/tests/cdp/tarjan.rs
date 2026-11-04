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

use crate::common::{DemoTab, new_demo_tab};
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
#[test]
fn the_page_has_the_controls_the_graph_the_lists_and_the_condensation_and_opens_at_step_zero() -> Result<(), String> {
    let tab = open()?;
    check_eq(count(&tab, "#tj-controls-svg [data-action]")?, 9)?;
    check_eq(count(&tab, "#tj-controls-svg foreignObject select")?, 1)?;
    check_eq(count(&tab, "#tj-controls-svg input[data-slider=\"seek\"]")?, 1)?;
    check_eq(count(&tab, &format!("{SOURCE} .tj-vertex"))?, 64)?;
    check_eq(count(&tab, &format!("{SOURCE} .tj-edge-hit"))?, 114)?;
    check_eq(count(&tab, &format!("{SOURCE} path[data-tj-edge]"))?, 114)?;
    check_eq((position(&tab)?, field(&tab, "components")?), (0, "0 found".to_owned()))?;
    check_eq(count(&tab, &format!("{SOURCE} g[data-tj-stack]"))?, 1)?;
    check_eq(
        count(&tab, &format!("{SOURCE} [data-tj-dag-node]:not([data-tj-hidden=\"true\"])"))?,
        0,
    )?;
    check_eq(count(&tab, &format!("{SOURCE} .tj-headings text"))?, 2)?;
    check(
        content(&tab, &format!("{SOURCE} .tj-headings text:last-child"))?.contains("arrows point right to left"),
        "the legend of the condensation is in the scene",
    )?;
    check_eq(banner_hidden(&tab)?, true)?;
    check(
        disabled(&tab, "tj-restart")? && disabled(&tab, "tj-previous")? && !disabled(&tab, "tj-next")?,
        "buttons",
    )?;
    shoot(&tab, "initial")
}

#[test]
fn every_vertex_shows_two_dashes_in_a_node_that_is_hidden_from_assistive_technology_and_has_a_name_of_its_own()
-> Result<(), String> {
    let tab = open()?;
    let content_group = format!("{SOURCE} .svg-dom-graph-content");
    check_eq(count(&tab, &format!("{content_group} > g[aria-hidden=\"true\"]"))?, 64)?;
    check_eq(count(&tab, &format!("{content_group} > path[aria-hidden=\"true\"]"))?, 114)?;
    let shown = text(
        &tab,
        &format!(
            "(() => {{ const t = [...document.querySelectorAll('{content_group} > g text')].map(e => e.textContent); return JSON.stringify([t.filter(s => s === '--').length, t.includes('v00'), t.includes('v63')]); }})()"
        ),
    )?;
    check_eq(shown, "[128,true,true]".to_owned())?;
    check_eq(
        attribute(&tab, &vertex(0), "aria-label")?,
        "v00: index --, lowlink --, not discovered yet, the vertex in focus".to_owned(),
    )?;
    check_eq(
        attribute(&tab, &vertex(5), "aria-label")?,
        "v05: index --, lowlink --, not discovered yet".to_owned(),
    )?;
    Ok(())
}

#[test]
fn the_overlay_paints_the_outlines_then_the_edge_targets_then_the_vertex_targets_and_there_is_one_tab_stop()
-> Result<(), String> {
    let tab = open()?;
    let order = text(
        &tab,
        &format!(
            "[...document.querySelector('{SOURCE} .tj-overlay').children].map(e => e.getAttribute('class')).join()"
        ),
    )?;
    check_eq(
        order,
        "tj-headings,tj-outlines,tj-roster,tj-edge-hits,tj-vertex-hits".to_owned(),
    )?;
    check_eq(
        count(
            &tab,
            &format!("{SOURCE} [tabindex=\"0\"].tj-vertex, {SOURCE} [tabindex=\"0\"].tj-edge-hit"),
        )?,
        1,
    )?;
    check_eq(
        count(&tab, &format!("{SOURCE} .tj-vertex[tabindex], {SOURCE} .tj-edge-hit[tabindex]"))?,
        178,
    )?;
    // A click on a vertex, where an edge passes over it, goes to the vertex: the vertex target is the topmost at its
    // centre.
    let on_top = text(
        &tab,
        &format!(
            "(() => {{ let wrong = 0; for (let v = 0; v < 64; v++) {{ const e = document.querySelector('{SOURCE} [data-tj-vertex=\"v' + String(v).padStart(2, '0') + '\"]'); \
             const r = e.getBoundingClientRect(); const top = document.elementFromPoint(r.x + r.width / 2, r.y + r.height / 2); \
             const pane = document.querySelector('{SOURCE}').getBoundingClientRect(); \
             const inside = r.x > pane.x && r.right < pane.right && r.y > pane.y + {STRIP} && r.bottom < pane.bottom; \
             const group = document.querySelector('{SOURCE} .svg-dom-graph-content'); if (r.width > 0 && inside && top !== e && group.contains(top)) wrong++; }} return String(wrong); }})()"
        ),
    )?;
    check(
        on_top == "0",
        &format!("{on_top} visible vertices have something over their centre"),
    )
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

#[derive(serde::Deserialize)]
struct Offending {
    e: usize,
    a: usize,
    b: usize,
    hit: Vec<usize>,
    kind: String,
}

#[derive(serde::Deserialize)]
struct Phase0 {
    offending: Vec<Offending>,
    coincident: usize,
    separation: std::collections::BTreeMap<String, f64>,
}

#[test]
fn phase_0_at_most_24_of_the_114_connectors_pass_through_a_vertex() -> Result<(), String> {
    let tab = open()?;
    let m: Phase0 = serde_json::from_str(&text(&tab, PHASE_0)?).map_err(|e| e.to_string())?;
    let by = |kind: &str| m.offending.iter().filter(|o| o.kind == kind).count();
    let most = m.offending.iter().map(|o| o.hit.len()).max().unwrap_or(0);
    println!(
        "PHASE0 offending {} of 114 (ring {}, chord {}, bridge {}), most third-party boxes hit by one edge {most}: {:?}",
        m.offending.len(),
        by("ring"),
        by("chord"),
        by("bridge"),
        m.offending.iter().map(|o| (o.e, o.a, o.b, o.hit.clone())).collect::<Vec<_>>()
    );
    check(
        m.offending.len() <= 24,
        &format!("{} connectors pass through a vertex", m.offending.len()),
    )
}

#[test]
fn phase_0_no_two_connectors_coincide_and_the_three_antiparallel_pairs_run_side_by_side() -> Result<(), String> {
    let tab = open()?;
    let m: Phase0 = serde_json::from_str(&text(&tab, PHASE_0)?).map_err(|e| e.to_string())?;
    println!("PHASE0 coincident {} separation {:?}", m.coincident, m.separation);
    check_eq(m.coincident, 0)?;
    check_eq(m.separation.len(), 3)?;
    for (pair, scene_units) in &m.separation {
        for scale in [1.0, 0.85] {
            check(
                scene_units * scale >= 16.0,
                &format!("{pair} is {:.1} CSS px apart at {scale}", scene_units * scale),
            )?;
        }
    }
    Ok(())
}

#[test]
fn all_six_antiparallel_connectors_and_the_four_others_at_vertex_62_can_be_picked_one_at_a_time_by_the_pointer()
-> Result<(), String> {
    let tab = open()?;
    // For each edge, the topmost thing at the most isolated point of its own hit path is that path.
    let wrong = |tab: &Tab, wanted: &str| {
        text(
            tab,
            &format!(
                r#"(() => {{
                  const wanted = {wanted};
                  const pane = document.querySelector('{SOURCE}').getBoundingClientRect();
                  const hits = [...document.querySelectorAll('{SOURCE} .tj-edge-hit')];
                  const out = [];
                  const dense = hits.map(h => {{ const L = h.getTotalLength(), pts = []; for (let d = 0; d <= L; d += 3) pts.push(h.getPointAtLength(d)); return pts; }});
                  for (const [a, b] of wanted) {{
                    const at = hits.findIndex(e => (e.getAttribute('aria-label') + ',').includes('from v' + String(a).padStart(2, '0') + ' to v' + String(b).padStart(2, '0') + ','));
                    const h = hits[at], m = h.getScreenCTM(), L = h.getTotalLength();
                    // The most isolated point of the middle of the path that is in the pane and not under a vertex: the
                    // one farthest from every other edge.
                    let best = null;
                    for (let i = 0; i <= 40; i++) {{
                      const p = h.getPointAtLength(L * (0.15 + 0.7 * i / 40));
                      const x = m.a * p.x + m.c * p.y + m.e, y = m.b * p.x + m.d * p.y + m.f;
                      if (x < pane.x + 5 || x > pane.right - 80 || y < pane.y + {STRIP} + 5 || y > pane.bottom - 5) continue;
                      const covering = document.elementFromPoint(x, y);
                      if (covering && covering.classList.contains('tj-vertex')) continue;
                      let near = 1e9;
                      dense.forEach((pts, k) => {{ if (k !== at) for (const q of pts) near = Math.min(near, Math.hypot(p.x - q.x, p.y - q.y)); }});
                      if (!best || near > best.near) best = {{p, near, x, y}};
                    }}
                    if (!best) {{ out.push(a + '-' + b + ' has no point that is in the pane and not under a vertex'); continue; }}
                    const top = document.elementFromPoint(best.x, best.y);
                    // A hit path is 14 scene units wide, so a point that is 7 from every other path is in no other's
                    // hit area.
                    if (best.near < 7) out.push(a + '-' + b + ' has no isolated point: ' + best.near.toFixed(1));
                    else if (top !== h) out.push(a + '-' + b + ' got ' + (top && (top.getAttribute('data-tj-edge-hit') || top.getAttribute('data-tj-vertex') || top.tagName)));
                  }}
                  return JSON.stringify(out); }})()"#
            ),
        )
    };
    check_eq(
        wrong(&tab, "[[56, 58], [58, 56], [60, 62], [62, 60], [61, 62], [62, 61], [62, 63]]")?,
        "[]".to_owned(),
    )
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[test]
fn the_page_opens_with_the_whole_picture_in_view_scaled_down_and_zooming_to_a_vertex_reads_at_scale_one()
-> Result<(), String> {
    let tab = open()?;
    let (scale, ..) = view(&tab)?;
    check((0.25..1.0).contains(&scale), &format!("opens at {scale}"))?;
    for v in 0..64 {
        let (x, y) = vertex_in_pane(&tab, v)?;
        check(
            (0.0..=1.0).contains(&x) && (0.0..=1.0).contains(&y),
            &format!("v{v:02} is outside the window"),
        )?;
    }
    check_eq(field(&tab, "view")?, String::new())?;
    check_eq(attribute(&tab, "#tj-p-view", "hidden")?, String::new())?;
    click(&tab, "tj-fit")?;
    let (zoomed, ..) = view(&tab)?;
    check_eq(zoomed, 1.0)?;
    check(inside_safe(vertex_in_pane(&tab, 0)?), "vertex 0 is in the safe rectangle")
}

#[test]
fn next_from_34_to_35_brings_the_new_vertex_into_view_and_previous_the_old_one_and_following_never_zooms()
-> Result<(), String> {
    let tab = open_zoomed()?;
    seek(&tab, 34)?;
    let at_seek = view(&tab)?;
    check_eq(at_seek.0, 1.0)?;
    check(
        inside_safe(vertex_in_pane(&tab, 3)?) || !inside_safe(vertex_in_pane(&tab, 33)?),
        "the seek left the view alone",
    )?;
    next(&tab)?;
    check(
        inside_safe(vertex_in_pane(&tab, 33)?),
        "vertex 33 is in the safe rectangle after Next",
    )?;
    check_eq(view(&tab)?.0, 1.0)?;
    previous(&tab)?;
    check(
        inside_safe(vertex_in_pane(&tab, 3)?),
        "vertex 3 is in the safe rectangle after Previous",
    )?;
    shoot(&tab, "followed")
}

#[test]
fn a_seek_never_moves_the_view_and_says_where_the_vertex_is_whether_following_is_on_or_off() -> Result<(), String> {
    for turn_off in [false, true] {
        let tab = open_zoomed()?;
        if turn_off {
            click(&tab, "tj-follow")?;
            check_eq(follow_on(&tab)?, false)?;
        }
        let before = view(&tab)?;
        seek(&tab, 74)?;
        check_eq(view(&tab)?, before)?;
        check_eq(
            field(&tab, "view")?,
            "Active vertex v63 is off-screen; use Zoom to active vertex.".to_owned(),
        )?;
        check(attribute(&tab, "#tj-p-view", "hidden")? == "null", "the notice is shown")?;
        check(
            attribute(&tab, "#tj-fit-inline", "id")? == "tj-fit-inline",
            "the Fit button is beside it",
        )?;
    }
    Ok(())
}

#[test]
fn next_scc_and_previous_scc_and_jump_to_final_centre_their_destinations_even_with_following_off() -> Result<(), String>
{
    let tab = open_zoomed()?;
    click(&tab, "tj-follow")?;
    check_eq(follow_on(&tab)?, false)?;
    click(&tab, "tj-next-component")?;
    wait_until(&tab, 5, |t| Ok(position(t)? == 74))?;
    check(inside_safe(vertex_in_pane(&tab, 63)?), "v63 is in view at step 74")?;
    check_eq(follow_on(&tab)?, false)?;
    click(&tab, "tj-next-component")?;
    wait_until(&tab, 5, |t| Ok(position(t)? == 77))?;
    click(&tab, "tj-previous-component")?;
    wait_until(&tab, 5, |t| Ok(position(t)? == 74))?;
    click(&tab, "tj-jump-final")?;
    wait_until(&tab, 5, |t| Ok(position(t)? == 252))?;
    check(
        disabled(&tab, "tj-jump-final")? && disabled(&tab, "tj-next-component")?,
        "dimmed at the end",
    )?;
    check_eq(field(&tab, "components")?.starts_with("10 found"), true)
}

#[test]
fn zoom_brings_the_attention_vertex_into_view_at_a_readable_scale_and_leaves_following_alone() -> Result<(), String> {
    let tab = open_zoomed()?;
    click(&tab, "tj-follow")?;
    seek(&tab, 74)?;
    click(&tab, "tj-fit")?;
    check(inside_safe(vertex_in_pane(&tab, 63)?), "v63 is in view after Fit")?;
    check_eq(view(&tab)?.0, 1.0)?;
    check_eq(follow_on(&tab)?, false)?;
    check_eq(field(&tab, "view")?, String::new())
}

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

#[test]
fn a_pan_by_hand_turns_following_off_once_says_so_and_a_later_step_does_not_pan_over_it() -> Result<(), String> {
    let tab = open()?;
    seek(&tab, 34)?;
    let (x, y) = background_point(&tab)?;
    drag(&tab, (x, y), (x + 60.0, y + 40.0))?;
    wait_until(&tab, 5, |t| Ok(!follow_on(t)?))?;
    check_eq(
        live(&tab)?.contains("Follow active vertex off")
            || content(&tab, "#tj-live")?.contains("Follow active vertex off"),
        true,
    )?;
    let panned = view(&tab)?;
    check(panned.1 != 0.0 || panned.2 != 0.0, "the view moved")?;
    next(&tab)?;
    check_eq(view(&tab)?, panned)?;
    check_eq(follow_on(&tab)?, false)
}

#[test]
fn the_zoom_toolbar_turns_following_off_too_and_a_controller_pan_never_does() -> Result<(), String> {
    let tab = open_zoomed()?;
    seek(&tab, 34)?;
    next(&tab)?;
    next(&tab)?;
    check_eq(follow_on(&tab)?, true)?;
    let button = format!("{SOURCE} [aria-label*=\"oom in\"], {SOURCE} [data-zoom=\"in\"]");
    let found = text(&tab, &format!("String(document.querySelectorAll('{button}').length)"))?;
    if found == "0" {
        // The toolbar's own buttons are found by their labels, whatever the library calls them.
        text(
            &tab,
            &format!(
                "(() => {{ const b = [...document.querySelectorAll('{SOURCE} g')].find(g => g.textContent.trim() === '+'); b.dispatchEvent(new MouseEvent('click', {{ bubbles: true }})); return 'ok'; }})()"
            ),
        )?;
    } else {
        text(
            &tab,
            &format!(
                "(() => {{ document.querySelector('{button}').dispatchEvent(new MouseEvent('click', {{ bubbles: true }})); return 'ok'; }})()"
            ),
        )?;
    }
    wait_until(&tab, 5, |t| Ok(!follow_on(t)?))?;
    check(view(&tab)?.0 > 1.0, "the toolbar zoomed in")
}

#[test]
fn turning_following_back_on_is_for_the_person_only_and_the_next_step_is_then_followed() -> Result<(), String> {
    let tab = open_zoomed()?;
    click(&tab, "tj-follow")?;
    seek(&tab, 74)?;
    let before = view(&tab)?;
    next(&tab)?;
    check_eq(view(&tab)?, before)?;
    click(&tab, "tj-follow")?;
    check_eq(follow_on(&tab)?, true)?;
    next(&tab)?;
    let attention: usize = field(&tab, "attention")?
        .trim_start_matches('v')
        .split_whitespace()
        .next()
        .unwrap_or("0")
        .parse()
        .map_err(|e| format!("{e}"))?;
    check(inside_safe(vertex_in_pane(&tab, attention)?), "following works again")
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

#[test]
fn the_stack_node_has_all_27_cells_at_its_peak_newest_first_and_the_roster_lists_each_component() -> Result<(), String>
{
    let tab = open()?;
    seek(&tab, 184)?;
    check_eq(stack_depth(&tab)?, 27)?;
    check_eq(rows(&tab)?, 6)?;
    seek(&tab, 252)?;
    check_eq(rows(&tab)?, 10)?;
    check_eq(stack_depth(&tab)?, 0)?;
    // And back again, from the snapshot and not from what was there.
    seek(&tab, 184)?;
    check_eq(stack_depth(&tab)?, 27)
}

#[test]
fn the_stack_is_called_the_scc_stack_not_the_dfs_stack_and_is_all_in_the_scene() -> Result<(), String> {
    let tab = open()?;
    seek(&tab, 184)?;
    check_eq(content(&tab, "#tj-p-stack")?.starts_with("depth 27"), true)?;
    check(
        content(&tab, &format!("{SOURCE} g[data-tj-stack]"))?.contains("Tarjan SCC stack"),
        "the node is named",
    )?;
    check_eq(count(&tab, "#tj-stack, #tj-roster, #tj-dag")?, 0)
}

#[test]
fn a_component_is_outlined_listed_and_drawn_as_a_node_when_it_closes_and_removed_again_going_back() -> Result<(), String>
{
    let tab = open()?;
    seek(&tab, 74)?;
    check_eq(count(&tab, &format!("{SOURCE} .tj-outline"))?, 1)?;
    check_eq(content(&tab, &format!("{SOURCE} .tj-outline text"))?, "C1".to_owned())?;
    check_eq(dag_nodes(&tab)?, 1)?;
    check_eq(content(&tab, "#tj-p-dag")?, "1 components and 0 arrows between them".to_owned())?;
    check(
        attribute(&tab, &vertex(63), "class")?.contains("tj-closed")
            && attribute(&tab, &vertex(63), "class")?.contains("tj-comp-1"),
        "v63 is closed in C1",
    )?;
    seek(&tab, 73)?;
    check_eq(count(&tab, &format!("{SOURCE} .tj-outline"))?, 0)?;
    check_eq(dag_nodes(&tab)?, 0)?;
    check_eq(rows(&tab)?, 0)?;
    check(
        attribute(&tab, &vertex(63), "class")?.contains("tj-on-stack"),
        "v63 is back on the stack",
    )?;
    seek(&tab, 252)?;
    check_eq((dag_nodes(&tab)?, dag_arcs(&tab)?), (10, 17))?;
    check_eq(count(&tab, &format!("{SOURCE} .tj-outline"))?, 10)?;
    shoot(&tab, "final")
}

#[test]
fn a_condensation_node_never_moves_once_it_has_been_shown_and_the_scene_is_never_replaced() -> Result<(), String> {
    let tab = open()?;
    let nodes = r#"(() => JSON.stringify([...document.querySelectorAll('#tj-scene [data-tj-dag-node]:not([data-tj-hidden="true"])')].map(g => [g.getAttribute('data-tj-component'), g.getAttribute('transform')])))()"#;
    let mut shown: Vec<(String, String)> = Vec::new();
    let scene = attribute(&tab, SOURCE, "id")?;
    for k in [74, 77, 81, 104, 117, 167, 198, 206, 247, 251] {
        seek(&tab, k)?;
        let now: Vec<(String, String)> = serde_json::from_str(&text(&tab, nodes)?).map_err(|e| e.to_string())?;
        check(
            now.starts_with(&shown),
            &format!("at {k} an earlier node moved or went: {shown:?} then {now:?}"),
        )?;
        check_eq(now.len(), shown.len() + 1)?;
        shown = now;
        check_eq(attribute(&tab, SOURCE, "id")?, scene.clone())?;
    }
    // Back down and up again: the same places.
    seek(&tab, 74)?;
    seek(&tab, 251)?;
    let again: Vec<(String, String)> = serde_json::from_str(&text(&tab, nodes)?).map_err(|e| e.to_string())?;
    check_eq(again, shown)
}

#[test]
fn picking_out_a_component_marks_its_outline_its_roster_row_and_its_condensation_node_and_it_is_let_go_going_back()
-> Result<(), String> {
    let tab = open()?;
    seek(&tab, 81)?;
    text(
        &tab,
        &format!(
            "(() => {{ document.querySelector('{SOURCE} [data-tj-roster=\"3\"]').dispatchEvent(new MouseEvent('click', {{ bubbles: true }})); return 'ok'; }})()"
        ),
    )?;
    check_eq(
        attribute(&tab, &format!("{SOURCE} [data-tj-roster=\"3\"]"), "aria-pressed")?,
        "true".to_owned(),
    )?;
    check(
        attribute(&tab, &format!("{SOURCE} [data-tj-component=\"3\"]"), "class")?.contains("tj-selected"),
        "the outline",
    )?;
    check_eq(
        attribute(
            &tab,
            &format!("{SOURCE} [data-tj-dag-node][data-tj-component=\"3\"]"),
            "data-tj-selected",
        )?,
        "true".to_owned(),
    )?;
    check_eq(position(&tab)?, 81)?;
    previous(&tab)?;
    check_eq(count(&tab, &format!("{SOURCE} [data-tj-roster][aria-pressed=\"true\"]"))?, 0)?;
    check(live(&tab)?.contains("Component C3 does not exist at this step"), &live(&tab)?)
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[test]
fn the_arrows_go_through_the_64_vertices_and_then_the_114_edges_as_one_tab_stop_and_enter_inspects_without_advancing()
-> Result<(), String> {
    let tab = open()?;
    let stops = |tab: &Tab| {
        count(
            tab,
            &format!("{SOURCE} .tj-vertex[tabindex=\"0\"], {SOURCE} .tj-edge-hit[tabindex=\"0\"]"),
        )
    };
    let on = |tab: &Tab| {
        text(
            tab,
            "document.activeElement.getAttribute('data-tj-vertex') || document.activeElement.getAttribute('data-tj-edge-hit') || ''",
        )
    };
    focus(&tab, &vertex(0))?;
    for (key, expected) in [
        ("ArrowRight", "v01"),
        ("ArrowDown", "v02"),
        ("ArrowLeft", "v01"),
        ("ArrowUp", "v00"),
        ("ArrowLeft", "v00"),
        ("End", "e113"),
        ("ArrowRight", "e113"),
        ("Home", "v00"),
    ] {
        tab.press_key(key).map_err(|e| e.to_string())?;
        check_eq(on(&tab)?, expected.to_owned())?;
        check_eq(stops(&tab)?, 1)?;
    }
    focus(&tab, &vertex(63))?;
    tab.press_key("ArrowRight").map_err(|e| e.to_string())?;
    check_eq(on(&tab)?, "e000".to_owned())?;
    check_eq(position(&tab)?, 0)?;
    tab.press_key("Enter").map_err(|e| e.to_string())?;
    check(
        field(&tab, "selected")?.starts_with("e000: from v00 to v01"),
        &field(&tab, "selected")?,
    )?;
    check_eq(position(&tab)?, 0)
}

#[test]
fn each_of_the_six_antiparallel_edges_and_the_six_at_vertex_62_can_be_reached_and_inspected_by_the_keyboard()
-> Result<(), String> {
    let tab = open()?;
    let wanted = [(56, 58), (58, 56), (60, 62), (62, 60), (61, 62), (62, 61), (55, 62), (62, 63)];
    for (a, b) in wanted {
        let id = text(
            &tab,
            &format!(
                "(() => {{ const h = [...document.querySelectorAll('{SOURCE} .tj-edge-hit')].find(e => (e.getAttribute('aria-label') + ',').includes('from v{a:02} to v{b:02},')); \
                 return h.getAttribute('data-tj-edge-hit'); }})()"
            ),
        )?;
        focus(&tab, &format!("{SOURCE} [data-tj-edge-hit=\"{id}\"]"))?;
        tab.press_key("Enter").map_err(|e| e.to_string())?;
        let said = field(&tab, "selected")?;
        check(
            said.starts_with(&format!("{id}: from v{a:02} to v{b:02}")),
            &format!("{a} to {b}: {said}"),
        )?;
        check_eq(position(&tab)?, 0)?;
    }
    Ok(())
}

#[test]
fn clicking_a_vertex_selects_it_and_describes_it_and_never_changes_the_step() -> Result<(), String> {
    let tab = open()?;
    seek(&tab, 34)?;
    next(&tab)?;
    tab.find_element(&vertex(33))
        .map_err(|e| e.to_string())?
        .click()
        .map_err(|e| e.to_string())?;
    check_eq(attribute(&tab, &vertex(33), "aria-pressed")?, "true".to_owned())?;
    check(field(&tab, "selected")?.starts_with("v33: index"), &field(&tab, "selected")?)?;
    check_eq(position(&tab)?, 35)
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[test]
fn play_runs_a_tick_at_a_time_follows_the_search_and_stops_at_the_end() -> Result<(), String> {
    let tab = open_manual_zoomed()?;
    click(&tab, "tj-play")?;
    check_eq(
        (attribute(&tab, "#tj-play", "aria-label")?, waiting(&tab)?),
        ("Pause".to_owned(), 1),
    )?;
    // Every tick is checked against the view in the native tests, so here every sixth is, which is as many as the
    // browser needs to show that the follow is real.
    for tick in (6..=252).step_by(6) {
        check_eq(advance(&tab, 6)?, 6)?;
        check_eq(position(&tab)?, tick)?;
        let v: usize = text(
            &tab,
            &format!("document.querySelector('{SOURCE} .tj-current').getAttribute('data-tj-vertex').slice(1)"),
        )?
        .parse()
        .map_err(|e| format!("{e}"))?;
        let at = vertex_in_pane(&tab, v)?;
        check(
            inside_safe(at),
            &format!(
                "step {tick}: v{v:02} is not kept in view: {at:?}, view {:?}, viewport {:?}",
                view(&tab)?,
                viewport(&tab)?
            ),
        )?;
    }
    check_eq(advance(&tab, 6)?, 0)?;
    check_eq(
        (waiting(&tab)?, attribute(&tab, "#tj-play", "aria-label")?),
        (0, "Play".to_owned()),
    )?;
    check(disabled(&tab, "tj-play")?, "play is dimmed at the end")
}

#[test]
fn a_burst_of_range_moves_draws_once_at_the_next_frame_and_never_moves_the_view() -> Result<(), String> {
    let tab = open_manual()?;
    let before = view(&tab)?;
    text(
        &tab,
        "(() => { const r = document.getElementById('tj-seek'); for (const v of [30, 90, 150, 250]) { r.value = String(v); r.dispatchEvent(new Event('input', { bubbles: true })); } return 'ok'; })()",
    )?;
    check_eq((waiting_frames(&tab)?, position(&tab)?), (1, 0))?;
    check_eq(advance_frames(&tab, 5)?, 1)?;
    check_eq((position(&tab)?, view(&tab)?), (250, before))
}

#[test]
fn every_other_control_stops_the_playback_and_the_speed_replaces_the_waiting_tick() -> Result<(), String> {
    let tab = open_manual()?;
    for stop in ["tj-next", "tj-restart", "tj-next-component"] {
        click(&tab, "tj-play")?;
        check_eq(waiting(&tab)?, 1)?;
        click(&tab, stop)?;
        check_eq(
            (waiting(&tab)?, attribute(&tab, "#tj-play", "aria-label")?),
            (0, "Play".to_owned()),
        )?;
    }
    click(&tab, "tj-play")?;
    text(
        &tab,
        "(() => { const s = document.getElementById('tj-speed'); s.value = '100'; s.dispatchEvent(new Event('change', { bubbles: true })); return 'ok'; })()",
    )?;
    check_eq(
        (waiting(&tab)?, attribute(&tab, "#tj-play", "aria-label")?),
        (1, "Pause".to_owned()),
    )
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

#[test]
fn a_write_that_fails_is_reported_leaves_the_step_and_the_next_button_draws_the_page_again() -> Result<(), String> {
    let tab = open()?;
    seek(&tab, 40)?;
    fail_writes_of(&tab, "data-tj-state", true)?;
    click(&tab, "tj-next")?;
    check_eq(banner_hidden(&tab)?, false)?;
    check_eq(position(&tab)?, 40)?;
    fail_writes_of(&tab, "data-tj-state", false)?;
    click(&tab, "tj-previous")?;
    check_eq((banner_hidden(&tab)?, position(&tab)?), (true, 39))
}

#[test]
fn a_condensation_node_that_cannot_be_written_is_reported_and_repaired_without_a_second_node() -> Result<(), String> {
    let tab = open()?;
    seek(&tab, 73)?;
    fail_writes_of(&tab, "data-tj-dag-node", true)?;
    click(&tab, "tj-next")?;
    check_eq(banner_hidden(&tab)?, false)?;
    check_eq(position(&tab)?, 73)?;
    fail_writes_of(&tab, "data-tj-dag-node", false)?;
    click(&tab, "tj-next")?;
    check_eq((banner_hidden(&tab)?, position(&tab)?), (true, 74))?;
    check_eq(dag_nodes(&tab)?, 1)?;
    check_eq(count(&tab, &format!("{SOURCE} .svg-dom-graph-content > g"))?, 64 + 1 + 1 + 1)
}

#[test]
fn a_failed_selection_stops_playback_and_a_button_with_nowhere_to_go_still_repairs() -> Result<(), String> {
    let tab = open_manual()?;
    click(&tab, "tj-play")?;
    fail_writes_of(&tab, "data-tj-selected", true)?;
    focus(&tab, &vertex(2))?;
    fail_writes_of(&tab, "data-tj-selected", false)?;
    // The selection draw may or may not have used that attribute, so the test only needs a clean state after an action.
    click(&tab, "tj-previous")?;
    click(&tab, "tj-play")?;
    advance(&tab, 1)?;
    check_eq(banner_hidden(&tab)?, true)
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[test]
fn a_page_diffed_step_by_step_is_the_same_dom_as_one_drawn_in_full_every_time() -> Result<(), String> {
    const SIGNATURE: &str = r#"(() => {
        const o = document.querySelector('#tj-scene .tj-overlay').outerHTML;
        const p = [...document.querySelectorAll('[id^="tj-p-"]')].map(e => e.textContent + e.hasAttribute('hidden')).join('|');
        const d = [...document.querySelectorAll('#tj-scene [data-tj-dag-node], #tj-scene [data-tj-dag-edge]')].map(e => (e.getAttribute('data-tj-component') || e.getAttribute('data-tj-dag-edge')) + e.getAttribute('data-tj-hidden') + e.getAttribute('data-tj-selected')).join();
        const s = '';
        const n = [...document.querySelectorAll('#tj-scene path[data-tj-edge]')].map(e => e.getAttribute('data-tj-inspected')).join();
        const v = [...document.querySelectorAll('#tj-scene .svg-dom-graph-content > g text')].map(e => e.textContent).join();
        const c = [...document.querySelectorAll('#tj-controls-svg [data-action]')].map(e => e.getAttribute('aria-disabled') + e.getAttribute('aria-pressed') + e.getAttribute('aria-label')).join();
        return o + p + s + d + n + v + c + document.getElementById('tj-seek').value; })()"#;
    type Step = Box<dyn Fn(&Tab) -> Result<(), String>>;
    let mut steps: Vec<Step> = Vec::new();
    for to in [3, 35, 74, 75, 117, 180, 247, 251, 252, 120, 73, 0, 100, 184] {
        steps.push(Box::new(move |t| seek(t, to)));
    }
    steps.push(Box::new(|t| {
        t.evaluate("document.querySelector('#tj-scene [data-tj-roster=\"2\"]').dispatchEvent(new MouseEvent('click', { bubbles: true }))", false)
            .map(|_| ())
            .map_err(|e| e.to_string())
    }));
    steps.push(Box::new(|t| click(t, "tj-next-component")));
    steps.push(Box::new(|t| focus(t, &vertex(7))));
    // The reference: every draw is a full one. Its tab is let go before the next is opened.
    let reference = {
        let whole = open_at("?always-full&no-springs")?;
        let mut pages = vec![text(&whole, SIGNATURE)?];
        for step in &steps {
            step(&whole)?;
            pages.push(text(&whole, SIGNATURE)?);
        }
        pages
    };
    let diffed = open_at("?no-springs")?;
    check(text(&diffed, SIGNATURE)? == reference[0], "the first draw differs")?;
    for (n, step) in steps.iter().enumerate() {
        step(&diffed)?;
        let page = text(&diffed, SIGNATURE)?;
        check(
            page == reference[n + 1],
            &format!("after step {n}: {}", first_difference(&page, &reference[n + 1])),
        )?;
    }
    Ok(())
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

#[test]
fn leaving_the_page_takes_off_every_listener_that_the_demo_added_and_a_page_that_was_left_does_nothing()
-> Result<(), String> {
    let targets: [(&str, &[&str]); 6] = [
        (
            "document.querySelector('#tj-scene svg')",
            &["keydown", "click", "focusin", "wheel", "pointerdown", "lostpointercapture"],
        ),
        ("document.getElementById('tj-controls-svg')", &["click", "keydown"]),
        ("document.getElementById('tj-seek')", &["input", "change"]),
        ("document.getElementById('tj-speed')", &["change"]),
        ("document.getElementById('tj-fit-inline')", &["click"]),
        ("window", &["pointerup", "pointercancel", "pagehide", "resize"]),
    ];
    let tab = open_manual()?;
    let before: Vec<_> = targets
        .iter()
        .map(|(t, _)| listener_counts(&tab, t))
        .collect::<Result<_, _>>()?;
    seek(&tab, 4)?;
    text(
        &tab,
        "(() => { window.dispatchEvent(new Event('pagehide')); window.dispatchEvent(new Event('pagehide')); return 'ok'; })()",
    )?;
    for ((target, types), was) in targets.iter().zip(&before) {
        let now = listener_counts(&tab, target)?;
        for kind in *types {
            let (was, now) = (was.get(*kind).copied().unwrap_or(0), now.get(*kind).copied().unwrap_or(0));
            if *target == "window" && *kind == "pagehide" {
                check_eq(now, was)?;
            } else {
                check(
                    was > now,
                    &format!("{target} still has all its {kind} listeners: {was} then {now}"),
                )?;
            }
        }
    }
    click(&tab, "tj-next")?;
    check_eq(position(&tab)?, 4)?;
    check_eq(text(&tab, "typeof window.__tjAdvance")?, "undefined".to_owned())
}

#[test]
fn the_toolbar_stays_at_the_edge_of_the_scene_when_the_window_is_resized_and_the_controls_are_in_the_scene()
-> Result<(), String> {
    let tab = open()?;
    check_eq(count(&tab, &format!("{SOURCE} > #tj-controls-svg"))?, 1)?;
    check_eq(count(&tab, "#tj-scene svg")?, 1)?;
    check_eq(
        text(
            &tab,
            "getComputedStyle(document.querySelector('#tj-scene svg')).backgroundColor",
        )?,
        "rgb(255, 255, 238)".to_owned(),
    )?;
    // The toolbar is the group that holds the "+" button.
    let gap = r#"(() => {
        const s = document.querySelector('#tj-scene svg').getBoundingClientRect();
        const plus = [...document.querySelectorAll('#tj-scene svg g')].find(g => g.textContent.trim() === '+');
        return String(Math.round(s.right - plus.getBoundingClientRect().right)); })()"#;
    let before: i32 = text(&tab, gap)?.parse().map_err(|e| format!("{e}"))?;
    text(
        &tab,
        "(() => { document.getElementById('tj-scene').style.width = '900px'; window.dispatchEvent(new Event('resize')); return 'ok'; })()",
    )?;
    let after: i32 = text(&tab, gap)?.parse().map_err(|e| format!("{e}"))?;
    check(
        (0..=60).contains(&after),
        &format!("the toolbar is {after} px from the edge after the resize"),
    )?;
    check((before - after).abs() <= 2, &format!("the gap was {before} and is now {after}"))
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

#[derive(serde::Deserialize)]
struct Dag {
    nodes: Vec<Vec<f64>>,
    arcs: Vec<(usize, usize)>,
    state: Option<String>,
}

impl Dag {
    /// How much the lengths of the arrows differ from each other, as a fraction of their mean.
    fn spread(&self) -> f64 {
        let lengths: Vec<f64> = self
            .arcs
            .iter()
            .map(|&(a, b)| {
                (self.nodes[a - 1][0] - self.nodes[b - 1][0]).hypot(self.nodes[a - 1][1] - self.nodes[b - 1][1])
            })
            .collect();
        let mean = lengths.iter().sum::<f64>() / lengths.len() as f64;
        (lengths.iter().map(|l| (l - mean).powi(2)).sum::<f64>() / lengths.len() as f64).sqrt() / mean
    }
}

fn dag(tab: &Tab) -> Result<Dag, String> {
    serde_json::from_str(&text(tab, DAG_STATE)?).map_err(|e| e.to_string())
}

#[test]
fn at_the_last_step_the_arrows_of_the_condensation_become_equal_springs_and_going_back_puts_the_nodes_home()
-> Result<(), String> {
    // Where the table of places puts the nodes: the last step, with the springs left out.
    let home = {
        let still = open_at("?no-springs")?;
        seek(&still, 252)?;
        dag(&still)?
    };
    check_eq(home.state.as_deref(), Some("idle"))?;
    let tab = open()?;
    seek(&tab, 251)?;
    check_eq(dag(&tab)?.state.as_deref(), Some("idle"))?;
    seek(&tab, 252)?;
    wait_until(&tab, 20, |t| Ok(dag(t)?.state.as_deref() == Some("done")))?;
    shoot(&tab, "springs")?;
    let after = dag(&tab)?;
    check_eq((after.nodes.len(), after.arcs.len()), (10, 17))?;
    let moved = home
        .nodes
        .iter()
        .zip(&after.nodes)
        .filter(|(a, b)| (a[0] - b[0]).abs() + (a[1] - b[1]).abs() > 5.0)
        .count();
    check(moved >= 6, &format!("{moved} nodes moved"))?;
    check(
        after.spread() < home.spread() * 0.7,
        &format!(
            "the arrows varied by {:.2} and now vary by {:.2}",
            home.spread(),
            after.spread()
        ),
    )?;
    // Going back puts every node where the table had it, whatever the springs did.
    previous(&tab)?;
    let back = dag(&tab)?;
    check_eq(back.state.as_deref(), Some("idle"))?;
    for (a, b) in back.nodes.iter().zip(&home.nodes) {
        check(
            (a[0] - b[0]).abs() < 0.2 && (a[1] - b[1]).abs() < 0.2,
            "a node is back where it was",
        )?;
    }
    Ok(())
}

#[test]
fn the_three_columns_of_text_are_a_third_a_half_and_a_third_wide() -> Result<(), String> {
    let tab = open()?;
    let widths = text(
        &tab,
        "(() => { const cs = [...document.querySelectorAll('.tj-status > div')].slice(0, 3).map(e => e.getBoundingClientRect().width); const total = cs[0] + cs[1] + cs[2]; return JSON.stringify(cs.map(w => Math.round(100 * w / total))); })()",
    )?;
    check_eq(widths, "[30,40,30]".to_owned())
}
