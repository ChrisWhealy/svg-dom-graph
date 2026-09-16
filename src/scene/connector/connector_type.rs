// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// How [Scene::add_edge_with](crate::scene::Scene::add_edge_with) and
/// [Scene::set_connector_type](crate::scene::Scene::set_connector_type) route a connector.
///
/// `#[non_exhaustive]` is used here because this type is expected to grow: a Bezier-curved connector is a likely future
/// addition. Matching on this outside the crate requires a wildcard arm; constructing an existing variant
/// is unaffected.
///
/// ***A note on `Copy`***
///
/// Deriving `Copy` is a deliberate compatibility commitment, not an oversight. Removing `Copy` later is a breaking
/// change, so every field any variant gains (including some future variant) must itself also implement `Copy`. See the
/// same note on [`DragOptions`](crate::scene::DragOptions), which shares the same commitment.
#[derive(Debug, Clone, Copy, PartialEq)]
#[non_exhaustive]
pub enum ConnectorType {
    /// A straight line from one box's boundary to the other's.
    ///
    /// Each end lands where the ray between the box's centres crosses the boundaries.
    Straight,
    /// Horizontal and vertical segments only, joined at corners whose radius varies from 0 pixels (90º corner) up to
    /// half the connector's length.
    ///
    /// Unless fixing points are defined for the node's edges, each end is anchored at the midpoint of the horizontal or
    /// vertical side first intersected by a ray cast between the box's centres.
    Elbow {
        /// How far to round each corner, in this scene's user-space units. `0.0` draws a sharp, 90º corner.
        ///
        /// Shrinks at each corner so it never reaches past half the length of either segment meeting there. A tight
        /// elbow rounds less. It never passes its own endpoint or a neighbouring corner.
        corner_radius: f64,
    },
}
