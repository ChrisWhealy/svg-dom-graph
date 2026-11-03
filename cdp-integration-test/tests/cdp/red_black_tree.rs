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

use crate::common::{DemoTab, new_demo_tab};
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

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
// The page.

#[test]
fn the_page_opens_at_the_empty_tree_with_the_picture_hidden_from_the_tree_of_roles() -> Result<(), String> {
    let tab = open()?;
    check_eq((position(&tab)?, last_step(&tab)?), (0, 29))?;
    check_eq(field(&tab, "state")?, "Settled: a valid red-black tree".to_owned())?;
    check_eq(count(&tab, &format!("{STAGE} [role=treeitem]"))?, 0)?;
    check_eq(attribute(&tab, &format!("{STAGE} .rbt-overlay"), "role")?, "tree".to_owned())?;
    check_eq(
        attribute(&tab, &format!("{STAGE} .rbt-overlay"), "aria-label")?,
        "Red-black tree, empty".to_owned(),
    )?;
    check_eq(
        attribute(&tab, &format!("{STAGE} .rbt-scene"), "aria-hidden")?,
        "true".to_owned(),
    )?;
    for (action, disabled) in [("restart", "true"), ("previous", "true"), ("next", "false"), ("play", "false")] {
        check_eq(
            attribute(&tab, &format!("[data-action=\"{action}\"]"), "aria-disabled")?,
            disabled.to_owned(),
        )?;
    }
    check(banner_hidden(&tab)?, "no error")?;
    check_eq(count(&tab, "#rbt-staging svg > *")?, 0)
}

#[test]
fn every_button_and_the_range_are_svg_in_the_stage_and_the_range_is_html_in_a_foreign_object() -> Result<(), String> {
    let tab = open()?;
    check_eq(count(&tab, "#panel-red-black-tree button")?, 0)?;
    check_eq(count(&tab, "#rbt-stage-svg g.rbt-controls > [role=button][data-action]")?, 7)?;
    check_eq(count(&tab, "#rbt-stage-svg g.rbt-controls > [role=button][tabindex]")?, 7)?;
    check_eq(count(&tab, "#rbt-stage-svg foreignObject input[type=\"range\"]#rbt-seek")?, 1)?;
    check_eq(count(&tab, "#panel-red-black-tree input")?, 1)?;
    // The controls are above the tree and outside the part that is swapped.
    let outside = text(
        &tab,
        "String(!document.querySelector('#rbt-stage .rbt-board-host').querySelector('[data-action], foreignObject'))",
    )?;
    check_eq(outside, "true".to_owned())?;
    let above = text(
        &tab,
        "(() => { const b = document.querySelector('[data-action=\"next\"]').getBoundingClientRect(); \
         const t = document.querySelector('#rbt-stage .rbt-scene').getBoundingClientRect(); \
         return String(b.bottom <= t.top); })()",
    )?;
    check_eq(above, "true".to_owned())?;
    // A swap does not replace them: the same elements are there after a step.
    text(
        &tab,
        "(() => { window.__b = document.querySelector('[data-action=\"next\"]'); window.__r = document.getElementById('rbt-seek'); return 'ok'; })()",
    )?;
    click_button(&tab, "next")?;
    check_eq(
        text(
            &tab,
            "String(window.__b === document.querySelector('[data-action=\"next\"]') && window.__r === document.getElementById('rbt-seek'))",
        )?,
        "true".to_owned(),
    )?;
    check_eq(attribute(&tab, "[data-action=\"play\"]", "aria-label")?, "Play".to_owned())
}

#[test]
fn every_button_label_fits_inside_its_button_with_room_at_each_end() -> Result<(), String> {
    let tab = open()?;
    let report = json(
        &tab,
        r#"(() => JSON.stringify([...document.querySelectorAll('#rbt-stage g.rbt-controls > [data-action]')].map(g => {
            const r = g.querySelector('rect').getBoundingClientRect(), t = g.querySelector('text').getBoundingClientRect();
            return [g.getAttribute('data-action'), t.left - r.left, r.right - t.right,
                parseFloat(g.querySelector('text').getAttribute('font-size'))];
        })))()"#,
    )?;
    let rows = report.as_array().ok_or("no report")?;
    check_eq(rows.len(), 7)?;
    for row in rows {
        let (name, left, right, size) = (
            &row[0],
            row[1].as_f64().unwrap_or(-1.0),
            row[2].as_f64().unwrap_or(-1.0),
            row[3].as_f64().unwrap_or(0.0),
        );
        check(
            left >= 4.0 && right >= 4.0,
            &format!("{name}: the label has {left:.1} and {right:.1} units of room"),
        )?;
        check(size >= 9.0, &format!("{name}: the label is too small to read: {size}"))?;
    }
    // The long ones are made smaller, and the short ones are left at the library's size.
    let size_of = |i: usize| rows[i][3].as_f64().unwrap_or(0.0);
    check(
        size_of(4) < 14.0 || size_of(5) < 14.0 || size_of(6) < 14.0,
        "a long label shrinks",
    )?;
    check_eq(size_of(0), 14.0)
}

#[test]
fn a_button_takes_enter_and_space_from_the_keyboard() -> Result<(), String> {
    let tab = open()?;
    text(
        &tab,
        "(() => { document.querySelector('[data-action=\"next\"]').focus(); return 'ok'; })()",
    )?;
    press(&tab, "Enter")?;
    check_eq(position(&tab)?, 1)?;
    press(&tab, " ")?;
    check_eq(position(&tab)?, 2)?;
    check_eq(focused(&tab)?, "button:next".to_owned())
}

#[test]
fn the_range_keys_move_one_step_at_a_time_and_its_arrow_keys_work() -> Result<(), String> {
    let tab = open()?;
    text(&tab, "(() => { document.getElementById('rbt-seek').focus(); return 'ok'; })()")?;
    press(&tab, "ArrowRight")?;
    press(&tab, "ArrowRight")?;
    check_eq(position(&tab)?, 2)?;
    check_eq(attribute(&tab, "#rbt-seek", "aria-valuetext")?, "Step 2 of 29".to_owned())?;
    check_eq(content(&tab, "#rbt-stage .scene-slider-value")?, "Step 2 of 29".to_owned())
}

#[test]
fn the_keyboard_help_is_visible_and_says_the_right_key_limit() -> Result<(), String> {
    let tab = open()?;
    let help = content(&tab, "#rbt-help")?;
    check(
        help.contains("Right goes to the first available child (left before right)"),
        &help,
    )?;
    check(help.contains("use Up/Down to reach the other child"), &help)
}

// The picture.

#[test]
fn every_step_draws_one_circle_per_key_and_one_connector_per_link_at_the_circles() -> Result<(), String> {
    let tab = open()?;
    let last = last_step(&tab)?;
    for step in 0..=last {
        seek(&tab, step)?;
        let keys = count(&tab, &format!("{STAGE} [role=treeitem]"))?;
        check_eq(count(&tab, &format!("{STAGE} .rbt-scene ellipse"))?, keys)
            .map_err(|e| format!("step {step}: {e}"))?;
        let links = drawn_links(&tab).map_err(|e| format!("step {step}: {e}"))?;
        check_eq(links.len(), keys.saturating_sub(1)).map_err(|e| format!("step {step}: {e}"))?;
        check_eq(links, tree_links(&tab)?).map_err(|e| format!("step {step}: the picture and the tree differ: {e}"))?;
        check(overlay_error(&tab)? < 0.6, &format!("step {step}: the tree is off its circles"))?;
        check_eq(count(&tab, "#rbt-staging svg > *")?, 0).map_err(|e| format!("step {step}: {e}"))?;
    }
    Ok(())
}

#[test]
fn the_checkpoints_have_exactly_the_golden_links() -> Result<(), String> {
    let tab = open()?;
    click_button(&tab, "jump-0")?;
    check_eq(drawn_links(&tab)?, vec![(5, 1), (5, 10)])?;
    check(field(&tab, "event")?.contains("Right-rotate at 10"), &field(&tab, "event")?)?;
    click_button(&tab, "jump-1")?;
    check_eq(drawn_links(&tab)?, vec![(5, 1), (5, 15), (15, 10), (15, 20)])?;
    check(field(&tab, "event")?.contains("Left-rotate at 10"), &field(&tab, "event")?)?;
    let last = last_step(&tab)?;
    seek(&tab, last)?;
    check_eq(drawn_links(&tab)?, vec![(5, 1), (5, 12), (12, 10), (12, 20)])?;
    check_eq(count(&tab, &format!("{STAGE} [data-rbt-key=\"15\"]"))?, 0)?;
    check(
        !content(&tab, &format!("{STAGE} .rbt-scene"))?.contains("15 "),
        "no circle says 15",
    )
}

#[test]
fn the_transplant_moves_the_original_node_and_the_old_connectors_are_gone() -> Result<(), String> {
    let tab = open()?;
    click_button(&tab, "jump-2")?;
    check_eq(drawn_links(&tab)?, vec![(5, 1), (5, 15), (10, 12), (15, 10), (15, 20)])?;
    click_button(&tab, "next")?;
    check(field(&tab, "event")?.contains("Transplant"), &field(&tab, "event")?)?;
    check_eq(
        field(&tab, "state")?,
        "Repair in progress: not yet a valid red-black tree".to_owned(),
    )?;
    check_eq(drawn_links(&tab)?, vec![(5, 1), (5, 20), (10, 12), (20, 10)])?;
    check_eq(
        attribute(&tab, &format!("{STAGE} [data-rbt-key=\"20\"]"), "data-rbt-colour")?,
        "red".to_owned(),
    )
}

#[test]
fn forward_backward_and_a_direct_seek_draw_the_same_step() -> Result<(), String> {
    let tab = open()?;
    let last = last_step(&tab)?;
    let mut forward = Vec::new();
    for step in 0..=last {
        seek(&tab, step)?;
        forward.push(digest(&tab)?);
    }
    for step in (0..=last).rev() {
        seek(&tab, step)?;
        check_eq(digest(&tab)?, forward[step].clone()).map_err(|e| format!("backward at {step}: {e}"))?;
    }
    for step in [17, 3, 28, 0, 22, 9, 29, 14] {
        seek(&tab, step)?;
        check_eq(digest(&tab)?, forward[step].clone()).map_err(|e| format!("direct to {step}: {e}"))?;
    }
    Ok(())
}

#[test]
fn a_black_node_and_a_red_node_say_their_colour_in_the_label_and_not_only_in_the_fill() -> Result<(), String> {
    let tab = open()?;
    click_button(&tab, "jump-1")?;
    let labels = text(
        &tab,
        r#"(() => JSON.stringify([...document.querySelectorAll('#rbt-stage .rbt-scene ellipse')].map(e =>
            [e.parentElement.querySelector('text').textContent, e.getAttribute('fill')]).sort()))()"#,
    )?;
    check(
        labels.contains("\"10 R\",\"#fecaca\"") && labels.contains("\"15 B\",\"#94a3b8\""),
        &labels,
    )
}

// The semantic tree.

#[test]
fn the_tree_has_truthful_roles_levels_positions_names_and_one_tab_stop() -> Result<(), String> {
    let tab = open()?;
    click_button(&tab, "jump-2")?;
    let rows = json(
        &tab,
        r#"(() => JSON.stringify([...document.querySelectorAll('#rbt-stage [role=treeitem]')].map(g => [
            +g.getAttribute('data-rbt-key'), g.getAttribute('aria-level'), g.getAttribute('aria-posinset'),
            g.getAttribute('aria-setsize'), g.getAttribute('aria-expanded'), g.getAttribute('aria-label'),
            g.getAttribute('aria-selected')])))()"#,
    )?;
    let row = |key: u64| -> Result<Vec<Option<String>>, String> {
        rows.as_array()
            .and_then(|a| a.iter().find(|r| r[0].as_u64() == Some(key)))
            .map(|r| {
                r.as_array()
                    .map(|c| c.iter().map(|v| v.as_str().map(str::to_owned)).collect())
                    .unwrap_or_default()
            })
            .ok_or_else(|| format!("no item {key}"))
    };
    let s = |v: &str| Some(v.to_owned());
    // The tree at the choice of a successor: 5 has 1 and 15, 15 has 10 and 20, and 10 has the right child 12.
    check_eq(
        row(5)?[1..6].to_vec(),
        vec![s("1"), s("1"), s("1"), s("true"), s("5, black, root")],
    )?;
    check_eq(
        row(15)?[1..6].to_vec(),
        vec![s("2"), s("2"), s("2"), s("true"), s("15, red, right child of 5")],
    )?;
    check_eq(
        row(10)?[1..6].to_vec(),
        vec![s("3"), s("1"), s("2"), s("true"), s("10, black, left child of 15")],
    )?;
    check_eq(
        row(20)?[1..6].to_vec(),
        vec![s("3"), s("2"), s("2"), None, s("20, black, right child of 15")],
    )?;
    check_eq(
        row(12)?[1..6].to_vec(),
        vec![s("4"), s("1"), s("1"), None, s("12, red, right child of 10")],
    )?;
    check_eq(count(&tab, &format!("{STAGE} .rbt-overlay [tabindex=\"0\"]"))?, 1)?;
    check_eq(
        attribute(&tab, &format!("{STAGE} .rbt-overlay [tabindex=\"0\"]"), "data-rbt-key")?,
        "5".to_owned(),
    )?;
    // Only a key with children holds a group: 5, 15 and 10.
    check_eq(count(&tab, &format!("{STAGE} [role=group]"))?, 3)?;
    check_eq(count(&tab, &format!("{STAGE} .rbt-scene [tabindex]"))?, 0)?;
    check_eq(count(&tab, &format!("{STAGE} [role=application]"))?, 0)
}

#[test]
fn every_item_has_aria_selected_true_or_false() -> Result<(), String> {
    let tab = open()?;
    click_button(&tab, "jump-1")?;
    check_eq(count(&tab, &format!("{STAGE} [role=treeitem]:not([aria-selected])"))?, 0)?;
    check_eq(count(&tab, &format!("{STAGE} [aria-selected=\"true\"]"))?, 0)
}

// The keyboard.

#[test]
fn tab_enters_the_tree_once_and_leaves_it_and_the_arrows_go_where_they_should() -> Result<(), String> {
    let tab = open()?;
    click_button(&tab, "jump-2")?;
    let step = position(&tab)?;
    // From the range, one Tab lands on the tab stop, which is the root, and another leaves the tree.
    text(&tab, "(() => { document.getElementById('rbt-seek').focus(); return 'ok'; })()")?;
    press(&tab, "Tab")?;
    check_eq(focused(&tab)?, "key:5".to_owned())?;
    press(&tab, "Tab")?;
    check(!focused(&tab)?.starts_with("key:") && focused(&tab)? != "tree", &focused(&tab)?)?;
    // Back in the tree: 5 has the children 1 and 15.
    focus_item(&tab, 5)?;
    press(&tab, "ArrowRight")?;
    check_eq(focused(&tab)?, "key:1".to_owned())?;
    press(&tab, "ArrowRight")?;
    check_eq(focused(&tab)?, "key:1".to_owned())?; // a leaf goes nowhere
    press(&tab, "ArrowLeft")?;
    check_eq(focused(&tab)?, "key:5".to_owned())?;
    press(&tab, "ArrowLeft")?;
    check_eq(focused(&tab)?, "key:5".to_owned())?; // the root has no parent
    press(&tab, "ArrowDown")?;
    check_eq(focused(&tab)?, "key:10".to_owned())?; // sorted order: 1, 5, 10, 12, 15, 20
    press(&tab, "ArrowDown")?;
    press(&tab, "ArrowDown")?;
    check_eq(focused(&tab)?, "key:15".to_owned())?;
    press(&tab, "ArrowRight")?;
    check_eq(focused(&tab)?, "key:10".to_owned())?; // two children: the left first
    press(&tab, "ArrowUp")?;
    check_eq(focused(&tab)?, "key:5".to_owned())?;
    press(&tab, "End")?;
    check_eq(focused(&tab)?, "key:20".to_owned())?;
    press(&tab, "Home")?;
    check_eq(focused(&tab)?, "key:1".to_owned())?;
    check_eq(count(&tab, &format!("{STAGE} .rbt-overlay [tabindex=\"0\"]"))?, 1)?;
    check_eq(
        attribute(&tab, &format!("{STAGE} .rbt-overlay [tabindex=\"0\"]"), "data-rbt-key")?,
        "1".to_owned(),
    )?;
    check_eq(position(&tab)?, step)
}

#[test]
fn the_right_child_of_a_node_with_two_children_is_reached_with_down() -> Result<(), String> {
    let tab = open()?;
    click_button(&tab, "jump-2")?;
    focus_item(&tab, 15)?;
    press(&tab, "ArrowRight")?;
    check_eq(focused(&tab)?, "key:10".to_owned())?;
    // Down from 10 goes to 12 in sorted order, and then 15 and 20: the right child 20 is two Downs from 15's left.
    focus_item(&tab, 15)?;
    press(&tab, "ArrowDown")?;
    check_eq(focused(&tab)?, "key:20".to_owned())
}

#[test]
fn enter_and_space_select_the_focused_key_and_navigation_never_does() -> Result<(), String> {
    let tab = open()?;
    click_button(&tab, "jump-1")?;
    let step = position(&tab)?;
    focus_item(&tab, 5)?;
    press(&tab, "ArrowRight")?;
    check_eq(count(&tab, &format!("{STAGE} [aria-selected=\"true\"]"))?, 0)?;
    press(&tab, "Enter")?;
    check_eq(count(&tab, &format!("{STAGE} [aria-selected=\"true\"]"))?, 1)?;
    check_eq(
        attribute(&tab, &format!("{STAGE} [aria-selected=\"true\"]"), "data-rbt-key")?,
        "1".to_owned(),
    )?;
    check_eq(field(&tab, "selected")?, "1, black".to_owned())?;
    press(&tab, "ArrowLeft")?;
    press(&tab, " ")?;
    check_eq(
        attribute(&tab, &format!("{STAGE} [aria-selected=\"true\"]"), "data-rbt-key")?,
        "5".to_owned(),
    )?;
    check(live(&tab)?.contains("Selected 5, black, root"), &live(&tab)?)?;
    check_eq(position(&tab)?, step)
}

#[test]
fn the_navigation_keys_do_not_scroll_the_page() -> Result<(), String> {
    let tab = open()?;
    click_button(&tab, "jump-1")?;
    focus_item(&tab, 5)?;
    let prevented = text(
        &tab,
        r#"(() => { const out = []; for (const key of ['ArrowDown', 'Home', 'End', ' ', 'a', 'Tab']) {
            const e = new KeyboardEvent('keydown', { key, bubbles: true, cancelable: true });
            document.querySelector('#rbt-stage [data-rbt-key="5"]').dispatchEvent(e); out.push(e.defaultPrevented); }
            return JSON.stringify(out); })()"#,
    )?;
    check_eq(prevented, "[true,true,true,true,false,false]".to_owned())
}

// The selection.

#[test]
fn a_selected_15_shows_as_20_after_the_deletion_and_comes_back_on_reversing() -> Result<(), String> {
    let tab = open()?;
    click_button(&tab, "jump-2")?;
    click_item(&tab, 15)?;
    check_eq(
        attribute(&tab, &format!("{STAGE} [aria-selected=\"true\"]"), "data-rbt-key")?,
        "15".to_owned(),
    )?;
    click_button(&tab, "next")?;
    check_eq(
        attribute(&tab, &format!("{STAGE} [aria-selected=\"true\"]"), "data-rbt-key")?,
        "20".to_owned(),
    )?;
    check(
        field(&tab, "note")?.contains("took the place of your selected 15"),
        &field(&tab, "note")?,
    )?;
    check_eq(field(&tab, "selected")?, "20, red".to_owned())?;
    let last = last_step(&tab)?;
    seek(&tab, last)?;
    check_eq(
        attribute(&tab, &format!("{STAGE} [aria-selected=\"true\"]"), "data-rbt-key")?,
        "20".to_owned(),
    )?;
    click_button(&tab, "jump-2")?;
    check_eq(
        attribute(&tab, &format!("{STAGE} [aria-selected=\"true\"]"), "data-rbt-key")?,
        "15".to_owned(),
    )?;
    check_eq(field(&tab, "note")?, "\u{2014}".to_owned())
}

#[test]
fn choosing_20_after_the_deletion_stops_the_return_to_15() -> Result<(), String> {
    let tab = open()?;
    click_button(&tab, "jump-2")?;
    click_item(&tab, 15)?;
    click_button(&tab, "next")?;
    click_item(&tab, 20)?;
    click_button(&tab, "previous")?;
    check_eq(
        attribute(&tab, &format!("{STAGE} [aria-selected=\"true\"]"), "data-rbt-key")?,
        "20".to_owned(),
    )?;
    check_eq(
        count(&tab, &format!("{STAGE} [data-rbt-key=\"15\"][aria-selected=\"true\"]"))?,
        0,
    )
}

#[test]
fn a_key_that_is_not_yet_inserted_falls_back_to_the_root() -> Result<(), String> {
    let tab = open()?;
    click_button(&tab, "jump-2")?;
    click_item(&tab, 12)?;
    seek(&tab, 5)?;
    check_eq(count(&tab, &format!("{STAGE} [aria-selected=\"true\"]"))?, 1)?;
    check(
        field(&tab, "note")?.contains("not in the tree at this step"),
        &field(&tab, "note")?,
    )?;
    click_button(&tab, "jump-2")?;
    check_eq(
        attribute(&tab, &format!("{STAGE} [aria-selected=\"true\"]"), "data-rbt-key")?,
        "12".to_owned(),
    )
}

// The focus.

#[test]
fn the_focus_follows_its_key_through_a_rotation() -> Result<(), String> {
    let tab = open()?;
    click_button(&tab, "jump-1")?; // 10 is a red leaf-side node here, and the next steps change its place
    focus_item(&tab, 10)?;
    let before = attribute(&tab, &format!("{STAGE} [data-rbt-key=\"10\"]"), "aria-label")?;
    click_button(&tab, "next")?;
    check_eq(focused(&tab)?, "key:10".to_owned())?;
    check_eq(position(&tab)?, 17)?;
    click_button(&tab, "previous")?;
    check_eq(focused(&tab)?, "key:10".to_owned())?;
    check_eq(
        attribute(&tab, &format!("{STAGE} [data-rbt-key=\"10\"]"), "aria-label")?,
        before,
    )
}

#[test]
fn focus_on_15_goes_to_20_when_15_is_deleted_and_back_to_15_on_reversing() -> Result<(), String> {
    let tab = open()?;
    click_button(&tab, "jump-2")?;
    focus_item(&tab, 15)?;
    click_button(&tab, "next")?;
    check_eq(focused(&tab)?, "key:20".to_owned())?;
    check_eq(
        attribute(&tab, &format!("{STAGE} .rbt-overlay [tabindex=\"0\"]"), "data-rbt-key")?,
        "20".to_owned(),
    )?;
    click_button(&tab, "previous")?;
    check_eq(focused(&tab)?, "key:15".to_owned())?;
    check_eq(
        attribute(&tab, &format!("{STAGE} .rbt-overlay [tabindex=\"0\"]"), "data-rbt-key")?,
        "15".to_owned(),
    )
}

#[test]
fn tabbing_onto_a_replacement_does_not_forget_the_key_it_replaced() -> Result<(), String> {
    let tab = open()?;
    click_button(&tab, "jump-2")?;
    focus_item(&tab, 15)?;
    click_button(&tab, "next")?;
    // The tab stop is on 20. Leave the tree and Tab back in: it lands on 20, and the intent stays 15.
    text(&tab, "(() => { document.getElementById('rbt-seek').focus(); return 'ok'; })()")?;
    press(&tab, "Tab")?;
    check_eq(focused(&tab)?, "key:20".to_owned())?;
    click_button(&tab, "previous")?;
    check_eq(focused(&tab)?, "key:15".to_owned())
}

#[test]
fn a_node_that_is_not_yet_inserted_sends_the_focus_to_the_root() -> Result<(), String> {
    let tab = open()?;
    click_button(&tab, "jump-2")?;
    focus_item(&tab, 12)?;
    // At step 5 only 10 and 5 are in the tree, and the root is 10.
    seek(&tab, 5)?;
    check_eq(focused(&tab)?, "key:10".to_owned())
}

#[test]
fn an_empty_tree_sends_the_focus_to_the_tree_itself() -> Result<(), String> {
    let tab = open()?;
    click_button(&tab, "jump-0")?;
    focus_item(&tab, 5)?;
    seek(&tab, 0)?;
    check_eq(focused(&tab)?, "tree".to_owned())
}

#[test]
fn a_button_keeps_the_focus_through_every_step_and_the_ends_say_so() -> Result<(), String> {
    let tab = open()?;
    text(
        &tab,
        "(() => { document.querySelector('[data-action=\"next\"]').focus(); return 'ok'; })()",
    )?;
    let button = text(&tab, "(() => { window.__next = document.activeElement; return 'ok'; })()")?;
    check_eq(button, "ok".to_owned())?;
    let last = last_step(&tab)?;
    for step in 1..=last {
        click_button(&tab, "next")?;
        check_eq(position(&tab)?, step)?;
        check_eq(
            text(&tab, "String(document.activeElement === window.__next)")?,
            "true".to_owned(),
        )
        .map_err(|e| format!("step {step}: {e}"))?;
    }
    click_button(&tab, "next")?;
    check_eq(live(&tab)?, "Already at the last step.".to_owned())?;
    check_eq(attribute(&tab, "[data-action=\"next\"]", "aria-disabled")?, "true".to_owned())?;
    check_eq(
        text(&tab, "String(document.activeElement === window.__next)")?,
        "true".to_owned(),
    )?;
    click_button(&tab, "restart")?;
    click_button(&tab, "previous")?;
    check_eq(live(&tab)?, "Already at the first step.".to_owned())
}

#[test]
fn nothing_steals_the_focus_from_elsewhere_on_the_page() -> Result<(), String> {
    let tab = open()?;
    text(&tab, "(() => { document.getElementById('rbt-seek').focus(); return 'ok'; })()")?;
    for _ in 0..4 {
        click_button(&tab, "next")?;
    }
    check_eq(focused(&tab)?, "id:rbt-seek".to_owned())
}

// The controls.

#[test]
fn the_three_jumps_land_just_after_their_event_and_the_construction_left_rotation_is_the_first() -> Result<(), String> {
    let tab = open()?;
    click_button(&tab, "jump-0")?;
    let a = position(&tab)?;
    check(field(&tab, "event")?.contains("Right-rotate at 10"), &field(&tab, "event")?)?;
    click_button(&tab, "jump-1")?;
    let b = position(&tab)?;
    check(field(&tab, "event")?.contains("Left-rotate at 10"), &field(&tab, "event")?)?;
    click_button(&tab, "jump-2")?;
    let c = position(&tab)?;
    check(field(&tab, "event")?.contains("successor"), &field(&tab, "event")?)?;
    check(a < b && b < c, "in order")?;
    check(live(&tab)?.starts_with(&format!("Step {c} of")), &live(&tab)?)
}

#[test]
fn the_range_goes_to_any_step_and_says_where_it_rests_once() -> Result<(), String> {
    let tab = open()?;
    text(
        &tab,
        "(() => { const r = document.getElementById('rbt-seek'); for (const v of ['3', '9', '12']) { r.value = v; \
         r.dispatchEvent(new Event('input', { bubbles: true })); } return 'ok'; })()",
    )?;
    check_eq(position(&tab)?, 12)?;
    check_eq(live(&tab)?, String::new())?;
    text(
        &tab,
        "(() => { document.getElementById('rbt-seek').dispatchEvent(new Event('change', { bubbles: true })); return 'ok'; })()",
    )?;
    check(live(&tab)?.starts_with("Step 12 of"), &live(&tab)?)
}

#[test]
fn a_transitional_step_says_repair_in_progress_and_a_finished_one_says_it_is_valid() -> Result<(), String> {
    let tab = open()?;
    click_button(&tab, "jump-2")?;
    click_button(&tab, "next")?;
    check(
        live(&tab)?.contains("Repair in progress, not yet a valid red-black tree"),
        &live(&tab)?,
    )?;
    let last = last_step(&tab)?;
    seek(&tab, last)?;
    click_button(&tab, "previous")?;
    click_button(&tab, "next")?;
    check(!live(&tab)?.contains("Repair in progress"), &live(&tab)?)?;
    check(
        live(&tab)?.contains("Delete 15 complete: a valid red-black tree of 5 keys, root 5"),
        &live(&tab)?,
    )
}

// Play.

#[test]
fn play_runs_one_tick_at_a_time_and_announces_only_the_big_moves() -> Result<(), String> {
    let tab = open_manual()?;
    click_button(&tab, "play")?;
    check_eq(
        (waiting(&tab)?, content(&tab, "[data-action=\"play\"]")?),
        (1, "Pause".to_owned()),
    )?;
    check_eq(attribute(&tab, "[data-action=\"play\"]", "aria-label")?, "Pause".to_owned())?;
    check_eq(advance(&tab, 1)?, 1)?;
    check_eq(position(&tab)?, 1)?;
    check_eq(live(&tab)?, String::new())?; // an insertion event is a small move
    let last = last_step(&tab)?;
    check_eq(advance(&tab, 1000)?, last - 1)?;
    check_eq((position(&tab)?, waiting(&tab)?), (last, 0))?;
    check_eq(content(&tab, "[data-action=\"play\"]")?, "Play".to_owned())?;
    check(live(&tab)?.contains("Delete 15 complete"), &live(&tab)?)
}

#[test]
fn pausing_says_where_the_run_is_and_a_stale_tick_does_nothing() -> Result<(), String> {
    let tab = open_manual()?;
    click_button(&tab, "play")?;
    advance(&tab, 1)?;
    click_button(&tab, "play")?;
    check_eq((waiting(&tab)?, position(&tab)?), (0, 1))?;
    check(live(&tab)?.starts_with("Step 1 of"), &live(&tab)?)?;
    check_eq(advance(&tab, 5)?, 0)?;
    check_eq(position(&tab)?, 1)
}

#[test]
fn every_other_control_stops_the_playback() -> Result<(), String> {
    for action in ["next", "previous", "restart", "jump-0"] {
        let tab = open_manual()?;
        seek(&tab, 4)?;
        click_button(&tab, "play")?;
        check_eq(waiting(&tab)?, 1)?;
        click_button(&tab, action)?;
        check_eq(
            (waiting(&tab)?, content(&tab, "[data-action=\"play\"]")?),
            (0, "Play".to_owned()),
        )
        .map_err(|e| format!("{action}: {e}"))?;
    }
    let tab = open_manual()?;
    click_button(&tab, "play")?;
    seek(&tab, 7)?;
    check_eq(waiting(&tab)?, 0)
}

#[test]
fn the_hooks_come_off_the_window_when_the_page_is_left() -> Result<(), String> {
    let tab = open_manual()?;
    check_eq(text(&tab, "typeof window.__rbtAdvance")?, "function".to_owned())?;
    text(&tab, "(() => { window.dispatchEvent(new Event('pagehide')); return 'ok'; })()")?;
    check_eq(
        text(&tab, "typeof window.__rbtAdvance + typeof window.__rbtPending")?,
        "undefinedundefined".to_owned(),
    )?;
    click_button(&tab, "next")?;
    check_eq(position(&tab)?, 0)
}

// Pan and zoom.

#[test]
fn pan_and_zoom_are_off_so_the_picture_cannot_leave_the_tree() -> Result<(), String> {
    let tab = open()?;
    click_button(&tab, "jump-1")?;
    let before = text(
        &tab,
        &format!(
            "document.querySelector('{STAGE} .svg-dom-graph-content').outerHTML.length + '|' + document.querySelector('{STAGE} .svg-dom-graph-content').getAttribute('transform')"
        ),
    )?;
    text(
        &tab,
        &format!(
            "(() => {{ const s = document.querySelector('{STAGE} .rbt-scene'); \
             s.dispatchEvent(new WheelEvent('wheel', {{ deltaY: -400, ctrlKey: true, bubbles: true, cancelable: true }})); \
             return 'ok'; }})()"
        ),
    )?;
    let after = text(
        &tab,
        &format!(
            "document.querySelector('{STAGE} .svg-dom-graph-content').outerHTML.length + '|' + document.querySelector('{STAGE} .svg-dom-graph-content').getAttribute('transform')"
        ),
    )?;
    check_eq(after, before)?;
    check_eq(overlay_error(&tab)? < 0.6, true)?;
    check_eq(count(&tab, &format!("{STAGE} [role=application]"))?, 0)
}

// Faults.

#[test]
fn a_tree_that_cannot_be_built_leaves_the_old_one_and_the_step() -> Result<(), String> {
    let tab = open()?;
    seek(&tab, 10)?;
    let before = digest(&tab)?;
    let root = attribute(&tab, &format!("{STAGE} .rbt-scene"), "id")?;
    fail_writes_of(&tab, "aria-level", true)?;
    click_button(&tab, "next")?;
    fail_writes_of(&tab, "aria-level", false)?;
    check(!banner_hidden(&tab)?, "reported")?;
    check_eq(position(&tab)?, 10)?;
    check_eq(attribute(&tab, &format!("{STAGE} .rbt-scene"), "id")?, root)?;
    check_eq(digest(&tab)?, before)?;
    check_eq(count(&tab, &format!("{STAGE} .rbt-board"))?, 1)?;
    check_eq(count(&tab, "#rbt-staging svg > *")?, 0)?;
    click_button(&tab, "next")?;
    check_eq(position(&tab)?, 11)?;
    check(banner_hidden(&tab)?, "cleared by a step that works")
}

#[test]
fn a_control_that_cannot_be_written_puts_back_the_others_and_leaves_the_old_tree() -> Result<(), String> {
    for name in ["aria-valuetext", "aria-disabled", "max"] {
        let tab = open()?;
        seek(&tab, 10)?;
        let before = (
            position(&tab)?,
            field(&tab, "event")?,
            attribute(&tab, "[data-action=\"previous\"]", "aria-disabled")?,
        );
        fail_writes_of(&tab, name, true)?;
        click_button(&tab, "next")?;
        fail_writes_of(&tab, name, false)?;
        check(!banner_hidden(&tab)?, &format!("{name}: reported"))?;
        check_eq(
            (
                position(&tab)?,
                field(&tab, "event")?,
                attribute(&tab, "[data-action=\"previous\"]", "aria-disabled")?,
            ),
            before,
        )
        .map_err(|e| format!("{name}: {e}"))?;
        click_button(&tab, "next")?;
        check_eq(position(&tab)?, 11).map_err(|e| format!("{name}: {e}"))?;
    }
    Ok(())
}

#[test]
fn a_swap_that_throws_before_it_changes_the_stage_leaves_the_old_tree() -> Result<(), String> {
    let tab = open()?;
    seek(&tab, 12)?;
    let before = digest(&tab)?;
    fail_replacement(&tab, true, false)?;
    click_button(&tab, "next")?;
    fail_replacement(&tab, false, false)?;
    check(!banner_hidden(&tab)?, "reported")?;
    check_eq((position(&tab)?, digest(&tab)?), (12, before))?;
    check_eq(count(&tab, &format!("{STAGE} .rbt-board"))?, 1)?;
    check_eq(count(&tab, "#rbt-staging svg > *")?, 0)?;
    click_button(&tab, "next")?;
    check_eq(position(&tab)?, 13)
}

#[test]
fn a_swap_that_throws_after_it_changed_the_stage_puts_the_old_tree_back() -> Result<(), String> {
    let tab = open()?;
    seek(&tab, 12)?;
    let before = digest(&tab)?;
    let scene = attribute(&tab, &format!("{STAGE} .rbt-scene"), "id")?;
    fail_replacement(&tab, true, true)?;
    click_button(&tab, "next")?;
    fail_replacement(&tab, false, true)?;
    check(!banner_hidden(&tab)?, "reported")?;
    check_eq(position(&tab)?, 12)?;
    check_eq(count(&tab, &format!("{STAGE} .rbt-board"))?, 1)?;
    check_eq(attribute(&tab, &format!("{STAGE} .rbt-scene"), "id")?, scene)?;
    check_eq(digest(&tab)?, before)?;
    check_eq(count(&tab, "#rbt-staging svg > *")?, 0)?;
    click_button(&tab, "next")?;
    check_eq(position(&tab)?, 13)?;
    check(banner_hidden(&tab)?, "repaired")
}

#[test]
fn a_focus_that_cannot_be_put_back_is_reported_and_the_next_action_repairs_it_even_at_an_end() -> Result<(), String> {
    let tab = open()?;
    seek(&tab, 0)?;
    click_button(&tab, "jump-2")?;
    focus_item(&tab, 5)?;
    fail_focus(&tab, true)?;
    click_button(&tab, "next")?;
    fail_focus(&tab, false)?;
    check(!banner_hidden(&tab)?, "reported")?;
    let at = position(&tab)?;
    // The step did not advance in the model, though the tree on the page is the next one.
    click_button(&tab, "restart")?;
    check_eq(position(&tab)?, 0)?;
    click_button(&tab, "previous")?; // nowhere to go, and still draws a page that is out of date
    check(banner_hidden(&tab)?, "repaired by a button with nowhere to go")?;
    check_eq(position(&tab)?, 0)?;
    check_eq(count(&tab, &format!("{STAGE} [role=treeitem]"))?, 0)?;
    check(at > 0, "it had been somewhere")
}

#[test]
fn a_play_label_that_cannot_be_written_leaves_the_page_dirty_and_a_boundary_press_repairs_it() -> Result<(), String> {
    let tab = open_manual()?;
    click_button(&tab, "play")?;
    check_eq(content(&tab, "[data-action=\"play\"]")?, "Pause".to_owned())?;
    fail_writes_of(&tab, "aria-label", true)?;
    click_button(&tab, "play")?;
    fail_writes_of(&tab, "aria-label", false)?;
    check_eq(waiting(&tab)?, 0)?;
    check(!banner_hidden(&tab)?, "reported")?;
    click_button(&tab, "previous")?; // the run is at its first step: nowhere to go
    check(banner_hidden(&tab)?, "repaired")?;
    check_eq(live(&tab)?, "Already at the first step.".to_owned())?;
    click_button(&tab, "play")?;
    check_eq(waiting(&tab)?, 1)
}

#[test]
fn a_ring_that_cannot_be_written_is_reported_and_repaired_by_the_next_step() -> Result<(), String> {
    let tab = open()?;
    click_button(&tab, "jump-1")?;
    fail_writes_of(&tab, "aria-selected", true)?;
    click_item(&tab, 10)?;
    fail_writes_of(&tab, "aria-selected", false)?;
    check(!banner_hidden(&tab)?, "reported")?;
    click_button(&tab, "next")?;
    check(banner_hidden(&tab)?, "repaired")?;
    check_eq(
        attribute(&tab, &format!("{STAGE} [aria-selected=\"true\"]"), "data-rbt-key")?,
        "10".to_owned(),
    )
}
