//! Marking a data node's cells that have no value yet: `Scene::set_unreached_cells`.

use super::selection::{Highlight, try_for_each_difference};
use crate::{
    error::Error,
    model::node::{NodeContent, NodeId},
    scene::{Scene, Selection},
};

impl Scene {
    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    /// Marks the cells at flat indices `cells` of node `id` as *not yet computed*, and draws them faint.
    ///
    /// A walk through an algorithm fills its arrays in step by step. Until a cell's step is reached it holds a
    /// placeholder, usually zero. A computed zero looks exactly the same. Marking the cells not yet reached tells the two
    /// apart. When the walk reaches a cell, take it out of `cells` and write its value with
    /// [`set_data_values`](Self::set_data_values).
    ///
    /// # Independent of the other marks
    ///
    /// This never reads or changes `id`'s own [`Selection`] or its secondary cells, and neither of those changes it. A
    /// cell that is selected, in a selected row or column, or secondary shows that mark at full strength, whether or not
    /// it is in `cells`. When the mark moves away, a cell still in `cells` goes faint again. So a walk can mark the cell
    /// it is standing on without first removing it from `cells`.
    ///
    /// # Rendering
    ///
    /// A cell in `cells` keeps its own colour and border, and is drawn with its box and its digits at reduced opacity.
    /// Opacity alone is not a reliable channel, so the node's own `aria-label` and tooltip gain `", not yet computed:
    /// cells 16 to 63"`. A run of three or more consecutive cells reads as a range. A node with a single cell reads just
    /// `", not yet computed"`.
    ///
    /// # Hex dumps
    ///
    /// A [`DataFormat::HexDump`](crate::scene::DataFormat::HexDump) has a cell for each line, but `cells` names its
    /// *bytes*, as [`Selection::Cell`] does. An unreached byte is dimmed over its two hex digits and over its character.
    ///
    /// # Replacement
    ///
    /// `cells` replaces whatever was marked before. An empty slice clears it. Order and duplicates do not matter. An
    /// identical set to the current one is an immediate no-op, and otherwise only the cells that enter or leave the set
    /// are redrawn.
    ///
    /// # Errors
    ///
    /// Returns [`Error::UnknownNode`] if `id` does not name a node in this scene.
    ///
    /// Returns [`Error::InvalidSelection`], carrying the offending index as a [`Selection::Cell`], if `id` names a plain
    /// label or container node. It also returns it if any index in `cells` is out of range for `id`'s own value count.
    /// Checked before redrawing any cell, so a rejected call leaves every cell exactly as it was.
    ///
    /// Also returns a wrapped [`Error::Svg`] if redrawing a cell fails partway through, with the same "can leave some
    /// cells already redrawn" property [`set_selection`](Self::set_selection) documents.
    pub fn set_unreached_cells(&self, id: NodeId, cells: &[usize]) -> Result<(), Error> {
        let mut inner = self.inner.borrow_mut();

        let content = match &inner.graph.node(id).ok_or(Error::UnknownNode(id))?.content {
            NodeContent::Data(content) => content,
            NodeContent::Label(_)
            | NodeContent::Container(_)
            | NodeContent::Markdown(_)
            | NodeContent::TextInput(_) => {
                return Err(Error::InvalidSelection(id, Selection::None));
            },
        };
        // A hex dump is marked a byte at a time, as it is selected.
        let len = content.selectable_len();
        if let Some(&bad) = cells.iter().find(|&&i| i >= len) {
            return Err(Error::InvalidSelection(id, Selection::Cell(bad)));
        }
        let dumped = content.hex_dump().map(|_| len);
        let base_colour = content.type_colour();
        let highlight = {
            let selection = inner.node_handle(id).ok_or(Error::UnknownNode(id))?.selection;
            // `selection` was accepted by an earlier call against this same content, so it always resolves.
            Highlight::resolve(content, selection).unwrap_or_default()
        };

        // The same indices again, already sorted and without duplicates, change nothing. That is checked before the
        // copy below, which still normalises anything else.
        let handles = inner.node_handle_mut(id).ok_or(Error::UnknownNode(id))?;
        if handles.unreached == cells {
            return Ok(handles.resync_label()?);
        }

        let mut new_unreached = cells.to_vec();
        new_unreached.sort_unstable();
        new_unreached.dedup();

        // A dump has no cell for a byte, so its bytes are dimmed by blocks drawn over its lines. See `HexMarks`.
        if let Some(len) = dumped {
            let inner = &mut *inner;
            let handles = inner.node_handles.get_mut(id.index).ok_or(Error::UnknownNode(id))?;
            if handles.unreached == new_unreached {
                return Ok(handles.resync_label()?);
            }
            if let (Some(marks), Some(grid)) = (&mut handles.decor.hex_marks, handles.cell_grid) {
                marks.show_unreached(&inner.svg, &grid, len, &new_unreached, base_colour)?;
                handles.unreached = new_unreached;
                handles.refresh_label()?;
                return Ok(());
            }
        }

        let handles = inner.node_handle_mut(id).ok_or(Error::UnknownNode(id))?;
        if handles.unreached == new_unreached {
            return Ok(handles.resync_label()?);
        }
        let cell_stroke_width = handles.cell_stroke_width;
        let single_value = handles.cell_grid.is_some_and(|grid| grid.single_value);

        // Only a cell that enters or leaves the set can change style, so walk the two sorted lists' own symmetric
        // difference rather than every cell in the grid.
        try_for_each_difference(
            &handles.unreached,
            &new_unreached,
            |i, now_in_set| -> Result<(), svg_dom::Error> {
                let Some(cell) = handles.cell_rects.get(i) else { return Ok(()) };
                let secondary = handles.secondary.binary_search(&i).is_ok();
                let style_with =
                    |unreached: bool| highlight.style(i, secondary, unreached, base_colour, cell_stroke_width);
                let new_style = style_with(now_in_set);
                let old_style = style_with(!now_in_set);
                if new_style != old_style {
                    new_style.apply_from(old_style, single_value, cell, handles.cell_texts.get(i))?;
                }
                Ok(())
            },
        )?;

        handles.unreached = new_unreached;
        handles.refresh_label()?;
        Ok(())
    }
}
