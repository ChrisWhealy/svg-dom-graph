//! Where everything in the Dijkstra scene goes, and the text that each cell shows. Pure: nothing here touches the DOM,
//! so the layout and every string can be tested without a browser.
//!
//! The scene has the graph on the left, with a box for each vertex. On the right are three arrays side by side, one row
//! for each vertex: the distance, the vertex it was reached from, and whether it is settled. The queue is beside them, to
//! the right of the last. Under the graph is a note about the step. That makes the scene wide and not tall.
//!
//! Nothing is placed from what a step shows. The graph's boxes are where the demo, or the person dragging them, left
//! them. The arrays and the queue are placed from their sizes, which do not change from step to step: see
//! [`widest_by`]. So no step moves anything, however long its text is.

use super::algorithm::{Graph, Snapshot};
use demo_common::util::MARGIN;
use svg_dom::root::utils::{Point, Rect, Size};

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The name of each vertex. A static slice, so that it labels the rows of the arrays.
pub(crate) const NAMES: [&str; 6] = ["A", "B", "C", "D", "E", "F"];

/// How many entries of the queue are shown. The row after them says how many more there are.
pub(super) const QUEUE_ENTRIES: usize = 8;

/// How many cells the queue always has: its entries, and the row that says how many more there are.
pub(super) const QUEUE_CELLS: usize = QUEUE_ENTRIES + 1;

/// What a distance that has not been reached, and a vertex that has no predecessor, are written as. These are not numbers
/// that a distance could have, and are not names either.
pub(super) const UNREACHED: &str = "\u{221E}";
pub(super) const NO_PREDECESSOR: &str = "\u{2014}";

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The room east of the scene for the zoom toolbar.
pub(super) const TOOLBAR_ROOM: f64 = 80.0;

/// Where the top of the scene's content is: below the stepping toolbar, which is fixed to the top edge.
pub(super) const TOP: f64 = 64.0;

/// The radius of a vertex, which is a circle. A single letter fits it with room to spare, so it is never made larger,
/// and the focus ring has room.
pub(crate) const VERTEX_RADIUS: f64 = 22.0;

/// How wide the part of the scene that holds the graph is, and how tall.
const GRAPH_WIDTH: f64 = 560.0;
const GRAPH_HEIGHT: f64 = 310.0;

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The part of the scene that holds the graph: where a vertex can be dragged to, and the one rectangle that anything
/// else that needs the edge of the graph, such as the weighted springs demo's physics, must use. It is derived from the
/// same constants as the layout, so there is never a second copy of it to drift.
pub(crate) fn graph_region() -> Rect {
    Rect {
        origin: Point::new(MARGIN, TOP),
        size: Size::new(GRAPH_WIDTH, GRAPH_HEIGHT),
    }
}

/// The gap between the graph and the arrays, between one array and the next, and between rows of the scene.
const GAP: f64 = 50.0;
const ARRAY_GAP: f64 = 24.0;
const ROW_GAP: f64 = 30.0;

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The centre of the circle whose enclosing square has its top-left corner at `origin`. A vertex is kept as the top-left
/// corner of its square, which is what the scene's own positions are, and is given to the scene as a centre.
pub(super) fn vertex_centre(origin: Point) -> Point {
    Point::new(origin.x + VERTEX_RADIUS, origin.y + VERTEX_RADIUS)
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Where the square that encloses each vertex starts, in the order of [`NAMES`]. The graph is laid out by hand: no
/// connector passes through a vertex, and the two that leave C go off in different directions, so their labels have
/// room.
///
/// The numbers are where the centres are. The vertices were boxes of 56 by 36 before they were circles, and the centres
/// stay where they were, so the corner of the 44 by 44 square is 6 further right and 4 higher than that of the box.
pub(crate) fn vertex_origins() -> [Point; 6] {
    let at = |x: f64, y: f64| Point::new(MARGIN + x + 6.0, TOP + y - 4.0);
    [
        at(0.0, 124.0),   // A
        at(166.0, 14.0),  // B
        at(150.0, 234.0), // C
        at(336.0, 124.0), // D
        at(346.0, 264.0), // E
        at(500.0, 180.0), // F
    ]
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Where everything but the graph goes.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) struct Placement {
    pub(super) distance: Point,
    pub(super) previous: Point,
    pub(super) settled: Point,
    pub(super) queue: Point,
    pub(super) note: Point,
    pub(super) note_width: f64,
    /// The size of the scene: as tall as the lowest thing needs, and with room east for the zoom toolbar.
    pub(super) canvas: Size,
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Where the three arrays, the queue and the note go, given the size of each array and of the queue, which do not change
/// from step to step. The arrays are side by side, with the queue to the right of them, in line with their tops, and the
/// note is below the graph.
///
/// `note_room` is how tall the note can be before it would reach the bottom of the scene. A taller note makes the
/// scene taller, and moves nothing.
pub(super) fn place(arrays: [Size; 3], queue: Size, note_room: f64) -> Placement {
    let left = MARGIN + GRAPH_WIDTH + GAP;
    let distance = Point::new(left, TOP);
    let previous = Point::new(distance.x + arrays[0].width + ARRAY_GAP, TOP);
    let settled = Point::new(previous.x + arrays[1].width + ARRAY_GAP, TOP);
    let tallest = arrays.iter().map(|size| size.height).fold(0.0, f64::max);
    let queue_origin = Point::new(settled.x + arrays[2].width + ARRAY_GAP, TOP);
    let note = Point::new(MARGIN, TOP + GRAPH_HEIGHT + ROW_GAP);
    let right = queue_origin.x + queue.width;
    let bottom = (TOP + tallest).max(queue_origin.y + queue.height).max(note.y + note_room);
    Placement {
        distance,
        previous,
        settled,
        queue: queue_origin,
        note,
        note_width: GRAPH_WIDTH,
        canvas: Size::new(right + MARGIN + TOOLBAR_ROOM, bottom + MARGIN),
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// A distance as a cell shows it: the number, or [`UNREACHED`].
pub(super) fn distance_text(distance: Option<u64>) -> String {
    distance.map_or_else(|| UNREACHED.to_owned(), |distance| distance.to_string())
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// A predecessor as a cell shows it: the name of the vertex, or [`NO_PREDECESSOR`].
pub(super) fn previous_text(graph: &Graph, predecessor: Option<usize>) -> String {
    predecessor.map_or_else(|| NO_PREDECESSOR.to_owned(), |vertex| graph.names()[vertex].clone())
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The queue as its cells show it, always [`QUEUE_CELLS`] of them, nearest entry first. An entry is `(distance, vertex)`,
/// and says `stale` after it if it is out of date. The rows after the entries are empty. If there are more entries than
/// [`QUEUE_ENTRIES`], the last cell says how many more.
///
/// This is the order the entries would be taken in, which is not the order a binary heap keeps them in.
pub(super) fn queue_texts(graph: &Graph, snapshot: &Snapshot) -> Vec<String> {
    let mut cells: Vec<String> = snapshot
        .queue
        .iter()
        .take(QUEUE_ENTRIES)
        .map(|entry| {
            let stale = if entry.stale { " stale" } else { "" };
            format!("({}, {}){stale}", entry.distance, graph.names()[entry.vertex])
        })
        .collect();
    cells.resize(QUEUE_ENTRIES, String::new());
    let more = snapshot.queue.len().saturating_sub(QUEUE_ENTRIES);
    cells.push(if more > 0 { format!("+{more} more") } else { String::new() });
    debug_assert_eq!(cells.len(), QUEUE_CELLS);
    cells
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The text of each cell of the distance array, the previous array and the queue, for one step.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct Texts {
    pub(super) distance: Vec<String>,
    pub(super) previous: Vec<String>,
    pub(super) queue: Vec<String>,
}

impl Texts {
    pub(super) fn of(graph: &Graph, snapshot: &Snapshot) -> Self {
        Self {
            distance: snapshot.distance.iter().map(|&distance| distance_text(distance)).collect(),
            previous: snapshot
                .predecessor
                .iter()
                .map(|&vertex| previous_text(graph, vertex))
                .collect(),
            queue: queue_texts(graph, snapshot),
        }
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The string of `strings` that `measure` finds widest, and the first of them if several are equally wide. `None` for
/// no strings. The caller measures in the browser, in the cell's own font: a character such as `\u{221E}` can come from a
/// font whose advance is not that of the others, so no count of characters says which string is widest.
///
/// Used as a cell-width hint, over every string a column ever shows, so that the column is as wide for the first step as
/// for the last.
pub(super) fn widest_by<E>(
    strings: impl IntoIterator<Item = String>,
    mut measure: impl FnMut(&str) -> Result<f64, E>,
) -> Result<Option<String>, E> {
    let mut widest: Option<(f64, String)> = None;
    for string in strings {
        let width = measure(&string)?;
        if widest.as_ref().is_none_or(|(best, _)| width > *best) {
            widest = Some((width, string));
        }
    }
    Ok(widest.map(|(_, string)| string))
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[cfg(test)]
mod unit_tests;
