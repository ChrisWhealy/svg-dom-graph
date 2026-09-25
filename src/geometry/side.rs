// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// One side of a rectangle — a box's boundary, or the visible area of a scene.
///
/// Anchors an elbowed connector so it leaves a box exactly horizontally or exactly vertically. Also names the edge of a
/// scene that its toolbar is fixed to — see [`ToolbarOptions::edge`](crate::scene::ToolbarOptions::edge).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Side {
    North,
    South,
    East,
    West,
}
