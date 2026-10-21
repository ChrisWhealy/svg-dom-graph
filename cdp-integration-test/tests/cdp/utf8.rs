//! The UTF-8 panel in the built demo gallery: the four buttons that choose a character, the rows of bits that grow and
//! shrink with the character, the groups of code point bits lined up over the bytes they fill, the template bits
//! highlighted in the bits of the bytes, and the keyboard focus staying on
//! a button across the redraw that every choice makes.
//!
//! The encoding itself is covered by native unit tests against the standard library's own encoder for every character.
//! What they cannot reach is the browser: that a real choice redraws the scene with the right nodes, and what the redraw
//! keeps.
//!
//! Each length of character is a test of its own, so the CI deals them out to different shards.

use crate::common::{DemoTab, new_demo_tab};
use headless_chrome::Tab;
use std::time::Duration;

const DIAGRAM: &str = "#utf8-diagram";

fn open() -> Result<DemoTab, String> {
    new_demo_tab(
        "index.html#panel-utf8",
        &format!("{DIAGRAM} g[data-action=\"choose-0\"]"),
        (2400, 2400),
    )
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

/// Presses the button drawn in the scene with `action`.
fn press(tab: &Tab, action: &str) -> Result<(), String> {
    tab.evaluate(
        &format!(
            "document.querySelector('{DIAGRAM} g[data-action=\"{action}\"]') \
             .dispatchEvent(new MouseEvent('click', {{ bubbles: true }}))"
        ),
        false,
    )
    .map_err(|e| format!("could not press {action}: {e}"))?;
    std::thread::sleep(Duration::from_millis(300));
    Ok(())
}

/// The line under the diagram.
fn result(tab: &Tab) -> Result<String, String> {
    text(tab, "document.getElementById('utf8-result').textContent")
}

/// How many nodes of the diagram have an accessible name starting with `name`.
fn nodes_named(tab: &Tab, name: &str) -> Result<usize, String> {
    text(
        tab,
        &format!("String(document.querySelectorAll('{DIAGRAM} g[aria-label^=\"{name}\"]').length)"),
    )?
    .parse()
    .map_err(|e| format!("could not count {name}: {e}"))
}

/// The accessible name of the node that starts with `name`.
fn label(tab: &Tab, name: &str) -> Result<String, String> {
    text(
        tab,
        &format!("document.querySelector('{DIAGRAM} g[aria-label^=\"{name}\"]').getAttribute('aria-label')"),
    )
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The panel opens on the one-byte character, with a button for each of the four, the first pressed, and one column of
/// four nodes.
#[test]
fn the_panel_opens_on_the_one_byte_character_with_four_buttons() -> Result<(), String> {
    let tab = open()?;
    check_eq(result(&tab)?.as_str(), "A U+0041 is 41")?;
    for index in 0..4 {
        let pressed = text(
            &tab,
            &format!(
                "document.querySelector('{DIAGRAM} g[data-action=\"choose-{index}\"]').getAttribute('aria-pressed')"
            ),
        )?;
        check_eq(pressed.as_str(), if index == 0 { "true" } else { "false" })?;
    }
    // "Code point" starts the names of two nodes: the code point, and its bits.
    check_eq(nodes_named(&tab, "Code point")?, 2)?;
    for name in ["Code point bits", "UTF-8 bits", "UTF-8 bytes"] {
        check_eq(nodes_named(&tab, name)?, 1)?;
    }
    Ok(())
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The name of the node holding the bits of byte `byte` of a sequence of `len`, whose name starts with `base`.
fn byte_node(base: &str, len: usize, byte: usize) -> String {
    if len == 1 { base.to_owned() } else { format!("{base} (byte {})", byte + 1) }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Choosing the character at `index` encodes it as `expected`, with a node of bits for each byte, and the bits of the
/// template are highlighted in each: in the first byte the leading ones and the zero after them, and in every other byte
/// the `10` that starts it. `highlighted` is what each byte's node says.
fn encodes_to(index: usize, expected: &str, highlighted: &[&str]) -> Result<(), String> {
    let tab = open()?;
    press(&tab, &format!("choose-{index}"))?;
    check_eq(result(&tab)?.ends_with(expected), true)?;
    check_eq(nodes_named(&tab, "UTF-8 bytes")?, 1)?;
    check_eq(nodes_named(&tab, "UTF-8 bits")?, highlighted.len())?;
    check_eq(
        nodes_named(&tab, if highlighted.len() == 1 { "Code point bits" } else { "Bits " })?,
        highlighted.len(),
    )?;
    for (byte, cells) in highlighted.iter().enumerate() {
        let name = label(&tab, &byte_node("UTF-8 bits", highlighted.len(), byte))?;
        check(
            name.contains(&format!("also highlighted: {cells}")),
            &format!("the template bits of byte {byte} are not highlighted: {name}"),
        )?;
    }
    Ok(())
}

#[test]
fn a_one_byte_character_has_one_template_bit() -> Result<(), String> {
    encodes_to(0, "41", &["cell 0"])
}

#[test]
fn a_two_byte_character_has_the_template_of_two_bytes() -> Result<(), String> {
    encodes_to(1, "C2 A3", &["cells 0 to 2", "cells 0, 1"])
}

#[test]
fn a_three_byte_character_has_the_template_of_three_bytes() -> Result<(), String> {
    encodes_to(2, "E2 82 AC", &["cells 0 to 3", "cells 0, 1", "cells 0, 1"])
}

#[test]
fn a_four_byte_character_has_the_template_of_four_bytes() -> Result<(), String> {
    encodes_to(3, "F0 9F 98 80", &["cells 0 to 4", "cells 0, 1", "cells 0, 1", "cells 0, 1"])
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The left and right edges of the nodes of one row, boxes included, as the page draws them, from left to right. A row is
/// every node whose accessible name starts with `prefix`.
fn row_edges(tab: &Tab, prefix: &str) -> Result<Vec<(f64, f64)>, String> {
    let json = text(
        tab,
        &format!(
            "JSON.stringify([...document.querySelectorAll('{DIAGRAM} g[aria-label^=\"{prefix}\"]')] \
             .map(g => {{ const r = g.getBoundingClientRect(); return [r.left, r.right]; }}) \
             .sort((a, b) => a[0] - b[0]))"
        ),
    )?;
    serde_json::from_str(&json).map_err(|e| format!("could not read the row {prefix}: {e}"))
}

/// Where a person sees the middle of a row: the middle of the node in the middle if there is an odd number of nodes, and
/// the middle of the gap between the two in the middle if there is an even number.
fn middle_of(edges: &[(f64, f64)]) -> f64 {
    let half = edges.len() / 2;
    if edges.len() % 2 == 1 {
        (edges[half].0 + edges[half].1) / 2.0
    } else {
        (edges[half - 1].1 + edges[half].0) / 2.0
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The four rows of nodes share a midline. With an odd number of nodes in a row, the one in the middle is on it, and with
/// an even number the gap between the two in the middle is. The code point and the bytes in hexadecimal are on it too, so
/// they are centred above and below the rows of bits. No two nodes of a row overlap, and `len` nodes are in each row of
/// bits.
fn rows_are_centred(index: usize, len: usize) -> Result<(), String> {
    let tab = open()?;
    press(&tab, &format!("choose-{index}"))?;
    let centre_of = |name: &str| -> Result<f64, String> {
        let edges = row_edges(&tab, name)?;
        check_eq(edges.len(), 1)?;
        Ok((edges[0].0 + edges[0].1) / 2.0)
    };
    let axis = centre_of("Code point:")?;
    check(
        (centre_of("UTF-8 bytes")? - axis).abs() < 1.0,
        "the bytes in hexadecimal are not on the axis",
    )?;
    let groups = row_edges(&tab, if len == 1 { "Code point bits" } else { "Bits " })?;
    let bytes = row_edges(&tab, "UTF-8 bits")?;
    check_eq(groups.len(), len)?;
    check_eq(bytes.len(), len)?;
    for (name, row) in [("groups", &groups), ("bytes", &bytes)] {
        for pair in row.windows(2) {
            check(pair[0].1 <= pair[1].0, &format!("two nodes of the {name} overlap: {pair:?}"))?;
        }
        // Each group is centred over its byte, so the groups of two bytes, 5 and 6 bits wide, have the gap between them a
        // few pixels off the midline. Every other row has its middle exactly on it.
        let allowed = if name == "groups" && len == 2 { 8.0 } else { 1.0 };
        let offset = middle_of(row) - axis;
        check(
            offset.abs() < allowed,
            &format!("the middle of the {name} is {offset}px from the midline"),
        )?;
    }
    // Each group is over the byte it fills, in the same column, however uneven the groups are.
    for (n, (group, byte)) in groups.iter().zip(&bytes).enumerate() {
        let (above, below) = ((group.0 + group.1) / 2.0, (byte.0 + byte.1) / 2.0);
        check(
            (above - below).abs() < 1.0,
            &format!("group {n} is centred on {above}, over a byte centred on {below}"),
        )?;
    }
    Ok(())
}

#[test]
fn the_four_nodes_of_a_one_byte_character_share_a_midline() -> Result<(), String> {
    rows_are_centred(0, 1)
}

#[test]
fn the_gaps_of_a_two_byte_character_are_on_the_midline() -> Result<(), String> {
    rows_are_centred(1, 2)
}

#[test]
fn the_middle_nodes_of_a_three_byte_character_share_a_midline() -> Result<(), String> {
    rows_are_centred(2, 3)
}

#[test]
fn the_gaps_of_a_four_byte_character_are_on_the_midline() -> Result<(), String> {
    rows_are_centred(3, 4)
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The buttons, the note under them and the code point share a midline, whichever character is chosen. The note is as wide
/// as the buttons together.
fn shares_a_midline(index: usize) -> Result<(), String> {
    let tab = open()?;
    press(&tab, &format!("choose-{index}"))?;
    let json = text(
        &tab,
        &format!(
            "(() => {{ const box = e => {{ const r = e.getBoundingClientRect(); return [r.left, r.right]; }}; \
             const buttons = [...document.querySelectorAll('{DIAGRAM} g[data-action]')].map(box); \
             const note = box(document.querySelector('{DIAGRAM} g[aria-label=\"Text\"]')); \
             const point = box(document.querySelector('{DIAGRAM} g[aria-label^=\"Code point:\"]')); \
             return JSON.stringify({{ buttons: [Math.min(...buttons.map(b => b[0])), Math.max(...buttons.map(b => b[1]))], \
             note, point }}); }})()"
        ),
    )?;
    let edges: serde_json::Value = serde_json::from_str(&json).map_err(|e| e.to_string())?;
    let middle = |key: &str| -> Result<f64, String> {
        let pair = edges[key].as_array().ok_or_else(|| format!("no {key}"))?;
        let (left, right) = (pair[0].as_f64().ok_or("bad left")?, pair[1].as_f64().ok_or("bad right")?);
        Ok((left + right) / 2.0)
    };
    let buttons = middle("buttons")?;
    check(
        (middle("note")? - buttons).abs() < 1.0,
        "the note is not centred under the buttons",
    )?;
    check(
        (middle("point")? - buttons).abs() < 1.0,
        "the code point is not centred under the buttons",
    )?;
    let widths = |key: &str| -> Result<f64, String> {
        let pair = edges[key].as_array().ok_or_else(|| format!("no {key}"))?;
        Ok(pair[1].as_f64().ok_or("bad right")? - pair[0].as_f64().ok_or("bad left")?)
    };
    check(
        (widths("note")? - widths("buttons")?).abs() < 1.0,
        "the note is not as wide as the buttons",
    )
}

#[test]
fn the_buttons_the_note_and_a_one_byte_graph_share_a_midline() -> Result<(), String> {
    shares_a_midline(0)
}

#[test]
fn the_buttons_the_note_and_a_four_byte_graph_share_a_midline() -> Result<(), String> {
    shares_a_midline(3)
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The leftmost node of the diagram is a margin from its left edge, for a character of any length, and not further in
/// because the axis was placed with a margin counted twice. The widest block is the row of four buttons, or the row of four
/// bytes, so for a short character the buttons and the note set the left edge.
fn left_margin(index: usize) -> Result<f64, String> {
    let tab = open()?;
    press(&tab, &format!("choose-{index}"))?;
    text(
        &tab,
        &format!(
            "(() => {{ const svg = document.querySelector('{DIAGRAM}').getBoundingClientRect().left; \
             return String(Math.min(...[...document.querySelectorAll('{DIAGRAM} g[aria-label]')] \
             .map(g => g.getBoundingClientRect().left)) - svg); }})()"
        ),
    )?
    .parse()
    .map_err(|e| format!("could not read the margin: {e}"))
}

#[test]
fn a_one_byte_character_has_one_margin_at_the_left() -> Result<(), String> {
    let margin = left_margin(0)?;
    check(
        (18.0..=30.0).contains(&margin),
        &format!("the left margin is {margin}px, not about 20"),
    )
}

#[test]
fn a_four_byte_character_has_one_margin_at_the_left() -> Result<(), String> {
    let margin = left_margin(3)?;
    check(
        (18.0..=30.0).contains(&margin),
        &format!("the left margin is {margin}px, not about 20"),
    )
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The scene's content layer `transform`, which is its zoom and pan, or an empty string if it has none.
fn view(tab: &Tab) -> Result<String, String> {
    text(
        tab,
        &format!("(document.querySelector('{DIAGRAM} > g.svg-dom-graph-content')?.getAttribute('transform') ?? '')"),
    )
}

/// The digits in the cells of every node whose accessible name starts with `prefix`, in the order the nodes were drawn,
/// one after the other. The first `<text>` of a node is its name, so it is left out.
fn digits_of(tab: &Tab, prefix: &str) -> Result<String, String> {
    text(
        tab,
        &format!(
            "[...document.querySelectorAll('{DIAGRAM} g[aria-label^=\"{prefix}\"]')] \
             .map(g => [...g.querySelectorAll('text')].slice(1).map(t => t.textContent).join('')).join('')"
        ),
    )
}

/// How many buttons say that they are pressed, and the action of the first that does.
fn pressed(tab: &Tab) -> Result<String, String> {
    text(
        tab,
        &format!(
            "(() => {{ const on = [...document.querySelectorAll('{DIAGRAM} g[data-action][aria-pressed=\"true\"]')]; \
             return on.length + ':' + (on[0]?.getAttribute('data-action') ?? 'none'); }})()"
        ),
    )
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Choosing characters again and again, in an order that goes up and down in length, draws the right scene each time,
/// and the zoom is kept. The scene is drawn from scratch for every choice, so a node, a button or a listener left behind
/// by one would show in the next. After each choice there is exactly one button pressed, the one chosen, the nodes are
/// those of the character, and their cells hold its bits: the bits of the code point in the groups, and every bit of the
/// bytes in the byte nodes. The expected bits are worked out here from the character, not read from the page.
#[test]
fn choosing_characters_again_and_again_draws_each_one_and_keeps_the_zoom() -> Result<(), String> {
    const CHARACTERS: [char; 4] = ['A', '£', '€', '😀'];
    let tab = open()?;
    tab.find_element(&format!("{DIAGRAM} g[aria-label=\"Zoom in\"]"))
        .map_err(|e| format!("could not find Zoom in: {e}"))?
        .click()
        .map_err(|e| format!("could not zoom in: {e}"))?;
    std::thread::sleep(Duration::from_millis(300));
    let zoomed = view(&tab)?;
    check(!zoomed.is_empty(), "zooming in should change the view")?;

    // From one byte to four, down to two, to one, and on, twice round.
    for index in [0, 3, 1, 0, 2, 3, 1, 2, 0, 3] {
        press(&tab, &format!("choose-{index}"))?;
        let c = CHARACTERS[index];
        let mut encoded = [0_u8; 4];
        let bytes = c.encode_utf8(&mut encoded).as_bytes();
        let len = bytes.len();
        let hex = bytes.iter().map(|b| format!("{b:02X}")).collect::<Vec<_>>().join(" ");

        check_eq(pressed(&tab)?, format!("1:choose-{index}"))?;
        check(result(&tab)?.ends_with(&hex), &format!("{c}: the result is not {hex}"))?;
        check_eq(nodes_named(&tab, "UTF-8 bits")?, len)?;
        check_eq(nodes_named(&tab, if len == 1 { "Code point bits" } else { "Bits " })?, len)?;
        check_eq(nodes_named(&tab, "UTF-8 bytes")?, 1)?;
        check_eq(nodes_named(&tab, "Code point:")?, 1)?;
        check_eq(view(&tab)?, zoomed.clone())?;

        // Every bit of the bytes, and the bits of the code point that go into them: 7, 11, 16 or 21.
        let bits: String = bytes.iter().map(|b| format!("{b:08b}")).collect();
        check_eq(digits_of(&tab, "UTF-8 bits")?, bits)?;
        let payload_len = [7, 11, 16, 21][len - 1];
        let payload = format!("{:0payload_len$b}", u32::from(c));
        check_eq(digits_of(&tab, if len == 1 { "Code point bits" } else { "Bits " })?, payload)?;
    }
    Ok(())
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// A button keeps the keyboard focus across the redraw that choosing it makes, and shows as the pressed one.
#[test]
fn a_button_keeps_the_keyboard_focus_across_the_redraw() -> Result<(), String> {
    let tab = open()?;
    tab.evaluate(
        &format!("document.querySelector('{DIAGRAM} g[data-action=\"choose-2\"]').focus()"),
        false,
    )
    .map_err(|e| format!("could not focus the button: {e}"))?;
    tab.press_key("Enter").map_err(|e| format!("could not press Enter: {e}"))?;
    std::thread::sleep(Duration::from_millis(300));
    check_eq(
        text(&tab, "document.activeElement?.getAttribute('data-action') ?? 'none'")?.as_str(),
        "choose-2",
    )?;
    check_eq(
        text(
            &tab,
            &format!("document.querySelector('{DIAGRAM} g[data-action=\"choose-2\"]').getAttribute('aria-pressed')"),
        )?
        .as_str(),
        "true",
    )
}
