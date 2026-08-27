//! Connector configuration and the `Scene` methods that draw a connector.

use super::{ConnectorHandle, Scene};
use crate::{
    error::Error,
    geometry::{elbow_path_into, elbow_vertices, straight_vertices},
    model::{edge::EdgeId, node::NodeId},
};
use svg_dom::root::utils::{Point, Rect};

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// How [`Scene::add_edge_with`]/[`Scene::set_connector_type`] routes a connector.
///
/// `#[non_exhaustive]` is used here because this type is expected to grow: a Bezier-curved connector is a likely
/// future addition. Matching on this outside the crate requires a wildcard arm; constructing an existing variant is
/// unaffected.
///
/// ***A note on `Copy`***
///
/// Deriving `Copy` is a deliberate compatibility commitment, not an oversight. Removing `Copy` later is a breaking
/// change, so every field any variant gains — including a future variant — must itself stay `Copy`. See the same
/// note on [`DragOptions`](crate::scene::DragOptions), which shares the same commitment.
#[derive(Debug, Clone, Copy, PartialEq)]
#[non_exhaustive]
pub enum ConnectorType {
    /// A straight line from one box's boundary to the other's.
    ///
    /// Each end lands where the ray from that box's own centre toward the other box's centre crosses its boundary —
    /// this is the crate's original, pre-elbow connector style.
    Straight,
    /// Horizontal and vertical segments only, joined at 90-degree corners.
    ///
    /// Each end is anchored at the midpoint of the horizontal or vertical side intersected first by a ray from that
    /// box's centre toward the other box's centre.
    Elbow {
        /// How far to round each corner, in this scene's user-space units. `0.0` draws a sharp corner.
        ///
        /// Shrinks at each corner so it never reaches past half the length of either segment meeting there. A
        /// tight elbow rounds less. It never passes its own endpoint or a neighbouring corner.
        corner_radius: f64,
    },
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Configures how [`Scene::add_edge_with`] draws a connector.
///
/// Build one either with [`ConnectorOptions::default`] or with
/// [`with_connector_type`](Self::with_connector_type).
/// A struct literal does not compile outside this crate.
///
/// ***A note on `Copy`***
///
/// Deriving `Copy` is a deliberate compatibility commitment, not an oversight: removing `Copy` later is a breaking
/// change, so every field this type gains must itself stay `Copy`. See the same note on
/// [`ConnectorType`], which this type carries, and on [`DragOptions`](crate::scene::DragOptions), which shares the
/// same commitment.
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
    /// An elbowed connector with a sharp, unrounded corner: [`ConnectorType::Elbow`] with `corner_radius: 0.0`.
    fn default() -> Self {
        Self {
            connector_type: ConnectorType::Elbow { corner_radius: 0.0 },
        }
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Returns [`Error::InvalidCornerRadius`] if `connector_type` is [`ConnectorType::Elbow`] with a corner radius that
/// is not a finite value `>= 0.0`. Every other variant has nothing to validate.
fn validate_connector_type(connector_type: ConnectorType) -> Result<(), Error> {
    match connector_type {
        ConnectorType::Elbow { corner_radius } if !corner_radius.is_finite() || corner_radius < 0.0 => {
            Err(Error::InvalidCornerRadius(corner_radius))
        },
        _ => Ok(()),
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The connector's own corner points, and the corner radius to round them by, for `connector_type` between
/// `from` and `to`.
///
/// A [`ConnectorType::Straight`] connector has no corners to round, so its radius is always `0.0`.
pub(crate) fn route(connector_type: ConnectorType, from: Rect, to: Rect) -> (Vec<Point>, f64) {
    match connector_type {
        ConnectorType::Straight => (straight_vertices(from, to), 0.0),
        ConnectorType::Elbow { corner_radius } => (elbow_vertices(from, to), corner_radius),
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
impl Scene {
    /// Adds a directed edge to the graph, draws its arrow-tipped connector with a sharp-cornered elbow route, and
    /// returns its id.
    ///
    /// Equivalent to [`add_edge_with`](Self::add_edge_with) with [`ConnectorOptions::default`].
    ///
    /// # Errors
    ///
    /// Returns [`Error::UnknownNode`] if `from` or `to` does not name a node in this scene — for example, a
    /// `NodeId` from a different `Scene`.
    /// Returns [`Error::SelfLoopUnsupported`] if `from` and `to` are the same node in this scene — not yet
    /// supported, see that variant's own doc comment for why.
    pub fn add_edge(&self, from: NodeId, to: NodeId) -> Result<EdgeId, Error> {
        self.add_edge_with(from, to, ConnectorOptions::default())
    }

    /// Adds a directed edge to the graph, draws its arrow-tipped connector with `options` controlling how it
    /// routes, and returns its id.
    ///
    /// See [`ConnectorType`] for the routing styles available, and the anchor rule each one follows.
    ///
    /// # Errors
    ///
    /// Returns [`Error::InvalidCornerRadius`] if `options.connector_type` is [`ConnectorType::Elbow`] with a
    /// corner radius that is not a finite value `>= 0.0`. Checked before drawing anything or touching the graph,
    /// so a rejected call leaves the scene exactly as it was.
    ///
    /// Returns [`Error::UnknownNode`] if `from` or `to` does not name a node in this scene — for example, a
    /// `NodeId` from a different `Scene`.
    /// Checked before the self-loop check below, so a foreign id is always reported as unknown, even if `from` and
    /// `to` are the same foreign id.
    /// Returns [`Error::SelfLoopUnsupported`] if `from` and `to` are the same node in this scene — not yet
    /// supported, see that variant's own doc comment for why.
    pub fn add_edge_with(&self, from: NodeId, to: NodeId, options: ConnectorOptions) -> Result<EdgeId, Error> {
        validate_connector_type(options.connector_type)?;

        let mut inner = self.inner.borrow_mut();
        let from_rect = inner.node_rect(from)?;
        let to_rect = inner.node_rect(to)?;

        if from == to {
            return Err(Error::SelfLoopUnsupported(from));
        }

        let (vertices, radius) = route(options.connector_type, from_rect, to_rect);
        let mut d = String::new();
        elbow_path_into(&vertices, radius, &mut d);

        let path = inner.svg.path(&d)?;
        path.set_fill("none")?;
        path.set_stroke("#555")?;
        path.set_stroke_width(1.5)?;
        path.set_marker_end_ref(&inner.arrow)?;

        let id = inner.graph.add_edge(from, to);
        inner.edge_handles.insert(
            id,
            ConnectorHandle {
                path,
                connector_type: options.connector_type,
            },
        );
        Ok(id)
    }

    /// Updates edge `id`'s connector type, and redraws it immediately with the new value.
    ///
    /// Every later reroute, as either endpoint moves, keeps using this new type. This is the only way to change
    /// an edge's connector type after [`Scene::add_edge`] or [`Scene::add_edge_with`] first draws it. A live
    /// control can use it to switch between [`ConnectorType::Straight`] and [`ConnectorType::Elbow`], or to adjust
    /// an elbow's corner radius.
    ///
    /// # Errors
    ///
    /// Returns [`Error::InvalidCornerRadius`] if `connector_type` is [`ConnectorType::Elbow`] with a corner radius
    /// that is not a finite value `>= 0.0`. Checked before touching the scene, so a rejected call leaves the
    /// connector exactly as it was.
    ///
    /// Returns [`Error::UnknownEdge`] if `id` does not name an edge in this scene.
    ///
    /// Also returns an error — [`Error::UnknownNode`] or a wrapped [`Error::Svg`] — if the redraw itself fails
    /// once underway. The stored connector type is only updated once the new path has actually been written. A
    /// failure here leaves `id` rendered and recorded exactly as it was before the call.
    pub fn set_connector_type(&self, id: EdgeId, connector_type: ConnectorType) -> Result<(), Error> {
        validate_connector_type(connector_type)?;

        let mut inner = self.inner.borrow_mut();
        let mut scratch = String::new();
        inner.redraw_edge_with_type(id, connector_type, &mut scratch)
    }
}
