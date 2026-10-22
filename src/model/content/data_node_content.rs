use super::{
    bool_format::{ICON_FALSE, ICON_TRUE},
    *,
};
use crate::colours::{BOOL_FALSE_INK, BOOL_TRUE_INK, TEXT_FILL};

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
    labelling: Option<LabellingStyle>,
    toolbar: bool,
    cell_width_hint: Option<String>,
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
            labelling: None,
            toolbar: false,
            cell_width_hint: None,
        }
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    /// Returns `self` with a row reserved under its name for a selection toolbar, inside the node's own box.
    ///
    /// Only a named node (see [`Scene::add_named_data_node`](crate::scene::Scene::add_named_data_node)) has a box
    /// round its content to hold one, so this has no effect on an unnamed node. The row is empty until
    /// [`Scene::show_selection_toolbar`](crate::scene::Scene::show_selection_toolbar) is called for the node. That
    /// call then puts its bar in the row, where it moves, scales and is hidden with the node, and not at the edge of the
    /// scene as it otherwise would be.
    #[must_use]
    pub fn with_toolbar(mut self) -> Self {
        self.toolbar = true;
        self
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    /// Returns `self` with every cell at least as wide as `sample` is when drawn, so that a node whose strings are
    /// replaced later does not outgrow its cells. See [`NodeValues::Text`] for why a string's width is not fixed.
    ///
    /// The width of the cells is the widest of the measured widths of `sample` and of every string the node holds,
    /// measured in the cell's own font. So `sample` sets a floor, and a longer string still gets the room it needs. A
    /// caller who wants a width that never changes chooses `sample` as the widest string the node will ever show, and
    /// measures nothing itself. `sample` is only measured. It is never drawn, never counted as a value, and never part of
    /// the node's accessible name. It has an effect on [`DataFormat::Text`] content only, and is checked like a cell: a
    /// line break or another control character in it is rejected with
    /// [`Error::InvalidTextContent`](crate::error::Error::InvalidTextContent).
    #[must_use]
    pub fn with_cell_width_hint(mut self, sample: &str) -> Self {
        self.cell_width_hint = Some(sample.to_owned());
        self
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    /// The sample string [`with_cell_width_hint`](Self::with_cell_width_hint) gave, if any.
    pub(crate) fn cell_width_hint(&self) -> Option<&str> {
        self.cell_width_hint.as_deref()
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    /// The strings of [`DataFormat::Text`] content, and none for any other.
    pub(crate) fn text_cells(&self) -> Option<&[String]> {
        self.values.text().filter(|_| self.is_text())
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    /// `true` for [`DataFormat::Text`] content.
    pub(crate) fn is_text(&self) -> bool {
        self.format == DataFormat::Text
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    /// `false` unless [`DataFormat::Text`] and [`NodeValues::Text`] go together, every string is one line with no control
    /// character in it, and so is any cell-width hint. Every content that has no text is valid here.
    pub(crate) fn text_is_valid(&self) -> bool {
        if self.is_text() != self.values.is_text() {
            return false;
        }
        let one_line = |text: &String| !text.chars().any(char::is_control);
        self.values.text().is_none_or(|cells| cells.iter().all(one_line))
            && self.cell_width_hint.as_ref().is_none_or(one_line)
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    /// Whether [`with_toolbar`](Self::with_toolbar) reserved a row for a selection toolbar.
    pub(crate) fn has_toolbar_row(&self) -> bool {
        self.toolbar
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    /// How many positions a selection walks through. Every cell, except that a [`DataFormat::HexDump`] is walked a byte
    /// at a time, and not a line.
    pub(crate) fn selectable_len(&self) -> usize {
        match (self.hex_dump(), &self.values) {
            (Some(_), NodeValues::U8(bytes)) => bytes.len(),
            _ => self.len(),
        }
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    /// Byte `index` of a [`DataFormat::HexDump`], and its line and place in that line. `None` for any other content, or
    /// past the last byte.
    pub(crate) fn hex_dump_byte(&self, index: usize) -> Option<(u8, usize, usize)> {
        self.hex_dump()?;
        let byte = *self.bytes().get(index)?;
        Some((
            byte,
            index / hex_dump_format::BYTES_PER_LINE,
            index % hex_dump_format::BYTES_PER_LINE,
        ))
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

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
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

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    /// Returns `self` with every element labelled by its own index, `0` to `N - 1`: [`LabellingStyle::Numeric`].
    ///
    /// Labelling is off unless this or [`with_labelling_style`](Self::with_labelling_style) is called, so existing
    /// content draws exactly as before. A label is drawn in the data text's own size, outside the cells, in the grid box's
    /// own left padding. That padding grows only if the label needs more room than it has. A grid of one column has a
    /// label for every element. A wider grid has one beside each row, naming that row's first element. A node with one
    /// value has no grid to label, so this has no effect on it, nor on [`DataFormat::PlainText`].
    #[must_use]
    pub fn with_labels(self) -> Self {
        self.with_labelling_style(LabellingStyle::default())
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    /// Returns `self` with its grid labelled in `style`, which also turns labelling on. A numeric or alphabetic style
    /// labels every element, so a wider grid shows the label of each row's first element. [`LabellingStyle::Text`] labels
    /// every row with a string of the caller's own choosing, by row number and not by element. See [`LabellingStyle`]
    /// for what each style produces, and [`with_labels`](Self::with_labels) for where a label is drawn.
    #[must_use]
    pub fn with_labelling_style(mut self, style: LabellingStyle) -> Self {
        self.labelling = Some(style);
        self
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    /// The style to label each element in, or `None` for no labels. Always `None` for a node with one value, which has
    /// no grid to label.
    pub(crate) fn labelling(&self) -> Option<LabellingStyle> {
        if self.is_single_value() { None } else { self.labelling }
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    /// How many columns make up one group — see [`with_column_groups`](Self::with_column_groups); `0` for none.
    pub(crate) fn column_group(&self) -> usize {
        self.column_group
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
    /// Replaces this content's own values with `values`, keeping its format, layout and byte order. `update(index,
    /// formatted)` is called for each cell whose value changed, in order, and for no other. A cell that holds the same
    /// value as before is not even formatted, and nothing is called at all if no value changed.
    ///
    /// The new values are committed only after every `update` has succeeded. If one returns an error, that error is
    /// returned and this content still holds its old values. A caller that retries with the same `values` therefore sees
    /// every cell that was not yet confirmed as changed again, including those it had already written.
    ///
    /// Returns `Ok(false)`, changing nothing and calling nothing, unless `values` is the same type, signed or unsigned integer or float, of the same bit width if they are bit values, and holds the
    /// same number of values. So the grid keeps exactly the same shape it was drawn with. [`DataFormat::PlainText`]
    /// content also needs printable ASCII, and counts as one cell, whole. [`DataFormat::Text`] content needs strings with
    /// no control character in them. A cell's width is not changed to fit a longer string. See
    /// [`with_cell_width_hint`](Self::with_cell_width_hint).
    pub(crate) fn try_replace_values<E>(
        &mut self,
        values: NodeValues,
        scratch: &mut String,
        mut update: impl FnMut(usize, &str) -> Result<(), E>,
    ) -> Result<bool, E> {
        if !self.values.same_kind(&values) || self.values.len() != values.len() {
            return Ok(false);
        }
        if self.format == DataFormat::PlainText && !is_printable_ascii(&values) {
            return Ok(false);
        }
        // A replacement string is held to the same rule as one the node was built with.
        if values
            .text()
            .is_some_and(|cells| cells.iter().any(|text| text.chars().any(char::is_control)))
        {
            return Ok(false);
        }
        if let (Some(format), NodeValues::U8(new)) = (self.hex_dump(), &values) {
            // The values have the same count, so the dump has the same lines, and the line holding the total length, if
            // shown, cannot differ. A line is formatted only if its 16 bytes changed.
            let per_line = hex_dump_format::BYTES_PER_LINE;
            for (index, (old, new_chunk)) in self.bytes().chunks(per_line).zip(new.chunks(per_line)).enumerate() {
                if old != new_chunk {
                    hex_dump_format::format_line(new, format, index, scratch);
                    update(index, scratch)?;
                }
            }
        } else if self.is_plain_text() {
            if values != self.values && self.len() == 1 {
                scratch.clear();
                if let NodeValues::U8(v) = &values {
                    scratch.extend(v.iter().map(|&b| char::from(b)));
                }
                update(0, scratch)?;
            }
        } else {
            values.try_for_each_changed_cell_string(&self.values, self.format, self.byte_order, scratch, update)?;
        }
        self.values = values;
        Ok(true)
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    /// `true` unless `values` is exactly what this content holds now. A caller can skip formatting anything for a
    /// replacement that changes nothing.
    pub(crate) fn differs_from(&self, values: &NodeValues) -> bool {
        !self.values.same_as(values)
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    /// How many values this holds. [`DataFormat::PlainText`] content is one value, however many characters it has.
    pub(crate) fn len(&self) -> usize {
        if self.is_plain_text() {
            usize::from(self.values.len() > 0)
        } else if let (Some(format), NodeValues::U8(bytes)) = (self.hex_dump(), &self.values) {
            hex_dump_format::line_count(bytes.len(), format)
        } else {
            self.values.len()
        }
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    /// `true` for [`DataFormat::Ascii`] content.
    pub(crate) fn is_ascii(&self) -> bool {
        self.format == DataFormat::Ascii
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    /// How a [`DataFormat::HexDump`] writes its bytes, and `None` for any other content.
    pub(crate) fn hex_dump(&self) -> Option<HexDumpFormat> {
        match self.format {
            DataFormat::HexDump(format) => Some(format),
            _ => None,
        }
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    /// Returns the bytes of `u8` content, and none for any other.
    pub(crate) fn bytes(&self) -> &[u8] {
        match &self.values {
            NodeValues::U8(bytes) => bytes,
            _ => &[],
        }
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    /// `false` for [`DataFormat::HexDump`] content that is not `u8`. Every other content is valid here.
    pub(crate) fn hex_dump_is_valid(&self) -> bool {
        self.hex_dump().is_none() || matches!(self.values, NodeValues::U8(_))
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    /// `true` for content whose text has runs of spaces that must be drawn as they are: [`DataFormat::PlainText`] and
    /// [`DataFormat::HexDump`].
    pub(crate) fn keeps_spaces(&self) -> bool {
        self.is_plain_text() || self.hex_dump().is_some()
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    /// `true` for content whose spaces must be drawn as they are: [`keeps_spaces`](Self::keeps_spaces), and
    /// [`DataFormat::Text`], whose strings may have runs of spaces in them. Unlike a plain text or hex dump node, a text
    /// node is still a grid, so it still has its layout checked.
    pub(crate) fn preserves_spaces(&self) -> bool {
        self.keeps_spaces() || self.is_text()
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    /// `true` for [`DataFormat::PlainText`] content.
    pub(crate) fn is_plain_text(&self) -> bool {
        self.format == DataFormat::PlainText
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    /// `false` for [`DataFormat::PlainText`] content that is not `u8` or holds a byte outside `0x20..=0x7E`. Every other
    /// content is valid here.
    pub(crate) fn plain_text_is_valid(&self) -> bool {
        !self.is_plain_text() || is_printable_ascii(&self.values)
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    /// `true` if this content holds `f32` or `f64` values.
    pub(crate) fn holds_floats(&self) -> bool {
        self.values.is_float()
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    /// `false` for [`DataFormat::Float`] content whose values are not `f32` or `f64`, or whose format asks for more digits
    /// than [`MAX_FLOAT_DIGITS`](super::MAX_FLOAT_DIGITS) or for no significant digits. Every other content is valid.
    pub(crate) fn float_format_is_valid(&self) -> bool {
        match self.format {
            DataFormat::Float(float_format) => self.values.is_float() && float_format.is_valid(),
            _ => true,
        }
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    /// `true` if this content holds text.
    pub(crate) fn holds_text(&self) -> bool {
        self.values.is_text()
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    /// `true` if this content holds `Uuid` values.
    pub(crate) fn holds_uuids(&self) -> bool {
        self.values.is_uuid()
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    /// `false` for [`DataFormat::Uuid`] content whose values are not `Uuid`. Every other content is valid.
    pub(crate) fn uuid_format_is_valid(&self) -> bool {
        !matches!(self.format, DataFormat::Uuid(_)) || self.values.is_uuid()
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    /// `true` if this content holds `Bool` values.
    pub(crate) fn holds_bools(&self) -> bool {
        self.values.is_bool()
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    /// `false` for [`DataFormat::Bool`] content whose values are not `Bool`. Every other content is valid.
    pub(crate) fn bool_format_is_valid(&self) -> bool {
        !matches!(self.format, DataFormat::Bool(_)) || self.values.is_bool()
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    /// `true` for [`BoolFormat::Icon`] content, which draws a mark and not a word.
    pub(crate) fn is_bool_icon(&self) -> bool {
        self.format == DataFormat::Bool(BoolFormat::Icon)
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    /// The fill of a cell that shows `text`: green for a check mark, red for a cross, and the usual text colour for
    /// every other cell. `text` is what [`for_each_cell_string`](Self::for_each_cell_string) gave that cell.
    pub(crate) fn ink(&self, text: &str) -> &'static str {
        if self.hex_dump().is_some() {
            return crate::colours::TERMINAL_INK;
        }
        Self::ink_for(self.is_bool_icon(), text)
    }

    /// [`ink`](Self::ink) for a caller that only knows whether the content is [`BoolFormat::Icon`], and so cannot
    /// borrow the content itself.
    pub(crate) fn ink_for(icon: bool, text: &str) -> &'static str {
        match text {
            ICON_TRUE if icon => BOOL_TRUE_INK,
            ICON_FALSE if icon => BOOL_FALSE_INK,
            _ => TEXT_FILL,
        }
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    /// What a screen reader should be told for a cell that shows `text`: `true` or `false` for a mark, and `text`
    /// itself for every other cell, or `empty` for a cell with no text. A mark is a picture, and its glyph would be read
    /// out as its Unicode name.
    pub(crate) fn spoken<'a>(&self, text: &'a str) -> &'a str {
        Self::spoken_for(self.is_bool_icon(), text)
    }

    /// [`spoken`](Self::spoken) for a caller that only knows whether the content is [`BoolFormat::Icon`].
    pub(crate) fn spoken_for(icon: bool, text: &str) -> &str {
        match text {
            ICON_TRUE if icon => "true",
            ICON_FALSE if icon => "false",
            // Only a text cell can be empty, and a blank cell would be read out as nothing at all.
            "" => "empty",
            _ => text,
        }
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    /// Both marks of [`BoolFormat::Icon`] content, and `None` for any other content. A cell is drawn as wide as the
    /// wider of the two, since the node may be asked to change from either to the other.
    pub(crate) fn bool_icon_marks(&self) -> Option<[&'static str; 2]> {
        self.is_bool_icon().then_some([ICON_TRUE, ICON_FALSE])
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
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
    ///
    /// A [`DataFormat::HexDump`] is never one, even if it is a single line. It is always drawn as a terminal screen of
    /// lines, and a line is not a value that stands alone.
    pub(crate) fn is_single_value(&self) -> bool {
        self.len() == 1 && self.hex_dump().is_none()
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
        if self.hex_dump().is_some() {
            return (self.len(), 1);
        }
        grid_shape(self.len(), self.layout)
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    /// How many rows of [`shape`](Self::shape) hold at least one value. A layout can ask for more rows than there are
    /// values to fill, and a row with no cell is drawn with no label, so it has no bearing on how much room labels need.
    pub(crate) fn populated_rows(&self) -> usize {
        let (rows, cols) = self.shape();
        self.len().div_ceil(cols.max(1)).min(rows)
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
        if let Some(format) = self.hex_dump() {
            for index in 0..self.len() {
                hex_dump_format::format_line(self.bytes(), format, index, scratch);
                f(index, scratch);
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
        if let Some(format) = self.hex_dump() {
            hex_dump_format::format_line(self.bytes(), format, 0, out);
            return self.len() > 0;
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
        // Every line is padded to the same width, so the first is as wide as any.
        if let Some(format) = self.hex_dump() {
            out.clear();
            if self.len() > 0 {
                hex_dump_format::format_line(self.bytes(), format, 0, out);
            }
            return;
        }
        self.values.widest_cell_string(self.format, self.byte_order, out);
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    /// The pastel colour identifying this content's own value type — see [`NodeValues::type_colour`].
    pub(crate) fn type_colour(&self) -> &'static str {
        if self.hex_dump().is_some() {
            return crate::colours::TERMINAL_BG;
        }
        if self.is_plain_text() {
            return crate::colours::PLAIN_BOX_FILL;
        }
        self.values.type_colour()
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    /// This content's own type name ("u8"/"u16"/"u32"/"u64"/"i8"/"i16"/"i32"/"i64"/"f32"/"f64") — see [`NodeValues::type_name`].
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
        // A hex dump is selected a byte at a time, so only nothing, or one byte, can be selected in it.
        if self.hex_dump().is_some() {
            return match selection {
                Selection::None => Some((ResolvedBand::None, None)),
                Selection::Cell(i) => (i < self.selectable_len()).then_some((ResolvedBand::None, Some(i))),
                Selection::Row { .. } | Selection::Column { .. } => None,
            };
        }
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
        if flat_index >= self.selectable_len() {
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
        (index < self.selectable_len()).then_some(index)
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// `true` if `values` is `u8` and every byte is printable ASCII, `0x20..=0x7E`.
fn is_printable_ascii(values: &NodeValues) -> bool {
    matches!(values, NodeValues::U8(v) if v.iter().all(|b| (0x20..=0x7E).contains(b)))
}
