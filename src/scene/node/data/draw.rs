//! Drawing a data node: measuring its cells, sizing the box to fit them, and rendering the grid.

use super::super::{
    CELL_HEIGHT, CELL_PADDING, EdgeAnchors, GRID_FONT_FAMILY, GRID_FONT_SIZE, LABEL_FONT_SIZE, LABEL_ROW_HEIGHT,
    OUTER_PADDING, render_guard::RenderGuard,
};
use crate::{
    colours::{BOX_STROKE, NAMED_BOX_FILL, PLAIN_BOX_FILL, TEXT_FILL},
    error::Error,
    scene::{BoxHandles, DataNodeContent, Selection},
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
    // `content` holds, or whether `name` is given. See `RenderGuard::release`'s own doc comment. Each of the name
    // label's own text/outer-box elements below is released again, in turn, before the next is created.
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

    // Finds and formats the one value guaranteed to need the widest rendered cell, without formatting every value just
    // to compare the resulting text lengths. See `DataNodeContent::widest_cell_string`'s own doc comment for how.
    let mut widest = String::new();
    content.widest_cell_string(&mut widest);

    // `widest`'s own real content is measured once, via a throwaway element that never becomes one of `group`'s own
    // children. It is tracked for rollback like any other fallible-construction element. It is then removed the moment
    // it has served its purpose, rather than kept around as one of the real cells.
    let measure_el = svg.text(origin, &widest)?;
    guard.track(measure_el.clone());
    measure_el.set_font_family(GRID_FONT_FAMILY)?;
    measure_el.set_font_size(GRID_FONT_SIZE)?;
    if content.is_plain_text() {
        measure_el.set_attr("style", "white-space: pre")?;
    }
    let mut max_width = measure_el.bounding_box()?.size.width;
    // Edge case: Every `Ascii` cell has the same character count, but not necessarily the same rendered width. In the
    // case that a fallback font is used, this may result in the substitute glyphs `␣` and `·` having different
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

    // `content_origin` is where the content box drawn below sits, local to `group`. It is `(0, 0)` exactly as before
    // this parameter existed, unless `name` wraps it in a further named outer box. In that case it is inset and centred
    // under that outer box's own label row instead. `named_outer_rect` is that wrapping box's own `<rect>`, which is
    // `Some` only when `name` draws one. It is read back below once `rect_el` is in scope too, to decide
    // `BoxHandles::outer_rect`: the named wrapper when there is one, `rect_el` itself otherwise.
    let (content_origin, size, named_outer_rect) = if let Some(name) = name {
        let label_el = svg.text(origin, name)?;
        guard.track(label_el.clone());
        label_el.set_text_anchor(TextAnchor::Middle)?;
        label_el.set_dominant_baseline(DominantBaseline::Middle)?;
        label_el.set_font_size(LABEL_FONT_SIZE)?;
        label_el.set_fill(TEXT_FILL)?;
        let label_width = label_el.bounding_box()?.size.width;

        // Same competition `draw_operator_box` resolves for its own label vs. value cell. The content box's own width,
        // plus `OUTER_PADDING` clear on either side, competes against the label's own width, plus `CELL_PADDING`.
        // Whichever needs more room sets the outer box's own width. Either way the content box itself never reaches the
        // outer box's own left/right edges.
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
    let mut cell_geometry = Vec::with_capacity(len);
    if single_value {
        cell_rects.push(rect_el.clone());
        cell_geometry.push(Rect {
            origin: content_origin,
            size: content_size,
        });
    }

    // Captured only in the `single_value` case. It is the same formatted text that one cell's own `<text>` renders,
    // reused again below for the node's own `aria-label`. So the value reads as text without visiting the cell
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
            if content.is_plain_text() {
                text.set_attr("style", "white-space: pre")?;
            }
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
                cell_geometry.push(Rect {
                    origin: cell_origin,
                    size: cell_size,
                });
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
    // tooltip too. Assistive technology usually announces a group before its children. So a reader need not visit every
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
            cell_geometry,
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
