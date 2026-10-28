//! Where everything in the FFT overview goes, and which connector joins what. Pure: nothing here touches the DOM.
//!
//! The overview has one column for each boundary `S0 … SL`, and one row for each lane. Every node is the same size,
//! measured for the widest number it will ever show, so no step moves anything. Between two columns, a butterfly joins
//! two lanes of the left column to the same two lanes of the right. It uses four straight connectors, which cross in
//! the middle as the wings of a butterfly do.

use demo_common::util::MARGIN;
use svg_dom::root::utils::{Point, Size};

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Where the top of the content is: below the stepping toolbar, which is fixed to the top edge.
pub(super) const TOP: f64 = 64.0;

/// Room east of the last column for the zoom toolbar.
pub(super) const TOOLBAR_ROOM: f64 = 80.0;

/// The gap between the heading of a column and its first node, and between two rows.
const HEADING_GAP: f64 = 14.0;
const ROW_GAP: f64 = 12.0;

/// The gap between two columns. The sixteen-point scene has longer connectors, so its columns are further apart.
pub(super) const fn column_gap(lanes: usize) -> f64 {
    if lanes <= 8 { 130.0 } else { 190.0 }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The grid of the overview.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) struct Grid {
    node: Size,
    heading_height: f64,
    lanes: usize,
    boundaries: usize,
}

impl Grid {
    /// The grid of `lanes` rows and `boundaries` columns of nodes the size of `node`, with headings `heading_height`
    /// tall.
    pub(super) const fn new(lanes: usize, boundaries: usize, node: Size, heading_height: f64) -> Self {
        Self {
            node,
            heading_height,
            lanes,
            boundaries,
        }
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    /// How far apart the left edges of two columns are.
    fn pitch(&self) -> f64 {
        self.node.width + column_gap(self.lanes)
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    /// The left edge of a column.
    pub(super) fn column_x(&self, boundary: usize) -> f64 {
        MARGIN + self.pitch() * boundary as f64
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    /// The top-left corner of the heading of a column.
    pub(super) fn heading_origin(&self, boundary: usize) -> Point {
        Point::new(self.column_x(boundary), TOP)
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    /// The top-left corner of the node in `lane` of a column.
    pub(super) fn node_origin(&self, boundary: usize, lane: usize) -> Point {
        let top = TOP + self.heading_height + HEADING_GAP;
        Point::new(self.column_x(boundary), top + (self.node.height + ROW_GAP) * lane as f64)
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    /// The size of the scene: the last node and the margins, and room for the zoom toolbar.
    pub(super) fn canvas(&self) -> Size {
        let last = self.node_origin(self.boundaries - 1, self.lanes - 1);
        Size::new(
            last.x + self.node.width + MARGIN + TOOLBAR_ROOM,
            last.y + self.node.height + MARGIN,
        )
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// One connector of a butterfly: from a lane of the left column to a lane of the right. It leaves and arrives a
/// fraction of the way down a side, so the four connectors of a butterfly use four different points.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) struct Dependency {
    pub(super) source: usize,
    pub(super) target: usize,
    pub(super) from_position: f64,
    pub(super) to_position: f64,
}

/// How far down a side the upper of two connectors meets it, and the lower.
const UPPER_POSITION: f64 = 0.3;
const LOWER_POSITION: f64 = 0.7;

/// The four connectors of the butterfly of `upper` and `lower`: `a → sum`, `a → difference`, `b → sum` and
/// `b → difference`, in that order. A connector to the upper lane leaves by the upper point, and one to the lower lane
/// by the lower. A connector from `a` arrives at the upper point, and one from `b` at the lower.
pub(super) const fn dependencies(upper: usize, lower: usize) -> [Dependency; 4] {
    [
        Dependency {
            source: upper,
            target: upper,
            from_position: UPPER_POSITION,
            to_position: UPPER_POSITION,
        },
        Dependency {
            source: upper,
            target: lower,
            from_position: LOWER_POSITION,
            to_position: UPPER_POSITION,
        },
        Dependency {
            source: lower,
            target: upper,
            from_position: UPPER_POSITION,
            to_position: LOWER_POSITION,
        },
        Dependency {
            source: lower,
            target: lower,
            from_position: LOWER_POSITION,
            to_position: LOWER_POSITION,
        },
    ]
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The name of the node in `lane` of boundary `boundary`. The input column also says which sample the lane starts with.
pub(super) fn lane_name(boundary: usize, lane: usize, source: usize) -> String {
    if boundary == 0 {
        format!("S0[{lane}]=x[{source}]")
    } else {
        format!("S{boundary}[{lane}]")
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The heading of a column.
pub(super) fn heading(boundary: usize) -> String {
    if boundary == 0 {
        "S0 \u{00B7} bit-reversed input".to_owned()
    } else {
        format!("S{boundary} \u{00B7} block width {}", 1_usize << boundary)
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The item of `items` that is widest, or `None` if there are none. `width` measures one; the first of equals wins.
///
/// # Errors
///
/// Returns the first error `width` returns.
pub(super) fn widest_by<T>(
    items: impl IntoIterator<Item = T>,
    mut width: impl FnMut(&T) -> Result<f64, String>,
) -> Result<Option<T>, String> {
    let mut best: Option<(f64, T)> = None;
    for item in items {
        let w = width(&item)?;
        if best.as_ref().is_none_or(|(widest, _)| w > *widest) {
            best = Some((w, item));
        }
    }
    Ok(best.map(|(_, item)| item))
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[cfg(test)]
mod unit_tests;
