//! This crate's own error type.
//!
//! Wraps [`svg_dom::Error`] for anything that comes from the underlying DOM library, and adds variants for graph-domain
//! problems detected by this crate.  For example, a [`NodeId`]/[`EdgeId`] used with a `Scene` that did not create it.

use crate::model::{edge::EdgeId, node::NodeId};
use std::fmt;

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// An error from a `svg-dom-graph` operation.
#[derive(Debug)]
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
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::Svg(err) => write!(f, "{err}"),
            Error::UnknownNode(id) => write!(f, "node {id:?} does not belong to this Scene"),
            Error::UnknownEdge(id) => write!(f, "edge {id:?} does not belong to this Scene"),
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
