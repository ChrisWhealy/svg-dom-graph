//! An edge's label: a small box with a weight or a name in it, drawn beside the edge's connector.
//!
//! The label is a group of its own in the scene's content layer, beside the connector's path and not inside it. It is
//! not a node, so it has no edges, cannot be dragged and takes no part in a collision. Both the path and the label live
//! under the content layer, so panning and zooming move them together without either being routed again.
//!
//! Where the label goes is decided by [`crate::geometry::label`], which is pure. This module does the drawing, and the
//! bookkeeping that keeps a label beside its connector as the connector is routed again.
//!
//! # When a label is placed
//!
//! A connector's path and its label are both written from the one route that was just worked out. The route is kept in
//! the connector's handle, so a label is never placed from a route other than the one the path shows. A label is placed
//! again whenever its connector is: a node it joins is moved, an elbow is changed to a straight line, or the fixing
//! points of a node it joins are changed. See [`SceneInner::place_labels`].
//!
//! # When a write fails
//!
//! A label's box is moved by writing the `transform` of its group. What the label's handle says about where it is only
//! changes after that write has succeeded. So if a write fails, the handle still says where the label really is. The
//! error is returned to the caller, and the next time the connector is routed the label is placed again and moved if
//! it needs to be. Nothing marks the label as stale, because nothing needs to.
//!
//! # Ordering
//!
//! Labels are placed in the order their edges were added. A label that was placed earlier has priority: it is never
//! moved to make room for one placed later. See [`SceneInner::place_labels`] for the one place a lower-priority label
//! is moved.

use crate::{
    colours::{EDGE_LABEL_FILL, EDGE_LABEL_STROKE, TEXT_FILL},
    error::Error,
    geometry::label::{PlacerInput, place},
    model::edge::EdgeId,
    scene::{Scene, scene_inner::SceneInner},
};
use svg_dom::{
    DominantBaseline, SvgNode, SvgRoot, TextAnchor,
    root::utils::{Point, Rect, Size},
};

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The size of an edge label's text.
const LABEL_FONT_SIZE: f64 = 12.0;

/// The font of an edge label's text.
const LABEL_FONT_FAMILY: &str = "system-ui, sans-serif";

/// How much room the label's box leaves round its text, on each side.
const PADDING_X: f64 = 5.0;
const PADDING_Y: f64 = 3.0;

/// The attribute that marks the group of an edge label, whose value is the index of the edge it labels. A test, or a
/// page, can find the label of an edge with it without relying on the order of the elements in the document.
pub(crate) const LABEL_ATTRIBUTE: &str = "data-svg-dom-graph-edge-label";

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Where a label is now, and which candidate of its route that is.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct Placed {
    pub(crate) rect: Rect,
    pub(crate) candidate: usize,
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The rendered label of one edge.
pub(crate) struct EdgeLabel {
    group: SvgNode,
    background: SvgNode,
    text: SvgNode,
    /// The text of the label, as the caller gave it.
    content: String,
    /// The size of the label's box, measured once when the text was set. A label is placed from this, so no text is
    /// measured when a connector is dragged.
    size: Size,
    /// Where the label's group is drawn, as of the last write that succeeded. `None` before the first.
    placed: Option<Placed>,
}

impl EdgeLabel {
    /// Draws a label of `content` for the edge with index `edge_index`, and measures it. The group is not yet in the
    /// scene, and has no place: the caller attaches it, and places it.
    ///
    /// `name` is the label's accessible name, which says which edge it is on.
    fn draw(svg: &SvgRoot, edge_index: usize, content: &str, name: &str) -> Result<Self, Error> {
        let text = svg.text(Point::origin(), content)?;
        let drawn = (|| -> Result<Self, Error> {
            text.set_font_family(LABEL_FONT_FAMILY)?;
            text.set_font_size(LABEL_FONT_SIZE)?;
            text.set_text_anchor(TextAnchor::Middle)?;
            text.set_dominant_baseline(DominantBaseline::Middle)?;
            text.set_fill(TEXT_FILL)?;
            text.set_attr("style", "white-space: pre")?;
            let size = measured_box(&text)?;

            let background = svg.rect(Point::origin(), size)?;
            background.set_fill(EDGE_LABEL_FILL)?;
            background.set_stroke(EDGE_LABEL_STROKE)?;
            background.set_stroke_width(1.0)?;
            background.set_attr("rx", "3")?;

            let group = svg.group()?;
            group.append(&background)?;
            group.append(&text)?;
            centre_text(&text, size)?;
            group.set_attr(LABEL_ATTRIBUTE, &edge_index.to_string())?;
            group.set_attr("role", "group")?;
            group.set_attr("aria-label", name)?;
            group.set_title(name)?;
            // The label is not a control. A pointer over it reaches whatever is under it, so a label never gets in the
            // way of dragging a node or the background.
            group.set_attr("pointer-events", "none")?;
            Ok(Self {
                group,
                background,
                text: text.clone(),
                content: content.to_owned(),
                size,
                placed: None,
            })
        })();
        if drawn.is_err() {
            text.remove();
        }
        drawn
    }

    pub(crate) fn rect(&self) -> Option<Rect> {
        self.placed.map(|placed| placed.rect)
    }

    pub(crate) fn candidate(&self) -> Option<usize> {
        self.placed.map(|placed| placed.candidate)
    }

    pub(crate) fn remove(&self) {
        self.group.remove();
    }

    /// Writes new text for the label, measures it again, and sizes the box to it. The label keeps its place, and is
    /// moved if it needs to be by the placement that follows.
    fn set_content(&mut self, content: &str, name: &str) -> Result<(), Error> {
        self.text.set_text(content);
        let size = measured_box(&self.text)?;
        self.background.set_attr("width", &(size.width).to_string())?;
        self.background.set_attr("height", &(size.height).to_string())?;
        centre_text(&self.text, size)?;
        self.group.set_attr("aria-label", name)?;
        self.group.set_title(name)?;
        self.content.clear();
        self.content.push_str(content);
        self.size = size;
        // The box may have a new size, so it is not where it was placed. The next placement writes it again.
        self.placed = None;
        Ok(())
    }

    /// Moves the label's box to `placed`, writing nothing if it is already there. What the handle says is only changed
    /// once the write has succeeded. See "When a write fails" in this module's own doc comment.
    fn move_to(&mut self, placed: Placed, scratch: &mut String) -> Result<(), Error> {
        if self.placed.is_some_and(|now| now.rect.origin == placed.rect.origin) {
            self.placed = Some(placed);
            return Ok(());
        }
        let origin = placed.rect.origin;
        self.group
            .set_transform_fmt(scratch, format_args!("translate({}, {})", origin.x, origin.y))?;
        self.placed = Some(placed);
        Ok(())
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The size of the box for the text of `text`: what the browser measures it as, plus the padding.
fn measured_box(text: &SvgNode) -> Result<Size, Error> {
    let measured = text.bounding_box()?.size;
    Ok(Size::new(measured.width + 2.0 * PADDING_X, measured.height + 2.0 * PADDING_Y))
}

/// Puts `text` in the middle of a box of `size`, which has its own origin at `(0, 0)`.
fn centre_text(text: &SvgNode, size: Size) -> Result<(), Error> {
    text.set_attr("x", &(size.width / 2.0).to_string())?;
    text.set_attr("y", &(size.height / 2.0).to_string())?;
    Ok(())
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Whether `label` is acceptable: not empty, and one line without any control character.
fn is_valid(label: &str) -> bool {
    !label.is_empty() && !label.chars().any(char::is_control)
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
impl SceneInner {
    /// How the label of edge `id` describes itself: which edge it is on, and what it says.
    fn edge_label_name(&self, id: EdgeId, content: &str) -> Result<String, Error> {
        let edge = self.graph.edge(id).ok_or(Error::UnknownEdge(id))?;
        let (from, to) = (edge.from, edge.to);
        let from = self.node_handle(from).ok_or(Error::UnknownNode(from))?.current_ref_name();
        let to = self.node_handle(to).ok_or(Error::UnknownNode(to))?.current_ref_name();
        Ok(format!("Edge from {from} to {to}: {content}"))
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    /// Places the labels of `moved`, which are edges whose route has just been written, and then any label of lower
    /// priority that is now in the way of one of them. It does nothing, and costs nothing, in a scene with no labels.
    ///
    /// Edges are ranked by the order they were added in: an edge that was added earlier has priority. There are two
    /// passes, and never more, so the work is bounded and the result cannot oscillate.
    ///
    /// 1. **Initial placement.** Each label of `moved`, in priority order, is placed so that it keeps clear of the nodes
    ///    and of every label of higher priority, whether that label has just been placed or is where it was.
    /// 2. **One forward reconciliation.** A label that was not moved, and has lower priority than one that was, may now
    ///    overlap it. Such a label is placed again, against the nodes and every label of higher priority. A label of
    ///    higher priority than the one it overlaps is never moved, and nor is one that overlaps nothing.
    ///
    /// Moving a label in the second pass may put it in the way of a label of still lower priority. That is accepted. It
    /// is not worked out again, so the passes never chase one another.
    ///
    /// A label keeps the candidate it had while that is still clear, so that it does not jump about as a connector is
    /// dragged. See [`crate::geometry::label`].
    ///
    /// The nodes are read once, for the whole call. Each label is checked against them by a bounding box first, inside
    /// the placer, so a node a long way off costs a comparison and nothing more.
    ///
    /// # Errors
    ///
    /// Returns the error of the first write that fails. A label whose write failed is where it was, and it is placed
    /// again the next time its connector is routed. See "When a write fails" in this module's own doc comment.
    pub(crate) fn place_labels(&mut self, moved: &[EdgeId], scratch: &mut String) -> Result<(), Error> {
        if self.label_count == 0 {
            return Ok(());
        }
        let mut first_pass: Vec<usize> = moved
            .iter()
            .filter(|id| id.graph == self.graph.id)
            .map(|id| id.index)
            .filter(|&index| self.edge_handles.get(index).is_some_and(|handle| handle.label.is_some()))
            .collect();
        first_pass.sort_unstable();
        first_pass.dedup();
        if first_pass.is_empty() {
            return Ok(());
        }
        let vertices: Vec<Rect> = self.graph.nodes.iter().map(|node| node.rect).collect();

        for &index in &first_pass {
            self.place_one(index, &vertices, scratch)?;
        }

        // Every label that is now where it was not, and could be overlapped by one of lower priority.
        let placed: Vec<(usize, Rect)> = first_pass
            .iter()
            .filter_map(|&index| Some((index, self.edge_handles[index].label.as_ref()?.rect()?)))
            .collect();
        let mut lower: Vec<usize> = (0..self.edge_handles.len())
            .filter(|index| !first_pass.contains(index))
            .filter(|&index| {
                let Some(rect) = self.edge_handles[index].label.as_ref().and_then(EdgeLabel::rect) else {
                    return false;
                };
                placed
                    .iter()
                    .any(|&(above, other)| above < index && overlaps(padded(rect), other))
            })
            .collect();
        lower.sort_unstable();
        for index in lower {
            self.place_one(index, &vertices, scratch)?;
        }
        Ok(())
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    /// Places the label of the edge with index `index` from the route its connector was last written from, keeping clear
    /// of `vertices` and of every label of higher priority, and writes it if it has moved.
    fn place_one(&mut self, index: usize, vertices: &[Rect], scratch: &mut String) -> Result<(), Error> {
        let above: Vec<Rect> = self
            .edge_handles
            .iter()
            .take(index)
            .filter_map(|handle| handle.label.as_ref().and_then(EdgeLabel::rect))
            .collect();
        let handle = &self.edge_handles[index];
        let Some(label) = handle.label.as_ref() else {
            return Ok(());
        };
        let route = handle.route.get();
        let input = PlacerInput {
            route: &route,
            corner_radius: handle.radius.get(),
            label: label.size,
            vertices,
            others: &above,
            previous: label.candidate(),
        };
        let Some(placement) = place(&input) else {
            return Ok(());
        };
        let placed = Placed {
            rect: placement.rect,
            candidate: placement.candidate,
        };
        let label = self.edge_handles[index]
            .label
            .as_mut()
            .ok_or(Error::UnknownEdge(EdgeId { graph: self.graph.id, index }))?;
        label.move_to(placed, scratch)
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// `rect` made a little larger on every side, so that two labels that nearly touch count as overlapping.
fn padded(rect: Rect) -> Rect {
    Rect {
        origin: Point::new(rect.origin.x - 2.0, rect.origin.y - 2.0),
        size: Size::new(rect.size.width + 4.0, rect.size.height + 4.0),
    }
}

fn overlaps(a: Rect, b: Rect) -> bool {
    crate::geometry::rects_overlap(a, b)
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
impl Scene {
    /// Gives edge `id` a label with `label` in it, or takes its label away if `label` is `None`. A label is a small box
    /// with a weight or a name in it, drawn beside the edge's connector, with the text in the scene's own font.
    ///
    /// The label is not a node. It has no connectors, is not draggable, and the library does not look at what it says.
    /// A caller who labels the edges of a weighted graph with the weights does the arithmetic with them itself.
    ///
    /// It is placed on a straight run of the connector, clear of the line, the arrowhead and every node, and clear of
    /// the labels of edges that were added earlier. It is placed again whenever the connector is routed again: when
    /// either node of the edge is dragged or moved, when the connector type is changed, and when the fixing points of
    /// either node are. It keeps its place while it still can, so it does not jump about during a drag. If the graph is
    /// too crowded for any place to be free of every overlap, the label is put where it overlaps least. It is never
    /// hidden. The rules are in the crate's `geometry::label` module, and `docs/library/connectors.md` sets them out.
    ///
    /// The label has a background, so that it reads against a line, and is named for assistive technology as
    /// `"Edge from A to B: 7"`. It is in the same layer as the connector, so panning and zooming move the two together.
    ///
    /// Calling this again for an edge changes its text. Calling it with the text the label already has changes nothing
    /// but is placed again, which puts right a label whose last move failed.
    ///
    /// # Errors
    ///
    /// Returns [`Error::InvalidEdgeLabel`] if `label` is `Some` and is empty or has a control character in it, which
    /// includes a line break. Checked before the scene is touched.
    ///
    /// Returns [`Error::UnknownEdge`] if `id` does not name an edge in this scene.
    ///
    /// Also returns a wrapped [`Error::Svg`] if drawing or moving the label fails. A label that could not be drawn is
    /// not left behind. One that could not be moved is where it was, and is moved the next time its connector is routed.
    pub fn set_edge_label(&self, id: EdgeId, label: Option<&str>) -> Result<(), Error> {
        if label.is_some_and(|label| !is_valid(label)) {
            return Err(Error::InvalidEdgeLabel);
        }
        let mut inner = self.inner.borrow_mut();
        if inner.edge_handle(id).is_none() {
            return Err(Error::UnknownEdge(id));
        }

        let Some(content) = label else {
            let removed = inner.edge_handle_mut(id).and_then(|handle| handle.label.take());
            if let Some(removed) = removed {
                removed.remove();
                inner.label_count -= 1;
            }
            return Ok(());
        };

        let name = inner.edge_label_name(id, content)?;
        let exists = inner.edge_handle(id).is_some_and(|handle| handle.label.is_some());
        if exists {
            if let Some(existing) = inner.edge_handle_mut(id).and_then(|handle| handle.label.as_mut()) {
                if existing.content != content {
                    existing.set_content(content, &name)?;
                }
            }
        } else {
            let drawn = EdgeLabel::draw(&inner.svg, id.index, content, &name)?;
            if let Err(e) = inner.attach(&drawn.group) {
                drawn.remove();
                return Err(e);
            }
            if let Some(handle) = inner.edge_handle_mut(id) {
                handle.label = Some(drawn);
            }
            inner.label_count += 1;
        }

        let mut scratch = std::mem::take(&mut inner.scratch);
        let placed = inner.place_labels(&[id], &mut scratch);
        inner.scratch = scratch;
        placed
    }
}
