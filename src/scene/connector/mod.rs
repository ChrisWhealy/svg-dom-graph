//! Connector configuration and the `Scene` methods that draw a connector.
mod connector_handle;
mod connector_options;
mod connector_type;
mod edge_label;

use super::{Scene, node::EdgeAnchors};
use crate::{
    colours::{CONNECTOR_STROKE, EDGE_FOCUS_STROKE},
    error::{EdgeEnd, EndpointProblem, Error},
    geometry::{
        binary_operator_elbow_route, boundary_point, centre, edge_anchor, elbow_path_into, elbow_route,
        ellipse::{ClockPosition, EllipseRadii, automatic_cardinal, boundary_towards, clock_point},
        forced_anchor, position_anchor,
        route::Route,
        route::straight_route,
        side::Side,
        snapped_anchor,
    },
    model::{edge::EdgeId, node::NodeId},
};
pub(crate) use connector_handle::{ConnectorHandle, Pin};
pub use connector_options::ConnectorOptions;
pub use connector_type::ConnectorType;
use std::cell::Cell;
use svg_dom::root::utils::{Point, Rect};

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// How thick a connector's line is.
const CONNECTOR_WIDTH: f64 = 1.5;

/// How thick a focused connector's line is: more than twice as thick, so that it can be told apart without its colour.
const FOCUSED_WIDTH: f64 = 3.5;

/// The attribute a focused connector's path carries, for a test or a stylesheet to find it by.
pub(crate) const FOCUS_ATTRIBUTE: &str = "data-svg-dom-graph-edge-focus";

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Returns [`Error::InvalidCornerRadius`] if `connector_type` is [`ConnectorType::Elbow`] with a corner radius that is
/// not a finite, non-negative value `>= 0.0`. No validation is required for the other `ConnectorType` variants.
fn validate_connector_options(options: ConnectorOptions) -> Result<(), Error> {
    for position in [options.from_position, options.to_position].into_iter().flatten() {
        if !(0.0..=1.0).contains(&position) {
            return Err(Error::InvalidConnectorPosition(position));
        }
    }
    validate_connector_type(options.connector_type)
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Returns [`Error::InvalidCornerRadius`] as [`validate_connector_type`] does.
fn validate_connector_type(connector_type: ConnectorType) -> Result<(), Error> {
    match connector_type {
        ConnectorType::Elbow { corner_radius } if !corner_radius.is_finite() || corner_radius < 0.0 => {
            Err(Error::InvalidCornerRadius(corner_radius))
        },
        _ => Ok(()),
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Checks the options of both ends of a new edge against the shapes of the nodes there, before anything is drawn.
///
/// An end that is a rectangle must have no clock position. An end that is an ellipse must not combine a clock position
/// with a forced side, nor be given a position along a side, and an elbow can use only a cardinal clock position. Both
/// ends are checked before either is acted on, so a bad option on one cannot leave a connector half made at the other.
fn validate_ends(inner: &super::SceneInner, from: NodeId, to: NodeId, options: ConnectorOptions) -> Result<(), Error> {
    let ends = [
        (
            EdgeEnd::From,
            from,
            options.from_side,
            options.from_position,
            options.from_clock,
        ),
        (EdgeEnd::To, to, options.to_side, options.to_position, options.to_clock),
    ];
    for (endpoint, node, side, position, clock) in ends {
        let is_ellipse = inner.graph.node(node).is_some_and(|node| node.ellipse.is_some());
        let problem = |problem| Err(Error::InvalidEllipseEndpoint { endpoint, problem });
        match (is_ellipse, clock) {
            (false, Some(_)) => return problem(EndpointProblem::ClockOnRectangle),
            (true, Some(_)) if side.is_some() => return problem(EndpointProblem::ClockWithSide),
            _ => {},
        }
        if is_ellipse && position.is_some() {
            return problem(EndpointProblem::FractionOnEllipse);
        }
        let is_elbow = matches!(options.connector_type, ConnectorType::Elbow { .. });
        if let Some(position) = clock.filter(|position| is_elbow && !position.is_cardinal()) {
            return Err(Error::InvalidEllipseElbowClock(position));
        }
    }
    Ok(())
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// One endpoint's anchor point for a straight connector, honouring `anchors` and `forced_side`.
///
/// `forced_side: Some(side)` anchors on `side` regardless of `towards`, via [`forced_anchor`] — see
/// [`Scene::add_edge_with`]'s own doc comment for when a caller needs this. `None` leaves the side to be chosen
/// automatically: `Some(EdgeAnchors(n))` snaps to the nearest of `n` evenly-spaced candidates — see [`snapped_anchor`].
/// `None` for both keeps [`ConnectorType::Straight`]'s own default, calculated as a continuous ray crossing — see
/// [`boundary_point`].
fn straight_anchor(rect: Rect, towards: Point, anchors: Option<EdgeAnchors>, forced_side: Option<Pin>) -> Point {
    if let Some(Pin { side, position }) = forced_side {
        if let Some(fraction) = position {
            return position_anchor(rect, side, fraction);
        }
        return forced_anchor(rect, towards, side, anchors.map(|EdgeAnchors(n)| n));
    }
    match anchors {
        Some(EdgeAnchors(n)) => snapped_anchor(rect, towards, n).0,
        None => boundary_point(rect, towards),
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// One endpoint's anchor point and side for an elbowed connector, honouring `anchors` and `forced_side`.
///
/// `forced_side: Some(side)` anchors on `side` regardless of `towards`, via [`forced_anchor`] — see
/// [`Scene::add_edge_with`]'s own doc comment for when a caller needs this. `None` leaves the side to be chosen
/// automatically: `Some(EdgeAnchors(n))` snaps to the nearest of `n` evenly-spaced candidates — see [`snapped_anchor`].
/// `None` for both keeps [`ConnectorType::Elbow`]'s own default, calculated as the crossed side's own midpoint — see
/// [`edge_anchor`].
fn elbow_anchor(rect: Rect, towards: Point, anchors: Option<EdgeAnchors>, forced_side: Option<Pin>) -> (Point, Side) {
    if let Some(Pin { side, position }) = forced_side {
        if let Some(fraction) = position {
            return (position_anchor(rect, side, fraction), side);
        }
        return (forced_anchor(rect, towards, side, anchors.map(|EdgeAnchors(n)| n)), side);
    }
    match anchors {
        Some(EdgeAnchors(n)) => snapped_anchor(rect, towards, n),
        None => edge_anchor(rect, towards),
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// What [`route`] needs to know about one end of an edge besides its rectangle: whether the node there is an ellipse,
/// and the clock position the edge was given for that end.
///
/// A rectangle end has neither, and keeps every rule it had before ellipses existed.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub(crate) struct End {
    /// The radii of the ellipse at this end, or `None` for a rectangle. The ellipse is centred on the rectangle's centre.
    pub(crate) radii: Option<EllipseRadii>,
    /// The minute the edge was given for this end. Only ever `Some` for an ellipse.
    pub(crate) clock: Option<ClockPosition>,
}

impl End {
    /// An end that is a rectangle.
    pub(crate) const RECTANGLE: Self = Self { radii: None, clock: None };
}

/// The cardinal clock position of a side.
fn cardinal_of(side: Side) -> ClockPosition {
    match side {
        Side::North => ClockPosition::NORTH,
        Side::East => ClockPosition::EAST,
        Side::South => ClockPosition::SOUTH,
        Side::West => ClockPosition::WEST,
    }
}

/// The side that a cardinal clock position leaves by. A position that is not a cardinal has no side, and is east.
fn side_of(position: ClockPosition) -> Side {
    match position {
        ClockPosition::NORTH => Side::North,
        ClockPosition::SOUTH => Side::South,
        ClockPosition::WEST => Side::West,
        _ => Side::East,
    }
}

/// One endpoint's anchor point for a straight connector at an ellipse centred at `centre`.
///
/// A forced side lands on that side's cardinal point, and an explicit clock position on its own point. With neither, the
/// end is where the line towards the other node's centre crosses the outline, and it is never snapped to a cardinal.
/// Coincident centres give the east point. If the numbers overflow, the centre is used, which cannot happen for an
/// ellipse that was accepted.
fn ellipse_straight_anchor(
    centre_point: Point,
    radii: EllipseRadii,
    forced_side: Option<Pin>,
    clock: Option<ClockPosition>,
    towards: Point,
) -> Point {
    let point = if let Some(Pin { side, .. }) = forced_side {
        clock_point(centre_point, radii, cardinal_of(side))
    } else if let Some(position) = clock {
        clock_point(centre_point, radii, position)
    } else {
        boundary_towards(centre_point, radii, towards)
    };
    point.unwrap_or(centre_point)
}

/// One endpoint's anchor point and side for an elbowed connector at an ellipse centred at `centre`.
///
/// The end is always at a cardinal point: the forced side, else the cardinal clock position the edge was given, else the
/// cardinal that the other node's centre is nearest to by angle, worked out from this ellipse's own centre. The side is
/// the one that cardinal leaves by, which is what the elbow router needs.
fn ellipse_elbow_anchor(
    centre_point: Point,
    radii: EllipseRadii,
    forced_side: Option<Pin>,
    clock: Option<ClockPosition>,
    towards: Point,
) -> (Point, Side) {
    let cardinal = if let Some(Pin { side, .. }) = forced_side {
        cardinal_of(side)
    } else if let Some(position) = clock.filter(|position| position.is_cardinal()) {
        position
    } else {
        automatic_cardinal(centre_point, towards)
    };
    let point = clock_point(centre_point, radii, cardinal).unwrap_or(centre_point);
    (point, side_of(cardinal))
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Everything [`route`] needs to give a binary operator node's own same-side input its non-crossing route — see
/// [`crate::geometry::binary_operator_elbow_route`]'s own doc comment for the scheme this exists to feed.
///
/// [`SceneInner::binary_operator_to_override`](super::SceneInner::binary_operator_to_override) is the only place that
/// builds one.
pub(crate) struct BinaryOperatorRoute {
    /// This edge's own already-split anchor point on the operator — see [`crate::geometry::binary_operator_anchors`]
    pub(crate) anchor: Point,
    /// The side of the operator `anchor` sits on.
    pub(crate) side: Side,
    /// The sibling edge's own already-split anchor point on the operator — `Some` only when the sibling input lands on
    /// this same `side`, the one case [`binary_operator_elbow_route`]'s own sibling-aware routing applies.
    ///
    /// `None` when the two inputs land on different sides of the operator. [`binary_operator_anchors`]'s own doc
    /// comment already documents that a different-side input behaves exactly like an ordinary edge would. `route`
    /// honours that here by falling back to plain [`elbow_route`]. It does not call [`binary_operator_elbow_route`]
    /// with an unrelated sibling coordinate from a genuinely different side, which that function's own drift comparison
    /// assumes never happens.
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
/// endpoint's — one endpoint can use `None` while the other uses `Some`. `from_side`/`to_side` are this connector's own
/// forced sides, from [`ConnectorOptions::from_side`](crate::scene::ConnectorOptions::from_side)/
/// [`to_side`](crate::scene::ConnectorOptions::to_side) — independent of each other and of `from_anchors`/
/// `to_anchors`, the same way `EdgeAnchors` already is.
///
/// `to_override`, when `Some`, replaces the `to`-side anchor this would otherwise compute from `to_anchors`/ `to_side`.
/// See [`SceneInner::binary_operator_to_override`](super::SceneInner::binary_operator_to_override) for the one case
/// that supplies it. That is a binary operator node's own two inputs, split apart and routed clear of each other when
/// they land on the same side.
#[allow(clippy::too_many_arguments)]
pub(crate) fn route(
    connector_type: ConnectorType,
    from: Rect,
    from_anchors: Option<EdgeAnchors>,
    from_side: Option<Pin>,
    to: Rect,
    to_anchors: Option<EdgeAnchors>,
    to_side: Option<Pin>,
    from_end: End,
    to_end: End,
    to_override: Option<BinaryOperatorRoute>,
) -> (Route, f64) {
    let from_centre = centre(from);
    let to_centre = centre(to);

    // Each end is worked out from its own shape and its own direction: the other end is only ever a centre to aim at.
    let straight_end = |rect: Rect, own_centre: Point, towards: Point, anchors, side, end: End| match end.radii {
        Some(radii) => ellipse_straight_anchor(own_centre, radii, side, end.clock, towards),
        None => straight_anchor(rect, towards, anchors, side),
    };
    let elbow_end = |rect: Rect, own_centre: Point, towards: Point, anchors, side, end: End| match end.radii {
        Some(radii) => ellipse_elbow_anchor(own_centre, radii, side, end.clock, towards),
        None => elbow_anchor(rect, towards, anchors, side),
    };

    match connector_type {
        ConnectorType::Straight => {
            let start = straight_end(from, from_centre, to_centre, from_anchors, from_side, from_end);
            let end = to_override.map_or_else(
                || straight_end(to, to_centre, from_centre, to_anchors, to_side, to_end),
                |o| o.anchor,
            );
            (straight_route(start, end), 0.0)
        },
        ConnectorType::Elbow { corner_radius } => {
            let (start, start_side) = elbow_end(from, from_centre, to_centre, from_anchors, from_side, from_end);
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
                    let (end, end_side) = elbow_end(to, to_centre, from_centre, to_anchors, to_side, to_end);
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
    /// Returns [`Error::InvalidConnectorPosition`] if `options.from_position`/`to_position` is not finite and in
    /// `0.0..=1.0`.
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
        validate_connector_options(options)?;

        let mut inner = self.inner.borrow_mut();
        let from_rect = inner.node_rect(from)?;
        let from_anchors = inner.node_edge_anchors(from)?;
        let to_rect = inner.node_rect(to)?;
        let to_anchors = inner.node_edge_anchors(to)?;

        if from == to {
            return Err(Error::SelfLoopUnsupported(from));
        }

        validate_ends(&inner, from, to, options)?;

        let to_override = inner.binary_operator_to_override(from, to);
        let (from_end, to_end) = (inner.end_of(from, options.from_clock), inner.end_of(to, options.to_clock));
        let (vertices, radius) = route(
            options.connector_type,
            from_rect,
            from_anchors,
            options.from_side.map(|side| Pin {
                side,
                position: options.from_position,
            }),
            to_rect,
            to_anchors,
            options.to_side.map(|side| Pin {
                side,
                position: options.to_position,
            }),
            from_end,
            to_end,
            to_override,
        );
        // Taken out for the call so `inner.svg` can be borrowed for it without also needing `inner` mutability. See
        // `SceneInner::scratch`'s own doc comment for why this, rather than a fresh `String` per new edge.
        let mut d = std::mem::take(&mut inner.scratch);
        elbow_path_into(&vertices, radius, &mut d);
        let path_result = inner.svg.path(&d);
        inner.scratch = d;
        let path = path_result?;
        inner.attach(&path)?;
        path.set_fill("none")?;
        path.set_stroke(CONNECTOR_STROKE)?;
        path.set_stroke_width(CONNECTOR_WIDTH)?;
        path.set_marker_end_ref(&inner.arrow)?;

        let id = inner.graph.add_edge(from, to);
        inner.insert_edge_handle(
            id,
            ConnectorHandle {
                path,
                connector_type: options.connector_type,
                from_side: options.from_side,
                to_side: options.to_side,
                from_position: options.from_position,
                to_position: options.to_position,
                from_clock: options.from_clock,
                to_clock: options.to_clock,
                port_marker: None,
                route: Cell::new(vertices),
                radius: Cell::new(radius),
                focused: false,
                label: None,
            },
        );

        // Every edge, not just an operator's own auto-wired input, gets both endpoints' own descriptions extended this
        // way. So a `<path>` is never the only place that conveys which node feeds which — see
        // `SceneInner::append_relationship`'s own doc comment.
        let from_name = inner.node_handle(from).ok_or(Error::UnknownNode(from))?.current_ref_name();
        let to_name = inner.node_handle(to).ok_or(Error::UnknownNode(to))?.current_ref_name();
        inner.append_relationship(from, &format!("Output to {to_name}"))?;
        inner.append_relationship(to, &format!("Input from {from_name}"))?;

        Ok(id)
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    /// Emphasises the connector of edge `id` if `focused` is `true`, and returns it to the ordinary look if `false`. A
    /// focused connector is drawn in a deep blue and more than twice as thick, and carries `aria-current="true"`.
    ///
    /// Focus is a state of each edge on its own. Any number of edges can be focused at once, which is how a whole path
    /// is shown. It is not "the current edge", and focusing one never unfocuses another. It changes the look of the line
    /// and nothing else: the route, the endpoints, the label, and every node, and what the caller knows, are as they were.
    /// It is independent of [`set_focus`](Self::set_focus), which rings a node. The two draw on different elements and
    /// neither overrides the other.
    ///
    /// Colour is not the only thing that sets a focused edge apart: the extra thickness does, and so does the
    /// attribute. A caller who needs the state in words, such as a step explanation, writes it itself.
    ///
    /// Setting an edge to the state it is already in changes nothing and writes nothing. If a write fails part way,
    /// the edge is still recorded in its old state, so repeating the call writes it all again.
    ///
    /// # Errors
    ///
    /// Returns [`Error::UnknownEdge`] if `id` does not name an edge in this scene, or a wrapped [`Error::Svg`] if a write
    /// fails.
    pub fn set_edge_focus(&self, id: EdgeId, focused: bool) -> Result<(), Error> {
        let mut inner = self.inner.borrow_mut();
        let handle = inner.edge_handle_mut(id).ok_or(Error::UnknownEdge(id))?;
        if handle.focused == focused {
            return Ok(());
        }
        if focused {
            handle.path.set_stroke(EDGE_FOCUS_STROKE)?;
            handle.path.set_stroke_width(FOCUSED_WIDTH)?;
            handle.path.set_attr("aria-current", "true")?;
            handle.path.set_attr(FOCUS_ATTRIBUTE, "true")?;
        } else {
            handle.path.set_stroke(CONNECTOR_STROKE)?;
            handle.path.set_stroke_width(CONNECTOR_WIDTH)?;
            handle.path.remove_attr("aria-current")?;
            handle.path.remove_attr(FOCUS_ATTRIBUTE)?;
        }
        handle.focused = focused;
        Ok(())
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
    /// Returns [`Error::InvalidEllipseElbowClock`] if `connector_type` is an elbow and either end of the edge was given a
    /// clock position that is not 0, 15, 30 or 45. Checked before anything is redrawn or recorded, so the edge is left as
    /// it was.
    ///
    /// Also returns an error — [`Error::UnknownNode`] or a wrapped [`Error::Svg`] — if the redraw itself fails once
    /// underway. The stored connector type is only updated once the new path has actually been written. A failure of
    /// the path leaves `id` rendered and recorded exactly as it was before the call.
    ///
    /// The edge's label is placed after that, and a failure there is also returned, but by then the new type is
    /// already drawn and recorded: the call has partly succeeded. The label is where it was, and is placed again by the
    /// next call that places any label, by the end of a drag, or by a move that leaves a node where it is, whichever
    /// comes first.
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
        // An elbow can use only a cardinal clock position, so a straight edge that has any other cannot become one. This
        // is checked before anything is redrawn or recorded.
        if matches!(connector_type, ConnectorType::Elbow { .. }) {
            for position in [handle.from_clock, handle.to_clock].into_iter().flatten() {
                if !position.is_cardinal() {
                    return Err(Error::InvalidEllipseElbowClock(position));
                }
            }
        }

        // Taken out for the call so `redraw_edge_with_type` can freely borrow the rest of `inner`, then put back. See
        // `SceneInner::scratch`'s own doc comment for why this, rather than a fresh `String` per call.
        let mut scratch = std::mem::take(&mut inner.scratch);
        let result = inner.redraw_edge_with_type(id, connector_type, &mut scratch);
        inner.scratch = scratch;
        result
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[cfg(test)]
mod unit_tests;
