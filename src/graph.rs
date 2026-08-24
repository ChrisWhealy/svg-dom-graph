//! Builds the demo scene: three boxes arranged as a small directed tree, connected by arrow-tipped connectors.
//!
//! The two child boxes are draggable.
//! Dragging one recomputes and redraws the connector attached to it, on every pointer-move.

use crate::geometry::boundary_point;
use std::{cell::Cell, rc::Rc};
use svg_dom::{
    DominantBaseline, Error, MarkerUnits, SvgMarker, SvgNode, SvgRoot, TextAnchor,
    root::utils::{Point, Rect, Size},
};

/// One node in the graph: a labelled box whose position can change.
///
/// `rect` is a shared, mutable cell rather than a plain value.
/// Dragging a box writes its new position directly into this cell.
/// Anything holding a clone of the same `Rc` can read the box's current position.
/// A `GraphEdge`, for example, needs no separate callback to learn that a box moved.
///
/// Routing a connector between two boxes only ever needs their current `Rect`s.
/// The geometry in [`crate::geometry`] does not depend on how, or whether, a box has already been drawn.
pub struct GraphBox {
    pub rect: Rc<Cell<Rect>>,
    pub label: &'static str,
}

impl GraphBox {
    pub fn new(top_left: Point, size: Size, label: &'static str) -> Self {
        Self {
            rect: Rc::new(Cell::new(Rect { origin: top_left, size })),
            label,
        }
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The rendered elements that make up one box, kept so a drag handler can reposition them.
struct BoxHandles {
    /// The `<g>` wrapping `rect_el` and `label_el`.
    /// Event listeners attach here, so a click on either child starts a drag.
    group: SvgNode,
    rect_el: SvgNode,
    label_el: SvgNode,
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// A directed connector between two boxes.
///
/// `from`/`to` are clones of the same `Rc<Cell<Rect>>` each box owns.
/// [`redraw`](Self::redraw) always reads the boxes' current positions, so it stays correct after either box moves.
struct GraphEdge {
    from: Rc<Cell<Rect>>,
    to: Rc<Cell<Rect>>,
    connector: SvgNode,
}

impl GraphEdge {
    /// Recomputes both endpoints from the current box positions and rewrites the connector's line coordinates.
    ///
    /// `scratch` is a caller-owned buffer, reused across calls to avoid a fresh allocation on every redraw.
    /// See [`SvgNode::set_attr_display`].
    fn redraw(&self, scratch: &mut String) -> Result<(), Error> {
        let from_rect = self.from.get();
        let to_rect = self.to.get();

        let start = boundary_point(from_rect, box_centre(to_rect));
        let end = boundary_point(to_rect, box_centre(from_rect));

        self.connector.set_attr_display(scratch, "x1", start.x)?;
        self.connector.set_attr_display(scratch, "y1", start.y)?;
        self.connector.set_attr_display(scratch, "x2", end.x)?;
        self.connector.set_attr_display(scratch, "y2", end.y)?;

        Ok(())
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The centre point of a box's rectangle.
fn box_centre(rect: Rect) -> Point {
    Point::new(rect.origin.x + rect.size.width / 2.0, rect.origin.y + rect.size.height / 2.0)
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Defines a small filled-triangle arrowhead marker in `<defs>` and returns its handle.
///
/// `ref_x`/`ref_y` place the marker's anchor point (the tip of the triangle) at the very end of the line it attaches
/// to.
/// `orient("auto")` then rotates the marker to follow that line's own direction.
fn define_arrow_marker(svg: &SvgRoot) -> Result<SvgMarker, Error> {
    let defs = svg.defs()?;
    let marker = defs.marker("arrow")?;

    marker.set_units(MarkerUnits::UserSpaceOnUse)?;
    marker.set_marker_width(10.0)?;
    marker.set_marker_height(7.0)?;
    marker.set_ref_x(9.0)?;
    marker.set_ref_y(3.5)?;
    marker.set_orient("auto")?;
    marker.polygon(&[Point::new(0.0, 0.0), Point::new(10.0, 3.5), Point::new(0.0, 7.0)])?;

    Ok(marker)
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Draws a box's rectangle and its centred label, grouped under one `<g>`, and returns their handles.
fn draw_box(svg: &SvgRoot, node: &GraphBox) -> Result<BoxHandles, Error> {
    let group = svg.group()?;
    let rect = node.rect.get();

    let rect_el = svg.rect(rect.origin, rect.size)?;
    rect_el.set_fill("#eef4ff")?;
    rect_el.set_stroke("#2a5db0")?;
    rect_el.set_stroke_width(1.5)?;

    let label_el = svg.text(box_centre(rect), node.label)?;
    label_el.set_text_anchor(TextAnchor::Middle)?;
    label_el.set_dominant_baseline(DominantBaseline::Middle)?;
    label_el.set_font_size(14.0)?;
    label_el.set_fill("#1b1b1b")?;

    group.append(&rect_el)?;
    group.append(&label_el)?;

    Ok(BoxHandles { group, rect_el, label_el })
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Draws a directed connector from `from`'s boundary to `to`'s boundary, with an arrowhead at the `to` end.
///
/// Both endpoints are computed from the two boxes' centres via [`boundary_point`].
/// The connector then starts and ends exactly on each box's edge, not passing through its interior.
fn draw_edge(svg: &SvgRoot, from: &GraphBox, to: &GraphBox, arrow: &SvgMarker) -> Result<GraphEdge, Error> {
    let from_rect = from.rect.get();
    let to_rect = to.rect.get();

    let start = boundary_point(from_rect, box_centre(to_rect));
    let end = boundary_point(to_rect, box_centre(from_rect));

    let connector = svg.line(start, end)?;
    connector.set_stroke("#555")?;
    connector.set_stroke_width(1.5)?;
    connector.set_marker_end_ref(arrow)?;

    Ok(GraphEdge {
        from: from.rect.clone(),
        to: to.rect.clone(),
        connector,
    })
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The pointer position and box origin recorded when a drag starts.
///
/// A delta between the pointer's current position and `pointer` gives how far to move `box_origin`.
#[derive(Clone, Copy)]
struct DragStart {
    pointer: Point,
    box_origin: Point,
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Wires up pointer dragging on `handles.group`, moving `node` and redrawing `edges` as the pointer moves.
///
/// This demo's `<svg>` has no CSS scaling and its `viewBox` matches its pixel size, so one CSS pixel of pointer
/// movement equals one user-space unit.
/// A scaled or resized `<svg>` would need to convert `clientX`/`clientY` through the SVG's current transform first.
fn make_draggable(node: &GraphBox, handles: &BoxHandles, edges: Vec<GraphEdge>) -> Result<(), Error> {
    handles.group.set_attr("style", "cursor: grab; touch-action: none;")?;

    let drag_start: Rc<Cell<Option<DragStart>>> = Rc::new(Cell::new(None));

    {
        let group = handles.group.clone();
        let rect_state = node.rect.clone();
        let drag_start = drag_start.clone();
        handles.group.on_pointerdown(move |evt| {
            let _ = group.as_element().set_pointer_capture(evt.pointer_id());
            let _ = group.set_attr("style", "cursor: grabbing; touch-action: none;");
            drag_start.set(Some(DragStart {
                pointer: Point::new(evt.client_x() as f64, evt.client_y() as f64),
                box_origin: rect_state.get().origin,
            }));
        })?;
    }

    {
        let rect_state = node.rect.clone();
        let rect_el = handles.rect_el.clone();
        let label_el = handles.label_el.clone();
        let drag_start = drag_start.clone();
        // Reused across every pointermove call in this drag — and across drags, since the closure's environment
        // persists between invocations — rather than allocating a fresh String each time. See
        // `SvgNode::set_attr_display`'s own doc comment for why this pattern exists.
        let mut scratch = String::new();
        handles.group.on_pointermove(move |evt| {
            let Some(start) = drag_start.get() else { return };

            let pointer_now = Point::new(evt.client_x() as f64, evt.client_y() as f64);
            let new_origin = Point::new(
                start.box_origin.x + (pointer_now.x - start.pointer.x),
                start.box_origin.y + (pointer_now.y - start.pointer.y),
            );

            let size = rect_state.get().size;
            rect_state.set(Rect { origin: new_origin, size });

            let _ = rect_el.set_attr_display(&mut scratch, "x", new_origin.x);
            let _ = rect_el.set_attr_display(&mut scratch, "y", new_origin.y);

            let centre = box_centre(rect_state.get());
            let _ = label_el.set_attr_display(&mut scratch, "x", centre.x);
            let _ = label_el.set_attr_display(&mut scratch, "y", centre.y);

            for edge in &edges {
                let _ = edge.redraw(&mut scratch);
            }
        })?;
    }

    {
        let group = handles.group.clone();
        let drag_start = drag_start.clone();
        handles.group.on_pointerup(move |evt| {
            let _ = group.as_element().release_pointer_capture(evt.pointer_id());
            let _ = group.set_attr("style", "cursor: grab; touch-action: none;");
            drag_start.set(None);
        })?;
    }

    Ok(())
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Builds the demo scene: a root box with two children, connected by directed, arrow-tipped edges.
/// This is a minimal directed tree — the simplest case of the general graph this crate targets.
///
/// The two child boxes are draggable.
/// Their connectors stay attached to the root and redraw as each child moves.
pub fn build_demo_tree(svg: &SvgRoot) -> Result<(), Error> {
    let arrow = define_arrow_marker(svg)?;

    let box_size = Size::new(90.0, 50.0);
    let root = GraphBox::new(Point::new(155.0, 20.0), box_size, "Root");
    let left = GraphBox::new(Point::new(25.0, 180.0), box_size, "Left child");
    let right = GraphBox::new(Point::new(285.0, 180.0), box_size, "Right child");

    draw_box(svg, &root)?;
    let left_handles = draw_box(svg, &left)?;
    let right_handles = draw_box(svg, &right)?;

    let left_edge = draw_edge(svg, &root, &left, &arrow)?;
    let right_edge = draw_edge(svg, &root, &right, &arrow)?;

    make_draggable(&left, &left_handles, vec![left_edge])?;
    make_draggable(&right, &right_handles, vec![right_edge])?;

    Ok(())
}
