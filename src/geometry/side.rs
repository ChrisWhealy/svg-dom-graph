// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// One side of a box's boundary.
///
/// Anchors an elbowed connector so it leaves a box exactly horizontally or exactly vertically.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Side {
    North,
    South,
    East,
    West,
}
