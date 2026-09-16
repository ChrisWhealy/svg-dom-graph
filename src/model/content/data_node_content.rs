use super::*;

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// A node's content: a set of typed numeric values, displayed as a grid as close to square as the value count allows —
/// see this module's own doc comment for the exact layout, formatting, and colouring rules.
///
/// Build one with [`DataNodeContent::new`]. Unlike [`NodeOptions`](crate::scene::NodeOptions)/
/// [`DragOptions`](crate::scene::DragOptions), there is no sensible all-default state to build one on top of — the
/// values are mandatory — so this is a plain constructor rather than a `default()` plus `with_*` builder.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DataNodeContent {
    values: NodeValues,
    format: DataFormat,
    layout: GridLayout,
    byte_order: ByteOrder,
}

impl DataNodeContent {
    /// Builds a [`DataNodeContent`] displaying `values`, formatted as `format`. This arranges the grid per
    /// [`GridLayout::Automatic`] — see [`with_layout`](Self::with_layout) to override that. It orders bytes per
    /// [`ByteOrder::BigEndian`] — see [`with_byte_order`](Self::with_byte_order) to override that.
    ///
    /// An empty `values` is accepted here. This is the same deferred-validation convention
    /// [`DragOptions::with_bounds`](crate::scene::DragOptions::with_bounds)/
    /// [`NodeOptions::with_edge_anchors`](crate::scene::NodeOptions::with_edge_anchors) already follow. It is rejected
    /// instead by [`Scene::add_data_node_with`](crate::scene::Scene::add_data_node_with), with
    /// [`Error::EmptyNodeContent`](crate::error::Error::EmptyNodeContent) — the only place that actually needs a grid
    /// to draw.
    #[must_use]
    pub fn new(values: NodeValues, format: DataFormat) -> Self {
        Self {
            values,
            format,
            layout: GridLayout::default(),
            byte_order: ByteOrder::default(),
        }
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    /// Returns `self` with `layout` overriding [`GridLayout::Automatic`]'s own cell-count-only rule. See
    /// [`GridLayout`]'s own doc comment for what each variant does. See this module's own doc comment ("Grid shape")
    /// for why `Automatic` alone is not always enough.
    ///
    /// `layout`'s own `Columns`/`Rows`/`MaxColumns` value is accepted here even if `0`. This is the same
    /// deferred-validation convention `new`'s own doc comment describes. It is rejected instead by
    /// [`Scene::add_data_node_with`](crate::scene::Scene::add_data_node_with),
    /// with [`Error::InvalidGridLayout`](crate::error::Error::InvalidGridLayout).
    #[must_use]
    pub fn with_layout(mut self, layout: GridLayout) -> Self {
        self.layout = layout;
        self
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    /// Returns `self` with `byte_order` overriding [`ByteOrder::BigEndian`]'s own default. See [`ByteOrder`]'s own doc
    /// comment for what each variant does. See this module's own doc comment ("Formatting") for when `LittleEndian` is
    /// the right choice.
    #[must_use]
    pub fn with_byte_order(mut self, byte_order: ByteOrder) -> Self {
        self.byte_order = byte_order;
        self
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    /// How many values this holds.
    pub(crate) fn len(&self) -> usize {
        self.values.len()
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    /// `true` for exactly one value.
    ///
    /// A single value has no sibling to be told apart from. So `draw_content_box` skips the per-value inner box
    /// [`NodeValues::type_color`] would otherwise use. It applies that colour straight to the node's own single box
    /// instead. That gives one box, one colour, and no redundant box-within-a-box.
    pub(crate) fn is_single_value(&self) -> bool {
        self.values.len() == 1
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    /// This content's own [`GridLayout`] — `Automatic` unless [`with_layout`](Self::with_layout) overrode it.
    pub(crate) fn layout(&self) -> GridLayout {
        self.layout
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    /// The `(rows, cols)` grid this content renders as — see [`grid_shape`].
    pub(crate) fn shape(&self) -> (usize, usize) {
        grid_shape(self.values.len(), self.layout)
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    /// Every value's own formatted cell text, in the same order they were supplied. One string per value, ready for
    /// `draw_content_box` to place one at a time into the grid [`DataNodeContent::shape`] describes.
    pub(crate) fn cells(&self) -> Vec<String> {
        self.values.cell_strings(self.format, self.byte_order)
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    /// The pastel colour identifying this content's own value type — see [`NodeValues::type_color`].
    pub(crate) fn type_color(&self) -> &'static str {
        self.values.type_color()
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    /// This content's own type name ("u8"/"u16"/"u32"/"u64") — see [`NodeValues::type_name`].
    pub(crate) fn type_name(&self) -> &'static str {
        self.values.type_name()
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    /// Resolves `selection` against this content's own actual value count and grid shape into the flat cell
    /// indices [`Scene::set_selection`](crate::scene::Scene::set_selection) should recolour.
    ///
    /// Returns `(band, focus)`. Every index in `band` gets the row/column-level highlight colour.
    ///
    /// The further index in `focus`, if any, gets the stronger cell-level colour instead, overriding `band` for
    /// that one cell.
    ///
    /// Returns `None` if `selection` names an index out of range for this content's own value count
    /// ([`Selection::Cell`]) or grid shape ([`Selection::Row`]/[`Selection::Column`], and their own optional
    /// `col`/`row`).
    pub(crate) fn resolve_selection(&self, selection: Selection) -> Option<(Vec<usize>, Option<usize>)> {
        let len = self.len();
        let (rows, cols) = self.shape();

        match selection {
            Selection::None => Some((Vec::new(), None)),
            Selection::Cell(i) => (i < len).then(|| (Vec::new(), Some(i))),
            Selection::Row { row, col } => {
                if row >= rows || col.is_some_and(|c| c >= cols) {
                    return None;
                }
                let band = (0..cols).map(|c| row * cols + c).collect();
                Some((band, col.map(|c| row * cols + c)))
            },
            Selection::Column { col, row } => {
                if col >= cols || row.is_some_and(|r| r >= rows) {
                    return None;
                }
                let band = (0..rows).map(|r| r * cols + col).collect();
                Some((band, row.map(|r| r * cols + col)))
            },
        }
    }
}
