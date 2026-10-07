use crate::scene::{ConnectorType, Side};

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Configures how [Scene::add_edge_with](crate::scene::Scene::add_edge_with) draws a connector.
///
/// Build one either with [`ConnectorOptions::default`] or with [`with_connector_type`](Self::with_connector_type)/
/// [`with_from_side`](Self::with_from_side)/[`with_to_side`](Self::with_to_side). A struct literal does not compile
/// outside this crate.
///
/// ***A note on `Copy`***
///
/// Deriving `Copy` is a deliberate compatibility commitment, not an oversight: removing `Copy` later is a breaking
/// change, so every field this type gains must itself stay `Copy`. See the same note on [`ConnectorType`], which this
/// type carries, and on [`DragOptions`](crate::scene::DragOptions), which shares the same commitment.
#[derive(Debug, Clone, Copy, PartialEq)]
#[non_exhaustive]
pub struct ConnectorOptions {
    /// How this connector routes — see [`ConnectorType`].
    pub connector_type: ConnectorType,
    /// Forces this connector's own `from` endpoint to leave that node on this side. Otherwise it would leave on
    /// whichever side a ray from its own centre toward the other endpoint crosses first. `None` (the default) leaves
    /// that automatic choice in place. See [`with_from_side`](Self::with_from_side).
    pub from_side: Option<Side>,
    /// The same forced choice as [`from_side`](Self::from_side), for this connector's own `to` endpoint instead.
    ///
    /// Not honoured for an edge into a two-input operator node from one of that operator's own operands. See
    /// [`with_to_side`](Self::with_to_side).
    pub to_side: Option<Side>,
    /// Where along the forced [`from_side`](Self::from_side) this connector's own `from` endpoint sits, as a fraction
    /// from `0.0` to `1.0`. `None` (the default) leaves the position to the node's own anchor rule. Has no routing
    /// effect unless `from_side` is `Some`, but is validated either way. See
    /// [`with_from_position`](Self::with_from_position).
    pub from_position: Option<f64>,
    /// The same choice as [`from_position`](Self::from_position), for this connector's own `to` endpoint instead.
    pub to_position: Option<f64>,
}

impl ConnectorOptions {
    /// Returns `self` with `connector_type` set to `connector_type`.
    ///
    /// ```
    /// use svg_dom_graph::scene::{ConnectorOptions, ConnectorType};
    /// let options = ConnectorOptions::default().with_connector_type(ConnectorType::Straight);
    /// assert_eq!(options.connector_type, ConnectorType::Straight);
    /// ```
    #[must_use]
    pub fn with_connector_type(mut self, connector_type: ConnectorType) -> Self {
        self.connector_type = connector_type;
        self
    }

    /// Returns `self` with `from_side` set to `side` — `Some(side)` forces the connector's own `from` endpoint to leave
    /// that node on `side`; `None` restores the automatic choice.
    ///
    /// Forcing a side never fails, and never fails to draw. Whatever route the elbow/straight geometry produces between
    /// the two forced (or automatically chosen) endpoints is drawn as-is. That holds however awkward the result looks
    /// for a particular pair of node positions. Picking positions that make the forced route read sensibly is left
    /// entirely to the caller, the same way choosing sensible node positions already is.
    ///
    /// ```
    /// use svg_dom_graph::scene::{ConnectorOptions, Side};
    /// let options = ConnectorOptions::default().with_from_side(Some(Side::North));
    /// assert_eq!(options.from_side, Some(Side::North));
    /// ```
    #[must_use]
    pub fn with_from_side(mut self, side: Option<Side>) -> Self {
        self.from_side = side;
        self
    }

    /// Returns `self` with `from_position` set to `position`. `Some(f)` pins the connector's own `from` endpoint `f` of
    /// the way along the forced [`from_side`](Self::from_side), from the west end of a North/South side or the north
    /// end of an East/West one. `0.0` is that end, `0.5` the midpoint and `1.0` the other end.
    ///
    /// The point no longer depends on the other endpoint, or on the node's own
    /// [`EdgeAnchors`](crate::scene::EdgeAnchors). A caller can use it to leave a node exactly below one of its own
    /// columns, say. It has no routing effect unless a side is forced too, but it is validated either way. A malformed
    /// value is then caught at once, not only once a caller later adds a side. `None` restores the node's own anchor
    /// rule.
    ///
    /// Rejected by [`Scene::add_edge_with`](crate::scene::Scene::add_edge_with) with
    /// [`Error::InvalidConnectorPosition`](crate::error::Error::InvalidConnectorPosition) unless it is finite and in
    /// `0.0..=1.0`, whether or not a side is forced.
    ///
    /// ```
    /// use svg_dom_graph::scene::{ConnectorOptions, Side};
    /// let options = ConnectorOptions::default().with_from_side(Some(Side::South)).with_from_position(Some(0.25));
    /// assert_eq!(options.from_position, Some(0.25));
    /// ```
    #[must_use]
    pub fn with_from_position(mut self, position: Option<f64>) -> Self {
        self.from_position = position;
        self
    }

    /// The same pin as [`with_from_position`](Self::with_from_position), for this connector's own `to` endpoint
    /// instead.
    #[must_use]
    pub fn with_to_position(mut self, position: Option<f64>) -> Self {
        self.to_position = position;
        self
    }

    /// The same forced choice as [`with_from_side`](Self::with_from_side), for this connector's own `to` endpoint
    /// instead.
    ///
    /// ***Exception: a two-input operator's own inputs.*** An edge from one of a binary or arithmetic operator node's
    /// own two operands into that operator is anchored by the operator itself. It keeps its two inputs apart and routes
    /// them so they do not cross. That anchoring overrides `to_side` and [`to_position`](Self::to_position), which are
    /// accepted and validated but have no routing effect on such an edge. `from_side` and `from_position` still apply.
    ///
    /// ```
    /// use svg_dom_graph::scene::{ConnectorOptions, Side};
    /// let options = ConnectorOptions::default().with_to_side(Some(Side::North));
    /// assert_eq!(options.to_side, Some(Side::North));
    /// ```
    #[must_use]
    pub fn with_to_side(mut self, side: Option<Side>) -> Self {
        self.to_side = side;
        self
    }
}

impl Default for ConnectorOptions {
    /// An elbowed connector with a sharp, 90º corner ([`ConnectorType::Elbow`] with `corner_radius: 0.0`), with both
    /// endpoints' own sides chosen automatically (`from_side`/`to_side` both `None`).
    fn default() -> Self {
        Self {
            connector_type: ConnectorType::Elbow { corner_radius: 0.0 },
            from_side: None,
            to_side: None,
            from_position: None,
            to_position: None,
        }
    }
}
