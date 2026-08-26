// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// What [`crate::scene::Scene::make_draggable_with`] does when a drop event leaves the dragged node overlapping
/// some other node.
/// Without this, this crate does not otherwise have any opinion on whether nodes may or may not overlap.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum CollisionPolicy {
    /// Leaves the dropped node exactly where the pointer released it, even if that overlaps another node.
    /// Choose this when overlapping nodes are a legitimate outcome for the caller's own graph.
    Allow,
    /// The dropped node is pushed back along the line between its pre-drag position and the centre of the about-to-be
    /// overlapped node, plus a padding distance..
    ///
    /// This can be useful if it is not appropriate for a node to be dropped at the `on_pointerup` location after
    /// a drag operation.
    ///
    /// ***IMPORTANT***
    ///
    /// This is a best-effort, single-pass correction, not a guarantee that the node ends up clear of every other node.
    /// It resolves against only the nearest node the drop overlaps. The corrected position might still overlap some
    /// other node in the proiximity of the one that would be overlapped.
    ///
    /// [`crate::scene::Scene::add_node`] also does not itself reject an overlapping starting position, so it is not
    /// possible to offer a "nodes never overlap" guarantee.  All we can say is that the likelihood of overlap is
    /// reduced.
    PushClear {
        /// Extra clearance kept between the dropped node and whatever it overlapped, in this scene's user-space
        /// units, so the two end up with a visible gap rather than touching edges.
        padding: f64,
    },
}
