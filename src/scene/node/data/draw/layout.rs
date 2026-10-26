//! The measured size of a cell and the arithmetic of where each cell sits.

use super::{
    CELL_GAP, ELEMENT_LABEL_MARGIN, GROUP_GAP, MATRIX_BRACKET_ROOM, MATRIX_LINE_HEIGHT, TERMINAL_LINE_HEIGHT,
    cells::write_cell_text,
};
use crate::{
    error::Error,
    model::content::{CellText, ICON_GAP, ICON_MARGIN, full_width, matrix_rows},
    scene::{
        CellGrid, DataNodeContent, group_gaps,
        node::{CELL_HEIGHT, CELL_PADDING, GRID_FONT_FAMILY, GRID_FONT_SIZE, OUTER_PADDING, render_guard::RenderGuard},
    },
};
use svg_dom::{
    SvgRoot,
    root::utils::{Point, Size},
};

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The measured size of one cell, and the grid shape around it. Everything else about where a cell sits follows from
/// these by arithmetic, so that arithmetic is pure and lives here, away from the DOM.
pub(super) struct GridLayoutMetrics {
    /// One cell's own box, the widest value's measured text plus [`CELL_PADDING`] on every side.
    pub(super) cell: Size,
    /// The padding between the grid box's own left edge and its first column. [`OUTER_PADDING`], or more when row labels
    /// need the room.
    pub(super) left_pad: f64,
    pub(super) rows: usize,
    pub(super) cols: usize,
    /// How many columns make up one group, or `0` for none. See `DataNodeContent::with_column_groups`.
    pub(super) column_group: usize,
    pub(super) single_value: bool,
    /// The gap between one cell and the next, [`CELL_GAP`], or none for a hex dump, whose lines touch.
    pub(super) gap: f64,
    /// The room at the front of each cell for its picture and the gap after it, or `0.0` for a cell with none. The text
    /// is centred in the rest.
    pub(super) picture_room: f64,
}

impl GridLayoutMetrics {
    /// Measures the widest cell of `content` in the real browser, and builds the metrics from it.
    ///
    /// Finds and formats the one value guaranteed to need the widest rendered cell, without formatting every value just
    /// to compare the resulting text lengths. See `DataNodeContent::widest_cell_string`'s own doc comment for how. Its
    /// own real content is then measured once, via a throwaway element that never becomes one of the group's own
    /// children. It is tracked for rollback like any other fallible-construction element. It is then removed the moment
    /// it has served its purpose, rather than kept around as one of the real cells.
    pub(super) fn measure(svg: &SvgRoot, guard: &mut RenderGuard, content: &DataNodeContent) -> Result<Self, Error> {
        let mut widest = String::new();
        content.widest_cell_string(&mut widest);
        // A matrix is two rows of the same length, so its width is that of one row.
        if content.is_matrix() {
            if let Some((row, _)) = matrix_rows(&widest) {
                widest = row.to_owned();
            }
        }
        // A hex dump is measured as a run of characters that are not spaces, whose width is that of a full line, because
        // a line ends in spaces that a measurement might leave out. Under a monospace font every character is as wide
        // as any other, so it also gives the width of one character, which places the marks of a selected byte.
        if let Some(format) = content.hex_dump() {
            widest.clear();
            widest.extend(std::iter::repeat_n('0', full_width(format)));
        }

        let measure_el = svg.text(Point::origin(), &widest)?;
        guard.track(measure_el.clone());
        measure_el.set_font_family(GRID_FONT_FAMILY)?;
        measure_el.set_font_size(GRID_FONT_SIZE)?;
        // An exponent is drawn smaller and raised, so it is measured as drawn, and not as a full-size run of characters.
        if content.cell_text() == CellText::Exponent {
            write_cell_text(&measure_el, &widest, CellText::Exponent, None)?;
        }
        if content.preserves_spaces() {
            measure_el.set_attr("style", "white-space: pre")?;
        }
        let mut max_width = measure_el.bounding_box()?.size.width;
        // Edge case: Every `Ascii` cell has the same character count, but not necessarily the same rendered width. In
        // the case that a fallback font is used, this may result in the substitute glyphs `␣` and `·` having different
        // advance widths than those from the normal font. So measure a full-length run of each and let the widest win.
        if content.is_ascii() {
            let chars = widest.chars().count();
            for substitute in ['\u{2423}', '\u{B7}'] {
                widest.clear();
                widest.extend(std::iter::repeat_n(substitute, chars));
                measure_el.set_text(&widest);
                max_width = max_width.max(measure_el.bounding_box()?.size.width);
            }
        }
        // Edge case: a check mark and a cross come from a symbol font that may not give them the same advance width.
        // The cell is wide enough for the wider of the two, because a Boolean node is meant to change from either to
        // the other.
        if let Some(marks) = content.bool_icon_marks() {
            for mark in marks {
                measure_el.set_text(mark);
                max_width = max_width.max(measure_el.bounding_box()?.size.width);
            }
        }
        // Edge case: the width of a character depends on the font it comes from, and a string may hold characters, such
        // as `∞` and `—`, that the monospace font does not have. So every text cell is measured as it is, and so is the
        // caller's width hint, and the widest wins. Character counts say nothing here.
        if let Some(cells) = content.text_cells() {
            for text in cells.iter().map(String::as_str).chain(content.cell_width_hint()) {
                measure_el.set_text(text);
                max_width = max_width.max(measure_el.bounding_box()?.size.width);
            }
        }
        measure_el.remove();
        guard.release();

        // Row labels sit in the grid box's own left padding, so it grows if they need more than it has. A generated
        // label never gets shorter as its index grows, so the last row's own is the widest. A caller's own text could be
        // any width, whatever its length, so each one is measured and the widest wins. Only a row that has a cell is
        // labelled, so a row the layout asks for but has no value for takes no room.
        let (rows, cols) = content.shape();
        let labelled_rows = content.populated_rows();
        let mut left_pad = OUTER_PADDING;
        if let Some(style) = content.labelling() {
            style.widest_label_into(labelled_rows, cols, &mut widest);
            let label_el = svg.text(Point::origin(), &widest)?;
            guard.track(label_el.clone());
            label_el.set_font_family(GRID_FONT_FAMILY)?;
            label_el.set_font_size(GRID_FONT_SIZE)?;
            let mut label_width = label_el.bounding_box()?.size.width;
            for label in style.text_labels(labelled_rows) {
                label_el.set_text(label);
                label_width = label_width.max(label_el.bounding_box()?.size.width);
            }
            left_pad = left_pad.max(label_width + 2.0 * ELEMENT_LABEL_MARGIN);
            label_el.remove();
            guard.release();
        }

        // A picture is as high as `icon_scale` lines of text, and a cell that is not tall enough for it grows.
        let picture_side = if content.icon_kind().is_some() {
            content.icon_scale() * CELL_HEIGHT
        } else {
            0.0
        };
        let picture_room = if picture_side > 0.0 { picture_side + ICON_GAP } else { 0.0 };
        let cell_height = (CELL_HEIGHT + 2.0 * CELL_PADDING).max(picture_side + 2.0 * ICON_MARGIN);

        // A hex dump is a screen of text, so a line is as wide as its text and as tall as the terminal sets it, with
        // nothing round it and nothing between it and the next. The node's own padding is the screen's margin.
        let (cell, gap) = if content.hex_dump().is_some() {
            (Size::new(max_width, TERMINAL_LINE_HEIGHT), 0.0)
        } else if content.is_matrix() {
            // Two rows between a pair of brackets.
            (
                Size::new(
                    max_width + 2.0 * (CELL_PADDING + MATRIX_BRACKET_ROOM),
                    2.0 * MATRIX_LINE_HEIGHT + 2.0 * CELL_PADDING,
                ),
                CELL_GAP,
            )
        } else {
            (Size::new(max_width + 2.0 * CELL_PADDING + picture_room, cell_height), CELL_GAP)
        };
        Ok(Self::new(
            cell,
            left_pad,
            (rows, cols),
            content.column_group(),
            content.is_single_value(),
            gap,
            picture_room,
        ))
    }

    pub(super) fn new(
        cell: Size,
        left_pad: f64,
        (rows, cols): (usize, usize),
        column_group: usize,
        single_value: bool,
        gap: f64,
        picture_room: f64,
    ) -> Self {
        Self {
            cell,
            left_pad,
            rows,
            cols,
            column_group,
            single_value,
            gap,
            picture_room,
        }
    }

    /// The content box's own size. A single value is just its one cell. A grid is every cell, the gaps between them, and
    /// [`OUTER_PADDING`] all round.
    #[allow(clippy::cast_precision_loss)]
    pub(super) fn content_size(&self) -> Size {
        if self.single_value {
            return self.cell;
        }
        Size::new(
            self.cols as f64 * self.cell.width
                + (self.cols as f64 - 1.0) * self.gap
                + group_gaps(self.column_group, self.cols) as f64 * GROUP_GAP
                + self.left_pad
                + OUTER_PADDING,
            self.rows as f64 * self.cell.height + (self.rows as f64 - 1.0) * self.gap + 2.0 * OUTER_PADDING,
        )
    }

    /// Where every cell of this layout sits, for a node of `len` cells whose content box has origin `content_origin`.
    /// Drawing and `Scene::cell_rect` both place cells through the [`CellGrid`] this returns.
    pub(super) fn grid(&self, content_origin: Point, len: usize) -> CellGrid {
        CellGrid {
            content_origin,
            cell: self.cell,
            left_pad: self.left_pad,
            top_pad: OUTER_PADDING,
            gap: self.gap,
            group_gap: GROUP_GAP,
            cols: self.cols,
            column_group: self.column_group,
            len,
            single_value: self.single_value,
        }
    }
}
