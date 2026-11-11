//! The Edmonds-Karp panel in the built demo gallery: a scene with selectors, buttons and a range above a graph, a text
//! panel beside it, two graphs, and a playback that a test can drive one tick at a time.
//!
//! Native unit tests cover the algorithm, what each step shows, the lanes, and every rule of the controls. What they
//! cannot reach is the browser. These tests check the following:
//!
//! - The controls are in the scene, the graph is built in layers with stable `data-ek-*` locators and IDs that carry
//!   the instance, the generation and the graph, and every vertex and edge has a name.
//! - The canonical run reaches a flow of 23 and a cut of 23, with S and T shaded and only the edges that cross from S
//!   to T marked. The second graph shows its reverse transition, and e0 going from 2 to 1.
//! - The keyboard moves between the vertices and the edges in list order as one Tab stop, a click on a dashed
//!   transition focuses its edge, and neither changes the step.
//! - The toggles change the picture and not the run, and the queue is always in the text.
//! - The buttons step, jump between augmentations and to the cut, a button at an end is dimmed but keeps the focus, and
//!   says so.
//! - The range draws once for a burst of moves, at the next frame, and says where it rests once.
//! - Play runs one tick at a time on a clock that the test advances, and stops at the end and at every other control.
//! - A write that fails is reported, leaves the step where it was, and is put right by the next action, even one with
//!   nowhere to go. A switch of graph that fails, in a control or in the replacement, leaves the old graph whole.
//! - A page that was diffed step by step is the same, as the DOM, as one that was drawn in full at every step, at every
//!   step and after seeks in any order.
//! - The manual clock's hooks come off the window when the page is left.
//!
//! Like [`crate::game_of_life`], these run against the built demo gallery.

use crate::common::{demo_tab::DemoTab, new_demo_tab};
use headless_chrome::Tab;
use std::time::{Duration, Instant};

const SVG: &str = "#ek-stage-svg";
const VIEWPORT: (u32, u32) = (1400, 1700);

fn open_at(query: &str) -> Result<DemoTab, String> {
    new_demo_tab(
        &format!("index.html{query}#panel-edmonds-karp"),
        &format!("{SVG} .ek-root"),
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

fn attribute(tab: &Tab, selector: &str, name: &str) -> Result<String, String> {
    text(
        tab,
        &format!("String(document.querySelector('{selector}').getAttribute('{name}'))"),
    )
}

fn content(tab: &Tab, selector: &str) -> Result<String, String> {
    text(tab, &format!("document.querySelector('{selector}').textContent"))
}

/// The text of one field of the panel, by the name that the page gives it: `step`, `phase`, `event`, `queue` and so on.
fn field(tab: &Tab, name: &str) -> Result<String, String> {
    content(tab, &format!("#ek-p-{name}"))
}

fn position(tab: &Tab) -> Result<usize, String> {
    field(tab, "step")?
        .split_whitespace()
        .nth(1)
        .and_then(|n| n.parse().ok())
        .ok_or_else(|| "the step is not a number".to_owned())
}

fn last_step(tab: &Tab) -> Result<usize, String> {
    field(tab, "step")?
        .split_whitespace()
        .nth(3)
        .and_then(|n| n.parse().ok())
        .ok_or_else(|| "the last step is not a number".to_owned())
}

fn live(tab: &Tab) -> Result<String, String> {
    Ok(content(tab, "#ek-live")?.replace('\u{a0}', ""))
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

/// Moves the range to `to` as a person does who drags and lets go, and waits for the step to be shown.
fn seek(tab: &Tab, to: usize) -> Result<(), String> {
    text(
        tab,
        &format!(
            "(() => {{ const r = document.getElementById('ek-seek'); r.value = '{to}'; \
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

fn advance(tab: &Tab, n: usize) -> Result<usize, String> {
    text(tab, &format!("String(window.__ekAdvance({n}))"))?
        .parse()
        .map_err(|e| format!("{e}"))
}

fn advance_frames(tab: &Tab, n: usize) -> Result<usize, String> {
    text(tab, &format!("String(window.__ekAdvanceFrame({n}))"))?
        .parse()
        .map_err(|e| format!("{e}"))
}

fn waiting(tab: &Tab) -> Result<usize, String> {
    text(tab, "String(window.__ekPending())")?.parse().map_err(|e| format!("{e}"))
}

fn waiting_frames(tab: &Tab) -> Result<usize, String> {
    text(tab, "String(window.__ekPendingFrames())")?
        .parse()
        .map_err(|e| format!("{e}"))
}

fn edge(e: usize) -> String {
    format!("{SVG} g.ek-edge[data-ek-edge=\"e{e}\"]")
}

fn vertex(name: &str) -> String {
    format!("{SVG} g.ek-node[data-ek-vertex=\"{name}\"]")
}

fn lane(e: usize, d: &str) -> String {
    format!("{SVG} g.ek-res[data-ek-edge=\"e{e}\"][data-ek-direction=\"{d}\"]")
}

fn class_of(tab: &Tab, selector: &str) -> Result<String, String> {
    attribute(tab, selector, "class")
}

fn has_class(tab: &Tab, selector: &str, class: &str) -> Result<bool, String> {
    Ok(class_of(tab, selector)?.split_whitespace().any(|c| c == class))
}

/// The label of the original edge `e`, as it is written beside it. The labels follow the edges in the same order.
fn edge_label(tab: &Tab, e: usize) -> Result<String, String> {
    text(
        tab,
        &format!("document.querySelectorAll('{SVG} .ek-labels .ek-label')[{e}].textContent"),
    )
}

fn play_label(tab: &Tab) -> Result<String, String> {
    content(tab, "#ek-play")
}

fn banner_hidden(tab: &Tab) -> Result<bool, String> {
    Ok(text(tab, "String(document.getElementById('ek-error').hidden)")? == "true")
}

fn focused(tab: &Tab) -> Result<String, String> {
    text(
        tab,
        "document.activeElement ? (document.activeElement.id || document.activeElement.tagName) : ''",
    )
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

/// Makes `removeAttribute` throw for `name`, or stops it.
fn fail_removals_of(tab: &Tab, name: &str, failing: bool) -> Result<(), String> {
    let script = if failing {
        format!(
            "(() => {{ const p = Element.prototype; if (!p.__removed) {{ p.__removed = p.removeAttribute; }} \
             p.removeAttribute = function (n) {{ if (n === '{name}') {{ throw new Error('injected'); }} \
             return p.__removed.apply(this, arguments); }}; return 'ok'; }})()"
        )
    } else {
        "(() => { const p = Element.prototype; if (p.__removed) { p.removeAttribute = p.__removed; } return 'ok'; })()"
            .to_owned()
    };
    text(tab, &script).map(|_| ())
}

/// Makes the browser's own `replaceChildren` throw, which is the call that replaces the scene. With `after` it throws
/// once the host has already been changed, and without it, before anything is.
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

fn root_id(tab: &Tab) -> Result<String, String> {
    attribute(tab, &format!("{SVG} .ek-root"), "id")
}

// The same page, by every way of getting to a step.

/// Where two pages first differ, with a little of each around it, so that a failure says what.
fn first_difference(a: &str, b: &str) -> String {
    let at = a
        .chars()
        .zip(b.chars())
        .position(|(x, y)| x != y)
        .unwrap_or(a.len().min(b.len()));
    let around = |s: &str| s.chars().skip(at.saturating_sub(80)).take(220).collect::<String>();
    format!("at {at}: diffed {:?} but drawn in full {:?}", around(a), around(b))
}

/// What a person could see and hear of the page: the whole scene, with the attributes of every element in order of
/// their names, since the order that they were written in is not part of the page, and every field of the text panel.
fn page_html(tab: &Tab) -> Result<String, String> {
    text(
        tab,
        "(() => { const ser = (n) => { \
           if (n.nodeType === 3) { return n.textContent; } \
           if (n.nodeType !== 1) { return ''; } \
           const attrs = Array.from(n.attributes).map(a => a.name + '=' + a.value); \
           if (n.tagName === 'INPUT' || n.tagName === 'SELECT') { attrs.push('value=' + n.value); } \
           let s = '<' + n.tagName + ' ' + attrs.sort().join(' ') + '>'; \
           for (const c of n.childNodes) { s += ser(c); } \
           return s + '</' + n.tagName + '>'; }; \
           return ser(document.getElementById('ek-stage-svg')) + '|' + \
             Array.from(document.querySelectorAll('.ek-status dd')).map(d => d.textContent).join('|'); })()",
    )
}

mod unit_tests;
