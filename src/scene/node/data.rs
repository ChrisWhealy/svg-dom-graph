//! Data nodes — a grid of typed values instead of a plain text label, and their own cell/row/column
//! [`Selection`](crate::scene::Selection) highlighting. See `super::content` for the pure grid-shape/formatting
//! logic; everything DOM-specific (rendering the grid, sizing the box to fit it, recolouring cells) lives here. See
//! [`super::plain`] for a node with a plain text label instead, and [`super::operator`] for one labelled with the
//! operation that produced its own value.

use super::{
    CELL_HEIGHT, CELL_PADDING, EdgeAnchors, GRID_FONT_FAMILY, GRID_FONT_SIZE, LABEL_FONT_SIZE, LABEL_ROW_HEIGHT,
    NodeOptions, OUTER_PADDING, render_guard::RenderGuard, validate_edge_anchors,
};
use crate::{
    colours::{BOX_STROKE, NAMED_BOX_FILL, PLAIN_BOX_FILL, SELECTION_BAND, SELECTION_FOCUS, TEXT_FILL},
    error::Error,
    model::{
        content::ResolvedBand,
        node::{NodeContent, NodeId},
    },
    scene::{BoxHandles, DataNodeContent, Scene, Selection},
};
use svg_dom::{
    DominantBaseline, SvgRoot, TextAnchor,
    root::utils::{Point, Rect, Size},
};

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The gap left between adjacent value cells in a multi-value grid, so the node's own background colour shows through
/// as a visible seam between them — this, together with each cell's own [`NodeValues::type_colour`], is what lets a
/// reader tell where one value ends and the next begins, rather than reading a wall of digits with no indication of
/// which byte belongs to which value.
///
/// [`NodeValues`]: crate::model::content::NodeValues
const CELL_GAP: f64 = 6.0;

/// `Scene::set_selection`'s own row/column-level stroke width, thicker than every cell's own default border (see
/// [`BoxHandles::cell_stroke_width`]).
///
/// Colour alone is not a reliable channel: it conveys nothing to assistive technology, and can be hard to tell
/// apart for a colour-blind reader. A band is therefore also distinguishable by its own thicker border, the same
/// "not colour alone" reasoning [`NodeValues::type_colour`](crate::model::content::NodeValues::type_colour)'s own
/// `<title>`/`aria-label` pairing already follows.
///
/// Already formatted — see [`BoxHandles::cell_stroke_width`]'s own doc comment for why.
const SELECTION_BAND_STROKE_WIDTH: &str = "2";

/// `Scene::set_selection`'s own cell-level stroke width, thicker again than [`SELECTION_BAND_STROKE_WIDTH`], so the
/// focused cell stays visually distinct from a plain band even with colour perception set aside entirely.
///
/// Already formatted, for the same reason [`SELECTION_BAND_STROKE_WIDTH`] is.
const SELECTION_FOCUS_STROKE_WIDTH: &str = "3.5";

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Cell `i`'s own fill colour and stroke width under a resolved `focus`/`band`, against `base_colour`/
/// `base_stroke_width` for a cell neither names.
///
/// `Scene::set_selection` calls this twice — once for the old selection, once for the new one — for each cell it
/// visits, and only writes to the DOM when the two results differ. It visits only the cells a changed old/new focus
/// or band could plausibly affect, not every cell in the grid — see its own doc comment.
fn cell_style(
    i: usize,
    focus: Option<usize>,
    band: ResolvedBand,
    base_colour: &'static str,
    base_stroke_width: &'static str,
) -> (&'static str, &'static str) {
    if Some(i) == focus {
        (SELECTION_FOCUS, SELECTION_FOCUS_STROKE_WIDTH)
    } else if band.contains(i) {
        (SELECTION_BAND, SELECTION_BAND_STROKE_WIDTH)
    } else {
        (base_colour, base_stroke_width)
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Draws a data node's rectangle and its grid of value cells, grouped under one `<g>`, and returns their handles
/// alongside the box's own final `Rect` — computed here, not supplied by the caller.
///
/// Every value gets its own `<text>` element (monospace — see [`GRID_FONT_FAMILY`]). Under a monospace font,
/// character count alone determines a cell's own rendered width. Every string [`DataNodeContent::cells`]/
/// [`DataNodeContent::for_each_cell_string`] produces is ASCII, so byte length already is character count. So only
/// the widest cell's own text is ever read back via [`SvgNode::bounding_box`](svg_dom::SvgNode::bounding_box) — the
/// same "measure, don't estimate" approach [`shrink_label_to_fit`](super::plain::shrink_label_to_fit) already uses
/// for plain labels, applied once per node, not once per cell. Every cell then shares that one measured width plus
/// [`CELL_PADDING`], so the grid's rows and columns still line up even when [`DataFormat::Decimal`] values differ
/// in digit count.
///
/// Formats and places each cell streaming — never collecting a `Vec<String>` of every cell's own text, or a
/// `Vec<SvgNode>` of every `<text>` element, regardless of how many values `content` holds, and never formatting
/// any value twice:
///
/// 1. [`DataNodeContent::widest_cell_string`] identifies and formats the one value guaranteed to need the widest
///    cell, without formatting every value first — see that method's own doc comment for how. A throwaway element
///    built from it, once `cell_size` is known, is all `bounding_box()` ever needs; which specific value that was
///    is otherwise irrelevant, since any string of the same length would measure identically under a monospace
///    font.
/// 2. [`DataNodeContent::for_each_cell_string`] then formats every value once, reusing one buffer, and this time
///    creates each cell's own `<text>` (and, for a multi-value grid, its own `<rect>`) directly at its final
///    position, appending each immediately rather than deferring every cell's own placement to a later pass. The
///    one value pass 1 already formatted is formatted again here, along with every other — a single value's worth
///    of repeated work, not repeated for the whole node.
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
/// A [`RenderGuard`] covers this function's own DOM construction, as in
/// [`draw_box`](super::plain::draw_box). Streaming each cell — create, style, append, immediately — keeps the
/// window this matters for down to one cell (two nodes, briefly, for a multi-value grid's own rect-then-text pair)
/// at a time, via [`RenderGuard::release`], rather than every cell created so far staying tracked until the whole
/// node finishes.
///
/// `scratch` is a caller-owned buffer — `SceneInner::scratch`, in every real caller — reused for this call's own
/// per-cell `x`/`y`/`transform` formatting, the same reasoning [`draw_box`](super::plain::draw_box)'s own `scratch`
/// parameter follows. This is a distinct concern from pass 1/2's own per-value formatting buffer above, which holds
/// cell *content*, not attribute values, and stays a plain local: nothing outside a single `draw_content_box` call
/// ever needs it.
///
/// `name`, when `Some`, wraps the box described above in a further outer box of its own, with `name` in a label
/// row above it — the same "outer box labelled with a name, inset value box beneath it" shape
/// [`draw_operator_box`](super::operator::draw_operator_box) already draws for an operator's own result. The
/// returned `Rect` is then that outer box's own, so an incoming connector anchors to it, never to the inner
/// (unlabelled, exactly as drawn below) content box directly — see [`draw_operator_box`]'s own module doc comment
/// for why a connector must never land on an inset inner box. `name` is `None` for every plain (unnamed) data
/// node, which renders exactly as before this parameter existed: no label row, no further outer box, the content
/// box itself starting flush at local `(0, 0)`.
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
    // At most two nodes are ever loose (created but not yet appended) at once here — see `RenderGuard::release`'s
    // own doc comment — regardless of how many values `content` holds, or whether `name` is given: each of the
    // name label's own text/outer-box elements below is released again, in turn, before the next is created.
    let mut guard = RenderGuard::new(group.clone());
    let type_colour = content.type_colour();
    let type_name = content.type_name();
    let origin = Point::origin();
    let len = content.len();
    let (grid_rows, grid_cols) = content.shape();
    let single_value = content.is_single_value();

    // Finds and formats the one value guaranteed to need the widest rendered cell — see
    // `DataNodeContent::widest_cell_string`'s own doc comment for how, without formatting every value just to
    // compare the resulting text lengths.
    let mut widest = String::new();
    content.widest_cell_string(&mut widest);

    // `widest`'s own real content is measured once, via a throwaway element that never becomes one of `group`'s
    // own children: tracked for rollback like any other fallible-construction element, then removed the moment it
    // has served its purpose, rather than kept around as one of the real cells.
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
            grid_cols as f64 * cell_size.width + (grid_cols as f64 - 1.0) * CELL_GAP + 2.0 * OUTER_PADDING,
            grid_rows as f64 * cell_size.height + (grid_rows as f64 - 1.0) * CELL_GAP + 2.0 * OUTER_PADDING,
        )
    };

    // `content_origin` is where the content box drawn below sits, local to `group` — `(0, 0)` exactly as before
    // this parameter existed, unless `name` wraps it in a further named outer box, in which case it is inset and
    // centred under that outer box's own label row instead.
    let (content_origin, size) = if let Some(name) = name {
        let label_el = svg.text(origin, name)?;
        guard.track(label_el.clone());
        label_el.set_text_anchor(TextAnchor::Middle)?;
        label_el.set_dominant_baseline(DominantBaseline::Middle)?;
        label_el.set_font_size(LABEL_FONT_SIZE)?;
        label_el.set_fill(TEXT_FILL)?;
        let label_width = label_el.bounding_box()?.size.width;

        // Same competition `draw_operator_box` resolves for its own label vs. value cell: the content box's own
        // width, plus `OUTER_PADDING` clear on either side, against the label's own width, plus `CELL_PADDING` —
        // whichever needs more room sets the outer box's own width. Either way the content box itself never
        // reaches the outer box's own left/right edges.
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

        (Point::new((box_width - content_size.width) / 2.0, LABEL_ROW_HEIGHT), box_size)
    } else {
        (origin, content_size)
    };
    let rect = Rect { origin: top_left, size };

    let rect_el = svg.rect(content_origin, content_size)?;
    guard.track(rect_el.clone());
    rect_el.set_fill(if single_value { type_colour } else { PLAIN_BOX_FILL })?;
    rect_el.set_stroke(BOX_STROKE)?;
    rect_el.set_stroke_width(1.5)?;
    group.append(&rect_el)?;
    guard.release();

    // Streamed rendering pass — see this function's own doc comment. `text_scratch` is reused for every cell's own
    // formatted text.
    let mut text_scratch = String::new();
    let mut cell_rects = Vec::with_capacity(if single_value { 1 } else { len });
    if single_value {
        cell_rects.push(rect_el.clone());
    }

    // Captured only in the `single_value` case — the same formatted text that one cell's own `<text>` renders,
    // reused again below for the node's own `aria-label`, so the value reads as text without visiting the cell
    // directly. Left empty for a multi-value grid, whose own `aria-label` names a value count instead — see that
    // `node_label` match below.
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
                    content_origin.x + OUTER_PADDING + col as f64 * (cell_size.width + CELL_GAP),
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

    // This names the whole node for assistive technology, and — via `<title>` below — for the browser's own
    // mouse-hover tooltip too. Assistive technology usually announces a group before its children, so a reader
    // need not visit every individual cell to learn the node's own name, type, or (for a single value) its real
    // value.
    //
    // `aria-label` only names an element whose role supports naming. A bare `<g>` has no implicit role, so its
    // `aria-label` may go unexposed without an explicit `role="group"` alongside it. `group` was chosen over
    // `img` deliberately: `img` presents its descendants as one atomic image, hiding the individual cell values
    // an assistive technology user could otherwise still reach.
    //
    // A multi-value grid's own row/column shape is spelled out here too — "2 rows by 3 columns". Not just left
    // implicit in each cell's own `x`/`y` position, which conveys nothing to assistive technology. Each cell's own
    // `aria-label`, set below, names its row and column directly, the same "not visual position alone" reasoning.
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

    // A `<title>` is only a native tooltip/accessible name for its own direct parent, not for a sibling.
    // So it belongs on `group`, the one element every rect and every text drawn above actually shares as a
    // parent. It does not belong on any individual cell's own rect. That rect is a sibling of that cell's text,
    // not an ancestor of it. The two would never share the tooltip that way. This is exactly why an earlier
    // version of this function attached a `<title>` to each rect/text individually. That version still failed to
    // show a tooltip over the rendered digits. Putting the title on `group` instead also avoids a different
    // problem: a `<title>` as one of `text`'s own DOM children would leak its text into `text.textContent`. That
    // would mix the title text in with the actual rendered digits.
    //
    // Set to `node_label` — the same text `aria-label` carries — so the browser's own mouse-hover tooltip reads
    // exactly what a screen reader announces, not just the node's own type. `Scene::set_selection` keeps the two
    // in sync afterward too, rewriting this alongside `aria-label` on every selection change.
    group.set_title(&node_label)?;

    guard.disarm();
    let base_label_len = node_label.len();
    Ok((
        BoxHandles {
            group,
            draggable: false,
            edge_anchors,
            binary_operator_inputs: None,
            binary_operator_input_edges: None,
            cell_rects,
            cell_stroke_width: if single_value { "1.5" } else { "1" },
            selection: Selection::None,
            aria_label: node_label,
            base_label_len,
            ref_name,
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
    /// Gives a raw value a variable name.
    /// Adds a data node to the graph, exactly as [`add_data_node`](Self::add_data_node), but wrapped in a further
    /// outer box of its own labelled `name`. The same shape an "outer box labelled with a name with an inset value box
    /// beneath it" already drawn for an operator node's own result.
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
    /// Adds a named data node, as [`add_named_data_node`](Self::add_named_data_node), but with `options`
    /// controlling how many connector fixing points this node's sides offer — see [`EdgeAnchors`].
    ///
    /// An incoming connector still anchors to this node's own *outer* (named) box, never to the inner content box
    /// `name` wraps — the same reasoning [`add_binary_operator_node`](Self::add_binary_operator_node)'s own module
    /// doc comment gives for why a connector must never land on an inset inner box.
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
    /// Shared implementation behind [`add_data_node_with`](Self::add_data_node_with) and
    /// [`add_named_data_node_with`](Self::add_named_data_node_with) — every check, and the rendered box itself
    /// (modulo `name`'s own outer wrapper), is identical between a plain and a named data node.
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

        if content.len() == 0 {
            return Err(Error::EmptyNodeContent);
        }
        if !content.layout().is_valid() {
            return Err(Error::InvalidGridLayout(content.layout()));
        }
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
    /// This is the only way to change a node's own selection after it is first drawn. A live "previous"/"next"
    /// control stepping through an array as it is processed is one example.
    ///
    /// [`Selection::None`] clears back to every cell's own default `NodeValues::type_colour`. So there is no need to
    /// clear before setting a new selection.
    ///
    /// An identical `selection` to `id`'s own current one is an immediate no-op — no cell is touched, and no
    /// `aria-label` write happens. Otherwise, only the cells whose own colour/stroke category (focused, banded, or
    /// default) actually changes between the old selection and the new one are even examined, let alone written to
    /// — the old/new focus cells, plus each band's own members, via `ResolvedBand::for_each_index`. A cell in
    /// neither band, and not a focus either way, is never visited: its category cannot have changed. A live
    /// "previous"/"next" control stepping through an array as it is processed only ever touches a handful of cells
    /// per step, however large the array — this is the hot path that shape is optimised for, in both the DOM writes
    /// it performs and the Rust computation that decides them.
    ///
    /// Also gives the focused cell, and, less strongly, a banded row/column, a thicker stroke than its own default
    /// border. It also rebuilds the node's own `aria-label` to describe the current selection as text. Neither
    /// depends on colour alone — the same reasoning `NodeValues::type_colour`'s own `<title>`/`aria-label` pairing
    /// already follows.
    ///
    /// This updates the node's own accessible name, making the current selection available to assistive technology.
    /// It does not create a live-region announcement, so a screen reader whose virtual cursor sits elsewhere may not
    /// notice the change until the user navigates back to this node. An application needing an immediate announcement
    /// should provide its own status/live region; this method does not.
    ///
    /// # Errors
    ///
    /// Returns [`Error::UnknownNode`] if `id` does not name a node in this scene.
    ///
    /// Returns [`Error::InvalidSelection`] if `id` names a plain label node. Also returns it if `selection` names
    /// a cell/row/column index out of range for `id`'s own actual value count or grid shape. Checked before
    /// recolouring any cell, so a rejected call leaves every cell's own colour exactly as it was.
    ///
    /// Also returns a wrapped [`Error::Svg`] if recolouring a cell fails partway through. A failure here can leave
    /// some cells already recoloured and others not — the same documented property
    /// [`set_edge_anchors`](Self::set_edge_anchors) already carries for its own incident redraws.
    pub fn set_selection(&self, id: NodeId, selection: Selection) -> Result<(), Error> {
        let mut inner = self.inner.borrow_mut();

        let content = match &inner.graph.node(id).ok_or(Error::UnknownNode(id))?.content {
            NodeContent::Data(content) => content,
            NodeContent::Label(_) => return Err(Error::InvalidSelection(id, selection)),
        };
        let (new_band, new_focus) = content
            .resolve_selection(selection)
            .ok_or(Error::InvalidSelection(id, selection))?;
        let base_colour = content.type_colour();

        let old_selection = inner.node_handle(id).ok_or(Error::UnknownNode(id))?.selection;
        if old_selection == selection {
            return Ok(());
        }
        // `old_selection` was itself accepted by an earlier, successful `set_selection` call against this same,
        // unchanged content (or is the default `Selection::None`, always valid), so it always resolves here too.
        let (old_band, old_focus) = content.resolve_selection(old_selection).unwrap_or((ResolvedBand::None, None));

        let handles = inner.node_handle_mut(id).ok_or(Error::UnknownNode(id))?;
        let cell_stroke_width = handles.cell_stroke_width;
        let cell_rects = &handles.cell_rects;
        let len = cell_rects.len();

        // Every index whose own category (focused, banded, or default) could possibly differ between the old
        // selection and the new one — never the whole grid, and each visited at most once. A cell outside this set
        // is provably unchanged: it is neither an old/new focus, nor in the symmetric difference of the two bands,
        // so `cell_style` resolves it to the same category either way. See `ResolvedBand::for_each_index`'s own
        // doc comment.
        let mut result = Ok(());
        let mut restyle = |i: usize| {
            if result.is_err() {
                return;
            }
            let Some(cell) = cell_rects.get(i) else { return };
            let old_style = cell_style(i, old_focus, old_band, base_colour, cell_stroke_width);
            let new_style = cell_style(i, new_focus, new_band, base_colour, cell_stroke_width);
            if new_style == old_style {
                return;
            }
            result = cell
                .set_fill(new_style.0)
                .and_then(|()| cell.set_attr("stroke-width", new_style.1));
        };
        if let Some(i) = old_focus {
            restyle(i);
        }
        // Only if it differs from `old_focus` — otherwise this index was already visited above, and a second visit
        // here would just repeat the same comparison.
        if new_focus != old_focus {
            if let Some(i) = new_focus {
                restyle(i);
            }
        }
        if old_band != new_band {
            // True symmetric difference, not each band walked in full: a member of both bands (their intersection)
            // is skipped in both traversals below, since its own category cannot have changed between them either
            // — and a focus index is skipped here too, since it was already visited, explicitly, above.
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
        handles.aria_label.truncate(handles.base_label_len);
        selection.describe_into(&mut handles.aria_label);
        handles.group.set_attr("aria-label", &handles.aria_label)?;
        // Keeps the browser's own mouse-hover tooltip reading exactly the same text as `aria-label` — see
        // `draw_content_box`'s own doc comment on why `<title>` is set to that same text at construction.
        handles.group.set_title(&handles.aria_label)?;

        Ok(())
    }
}
