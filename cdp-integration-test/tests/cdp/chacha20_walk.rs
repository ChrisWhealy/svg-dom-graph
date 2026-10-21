//! The ChaCha20 demo's own stepping toolbar, driven by real clicks and key presses in Chrome.
//!
//! The demo's arithmetic is covered by native unit tests. What they cannot reach is the browser-facing behaviour of the
//! walk: which diagram and which values the page shows at each position, what a redraw keeps (zoom, pan, keyboard
//! focus) and whether nodes stay draggable inside the diagram after it has been redrawn at a new size.
//!
//! The walk has 961 positions: `None` (unstarted), then `0..=959` for the 80 quarter rounds of 12 steps each, then 960
//! for the keystream block. Moving between 959 and 960 is the one transition that redraws the whole scene, so those two
//! are where a redraw could drop the view, the focus or the drag limits.
//!
//! Unlike the other scenarios in this binary, these run against the built demo gallery's `chacha20-window.html`, not
//! the fixture. See [`crate::common::new_demo_tab`].

use crate::common::{DemoTab, drag, mouse_event, new_demo_tab};
use headless_chrome::{Tab, protocol::cdp::Input};
use std::time::{Duration, Instant};

const DIAGRAM: &str = "#chacha20-diagram";
/// Large enough to hold the whole diagram, which is taller than Chrome's headless window, with room to drag past it.
const VIEWPORT: (u32, u32) = (1800, 1800);
/// The last quarter round's last step, and the keystream block that follows it.
const LAST_STEP: usize = 959;
const KEYSTREAM: usize = 960;

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
fn open() -> Result<DemoTab, String> {
    new_demo_tab("chacha20-window.html", &format!("{DIAGRAM} g[aria-label^=\"Step:\"]"), VIEWPORT)
}

fn check(condition: bool, message: &str) -> Result<(), String> {
    if condition { Ok(()) } else { Err(message.to_owned()) }
}

/// Evaluates `expression` in the page and returns its string value.
fn text(tab: &Tab, expression: &str) -> Result<String, String> {
    tab.evaluate(expression, false)
        .map_err(|e| format!("could not evaluate {expression}: {e}"))?
        .value
        .and_then(|v| v.as_str().map(str::to_owned))
        .ok_or_else(|| format!("{expression} did not return a string"))
}

/// The aria-label of the first element matching `selector`, or an empty string if none does.
fn label(tab: &Tab, selector: &str) -> Result<String, String> {
    text(
        tab,
        &format!("(document.querySelector('{selector}')?.getAttribute('aria-label') ?? '')"),
    )
}

/// The walk's position, read from the hidden "Step" driver's own selection. `None` is the unstarted walk.
fn position(tab: &Tab) -> Result<Option<usize>, String> {
    let name = label(tab, &format!("{DIAGRAM} g[aria-label^=\"Step:\"]"))?;
    let Some((before, _)) = name.split_once(" selected") else {
        return Ok(None);
    };
    before
        .rsplit("cell ")
        .next()
        .and_then(|n| n.parse().ok())
        .map(Some)
        .ok_or_else(|| format!("could not read a cell from {name:?}"))
}

fn heading(tab: &Tab) -> Result<String, String> {
    text(tab, "(document.querySelector('[role=heading]')?.textContent ?? '')")
}

/// Whether the keystream block, and not the quarter round, is the diagram on show.
fn is_keystream(tab: &Tab) -> Result<bool, String> {
    Ok(!label(tab, &format!("{DIAGRAM} g[aria-label^=\"keystream block\"]"))?.is_empty())
}

/// The aria-label of the `index`th operator node whose label starts with `operator`.
fn operator(tab: &Tab, operator: &str, index: usize) -> Result<String, String> {
    text(
        tab,
        &format!(
            "([...document.querySelectorAll('{DIAGRAM} g[aria-label^=\"{operator} result\"]')][{index}]?.getAttribute('aria-label') ?? '')"
        ),
    )
}

/// Waits until `position` returns something other than `from`, or fails after five seconds.
fn wait_for_move(tab: &Tab, from: Option<usize>) -> Result<(), String> {
    let deadline = Instant::now() + Duration::from_secs(5);
    while Instant::now() < deadline {
        if position(tab)? != from {
            // A move to the keystream block redraws the scene. Give the redraw its own moment to finish attaching.
            std::thread::sleep(Duration::from_millis(100));
            return Ok(());
        }
        std::thread::sleep(Duration::from_millis(20));
    }
    Err(format!("the walk never left {from:?}"))
}

fn button<'a>(tab: &'a Tab, name: &str) -> Result<headless_chrome::Element<'a>, String> {
    tab.find_element(&format!("{DIAGRAM} g[aria-label=\"{name}\"]"))
        .map_err(|e| format!("could not find the {name:?} button: {e}"))
}

/// Presses a toolbar button with a real mouse click and waits for the walk to move.
fn click(tab: &Tab, name: &str) -> Result<(), String> {
    let from = position(tab)?;
    button(tab, name)?
        .click()
        .map_err(|e| format!("could not click {name:?}: {e}"))?;
    wait_for_move(tab, from)
}

/// Walks to `target`, taking the 48-step round jumps while the distance allows and single steps for the rest.
fn goto(tab: &Tab, target: usize) -> Result<(), String> {
    for _ in 0..200 {
        let now = position(tab)?;
        if now == Some(target) {
            return Ok(());
        }
        let distance = target as isize - now.map_or(-1, |p| p as isize);
        let name = match distance {
            48.. => "Next round",
            1.. => "Next selection",
            ..=-48 => "Previous round",
            _ => "Previous selection",
        };
        click(tab, name)?;
    }
    Err(format!("could not reach position {target}, stopped at {:?}", position(tab)?))
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The first step is the first quarter round's first `ADD`: before it, every operator is unreached.
#[test]
fn the_unstarted_walk_moves_to_the_first_step_of_the_first_quarter_round() -> Result<(), String> {
    let tab = open()?;
    check(position(&tab)?.is_none(), "the walk did not start unstarted")?;
    check(
        heading(&tab)?.contains("round 1 of 20: column quarter round 1 of 4"),
        &format!("unexpected heading {:?}", heading(&tab)?),
    )?;
    let first_add = operator(&tab, "ADD", 0)?;
    check(
        first_add.contains("not yet computed"),
        &format!("the first ADD was reached: {first_add}"),
    )?;

    click(&tab, "Next selection")?;
    check(position(&tab)? == Some(0), "Next did not move to position 0")?;
    let first_add = operator(&tab, "ADD", 0)?;
    // 0x61707865 + 0x03020100, the RFC 8439 block's constant and first key word.
    check(
        first_add.contains("64 72 79 65") && !first_add.contains("not yet computed"),
        &format!("the first ADD does not hold its result: {first_add}"),
    )?;
    check(
        operator(&tab, "XOR", 0)?.contains("not yet computed"),
        "the first XOR was reached too soon",
    )
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Position 11 is the first quarter round's last step, and 12 begins the second one, on the second column.
#[test]
fn position_12_begins_the_next_quarter_round() -> Result<(), String> {
    let tab = open()?;
    goto(&tab, 11)?;
    check(
        heading(&tab)?.ends_with("column quarter round 1 of 4"),
        &format!("unexpected heading at 11: {:?}", heading(&tab)?),
    )?;
    check(
        !operator(&tab, "XOR", 0)?.contains("not yet computed"),
        "the first quarter round was not complete at position 11",
    )?;
    let before = label(&tab, &format!("{DIAGRAM} g[aria-label^=\"state before\"]"))?;
    check(
        before.contains("highlighted: cells 0, 4, 8, 12"),
        &format!("column 0 at 11: {before}"),
    )?;

    click(&tab, "Next selection")?;
    check(position(&tab)? == Some(12), "did not move to 12")?;
    check(
        heading(&tab)?.ends_with("column quarter round 2 of 4"),
        &format!("unexpected heading at 12: {:?}", heading(&tab)?),
    )?;
    check(
        operator(&tab, "XOR", 0)?.contains("not yet computed"),
        "the second quarter round started with its first XOR already reached",
    )?;
    let before = label(&tab, &format!("{DIAGRAM} g[aria-label^=\"state before\"]"))?;
    check(
        before.contains("highlighted: cells 1, 5, 9, 13"),
        &format!("column 1 at 12: {before}"),
    )
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Position 47 ends the first round, which mixes columns, and 48 begins the second, which mixes diagonals.
#[test]
fn position_48_changes_from_a_column_round_to_a_diagonal_round() -> Result<(), String> {
    let tab = open()?;
    goto(&tab, 47)?;
    check(
        heading(&tab)?.contains("round 1 of 20: column quarter round 4 of 4"),
        &format!("unexpected heading at 47: {:?}", heading(&tab)?),
    )?;

    click(&tab, "Next selection")?;
    check(position(&tab)? == Some(48), "did not move to 48")?;
    check(
        heading(&tab)?.contains("round 2 of 20: diagonal quarter round 1 of 4"),
        &format!("unexpected heading at 48: {:?}", heading(&tab)?),
    )?;
    let before = label(&tab, &format!("{DIAGRAM} g[aria-label^=\"state before\"]"))?;
    check(
        before.contains("highlighted: cells 0, 5, 10, 15"),
        &format!("diagonal 0 at 48: {before}"),
    )
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The scene's content layer `transform`, which is its zoom and pan, or an empty string if it has none.
fn view(tab: &Tab) -> Result<String, String> {
    text(
        tab,
        &format!("(document.querySelector('{DIAGRAM} > g.svg-dom-graph-content')?.getAttribute('transform') ?? '')"),
    )
}

/// The aria-label of the element holding keyboard focus.
fn focused(tab: &Tab) -> Result<String, String> {
    text(
        tab,
        "(document.activeElement?.closest?.('[aria-label]')?.getAttribute('aria-label') ?? '')",
    )
}

/// Moves to `name` by keyboard alone: focus the button, press Enter, and wait for the walk to move.
fn press(tab: &Tab, name: &str) -> Result<(), String> {
    let from = position(tab)?;
    button(tab, name)?
        .focus()
        .map_err(|e| format!("could not focus {name:?}: {e}"))?;
    tab.press_key("Enter")
        .map_err(|e| format!("could not press Enter on {name:?}: {e}"))?;
    wait_for_move(tab, from)
}

/// Zooms in once with the toolbar and pans the scene with a real drag of empty background.
fn zoom_and_pan(tab: &Tab) -> Result<(), String> {
    button(tab, "Zoom in")?.click().map_err(|e| format!("could not zoom in: {e}"))?;
    let origin = text(
        tab,
        &format!(
            "(() => {{ const r = document.querySelector('{DIAGRAM}').getBoundingClientRect(); return r.left + ',' + r.top; }})()"
        ),
    )?;
    let (left, top) = origin.split_once(',').ok_or("no svg origin")?;
    let (left, top): (f64, f64) = (left.parse().map_err(|_| "bad left")?, top.parse().map_err(|_| "bad top")?);
    // Empty background: left of every node (the first starts at x = 40), and well below the title.
    let start = (left + 10.0, top + 700.0);
    mouse_event(tab, Input::DispatchMouseEventTypeOption::MouseMoved, start, None)?;
    drag(
        tab,
        &[start, (start.0 + 10.0, start.1 + 15.0), (start.0 + 25.0, start.1 + 30.0)],
    )?;
    check(!view(tab)?.is_empty(), "the zoom and pan did not change the view")
}

/// Crossing from the last quarter round to the keystream block redraws the whole scene. The view, the toolbar and the
/// keyboard focus must come through it, and through the redraw back again.
#[test]
fn the_keystream_block_redraw_keeps_the_view_the_toolbar_and_the_focus() -> Result<(), String> {
    let tab = open()?;
    goto(&tab, LAST_STEP)?;
    check(!is_keystream(&tab)?, "the keystream block was shown before its position")?;
    check(
        heading(&tab)?.contains("round 20 of 20"),
        &format!("unexpected heading at 959: {:?}", heading(&tab)?),
    )?;
    zoom_and_pan(&tab)?;
    let view_before = view(&tab)?;

    press(&tab, "Next selection")?;
    check(position(&tab)? == Some(KEYSTREAM), "did not move to 960")?;
    check(is_keystream(&tab)?, "the keystream block is not on show at 960")?;
    check(
        heading(&tab)?.contains("keystream block"),
        &format!("unexpected heading at 960: {:?}", heading(&tab)?),
    )?;
    check(
        view(&tab)? == view_before,
        &format!("the view changed: {view_before:?} -> {:?}", view(&tab)?),
    )?;
    check(
        focused(&tab)? == "Next selection",
        &format!("focus was lost, and is on {:?}", focused(&tab)?),
    )?;
    // The toolbar is rebuilt with the scene, so every button must be there and the walk must still be at 960.
    for name in [
        "Previous round",
        "Previous selection",
        "Next selection",
        "Next round",
        "Restart selection",
    ] {
        button(&tab, name)?;
    }

    press(&tab, "Previous selection")?;
    check(position(&tab)? == Some(LAST_STEP), "did not move back to 959")?;
    check(!is_keystream(&tab)?, "the keystream block is still on show at 959")?;
    check(
        heading(&tab)?.contains("round 20 of 20"),
        &format!("unexpected heading back at 959: {:?}", heading(&tab)?),
    )?;
    check(
        view(&tab)? == view_before,
        &format!("the view changed on the way back: {:?}", view(&tab)?),
    )?;
    check(
        focused(&tab)? == "Previous selection",
        &format!("focus was lost on the way back, and is on {:?}", focused(&tab)?),
    )?;
    // The quarter round must be whole again, with every value shown.
    check(
        !operator(&tab, "XOR", 0)?.contains("not yet computed"),
        "the redrawn quarter round lost its values",
    )
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Restart goes back to the unstarted walk from anywhere: a quarter round, a later round, or the keystream block.
#[test]
fn restart_returns_to_the_unstarted_diagram_from_any_position() -> Result<(), String> {
    let tab = open()?;
    for from in [30, KEYSTREAM] {
        goto(&tab, from)?;
        click(&tab, "Restart selection")?;
        check(
            position(&tab)?.is_none(),
            &format!("restart from {from} left the walk at {:?}", position(&tab)?),
        )?;
        check(
            !is_keystream(&tab)?,
            &format!("restart from {from} left the keystream block on show"),
        )?;
        check(
            heading(&tab)?.contains("round 1 of 20: column quarter round 1 of 4"),
            &format!("restart from {from} gave the heading {:?}", heading(&tab)?),
        )?;
        for index in 0..4 {
            let add = operator(&tab, "ADD", index)?;
            check(
                add.contains("u32 = 00 00 00 00") && add.contains("not yet computed"),
                &format!("restart from {from} left ADD {index} with a value: {add}"),
            )?;
        }
    }
    Ok(())
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// A node's box and the view box, as `(x, y, width, height, view_width, view_height)`.
fn geometry(tab: &Tab, prefix: &str) -> Result<[f64; 6], String> {
    let reading = text(
        tab,
        &format!(
            "(() => {{
                const g = document.querySelector('{DIAGRAM} g[aria-label^=\"{prefix}\"]');
                const [x, y] = g.getAttribute('transform').match(/-?[0-9.]+/g);
                const rect = g.querySelector('rect');
                const view = document.querySelector('{DIAGRAM}').viewBox.baseVal;
                return [x, y, rect.getAttribute('width'), rect.getAttribute('height'), view.width, view.height].join(',');
            }})()"
        ),
    )?;
    let numbers: Vec<f64> = reading
        .split(',')
        .map(|n| n.parse().map_err(|_| format!("{n:?} in {reading:?} is not a number")))
        .collect::<Result<_, _>>()?;
    numbers.try_into().map_err(|_| format!("unexpected geometry {reading:?}"))
}

/// Drags the node labelled `prefix` past the `<svg>`'s bottom-right corner, then past its top-left corner, and checks
/// it stops inside the view box both times.
///
/// The drag limits are those the node was given when it was drawn. If they were read before the `<svg>` was fitted to
/// its content, they are the old, larger size, and a node can be dragged out of the visible diagram.
fn drag_to_each_corner(tab: &Tab, prefix: &str) -> Result<(), String> {
    let rect = tab
        .find_element(&format!("{DIAGRAM} g[aria-label^=\"{prefix}\"] rect"))
        .map_err(|e| format!("could not find {prefix:?}: {e}"))?;
    let edges = text(
        tab,
        &format!(
            "(() => {{ const r = document.querySelector('{DIAGRAM}').getBoundingClientRect(); return [r.left, r.top, r.right, r.bottom].join(','); }})()"
        ),
    )?;
    let edges: Vec<f64> = edges.split(',').filter_map(|n| n.parse().ok()).collect();
    let [left, top, right, bottom] = edges[..] else {
        return Err(format!("unexpected svg edges {edges:?}"));
    };

    for (name, target) in [
        ("bottom-right", (right + 120.0, bottom + 120.0)),
        ("top-left", (left - 120.0, top - 120.0)),
    ] {
        let start = rect.get_midpoint().map_err(|e| format!("could not locate {prefix:?}: {e}"))?;
        drag(
            tab,
            &[
                (start.x, start.y),
                ((start.x + target.0) / 2.0, (start.y + target.1) / 2.0),
                target,
            ],
        )?;
        std::thread::sleep(Duration::from_millis(100));
        let [x, y, width, height, view_width, view_height] = geometry(tab, prefix)?;
        check(
            x >= -1.0 && y >= -1.0 && x + width <= view_width + 1.0 && y + height <= view_height + 1.0,
            &format!(
                "{prefix:?} dragged to the {name} sits at ({x}, {y}) size {width}x{height}, outside the {view_width}x{view_height} view box"
            ),
        )?;
    }
    Ok(())
}

/// Dragging stays inside the diagram's own edges in both layouts, and after changing between them: the `<svg>` is
/// resized to fit each, so each must get its own limits.
#[test]
fn nodes_stay_inside_the_view_box_in_both_layouts_and_after_changing_between_them() -> Result<(), String> {
    let tab = open()?;
    drag_to_each_corner(&tab, "state before")?;

    goto(&tab, KEYSTREAM)?;
    drag_to_each_corner(&tab, "keystream block")?;

    goto(&tab, LAST_STEP)?;
    drag_to_each_corner(&tab, "state before")
}
