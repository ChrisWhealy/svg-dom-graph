use super::EdgeAnchors;

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Configures how [Scene::add_node_with](crate::scene::Scene::add_node_with) anchors connectors to a node.
///
/// Build one either with [`NodeOptions::default`] or with [`with_edge_anchors`](Self::with_edge_anchors). A struct
/// literal does not compile outside this crate.
///
/// ***A note on `Copy`***
///
/// Deriving `Copy` is a deliberate compatibility commitment, not an oversight. Removing `Copy` later is a breaking
/// change, so every field this type gains must itself stay `Copy`. See the same note on
/// [`DragOptions`](crate::scene::DragOptions), which shares the same commitment.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[non_exhaustive]
pub struct NodeOptions {
    /// How many evenly spaced connector fixing points this node's sides offer — see [`EdgeAnchors`].
    ///
    /// `None` (the default) keeps each connector's own default anchor rule.
    pub edge_anchors: Option<EdgeAnchors>,
}

impl NodeOptions {
    /// Returns `self` with `edge_anchors` set to `edge_anchors`.
    ///
    /// ```
    /// use svg_dom_graph::scene::{EdgeAnchors, NodeOptions};
    /// let options = NodeOptions::default().with_edge_anchors(Some(EdgeAnchors(3)));
    /// assert_eq!(options.edge_anchors, Some(EdgeAnchors(3)));
    /// ```
    #[must_use]
    pub fn with_edge_anchors(mut self, edge_anchors: Option<EdgeAnchors>) -> Self {
        self.edge_anchors = edge_anchors;
        self
    }
}
