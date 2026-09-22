use crate::scene::connector::connector_type::ConnectorType;
use svg_dom::SvgNode;

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The rendered `<path>` for one edge's connector, plus the [`ConnectorType`] it was created with.
///
/// `redraw_edge` has no other way to learn an edge's connector type once a node move forces a reroute. This value must
/// live alongside the rendered handle, not just get used once at creation.
pub(crate) struct ConnectorHandle {
    pub(crate) path: SvgNode,
    pub(crate) connector_type: ConnectorType,

    /// The small "L"/"R" text label marking this edge's operand identity at a non-commutative two-input operator's
    /// own input anchor. `None` for every other edge — see `add_two_input_operator_node_with`'s own `commutes`
    /// parameter.
    pub(crate) port_marker: Option<SvgNode>,
}
