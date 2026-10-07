use crate::{geometry::side::Side, scene::connector::connector_type::ConnectorType};
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

    /// The small "L"/"R" text label marking this edge's operand identity at a non-commutative two-input operator's own
    /// input anchor. `None` for every other edge — see `add_two_input_operator_node_with`'s own `commutes` parameter.
    pub(crate) port_marker: Option<SvgNode>,
}

impl ConnectorHandle {
    /// The `from` endpoint's forced side and pin, if a side is forced.
    pub(crate) fn from_pin(&self) -> Option<Pin> {
        self.from_side.map(|side| Pin {
            side,
            position: self.from_position,
        })
    }

    /// The `to` endpoint's forced side and pin, if a side is forced.
    pub(crate) fn to_pin(&self) -> Option<Pin> {
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
