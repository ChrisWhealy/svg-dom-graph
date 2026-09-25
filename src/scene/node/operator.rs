//! Operator nodes — a data node's own single value, labelled with the operation that produced it. See
//! `Scene::add_unary_operator_node`/`Scene::add_binary_operator_node`/`Scene::add_arithmetic_operator_node` for why
//! this crate never computes that value itself. See [`super::plain`] for a node with a plain text label instead,
//! and [`super::data`] for one whose content is a [`DataNodeContent`] grid with no operator label at all.
//!
//! An incoming connector anchors somewhere on the node's own outer perimeter — never on some inner sub-region —
//! the same as for any other node. So the value cell [`draw_operator_box`] draws is inset from every outer edge by
//! [`OUTER_PADDING`], never flush against it. Otherwise a connector anchored low on the box would look like it
//! terminates at the *result*, when what it actually feeds is the *operation* the whole node represents.

use super::{
    CELL_HEIGHT, CELL_PADDING, EdgeAnchors, GRID_FONT_FAMILY, GRID_FONT_SIZE, LABEL_FONT_SIZE, LABEL_ROW_HEIGHT,
    NodeOptions, OUTER_PADDING, construction_guard::OperatorConstructionGuard, render_guard::RenderGuard,
    validate_edge_anchors,
};
use crate::{
    colours::{BOX_STROKE, CONNECTOR_STROKE, PLAIN_BOX_FILL, TEXT_FILL},
    error::Error,
    geometry::{binary_operator_anchors, centre, port_marker_position, side::Side},
    model::{
        graph::Graph,
        node::{NodeContent, NodeId},
    },
    scene::{ArithmeticOperator, BinaryOperator, BoxHandles, DataNodeContent, Scene, Selection, UnaryOperator},
};
use svg_dom::{
    DominantBaseline, SvgNode, SvgRoot, TextAnchor,
    root::utils::{Point, Rect, Size},
};

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
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

/// A non-commutative operator's own "L"/"R" port marker font size, in user-space units — smaller than
/// [`LABEL_FONT_SIZE`], so the marker reads as a secondary annotation rather than competing with the node's own
/// label, but large enough to stay legible once offset clear of the connector's own arrowhead.
const PORT_MARKER_FONT_SIZE: f64 = LABEL_FONT_SIZE * 0.9;

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Draws an operator node: a label row naming the operation, with `result`'s own single value in its own inset
/// cell beneath it, grouped under one `<g>`, and returns their handles alongside the box's own final `Rect`.
///
/// The value cell is sized exactly like a single-value [`DataNodeContent`]'s own cell —
/// [`draw_content_box`](super::data::draw_content_box)'s own single-value path — but, unlike there, never grows to
/// fill the node's own outer box. It stays inset by [`OUTER_PADDING`] instead, for the reason this module's own
/// doc comment gives. `label` renders in the plain style [`draw_box`](super::plain::draw_box) already uses for an
/// ordinary node's own text, measured the same "read the real rendered width back" way every cell in this file
/// already is.
///
/// A [`RenderGuard`] covers this function's own DOM construction, for the same reason as
/// [`draw_content_box`](super::data::draw_content_box).
///
/// `scratch` is a caller-owned buffer — `SceneInner::scratch`, in every real caller — reused for this call's own
/// `x`/`y`/`transform` formatting, the same reasoning [`draw_box`](super::plain::draw_box)'s own `scratch`
/// parameter follows. It also holds `result`'s own formatted value text for the brief window between
/// [`DataNodeContent::single_cell_string_into`] writing it and [`SvgRoot::text`] copying it into a new `<text>`
/// element — never a separate, one-off `String` allocated just for that.
fn draw_operator_box(
    svg: &SvgRoot,
    scratch: &mut String,
    top_left: Point,
    label: &str,
    result: &DataNodeContent,
    edge_anchors: Option<EdgeAnchors>,
) -> Result<(BoxHandles, Rect), Error> {
    let group = svg.group()?;
    // Always exactly 4: the label, the value text, and the outer/value-cell rects.
    let mut guard = RenderGuard::new(group.clone());
    let type_colour = result.type_colour();
    let origin = Point::origin();

    let label_el = svg.text(origin, label)?;
    guard.track(label_el.clone());
    label_el.set_text_anchor(TextAnchor::Middle)?;
    label_el.set_dominant_baseline(DominantBaseline::Middle)?;
    label_el.set_font_size(LABEL_FONT_SIZE)?;
    label_el.set_fill(TEXT_FILL)?;
    let label_width = label_el.bounding_box()?.size.width;

    if !result.single_cell_string_into(scratch) {
        return Err(Error::Svg(svg_dom::Error::Dom(
            "draw_operator_box: expected exactly one value".into(),
        )));
    }
    // Cloned out now, before `scratch` is reused below for `x`/`y`/`transform` formatting — this is the same
    // formatted text `value_el` shows, reused again for the node's own `aria-label`.
    let value_text = scratch.clone();
    let value_el = svg.text(origin, scratch.as_str())?;
    guard.track(value_el.clone());
    value_el.set_text_anchor(TextAnchor::Middle)?;
    value_el.set_dominant_baseline(DominantBaseline::Middle)?;
    value_el.set_font_family(GRID_FONT_FAMILY)?;
    value_el.set_font_size(GRID_FONT_SIZE)?;
    value_el.set_fill(TEXT_FILL)?;
    let value_width = value_el.bounding_box()?.size.width;

    // The value cell's own width, plus `OUTER_PADDING` kept clear on either side of it, competes with the label's
    // own width, plus `CELL_PADDING`, for the box's own final width — whichever of the two needs more room wins.
    // Either way the value cell itself never reaches the box's own left/right edges.
    let value_cell_size = Size::new(value_width + 2.0 * CELL_PADDING, CELL_HEIGHT + 2.0 * CELL_PADDING);
    let box_width = (label_width + 2.0 * CELL_PADDING).max(value_cell_size.width + 2.0 * OUTER_PADDING);
    // `OUTER_PADDING` again below the value cell, so it never reaches the box's own bottom edge either. Above it,
    // the label row's own height already keeps it clear of the box's own top edge.
    let size = Size::new(box_width, LABEL_ROW_HEIGHT + value_cell_size.height + OUTER_PADDING);
    let rect = Rect { origin: top_left, size };

    let outer_el = svg.rect(origin, size)?;
    guard.track(outer_el.clone());
    outer_el.set_fill(PLAIN_BOX_FILL)?;
    outer_el.set_stroke(BOX_STROKE)?;
    outer_el.set_stroke_width(1.5)?;
    group.append(&outer_el)?;

    let value_cell_origin = Point::new((box_width - value_cell_size.width) / 2.0, LABEL_ROW_HEIGHT);
    let value_cell_el = svg.rect(value_cell_origin, value_cell_size)?;
    guard.track(value_cell_el.clone());
    value_cell_el.set_fill(type_colour)?;
    value_cell_el.set_stroke(BOX_STROKE)?;
    value_cell_el.set_stroke_width(1.0)?;
    group.append(&value_cell_el)?;

    label_el.set_attr_display(scratch, "x", box_width / 2.0)?;
    label_el.set_attr_display(scratch, "y", LABEL_ROW_HEIGHT / 2.0)?;
    group.append(&label_el)?;

    // Horizontally centred the same as the value cell itself — `box_width / 2.0` either way, since the cell is
    // itself centred in the box.
    value_el.set_attr_display(scratch, "x", box_width / 2.0)?;
    value_el.set_attr_display(scratch, "y", value_cell_origin.y + value_cell_size.height / 2.0)?;
    group.append(&value_el)?;

    // See `draw_box`'s own comment on its matching call for why `set_transform_fmt`, not `set_translate`.
    group.set_transform_fmt(scratch, format_args!("translate({}, {})", top_left.x, top_left.y))?;

    // Same reasoning as `draw_content_box`'s own `<title>`/`aria-label` pair: colour alone conveys the result's own
    // type to neither assistive technology nor a colour-blind reader. The label names the operator and shows its
    // own real result value — `value_text`, the same formatted text `value_el` itself renders — so the result is
    // available as text even without visiting the value cell directly.
    group.set_attr("role", "group")?;
    let node_label = format!("{label} result = {value_text}");
    group.set_attr("aria-label", &node_label)?;
    // Set to `node_label` — the same text `aria-label` carries — so the browser's own mouse-hover tooltip reads
    // exactly what a screen reader announces, not just the result's own type. `Scene::set_selection` keeps the
    // two in sync afterward too, since an operator's own result is itself a `DataNodeContent` node like any
    // other — see that method's own doc comment.
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
            cell_rects: vec![value_cell_el],
            cell_stroke_width: "1",
            selection: Selection::None,
            aria_label: node_label,
            base_label_len,
            ref_name: label.to_owned(),
        },
        rect,
    ))
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Draws one non-commutative two-input operator's own port marker: a small `glyph` ("L" or "R") placed at
/// [`port_marker_position`]'s own point for `anchor`/`side`.
///
/// A `SUB`/`DIV`/`MOD` node's own anti-crossing routing can freely reassign which operand's connector lands on
/// which side, to avoid the two connectors crossing — see this module's own parent doc comment. That is harmless
/// for a commutative operator, but not for one of these, where swapping the operands changes the result. This
/// marker gives the rendered diagram a stable, operand-identity-tied indicator that survives any such
/// reassignment, so a reader can always tell `inputs.0` from `inputs.1` from the diagram alone.
///
/// `aria_label` ("left operand"/"right operand") replaces `glyph` as this marker's own accessible name — `role`
/// `"img"` tells assistive technology to announce that name instead of reading the single-letter glyph literally.
///
/// Created as a direct child of the SVG root, the same absolute-coordinate way a connector's own `<path>` is — see
/// [`crate::scene::connector`]. [`crate::scene::SceneInner::redraw_binary_operator_inputs`] repositions this
/// marker, rather than recreating it, on every later redraw.
fn draw_port_marker(svg: &SvgRoot, anchor: Point, side: Side, glyph: &str, aria_label: &str) -> Result<SvgNode, Error> {
    let marker = svg.text(port_marker_position(anchor, side), glyph)?;
    marker.set_text_anchor(TextAnchor::Middle)?;
    marker.set_dominant_baseline(DominantBaseline::Middle)?;
    marker.set_font_size(PORT_MARKER_FONT_SIZE)?;
    marker.set_fill(CONNECTOR_STROKE)?;
    marker.set_attr("role", "img")?;
    marker.set_attr("aria-label", aria_label)?;
    Ok(marker)
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
impl Scene {
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
            inner.attach(&handles.group)?;
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
        self.add_two_input_operator_node_with(top_left, operator.label(), inputs, result, options, operator.commutes())
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    /// Adds an arithmetic operator node to the graph — labelled with `operator`, showing `result`'s own single
    /// value, and wired with incoming edges from `inputs.0` and `inputs.1` — and returns its id.
    ///
    /// `svg-dom-graph` never evaluates `operator` itself. `result` must already be `operator` applied to `inputs`'
    /// own two values, computed by the caller. See this module's own doc comment ("Operator nodes").
    ///
    /// Unlike [`add_binary_operator_node`](Self::add_binary_operator_node)'s own [`BinaryOperator`], the
    /// [`ArithmeticOperator`] variant is only commutative for `add` and `multiply` — see its own doc comment.
    /// `inputs.0` is always the left-hand operand and `inputs.1` the right-hand one.
    ///
    /// Equivalent to [`add_arithmetic_operator_node_with`](Self::add_arithmetic_operator_node_with) with
    /// [`NodeOptions::default`].
    ///
    /// # Errors
    ///
    /// See [`add_arithmetic_operator_node_with`](Self::add_arithmetic_operator_node_with)'s own `# Errors` section.
    pub fn add_arithmetic_operator_node(
        &self,
        top_left: Point,
        operator: ArithmeticOperator,
        inputs: (NodeId, NodeId),
        result: DataNodeContent,
    ) -> Result<NodeId, Error> {
        self.add_arithmetic_operator_node_with(top_left, operator, inputs, result, NodeOptions::default())
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    /// Adds an arithmetic operator node, as [`add_arithmetic_operator_node`](Self::add_arithmetic_operator_node),
    /// but with `options` controlling how many connector fixing points this node's sides offer — see
    /// [`EdgeAnchors`].
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
    /// Returns [`Error::DuplicateOperands`] if `inputs.0` and `inputs.1` name the same node — an arithmetic
    /// operator's two operands must be distinct.
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
    pub fn add_arithmetic_operator_node_with(
        &self,
        top_left: Point,
        operator: ArithmeticOperator,
        inputs: (NodeId, NodeId),
        result: DataNodeContent,
        options: NodeOptions,
    ) -> Result<NodeId, Error> {
        self.add_two_input_operator_node_with(top_left, operator.label(), inputs, result, options, operator.commutes())
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    /// Shared implementation behind [`add_binary_operator_node_with`](Self::add_binary_operator_node_with) and
    /// [`add_arithmetic_operator_node_with`](Self::add_arithmetic_operator_node_with) — every check, and the
    /// rendered box itself, is identical between a bitwise and an arithmetic operator node. Only the rendered
    /// `label` text differs, already resolved by the caller from its own operator enum.
    ///
    /// See either public wrapper's own doc comment for the full contract this enforces.
    fn add_two_input_operator_node_with(
        &self,
        top_left: Point,
        label: &str,
        inputs: (NodeId, NodeId),
        result: DataNodeContent,
        options: NodeOptions,
        commutes: bool,
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

        let (id, rect) = {
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

            // See `add_node_with`'s own matching comment for why `scratch` is taken out for the call.
            let mut scratch = std::mem::take(&mut inner.scratch);
            let draw_result =
                draw_operator_box(&inner.svg, &mut scratch, top_left, label, &result, options.edge_anchors);
            inner.scratch = scratch;
            let (mut handles, rect) = draw_result?;
            inner.attach(&handles.group)?;
            handles.binary_operator_inputs = Some(inputs);
            let id = inner.graph.add_node(rect, result);
            inner.insert_node_handle(id, handles);
            (id, rect)
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

        // Non-commutative operators only — `SUB`/`DIV`/`MOD` — where `inputs.0`/`inputs.1` order changes the
        // result. See `draw_port_marker`'s own doc comment for why this stable indicator is needed and what it
        // shows. Skipped for a commutative operator, where operand order carries no meaning to mark.
        if !commutes {
            let mut inner = self.inner.borrow_mut();
            let fixing_points = options.edge_anchors.map(|EdgeAnchors(n)| n);
            let a_centre = centre(inner.node_rect(inputs.0)?);
            let b_centre = centre(inner.node_rect(inputs.1)?);
            let [(anchor_a, side_a), (anchor_b, side_b)] =
                binary_operator_anchors(rect, a_centre, b_centre, fixing_points);

            let marker_a = draw_port_marker(&inner.svg, anchor_a, side_a, "L", "left operand")?;
            inner.attach(&marker_a)?;
            inner.edge_handle_mut(edge_a).ok_or(Error::UnknownEdge(edge_a))?.port_marker = Some(marker_a);

            let marker_b = draw_port_marker(&inner.svg, anchor_b, side_b, "R", "right operand")?;
            inner.attach(&marker_b)?;
            inner.edge_handle_mut(edge_b).ok_or(Error::UnknownEdge(edge_b))?.port_marker = Some(marker_b);
        }

        guard.disarm();
        Ok(id)
    }
}
