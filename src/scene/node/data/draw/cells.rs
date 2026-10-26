//! The cells of a data node: their text, accessible names, matrix brackets and row labels.

use super::{
    ELEMENT_LABEL_MARGIN, EXPONENT_RISE, EXPONENT_SCALE, MATRIX_BRACKET_ARM, MATRIX_LINE_HEIGHT,
    layout::GridLayoutMetrics, pictures::draw_picture,
};
use crate::{
    colours::{BOX_STROKE, MATRIX_BRACKET_STROKE, TEXT_FILL},
    error::Error,
    model::content::{CellText, exponent_parts, matrix_rows},
    scene::{
        CellGrid, CellPicture, DataNodeContent, LabellingStyle,
        node::{CELL_PADDING, GRID_FONT_FAMILY, GRID_FONT_SIZE, halo_icon, render_guard::RenderGuard},
    },
};
use svg_dom::{
    DominantBaseline, SvgNode, SvgRoot, TextAnchor,
    root::utils::{Point, Size},
};

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Writes `cell_text` as the content of a cell's `<text>`, built as `style` says. Any `<tspan>` the text already had is
/// replaced.
///
/// * [`CellText::Plain`] is one run of text, set as the text itself.
/// * [`CellText::Matrix`] is two rows. Each is a `<tspan>` of its own, centred on `centre_x`, and the second is one
///   [`MATRIX_LINE_HEIGHT`] below the first. Without a `centre_x` it is drawn as one line instead.
/// * [`CellText::Exponent`] is the base, and then the exponent as a `<tspan>` that is smaller and raised. It follows the
///   base on the same line, so it needs no position of its own.
///
/// Drawing a cell, measuring one, and rewriting its value all go through here, so none can disagree about how a cell is
/// built.
pub(crate) fn write_cell_text(
    text: &SvgNode,
    cell_text: &str,
    style: CellText,
    centre_x: Option<f64>,
) -> Result<(), svg_dom::Error> {
    match (style, centre_x) {
        (CellText::Matrix, Some(x)) => {
            if let Some((first, second)) = matrix_rows(cell_text) {
                // Setting the text content removes every child, the old rows included.
                text.set_text("");
                text.tspan_line(x, 0.0, first)?;
                text.tspan_line(x, MATRIX_LINE_HEIGHT, second)?;
                return Ok(());
            }
        },
        (CellText::Exponent, _) => {
            if let Some((base, exponent)) = exponent_parts(cell_text) {
                text.set_text("");
                text.tspan(base)?;
                let raised = text.tspan_dy(-GRID_FONT_SIZE * EXPONENT_RISE, exponent)?;
                raised.set_font_size(GRID_FONT_SIZE * EXPONENT_SCALE)?;
                return Ok(());
            }
        },
        _ => {},
    }
    text.set_text(cell_text);
    Ok(())
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The `y` of a matrix cell's `<text>`, which is where its first row is centred. The second row is one line lower, so
/// the pair is centred on the middle of a cell that starts at `top` and is `height` high.
pub(crate) fn matrix_text_y(top: f64, height: f64) -> f64 {
    top + (height - MATRIX_LINE_HEIGHT) / 2.0
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// What [`draw_cells`] drew, flat and in row-major order.
pub(super) struct DrawnCells {
    /// Every cell's own `<rect>`. For a single value, the content box itself.
    pub(super) rects: Vec<SvgNode>,
    /// Every cell's own `<text>`.
    pub(super) texts: Vec<SvgNode>,
    /// Where every cell sits, local to the group.
    pub(super) grid: CellGrid,
    /// The picture at the front of every cell of a polar or rectangular complex number, in cell order, and empty for
    /// any other content.
    pub(super) pictures: Vec<CellPicture>,
    /// The one value's own formatted text, for a single value. Empty for a grid, whose own `aria-label` names a value
    /// count instead. See [`describe`].
    pub(super) single_value_text: String,
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Formats and places each cell, streaming. See [`draw_content_box`]'s own doc comment for why nothing is collected up
/// front. `text_scratch` is reused for every cell's own formatted text.
///
/// A single value has no inner box, so its text sits centred in `content_rect_el` itself. Each value of a grid gets
/// its own `<rect>`, positioned by [`CellGrid::cell_rect`], and its text is given an accessible name that
/// carries its row and column. A bare digit string, read on its own, says nothing about which row/column it belongs to.
/// That relationship exists only in the cell's own `x`/`y`, invisible to assistive technology.
#[allow(clippy::too_many_arguments)]
pub(super) fn draw_cells(
    svg: &SvgRoot,
    scratch: &mut String,
    group: &SvgNode,
    guard: &mut RenderGuard,
    content: &DataNodeContent,
    layout: &GridLayoutMetrics,
    content_origin: Point,
    content_rect_el: &SvgNode,
) -> Result<DrawnCells, Error> {
    let len = content.len();
    let single_value = layout.single_value;
    let grid = layout.grid(content_origin, len);
    let type_colour = content.type_colour();
    let mut cells = DrawnCells {
        rects: Vec::with_capacity(if single_value { 1 } else { len }),
        texts: Vec::with_capacity(len),
        grid,
        pictures: Vec::new(),
        single_value_text: String::new(),
    };
    if single_value {
        cells.rects.push(content_rect_el.clone());
    }

    let style = content.cell_text();
    let picture_kind = content.icon_kind();
    let max_modulus = content.max_modulus();
    let mut text_scratch = String::new();
    let mut label_scratch = String::new();
    let mut name_scratch = String::new();
    let mut error: Option<Error> = None;
    content.for_each_cell_string(&mut text_scratch, |i, cell_text| {
        if error.is_some() {
            return;
        }
        let result = (|| -> Result<(), Error> {
            let text = svg.text(Point::origin(), if style == CellText::Plain { cell_text } else { "" })?;
            guard.track(text.clone());
            text.set_text_anchor(TextAnchor::Middle)?;
            text.set_dominant_baseline(DominantBaseline::Middle)?;
            text.set_font_family(GRID_FONT_FAMILY)?;
            text.set_font_size(GRID_FONT_SIZE)?;
            text.set_fill(content.ink(cell_text))?;
            if content.is_bool_icon() {
                halo_icon(&text)?;
            }
            if content.preserves_spaces() {
                text.set_attr("style", "white-space: pre")?;
            }
            cells.texts.push(text.clone());

            // Where this cell is. A single value fills the content box, which is already drawn, and each value of a grid
            // has a rectangle of its own.
            let cell_origin = if single_value {
                cells.single_value_text.push_str(content.spoken(cell_text));
                content_origin
            } else {
                let Some(cell_origin) = grid.cell_rect(i).map(|r| r.origin) else {
                    return Ok(());
                };
                let cell_rect = svg.rect(cell_origin, layout.cell)?;
                guard.track(cell_rect.clone());
                cell_rect.set_fill(type_colour)?;
                cell_rect.set_stroke(BOX_STROKE)?;
                cell_rect.set_stroke_width(if content.hex_dump().is_some() { 0.0 } else { 1.0 })?;
                group.append(&cell_rect)?;
                guard.release();
                cells.rects.push(cell_rect);
                cell_origin
            };

            // The text is centred in what is left of the cell after the room for its picture.
            let centre_x = cell_origin.x + (layout.cell.width + layout.picture_room) / 2.0;
            text.set_attr_display(scratch, "x", centre_x)?;
            let centre_y = if style == CellText::Matrix {
                matrix_text_y(cell_origin.y, layout.cell.height)
            } else {
                cell_origin.y + layout.cell.height / 2.0
            };
            text.set_attr_display(scratch, "y", centre_y)?;
            if style != CellText::Plain {
                write_cell_text(&text, cell_text, style, Some(centre_x))?;
            }
            if style == CellText::Matrix {
                draw_matrix_brackets(svg, group, guard, cell_origin, layout.cell)?;
            }
            if let Some(kind) = picture_kind {
                let vector = content.complex_at(i);
                cells
                    .pictures
                    .push(draw_picture(svg, group, guard, cell_origin, layout, kind, vector, max_modulus)?);
            }

            if !single_value {
                let col = i % layout.cols;
                if let (Some(style), 0) = (content.labelling(), col) {
                    style.label_into(i, layout.cols, &mut label_scratch);
                    // A caller's own text can leave a row without a label. There is nothing to draw for it, so no
                    // element is made.
                    if !label_scratch.is_empty() {
                        draw_row_label(svg, scratch, group, guard, &label_scratch, cell_origin, layout.cell)?;
                    }
                }
                cell_name_into(
                    content.labelling(),
                    i,
                    layout.cols,
                    content.spoken(cell_text),
                    &mut name_scratch,
                );
                text.set_attr("aria-label", &name_scratch)?;
            }
            group.append(&text)?;
            guard.release();
            Ok(())
        })();
        if let Err(e) = result {
            error = Some(e);
        }
    });
    match error {
        Some(e) => Err(e),
        None => Ok(cells),
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Writes the accessible name of grid cell `i`, in a grid of `cols` columns, showing `text`, into `out`, replacing what
/// it held. It names the cell's row and column, and its label first if `labelling` is on, as in "element a, row
/// 0, column 0: 6A 09 E6 67". A [`LabellingStyle::Text`] label names a row, so it reads "row label ms[n-3], row 0, column
/// 0: 6A 09 E6 67", and is left out altogether for a row it gives no text. Drawing a cell and rewriting its value both build the name here, so the two cannot
/// disagree. A caller reuses one `out` for every cell, so a name allocates only if it outgrows the buffer.
pub(crate) fn cell_name_into(labelling: Option<LabellingStyle>, i: usize, cols: usize, text: &str, out: &mut String) {
    use std::fmt::Write;
    out.clear();
    match labelling {
        // A caller's own text names the cell's row, not the cell, so it is introduced as a row label. A row it gives no
        // text for has nothing to quote, and the name goes without it, rather than read "row label , row 2".
        Some(style @ LabellingStyle::Text(_)) => {
            let start = out.len();
            out.push_str("row label ");
            let text_start = out.len();
            style.append_label(i, cols, out);
            if out.len() == text_start {
                out.truncate(start);
            } else {
                out.push_str(", ");
            }
        },
        Some(style) => {
            out.push_str("element ");
            style.append_label(i, cols, out);
            out.push_str(", ");
        },
        None => {},
    }
    let _ = write!(out, "row {}, column {}: {text}", i / cols, i % cols);
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Draws the two tall brackets of a matrix cell that has `origin` and `size`, as one path. They are decoration: hidden from
/// assistive technology, which is told the matrix by the cell's text, and they take no pointer events.
pub(super) fn draw_matrix_brackets(
    svg: &SvgRoot,
    group: &SvgNode,
    guard: &mut RenderGuard,
    origin: Point,
    size: Size,
) -> Result<(), Error> {
    let (left, right) = (origin.x + CELL_PADDING, origin.x + size.width - CELL_PADDING);
    let (top, bottom) = (origin.y + CELL_PADDING * 0.75, origin.y + size.height - CELL_PADDING * 0.75);
    let arm = MATRIX_BRACKET_ARM;
    let d = format!(
        "M {} {top} H {left} V {bottom} H {} M {} {top} H {right} V {bottom} H {}",
        left + arm,
        left + arm,
        right - arm,
        right - arm,
    );
    let path = svg.path(&d)?;
    guard.track(path.clone());
    path.set_fill("none")?;
    path.set_stroke(MATRIX_BRACKET_STROKE)?;
    path.set_stroke_width(1.5)?;
    path.set_attr("aria-hidden", "true")?;
    path.set_attr("pointer-events", "none")?;
    group.append(&path)?;
    guard.release();
    Ok(())
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Draws row label `label` just left of the first cell of its row, at `cell_origin`, outside the cell and vertically
/// centred on it. It is hidden from assistive technology, because the cell's own value text already names the element
/// in its `aria-label`.
pub(super) fn draw_row_label(
    svg: &SvgRoot,
    scratch: &mut String,
    group: &SvgNode,
    guard: &mut RenderGuard,
    label: &str,
    cell_origin: Point,
    cell: Size,
) -> Result<(), Error> {
    let el = svg.text(Point::origin(), label)?;
    guard.track(el.clone());
    el.set_text_anchor(TextAnchor::End)?;
    el.set_dominant_baseline(DominantBaseline::Middle)?;
    el.set_font_family(GRID_FONT_FAMILY)?;
    el.set_font_size(GRID_FONT_SIZE)?;
    el.set_fill(TEXT_FILL)?;
    el.set_attr("aria-hidden", "true")?;
    el.set_attr("pointer-events", "none")?;
    el.set_attr_display(scratch, "x", cell_origin.x - ELEMENT_LABEL_MARGIN)?;
    el.set_attr_display(scratch, "y", cell_origin.y + cell.height / 2.0)?;
    group.append(&el)?;
    guard.release();
    Ok(())
}
