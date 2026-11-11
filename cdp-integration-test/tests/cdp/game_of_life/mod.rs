//! The Game of Life panel in the built demo gallery: one scene, 560 pixels wide, with a board of 40 by 30 cells that a
//! person draws on, buttons above and below it, and a slider for the speed.
//!
//! Native unit tests cover the rules, the patterns, the edges, the pace and where everything is in the scene. What they
//! cannot reach is the browser. These tests check the following:
//!
//! - The scene has the board, thirteen buttons and a slider, and starts with a glider that is going.
//! - Pause, Play and Step work, and Step works out the rules: a blinker turns, and turns back.
//! - A click brings a cell to life or kills it, and a drag gives every cell it crosses the state of the first, even
//!   when the browser reports the pointer only at the ends of the drag.
//! - A drag ends when the pointer is let go, cancelled or loses its capture, and only the pointer that makes it can
//!   move it on or end it.
//! - The arrow keys move the cursor, which wraps, and Space changes the cell under it, and it is said for assistive
//!   technology.
//! - Each pattern replaces the board, and Random and Clear work.
//! - Each spaceship is added at the left edge, level with the cursor, without clearing the board or stopping it, and
//!   flies to the right.
//! - The edges wrap or stop, as the button says.
//! - The slider sets the speed.
//! - A write to the page that fails leaves the board right and the page out of date, which is reported, and the next
//!   button, key, drag or frame draws the board again.
//!
//! Like [`crate::convolution`], these run against the built demo gallery.

use crate::common::{demo_tab::DemoTab, drag, mouse_event, new_demo_tab};
use headless_chrome::{Tab, protocol::cdp::Input::DispatchMouseEventTypeOption as Kind};
use std::time::{Duration, Instant};

const SVG: &str = "#game-of-life-stage-svg";
const READOUT: &str = "#game-of-life-readout";
const CURSOR_READOUT: &str = "#game-of-life-cursor";
const VIEWPORT: (u32, u32) = (1400, 1500);

/// The size of a cell, how far the board is down the scene, and how many cells are across and down it.
const CELL: f64 = 14.0;
const TOP: f64 = 92.0;
const COLS: usize = 40;
const ROWS: usize = 30;

fn open() -> Result<DemoTab, String> {
    new_demo_tab("index.html#panel-game-of-life", &format!("{SVG} .life-board"), VIEWPORT)
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

/// The attribute of the first element that matches `selector`.
fn attribute(tab: &Tab, selector: &str, name: &str) -> Result<String, String> {
    text(
        tab,
        &format!("String(document.querySelector('{selector}').getAttribute('{name}'))"),
    )
}

/// The generation and the number of live cells, as the page writes them above the board.
fn readout(tab: &Tab) -> Result<(u64, usize), String> {
    let said = text(tab, &format!("document.querySelector('{READOUT}').textContent"))?;
    let numbers: Vec<u64> = said
        .split(|c: char| !c.is_ascii_digit())
        .filter(|n| !n.is_empty())
        .filter_map(|n| n.parse().ok())
        .collect();
    match numbers.as_slice() {
        [generation, population] => Ok((*generation, *population as usize)),
        other => Err(format!("cannot read {said:?}: {other:?}")),
    }
}

fn generation(tab: &Tab) -> Result<u64, String> {
    Ok(readout(tab)?.0)
}

fn population(tab: &Tab) -> Result<usize, String> {
    Ok(readout(tab)?.1)
}

/// The `d` of the path that draws the live cells.
fn live_path(tab: &Tab) -> Result<String, String> {
    attribute(tab, &format!("{SVG} .life-board path[fill=\"#0f766e\"]"), "d")
}

/// Waits until `condition` is true, for as long as `seconds`.
fn wait_until(tab: &Tab, seconds: u64, mut condition: impl FnMut(&Tab) -> Result<bool, String>) -> Result<(), String> {
    let start = Instant::now();
    while start.elapsed() < Duration::from_secs(seconds) {
        if condition(tab)? {
            return Ok(());
        }
        std::thread::sleep(Duration::from_millis(50));
    }
    Err("it did not happen in time".to_owned())
}

/// Presses the button whose action is `action`, as a click on it.
fn press_button(tab: &Tab, action: &str) -> Result<(), String> {
    text(
        tab,
        &format!(
            "(() => {{ document.querySelector('{SVG} [data-action=\"{action}\"]')\
             .dispatchEvent(new MouseEvent('click', {{ bubbles: true }})); return 'ok'; }})()"
        ),
    )
    .map(|_| ())
}

fn label_of(tab: &Tab, action: &str) -> Result<String, String> {
    attribute(tab, &format!("{SVG} [data-action=\"{action}\"]"), "aria-label")
}

/// Stops the generations, so that what a test looks at stays as it is.
fn pause(tab: &Tab) -> Result<(), String> {
    if label_of(tab, "play")? == "Pause" {
        press_button(tab, "play")?;
    }
    check_eq(label_of(tab, "play")?, "Play".to_owned())
}

/// Where the middle of the cell at `col` and `row` is on the page.
fn centre(tab: &Tab, col: usize, row: usize) -> Result<(f64, f64), String> {
    let corner = text(
        tab,
        &format!(
            "(() => {{ const r = document.querySelector('{SVG}').getBoundingClientRect(); \
             return r.left + ',' + r.top; }})()"
        ),
    )?;
    let (left, top) = corner.split_once(',').ok_or("the corner is two numbers")?;
    let (left, top): (f64, f64) = (
        left.parse().map_err(|e| format!("{e}"))?,
        top.parse().map_err(|e| format!("{e}"))?,
    );
    Ok((
        (left + col as f64 * CELL + CELL / 2.0).round(),
        (top + TOP + row as f64 * CELL + CELL / 2.0).round(),
    ))
}

/// Clicks the cell at `col` and `row`.
fn click_cell(tab: &Tab, col: usize, row: usize) -> Result<(), String> {
    let at = centre(tab, col, row)?;
    drag(tab, &[at, at])
}

/// Presses on the cell at `from` and drags along the row to the cell at `to`.
fn drag_along(tab: &Tab, row: usize, from: usize, to: usize) -> Result<(), String> {
    let points = (from..=to).map(|col| centre(tab, col, row)).collect::<Result<Vec<_>, _>>()?;
    drag(tab, &points)
}

fn press(tab: &Tab, key: &str) -> Result<(), String> {
    tab.press_key(key).map_err(|e| format!("could not press {key}: {e}"))?;
    Ok(())
}

fn focus_board(tab: &Tab) -> Result<(), String> {
    text(tab, &format!("document.querySelector('{SVG} .life-board').focus(), 'ok'")).map(|_| ())
}

/// Clears the board, with the generations stopped, which is how most tests begin.
fn emptied() -> Result<DemoTab, String> {
    let tab = open()?;
    pause(&tab)?;
    press_button(&tab, "clear")?;
    check_eq(readout(&tab)?, (0, 0))?;
    Ok(tab)
}

/// The places that the rectangles of the live path start at, as `(x, y)`, which says where the cells are.
fn starts(path: &str) -> Vec<(f64, f64)> {
    path.split('M')
        .filter(|piece| !piece.is_empty())
        .filter_map(|piece| {
            let (x, rest) = piece.split_once(' ')?;
            let y: String = rest.chars().take_while(|c| c.is_ascii_digit() || *c == '.').collect();
            Some((x.parse().ok()?, y.parse().ok()?))
        })
        .collect()
}

/// The pointer of a mouse that the browser is driven by, which is 1 in Chrome.
const MOUSE: u32 = 1;

/// Presses the mouse button down at `at`, and leaves it down.
fn press_down(tab: &Tab, at: (f64, f64)) -> Result<(), String> {
    mouse_event(tab, Kind::MouseMoved, at, None)?;
    mouse_event(tab, Kind::MousePressed, at, Some(1))
}

/// Moves the mouse to `at` with the button down.
fn move_held(tab: &Tab, at: (f64, f64)) -> Result<(), String> {
    mouse_event(tab, Kind::MouseMoved, at, Some(1))
}

/// Lets the mouse button go at `at`.
fn let_go(tab: &Tab, at: (f64, f64)) -> Result<(), String> {
    mouse_event(tab, Kind::MouseReleased, at, Some(0))
}

/// Sends the scene a pointer event of `kind` for the pointer `id` at `at`, as another pointer or the browser would.
fn fire(tab: &Tab, kind: &str, id: u32, at: (f64, f64)) -> Result<(), String> {
    text(
        tab,
        &format!(
            "(() => {{ document.querySelector('{SVG}').dispatchEvent(new PointerEvent('{kind}', \
             {{ pointerId: {id}, bubbles: true, cancelable: true, clientX: {}, clientY: {}, button: 0, \
             isPrimary: true }})); return 'ok'; }})()",
            at.0, at.1
        ),
    )
    .map(|_| ())
}

/// Makes every write of the attribute `name` fail with an exception, as the browser can when a page is under pressure,
/// or puts the browser's own write back.
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
    Ok(text(tab, "String(document.querySelector('#game-of-life-error').hidden)")? == "true")
}

fn banner(tab: &Tab) -> Result<String, String> {
    text(tab, "document.querySelector('#game-of-life-error').textContent")
}

mod unit_tests;
