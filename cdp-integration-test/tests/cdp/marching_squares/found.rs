//! What the marching squares panel's geometry check found for each level.

/// What the geometry check found for each level.
pub(crate) struct Found {
    pub(crate) lines: usize,
    pub(crate) open: usize,
    pub(crate) closed: usize,
    pub(crate) boundary_open: usize,
    pub(crate) seams: usize,
    pub(crate) max_edge: f64,
    pub(crate) max_level: f64,
    pub(crate) max_readback: f64,
}
