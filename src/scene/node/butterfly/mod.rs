//! The butterfly: a multiplication, an addition and a subtraction drawn and moved as one unit.
//!
//! A butterfly is the building block of a fast Fourier transform. It takes two values `a` and `b` and a twiddle factor
//! `w`, and produces `a + w·b` and `a − w·b`. [`Scene::add_butterfly_node`] draws it as three ordinary operator nodes
//! inside a labelled frame, in the layout of the usual signal-flow diagram:
//!
//! ```text
//!   ┌ Butterfly ──────────────────┐
//!   (A)──────────────—───► [ ADD ]│
//!   │                ╲ ╱          │
//!   (B)─┐             ╳           │
//!   (W)─┴► [ MUL ] ──╱ ╲─► [ SUB ]│
//!   └─────────────────────────────┘
//! ```
//!
//! The three inputs arrive at three small circles on the frame's west edge, labelled `A`, `B` and `W`. They are the
//! frame's fixing points. A connector from each of `a`, `b` and `w` ends at its circle, and the connectors inside the
//! frame start at them: `A` to the addition and to the subtraction, `B` and `W` to the multiplication, and the product to
//! the addition and the subtraction. So the two lines from `A` and the product to the subtraction and the addition cross
//! once, and no other two do. Every connector is straight except two: the product runs up out of the multiplication's
//! north edge into the addition, and `A` runs into the subtraction, each as an elbow. Each operator node takes its two
//! inputs on its own west side, and its output leaves from the middle of its east side.
//!
//! The frame is a plain node that is draggable. Dragging it carries the circles and the three operator nodes along, and
//! their connectors follow. All of them are ordinary nodes in every other respect: `Scene::set_data_values` replaces the
//! results of the operator nodes, and edges can leave them. As for every operator, `svg-dom-graph` never evaluates the
//! arithmetic. The caller supplies the three results.

use super::{NodeOptions, construction_guard::SceneRollback};
use crate::geometry::SAME_SIDE_CLEARANCE;
use crate::{
    error::Error,
    model::{edge::EdgeId, node::NodeId},
    scene::{
        ArithmeticOperator, CollisionPolicy, ConnectorOptions, ConnectorType, DataNodeContent, DragOptions, DragPhase,
        EllipseOptions, Scene, Side,
    },
};
use std::rc::Rc;
use svg_dom::{
    DominantBaseline, TextAnchor,
    root::utils::{Point, Rect, Size},
};

/// The gap between the frame's edge and the operator nodes inside it.
const FRAME_PADDING: f64 = 14.0;
/// The room above the addition for the frame's title.
const TITLE_ROOM: f64 = 30.0;
/// The gap between the multiplication and the two nodes that read its product.
///
/// An elbow into one of those nodes turns on a channel [`SAME_SIDE_CLEARANCE`] in front of it. So with a gap of twice that,
/// the channel is exactly half way across the gap, and the elbow from `A` into the subtraction runs down the middle of
/// the space between the multiplication and the subtraction.
const COLUMN_GAP: f64 = 2.0 * SAME_SIDE_CLEARANCE;
/// The gap between the addition and the subtraction, which the crossing lines span.
const ROW_GAP: f64 = 60.0;
/// How far the frame's node reaches west of the box that is drawn, to hold the half of each port that sticks out of it.
///
/// A node is kept inside the visible area as a rectangle, and that is the frame's rectangle. So it must contain the ports as
/// well as the box, or dragging the frame to the left edge would clip them. It is a little more than a port's radius, since
/// a port grows to hold its letter, and a letter is wider in some browsers than in others.
const PORT_PROTRUSION: f64 = 10.0;
/// The radius of an input port, a circle centred on the frame's west edge. It is as small as holds its letter.
const PORT_RADIUS: f64 = 8.0;
/// The size of a port's letter, smaller than a node's label so that the port can be small.
const PORT_FONT_SIZE: f64 = 9.0;
/// The clear space between a port's letter and its outline.
const PORT_MARGIN: f64 = 1.0;
/// How far the multiplication stands from the ports, so the connectors from `B` and `W` have room for their arrowheads.
const PORT_RUN: f64 = 34.0;
/// The gap between the two ports that feed the multiplication, edge to edge.
const PORT_GAP: f64 = 10.0;
/// The fill of the frame, lighter than a plain node so the operator nodes stand out from it.
const FRAME_FILL: &str = "#eef1f6";
/// The frame's label, which is also its accessible name.
const FRAME_LABEL: &str = "Radix-2 Butterfly";

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The three data nodes a butterfly reads: `a`, `b` and the twiddle factor `w`.
///
/// The butterfly computes `a + w·b` and `a − w·b`. The twiddle factor and `b` must be different nodes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub struct ButterflyOperands {
    pub a: NodeId,
    pub b: NodeId,
    pub twiddle: NodeId,
}

impl ButterflyOperands {
    #[must_use]
    pub fn new(a: NodeId, b: NodeId, twiddle: NodeId) -> Self {
        Self { a, b, twiddle }
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The three values a butterfly shows, each already worked out by the caller: `product` is `w·b`, `sum` is `a + product`
/// and `difference` is `a − product`.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub struct ButterflyResults {
    pub product: DataNodeContent,
    pub sum: DataNodeContent,
    pub difference: DataNodeContent,
}

impl ButterflyResults {
    #[must_use]
    pub fn new(product: DataNodeContent, sum: DataNodeContent, difference: DataNodeContent) -> Self {
        Self { product, sum, difference }
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The four nodes of a butterfly that [`Scene::add_butterfly_node`] returns.
///
/// Use `sum` and `difference` as the operands of whatever reads the butterfly's outputs. Use `product`, `sum` and
/// `difference` with `Scene::set_data_values` to show new results. The three ports are circle nodes that hold no values.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub struct Butterfly {
    /// The labelled frame, which is the node to drag.
    pub frame: NodeId,
    /// The input port `A`, a circle on the frame's west edge. Its connector from `a` ends here, and the connectors to the
    /// addition and the subtraction start here.
    pub port_a: NodeId,
    /// The input port `B`, which feeds the multiplication.
    pub port_b: NodeId,
    /// The input port `W`, the twiddle factor's, which feeds the multiplication.
    pub port_w: NodeId,
    /// The multiplication `w·b`.
    pub product: NodeId,
    /// The addition `a + w·b`.
    pub sum: NodeId,
    /// The subtraction `a − w·b`.
    pub difference: NodeId,
}

impl Butterfly {
    /// Every node that is carried along with the frame.
    fn members(self) -> [NodeId; 6] {
        [self.port_a, self.port_b, self.port_w, self.product, self.sum, self.difference]
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Where each part of a butterfly goes, measured before anything is drawn.
struct Layout {
    frame: Rect,
    product: Point,
    sum: Point,
    difference: Point,
    /// The centres of the three input ports `A`, `B` and `W`. Each is on the frame's west edge.
    ports: [Point; 3],
}

/// The heights at which the two inputs of an operator node on its west side meet a box `height` high.
///
/// A node with no fixing points configured splits its two inputs over the outer two of three candidates, a quarter and
/// three quarters of the way down. Its output then leaves from the middle of the opposite side.
fn input_slots(height: f64) -> [f64; 2] {
    [height / 4.0, height * 3.0 / 4.0]
}

/// Lays the three operator boxes out in two rows and two columns inside a frame whose top left is `top_left`.
///
/// The addition is in the first row and the second column, the multiplication in the second row and the first column,
/// and the subtraction in the second row and the second column. The multiplication stands where the middle of its east
/// side is level with the subtraction's second input, so the product runs horizontally into the subtraction, and up
/// from the multiplication's north edge into the addition. Input `A` runs straight into the
/// addition and slopes down into the subtraction, so those two lines cross once, between the columns. There is room above
/// the addition for the frame's title.
///
/// The three input ports are circles centred on the west edge of the box that is drawn, which is [`PORT_PROTRUSION`] in
/// from the west edge of the frame's rectangle, so that the rectangle holds them whole. `A` is level with the addition's first fixing
/// point. `B` and `W` are one above the other, centred on the multiplication, and far enough apart not to touch.
fn layout(top_left: Point, product: Size, sum: Size, difference: Size) -> Layout {
    let left = PORT_PROTRUSION + PORT_RADIUS + PORT_RUN;
    let right_x = left + product.width + COLUMN_GAP;
    let right_width = sum.width.max(difference.width);
    let first_row = TITLE_ROOM;
    // The product leaves from the middle of the multiplication's east side, and runs into the subtraction's second
    // input. Put the multiplication where those two are level, so that the connector between them is horizontal.
    let [_, subtraction_second_input] = input_slots(difference.height);
    let second_row = first_row + sum.height + ROW_GAP;
    let product_offset = subtraction_second_input - product.height / 2.0;
    // Whichever of the two sticks out above the other sets the top of the second row.
    let shift = (-product_offset).max(0.0);
    let (difference_y, product_y) = (second_row + shift, second_row + shift + product_offset);
    let bottom = (difference_y + difference.height).max(product_y + product.height);
    let [a_slot, _] = input_slots(sum.height);
    let apart = PORT_RADIUS + PORT_GAP / 2.0;
    let product_middle = product_y + product.height / 2.0;
    let at = |x: f64, y: f64| Point::new(top_left.x + x, top_left.y + y);
    Layout {
        frame: Rect {
            origin: top_left,
            size: Size::new(right_x + right_width + FRAME_PADDING, bottom + FRAME_PADDING),
        },
        product: at(left, product_y),
        sum: at(right_x, first_row),
        difference: at(right_x, difference_y),
        ports: [
            at(PORT_PROTRUSION, first_row + a_slot),
            at(PORT_PROTRUSION, product_middle - apart),
            at(PORT_PROTRUSION, product_middle + apart),
        ],
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Returns [`Error::ButterflyNotComplex`] unless `content` is one complex number.
fn require_complex_result(content: &DataNodeContent) -> Result<(), Error> {
    if content.holds_complex() && content.len() == 1 {
        Ok(())
    } else {
        Err(Error::ButterflyNotComplex(None))
    }
}

impl Scene {
    /// Adds a butterfly: a multiplication, an addition and a subtraction inside one draggable frame. It wires them to
    /// `operands` and returns their ids.
    ///
    /// The multiplication reads the twiddle factor and `b`. The addition and the subtraction each read `a` and the
    /// product, so they show `a + w·b` and `a − w·b`. `svg-dom-graph` never does this arithmetic. `results` must already
    /// hold the three values, worked out by the caller.
    ///
    /// `top_left` is the top left corner of the frame's rectangle, which holds the frame's box, its three operator nodes
    /// and the half of each input port that sticks out west of the box. The box itself begins a little east of that
    /// corner. So dragging the frame to the edge of the visible area keeps the ports whole.
    /// The frame is already draggable, and dragging it carries the three operator nodes along.
    /// To move a butterfly from code, use [`move_butterfly`](Self::move_butterfly).
    /// Do not call `make_draggable` on [`Butterfly::frame`]: it is already draggable.
    ///
    /// # Errors
    ///
    /// * [`Error::InvalidNodeGeometry`] if `top_left` is not finite.
    /// * [`Error::UnknownNode`] or [`Error::OperandNotData`] if an operand is not a data node of this scene.
    /// * [`Error::ButterflyNotComplex`] if an operand or a result is not a single complex number.
    /// * [`Error::OperatorTypeMismatch`] if the operands and results are not all `Complex32` or all `Complex64`.
    /// * [`Error::DuplicateOperands`] if the twiddle factor and `b` are the same node.
    /// * [`Error::OperatorResultNotSingleValue`], [`Error::EmptyNodeContent`] or [`Error::InvalidComplexFormat`] for a
    ///   result that no operator node could show.
    ///
    /// Every one of these is checked before anything is drawn. A failure while drawing removes everything that this
    /// call added, so a failed call leaves the scene as it was.
    pub fn add_butterfly_node(
        &self,
        top_left: Point,
        operands: ButterflyOperands,
        results: ButterflyResults,
    ) -> Result<Butterfly, Error> {
        if !top_left.x.is_finite() || !top_left.y.is_finite() {
            return Err(Error::InvalidNodeGeometry(Rect {
                origin: top_left,
                size: Size::new(0.0, 0.0),
            }));
        }
        if operands.twiddle == operands.b {
            return Err(Error::DuplicateOperands(operands.b));
        }
        {
            let inner = self.inner.borrow();
            for id in [operands.a, operands.b, operands.twiddle] {
                // Each operand is one complex number, as each result is: a grid of them has no single value to multiply.
                let content = super::operator::operand_content(&inner.graph, id)?;
                if !content.holds_complex() || content.len() != 1 {
                    return Err(Error::ButterflyNotComplex(Some(id)));
                }
            }
            for result in [&results.product, &results.sum, &results.difference] {
                require_complex_result(result)?;
                super::operator::check_operand_types(
                    &inner.graph,
                    &[operands.a, operands.b, operands.twiddle],
                    result,
                )?;
            }
        }

        let product_size = self.measure_operator_box(ArithmeticOperator::Multiply.label(), &results.product)?;
        let sum_size = self.measure_operator_box(ArithmeticOperator::Add.label(), &results.sum)?;
        let difference_size = self.measure_operator_box(ArithmeticOperator::Subtract.label(), &results.difference)?;
        let layout = layout(top_left, product_size, sum_size, difference_size);

        let rollback = SceneRollback::new(self.clone());
        // The frame goes first, so it is drawn behind the operator nodes that sit on it.
        let frame = self.add_node_with(
            layout.frame.origin,
            layout.frame.size,
            FRAME_LABEL,
            NodeOptions::default().with_fill(Some(FRAME_FILL)),
        )?;
        self.shape_frame(frame, layout.frame.size.width)?;
        // The three input ports are circles on the frame's west edge. They are nodes, so a connector can end at one, and
        // another can start from it.
        let port = |at: Point, name: &str| {
            self.add_circle_node_with(
                at,
                PORT_RADIUS,
                name,
                EllipseOptions::default()
                    .with_inner_margin(PORT_MARGIN)
                    .with_font_size(Some(PORT_FONT_SIZE)),
            )
        };
        let [at_a, at_b, at_w] = layout.ports;
        let (port_a, port_b, port_w) = (port(at_a, "A")?, port(at_b, "B")?, port(at_w, "W")?);
        // Each of `a`, `b` and the twiddle factor is wired to its own port.
        let straight = ConnectorOptions::default().with_connector_type(ConnectorType::Straight);
        for (from, to) in [(operands.a, port_a), (operands.b, port_b), (operands.twiddle, port_w)] {
            self.add_edge_with(from, to, straight)?;
        }
        // Both inputs of every operator node arrive on its west side, one above the other, and its output leaves from the
        // middle of its east side. The operands are the ports and the product. The values they stand for were checked
        // above, so the operands are not checked.
        let options = NodeOptions::default().with_input_side(Some(Side::West));
        let product = self.add_arithmetic_operator_between_ports(
            layout.product,
            ArithmeticOperator::Multiply,
            (port_w, port_b),
            results.product,
            options,
        )?;
        let sum = self.add_arithmetic_operator_between_ports(
            layout.sum,
            ArithmeticOperator::Add,
            (port_a, product),
            results.sum,
            options,
        )?;
        let difference = self.add_arithmetic_operator_between_ports(
            layout.difference,
            ArithmeticOperator::Subtract,
            (port_a, product),
            results.difference,
            options,
        )?;
        // Every connector is straight but two. The product runs up out of the multiplication's north edge into the
        // addition, and `A` runs across into the subtraction's "L" input, each as an elbow.
        for operator in [product, sum, difference] {
            self.draw_inputs_straight(operator)?;
        }
        let elbow = ConnectorType::Elbow { corner_radius: 0.0 };
        self.set_connector_type(self.input_edges(sum)?.1, elbow)?;
        self.set_connector_type(self.input_edges(difference)?.0, elbow)?;
        let butterfly = Butterfly {
            frame,
            port_a,
            port_b,
            port_w,
            product,
            sum,
            difference,
        };
        // Where each member sits in the frame, measured once, which is what every later move puts it back to.
        let frame_origin = self.node_rect(frame)?.origin;
        let mut offsets = [Point::origin(); 6];
        for (offset, id) in offsets.iter_mut().zip(butterfly.members()) {
            let at = self.node_rect(id)?.origin;
            *offset = Point::new(at.x - frame_origin.x, at.y - frame_origin.y);
        }
        let observer = self.carry_members_with_frame(butterfly, offsets)?;
        self.inner.borrow_mut().butterflies.push(ButterflyUnit {
            frame,
            offsets,
            _observer: observer,
        });
        rollback.disarm();
        Ok(butterfly)
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    /// The two input connectors of operator node `operator`, in the order of its operands.
    fn input_edges(&self, operator: NodeId) -> Result<(EdgeId, EdgeId), Error> {
        self.inner
            .borrow()
            .node_handle(operator)
            .and_then(|handles| handles.binary_operator_input_edges)
            .ok_or(Error::UnknownNode(operator))
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    /// Redraws both input connectors of operator node `operator` as straight lines.
    fn draw_inputs_straight(&self, operator: NodeId) -> Result<(), Error> {
        let edges = self.input_edges(operator)?;
        self.set_connector_type(edges.0, ConnectorType::Straight)?;
        self.set_connector_type(edges.1, ConnectorType::Straight)
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    /// Draws the frame's box [`PORT_PROTRUSION`] in from the west edge of its rectangle, and moves its title to the box's top
    /// left corner, clear of the ports and of the connectors that cross the frame's middle.
    ///
    /// The frame is made as a plain node, whose box fills its rectangle. The rectangle is the width that keeps the ports
    /// inside it, so the box is narrowed to leave them room, by moving its west edge east.
    fn shape_frame(&self, frame: NodeId, width: f64) -> Result<(), Error> {
        let inner = self.inner.borrow();
        let handles = inner.node_handle(frame).ok_or(Error::UnknownNode(frame))?;
        handles.outer_rect.set_attr("x", &PORT_PROTRUSION.to_string())?;
        handles.outer_rect.set_attr("width", &(width - PORT_PROTRUSION).to_string())?;
        if let Some(title) = handles.group.query_selector("text")? {
            title.set_text_anchor(TextAnchor::Start)?;
            title.set_dominant_baseline(DominantBaseline::Hanging)?;
            title.set_attr("x", &(PORT_PROTRUSION + PORT_RADIUS + FRAME_PADDING / 2.0).to_string())?;
            title.set_attr("y", &(FRAME_PADDING / 2.0).to_string())?;
        }
        Ok(())
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    /// Moves a butterfly so that its frame's top left corner is at `top_left`, and its three ports and three operator nodes
    /// go with it, each to the same place in the frame as before. Every connector that reaches any of them is drawn again.
    ///
    /// # Errors
    ///
    /// The errors of [`move_node`](Self::move_node), and [`Error::UnknownNode`] if `butterfly` was not made by this scene.
    ///
    /// The frame moves first, and if it cannot, nothing else moves. Otherwise every other node is moved, whether or not an
    /// earlier one failed, and the first error is returned. Each node goes to a place worked out from where the frame
    /// now is, not by a distance, so moving again to the same `top_left` puts any node that was left behind right, and
    /// leaves every other node exactly where it is.
    pub fn move_butterfly(&self, butterfly: &Butterfly, top_left: Point) -> Result<(), Error> {
        let offsets = self.butterfly_offsets(butterfly.frame)?;
        self.move_node(butterfly.frame, top_left)?;
        self.place_members(butterfly, top_left, &offsets)
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    /// The offsets of each port and operator node of the butterfly whose frame is `frame`, from the frame's top left
    /// corner. They were measured when the butterfly was made, and never change.
    fn butterfly_offsets(&self, frame: NodeId) -> Result<[Point; 6], Error> {
        self.inner
            .borrow()
            .butterflies
            .iter()
            .find(|unit| unit.frame == frame)
            .map(|unit| unit.offsets)
            .ok_or(Error::UnknownNode(frame))
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    /// Moves each port and operator node of `butterfly` to where `frame_origin` and its own offset in the frame put it.
    ///
    /// Each move is to an absolute place, so it can be made again: a node that is already there is not moved further,
    /// and one that was left behind catches up. Every node is tried even if an earlier one fails, so one that cannot
    /// move does not strand the rest. The first error is the one returned.
    fn place_members(&self, butterfly: &Butterfly, frame_origin: Point, offsets: &[Point; 6]) -> Result<(), Error> {
        let mut first_error = None;
        for (id, target) in butterfly.members().into_iter().zip(member_targets(frame_origin, offsets)) {
            if let Err(error) = self.move_node(id, target) {
                first_error.get_or_insert(error);
            }
        }
        first_error.map_or(Ok(()), Err)
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    /// Makes the frame of `butterfly` draggable, and keeps its ports and operator nodes in place on it.
    ///
    /// Each report of the drag, whether the start, a move, the end or a cancel, carries the frame's origin. Every one
    /// puts each member where that origin and the member's own offset say, so no report depends on one before it. A
    /// member that could not be moved is tried again by the next report, which is idempotent for the rest. The drag
    /// cannot report an error, so a failure is not returned.
    fn carry_members_with_frame(
        &self,
        butterfly: Butterfly,
        offsets: [Point; 6],
    ) -> Result<crate::scene::DragObserverHandle, Error> {
        let scene = Rc::downgrade(&self.inner);
        // The frame lies over its own operator nodes, so a drop that pushed it clear of every node it overlaps would
        // always push it away from them. Nothing it is dropped on is checked.
        let options = DragOptions::default().with_collision(CollisionPolicy::Allow);
        self.make_draggable_with_observer(butterfly.frame, options, move |_, phase| {
            let origin = match phase {
                DragPhase::Start { origin }
                | DragPhase::Applied { origin }
                | DragPhase::End { origin }
                | DragPhase::Cancel { origin } => origin,
            };
            let Some(inner) = scene.upgrade() else { return };
            let _ = (Scene { inner }).place_members(&butterfly, origin, &offsets);
        })
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Where each member of a butterfly goes when its frame's top left corner is at `frame_origin`: the corner plus the
/// member's own offset, in the order of [`Butterfly::members`].
fn member_targets(frame_origin: Point, offsets: &[Point; 6]) -> [Point; 6] {
    offsets.map(|offset| Point::new(frame_origin.x + offset.x, frame_origin.y + offset.y))
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// A butterfly that this scene keeps: where each member sits in its frame, and the drag observer that keeps them there.
///
/// The scene owns it, so the observer stops with the scene.
pub(in crate::scene) struct ButterflyUnit {
    frame: NodeId,
    /// The offsets of the members from the frame's top left corner, in the order of [`Butterfly::members`].
    offsets: [Point; 6],
    _observer: crate::scene::DragObserverHandle,
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[cfg(test)]
mod unit_tests;
