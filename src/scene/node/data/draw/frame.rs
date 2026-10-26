//! The box around a data node: its optional name frame, its toolbar row, and its measured size.

use super::TOOLBAR_MIN_WIDTH;
use super::TOOLBAR_ROW_HEIGHT;
use super::layout::GridLayoutMetrics;
use crate::{
    colours::{BOX_STROKE, NAMED_BOX_FILL, PLAIN_BOX_FILL, TEXT_FILL},
    error::Error,
    scene::{
        DataNodeContent,
        node::{CELL_PADDING, LABEL_FONT_SIZE, LABEL_ROW_HEIGHT, OUTER_PADDING, render_guard::RenderGuard},
    },
};
use svg_dom::{
    DominantBaseline, SvgNode, SvgRoot, TextAnchor,
    root::utils::{Point, Rect, Size},
};

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// What [`draw_name_frame`] drew: where the content box sits, the node's own overall size, and the wrapping box if the
/// node is named.
pub(super) struct NameFrame {
    /// Where the content box sits, local to the group. `(0, 0)` unless `name` wraps it in a further named outer box. In
    /// that case it is inset and centred under that outer box's own label row instead.
    pub(super) content_origin: Point,
    /// The node's own overall size. That of the outer box if there is one, otherwise the content box's own.
    pub(super) size: Size,
    /// The wrapping box's own `<rect>`, `Some` only when `name` draws one.
    pub(super) outer_el: Option<SvgNode>,
    /// The row held for a selection toolbar, local to the group, if the node has one.
    pub(super) toolbar_slot: Option<Rect>,
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Creates the `<text>` that shows a node's `name` in its label row, tracked by `guard`. [`draw_name_frame`] draws it, and
/// [`measure_content_box`] only reads its width, so both see the same font.
pub(super) fn name_label(svg: &SvgRoot, guard: &mut RenderGuard, name: &str) -> Result<SvgNode, Error> {
    let label_el = svg.text(Point::origin(), name)?;
    guard.track(label_el.clone());
    label_el.set_text_anchor(TextAnchor::Middle)?;
    label_el.set_dominant_baseline(DominantBaseline::Middle)?;
    label_el.set_font_size(LABEL_FONT_SIZE)?;
    label_el.set_fill(TEXT_FILL)?;
    Ok(label_el)
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The size of the named wrapper around a content box of `content_size`, given the measured width of the name. Whichever
/// is wider sets the box: the name plus [`CELL_PADDING`] each side, or the content box plus [`OUTER_PADDING`] each side.
/// The height adds the label row above, the `toolbar_row` under it, if the node has one, and [`OUTER_PADDING`] below.
pub(super) fn named_frame_size(label_width: f64, content_size: Size, toolbar_row: f64) -> Size {
    let toolbar_width = if toolbar_row > 0.0 { TOOLBAR_MIN_WIDTH } else { 0.0 };
    Size::new(
        (label_width + 2.0 * CELL_PADDING)
            .max(content_size.width + 2.0 * OUTER_PADDING)
            .max(toolbar_width),
        LABEL_ROW_HEIGHT + toolbar_row + content_size.height + OUTER_PADDING,
    )
}

/// The height of the row a named node holds for a selection toolbar: [`TOOLBAR_ROW_HEIGHT`] if `content` asked for one
/// and the node has a name, and nothing otherwise.
pub(super) fn toolbar_row(name: Option<&str>, content: &DataNodeContent) -> f64 {
    if name.is_some() && content.has_toolbar_row() {
        TOOLBAR_ROW_HEIGHT
    } else {
        0.0
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The size [`draw_content_box`] would give a node of `name` and `content`, without drawing it: no cell `<rect>` or
/// `<text>`, no handles and no accessible names. Only the few elements measuring needs are created, and all are removed
/// again.
///
/// It uses the same [`GridLayoutMetrics::measure`], [`GridLayoutMetrics::content_size`], [`name_label`] and
/// [`named_frame_size`] that drawing does, so the two cannot disagree. Finding the widest cell can still look at every
/// value under decimal formatting, but the DOM work no longer grows with the value count.
///
/// # Errors
///
/// Returns `Err` if `content` is empty, or if creating or measuring an element fails.
pub(crate) fn measure_content_box(svg: &SvgRoot, name: Option<&str>, content: &DataNodeContent) -> Result<Size, Error> {
    if content.len() == 0 {
        return Err(Error::Svg(svg_dom::Error::Dom(
            "measure_content_box: content length must be > 0".into(),
        )));
    }
    // Every temporary element is removed by hand as soon as it has been measured. The guard only removes one that a
    // failure left behind, so no group is needed.
    let mut guard = RenderGuard::for_measurement();
    let content_size = GridLayoutMetrics::measure(svg, &mut guard, content)?.content_size();
    let size = match name {
        Some(name) => {
            let label_el = name_label(svg, &mut guard, name)?;
            let label_width = label_el.bounding_box()?.size.width;
            label_el.remove();
            guard.release();
            named_frame_size(label_width, content_size, toolbar_row(Some(name), content))
        },
        None => content_size,
    };
    guard.disarm();
    Ok(size)
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Draws the named wrapper, if `name` is given: an outer box with `name` in a label row above where the content box
/// will sit. Without a name there is nothing to draw, and the content box starts flush at `(0, 0)`.
///
/// Same competition `draw_operator_box` resolves for its own label vs. value cell. The content box's own width, plus
/// [`OUTER_PADDING`] clear on either side, competes against the label's own width, plus [`CELL_PADDING`]. Whichever
/// needs more room sets the outer box's own width. Either way the content box itself never reaches the outer box's own
/// left/right edges.
pub(super) fn draw_name_frame(
    svg: &SvgRoot,
    scratch: &mut String,
    group: &SvgNode,
    guard: &mut RenderGuard,
    name: Option<&str>,
    content: &DataNodeContent,
    content_size: Size,
) -> Result<NameFrame, Error> {
    let origin = Point::origin();
    let Some(name) = name else {
        return Ok(NameFrame {
            content_origin: origin,
            size: content_size,
            outer_el: None,
            toolbar_slot: None,
        });
    };
    let toolbar_row = toolbar_row(Some(name), content);

    let label_el = name_label(svg, guard, name)?;
    let label_width = label_el.bounding_box()?.size.width;

    let box_size = named_frame_size(label_width, content_size, toolbar_row);
    let box_width = box_size.width;

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
        content_origin: Point::new((box_width - content_size.width) / 2.0, LABEL_ROW_HEIGHT + toolbar_row),
        size: box_size,
        outer_el: Some(outer_el),
        toolbar_slot: (toolbar_row > 0.0).then(|| Rect {
            origin: Point::new(0.0, LABEL_ROW_HEIGHT),
            size: Size::new(box_width, toolbar_row),
        }),
    })
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Draws the content box itself. A single value fills it with the value's own type colour. A grid leaves it the plain
/// box colour, for its cells to sit in.
pub(super) fn draw_content_rect(
    svg: &SvgRoot,
    group: &SvgNode,
    guard: &mut RenderGuard,
    content: &DataNodeContent,
    content_origin: Point,
    content_size: Size,
) -> Result<SvgNode, Error> {
    let rect_el = svg.rect(content_origin, content_size)?;
    guard.track(rect_el.clone());
    rect_el.set_fill(if content.is_single_value() || content.hex_dump().is_some() {
        content.type_colour()
    } else {
        PLAIN_BOX_FILL
    })?;
    rect_el.set_stroke(BOX_STROKE)?;
    rect_el.set_stroke_width(1.5)?;
    group.append(&rect_el)?;
    guard.release();
    Ok(rect_el)
}
