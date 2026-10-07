//! Data nodes — a grid of typed values instead of a plain text label, and their own cell/row/column
//! [`Selection`](crate::scene::Selection) highlighting. See `super::content` for the pure grid-shape/formatting logic;
//! everything DOM-specific (rendering the grid, sizing the box to fit it, recolouring cells) lives here. See
//! [`super::plain`] for a node with a plain text label instead, and [`super::operator`] for one labelled with the
//! operation that produced its own value.

use super::{
    CELL_HEIGHT, CELL_PADDING, EdgeAnchors, GRID_FONT_FAMILY, GRID_FONT_SIZE, LABEL_FONT_SIZE, LABEL_ROW_HEIGHT,
    NodeOptions, OUTER_PADDING, render_guard::RenderGuard, validate_data_content, validate_edge_anchors,
};
use crate::{
    colours::{
        BOX_STROKE, NAMED_BOX_FILL, PLAIN_BOX_FILL, SELECTION_BAND, SELECTION_FOCUS, SELECTION_SECONDARY,
        SELECTION_SECONDARY_STROKE, TEXT_FILL,
    },
    error::Error,
    model::{
        content::ResolvedBand,
        node::{NodeContent, NodeId},
    },
    scene::{BoxHandles, DataNodeContent, NodeValues, Scene, Selection},
};
use svg_dom::{
    DominantBaseline, SvgRoot, TextAnchor,
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

/// `Scene::set_selection`'s own row/column-level stroke width, thicker than every cell's own default border (see
/// [`BoxHandles::cell_stroke_width`]).
///
/// Colour alone is not a reliable channel: it conveys nothing to assistive technology, and can be hard to tell apart
/// for a colour-blind reader. A band is therefore also distinguishable by its own thicker border, the same "not colour
/// alone" reasoning [`NodeValues::type_colour`](crate::model::content::NodeValues::type_colour)'s own
/// `<title>`/`aria-label` pairing already follows.
///
/// Already formatted — see [`BoxHandles::cell_stroke_width`]'s own doc comment for why.
const SELECTION_BAND_STROKE_WIDTH: &str = "2";

/// `Scene::set_selection`'s own cell-level stroke width, thicker again than [`SELECTION_BAND_STROKE_WIDTH`], so the
/// focused cell stays visually distinct from a plain band even with colour perception set aside entirely.
///
/// Already formatted, for the same reason [`SELECTION_BAND_STROKE_WIDTH`] is.
const SELECTION_FOCUS_STROKE_WIDTH: &str = "3.5";

/// A secondary-selected cell's own stroke width, between a plain cell's own border and [`SELECTION_BAND_STROKE_WIDTH`].
const SELECTION_SECONDARY_STROKE_WIDTH: &str = "2";

/// The dash pattern a secondary-selected cell's own outline is drawn with. Every other cell is `"none"`, a solid line.
const SELECTION_SECONDARY_DASH: &str = "5 3";

/// Every attribute `Scene::set_selection`/`Scene::set_secondary_selection` ever write to a cell's own `<rect>`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct CellStyle {
    fill: &'static str,
    stroke_width: &'static str,
    stroke: &'static str,
    dash: &'static str,
}

impl CellStyle {
    /// Writes this style onto `cell`.
    fn apply(self, cell: &svg_dom::SvgNode) -> Result<(), svg_dom::Error> {
        cell.set_fill(self.fill)?;
        cell.set_attr("stroke-width", self.stroke_width)?;
        cell.set_attr("stroke", self.stroke)?;
        cell.set_attr("stroke-dasharray", self.dash)
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Cell `i`'s own style under a resolved `focus`/`band` and a `secondary` flag, against `base_colour`/
/// `base_stroke_width` for a cell none of them names.
///
/// Precedence: the focused cell, then a secondary one, then a banded one, then the default. So a cell that is both
/// focused and secondary shows as focused. Each of the three is a flag, not an index list: the caller already knows
/// whether its cell is a member, and this stays allocation-free.
///
/// `Scene::set_selection` and `Scene::set_secondary_selection` call this twice — once for the old state, once for the
/// new one — for each cell they visit, and only write to the DOM when the two results differ. Each visits only the
/// cells a changed old/new focus, band, or secondary set could plausibly affect, not every cell in the grid.
fn cell_style(
    focus: bool,
    banded: bool,
    secondary: bool,
    base_colour: &'static str,
    base_stroke_width: &'static str,
) -> CellStyle {
    let (fill, stroke_width, stroke, dash) = if focus {
        (SELECTION_FOCUS, SELECTION_FOCUS_STROKE_WIDTH, BOX_STROKE, "none")
    } else if secondary {
        (
            SELECTION_SECONDARY,
            SELECTION_SECONDARY_STROKE_WIDTH,
            SELECTION_SECONDARY_STROKE,
            SELECTION_SECONDARY_DASH,
        )
    } else if banded {
        (SELECTION_BAND, SELECTION_BAND_STROKE_WIDTH, BOX_STROKE, "none")
    } else {
        (base_colour, base_stroke_width, BOX_STROKE, "none")
    };
    CellStyle {
        fill,
        stroke_width,
        stroke,
        dash,
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Draws a data node's rectangle and its grid of value cells, grouped under one `<g>`, and returns their handles
/// alongside the box's own final `Rect` — computed here, not supplied by the caller.
///
/// Every value gets its own `<text>` element (monospace — see [`GRID_FONT_FAMILY`]). Under a monospace font, character
/// count alone determines a cell's own rendered width. Every string [`DataNodeContent::cells`]/
/// [`DataNodeContent::for_each_cell_string`] produces is ASCII, so byte length already is character count. So only the
/// widest cell's own text is ever read back via [`SvgNode::bounding_box`](svg_dom::SvgNode::bounding_box) — the same
/// "measure, don't estimate" approach [`shrink_label_to_fit`](super::plain::shrink_label_to_fit) already uses for plain
/// labels, applied once per node, not once per cell. Every cell then shares that one measured width plus
/// [`CELL_PADDING`], so the grid's rows and columns still line up even when [`DataFormat::Decimal`] values differ in
/// digit count.
///
/// Formats and places each cell streaming — never collecting a `Vec<String>` of every cell's own text, or a
/// `Vec<SvgNode>` of every `<text>` element, regardless of how many values `content` holds, and never formatting any
/// value twice:
///
/// 1. [`DataNodeContent::widest_cell_string`] identifies and formats the one value guaranteed to need the widest cell,
///    without formatting every value first — see that method's own doc comment for how. A throwaway element built from
///    it, once `cell_size` is known, is all `bounding_box()` ever needs; which specific value that was is otherwise
///    irrelevant, since any string of the same length would measure identically under a monospace font.
/// 2. [`DataNodeContent::for_each_cell_string`] then formats every value once, reusing one buffer, and this time
///    creates each cell's own `<text>` (and, for a multi-value grid, its own `<rect>`) directly at its final position,
///    appending each immediately rather than deferring every cell's own placement to a later pass. The one value pass 1
///    already formatted is formatted again here, along with every other — a single value's worth of repeated work, not
///    repeated for the whole node.
///
/// [`DataNodeContent::is_single_value`] decides which of two layouts is drawn:
///
/// - A single value has no sibling to be told apart from, so it gets no inner cell box at all — the node's own
///   `rect_el` is filled directly with [`NodeValues::type_colour`], and the value's text sits centred in it.
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
/// each cell — create, style, append, immediately — keeps the window this matters for down to one cell (two nodes,
/// briefly, for a multi-value grid's own rect-then-text pair) at a time, via [`RenderGuard::release`], rather than
/// every cell created so far staying tracked until the whole node finishes.
///
/// `scratch` is a caller-owned buffer — `SceneInner::scratch`, in every real caller — reused for this call's own
/// per-cell `x`/`y`/`transform` formatting, the same reasoning [`draw_box`](super::plain::draw_box)'s own `scratch`
/// parameter follows. This is a distinct concern from pass 1/2's own per-value formatting buffer above, which holds
/// cell *content*, not attribute values, and stays a plain local: nothing outside a single `draw_content_box` call ever
/// needs it.
///
/// `name`, when `Some`, wraps the box described above in a further outer box of its own, with `name` in a label row
/// above it — the same "outer box labelled with a name, inset value box beneath it" shape
/// [`draw_operator_box`](super::operator::draw_operator_box) already draws for an operator's own result. The returned
/// `Rect` is then that outer box's own, so an incoming connector anchors to it, never to the inner (unlabelled, exactly
/// as drawn below) content box directly — see [`draw_operator_box`]'s own module doc comment for why a connector must
/// never land on an inset inner box. `name` is `None` for every plain (unnamed) data node, which renders exactly as
/// before this parameter existed: no label row, no further outer box, the content box itself starting flush at local
/// `(0, 0)`.
// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Returns [`Error::EmptyNodeName`] if `name` is empty or holds only whitespace — see that variant's own doc comment
/// for why a blank name is rejected outright, rather than merely drawing an oddly-worded label. Shared by every path
/// that draws a named data node — construction and measurement alike.
fn validate_node_name(name: &str) -> Result<(), Error> {
    if name.trim().is_empty() {
        return Err(Error::EmptyNodeName);
    }
    Ok(())
}

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
    // At most two nodes are ever loose (created but not yet appended) at once here — see `RenderGuard::release`'s own
    // doc comment — regardless of how many values `content` holds, or whether `name` is given: each of the name label's
    // own text/outer-box elements below is released again, in turn, before the next is created.
    let mut guard = RenderGuard::new(group.clone());
    let type_colour = content.type_colour();
    let type_name = content.type_name();
    let origin = Point::origin();
    let len = content.len();
    let (grid_rows, grid_cols) = content.shape();
    let single_value = content.is_single_value();
    // How many wider gaps separate one group of columns from the next: none without groups, and never one after the
    // last group.
    let column_group = content.column_group();
    let group_gaps = |cols: usize| cols.saturating_sub(1).checked_div(column_group).unwrap_or(0);

    // Finds and formats the one value guaranteed to need the widest rendered cell — see
    // `DataNodeContent::widest_cell_string`'s own doc comment for how, without formatting every value just to compare
    // the resulting text lengths.
    let mut widest = String::new();
    content.widest_cell_string(&mut widest);

    // `widest`'s own real content is measured once, via a throwaway element that never becomes one of `group`'s own
    // children: tracked for rollback like any other fallible-construction element, then removed the moment it has
    // served its purpose, rather than kept around as one of the real cells.
    let measure_el = svg.text(origin, &widest)?;
    guard.track(measure_el.clone());
    measure_el.set_font_family(GRID_FONT_FAMILY)?;
    measure_el.set_font_size(GRID_FONT_SIZE)?;
    let max_width = measure_el.bounding_box()?.size.width;
    measure_el.remove();
    guard.release();

    let cell_size = Size::new(max_width + 2.0 * CELL_PADDING, CELL_HEIGHT + 2.0 * CELL_PADDING);

    let content_size = if single_value {
        cell_size
    } else {
        #[allow(clippy::cast_precision_loss)]
        Size::new(
            grid_cols as f64 * cell_size.width
                + (grid_cols as f64 - 1.0) * CELL_GAP
                + group_gaps(grid_cols) as f64 * GROUP_GAP
                + 2.0 * OUTER_PADDING,
            grid_rows as f64 * cell_size.height + (grid_rows as f64 - 1.0) * CELL_GAP + 2.0 * OUTER_PADDING,
        )
    };

    // `content_origin` is where the content box drawn below sits, local to `group` — `(0, 0)` exactly as before this
    // parameter existed, unless `name` wraps it in a further named outer box, in which case it is inset and centred
    // under that outer box's own label row instead. `named_outer_rect` is that wrapping box's own `<rect>` — `Some`
    // only when `name` draws one — read back below once `rect_el` is in scope too, to decide `BoxHandles::outer_rect`:
    // the named wrapper when there is one, `rect_el` itself otherwise.
    let (content_origin, size, named_outer_rect) = if let Some(name) = name {
        let label_el = svg.text(origin, name)?;
        guard.track(label_el.clone());
        label_el.set_text_anchor(TextAnchor::Middle)?;
        label_el.set_dominant_baseline(DominantBaseline::Middle)?;
        label_el.set_font_size(LABEL_FONT_SIZE)?;
        label_el.set_fill(TEXT_FILL)?;
        let label_width = label_el.bounding_box()?.size.width;

        // Same competition `draw_operator_box` resolves for its own label vs. value cell: the content box's own width,
        // plus `OUTER_PADDING` clear on either side, against the label's own width, plus `CELL_PADDING` — whichever
        // needs more room sets the outer box's own width. Either way the content box itself never reaches the outer
        // box's own left/right edges.
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

        (
            Point::new((box_width - content_size.width) / 2.0, LABEL_ROW_HEIGHT),
            box_size,
            Some(outer_el),
        )
    } else {
        (origin, content_size, None)
    };
    let rect = Rect { origin: top_left, size };

    let rect_el = svg.rect(content_origin, content_size)?;
    guard.track(rect_el.clone());
    rect_el.set_fill(if single_value { type_colour } else { PLAIN_BOX_FILL })?;
    rect_el.set_stroke(BOX_STROKE)?;
    rect_el.set_stroke_width(1.5)?;
    group.append(&rect_el)?;
    guard.release();
    let outer_rect = named_outer_rect.unwrap_or_else(|| rect_el.clone());

    // Streamed rendering pass — see this function's own doc comment. `text_scratch` is reused for every cell's own
    // formatted text.
    let mut text_scratch = String::new();
    let mut cell_rects = Vec::with_capacity(if single_value { 1 } else { len });
    let mut cell_texts = Vec::with_capacity(len);
    if single_value {
        cell_rects.push(rect_el.clone());
    }

    // Captured only in the `single_value` case — the same formatted text that one cell's own `<text>` renders, reused
    // again below for the node's own `aria-label`, so the value reads as text without visiting the cell directly. Left
    // empty for a multi-value grid, whose own `aria-label` names a value count instead — see that `node_label` match
    // below.
    let mut single_value_text = String::new();

    let mut error: Option<Error> = None;
    content.for_each_cell_string(&mut text_scratch, |i, cell_text| {
        if error.is_some() {
            return;
        }
        let result = (|| -> Result<(), Error> {
            let text = svg.text(origin, cell_text)?;
            guard.track(text.clone());
            text.set_text_anchor(TextAnchor::Middle)?;
            text.set_dominant_baseline(DominantBaseline::Middle)?;
            text.set_font_family(GRID_FONT_FAMILY)?;
            text.set_font_size(GRID_FONT_SIZE)?;
            text.set_fill(TEXT_FILL)?;
            cell_texts.push(text.clone());

            if single_value {
                single_value_text.push_str(cell_text);
                text.set_attr_display(scratch, "x", content_origin.x + cell_size.width / 2.0)?;
                text.set_attr_display(scratch, "y", content_origin.y + cell_size.height / 2.0)?;
                group.append(&text)?;
                guard.release();
            } else {
                #[allow(clippy::cast_precision_loss)]
                let (row, col) = (i / grid_cols, i % grid_cols);
                #[allow(clippy::cast_precision_loss)]
                let cell_origin = Point::new(
                    content_origin.x
                        + OUTER_PADDING
                        + col as f64 * (cell_size.width + CELL_GAP)
                        + group_gaps(col + 1) as f64 * GROUP_GAP,
                    content_origin.y + OUTER_PADDING + row as f64 * (cell_size.height + CELL_GAP),
                );

                let cell_rect = svg.rect(cell_origin, cell_size)?;
                guard.track(cell_rect.clone());
                cell_rect.set_fill(type_colour)?;
                cell_rect.set_stroke(BOX_STROKE)?;
                cell_rect.set_stroke_width(1.0)?;
                group.append(&cell_rect)?;
                guard.release();

                text.set_attr_display(scratch, "x", cell_origin.x + cell_size.width / 2.0)?;
                text.set_attr_display(scratch, "y", cell_origin.y + cell_size.height / 2.0)?;
                // A bare digit string, read on its own, says nothing about which row/column it belongs to — that
                // relationship exists only in `cell_origin`'s own `x`/`y`, invisible to assistive technology. This
                // overrides `text`'s own default accessible name (its rendered digits) with its row and column too.
                text.set_attr_display(scratch, "aria-label", format_args!("row {row}, column {col}: {cell_text}"))?;
                group.append(&text)?;
                guard.release();

                cell_rects.push(cell_rect);
            }
            Ok(())
        })();
        if let Err(e) = result {
            error = Some(e);
        }
    });
    if let Some(e) = error {
        return Err(e);
    }

    // See `draw_box`'s own comment on its matching call for why `set_transform_fmt`, not `set_translate`.
    group.set_transform_fmt(scratch, format_args!("translate({}, {})", top_left.x, top_left.y))?;

    // This names the whole node for assistive technology, and — via `<title>` below — for the browser's own mouse-hover
    // tooltip too. Assistive technology usually announces a group before its children, so a reader need not visit every
    // individual cell to learn the node's own name, type, or (for a single value) its real value.
    //
    // `aria-label` only names an element whose role supports naming. A bare `<g>` has no implicit role, so its
    // `aria-label` may go unexposed without an explicit `role="group"` alongside it. `group` was chosen over `img`
    // deliberately: `img` presents its descendants as one atomic image, hiding the individual cell values an assistive
    // technology user could otherwise still reach.
    //
    // A multi-value grid's own row/column shape is spelled out here too — "2 rows by 3 columns". Not just left implicit
    // in each cell's own `x`/`y` position, which conveys nothing to assistive technology. Each cell's own `aria-label`,
    // set below, names its row and column directly, the same "not visual position alone" reasoning.
    let row_word = if grid_rows == 1 { "row" } else { "rows" };
    let col_word = if grid_cols == 1 { "column" } else { "columns" };
    let node_label = match (name, single_value) {
        (Some(name), true) => format!("{name}: {type_name} = {single_value_text}"),
        (Some(name), false) => format!(
            "{name}: {type_name} data grid, {grid_rows} {row_word} by {grid_cols} {col_word}, {} values",
            content.len()
        ),
        (None, true) => format!("{type_name} = {single_value_text}"),
        (None, false) => format!(
            "{type_name} data grid, {grid_rows} {row_word} by {grid_cols} {col_word}, {} values",
            content.len()
        ),
    };
    // A named node is called by that name. An unnamed one, having none, is called by its own type instead. See
    // `BoxHandles::ref_name`'s own doc comment.
    let ref_name = name.unwrap_or(type_name).to_owned();
    group.set_attr("role", "group")?;
    group.set_attr("aria-label", &node_label)?;

    // A `<title>` is only a native tooltip/accessible name for its own direct parent, not for a sibling. So it belongs
    // on `group`, the one element every rect and every text drawn above actually shares as a parent. It does not belong
    // on any individual cell's own rect. That rect is a sibling of that cell's text, not an ancestor of it. The two
    // would never share the tooltip that way. This is exactly why an earlier version of this function attached a
    // `<title>` to each rect/text individually. That version still failed to show a tooltip over the rendered digits.
    // Putting the title on `group` instead also avoids a different problem: a `<title>` as one of `text`'s own DOM
    // children would leak its text into `text.textContent`. That would mix the title text in with the actual rendered
    // digits.
    //
    // Set to `node_label` — the same text `aria-label` carries — so the browser's own mouse-hover tooltip reads exactly
    // what a screen reader announces, not just the node's own type. `Scene::set_selection` keeps the two in sync
    // afterward too, rewriting this alongside `aria-label` on every selection change.
    group.set_title(&node_label)?;

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
            outer_rect,
            cell_rects,
            cell_texts,
            cell_stroke_width: if single_value { "1.5" } else { "1" },
            selection: Selection::None,
            secondary: Vec::new(),
            aria_label: node_label,
            base_label_len,
            ref_name,
            child: None,
        },
        rect,
    ))
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
impl Scene {
    /// Adds a data node to the graph — one whose visible content is `content`'s own grid of values (see
    /// [`DataNodeContent`]) rather than a plain text label — and returns its id.
    ///
    /// Unlike [`add_node`](Self::add_node), there is no `size` parameter: the box is always sized to fit `content`'s
    /// rendered grid exactly — see [`DataNodeContent`]'s own doc comment for the layout and formatting rules, and
    /// `draw_content_box`'s own doc comment for how the fit is computed.
    ///
    /// Equivalent to [`add_data_node_with`](Self::add_data_node_with) with [`NodeOptions::default`].
    ///
    /// # Errors
    ///
    /// Returns [`Error::EmptyNodeContent`] if `content` holds no values — there is no grid to draw.
    ///
    /// Returns [`Error::InvalidGridLayout`] if `content`'s own [`GridLayout`](crate::scene::GridLayout) wraps `0`.
    ///
    /// Returns [`Error::InvalidNodeGeometry`] if `top_left`'s coordinates are not finite.
    pub fn add_data_node(&self, top_left: Point, content: DataNodeContent) -> Result<NodeId, Error> {
        self.add_data_node_with(top_left, content, NodeOptions::default())
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    /// Adds a data node to the graph, as [`add_data_node`](Self::add_data_node), but with `options` controlling how
    /// many connector fixing points this node's sides offer — see [`EdgeAnchors`].
    ///
    /// # Errors
    ///
    /// Returns [`Error::InvalidEdgeAnchors`] if `options.edge_anchors` is `Some(EdgeAnchors(0))`. Checked before
    /// drawing anything or touching the graph's model, so a rejected call leaves the scene exactly as it was.
    ///
    /// Returns [`Error::EmptyNodeContent`] if `content` holds no values. Also checked before drawing anything.
    ///
    /// Returns [`Error::InvalidGridLayout`] if `content`'s own [`GridLayout`](crate::scene::GridLayout) wraps `0` —
    /// `Columns(0)`, `Rows(0)`, or `MaxColumns(0)`. Also checked before drawing anything.
    ///
    /// Returns [`Error::InvalidNodeGeometry`] if `top_left`'s coordinates are not finite. Unlike
    /// [`add_node_with`](Self::add_node_with), there is no caller-supplied size to validate — the box is always sized
    /// to fit `content`.
    pub fn add_data_node_with(
        &self,
        top_left: Point,
        content: DataNodeContent,
        options: NodeOptions,
    ) -> Result<NodeId, Error> {
        self.add_data_node_with_impl(top_left, None, content, options)
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    /// Gives a raw value a variable name. Adds a data node to the graph, exactly as
    /// [`add_data_node`](Self::add_data_node), but wrapped in a further outer box of its own labelled `name`. The same
    /// shape an "outer box labelled with a name with an inset value box beneath it" already drawn for an operator
    /// node's own result.
    ///
    /// Equivalent to [`add_named_data_node_with`](Self::add_named_data_node_with) with [`NodeOptions::default`].
    ///
    /// # Errors
    ///
    /// See [`add_named_data_node_with`](Self::add_named_data_node_with)'s own `# Errors` section.
    pub fn add_named_data_node(&self, top_left: Point, name: &str, content: DataNodeContent) -> Result<NodeId, Error> {
        self.add_named_data_node_with(top_left, name, content, NodeOptions::default())
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    /// Adds a named data node, as [`add_named_data_node`](Self::add_named_data_node), but with `options` controlling
    /// how many connector fixing points this node's sides offer — see [`EdgeAnchors`].
    ///
    /// An incoming connector still anchors to this node's own *outer* (named) box, never to the inner content box
    /// `name` wraps — the same reasoning [`add_binary_operator_node`](Self::add_binary_operator_node)'s own module doc
    /// comment gives for why a connector must never land on an inset inner box.
    ///
    /// `name` is not stored in the graph's own model — unlike [`add_node`](Self::add_node)'s own `label`, it exists
    /// only to draw this one label row, the same way an operator's own label (`"ADD"`, `"XOR"`, …) is never stored
    /// either. Query this node's own value(s) back through `content` itself, exactly as for an unnamed data node.
    ///
    /// # Errors
    ///
    /// Returns [`Error::InvalidEdgeAnchors`] if `options.edge_anchors` is `Some(EdgeAnchors(0))`. Checked before
    /// drawing anything or touching the graph's model, so a rejected call leaves the scene exactly as it was.
    ///
    /// Returns [`Error::EmptyNodeName`] if `name` is empty, or holds only whitespace. Also checked before drawing
    /// anything — see [`Error::EmptyNodeName`]'s own doc comment for why a blank name is rejected outright, rather than
    /// merely drawing an oddly-worded label.
    ///
    /// Returns [`Error::EmptyNodeContent`] if `content` holds no values. Also checked before drawing anything.
    ///
    /// Returns [`Error::InvalidGridLayout`] if `content`'s own [`GridLayout`](crate::scene::GridLayout) wraps `0` —
    /// `Columns(0)`, `Rows(0)`, or `MaxColumns(0)`. Also checked before drawing anything.
    ///
    /// Returns [`Error::InvalidNodeGeometry`] if `top_left`'s coordinates are not finite. Unlike
    /// [`add_node_with`](Self::add_node_with), there is no caller-supplied size to validate — the box is always sized
    /// to fit `content` and `name` together.
    pub fn add_named_data_node_with(
        &self,
        top_left: Point,
        name: &str,
        content: DataNodeContent,
        options: NodeOptions,
    ) -> Result<NodeId, Error> {
        self.add_data_node_with_impl(top_left, Some(name), content, options)
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    /// The `Size` at which an unnamed data node showing `content` would render in *this* `Scene`. This is exactly what
    /// [`add_data_node`](Self::add_data_node) followed by [`Scene::node_rect`] on the result would report, without ever
    /// adding `content` to the graph to find out.
    ///
    /// To ensure that the same styling context is used, the node is drawn into this `Scene`'s own `SvgRoot`, then the
    /// result is measured, and the node is removed again before returning.
    ///
    /// Structurally, nothing about this call persists: no [`NodeId`] is returned because nothing remains to address
    /// afterward, no graph node is created, no node handle is registered, and no edge or accessibility/navigation state
    /// is touched. The drawn content itself exists only for the instant between `draw_content_box` returning and this
    /// function removing it again — in practice never visible, selectable, or reachable by assistive technology, though
    /// that stronger claim about transient browser behaviour is not itself something a test here establishes, only the
    /// structural absence above.
    ///
    /// This is most useful for sizing content that cannot be known until runtime. Character count is not equivalent to
    /// rendered width in general, so this always performs a real measurement rather than attempting a best guess.
    ///
    /// # Errors
    ///
    /// Returns [`Error::EmptyNodeContent`] if `content` holds no values, or [`Error::InvalidGridLayout`] if its own
    /// [`GridLayout`](crate::scene::GridLayout) wraps `0` — the identical validation
    /// [`add_data_node`](Self::add_data_node) already applies, not a second copy of it.
    pub fn measure_data_node(&self, content: &DataNodeContent) -> Result<Size, Error> {
        self.measure_data_node_impl(None, content)
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    /// The named-node counterpart to [`measure_data_node`](Self::measure_data_node) — `name`'s own label row is
    /// included in the measured size. See that method's own doc comment for everything else.
    ///
    /// # Errors
    ///
    /// As [`measure_data_node`](Self::measure_data_node), plus [`Error::EmptyNodeName`] if `name` is empty or holds
    /// only whitespace — the same rejection [`add_named_data_node`](Self::add_named_data_node) already applies.
    pub fn measure_named_data_node(&self, name: &str, content: &DataNodeContent) -> Result<Size, Error> {
        self.measure_data_node_impl(Some(name), content)
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    /// Shared implementation behind [`measure_data_node`](Self::measure_data_node) and
    /// [`measure_named_data_node`](Self::measure_named_data_node) — the measurement counterpart to
    /// [`add_data_node_with_impl`](Self::add_data_node_with_impl): the same drawing call, stopped before
    /// `inner.attach`/`inner.graph.add_node`/`inner.insert_node_handle` ever run, with the drawn group removed again
    /// instead.
    fn measure_data_node_impl(&self, name: Option<&str>, content: &DataNodeContent) -> Result<Size, Error> {
        if let Some(name) = name {
            validate_node_name(name)?;
        }
        validate_data_content(content)?;

        let mut inner = self.inner.borrow_mut();
        // See `add_data_node_with_impl`'s own matching comment for why `scratch` is taken out for the call.
        let mut scratch = std::mem::take(&mut inner.scratch);
        let result = draw_content_box(&inner.svg, &mut scratch, Point::origin(), name, content, None);
        inner.scratch = scratch;
        let (handles, rect) = result?;
        handles.group.remove();
        Ok(rect.size)
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    /// Shared implementation behind [`add_data_node_with`](Self::add_data_node_with) and
    /// [`add_named_data_node_with`](Self::add_named_data_node_with) — every check, and the rendered box itself (modulo
    /// `name`'s own outer wrapper), is identical between a plain and a named data node.
    ///
    /// See either public wrapper's own doc comment for the full contract this enforces.
    fn add_data_node_with_impl(
        &self,
        top_left: Point,
        name: Option<&str>,
        content: DataNodeContent,
        options: NodeOptions,
    ) -> Result<NodeId, Error> {
        validate_edge_anchors(options.edge_anchors)?;
        if let Some(name) = name {
            validate_node_name(name)?;
        }
        validate_data_content(&content)?;
        if !top_left.x.is_finite() || !top_left.y.is_finite() {
            return Err(Error::InvalidNodeGeometry(Rect {
                origin: top_left,
                size: Size::new(0.0, 0.0),
            }));
        }

        let mut inner = self.inner.borrow_mut();
        // See `add_node_with`'s own matching comment for why `scratch` is taken out for the call.
        let mut scratch = std::mem::take(&mut inner.scratch);
        let result = draw_content_box(&inner.svg, &mut scratch, top_left, name, &content, options.edge_anchors);
        inner.scratch = scratch;
        let (handles, rect) = result?;
        inner.attach(&handles.group)?;
        let id = inner.graph.add_node(rect, content);
        inner.insert_node_handle(id, handles);
        Ok(id)
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    /// Highlights node `id`'s own cell(s) per `selection`, and recolours every affected cell immediately.
    ///
    /// `id` must be a [`DataNodeContent`] node. It may be drawn via [`add_data_node`](Self::add_data_node)/
    /// [`add_data_node_with`](Self::add_data_node_with), or be an operator node's own single-value result (see
    /// [`add_unary_operator_node`](Self::add_unary_operator_node)/
    /// [`add_binary_operator_node`](Self::add_binary_operator_node)). A plain label node has no cells to highlight.
    ///
    /// This is the only way to change a node's own selection after it is first drawn. A live "previous"/"next" control
    /// stepping through an array as it is processed is one example.
    ///
    /// [`Selection::None`] clears back to every cell's own default `NodeValues::type_colour`. So there is no need to
    /// clear before setting a new selection.
    ///
    /// An identical `selection` to `id`'s own current one is an immediate no-op — no cell is touched, and no
    /// `aria-label` write happens. Otherwise, only the cells whose own colour/stroke category (focused, banded, or
    /// default) actually changes between the old selection and the new one are even examined, let alone written to —
    /// the old/new focus cells, plus each band's own members, via `ResolvedBand::for_each_index`. A cell in neither
    /// band, and not a focus either way, is never visited: its category cannot have changed. A live "previous"/"next"
    /// control stepping through an array as it is processed only ever touches a handful of cells per step, however
    /// large the array — this is the hot path that shape is optimised for, in both the DOM writes it performs and the
    /// Rust computation that decides them.
    ///
    /// Also gives the focused cell, and, less strongly, a banded row/column, a thicker stroke than its own default
    /// border. It also rebuilds the node's own `aria-label` to describe the current selection as text. Neither depends
    /// on colour alone — the same reasoning `NodeValues::type_colour`'s own `<title>`/`aria-label` pairing already
    /// follows.
    ///
    /// This updates the node's own accessible name, making the current selection available to assistive technology. It
    /// does not create a live-region announcement, so a screen reader whose virtual cursor sits elsewhere may not
    /// notice the change until the user navigates back to this node. An application needing an immediate announcement
    /// should provide its own status/live region; this method does not.
    ///
    /// # Errors
    ///
    /// Returns [`Error::UnknownNode`] if `id` does not name a node in this scene.
    ///
    /// Returns [`Error::InvalidSelection`] if `id` names a plain label node. Also returns it if `selection` names a
    /// cell/row/column index out of range for `id`'s own actual value count or grid shape. Checked before recolouring
    /// any cell, so a rejected call leaves every cell's own colour exactly as it was.
    ///
    /// Also returns a wrapped [`Error::Svg`] if recolouring a cell fails partway through. A failure here can leave some
    /// cells already recoloured and others not — the same documented property
    /// [`set_edge_anchors`](Self::set_edge_anchors) already carries for its own incident redraws.
    pub fn set_selection(&self, id: NodeId, selection: Selection) -> Result<(), Error> {
        let mut inner = self.inner.borrow_mut();

        let content = match &inner.graph.node(id).ok_or(Error::UnknownNode(id))?.content {
            NodeContent::Data(content) => content,
            NodeContent::Label(_) | NodeContent::Container(_) => return Err(Error::InvalidSelection(id, selection)),
        };
        let (new_band, new_focus) = content
            .resolve_selection(selection)
            .ok_or(Error::InvalidSelection(id, selection))?;
        let base_colour = content.type_colour();

        let old_selection = inner.node_handle(id).ok_or(Error::UnknownNode(id))?.selection;
        if old_selection == selection {
            // No cell needs recolouring, but a selection toolbar just installed against this node (its own commit is
            // exactly a same-as-current `set_selection` call whenever the node's `Selection` already happened to be
            // `Selection::None`) has never had its own button states synced at all — see
            // `sync_selection_toolbar_state`'s own doc comment. Skipping this call here would leave every button with
            // no `aria-disabled`/`opacity` written, not just a stale one.
            let _ = inner.sync_selection_toolbar_state();
            return Ok(());
        }
        // `old_selection` was itself accepted by an earlier, successful `set_selection` call against this same,
        // unchanged content (or is the default `Selection::None`, always valid), so it always resolves here too.
        let (old_band, old_focus) = content.resolve_selection(old_selection).unwrap_or((ResolvedBand::None, None));

        let handles = inner.node_handle_mut(id).ok_or(Error::UnknownNode(id))?;
        let cell_stroke_width = handles.cell_stroke_width;
        let cell_rects = &handles.cell_rects;
        let secondary = &handles.secondary;
        let len = cell_rects.len();

        // Every index whose own category (focused, banded, or default) could possibly differ between the old selection
        // and the new one — never the whole grid, and each visited at most once. A cell outside this set is provably
        // unchanged: it is neither an old/new focus, nor in the symmetric difference of the two bands, so `cell_style`
        // resolves it to the same category either way. See `ResolvedBand::for_each_index`'s own doc comment.
        let mut result = Ok(());
        let mut restyle = |i: usize| {
            if result.is_err() {
                return;
            }
            let Some(cell) = cell_rects.get(i) else { return };
            let is_secondary = secondary.binary_search(&i).is_ok();
            let old_style = cell_style(
                Some(i) == old_focus,
                old_band.contains(i),
                is_secondary,
                base_colour,
                cell_stroke_width,
            );
            let new_style = cell_style(
                Some(i) == new_focus,
                new_band.contains(i),
                is_secondary,
                base_colour,
                cell_stroke_width,
            );
            if new_style == old_style {
                return;
            }
            result = new_style.apply(cell);
        };
        if let Some(i) = old_focus {
            restyle(i);
        }
        // Only if it differs from `old_focus` — otherwise this index was already visited above, and a second visit here
        // would just repeat the same comparison.
        if new_focus != old_focus {
            if let Some(i) = new_focus {
                restyle(i);
            }
        }
        if old_band != new_band {
            // True symmetric difference, not each band walked in full: a member of both bands (their intersection) is
            // skipped in both traversals below, since its own category cannot have changed between them either — and a
            // focus index is skipped here too, since it was already visited, explicitly, above.
            let already_visited = |i: usize| new_band.contains(i) || Some(i) == old_focus || Some(i) == new_focus;
            old_band.for_each_index(len, |i| {
                if !already_visited(i) {
                    restyle(i);
                }
            });
            let already_visited = |i: usize| old_band.contains(i) || Some(i) == old_focus || Some(i) == new_focus;
            new_band.for_each_index(len, |i| {
                if !already_visited(i) {
                    restyle(i);
                }
            });
        }
        result?;

        handles.selection = selection;
        handles.refresh_label()?;

        // The selection has already changed, so a failure to keep a selection toolbar's own button states in sync with
        // it is not reported as this call's own failure — same reasoning as `SceneInner::flush_view`'s own `let _ =
        // self.sync_toolbar_state();`. The next selection change puts it right.
        let _ = inner.sync_selection_toolbar_state();

        Ok(())
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    /// Marks the cells at flat indices `cells` of node `id` as *secondary*: derived from the current selection, and
    /// part of the same step, without being the one cell (or row/column) [`set_selection`](Self::set_selection) names.
    ///
    /// A walk often reads more than the cell it is standing on. SHA3's `Chi` step, standing on `A[x, y]`, also reads
    /// `A[x + 1, y]` and `A[x + 2, y]`; `Pi` writes that same step's value to a cell elsewhere. `cells` names those
    /// derived cells, on this node or on any other data node, so a reader can see everything one step touches.
    ///
    /// # Independent of `Selection`
    ///
    /// This never reads or changes `id`'s own [`Selection`], and [`set_selection`](Self::set_selection) never changes
    /// the secondary cells. A selection toolbar's own stepping is therefore unaffected: it only ever reads the primary
    /// `Selection`. The two are set separately — typically both on every step.
    ///
    /// # Rendering
    ///
    /// A secondary cell has its own teal fill and a dashed outline. Both, since colour alone is not a reliable channel
    /// — the same reasoning [`set_selection`](Self::set_selection) follows for its own thicker borders. Where a cell is
    /// also primary-selected, the primary highlight wins. Where it is also inside a selected row or column, the
    /// secondary one wins.
    ///
    /// The node's own `aria-label` and tooltip gain `", also highlighted: cells 3, 4"`.
    ///
    /// # Replacement
    ///
    /// `cells` replaces whatever was secondary before. An empty slice clears it. Order and duplicates do not matter.
    /// There is no limit on how many cells can be secondary beyond the node's own value count. An identical set to the
    /// current one is an immediate no-op, and otherwise only the cells that enter or leave the set are recoloured.
    ///
    /// # Errors
    ///
    /// Returns [`Error::UnknownNode`] if `id` does not name a node in this scene.
    ///
    /// Returns [`Error::InvalidSelection`] — carrying the offending index as a [`Selection::Cell`] — if `id` names a
    /// plain label or container node, or if any index in `cells` is out of range for `id`'s own value count. Checked
    /// before recolouring any cell, so a rejected call leaves every cell exactly as it was.
    ///
    /// Also returns a wrapped [`Error::Svg`] if recolouring a cell fails partway through, with the same "can leave some
    /// cells already recoloured" property [`set_selection`](Self::set_selection) documents.
    pub fn set_secondary_selection(&self, id: NodeId, cells: &[usize]) -> Result<(), Error> {
        let mut inner = self.inner.borrow_mut();

        let content = match &inner.graph.node(id).ok_or(Error::UnknownNode(id))?.content {
            NodeContent::Data(content) => content,
            NodeContent::Label(_) | NodeContent::Container(_) => {
                return Err(Error::InvalidSelection(id, Selection::None));
            },
        };
        let len = content.len();
        if let Some(&bad) = cells.iter().find(|&&i| i >= len) {
            return Err(Error::InvalidSelection(id, Selection::Cell(bad)));
        }
        let base_colour = content.type_colour();
        let (band, focus) = {
            let selection = inner.node_handle(id).ok_or(Error::UnknownNode(id))?.selection;
            // `selection` was accepted by an earlier call against this same content, so it always resolves.
            content.resolve_selection(selection).unwrap_or((ResolvedBand::None, None))
        };

        let mut new_secondary = cells.to_vec();
        new_secondary.sort_unstable();
        new_secondary.dedup();

        let handles = inner.node_handle_mut(id).ok_or(Error::UnknownNode(id))?;
        if handles.secondary == new_secondary {
            return Ok(());
        }
        let cell_stroke_width = handles.cell_stroke_width;

        // Only a cell that enters or leaves the set can change style, so walk the two sorted lists' own symmetric
        // difference rather than every cell in the grid.
        let old = &handles.secondary;
        let mut changed = Vec::with_capacity(old.len() + new_secondary.len());
        changed.extend(old.iter().filter(|i| new_secondary.binary_search(i).is_err()));
        changed.extend(new_secondary.iter().filter(|i| old.binary_search(i).is_err()));

        let mut result = Ok(());
        for i in changed {
            let Some(cell) = handles.cell_rects.get(i) else { continue };
            let style_with = |secondary: bool| {
                cell_style(Some(i) == focus, band.contains(i), secondary, base_colour, cell_stroke_width)
            };
            let new_style = style_with(new_secondary.binary_search(&i).is_ok());
            if new_style != style_with(old.binary_search(&i).is_ok()) {
                result = new_style.apply(cell);
                if result.is_err() {
                    break;
                }
            }
        }
        result?;

        handles.secondary = new_secondary;
        handles.refresh_label()?;
        Ok(())
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    /// Replaces the values shown by data node `id` with `values`, rewriting each cell's own text in place.
    ///
    /// For a node whose content depends on a step the host walks through — a function's own output array, initialised
    /// to zeros until the walk reaches the step that produces it. Without this, a host could only draw such a node
    /// once, with whatever it held at that moment.
    ///
    /// `values` must be the same width of integer, and the same number of values, as the node was drawn with. The grid
    /// then keeps exactly the size, shape and position it already has, and so does every connector attached to it. The
    /// node's own selection, secondary cells and colours are untouched. Each cell keeps the width it was drawn with, so
    /// a [`crate::model::content::DataFormat::Decimal`] value with more digits than any value shown when the node was
    /// drawn will overflow its cell; [`crate::model::content::DataFormat::Hexadecimal`] and
    /// [`crate::model::content::DataFormat::Binary`] values never change width.
    ///
    /// Only for a node with two or more values drawn via [`add_data_node`](Self::add_data_node)/
    /// [`add_named_data_node`](Self::add_named_data_node) and their `_with` variants. A single-value node, and an
    /// operator node's own result, are rejected: their accessible name quotes the value itself.
    ///
    /// # Errors
    ///
    /// Returns [`Error::UnknownNode`] if `id` does not name a node in this scene.
    ///
    /// Returns [`Error::IncompatibleNodeValues`] if `id` is not a multi-value data node, or if `values` has a different
    /// integer width or a different number of values. Checked before changing anything.
    ///
    /// Also returns a wrapped [`Error::Svg`] if rewriting a cell fails partway through, which can leave some cells
    /// already rewritten.
    pub fn set_data_values(&self, id: NodeId, values: NodeValues) -> Result<(), Error> {
        let mut inner = self.inner.borrow_mut();

        let node = inner.graph.node_mut(id).ok_or(Error::UnknownNode(id))?;
        let NodeContent::Data(content) = &mut node.content else {
            return Err(Error::IncompatibleNodeValues(id));
        };
        if content.is_single_value() || !content.replace_values(values) {
            return Err(Error::IncompatibleNodeValues(id));
        }

        // Formatted into owned strings first: `content` is borrowed from the graph, and the handles that hold the
        // cells' own `<text>` elements live elsewhere in the same `SceneInner`.
        let (_, cols) = content.shape();
        let mut texts: Vec<String> = Vec::with_capacity(content.len());
        let mut scratch = String::new();
        content.for_each_cell_string(&mut scratch, |_, text| texts.push(text.to_owned()));

        let handles = inner.node_handle_mut(id).ok_or(Error::UnknownNode(id))?;
        if handles.cell_texts.len() != texts.len() {
            return Err(Error::IncompatibleNodeValues(id));
        }
        for (i, (cell, text)) in handles.cell_texts.iter().zip(&texts).enumerate() {
            cell.set_text(text);
            // Matches the accessible name `draw_content_box` gave this cell when it drew it.
            cell.set_attr("aria-label", &format!("row {}, column {}: {text}", i / cols, i % cols))?;
        }
        Ok(())
    }
}
