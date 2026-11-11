//! The N-Queens panel in the built demo gallery: a scene with selectors, buttons and a range above a board of squares
//! in layers, a status beside it, and a playback that a test can drive one tick at a time.
//!
//! Native unit tests cover the search, what each step shows, and every rule of the controls. What they cannot reach is
//! the browser. These tests check the following:
//!
//! - The selectors, the six buttons and the range are drawn in the scene, none of them overlaps the board, and a button
//!   works from the keyboard.
//! - The board is a grid of squares with one tab stop, drawn in layers, with the focus ring on top, and it keeps the
//!   same elements across steps.
//! - A Try outlines a square and draws three rays that do not depend on the outcome, a Reject marks the reason, a
//!   Place draws a queen, and a solution is framed.
//! - The range goes straight to a step, a burst of moves is drawn once, and letting it rest says where it is once.
//! - The buttons step, restart and jump between solutions, a button at an end is dimmed but keeps the focus, and says
//!   so.
//! - Play runs one tick at a time on a clock that the test advances, announces only a solution and the end, and stops
//!   at the end.
//! - The arrow keys, Home and End move between squares, a click moves the focus and not the search, and the ring is
//!   shown only while the grid has the focus.
//! - A new size of board replaces the grid, and stops the playback.
//! - A move of the range that is waiting for its frame when the board size changes is dropped, and does not carry the
//!   new board to its last step.
//! - A new board that cannot be made, because a write to the controls fails, leaves the old board, its trace, its step
//!   and the end of the range as they were, and the selector says the old size.
//! - The manual clock's hooks are on the window only while the page is, come off when it is left, and are not taken
//!   off if another clock has replaced them. A page with the real clock has none.
//! - A timer that cannot be set, when Play starts and when a later tick is scheduled, stops the playback and says so,
//!   and Play works again once timers do.
//! - A write that fails is reported and leaves the step where it was, and the next button, even one with nowhere to go,
//!   draws the board again.
//!
//! Like [`crate::game_of_life`], these run against the built demo gallery.

use crate::common::{demo_tab::DemoTab, drag, new_demo_tab};
use headless_chrome::Tab;
use std::time::{Duration, Instant};

const SVG: &str = "#n-queens-stage-svg";
const VIEWPORT: (u32, u32) = (1400, 1500);

fn open_at(query: &str) -> Result<DemoTab, String> {
    new_demo_tab(
        &format!("index.html{query}#panel-n-queens"),
        &format!("{SVG} .nq-board"),
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
    content(tab, &format!("#n-queens-st-{name}"))
}

fn position(tab: &Tab) -> Result<usize, String> {
    field(tab, "position")?
        .split('/')
        .next()
        .and_then(|first| first.parse().ok())
        .ok_or_else(|| "the position is not a number".to_owned())
}

fn last_step(tab: &Tab) -> Result<usize, String> {
    field(tab, "position")?
        .split('/')
        .nth(1)
        .and_then(|last| last.parse().ok())
        .ok_or_else(|| "the last step is not a number".to_owned())
}

fn live(tab: &Tab) -> Result<String, String> {
    Ok(content(tab, "#n-queens-live")?.replace('\u{a0}', ""))
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

/// Moves the range to `to` as a person drags it, which fires `input`, and optionally lets go, which fires `change`.
fn move_range(tab: &Tab, to: usize, release: bool) -> Result<(), String> {
    let fire = if release {
        "r.dispatchEvent(new Event('change', { bubbles: true }));"
    } else {
        ""
    };
    text(
        tab,
        &format!(
            "(() => {{ const r = document.getElementById('n-queens-seek'); r.value = '{to}'; \
             r.dispatchEvent(new Event('input', {{ bubbles: true }})); {fire} return 'ok'; }})()"
        ),
    )
    .map(|_| ())
}

/// Goes straight to step `to`, as a person who drags the range and lets go.
fn seek(tab: &Tab, to: usize) -> Result<(), String> {
    move_range(tab, to, true)?;
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

/// Runs `n` ticks of the clock that the test holds, and says how many ran.
fn advance(tab: &Tab, n: usize) -> Result<usize, String> {
    text(tab, &format!("String(window.__nQueensAdvance({n}))"))?
        .parse()
        .map_err(|e| format!("{e}"))
}

fn waiting(tab: &Tab) -> Result<usize, String> {
    text(tab, "String(window.__nQueensPending())")?
        .parse()
        .map_err(|e| format!("{e}"))
}

/// The class of the `i`th ray, for 0, 1, 2.
fn ray_class(tab: &Tab, i: usize) -> Result<String, String> {
    attribute(tab, &format!("{SVG} .nq-rays path:nth-child({})", i + 1), "class")
}

fn rays(tab: &Tab) -> Result<Vec<(String, String)>, String> {
    (0..3)
        .map(|i| {
            Ok((
                ray_class(tab, i)?,
                attribute(tab, &format!("{SVG} .nq-rays path:nth-child({})", i + 1), "d")?,
            ))
        })
        .collect()
}

fn queens(tab: &Tab) -> Result<usize, String> {
    count(tab, &format!("{SVG} .nq-glyph.nq-on"))
}

fn cell_label(tab: &Tab, row: usize, col: usize) -> Result<String, String> {
    attribute(tab, &format!("#nq-r{row}-c{col}"), "aria-label")
}

fn outline_class(tab: &Tab) -> Result<String, String> {
    attribute(tab, &format!("{SVG} .nq-outline"), "class")
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

fn banner_hidden(tab: &Tab) -> Result<bool, String> {
    Ok(text(tab, "String(document.getElementById('n-queens-error').hidden)")? == "true")
}

fn focused(tab: &Tab) -> Result<String, String> {
    text(
        tab,
        "document.activeElement ? (document.activeElement.id || document.activeElement.tagName) : ''",
    )
}

fn active_cell(tab: &Tab) -> Result<String, String> {
    text(
        tab,
        &format!(
            "Array.from(document.querySelectorAll('{SVG} [role=gridcell][tabindex=\"0\"]')).map(c => c.id).join(',')"
        ),
    )
}

fn ring_shown(tab: &Tab) -> Result<bool, String> {
    Ok(!attribute(tab, &format!("{SVG} .nq-ring"), "class")?.contains("nq-off"))
}

/// The middle of the element with `id` on the page.
fn centre_of(tab: &Tab, id: &str) -> Result<(f64, f64), String> {
    let at = text(
        tab,
        &format!(
            "(() => {{ const r = document.getElementById('{id}').getBoundingClientRect(); \
             return (r.left + r.width / 2) + ',' + (r.top + r.height / 2); }})()"
        ),
    )?;
    let (x, y) = at.split_once(',').ok_or("two numbers")?;
    Ok((
        x.parse::<f64>().map_err(|e| e.to_string())?.round(),
        y.parse::<f64>().map_err(|e| e.to_string())?.round(),
    ))
}

// A new size that cannot be made.

/// The number of squares on the board, which is how big it is.
fn squares(tab: &Tab) -> Result<usize, String> {
    count(tab, &format!("{SVG} [role=gridcell]"))
}

// A timer that cannot be set.

/// Makes `setTimeout` throw for the delay of a tick at the default speed, which is 50 milliseconds, after the first
/// `allowed` of them, or puts the browser's own back. Timers for any other delay are left alone.
fn fail_timers(tab: &Tab, allowed: Option<u32>) -> Result<(), String> {
    let script = match allowed {
        Some(allowed) => format!(
            "(() => {{ if (!window.__setTimeout) {{ window.__setTimeout = window.setTimeout; }} let n = 0; \
             const real = window.__setTimeout; \
             window.setTimeout = function (f, d) {{ if (d === 50 && ++n > {allowed}) {{ throw new Error('injected'); }} \
             return real.apply(this, arguments); }}; return 'ok'; }})()"
        ),
        None => "(() => { if (window.__setTimeout) { window.setTimeout = window.__setTimeout; } return 'ok'; })()"
            .to_owned(),
    };
    text(tab, &script).map(|_| ())
}

fn play_label(tab: &Tab) -> Result<String, String> {
    content(tab, "#n-queens-play")
}

// The manual clock's hooks.

fn hook_types(tab: &Tab) -> Result<String, String> {
    text(tab, "typeof window.__nQueensAdvance + ',' + typeof window.__nQueensPending")
}

/// Says that the page is being left, as the browser does, which tears the controller down.
fn leave(tab: &Tab) -> Result<(), String> {
    text(tab, "(() => { window.dispatchEvent(new Event('pagehide')); return 'ok'; })()").map(|_| ())
}

mod unit_tests;
