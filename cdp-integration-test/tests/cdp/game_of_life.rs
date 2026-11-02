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

use crate::common::{DemoTab, drag, mouse_event, new_demo_tab};
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

#[test]
fn the_scene_has_the_board_thirteen_buttons_and_a_slider() -> Result<(), String> {
    let tab = open()?;
    check_eq(attribute(&tab, SVG, "width")?, "560".to_owned())?;
    check_eq(attribute(&tab, SVG, "height")?, format!("{}", TOP + CELL * ROWS as f64 + 80.0))?;
    check_eq(count(&tab, &format!("{SVG} [data-action]"))?, 13)?;
    check_eq(count(&tab, &format!("{SVG} input[data-slider=\"speed\"]"))?, 1)?;
    check_eq(count(&tab, &format!("{SVG} .life-board"))?, 1)?;
    let names: Vec<String> = ["play", "step", "random", "clear", "wrap"]
        .iter()
        .map(|a| label_of(&tab, a))
        .collect::<Result<_, _>>()?;
    check_eq(
        names,
        ["Pause", "Step", "Random", "Clear", "Wrap edges"].map(str::to_owned).to_vec(),
    )?;
    for pattern in ["glider", "blinker", "r-pentomino", "acorn", "glider-gun"] {
        check_eq(count(&tab, &format!("{SVG} [data-action=\"pattern-{pattern}\"]"))?, 1)?;
    }
    let ships: Vec<String> = ["lwss", "mwss", "hwss"]
        .iter()
        .map(|id| label_of(&tab, &format!("spaceship-{id}")))
        .collect::<Result<_, _>>()?;
    check_eq(
        ships,
        ["Light spaceship", "Medium spaceship", "Heavy spaceship"]
            .map(str::to_owned)
            .to_vec(),
    )?;
    Ok(())
}

#[test]
fn the_page_starts_with_a_glider_that_is_going() -> Result<(), String> {
    let tab = open()?;
    check_eq(population(&tab)?, 5)?;
    check_eq(label_of(&tab, "play")?, "Pause".to_owned())?;
    wait_until(&tab, 10, |tab| Ok(generation(tab)? > 0))?;
    check_eq(population(&tab)?, 5)
}

#[test]
fn pause_stops_the_generations_and_play_starts_them_again() -> Result<(), String> {
    let tab = open()?;
    pause(&tab)?;
    let stopped = generation(&tab)?;
    std::thread::sleep(Duration::from_millis(700));
    check_eq(generation(&tab)?, stopped)?;
    press_button(&tab, "play")?;
    check_eq(label_of(&tab, "play")?, "Pause".to_owned())?;
    wait_until(&tab, 10, |tab| Ok(generation(tab)? > stopped))
}

#[test]
fn step_stops_the_generations_and_works_out_exactly_one() -> Result<(), String> {
    let tab = open()?;
    press_button(&tab, "step")?;
    check_eq(label_of(&tab, "play")?, "Play".to_owned())?;
    let after_one = generation(&tab)?;
    let path = live_path(&tab)?;
    std::thread::sleep(Duration::from_millis(500));
    check_eq(generation(&tab)?, after_one)?;
    press_button(&tab, "step")?;
    check_eq(generation(&tab)?, after_one + 1)?;
    check(live_path(&tab)? != path, "the cells moved")
}

#[test]
fn a_blinker_turns_on_a_step_and_turns_back_on_the_next() -> Result<(), String> {
    let tab = emptied()?;
    for col in 10..=12 {
        click_cell(&tab, col, 5)?;
    }
    check_eq(population(&tab)?, 3)?;
    let across = live_path(&tab)?;
    check_eq(across.matches('M').count(), 1)?;
    press_button(&tab, "step")?;
    let down = live_path(&tab)?;
    check_eq(population(&tab)?, 3)?;
    check_eq(down.matches('M').count(), 3)?;
    check(down != across, "it is a column now")?;
    press_button(&tab, "step")?;
    check_eq(live_path(&tab)?, across)?;
    check_eq(generation(&tab)?, 2)
}

#[test]
fn a_lone_cell_dies_and_a_block_survives() -> Result<(), String> {
    let tab = emptied()?;
    click_cell(&tab, 20, 10)?;
    press_button(&tab, "step")?;
    check_eq(population(&tab)?, 0)?;
    for (col, row) in [(5, 5), (6, 5), (5, 6), (6, 6)] {
        click_cell(&tab, col, row)?;
    }
    let block = live_path(&tab)?;
    press_button(&tab, "step")?;
    press_button(&tab, "step")?;
    check_eq(live_path(&tab)?, block)
}

#[test]
fn a_click_brings_a_cell_to_life_and_a_second_kills_it() -> Result<(), String> {
    let tab = emptied()?;
    click_cell(&tab, 7, 8)?;
    check_eq(population(&tab)?, 1)?;
    check_eq(live_path(&tab)?, format!("M98 {}h14v14h-14z", TOP + 8.0 * CELL))?;
    click_cell(&tab, 7, 8)?;
    check_eq(population(&tab)?, 0)
}

#[test]
fn a_drag_gives_every_cell_it_crosses_the_state_of_the_first() -> Result<(), String> {
    let tab = emptied()?;
    drag_along(&tab, 12, 3, 9)?;
    check_eq(population(&tab)?, 7)?;
    check_eq(live_path(&tab)?.matches('M').count(), 1)?;
    // A drag that starts on a live cell kills each cell that it goes over, and brings none to life.
    drag_along(&tab, 12, 5, 12)?;
    check_eq(population(&tab)?, 2)
}

#[test]
fn a_drag_that_is_reported_only_at_its_ends_still_paints_every_cell_between_them() -> Result<(), String> {
    // Two points are a press and a release, with no move in between, as when the browser coalesces a fast drag.
    let tab = emptied()?;
    drag(&tab, &[centre(&tab, 3, 12)?, centre(&tab, 20, 12)?])?;
    check_eq(population(&tab)?, 18)?;
    check_eq(live_path(&tab)?.matches('M').count(), 1)?;
    check(
        live_path(&tab)?.contains("h252v14h-252z"),
        "one run of 18 cells, which is 252 wide",
    )
}

#[test]
fn a_fast_drag_down_a_column_or_along_a_diagonal_has_no_gaps_either() -> Result<(), String> {
    let tab = emptied()?;
    drag(&tab, &[centre(&tab, 5, 2)?, centre(&tab, 5, 25)?])?;
    check_eq(population(&tab)?, 24)?;
    press_button(&tab, "clear")?;
    drag(&tab, &[centre(&tab, 5, 5)?, centre(&tab, 15, 15)?])?;
    check_eq(population(&tab)?, 11)?;
    let cells = starts(&live_path(&tab)?);
    let expected: Vec<(f64, f64)> = (0..=10).map(|i| ((5 + i) as f64 * CELL, TOP + (5 + i) as f64 * CELL)).collect();
    check_eq(cells, expected)
}

#[test]
fn a_fast_drag_that_starts_on_a_live_cell_kills_every_cell_on_its_way() -> Result<(), String> {
    let tab = emptied()?;
    drag_along(&tab, 8, 2, 30)?;
    check_eq(population(&tab)?, 29)?;
    drag(&tab, &[centre(&tab, 4, 8)?, centre(&tab, 25, 8)?])?;
    check_eq(population(&tab)?, 29 - 22)
}

#[test]
fn a_drag_that_leaves_the_board_and_comes_back_does_not_draw_across_it() -> Result<(), String> {
    let tab = emptied()?;
    let start = centre(&tab, 2, 2)?;
    let above = (start.0, start.1 - 60.0);
    let back = centre(&tab, 30, 20)?;
    drag(&tab, &[start, above, back])?;
    // The two cells where it was pressed and where it came back, and not the line between them.
    check_eq(population(&tab)?, 2)
}

#[test]
fn a_press_in_the_strips_above_and_below_the_board_changes_no_cell() -> Result<(), String> {
    let tab = emptied()?;
    let corner = centre(&tab, 0, 0)?;
    let above = (corner.0 + 200.0, corner.1 - CELL / 2.0 - 3.0);
    drag(&tab, &[above, above])?;
    check_eq(population(&tab)?, 0)
}

#[test]
fn the_arrow_keys_move_the_cursor_which_goes_round_and_space_changes_the_cell() -> Result<(), String> {
    let tab = emptied()?;
    focus_board(&tab)?;
    let cursor = |tab: &Tab| -> Result<(f64, f64), String> {
        let box_ = format!("{SVG} .life-cursor");
        Ok((
            attribute(tab, &box_, "x")?.parse().map_err(|e| format!("{e}"))?,
            attribute(tab, &box_, "y")?.parse().map_err(|e| format!("{e}"))?,
        ))
    };
    let first = cursor(&tab)?;
    check_eq(first, (20.0 * CELL, TOP + 15.0 * CELL))?;
    press(&tab, "ArrowRight")?;
    check_eq(cursor(&tab)?, (21.0 * CELL, TOP + 15.0 * CELL))?;
    press(&tab, "ArrowUp")?;
    check_eq(cursor(&tab)?, (21.0 * CELL, TOP + 14.0 * CELL))?;
    press(&tab, " ")?;
    check_eq(population(&tab)?, 1)?;
    check_eq(
        text(&tab, &format!("document.querySelector('{CURSOR_READOUT}').textContent"))?,
        "Column 22, row 15, alive".to_owned(),
    )?;
    press(&tab, " ")?;
    check_eq(population(&tab)?, 0)?;
    check_eq(
        text(&tab, &format!("document.querySelector('{CURSOR_READOUT}').textContent"))?,
        "Column 22, row 15, dead".to_owned(),
    )?;
    // At the left edge, the cursor comes back at the right one.
    for _ in 0..COLS {
        press(&tab, "ArrowLeft")?;
    }
    check_eq(cursor(&tab)?, (21.0 * CELL, TOP + 14.0 * CELL))?;
    for _ in 0..22 {
        press(&tab, "ArrowLeft")?;
    }
    check_eq(cursor(&tab)?.0, (COLS as f64 - 1.0) * CELL)
}

#[test]
fn the_cursor_is_only_seen_while_the_board_has_the_focus() -> Result<(), String> {
    let tab = open()?;
    let shown = |tab: &Tab| -> Result<String, String> {
        text(
            tab,
            &format!("getComputedStyle(document.querySelector('{SVG} .life-cursor')).display"),
        )
    };
    check_eq(shown(&tab)?, "none".to_owned())?;
    focus_board(&tab)?;
    check_eq(shown(&tab)?, "inline".to_owned())
}

#[test]
fn each_pattern_replaces_the_board_and_starts_the_count_again() -> Result<(), String> {
    let tab = open()?;
    for (pattern, cells) in [
        ("glider-gun", 36),
        ("acorn", 7),
        ("r-pentomino", 5),
        ("blinker", 3),
        ("glider", 5),
    ] {
        press_button(&tab, "step")?;
        press_button(&tab, &format!("pattern-{pattern}"))?;
        check_eq(readout(&tab)?, (0, cells))?;
    }
    Ok(())
}

#[test]
fn the_gosper_gun_makes_a_glider_every_thirty_generations_once_the_edges_stop() -> Result<(), String> {
    let tab = open()?;
    pause(&tab)?;
    press_button(&tab, "wrap")?;
    press_button(&tab, "pattern-glider-gun")?;
    for _ in 0..30 {
        press_button(&tab, "step")?;
    }
    check_eq(readout(&tab)?, (30, 41))
}

#[test]
fn random_fills_a_third_of_the_board_and_clear_empties_it() -> Result<(), String> {
    let tab = open()?;
    pause(&tab)?;
    press_button(&tab, "random")?;
    let (generation, alive) = readout(&tab)?;
    check_eq(generation, 0)?;
    let cells = COLS * ROWS;
    check(
        alive > cells / 5 && alive < cells * 2 / 5,
        &format!("{alive} of {cells} is about 30%"),
    )?;
    press_button(&tab, "clear")?;
    check_eq(readout(&tab)?, (0, 0))?;
    check_eq(live_path(&tab)?, String::new())
}

#[test]
fn where_the_edges_wrap_a_row_on_the_top_edge_is_next_to_the_bottom_one_and_where_they_stop_it_is_not()
-> Result<(), String> {
    let tab = emptied()?;
    check_eq(
        attribute(&tab, &format!("{SVG} [data-action=\"wrap\"]"), "aria-pressed")?,
        "true".to_owned(),
    )?;
    drag_along(&tab, 0, 10, 12)?;
    press_button(&tab, "step")?;
    // The three at the top turn into a column that goes up over the edge and comes in at the bottom.
    check_eq(population(&tab)?, 3)?;
    check(
        live_path(&tab)?.contains(&format!("{}h14", TOP + 29.0 * CELL)),
        "a cell is on the bottom row",
    )?;

    press_button(&tab, "clear")?;
    press_button(&tab, "wrap")?;
    check_eq(
        attribute(&tab, &format!("{SVG} [data-action=\"wrap\"]"), "aria-pressed")?,
        "false".to_owned(),
    )?;
    check(
        attribute(&tab, SVG, "aria-label")?.contains("stop"),
        "the scene says that the edges stop",
    )?;
    drag_along(&tab, 0, 10, 12)?;
    press_button(&tab, "step")?;
    check_eq(population(&tab)?, 2)?;
    check(
        !live_path(&tab)?.contains(&format!("{}h14", TOP + 29.0 * CELL)),
        "nothing is on the bottom row",
    )
}

#[test]
fn the_slider_sets_the_speed_in_whole_numbers_from_1_to_30() -> Result<(), String> {
    let tab = open()?;
    let slider = |tab: &Tab, property: &str| -> Result<String, String> {
        let read = if property.contains('-') {
            format!("i.getAttribute('{property}')")
        } else {
            format!("i.{property}")
        };
        text(
            tab,
            &format!(
                "(() => {{ const i = document.querySelector('{SVG} input[data-slider=\"speed\"]'); \
                 return String({read}); }})()"
            ),
        )
    };
    check_eq(slider(&tab, "min")?, "1".to_owned())?;
    check_eq(slider(&tab, "max")?, "30".to_owned())?;
    check_eq(slider(&tab, "step")?, "1".to_owned())?;
    check_eq(slider(&tab, "value")?, "8".to_owned())?;
    text(
        &tab,
        &format!("document.querySelector('{SVG} input[data-slider=\"speed\"]').focus(), 'ok'"),
    )?;
    press(&tab, "ArrowRight")?;
    check_eq(slider(&tab, "value")?, "9".to_owned())?;
    check_eq(slider(&tab, "aria-valuetext")?, "9 per second".to_owned())
}

#[test]
fn a_faster_speed_works_out_more_generations_in_the_same_time() -> Result<(), String> {
    let tab = open()?;
    let focus_slider = format!("document.querySelector('{SVG} input[data-slider=\"speed\"]').focus(), 'ok'");
    pause(&tab)?;
    text(&tab, &focus_slider)?;
    press(&tab, "End")?;
    press_button(&tab, "clear")?;
    press_button(&tab, "play")?;
    std::thread::sleep(Duration::from_millis(1500));
    let fast = generation(&tab)?;
    pause(&tab)?;
    text(&tab, &focus_slider)?;
    press(&tab, "Home")?;
    press_button(&tab, "clear")?;
    press_button(&tab, "play")?;
    std::thread::sleep(Duration::from_millis(1500));
    let slow = generation(&tab)?;
    check(fast >= 8, &format!("30 a second for a second and a half, not {fast}"))?;
    check(slow <= 3, &format!("1 a second for a second and a half, not {slow}"))?;
    check(fast > slow, "faster is more")
}

#[test]
fn the_board_and_the_scene_describe_themselves_and_the_readout_is_only_announced_while_paused() -> Result<(), String> {
    let tab = open()?;
    let board = attribute(&tab, &format!("{SVG} .life-board"), "aria-label")?;
    check(board.contains("40 columns by 30 rows") && board.contains("arrow keys"), &board)?;
    check_eq(
        attribute(&tab, &format!("{SVG} .life-board"), "role")?,
        "application".to_owned(),
    )?;
    check_eq(attribute(&tab, &format!("{SVG} .life-board"), "tabindex")?, "0".to_owned())?;
    check(
        attribute(&tab, SVG, "aria-label")?.contains("wrap round"),
        "the scene says the edges wrap",
    )?;
    check_eq(attribute(&tab, READOUT, "aria-live")?, "off".to_owned())?;
    pause(&tab)?;
    check_eq(attribute(&tab, READOUT, "aria-live")?, "polite".to_owned())?;
    check_eq(attribute(&tab, CURSOR_READOUT, "aria-live")?, "polite".to_owned())
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

#[test]
fn each_spaceship_is_added_with_the_cells_that_its_weight_says() -> Result<(), String> {
    let tab = emptied()?;
    for (id, cells) in [("lwss", 9), ("mwss", 11), ("hwss", 13)] {
        press_button(&tab, "clear")?;
        press_button(&tab, &format!("spaceship-{id}"))?;
        check_eq(population(&tab)?, cells)?;
    }
    Ok(())
}

#[test]
fn a_spaceship_is_added_at_the_left_edge_level_with_the_cursor() -> Result<(), String> {
    let tab = emptied()?;
    press_button(&tab, "spaceship-lwss")?;
    let first = starts(&live_path(&tab)?);
    // The cursor starts at row 15, so the 4 rows of the ship have their top at row 13.
    check(first.iter().all(|&(x, _)| x < 5.0 * CELL), &format!("{first:?}"))?;
    check_eq(first[0], (0.0, TOP + 13.0 * CELL))?;

    // Five rows up, the cursor is at row 10 counting from 0, and a ship of 4 rows is level with it from row 8.
    press_button(&tab, "clear")?;
    focus_board(&tab)?;
    for _ in 0..5 {
        press(&tab, "ArrowUp")?;
    }
    press_button(&tab, "spaceship-lwss")?;
    check_eq(starts(&live_path(&tab)?)[0], (0.0, TOP + 8.0 * CELL))
}

#[test]
fn a_spaceship_is_added_to_what_is_on_the_board_and_does_not_restart_the_count() -> Result<(), String> {
    let tab = emptied()?;
    click_cell(&tab, 30, 3)?;
    press_button(&tab, "step")?;
    press_button(&tab, "step")?;
    check_eq(readout(&tab)?, (2, 0))?;
    click_cell(&tab, 30, 3)?;
    press_button(&tab, "spaceship-hwss")?;
    check_eq(readout(&tab)?, (2, 14))?;
    press_button(&tab, "spaceship-hwss")?;
    check_eq(population(&tab)?, 14)
}

#[test]
fn adding_a_spaceship_leaves_the_generations_going_if_they_were_going() -> Result<(), String> {
    let tab = open()?;
    check_eq(label_of(&tab, "play")?, "Pause".to_owned())?;
    press_button(&tab, "spaceship-mwss")?;
    check_eq(label_of(&tab, "play")?, "Pause".to_owned())?;
    let seen = generation(&tab)?;
    wait_until(&tab, 10, |tab| Ok(generation(tab)? > seen))
}

#[test]
fn a_spaceship_flies_two_cells_to_the_right_every_four_generations() -> Result<(), String> {
    let tab = emptied()?;
    press_button(&tab, "wrap")?;
    press_button(&tab, "spaceship-lwss")?;
    let before = starts(&live_path(&tab)?);
    for _ in 0..4 {
        press_button(&tab, "step")?;
    }
    let after = starts(&live_path(&tab)?);
    check_eq(population(&tab)?, 9)?;
    let moved: Vec<(f64, f64)> = before.iter().map(|&(x, y)| (x + 2.0 * CELL, y)).collect();
    check_eq(after, moved)?;
    for _ in 0..4 {
        press_button(&tab, "step")?;
    }
    check_eq(starts(&live_path(&tab)?)[0].0, before[0].0 + 4.0 * CELL)
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

#[test]
fn a_drag_that_the_browser_cancels_ends_and_the_pointer_then_paints_nothing_until_it_is_pressed_again()
-> Result<(), String> {
    let tab = emptied()?;
    press_down(&tab, centre(&tab, 3, 5)?)?;
    check_eq(population(&tab)?, 1)?;
    fire(&tab, "pointercancel", MOUSE, centre(&tab, 3, 5)?)?;
    move_held(&tab, centre(&tab, 8, 5)?)?;
    check_eq(population(&tab)?, 1)?;
    let_go(&tab, centre(&tab, 8, 5)?)?;
    check_eq(population(&tab)?, 1)?;
    // The next press starts a drag as usual, so nothing was left over.
    drag(&tab, &[centre(&tab, 10, 5)?, centre(&tab, 12, 5)?])?;
    check_eq(population(&tab)?, 4)
}

#[test]
fn a_drag_whose_pointer_capture_is_lost_ends_without_a_pointer_up() -> Result<(), String> {
    let tab = emptied()?;
    press_down(&tab, centre(&tab, 3, 5)?)?;
    check_eq(
        text(
            &tab,
            &format!("String(document.querySelector('{SVG}').hasPointerCapture({MOUSE}))"),
        )?,
        "true".to_owned(),
    )?;
    // Giving up the capture makes the browser report that it was lost, and nothing else.
    text(
        &tab,
        &format!("document.querySelector('{SVG}').releasePointerCapture({MOUSE}), 'ok'"),
    )?;
    move_held(&tab, centre(&tab, 8, 5)?)?;
    move_held(&tab, centre(&tab, 9, 5)?)?;
    check_eq(population(&tab)?, 1)?;
    let_go(&tab, centre(&tab, 9, 5)?)?;
    drag(&tab, &[centre(&tab, 20, 9)?, centre(&tab, 22, 9)?])?;
    check_eq(population(&tab)?, 4)
}

#[test]
fn a_pointer_that_is_not_making_the_drag_cannot_move_it_on_or_end_it() -> Result<(), String> {
    let tab = emptied()?;
    press_down(&tab, centre(&tab, 3, 5)?)?;
    fire(&tab, "pointermove", 99, centre(&tab, 6, 5)?)?;
    check_eq(population(&tab)?, 1)?;
    fire(&tab, "pointerup", 99, centre(&tab, 6, 5)?)?;
    fire(&tab, "pointercancel", 99, centre(&tab, 6, 5)?)?;
    fire(&tab, "lostpointercapture", 99, centre(&tab, 6, 5)?)?;
    // The drag of the mouse goes on, and joins the cell where it began to the one it is at now.
    move_held(&tab, centre(&tab, 9, 5)?)?;
    check_eq(population(&tab)?, 7)?;
    let_go(&tab, centre(&tab, 9, 5)?)
}

#[test]
fn a_second_pointer_that_goes_down_while_one_is_dragging_does_not_start_a_drag_of_its_own() -> Result<(), String> {
    let tab = emptied()?;
    press_down(&tab, centre(&tab, 3, 5)?)?;
    fire(&tab, "pointerdown", 99, centre(&tab, 20, 10)?)?;
    check_eq(population(&tab)?, 1)?;
    fire(&tab, "pointermove", 99, centre(&tab, 22, 10)?)?;
    check_eq(population(&tab)?, 1)?;
    let_go(&tab, centre(&tab, 3, 5)?)
}

#[test]
fn a_drag_whose_end_was_never_reported_is_ended_by_the_next_pointer_that_goes_down() -> Result<(), String> {
    let tab = emptied()?;
    // The browser has no pointer 7 to capture, so this drag has no capture, and nothing will report that it ends.
    fire(&tab, "pointerdown", 7, centre(&tab, 3, 5)?)?;
    check_eq(population(&tab)?, 1)?;
    fire(&tab, "pointerdown", 8, centre(&tab, 6, 5)?)?;
    check_eq(population(&tab)?, 2)?;
    // Pointer 7 is not making the drag now, and pointer 8 is.
    fire(&tab, "pointermove", 7, centre(&tab, 12, 5)?)?;
    check_eq(population(&tab)?, 2)?;
    fire(&tab, "pointermove", 8, centre(&tab, 8, 5)?)?;
    check_eq(population(&tab)?, 4)?;
    fire(&tab, "pointerup", 8, centre(&tab, 8, 5)?)?;
    fire(&tab, "pointermove", 8, centre(&tab, 10, 5)?)?;
    check_eq(population(&tab)?, 4)
}

#[test]
fn a_pointer_that_is_let_go_paints_nothing_when_it_moves_again() -> Result<(), String> {
    let tab = emptied()?;
    fire(&tab, "pointerdown", 5, centre(&tab, 2, 2)?)?;
    fire(&tab, "pointerup", 5, centre(&tab, 2, 2)?)?;
    fire(&tab, "pointermove", 5, centre(&tab, 9, 2)?)?;
    check_eq(population(&tab)?, 1)?;
    // The loss of the capture that follows the pointer being let go is not a second end, and does no harm.
    fire(&tab, "lostpointercapture", 5, centre(&tab, 9, 2)?)?;
    check_eq(population(&tab)?, 1)?;
    drag(&tab, &[centre(&tab, 10, 2)?, centre(&tab, 11, 2)?])?;
    check_eq(population(&tab)?, 3)
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

#[test]
fn a_cell_that_cannot_be_drawn_is_reported_and_the_next_cell_draws_both() -> Result<(), String> {
    let tab = emptied()?;
    check(banner_hidden(&tab)?, "there is no error to begin with")?;
    // The cells of the live path are written as its `d`, and nothing else on the page is written as one.
    fail_writes_of(&tab, "d", true)?;
    click_cell(&tab, 5, 5)?;
    fail_writes_of(&tab, "d", false)?;
    check(!banner_hidden(&tab)?, "the page says that it went wrong")?;
    let said = banner(&tab)?;
    check(said.contains("may be out of date") && said.contains("draws it again"), &said)?;
    // The board has the cell, and the page does not show it yet.
    check_eq(live_path(&tab)?, String::new())?;
    check_eq(population(&tab)?, 0)?;

    click_cell(&tab, 9, 5)?;
    check_eq(population(&tab)?, 2)?;
    check_eq(live_path(&tab)?.matches('M').count(), 2)?;
    check(banner_hidden(&tab)?, "a draw that works hides the error")
}

#[test]
fn a_generation_that_cannot_be_drawn_is_drawn_by_a_control_that_draws_nothing_itself() -> Result<(), String> {
    let tab = emptied()?;
    for col in 10..=12 {
        click_cell(&tab, col, 5)?;
    }
    let across = live_path(&tab)?;
    fail_writes_of(&tab, "d", true)?;
    press_button(&tab, "step")?;
    fail_writes_of(&tab, "d", false)?;
    check(!banner_hidden(&tab)?, "the page says that it went wrong")?;
    // The board has gone on a generation, and the page still shows the one before.
    check_eq(live_path(&tab)?, across)?;
    check_eq(generation(&tab)?, 0)?;
    // Moving the slider does not draw the board, but it draws it again first.
    text(
        &tab,
        &format!("document.querySelector('{SVG} input[data-slider=\"speed\"]').focus(), 'ok'"),
    )?;
    press(&tab, "ArrowRight")?;
    check_eq(generation(&tab)?, 1)?;
    check_eq(live_path(&tab)?.matches('M').count(), 3)?;
    check(banner_hidden(&tab)?, "and what the failure said is hidden")
}

#[test]
fn a_frame_that_cannot_be_drawn_stops_the_generations_says_so_on_the_button_and_play_draws_it_again()
-> Result<(), String> {
    let tab = open()?;
    check_eq(label_of(&tab, "play")?, "Pause".to_owned())?;
    fail_writes_of(&tab, "d", true)?;
    wait_until(&tab, 10, |tab| Ok(!banner_hidden(tab)?))?;
    fail_writes_of(&tab, "d", false)?;
    // It has stopped, and the button says what it will do, which is not what it said before.
    check_eq(label_of(&tab, "play")?, "Play".to_owned())?;
    check_eq(attribute(&tab, READOUT, "aria-live")?, "polite".to_owned())?;
    let stopped = generation(&tab)?;
    std::thread::sleep(Duration::from_millis(500));
    check_eq(generation(&tab)?, stopped)?;
    press_button(&tab, "play")?;
    check(banner_hidden(&tab)?, "Play draws the board again first, and the error goes")?;
    check_eq(label_of(&tab, "play")?, "Pause".to_owned())?;
    wait_until(&tab, 10, |tab| Ok(generation(tab)? > stopped + 1))
}

#[test]
fn a_cell_changed_by_a_key_that_cannot_be_drawn_is_drawn_by_the_next_key() -> Result<(), String> {
    let tab = emptied()?;
    focus_board(&tab)?;
    fail_writes_of(&tab, "d", true)?;
    press(&tab, " ")?;
    fail_writes_of(&tab, "d", false)?;
    check(!banner_hidden(&tab)?, "the page says that it went wrong")?;
    check_eq(live_path(&tab)?, String::new())?;
    press(&tab, "ArrowRight")?;
    check_eq(population(&tab)?, 1)?;
    check_eq(live_path(&tab)?.matches('M').count(), 1)?;
    check(banner_hidden(&tab)?, "the error goes")
}

#[test]
fn an_error_that_goes_away_during_a_click_does_not_move_the_board_under_the_pointer_and_paint_other_cells()
-> Result<(), String> {
    let tab = emptied()?;
    fail_writes_of(&tab, "d", true)?;
    click_cell(&tab, 5, 5)?;
    fail_writes_of(&tab, "d", false)?;
    check(
        !banner_hidden(&tab)?,
        "the error is shown above the board, which moves the board down",
    )?;
    // This click draws, which hides the error and would move the board up under the pointer, if that was not left until
    // the click is over. Cells that the pointer was then over are not to be painted.
    click_cell(&tab, 9, 5)?;
    check_eq(population(&tab)?, 2)?;
    check_eq(live_path(&tab)?.matches('M').count(), 2)?;
    check(banner_hidden(&tab)?, "and the error does go, when the click is over")
}

#[test]
fn an_error_that_appears_during_a_drag_does_not_move_the_board_under_the_pointer_and_paint_other_cells()
-> Result<(), String> {
    let tab = emptied()?;
    fail_writes_of(&tab, "d", true)?;
    drag(&tab, &[centre(&tab, 5, 5)?, centre(&tab, 9, 5)?])?;
    fail_writes_of(&tab, "d", false)?;
    check(!banner_hidden(&tab)?, "the error is shown when the drag is over")?;
    // The board has the 5 cells that the pointer crossed, and the page shows them when it is next drawn.
    text(
        &tab,
        &format!("document.querySelector('{SVG} input[data-slider=\"speed\"]').focus(), 'ok'"),
    )?;
    press(&tab, "ArrowRight")?;
    check_eq(population(&tab)?, 5)?;
    check_eq(live_path(&tab)?.matches('M').count(), 1)
}
