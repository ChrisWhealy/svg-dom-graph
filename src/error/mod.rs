//! This crate's own error type.
//!
//! Wraps [`svg_dom::Error`] for anything that comes from the underlying DOM library, and adds variants for graph-domain
//! problems detected by this crate.  For example, a [`NodeId`]/[`EdgeId`] used with a `Scene` that did not create it.

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
    /// The `NodeId` cannot be found in the `Scene`.
    /// All `NodeId`'s are `Scene`-specific: a `NodeId` from scene `a` cannot be used in scene `b`. So this
    /// error can occur if a `NodeId` is accidentally passed to some other scene.
    UnknownNode(NodeId),
    /// An `EdgeId` does not name an edge in the `Scene` it was used with.
    ///
    /// See [`Error::UnknownNode`] for why this can happen.
    UnknownEdge(EdgeId),
    /// `Scene::add_edge` was asked to connect a node to itself.
    ///
    /// Not supported yet: both ends of a self-edge share one rectangle, so both endpoints resolve to that
    /// rectangle's own centre — `geometry::boundary_point` returns the centre unchanged when the target point is
    /// already the centre. The result would be a zero-length line, not a real connector. Rejecting the call now,
    /// rather than silently returning an `EdgeId` for a connector nobody can see, keeps room to add real loop-edge
    /// routing later as an additive relaxation of this same method.
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
    /// A negative padding pulls the corrected position back inside the clearance boundary instead of extending
    /// it, and a non-finite value (`NaN`, `+inf`, `-inf`) propagates straight through `nearest_clear_centre` into
    /// the resulting coordinates. Rejected before any other state changes, so the scene's existing nodes are left
    /// exactly as they were.
    InvalidCollisionPadding(f64),
    /// `Scene::add_node` was given an origin or size that is not valid rectangle geometry.
    ///
    /// Every field of `rect` must be finite: SVG defines a negative `<rect>` `width`/`height` as illegal, and a
    /// non-finite coordinate or dimension would otherwise sit in the graph's model and contaminate every later
    /// geometry calculation it takes part in — `box_centre`, `boundary_point`, overlap detection, connector
    /// routing, and collision resolution all use it. `width` and `height` must also both be strictly positive: a
    /// zero-sized node has no visible box, and gives connector routing no direction to point at it in
    /// (`boundary_point` needs a well-defined interior to aim a ray at).
    ///
    /// Rejected before drawing anything or touching the graph's model, so a rejected call leaves the scene exactly
    /// as it was.
    InvalidNodeGeometry(Rect),
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
            Error::InvalidNodeGeometry(rect) => {
                write!(
                    f,
                    "node geometry {rect:?} is invalid: origin and size must be finite, and width/height must both be > 0.0"
                )
            },
        }
    }
}

impl std::error::Error for Error {
    /// Exposes the wrapped [`svg_dom::Error`] for [`Error::Svg`], so error-reporting tools and callers walking the
    /// standard error chain can discover it. `Display` already forwards its message, but that alone does not help code
    /// that specifically walks using `source()`.
    ///
    /// Every other variant originates in this crate itself, not from wrapping another error, so `None` is correct for
    /// them.  This is the default this method would return without being overridden at all.
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Svg(err) => Some(err),
            _ => None,
        }
    }
}

impl From<svg_dom::Error> for Error {
    fn from(err: svg_dom::Error) -> Self {
        Error::Svg(err)
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[cfg(test)]
mod unit_tests;
