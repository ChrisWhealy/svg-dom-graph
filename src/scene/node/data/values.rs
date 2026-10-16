//! Reading and replacing a data node's cells: `Scene::cell_rect` and `Scene::set_data_values`.

use super::draw::cell_name_into;
use crate::{
    error::Error,
    model::node::{NodeContent, NodeId},
    scene::{NodeValues, Scene, Selection, scene_inner::SceneInner},
};
use svg_dom::root::utils::{Point, Rect};

impl Scene {
    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    /// The rectangle of cell `index` of data node `id`, in this scene's own coordinates, where `index` is the cell's
    /// flat row-major position. A single-value node's one cell is its whole content box.
    ///
    /// Reflects the node's own current position, so it follows a drag or a [`move_node`](Self::move_node). A caller can
    /// use it to line something up with one column of a grid, say. Connectors still land only on a node's own outer
    /// perimeter.
    ///
    /// # Errors
    ///
    /// Returns [`Error::UnknownNode`] if `id` does not name a node in this scene. Returns [`Error::InvalidSelection`],
    /// naming `Selection::Cell(index)`, if `id` is not a data node or `index` is out of range for it.
    pub fn cell_rect(&self, id: NodeId, index: usize) -> Result<Rect, Error> {
        let inner = self.inner.borrow();
        let origin = inner.node_rect(id)?.origin;
        let local = inner
            .node_handle(id)
            .and_then(|handles| handles.cell_grid.as_ref())
            .and_then(|grid| grid.cell_rect(index))
            .ok_or(Error::InvalidSelection(id, Selection::Cell(index)))?;
        Ok(Rect {
            origin: Point::new(origin.x + local.origin.x, origin.y + local.origin.y),
            size: local.size,
        })
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    /// Replaces the values shown by data node `id` with `values`, rewriting each cell's own text in place.
    ///
    /// For a node whose content depends on a step the host walks through, such as a function's own output array. It is
    /// initialised to zeros until the walk reaches the step that produces it. Without this, a host could only draw such
    /// a node once, with whatever it held at that moment.
    ///
    /// Only cells whose value changed are formatted and rewritten. Replacing a node's values with the ones it already
    /// holds does nothing to the document, so a caller can pass the whole state each step without the unchanged part
    /// costing anything.
    ///
    /// `values` must be the same type, and the same number of values, as the node was drawn with. An `I16` node cannot
    /// be given `U16` or `F32` values, nor an `F32` node `F64` ones. The grid
    /// then keeps exactly the size, shape and position it already has, and so does every connector attached to it. The
    /// node's own selection, secondary cells and colours are untouched. Each cell keeps the width it was drawn with. So
    /// a [`crate::model::content::DataFormat::Decimal`] integer with more digits than any value shown when the node was
    /// drawn will overflow its cell. So will a float that needs more characters under its format: more integer digits, a
    /// longer exponent, a minus sign, or `-inf`. [`crate::model::content::DataFormat::Hexadecimal`] and
    /// [`crate::model::content::DataFormat::Binary`] values never change width.
    ///
    /// For a node drawn via [`add_data_node`](Self::add_data_node) or [`add_named_data_node`](Self::add_named_data_node)
    /// and their `_with` variants. A single-value node is replaced too. Its own accessible name and tooltip quote the
    /// value, so those are rewritten as well, keeping every relationship and selection clause after it. An operator
    /// node's own result is replaced the same way. The library never evaluates an operator, so the caller works out
    /// the new result just as it did when it added the node. Nothing downstream of the node is changed: a node that
    /// takes this one as an operand keeps the result it was given.
    ///
    /// # Errors
    ///
    /// Returns [`Error::UnknownNode`] if `id` does not name a node in this scene.
    ///
    /// Returns [`Error::IncompatibleNodeValues`] if `id` is not a data node or an operator node, or if `values`
    /// has a different type or a different number of values. Checked before changing anything.
    ///
    /// Also returns a wrapped [`Error::Svg`] if rewriting a cell fails partway through, which can leave some cells
    /// already rewritten. The node's own values stay as they were then, so calling this again with the same `values`
    /// rewrites every cell that differs from them, including those already written, and repairs the node.
    pub fn set_data_values(&self, id: NodeId, values: NodeValues) -> Result<(), Error> {
        let mut inner = self.inner.borrow_mut();

        // Everything that can reject the call is checked before anything changes. Only a node drawn from a
        // `DataNodeContent`, or an operator node, whose result is one too, can be replaced.
        //
        // `graph` and `node_handles` are separate fields, so the content and the cells' own `<text>` elements are
        // borrowed side by side. Each cell is then rewritten as it is formatted, with no list of texts built first.
        let SceneInner { graph, node_handles, .. } = &mut *inner;
        let node = graph.node_mut(id).ok_or(Error::UnknownNode(id))?;
        let NodeContent::Data(content) = &mut node.content else {
            return Err(Error::IncompatibleNodeValues(id));
        };
        let handles = node_handles.get_mut(id.index).ok_or(Error::UnknownNode(id))?;
        if !handles.replaceable {
            return Err(Error::IncompatibleNodeValues(id));
        }

        // The old text of a single value, kept to find it again in the node's own accessible name below. A replacement
        // that changes nothing never writes the name, so it formats nothing.
        let single_value = content.is_single_value();
        let icon = content.is_bool_icon();
        let mut old_text = String::new();
        if single_value && content.differs_from(&values) {
            content.single_cell_string_into(&mut old_text);
            // The accessible name quotes what is said, and a mark is said as the word. Every other value is said as it
            // is written, so its text is kept as it is and nothing is allocated.
            if icon {
                old_text = crate::scene::DataNodeContent::spoken_for(icon, &old_text).to_owned();
            }
        }
        let (_, cols) = content.shape();
        let labelling = content.labelling();
        let type_name = content.type_name();

        // Only a cell whose value changed is called back, and only it is rewritten. The model commits the new values
        // only once every cell has been written. A failure part way therefore leaves the old values in the model, so
        // retrying with the same values rewrites every cell again, those already written included.
        let mut scratch = String::new();
        let mut name_scratch = String::new();
        let accepted = content.try_replace_values(values, &mut scratch, |i, text| -> Result<(), svg_dom::Error> {
            let Some(cell) = handles.cell_texts.get(i) else { return Ok(()) };
            cell.set_text(text);
            // A mark's colour follows its shape, and its accessible name says the word and not the mark. No other text
            // ever changes colour, so only a mark is given a fill, and every other cell is written no more than before.
            if icon {
                cell.set_fill(crate::scene::DataNodeContent::ink_for(icon, text))?;
            }
            let text = crate::scene::DataNodeContent::spoken_for(icon, text);
            if single_value {
                // A single value's text has no name of its own: the node's own accessible name quotes the value
                // instead, and is part of the same write.
                handles.replace_label_value(type_name, &old_text, text)
            } else {
                // A grid's cell is named by its row and column, matching what `draw_content_box` gave it.
                cell_name_into(labelling, i, cols, text, &mut name_scratch);
                cell.set_attr("aria-label", &name_scratch)
            }
        })?;
        if !accepted {
            return Err(Error::IncompatibleNodeValues(id));
        }
        // The marks over a selected byte of a hex dump show what the byte was, so they are written again.
        if let (Some(marks), Some(grid), Selection::Cell(i)) =
            (&handles.decor.hex_marks, handles.cell_grid, handles.selection)
        {
            marks.show(&mut String::new(), &grid, content.hex_dump_byte(i))?;
        }
        Ok(())
    }
}
