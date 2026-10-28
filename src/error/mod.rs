//! This crate's own error type.
//!
//! Wraps [`svg_dom::Error`] for anything that comes from the underlying DOM library, and adds variants for graph-domain
//! problems detected by this crate. For example, a [`NodeId`]/[`EdgeId`] used with a `Scene` that did not create it.

use crate::geometry::ellipse::ClockPosition;
use crate::model::{edge::EdgeId, node::NodeId};
use std::fmt;
use svg_dom::root::utils::{Point, Rect};

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Which end of an edge a problem is about.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum EdgeEnd {
    /// The node the edge leaves.
    From,
    /// The node the edge arrives at.
    To,
}

impl fmt::Display for EdgeEnd {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            EdgeEnd::From => "from",
            EdgeEnd::To => "to",
        })
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// What is wrong with the options given for one end of an edge that touches an elliptical node.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum EndpointProblem {
    /// A clock position was given for a node that is a rectangle. Clock positions are for ellipses only.
    ClockOnRectangle,
    /// A position along a side, as a fraction, was given for an ellipse. An ellipse has no sides to measure along.
    FractionOnEllipse,
    /// A clock position and a forced side were both given for the same end. They name the same thing in two ways, so
    /// neither can be left to win.
    ClockWithSide,
}

impl fmt::Display for EndpointProblem {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            EndpointProblem::ClockOnRectangle => {
                "a clock position is only for an ellipse, and this node is a rectangle"
            },
            EndpointProblem::FractionOnEllipse => "a position along a side is not for an ellipse, which has no sides",
            EndpointProblem::ClockWithSide => "a clock position and a forced side cannot both be given for one end",
        })
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// An error from a `svg-dom-graph` operation.
#[derive(Debug)]
#[non_exhaustive]
pub enum Error {
    /// An error from the underlying `svg-dom` library: DOM creation, an attribute write, and so on.
    Svg(svg_dom::Error),
    /// The `NodeId` cannot be found in the `Scene`. All `NodeId`'s are `Scene`-specific: a `NodeId` from scene `a`
    /// cannot be used in scene `b`. So this error can occur if a `NodeId` is accidentally passed to some other scene.
    UnknownNode(NodeId),
    /// An `EdgeId` does not name an edge in the `Scene` it was used with.
    ///
    /// See [`Error::UnknownNode`] for why this can happen.
    UnknownEdge(EdgeId),
    /// `Scene::add_edge` was asked to connect a node to itself.
    ///
    /// Having both endpoints connect to the same node is not supported yet.
    ///
    /// Rejecting the call now keeps room to add real loop-edge routing later, as an additive relaxation of this same
    /// method.
    ///
    /// Silently returning an `EdgeId` for a connector nobody can see would be worse.
    SelfLoopUnsupported(NodeId),
    /// `Scene::make_draggable`/`Scene::make_draggable_with` was called more than once for the same node.
    ///
    /// `svg-dom`'s listener registration is append-only. A second call would not replace the first. It would add a
    /// second, independent set of pointer listeners and drag-state alongside it, both responding to the same events.
    /// Rejecting the second call keeps that from happening silently.
    AlreadyDraggable(NodeId),
    /// `Scene::make_draggable_with` was given a `CollisionPolicy::PushClear` padding that is not a finite value `>=
    /// 0.0`.
    ///
    /// A negative padding pulls the corrected position back inside the clearance boundary instead of extending it. A
    /// non-finite value (`NaN`, `+inf`, `-inf`) propagates straight through `nearest_clear_centre` into the resulting
    /// coordinates. Rejected before any other state changes, so the scene's existing nodes are left exactly as they
    /// were.
    InvalidCollisionPadding(f64),
    /// `Scene::make_draggable_with` was given an invalid `DragOptions::bounds` value. The origin and size must be
    /// constructed from finite, non-negative values.
    ///
    /// A non-finite coordinate propagates straight through `clamp_to_bounds` into the clamped origin it returns on
    /// every drag move. A negative width or height has no meaning as a bounding rectangle.
    ///
    /// Zero width or height is allowed. `clamp_to_bounds` already gives that case a deterministic result: the dragged
    /// node pins to `bounds`'s own near edge on that axis.
    InvalidDragBounds(Rect),
    /// `Scene::add_edge_with` or `Scene::set_connector_type` was given a `ConnectorType::Elbow` corner radius that is
    /// not a finite value `>= 0.0`.
    ///
    /// A negative radius has no meaning for a rounded corner, and a non-finite value (`NaN`, `+inf`, `-inf`) propagates
    /// straight through `elbow_path_into` into the resulting path data. Rejected before any other state changes, so the
    /// scene's existing nodes and edges are left exactly as they were.
    InvalidCornerRadius(f64),
    /// `Scene::add_edge_with` was given a `ConnectorOptions::from_position`/`to_position` that is not a finite value in
    /// `0.0..=1.0`.
    ///
    /// The position is a fraction of the way along a node's side, so a value outside that range would pin the
    /// connector off the node. Rejected before any other state changes, so the scene is left exactly as it was.
    InvalidConnectorPosition(f64),
    /// `Scene::add_node_with` or `Scene::set_edge_anchors` was given `Some(EdgeAnchors(0))`.
    ///
    /// Zero fixing points has no meaning: a side with no candidate point cannot anchor a connector. Use `None` instead,
    /// to keep each connector's own default anchor rule.
    InvalidEdgeAnchors(u8),
    /// `Scene::add_node` or `Scene::add_node_with` was given an origin or size that is not valid rectangle geometry.
    ///
    /// Every field of `rect` must be finite: SVG defines a negative `<rect>` `width`/`height` as illegal. An infinite
    /// coordinate or dimension would otherwise sit in the graph's model and contaminate every later geometry
    /// calculation in which it takes part. `box_centre`, `boundary_point`, overlap detection, connector routing, and
    /// collision resolution all use it. `width` and `height` must also both be strictly positive. A zero-sized node has
    /// no visible box. It also gives connector routing no direction to point at it in (`boundary_point` needs a
    /// well-defined interior to aim a ray at).
    ///
    /// Rejected before drawing anything or touching the graph's model, so a rejected call leaves the scene exactly as
    /// it was.
    InvalidNodeGeometry(Rect),
    /// `Scene::add_data_node`/`Scene::add_data_node_with` was given a [`crate::scene::DataNodeContent`] with no values.
    ///
    /// If there is no data to draw inside a grid, then no sensible box size can be computed. This condition is rejected
    /// before drawing anything or touching the graph's model, so a rejected call leaves the scene unchanged.
    EmptyNodeContent,
    /// `DataFormat::PlainText` content that is not `u8`, or holds a byte outside the printable ASCII range
    /// `0x20..=0x7E`. Rejected before drawing anything or touching the graph's model, so a rejected call leaves the
    /// scene unchanged.
    InvalidPlainText,
    /// `DataFormat::HexDump` content that is not `u8`. Rejected before drawing anything or touching the graph's model,
    /// so a rejected call leaves the scene unchanged.
    InvalidHexDumpFormat,
    /// [`BitValues::new`](crate::scene::BitValues::new) was given a `width` that is `0` or more than
    /// [`MAX_BIT_WIDTH`](crate::scene::MAX_BIT_WIDTH), with `index` `None`, or a value that does not fit in `width`
    /// bits, with `index` that of the first one.
    InvalidBitValues { width: u8, index: Option<usize> },
    /// `DataFormat::Float` content that is not `f32` or `f64`, or whose [`crate::scene::FloatFormat`] asks for more than
    /// [`crate::scene::MAX_FLOAT_DIGITS`] digits, or for no significant digits at all. Rejected before drawing anything
    /// or touching the graph's model, so a rejected call leaves the scene unchanged.
    InvalidFloatFormat,
    /// `DataFormat::Uuid` content that is not `Uuid` values. Rejected before drawing anything or touching the graph's
    /// model, so a rejected call leaves the scene unchanged.
    InvalidUuidFormat,
    /// `DataFormat::Bool` content that is not `Bool` values. Rejected before drawing anything or touching the graph's
    /// model, so a rejected call leaves the scene unchanged.
    InvalidBoolFormat,
    /// `DataFormat::ComplexNumber` content that is not `Complex32` or `Complex64` values, or whose
    /// [`crate::scene::ComplexStyle`] asks for more than [`crate::scene::MAX_FLOAT_DIGITS`] digits, or for no significant
    /// digits at all, or the [`Matrix`](crate::scene::ComplexNumberFormat::Matrix) form as the result of an operator node,
    /// whose cell is one line high. Rejected before drawing anything or touching the graph's model, so a rejected call
    /// leaves the scene unchanged.
    InvalidComplexFormat,
    /// `DataFormat::Text` content that is not `Text` values, `Text` values under another format, or a string that holds a
    /// line break or another control character. Rejected before drawing anything or touching the graph's model, so a
    /// rejected call leaves the scene unchanged.
    InvalidTextContent,
    /// `Scene::add_named_data_node`/`Scene::add_named_data_node_with` was given a `name` that is empty, or holds only
    /// whitespace.
    ///
    /// A later node's own description names this one by `name` — see `BoxHandles::ref_name`'s own doc comment. An empty
    /// or blank name would still draw, but its own accessible name would then read as `": u8 = 12"` rather than naming
    /// anything. Rejected before drawing anything or touching the graph's model, so a rejected call leaves the scene
    /// unchanged.
    EmptyNodeName,
    /// `Scene::add_data_node`/`Scene::add_data_node_with` was given a [`crate::scene::DataNodeContent`] whose
    /// [`crate::scene::GridLayout`] wraps `0` — `Columns(0)`, `Rows(0)`, or `MaxColumns(0)`.
    ///
    /// Zero columns (or rows) has no meaning: there is nowhere to place any value. Rejected before drawing anything or
    /// touching the graph's model, so a rejected call leaves the scene unchanged.
    InvalidGridLayout(crate::scene::GridLayout),
    /// `Scene::add_markdown_node`/`Scene::add_markdown_node_with` was given text with nothing to draw: it is empty, or
    /// holds only whitespace. Rejected before drawing anything or touching the graph's model, so a rejected call leaves
    /// the scene unchanged.
    EmptyMarkdown,
    /// `Scene::text_input_value` or `Scene::set_text_input_value` was given a node that is not a text input node.
    NotATextInput(NodeId),
    /// `Scene::add_plot_node` was given a [`crate::scene::PlotConfig`] that cannot be drawn, or
    /// `Scene::set_plot_description` was given an empty description. The reason is the
    /// [`crate::scene::PlotConfigError`]. Rejected before drawing anything or touching the graph's model, so a rejected
    /// call leaves the scene unchanged.
    InvalidPlotConfig(crate::scene::PlotConfigError),
    /// A plot method was given a node that is not a plot node.
    NotAPlot(NodeId),
    /// `Scene::on_plot_click` was called more than once for the same node.
    ///
    /// Same reasoning as [`Error::AlreadyDraggable`]: `svg-dom`'s listener registration is append-only, so a second
    /// call would add a second, independent click listener rather than replacing the first.
    AlreadyHasClickHandler(NodeId),
    /// An operator node's operand names a node whose content is a plain label, not a [`crate::scene::DataNodeContent`].
    ///
    /// Only typed data has a value an operator could plausibly act on. Rejected before drawing anything or touching the
    /// graph's model, so a rejected call leaves the scene unchanged.
    OperandNotData(NodeId),
    /// An operator node's own operand(s) and result did not all share one [`crate::scene::NodeValues`] type.
    ///
    /// Both of the operands to a two-input operator, and the supplied result must all share the same type. A signed
    /// integer is not the same type as its unsigned equivalent, and an `f32` is not an `f64` — see
    /// `Scene::add_binary_operator_node_with`'s / `Scene::add_arithmetic_operator_node_with`'s own doc comments.
    /// Rejected before drawing anything or touching the graph's model, so a rejected call leaves the scene unchanged.
    OperatorTypeMismatch { expected: &'static str, found: &'static str },
    /// An operator node's own `result` held other than exactly one value.
    ///
    /// An operator always produces one value, never a grid of them. Rejected before drawing anything or touching the
    /// graph's model, so a rejected call leaves the scene unchanged.
    OperatorResultNotSingleValue(usize),
    /// A unary or binary operator node was given floating point operands.
    ///
    /// `NOT`, `AND`, `XOR`, the shifts and the rotations act on the bits of an integer, and have no meaning for a float
    /// in ordinary arithmetic. An arithmetic operator node accepts floats. Rejected before drawing anything or touching
    /// the graph's model, so a rejected call leaves the scene unchanged.
    OperatorNeedsIntegers { operator: String, found: &'static str },
    /// An operator node of any kind was given UUIDs as operands.
    ///
    /// A UUID is an identifier, not a number, so no operator has a meaning for one. A 128-bit number is `U128` or
    /// `I128`. Rejected before drawing anything or touching the graph's model, so a rejected call leaves the scene
    /// unchanged.
    OperatorOnUuid { operator: String },
    /// `Scene::set_edge_label` was given a label that is empty, or has a line break or another control character in it.
    /// Rejected before touching the scene, so a rejected call leaves the edge as it was.
    InvalidEdgeLabel,
    /// An operator node of any kind was given text as operands.
    ///
    /// A string is not a number, so no operator has a meaning for one. Rejected before drawing anything or touching the
    /// graph's model, so a rejected call leaves the scene unchanged.
    OperatorOnText { operator: String },
    /// An operator node was given Booleans as operands, and it is not one that a Boolean has a meaning under.
    ///
    /// `NOT`, `AND`, `OR`, `XOR`, `NAND`, `NOR` and `XNOR` are accepted. A shift, a rotation, a bit or byte reversal
    /// and every arithmetic operator are not, since there is no sum of two Booleans and no `true` to rotate. Rejected
    /// before drawing anything or touching the graph's model, so a rejected call leaves the scene unchanged.
    OperatorOnBool { operator: String },
    /// An operator node was given complex numbers as operands, and it is not one that a complex number has a meaning
    /// under.
    ///
    /// Addition, subtraction, multiplication and division are accepted. Every unary operator, every binary (bitwise)
    /// operator and modulus are not, since a complex number has no bits to rotate and no remainder. Rejected before
    /// drawing anything or touching the graph's model, so a rejected call leaves the scene unchanged.
    OperatorOnComplex { operator: String },
    /// A two-input operator constructor — `Scene::add_binary_operator_node`/`_with`, or
    /// `Scene::add_arithmetic_operator_node`/`_with` — was given the same node as both of `inputs`.
    ///
    /// Both the operands of a two-input operator must be distinct nodes — combining a node with itself has no second
    /// "other side" to route a connector to. Rejected before drawing anything or touching the graph's model, so a
    /// rejected call leaves the scene unchanged.
    DuplicateOperands(NodeId),
    /// `Scene::add_butterfly_node` was given an operand or a result that does not hold complex numbers.
    ///
    /// Operands and results of a butterfly must all be single `Complex32` or all single `Complex64` values, because the
    /// butterfly is the building block of a Fourier transform. `NodeId` names the first operand that is not complex, and
    /// is `None` if it was a result. Rejected before drawing anything or touching the graph's model.
    ButterflyNotComplex(Option<NodeId>),
    /// `Scene::set_selection` was given a [`crate::scene::Selection`] that cannot be applied to node `id`.
    ///
    /// Either `id` names a plain label node, which has no cells to select. Or `selection` names a cell/row/column index
    /// out of range for `id`'s own actual value count or grid shape.
    ///
    /// Rejected before recolouring any cell, so a rejected call leaves every cell's own colour exactly as it was.
    InvalidSelection(NodeId, crate::scene::Selection),
    /// `Scene::set_data_values` was given values that cannot be shown in this node. The node may be a plain label,
    /// which has no values. The values may have a different type, such as `U16` for an `I16` node or `F64` for an
    /// `F32` one. They may also have a different number of values than the node was drawn with.
    ///
    /// Rejected before changing anything, so a rejected call leaves every cell exactly as it was.
    IncompatibleNodeValues(NodeId),
    /// `Scene::show_toolbar` was given a [`crate::scene::ToolbarOptions`] with a non-finite length, a `button_height`
    /// that is not `> 0.0`, or a negative `gap` or `margin`.
    ///
    /// Rejected before drawing anything or touching any toolbar already shown, so a rejected call leaves the scene
    /// unchanged.
    InvalidToolbarOptions(crate::scene::ToolbarOptions),
    /// `Scene::show_selection_toolbar` was given a [`crate::scene::SelectionToolbarOptions`] with a non-finite length,
    /// a `button_height` that is not `> 0.0`, or a negative `gap` or `margin`.
    ///
    /// Rejected before drawing anything or touching any selection toolbar already shown, so a rejected call leaves the
    /// scene unchanged.
    InvalidSelectionToolbarOptions(crate::scene::SelectionToolbarOptions),
    /// `Scene::show_scene_title` was given a [`crate::scene::SceneTitleOptions`] with a `font_size` that is not a
    /// finite value `> 0.0`, a `margin` that is not a finite value `>= 0.0`, or an `aria_level` of `0`.
    ///
    /// Rejected before drawing anything or touching any title already shown, so a rejected call leaves the scene
    /// unchanged.
    InvalidSceneTitleOptions(crate::scene::SceneTitleOptions),
    /// `Scene::enter` was called with a `NodeId` that names a `Label` or `Data` node, not a container node.
    NotAContainerNode(NodeId),
    /// `Scene::enter`or `Scene::exit` was called on a `Scene` that is not the one actually shown and receiving input.
    ///
    /// Checked before hiding or showing anything, so a rejected call leaves every `Scene` in the tree exactly as
    /// visible, and exactly as focused, as it was.
    NotFocused,
    /// `Scene::add_container_node` or `add_container_node_with` was given a `child` that is not currently the focused
    /// `Scene` of its own tree.
    ///
    /// A subtree may only be grafted into another scene tree by its own root, while that root is still focused — never
    /// by one of its own hidden descendants. Rejected before drawing anything or touching either scene's own model, so
    /// a rejected call leaves both scenes exactly as they were.
    ChildNotFocused,
    /// `Scene::add_container_node` or `add_container_node_with` was given a `child` `Scene` that already has a live
    /// parent — one whose own `Scene` has not yet been dropped.
    ///
    /// A nested `Scene` has exactly one owner at a time. A `child` whose previous parent has since been dropped is not
    /// rejected this way: it is simply detached, and may be attached again. Rejected before drawing anything or
    /// touching either scene's own model, so a rejected call leaves both scenes exactly as they were.
    AlreadyNested,
    /// `Scene::make_enterable` was called more than once for the same node.
    ///
    /// Same reasoning as [`Error::AlreadyDraggable`]: `svg-dom`'s listener registration is append-only, so a second
    /// call would add a second, independent click/keydown listener rather than replacing the first.
    AlreadyEnterable(NodeId),
    /// `Scene::add_container_node`/`add_container_node_with` was given a `child` that is `self`, or that is already an
    /// ancestor of `self` in the scene tree.
    ///
    /// Either would close a cycle through the strong `Rc` chain nested `Scene` ownership is built from. No node has
    /// been added yet when this is checked. So, unlike [`Error::AlreadyDraggable`]'s/[`Error::SelfLoopUnsupported`]'s
    /// own `NodeId`, there is no existing node to name here. It is `self` and `child`, both already in the caller's own
    /// hands, that are the wrong combination. No particular node in either one's graph is at fault. Rejected before
    /// drawing anything or touching either scene's own model, so a rejected call leaves both scenes exactly as they
    /// were.
    SelfNesting,
    /// `Scene::set_view` was given a [`crate::scene::ViewTransform`] whose `scale` is not within `0.25..=4.0`, or whose
    /// `tx`/`ty` is not finite.
    ///
    /// The same range `Scene::zoom_in`/`Scene::zoom_out` already clamp to, so a restored view can never put a `Scene`'s
    /// own zoom buttons at odds with what `Scene::zoom_scale` reports. Rejected before writing anything, so a rejected
    /// call leaves the current view exactly as it was.
    InvalidView(crate::scene::ViewTransform),
    /// `Scene::add_ellipse_node` or `Scene::add_ellipse_node_with` was given a centre or radii that cannot make an
    /// ellipse, or radii that the label makes too large to represent.
    ///
    /// The centre must be finite. Both radii must be finite and greater than zero. The radii that the label needs, and
    /// the rectangle that encloses them, must also be finite. Rejected before drawing anything or touching the graph, so
    /// a rejected call leaves the scene exactly as it was.
    InvalidEllipseGeometry {
        /// The centre that was given.
        centre: Point,
        /// The horizontal radius that was given.
        rx: f64,
        /// The vertical radius that was given.
        ry: f64,
    },
    /// `EllipseOptions::inner_margin` is not a finite value `>= 0.0`. Rejected before any SVG is created.
    InvalidEllipseMargin(f64),
    /// An ellipse or circle was given a label font size that is not finite and greater than zero. Rejected before
    /// anything is drawn.
    InvalidEllipseFontSize(f64),
    /// The options for one end of an edge cannot be used with the node at that end. See [`EndpointProblem`].
    ///
    /// Rejected before the edge is drawn or recorded, so a rejected call leaves the scene exactly as it was.
    InvalidEllipseEndpoint {
        /// Which end of the edge.
        endpoint: EdgeEnd,
        /// What is wrong with it.
        problem: EndpointProblem,
    },
    /// An elbow connector was asked to leave an ellipse at a clock position that is not 0, 15, 30 or 45. An elbow
    /// leaves by a cardinal point only. A straight connector can use any minute.
    ///
    /// Returned both when an edge is added and when `Scene::set_connector_type` would turn a straight edge that has such
    /// a clock position into an elbow. In both cases nothing is changed.
    InvalidEllipseElbowClock(ClockPosition),
    /// `Scene::set_edge_anchors` was called on an elliptical node. `EdgeAnchors(n)` means `n` fixing points on each
    /// side of a rectangle, and an ellipse has no sides. Nothing is changed.
    EdgeAnchorsUnsupportedForEllipse(NodeId),
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::Svg(err) => write!(f, "{err}"),
            Error::UnknownNode(id) => write!(f, "node {id:?} does not belong to this Scene"),
            Error::UnknownEdge(id) => write!(f, "edge {id:?} does not belong to this Scene"),
            Error::SelfLoopUnsupported(id) => write!(f, "self-loop on node {id:?} is not yet supported"),
            Error::InvalidEllipseGeometry { centre, rx, ry } => write!(
                f,
                "ellipse centre {centre:?} with radii {rx} and {ry} is invalid: the centre and the radii, as the label \
                 makes them, must be finite, and the radii must be > 0.0"
            ),
            Error::InvalidEllipseFontSize(size) => {
                write!(
                    f,
                    "an ellipse's label font size must be finite and greater than zero, got {size}"
                )
            },
            Error::InvalidEllipseMargin(margin) => {
                write!(f, "ellipse inner margin {margin} is not a finite value >= 0.0")
            },
            Error::InvalidEllipseEndpoint { endpoint, problem } => {
                write!(f, "invalid options for the {endpoint} end of the edge: {problem}")
            },
            Error::InvalidEllipseElbowClock(position) => write!(
                f,
                "an elbow connector can leave an ellipse only at clock position 0, 15, 30 or 45, not {}",
                position.minute()
            ),
            Error::EdgeAnchorsUnsupportedForEllipse(id) => {
                write!(f, "node {id:?} is an ellipse, which has no sides to put fixing points on")
            },
            Error::AlreadyDraggable(id) => write!(f, "node {id:?} is already draggable"),
            Error::AlreadyEnterable(id) => write!(f, "node {id:?} is already enterable"),
            Error::InvalidCollisionPadding(padding) => {
                write!(f, "collision padding {padding} is not a finite value >= 0.0")
            },
            Error::InvalidDragBounds(rect) => {
                write!(
                    f,
                    "drag bounds {rect:?} is invalid: origin and size must be finite, and width/height must be positive"
                )
            },
            Error::InvalidCornerRadius(radius) => {
                write!(f, "corner radius {radius} is not a finite value >= 0.0")
            },
            Error::InvalidConnectorPosition(position) => {
                write!(f, "connector position {position} is not a finite value in 0.0..=1.0")
            },
            Error::InvalidEdgeAnchors(n) => {
                write!(f, "edge anchor count {n} must be >= 1")
            },
            Error::InvalidNodeGeometry(rect) => {
                write!(
                    f,
                    "node geometry {rect:?} is invalid: origin and size must be finite, and width/height must both be > 0.0"
                )
            },
            Error::EmptyNodeContent => write!(f, "DataNodeContent must have at least one value to display"),
            Error::InvalidPlainText => {
                write!(
                    f,
                    "PlainText content must be u8 values in the printable ASCII range 0x20..=0x7E"
                )
            },
            Error::InvalidHexDumpFormat => write!(f, "a hex dump format needs u8 values"),
            Error::InvalidBitValues { width, index: None } => {
                write!(
                    f,
                    "bit values must be from 1 to {max} bits wide, not {width}",
                    max = crate::scene::MAX_BIT_WIDTH
                )
            },
            Error::InvalidBitValues { width, index: Some(index) } => {
                write!(f, "the value at index {index} does not fit into {width} bits")
            },
            Error::InvalidFloatFormat => {
                write!(
                    f,
                    "a float format needs f32 or f64 values, from 1 to {max} significant digits, or from 0 to {max} decimal places",
                    max = crate::scene::MAX_FLOAT_DIGITS
                )
            },
            Error::InvalidUuidFormat => write!(f, "a UUID format needs Uuid values"),
            Error::InvalidBoolFormat => write!(f, "a Boolean format needs Bool values"),
            Error::InvalidComplexFormat => {
                write!(
                    f,
                    "a complex number format needs Complex32 or Complex64 values, from 1 to {max} significant digits, or from 0 to {max} decimal places",
                    max = crate::scene::MAX_FLOAT_DIGITS
                )
            },
            Error::InvalidTextContent => {
                write!(
                    f,
                    "text values and the text format go together, and a text cell must have control characters such as CR or LF"
                )
            },
            Error::EmptyNodeName => write!(f, "name must not be empty or hold only whitespace"),
            Error::InvalidGridLayout(layout) => {
                write!(f, "grid layout {layout:?} must use a column/row count >= 1")
            },
            Error::EmptyMarkdown => write!(f, "markdown must hold at least one heading, paragraph or list item"),
            Error::NotATextInput(id) => write!(f, "node {id:?} is not a text input node"),
            Error::InvalidPlotConfig(reason) => write!(f, "this plot cannot be drawn: {reason}"),
            Error::NotAPlot(id) => write!(f, "node {id:?} is not a plot node"),
            Error::AlreadyHasClickHandler(id) => write!(f, "plot node {id:?} already has a click handler"),
            Error::OperandNotData(id) => write!(f, "node {id:?} is a plain label, not a DataNodeContent"),
            Error::OperatorTypeMismatch { expected, found } => {
                write!(f, "operator type mismatch: expected {expected}, found {found}")
            },
            Error::OperatorNeedsIntegers { operator, found } => {
                write!(
                    f,
                    "{operator} acts on the bits of an integer, so its operands cannot be {found}. An arithmetic operator takes floats"
                )
            },
            Error::OperatorOnUuid { operator } => {
                write!(f, "{operator} cannot act on UUIDs: a UUID is an identifier, not a number")
            },
            Error::InvalidEdgeLabel => {
                write!(
                    f,
                    "an edge label must not be empty, and must be one line without control characters"
                )
            },
            Error::OperatorOnText { operator } => {
                write!(f, "{operator} cannot act on text: a string is not a number")
            },
            Error::OperatorOnBool { operator } => {
                write!(
                    f,
                    "{operator} cannot act on bool: only NOT, AND, OR, XOR, NAND, NOR and XNOR can"
                )
            },
            Error::OperatorOnComplex { operator } => {
                write!(f, "{operator} cannot act on complex numbers: only ADD, SUB, MUL and DIV can")
            },
            Error::OperatorResultNotSingleValue(len) => {
                write!(f, "operator result must hold exactly one value, found {len}")
            },
            Error::DuplicateOperands(id) => {
                write!(f, "node {id:?} cannot be used as both operands of a two-input operator node")
            },
            Error::ButterflyNotComplex(Some(id)) => {
                write!(f, "butterfly operand {id:?} must hold one complex number")
            },
            Error::ButterflyNotComplex(None) => write!(f, "butterfly results must each hold one complex number"),
            Error::IncompatibleNodeValues(id) => write!(
                f,
                "node {id:?} cannot show those values: it must be a data node or an operator node, and they must be the same type and count"
            ),
            Error::InvalidSelection(id, selection) => {
                write!(f, "selection {selection:?} cannot be applied to node {id:?}")
            },
            Error::InvalidToolbarOptions(options) => {
                write!(
                    f,
                    "toolbar options {options:?} are invalid: button_height must be finite and > 0.0, and gap and margin must be finite and >= 0.0"
                )
            },
            Error::InvalidSelectionToolbarOptions(options) => {
                write!(
                    f,
                    "selection toolbar options {options:?} are invalid: button_height must be finite and > 0.0, and gap and margin must be finite and >= 0.0"
                )
            },
            Error::InvalidSceneTitleOptions(options) => {
                write!(
                    f,
                    "scene title options {options:?} are invalid: font_size must be finite and > 0.0, margin must be finite and >= 0.0, and aria_level must be >= 1"
                )
            },
            Error::NotAContainerNode(id) => write!(f, "node {id:?} is not a container node"),
            Error::NotFocused => write!(f, "this Scene is not its own scene tree's currently focused Scene"),
            Error::ChildNotFocused => {
                write!(f, "the supplied child Scene is not the currently focused Scene of its own tree")
            },
            Error::AlreadyNested => write!(f, "the supplied child Scene already has a live parent"),
            Error::SelfNesting => {
                write!(
                    f,
                    "a Scene cannot be given a child Scene that is itself or one of its own ancestors"
                )
            },
            Error::InvalidView(view) => {
                write!(
                    f,
                    "view {view:?} is invalid: scale must be finite and within 0.25..=4.0, and tx/ty must be finite"
                )
            },
        }
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
impl std::error::Error for Error {
    /// Exposes the wrapped [`svg_dom::Error`] for [`Error::Svg`], so error-reporting tools and callers walking the
    /// standard error chain can discover it. `Display` already forwards its message, but that alone does not help code
    /// that specifically walks using `source()`.
    ///
    /// Every other variant originates in this crate itself, not from wrapping another error, so `None` is correct for
    /// them. This is the default this method would return without being overridden at all.
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Svg(err) => Some(err),
            _ => None,
        }
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
impl From<svg_dom::Error> for Error {
    fn from(err: svg_dom::Error) -> Self {
        Error::Svg(err)
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[cfg(test)]
mod unit_tests;
