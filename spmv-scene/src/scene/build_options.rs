//! What a build adds to the scene so that a person can read it.

/// What a build adds to the scene so that a person can read it. The probes use neither, so that the figures they record
/// are those of the scene alone.
#[derive(Debug, Clone, Copy, Default)]
pub struct BuildOptions {
    /// Gives every edge a class for its sign and its size and every node a tag, so that a node can be selected and its
    /// edges highlighted.
    pub readable: bool,
    /// Writes the value of each edge on it. Every label is more elements, so this is for a small graph.
    pub edge_labels: bool,
    /// Where the scene's own space begins, which moves the layout and not the view box (see [`Layout::shifted`]).
    pub origin: (f64, f64),
    /// Draws the multiplication as the operations that do it, for the row that is selected, and not as a graph (see
    /// [`crate::worked`]).
    pub worked: bool,
}
