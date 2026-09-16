use crate::scene::ConnectorType;

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Configures how [Scene::add_edge_with](crate::scene::Scene::add_edge_with) draws a connector.
///
/// Build one either with [`ConnectorOptions::default`] or with [`with_connector_type`](Self::with_connector_type). A
/// struct literal does not compile outside this crate.
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
}

impl Default for ConnectorOptions {
    /// An elbowed connector with a sharp, 90º corner: i.e. [`ConnectorType::Elbow`] with `corner_radius: 0.0`.
    fn default() -> Self {
        Self {
            connector_type: ConnectorType::Elbow { corner_radius: 0.0 },
        }
    }
}
