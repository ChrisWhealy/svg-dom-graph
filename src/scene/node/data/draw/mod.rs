//! Drawing a data node: measuring its cells, sizing the box to fit them, and rendering the grid.
//!
//! The work is split by concern: `layout` for where cells sit, `frame` for the box and name around them, `cells` for the
//! text in them, `pictures` for the Argand pictures, and `marks` for the bytes of a hex dump. This module holds the
//! shared measurements, `draw_content_box` and `describe`.

use self::{
    cells::draw_cells, frame::draw_content_rect, frame::draw_name_frame, layout::GridLayoutMetrics,
    marks::draw_hex_marks,
};
use crate::{
    error::Error,
    scene::{
        BoxHandles, DataDecor, DataNodeContent, Selection,
        node::{EdgeAnchors, GRID_FONT_SIZE, label_group, render_guard::RenderGuard},
    },
};
use svg_dom::{
    SvgRoot,
    root::utils::{Point, Rect},
};

mod cells;
mod frame;
mod layout;
mod marks;
mod pictures;

pub(super) use cells::cell_name_into;
pub(crate) use cells::write_cell_text;
pub(crate) use frame::measure_content_box;
pub(crate) use marks::{new_dim_block, new_mark};
pub(crate) use pictures::{pictures_to_write, update_picture};

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The gap left between adjacent value cells in a multi-value grid, so the node's own background colour shows through
/// as a visible seam between them. This seam, together with each cell's own [`NodeValues::type_colour`], lets a reader
/// tell where one value ends and the next begins. Without it, the grid would read as a wall of digits with no
/// indication of which byte belongs to which value.
///
/// [`NodeValues`]: crate::model::content::NodeValues
pub(super) const CELL_GAP: f64 = 6.0;

/// The extra gap added between two groups of columns — see `DataNodeContent::with_column_groups` — on top of
/// [`CELL_GAP`]. Small enough that a group still reads as part of one grid.
pub(super) const GROUP_GAP: f64 = 8.0;

/// Clear space on each side of a row label, in the grid box's own left padding.
pub(super) const ELEMENT_LABEL_MARGIN: f64 = 4.0;

/// The height of the row a named data node reserves under its name for a selection toolbar, when its content asks for
/// one with `DataNodeContent::with_toolbar`: the tallest button a docked bar draws, with a margin above and below.
pub(crate) const TOOLBAR_ROW_HEIGHT: f64 = 36.0;

/// The narrowest a named data node with a toolbar row is drawn, so that the bar of Prev, Next and Restart fits. A wider
/// bar, one with the stride buttons, is the caller's to make room for.
pub(super) const TOOLBAR_MIN_WIDTH: f64 = 220.0;

/// The distance between the two rows of a [`ComplexNumberFormat::Matrix`](crate::scene::ComplexNumberFormat::Matrix)
/// cell, from baseline to baseline. A little more than the font size, so that the rows do not touch.
pub(crate) const MATRIX_LINE_HEIGHT: f64 = GRID_FONT_SIZE * 1.3;

/// The room a matrix cell leaves for a bracket on each side of its rows, on top of [`CELL_PADDING`].
pub(super) const MATRIX_BRACKET_ROOM: f64 = 8.0;

/// How much smaller than the cell's own text an exponent is, and how far it is raised, as parts of the font size.
pub(super) const EXPONENT_SCALE: f64 = 0.72;
pub(super) const EXPONENT_RISE: f64 = 0.42;

/// How far the arms of a matrix bracket reach in towards the rows.
pub(super) const MATRIX_BRACKET_ARM: f64 = 4.0;

/// The height of one line of a hex dump, which a terminal sets close: with no box or gap around it, the lines of a
/// dump are as tall as their text, plus a little leading.
pub(super) const TERMINAL_LINE_HEIGHT: f64 = GRID_FONT_SIZE * 1.25;

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Draws a data node's rectangle and its grid of value cells, grouped under one `<g>`. Returns their handles alongside
/// the box's own final `Rect`, which is computed here, not supplied by the caller.
///
/// Every value gets its own `<text>` element (monospace — see [`GRID_FONT_FAMILY`]). Under a monospace font, character
/// count alone determines a cell's own rendered width. Every string [`DataNodeContent::cells`]/
/// [`DataNodeContent::for_each_cell_string`] produces is ASCII, so byte length already is character count. So only the
/// widest cell's own text is ever read back via [`SvgNode::bounding_box`](svg_dom::SvgNode::bounding_box). This is the
/// same "measure, don't estimate" approach [`shrink_label_to_fit`](super::plain::shrink_label_to_fit) already uses for
/// plain labels, applied once per node, not once per cell. Every cell then shares that one measured width plus
/// [`CELL_PADDING`], so the grid's rows and columns still line up even when [`DataFormat::Decimal`] values differ in
/// digit count.
///
/// Formats and places each cell streaming. It never collects a `Vec<String>` of every cell's own text, or a
/// `Vec<SvgNode>` of every `<text>` element, regardless of how many values `content` holds. It never formats any value
/// twice, as follows:
///
/// 1. [`DataNodeContent::widest_cell_string`] identifies and formats the one value guaranteed to need the widest cell,
///    without formatting every value first — see that method's own doc comment for how. A throwaway element built from
///    it, once `cell_size` is known, is all `bounding_box()` ever needs. Which specific value that was is otherwise
///    irrelevant, since any string of the same length would measure identically under a monospace font.
/// 2. [`DataNodeContent::for_each_cell_string`] then formats every value once, reusing one buffer. This time it creates
///    each cell's own `<text>` (and, for a multi-value grid, its own `<rect>`) directly at its final position. It
///    appends each immediately, rather than deferring every cell's own placement to a later pass. The one value pass 1
///    already formatted is formatted again here, along with every other — a single value's worth of repeated work, not
///    repeated for the whole node.
///
/// [`DataNodeContent::is_single_value`] decides which of two layouts is drawn:
///
/// - A single value has no sibling to be told apart from, so it gets no inner cell box at all. The node's own `rect_el`
///   is filled directly with [`NodeValues::type_colour`], and the value's text sits centred in it.
/// - Two or more values each get their own small [`NodeValues::type_colour`]-filled `<rect>`, arranged into the
///   `content.shape()` grid with [`CELL_GAP`] between them, inside the node's own (unchanged, light blue) outer box.
///
/// [`DataFormat::Decimal`]: crate::model::content::DataFormat
/// [`NodeValues`]: crate::model::content::NodeValues
/// [`DataNodeContent::is_single_value`]: crate::model::content::DataNodeContent::is_single_value
///
/// Every child — the outer box and every cell's own rect/text — is drawn in local coordinates, relative to `(0, 0)`,
/// not `top_left`. The group itself carries `top_left` as a `transform="translate(...)"` instead.
///
/// A grid can hold arbitrarily many cells. Without local coordinates, moving the node later would mean rewriting every
/// cell's own `x`/`y` attributes on every pointer move. See [`SceneInner::move_node`].
///
/// A [`RenderGuard`] covers this function's own DOM construction, as in [`draw_box`](super::plain::draw_box). Streaming
/// each cell (create, style, append, immediately) keeps the window this matters for down to one cell at a time. That is
/// two nodes, briefly, for a multi-value grid's own rect-then-text pair. It works via [`RenderGuard::release`], rather
/// than every cell created so far staying tracked until the whole node finishes.
///
/// `scratch` is a caller-owned buffer — `SceneInner::scratch`, in every real caller — reused for this call's own
/// per-cell `x`/`y`/`transform` formatting, the same reasoning [`draw_box`](super::plain::draw_box)'s own `scratch`
/// parameter follows. This is a distinct concern from pass 1/2's own per-value formatting buffer above. That buffer
/// holds cell *content*, not attribute values, and stays a plain local. Nothing outside a single `draw_content_box`
/// call ever needs it.
///
/// `name`, when `Some`, wraps the box described above in a further outer box of its own, with `name` in a label row
/// above it. This is the same "outer box labelled with a name, inset value box beneath it" shape
/// [`draw_operator_box`](super::operator::draw_operator_box) already draws for an operator's own result. The returned
/// `Rect` is then that outer box's own. So an incoming connector anchors to it, never to the inner (unlabelled, exactly
/// as drawn below) content box directly. See [`draw_operator_box`]'s own module doc comment for why a connector must
/// never land on an inset inner box. `name` is `None` for every plain (unnamed) data node, which renders exactly as
/// before this parameter existed. It has no label row and no further outer box, and the content box itself starts flush
pub(super) fn draw_content_box(
    svg: &SvgRoot,
    scratch: &mut String,
    top_left: Point,
    name: Option<&str>,
    content: &DataNodeContent,
    edge_anchors: Option<EdgeAnchors>,
) -> Result<(BoxHandles, Rect), Error> {
    if content.len() == 0 {
        return Err(Error::Svg(svg_dom::Error::Dom(
            "draw_content_box: content length must be > 0".into(),
        )));
    }

    let group = svg.group()?;
    // At most two nodes are ever loose (created but not yet appended) at once here, regardless of how many values
    // `content` holds, or whether `name` is given. See `RenderGuard::release`'s own doc comment. Each helper below
    // releases whatever it tracked, in turn, before it creates the next.
    let mut guard = RenderGuard::new(group.clone());

    let layout = GridLayoutMetrics::measure(svg, &mut guard, content)?;
    let content_size = layout.content_size();
    let frame = draw_name_frame(svg, scratch, &group, &mut guard, name, content, content_size)?;
    let content_rect_el = draw_content_rect(svg, &group, &mut guard, content, frame.content_origin, content_size)?;
    let cells = draw_cells(
        svg,
        scratch,
        &group,
        &mut guard,
        content,
        &layout,
        frame.content_origin,
        &content_rect_el,
    )?;

    let hex_marks = match content.hex_dump() {
        Some(format) => Some(draw_hex_marks(svg, &group, &mut guard, format, layout.cell.width)?),
        None => None,
    };

    // See `draw_box`'s own comment on its matching call for why `set_transform_fmt`, not `set_translate`.
    group.set_transform_fmt(scratch, format_args!("translate({}, {})", top_left.x, top_left.y))?;

    let node_label = describe(name, content, &cells.single_value_text);
    label_group(&group, &node_label)?;

    guard.disarm();
    let base_label_len = node_label.len();
    Ok((
        BoxHandles {
            group,
            draggable: false,
            enterable: false,
            edge_anchors,
            binary_operator_inputs: None,
            operator_input_side: None,
            binary_operator_input_edges: None,
            // The named wrapper when there is one, the content box itself otherwise.
            outer_rect: frame.outer_el.unwrap_or(content_rect_el),
            cell_rects: cells.rects,
            cell_texts: cells.texts,
            replaceable: true,
            cell_grid: Some(cells.grid),
            decor: DataDecor {
                hex_marks,
                toolbar_slot: frame.toolbar_slot,
                pictures: cells.pictures,
            },
            cell_stroke_width: match (content.is_single_value(), content.hex_dump()) {
                (true, _) => "1.5",
                // A line of a dump has no border, until a selection draws one.
                (_, Some(_)) => "0",
                _ => "1",
            },
            selection: Selection::None,
            secondary: Vec::new(),
            unreached: Vec::new(),
            aria_label: node_label,
            base_label_len,
            label_stale: false,
            edge_anchors_stale: false,
            // A named node is called by that name. An unnamed one, having none, is called by its own type instead. See
            // `BoxHandles::ref_name`'s own doc comment.
            ref_name: name.unwrap_or(content.type_name()).to_owned(),
            child: None,
            plot: None,
        },
        Rect {
            origin: top_left,
            size: frame.size,
        },
    ))
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The accessible name of the whole node, which is also its tooltip text.
///
/// Assistive technology usually announces a group before its children. So a reader need not visit every individual
/// cell to learn the node's own name, type, or (for a single value) its real value. A multi-value grid's own row/column
/// shape is spelled out too, as "2 rows by 3 columns", and not left implicit in each cell's own `x`/`y` position. Each
/// cell's own `aria-label` names its row and column directly, the same "not visual position alone" reasoning.
///
/// `single_value_text` is the one value's own formatted text, as [`DrawnCells`] captured it. It is unused for a grid.
fn describe(name: Option<&str>, content: &DataNodeContent, single_value_text: &str) -> String {
    let type_name = content.type_name();
    let (rows, cols) = content.shape();
    let row_word = if rows == 1 { "row" } else { "rows" };
    let col_word = if cols == 1 { "column" } else { "columns" };
    let value = if let Some(bytes) = content.hex_dump().map(|_| content.selectable_len()) {
        // A dump's cells are its lines, which says nothing about its data, so it is described by its bytes. The line
        // that holds the total length, when shown, is not data and is not counted.
        let lines = bytes.div_ceil(crate::model::content::BYTES_PER_LINE);
        let (byte_word, line_word) = (
            if bytes == 1 { "byte" } else { "bytes" },
            if lines == 1 { "line" } else { "lines" },
        );
        format!("hex dump, {bytes} {byte_word} across {lines} data {line_word}")
    } else if content.is_single_value() {
        format!("{type_name} = {single_value_text}")
    } else {
        format!(
            "{type_name} data grid, {rows} {row_word} by {cols} {col_word}, {} values",
            content.len()
        )
    };
    match name {
        Some(name) => format!("{name}: {value}"),
        None => value,
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[cfg(test)]
mod unit_tests;
