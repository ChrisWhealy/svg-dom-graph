//! This crate's own error type.
//!
//! Wraps [`svg_dom::Error`] for anything that comes from the underlying DOM library, and adds variants for graph-domain
//! problems detected by this crate.  For example, a [`NodeId`]/[`EdgeId`] used with a `Scene` that did not create it.

use crate::model::{edge::EdgeId, node::NodeId};
use std::fmt;

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// An error from a `svg-dom-graph` operation.
#[derive(Debug)]
#[non_exhaustive]
pub enum Error {
    /// An error from the underlying `svg-dom` library: DOM creation, an attribute write, and so on.
    Svg(svg_dom::Error),
    /// A `NodeId` does not name a node in the `Scene` it was used with.
    ///
    /// This happens when a `NodeId` from one `Scene` is passed to a different `Scene` — each `Scene` recognises
    /// only the ids its own node-adding methods produced.
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
        }
    }
}

impl std::error::Error for Error {}

impl From<svg_dom::Error> for Error {
    fn from(err: svg_dom::Error) -> Self {
        Error::Svg(err)
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[cfg(test)]
mod unit_tests;
