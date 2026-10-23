//! What a force-directed demo needs that is not particular to one graph.
//!
//! - [`solver`] — the pure numerical model: damped springs, a rate-limited kinematic proxy for a held particle, contact
//!   and boundary projection, and the rules for sleeping. No DOM.
//! - [`clock`] — how a browser's frame timestamps become a bounded number of fixed physics steps. No DOM.
//! - [`bridge`] — a lossless, ordered queue of commands into one controller, which cannot be re-entered. No DOM.
//! - [`animation`] — a cancellable `requestAnimationFrame` that hands its callback the browser's timestamp.
//!
//! Only [`animation`] touches the browser. The rest is tested with a plain `cargo test`.

pub mod animation;
pub mod bridge;
pub mod clock;
pub mod solver;
