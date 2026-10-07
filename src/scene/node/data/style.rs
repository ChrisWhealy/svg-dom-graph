//! How a data node's cells are styled: the stroke constants, and the one place that decides a cell's fill, stroke and
//! dash from its focus, band and secondary state.

use crate::colours::{BOX_STROKE, SELECTION_BAND, SELECTION_FOCUS, SELECTION_SECONDARY, SELECTION_SECONDARY_STROKE};

/// `Scene::set_selection`'s own row/column-level stroke width, thicker than every cell's own default border (see
/// [`BoxHandles::cell_stroke_width`]).
///
/// Colour alone is not a reliable channel: it conveys nothing to assistive technology, and can be hard to tell apart
/// for a colour-blind reader. A band is therefore also distinguishable by its own thicker border, the same "not colour
/// alone" reasoning [`NodeValues::type_colour`](crate::model::content::NodeValues::type_colour)'s own
/// `<title>`/`aria-label` pairing already follows.
///
/// Already formatted — see [`BoxHandles::cell_stroke_width`]'s own doc comment for why.
pub(super) const SELECTION_BAND_STROKE_WIDTH: &str = "2";

/// `Scene::set_selection`'s own cell-level stroke width, thicker again than [`SELECTION_BAND_STROKE_WIDTH`], so the
/// focused cell stays visually distinct from a plain band even with colour perception set aside entirely.
///
/// Already formatted, for the same reason [`SELECTION_BAND_STROKE_WIDTH`] is.
pub(super) const SELECTION_FOCUS_STROKE_WIDTH: &str = "3.5";

/// A secondary-selected cell's own stroke width, between a plain cell's own border and [`SELECTION_BAND_STROKE_WIDTH`].
pub(super) const SELECTION_SECONDARY_STROKE_WIDTH: &str = "2";

/// The dash pattern a secondary-selected cell's own outline is drawn with. Every other cell is `"none"`, a solid line.
pub(super) const SELECTION_SECONDARY_DASH: &str = "5 3";

/// Every attribute `Scene::set_selection`/`Scene::set_secondary_selection` ever write to a cell's own `<rect>`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct CellStyle {
    pub(super) fill: &'static str,
    pub(super) stroke_width: &'static str,
    pub(super) stroke: &'static str,
    pub(super) dash: &'static str,
}

impl CellStyle {
    /// Writes this style onto `cell`.
    pub(super) fn apply(self, cell: &svg_dom::SvgNode) -> Result<(), svg_dom::Error> {
        cell.set_fill(self.fill)?;
        cell.set_attr("stroke-width", self.stroke_width)?;
        cell.set_attr("stroke", self.stroke)?;
        cell.set_attr("stroke-dasharray", self.dash)
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Cell `i`'s own style under a resolved `focus`/`band` and a `secondary` flag, against `base_colour`/
/// `base_stroke_width` for a cell none of them names.
///
/// Precedence: the focused cell, then a secondary one, then a banded one, then the default. So a cell that is both
/// focused and secondary shows as focused. Each of the three is a flag, not an index list: the caller already knows
/// whether its cell is a member, and this stays allocation-free.
///
/// `Scene::set_selection` and `Scene::set_secondary_selection` call this twice for each cell they visit: once for the
/// old state, once for the new one. They only write to the DOM when the two results differ. Each visits only the cells
/// a changed old/new focus, band, or secondary set could plausibly affect, not every cell in the grid.
pub(super) fn cell_style(
    focus: bool,
    banded: bool,
    secondary: bool,
    base_colour: &'static str,
    base_stroke_width: &'static str,
) -> CellStyle {
    let (fill, stroke_width, stroke, dash) = if focus {
        (SELECTION_FOCUS, SELECTION_FOCUS_STROKE_WIDTH, BOX_STROKE, "none")
    } else if secondary {
        (
            SELECTION_SECONDARY,
            SELECTION_SECONDARY_STROKE_WIDTH,
            SELECTION_SECONDARY_STROKE,
            SELECTION_SECONDARY_DASH,
        )
    } else if banded {
        (SELECTION_BAND, SELECTION_BAND_STROKE_WIDTH, BOX_STROKE, "none")
    } else {
        (base_colour, base_stroke_width, BOX_STROKE, "none")
    };
    CellStyle {
        fill,
        stroke_width,
        stroke,
        dash,
    }
}
