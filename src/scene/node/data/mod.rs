//! Data nodes — a grid of typed values instead of a plain text label, and their own cell/row/column
//! [`Selection`](crate::scene::Selection) highlighting. See `super::content` for the pure grid-shape/formatting logic;
//! everything DOM-specific (rendering the grid, sizing the box to fit it, recolouring cells) lives here. See
//! [`super::plain`] for a node with a plain text label instead, and [`super::operator`] for one labelled with the
//! operation that produced its own value.
//!
//! - [`construct`] — `add_data_node*`, `add_named_data_node*` and `measure_*`.
//! - [`draw`] — measuring the cells, sizing the box to fit them, and rendering the grid.
//! - [`style`] — each cell's fill, stroke and dash under a selection.
//! - [`selection`] — `set_selection` and `set_secondary_selection`.
//! - [`unreached`] — `set_unreached_cells`.
//! - [`values`] — `cell_rect` and `set_data_values`.

use crate::error::Error;

mod construct;
mod draw;
mod selection;
mod style;
mod unreached;
mod values;

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Returns [`Error::EmptyNodeName`] if `name` is empty or holds only whitespace. See that variant's own doc comment for
/// why a blank name is rejected outright, rather than merely drawing an oddly-worded label. Shared by every path that
/// draws a named data node — construction and measurement alike.
pub(super) fn validate_node_name(name: &str) -> Result<(), Error> {
    if name.trim().is_empty() {
        return Err(Error::EmptyNodeName);
    }
    Ok(())
}
