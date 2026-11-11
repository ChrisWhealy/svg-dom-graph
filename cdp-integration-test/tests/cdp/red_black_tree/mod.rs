//! The red-black tree panel in the built demo gallery: a real `Scene` of circles and connectors, drawn again for every
//! step, with a semantic tree above it that the keyboard and a screen reader use.
//!
//! Native unit tests cover the algorithm, the rules of the controls, the resolution of a choice and the keyboard. What
//! they cannot reach is the browser. These tests check the following:
//!
//! - Every step, forward, backward and by a direct seek, draws the same: one circle for each key, one connector for
//!   each parent link, and the connectors' real endpoints on the circles that the links name, within a tolerance.
//! - The checkpoints have exactly the golden links, so a rotation or a transplant leaves no stale or duplicate
//!   connector.
//! - The semantic tree has the right roles, levels, positions and names, one tab stop, and the picture is left out of
//!   it.
//! - The keyboard, pressed for real, moves through the tree as specified, and Tab enters and leaves it once.
//! - A selection survives the deletion: 15 shows as 20 and comes back on reversing, and choosing 20 stops that.
//! - The focus follows its key through a swap, a button keeps its focus through every step, and nothing steals it.
//! - Play runs one tick at a time on a clock that the test advances, and stops at the end and at every other control.
//! - A write that fails, a swap that throws before or after it changes the stage, or a focus that cannot be put back,
//!   is reported, keeps the step where it was, and is put right by the next action.
//! - Pan and zoom are off, and the hidden place the tree is built in is empty between steps.
//!
//! Like [`crate::edmonds_karp`], these run against the built demo gallery.

use crate::common::{demo_tab::DemoTab, new_demo_tab};
use headless_chrome::Tab;
use serde_json::Value;
use std::time::{Duration, Instant};

const STAGE: &str = "#rbt-stage";
const VIEWPORT: (u32, u32) = (1400, 1700);

fn open_at(query: &str) -> Result<DemoTab, String> {
    new_demo_tab(
        &format!("index.html{query}#panel-red-black-tree"),
        &format!("{STAGE} .rbt-board"),
        VIEWPORT,
    )
}

fn open() -> Result<DemoTab, String> {
    open_at("")
}

/// The panel with the playback on a clock that only the test advances.
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
    content(tab, &format!("#rbt-p-{name}"))
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
    Ok(content(tab, "#rbt-live")?.replace('\u{a0}', ""))
}

fn banner_hidden(tab: &Tab) -> Result<bool, String> {
    Ok(text(tab, "String(document.getElementById('rbt-error').hidden)")? == "true")
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

fn click_item(tab: &Tab, key: u16) -> Result<(), String> {
    text(
        tab,
        &format!(
            "(() => {{ document.querySelector('{STAGE} [data-rbt-key=\"{key}\"] > .rbt-hit').dispatchEvent(new \
             MouseEvent('click', {{ bubbles: true }})); return 'ok'; }})()"
        ),
    )
    .map(|_| ())
}

fn focus_item(tab: &Tab, key: u16) -> Result<(), String> {
    text(
        tab,
        &format!("(() => {{ document.querySelector('{STAGE} [data-rbt-key=\"{key}\"]').focus(); return 'ok'; }})()"),
    )
    .map(|_| ())
}

/// The key of the item that has the focus, or what else has it: a button's action, the tree, or `other`.
fn focused(tab: &Tab) -> Result<String, String> {
    text(
        tab,
        "(() => { const a = document.activeElement; if (!a) return 'none'; \
         const k = a.closest && a.closest('[data-rbt-key]'); if (k) return 'key:' + k.getAttribute('data-rbt-key'); \
         if (a.classList && a.classList.contains('rbt-overlay')) return 'tree'; \
         if (a.getAttribute && a.getAttribute('data-action')) return 'button:' + a.getAttribute('data-action'); \
         return a.id ? 'id:' + a.id : 'other'; })()",
    )
}

/// Moves the range to `to` as a person does who drags and lets go.
fn seek(tab: &Tab, to: usize) -> Result<(), String> {
    text(
        tab,
        &format!(
            "(() => {{ const r = document.getElementById('rbt-seek'); r.value = '{to}'; \
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

fn press(tab: &Tab, key: &str) -> Result<(), String> {
    tab.press_key(key)
        .map(|_| ())
        .map_err(|e| format!("could not press {key}: {e}"))
}

fn advance(tab: &Tab, n: usize) -> Result<usize, String> {
    text(tab, &format!("String(window.__rbtAdvance({n}))"))?
        .parse()
        .map_err(|e| format!("{e}"))
}

fn waiting(tab: &Tab) -> Result<usize, String> {
    text(tab, "String(window.__rbtPending())")?.parse().map_err(|e| format!("{e}"))
}

/// Makes `setAttribute` throw for `name`, or stops it.
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

/// Makes the browser's own `replaceChildren` throw, which is the call that swaps the tree in. With `after` it throws
/// once the stage has already been changed, and without it, before anything is.
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

/// Makes `focus()` do nothing, which stands in for a focus that cannot be put back.
fn fail_focus(tab: &Tab, failing: bool) -> Result<(), String> {
    let script = if failing {
        "(() => { const p = SVGElement.prototype; if (!p.__focused) { p.__focused = p.focus; } \
         p.focus = function () {}; return 'ok'; })()"
    } else {
        "(() => { const p = SVGElement.prototype; if (p.__focused) { p.focus = p.__focused; } return 'ok'; })()"
    };
    text(tab, script).map(|_| ())
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The connectors as they are drawn: for each one the key of the circle its start is on, the key of the circle its end
/// is on, and how far each end is from where the straight line between the two centres crosses the outline, in the
/// units of the scene. The ends are read from the real path with `getPointAtLength`, in screen space.
fn connectors(tab: &Tab) -> Result<Vec<(u16, u16, f64)>, String> {
    let report = json(
        tab,
        r#"(() => {
        const svg = document.querySelector('#rbt-stage .rbt-scene');
        const nodes = [...svg.querySelectorAll('ellipse')].map(e => {
            const m = e.getScreenCTM(), r = e.getBoundingClientRect();
            const key = parseInt(e.parentElement.querySelector('text').textContent);
            return { key, cx: r.left + r.width / 2, cy: r.top + r.height / 2, r: parseFloat(e.getAttribute('rx')) * m.a };
        });
        const boundary = (n, q) => { const dx = q.cx - n.cx, dy = q.cy - n.cy, d = Math.hypot(dx, dy);
            return { x: n.cx + n.r * dx / d, y: n.cy + n.r * dy / d }; };
        const nearest = p => nodes.map(n => ({ n, e: Math.abs(Math.hypot(p.x - n.cx, p.y - n.cy) - n.r) }))
            .sort((a, b) => a.e - b.e)[0].n;
        return JSON.stringify([...svg.querySelectorAll('path[marker-end]')].map(p => {
            const m = p.getScreenCTM(), total = p.getTotalLength();
            const at = l => { const q = p.getPointAtLength(l);
                return { x: m.a * q.x + m.c * q.y + m.e, y: m.b * q.x + m.d * q.y + m.f }; };
            const s = at(0), e = at(total), a = nearest(s), b = nearest(e);
            const es = boundary(a, b), ee = boundary(b, a);
            return [a.key, b.key, Math.max(Math.hypot(s.x - es.x, s.y - es.y), Math.hypot(e.x - ee.x, e.y - ee.y)) / m.a];
        }));
    })()"#,
    )?;
    report
        .as_array()
        .ok_or("no report")?
        .iter()
        .map(|row| {
            let r = row.as_array().ok_or("a row")?;
            Ok((
                r[0].as_u64().ok_or("a key")? as u16,
                r[1].as_u64().ok_or("a key")? as u16,
                r[2].as_f64().ok_or("an error")?,
            ))
        })
        .collect()
}

/// The links that the semantic tree says, as `(parent, child)` in order.
fn tree_links(tab: &Tab) -> Result<Vec<(u16, u16)>, String> {
    let links = json(
        tab,
        r#"(() => JSON.stringify([...document.querySelectorAll('#rbt-stage [role=treeitem]')].flatMap(g => {
            const p = g.parentElement.closest('[role=treeitem]');
            return p ? [[+p.getAttribute('data-rbt-key'), +g.getAttribute('data-rbt-key')]] : [];
        })))()"#,
    )?;
    let mut pairs: Vec<(u16, u16)> = links
        .as_array()
        .ok_or("links")?
        .iter()
        .map(|r| (r[0].as_u64().unwrap_or(0) as u16, r[1].as_u64().unwrap_or(0) as u16))
        .collect();
    pairs.sort_unstable();
    Ok(pairs)
}

/// The drawn connectors as a sorted list of `(parent, child)`, after checking that each is within the tolerance.
fn drawn_links(tab: &Tab) -> Result<Vec<(u16, u16)>, String> {
    let mut pairs = Vec::new();
    for (from, to, error) in connectors(tab)? {
        check(
            error <= 1.5,
            &format!("the connector {from}→{to} ends {error:.2} units from the outline"),
        )?;
        pairs.push((from, to));
    }
    pairs.sort_unstable();
    Ok(pairs)
}

/// How far each hit circle of the tree is from the centre of the circle of its key in the picture.
fn overlay_error(tab: &Tab) -> Result<f64, String> {
    text(
        tab,
        r#"(() => { const svg = document.querySelector('#rbt-stage .rbt-scene'); let worst = 0;
        for (const e of svg.querySelectorAll('ellipse')) {
            const key = parseInt(e.parentElement.querySelector('text').textContent);
            const hit = document.querySelector('#rbt-stage [data-rbt-key="' + key + '"] > .rbt-hit');
            const a = e.getBoundingClientRect(), b = hit.getBoundingClientRect();
            worst = Math.max(worst, Math.hypot(a.left + a.width / 2 - (b.left + b.width / 2), a.top + a.height / 2 - (b.top + b.height / 2)));
        } return String(worst); })()"#,
    )?
    .parse()
    .map_err(|e| format!("{e}"))
}

/// Everything about a step that a person can tell, as one string, so that two ways of reaching a step can be compared.
fn digest(tab: &Tab) -> Result<String, String> {
    let links = drawn_links(tab)?;
    let items = text(
        tab,
        r#"(() => JSON.stringify([...document.querySelectorAll('#rbt-stage [role=treeitem]')].map(g => [
            g.getAttribute('data-rbt-key'), g.getAttribute('data-rbt-colour'), g.getAttribute('aria-level'),
            g.getAttribute('aria-posinset'), g.getAttribute('aria-setsize'), g.getAttribute('aria-selected'),
            g.getAttribute('tabindex'), g.getAttribute('aria-label')])))()"#,
    )?;
    let circles = text(
        tab,
        r#"(() => JSON.stringify([...document.querySelectorAll('#rbt-stage .rbt-scene ellipse')].map(e => [
            e.parentElement.querySelector('text').textContent, e.getAttribute('fill'),
            e.parentElement.getAttribute('transform')]).sort()))()"#,
    )?;
    Ok(format!(
        "{links:?}|{items}|{circles}|{}|{}|{}",
        field(tab, "state")?,
        field(tab, "event")?,
        field(tab, "selected")?
    ))
}

mod unit_tests;
