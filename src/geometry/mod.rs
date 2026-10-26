//! Pure geometry helpers for routing connectors between graph boxes.
//!
//! Kept free of any DOM/wasm dependency, so it stays testable with a plain `cargo test`. This mirrors how `svg-dom`
//! itself separates pure geometry math from its DOM-facing code.

pub(crate) mod anchor;
pub(crate) mod elbow;
pub(crate) mod ellipse;
pub(crate) mod label;
pub(crate) mod operator_ports;
pub(crate) mod plane;
pub(crate) mod route;
pub(crate) mod side;
pub(crate) mod view;

pub(crate) use anchor::*;
pub(crate) use elbow::*;
pub(crate) use operator_ports::*;
pub use plane::boundary_point;
pub(crate) use plane::*;

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[cfg(test)]
mod unit_tests;
