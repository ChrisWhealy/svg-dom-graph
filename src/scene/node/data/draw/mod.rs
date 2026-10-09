//! Drawing a data node: measuring its cells, sizing the box to fit them, and rendering the grid.

use super::super::{
    CELL_HEIGHT, CELL_PADDING, EdgeAnchors, GRID_FONT_FAMILY, GRID_FONT_SIZE, LABEL_FONT_SIZE, LABEL_ROW_HEIGHT,
    OUTER_PADDING, label_group, render_guard::RenderGuard,
};
use crate::{
    colours::{BOX_STROKE, NAMED_BOX_FILL, PLAIN_BOX_FILL, TEXT_FILL},
    error::Error,
    scene::{BoxHandles, DataNodeContent, Selection},
};
use svg_dom::{
    DominantBaseline, SvgNode, SvgRoot, TextAnchor,
    root::utils::{Point, Rect, Size},
};

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The gap left between adjacent value cells in a multi-value grid, so the node's own background colour shows through
/// as a visible seam between them. This seam, together with each cell's own [`NodeValues::type_colour`], lets a reader
/// tell where one value ends and the next begins. Without it, the grid would read as a wall of digits with no
/// indication of which byte belongs to which value.
///
/// [`NodeValues`]: crate::model::content::NodeValues
const CELL_GAP: f64 = 6.0;

/// The extra gap added between two groups of columns — see `DataNodeContent::with_column_groups` — on top of
/// [`CELL_GAP`]. Small enough that a group still reads as part of one grid.
const GROUP_GAP: f64 = 8.0;

/// Clear space on each side of a row label, in the grid box's own left padding.
const ELEMENT_LABEL_MARGIN: f64 = 4.0;

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
    let frame = draw_name_frame(svg, scratch, &group, &mut guard, name, content_size)?;
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
            binary_operator_input_edges: None,
            // The named wrapper when there is one, the content box itself otherwise.
            outer_rect: frame.outer_el.unwrap_or(content_rect_el),
            cell_rects: cells.rects,
            cell_texts: cells.texts,
            replaceable: true,
            cell_geometry: cells.geometry,
            cell_stroke_width: if content.is_single_value() { "1.5" } else { "1" },
            selection: Selection::None,
            secondary: Vec::new(),
            unreached: Vec::new(),
            aria_label: node_label,
            base_label_len,
            // A named node is called by that name. An unnamed one, having none, is called by its own type instead. See
            // `BoxHandles::ref_name`'s own doc comment.
            ref_name: name.unwrap_or(content.type_name()).to_owned(),
            child: None,
        },
        Rect {
            origin: top_left,
            size: frame.size,
        },
    ))
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The measured size of one cell, and the grid shape around it. Everything else about where a cell sits follows from
/// these by arithmetic, so that arithmetic is pure and lives here, away from the DOM.
struct GridLayoutMetrics {
    /// One cell's own box, the widest value's measured text plus [`CELL_PADDING`] on every side.
    cell: Size,
    /// The padding between the grid box's own left edge and its first column. [`OUTER_PADDING`], or more when row labels
    /// need the room.
    left_pad: f64,
    rows: usize,
    cols: usize,
    /// How many columns make up one group, or `0` for none. See `DataNodeContent::with_column_groups`.
    column_group: usize,
    single_value: bool,
}

impl GridLayoutMetrics {
    /// Measures the widest cell of `content` in the real browser, and builds the metrics from it.
    ///
    /// Finds and formats the one value guaranteed to need the widest rendered cell, without formatting every value just
    /// to compare the resulting text lengths. See `DataNodeContent::widest_cell_string`'s own doc comment for how. Its
    /// own real content is then measured once, via a throwaway element that never becomes one of the group's own
    /// children. It is tracked for rollback like any other fallible-construction element. It is then removed the moment
    /// it has served its purpose, rather than kept around as one of the real cells.
    fn measure(svg: &SvgRoot, guard: &mut RenderGuard, content: &DataNodeContent) -> Result<Self, Error> {
        let mut widest = String::new();
        content.widest_cell_string(&mut widest);

        let measure_el = svg.text(Point::origin(), &widest)?;
        guard.track(measure_el.clone());
        measure_el.set_font_family(GRID_FONT_FAMILY)?;
        measure_el.set_font_size(GRID_FONT_SIZE)?;
        if content.is_plain_text() {
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
        measure_el.remove();
        guard.release();

        // Row labels sit in the grid box's own left padding, so it grows if they need more than it has. A label never gets
        // shorter as its index grows, so the last row's own is the longest.
        let (rows, cols) = content.shape();
        let mut left_pad = OUTER_PADDING;
        if let Some(style) = content.labelling() {
            style.label_into((rows - 1) * cols, &mut widest);
            let label_el = svg.text(Point::origin(), &widest)?;
            guard.track(label_el.clone());
            label_el.set_font_family(GRID_FONT_FAMILY)?;
            label_el.set_font_size(GRID_FONT_SIZE)?;
            left_pad = left_pad.max(label_el.bounding_box()?.size.width + 2.0 * ELEMENT_LABEL_MARGIN);
            label_el.remove();
            guard.release();
        }

        Ok(Self::new(
            Size::new(max_width + 2.0 * CELL_PADDING, CELL_HEIGHT + 2.0 * CELL_PADDING),
            left_pad,
            (rows, cols),
            content.column_group(),
            content.is_single_value(),
        ))
    }

    fn new(cell: Size, left_pad: f64, (rows, cols): (usize, usize), column_group: usize, single_value: bool) -> Self {
        Self {
            cell,
            left_pad,
            rows,
            cols,
            column_group,
            single_value,
        }
    }

    /// How many wider gaps separate one group of columns from the next, among the first `cols` columns. None without
    /// groups, and never one after the last group.
    fn group_gaps(&self, cols: usize) -> usize {
        cols.saturating_sub(1).checked_div(self.column_group).unwrap_or(0)
    }

    /// The content box's own size. A single value is just its one cell. A grid is every cell, the gaps between them, and
    /// [`OUTER_PADDING`] all round.
    #[allow(clippy::cast_precision_loss)]
    fn content_size(&self) -> Size {
        if self.single_value {
            return self.cell;
        }
        Size::new(
            self.cols as f64 * self.cell.width
                + (self.cols as f64 - 1.0) * CELL_GAP
                + self.group_gaps(self.cols) as f64 * GROUP_GAP
                + self.left_pad
                + OUTER_PADDING,
            self.rows as f64 * self.cell.height + (self.rows as f64 - 1.0) * CELL_GAP + 2.0 * OUTER_PADDING,
        )
    }

    /// Where cell `i` of a grid sits, given the content box's own origin. The cell's own flat, row-major index is `i`.
    #[allow(clippy::cast_precision_loss)]
    fn cell_origin(&self, content_origin: Point, i: usize) -> Point {
        let (row, col) = (i / self.cols, i % self.cols);
        Point::new(
            content_origin.x
                + self.left_pad
                + col as f64 * (self.cell.width + CELL_GAP)
                + self.group_gaps(col + 1) as f64 * GROUP_GAP,
            content_origin.y + OUTER_PADDING + row as f64 * (self.cell.height + CELL_GAP),
        )
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// What [`draw_name_frame`] drew: where the content box sits, the node's own overall size, and the wrapping box if the
/// node is named.
struct NameFrame {
    /// Where the content box sits, local to the group. `(0, 0)` unless `name` wraps it in a further named outer box. In
    /// that case it is inset and centred under that outer box's own label row instead.
    content_origin: Point,
    /// The node's own overall size. That of the outer box if there is one, otherwise the content box's own.
    size: Size,
    /// The wrapping box's own `<rect>`, `Some` only when `name` draws one.
    outer_el: Option<SvgNode>,
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Draws the named wrapper, if `name` is given: an outer box with `name` in a label row above where the content box
/// will sit. Without a name there is nothing to draw, and the content box starts flush at `(0, 0)`.
///
/// Same competition `draw_operator_box` resolves for its own label vs. value cell. The content box's own width, plus
/// [`OUTER_PADDING`] clear on either side, competes against the label's own width, plus [`CELL_PADDING`]. Whichever
/// needs more room sets the outer box's own width. Either way the content box itself never reaches the outer box's own
/// left/right edges.
fn draw_name_frame(
    svg: &SvgRoot,
    scratch: &mut String,
    group: &SvgNode,
    guard: &mut RenderGuard,
    name: Option<&str>,
    content_size: Size,
) -> Result<NameFrame, Error> {
    let origin = Point::origin();
    let Some(name) = name else {
        return Ok(NameFrame {
            content_origin: origin,
            size: content_size,
            outer_el: None,
        });
    };

    let label_el = svg.text(origin, name)?;
    guard.track(label_el.clone());
    label_el.set_text_anchor(TextAnchor::Middle)?;
    label_el.set_dominant_baseline(DominantBaseline::Middle)?;
    label_el.set_font_size(LABEL_FONT_SIZE)?;
    label_el.set_fill(TEXT_FILL)?;
    let label_width = label_el.bounding_box()?.size.width;

    let box_width = (label_width + 2.0 * CELL_PADDING).max(content_size.width + 2.0 * OUTER_PADDING);
    let box_size = Size::new(box_width, LABEL_ROW_HEIGHT + content_size.height + OUTER_PADDING);

    let outer_el = svg.rect(origin, box_size)?;
    guard.track(outer_el.clone());
    outer_el.set_fill(NAMED_BOX_FILL)?;
    outer_el.set_stroke(BOX_STROKE)?;
    outer_el.set_stroke_width(1.5)?;
    group.append(&outer_el)?;
    guard.release();

    label_el.set_attr_display(scratch, "x", box_width / 2.0)?;
    label_el.set_attr_display(scratch, "y", LABEL_ROW_HEIGHT / 2.0)?;
    group.append(&label_el)?;
    guard.release();

    Ok(NameFrame {
        content_origin: Point::new((box_width - content_size.width) / 2.0, LABEL_ROW_HEIGHT),
        size: box_size,
        outer_el: Some(outer_el),
    })
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Draws the content box itself. A single value fills it with the value's own type colour. A grid leaves it the plain
/// box colour, for its cells to sit in.
fn draw_content_rect(
    svg: &SvgRoot,
    group: &SvgNode,
    guard: &mut RenderGuard,
    content: &DataNodeContent,
    content_origin: Point,
    content_size: Size,
) -> Result<SvgNode, Error> {
    let rect_el = svg.rect(content_origin, content_size)?;
    guard.track(rect_el.clone());
    rect_el.set_fill(if content.is_single_value() { content.type_colour() } else { PLAIN_BOX_FILL })?;
    rect_el.set_stroke(BOX_STROKE)?;
    rect_el.set_stroke_width(1.5)?;
    group.append(&rect_el)?;
    guard.release();
    Ok(rect_el)
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// What [`draw_cells`] drew, flat and in row-major order.
struct DrawnCells {
    /// Every cell's own `<rect>`. For a single value, the content box itself.
    rects: Vec<SvgNode>,
    /// Every cell's own `<text>`.
    texts: Vec<SvgNode>,
    /// Every cell's own box, local to the group.
    geometry: Vec<Rect>,
    /// The one value's own formatted text, for a single value. Empty for a grid, whose own `aria-label` names a value
    /// count instead. See [`describe`].
    single_value_text: String,
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Formats and places each cell, streaming. See [`draw_content_box`]'s own doc comment for why nothing is collected up
/// front. `text_scratch` is reused for every cell's own formatted text.
///
/// A single value has no inner box, so its text sits centred in `content_rect_el` itself. Each value of a grid gets
/// its own `<rect>`, positioned by [`GridLayoutMetrics::cell_origin`], and its text is given an accessible name that
/// carries its row and column. A bare digit string, read on its own, says nothing about which row/column it belongs to.
/// That relationship exists only in the cell's own `x`/`y`, invisible to assistive technology.
#[allow(clippy::too_many_arguments)]
fn draw_cells(
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
    let type_colour = content.type_colour();
    let mut cells = DrawnCells {
        rects: Vec::with_capacity(if single_value { 1 } else { len }),
        texts: Vec::with_capacity(len),
        geometry: Vec::with_capacity(len),
        single_value_text: String::new(),
    };
    if single_value {
        cells.rects.push(content_rect_el.clone());
        cells.geometry.push(Rect {
            origin: content_origin,
            size: layout.cell,
        });
    }

    let mut text_scratch = String::new();
    let mut label_scratch = String::new();
    let mut error: Option<Error> = None;
    content.for_each_cell_string(&mut text_scratch, |i, cell_text| {
        if error.is_some() {
            return;
        }
        let result = (|| -> Result<(), Error> {
            let text = svg.text(Point::origin(), cell_text)?;
            guard.track(text.clone());
            text.set_text_anchor(TextAnchor::Middle)?;
            text.set_dominant_baseline(DominantBaseline::Middle)?;
            text.set_font_family(GRID_FONT_FAMILY)?;
            text.set_font_size(GRID_FONT_SIZE)?;
            text.set_fill(TEXT_FILL)?;
            if content.is_plain_text() {
                text.set_attr("style", "white-space: pre")?;
            }
            cells.texts.push(text.clone());

            if single_value {
                cells.single_value_text.push_str(cell_text);
                text.set_attr_display(scratch, "x", content_origin.x + layout.cell.width / 2.0)?;
                text.set_attr_display(scratch, "y", content_origin.y + layout.cell.height / 2.0)?;
                group.append(&text)?;
                guard.release();
            } else {
                let cell_origin = layout.cell_origin(content_origin, i);

                let cell_rect = svg.rect(cell_origin, layout.cell)?;
                guard.track(cell_rect.clone());
                cell_rect.set_fill(type_colour)?;
                cell_rect.set_stroke(BOX_STROKE)?;
                cell_rect.set_stroke_width(1.0)?;
                group.append(&cell_rect)?;
                guard.release();

                text.set_attr_display(scratch, "x", cell_origin.x + layout.cell.width / 2.0)?;
                text.set_attr_display(scratch, "y", cell_origin.y + layout.cell.height / 2.0)?;
                let (row, col) = (i / layout.cols, i % layout.cols);
                match content.labelling() {
                    Some(style) => {
                        style.label_into(i, &mut label_scratch);
                        if col == 0 {
                            draw_row_label(svg, scratch, group, guard, &label_scratch, cell_origin, layout.cell)?;
                        }
                        text.set_attr_display(
                            scratch,
                            "aria-label",
                            format_args!("element {label_scratch}, row {row}, column {col}: {cell_text}"),
                        )?;
                    },
                    None => text.set_attr_display(
                        scratch,
                        "aria-label",
                        format_args!("row {row}, column {col}: {cell_text}"),
                    )?,
                }
                group.append(&text)?;
                guard.release();

                cells.rects.push(cell_rect);
                cells.geometry.push(Rect {
                    origin: cell_origin,
                    size: layout.cell,
                });
            }
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
/// Draws row label `label` just left of the first cell of its row, at `cell_origin`, outside the cell and vertically
/// centred on it. It is hidden from assistive technology, because the cell's own value text already names the element
/// in its `aria-label`.
fn draw_row_label(
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
    let value = if content.is_single_value() {
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
