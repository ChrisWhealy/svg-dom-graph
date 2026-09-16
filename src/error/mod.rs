//! This crate's own error type.
//!
//! Wraps [`svg_dom::Error`] for anything that comes from the underlying DOM library, and adds variants for graph-domain
//! problems detected by this crate. For example, a [`NodeId`]/[`EdgeId`] used with a `Scene` that did not create it.

use crate::model::{edge::EdgeId, node::NodeId};
use std::fmt;
use svg_dom::root::utils::Rect;

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
    /// Rejecting the call now keeps room to add real loop-edge routing later, as an additive relaxation of this
    /// same method.
    ///
    /// Silently returning an `EdgeId` for a connector nobody can see would be worse.
    SelfLoopUnsupported(NodeId),
    /// `Scene::make_draggable`/`Scene::make_draggable_with` was called more than once for the same node.
    ///
    /// `svg-dom`'s listener registration is append-only — a second call would not replace the first, it would add a
    /// second, independent set of pointer listeners and drag-state alongside it, both responding to the same events.
    /// Rejecting the second call keeps that from happening silently.
    AlreadyDraggable(NodeId),
    /// `Scene::make_draggable_with` was given a `CollisionPolicy::PushClear` padding that is not a finite value
    /// `>= 0.0`.
    ///
    /// A negative padding pulls the corrected position back inside the clearance boundary instead of extending it, and
    /// a non-finite value (`NaN`, `+inf`, `-inf`) propagates straight through `nearest_clear_centre` into the resulting
    /// coordinates. Rejected before any other state changes, so the scene's existing nodes are left exactly as
    /// they were.
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
    /// `Scene::add_node_with` or `Scene::set_edge_anchors` was given `Some(EdgeAnchors(0))`.
    ///
    /// Zero fixing points has no meaning: a side with no candidate point cannot anchor a connector. Use `None` instead,
    /// to keep each connector's own default anchor rule.
    InvalidEdgeAnchors(u8),
    /// `Scene::add_node` or `Scene::add_node_with` was given an origin or size that is not valid rectangle geometry.
    ///
    /// Every field of `rect` must be finite: SVG defines a negative `<rect>` `width`/`height` as illegal. An infinite
    /// coordinate or dimension would otherwise sit in the graph's model and contaminate every later geometry
    /// calculation in which it takes part — `box_centre`, `boundary_point`, overlap detection, connector routing, and
    /// collision resolution all use it. `width` and `height` must also both be strictly positive: a zero-sized node has
    /// no visible box, and gives connector routing no direction to point at it in (`boundary_point` needs a
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
    /// `Scene::add_data_node`/`Scene::add_data_node_with` was given a [`crate::scene::DataNodeContent`] whose
    /// [`crate::scene::GridLayout`] wraps `0` — `Columns(0)`, `Rows(0)`, or `MaxColumns(0)`.
    ///
    /// Zero columns (or rows) has no meaning: there is nowhere to place any value. Rejected before drawing anything or
    /// touching the graph's model, so a rejected call leaves the scene unchanged.
    InvalidGridLayout(crate::scene::GridLayout),
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::Svg(err) => write!(f, "{err}"),
            Error::UnknownNode(id) => write!(f, "node {id:?} does not belong to this Scene"),
            Error::UnknownEdge(id) => write!(f, "edge {id:?} does not belong to this Scene"),
            Error::SelfLoopUnsupported(id) => write!(f, "self-loop on node {id:?} is not yet supported"),
            Error::AlreadyDraggable(id) => write!(f, "node {id:?} is already draggable"),
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
            Error::InvalidGridLayout(layout) => {
                write!(f, "grid layout {layout:?} must use a column/row count >= 1")
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
