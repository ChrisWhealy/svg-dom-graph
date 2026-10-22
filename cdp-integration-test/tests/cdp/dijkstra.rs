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

use crate::common::{DemoTab, drag, new_demo_tab};
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

/// A rectangle as the page reports it.
#[derive(Deserialize, Clone, Copy, Debug, PartialEq)]
struct Box4 {
    left: f64,
    top: f64,
    width: f64,
    height: f64,
}

impl Box4 {
    fn overlaps(self, other: Self) -> bool {
        self.left < other.left + other.width
            && self.left + self.width > other.left
            && self.top < other.top + other.height
            && self.top + self.height > other.top
    }

    fn centre(self) -> (f64, f64) {
        (self.left + self.width / 2.0, self.top + self.height / 2.0)
    }
}

/// Evaluates `expression`, which gives a JSON string, and reads it as a `T`.
fn json<T: for<'de> Deserialize<'de>>(tab: &Tab, expression: &str) -> Result<T, String> {
    let raw = text(tab, &format!("JSON.stringify({expression})"))?;
    serde_json::from_str(&raw).map_err(|e| format!("could not read {raw}: {e}"))
}

/// The vertex boxes, A to F: nodes of the content layer that have a rectangle and a text with a vertex's name. A node's
/// group also holds a `title` for its accessible name, so the rectangle and the text are found by tag, not by position.
const VERTEX_GROUPS: &str = "[...document.querySelectorAll('#dijkstra-diagram > g.svg-dom-graph-content > g')] \
     .filter(g => g.querySelector(':scope > rect') && /^[A-F]$/.test(g.querySelector(':scope > text')?.textContent ?? ''))";
/// The rectangle and the name of a vertex group `g`.
const VERTEX_RECT: &str = "g.querySelector(':scope > rect')";
const VERTEX_NAME: &str = "g.querySelector(':scope > text').textContent";

fn vertex_boxes(tab: &Tab) -> Result<Vec<Box4>, String> {
    json(
        tab,
        &format!(
            "{VERTEX_GROUPS}.sort((a, b) => \
             a.querySelector(':scope > text').textContent.localeCompare(b.querySelector(':scope > text').textContent)) \
             .map(g => {{ const r = {VERTEX_RECT}.getBoundingClientRect(); \
             return {{ left: r.left, top: r.top, width: r.width, height: r.height }}; }})"
        ),
    )
}

/// The vertices that have the focus ring, by name.
fn ringed(tab: &Tab) -> Result<Vec<String>, String> {
    json(
        tab,
        &format!(
            "{VERTEX_GROUPS}.filter(g => {VERTEX_RECT}.getAttribute('stroke-width') === '3') \
             .map(g => {VERTEX_NAME}).sort()"
        ),
    )
}

/// The vertices that are filled as settled, by name.
fn filled_settled(tab: &Tab) -> Result<Vec<String>, String> {
    json(
        tab,
        &format!(
            "{VERTEX_GROUPS}.filter(g => {VERTEX_RECT}.getAttribute('fill') === '#d9c9f3') \
             .map(g => {VERTEX_NAME}).sort()"
        ),
    )
}

/// One edge label as the page draws it: its text, its box, and its index.
#[derive(Deserialize, Debug)]
struct Label {
    index: usize,
    text: String,
    #[serde(flatten)]
    area: Box4,
    name: String,
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

/// Where one node is, and how big, as the scene itself has them.
#[derive(Deserialize, Debug, PartialEq)]
struct Placed {
    at: String,
    width: f64,
    height: f64,
}

/// Where the three arrays, the queue and the vertices are, and how big: each node's own `transform` and the size of its
/// first rectangle. Unlike a bounding box on the page, this does not change when a selection or a focus ring thickens a
/// stroke.
fn placed(tab: &Tab) -> Result<Vec<Placed>, String> {
    json(
        tab,
        &format!(
            "[...['Distance:', 'Previous:', 'Settled:', 'Queue'] \
               .map(name => document.querySelector(`#dijkstra-diagram g[aria-label^=\"${{name}}\"]`)), \
             ...{VERTEX_GROUPS}.sort((a, b) => \
               a.querySelector(':scope > text').textContent.localeCompare(b.querySelector(':scope > text').textContent))] \
             .map(g => {{ const r = g.querySelector('rect'); \
             return {{ at: g.getAttribute('transform'), width: Number(r.getAttribute('width')), \
             height: Number(r.getAttribute('height')) }}; }})"
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

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The panel opens on the reference graph with the walk at its start. Every edge has a label with its weight, in the
/// order of the edges, and named for assistive technology. The distances are `0` and `∞`, drawn as text, the predecessors
/// are dashes, nothing is settled, and the queue holds the source, with the rest of its rows empty and read out as such.
#[test]
fn the_panel_opens_on_the_graph_with_labelled_edges_and_arrays_of_text() -> Result<(), String> {
    let tab = open()?;
    check_eq(vertex_boxes(&tab)?.len(), 6)?;
    let labels = labels(&tab)?;
    check_eq(labels.iter().map(|l| l.index).collect::<Vec<_>>(), (0..WEIGHTS.len()).collect())?;
    check_eq(labels.iter().map(|l| l.text.as_str()).collect::<Vec<_>>(), WEIGHTS.to_vec())?;
    check(
        labels
            .iter()
            .all(|l| l.name.starts_with("Edge from ") && l.name.ends_with(&format!(": {}", l.text))),
        &format!(
            "each label is named for its edge: {:?}",
            labels.iter().map(|l| &l.name).collect::<Vec<_>>()
        ),
    )?;
    check_eq(labels[0].name.as_str(), "Edge from A to B: 4")?;

    check_eq(
        cells(&tab, "Distance")?,
        ["0", "\u{221E}", "\u{221E}", "\u{221E}", "\u{221E}", "\u{221E}"]
            .map(String::from)
            .to_vec(),
    )?;
    check_eq(cells(&tab, "Previous")?, vec!["\u{2014}".to_owned(); 6])?;
    check_eq(cells(&tab, "Settled")?, vec!["false".to_owned(); 6])?;
    let queue = cells(&tab, "Queue, nearest first")?;
    check_eq(queue.len(), 9)?;
    check_eq(queue[0].as_str(), "(0, A)")?;
    check(
        queue[1..].iter().all(String::is_empty),
        &format!("the rest of the queue is empty: {queue:?}"),
    )?;
    let names = cell_names(&tab, "Queue, nearest first")?;
    check(
        names[1..].iter().all(|n| n.ends_with(": empty")),
        &format!("empty cells are read out as empty: {names:?}"),
    )?;
    check(
        cell_names(&tab, "Distance")?[1].ends_with(": \u{221E}"),
        "the unreached mark is in the name",
    )?;
    check_eq(step_position(&tab)?, Some(0))?;
    check(said(&tab)?.starts_with("Step 1 of "), "the first step is announced")
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Every label is clear of the boxes, of the other labels and of its own line, at rest. Then C is dragged with a real
/// mouse through several places, and C to D and C to E, whose routes leave C together, keep their labels apart and clear
/// of the lines, which have followed.
#[test]
fn labels_stay_beside_their_connectors_and_apart_at_rest_and_while_a_vertex_is_dragged() -> Result<(), String> {
    let tab = open()?;
    let check_all = |when: &str| -> Result<(), String> {
        let (boxes, labels) = (vertex_boxes(&tab)?, labels(&tab)?);
        for (i, label) in labels.iter().enumerate() {
            let line = line(&tab, i)?;
            check(
                !line.iter().any(|&(x, y)| {
                    x > label.area.left
                        && x < label.area.left + label.area.width
                        && y > label.area.top
                        && y < label.area.top + label.area.height
                }),
                &format!("{when}: the line of edge {i} passes through its label {:?}", label.area),
            )?;
            for (v, &vertex) in boxes.iter().enumerate() {
                check(
                    !label.area.overlaps(vertex),
                    &format!("{when}: the label of edge {i} is on {}", VERTICES[v]),
                )?;
            }
            for (j, other) in labels.iter().enumerate().skip(i + 1) {
                check(!label.area.overlaps(other.area), &format!("{when}: labels {i} and {j} overlap"))?;
            }
        }
        Ok(())
    };
    check_all("at rest")?;

    for (to, name) in [
        ((0.0, -90.0), "C moved up"),
        ((60.0, -40.0), "C moved up and right"),
        ((0.0, 0.0), "C back"),
    ] {
        let c = vertex_boxes(&tab)?[2].centre();
        drag(&tab, &[c, (c.0 + to.0 / 2.0, c.1 + to.1 / 2.0), (c.0 + to.0, c.1 + to.1)])?;
        std::thread::sleep(Duration::from_millis(150));
        check_all(name)?;
    }
    Ok(())
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// A step that examines an edge emphasises that edge and no other, and rings the two vertices it joins. The arrays have
/// no focus ring, and the vertices no selection: no node has both. At the last step every edge of the route to F is
/// emphasised at once, and no other, and the vertices are all settled.
#[test]
fn the_examined_edge_and_then_the_whole_route_are_emphasised() -> Result<(), String> {
    let tab = open()?;
    check(
        stroke_widths(&tab)?.iter().all(|w| w == "1.5"),
        "no edge is emphasised at the start",
    )?;
    check(ringed(&tab)?.is_empty(), "no vertex is ringed at the start")?;

    goto_note(&tab, "Edge A to B")?;
    let thick: Vec<usize> = stroke_widths(&tab)?
        .iter()
        .enumerate()
        .filter(|(_, w)| *w == "3.5")
        .map(|(i, _)| i)
        .collect();
    check_eq(thick, vec![0])?;
    check_eq(ringed(&tab)?, vec!["A".to_owned(), "B".to_owned()])?;
    check_eq(
        text(
            &tab,
            &format!("document.querySelectorAll('{DIAGRAM} path[aria-current=\"true\"]').length.toString()"),
        )?
        .as_str(),
        "1",
    )?;
    // A data node takes a selection and a plain one a ring, and never both: no array has a ring.
    let ring_on_array = text(
        &tab,
        &format!(
            "[...document.querySelectorAll('{DIAGRAM} g[aria-label*=\"data grid\"] rect')].some(r => r.getAttribute('stroke-width') === '3').toString()"
        ),
    )?;
    check_eq(ring_on_array.as_str(), "false")?;

    let last = step_count(&tab)? - 1;
    goto(&tab, last)?;
    let thick: Vec<usize> = stroke_widths(&tab)?
        .iter()
        .enumerate()
        .filter(|(_, w)| *w == "3.5")
        .map(|(i, _)| i)
        .collect();
    check_eq(thick, ROUTE.to_vec())?;
    check(
        said(&tab)?.contains("A to C to B to D to E to F"),
        &format!("the route is said: {:?}", said(&tab)?),
    )?;
    check(said(&tab)?.contains("length of 13"), "and its length")?;
    check_eq(
        cells(&tab, "Distance")?,
        ["0", "3", "2", "8", "10", "13"].map(String::from).to_vec(),
    )?;
    check_eq(
        cells(&tab, "Previous")?,
        ["\u{2014}", "C", "A", "B", "D", "E"].map(String::from).to_vec(),
    )?;
    check_eq(filled_settled(&tab)?.len(), 6)
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// A step draws the scene again, and keeps the zoom, where a vertex was dragged to, and the keyboard focus on the
/// toolbar button that had it, and does the same going back.
#[test]
fn a_redraw_keeps_the_zoom_the_dragged_vertex_and_the_keyboard_focus() -> Result<(), String> {
    let tab = open()?;
    tab.find_element(&format!("{DIAGRAM} g[aria-label=\"Zoom in\"]"))
        .map_err(|e| format!("could not find Zoom in: {e}"))?
        .click()
        .map_err(|e| format!("could not zoom in: {e}"))?;
    std::thread::sleep(Duration::from_millis(300));
    let zoomed = view(&tab)?;
    check(!zoomed.is_empty(), "zooming in changes the view")?;

    let b = vertex_boxes(&tab)?[1].centre();
    drag(&tab, &[b, (b.0 + 30.0, b.1 + 20.0), (b.0 + 60.0, b.1 + 40.0)])?;
    std::thread::sleep(Duration::from_millis(150));
    let moved = vertex_boxes(&tab)?[1];
    check(
        (moved.centre().0 - (b.0 + 60.0)).abs() < 1.5,
        &format!("B was dragged: {moved:?}"),
    )?;

    tab.evaluate(
        &format!("document.querySelector('{DIAGRAM} g[aria-label=\"Next selection\"]').focus()"),
        false,
    )
    .map_err(|e| format!("could not focus Next: {e}"))?;
    let from = step_position(&tab)?;
    tab.press_key("Enter").map_err(|e| format!("could not press Enter: {e}"))?;
    wait_for_move(&tab, from)?;

    check_eq(view(&tab)?, zoomed.clone())?;
    let after = vertex_boxes(&tab)?[1];
    check(
        (after.left - moved.left).abs() < 0.5 && (after.top - moved.top).abs() < 0.5,
        &format!("B went back to where it was dragged: {moved:?} then {after:?}"),
    )?;
    check_eq(
        text(&tab, "document.activeElement?.getAttribute('aria-label') ?? 'none'")?.as_str(),
        "Next selection",
    )?;
    goto(&tab, 0)?;
    check_eq(view(&tab)?, zoomed)?;
    let back = vertex_boxes(&tab)?[1];
    check(
        (back.left - moved.left).abs() < 0.5 && (back.top - moved.top).abs() < 0.5,
        "and again going back",
    )
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Nothing but the note moves or changes size from one step to the next: the three arrays, the queue and every vertex
/// are exactly where they were at the start in every step, however much the text in them changes.
#[test]
fn the_arrays_the_queue_and_the_vertices_never_move_or_change_size() -> Result<(), String> {
    let tab = open()?;
    let before = placed(&tab)?;
    check_eq(before.len(), 4 + VERTICES.len())?;
    let count = step_count(&tab)?;
    for position in 1..count {
        click(&tab, "Next selection")?;
        check(
            same_places(&before, &placed(&tab)?),
            &format!("an array, the queue or a vertex moved or resized at step {position}"),
        )?;
    }
    check_eq(step_position(&tab)?, Some(count - 1))
}
