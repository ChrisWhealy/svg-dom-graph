//! The Dijkstra panel in the built demo gallery: the graph with its labelled edges, the three arrays and the queue, the
//! emphasis on the examined edge and on the final route, and what a redraw keeps.
//!
//! The algorithm is covered by native unit tests, against a reference graph, a second algorithm run on random graphs, and
//! every case the design lists: ties, stale entries, overflow, parallel edges. The label placer has its own native tests
//! too, down to the points that make a label leave a place. What they cannot reach is the browser: that the text and
//! symbols are drawn and named, that a label really is beside its line and clear of the others when a box is dragged
//! with a real mouse, that edges are emphasised, and that the scene drawn again for each step keeps the zoom, the dragged
//! boxes and the keyboard focus, and moves nothing else.
//!
//! The walk is the same for every demo that steps, so its helpers are shared: see [`crate::sorting::walk_helpers`].
//!
//! Like [`crate::hamming`], these run against the built demo gallery.

mod box4;
mod label;
mod placed;

use crate::common::{demo_tab::DemoTab, drag, new_demo_tab};
use crate::dijkstra::{box4::Box4, label::Label, placed::Placed};
use headless_chrome::Tab;
use serde::Deserialize;
use std::time::{Duration, Instant};

crate::sorting::walk_helpers!("#dijkstra-diagram");

const VIEWPORT: (u32, u32) = (1800, 1800);
const VERTICES: [&str; 6] = ["A", "B", "C", "D", "E", "F"];
/// The weight of each edge, in the order the demo draws them, which is the order their labels are ranked in.
const WEIGHTS: [&str; 9] = ["4", "2", "1", "5", "8", "10", "2", "6", "3"];
/// The edges of the route to F, as the indices of `WEIGHTS`: A to C, C to B, B to D, D to E and E to F.
const ROUTE: [usize; 5] = [1, 2, 3, 6, 8];

fn open() -> Result<DemoTab, String> {
    new_demo_tab(
        "index.html#panel-dijkstra",
        &format!("{DIAGRAM} g[aria-label^=\"Distance:\"]"),
        VIEWPORT,
    )
}

fn check(condition: bool, message: &str) -> Result<(), String> {
    if condition { Ok(()) } else { Err(message.to_owned()) }
}

fn check_eq<T: PartialEq + std::fmt::Debug>(got: T, expected: T) -> Result<(), String> {
    check(got == expected, &format!("expected {expected:?}, got {got:?}"))
}

/// Evaluates `expression`, which gives a JSON string, and reads it as a `T`.
fn json<T: for<'de> Deserialize<'de>>(tab: &Tab, expression: &str) -> Result<T, String> {
    let raw = text(tab, &format!("JSON.stringify({expression})"))?;
    serde_json::from_str(&raw).map_err(|e| format!("could not read {raw}: {e}"))
}

/// The vertices, A to F: nodes of the content layer that have an ellipse and a text with a vertex's name. A vertex is a
/// circle, which is an `<ellipse>`. A node's group also holds a `title` for its accessible name, so the ellipse and the
/// text are found by tag, not by position.
const VERTEX_GROUPS: &str = "[...document.querySelectorAll('#dijkstra-diagram > g.svg-dom-graph-content > g')] \
     .filter(g => g.querySelector(':scope > ellipse') && /^[A-F]$/.test(g.querySelector(':scope > text')?.textContent ?? ''))";
/// The circle and the name of a vertex group `g`.
const VERTEX_SHAPE: &str = "g.querySelector(':scope > ellipse')";
const VERTEX_NAME: &str = "g.querySelector(':scope > text').textContent";

fn vertex_boxes(tab: &Tab) -> Result<Vec<Box4>, String> {
    json(
        tab,
        &format!(
            "{VERTEX_GROUPS}.sort((a, b) => \
             a.querySelector(':scope > text').textContent.localeCompare(b.querySelector(':scope > text').textContent)) \
             .map(g => {{ const r = {VERTEX_SHAPE}.getBoundingClientRect(); \
             return {{ left: r.left, top: r.top, width: r.width, height: r.height }}; }})"
        ),
    )
}

/// The vertices that have the focus ring, by name.
fn ringed(tab: &Tab) -> Result<Vec<String>, String> {
    json(
        tab,
        &format!(
            "{VERTEX_GROUPS}.filter(g => {VERTEX_SHAPE}.getAttribute('stroke-width') === '3') \
             .map(g => {VERTEX_NAME}).sort()"
        ),
    )
}

/// The vertices that are filled as settled, by name.
fn filled_settled(tab: &Tab) -> Result<Vec<String>, String> {
    json(
        tab,
        &format!(
            "{VERTEX_GROUPS}.filter(g => {VERTEX_SHAPE}.getAttribute('fill') === '#d9c9f3') \
             .map(g => {VERTEX_NAME}).sort()"
        ),
    )
}

fn labels(tab: &Tab) -> Result<Vec<Label>, String> {
    json(
        tab,
        "[...document.querySelectorAll('#dijkstra-diagram [data-svg-dom-graph-edge-label]')] \
         .map(g => { const r = g.querySelector('rect').getBoundingClientRect(); \
         return { index: Number(g.getAttribute('data-svg-dom-graph-edge-label')), text: g.querySelector('text').textContent, \
         name: g.getAttribute('aria-label'), left: r.left, top: r.top, width: r.width, height: r.height }; }) \
         .sort((a, b) => a.index - b.index)",
    )
}

/// Points sampled along connector `index`, in the page's pixels.
fn line(tab: &Tab, index: usize) -> Result<Vec<(f64, f64)>, String> {
    json(
        tab,
        &format!(
            "(() => {{ const p = document.querySelectorAll('#dijkstra-diagram > g.svg-dom-graph-content > path')[{index}]; \
             const m = p.getScreenCTM(); const n = p.getTotalLength(); \
             return Array.from({{ length: 101 }}, (_, i) => {{ const q = p.getPointAtLength(n * i / 100); \
             return [m.a * q.x + m.c * q.y + m.e, m.b * q.x + m.d * q.y + m.f]; }}); }})()"
        ),
    )
}

/// The width of connector `index`'s stroke.
fn stroke_widths(tab: &Tab) -> Result<Vec<String>, String> {
    json(
        tab,
        "[...document.querySelectorAll('#dijkstra-diagram > g.svg-dom-graph-content > path')].map(p => p.getAttribute('stroke-width'))",
    )
}

/// The text of each cell of the array whose name starts with `name`.
fn cells(tab: &Tab, name: &str) -> Result<Vec<String>, String> {
    json(
        tab,
        &format!(
            "[...document.querySelectorAll('#dijkstra-diagram g[aria-label^=\"{name}:\"] text[aria-label]')] \
             .map(t => t.textContent)"
        ),
    )
}

/// The accessible name of each cell of the array whose name starts with `name`.
fn cell_names(tab: &Tab, name: &str) -> Result<Vec<String>, String> {
    json(
        tab,
        &format!(
            "[...document.querySelectorAll('#dijkstra-diagram g[aria-label^=\"{name}:\"] text[aria-label]')] \
             .map(t => t.getAttribute('aria-label'))"
        ),
    )
}

/// Where the three arrays, the queue and the vertices are, and how big: each node's own `transform` and the size of its
/// first rectangle, or the diameters of a vertex's circle. Unlike a bounding box on the page, this does not change when a selection or a focus ring thickens a
/// stroke.
fn placed(tab: &Tab) -> Result<Vec<Placed>, String> {
    json(
        tab,
        &format!(
            "[...['Distance:', 'Previous:', 'Settled:', 'Queue'] \
               .map(name => document.querySelector(`#dijkstra-diagram g[aria-label^=\"${{name}}\"]`)), \
             ...{VERTEX_GROUPS}.sort((a, b) => \
               a.querySelector(':scope > text').textContent.localeCompare(b.querySelector(':scope > text').textContent))] \
             .map(g => {{ const r = g.querySelector('rect, ellipse'); const round = r.tagName === 'ellipse'; \
             return {{ at: g.getAttribute('transform'), \
             width: round ? 2 * Number(r.getAttribute('rx')) : Number(r.getAttribute('width')), \
             height: round ? 2 * Number(r.getAttribute('ry')) : Number(r.getAttribute('height')) }}; }})"
        ),
    )
}

/// Whether every node is where it was, exactly, and as big as it was to within `SIZE_TOLERANCE`.
fn same_places(before: &[Placed], after: &[Placed]) -> bool {
    before.len() == after.len()
        && before.iter().zip(after).all(|(x, y)| {
            x.at == y.at && (x.width - y.width).abs() < SIZE_TOLERANCE && (x.height - y.height).abs() < SIZE_TOLERANCE
        })
}

/// How much the size of a node may differ between two steps. The browser measures the same text a little differently
/// from one layout to the next, by a few hundredths of a unit, and that is not a node changing size.
const SIZE_TOLERANCE: f64 = 0.25;

mod unit_tests;
