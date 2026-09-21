//! Node configuration and the `Scene` methods that add or reconfigure a node.

mod construction_guard;
mod edge_anchors;
mod node_options;
mod render_guard;

use super::{BinaryOperator, BoxHandles, DataNodeContent, Scene, Selection, UnaryOperator, box_centre};
use crate::{
    error::Error,
    model::{
        content::ResolvedBand,
        graph::Graph,
        node::{NodeContent, NodeId},
    },
};
use construction_guard::OperatorConstructionGuard;
pub use edge_anchors::EdgeAnchors;
pub use node_options::NodeOptions;
use render_guard::RenderGuard;
use svg_dom::{
    DominantBaseline, SvgNode, SvgRoot, TextAnchor,
    root::utils::{Point, Rect, Size},
};

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Returns [`Error::InvalidEdgeAnchors`] if `edge_anchors` is `Some(EdgeAnchors(0))`. `None` and every
/// `Some(EdgeAnchors(1..))` are valid.
fn validate_edge_anchors(edge_anchors: Option<EdgeAnchors>) -> Result<(), Error> {
    match edge_anchors {
        Some(EdgeAnchors(0)) => Err(Error::InvalidEdgeAnchors(0)),
        _ => Ok(()),
    }
}

/// `id`'s own [`DataNodeContent`], for use as an operator node's operand.
///
/// Returns [`Error::UnknownNode`] if `id` does not name a node in `graph`, or [`Error::OperandNotData`] if it names
/// a plain label node instead.
fn operand_content(graph: &Graph, id: NodeId) -> Result<&DataNodeContent, Error> {
    match &graph.node(id).ok_or(Error::UnknownNode(id))?.content {
        NodeContent::Data(content) => Ok(content),
        NodeContent::Label(_) => Err(Error::OperandNotData(id)),
    }
}

/// Returns [`Error::EmptyNodeContent`]/[`Error::InvalidGridLayout`] under the same conditions
/// [`Scene::add_data_node_with`] already rejects `content` for, plus [`Error::OperatorResultNotSingleValue`] if
/// `result` holds anything other than exactly one value — an operator always produces one value, never a grid.
fn validate_operator_result(result: &DataNodeContent) -> Result<(), Error> {
    if result.len() == 0 {
        return Err(Error::EmptyNodeContent);
    }
    if !result.layout().is_valid() {
        return Err(Error::InvalidGridLayout(result.layout()));
    }
    if result.len() != 1 {
        return Err(Error::OperatorResultNotSingleValue(result.len()));
    }
    Ok(())
}

/// The label's default font size, in user-space units, before [`shrink_label_to_fit`] ever considers scaling it down.
const LABEL_FONT_SIZE: f64 = 14.0;

/// The minimum gap, in user-space units, kept clear between a label's own rendered edges and its node's four sides.
/// [`shrink_label_to_fit`] shrinks the label's font size, proportionally, whenever it would otherwise come closer than
/// this to the box — a label short enough to fit at [`LABEL_FONT_SIZE`] is left untouched.
const LABEL_MARGIN: f64 = 5.0;

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Shrinks `label`'s own font size, proportionally, so its rendered bounding box fits within `size` once
/// [`LABEL_MARGIN`] is kept clear on every side — otherwise a label close to as wide (or tall) as its box renders with
/// its text sitting flush against, or spilling past, the box's own edges.
///
/// Reads `label`'s real, rendered bounding box (`getBBox()`, via [`SvgNode::bounding_box`]) rather than estimating
/// character widths, so this stays correct for whatever font the browser actually substitutes, with no per-glyph
/// metrics table to keep in sync. Only ever shrinks — a label that already fits at [`LABEL_FONT_SIZE`] keeps that size
/// exactly, rather than being nudged to fill the available room.
fn shrink_label_to_fit(label: &SvgNode, size: Size) -> Result<(), Error> {
    let bbox = label.bounding_box()?;
    let max_width = (size.width - 2.0 * LABEL_MARGIN).max(0.0);
    let max_height = (size.height - 2.0 * LABEL_MARGIN).max(0.0);

    let width_scale = if bbox.size.width > 0.0 { max_width / bbox.size.width } else { 1.0 };
    let height_scale = if bbox.size.height > 0.0 { max_height / bbox.size.height } else { 1.0 };
    let scale = width_scale.min(height_scale).min(1.0);

    if scale < 1.0 {
        label.set_font_size(LABEL_FONT_SIZE * scale)?;
    }
    Ok(())
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Draws a box's rectangle and its centred label, grouped under one `<g>`, and returns their handles.
///
/// Every child is drawn in local coordinates, relative to `(0, 0)`, not `rect.origin`. The group itself carries
/// `rect.origin` as a `transform="translate(...)"` instead. So moving the box later only ever updates this one
/// transform, regardless of how many children the group holds. See [`SceneInner::move_node`].
///
/// A [`RenderGuard`] covers this function's own DOM construction: any `?` failing partway through removes whatever was
/// already created, rather than leaving stray elements behind.
///
/// `scratch` is a caller-owned buffer — `SceneInner::scratch`, in every real caller — reused for this call's own
/// `transform` formatting rather than allocating a fresh `String` for it, the same reasoning every other one-shot
/// path/attribute buffer in this crate already follows.
fn draw_box(
    svg: &SvgRoot,
    scratch: &mut String,
    rect: Rect,
    label: &str,
    edge_anchors: Option<EdgeAnchors>,
) -> Result<BoxHandles, Error> {
    let group = svg.group()?;
    let mut guard = RenderGuard::new(group.clone());
    let local_rect = Rect {
        origin: Point::origin(),
        size: rect.size,
    };

    let rect_el = svg.rect(local_rect.origin, local_rect.size)?;
    guard.track(rect_el.clone());
    rect_el.set_fill("#eef4ff")?;
    rect_el.set_stroke("#2a5db0")?;
    rect_el.set_stroke_width(1.5)?;

    let label_el = svg.text(box_centre(local_rect), label)?;
    guard.track(label_el.clone());
    label_el.set_text_anchor(TextAnchor::Middle)?;
    label_el.set_dominant_baseline(DominantBaseline::Middle)?;
    label_el.set_font_size(LABEL_FONT_SIZE)?;
    label_el.set_fill("#1b1b1b")?;
    shrink_label_to_fit(&label_el, local_rect.size)?;

    group.append(&rect_el)?;
    group.append(&label_el)?;

    // Not `set_translate`. Its fixed one-decimal-place precision would quantise the rendered position away from
    // the model's own `rect.origin`, by up to 0.05 user-space units. That is harmless to the eye. But it is a
    // real mismatch for code that re-derives a position from the rendered DOM, rather than from the model.
    // Several of this crate's own browser tests do exactly that. `set_transform_fmt` writes `Display`'s full
    // precision instead, at the cost of a (typically) longer attribute string.
    group.set_transform_fmt(scratch, format_args!("translate({}, {})", rect.origin.x, rect.origin.y))?;

    guard.disarm();
    Ok(BoxHandles {
        group,
        draggable: false,
        edge_anchors,
        binary_operator_inputs: None,
        binary_operator_input_edges: None,
        cell_rects: Vec::new(),
        cell_stroke_width: "",
        selection: Selection::None,
        aria_label: String::new(),
        base_label_len: 0,
    })
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
// Data nodes — a grid of typed values instead of a plain text label. See `super::content` for the pure
// grid-shape/formatting logic; everything DOM-specific (rendering the grid, sizing the box to fit it) lives here.
// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -

/// Font size for a data node's cell text, in user-space units. Deliberately smaller than [`LABEL_FONT_SIZE`]: a
/// byte-group value (e.g. `"F0 E1 D2 C3 B4 A5 96 87"`) is far longer than a typical plain label, so a slightly smaller
/// size keeps a modest grid from demanding an oversized box by default.
const GRID_FONT_SIZE: f64 = 13.0;

/// A generic monospace font stack. Digits render at a uniform width under a monospace font — a proportional font would
/// render `"1"` narrower than `"8"`, throwing off a byte-group's own internal alignment. Several names are offered
/// since not every browser/OS ships the same monospace font; `monospace` itself is the universally supported fallback.
const GRID_FONT_FAMILY: &str = "ui-monospace, SFMono-Regular, Menlo, Consolas, monospace";

/// The height of one value's own cell, in user-space units — a fixed multiple of [`GRID_FONT_SIZE`], not measured.
///
/// Unlike a cell's width — which depends entirely on how many characters its own value holds, and so is measured via
/// [`SvgNode::bounding_box`] — a monospace font's own line height at one fixed size is predictable enough that
/// measuring it separately for every node would only add overhead, not accuracy.
const CELL_HEIGHT: f64 = GRID_FONT_SIZE * 1.4;

/// The gap kept clear, on every side, between one value's own text and that value's own cell edges.
const CELL_PADDING: f64 = 6.0;

/// The gap left between adjacent value cells in a multi-value grid, so the node's own background colour shows through
/// as a visible seam between them — this, together with each cell's own [`NodeValues::type_color`], is what lets a
/// reader tell where one value ends and the next begins, rather than reading a wall of digits with no indication of
/// which byte belongs to which value.
///
/// [`NodeValues`]: super::content::NodeValues
const CELL_GAP: f64 = 6.0;

/// The gap kept clear, on every side, between a multi-value grid's own cells and the node's outer box edges.
const OUTER_PADDING: f64 = 10.0;

/// `Scene::set_selection`'s own row/column-level highlight colour — a warm yellow, chosen to read clearly against
/// every [`NodeValues::type_color`](super::content::NodeValues::type_color) pastel and against the plain `#eef4ff`
/// outer box alike. Marks "we are now processing this row/column" in a [`Selection::Row`]/[`Selection::Column`]
/// walk.
const SELECTION_BAND_COLOR: &str = "#ffe066";

/// `Scene::set_selection`'s own cell-level highlight colour — a stronger orange-red, overriding
/// [`SELECTION_BAND_COLOR`] for the one cell a [`Selection::Cell`], or a `Row`/`Column`'s own optional cell, names.
/// Marks "and specifically this element."
const SELECTION_FOCUS_COLOR: &str = "#ff6b4a";

/// `Scene::set_selection`'s own row/column-level stroke width, thicker than every cell's own default border (see
/// [`BoxHandles::cell_stroke_width`](super::BoxHandles::cell_stroke_width)).
///
/// Colour alone is not a reliable channel: it conveys nothing to assistive technology, and can be hard to tell
/// apart for a colour-blind reader. A band is therefore also distinguishable by its own thicker border, the same
/// "not colour alone" reasoning [`NodeValues::type_color`](super::content::NodeValues::type_color)'s own
/// `<title>`/`aria-label` pairing already follows.
///
/// Already formatted — see [`BoxHandles::cell_stroke_width`](super::BoxHandles::cell_stroke_width)'s own doc
/// comment for why.
const SELECTION_BAND_STROKE_WIDTH: &str = "2";

/// `Scene::set_selection`'s own cell-level stroke width, thicker again than [`SELECTION_BAND_STROKE_WIDTH`], so the
/// focused cell stays visually distinct from a plain band even with colour perception set aside entirely.
///
/// Already formatted, for the same reason [`SELECTION_BAND_STROKE_WIDTH`] is.
const SELECTION_FOCUS_STROKE_WIDTH: &str = "3.5";

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Cell `i`'s own fill colour and stroke width under a resolved `focus`/`band`, against `base_color`/
/// `base_stroke_width` for a cell neither names.
///
/// `Scene::set_selection` calls this twice — once for the old selection, once for the new one — for each cell it
/// visits, and only writes to the DOM when the two results differ. It visits only the cells a changed old/new focus
/// or band could plausibly affect, not every cell in the grid — see its own doc comment.
fn cell_style(
    i: usize,
    focus: Option<usize>,
    band: ResolvedBand,
    base_color: &'static str,
    base_stroke_width: &'static str,
) -> (&'static str, &'static str) {
    if Some(i) == focus {
        (SELECTION_FOCUS_COLOR, SELECTION_FOCUS_STROKE_WIDTH)
    } else if band.contains(i) {
        (SELECTION_BAND_COLOR, SELECTION_BAND_STROKE_WIDTH)
    } else {
        (base_color, base_stroke_width)
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Draws a data node's rectangle and its grid of value cells, grouped under one `<g>`, and returns their handles
/// alongside the box's own final `Rect` — computed here, not supplied by the caller.
///
/// Every value gets its own `<text>` element (monospace — see [`GRID_FONT_FAMILY`]). Under a monospace font,
/// character count alone determines a cell's own rendered width. Every string [`DataNodeContent::cells`]/
/// [`DataNodeContent::for_each_cell_string`] produces is ASCII, so byte length already is character count. So only
/// the widest cell's own text is ever read back via [`SvgNode::bounding_box`] — the same "measure, don't estimate"
/// approach [`shrink_label_to_fit`] already uses for plain labels, applied once per node, not once per cell. Every
/// cell then shares that one measured width plus [`CELL_PADDING`], so the grid's rows and columns still line up
/// even when [`DataFormat::Decimal`] values differ in digit count.
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
///   `rect_el` is filled directly with [`NodeValues::type_color`], and the value's text sits centred in it.
/// - Two or more values each get their own small [`NodeValues::type_color`]-filled `<rect>`, arranged into the
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
/// A [`RenderGuard`] covers this function's own DOM construction, as in [`draw_box`]. Streaming each cell — create,
/// style, append, immediately — keeps the window this matters for down to one cell (two nodes, briefly, for a
/// multi-value grid's own rect-then-text pair) at a time, via [`RenderGuard::release`], rather than every cell
/// created so far staying tracked until the whole node finishes.
///
/// `scratch` is a caller-owned buffer — `SceneInner::scratch`, in every real caller — reused for this call's own
/// per-cell `x`/`y`/`transform` formatting, the same reasoning [`draw_box`]'s own `scratch` parameter follows. This
/// is a distinct concern from pass 1/2's own per-value formatting buffer above, which holds cell *content*, not
/// attribute values, and stays a plain local: nothing outside a single `draw_content_box` call ever needs it.
fn draw_content_box(
    svg: &SvgRoot,
    scratch: &mut String,
    top_left: Point,
    content: &DataNodeContent,
    edge_anchors: Option<EdgeAnchors>,
) -> Result<(BoxHandles, Rect), Error> {
    if content.len() == 0 {
        return Err(Error::Svg(svg_dom::Error::Dom(
            "draw_content_box: content has no values".into(),
        )));
    }

    let group = svg.group()?;
    // At most two nodes are ever loose (created but not yet appended) at once here — see `RenderGuard::release`'s
    // own doc comment — regardless of how many values `content` holds.
    let mut guard = RenderGuard::new(group.clone());
    let type_color = content.type_color();
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

    let size = if single_value {
        cell_size
    } else {
        #[allow(clippy::cast_precision_loss)]
        Size::new(
            grid_cols as f64 * cell_size.width + (grid_cols as f64 - 1.0) * CELL_GAP + 2.0 * OUTER_PADDING,
            grid_rows as f64 * cell_size.height + (grid_rows as f64 - 1.0) * CELL_GAP + 2.0 * OUTER_PADDING,
        )
    };
    let rect = Rect { origin: top_left, size };

    let rect_el = svg.rect(origin, size)?;
    guard.track(rect_el.clone());
    rect_el.set_fill(if single_value { type_color } else { "#eef4ff" })?;
    rect_el.set_stroke("#2a5db0")?;
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
            text.set_fill("#1b1b1b")?;

            if single_value {
                text.set_attr_display(scratch, "x", cell_size.width / 2.0)?;
                text.set_attr_display(scratch, "y", cell_size.height / 2.0)?;
                group.append(&text)?;
                guard.release();
            } else {
                #[allow(clippy::cast_precision_loss)]
                let (row, col) = (i / grid_cols, i % grid_cols);
                #[allow(clippy::cast_precision_loss)]
                let cell_origin = Point::new(
                    OUTER_PADDING + col as f64 * (cell_size.width + CELL_GAP),
                    OUTER_PADDING + row as f64 * (cell_size.height + CELL_GAP),
                );

                let cell_rect = svg.rect(cell_origin, cell_size)?;
                guard.track(cell_rect.clone());
                cell_rect.set_fill(type_color)?;
                cell_rect.set_stroke("#2a5db0")?;
                cell_rect.set_stroke_width(1.0)?;
                group.append(&cell_rect)?;
                guard.release();

                text.set_attr_display(scratch, "x", cell_origin.x + cell_size.width / 2.0)?;
                text.set_attr_display(scratch, "y", cell_origin.y + cell_size.height / 2.0)?;
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

    // A `<title>` is only a native tooltip/accessible name for its own direct parent, not for a sibling.
    // So it belongs on `group`, the one element every rect and every text drawn above actually shares as a
    // parent. It does not belong on any individual cell's own rect. That rect is a sibling of that cell's text,
    // not an ancestor of it. The two would never share the tooltip that way. This is exactly why an earlier
    // version of this function attached a `<title>` to each rect/text individually. That version still failed to
    // show a tooltip over the rendered digits. Putting the title on `group` instead also avoids a different
    // problem: a `<title>` as one of `text`'s own DOM children would leak its text into `text.textContent`. That
    // would mix the title text in with the actual rendered digits.
    //
    // One `<title>` for the whole node reads correctly for every cell here regardless. Every value in a
    // `DataNodeContent` shares the same type — see [`NodeValues`]'s own doc comment on that. So `type_name` is
    // the same string for every cell the mouse pointer could be hovering over.
    group.set_title(type_name)?;

    // This names the whole node for assistive technology. Assistive technology usually announces a group before
    // its children. So a reader need not visit every individual cell to learn the node's type.
    //
    // `aria-label` only names an element whose role supports naming. A bare `<g>` has no implicit role, so its
    // `aria-label` may go unexposed without an explicit `role="group"` alongside it. `group` was chosen over
    // `img` deliberately: `img` presents its descendants as one atomic image, hiding the individual cell values
    // an assistive technology user could otherwise still reach.
    let node_label = if single_value {
        format!("{type_name} value")
    } else {
        format!("{type_name} data grid, {} values", content.len())
    };
    group.set_attr("role", "group")?;
    group.set_attr("aria-label", &node_label)?;

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
        },
        rect,
    ))
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
// Operator nodes — a data node's own single value, labelled with the operation that produced it. See
// `Scene::add_unary_operator_node`/`Scene::add_binary_operator_node` for why this crate never computes that value
// itself.
// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -

/// The height of an operator node's own label row, in user-space units — the same fixed-multiple-of-font-size
/// approach [`CELL_HEIGHT`] already uses, at [`LABEL_FONT_SIZE`] rather than [`GRID_FONT_SIZE`].
const OPERATOR_LABEL_ROW_HEIGHT: f64 = LABEL_FONT_SIZE * 1.4 + 2.0 * CELL_PADDING;

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Draws an operator node: a label row naming the operation, stacked above `result`'s own single value, grouped
/// under one `<g>`, and returns their handles alongside the box's own final `Rect`.
///
/// `result` renders exactly like a single-value [`DataNodeContent`] — [`draw_content_box`]'s own single-value path
/// — with one extra row above it: `label`, in the plain style [`draw_box`] already uses for an ordinary node's own
/// text, measured the same "read the real rendered width back" way every cell in this file already is.
///
/// A [`RenderGuard`] covers this function's own DOM construction, for the same reason as [`draw_content_box`].
///
/// `scratch` is a caller-owned buffer — `SceneInner::scratch`, in every real caller — reused for this call's own
/// `x`/`y`/`transform` formatting, the same reasoning [`draw_box`]'s own `scratch` parameter follows. It also
/// holds `result`'s own formatted value text for the brief window between [`DataNodeContent::single_cell_string_into`]
/// writing it and [`SvgRoot::text`] copying it into a new `<text>` element — never a separate, one-off `String`
/// allocated just for that.
fn draw_operator_box(
    svg: &SvgRoot,
    scratch: &mut String,
    top_left: Point,
    label: &str,
    result: &DataNodeContent,
    edge_anchors: Option<EdgeAnchors>,
) -> Result<(BoxHandles, Rect), Error> {
    let group = svg.group()?;
    // Always exactly 4: the label, the value text, and the outer/value-row rects.
    let mut guard = RenderGuard::new(group.clone());
    let type_color = result.type_color();
    let type_name = result.type_name();
    let origin = Point::origin();

    let label_el = svg.text(origin, label)?;
    guard.track(label_el.clone());
    label_el.set_text_anchor(TextAnchor::Middle)?;
    label_el.set_dominant_baseline(DominantBaseline::Middle)?;
    label_el.set_font_size(LABEL_FONT_SIZE)?;
    label_el.set_fill("#1b1b1b")?;
    let label_width = label_el.bounding_box()?.size.width;

    if !result.single_cell_string_into(scratch) {
        return Err(Error::Svg(svg_dom::Error::Dom(
            "draw_operator_box: expected exactly one value".into(),
        )));
    }
    let value_el = svg.text(origin, scratch.as_str())?;
    guard.track(value_el.clone());
    value_el.set_text_anchor(TextAnchor::Middle)?;
    value_el.set_dominant_baseline(DominantBaseline::Middle)?;
    value_el.set_font_family(GRID_FONT_FAMILY)?;
    value_el.set_font_size(GRID_FONT_SIZE)?;
    value_el.set_fill("#1b1b1b")?;
    let value_width = value_el.bounding_box()?.size.width;

    let box_width = (label_width.max(value_width)) + 2.0 * CELL_PADDING;
    let value_row_height = CELL_HEIGHT + 2.0 * CELL_PADDING;
    let size = Size::new(box_width, OPERATOR_LABEL_ROW_HEIGHT + value_row_height);
    let rect = Rect { origin: top_left, size };

    let outer_el = svg.rect(origin, size)?;
    guard.track(outer_el.clone());
    outer_el.set_fill("#eef4ff")?;
    outer_el.set_stroke("#2a5db0")?;
    outer_el.set_stroke_width(1.5)?;
    group.append(&outer_el)?;

    let value_row_origin = Point::new(0.0, OPERATOR_LABEL_ROW_HEIGHT);
    let value_row_el = svg.rect(value_row_origin, Size::new(box_width, value_row_height))?;
    guard.track(value_row_el.clone());
    value_row_el.set_fill(type_color)?;
    value_row_el.set_stroke("#2a5db0")?;
    value_row_el.set_stroke_width(1.0)?;
    group.append(&value_row_el)?;

    label_el.set_attr_display(scratch, "x", box_width / 2.0)?;
    label_el.set_attr_display(scratch, "y", OPERATOR_LABEL_ROW_HEIGHT / 2.0)?;
    group.append(&label_el)?;

    value_el.set_attr_display(scratch, "x", box_width / 2.0)?;
    value_el.set_attr_display(scratch, "y", OPERATOR_LABEL_ROW_HEIGHT + value_row_height / 2.0)?;
    group.append(&value_el)?;

    // See `draw_box`'s own comment on its matching call for why `set_transform_fmt`, not `set_translate`.
    group.set_transform_fmt(scratch, format_args!("translate({}, {})", top_left.x, top_left.y))?;

    // Same reasoning as `draw_content_box`'s own `<title>`/`aria-label` pair: colour alone conveys the result's own
    // type to neither assistive technology nor a colour-blind reader.
    group.set_title(type_name)?;
    group.set_attr("role", "group")?;
    let node_label = format!("{label} result, {type_name}");
    group.set_attr("aria-label", &node_label)?;

    guard.disarm();
    let base_label_len = node_label.len();
    Ok((
        BoxHandles {
            group,
            draggable: false,
            edge_anchors,
            binary_operator_inputs: None,
            binary_operator_input_edges: None,
            cell_rects: vec![value_row_el],
            cell_stroke_width: "1",
            selection: Selection::None,
            aria_label: node_label,
            base_label_len,
        },
        rect,
    ))
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
impl Scene {
    /// Adds a node to the graph, draws its box and label, and returns its id.
    ///
    /// Equivalent to [`add_node_with`](Self::add_node_with) with [`NodeOptions::default`].
    ///
    /// # Errors
    ///
    /// Returns [`Error::InvalidNodeGeometry`] if `top_left`'s coordinates or `size`'s dimensions are not finite, or if
    /// `size`'s width or height is not strictly positive — see that variant's own doc comment for why.
    pub fn add_node(&self, top_left: Point, size: Size, label: impl Into<String>) -> Result<NodeId, Error> {
        self.add_node_with(top_left, size, label, NodeOptions::default())
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    /// Adds a node to the graph, draws its box and label, and returns its id. `options` controls how many connector
    /// fixing points this node's sides offer — see [`EdgeAnchors`].
    ///
    /// # Errors
    ///
    /// Returns [`Error::InvalidEdgeAnchors`] if `options.edge_anchors` is `Some(EdgeAnchors(0))`. Checked before
    /// drawing anything or touching the graph's model, so a rejected call leaves the scene exactly as it was.
    ///
    /// Returns [`Error::InvalidNodeGeometry`] if `top_left`'s coordinates or `size`'s dimensions are not finite, or if
    /// `size`'s width or height is not strictly positive — see that variant's own doc comment for why. Checked before
    /// drawing anything or touching the graph's model, so a rejected call leaves the scene exactly as it was.
    pub fn add_node_with(
        &self,
        top_left: Point,
        size: Size,
        label: impl Into<String>,
        options: NodeOptions,
    ) -> Result<NodeId, Error> {
        validate_edge_anchors(options.edge_anchors)?;

        let rect = Rect { origin: top_left, size };
        if !top_left.x.is_finite()
            || !top_left.y.is_finite()
            || !size.width.is_finite()
            || !size.height.is_finite()
            || size.width <= 0.0
            || size.height <= 0.0
        {
            return Err(Error::InvalidNodeGeometry(rect));
        }

        let label = label.into();
        let mut inner = self.inner.borrow_mut();
        // Taken out for the call so `draw_box` can format into it without also needing `&inner.svg` to borrow
        // `inner` in two conflicting ways at once — see `SceneInner::scratch`'s own doc comment for why this,
        // rather than a fresh `String` per call.
        let mut scratch = std::mem::take(&mut inner.scratch);
        let result = draw_box(&inner.svg, &mut scratch, rect, &label, options.edge_anchors);
        inner.scratch = scratch;
        let handles = result?;
        let id = inner.graph.add_node(rect, label);
        inner.insert_node_handle(id, handles);
        Ok(id)
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
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
        let result = draw_content_box(&inner.svg, &mut scratch, top_left, &content, options.edge_anchors);
        inner.scratch = scratch;
        let (handles, rect) = result?;
        let id = inner.graph.add_node(rect, content);
        inner.insert_node_handle(id, handles);
        Ok(id)
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    /// Updates node `id`'s [`EdgeAnchors`] configuration, and redraws every incident connector immediately with the
    /// new value.
    ///
    /// This is the only way to change a node's anchor configuration after [`Scene::add_node`] or
    /// [`Scene::add_node_with`] first draws it — for example, from a live slider control.
    ///
    /// An `edge_anchors` identical to `id`'s own current configuration is an immediate no-op: no incident edge is
    /// redrawn. [`EdgeAnchors`] is `Copy` and `Eq`, so this comparison is free next to the DOM writes it can skip.
    ///
    /// # Errors
    ///
    /// Returns [`Error::InvalidEdgeAnchors`] if `edge_anchors` is `Some(EdgeAnchors(0))`. Checked before touching the
    /// scene, so a rejected call leaves the node exactly as it was.
    ///
    /// Returns [`Error::UnknownNode`] if `id` does not name a node in this scene.
    ///
    /// Also returns an error — [`Error::UnknownEdge`] or a wrapped [`Error::Svg`] — if redrawing an incident connector
    /// fails partway through. A failure here can leave some incident connectors already redrawn and others not.
    /// Dragging a node already carries this same property for its own incident redraws, so this is not a new,
    /// weaker guarantee.
    pub fn set_edge_anchors(&self, id: NodeId, edge_anchors: Option<EdgeAnchors>) -> Result<(), Error> {
        validate_edge_anchors(edge_anchors)?;

        let mut inner = self.inner.borrow_mut();
        let handles = inner.node_handle_mut(id).ok_or(Error::UnknownNode(id))?;
        if handles.edge_anchors == edge_anchors {
            return Ok(());
        }
        handles.edge_anchors = edge_anchors;
        let own_input_edges = handles.binary_operator_input_edges;

        // Taken out for the call so `redraw_edge`/`redraw_binary_operator_inputs` can freely borrow the rest of
        // `inner` on every iteration, then put back — see `SceneInner::scratch`'s own doc comment for why this,
        // rather than a fresh `String` per call.
        let mut scratch = std::mem::take(&mut inner.scratch);

        // `id` is itself a binary operator node, so `edge_anchors` is *its own* fixing-point configuration: it
        // feeds `binary_operator_anchors` for both input edges at once, and both are redrawn together, once, here
        // — the same reasoning `SceneInner::move_node` follows for `redraw_binary_operator_inputs`. Changing an
        // ordinary node's, or an operand's own, `edge_anchors` never needs this: it only ever affects that one
        // node's own from-side anchor, never the operator-side split.
        if own_input_edges.is_some() {
            if let Err(e) = inner.redraw_binary_operator_inputs(id, &mut scratch) {
                inner.scratch = scratch;
                return Err(e);
            }
        }

        for edge_id in inner.graph.incident_edges(id) {
            if own_input_edges.is_some_and(|(a, b)| *edge_id == a || *edge_id == b) {
                continue; // already redrawn together, above
            }
            if let Err(e) = inner.redraw_edge(*edge_id, &mut scratch) {
                inner.scratch = scratch;
                return Err(e);
            }
        }
        inner.scratch = scratch;
        Ok(())
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
    /// [`Selection::None`] clears back to every cell's own default `NodeValues::type_color`. So there is no need to
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
    /// depends on colour alone — the same reasoning `NodeValues::type_color`'s own `<title>`/`aria-label` pairing
    /// already follows.
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
        let base_color = content.type_color();

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
            let old_style = cell_style(i, old_focus, old_band, base_color, cell_stroke_width);
            let new_style = cell_style(i, new_focus, new_band, base_color, cell_stroke_width);
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

        Ok(())
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    /// Adds a unary operator node to the graph — labelled with `operator`, showing `result`'s own single value, and
    /// wired with an incoming edge from `input` — and returns its id.
    ///
    /// `svg-dom-graph` never evaluates `operator` itself. `result` must already be `input`'s own value transformed
    /// the way `operator` names — the caller computes it, exactly as it already computes every other data node's
    /// own values. See this module's own doc comment ("Operator nodes").
    ///
    /// Equivalent to [`add_unary_operator_node_with`](Self::add_unary_operator_node_with) with
    /// [`NodeOptions::default`].
    ///
    /// # Errors
    ///
    /// See [`add_unary_operator_node_with`](Self::add_unary_operator_node_with)'s own `# Errors` section.
    pub fn add_unary_operator_node(
        &self,
        top_left: Point,
        operator: UnaryOperator,
        input: NodeId,
        result: DataNodeContent,
    ) -> Result<NodeId, Error> {
        self.add_unary_operator_node_with(top_left, operator, input, result, NodeOptions::default())
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    /// Adds a unary operator node, as [`add_unary_operator_node`](Self::add_unary_operator_node), but with `options`
    /// controlling how many connector fixing points this node's sides offer — see [`EdgeAnchors`].
    ///
    /// # Errors
    ///
    /// Returns [`Error::InvalidEdgeAnchors`] if `options.edge_anchors` is `Some(EdgeAnchors(0))`.
    ///
    /// Returns [`Error::InvalidNodeGeometry`] if `top_left`'s coordinates are not finite.
    ///
    /// Returns [`Error::EmptyNodeContent`] if `result` holds no values, or [`Error::InvalidGridLayout`] if its own
    /// [`GridLayout`](crate::scene::GridLayout) wraps `0`. Returns [`Error::OperatorResultNotSingleValue`] if
    /// `result` holds more than one value — an operator always produces one value, never a grid.
    ///
    /// Returns [`Error::UnknownNode`] if `input` does not name a node in this scene, or [`Error::OperandNotData`] if
    /// it names a plain label node rather than a [`DataNodeContent`] one.
    ///
    /// Returns [`Error::OperatorTypeMismatch`] if `result`'s own value width does not match `input`'s.
    ///
    /// Every check above runs before drawing anything or touching the graph's model, so a rejected call leaves the
    /// scene exactly as it was.
    ///
    /// A failure drawing the auto-wired input edge, after the node itself was already created, is rolled back too.
    /// The node is removed again, so a failed call never leaves a partial operator behind.
    pub fn add_unary_operator_node_with(
        &self,
        top_left: Point,
        operator: UnaryOperator,
        input: NodeId,
        result: DataNodeContent,
        options: NodeOptions,
    ) -> Result<NodeId, Error> {
        validate_edge_anchors(options.edge_anchors)?;
        validate_operator_result(&result)?;
        if !top_left.x.is_finite() || !top_left.y.is_finite() {
            return Err(Error::InvalidNodeGeometry(Rect {
                origin: top_left,
                size: Size::new(0.0, 0.0),
            }));
        }

        let id = {
            let mut inner = self.inner.borrow_mut();
            let operand_type = operand_content(&inner.graph, input)?.type_name();
            if operand_type != result.type_name() {
                return Err(Error::OperatorTypeMismatch {
                    expected: operand_type,
                    found: result.type_name(),
                });
            }

            let label = operator.label();
            // See `add_node_with`'s own matching comment for why `scratch` is taken out for the call.
            let mut scratch = std::mem::take(&mut inner.scratch);
            let draw_result =
                draw_operator_box(&inner.svg, &mut scratch, top_left, &label, &result, options.edge_anchors);
            inner.scratch = scratch;
            let (handles, rect) = draw_result?;
            let id = inner.graph.add_node(rect, result);
            inner.insert_node_handle(id, handles);
            id
        };

        let mut guard = OperatorConstructionGuard::new(self.clone(), id);
        guard.track_edge(self.add_edge(input, id)?);
        guard.disarm();
        Ok(id)
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    /// Adds a binary operator node to the graph — labelled with `operator`, showing `result`'s own single value, and
    /// wired with incoming edges from `inputs.0` and `inputs.1` — and returns its id.
    ///
    /// `svg-dom-graph` never evaluates `operator` itself. `result` must already be `operator` applied to `inputs`'
    /// own two values, computed by the caller. See this module's own doc comment ("Operator nodes").
    ///
    /// Equivalent to [`add_binary_operator_node_with`](Self::add_binary_operator_node_with) with
    /// [`NodeOptions::default`].
    ///
    /// # Errors
    ///
    /// See [`add_binary_operator_node_with`](Self::add_binary_operator_node_with)'s own `# Errors` section.
    pub fn add_binary_operator_node(
        &self,
        top_left: Point,
        operator: BinaryOperator,
        inputs: (NodeId, NodeId),
        result: DataNodeContent,
    ) -> Result<NodeId, Error> {
        self.add_binary_operator_node_with(top_left, operator, inputs, result, NodeOptions::default())
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    /// Adds a binary operator node, as [`add_binary_operator_node`](Self::add_binary_operator_node), but with `options`
    /// controlling how many connector fixing points this node's sides offer — see [`EdgeAnchors`].
    ///
    /// # Errors
    ///
    /// Returns [`Error::InvalidEdgeAnchors`] if `options.edge_anchors` is `Some(EdgeAnchors(0))`.
    ///
    /// Returns [`Error::InvalidNodeGeometry`] if `top_left`'s coordinates are not finite.
    ///
    /// Returns [`Error::EmptyNodeContent`] if `result` holds no values, or [`Error::InvalidGridLayout`] if its own
    /// [`GridLayout`](crate::scene::GridLayout) wraps `0`. Returns [`Error::OperatorResultNotSingleValue`] if `result`
    /// holds more than one value — an operator always produces one value, never a grid.
    ///
    /// Returns [`Error::DuplicateOperands`] if `inputs.0` and `inputs.1` name the same node — a binary operator's two
    /// operands must be distinct.
    ///
    /// Returns [`Error::UnknownNode`] if either of `inputs` does not name a node in this scene, or
    /// [`Error::OperandNotData`] if it names a plain label node rather than a [`DataNodeContent`] one. `inputs.0` is
    /// checked first.
    ///
    /// Returns [`Error::OperatorTypeMismatch`] if `inputs.0` and `inputs.1` do not share one value width, or if
    /// `result`'s own value width does not match theirs — both operands, and the result, must share one
    /// [`NodeValues`](crate::scene::NodeValues) width.
    ///
    /// Every check above runs before drawing anything or touching the graph's model, so a rejected call leaves the
    /// scene exactly as it was.
    ///
    /// A failure drawing either auto-wired input edge, after the node itself was already created, is rolled back too.
    /// The node, and whichever of its two input edges had already been wired, are removed again. So a failed call never
    /// leaves a partial operator behind.
    pub fn add_binary_operator_node_with(
        &self,
        top_left: Point,
        operator: BinaryOperator,
        inputs: (NodeId, NodeId),
        result: DataNodeContent,
        options: NodeOptions,
    ) -> Result<NodeId, Error> {
        validate_edge_anchors(options.edge_anchors)?;
        validate_operator_result(&result)?;
        if inputs.0 == inputs.1 {
            return Err(Error::DuplicateOperands(inputs.0));
        }
        if !top_left.x.is_finite() || !top_left.y.is_finite() {
            return Err(Error::InvalidNodeGeometry(Rect {
                origin: top_left,
                size: Size::new(0.0, 0.0),
            }));
        }

        let id = {
            let mut inner = self.inner.borrow_mut();
            let left_type = operand_content(&inner.graph, inputs.0)?.type_name();
            let right_type = operand_content(&inner.graph, inputs.1)?.type_name();
            if left_type != right_type {
                return Err(Error::OperatorTypeMismatch {
                    expected: left_type,
                    found: right_type,
                });
            }
            if left_type != result.type_name() {
                return Err(Error::OperatorTypeMismatch {
                    expected: left_type,
                    found: result.type_name(),
                });
            }

            let label = operator.label();
            // See `add_node_with`'s own matching comment for why `scratch` is taken out for the call.
            let mut scratch = std::mem::take(&mut inner.scratch);
            let draw_result =
                draw_operator_box(&inner.svg, &mut scratch, top_left, label, &result, options.edge_anchors);
            inner.scratch = scratch;
            let (mut handles, rect) = draw_result?;
            handles.binary_operator_inputs = Some(inputs);
            let id = inner.graph.add_node(rect, result);
            inner.insert_node_handle(id, handles);
            id
        };

        let mut guard = OperatorConstructionGuard::new(self.clone(), id);
        let edge_a = self.add_edge(inputs.0, id)?;
        guard.track_edge(edge_a);
        let edge_b = self.add_edge(inputs.1, id)?;
        guard.track_edge(edge_b);

        // Both auto-wired edges exist now, so this operator's own input-edge pair — otherwise unknowable until this
        // point — can be cached once here. `SceneInner::redraw_binary_operator_inputs` reads it on every later
        // drag, instead of searching either operand's own incident edges for it.
        self.inner
            .borrow_mut()
            .node_handle_mut(id)
            .ok_or(Error::UnknownNode(id))?
            .binary_operator_input_edges = Some((edge_a, edge_b));

        guard.disarm();
        Ok(id)
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[cfg(test)]
mod unit_tests;
