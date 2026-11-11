//! The UTF-8 panel in the built demo gallery: the four buttons that choose a character, the rows of bits that grow and
//! shrink with the character, the groups of code point bits lined up over the bytes they fill, the template bits
//! highlighted in the bits of the bytes, and the keyboard focus staying on a button across the redraw that every choice
//! makes.
//!
//! The encoding itself is covered by native unit tests against the standard library's own encoder for every character.
//! What they cannot reach is the browser: that a real choice redraws the scene with the right nodes, and what the
//! redraw keeps.
//!
//! Each length of character is a test of its own, so the CI deals them out to different shards.

use crate::common::{demo_tab::DemoTab, new_demo_tab};
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

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The name of the node holding the bits of byte `byte` of a sequence of `len`, whose name starts with `base`.
fn byte_node(base: &str, len: usize, byte: usize) -> String {
    if len == 1 { base.to_owned() } else { format!("{base} (byte {})", byte + 1) }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Choosing the character at `index` encodes it as `expected`, with a node of bits for each byte, and the bits of the
/// template are highlighted in each: in the first byte the leading ones and the zero after them, and in every other
/// byte the `10` that starts it. `highlighted` is what each byte's node says.
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

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The left and right edges of the nodes of one row, boxes included, as the page draws them, from left to right. A row
/// is every node whose accessible name starts with `prefix`.
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

/// Where a person sees the middle of a row: the middle of the node in the middle if there is an odd number of nodes,
/// and the middle of the gap between the two in the middle if there is an even number.
fn middle_of(edges: &[(f64, f64)]) -> f64 {
    let half = edges.len() / 2;
    if edges.len() % 2 == 1 {
        (edges[half].0 + edges[half].1) / 2.0
    } else {
        (edges[half - 1].1 + edges[half].0) / 2.0
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The four rows of nodes share a midline. With an odd number of nodes in a row, the one in the middle is on it, and
/// with an even number the gap between the two in the middle is. The code point and the bytes in hexadecimal are on it
/// too, so they are centred above and below the rows of bits. No two nodes of a row overlap, and `len` nodes are in
/// each row of bits.
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
        // Each group is centred over its byte, so the groups of two bytes, 5 and 6 bits wide, have the gap between them
        // a few pixels off the midline. Every other row has its middle exactly on it.
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

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The buttons, the note under them and the code point share a midline, whichever character is chosen. The note is as
/// wide as the buttons together.
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

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// How far the nodes of the diagram reach outside the diagram's own box, in pixels: the part of the leftmost node that
/// is left of it, and of the rightmost that is right of it. Anything above zero is clipped, so part of a node is cut
/// off.
fn clipped(index: usize) -> Result<(f64, f64), String> {
    let tab = open()?;
    press(&tab, &format!("choose-{index}"))?;
    let json = text(
        &tab,
        &format!(
            "(() => {{ const svg = document.querySelector('{DIAGRAM}').getBoundingClientRect(); \
             const boxes = [...document.querySelectorAll('{DIAGRAM} g[aria-label]')] \
             .map(g => g.getBoundingClientRect()); \
             return JSON.stringify([Math.max(0, svg.left - Math.min(...boxes.map(b => b.left))), \
             Math.max(0, Math.max(...boxes.map(b => b.right)) - svg.right)]); }})()"
        ),
    )?;
    serde_json::from_str(&json).map_err(|e| format!("could not read how far the nodes reach: {e}"))
}

fn fits_inside_the_diagram(index: usize) -> Result<(), String> {
    let (left, right) = clipped(index)?;
    check(
        left <= 0.5 && right <= 0.5,
        &format!("the nodes reach {left}px past the left edge and {right}px past the right"),
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

mod unit_tests;
