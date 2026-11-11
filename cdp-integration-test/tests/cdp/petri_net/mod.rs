//! The Petri net panel in the built demo gallery: a scene with selectors, buttons and a range above the net, a text
//! panel beside it, three scripts and a manual branch, and a playback that a test can drive one tick at a time.
//!
//! Native unit tests cover the rules of the net, the scripts, what each snapshot shows, the layout and every rule of
//! the controller. What they cannot reach is the browser. These tests check the following:
//!
//! - The real scene has 20 places, 18 gates and 50 connectors, whose ends are where the layout says, with the Free arc
//!   above the Used arc at every gate, and with every spoke clear of the hub that it does not belong to.
//! - The library's own nodes are hidden from assistive technology, and every place and gate has a name, a status and
//!   one tab stop between them.
//! - The scripts show what the design says at its named checkpoints: a blocked producer and its release, two producers
//!   that compete for the last slot, and workers that overlap.
//! - Selecting never fires. In manual mode a click, a key and a double click fire once, a disabled transition says what
//!   it is missing, and returning restores the script where the branch began.
//! - Play runs one tick at a time on a clock that the test advances, and stops at the end and in manual mode.
//! - A write that fails is reported, leaves the snapshot where it was, and is put right by the next action.
//! - A page that was diffed step by step is the same, as the DOM, as one that was drawn in full at every step.
//!
//! Like [`crate::edmonds_karp`], these run against the built demo gallery.

mod arc;
mod bar;
mod hub;
mod measured;

use crate::common::{demo_tab::DemoTab, new_demo_tab};
use crate::petri_net::measured::Measured;
use headless_chrome::Tab;
use std::time::{Duration, Instant};

const SVG: &str = "#pn-stage-svg";
const VIEWPORT: (u32, u32) = (1400, 1500);

fn open_at(query: &str) -> Result<DemoTab, String> {
    new_demo_tab(
        &format!("index.html{query}#panel-petri-net"),
        &format!("{SVG} .pn-overlay"),
        VIEWPORT,
    )
}

fn open() -> Result<DemoTab, String> {
    open_at("")
}

/// The panel with the playback and the frames on a clock that only the test advances.
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

/// One field of the text panel, by the name the page gives it: `snapshot`, `history`, `event`, `enabled` and so on.
fn field(tab: &Tab, name: &str) -> Result<String, String> {
    content(tab, &format!("#pn-p-{name}"))
}

fn live(tab: &Tab) -> Result<String, String> {
    Ok(content(tab, "#pn-live")?.replace('\u{a0}', ""))
}

/// The snapshot that the page shows, which the first number of the panel says.
fn position(tab: &Tab) -> Result<usize, String> {
    field(tab, "snapshot")?
        .split(" of ")
        .next()
        .and_then(|n| n.parse().ok())
        .ok_or_else(|| "the snapshot field has no number".to_owned())
}

fn last_snapshot(tab: &Tab) -> Result<usize, String> {
    field(tab, "snapshot")?
        .split(" of ")
        .nth(1)
        .and_then(|n| n.parse().ok())
        .ok_or_else(|| "the snapshot field has no last number".to_owned())
}

fn place(name: &str) -> String {
    format!("{SVG} [data-pn-place=\"{name}\"]")
}

fn gate(name: &str) -> String {
    format!("{SVG} [data-pn-transition=\"{name}\"]")
}

fn status_of(tab: &Tab, name: &str) -> Result<String, String> {
    attribute(tab, &gate(name), "data-pn-status")
}

fn click(tab: &Tab, id: &str) -> Result<(), String> {
    text(
        tab,
        &format!(
            "(() => {{ document.getElementById('{id}').dispatchEvent(new MouseEvent('click', {{ bubbles: true }})); \
             return 'ok'; }})()"
        ),
    )
    .map(|_| ())
}

/// A click, and a double click, on the hit area of a target, as the browser sends them.
fn double_click(tab: &Tab, selector: &str) -> Result<(), String> {
    text(
        tab,
        &format!(
            "(() => {{ const t = document.querySelector('{selector} .pn-hit'); \
             for (const type of ['click', 'click', 'dblclick']) {{ \
               t.dispatchEvent(new MouseEvent(type, {{ bubbles: true, detail: type === 'dblclick' ? 2 : 1 }})); }} \
             return 'ok'; }})()"
        ),
    )
    .map(|_| ())
}

fn seek(tab: &Tab, to: usize) -> Result<(), String> {
    text(
        tab,
        &format!(
            "(() => {{ const r = document.getElementById('pn-seek'); r.value = '{to}'; \
             r.dispatchEvent(new Event('input', {{ bubbles: true }})); \
             r.dispatchEvent(new Event('change', {{ bubbles: true }})); return 'ok'; }})()"
        ),
    )?;
    wait_until(tab, 5, |tab| Ok(position(tab)? == to))
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

fn select(tab: &Tab, id: &str, value: &str) -> Result<(), String> {
    text(
        tab,
        &format!(
            "(() => {{ const s = document.getElementById('{id}'); s.value = '{value}'; \
             s.dispatchEvent(new Event('change', {{ bubbles: true }})); return 'ok'; }})()"
        ),
    )
    .map(|_| ())
}

fn checkpoint(tab: &Tab, n: usize) -> Result<(), String> {
    select(tab, "pn-checkpoint", &n.to_string())
}

fn scenario(tab: &Tab, key: &str) -> Result<(), String> {
    select(tab, "pn-scenario", key)
}

fn advance(tab: &Tab, n: usize) -> Result<usize, String> {
    text(tab, &format!("String(window.__pnAdvance({n}))"))?
        .parse()
        .map_err(|e| format!("{e}"))
}

fn advance_frames(tab: &Tab, n: usize) -> Result<usize, String> {
    text(tab, &format!("String(window.__pnAdvanceFrame({n}))"))?
        .parse()
        .map_err(|e| format!("{e}"))
}

fn waiting(tab: &Tab) -> Result<usize, String> {
    text(tab, "String(window.__pnPending())")?.parse().map_err(|e| format!("{e}"))
}

fn waiting_frames(tab: &Tab) -> Result<usize, String> {
    text(tab, "String(window.__pnPendingFrames())")?
        .parse()
        .map_err(|e| format!("{e}"))
}

fn focus(tab: &Tab, selector: &str) -> Result<(), String> {
    text(tab, &format!("document.querySelector('{selector}').focus(), 'ok'")).map(|_| ())
}

fn focused(tab: &Tab) -> Result<String, String> {
    text(
        tab,
        "(() => { const a = document.activeElement; return a.getAttribute('data-pn-place') || \
         a.getAttribute('data-pn-transition') || a.id || a.tagName; })()",
    )
}

fn banner_hidden(tab: &Tab) -> Result<bool, String> {
    Ok(attribute(tab, "#pn-error", "hidden")? != "null")
}

fn play_label(tab: &Tab) -> Result<String, String> {
    attribute(tab, "#pn-play", "aria-label")
}

fn disabled(tab: &Tab, id: &str) -> Result<bool, String> {
    Ok(attribute(tab, &format!("#{id}"), "aria-disabled")? == "true")
}

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

/// Writes a screenshot to the directory in `PN_SHOT`, if it is set, so that a person can look at the picture.
fn shoot(tab: &Tab, name: &str) -> Result<(), String> {
    let Ok(dir) = std::env::var("PN_SHOT") else {
        return Ok(());
    };
    tab.evaluate("document.querySelector('#pn-stage').scrollIntoView()", false)
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

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The measured geometry of the net, as one JSON string: each gate's bar, each arc's ends, and the clearances.
const MEASURE: &str = "(() => {
  const svg = document.querySelector('#pn-stage-svg');
  const num = s => s.match(/-?[0-9.]+/g).map(Number);
  const bars = {};
  for (const g of svg.querySelectorAll('.pn-gate')) {
    const b = g.querySelector('.pn-bar');
    bars[g.getAttribute('data-pn-transition')] = {x: +b.getAttribute('x'), y: +b.getAttribute('y'), w: +b.getAttribute('width'), h: +b.getAttribute('height')};
  }
  const hubs = {};
  for (const name of ['Free', 'Used']) {
    const c = svg.querySelector('[data-pn-place=\"' + name + '\"] .pn-hit');
    hubs[name] = {x: +c.getAttribute('cx'), y: +c.getAttribute('cy'), r: +c.getAttribute('r')};
  }
  const arcs = [...svg.querySelectorAll('path[data-pn-arc]')].map(p => { const n = num(p.getAttribute('d')); return {i: +p.getAttribute('data-pn-arc'), n}; });
  return JSON.stringify({bars, hubs, arcs});
})()";

fn measured(tab: &Tab) -> Result<Measured, String> {
    serde_json::from_str(&text(tab, MEASURE)?).map_err(|e| format!("{e}"))
}

/// The distance from the point `(px, py)` to the segment from `(ax, ay)` to `(bx, by)`.
fn distance_to_segment(p: (f64, f64), a: (f64, f64), b: (f64, f64)) -> f64 {
    let (dx, dy) = (b.0 - a.0, b.1 - a.1);
    let length_squared = dx * dx + dy * dy;
    let t = if length_squared == 0.0 {
        0.0
    } else {
        (((p.0 - a.0) * dx + (p.1 - a.1) * dy) / length_squared).clamp(0.0, 1.0)
    };
    ((p.0 - (a.0 + t * dx)).powi(2) + (p.1 - (a.1 + t * dy)).powi(2)).sqrt()
}

/// The names of the gate and the place at the ends of arc `i`, from the order that the scene draws them in: for each
/// gate in order, its inputs and then its outputs.
fn arc_ends() -> Vec<(String, String, bool)> {
    let place = |s: &str| s.to_owned();
    let mut arcs = Vec::new();
    for i in 0..4 {
        arcs.push((format!("Start[{i}]"), place(&format!("P{i}.Idle")), true));
        arcs.push((format!("Start[{i}]"), place(&format!("P{i}.Busy")), false));
        arcs.push((format!("Finish[{i}]"), place(&format!("P{i}.Busy")), true));
        arcs.push((format!("Finish[{i}]"), place(&format!("P{i}.Ready")), false));
        arcs.push((format!("Put[{i}]"), place(&format!("P{i}.Ready")), true));
        arcs.push((format!("Put[{i}]"), place("Free"), true));
        arcs.push((format!("Put[{i}]"), place(&format!("P{i}.Idle")), false));
        arcs.push((format!("Put[{i}]"), place("Used"), false));
    }
    for j in 0..3 {
        arcs.push((format!("Take[{j}]"), place(&format!("C{j}.Idle")), true));
        arcs.push((format!("Take[{j}]"), place("Used"), true));
        arcs.push((format!("Take[{j}]"), place(&format!("C{j}.Processing")), false));
        arcs.push((format!("Take[{j}]"), place("Free"), false));
        arcs.push((format!("Complete[{j}]"), place(&format!("C{j}.Processing")), true));
        arcs.push((format!("Complete[{j}]"), place(&format!("C{j}.Idle")), false));
    }
    arcs
}

/// One thing that a person does to the page, which the parity test does to two pages in turn.
type Step = Box<dyn Fn(&Tab) -> Result<(), String>>;

/// Where two pages first differ, with some of what is around it.
fn first_difference(a: &str, b: &str) -> String {
    let at = a.chars().zip(b.chars()).take_while(|(x, y)| x == y).count();
    let around = |s: &str| s.chars().skip(at.saturating_sub(80)).take(200).collect::<String>();
    format!("at {at}: diffed ...{}... but full ...{}...", around(a), around(b))
}

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

mod unit_tests;
