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
    /// `values` must be the same width of integer, and the same number of values, as the node was drawn with. The grid
    /// then keeps exactly the size, shape and position it already has, and so does every connector attached to it. The
    /// node's own selection, secondary cells and colours are untouched. Each cell keeps the width it was drawn with. So
    /// a [`crate::model::content::DataFormat::Decimal`] value with more digits than any value shown when the node was
    /// drawn will overflow its cell. [`crate::model::content::DataFormat::Hexadecimal`] and
    /// [`crate::model::content::DataFormat::Binary`] values never change width.
    ///
    /// For a node drawn via [`add_data_node`](Self::add_data_node) or [`add_named_data_node`](Self::add_named_data_node)
    /// and their `_with` variants. A single-value node is replaced too. Its own accessible name and tooltip quote the
    /// value, so those are rewritten as well, keeping every relationship and selection clause after it. An operator
    /// node's own result is rejected. Its value is computed by the caller when the node is added, and there is no cell
    /// of its own to rewrite.
    ///
    /// # Errors
    ///
    /// Returns [`Error::UnknownNode`] if `id` does not name a node in this scene.
    ///
    /// Returns [`Error::IncompatibleNodeValues`] if `id` is not a data node drawn with cells of its own, or if `values`
    /// has a different integer width or a different number of values. Checked before changing anything.
    ///
    /// Also returns a wrapped [`Error::Svg`] if rewriting a cell fails partway through, which can leave some cells
    /// already rewritten.
    pub fn set_data_values(&self, id: NodeId, values: NodeValues) -> Result<(), Error> {
        let mut inner = self.inner.borrow_mut();

        // Everything that can reject the call is checked before anything changes. Only a node drawn from a
        // `DataNodeContent` can be replaced. An operator node's own result is worked out by the caller when the node is
        // added.
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

        // The old text of a single value, kept to find it again in the node's own accessible name below.
        let single_value = content.is_single_value();
        let mut old_text = String::new();
        if single_value {
            content.single_cell_string_into(&mut old_text);
        }
        let (_, cols) = content.shape();
        let labelling = content.labelling();
        let type_name = content.type_name();

        // Only a cell whose value changed is called back, and only it is rewritten. The first DOM failure is kept, and
        // later cells are left alone.
        let mut scratch = String::new();
        let mut name_scratch = String::new();
        let mut failure: Option<svg_dom::Error> = None;
        let mut new_text: Option<String> = None;
        let accepted = content.replace_values(values, &mut scratch, |i, text| {
            if failure.is_some() {
                return;
            }
            let Some(cell) = handles.cell_texts.get(i) else { return };
            cell.set_text(text);
            // A grid's cell is named by its row and column, matching what `draw_content_box` gave it. A single value's
            // text has no such name: the node's own accessible name quotes the value instead.
            if single_value {
                new_text = Some(text.to_owned());
            } else {
                cell_name_into(labelling, i, cols, text, &mut name_scratch);
                if let Err(e) = cell.set_attr("aria-label", &name_scratch) {
                    failure = Some(e);
                }
            }
        });
        if !accepted {
            return Err(Error::IncompatibleNodeValues(id));
        }
        if let Some(e) = failure {
            return Err(e.into());
        }
        if let Some(new_text) = new_text {
            handles.replace_label_value(type_name, &old_text, &new_text)?;
        }
        Ok(())
    }
}
