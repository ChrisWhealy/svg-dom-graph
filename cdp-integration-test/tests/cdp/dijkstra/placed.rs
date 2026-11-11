//! Where one node of the Dijkstra panel is, and how big, as the scene itself has them.

use serde::Deserialize;

/// Where one node is, and how big, as the scene itself has them.
#[derive(Deserialize, Debug, PartialEq)]
pub(crate) struct Placed {
    pub(crate) at: String,
    pub(crate) width: f64,
    pub(crate) height: f64,
}
