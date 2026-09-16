//! Node configuration and the `Scene` methods that add or reconfigure a node.

mod edge_anchors;
mod node_options;
mod render_guard;

use super::{BoxHandles, DataNodeContent, Scene, box_centre};
use crate::{error::Error, model::node::NodeId};
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
fn draw_box(svg: &SvgRoot, rect: Rect, label: &str, edge_anchors: Option<EdgeAnchors>) -> Result<BoxHandles, Error> {
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

    let mut scratch = String::new();
    // Not `set_translate`. Its fixed one-decimal-place precision would quantise the rendered position away from
    // the model's own `rect.origin`, by up to 0.05 user-space units. That is harmless to the eye. But it is a
    // real mismatch for code that re-derives a position from the rendered DOM, rather than from the model.
    // Several of this crate's own browser tests do exactly that. `set_transform_fmt` writes `Display`'s full
    // precision instead, at the cost of a (typically) longer attribute string.
    group.set_transform_fmt(&mut scratch, format_args!("translate({}, {})", rect.origin.x, rect.origin.y))?;

    guard.disarm();
    Ok(BoxHandles {
        group,
        draggable: false,
        edge_anchors,
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

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Draws a data node's rectangle and its grid of value cells, grouped under one `<g>`, and returns their handles
/// alongside the box's own final `Rect` — computed here, not supplied by the caller.
///
/// Every value gets its own `<text>` element (monospace — see [`GRID_FONT_FAMILY`]); its real, rendered width is read
/// back via [`SvgNode::bounding_box`] — the same "measure, don't estimate" approach [`shrink_label_to_fit`] already
/// uses for plain labels, and for the same reason: it stays correct for whatever font the browser actually substitutes.
/// Every cell shares one uniform size, the widest value's own measured width plus [`CELL_PADDING`], so the grid's rows
/// and columns actually line up even when [`DataFormat::Decimal`] values differ in digit count.
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
/// A [`RenderGuard`] covers this function's own DOM construction. This matters more here than in [`draw_box`]. A grid
/// can measure many cells before any of them is appended into `group`. That widens the window in which a `?` failing
/// partway through would otherwise leave stray elements behind.
fn draw_content_box(
    svg: &SvgRoot,
    top_left: Point,
    content: &DataNodeContent,
    edge_anchors: Option<EdgeAnchors>,
) -> Result<(BoxHandles, Rect), Error> {
    let group = svg.group()?;
    let mut guard = RenderGuard::new(group.clone());
    let type_color = content.type_color();
    let type_name = content.type_name();
    let origin = Point::origin();

    // Render every value's text first, at a placeholder position — bounding_box() reports each element's own
    // local geometry (font, content, styling), unaffected by where it currently sits, so the true final position
    // is not needed yet.
    let mut texts = Vec::new();
    let mut max_width: f64 = 0.0;
    for cell_text in &content.cells() {
        let text = svg.text(origin, cell_text)?;
        guard.track(text.clone());
        text.set_text_anchor(TextAnchor::Middle)?;
        text.set_dominant_baseline(DominantBaseline::Middle)?;
        text.set_font_family(GRID_FONT_FAMILY)?;
        text.set_font_size(GRID_FONT_SIZE)?;
        text.set_fill("#1b1b1b")?;
        max_width = max_width.max(text.bounding_box()?.size.width);
        texts.push(text);
    }

    let cell_size = Size::new(max_width + 2.0 * CELL_PADDING, CELL_HEIGHT + 2.0 * CELL_PADDING);
    let single_value = content.is_single_value();

    let size = if single_value {
        cell_size
    } else {
        let (grid_rows, grid_cols) = content.shape();
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

    let mut scratch = String::new();
    if single_value {
        let text = texts
            .into_iter()
            .next()
            .ok_or_else(|| Error::Svg(svg_dom::Error::Dom("draw_content_box: expected exactly one value".into())))?;
        text.set_attr_display(&mut scratch, "x", cell_size.width / 2.0)?;
        text.set_attr_display(&mut scratch, "y", cell_size.height / 2.0)?;
        group.append(&text)?;
    } else {
        let (_, grid_cols) = content.shape();
        for (i, text) in texts.into_iter().enumerate() {
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

            text.set_attr_display(&mut scratch, "x", cell_origin.x + cell_size.width / 2.0)?;
            text.set_attr_display(&mut scratch, "y", cell_origin.y + cell_size.height / 2.0)?;
            group.append(&text)?;
        }
    }

    // See `draw_box`'s own comment on its matching call for why `set_transform_fmt`, not `set_translate`.
    group.set_transform_fmt(&mut scratch, format_args!("translate({}, {})", top_left.x, top_left.y))?;

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
    Ok((
        BoxHandles {
            group,
            draggable: false,
            edge_anchors,
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
        let handles = draw_box(&inner.svg, rect, &label, options.edge_anchors)?;
        let id = inner.graph.add_node(rect, label);
        inner.node_handles.insert(id, handles);
        Ok(id)
    }

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
        let (handles, rect) = draw_content_box(&inner.svg, top_left, &content, options.edge_anchors)?;
        let id = inner.graph.add_node(rect, content);
        inner.node_handles.insert(id, handles);
        Ok(id)
    }

    /// Updates node `id`'s [`EdgeAnchors`] configuration, and redraws every incident connector immediately with the
    /// new value.
    ///
    /// This is the only way to change a node's anchor configuration after [`Scene::add_node`] or
    /// [`Scene::add_node_with`] first draws it — for example, from a live slider control.
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
        inner.node_handles.get_mut(&id).ok_or(Error::UnknownNode(id))?.edge_anchors = edge_anchors;

        let mut scratch = String::new();
        let incident: Vec<_> = inner.graph.incident_edges(id).to_vec();
        for edge_id in incident {
            inner.redraw_edge(edge_id, &mut scratch)?;
        }
        Ok(())
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[cfg(test)]
mod unit_tests;
