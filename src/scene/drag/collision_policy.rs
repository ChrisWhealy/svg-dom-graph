// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// What [`Scene::make_draggable_with`] does when a drop leaves the dragged node overlapping another one.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum CollisionPolicy {
    /// Leaves the dropped node exactly where the pointer released it, even if that overlaps another node.
    ///
    /// Choose this when overlapping nodes are a legitimate outcome for the caller's own graph — this crate does
    /// not otherwise have an opinion on whether nodes may overlap.
    Allow,
    /// Pushes the dropped node back along the line from its pre-drag position, clear of whatever it overlaps.
    ///
    /// This is a best-effort, single-pass correction, not a guarantee that the node ends up clear of every other
    /// node. It resolves against only the nearest node the drop overlaps — the corrected position can still
    /// overlap a different node than the one it was pushed clear of. [`Scene::add_node`] also does not itself
    /// reject an overlapping starting position, so a hard "nodes never overlap" invariant is not achievable by
    /// this policy in general, only reduced.
    PushClear {
        /// Extra clearance kept between the dropped node and whatever it overlapped, in this scene's user-space
        /// units, so the two end up with a visible gap rather than touching edges.
        padding: f64,
    },
}
