//! The N-Queens panel in the built demo gallery: a scene with selectors, buttons and a range above a board of squares in
//! layers, a status beside it, and a playback that a test can drive one tick at a time.
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
//! - A write that fails is reported and leaves the step where it was, and the next button, even one with nowhere to go,
//!   draws the board again.
//!
//! Like [`crate::game_of_life`], these run against the built demo gallery.

use crate::common::{DemoTab, drag, new_demo_tab};
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

// The board.

#[test]
fn the_board_is_a_grid_of_squares_with_one_tab_stop() -> Result<(), String> {
    let tab = open()?;
    check_eq(count(&tab, &format!("{SVG} [role=grid]"))?, 1)?;
    check_eq(count(&tab, &format!("{SVG} [role=row]"))?, 8)?;
    check_eq(count(&tab, &format!("{SVG} [role=gridcell]"))?, 64)?;
    check_eq(count(&tab, &format!("{SVG} [role=gridcell][tabindex=\"0\"]"))?, 1)?;
    check_eq(count(&tab, &format!("{SVG} [role=gridcell][tabindex=\"-1\"]"))?, 63)?;
    check_eq(count(&tab, "#nq-r3-c5")?, 1)?;
    check_eq(cell_label(&tab, 3, 5)?, "empty".to_owned())?;
    check_eq(count(&tab, &format!("{SVG} .nq-fills rect"))?, 64)?;
    check_eq(count(&tab, &format!("{SVG} .nq-glyphs text"))?, 64)?;
    check_eq(count(&tab, &format!("{SVG} .nq-rays path"))?, 3)?;
    check_eq(queens(&tab)?, 0)
}

#[test]
fn the_layers_are_in_order_with_the_focus_ring_on_top() -> Result<(), String> {
    let tab = open()?;
    let order = text(
        &tab,
        &format!(
            "Array.from(document.querySelector('{SVG} .nq-board').children).map(c => c.getAttribute('class')).join(' ')"
        ),
    )?;
    let layers: Vec<&str> = order
        .split(' ')
        .filter(|c| c.starts_with("nq-") && !c.starts_with("nq-off"))
        .collect();
    check_eq(
        layers,
        vec![
            "nq-fills", "nq-rays", "nq-glyphs", "nq-outline", "nq-frame", "nq-grid", "nq-ring",
        ],
    )?;
    check_eq(attribute(&tab, &format!("{SVG} .nq-rays"), "aria-hidden")?, "true".to_owned())?;
    // Rays and outline never take the pointer or the focus.
    let hit = text(
        &tab,
        "(() => { const r = document.getElementById('nq-r2-c2').getBoundingClientRect(); \
         const e = document.elementFromPoint(r.left + r.width / 2, r.top + r.height / 2); return e.id; })()",
    )?;
    check_eq(hit, "nq-r2-c2".to_owned())
}

#[test]
fn the_page_opens_at_the_empty_board_paused() -> Result<(), String> {
    let tab = open()?;
    check_eq(field(&tab, "position")?, "0/33589".to_owned())?;
    check_eq(field(&tab, "kind")?, "Initial".to_owned())?;
    check_eq(field(&tab, "depth")?, "0 / 8".to_owned())?;
    check_eq(field(&tab, "solutions")?, "0 / 92".to_owned())?;
    check_eq(field(&tab, "playback")?, "Paused at 20 events per second".to_owned())?;
    check_eq(attribute(&tab, "#n-queens-seek", "max")?, "33589".to_owned())?;
    check(
        content(&tab, "#n-queens-board-info")?.contains("KiB"),
        "the page says what the trace takes",
    )
}

// What each step shows.

#[test]
fn a_try_outlines_the_square_and_draws_three_rays_a_place_draws_a_queen() -> Result<(), String> {
    let tab = open()?;
    click(&tab, "n-queens-next")?;
    check_eq(field(&tab, "kind")?, "Try".to_owned())?;
    check_eq(field(&tab, "cell")?, "row 1, column 1".to_owned())?;
    check(
        outline_class(&tab)?.contains("nq-candidate"),
        "the square is outlined as a candidate",
    )?;
    check_eq(cell_label(&tab, 0, 0)?, "candidate".to_owned())?;
    let drawn = rays(&tab)?;
    check(
        drawn.iter().all(|(class, d)| class == "nq-ray" && !d.is_empty()),
        "three neutral rays",
    )?;
    click(&tab, "n-queens-next")?;
    check_eq(field(&tab, "kind")?, "Place".to_owned())?;
    check_eq(queens(&tab)?, 1)?;
    check_eq(cell_label(&tab, 0, 0)?, "queen".to_owned())?;
    check(outline_class(&tab)?.contains("nq-off"), "no outline")?;
    check(rays(&tab)?.iter().all(|(class, _)| class.contains("nq-off")), "no rays")
}

#[test]
fn a_reject_marks_the_square_and_makes_the_reason_the_heavy_ray() -> Result<(), String> {
    let tab = open()?;
    seek(&tab, 3)?;
    let try_rays = rays(&tab)?;
    seek(&tab, 4)?;
    check_eq(field(&tab, "kind")?, "Reject".to_owned())?;
    check_eq(field(&tab, "reason")?, "column".to_owned())?;
    check(outline_class(&tab)?.contains("nq-rejected"), "a rejected outline")?;
    check_eq(cell_label(&tab, 1, 0)?, "rejected: column".to_owned())?;
    let reject_rays = rays(&tab)?;
    check_eq(reject_rays[0].0.clone(), "nq-ray nq-chosen".to_owned())?;
    check_eq(reject_rays[1].0.clone(), "nq-ray nq-muted".to_owned())?;
    check_eq(reject_rays[2].0.clone(), "nq-ray nq-muted".to_owned())?;
    let same_lines = try_rays.iter().zip(&reject_rays).all(|(a, b)| a.1 == b.1);
    check(same_lines, "the rays are the same lines as on the Try before it")
}

#[test]
fn the_rays_of_a_try_are_the_same_whether_or_not_a_queen_is_in_the_way() -> Result<(), String> {
    let tab = open()?;
    // Step 3 tries (row 2, column 1) under a queen, and is refused. Step 2 of a later branch tries it free.
    seek(&tab, 3)?;
    let first = rays(&tab)?;
    check(
        first.iter().all(|(class, _)| class == "nq-ray"),
        "neutral on a Try that is about to be refused",
    )?;
    seek(&tab, 2)?;
    seek(&tab, 3)?;
    check_eq(rays(&tab)?, first)
}

#[test]
fn a_solution_is_framed_and_shows_its_vector_and_ordinal() -> Result<(), String> {
    let tab = open()?;
    click(&tab, "n-queens-next-solution")?;
    check_eq(field(&tab, "kind")?, "Solution".to_owned())?;
    check_eq(field(&tab, "vector")?, "[1, 5, 8, 6, 3, 7, 2, 4]".to_owned())?;
    check_eq(field(&tab, "solutions")?, "1 / 92 (solution 1)".to_owned())?;
    check_eq(queens(&tab)?, 8)?;
    check(
        !attribute(&tab, &format!("{SVG} .nq-frame"), "class")?.contains("nq-off"),
        "the frame is drawn",
    )?;
    check(outline_class(&tab)?.contains("nq-off"), "no outline")?;
    click(&tab, "n-queens-next")?;
    check_eq(field(&tab, "kind")?, "Remove".to_owned())?;
    check(
        attribute(&tab, &format!("{SVG} .nq-frame"), "class")?.contains("nq-off"),
        "the frame is gone",
    )?;
    check(
        outline_class(&tab)?.contains("nq-removed"),
        "the square the queen left is outlined",
    )?;
    check_eq(queens(&tab)?, 7)
}

#[test]
fn the_last_step_is_exhausted_with_an_empty_board() -> Result<(), String> {
    let tab = open()?;
    let last = last_step(&tab)?;
    seek(&tab, last)?;
    check_eq(field(&tab, "kind")?, "Exhausted".to_owned())?;
    check_eq(queens(&tab)?, 0)?;
    check_eq(field(&tab, "solutions")?, "92 / 92".to_owned())
}

#[test]
fn going_back_draws_what_going_forward_did() -> Result<(), String> {
    let tab = open()?;
    seek(&tab, 40)?;
    let there = (rays(&tab)?, queens(&tab)?, outline_class(&tab)?, cell_label(&tab, 2, 3)?);
    seek(&tab, 4_000)?;
    seek(&tab, 40)?;
    check_eq(
        (rays(&tab)?, queens(&tab)?, outline_class(&tab)?, cell_label(&tab, 2, 3)?),
        there,
    )
}

#[test]
fn the_elements_of_the_board_are_the_same_at_every_step() -> Result<(), String> {
    let tab = open()?;
    let all = format!("{SVG} *");
    let before = count(&tab, &all)?;
    for to in [1, 2, 4, 20_000, 33_589, 0] {
        seek(&tab, to)?;
        check_eq(count(&tab, &all)?, before)?;
    }
    Ok(())
}

// The range.

#[test]
fn the_range_goes_straight_to_a_step() -> Result<(), String> {
    let tab = open()?;
    for to in [1, 20_000, 33_589, 0] {
        seek(&tab, to)?;
        check_eq(
            attribute(&tab, "#n-queens-seek", "aria-valuetext")?.starts_with(&format!("Step {to} of")),
            true,
        )?;
    }
    Ok(())
}

#[test]
fn a_burst_of_moves_is_drawn_once_at_the_latest_place_and_resting_says_it_once() -> Result<(), String> {
    let tab = open()?;
    text(
        &tab,
        "(() => { window.__draws = 0; const o = new MutationObserver(() => { window.__draws++; }); \
                o.observe(document.getElementById('n-queens-st-position'), { childList: true }); return 'ok'; })()",
    )?;
    // All three moves are in one task, as a fast drag between two frames is, so no frame comes in between.
    text(
        &tab,
        "(() => { const r = document.getElementById('n-queens-seek'); \
         for (const v of ['500', '800', '1200']) { r.value = v; r.dispatchEvent(new Event('input', { bubbles: true })); } \
         return 'ok'; })()",
    )?;
    wait_until(&tab, 5, |tab| Ok(position(tab)? == 1_200))?;
    check_eq(text(&tab, "String(window.__draws)")?, "1".to_owned())?;
    check_eq(live(&tab)?, String::new())?;
    move_range(&tab, 1_200, true)?;
    check(live(&tab)?.starts_with("Step 1200 of"), "it says where it is")
}

// The buttons.

#[test]
fn previous_next_and_restart_move_one_step_and_back_to_the_start() -> Result<(), String> {
    let tab = open()?;
    click(&tab, "n-queens-next")?;
    click(&tab, "n-queens-next")?;
    click(&tab, "n-queens-previous")?;
    check_eq(position(&tab)?, 1)?;
    seek(&tab, 300)?;
    click(&tab, "n-queens-restart")?;
    check_eq((position(&tab)?, queens(&tab)?), (0, 0))
}

#[test]
fn a_button_at_an_end_is_dimmed_keeps_the_focus_and_says_so() -> Result<(), String> {
    let tab = open()?;
    check_eq(attribute(&tab, "#n-queens-previous", "aria-disabled")?, "true".to_owned())?;
    check_eq(attribute(&tab, "#n-queens-restart", "aria-disabled")?, "true".to_owned())?;
    check_eq(attribute(&tab, "#n-queens-next", "aria-disabled")?, "false".to_owned())?;
    text(&tab, "document.getElementById('n-queens-previous').focus(), 'ok'")?;
    click(&tab, "n-queens-previous")?;
    check_eq(position(&tab)?, 0)?;
    check_eq(focused(&tab)?, "n-queens-previous".to_owned())?;
    check_eq(live(&tab)?, "Already at the first step.".to_owned())?;
    let last = last_step(&tab)?;
    seek(&tab, last)?;
    check_eq(attribute(&tab, "#n-queens-next", "aria-disabled")?, "true".to_owned())?;
    check_eq(attribute(&tab, "#n-queens-play", "aria-disabled")?, "true".to_owned())?;
    click(&tab, "n-queens-next")?;
    check_eq(live(&tab)?, "Already at the last step.".to_owned())
}

#[test]
fn the_solution_buttons_jump_strictly_between_all_92_solutions() -> Result<(), String> {
    let tab = open()?;
    click(&tab, "n-queens-previous-solution")?;
    check_eq(live(&tab)?, "There is no earlier solution.".to_owned())?;
    for ordinal in 1..=92 {
        click(&tab, "n-queens-next-solution")?;
        check_eq(field(&tab, "solutions")?, format!("{ordinal} / 92 (solution {ordinal})"))?;
    }
    click(&tab, "n-queens-next-solution")?;
    check_eq(live(&tab)?, "There is no later solution.".to_owned())?;
    check_eq(field(&tab, "solutions")?, "92 / 92 (solution 92)".to_owned())?;
    click(&tab, "n-queens-previous-solution")?;
    check_eq(field(&tab, "solutions")?, "91 / 92 (solution 91)".to_owned())
}

// Playing.

#[test]
fn play_runs_one_tick_at_a_time_and_pause_takes_the_tick_back() -> Result<(), String> {
    let tab = open_manual()?;
    click(&tab, "n-queens-play")?;
    check_eq(content(&tab, "#n-queens-play")?, "Pause".to_owned())?;
    check_eq(waiting(&tab)?, 1)?;
    check_eq(advance(&tab, 5)?, 5)?;
    check_eq((position(&tab)?, waiting(&tab)?), (5, 1))?;
    check_eq(live(&tab)?, String::new())?;
    click(&tab, "n-queens-play")?;
    check_eq((content(&tab, "#n-queens-play")?, waiting(&tab)?), ("Play".to_owned(), 0))?;
    check(live(&tab)?.starts_with("Paused at step 5 of"), "pausing says where")?;
    check_eq(advance(&tab, 5)?, 0)?;
    check_eq(position(&tab)?, 5)
}

#[test]
fn every_other_control_stops_the_playback() -> Result<(), String> {
    let tab = open_manual()?;
    for id in [
        "n-queens-next",
        "n-queens-previous",
        "n-queens-restart",
        "n-queens-next-solution",
    ] {
        click(&tab, "n-queens-play")?;
        check_eq(waiting(&tab)?, 1)?;
        click(&tab, id)?;
        check_eq((waiting(&tab)?, content(&tab, "#n-queens-play")?), (0, "Play".to_owned()))?;
    }
    click(&tab, "n-queens-play")?;
    move_range(&tab, 700, true)?;
    check_eq((waiting(&tab)?, position(&tab)?), (0, 700))?;
    click(&tab, "n-queens-play")?;
    select(&tab, "n-queens-size", "5")?;
    check_eq((waiting(&tab)?, position(&tab)?), (0, 0))
}

#[test]
fn playing_a_board_of_four_reaches_the_end_and_stops_there() -> Result<(), String> {
    let tab = open_manual()?;
    select(&tab, "n-queens-size", "4")?;
    check_eq(last_step(&tab)?, 139)?;
    click(&tab, "n-queens-play")?;
    check_eq(advance(&tab, 1_000)?, 139)?;
    check_eq((position(&tab)?, field(&tab, "kind")?), (139, "Exhausted".to_owned()))?;
    check_eq((waiting(&tab)?, content(&tab, "#n-queens-play")?), (0, "Play".to_owned()))?;
    click(&tab, "n-queens-play")?;
    check_eq((waiting(&tab)?, position(&tab)?), (0, 139))?;
    check_eq(live(&tab)?, "Already at the last step.".to_owned())
}

#[test]
fn only_a_solution_and_the_end_are_announced_while_playing() -> Result<(), String> {
    let tab = open_manual()?;
    select(&tab, "n-queens-size", "4")?;
    let first = {
        click(&tab, "n-queens-next-solution")?;
        let at = position(&tab)?;
        click(&tab, "n-queens-restart")?;
        at
    };
    let announced = live(&tab)?;
    click(&tab, "n-queens-play")?;
    advance(&tab, first - 1)?;
    check_eq(live(&tab)?, announced)?;
    advance(&tab, 1)?;
    check(live(&tab)?.contains("solution 1 of 2"), "the solution is announced")?;
    advance(&tab, 1_000)?;
    check(live(&tab)?.contains("finished"), "the end is announced")
}

// The keyboard and the pointer.

#[test]
fn the_arrow_keys_home_and_end_move_between_squares_and_do_not_move_the_search() -> Result<(), String> {
    let tab = open()?;
    text(&tab, "document.getElementById('nq-r0-c0').focus(), 'ok'")?;
    check_eq(active_cell(&tab)?, "nq-r0-c0".to_owned())?;
    check(ring_shown(&tab)?, "the ring is shown while the grid has the focus")?;
    for (key, expected) in [
        ("ArrowRight", "nq-r0-c1"),
        ("ArrowDown", "nq-r1-c1"),
        ("ArrowLeft", "nq-r1-c0"),
        ("ArrowLeft", "nq-r1-c0"),
        ("ArrowUp", "nq-r0-c0"),
        ("ArrowUp", "nq-r0-c0"),
        ("End", "nq-r0-c7"),
        ("Home", "nq-r0-c0"),
    ] {
        tab.press_key(key).map_err(|e| e.to_string())?;
        check_eq(active_cell(&tab)?, expected.to_owned())?;
        check_eq(focused(&tab)?, expected.to_owned())?;
    }
    tab.press_key("PageDown").map_err(|e| e.to_string())?;
    tab.press_key("Enter").map_err(|e| e.to_string())?;
    check_eq(active_cell(&tab)?, "nq-r0-c0".to_owned())?;
    check_eq(position(&tab)?, 0)?;
    check_eq(count(&tab, &format!("{SVG} [role=gridcell][tabindex=\"0\"]"))?, 1)
}

#[test]
fn control_with_home_and_end_go_to_the_corners() -> Result<(), String> {
    let tab = open()?;
    text(&tab, "document.getElementById('nq-r3-c3').focus(), 'ok'")?;
    for (key, expected) in [("End", "nq-r3-c7")] {
        tab.press_key(key).map_err(|e| e.to_string())?;
        check_eq(active_cell(&tab)?, expected.to_owned())?;
    }
    text(
        &tab,
        "(() => { const g = document.querySelector('#n-queens-stage-svg [role=gridcell][tabindex=\"0\"]'); \
         g.dispatchEvent(new KeyboardEvent('keydown', { key: 'End', ctrlKey: true, bubbles: true })); return 'ok'; })()",
    )?;
    check_eq(active_cell(&tab)?, "nq-r7-c7".to_owned())?;
    text(
        &tab,
        "(() => { const g = document.querySelector('#n-queens-stage-svg [role=gridcell][tabindex=\"0\"]'); \
         g.dispatchEvent(new KeyboardEvent('keydown', { key: 'Home', ctrlKey: true, bubbles: true })); return 'ok'; })()",
    )?;
    check_eq(active_cell(&tab)?, "nq-r0-c0".to_owned())
}

#[test]
fn a_click_on_a_square_moves_the_focus_and_shows_the_ring_but_not_the_search() -> Result<(), String> {
    let tab = open()?;
    seek(&tab, 2)?;
    let at = centre_of(&tab, "nq-r4-c5")?;
    drag(&tab, &[at, at])?;
    check_eq(active_cell(&tab)?, "nq-r4-c5".to_owned())?;
    check_eq(focused(&tab)?, "nq-r4-c5".to_owned())?;
    check(ring_shown(&tab)?, "the ring is shown after a mouse click")?;
    check_eq((position(&tab)?, queens(&tab)?), (2, 1))
}

#[test]
fn the_ring_is_hidden_when_the_focus_leaves_the_grid() -> Result<(), String> {
    let tab = open()?;
    text(&tab, "document.getElementById('nq-r2-c2').focus(), 'ok'")?;
    check(ring_shown(&tab)?, "shown")?;
    text(&tab, "document.getElementById('n-queens-next').focus(), 'ok'")?;
    check(!ring_shown(&tab)?, "hidden")?;
    text(&tab, "document.getElementById('nq-r2-c2').focus(), 'ok'")?;
    check(ring_shown(&tab)?, "shown again")
}

#[test]
fn the_ring_stays_on_the_active_square_across_steps() -> Result<(), String> {
    let tab = open()?;
    text(&tab, "document.getElementById('nq-r1-c1').focus(), 'ok'")?;
    seek(&tab, 3)?;
    check_eq(focused(&tab)?, "nq-r1-c1".to_owned())?;
    check(ring_shown(&tab)?, "still shown")
}

// A new size.

#[test]
fn a_new_size_replaces_the_grid_and_clamps_the_active_square() -> Result<(), String> {
    let tab = open()?;
    text(&tab, "document.getElementById('nq-r7-c7').focus(), 'ok'")?;
    select(&tab, "n-queens-size", "4")?;
    check_eq(count(&tab, &format!("{SVG} [role=gridcell]"))?, 16)?;
    check_eq(count(&tab, &format!("{SVG} [role=row]"))?, 4)?;
    check_eq((position(&tab)?, last_step(&tab)?), (0, 139))?;
    check_eq(field(&tab, "depth")?, "0 / 4".to_owned())?;
    check_eq(field(&tab, "solutions")?, "0 / 2".to_owned())?;
    check_eq(active_cell(&tab)?, "nq-r3-c3".to_owned())?;
    check_eq(attribute(&tab, "#n-queens-seek", "max")?, "139".to_owned())?;
    click(&tab, "n-queens-next-solution")?;
    check_eq(field(&tab, "vector")?, "[2, 4, 1, 3]".to_owned())?;
    click(&tab, "n-queens-next-solution")?;
    check_eq(field(&tab, "vector")?, "[3, 1, 4, 2]".to_owned())
}

#[test]
fn every_size_of_board_shows_its_own_number_of_solutions() -> Result<(), String> {
    let tab = open()?;
    for (n, solutions) in [(4, 2), (5, 10), (6, 4), (7, 40), (9, 352)] {
        select(&tab, "n-queens-size", &n.to_string())?;
        check_eq(field(&tab, "solutions")?, format!("0 / {solutions}"))?;
        check_eq(count(&tab, &format!("{SVG} [role=gridcell]"))?, n * n)?;
    }
    Ok(())
}

// A write that fails.

#[test]
fn a_write_that_fails_is_reported_and_the_next_button_draws_the_board_again() -> Result<(), String> {
    let tab = open()?;
    seek(&tab, 2)?;
    fail_writes_of(&tab, "d", true)?;
    click(&tab, "n-queens-next")?;
    fail_writes_of(&tab, "d", false)?;
    check(!banner_hidden(&tab)?, "the failure is reported")?;
    check(content(&tab, "#n-queens-error")?.contains("may be out of date"), "and says why")?;
    check_eq(position(&tab)?, 2)?;
    click(&tab, "n-queens-next")?;
    check_eq(position(&tab)?, 3)?;
    check(banner_hidden(&tab)?, "a draw that works hides it")?;
    check(
        rays(&tab)?.iter().all(|(class, d)| class == "nq-ray" && !d.is_empty()),
        "the rays are drawn",
    )
}

#[test]
fn a_button_with_nowhere_to_go_still_draws_a_board_that_is_out_of_date() -> Result<(), String> {
    let tab = open()?;
    let last = last_step(&tab)?;
    seek(&tab, last)?;
    fail_writes_of(&tab, "class", true)?;
    click(&tab, "n-queens-previous")?;
    fail_writes_of(&tab, "class", false)?;
    check(!banner_hidden(&tab)?, "reported")?;
    check_eq(position(&tab)?, last)?;
    click(&tab, "n-queens-next")?;
    check_eq(position(&tab)?, last)?;
    check(banner_hidden(&tab)?, "repaired by a button that had nowhere to go")?;
    check_eq(field(&tab, "kind")?, "Exhausted".to_owned())?;
    check_eq(queens(&tab)?, 0)
}

#[test]
fn a_failure_while_playing_stops_the_playback() -> Result<(), String> {
    let tab = open_manual()?;
    click(&tab, "n-queens-play")?;
    advance(&tab, 2)?;
    fail_writes_of(&tab, "d", true)?;
    advance(&tab, 1)?;
    fail_writes_of(&tab, "d", false)?;
    check_eq((waiting(&tab)?, content(&tab, "#n-queens-play")?), (0, "Play".to_owned()))?;
    check_eq(position(&tab)?, 2)?;
    check(!banner_hidden(&tab)?, "reported")
}

// The cost.

#[test]
fn a_step_and_a_jump_on_the_largest_board_are_quick() -> Result<(), String> {
    let tab = open()?;
    select(&tab, "n-queens-size", "9")?;
    check_eq(last_step(&tab)?, 153_502)?;
    let started = Instant::now();
    seek(&tab, 100_000)?;
    for _ in 0..20 {
        click(&tab, "n-queens-next")?;
    }
    let took = started.elapsed();
    eprintln!("n-queens: a jump and 20 steps on a board of 9 took {took:?}");
    check(took < Duration::from_secs(5), "a jump and twenty steps take under five seconds")
}

// The controls in the scene.

#[test]
fn the_selectors_the_buttons_and_the_range_are_all_inside_the_scene() -> Result<(), String> {
    let tab = open()?;
    check_eq(count(&tab, &format!("{SVG} [data-action]"))?, 6)?;
    check_eq(count(&tab, &format!("{SVG} foreignObject select"))?, 2)?;
    check_eq(count(&tab, &format!("{SVG} foreignObject input[type=\"range\"]"))?, 1)?;
    // None of them is left in the page's own HTML, outside the scene.
    check_eq(
        text(
            &tab,
            "String(Array.from(document.querySelectorAll('#panel-n-queens button, #panel-n-queens select, \
             #panel-n-queens input')).filter(c => !c.closest('#n-queens-stage-svg')).length)",
        )?,
        "0".to_owned(),
    )?;
    for id in ["restart", "previous", "next", "play", "previous-solution", "next-solution"] {
        check_eq(count(&tab, &format!("{SVG} g#n-queens-{id}[role=\"button\"]"))?, 1)?;
    }
    check_eq(attribute(&tab, "#n-queens-size", "aria-label")?, "Board size".to_owned())?;
    check_eq(attribute(&tab, "#n-queens-speed", "aria-label")?, "Speed".to_owned())?;
    check_eq(attribute(&tab, "#n-queens-seek", "aria-label")?, "Go to step".to_owned())?;
    check_eq(text(&tab, "document.getElementById('n-queens-size').value")?, "8".to_owned())?;
    check_eq(text(&tab, "document.getElementById('n-queens-speed').value")?, "20".to_owned())
}

#[test]
fn the_scene_is_wide_enough_for_the_controls_and_none_of_them_is_over_the_board() -> Result<(), String> {
    let tab = open()?;
    check_eq(attribute(&tab, SVG, "width")?, "640".to_owned())?;
    check_eq(attribute(&tab, SVG, "height")?, "636".to_owned())?;
    let overlap = text(
        &tab,
        &format!(
            "(() => {{ const scene = document.querySelector('{SVG}').getBoundingClientRect(); \
             const board = document.querySelector('{SVG} .nq-fills').getBoundingClientRect(); \
             const controls = Array.from(document.querySelectorAll('{SVG} [data-action], {SVG} foreignObject')); \
             const bad = controls.filter(c => {{ const r = c.getBoundingClientRect(); \
               return r.left < scene.left || r.right > scene.right || r.bottom > board.top || r.width === 0; }}); \
             return controls.length + ':' + bad.length; }})()"
        ),
    )?;
    check_eq(overlap, "9:0".to_owned())
}

#[test]
fn a_dimmed_button_is_still_in_the_tab_order() -> Result<(), String> {
    let tab = open()?;
    check_eq(attribute(&tab, "#n-queens-previous", "aria-disabled")?, "true".to_owned())?;
    check_eq(attribute(&tab, "#n-queens-previous", "tabindex")?, "0".to_owned())?;
    check_eq(attribute(&tab, "#n-queens-next", "tabindex")?, "0".to_owned())
}

#[test]
fn a_button_works_from_the_keyboard() -> Result<(), String> {
    let tab = open()?;
    text(&tab, "document.getElementById('n-queens-next').focus(), 'ok'")?;
    tab.press_key("Enter").map_err(|e| e.to_string())?;
    check_eq(position(&tab)?, 1)?;
    tab.press_key(" ").map_err(|e| e.to_string())?;
    check_eq(position(&tab)?, 2)?;
    check_eq(focused(&tab)?, "n-queens-next".to_owned())
}

#[test]
fn the_speed_selector_says_the_speed_in_the_status() -> Result<(), String> {
    let tab = open()?;
    select(&tab, "n-queens-speed", "50")?;
    check_eq(field(&tab, "playback")?, "Paused at 50 events per second".to_owned())?;
    select(&tab, "n-queens-speed", "1")?;
    check_eq(field(&tab, "playback")?, "Paused at 1 events per second".to_owned())
}

#[test]
fn a_new_size_leaves_the_controls_where_they_were() -> Result<(), String> {
    let tab = open()?;
    let before = text(
        &tab,
        &format!(
            "Array.from(document.querySelectorAll('{SVG} [data-action], {SVG} foreignObject')).map(c => {{ const r = c.getBoundingClientRect(); return [r.left, r.top, r.width].join(','); }}).join(';')"
        ),
    )?;
    select(&tab, "n-queens-size", "5")?;
    let after = text(
        &tab,
        &format!(
            "Array.from(document.querySelectorAll('{SVG} [data-action], {SVG} foreignObject')).map(c => {{ const r = c.getBoundingClientRect(); return [r.left, r.top, r.width].join(','); }}).join(';')"
        ),
    )?;
    check_eq(after, before)?;
    check_eq(text(&tab, "document.getElementById('n-queens-size').value")?, "5".to_owned())?;
    check_eq(attribute(&tab, "#n-queens-seek", "max")?, last_step(&tab)?.to_string())
}
