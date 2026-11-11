//! A slice that has been worked out and has not been committed.

use crate::projection::{Projection, View};
use service_sim_engine::Model;

/// A slice that has been worked out and has not been committed.
pub(super) struct Staged {
    pub(super) slice: u64,
    pub(super) model: Model,
    pub(super) projection: Projection,
    /// The selection that the projection was made for, which is published with it.
    pub(super) view: View,
}
