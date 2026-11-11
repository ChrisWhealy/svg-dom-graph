//! The QR code demo in the window the gallery's panel opens: the graph of the stages, the matrix of 21 by 21 modules,
//! the 62 steps, the buttons that jump to a stage, the list of roles, and the text node. The steps are the input, its
//! bytes, the four parts of the bitstream, the data codewords, 19 of Reed-Solomon, the function modules, 26 of
//! placement, 8 masks and the final symbol.
//!
//! The encoder, the snapshots and the path data are covered by native unit tests, and the encoder is compared with two
//! independent ones. What they cannot reach is the browser: that the scene is built once and only written to after
//! that, that going back shows exactly what going forward showed, that choosing a role and stepping stay independent,
//! that the toolbar and the buttons work from the keyboard, that zooming leaves the matrix alone, that typing starts
//! the walk again, that text which cannot be encoded leaves the symbol on show alone, and that the live region and the
//! matrix as text follow each step.

use crate::common::{demo_tab::DemoTab, new_demo_tab};
use headless_chrome::Tab;
use std::time::Duration;

const DIAGRAM: &str = "#qr-code-diagram";
const LAYER: &str = "#qr-code-diagram > g.svg-dom-graph-content";

/// The number of steps, and what each stage's first step is.
const STEPS: usize = 62;
const RS: usize = 7;
const FUNCTIONS: usize = 26;
const PLACEMENT: usize = 27;
const MASKS: usize = 53;
const FINAL: usize = 61;

/// The text the window opens with, which is 17 characters long, and so as long as a text can be.
const DEFAULT_TEXT: &str = "This is a QR code";

fn open() -> Result<DemoTab, String> {
    new_demo_tab(
        "qr-code-window.html",
        &format!("{DIAGRAM} g[data-action=\"jump-0\"]"),
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

/// The attribute `name` of the first element that matches `selector`.
fn attribute(tab: &Tab, selector: &str, name: &str) -> Result<String, String> {
    text(
        tab,
        &format!("document.querySelector('{selector}').getAttribute('{name}') ?? ''"),
    )
}

/// The `d` of one of the matrix's paths, which is `base` for the three colours, `role` and `highlight`.
fn path(tab: &Tab, which: &str) -> Result<String, String> {
    attribute(tab, &format!("#qr-code-{which}"), "d")
}

/// The three colour paths together, which change whenever any module's colour does.
fn colours(tab: &Tab) -> Result<String, String> {
    Ok(["unassigned", "light", "dark"]
        .iter()
        .map(|which| path(tab, which))
        .collect::<Result<Vec<_>, _>>()?
        .join(" | "))
}

/// How many rectangles, outlines or modules a path draws. Every one of them starts with `M`.
fn parts(path: &str) -> usize {
    path.matches('M').count()
}

/// Clicks the element that matches `selector` with a real mouse click, and waits for the redraw.
fn click(tab: &Tab, selector: &str) -> Result<(), String> {
    tab.find_element(selector)
        .map_err(|e| format!("could not find {selector}: {e}"))?
        .click()
        .map_err(|e| format!("could not click {selector}: {e}"))?;
    std::thread::sleep(Duration::from_millis(120));
    Ok(())
}

/// Presses the button drawn in the scene with `action`.
fn press(tab: &Tab, action: &str) -> Result<(), String> {
    click(tab, &format!("{DIAGRAM} g[data-action=\"{action}\"]"))
}

/// Presses a button of the stepping toolbar by its accessible name: `Next selection`, `Previous selection` or
/// `Restart selection`.
fn toolbar(tab: &Tab, name: &str) -> Result<(), String> {
    click(tab, &format!("{DIAGRAM} g[aria-label=\"{name}\"]"))
}

/// Presses Next `count` times.
fn next(tab: &Tab, count: usize) -> Result<(), String> {
    for _ in 0..count {
        toolbar(tab, "Next selection")?;
    }
    Ok(())
}

/// Goes to snapshot `index` by the button that jumps to it, which must be the first of a stage.
fn jump(tab: &Tab, index: usize) -> Result<(), String> {
    press(tab, &format!("jump-{index}"))
}

/// Everything the matrix shows: its three colours, the outline of the role, the highlight and its accessible name.
fn shown(tab: &Tab) -> Result<Vec<String>, String> {
    let mut all = Vec::new();
    for which in ["unassigned", "light", "dark", "role", "highlight"] {
        all.push(path(tab, which)?);
    }
    all.push(attribute(tab, "#qr-code-matrix", "aria-label")?);
    all.push(said(tab)?);
    Ok(all)
}

/// How many modules are dark, light and uncoloured, read from the matrix's accessible name.
fn counts(tab: &Tab) -> Result<(usize, usize, usize), String> {
    let found = text(
        tab,
        "(() => { const m = document.getElementById('qr-code-matrix').getAttribute('aria-label') \
         .match(/(\\d+) are dark, (\\d+) are light and (\\d+) have no colour/); \
         return m ? m.slice(1).join(',') : ''; })()",
    )?;
    let numbers: Vec<usize> = found.split(',').filter_map(|n| n.parse().ok()).collect();
    match numbers[..] {
        [dark, light, none] => Ok((dark, light, none)),
        _ => Err(format!("the accessible name of the matrix has no counts: {found:?}")),
    }
}

/// Whether a button, in the scene or on the toolbar, is disabled, as assistive technology is told.
fn disabled(tab: &Tab, selector: &str) -> Result<bool, String> {
    Ok(attribute(tab, selector, "aria-disabled")? == "true")
}

/// Whether the toolbar button with this accessible name is disabled.
fn toolbar_disabled(tab: &Tab, name: &str) -> Result<bool, String> {
    disabled(tab, &format!("{DIAGRAM} g[aria-label=\"{name}\"]"))
}

/// Whether the button `action` is pressed.
fn pressed(tab: &Tab, action: &str) -> Result<bool, String> {
    Ok(attribute(tab, &format!("{DIAGRAM} g[data-action=\"{action}\"]"), "aria-pressed")? == "true")
}

/// What the live region last said.
fn said(tab: &Tab) -> Result<String, String> {
    text(tab, "document.getElementById('qr-code-announcement').textContent")
}

/// The accessible name of the node whose name starts with `name`, which says which of its cells are not yet computed.
fn node(tab: &Tab, name: &str) -> Result<String, String> {
    attribute(tab, &format!("{DIAGRAM} g[aria-label^=\"{name}\"]"), "aria-label")
}

/// All the text in the scene, a block at a time, with the lines of each block joined by spaces and the unused lines of
/// a block left off.
fn scene_text(tab: &Tab) -> Result<String, String> {
    text(
        tab,
        &format!(
            "[...document.querySelectorAll('{LAYER} text')] \
             .map(t => [...t.querySelectorAll('tspan')].map(s => s.textContent).join(' ').trim()).join(' | ')"
        ),
    )
}

/// Replaces the text in the text node with `value`, as typing would, and waits for the scene to take it in.
fn type_text(tab: &Tab, value: &str) -> Result<(), String> {
    tab.evaluate(
        &format!(
            "(() => {{ const i = document.querySelector('{DIAGRAM} input'); i.focus(); i.value = {value:?}; \
             i.dispatchEvent(new Event('input', {{ bubbles: true }})); }})()"
        ),
        false,
    )
    .map_err(|e| format!("could not edit the text box: {e}"))?;
    std::thread::sleep(Duration::from_millis(300));
    Ok(())
}

/// What the text node holds now.
fn input_value(tab: &Tab) -> Result<String, String> {
    text(tab, &format!("document.querySelector('{DIAGRAM} input').value"))
}

/// Where the caret is in the text node, as the start and the end of the selection.
fn input_caret(tab: &Tab) -> Result<(u32, u32), String> {
    let found = text(
        tab,
        &format!(
            "(() => {{ const i = document.querySelector('{DIAGRAM} input'); \
             return i.selectionStart + ',' + i.selectionEnd; }})()"
        ),
    )?;
    let (start, end) = found.split_once(',').ok_or_else(|| format!("no caret in {found:?}"))?;
    Ok((
        start.parse().map_err(|e| format!("{e}"))?,
        end.parse().map_err(|e| format!("{e}"))?,
    ))
}

/// The error banner: its text, or an empty string while it is hidden.
fn banner(tab: &Tab) -> Result<String, String> {
    text(
        tab,
        "(() => { const b = document.getElementById('qr-code-error'); return b.hidden ? '' : b.textContent; })()",
    )
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
// Exporting the finished symbol
// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -

/// Makes the page keep the file an export would save, and the name it would save it as, and not save it. A real save
/// would put a file in the folder of whoever runs the tests.
fn catch_exports(tab: &Tab) -> Result<(), String> {
    tab.evaluate(
        "(() => { window.__saved = []; window.__blob = null; \
         const make = URL.createObjectURL.bind(URL); \
         URL.createObjectURL = (blob) => { window.__blob = blob; return make(blob); }; \
         HTMLAnchorElement.prototype.click = function () { window.__saved.push({ name: this.download }); }; })()",
        false,
    )
    .map_err(|e| format!("could not catch the exports: {e}"))?;
    Ok(())
}

/// What the page kept of the last export: how many downloads were requested, the name and the type of the file, and
/// what the browser's own XML reader makes of it.
///
/// The picture is read as a scanner would see it: each rectangle of the path makes some modules dark. It is returned as
/// `modules` rows of `0` and `1`, with the number of dark modules in the quiet zone, and the title and description.
fn exported(tab: &Tab) -> Result<serde_json::Value, String> {
    let json = tab
        .evaluate(
            "(async () => { \
               const first = window.__saved[0]; \
               const out = { saved: window.__saved.length, name: first ? first.name : null }; \
               if (!window.__blob) return JSON.stringify(out); \
               out.type = window.__blob.type; \
               const svg = await window.__blob.text(); \
               const doc = new DOMParser().parseFromString(svg, 'image/svg+xml'); \
               out.wellFormed = !doc.querySelector('parsererror'); \
               if (!out.wellFormed) return JSON.stringify(out); \
               const root = doc.documentElement; \
               const [, , width, height] = root.getAttribute('viewBox').split(' ').map(Number); \
               out.viewBox = root.getAttribute('viewBox'); \
               out.pixels = [root.getAttribute('width'), root.getAttribute('height')]; \
               out.description = doc.querySelector('desc').textContent; \
               out.markup = doc.querySelectorAll('b, script, a, image, use, foreignObject').length; \
               const modules = Array.from({ length: height }, () => Array(width).fill(0)); \
               for (const part of doc.querySelector('path').getAttribute('d').split('z')) { \
                 const m = part.trim().match(/^M(\\d+) (\\d+)h(\\d+)v1h-(\\d+)$/); \
                 if (!m) continue; \
                 for (let c = 0; c < Number(m[3]); c++) modules[Number(m[2])][Number(m[1]) + c] = 1; \
               } \
               out.rows = modules.map(row => row.join('')); \
               const edge = (r, c) => r < 4 || r >= height - 4 || c < 4 || c >= width - 4; \
               out.border = modules.flatMap((row, r) => row.filter((v, c) => v && edge(r, c))).length; \
               return JSON.stringify(out); \
             })()",
            true,
        )
        .map_err(|e| format!("could not read the export: {e}"))?
        .value
        .and_then(|v| v.as_str().map(str::to_owned))
        .ok_or_else(|| "the export gave no text".to_owned())?;
    serde_json::from_str(&json).map_err(|e| format!("the export is not JSON: {e}: {json}"))
}

/// The 21 rows of the matrix as text on the page, `1` for dark and `0` for light.
fn matrix_rows(tab: &Tab) -> Result<Vec<String>, String> {
    Ok(text(tab, "document.getElementById('qr-code-matrix-text').textContent")?
        .split('\n')
        .map(str::to_owned)
        .collect())
}

/// Checks that the export the page kept is the symbol on show, in a quiet zone of four light modules.
fn check_export_is_the_symbol(tab: &Tab) -> Result<serde_json::Value, String> {
    let saved = exported(tab)?;
    check_eq(saved["saved"].as_u64(), Some(1))?;
    check_eq(saved["name"].as_str(), Some("qr-code.svg"))?;
    check_eq(saved["type"].as_str(), Some("image/svg+xml"))?;
    check_eq(saved["wellFormed"].as_bool(), Some(true))?;
    check_eq(saved["viewBox"].as_str(), Some("0 0 29 29"))?;
    check_eq(
        (saved["pixels"][0].as_str(), saved["pixels"][1].as_str()),
        (Some("290"), Some("290")),
    )?;
    check_eq(saved["border"].as_u64(), Some(0))?;
    let rows: Vec<String> = saved["rows"]
        .as_array()
        .ok_or("the export has no rows")?
        .iter()
        .map(|row| row.as_str().unwrap_or_default().to_owned())
        .collect();
    check_eq(rows.len(), 29)?;
    let inner: Vec<String> = rows[4..25].iter().map(|row| row[4..25].to_owned()).collect();
    check_eq(inner, matrix_rows(tab)?)?;
    Ok(saved)
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Makes the page keep the address of every file that is made, and every address that is released, as well as catching
/// the downloads as [`catch_exports`] does.
fn watch_addresses(tab: &Tab) -> Result<(), String> {
    catch_exports(tab)?;
    tab.evaluate(
        "(() => { window.__made = []; window.__released = []; \
         const make = URL.createObjectURL.bind(URL); \
         const release = URL.revokeObjectURL.bind(URL); \
         URL.createObjectURL = (blob) => { const url = make(blob); window.__made.push(url); return url; }; \
         URL.revokeObjectURL = (url) => { window.__released.push(url); return release(url); }; })()",
        false,
    )
    .map_err(|e| format!("could not watch the addresses: {e}"))?;
    Ok(())
}

/// How many addresses were made and how many were released, and whether every one released was one that was made.
fn addresses(tab: &Tab) -> Result<(u64, u64, bool), String> {
    let found = text(
        tab,
        "JSON.stringify([window.__made.length, window.__released.length, \
         window.__released.every(url => window.__made.includes(url))])",
    )?;
    let values: serde_json::Value = serde_json::from_str(&found).map_err(|e| format!("{e}: {found}"))?;
    Ok((
        values[0].as_u64().ok_or("no count of addresses made")?,
        values[1].as_u64().ok_or("no count of addresses released")?,
        values[2].as_bool().ok_or("no answer about the addresses")?,
    ))
}

mod unit_tests;
