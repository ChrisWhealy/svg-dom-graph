//! Plain label nodes — a box with a single centred text label, no typed content. See [`super::data`] for a node
//! whose content is a [`DataNodeContent`] grid instead, and [`super::operator`] for one labelled with the
//! operation that produced its own value.

use super::{EdgeAnchors, LABEL_FONT_SIZE, NodeOptions, render_guard::RenderGuard, validate_edge_anchors};
use crate::{
    colours::{BOX_STROKE, PLAIN_BOX_FILL, TEXT_FILL},
    error::Error,
    model::node::NodeId,
    scene::{BoxHandles, Scene, Selection, box_centre},
};
use svg_dom::{
    DominantBaseline, SvgNode, SvgRoot, TextAnchor,
    root::utils::{Point, Rect, Size},
};

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
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
pub(super) fn draw_box(
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
    rect_el.set_fill(PLAIN_BOX_FILL)?;
    rect_el.set_stroke(BOX_STROKE)?;
    rect_el.set_stroke_width(1.5)?;

    let label_el = svg.text(box_centre(local_rect), label)?;
    guard.track(label_el.clone());
    label_el.set_text_anchor(TextAnchor::Middle)?;
    label_el.set_dominant_baseline(DominantBaseline::Middle)?;
    label_el.set_font_size(LABEL_FONT_SIZE)?;
    label_el.set_fill(TEXT_FILL)?;
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
        ref_name: label.to_owned(),
    })
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
        inner.attach(&handles.group)?;
        let id = inner.graph.add_node(rect, label);
        inner.insert_node_handle(id, handles);
        Ok(id)
    }
}
