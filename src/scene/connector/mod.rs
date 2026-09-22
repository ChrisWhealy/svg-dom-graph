//! Connector configuration and the `Scene` methods that draw a connector.
mod connector_handle;
mod connector_options;
mod connector_type;

use super::{Scene, node::EdgeAnchors};
use crate::{
    error::Error,
    geometry::{
        binary_operator_elbow_route, boundary_point, centre, edge_anchor, elbow_path_into, elbow_route, route::Route,
        route::straight_route, side::Side, snapped_anchor,
    },
    model::{edge::EdgeId, node::NodeId},
};
pub(crate) use connector_handle::ConnectorHandle;
pub use connector_options::ConnectorOptions;
pub use connector_type::ConnectorType;
use svg_dom::root::utils::{Point, Rect};

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Returns [`Error::InvalidCornerRadius`] if `connector_type` is [`ConnectorType::Elbow`] with a corner radius that is
/// not a finite, non-negative value `>= 0.0`. No validation is required for the other `ConnectorType` variants.
fn validate_connector_type(connector_type: ConnectorType) -> Result<(), Error> {
    match connector_type {
        ConnectorType::Elbow { corner_radius } if !corner_radius.is_finite() || corner_radius < 0.0 => {
            Err(Error::InvalidCornerRadius(corner_radius))
        },
        _ => Ok(()),
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// One endpoint's anchor point for a straight connector, honouring `anchors`.
///
/// `Some(EdgeAnchors(n))` snaps to the nearest of `n` evenly-spaced candidates — see [`snapped_anchor`]. `None` keeps
/// [`ConnectorType::Straight`]'s own default calculated as a continuous ray crossing — see [`boundary_point`].
fn straight_anchor(rect: Rect, towards: Point, anchors: Option<EdgeAnchors>) -> Point {
    match anchors {
        Some(EdgeAnchors(n)) => snapped_anchor(rect, towards, n).0,
        None => boundary_point(rect, towards),
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// One endpoint's anchor point and side for an elbowed connector, honouring `anchors`.
///
/// `Some(EdgeAnchors(n))` snaps to the nearest of `n` evenly-spaced candidates — see [`snapped_anchor`]. `None` keeps
/// [`ConnectorType::Elbow`]'s own default calculated as the crossed side's own midpoint — see [`edge_anchor`].
fn elbow_anchor(rect: Rect, towards: Point, anchors: Option<EdgeAnchors>) -> (Point, Side) {
    match anchors {
        Some(EdgeAnchors(n)) => snapped_anchor(rect, towards, n),
        None => edge_anchor(rect, towards),
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Everything [`route`] needs to give a binary operator node's own same-side input its non-crossing route — see
/// [`crate::geometry::binary_operator_elbow_route`]'s own doc comment for the scheme this exists to feed.
///
/// [`SceneInner::binary_operator_to_override`](super::SceneInner::binary_operator_to_override) is the only place
/// that builds one.
pub(crate) struct BinaryOperatorRoute {
    /// This edge's own already-split anchor point on the operator — see [`crate::geometry::binary_operator_anchors`]
    pub(crate) anchor: Point,
    /// The side of the operator `anchor` sits on.
    pub(crate) side: Side,
    /// The sibling edge's own already-split anchor point on the operator — `Some` only when the sibling input
    /// lands on this same `side`, the one case [`binary_operator_elbow_route`]'s own sibling-aware routing applies.
    ///
    /// `None` when the two inputs land on different sides of the operator. [`binary_operator_anchors`]'s own doc
    /// comment already documents that a different-side input behaves exactly like an ordinary edge would — `route`
    /// honours that here by falling back to plain [`elbow_route`] rather than calling
    /// [`binary_operator_elbow_route`] with an unrelated sibling coordinate from a genuinely different side, which
    /// that function's own drift comparison assumes never happens.
    ///
    /// [`binary_operator_anchors`]: crate::geometry::binary_operator_anchors
    pub(crate) sibling_end: Option<Point>,
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// A connector's own corner points and the corner radius by which it might be rounded. Exists for a `connector_type`
/// between `from` and `to`, each of which have their respective `from_anchors` and `to_anchors`.
///
/// A [`ConnectorType::Straight`] connector cannot have a corner radius, so its radius is always `0.0`.
///
/// `from_anchors` / `to_anchors` are each that node's own [`EdgeAnchors`] configuration, independent of the other
/// endpoint's — one endpoint can use `None` while the other uses `Some`.
///
/// `to_override`, when `Some`, replaces the `to`-side anchor this would otherwise compute from `to_anchors` — see
/// [`SceneInner::binary_operator_to_override`](super::SceneInner::binary_operator_to_override) for the one case
/// that supplies it: a binary operator node's own two inputs, split apart and routed clear of each other when they
/// land on the same side.
pub(crate) fn route(
    connector_type: ConnectorType,
    from: Rect,
    from_anchors: Option<EdgeAnchors>,
    to: Rect,
    to_anchors: Option<EdgeAnchors>,
    to_override: Option<BinaryOperatorRoute>,
) -> (Route, f64) {
    let from_centre = centre(from);
    let to_centre = centre(to);

    match connector_type {
        ConnectorType::Straight => {
            let start = straight_anchor(from, to_centre, from_anchors);
            let end = to_override.map_or_else(|| straight_anchor(to, from_centre, to_anchors), |o| o.anchor);
            (straight_route(start, end), 0.0)
        },
        ConnectorType::Elbow { corner_radius } => {
            let (start, start_side) = elbow_anchor(from, to_centre, from_anchors);
            let route = match to_override {
                Some(BinaryOperatorRoute {
                    anchor,
                    side,
                    sibling_end: Some(sibling_end),
                }) => binary_operator_elbow_route(start, start_side, anchor, side, sibling_end),
                Some(BinaryOperatorRoute {
                    anchor,
                    side,
                    sibling_end: None,
                }) => elbow_route(start, start_side, anchor, side),
                None => {
                    let (end, end_side) = elbow_anchor(to, from_centre, to_anchors);
                    elbow_route(start, start_side, end, end_side)
                },
            };
            (route, corner_radius)
        },
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
    /// Returns [`Error::UnknownNode`] if `from` or `to` does not name a node in this scene — for example, a `NodeId`
    /// from a different `Scene`. Returns [`Error::SelfLoopUnsupported`] if `from` and `to` are the same node in this
    /// scene — not yet supported, see that variant's own doc comment for why.
    pub fn add_edge(&self, from: NodeId, to: NodeId) -> Result<EdgeId, Error> {
        self.add_edge_with(from, to, ConnectorOptions::default())
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    /// Adds a directed edge to the graph, draws its arrow-tipped connector with `options` controlling how it routes,
    /// and returns its id.
    ///
    /// See [`ConnectorType`] for the routing styles available, and the anchor rule each one follows.
    ///
    /// # Errors
    ///
    /// Returns [`Error::InvalidCornerRadius`] if `options.connector_type` is [`ConnectorType::Elbow`] with a corner
    /// radius that is not a finite value `>= 0.0`. Checked before drawing anything or touching the graph, so a rejected
    /// call leaves the scene exactly as it was.
    ///
    /// Returns [`Error::UnknownNode`] if `from` or `to` does not name a node in this scene — for example, a `NodeId`
    /// from a different `Scene`. Checked before the self-loop check below, so a foreign id is always reported as
    /// unknown, even if `from` and `to` are the same foreign id. Returns [`Error::SelfLoopUnsupported`] if `from` and
    /// `to` are the same node in this scene — not yet supported, see that variant's own doc comment for why.
    pub fn add_edge_with(&self, from: NodeId, to: NodeId, options: ConnectorOptions) -> Result<EdgeId, Error> {
        validate_connector_type(options.connector_type)?;

        let mut inner = self.inner.borrow_mut();
        let from_rect = inner.node_rect(from)?;
        let from_anchors = inner.node_edge_anchors(from)?;
        let to_rect = inner.node_rect(to)?;
        let to_anchors = inner.node_edge_anchors(to)?;

        if from == to {
            return Err(Error::SelfLoopUnsupported(from));
        }

        let to_override = inner.binary_operator_to_override(from, to);
        let (vertices, radius) = route(
            options.connector_type,
            from_rect,
            from_anchors,
            to_rect,
            to_anchors,
            to_override,
        );
        // Taken out for the call so `inner.svg` can be borrowed for it without also needing `inner` mutability —
        // see `SceneInner::scratch`'s own doc comment for why this, rather than a fresh `String` per new edge.
        let mut d = std::mem::take(&mut inner.scratch);
        elbow_path_into(&vertices, radius, &mut d);
        let path_result = inner.svg.path(&d);
        inner.scratch = d;
        let path = path_result?;
        path.set_fill("none")?;
        path.set_stroke("#555")?;
        path.set_stroke_width(1.5)?;
        path.set_marker_end_ref(&inner.arrow)?;

        let id = inner.graph.add_edge(from, to);
        inner.insert_edge_handle(
            id,
            ConnectorHandle {
                path,
                connector_type: options.connector_type,
                port_marker: None,
            },
        );
        Ok(id)
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    /// Updates edge `id`'s connector type, and redraws it immediately with the new value.
    ///
    /// Every later reroute, as either endpoint moves, keeps using this new type. This is the only way to change an
    /// edge's connector type after [`Scene::add_edge`] or [`Scene::add_edge_with`] first draws it. A live control can
    /// use it to switch between [`ConnectorType::Straight`] and [`ConnectorType::Elbow`], or to adjust an elbow's
    /// corner radius.
    ///
    /// # Errors
    ///
    /// Returns [`Error::InvalidCornerRadius`] if `connector_type` is [`ConnectorType::Elbow`] with a corner radius that
    /// is not a finite value `>= 0.0`. Checked before touching the scene, so a rejected call leaves the connector
    /// exactly as it was.
    ///
    /// Returns [`Error::UnknownEdge`] if `id` does not name an edge in this scene.
    ///
    /// Also returns an error — [`Error::UnknownNode`] or a wrapped [`Error::Svg`] — if the redraw itself fails once
    /// underway. The stored connector type is only updated once the new path has actually been written. A failure here
    /// leaves `id` rendered and recorded exactly as it was before the call.
    ///
    /// A `connector_type` is received that is identical to `id`'s own current one, then this is an immediate no-op and
    /// we bail out early. [`ConnectorType`] derives `PartialEq`, so this comparison is free next. This is the check
    /// that matters most for a live corner-radius slider, which which otherwise fire this call once per input event
    /// with a value that has not actually changed.
    pub fn set_connector_type(&self, id: EdgeId, connector_type: ConnectorType) -> Result<(), Error> {
        validate_connector_type(connector_type)?;

        let mut inner = self.inner.borrow_mut();
        let handle = inner.edge_handle(id).ok_or(Error::UnknownEdge(id))?;
        if handle.connector_type == connector_type {
            return Ok(());
        }

        // Taken out for the call so `redraw_edge_with_type` can freely borrow the rest of `inner`, then put back —
        // see `SceneInner::scratch`'s own doc comment for why this, rather than a fresh `String` per call.
        let mut scratch = std::mem::take(&mut inner.scratch);
        let result = inner.redraw_edge_with_type(id, connector_type, &mut scratch);
        inner.scratch = scratch;
        result
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[cfg(test)]
mod unit_tests;
