//! The Markdown node and the text input node, in the built demo gallery's "Markdown node" and "Text input node" panels.
//!
//! The Markdown parser and its layout are covered by native unit tests. What they cannot reach is the browser: that the
//! lines are drawn inside the box, that bold, italic and code runs are drawn as such, that real typing reaches the text
//! input node's callback, and that typing in it, or a drag of the node, does not disturb the rest of the scene.

use crate::common::{DemoTab, drag, new_demo_tab};
use headless_chrome::Tab;
use std::time::Duration;

const MARKDOWN_DIAGRAM: &str = "#markdown-diagram";
const DIAGRAM: &str = "#text-input-diagram";

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The Markdown panel, once its node is drawn.
fn open_markdown() -> Result<DemoTab, String> {
    new_demo_tab(
        "index.html#panel-markdown",
        &format!("{MARKDOWN_DIAGRAM} g[aria-label^=\"Text\"]"),
        (1400, 1400),
    )
}

/// The text input panel, once its text box is drawn.
fn open() -> Result<DemoTab, String> {
    new_demo_tab("index.html#panel-text-input", &format!("{DIAGRAM} input"), (1400, 1400))
}

fn check(condition: bool, message: &str) -> Result<(), String> {
    if condition { Ok(()) } else { Err(message.to_owned()) }
}

fn check_eq<T: PartialEq + std::fmt::Debug>(got: T, expected: T) -> Result<(), String> {
    check(got == expected, &format!("expected {expected:?}, got {got:?}"))
}

fn text(tab: &Tab, expression: &str) -> Result<String, String> {
    tab.evaluate(expression, false)
        .map_err(|e| format!("could not evaluate {expression}: {e}"))?
        .value
        .and_then(|v| v.as_str().map(str::to_owned))
        .ok_or_else(|| format!("{expression} did not return a string"))
}

/// The scene's content layer `transform`, which is its zoom and pan, or an empty string if it has none.
fn view(tab: &Tab) -> Result<String, String> {
    text(
        tab,
        &format!("(document.querySelector('{DIAGRAM} > g.svg-dom-graph-content')?.getAttribute('transform') ?? '')"),
    )
}

fn echo(tab: &Tab) -> Result<String, String> {
    text(tab, "document.getElementById('text-input-echo').textContent")
}

/// The Markdown node's `<g>`: the first node group, which is the one holding the headings.
const MARKDOWN_GROUP: &str = "g[aria-label^=\"Text\"]";

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Every line is drawn inside the box, whatever its length, and the box is as tall as its text.
#[test]
fn every_line_of_the_markdown_is_drawn_inside_its_box() -> Result<(), String> {
    let tab = open_markdown()?;
    let json = text(
        &tab,
        &format!(
            "(() => {{ const g = document.querySelector('{MARKDOWN_DIAGRAM} {MARKDOWN_GROUP}'); \
                const box = g.querySelector('rect').getBBox(); \
                const lines = [...g.querySelectorAll('text')].map(t => {{ const b = t.getBBox(); \
                    return [t.textContent, b.x, b.y, b.x + b.width, b.y + b.height]; }}); \
                return JSON.stringify({{ box: [box.width, box.height], lines }}); }})()"
        ),
    )?;
    #[derive(serde::Deserialize)]
    struct Measured {
        r#box: (f64, f64),
        lines: Vec<(String, f64, f64, f64, f64)>,
    }
    let measured: Measured = serde_json::from_str(&json).map_err(|e| format!("{e}"))?;
    check(
        measured.lines.len() > 12,
        &format!("only {} lines were drawn", measured.lines.len()),
    )?;
    for (line, left, top, right, bottom) in &measured.lines {
        check(*left >= -0.5 && *top >= -0.5, &format!("{line:?} starts outside the box"))?;
        check(
            *right <= measured.r#box.0 + 0.5 && *bottom <= measured.r#box.1 + 0.5,
            &format!("{line:?} ends outside the {:?} box: {right}, {bottom}", measured.r#box),
        )?;
    }
    Ok(())
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// A text input node narrower than its label shrinks the label, so it stays inside the node's box.
#[test]
fn a_label_wider_than_its_text_input_node_shrinks_to_fit() -> Result<(), String> {
    let tab = open()?;
    let json = text(
        &tab,
        &format!(
            "(() => {{ const g = document.querySelector('{DIAGRAM} g[aria-label^=\"Long labels shrink!\"]'); \
                const box = g.querySelector('rect').getBBox(); const b = g.querySelector('text').getBBox(); \
                return JSON.stringify([b.x, b.x + b.width, box.width]); }})()"
        ),
    )?;
    let (left, right, width): (f64, f64, f64) = serde_json::from_str(&json).map_err(|e| format!("{e}"))?;
    check(
        left >= -0.5 && right <= width + 0.5,
        &format!("the label spans {left}..{right} in a box {width} wide"),
    )
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Bold, italic and code runs are drawn as tspans with the matching weight, slant and colour, and headings are larger.
#[test]
fn bold_italic_and_code_are_drawn_as_such_and_headings_are_larger() -> Result<(), String> {
    let tab = open_markdown()?;
    let json = text(
        &tab,
        &format!(
            "(() => {{ const g = document.querySelector('{MARKDOWN_DIAGRAM} {MARKDOWN_GROUP}'); \
                const run = (s) => [...g.querySelectorAll(s)].map(t => t.textContent); \
                const sizes = [...g.querySelectorAll('text')].map(t => [t.textContent.slice(0, 12), t.getAttribute('font-size')]); \
                return JSON.stringify({{ bold: run('tspan[font-weight=bold]'), italic: run('tspan[font-style=italic]'), \
                    code: run('tspan[fill=\"#a4380b\"]'), sizes }}); }})()"
        ),
    )?;
    #[derive(serde::Deserialize)]
    struct Runs {
        bold: Vec<String>,
        italic: Vec<String>,
        code: Vec<String>,
        sizes: Vec<(String, String)>,
    }
    let runs: Runs = serde_json::from_str(&json).map_err(|e| format!("{e}"))?;
    check(
        runs.bold.iter().any(|t| t == "bold"),
        &format!("no bold run in {:?}", runs.bold),
    )?;
    check(
        runs.italic.iter().any(|t| t == "italic"),
        &format!("no italic run in {:?}", runs.italic),
    )?;
    // `***both***` is bold and italic.
    check(
        runs.bold.iter().any(|t| t == "both") && runs.italic.iter().any(|t| t == "both"),
        "no bold italic run",
    )?;
    check(
        runs.code.iter().any(|t| t == "inline code"),
        &format!("no code run in {:?}", runs.code),
    )?;

    let size_of = |starts: &str| {
        runs.sizes
            .iter()
            .find(|(t, _)| t.starts_with(starts))
            .and_then(|(_, s)| s.parse::<f64>().ok())
    };
    let title = size_of("A Markdown").ok_or("no title")?;
    let section = size_of("Lists").ok_or("no section heading")?;
    let subsection = size_of("Escapes").ok_or("no subsection heading")?;
    check(
        title > section && section > subsection && subsection > 14.0,
        &format!("{title} {section} {subsection}"),
    )
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// An escaped marker is literal, and a word with no spaces that is longer than a line is split across lines.
#[test]
fn escapes_are_literal_and_a_too_long_word_is_split() -> Result<(), String> {
    let tab = open_markdown()?;
    let json = text(
        &tab,
        &format!(
            "JSON.stringify([...document.querySelectorAll('{MARKDOWN_DIAGRAM} {MARKDOWN_GROUP} text')].map(t => t.textContent))"
        ),
    )?;
    let lines: Vec<String> = serde_json::from_str(&json).map_err(|e| format!("{e}"))?;
    // A line break can fall inside the code, so read the lines as one paragraph.
    let all = lines.join(" ");
    check(
        all.contains("*not italic*") && all.contains("`not code`"),
        &format!("{lines:?}"),
    )?;
    // The long word is in the source whole, and no drawn line holds all of it. The pieces of it, read in order, are the
    // word again. A piece may end in the comma that follows the word, which is not part of it.
    let word = "0123456789".repeat(10);
    check(!lines.iter().any(|l| l.contains(&word)), "the long word was not split")?;
    let pieces: String = lines
        .iter()
        .flat_map(|l| l.split(' '))
        .filter(|w| w.len() > 5 && w.starts_with(|c: char| c.is_ascii_digit()))
        .map(|w| w.trim_end_matches(','))
        .collect();
    check_eq(pieces, word)
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Typing in the text box, with a real keyboard, reaches the callback, and does not pan or zoom the diagram even for the
/// keys that would, such as `+` and the arrow keys.
#[test]
fn real_typing_reaches_the_callback_and_never_pans_or_zooms() -> Result<(), String> {
    let tab = open()?;
    check_eq(echo(&tab)?, "Edit this text".to_owned())?;
    let before = view(&tab)?;

    let input = tab
        .find_element(&format!("{DIAGRAM} input"))
        .map_err(|e| format!("no text box: {e}"))?;
    input.click().map_err(|e| format!("{e}"))?;
    // End, then keys the scene's own keyboard handling would act on if they reached it.
    tab.press_key("End").map_err(|e| format!("{e}"))?;
    tab.type_str("+-=0 ok").map_err(|e| format!("{e}"))?;
    tab.press_key("ArrowLeft").map_err(|e| format!("{e}"))?;
    std::thread::sleep(Duration::from_millis(200));

    check_eq(echo(&tab)?, "Edit this text+-=0 ok".to_owned())?;
    check_eq(view(&tab)?, before)?;
    check_eq(text(&tab, "document.activeElement.tagName.toLowerCase()")?, "input".to_owned())
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The text box is named for assistive technology by the node's label, and its node is a labelled group.
#[test]
fn the_text_input_node_is_named_for_assistive_technology() -> Result<(), String> {
    let tab = open()?;
    check_eq(
        text(
            &tab,
            &format!("document.querySelector('{DIAGRAM} input').getAttribute('aria-label')"),
        )?,
        "Text to echo".to_owned(),
    )?;
    check_eq(
        text(
            &tab,
            &format!("document.querySelector('{DIAGRAM} g[aria-label^=\"Text to echo.\"]').getAttribute('role')"),
        )?,
        "group".to_owned(),
    )
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The Markdown node is a labelled group.
#[test]
fn the_markdown_node_is_a_labelled_group() -> Result<(), String> {
    let tab = open_markdown()?;
    check_eq(
        text(
            &tab,
            &format!("document.querySelector('{MARKDOWN_DIAGRAM} {MARKDOWN_GROUP}').getAttribute('role')"),
        )?,
        "group".to_owned(),
    )
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// A drag of the Markdown node moves it.
#[test]
fn the_markdown_node_drags() -> Result<(), String> {
    let tab = open_markdown()?;
    let transform = |tab: &Tab| {
        text(
            tab,
            &format!("document.querySelector('{MARKDOWN_DIAGRAM} {MARKDOWN_GROUP}').getAttribute('transform')"),
        )
    };
    let before = transform(&tab)?;
    // The heading is near the top left of the node.
    let at = text(
        &tab,
        &format!(
            "(() => {{ const r = document.querySelector('{MARKDOWN_DIAGRAM} {MARKDOWN_GROUP} rect').getBoundingClientRect(); \
                return (r.left + 30) + ',' + (r.top + 20); }})()"
        ),
    )?;
    let (x, y) = at.split_once(',').ok_or("no point")?;
    let (x, y): (f64, f64) = (x.parse().map_err(|_| "x")?, y.parse().map_err(|_| "y")?);
    drag(&tab, &[(x, y), (x + 20.0, y + 10.0), (x + 40.0, y + 25.0)])?;
    check(transform(&tab)? != before, "the node did not move")
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The text input node's `<g>`, found by its label.
const INPUT_GROUP: &str = "g[aria-label^=\"Text to echo\"]";

/// The `(left, top, right, bottom)` of the first element matching `selector`, in page pixels.
fn bounds(tab: &Tab, selector: &str) -> Result<(f64, f64, f64, f64), String> {
    let json = text(
        tab,
        &format!(
            "(() => {{ const r = document.querySelector('{selector}').getBoundingClientRect(); \
                return JSON.stringify([r.left, r.top, r.right, r.bottom]); }})()"
        ),
    )?;
    serde_json::from_str(&json).map_err(|e| format!("{e}"))
}

fn input_group_transform(tab: &Tab) -> Result<String, String> {
    text(
        tab,
        &format!("document.querySelector('{DIAGRAM} {INPUT_GROUP}').getAttribute('transform')"),
    )
}

fn caret(tab: &Tab) -> Result<(u32, u32), String> {
    let json = text(
        tab,
        &format!(
            "(() => {{ const i = document.querySelector('{DIAGRAM} input'); \
                return JSON.stringify([i.selectionStart, i.selectionEnd]); }})()"
        ),
    )?;
    serde_json::from_str(&json).map_err(|e| format!("{e}"))
}

fn calls(tab: &Tab) -> Result<String, String> {
    text(tab, "document.getElementById('text-input-calls').textContent")
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Replacing the text from code puts it in the text box, text outside the Basic Multilingual Plane included, and does
/// not call the callback, which then still hears of the next real keystroke.
#[test]
fn replacing_the_text_from_code_does_not_call_the_callback() -> Result<(), String> {
    let tab = open()?;
    check_eq(calls(&tab)?, "0".to_owned())?;
    tab.find_element("#text-input-replace")
        .map_err(|e| format!("no button: {e}"))?
        .click()
        .map_err(|e| format!("{e}"))?;
    std::thread::sleep(Duration::from_millis(200));
    check_eq(
        text(&tab, &format!("document.querySelector('{DIAGRAM} input').value"))?,
        "Replaced \u{2603} \u{1F600}!".to_owned(),
    )?;
    check_eq(calls(&tab)?, "0".to_owned())?;
    check_eq(echo(&tab)?, "Edit this text".to_owned())?;

    tab.find_element(&format!("{DIAGRAM} input"))
        .map_err(|e| format!("no text box: {e}"))?
        .click()
        .map_err(|e| format!("{e}"))?;
    tab.press_key("End").map_err(|e| format!("{e}"))?;
    tab.type_str("?").map_err(|e| format!("{e}"))?;
    std::thread::sleep(Duration::from_millis(200));
    check_eq(calls(&tab)?, "1".to_owned())?;
    check_eq(echo(&tab)?, "Replaced \u{2603} \u{1F600}!?".to_owned())
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// A pointer drag that starts on the node's label row moves the node. One that starts inside the text box selects text
/// and leaves the node where it is, and a click inside the text box places the caret there.
#[test]
fn a_drag_outside_the_text_box_moves_the_node_and_inside_it_selects_text() -> Result<(), String> {
    let tab = open()?;
    let start = input_group_transform(&tab)?;
    let (left, top, _, _) = bounds(&tab, &format!("{DIAGRAM} {INPUT_GROUP} rect"))?;

    // The label row is the top of the node, above the text box.
    drag(
        &tab,
        &[(left + 20.0, top + 4.0), (left + 30.0, top + 14.0), (left + 50.0, top + 30.0)],
    )?;
    let moved = input_group_transform(&tab)?;
    check(moved != start, "a drag from the label row did not move the node")?;

    // Inside the text box, a drag along the text selects it.
    let (l, t, _, b) = bounds(&tab, &format!("{DIAGRAM} input"))?;
    let y = (t + b) / 2.0;
    drag(&tab, &[(l + 10.0, y), (l + 40.0, y), (l + 80.0, y)])?;
    std::thread::sleep(Duration::from_millis(150));
    check_eq(input_group_transform(&tab)?, moved.clone())?;
    let (from, to) = caret(&tab)?;
    check(from < to, &format!("no text was selected: {from}..{to}"))?;

    // A click puts the caret between two characters, and does not move the node.
    tab.find_element(&format!("{DIAGRAM} input"))
        .map_err(|e| format!("no text box: {e}"))?
        .click()
        .map_err(|e| format!("{e}"))?;
    let (from, to) = caret(&tab)?;
    check(from == to, &format!("a click left a selection: {from}..{to}"))?;
    check_eq(input_group_transform(&tab)?, moved)
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The size `measure_markdown_node` reports is the size the node is then drawn at, and is the size of its drawn box.
#[test]
fn the_measured_size_of_the_markdown_node_is_the_size_it_is_drawn_at() -> Result<(), String> {
    let tab = open_markdown()?;
    let report = text(&tab, "document.getElementById('markdown-measure').textContent")?;
    let (measured, drawn) = report
        .strip_prefix("measured ")
        .and_then(|r| r.split_once(", drawn "))
        .ok_or_else(|| format!("unexpected report {report:?}"))?;
    check_eq(measured, drawn)?;

    let (w, h) = measured.split_once('x').ok_or("no size")?;
    let (w, h): (f64, f64) = (w.parse().map_err(|_| "width")?, h.parse().map_err(|_| "height")?);
    let json = text(
        &tab,
        &format!(
            "(() => {{ const b = document.querySelector('{MARKDOWN_DIAGRAM} {MARKDOWN_GROUP} rect').getBBox(); \
                return JSON.stringify([b.width, b.height]); }})()"
        ),
    )?;
    let (box_w, box_h): (f64, f64) = serde_json::from_str(&json).map_err(|e| format!("{e}"))?;
    check(
        (box_w - w).abs() < 0.5 && (box_h - h).abs() < 0.5,
        &format!("drawn {box_w}x{box_h}, measured {w}x{h}"),
    )
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Assistive technology reads the Markdown node's text as it is, the markers of its lists included.
#[test]
fn the_markdown_nodes_text_is_there_for_assistive_technology_to_read() -> Result<(), String> {
    let tab = open_markdown()?;
    let content = text(
        &tab,
        &format!("document.querySelector('{MARKDOWN_DIAGRAM} {MARKDOWN_GROUP}').textContent"),
    )?;
    for expected in ["A Markdown node", "Lists", "Another bullet.", "1.", "2.", "\u{2022}", "Escapes"] {
        check(content.contains(expected), &format!("{expected:?} is missing from {content:?}"))?;
    }
    Ok(())
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The text input node, the ASCII node and the hexdump node are centred on one vertical midline.
#[test]
fn the_three_nodes_share_a_vertical_midline() -> Result<(), String> {
    let tab = open()?;
    let middle = |selector: &str| bounds(&tab, selector).map(|(left, _, right, _)| (left + right) / 2.0);
    let input = middle(&format!("{DIAGRAM} {INPUT_GROUP} rect"))?;
    let ascii = middle(&format!("{DIAGRAM} g[aria-label^=\"The text as ASCII\"] rect"))?;
    let hexdump = middle(&format!("{DIAGRAM} g[aria-label^=\"The text as a hexdump\"] rect"))?;
    check(
        (input - ascii).abs() <= 1.5 && (input - hexdump).abs() <= 1.5,
        &format!("the midlines are {input}, {ascii} and {hexdump}"),
    )
}
