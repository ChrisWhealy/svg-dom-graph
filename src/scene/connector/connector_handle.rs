use crate::{geometry::side::Side, scene::connector::connector_type::ConnectorType};
use svg_dom::SvgNode;

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The rendered `<path>` for one edge's connector, plus everything about it `redraw_edge` has no other way to learn
/// once a node move forces a reroute — the [`ConnectorType`] and the forced `from`/`to` side (if any) it was created
/// with. These values must live alongside the rendered handle, not just get used once at creation.
pub(crate) struct ConnectorHandle {
    pub(crate) path: SvgNode,
    pub(crate) connector_type: ConnectorType,
    /// This connector's own forced `from`-endpoint side, from [`ConnectorOptions::from_side`](
    /// crate::scene::ConnectorOptions::from_side) — `None` for the ordinary, automatically chosen side every edge used
    /// before that option existed.
    pub(crate) from_side: Option<Side>,
    /// The same forced choice as [`from_side`](Self::from_side), for this connector's own `to` endpoint instead.
    pub(crate) to_side: Option<Side>,

    /// The small "L"/"R" text label marking this edge's operand identity at a non-commutative two-input operator's own
    /// input anchor. `None` for every other edge — see `add_two_input_operator_node_with`'s own `commutes` parameter.
    pub(crate) port_marker: Option<SvgNode>,
}
