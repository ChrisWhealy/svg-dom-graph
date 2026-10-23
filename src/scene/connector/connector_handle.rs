use super::edge_label::EdgeLabel;
use crate::{
    geometry::{route::Route, side::Side},
    scene::connector::connector_type::ConnectorType,
};
use std::cell::Cell;
use svg_dom::SvgNode;

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The rendered `<path>` for one edge's connector, plus everything about it `redraw_edge` has no other way to learn
/// once a node move forces a reroute. That is the [`ConnectorType`] and the forced `from`/`to` side (if any) it was
/// created with. These values must live alongside the rendered handle, not just get used once at creation.
pub(crate) struct ConnectorHandle {
    pub(crate) path: SvgNode,
    pub(crate) connector_type: ConnectorType,
    /// This connector's own forced `from`-endpoint side, from [`ConnectorOptions::from_side`](
    /// crate::scene::ConnectorOptions::from_side) — `None` for the ordinary, automatically chosen side every edge used
    /// before that option existed.
    pub(crate) from_side: Option<Side>,
    /// The same forced choice as [`from_side`](Self::from_side), for this connector's own `to` endpoint instead.
    pub(crate) to_side: Option<Side>,
    /// The fraction along `from_side` this connector's own `from` endpoint is pinned to, if any.
    pub(crate) from_position: Option<f64>,
    /// The same pin as [`from_position`](Self::from_position), for the `to` endpoint.
    pub(crate) to_position: Option<f64>,
    /// The clock positions this connector's own ends were given, for an end that is an ellipse. They are never changed
    /// after the edge is added.
    pub(crate) from_clock: Option<crate::scene::ClockPosition>,
    pub(crate) to_clock: Option<crate::scene::ClockPosition>,

    /// The small "L"/"R" text label marking this edge's operand identity at a non-commutative two-input operator's own
    /// input anchor. `None` for every other edge — see `add_two_input_operator_node_with`'s own `commutes` parameter.
    pub(crate) port_marker: Option<SvgNode>,

    /// The route this connector's path was last written from, and the radius its corners were rounded by. A label is
    /// placed from these, so that the path and the label always come from one route, and a label's place can be worked
    /// out again without computing the route a second time. Cells, so that a redraw that only has `&self` can store
    /// them.
    pub(crate) route: Cell<Route>,
    pub(crate) radius: Cell<f64>,

    /// Whether this edge is focused. See [`Scene::set_edge_focus`](crate::scene::Scene::set_edge_focus). It is only
    /// changed once the path's own attributes have been written, so a call that failed part way is repeated in full.
    pub(crate) focused: bool,

    /// This edge's label, if it has one. See [`Scene::set_edge_label`](crate::scene::Scene::set_edge_label).
    pub(crate) label: Option<EdgeLabel>,
}

impl ConnectorHandle {
    /// The `from` endpoint's forced side and pin, if a side is forced.
    pub(crate) fn source_pin(&self) -> Option<Pin> {
        self.from_side.map(|side| Pin {
            side,
            position: self.from_position,
        })
    }

    /// The `to` endpoint's forced side and pin, if a side is forced.
    pub(crate) fn target_pin(&self) -> Option<Pin> {
        self.to_side.map(|side| Pin {
            side,
            position: self.to_position,
        })
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// One endpoint's forced side, plus where along it the endpoint is pinned. `position` is `None` when only the side is
/// forced.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct Pin {
    pub(crate) side: Side,
    pub(crate) position: Option<f64>,
}
