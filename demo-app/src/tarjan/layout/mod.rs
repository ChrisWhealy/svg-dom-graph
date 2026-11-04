//! Where the 64 vertices are, and how their connectors leave them. Pure: it is worked out from the size of one vertex's
//! box, which the page measures, and nothing else, so the same box always gives the same picture.
//!
//! The vertices are on a staggered grid, in an order that does not follow the components. A person who watches the
//! search therefore cannot see a component before the search has found it, and a component is shown by the colour that
//! its vertices are given and by its label, not by a box round vertices that were already together. The order was
//! chosen by a search that puts as few connectors as it can through a vertex that is not theirs, and is kept in
//! [`SLOT`], so that a picture is the same each time.
//!
//! The same scene holds the stack, the roster of components and the condensation, in an area to the left of the graph.

use super::fixture::VERTICES;
use svg_dom_graph::scene::Side;

/// How far the box of a vertex is allowed to come to a connector that does not end at it, in scene units.
#[cfg(test)]
pub(crate) const CLEARANCE: f64 = 4.0;

/// The size of the grid, which has room for a few more places than there are vertices.
pub(crate) const COLUMNS: usize = 10;
#[cfg(test)]
pub(crate) const ROWS: usize = 7;

/// How far apart the places are, as a multiple of the box. A row of odd number is moved half a place to the right.
const PITCH: (f64, f64) = (1.5, 1.45);

/// The room round the whole picture.
const MARGIN: f64 = 40.0;

/// How much room there is to the left of the graph, which holds the stack, the roster and the condensation.
pub(crate) const LEFT_PANEL: f64 = 1230.0;

/// Where the node of the stack is, and how many cells it has, which is as many as the stack can be deep.
pub(crate) const STACK_AT: (f64, f64) = (12.0, 40.0);
pub(crate) const STACK_ROWS: usize = 27;

/// Where the roster of components starts, and the size of one of its rows and the gap between two.
pub(crate) const ROSTER_AT: (f64, f64) = (200.0, 70.0);
pub(crate) const ROSTER_ROW: (f64, f64) = (560.0, 56.0);
pub(crate) const ROSTER_GAP: f64 = 6.0;

/// Where the condensation's area starts, which is below the roster, and how much room it is given.
const DAG_AT: (f64, f64) = (200.0, 790.0);
pub(crate) const DAG_HEIGHT: f64 = 300.0;

/// The place on the grid that each vertex is in, counted along the rows from the top left. These were chosen by search
/// against the measured box and are kept.
pub(crate) const SLOT: [usize; VERTICES] = [
    6, 15, 5, 14, 25, 59, 8, 19, 9, 18, 17, 58, 7, 48, 57, 47, 55, 56, 67, 68, 69, 12, 22, 13, 4, 3, 66, 43, 33, 23,
    29, 1, 37, 26, 27, 38, 28, 39, 52, 44, 32, 21, 42, 64, 53, 45, 36, 35, 24, 34, 65, 11, 10, 0, 60, 20, 63, 46, 61,
    51, 41, 50, 30, 31,
];

/// Where along a side an antiparallel pair's two connectors are pinned, as a fraction of its length. One of the pair is
/// at the first and the other at the second, so that the two run side by side.
pub(crate) const LANES: (f64, f64) = (0.2, 0.8);

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// A rectangle, by its top left corner and its size.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct Rect {
    pub(crate) x: f64,
    pub(crate) y: f64,
    pub(crate) width: f64,
    pub(crate) height: f64,
}

impl Rect {
    fn centred(centre: (f64, f64), width: f64, height: f64) -> Self {
        Self {
            x: centre.0 - width / 2.0,
            y: centre.1 - height / 2.0,
            width,
            height,
        }
    }

    pub(crate) fn centre(&self) -> (f64, f64) {
        (self.x + self.width / 2.0, self.y + self.height / 2.0)
    }

    pub(crate) fn right(&self) -> f64 {
        self.x + self.width
    }

    pub(crate) fn bottom(&self) -> f64 {
        self.y + self.height
    }

    #[cfg(test)]
    fn grown(&self, by: f64) -> Self {
        Self {
            x: self.x - by,
            y: self.y - by,
            width: self.width + 2.0 * by,
            height: self.height + 2.0 * by,
        }
    }

    /// Whether the segment from `a` to `b` has a point inside this rectangle.
    #[cfg(test)]
    pub(crate) fn meets(&self, a: (f64, f64), b: (f64, f64)) -> bool {
        // Liang and Barsky: the part of the segment that is inside each pair of parallel sides.
        let (mut t0, mut t1) = (0.0f64, 1.0f64);
        for (p, q, lo, hi) in [(a.0, b.0, self.x, self.right()), (a.1, b.1, self.y, self.bottom())] {
            let d = q - p;
            if d.abs() < 1e-12 {
                if p < lo || p > hi {
                    return false;
                }
            } else {
                let (u, v) = ((lo - p) / d, (hi - p) / d);
                t0 = t0.max(u.min(v));
                t1 = t1.min(u.max(v));
                if t0 > t1 {
                    return false;
                }
            }
        }
        true
    }
}

/// Everything that is placed.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Layout {
    /// The size of the box of every vertex, as measured.
    pub(crate) box_size: (f64, f64),
    centres: Vec<(f64, f64)>,
    /// The size of the whole picture.
    pub(crate) size: (f64, f64),
    /// The top left of the area that the condensation is placed in.
    pub(crate) dag_origin: (f64, f64),
}

/// The centre of place `slot` of the grid, for boxes of this size.
pub(crate) fn place(slot: usize, width: f64, height: f64) -> (f64, f64) {
    let (row, column) = (slot / COLUMNS, slot % COLUMNS);
    let stagger = if row % 2 == 1 { PITCH.0 * width / 2.0 } else { 0.0 };
    (
        LEFT_PANEL + MARGIN + width / 2.0 + stagger + column as f64 * PITCH.0 * width,
        MARGIN + height / 2.0 + row as f64 * PITCH.1 * height,
    )
}

impl Layout {
    /// The layout for vertex boxes of this size.
    pub(crate) fn build(width: f64, height: f64) -> Self {
        Self::build_with(width, height, &SLOT)
    }

    /// The layout with a chosen place for each vertex, which the search for them uses.
    pub(crate) fn build_with(width: f64, height: f64, slots: &[usize; VERTICES]) -> Self {
        let centres: Vec<(f64, f64)> = slots.iter().map(|&s| place(s, width, height)).collect();
        let right = centres.iter().map(|c| c.0 + width / 2.0).fold(0.0, f64::max);
        let bottom = centres.iter().map(|c| c.1 + height / 2.0).fold(0.0, f64::max);
        let stack_bottom = STACK_AT.1 + 1040.0;
        Self {
            box_size: (width, height),
            centres,
            size: (right + MARGIN, bottom.max(stack_bottom).max(DAG_AT.1 + DAG_HEIGHT) + MARGIN),
            dag_origin: DAG_AT,
        }
    }

    /// The rectangle of row `i` of the roster, counted from the top.
    pub(crate) fn roster_row(i: usize) -> Rect {
        Rect {
            x: ROSTER_AT.0,
            y: ROSTER_AT.1 + i as f64 * (ROSTER_ROW.1 + ROSTER_GAP),
            width: ROSTER_ROW.0,
            height: ROSTER_ROW.1,
        }
    }

    /// Where the centre of a place of the condensation's table is, in the scene.
    pub(crate) fn dag_centre(&self, slot: (f64, f64)) -> (f64, f64) {
        (self.dag_origin.0 + slot.0, self.dag_origin.1 + slot.1)
    }

    pub(crate) fn centre(&self, v: usize) -> (f64, f64) {
        self.centres[v]
    }

    /// The box of vertex `v`.
    pub(crate) fn rect(&self, v: usize) -> Rect {
        Rect::centred(self.centres[v], self.box_size.0, self.box_size.1)
    }

    /// The smallest rectangle that has the boxes of these vertices in it, with this much room round.
    pub(crate) fn bounds_of(&self, members: &[usize], padding: f64) -> Option<Rect> {
        let boxes: Vec<Rect> = members.iter().map(|&v| self.rect(v)).collect();
        let left = boxes.iter().map(|b| b.x).reduce(f64::min)?;
        let top = boxes.iter().map(|b| b.y).reduce(f64::min)?;
        let right = boxes.iter().map(Rect::right).reduce(f64::max)?;
        let bottom = boxes.iter().map(Rect::bottom).reduce(f64::max)?;
        Some(Rect {
            x: left - padding,
            y: top - padding,
            width: right - left + 2.0 * padding,
            height: bottom - top + 2.0 * padding,
        })
    }

    /// The vertices that the straight line between the centres of `from` and `to` passes through, other than those two.
    #[cfg(test)]
    pub(crate) fn through(&self, from: usize, to: usize) -> Vec<usize> {
        (0..VERTICES)
            .filter(|&v| v != from && v != to)
            .filter(|&v| self.rect(v).grown(CLEARANCE).meets(self.centres[from], self.centres[to]))
            .collect()
    }

    /// The edges that pass through a vertex that is not theirs, with the vertices that they pass through.
    #[cfg(test)]
    pub(crate) fn occlusions(&self, edges: &[(usize, usize)]) -> Vec<(usize, Vec<usize>)> {
        edges
            .iter()
            .enumerate()
            .filter_map(|(i, &(a, b))| {
                let hit = self.through(a, b);
                (!hit.is_empty()).then_some((i, hit))
            })
            .collect()
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// How one end of a connector is fixed to its vertex.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) enum End {
    /// The library chooses, from a ray between the centres.
    Auto,
    /// Pinned to this side of the box, this fraction of the way along it.
    Pinned(Side, f64),
}

/// How both ends of a connector are fixed.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct Ends {
    pub(crate) from: End,
    pub(crate) to: End,
}

/// How the connector of the edge `from` to `to` leaves and arrives. An edge whose reverse is also an edge is pinned to
/// a lane, so that the two run side by side. Any other edge is left to the library.
pub(crate) fn ends(layout: &Layout, from: usize, to: usize, reverse_exists: bool) -> Ends {
    if !reverse_exists {
        return Ends { from: End::Auto, to: End::Auto };
    }
    let (a, b) = (layout.centre(from), layout.centre(to));
    let (dx, dy) = (b.0 - a.0, b.1 - a.1);
    let lane = if from < to { LANES.0 } else { LANES.1 };
    let (leave, arrive) = if dx.abs() >= dy.abs() {
        if dx >= 0.0 { (Side::East, Side::West) } else { (Side::West, Side::East) }
    } else if dy >= 0.0 {
        (Side::South, Side::North)
    } else {
        (Side::North, Side::South)
    };
    Ends {
        from: End::Pinned(leave, lane),
        to: End::Pinned(arrive, lane),
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[cfg(test)]
mod unit_tests;
