//! How long each part of a build took, in milliseconds.

/// How long each part of a build took, in milliseconds.
#[derive(Debug, Clone, Copy, Default)]
pub struct BuildTimings {
    pub nodes_ms: f64,
    pub edges_ms: f64,
    pub data_ms: f64,
    pub spy_ms: f64,
    /// The time to give the edges and nodes the classes that a person reads them by, which is nothing if that is not
    /// asked for.
    pub style_ms: f64,
    pub total_ms: f64,
}
