//! Markdown nodes: a box of prose, with headings, lists and bold, italic and code runs. See
//! [`crate::model::markdown`] for what the Markdown subset is and how its lines are wrapped. This file draws the lines
//! that module lays out.
//!
//! A Markdown node is a node like any other. It can be moved, and a connector can be anchored to it. It holds no cells,
//! so it has no selection. It is for explaining what a diagram is showing, next to the thing it explains.
//!
//! Every character is the same width, because the text is monospace. The width of one is measured once, from a real
//! element in the browser, so the wrapping is right for whatever font the browser substitutes.

use super::{
    EdgeAnchors, GRID_FONT_FAMILY, LABEL_FONT_SIZE, NodeOptions, OUTER_PADDING, render_guard::RenderGuard,
    validate_edge_anchors,
};
use crate::{
    colours::{BOX_STROKE, MARKDOWN_CODE_INK, PLAIN_BOX_FILL, TEXT_FILL},
    error::Error,
    model::{
        markdown::{self, Layout, Metrics},
        node::{NodeContent, NodeId},
    },
    scene::{BoxHandles, DataDecor, Scene, Selection},
};
use svg_dom::{
    DominantBaseline, SvgNode, SvgRoot, TextAnchor,
    root::utils::{Point, Rect, Size},
};

/// The body text's font size. Headings are larger, by [`crate::model::markdown`]'s own scale.
const BODY_FONT_SIZE: f64 = LABEL_FONT_SIZE;

/// How many characters the element that measures one character's width holds. Many, so that a small rounding in the
/// browser's own measurement is spread over all of them.
const MEASURE_CHARS: usize = 40;

/// The narrowest text area a node can have, so that a line always has room for a few characters.
const MIN_TEXT_WIDTH: f64 = 40.0;

/// What a node's accessible name reads, since its descendants are the text itself and are exposed as they are.
const ARIA_LABEL: &str = "Text";

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The width of one character of body text, measured from a real element, and removed again.
///
/// A browser that cannot measure, because the `<svg>` is not displayed, reports no width. Then the width of a character
/// is taken as 0.6 of the font size, which is close to what every common monospace font has.
fn measure_char_width(svg: &SvgRoot) -> Result<f64, Error> {
    let mut guard = RenderGuard::for_measurement();
    let el = svg.text(Point::origin(), &"0".repeat(MEASURE_CHARS))?;
    guard.track(el.clone());
    el.set_font_family(GRID_FONT_FAMILY)?;
    el.set_font_size(BODY_FONT_SIZE)?;
    let width = el.bounding_box()?.size.width;
    el.remove();
    guard.release();
    guard.disarm();

    let each = width / MEASURE_CHARS as f64;
    Ok(if each.is_finite() && each > 0.0 { each } else { BODY_FONT_SIZE * 0.6 })
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// A document, parsed and laid out for a particular width, and the size of the box that holds it.
struct Plan {
    layout: Layout,
    size: Size,
}

/// Parses and lays out `source` in a node `width` wide.
///
/// # Errors
///
/// Returns [`Error::InvalidNodeGeometry`] if `width` is not finite, leaves less than [`MIN_TEXT_WIDTH`] for the text,
/// or leaves less than the widest list marker in `source` needs, with one character beside it.
/// Returns [`Error::EmptyMarkdown`] if `source` has nothing to draw. Returns an error if measuring fails.
fn plan(svg: &SvgRoot, width: f64, source: &str) -> Result<Plan, Error> {
    if !width.is_finite() || width < 2.0 * OUTER_PADDING + MIN_TEXT_WIDTH {
        return Err(Error::InvalidNodeGeometry(Rect {
            origin: Point::origin(),
            size: Size::new(width, 0.0),
        }));
    }
    let blocks = markdown::parse(source);
    if blocks.is_empty() {
        return Err(Error::EmptyMarkdown);
    }
    let metrics = Metrics {
        char_width: measure_char_width(svg)?,
        font_size: BODY_FONT_SIZE,
    };
    // A numbered list can have a marker wider than the minimum, and then neither it nor its text would fit.
    if width - 2.0 * OUTER_PADDING < markdown::minimum_width(&blocks, metrics) {
        return Err(Error::InvalidNodeGeometry(Rect {
            origin: Point::origin(),
            size: Size::new(width, 0.0),
        }));
    }
    let layout = markdown::layout(&blocks, width - 2.0 * OUTER_PADDING, metrics);
    let size = Size::new(width, layout.height + 2.0 * OUTER_PADDING);
    Ok(Plan { layout, size })
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Draws one line of text: a `<text>` whose runs are `<tspan>`s, each in its own weight, slant and colour.
fn draw_line(svg: &SvgRoot, guard: &mut RenderGuard, group: &SvgNode, line: &markdown::Line) -> Result<(), Error> {
    let y = OUTER_PADDING + line.top + line.height / 2.0;

    if let Some(marker) = &line.marker {
        let el = svg.text(Point::new(OUTER_PADDING, y), marker)?;
        guard.track(el.clone());
        style_text(&el, line.size)?;
        group.append(&el)?;
        guard.release();
    }

    let el = svg.text(Point::new(OUTER_PADDING + line.indent, y), "")?;
    guard.track(el.clone());
    style_text(&el, line.size)?;
    for span in &line.spans {
        let run = el.tspan(&span.text)?;
        if span.style.bold {
            run.set_attr("font-weight", "bold")?;
        }
        if span.style.italic {
            run.set_attr("font-style", "italic")?;
        }
        if span.style.code {
            run.set_fill(MARKDOWN_CODE_INK)?;
        }
    }
    group.append(&el)?;
    guard.release();
    Ok(())
}

/// What every `<text>` of the node shares.
fn style_text(el: &SvgNode, size: f64) -> Result<(), Error> {
    el.set_text_anchor(TextAnchor::Start)?;
    el.set_dominant_baseline(DominantBaseline::Middle)?;
    el.set_font_family(GRID_FONT_FAMILY)?;
    el.set_font_size(size)?;
    el.set_fill(TEXT_FILL)?;
    // Keeps the single space between two words as it is, rather than letting the browser collapse it away at the edge of
    // a run.
    el.set_attr("style", "white-space: pre")?;
    Ok(())
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Draws a Markdown node's box and every line of its text, grouped under one `<g>`. Like every other node, everything
/// is drawn in local coordinates, and the group carries `top_left` as its `transform`, so moving the node rewrites that
/// one attribute and nothing else.
///
/// A [`RenderGuard`] removes whatever was drawn if any step fails.
fn draw_markdown_box(
    svg: &SvgRoot,
    scratch: &mut String,
    top_left: Point,
    plan: &Plan,
    name: &str,
    edge_anchors: Option<EdgeAnchors>,
) -> Result<(BoxHandles, Rect), Error> {
    let group = svg.group()?;
    let mut guard = RenderGuard::new(group.clone());

    let rect_el = svg.rect(Point::origin(), plan.size)?;
    guard.track(rect_el.clone());
    rect_el.set_fill(PLAIN_BOX_FILL)?;
    rect_el.set_stroke(BOX_STROKE)?;
    rect_el.set_stroke_width(1.5)?;
    group.append(&rect_el)?;
    guard.release();

    for line in &plan.layout.lines {
        draw_line(svg, &mut guard, &group, line)?;
    }

    group.set_attr("role", "group")?;
    group.set_attr("aria-label", ARIA_LABEL)?;
    group.set_transform_fmt(scratch, format_args!("translate({}, {})", top_left.x, top_left.y))?;
    guard.disarm();

    Ok((
        BoxHandles {
            group,
            draggable: false,
            enterable: false,
            edge_anchors,
            binary_operator_inputs: None,
            binary_operator_input_edges: None,
            outer_rect: rect_el,
            cell_rects: Vec::new(),
            cell_texts: Vec::new(),
            replaceable: false,
            cell_grid: None,
            decor: DataDecor::default(),
            cell_stroke_width: "",
            selection: Selection::None,
            secondary: Vec::new(),
            unreached: Vec::new(),
            aria_label: ARIA_LABEL.to_owned(),
            base_label_len: ARIA_LABEL.len(),
            label_stale: false,
            edge_anchors_stale: false,
            ref_name: name.to_owned(),
            child: None,
        },
        Rect {
            origin: top_left,
            size: plan.size,
        },
    ))
}

/// A short name for the node, so a later node's description can refer to it: the first line of its text, cut short.
fn short_name(source: &str) -> String {
    const MAX_CHARS: usize = 32;
    let plain = markdown::plain_text(&markdown::parse(source));
    let first = plain.lines().next().unwrap_or_default().trim();
    if first.chars().count() > MAX_CHARS {
        let cut: String = first.chars().take(MAX_CHARS).collect();
        format!("{}…", cut.trim_end())
    } else {
        first.to_owned()
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
impl Scene {
    /// Adds a node of Markdown text, `width` wide, and returns its id. The box grows downward to hold the text, so its
    /// height is whatever the text needs. [`node_rect`](Self::node_rect) reports it.
    ///
    /// Headings, paragraphs, bulleted and numbered lists, and bold, italic and code runs are drawn. The Markdown that is
    /// understood is listed under "Supported Markdown" below. Anything else is drawn as the characters it was typed
    /// with, so text that is not Markdown at all is a plain paragraph.
    ///
    /// Equivalent to [`add_markdown_node_with`](Self::add_markdown_node_with) with [`NodeOptions::default`].
    ///
    /// # Supported Markdown
    ///
    /// - `# `, `## ` and `### ` start a heading.
    /// - A line starting `- `, `* ` or `+ `, or a number then `.` or `)`, starts a list item. An indented line continues
    ///   it.
    /// - A blank line ends a paragraph. A line ending in two spaces or a backslash breaks the line.
    /// - `**bold**`, `*italic*`, `***both***` and `` `code` `` mark a run. A backslash makes the next punctuation literal.
    ///
    /// Links, images, quotes, tables, code blocks and nested lists are not supported. This crate does not depend on a
    /// Markdown parser, because the output is SVG text laid out here, not HTML.
    ///
    /// # Errors
    ///
    /// Returns [`Error::EmptyMarkdown`] if `markdown` holds nothing to draw. Returns [`Error::InvalidNodeGeometry`] if
    /// `top_left` is not finite, or `width` is not finite or is too narrow to hold a few characters of text, or the widest list marker in
    /// `markdown` and one character of text beside it.
    pub fn add_markdown_node(&self, top_left: Point, width: f64, markdown: &str) -> Result<NodeId, Error> {
        self.add_markdown_node_with(top_left, width, markdown, NodeOptions::default())
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    /// Adds a Markdown node, as [`add_markdown_node`](Self::add_markdown_node), but with `options` controlling how many
    /// connector fixing points its sides offer — see [`EdgeAnchors`].
    ///
    /// # Errors
    ///
    /// As [`add_markdown_node`](Self::add_markdown_node), plus [`Error::InvalidEdgeAnchors`] if `options.edge_anchors` is
    /// `Some(EdgeAnchors(0))`. Every check is made before anything is drawn, so a rejected call leaves the scene exactly
    /// as it was.
    pub fn add_markdown_node_with(
        &self,
        top_left: Point,
        width: f64,
        markdown: &str,
        options: NodeOptions,
    ) -> Result<NodeId, Error> {
        validate_edge_anchors(options.edge_anchors)?;
        if !top_left.x.is_finite() || !top_left.y.is_finite() {
            return Err(Error::InvalidNodeGeometry(Rect {
                origin: top_left,
                size: Size::new(width, 0.0),
            }));
        }

        let mut inner = self.inner.borrow_mut();
        let plan = plan(&inner.svg, width, markdown)?;
        let mut scratch = std::mem::take(&mut inner.scratch);
        let result = draw_markdown_box(
            &inner.svg,
            &mut scratch,
            top_left,
            &plan,
            &short_name(markdown),
            options.edge_anchors,
        );
        inner.scratch = scratch;
        let (handles, rect) = result?;
        inner.attach(&handles.group)?;
        let id = inner.graph.add_node(rect, NodeContent::Markdown(markdown.to_owned()));
        inner.insert_node_handle(id, handles);
        Ok(id)
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    /// The `Size` a Markdown node `width` wide would have for `markdown`, without adding it to the graph. This is what
    /// [`add_markdown_node`](Self::add_markdown_node) followed by [`node_rect`](Self::node_rect) would report, and it is
    /// the way to find where to put whatever sits below the node.
    ///
    /// # Measurement
    ///
    /// The width of a character is measured in the browser, and is taken as 0.6 of the font size when the browser cannot
    /// report one, as when the `<svg>` is not displayed. Lines break by character count, so a difference in that width
    /// between this call and the later one that draws the node can move a line break, and so change the height by a
    /// whole line. Measure and draw while the scene is displayed in the same way, and read the drawn node's real size
    /// from [`node_rect`](Self::node_rect) when exactness matters.
    ///
    /// # Errors
    ///
    /// As [`add_markdown_node`](Self::add_markdown_node), except for the check of `top_left`, which there is none of.
    pub fn measure_markdown_node(&self, width: f64, markdown: &str) -> Result<Size, Error> {
        Ok(plan(&self.inner.borrow().svg, width, markdown)?.size)
    }
}
