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
    /// Forces this connector's own `from` endpoint to leave that node on this side, rather than whichever side a ray
    /// from its own centre toward the other endpoint would otherwise cross first. `None` (the default) leaves that
    /// automatic choice in place. See [`with_from_side`](Self::with_from_side).
    pub from_side: Option<Side>,
    /// The same forced choice as [`from_side`](Self::from_side), for this connector's own `to` endpoint instead.
    pub to_side: Option<Side>,
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
    /// Forcing a side never fails, and never fails to draw: whatever route the elbow/straight geometry produces between
    /// the two forced (or automatically chosen) endpoints is drawn as-is, however awkward the result looks for a
    /// particular pair of node positions. Picking positions that make the forced route read sensibly is left entirely
    /// to the caller, the same way choosing sensible node positions already is.
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

    /// The same forced choice as [`with_from_side`](Self::with_from_side), for this connector's own `to` endpoint
    /// instead.
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
        }
    }
}
