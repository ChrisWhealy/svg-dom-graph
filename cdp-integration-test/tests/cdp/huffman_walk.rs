//! The Huffman demo's own window: its stepping toolbar and its text input node, driven by real clicks and key presses in
//! Chrome.
//!
//! The algorithm and the walk are covered by native unit tests. What they cannot reach is the browser-facing behaviour:
//! what the window shows at each position, that a step backwards restores what a step forwards drew, that editing the
//! text starts the walk again, which nodes the route through the tree marks, and what a redraw keeps (zoom, and keyboard
//! focus on a button or in the text box). Every step draws the whole scene again, so every step is a place where a
//! redraw could drop one of those.
//!
//! Most scenarios set the text to "abracadabra", which is small enough to work by hand. It has 4 merges and 5 distinct
//! bytes, so its walk has 11 positions: `0` counts, `1..=4` merge, `5` reads the lengths and `6..=10` assign the
//! canonical codes. Pass two continues from there, with one position for each input byte and a last one for the
//! flush. The demo's own default text is the first sentence of *Pride and Prejudice*, which has its own
//! scenarios below.
//!
//! Like [`crate::chacha20_walk`], these run against the built demo gallery's window page, not the fixture.

use crate::common::{DemoTab, new_demo_tab};
use headless_chrome::Tab;
use std::time::{Duration, Instant};

const DIAGRAM: &str = "#huffman-diagram";
const VIEWPORT: (u32, u32) = (1800, 2400);
const LAST: usize = 10;
/// "abracadabra" has 11 bytes, so pass two has 11 positions of packing and a last one for the flush.
const FLUSH: usize = LAST + 12;
/// The fill the library gives a cell marked by a secondary selection.
const SECONDARY_FILL: &str = "#7fd8be";

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The demo as it opens, with its own default text.
fn open_default() -> Result<DemoTab, String> {
    new_demo_tab("huffman-window.html", &format!("{DIAGRAM} g[aria-label^=\"Step:\"]"), VIEWPORT)
}

/// The demo with the text set to "abracadabra".
fn open() -> Result<DemoTab, String> {
    let tab = open_default()?;
    set_text(&tab, "abracadabra")?;
    Ok(tab)
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

/// The text of every box in the diagram, in the order they were drawn.
fn texts(tab: &Tab) -> Result<Vec<String>, String> {
    let json = text(
        tab,
        &format!("JSON.stringify([...document.querySelectorAll('{DIAGRAM} text')].map(t => t.textContent))"),
    )?;
    serde_json::from_str(&json).map_err(|e| format!("could not read the box texts: {e}"))
}

/// The lines of the `index`th Markdown node, joined with a space, which rebuilds its text. The first is the status line
/// and the second is the explanation of the step.
fn markdown(tab: &Tab, index: usize) -> Result<String, String> {
    text(
        tab,
        &format!(
            "[...document.querySelectorAll('{DIAGRAM} g[aria-label=\"Text\"]')][{index}] \
             ?.querySelectorAll('text') ? [...[...document.querySelectorAll('{DIAGRAM} g[aria-label=\"Text\"]')][{index}] \
             .querySelectorAll('text')].map(t => t.textContent).join(' ') : ''"
        ),
    )
}

/// What the step explains.
fn explanation(tab: &Tab) -> Result<String, String> {
    markdown(tab, 1)
}

/// What the status line says.
fn status(tab: &Tab) -> Result<String, String> {
    markdown(tab, 0)
}

/// The code table's five columns for each distinct byte, in the order shown, without the headings.
fn table(tab: &Tab) -> Result<Vec<Vec<String>>, String> {
    let all = texts(tab)?;
    let start = all.iter().position(|t| t == "Byte").ok_or("the table has no heading")?;
    // The hidden "Step" driver is drawn straight after the table, and is the first box that is not part of it.
    let cells: Vec<String> = all[start + 5..].iter().take_while(|c| *c != "Step").cloned().collect();
    Ok(cells.chunks(5).map(<[String]>::to_vec).collect())
}

/// How many leaves and branches the forest shows. A leaf's box reads "x n" for the byte `x` and its count `n`, and a
/// branch's reads only its weight, which is at least 2. The boxes that label a link read "0" or "1".
fn forest(tab: &Tab) -> Result<(usize, usize), String> {
    let all = texts(tab)?;
    let end = all.iter().position(|t| t == "Byte").ok_or("the table has no heading")?;
    let forest = &all[..end];
    let leaves = forest
        .iter()
        .filter(|t| {
            t.rsplit_once(" x")
                .is_some_and(|(_, n)| !n.is_empty() && n.bytes().all(|b| b.is_ascii_digit()))
        })
        .count();
    let branches = forest.iter().filter(|t| t.parse::<u32>().is_ok_and(|w| w >= 2)).count();
    Ok((leaves, branches))
}

/// The texts of the boxes the library has marked with a secondary selection, sorted. The marked box of a node with one
/// value is its outer rectangle, which is a sibling of its text.
fn marked(tab: &Tab) -> Result<Vec<String>, String> {
    let json = text(
        tab,
        &format!(
            "JSON.stringify([...document.querySelectorAll('{DIAGRAM} rect')] \
                 .filter(r => r.getAttribute('fill') === '{SECONDARY_FILL}') \
                 .map(r => r.parentElement.querySelector('text')?.textContent ?? '').sort())"
        ),
    )?;
    serde_json::from_str(&json).map_err(|e| format!("could not read the marked boxes: {e}"))
}

/// The walk's position, read from the hidden "Step" driver's own selection.
fn position(tab: &Tab) -> Result<Option<usize>, String> {
    let name = text(
        tab,
        &format!("(document.querySelector('{DIAGRAM} g[aria-label^=\"Step:\"]')?.getAttribute('aria-label') ?? '')"),
    )?;
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

/// Waits until `position` returns something other than `from`, or fails after five seconds.
fn wait_for_move(tab: &Tab, from: Option<usize>) -> Result<(), String> {
    let deadline = Instant::now() + Duration::from_secs(5);
    while Instant::now() < deadline {
        if position(tab)? != from {
            // Every move draws the scene again. Give the redraw its own moment to finish attaching.
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

/// Walks to `target` one step at a time.
fn goto(tab: &Tab, target: usize) -> Result<(), String> {
    for _ in 0..80 {
        let now = position(tab)?.unwrap_or(0);
        if now == target {
            return Ok(());
        }
        click(tab, if now < target { "Next selection" } else { "Previous selection" })?;
    }
    Err(format!("could not reach position {target}, stopped at {:?}", position(tab)?))
}

/// Replaces the text box's value and fires the `input` event a real keystroke would.
fn set_text(tab: &Tab, value: &str) -> Result<(), String> {
    tab.evaluate(
        &format!(
            "(() => {{ const i = document.querySelector('{DIAGRAM} input'); i.value = {value:?}; \
             i.dispatchEvent(new Event('input', {{ bubbles: true }})); }})()"
        ),
        false,
    )
    .map_err(|e| format!("could not edit the text box: {e}"))?;
    std::thread::sleep(Duration::from_millis(250));
    Ok(())
}

fn row(symbol: &str, count: &str, path: &str, length: &str, code: &str) -> Vec<String> {
    [symbol, count, path, length, code].map(str::to_owned).to_vec()
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The walk starts by counting: five trees of one node, the counts in the table, and nothing else filled in.
#[test]
fn the_walk_starts_by_counting_with_five_single_node_trees() -> Result<(), String> {
    let tab = open()?;
    check_eq(position(&tab)?, Some(0))?;
    check(
        explanation(&tab)?.starts_with("Counted 11 bytes, 5 of them distinct"),
        &format!("unexpected explanation {:?}", explanation(&tab)?),
    )?;
    check_eq(forest(&tab)?, (5, 0))?;
    check_eq(status(&tab)?, "11 bytes, 5 distinct, 4 merges.".to_owned())?;
    // The commonest byte first, and bytes that occur as often as each other in byte order.
    check_eq(
        table(&tab)?,
        vec![
            row("a", "5", "–", "–", "–"),
            row("b", "2", "–", "–", "–"),
            row("r", "2", "–", "–", "–"),
            row("c", "1", "–", "–", "–"),
            row("d", "1", "–", "–", "–"),
        ],
    )
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The first merge joins `c` and `d`, the two lightest, and draws the new branch with its two labelled links.
#[test]
fn the_first_merge_joins_the_two_lightest_trees() -> Result<(), String> {
    let tab = open()?;
    click(&tab, "Next selection")?;
    check_eq(position(&tab)?, Some(1))?;
    check(
        explanation(&tab)?
            .starts_with("Merge 1 of 4: the two lightest trees are 'c' (1) and 'd' (1). Joined, they weigh 2."),
        &format!("unexpected explanation {:?}", explanation(&tab)?),
    )?;
    check_eq(forest(&tab)?, (5, 1))?;
    // Both links of the one branch are labelled, left 0 and right 1.
    let all = texts(&tab)?;
    let forest_boxes = &all[..all.iter().position(|t| t == "Byte").ok_or("the table has no heading")?];
    check_eq(forest_boxes.iter().filter(|t| *t == "0").count(), 1)?;
    check_eq(forest_boxes.iter().filter(|t| *t == "1").count(), 1)
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// After the last merge there is one tree of nine boxes, and the table still has no lengths.
#[test]
fn the_last_merge_leaves_one_tree() -> Result<(), String> {
    let tab = open()?;
    goto(&tab, 4)?;
    check(
        explanation(&tab)?.starts_with(
            "Merge 4 of 4: the two lightest trees are 'a' (5) and the tree of weight 6. Joined, they weigh 11.",
        ),
        &format!("unexpected explanation {:?}", explanation(&tab)?),
    )?;
    // Five leaves and four branches, with eight links each labelled.
    check_eq(forest(&tab)?, (5, 4))?;
    check(
        table(&tab)?.iter().all(|r| r[2..] == ["–", "–", "–"]),
        "the table was filled in too soon",
    )
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Reading the lengths fills in the tree paths and lengths, and no canonical code yet. Then each step adds one code.
#[test]
fn the_lengths_come_before_the_canonical_codes_which_come_one_at_a_time() -> Result<(), String> {
    let tab = open()?;
    goto(&tab, 5)?;
    check(
        explanation(&tab)?.starts_with("Each code's length is the depth of its leaf"),
        "lengths explanation",
    )?;
    check_eq(
        table(&tab)?,
        vec![
            row("a", "5", "0", "1", "–"),
            row("b", "2", "110", "3", "–"),
            row("r", "2", "111", "3", "–"),
            row("c", "1", "100", "3", "–"),
            row("d", "1", "101", "3", "–"),
        ],
    )?;

    goto(&tab, 6)?;
    check(
        explanation(&tab)?.starts_with("Canonical codes start at all zeros: 'a' has length 1 and code 0."),
        &format!("unexpected explanation {:?}", explanation(&tab)?),
    )?;
    check_eq(table(&tab)?[0][4].as_str(), "0")?;
    check_eq(table(&tab)?[1][4].as_str(), "–")
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The last position has every canonical code. Where the tree's path and the canonical code differ, both are shown.
#[test]
fn the_last_step_has_every_canonical_code_and_shows_where_they_differ_from_the_tree() -> Result<(), String> {
    let tab = open()?;
    goto(&tab, LAST)?;
    check_eq(
        table(&tab)?,
        vec![
            row("a", "5", "0", "1", "0"),
            row("b", "2", "110", "3", "100"),
            row("r", "2", "111", "3", "111"),
            row("c", "1", "100", "3", "101"),
            row("d", "1", "101", "3", "110"),
        ],
    )?;
    // Pass one ends here, but the walk goes on into pass two.
    click(&tab, "Next selection")?;
    check_eq(position(&tab)?, Some(LAST + 1))
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// A step back from the end draws exactly what a step forward to the same position did.
#[test]
fn stepping_back_restores_what_stepping_forward_drew() -> Result<(), String> {
    let tab = open()?;
    goto(&tab, 3)?;
    let forward = (explanation(&tab)?, forest(&tab)?, table(&tab)?, marked(&tab)?);

    goto(&tab, LAST)?;
    goto(&tab, 3)?;
    check_eq((explanation(&tab)?, forest(&tab)?, table(&tab)?, marked(&tab)?), forward)?;

    goto(&tab, 0)?;
    check(explanation(&tab)?.starts_with("Counted 11"), "back at the start")?;
    check_eq(forest(&tab)?, (5, 0))
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Editing the text starts the walk again, from the new text.
#[test]
fn editing_the_text_starts_the_walk_again_from_the_new_text() -> Result<(), String> {
    let tab = open()?;
    goto(&tab, 7)?;

    set_text(&tab, "aab")?;
    check_eq(position(&tab)?, Some(0))?;
    check_eq(status(&tab)?, "3 bytes, 2 distinct, 1 merges.".to_owned())?;
    check_eq(forest(&tab)?, (2, 0))?;
    check_eq(table(&tab)?, vec![row("a", "2", "–", "–", "–"), row("b", "1", "–", "–", "–")])?;

    // 1 counting step, 1 merge, 1 lengths step, 2 codes.
    goto(&tab, 4)?;
    check_eq(table(&tab)?, vec![row("a", "2", "1", "1", "0"), row("b", "1", "0", "1", "1")])?;
    check_eq(position(&tab)?, Some(4))
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// A text that cannot be encoded says why, and leaves the diagram as it was.
#[test]
fn text_that_cannot_be_encoded_leaves_the_diagram_alone() -> Result<(), String> {
    let tab = open()?;
    goto(&tab, 2)?;
    let before = (explanation(&tab)?, table(&tab)?);

    set_text(&tab, "")?;
    check(
        status(&tab)?.starts_with("Nothing to encode."),
        &format!("status {:?}", status(&tab)?),
    )?;
    check_eq((explanation(&tab)?, table(&tab)?), before.clone())?;

    set_text(&tab, &"x".repeat(129))?;
    check(
        status(&tab)?.starts_with("129 bytes is too long. The limit is 128."),
        &format!("status {:?}", status(&tab)?),
    )?;
    check_eq((explanation(&tab)?, table(&tab)?), before)?;
    check_eq(position(&tab)?, Some(2))?;
    // The box still holds what was typed, so that it can be corrected.
    check_eq(
        text(&tab, &format!("document.querySelector('{DIAGRAM} input').value"))?.len(),
        129,
    )?;

    // The limit is in bytes: 65 two-byte characters are 130 bytes.
    set_text(&tab, &"é".repeat(65))?;
    check(
        status(&tab)?.starts_with("130 bytes is too long. The limit is 128."),
        &format!("status {:?}", status(&tab)?),
    )
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// A character of several bytes is counted as its separate bytes, each shown in hexadecimal.
#[test]
fn a_multi_byte_character_is_counted_as_its_separate_bytes() -> Result<(), String> {
    let tab = open()?;
    set_text(&tab, "é")?;
    check_eq(status(&tab)?, "2 bytes, 2 distinct, 1 merges.".to_owned())?;
    check_eq(
        table(&tab)?,
        vec![row("0xA9", "1", "–", "–", "–"), row("0xC3", "1", "–", "–", "–")],
    )
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// One distinct byte has nothing to merge, and is given a 1-bit code.
#[test]
fn one_distinct_byte_is_given_a_one_bit_code() -> Result<(), String> {
    let tab = open()?;
    set_text(&tab, "aaaa")?;
    check_eq(status(&tab)?, "4 bytes, 1 distinct, 0 merges.".to_owned())?;
    goto(&tab, 2)?;
    check_eq(table(&tab)?, vec![row("a", "4", "(root)", "1", "0")])?;
    // The route to the only byte is that byte's own box.
    check_eq(marked(&tab)?, vec!["a x4".to_owned()])
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The zoom survives a step, which draws the scene again.
#[test]
fn the_zoom_survives_a_redraw() -> Result<(), String> {
    let tab = open()?;
    button(&tab, "Zoom in")?.click().map_err(|e| format!("{e}"))?;
    std::thread::sleep(Duration::from_millis(200));
    let view = |tab: &Tab| {
        text(
            tab,
            &format!(
                "(document.querySelector('{DIAGRAM} > g.svg-dom-graph-content')?.getAttribute('transform') ?? '')"
            ),
        )
    };
    let zoomed = view(&tab)?;
    check(!zoomed.is_empty(), "zooming in did nothing")?;

    click(&tab, "Next selection")?;
    check_eq(view(&tab)?, zoomed)
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The keyboard focus stays on the toolbar button that was pressed, so a keyboard user can press it again.
#[test]
fn keyboard_focus_stays_on_the_pressed_button() -> Result<(), String> {
    let tab = open()?;
    button(&tab, "Next selection")?.focus().map_err(|e| format!("{e}"))?;
    let from = position(&tab)?;
    tab.press_key("Enter").map_err(|e| format!("{e}"))?;
    wait_for_move(&tab, from)?;
    check_eq(position(&tab)?, Some(1))?;
    check_eq(
        text(&tab, "document.activeElement?.getAttribute('aria-label') ?? ''")?,
        "Next selection".to_owned(),
    )?;

    // Pressed again without touching the mouse, it moves on.
    let from = position(&tab)?;
    tab.press_key("Enter").map_err(|e| format!("{e}"))?;
    wait_for_move(&tab, from)?;
    check_eq(position(&tab)?, Some(2))
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Typing in the text box, with a real keyboard, draws the scene again and leaves the keyboard focus and the caret in the
/// new text box, so that the next key goes where the last did.
#[test]
fn typing_keeps_the_focus_and_the_caret_through_each_redraw() -> Result<(), String> {
    let tab = open()?;
    let input = tab
        .find_element(&format!("{DIAGRAM} input"))
        .map_err(|e| format!("no text box: {e}"))?;
    input.click().map_err(|e| format!("{e}"))?;
    tab.press_key("End").map_err(|e| format!("{e}"))?;
    tab.type_str("xyz").map_err(|e| format!("{e}"))?;
    std::thread::sleep(Duration::from_millis(300));

    let value = text(&tab, &format!("document.querySelector('{DIAGRAM} input').value"))?;
    check_eq(value.as_str(), "abracadabraxyz")?;
    check_eq(status(&tab)?, "14 bytes, 8 distinct, 7 merges.".to_owned())?;
    check_eq(text(&tab, "document.activeElement.tagName.toLowerCase()")?, "input".to_owned())?;

    // The caret is where the typing left it, not at the start or the end of a fresh box. Move it, then type.
    tab.press_key("ArrowLeft").map_err(|e| format!("{e}"))?;
    tab.press_key("ArrowLeft").map_err(|e| format!("{e}"))?;
    tab.type_str("!").map_err(|e| format!("{e}"))?;
    std::thread::sleep(Duration::from_millis(300));
    check_eq(
        text(&tab, &format!("document.querySelector('{DIAGRAM} input').value"))?.as_str(),
        "abracadabrax!yz",
    )
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The demo opens on the first sentence of *Pride and Prejudice*, with its punctuation: 26 distinct bytes in 116.
#[test]
fn the_demo_opens_on_the_first_sentence_of_pride_and_prejudice() -> Result<(), String> {
    let tab = open_default()?;
    check_eq(status(&tab)?, "116 bytes, 26 distinct, 25 merges.".to_owned())?;
    check_eq(position(&tab)?, Some(0))?;
    check_eq(forest(&tab)?, (26, 0))?;
    let rows = table(&tab)?;
    check_eq(rows.len(), 26)?;
    // The space is 22 of the 116 bytes, so it is first. The comma, the full stop and the capital letter are bytes like any
    // other, and occur once each.
    check_eq(rows[0].clone(), row("space", "22", "–", "–", "–"))?;
    let counts: Vec<(&str, &str)> = rows.iter().map(|r| (r[0].as_str(), r[1].as_str())).collect();
    for expected in [(",", "1"), (".", "1"), ("I", "1"), ("a", "9"), ("n", "9")] {
        check(counts.contains(&expected), &format!("{expected:?} is not in {counts:?}"))?;
    }
    // Most often first, so the counts never go up.
    let numbers: Vec<u32> = rows.iter().filter_map(|r| r[1].parse().ok()).collect();
    check(
        numbers.windows(2).all(|w| w[0] >= w[1]),
        &format!("not in frequency order: {numbers:?}"),
    )?;
    check_eq(
        text(&tab, &format!("document.querySelector('{DIAGRAM} input').value"))?.len(),
        116,
    )
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Every merge of the long sentence works, ends in one tree of 26 leaves and 25 branches, and then every byte has a code.
#[test]
fn the_long_sentence_walks_from_26_trees_to_one_and_then_to_26_codes() -> Result<(), String> {
    let tab = open_default()?;
    goto(&tab, 25)?;
    check_eq(forest(&tab)?, (26, 25))?;
    check(
        explanation(&tab)?.starts_with("Merge 25 of 25"),
        &format!("unexpected explanation {:?}", explanation(&tab)?),
    )?;

    goto(&tab, 52)?;
    check_eq(position(&tab)?, Some(52))?;
    let rows = table(&tab)?;
    check(
        rows.iter().all(|r| r[2..].iter().all(|c| c != "–")),
        "a byte has no code at the last position",
    )?;
    // The space is by far the commonest byte, so it has the shortest code and its canonical code is all zeros.
    check(
        rows[0][4].chars().all(|c| c == '0'),
        &format!("the space's code is {:?}", rows[0][4]),
    )
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Each step explains itself in up to three paragraphs, and a merge says which side is which.
#[test]
fn each_step_explains_what_it_did_and_why() -> Result<(), String> {
    let tab = open()?;
    let counting = explanation(&tab)?;
    check(counting.contains("Each distinct byte is now a tree of one node"), &counting)?;
    check(counting.contains("rare bytes end up deepest"), &counting)?;

    click(&tab, "Next selection")?;
    let merge = explanation(&tab)?;
    check(merge.contains("The lighter goes left (0) and the other right (1)."), &merge)?;
    check(merge.ends_with("4 trees remain."), &merge)?;

    // The second merge joins 'b' and 'r', and the tree made by the first ties with 'r'.
    click(&tab, "Next selection")?;
    let tie = explanation(&tab)?;
    check(tie.contains("Another tree also weighs 2. Ties go to the tree made first"), &tie)
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The explanation of a step is bold where it names the merge, and is not drawn with its Markdown markers showing.
#[test]
fn the_explanation_is_formatted_and_shows_no_markers() -> Result<(), String> {
    let tab = open()?;
    click(&tab, "Next selection")?;
    let bold = text(
        &tab,
        &format!(
            "JSON.stringify([...document.querySelectorAll('{DIAGRAM} g[aria-label=\"Text\"]')][1] \
                 .querySelectorAll('tspan[font-weight=bold]') ? [...[...document.querySelectorAll('{DIAGRAM} g[aria-label=\"Text\"]')][1] \
                 .querySelectorAll('tspan[font-weight=bold]')].map(t => t.textContent) : [])"
        ),
    )?;
    check_eq(bold.as_str(), "[\"Merge 1 of 4\"]")?;
    check(!explanation(&tab)?.contains("**"), "the markers of bold text are showing")
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// A byte that Markdown reads as formatting is drawn as itself in the explanation, never as emphasis.
#[test]
fn a_byte_that_markdown_would_read_as_formatting_is_drawn_as_itself() -> Result<(), String> {
    let tab = open()?;
    set_text(&tab, "a*b*c_d_e")?;
    // Counting, then merges until the star is named, which is the third of the lightest trees to meet.
    let mut seen = String::new();
    for position in 0..=8 {
        goto(&tab, position)?;
        seen.push_str(&explanation(&tab)?);
        seen.push('\n');
    }
    check(
        seen.contains("'*'") && seen.contains("'_'"),
        &format!("the bytes are not named as themselves: {seen}"),
    )?;
    check(!seen.contains('\\'), &format!("a backslash of an escape is showing: {seen}"))
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The heading row of the table has a fill of its own, and the rows below it share the usual one.
#[test]
fn the_heading_row_has_a_fill_of_its_own() -> Result<(), String> {
    let tab = open()?;
    let json = text(
        &tab,
        &format!(
            "(() => {{ const cells = [...document.querySelectorAll('{DIAGRAM} g')].filter(g => ['Byte','Count','Tree path','Length','Code','a','5'] \
                 .includes(g.querySelector(':scope > text')?.textContent ?? '')); \
                 return JSON.stringify(cells.map(g => [g.querySelector('text').textContent, g.querySelector('rect').getAttribute('fill')])); }})()"
        ),
    )?;
    let fills: Vec<(String, String)> = serde_json::from_str(&json).map_err(|e| format!("{e}"))?;
    let fill_of = |label: &str| fills.iter().find(|(t, _)| t == label).map(|(_, f)| f.clone());
    let heading = fill_of("Byte").ok_or("no heading cell")?;
    for label in ["Count", "Tree path", "Length", "Code"] {
        check_eq(fill_of(label).as_ref(), Some(&heading))?;
    }
    // The first body row, a, and its count: both the usual light blue, and not the heading's.
    let body = fill_of("5").ok_or("no body cell")?;
    check(body != heading, &format!("the heading and the body share the fill {heading}"))
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// As each canonical code is assigned, the route from the root to that byte's leaf is marked with the library's secondary
/// selection: the boxes on it, and the label of each of its links. Nothing is marked at any other step.
#[test]
fn the_route_to_each_byte_is_marked_as_its_canonical_code_is_assigned() -> Result<(), String> {
    let tab = open()?;
    for position in 0..=5 {
        goto(&tab, position)?;
        check_eq(marked(&tab)?, Vec::<String>::new())?;
    }

    // 'a' is the root's left child: the root, `a`, and the one link, labelled 0.
    goto(&tab, 6)?;
    check_eq(marked(&tab)?, ["0", "11", "a x5"].map(str::to_owned).to_vec())?;

    // 'b' is 110: right, right, left. The root, 6 and 4 are on the route, and so is `b`, with links 1, 1 and 0.
    goto(&tab, 7)?;
    check_eq(
        marked(&tab)?,
        ["0", "1", "1", "11", "4", "6", "b x2"].map(str::to_owned).to_vec(),
    )?;

    // 'c' is 100: right, left, left. The root, 6 and 2, and `c`, with links 1, 0 and 0.
    goto(&tab, 8)?;
    check_eq(
        marked(&tab)?,
        ["0", "0", "1", "11", "2", "6", "c x1"].map(str::to_owned).to_vec(),
    )?;

    // Going back clears the old route and marks the new one: it is never added to.
    goto(&tab, 6)?;
    check_eq(marked(&tab)?, ["0", "11", "a x5"].map(str::to_owned).to_vec())
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The route's boxes are the right number deep for the byte's code length, in the long sentence too: each step marks a
/// box for every level down to the leaf, and a label for every link.
#[test]
fn a_marked_route_has_a_box_for_every_level_and_a_label_for_every_link() -> Result<(), String> {
    let tab = open_default()?;
    goto(&tab, 27)?;
    // Position 27 gives the first canonical code, to the byte with the shortest code. Its row is ringed and its length
    // is the number of links on the route.
    let rows = table(&tab)?;
    let length: usize = rows[0][3].parse().map_err(|_| "no length")?;
    let marked = marked(&tab)?;
    // The route has `length + 1` boxes and `length` labels, which read 0 or 1.
    let labels = marked.iter().filter(|t| *t == "0" || *t == "1").count();
    check_eq(labels, length)?;
    check_eq(marked.len(), 2 * length + 1)
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Every link has room for an elbow and an arrowhead on each side of its `0` or `1` label.
///
/// An elbow bends midway, and the second half of it holds the arrowhead, which is 10 pixels long, and 6 more of
/// clearance. So the gap above a label, and the gap below it, must each be at least twice that: 32 pixels. The
/// rectangles are read from the rendered boxes, so this checks what is drawn, not the constants that drew it.
#[test]
fn every_link_has_room_for_an_elbow_and_an_arrowhead() -> Result<(), String> {
    const MINIMUM_GAP: f64 = 32.0;
    let tab = open_default()?;
    goto(&tab, 25)?;

    // Every box in the forest, as [text, horizontal middle, top, height], in the scene's own units.
    let json = text(
        &tab,
        &format!(
            "JSON.stringify([...document.querySelectorAll('{DIAGRAM} text')].map(t => {{ \
                 const r = t.closest('g')?.querySelector('rect'); \
                 if (!r) return null; \
                 const b = r.getBBox(); \
                 return [t.textContent, b.x + b.width / 2, b.y, b.height]; \
             }}).filter(Boolean))"
        ),
    )?;
    let boxes: Vec<(String, f64, f64, f64)> =
        serde_json::from_str(&json).map_err(|e| format!("could not read the boxes: {e}"))?;
    let end = boxes.iter().position(|b| b.0 == "Byte").ok_or("the table has no heading")?;
    let forest = &boxes[..end];

    // A link's label is a box of one bit. A branch's weight is at least 2, so only a label reads 0 or 1.
    let labels: Vec<_> = forest.iter().filter(|b| b.0 == "0" || b.0 == "1").collect();
    check_eq(labels.len(), 50)?;
    for (_, x, top, height) in labels {
        let bottom = top + height;
        // The box straight below the label is the child it points at.
        let below = forest
            .iter()
            .filter(|b| (b.1 - x).abs() < 0.5 && b.2 >= bottom - 0.01)
            .map(|b| b.2 - bottom)
            .fold(f64::INFINITY, f64::min);
        // The lowest box wholly above the label is on the parent's level.
        let above = forest
            .iter()
            .filter(|b| b.2 + b.3 <= *top + 0.01)
            .map(|b| top - (b.2 + b.3))
            .fold(f64::INFINITY, f64::min);
        check(
            below >= MINIMUM_GAP - 0.5,
            &format!("only {below} pixels below the label at x={x}"),
        )?;
        check(
            above >= MINIMUM_GAP - 0.5,
            &format!("only {above} pixels above the label at x={x}"),
        )?;
    }
    Ok(())
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The gallery's own panel is only a description and a button that opens the window.
#[test]
fn the_gallery_panel_has_a_button_that_opens_the_window() -> Result<(), String> {
    let tab = new_demo_tab("index.html#panel-huffman", "#huffman-open", (1400, 1400))?;
    check_eq(
        text(&tab, "document.getElementById('huffman-open').textContent.trim()")?,
        "Open the demo in its own window".to_owned(),
    )?;
    check_eq(
        text(&tab, "String(document.querySelectorAll('#panel-huffman svg').length)")?,
        "0".to_owned(),
    )
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The caret is kept in UTF-16 code units, as the browser counts them. After a character that is two units, such as an
/// emoji, the redrawn text box has the caret just after it, not in the middle of it.
#[test]
fn the_caret_survives_a_redraw_after_a_character_of_two_utf16_units() -> Result<(), String> {
    let tab = open()?;
    // "a", an emoji of two units, then "b", with the caret after the emoji: unit 3.
    text(
        &tab,
        &format!(
            "(() => {{ const i = document.querySelector('{DIAGRAM} input'); i.focus(); \
                i.value = 'a\\u{{1F600}}b'; i.setSelectionRange(3, 3); \
                i.dispatchEvent(new Event('input', {{ bubbles: true }})); return 'ok'; }})()"
        ),
    )?;
    std::thread::sleep(Duration::from_millis(300));
    let caret = text(
        &tab,
        &format!(
            "(() => {{ const i = document.querySelector('{DIAGRAM} input'); \
                return i.selectionStart + ',' + i.selectionEnd + ',' + (document.activeElement === i); }})()"
        ),
    )?;
    check_eq(caret.as_str(), "3,3,true")?;
    tab.type_str("!").map_err(|e| format!("{e}"))?;
    std::thread::sleep(Duration::from_millis(300));
    check_eq(
        text(&tab, &format!("document.querySelector('{DIAGRAM} input').value"))?.as_str(),
        "a\u{1F600}!b",
    )
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
//   Pass two
// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
// "abracadabra" has the canonical codes a=0 b=100 c=101 d=110 r=111, so it packs to the bytes 0x4E, 0xAC and 0x9C with a
// bit of padding. The 'a' that is the fourth input byte completes the first byte.

/// The texts that follow the box titled `heading`, for as long as `keep` accepts them.
fn after(tab: &Tab, heading: &str, keep: impl Fn(&str) -> bool) -> Result<Vec<String>, String> {
    let all = texts(tab)?;
    let start = all
        .iter()
        .position(|t| t == heading)
        .ok_or_else(|| format!("there is no {heading:?} box"))?;
    Ok(all[start + 1..].iter().take_while(|t| keep(t)).cloned().collect())
}

fn is_hex_byte(t: &str) -> bool {
    t.len() == 2 && t.bytes().all(|b| b.is_ascii_digit() || (b'A'..=b'F').contains(&b))
}

/// The bytes written so far, as the two hex digits the hex dump shows for each. Before the first, the scene says so in a
/// plain box instead.
fn output(tab: &Tab) -> Result<Vec<String>, String> {
    if texts(tab)?.iter().any(|t| t == "Output bytes: none") {
        return Ok(Vec::new());
    }
    // A line of the dump holds its characters between `|` marks. The marks over a selected byte are texts too, but they
    // come after the lines.
    let lines = after(tab, "Output bytes", |t| t.contains('|'))?;
    Ok(lines
        .iter()
        .flat_map(|line| line.split('|').next().unwrap_or("").split_whitespace().map(str::to_owned))
        .collect())
}

/// What the accessible name of the node called `name` says is highlighted, such as `byte 0`, or `None` if nothing is.
fn highlight(tab: &Tab, name: &str) -> Result<Option<String>, String> {
    let label = text(
        tab,
        &format!(
            "([...document.querySelectorAll('{DIAGRAM} g[aria-label]')] \
             .find(g => g.getAttribute('aria-label').startsWith('{name}'))?.getAttribute('aria-label') ?? '')"
        ),
    )?;
    Ok(label.split_once("also highlighted: ").map(|(_, cell)| cell.to_owned()))
}

/// The text of each visible mark the hex dump draws over its secondary bytes: the two hex digits, or the character. The
/// mark's block has the secondary selection colour.
fn secondary_marks(tab: &Tab) -> Result<Vec<String>, String> {
    let json = text(
        tab,
        &format!(
            "JSON.stringify([...document.querySelectorAll('{DIAGRAM} g[aria-label^=\"Output bytes\"] rect')] \
             .filter(r => r.getAttribute('visibility') === 'visible' && r.getAttribute('fill') === '#7fd8be') \
             .map(r => r.nextElementSibling.textContent))"
        ),
    )?;
    serde_json::from_str(&json).map_err(|e| format!("could not read the marks: {e}"))
}

/// Whether the scene's title names pass two.
fn in_pass_two(tab: &Tab) -> Result<bool, String> {
    let all = texts(tab)?;
    Ok(all.iter().any(|t| t.contains("Phase 2")))
}

/// How many text input nodes the scene has.
fn text_inputs(tab: &Tab) -> Result<usize, String> {
    text(tab, &format!("String(document.querySelectorAll('{DIAGRAM} input').length)"))?
        .parse()
        .map_err(|e| format!("{e}"))
}

/// The first position of pass two.
const PASS_TWO: usize = LAST + 1;

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Pass two opens with its own title, and the text box becomes an array of the input's bytes with the first marked.
#[test]
fn pass_two_replaces_the_text_box_with_the_input_bytes_and_marks_the_first() -> Result<(), String> {
    let tab = open()?;
    goto(&tab, LAST)?;
    check(!in_pass_two(&tab)?, "pass one has the pass one title")?;
    check_eq(text_inputs(&tab)?, 1)?;

    click(&tab, "Next selection")?;
    check_eq(position(&tab)?, Some(PASS_TWO))?;
    check(in_pass_two(&tab)?, "pass two has the pass two title")?;
    check_eq(text_inputs(&tab)?, 0)?;
    check(
        explanation(&tab)?.starts_with("Byte 1 of 11: 'a' has the code 0"),
        &format!("unexpected explanation {:?}", explanation(&tab)?),
    )?;
    check_eq(output(&tab)?, Vec::<String>::new())?;
    // The table is complete, and its row for 'a' is the one ringed.
    check_eq(table(&tab)?.remove(0), row("a", "5", "0", "1", "0"))
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Stepping across the boundary, in both directions, visits every position once, and the scene follows.
#[test]
fn stepping_across_the_boundary_skips_and_repeats_no_position() -> Result<(), String> {
    let tab = open()?;
    goto(&tab, LAST - 1)?;
    for expected in LAST..=PASS_TWO + 1 {
        click(&tab, "Next selection")?;
        check_eq(position(&tab)?, Some(expected))?;
        check_eq(in_pass_two(&tab)?, expected >= PASS_TWO)?;
    }
    for expected in (LAST - 1..=PASS_TWO).rev() {
        click(&tab, "Previous selection")?;
        check_eq(position(&tab)?, Some(expected))?;
        check_eq(in_pass_two(&tab)?, expected >= PASS_TWO)?;
        check_eq(text_inputs(&tab)?, usize::from(expected < PASS_TWO))?;
    }
    Ok(())
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// A step writes a byte only when eight bits are ready, and marks exactly the bytes it wrote.
#[test]
fn a_step_marks_exactly_the_bytes_it_wrote() -> Result<(), String> {
    let tab = open()?;
    // The first three input bytes leave 7 bits pending, and the fourth completes 0100 1110, which is 4e.
    goto(&tab, PASS_TWO + 2)?;
    check_eq(output(&tab)?, Vec::<String>::new())?;
    goto(&tab, PASS_TWO + 3)?;
    check_eq(output(&tab)?, vec!["4e".to_owned()])?;
    check_eq(highlight(&tab, "Output bytes")?, Some("byte 0".to_owned()))?;
    // The byte is marked in the hex dump, over its two hex digits and over its character.
    check_eq(secondary_marks(&tab)?, ["4e", "N"].map(str::to_owned).to_vec())?;
    // The next step writes nothing, so the byte is no longer marked.
    goto(&tab, PASS_TWO + 4)?;
    check_eq(output(&tab)?, vec!["4e".to_owned()])?;
    check_eq(highlight(&tab, "Output bytes")?, None)?;
    check_eq(secondary_marks(&tab)?, Vec::<String>::new())
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The flush pads the last byte with zeros, shows the result in hexadecimal and compares its size with the input's.
#[test]
fn the_flush_pads_the_last_byte_and_shows_the_result_and_its_size() -> Result<(), String> {
    let tab = open()?;
    goto(&tab, FLUSH)?;
    check(
        explanation(&tab)?.starts_with("Flush: 7 pending bits cannot fill a byte, so 1 zero bit pad it out: 10011100."),
        &format!("unexpected explanation {:?}", explanation(&tab)?),
    )?;
    check_eq(output(&tab)?, ["4e", "ac", "9c"].map(str::to_owned).to_vec())?;
    check_eq(
        after(&tab, "Result in hexadecimal", is_hex_byte)?,
        ["4E", "AC", "9C"].map(str::to_owned).to_vec(),
    )?;

    let summary = markdown(&tab, 2)?;
    check(summary.contains("Original: 11 bytes (88 bits)"), &summary)?;
    check(
        summary.contains("Encoded: 3 bytes, which is 23 bits of code and 1 of padding"),
        &summary,
    )?;
    check(summary.contains("3.67 to 1"), &summary)?;
    check(summary.contains("27.3%"), &summary)?;

    // Next does nothing at the end: the walk clamps.
    button(&tab, "Next selection")?.click().map_err(|e| format!("{e}"))?;
    std::thread::sleep(Duration::from_millis(300));
    check_eq(position(&tab)?, Some(FLUSH))
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// A text whose codes fill whole bytes needs no padding, and the flush writes no byte of its own.
#[test]
fn a_byte_aligned_output_needs_no_padding() -> Result<(), String> {
    let tab = open()?;
    set_text(&tab, "aaaaaaaa")?;
    // 3 positions of pass one, 8 of packing, and the flush.
    goto(&tab, 11)?;
    check(
        explanation(&tab)?.contains("no padding is needed"),
        &format!("unexpected explanation {:?}", explanation(&tab)?),
    )?;
    check_eq(output(&tab)?, vec!["00".to_owned()])?;
    check_eq(after(&tab, "Result in hexadecimal", is_hex_byte)?, vec!["00".to_owned()])?;
    check(
        markdown(&tab, 2)?.contains("which is 8 bits of code and 0 of padding"),
        "the summary counts no padding",
    )
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Every byte of a multi-byte character is its own cell of the input array.
#[test]
fn the_input_array_has_a_cell_for_each_byte_not_each_character() -> Result<(), String> {
    let tab = open()?;
    set_text(&tab, "é")?;
    // 5 positions of pass one: counting, 1 merge, the lengths and 2 codes.
    goto(&tab, 5)?;
    check_eq(after(&tab, "Input bytes", |t| t == "·")?, vec!["·".to_owned(), "·".to_owned()])?;
    // The hexadecimal array tells the two bytes apart.
    check_eq(
        after(&tab, "Input bytes in hexadecimal", is_hex_byte)?,
        vec!["C3".to_owned(), "A9".to_owned()],
    )?;
    check(
        explanation(&tab)?.starts_with("Byte 1 of 2: '0xC3'"),
        &format!("unexpected explanation {:?}", explanation(&tab)?),
    )
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// A one-byte text still shows its whole summary, inside the diagram.
#[test]
fn a_short_text_still_shows_its_summary_inside_the_diagram() -> Result<(), String> {
    let tab = open()?;
    set_text(&tab, "a")?;
    // 3 positions of pass one, 1 of packing, and the flush.
    goto(&tab, 4)?;
    let summary = markdown(&tab, 2)?;
    check(summary.contains("Original: 1 byte (8 bits)"), &summary)?;
    let inside = text(
        &tab,
        &format!(
            "(() => {{ const d = document.querySelector('{DIAGRAM}').getBoundingClientRect(); \
             const t = [...document.querySelectorAll('{DIAGRAM} text')].find(t => t.textContent.startsWith('Compression')); \
             if (!t) return 'missing'; const r = t.getBoundingClientRect(); \
             return r.bottom <= d.bottom && r.right <= d.right ? 'inside' : 'outside'; }})()"
        ),
    )?;
    check_eq(inside.as_str(), "inside")
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The keyboard focus stays on the "Next selection" button as the walk crosses from pass one into pass two.
#[test]
fn keyboard_focus_survives_the_change_of_pass() -> Result<(), String> {
    let tab = open()?;
    goto(&tab, LAST)?;
    button(&tab, "Next selection")?.focus().map_err(|e| format!("{e}"))?;
    let from = position(&tab)?;
    tab.press_key("Enter").map_err(|e| format!("{e}"))?;
    wait_for_move(&tab, from)?;
    check_eq(position(&tab)?, Some(PASS_TWO))?;
    check_eq(
        text(&tab, "document.activeElement?.getAttribute('aria-label') ?? ''")?,
        "Next selection".to_owned(),
    )
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// "Next 8" and "Previous 8" move eight positions, across the change of pass, and stop at the ends.
#[test]
fn the_eight_step_buttons_move_eight_positions_across_both_passes() -> Result<(), String> {
    let tab = open()?;
    click(&tab, "Next 8")?;
    check_eq(position(&tab)?, Some(8))?;
    click(&tab, "Next 8")?;
    check_eq(position(&tab)?, Some(16))?;
    check(in_pass_two(&tab)?, "position 16 is in pass two")?;
    click(&tab, "Previous 8")?;
    check_eq(position(&tab)?, Some(8))?;
    check(!in_pass_two(&tab)?, "position 8 is in pass one")
}
