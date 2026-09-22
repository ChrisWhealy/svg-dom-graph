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
    CELL_HEIGHT, CELL_PADDING, EdgeAnchors, GRID_FONT_FAMILY, GRID_FONT_SIZE, LABEL_FONT_SIZE, NodeOptions,
    OUTER_PADDING, construction_guard::OperatorConstructionGuard, render_guard::RenderGuard, validate_edge_anchors,
};
use crate::{
    error::Error,
    model::{
        graph::Graph,
        node::{NodeContent, NodeId},
    },
    scene::{ArithmeticOperator, BinaryOperator, BoxHandles, DataNodeContent, Scene, Selection, UnaryOperator},
};
use svg_dom::{
    DominantBaseline, SvgRoot, TextAnchor,
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

/// The height of an operator node's own label row, in user-space units — the same fixed-multiple-of-font-size
/// approach [`CELL_HEIGHT`] already uses, at [`LABEL_FONT_SIZE`] rather than [`GRID_FONT_SIZE`].
const OPERATOR_LABEL_ROW_HEIGHT: f64 = LABEL_FONT_SIZE * 1.4 + 2.0 * CELL_PADDING;

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

    // The value cell's own width, plus `OUTER_PADDING` kept clear on either side of it, competes with the label's
    // own width, plus `CELL_PADDING`, for the box's own final width — whichever of the two needs more room wins.
    // Either way the value cell itself never reaches the box's own left/right edges.
    let value_cell_size = Size::new(value_width + 2.0 * CELL_PADDING, CELL_HEIGHT + 2.0 * CELL_PADDING);
    let box_width = (label_width + 2.0 * CELL_PADDING).max(value_cell_size.width + 2.0 * OUTER_PADDING);
    // `OUTER_PADDING` again below the value cell, so it never reaches the box's own bottom edge either. Above it,
    // the label row's own height already keeps it clear of the box's own top edge.
    let size = Size::new(box_width, OPERATOR_LABEL_ROW_HEIGHT + value_cell_size.height + OUTER_PADDING);
    let rect = Rect { origin: top_left, size };

    let outer_el = svg.rect(origin, size)?;
    guard.track(outer_el.clone());
    outer_el.set_fill("#eef4ff")?;
    outer_el.set_stroke("#2a5db0")?;
    outer_el.set_stroke_width(1.5)?;
    group.append(&outer_el)?;

    let value_cell_origin = Point::new((box_width - value_cell_size.width) / 2.0, OPERATOR_LABEL_ROW_HEIGHT);
    let value_cell_el = svg.rect(value_cell_origin, value_cell_size)?;
    guard.track(value_cell_el.clone());
    value_cell_el.set_fill(type_color)?;
    value_cell_el.set_stroke("#2a5db0")?;
    value_cell_el.set_stroke_width(1.0)?;
    group.append(&value_cell_el)?;

    label_el.set_attr_display(scratch, "x", box_width / 2.0)?;
    label_el.set_attr_display(scratch, "y", OPERATOR_LABEL_ROW_HEIGHT / 2.0)?;
    group.append(&label_el)?;

    // Horizontally centred the same as the value cell itself — `box_width / 2.0` either way, since the cell is
    // itself centred in the box.
    value_el.set_attr_display(scratch, "x", box_width / 2.0)?;
    value_el.set_attr_display(scratch, "y", value_cell_origin.y + value_cell_size.height / 2.0)?;
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
            cell_rects: vec![value_cell_el],
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
        self.add_two_input_operator_node_with(top_left, operator.label(), inputs, result, options)
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    /// Adds an arithmetic operator node to the graph — labelled with `operator`, showing `result`'s own single
    /// value, and wired with incoming edges from `inputs.0` and `inputs.1` — and returns its id.
    ///
    /// `svg-dom-graph` never evaluates `operator` itself. `result` must already be `operator` applied to `inputs`'
    /// own two values, computed by the caller. See this module's own doc comment ("Operator nodes").
    ///
    /// Unlike [`add_binary_operator_node`](Self::add_binary_operator_node)'s own [`BinaryOperator`], every
    /// [`ArithmeticOperator`] variant is non-commutative — see its own doc comment. `inputs.0` is always the
    /// left-hand operand, `inputs.1` the right-hand one.
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
        self.add_two_input_operator_node_with(top_left, operator.label(), inputs, result, options)
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
