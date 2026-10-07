use super::*;

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// A node's content: a set of typed numeric values, displayed as a grid as close to square as the value count allows.
/// See this module's own doc comment for the exact layout, formatting, and colouring rules.
///
/// Build one with [`DataNodeContent::new`]. Unlike [`NodeOptions`](crate::scene::NodeOptions)/
/// [`DragOptions`](crate::scene::DragOptions), there is no sensible all-default state to build one on top of, because
/// the values are mandatory. So this is a plain constructor rather than a `default()` plus `with_*` builder.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DataNodeContent {
    values: NodeValues,
    format: DataFormat,
    layout: GridLayout,
    byte_order: ByteOrder,
    column_group: usize,
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
            column_group: 0,
        }
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    /// Returns `self` with `layout` overriding [`GridLayout::Automatic`]'s own cell-count-only rule. See
    /// [`GridLayout`]'s own doc comment for what each variant does. See this module's own doc comment ("Grid shape")
    /// for why `Automatic` alone is not always enough.
    ///
    /// `layout`'s own `Columns`/`Rows`/`MaxColumns` value is accepted here even if `0`. This is the same
    /// deferred-validation convention `new`'s own doc comment describes. It is rejected instead by
    /// [`Scene::add_data_node_with`](crate::scene::Scene::add_data_node_with), with
    /// [`Error::InvalidGridLayout`](crate::error::Error::InvalidGridLayout).
    #[must_use]
    pub fn with_layout(mut self, layout: GridLayout) -> Self {
        self.layout = layout;
        self
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    /// Returns `self` with a slightly wider gap after every `columns` columns, so the grid reads as groups of `columns`
    /// cells. A hex-dump style `16` bytes per row with a wider gap after the eighth is, for example, `Columns(16)` plus
    /// `with_column_groups(8)`.
    ///
    /// Only the horizontal spacing changes. The grid keeps the shape [`GridLayout`] gives it, cells still index in the
    /// same row-major order, and a row or column selection still highlights the same cells. The gap is between groups
    /// only, never after the last one. `0` — the default — draws no groups.
    #[must_use]
    pub fn with_column_groups(mut self, columns: usize) -> Self {
        self.column_group = columns;
        self
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    /// How many columns make up one group — see [`with_column_groups`](Self::with_column_groups); `0` for none.
    pub(crate) fn column_group(&self) -> usize {
        self.column_group
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    /// Returns `self` with `byte_order` overriding [`ByteOrder::BigEndian`]'s own default. See [`ByteOrder`]'s own doc
    /// comment for what each variant does. See this module's own doc comment ("Formatting") for when `LittleEndian` is
    /// the right choice.
    #[must_use]
    pub fn with_byte_order(mut self, byte_order: ByteOrder) -> Self {
        self.byte_order = byte_order;
        self
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    /// Replaces this content's own values with `values`, keeping its format, layout and byte order.
    ///
    /// Returns `false`, changing nothing, unless `values` is the same integer width and holds the same number of
    /// values. So the grid keeps exactly the same shape it was drawn with.
    pub(crate) fn replace_values(&mut self, values: NodeValues) -> bool {
        if std::mem::discriminant(&self.values) != std::mem::discriminant(&values) || self.values.len() != values.len()
        {
            return false;
        }
        if self.format == DataFormat::PlainText && !is_printable_ascii(&values) {
            return false;
        }
        self.values = values;
        true
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    /// How many values this holds. [`DataFormat::PlainText`] content is one value, however many characters it has.
    pub(crate) fn len(&self) -> usize {
        if self.is_plain_text() {
            usize::from(self.values.len() > 0)
        } else {
            self.values.len()
        }
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    /// `true` for [`DataFormat::Ascii`] content.
    pub(crate) fn is_ascii(&self) -> bool {
        self.format == DataFormat::Ascii
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    /// `true` for [`DataFormat::PlainText`] content.
    pub(crate) fn is_plain_text(&self) -> bool {
        self.format == DataFormat::PlainText
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    /// `false` for [`DataFormat::PlainText`] content that is not `u8` or holds a byte outside `0x20..=0x7E`. Every other
    /// content is valid here.
    pub(crate) fn plain_text_is_valid(&self) -> bool {
        !self.is_plain_text() || is_printable_ascii(&self.values)
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    /// Writes the bytes of `PlainText` content into `out` as characters.
    fn plain_text_into(&self, out: &mut String) {
        out.clear();
        if let NodeValues::U8(v) = &self.values {
            out.extend(v.iter().map(|&b| char::from(b)));
        }
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    /// `true` for exactly one value.
    ///
    /// A single value has no sibling to be told apart from. So `draw_content_box` skips the per-value inner box
    /// [`NodeValues::type_colour`] would otherwise use. It applies that colour straight to the node's own single box
    /// instead. That gives one box, one colour, and no redundant box-within-a-box.
    pub(crate) fn is_single_value(&self) -> bool {
        self.len() == 1
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    /// This content's own [`GridLayout`] — `Automatic` unless [`with_layout`](Self::with_layout) overrode it.
    pub(crate) fn layout(&self) -> GridLayout {
        self.layout
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    /// The `(rows, cols)` grid this content renders as — see [`grid_shape`].
    pub(crate) fn shape(&self) -> (usize, usize) {
        if self.is_plain_text() {
            return (1, 1);
        }
        grid_shape(self.len(), self.layout)
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    /// Calls `f(index, formatted)` once for every value, in order, formatted per this content's own `format`/
    /// `byte_order` — one cell per value, not yet arranged into a grid (see [`shape`](Self::shape) for that).
    ///
    /// `draw_content_box` streams a data node's own cells through this one at a time, however many values it holds. It
    /// does not collect every formatted value into a `Vec<String>` up front. Nor does it collect the `Vec<SvgNode>` of
    /// `<text>` elements, one per element, it would otherwise take to render them. See
    /// [`NodeValues::for_each_cell_string`]'s own doc comment for the reused-buffer shape this passes straight through.
    pub(crate) fn for_each_cell_string(&self, scratch: &mut String, mut f: impl FnMut(usize, &str)) {
        if self.is_plain_text() {
            if self.len() == 1 {
                self.plain_text_into(scratch);
                f(0, scratch);
            }
            return;
        }
        self.values.for_each_cell_string(self.format, self.byte_order, scratch, f);
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    /// Formats this content's own single value into caller-owned `out` — `false`, leaving `out` untouched, if it holds
    /// no values at all.
    ///
    /// For `draw_operator_box`'s own already-validated single-value result. An operator always produces exactly one
    /// value, never a grid. So this never needs [`for_each_cell_string`](Self::for_each_cell_string)'s per-value
    /// iteration just to reach the one string it would ever visit. Nor does it allocate a fresh `String`:
    /// `draw_operator_box` passes its own construction-scratch buffer as `out`.
    pub(crate) fn single_cell_string_into(&self, out: &mut String) -> bool {
        if self.is_plain_text() {
            self.plain_text_into(out);
            return self.len() == 1;
        }
        self.values.single_cell_string_into(self.format, self.byte_order, out)
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    /// Formats into `out` whichever one value is guaranteed to render this content's own widest cell, under one
    /// monospace font. It does so without formatting every value first. See [`NodeValues::widest_cell_string`]'s own
    /// doc comment for how that value is chosen. Leaves `out` empty if this content holds no values at all.
    pub(crate) fn widest_cell_string(&self, out: &mut String) {
        if self.is_plain_text() {
            self.plain_text_into(out);
            return;
        }
        self.values.widest_cell_string(self.format, self.byte_order, out);
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    /// The pastel colour identifying this content's own value type — see [`NodeValues::type_colour`].
    pub(crate) fn type_colour(&self) -> &'static str {
        if self.is_plain_text() {
            return crate::colours::PLAIN_BOX_FILL;
        }
        self.values.type_colour()
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    /// This content's own type name ("u8"/"u16"/"u32"/"u64") — see [`NodeValues::type_name`].
    pub(crate) fn type_name(&self) -> &'static str {
        if self.is_plain_text() {
            return "text";
        }
        self.values.type_name()
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    /// Resolves `selection` against this content's own actual value count and grid shape into the `(band, focus)`
    /// [`Scene::set_selection`](crate::scene::Scene::set_selection) should recolour with.
    ///
    /// Every cell [`ResolvedBand::contains`] gets the row/column-level highlight colour. The further flat index in
    /// `focus`, if any, gets the stronger cell-level colour instead, overriding `band` for that one cell.
    ///
    /// Returns `None` if `selection` names an index out of range for this content's own value count
    /// ([`Selection::Cell`]) or grid shape ([`Selection::Row`]/[`Selection::Column`], and their own optional
    /// `col`/`row`).
    ///
    /// A row/column index within `rows`/`cols` is not always enough on its own. [`GridLayout::Automatic`] (and an
    /// over-specified [`GridLayout::Rows`]/[`GridLayout::Columns`]) can legitimately leave the grid's own last row or
    /// column short, or entirely empty, whenever this content's own value count does not divide evenly.
    ///
    /// Seven values arranged as a 3×3 grid, for example, leaves flat indices `7`/`8` with no real value at all. `focus`
    /// is therefore checked against this content's own actual `len`, not just against the grid's own row/column
    /// *shape*. `band`'s own arithmetic membership test never needs the same check. `Scene::set_selection` only ever
    /// queries it with a flat index already known to be real. See [`ResolvedBand`]'s own doc comment for why.
    pub(crate) fn resolve_selection(&self, selection: Selection) -> Option<(ResolvedBand, Option<usize>)> {
        let len = self.len();
        let (rows, cols) = self.shape();

        match selection {
            Selection::None => Some((ResolvedBand::None, None)),
            Selection::Cell(i) => (i < len).then_some((ResolvedBand::None, Some(i))),
            Selection::Row { row, col } => {
                if row >= rows || col.is_some_and(|c| c >= cols) {
                    return None;
                }
                let focus = col.map(|c| row * cols + c);
                if focus.is_some_and(|f| f >= len) {
                    return None;
                }
                Some((ResolvedBand::Row { row, cols }, focus))
            },
            Selection::Column { col, row } => {
                if col >= cols || row.is_some_and(|r| r >= rows) {
                    return None;
                }
                let focus = row.map(|r| r * cols + col);
                if focus.is_some_and(|f| f >= len) {
                    return None;
                }
                Some((ResolvedBand::Column { col, cols }, focus))
            },
        }
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    /// The walk at position (`flat_index`) shown by a data-node selection toolbar's [`Selection`]. It is
    /// [`Selection::Cell`] for a one-dimensional grid, meaning a single row or a single column (see [`Selection`]'s own
    /// doc comment for why that case only ever needs `Cell`). It is [`Selection::Row`] with a `col` for any other
    /// shape.
    ///
    /// Returns `None` if `flat_index` is out of range for this content's own actual value count.
    /// [`flat_index`](Self::flat_index) is the inverse of this, over the selections it can produce. See its own doc
    /// comment for why "over the selections it can produce" rather than "over `Selection`'s whole domain."
    #[must_use]
    pub fn natural_selection(&self, flat_index: usize) -> Option<Selection> {
        if flat_index >= self.len() {
            return None;
        }
        let (rows, cols) = self.shape();
        if rows <= 1 || cols <= 1 {
            return Some(Selection::Cell(flat_index));
        }
        Some(Selection::Row {
            row: flat_index / cols,
            col: Some(flat_index % cols),
        })
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    /// The inverse of [`natural_selection`](Self::natural_selection): the flat index `selection` names, if `selection`
    /// is exactly the shape `natural_selection` would produce for this content's own current grid shape.
    ///
    /// Returns `None` for any other `Selection` — a [`Selection::Column`], a [`Selection::Row`] with no `col`, a
    /// [`Selection::Cell`] on a genuinely two-dimensional shape, or an index out of range. Not only when `selection` is
    /// invalid, but also whenever this content's own current shape would never have produced it through
    /// [`natural_selection`](Self::natural_selection) itself. This is deliberately narrower than "any selection that
    /// resolves to a real cell." A data-node selection toolbar built on top of this treats that `None` exactly like its
    /// own unstarted state — see [`Scene::show_selection_toolbar`](crate::scene::Scene::show_selection_toolbar)'s own
    /// doc comment.
    #[must_use]
    pub fn flat_index(&self, selection: &Selection) -> Option<usize> {
        let (rows, cols) = self.shape();
        let one_dimensional = rows <= 1 || cols <= 1;
        let index = match (*selection, one_dimensional) {
            (Selection::Cell(i), true) => i,
            (Selection::Row { row, col: Some(col) }, false) => row * cols + col,
            _ => return None,
        };
        (index < self.len()).then_some(index)
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// `true` if `values` is `u8` and every byte is printable ASCII, `0x20..=0x7E`.
fn is_printable_ascii(values: &NodeValues) -> bool {
    matches!(values, NodeValues::U8(v) if v.iter().all(|b| (0x20..=0x7E).contains(b)))
}
