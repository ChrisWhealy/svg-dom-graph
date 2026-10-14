use super::EdgeAnchors;

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Configures how [Scene::add_node_with](crate::scene::Scene::add_node_with) anchors connectors to a node.
///
/// Build one either with [`NodeOptions::default`] or with [`with_edge_anchors`](Self::with_edge_anchors) and
/// [`with_fill`](Self::with_fill). A struct literal does not compile outside this crate.
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
    /// The fill of a plain node's box, as any SVG paint, such as `"#d6f2ee"` or `"lightsteelblue"`.
    ///
    /// `None` (the default) keeps the usual light blue. It is read by [`Scene::add_node_with`](crate::scene::Scene::
    /// add_node_with) only. A data node's own fill says what type it holds, so it is not changed by this.
    pub fill: Option<&'static str>,
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

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    /// Returns `self` with `fill` set to `fill`. This is how a plain node is told apart from its neighbours: a heading
    /// row of a table drawn from plain nodes, for example.
    ///
    /// Only [`Scene::add_node_with`](crate::scene::Scene::add_node_with) uses it. Colour alone does not tell a reader
    /// anything that assistive technology can use, so a node that is different in a way that matters should say so in
    /// its label too.
    ///
    /// ```
    /// use svg_dom_graph::scene::NodeOptions;
    /// let options = NodeOptions::default().with_fill(Some("#d6f2ee"));
    /// assert_eq!(options.fill, Some("#d6f2ee"));
    /// ```
    #[must_use]
    pub fn with_fill(mut self, fill: Option<&'static str>) -> Self {
        self.fill = fill;
        self
    }
}
